//! Autofill on the desktop (D-31, D-52): global shortcuts, the window in front, and filling
//! its fields. The decisions (what matches, which keys in which order) live in
//! `cleis_core::autofill`; this crate only talks to the operating system.
//!
//! - Windows: UI Automation (focused field kind, browser address, writing into native
//!   fields), `SendInput` through `enigo` for typing, Win32 for the window in front.
//! - macOS: AppKit for the app in front; typing through `enigo` (needs the Accessibility
//!   permission). Field kinds are not read yet: plans run as classic auto-type.
//! - Other systems: typing only, no window detection (development builds).
//!
//! Safety rule enforced by [`run`]: before every step the window in front must still be the
//! one the user asked to fill, so a secret is never typed into another window.

pub mod hotkeys;
mod platform;

use cleis_core::autofill::{Action, Context, Field, Focus, Key, Plan};
use cleis_core::settings::{FillMethod, TypingSpeed};
use std::time::Duration;
use thiserror::Error;
use zeroize::Zeroizing;

pub use platform::{accessibility_allowed, request_accessibility};

/// Opening Cleis without the master password (D-76).
pub mod unlock {
    /// Whether Windows Hello can sign for this account (Windows only for now).
    pub fn hello_available() -> bool {
        crate::platform::hello_available()
    }

    /// The Windows Hello signature of `challenge` by the key `name`: the user is asked first.
    /// Blocks until the prompt closes: call it off the interface thread.
    pub fn hello_sign(name: &str, challenge: &[u8]) -> Result<Vec<u8>, String> {
        crate::platform::hello_sign(name, challenge)
    }

    /// Seals a key for this system account ("Nothing": Windows DPAPI; elsewhere unchanged).
    pub fn protect(secret: &[u8]) -> Result<Vec<u8>, String> {
        crate::platform::protect(secret)
    }

    pub fn unprotect(stored: &[u8]) -> Result<Vec<u8>, String> {
        crate::platform::unprotect(stored)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FillError {
    #[error("the window changed before filling finished")]
    WindowChanged,
    #[error("the window to fill is gone")]
    WindowGone,
    #[error("the system did not let Cleis bring the window back in front")]
    CannotActivate,
    #[error("keyboard simulation failed: {0}")]
    Input(String),
    #[error("clipboard unavailable: {0}")]
    Clipboard(String),
    #[error("Cleis is not allowed to control the computer (Accessibility permission)")]
    NotAllowed,
    #[error("the focus is not in a text field")]
    NotAField,
}

/// A window, as the system names it: `HWND` on Windows, process id on macOS.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowRef(pub u64);

/// The window in front and what Cleis knows about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Foreground {
    pub window: WindowRef,
    pub context: Context,
    /// Monitor showing the window (Windows `HMONITOR`, the id GPUI gives that display), so
    /// Quick Access opens on the same screen.
    pub monitor: Option<u64>,
}

/// Window in front, read when a shortcut is pressed (before Quick Access takes the focus).
pub fn foreground() -> Option<Foreground> {
    platform::foreground()
}

/// Address of the active tab of a browser window, read through accessibility. Can take a few
/// hundred milliseconds the first time Chrome builds its accessibility tree: call it off the
/// interface thread.
pub fn browser_url(window: WindowRef) -> Option<String> {
    platform::browser_url(window)
}

/// Brings a window back in front (after Quick Access closes).
pub fn activate(window: WindowRef) -> Result<(), FillError> {
    platform::activate(window)
}

/// Shows or hides one of Cleis's own windows (main window kept in the background, tray). No
/// effect where the system does it itself (macOS hides the whole app).
pub fn set_window_visible(window: WindowRef, visible: bool) {
    platform::set_window_visible(window, visible);
}

/// Starts Cleis when the user signs in to the computer: `exe` with `args`. Windows: a value under
/// `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`; macOS: a LaunchAgent in
/// `~/Library/LaunchAgents`. Other systems: nothing.
pub fn set_open_at_login(on: bool, exe: &std::path::Path, args: &[&str]) -> Result<(), String> {
    platform::set_open_at_login(on, exe, args)
}

static LOG: std::sync::Mutex<Option<std::path::PathBuf>> = std::sync::Mutex::new(None);

/// Diagnostic log of fills (`--try-autofill`): steps, field kinds and outcomes, never values.
pub fn set_log_file(path: Option<std::path::PathBuf>) {
    if let Ok(mut l) = LOG.lock() {
        *l = path;
    }
}

pub fn log(line: &str) {
    use std::io::Write;
    let Ok(l) = LOG.lock() else { return };
    let Some(path) = l.as_ref() else { return };
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let _ = writeln!(f, "{t} {line}");
    }
}

