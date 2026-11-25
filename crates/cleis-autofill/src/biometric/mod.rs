//! Biometric unlock (D-66): the system checks it is the user (Windows Hello, Touch ID), then
//! gives back a key kept for this user only (DPAPI on Windows, the login keychain on macOS).
//! Linux has none: `available` is false.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use self::macos as sys;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows as sys;

#[cfg(not(any(windows, target_os = "macos")))]
mod other;
#[cfg(not(any(windows, target_os = "macos")))]
use self::other as sys;

/// End-to-end tests only (feature `test-support`): a stand-in that answers `verify` with the
/// given result and keeps secrets as they are.
#[cfg(feature = "test-support")]
pub static FAKE: std::sync::Mutex<Option<bool>> = std::sync::Mutex::new(None);

#[cfg(feature = "test-support")]
fn fake() -> Option<bool> {
    FAKE.lock().ok().and_then(|f| *f)
}

#[cfg(not(feature = "test-support"))]
fn fake() -> Option<bool> {
    None
}

/// "Windows Hello", "Touch ID", or "" where there is none.
pub fn name() -> &'static str {
    if fake().is_some() {
        return "Test";
    }
    sys::NAME
}

/// Whether this computer can check the user now (a reader or a PIN set up).
pub fn available() -> bool {
    fake().is_some() || sys::available()
}

/// Asks the system to check the user; `window`: the app's window, to show the prompt over it.
pub fn verify(reason: &str, window: Option<crate::WindowRef>) -> Result<(), String> {
    match fake() {
        Some(true) => Ok(()),
        Some(false) => Err("Canceled".into()),
        None => sys::verify(reason, window),
    }
}

/// Keeps `secret` for this user: returns what to store in a file beside the vault.
pub fn protect(id: &str, secret: &[u8]) -> Result<Vec<u8>, String> {
    if fake().is_some() {
        return Ok(secret.to_vec());
    }
    sys::protect(id, secret)
}

/// The secret kept by `protect`.
pub fn unprotect(id: &str, stored: &[u8]) -> Result<Vec<u8>, String> {
    if fake().is_some() {
        return Ok(stored.to_vec());
    }
    sys::unprotect(id, stored)
}

/// Removes the secret kept by `protect` (the file is the caller's).
pub fn forget(id: &str) {
    if fake().is_none() {
        sys::forget(id)
    }
}
