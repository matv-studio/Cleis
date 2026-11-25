//! Windows: Win32 for the window in front, UI Automation for fields and browser addresses.
//!
//! The only `unsafe` in Cleis outside its dependencies (D-52): plain Win32 calls with
//! buffers owned here; each block says what it relies on.
#![allow(unsafe_code)]

use super::keyboard::Keyboard;
use crate::{Driver, FillError, Foreground, WindowRef};
use cleis_core::autofill::{Context, Focus, Key};
use cleis_core::settings::TypingSpeed;
use std::time::{Duration, Instant};
use uiautomation::patterns::UIValuePattern;
use uiautomation::types::{ControlType, Handle};
use uiautomation::{UIAutomation, UIElement};
use windows::Win32::Foundation::{CloseHandle, HWND};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId, IsIconic, SW_RESTORE,
    SetForegroundWindow, ShowWindow,
};
use windows::core::PWSTR;

fn hwnd(w: WindowRef) -> HWND {
    HWND(w.0 as usize as *mut core::ffi::c_void)
}

fn front() -> HWND {
    // SAFETY: no arguments; returns a handle value (possibly null), never dereferenced here.
    unsafe { GetForegroundWindow() }
}

/// Full path of the program that owns `pid`.
fn program_path(pid: u32) -> Option<String> {
    // SAFETY: OpenProcess only reads its arguments; the handle is closed below on every path.
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    // SAFETY: `buf` is valid for `len` UTF-16 units and outlives the call; `len` is updated
    // to the number of units written.
    let ok = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
    }
    .is_ok();
    // SAFETY: `process` came from OpenProcess above and is closed once.
    let _ = unsafe { CloseHandle(process) };
    ok.then(|| String::from_utf16_lossy(&buf[..len as usize]))
}

fn title(h: HWND) -> String {
    let mut buf = [0u16; 512];
    // SAFETY: the slice is valid for its length; the function writes at most that many units.
    let n = unsafe { GetWindowTextW(h, &mut buf) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

pub fn foreground() -> Option<Foreground> {
    let h = front();
    if h.is_invalid() {
        return None;
    }
    let mut pid = 0u32;
    // SAFETY: `pid` is a valid u32 the call writes into.
    unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
    let path = program_path(pid).unwrap_or_default();
    let file = path.rsplit(['\\', '/']).next().unwrap_or("").to_owned();
    let app_name = file
        .rsplit_once('.')
        .map_or(file.as_str(), |(stem, _)| stem)
        .to_owned();
    Some(Foreground {
        window: WindowRef(h.0 as usize as u64),
        context: Context {
            program: file.to_ascii_lowercase(),
            app_name,
            title: title(h),
            url: None,
        },
    })
}

fn automation() -> Option<UIAutomation> {
    // Each fill runs on its own thread: initialise COM there (multithreaded apartment).
    UIAutomation::new().ok()
}

/// First edit box of a browser window: the address bar in Chrome, Edge, Brave, Vivaldi,
/// Opera and Firefox, which all place it before the page in their accessibility tree.
pub fn browser_url(w: WindowRef) -> Option<String> {
    let ua = automation()?;
    let root = ua.element_from_handle(Handle::from(w.0 as isize)).ok()?;
    let bar = ua
        .create_matcher()
        .from(root)
        .control_type(ControlType::Edit)
        .depth(16)
        .timeout(600)
        .find_first()
        .ok()?;
    let v = bar.get_pattern::<UIValuePattern>().ok()?.get_value().ok()?;
    let v = v.trim().to_owned();
    (!v.is_empty()).then_some(v)
}

pub fn activate(w: WindowRef) -> bool {
    let h = hwnd(w);
    // SAFETY: plain calls on a window handle; Windows checks it and fails on a stale one.
    unsafe {
        if IsIconic(h).as_bool() {
            let _ = ShowWindow(h, SW_RESTORE);
        }
        let _ = SetForegroundWindow(h);
    }
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(800) {
        if front() == h {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    front() == h
}

pub fn accessibility_allowed() -> bool {
    true
}

pub fn request_accessibility() -> bool {
    true
}

/// Waits until Ctrl, Shift, Alt and Windows are up: typing while the shortcut's keys are
/// still held would turn characters into shortcuts.
pub fn wait_modifiers_released(max: Duration) {
    let start = Instant::now();
    let held = || {
        [VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN]
            .iter()
            // SAFETY: reads the asynchronous key state; no pointers involved.
            .any(|vk| unsafe { GetAsyncKeyState(i32::from(vk.0)) } as u16 & 0x8000 != 0)
    };
    while held() && start.elapsed() < max {
        std::thread::sleep(Duration::from_millis(15));
    }
}

pub struct SystemDriver {
    window: HWND,
    kb: Keyboard,
    ua: Option<UIAutomation>,
}

impl SystemDriver {
    pub fn new(w: WindowRef) -> Result<Self, FillError> {
        Ok(Self {
            window: hwnd(w),
            kb: Keyboard::new()?,
            ua: automation(),
        })
    }

    fn focused(&self) -> Option<UIElement> {
        self.ua.as_ref()?.get_focused_element().ok()
    }
}

/// Frameworks whose edit boxes take a value set through UI Automation as if typed. Web pages
/// (Chrome, Firefox) do not always see such a change, so they are typed into.
const NATIVE: [&str; 4] = ["Win32", "WinForm", "WPF", "XAML"];

impl Driver for SystemDriver {
    fn still_target(&mut self) -> Result<bool, FillError> {
        Ok(front() == self.window)
    }

    fn focus(&mut self) -> Focus {
        let Some(e) = self.focused() else {
            return Focus::Unknown;
        };
        let Ok(kind) = e.get_control_type() else {
            return Focus::Unknown;
        };
        if !matches!(
            kind,
            ControlType::Edit | ControlType::ComboBox | ControlType::Document
        ) {
            return Focus::Other;
        }
        if e.is_password().unwrap_or(false) {
            return Focus::Password;
        }
        match e
            .get_pattern::<UIValuePattern>()
            .and_then(|p| p.get_value())
        {
            Ok(v) if v.is_empty() => Focus::EmptyText,
            Ok(_) => Focus::Text,
            Err(_) => Focus::Unknown,
        }
    }

    fn set_value(&mut self, value: &str) -> bool {
        let Some(e) = self.focused() else {
            return false;
        };
        let native = e
            .get_framework_id()
            .is_ok_and(|f| NATIVE.iter().any(|n| f.eq_ignore_ascii_case(n)));
        if !native {
            return false;
        }
        let Ok(p) = e.get_pattern::<UIValuePattern>() else {
            return false;
        };
        if p.is_readonly().unwrap_or(true) {
            return false;
        }
        p.set_value(value).is_ok()
    }

    fn type_text(&mut self, v: &str, s: TypingSpeed) -> Result<(), FillError> {
        self.kb.type_text(v, s)
    }

    fn press(&mut self, k: Key) -> Result<(), FillError> {
        self.kb.press(k)
    }

    fn paste(&mut self, v: &str) -> Result<(), FillError> {
        self.kb.paste(v)
    }

    fn pause(&mut self, d: Duration) {
        std::thread::sleep(d);
    }
}