/// Values a plan can put into fields. Wiped when dropped.
#[derive(Default)]
pub struct Values {
    pub username: Zeroizing<String>,
    pub password: Zeroizing<String>,
    /// Current 2FA code, computed just before the run.
    pub totp: Option<Zeroizing<String>>,
}

impl Values {
    fn get(&self, f: Field) -> &str {
        match f {
            Field::Username => &self.username,
            Field::Password => &self.password,
            Field::Totp => self.totp.as_deref().map_or("", |s| s.as_str()),
        }
    }
}

/// How a run ended.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Done,
    /// The focused field was not a password field where one was expected: these steps wait for
    /// the next "Fill best match" on the same window.
    Waiting(Vec<Action>),
}

/// What the executor needs from the system. Implemented per platform, and by a recorder in
/// tests.
pub trait Driver {
    /// The window being filled is still the one in front.
    fn still_target(&mut self) -> Result<bool, FillError>;
    fn focus(&mut self) -> Focus;
    /// Writes `value` into the focused field through accessibility. `false` when the field
    /// does not allow it (web pages, custom controls): the caller types instead.
    fn set_value(&mut self, value: &str) -> bool;
    fn type_text(&mut self, value: &str, speed: TypingSpeed) -> Result<(), FillError>;
    fn press(&mut self, key: Key) -> Result<(), FillError>;
    fn paste(&mut self, value: &str) -> Result<(), FillError>;
    fn pause(&mut self, d: Duration);
}

/// Runs a plan's actions against `driver`.
pub fn run<D: Driver>(
    driver: &mut D,
    actions: &[Action],
    values: &Values,
    method: FillMethod,
    speed: TypingSpeed,
) -> Result<Outcome, FillError> {
    for (i, a) in actions.iter().enumerate() {
        if !driver.still_target()? {
            return Err(FillError::WindowChanged);
        }
        match a {
            Action::Fill(f) => {
                let v = values.get(*f);
                if v.is_empty() {
                    continue;
                }
                // Never type a secret into a button, a menu or a list: its letters would act as
                // shortcuts there (seen on Windows with Notepad's settings, 25/11/2026).
                let focus = driver.focus();
                if focus == Focus::Other {
                    log(&format!("run: {f:?} refused, focus {focus:?}"));
                    return Err(FillError::NotAField);
                }
                match method {
                    FillMethod::Paste => driver.paste(v)?,
                    FillMethod::Type => driver.type_text(v, speed)?,
                    FillMethod::Automatic => {
                        if !driver.set_value(v) {
                            driver.type_text(v, speed)?;
                        }
                    }
                }
            }
            Action::Press(k) => driver.press(*k)?,
            Action::ExpectPassword { submit } => match driver.focus() {
                Focus::Password | Focus::Unknown => {}
                other => {
                    log(&format!("run: expected a password field, found {other:?}"));
                    if *submit {
                        driver.press(Key::Enter)?;
                    }
                    return Ok(Outcome::Waiting(actions[i + 1..].to_vec()));
                }
            },
            Action::IfEmptyText { then, otherwise } => {
                let branch = if driver.focus() == Focus::EmptyText {
                    then
                } else {
                    otherwise
                };
                if let Outcome::Waiting(rest) = run(driver, branch, values, method, speed)? {
                    let mut rest = rest;
                    rest.extend_from_slice(&actions[i + 1..]);
                    return Ok(Outcome::Waiting(rest));
                }
            }
        }
        // Let the app react (focus moves after Tab, pages validate fields).
        driver.pause(Duration::from_millis(60));
    }
    Ok(Outcome::Done)
}

