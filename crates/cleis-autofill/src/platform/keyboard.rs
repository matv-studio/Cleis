//! Keyboard and clipboard through `enigo` and `arboard`, shared by every system.

use crate::{FillError, char_delay};
use cleis_core::autofill::Key;
use cleis_core::settings::TypingSpeed;
use enigo::{Direction, Enigo, Keyboard as _, Settings};
use std::time::Duration;

pub struct Keyboard {
    enigo: Enigo,
}

fn input(e: impl std::fmt::Display) -> FillError {
    FillError::Input(e.to_string())
}

impl Keyboard {
    pub fn new() -> Result<Self, FillError> {
        let settings = Settings {
            // Never pop the macOS permission prompt in the middle of a fill: Settings › Autofill
            // asks for it explicitly.
            open_prompt_to_get_permissions: false,
            ..Settings::default()
        };
        Ok(Self {
            enigo: Enigo::new(&settings).map_err(input)?,
        })
    }

    pub fn type_text(&mut self, value: &str, speed: TypingSpeed) -> Result<(), FillError> {
        match char_delay(speed) {
            None => self.enigo.text(value).map_err(input),
            Some(d) => {
                let mut buf = [0u8; 4];
                for c in value.chars() {
                    self.enigo.text(c.encode_utf8(&mut buf)).map_err(input)?;
                    std::thread::sleep(d);
                }
                Ok(())
            }
        }
    }

    pub fn press(&mut self, key: Key) -> Result<(), FillError> {
        use enigo::Key as K;
        match key {
            Key::Tab => self.enigo.key(K::Tab, Direction::Click),
            Key::Enter => self.enigo.key(K::Return, Direction::Click),
            Key::BackTab => {
                self.enigo.key(K::Shift, Direction::Press).map_err(input)?;
                let r = self.enigo.key(K::Tab, Direction::Click);
                self.enigo
                    .key(K::Shift, Direction::Release)
                    .map_err(input)?;
                r
            }
        }
        .map_err(input)
    }

    /// Puts `value` on the clipboard, pastes it, then empties the clipboard.
    pub fn paste(&mut self, value: &str) -> Result<(), FillError> {
        use enigo::Key as K;
        let clip = |e: arboard::Error| FillError::Clipboard(e.to_string());
        let mut cb = arboard::Clipboard::new().map_err(clip)?;
        cb.set_text(value.to_owned()).map_err(clip)?;
        let modifier = if cfg!(target_os = "macos") {
            K::Meta
        } else {
            K::Control
        };
        self.enigo.key(modifier, Direction::Press).map_err(input)?;
        let r = self.enigo.key(K::Unicode('v'), Direction::Click);
        self.enigo
            .key(modifier, Direction::Release)
            .map_err(input)?;
        r.map_err(input)?;
        // The target reads the clipboard asynchronously: give it time before clearing.
        std::thread::sleep(Duration::from_millis(400));
        cb.clear().map_err(clip)
    }
}
