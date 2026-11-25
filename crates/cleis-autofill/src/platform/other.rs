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