/// Fills `window`: brings it back in front, waits for the shortcut keys to be released, reads
/// the focused field, asks `make` for the plan, and runs it. Returns how the run ended and the
/// steps the plan keeps for later (2FA page). Blocking: call from a background thread.
pub fn fill(
    window: WindowRef,
    values: &Values,
    method: FillMethod,
    speed: TypingSpeed,
    make: impl FnOnce(Focus) -> Plan,
) -> Result<(Outcome, Vec<Action>), FillError> {
    if !platform::accessibility_allowed() {
        return Err(FillError::NotAllowed);
    }
    log(&format!("fill: window {:x}, method {method:?}", window.0));
    if let Err(e) = platform::activate(window) {
        log(&format!("fill: {e}"));
        return Err(e);
    }
    std::thread::sleep(Duration::from_millis(150));
    platform::wait_modifiers_released(Duration::from_secs(2));
    let mut d = platform::SystemDriver::new(window)?;
    let focus = d.focus();
    let plan = make(focus);
    log(&format!(
        "fill: focus {focus:?}, plan {:?}, later {:?}",
        plan.now, plan.later
    ));
    let outcome = run(&mut d, &plan.now, values, method, speed);
    log(&format!("fill: outcome {outcome:?}"));
    Ok((outcome?, plan.later))
}

