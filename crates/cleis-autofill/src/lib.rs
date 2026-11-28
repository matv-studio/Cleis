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

/// Whether the system wants animations (Cleis' own, 26/11/2026, follow it).
pub fn animations_enabled() -> bool {
    platform::animations_enabled()
}

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

    /// Seals a secret for this system account ("Nothing", the PIN's device secret): Windows
    /// DPAPI; macOS a key kept in the login Keychain (D-83); elsewhere unchanged.
    pub fn protect(secret: &[u8]) -> Result<Vec<u8>, String> {
        crate::platform::protect(secret)
    }

    pub fn unprotect(stored: &[u8]) -> Result<Vec<u8>, String> {
        crate::platform::unprotect(stored)
    }

    /// Start of a secret sealed by the Keychain's key (macOS); anything else was written
    /// unsealed by an earlier version.
    pub const KEYCHAIN_MAGIC: &[u8; 4] = b"CLKC";
    /// Binds the sealed secret to its use.
    const KEYCHAIN_AAD: &[u8] = b"cleis/keychain/v1";

    /// `KEYCHAIN_MAGIC` then AES-256-GCM of `secret` (cleis-crypto's format, version byte first).
    pub fn keychain_seal(
        key: &cleis_crypto::SymmetricKey,
        secret: &[u8],
    ) -> Result<Vec<u8>, String> {
        let sealed = cleis_crypto::seal(key, KEYCHAIN_AAD, secret).map_err(|e| e.to_string())?;
        Ok([KEYCHAIN_MAGIC.as_slice(), &sealed].concat())
    }

    /// The secret of `keychain_seal`, or `None` if `stored` was not sealed this way.
    pub fn keychain_open(
        key: &cleis_crypto::SymmetricKey,
        stored: &[u8],
    ) -> Option<Result<Vec<u8>, String>> {
        let sealed = stored.strip_prefix(KEYCHAIN_MAGIC.as_slice())?;
        Some(cleis_crypto::open(key, KEYCHAIN_AAD, sealed).map_err(|e| e.to_string()))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn keychain_seal_round_trip_and_refusals() {
            let key = cleis_crypto::SymmetricKey::from_bytes([7; 32]);
            let sealed = keychain_seal(&key, b"device secret").unwrap();
            assert!(sealed.starts_with(KEYCHAIN_MAGIC));
            assert!(!sealed.windows(13).any(|w| w == b"device secret"));
            assert_eq!(
                keychain_open(&key, &sealed).unwrap().unwrap(),
                b"device secret"
            );
            // Another Mac's key (or another account's) opens nothing.
            let other = cleis_crypto::SymmetricKey::from_bytes([8; 32]);
            assert!(keychain_open(&other, &sealed).unwrap().is_err());
            // Altered: refused.
            let mut bad = sealed.clone();
            *bad.last_mut().unwrap() ^= 1;
            assert!(keychain_open(&key, &bad).unwrap().is_err());
            // Written unsealed by an earlier version: left to the caller.
            assert!(keychain_open(&key, b"raw secret").is_none());
        }
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
    /// Gives the keyboard to the window's visible password field: `Some(true)` done,
    /// `Some(false)` the window shows none, `None` the system cannot search it.
    fn focus_password_field(&mut self) -> Option<bool> {
        None
    }
    /// Another press asked this fill to stop waiting ([`interrupt`]).
    fn interrupted(&self) -> bool {
        INTERRUPT.load(std::sync::atomic::Ordering::SeqCst)
    }
}

