//! Linux and other systems: development builds only. Typing works (X11), but the window in
//! front is unknown, so nothing is matched automatically.

use super::keyboard::Keyboard;
use crate::{Driver, FillError, Foreground, WindowRef};
use cleis_core::autofill::{Focus, Key};
use cleis_core::settings::TypingSpeed;
use std::time::Duration;

pub fn foreground() -> Option<Foreground> {
    None
}

pub fn browser_url(_: WindowRef) -> Option<String> {
    None
}

pub fn activate(_: WindowRef) -> Result<(), FillError> {
    Ok(())
}

pub fn accessibility_allowed() -> bool {
    true
}

pub fn request_accessibility() -> bool {
    true
}

pub fn wait_modifiers_released(_: Duration) {}

pub struct SystemDriver {
    kb: Keyboard,
}

impl SystemDriver {
    pub fn new(_: WindowRef) -> Result<Self, FillError> {
        Ok(Self {
            kb: Keyboard::new()?,
        })
    }
}

impl Driver for SystemDriver {
    fn still_target(&mut self) -> Result<bool, FillError> {
        Ok(true)
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

pub fn set_window_visible(_: WindowRef, _: bool) {}

pub fn set_open_at_login(_: bool, _: &std::path::Path, _: &[&str]) -> Result<(), String> {
    Ok(())
}

/// Opening Cleis (D-76): Windows only for now.
pub fn hello_available() -> bool {
    false
}

pub fn hello_sign(_name: &str, _challenge: &[u8]) -> Result<Vec<u8>, String> {
    Err("Windows Hello is not available on this system".into())
}

/// "Nothing": the key is kept as is; the application writes it to a file only its user reads.
pub fn protect(secret: &[u8]) -> Result<Vec<u8>, String> {
    Ok(secret.to_vec())
}

pub fn unprotect(stored: &[u8]) -> Result<Vec<u8>, String> {
    Ok(stored.to_vec())
}