/// Delay between typed characters.
pub fn char_delay(speed: TypingSpeed) -> Option<Duration> {
    match speed {
        TypingSpeed::Fast => None,
        TypingSpeed::Normal => Some(Duration::from_millis(8)),
        TypingSpeed::Slow => Some(Duration::from_millis(35)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cleis_core::autofill::{PlanOptions, plan};

    /// Records what would reach the system; the focus follows a scripted form.
    struct Form {
        log: Vec<String>,
        /// Kinds of the fields in tab order; `at` is the focused one.
        fields: Vec<Focus>,
        at: usize,
        front: bool,
        accept_set: bool,
    }

    impl Driver for Form {
        fn still_target(&mut self) -> Result<bool, FillError> {
            Ok(self.front)
        }
        fn focus(&mut self) -> Focus {
            self.fields.get(self.at).copied().unwrap_or(Focus::Unknown)
        }
        fn set_value(&mut self, v: &str) -> bool {
            if self.accept_set {
                self.log.push(format!("set:{v}"));
            }
            self.accept_set
        }
        fn type_text(&mut self, v: &str, _: TypingSpeed) -> Result<(), FillError> {
            self.log.push(format!("type:{v}"));
            if self.fields.get(self.at) == Some(&Focus::EmptyText) {
                self.fields[self.at] = Focus::Text;
            }
            Ok(())
        }
        fn press(&mut self, k: Key) -> Result<(), FillError> {
            self.log.push(format!("{k:?}"));
            match k {
                Key::Tab => self.at += 1,
                Key::BackTab => self.at = self.at.saturating_sub(1),
                Key::Enter => {}
            }
            Ok(())
        }
        fn paste(&mut self, v: &str) -> Result<(), FillError> {
            self.log.push(format!("paste:{v}"));
            Ok(())
        }
        fn pause(&mut self, _: Duration) {}
    }

    fn values() -> Values {
        Values {
            username: Zeroizing::new("ines@example-studio.com".into()),
            password: Zeroizing::new("Bk-2026!".into()),
            totp: Some(Zeroizing::new("518204".into())),
        }
    }

    const O: PlanOptions = PlanOptions {
        press_enter: true,
        totp_after: true,
    };

    #[test]
    fn classic_form_from_the_email_field() {
        let mut f = Form {
            log: vec![],
            fields: vec![Focus::EmptyText, Focus::Password],
            at: 0,
            front: true,
            accept_set: false,
        };
        let p = plan(true, true, f.focus(), O);
        let r = run(
            &mut f,
            &p.now,
            &values(),
            FillMethod::Automatic,
            TypingSpeed::Fast,
        );
        assert_eq!(r, Ok(Outcome::Done));
        assert_eq!(
            f.log,
            [
                "type:ines@example-studio.com",
                "Tab",
                "type:Bk-2026!",
                "Enter"
            ]
        );
    }

    /// Mockup 17: the password field has the focus and the email field is empty.
    #[test]
    fn focus_in_password_fills_the_empty_email_first() {
        let mut f = Form {
            log: vec![],
            fields: vec![Focus::EmptyText, Focus::Password],
            at: 1,
            front: true,
            accept_set: true,
        };
        let p = plan(true, false, f.focus(), O);
        run(
            &mut f,
            &p.now,
            &values(),
            FillMethod::Automatic,
            TypingSpeed::Fast,
        )
        .unwrap();
        assert_eq!(
            f.log,
            [
                "BackTab",
                "set:ines@example-studio.com",
                "Tab",
                "set:Bk-2026!",
                "Enter"
            ]
        );
    }

    /// Email already there (remembered by the site): left untouched.
    #[test]
    fn prefilled_email_is_kept() {
        let mut f = Form {
            log: vec![],
            fields: vec![Focus::Text, Focus::Password],
            at: 1,
            front: true,
            accept_set: false,
        };
        let p = plan(true, false, f.focus(), O);
        run(
            &mut f,
            &p.now,
            &values(),
            FillMethod::Type,
            TypingSpeed::Fast,
        )
        .unwrap();
        assert_eq!(f.log, ["BackTab", "Tab", "type:Bk-2026!", "Enter"]);
    }

    /// The rest of a two-step sign-in while the focus is on a button: nothing is typed.
    #[test]
    fn secret_never_typed_into_a_button() {
        let mut f = Form {
            log: vec![],
            fields: vec![Focus::Other],
            at: 0,
            front: true,
            accept_set: false,
        };
        let r = run(
            &mut f,
            &[Action::Fill(Field::Password), Action::Press(Key::Enter)],
            &values(),
            FillMethod::Automatic,
            TypingSpeed::Fast,
        );
        assert!(matches!(r, Err(FillError::NotAField)));
        assert!(f.log.is_empty());
    }

    /// Two-step sign-in (email page, then password page): the password is not typed into
    /// the "Next" button; it waits for the next shortcut.
    #[test]
    fn two_step_sign_in_waits() {
        let mut f = Form {
            log: vec![],
            fields: vec![Focus::EmptyText, Focus::Other],
            at: 0,
            front: true,
            accept_set: false,
        };
        let p = plan(true, true, f.focus(), O);
        let r = run(
            &mut f,
            &p.now,
            &values(),
            FillMethod::Automatic,
            TypingSpeed::Fast,
        )
        .unwrap();
        assert_eq!(f.log, ["type:ines@example-studio.com", "Tab", "Enter"]);
        let Outcome::Waiting(rest) = r else { panic!() };
        assert_eq!(
            rest,
            vec![Action::Fill(Field::Password), Action::Press(Key::Enter)]
        );
    }

    #[test]
    fn stops_when_another_window_comes_in_front() {
        let mut f = Form {
            log: vec![],
            fields: vec![],
            at: 0,
            front: false,
            accept_set: false,
        };
        let p = plan(true, false, Focus::Unknown, O);
        assert_eq!(
            run(
                &mut f,
                &p.now,
                &values(),
                FillMethod::Type,
                TypingSpeed::Fast
            ),
            Err(FillError::WindowChanged)
        );
        assert!(f.log.is_empty(), "nothing may be typed: {:?}", f.log);
    }

    #[test]
    fn paste_method() {
        let mut f = Form {
            log: vec![],
            fields: vec![],
            at: 0,
            front: true,
            accept_set: true,
        };
        let p = plan(false, false, Focus::Unknown, O);
        run(
            &mut f,
            &p.now,
            &values(),
            FillMethod::Paste,
            TypingSpeed::Fast,
        )
        .unwrap();
        assert_eq!(f.log, ["paste:Bk-2026!", "Enter"]);
    }
}