static INTERRUPT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Asks the fill in progress to stop waiting (for the password page): its remaining steps are
/// returned as [`Outcome::Waiting`]. Another press of "Fill best match" does this, so two fills
/// never type at the same time.
pub fn interrupt() {
    INTERRUPT.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// The focus after a move (Tab): read again until it is a password field or the system cannot
/// tell, for up to [`FOCUS_SETTLE_MS`](cleis_core::autofill::FOCUS_SETTLE_MS).
fn settled_focus<D: Driver>(driver: &mut D) -> Result<Focus, FillError> {
    const STEP: u64 = 50;
    let mut focus = driver.focus();
    for _ in 0..(cleis_core::autofill::FOCUS_SETTLE_MS / STEP) {
        if matches!(focus, Focus::Password | Focus::Unknown) {
            break;
        }
        driver.pause(Duration::from_millis(STEP));
        if !driver.still_target()? {
            return Err(FillError::WindowChanged);
        }
        focus = driver.focus();
    }
    Ok(focus)
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
            Action::ExpectPassword => match settled_focus(driver)? {
                Focus::Password | Focus::Unknown => {}
                other => {
                    let rest = actions[i + 1..].to_vec();
                    match driver.focus_password_field() {
                        // A classic form whose password field is not next in tab order.
                        Some(true) => {
                            log(&format!(
                                "run: found {other:?} after Tab, password field focused"
                            ));
                            driver.pause(Duration::from_millis(60));
                        }
                        // No password field on this page: a two-page sign-in.
                        Some(false) => {
                            log(&format!(
                                "run: two-page sign-in, {other:?} after the username"
                            ));
                            // Back to the username field: Enter there sends the username page,
                            // never the link or button that Tab reached ("Forgot email?").
                            driver.press(Key::BackTab)?;
                            driver.pause(Duration::from_millis(60));
                            if !matches!(driver.focus(), Focus::Text | Focus::Unknown) {
                                log("run: the username field did not take the focus back");
                                return Ok(Outcome::Waiting(rest));
                            }
                            driver.press(Key::Enter)?;
                            if !wait_password(driver)? {
                                log("run: no password field came, the rest waits");
                                return Ok(Outcome::Waiting(rest));
                            }
                            log("run: password page, going on");
                            // The rest runs on the password page (it may stop again: 2FA in a
                            // third).
                            return run(driver, &rest, values, method, speed);
                        }
                        // Nothing sent on a guess: the user puts the keyboard in the password
                        // field and presses the shortcut again.
                        None => {
                            log(&format!("run: {other:?} after Tab, page not searchable"));
                            return Ok(Outcome::Waiting(rest));
                        }
                    }
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

/// Waits for the password page of a two-page sign-in: the keyboard in a password field of the
/// same window. `false` after [`TWO_PAGE_WAIT_MS`]; an error when another window comes in front.
fn wait_password<D: Driver>(driver: &mut D) -> Result<bool, FillError> {
    const STEP: u64 = 200;
    for n in 0..(cleis_core::autofill::TWO_PAGE_WAIT_MS / STEP) {
        driver.pause(Duration::from_millis(STEP));
        if driver.interrupted() {
            log("run: interrupted while waiting for the password page");
            return Ok(false);
        }
        if !driver.still_target()? {
            return Err(FillError::WindowChanged);
        }
        // The page put the keyboard there itself, or (searched every 600 ms) shows a password
        // field without focusing it.
        if driver.focus() == Focus::Password
            || (n % 3 == 2 && driver.focus_password_field() == Some(true))
        {
            // Let the page finish showing it before typing.
            driver.pause(Duration::from_millis(150));
            return Ok(true);
        }
    }
    Ok(false)
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
    INTERRUPT.store(false, std::sync::atomic::Ordering::SeqCst);
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
        /// Enter on a text field brings this page (its fields and the focused one) after the
        /// given number of pauses; `None` after Enter: nothing happens.
        next_page: Option<(usize, Vec<Focus>, usize)>,
        /// Counting down to the next page while it loads.
        loading: Option<usize>,
        /// Whether the page can be searched for its password field (UI Automation).
        search: bool,
        /// Pauses a Tab takes to move the focus (browsers move it a moment later).
        lag: usize,
        /// A Tab on its way: pauses left before the focus moves.
        moving: Option<usize>,
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
                Key::Tab if self.lag > 0 => self.moving = Some(self.lag),
                Key::Tab => self.at += 1,
                Key::BackTab => self.at = self.at.saturating_sub(1),
                Key::Enter => {
                    if matches!(self.focus(), Focus::Text | Focus::EmptyText)
                        && let Some((after, _, _)) = &self.next_page
                    {
                        self.loading = Some(*after);
                    }
                }
            }
            Ok(())
        }
        fn paste(&mut self, v: &str) -> Result<(), FillError> {
            self.log.push(format!("paste:{v}"));
            Ok(())
        }
        /// Tests run in parallel: the process-wide flag is not theirs.
        fn interrupted(&self) -> bool {
            false
        }
        fn focus_password_field(&mut self) -> Option<bool> {
            if !self.search {
                return None;
            }
            let Some(at) = self.fields.iter().position(|f| *f == Focus::Password) else {
                return Some(false);
            };
            if at != self.at {
                self.log.push("focus:password".into());
                self.at = at;
            }
            Some(true)
        }
        fn pause(&mut self, _: Duration) {
            match self.moving {
                Some(1) => {
                    self.at += 1;
                    self.moving = None;
                }
                Some(n) => self.moving = Some(n - 1),
                None => {}
            }
            match self.loading {
                Some(0) => {
                    if let Some((_, fields, at)) = self.next_page.take() {
                        self.log.push("page".into());
                        self.fields = fields;
                        self.at = at;
                    }
                    self.loading = None;
                }
                Some(n) => self.loading = Some(n - 1),
                None => {}
            }
        }
    }

    fn form(fields: Vec<Focus>, at: usize) -> Form {
        Form {
            log: vec![],
            fields,
            at,
            front: true,
            accept_set: false,
            next_page: None,
            loading: None,
            search: true,
            lag: 0,
            moving: None,
        }
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
            next_page: None,
            loading: None,
            search: true,
            lag: 0,
            moving: None,
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
            next_page: None,
            loading: None,
            search: true,
            lag: 0,
            moving: None,
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
            next_page: None,
            loading: None,
            search: true,
            lag: 0,
            moving: None,
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
            next_page: None,
            loading: None,
            search: true,
            lag: 0,
            moving: None,
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

    /// Two-page sign-in (0.2.4): the email page, then the password page. Tab reaches a link
    /// ("Forgot email?"): back to the email field, Enter there, the password page comes and its
    /// password field gets the keys; the password is filled and sent, the 2FA code waits.
    #[test]
    fn two_page_sign_in_goes_on() {
        let mut f = form(vec![Focus::EmptyText, Focus::Other], 0);
        f.next_page = Some((4, vec![Focus::Password, Focus::Other], 0));
        let p = plan(true, true, f.focus(), O);
        let r = run(
            &mut f,
            &p.now,
            &values(),
            FillMethod::Automatic,
            TypingSpeed::Fast,
        )
        .unwrap();
        assert_eq!(r, Outcome::Done);
        assert_eq!(
            f.log,
            [
                "type:ines@example-studio.com",
                "Tab",
                "BackTab",
                "Enter",
                "page",
                "type:Bk-2026!",
                "Enter"
            ]
        );
        assert_eq!(
            p.later,
            vec![Action::Fill(Field::Totp), Action::Press(Key::Enter)]
        );
    }

    /// No password page in time (an unknown account, a captcha): nothing more is typed; the
    /// password waits for the next shortcut on this window.
    #[test]
    fn two_page_sign_in_without_password_page_waits() {
        let mut f = form(vec![Focus::EmptyText, Focus::Other], 0);
        let p = plan(true, false, f.focus(), O);
        let r = run(
            &mut f,
            &p.now,
            &values(),
            FillMethod::Automatic,
            TypingSpeed::Fast,
        )
        .unwrap();
        assert_eq!(
            f.log,
            ["type:ines@example-studio.com", "Tab", "BackTab", "Enter"]
        );
        let Outcome::Waiting(rest) = r else { panic!() };
        assert_eq!(
            rest,
            vec![Action::Fill(Field::Password), Action::Press(Key::Enter)]
        );
    }

    /// Another window comes in front while the password page loads: stop, type nothing.
    #[test]
    fn two_page_sign_in_stops_when_the_window_changes() {
        struct Leaves(Form);
        impl Driver for Leaves {
            fn still_target(&mut self) -> Result<bool, FillError> {
                Ok(!self.0.log.iter().any(|l| l == "Enter"))
            }
            fn focus(&mut self) -> Focus {
                self.0.focus()
            }
            fn set_value(&mut self, v: &str) -> bool {
                self.0.set_value(v)
            }
            fn type_text(&mut self, v: &str, s: TypingSpeed) -> Result<(), FillError> {
                self.0.type_text(v, s)
            }
            fn press(&mut self, k: Key) -> Result<(), FillError> {
                self.0.press(k)
            }
            fn paste(&mut self, v: &str) -> Result<(), FillError> {
                self.0.paste(v)
            }
            fn pause(&mut self, d: Duration) {
                self.0.pause(d)
            }
            fn focus_password_field(&mut self) -> Option<bool> {
                self.0.focus_password_field()
            }
        }
        let mut f = form(vec![Focus::EmptyText, Focus::Other], 0);
        f.next_page = Some((2, vec![Focus::Password], 0));
        let mut d = Leaves(f);
        let p = plan(true, false, Focus::EmptyText, O);
        let r = run(
            &mut d,
            &p.now,
            &values(),
            FillMethod::Type,
            TypingSpeed::Fast,
        );
        assert_eq!(r, Err(FillError::WindowChanged));
        assert!(!d.0.log.iter().any(|l| l.contains("Bk-2026!")));
    }

    #[test]
    fn stops_when_another_window_comes_in_front() {
        let mut f = Form {
            log: vec![],
            fields: vec![],
            at: 0,
            front: false,
            accept_set: false,
            next_page: None,
            loading: None,
            search: true,
            lag: 0,
            moving: None,
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

    /// 0.2.4 bug: the browser moves the focus a moment after Tab; it was read too early (still
    /// the email field) and the form was sent with the email only. Now the focus settles first.
    #[test]
    fn classic_form_with_a_slow_focus() {
        let mut f = form(vec![Focus::EmptyText, Focus::Password], 0);
        f.lag = 4;
        let p = plan(true, false, f.focus(), O);
        let r = run(
            &mut f,
            &p.now,
            &values(),
            FillMethod::Type,
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

    /// A link between the two fields ("Forgot your email?"): the page has a password field, so
    /// it is not a two-page sign-in; the password field gets the keyboard and nothing is sent
    /// early.
    #[test]
    fn classic_form_with_a_link_between_the_fields() {
        let mut f = form(vec![Focus::EmptyText, Focus::Other, Focus::Password], 0);
        let p = plan(true, false, f.focus(), O);
        let r = run(
            &mut f,
            &p.now,
            &values(),
            FillMethod::Type,
            TypingSpeed::Fast,
        );
        assert_eq!(r, Ok(Outcome::Done));
        assert_eq!(
            f.log,
            [
                "type:ines@example-studio.com",
                "Tab",
                "focus:password",
                "type:Bk-2026!",
                "Enter"
            ]
        );
    }

    /// The page cannot be searched and the focus is not in a password field: nothing is sent
    /// on a guess, the password waits for the next press.
    #[test]
    fn unsearchable_page_waits_without_sending() {
        let mut f = form(vec![Focus::EmptyText, Focus::Text], 0);
        f.search = false;
        let p = plan(true, false, f.focus(), O);
        let r = run(
            &mut f,
            &p.now,
            &values(),
            FillMethod::Type,
            TypingSpeed::Fast,
        )
        .unwrap();
        assert_eq!(f.log, ["type:ines@example-studio.com", "Tab"]);
        assert_eq!(
            r,
            Outcome::Waiting(vec![
                Action::Fill(Field::Password),
                Action::Press(Key::Enter)
            ])
        );
    }

    /// The password page shows its field without focusing it: found by the search, filled.
    #[test]
    fn two_page_sign_in_finds_an_unfocused_password_field() {
        let mut f = form(vec![Focus::EmptyText, Focus::Other], 0);
        f.next_page = Some((2, vec![Focus::Other, Focus::Password], 0));
        let p = plan(true, false, f.focus(), O);
        let r = run(
            &mut f,
            &p.now,
            &values(),
            FillMethod::Type,
            TypingSpeed::Fast,
        )
        .unwrap();
        assert_eq!(r, Outcome::Done);
        assert_eq!(
            f.log,
            [
                "type:ines@example-studio.com",
                "Tab",
                "BackTab",
                "Enter",
                "page",
                "focus:password",
                "type:Bk-2026!",
                "Enter"
            ]
        );
    }

    /// A second press while the first fill waits for the password page: the first stops
    /// waiting (its password kept for the next press) instead of typing alongside the second.
    #[test]
    fn a_second_press_interrupts_the_wait() {
        struct Pressed(Form, bool);
        impl Driver for Pressed {
            fn still_target(&mut self) -> Result<bool, FillError> {
                self.0.still_target()
            }
            fn focus(&mut self) -> Focus {
                self.0.focus()
            }
            fn set_value(&mut self, v: &str) -> bool {
                self.0.set_value(v)
            }
            fn type_text(&mut self, v: &str, s: TypingSpeed) -> Result<(), FillError> {
                self.0.type_text(v, s)
            }
            fn press(&mut self, k: Key) -> Result<(), FillError> {
                self.0.press(k)
            }
            fn paste(&mut self, v: &str) -> Result<(), FillError> {
                self.0.paste(v)
            }
            fn pause(&mut self, d: Duration) {
                if self.0.log.last().is_some_and(|l| l == "Enter") {
                    self.1 = true;
                }
                self.0.pause(d)
            }
            fn focus_password_field(&mut self) -> Option<bool> {
                self.0.focus_password_field()
            }
            fn interrupted(&self) -> bool {
                self.1
            }
        }
        let mut d = Pressed(form(vec![Focus::EmptyText, Focus::Other], 0), false);
        let p = plan(true, false, Focus::EmptyText, O);
        let r = run(
            &mut d,
            &p.now,
            &values(),
            FillMethod::Type,
            TypingSpeed::Fast,
        )
        .unwrap();
        assert!(matches!(r, Outcome::Waiting(_)));
        assert!(!d.0.log.iter().any(|l| l.contains("Bk-2026!")));
    }

    #[test]
    fn paste_method() {
        let mut f = Form {
            log: vec![],
            fields: vec![],
            at: 0,
            front: true,
            accept_set: true,
            next_page: None,
            loading: None,
            search: true,
            lag: 0,
            moving: None,
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
