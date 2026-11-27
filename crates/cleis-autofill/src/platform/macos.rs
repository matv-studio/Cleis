//! macOS: the app in front through AppKit (`NSWorkspace`), typing through `enigo`, which
//! requires the Accessibility permission (System Settings › Privacy & Security).
//!
//! Field kinds and browser addresses are not read yet on macOS: plans run as classic
//! auto-type, and items are matched by link or window name.
#![allow(unsafe_code)]

use super::keyboard::Keyboard;
use crate::{Driver, FillError, Foreground, WindowRef};
use cleis_core::autofill::{Context, Focus, Key};
use cleis_core::settings::TypingSpeed;
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication, NSWorkspace};
use std::time::{Duration, Instant};

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

fn front_pid() -> Option<i32> {
    let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
    Some(app.processIdentifier())
}

pub fn foreground() -> Option<Foreground> {
    let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
    let pid = app.processIdentifier();
    let program = app
        .bundleIdentifier()
        .map(|s| s.to_string())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let app_name = app
        .localizedName()
        .map(|s| s.to_string())
        .unwrap_or_default();
    Some(Foreground {
        window: WindowRef(pid as u64),
        monitor: None,
        context: Context {
            program,
            title: app_name.clone(),
            app_name,
            url: None,
        },
    })
}

pub fn browser_url(_: WindowRef) -> Option<String> {
    None
}

pub fn activate(w: WindowRef) -> Result<(), FillError> {
    let pid = w.0 as i32;
    let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) else {
        return Err(FillError::WindowGone);
    };
    app.activateWithOptions(NSApplicationActivationOptions::empty());
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(800) {
        if front_pid() == Some(pid) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    if front_pid() == Some(pid) {
        Ok(())
    } else {
        Err(FillError::CannotActivate)
    }
}

pub fn accessibility_allowed() -> bool {
    // SAFETY: no arguments; reads this process's trust state.
    unsafe { AXIsProcessTrusted() }
}

/// Opens the system prompt that leads to Privacy & Security › Accessibility.
pub fn request_accessibility() -> bool {
    if accessibility_allowed() {
        return true;
    }
    // enigo shows the system prompt when created with the prompt setting on.
    let _ = enigo::Enigo::new(&enigo::Settings::default());
    accessibility_allowed()
}

/// enigo types independently of the keys held on the physical keyboard on macOS.
pub fn wait_modifiers_released(_: Duration) {}

pub struct SystemDriver {
    pid: i32,
    kb: Keyboard,
}

impl SystemDriver {
    pub fn new(w: WindowRef) -> Result<Self, FillError> {
        Ok(Self {
            pid: w.0 as i32,
            kb: Keyboard::new()?,
        })
    }
}

impl Driver for SystemDriver {
    fn still_target(&mut self) -> Result<bool, FillError> {
        Ok(front_pid() == Some(self.pid))
    }
    fn focus(&mut self) -> Focus {
        Focus::Unknown
    }
    fn set_value(&mut self, _: &str) -> bool {
        false
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

/// Nothing to mask: releasing a modifier opens no menu here.
pub fn mask_menu_key() {}

/// The app is hidden as a whole (`cx.hide()` in GPUI).
pub fn set_window_visible(_: WindowRef, _: bool) {}

const AGENT: &str = "studio.matv.cleis";

pub fn set_open_at_login(on: bool, exe: &std::path::Path, args: &[&str]) -> Result<(), String> {
    let home = std::env::var_os("HOME").ok_or("no home folder")?;
    let dir = std::path::Path::new(&home).join("Library/LaunchAgents");
    let file = dir.join(format!("{AGENT}.plist"));
    if !on {
        return match std::fs::remove_file(&file) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        };
    }
    let esc = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    let mut program = format!("<string>{}</string>", esc(&exe.to_string_lossy()));
    for a in args {
        program.push_str(&format!("<string>{}</string>", esc(a)));
    }
    let plist = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
<plist version=\"1.0\"><dict>\
<key>Label</key><string>{AGENT}</string>\
<key>ProgramArguments</key><array>{program}</array>\
<key>RunAtLoad</key><true/>\
</dict></plist>\n"
    );
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(&file, plist).map_err(|e| e.to_string())
}

/// Opening Cleis (D-76): Windows only for now.
pub fn hello_available() -> bool {
    false
}

pub fn hello_sign(_name: &str, _challenge: &[u8]) -> Result<Vec<u8>, String> {
    Err("Windows Hello is not available on this system".into())
}

/// D-83: the login Keychain keeps one random key for this user; the secrets of "Nothing" and of
/// the PIN are sealed with it (as DPAPI does on Windows). The Keychain asks the user once if
/// another build of Cleis reads it; refused or locked, nothing opens and the master password is
/// asked.
const KEYCHAIN_SERVICE: &str = "studio.matv.cleis";
const KEYCHAIN_ACCOUNT: &str = "device-key";
/// `errSecItemNotFound`.
const ITEM_NOT_FOUND: i32 = -25300;

fn keychain_error(e: security_framework::base::Error) -> String {
    e.message()
        .unwrap_or_else(|| format!("Keychain error {}", e.code()))
}

/// The Keychain's key; made and kept the first time when `create`.
fn device_key(create: bool) -> Result<cleis_crypto::SymmetricKey, String> {
    use security_framework::passwords::{
        PasswordOptions, generic_password, set_generic_password_options,
    };
    match generic_password(PasswordOptions::new_generic_password(
        KEYCHAIN_SERVICE,
        KEYCHAIN_ACCOUNT,
    )) {
        Ok(v) => {
            let v = zeroize::Zeroizing::new(v);
            cleis_crypto::SymmetricKey::from_slice(&v).map_err(|e| e.to_string())
        }
        Err(e) if e.code() == ITEM_NOT_FOUND && create => {
            let key = cleis_crypto::SymmetricKey::generate().map_err(|e| e.to_string())?;
            let mut o = PasswordOptions::new_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT);
            o.set_label("Cleis");
            o.set_description("Opens Cleis on this Mac without the master password");
            set_generic_password_options(key.as_bytes(), o).map_err(keychain_error)?;
            Ok(key)
        }
        Err(e) => Err(keychain_error(e)),
    }
}

pub fn protect(secret: &[u8]) -> Result<Vec<u8>, String> {
    crate::unlock::keychain_seal(&device_key(true)?, secret)
}

pub fn unprotect(stored: &[u8]) -> Result<Vec<u8>, String> {
    if !stored.starts_with(crate::unlock::KEYCHAIN_MAGIC) {
        // Written unsealed by an earlier version: read as is (sealed when next written).
        return Ok(stored.to_vec());
    }
    let key = device_key(false)?;
    crate::unlock::keychain_open(&key, stored).unwrap_or_else(|| Err("not sealed".into()))
}

/// Whether the system wants animations (macOS "Reduce motion": to read when the Mac app is built).
pub fn animations_enabled() -> bool {
    true
}
