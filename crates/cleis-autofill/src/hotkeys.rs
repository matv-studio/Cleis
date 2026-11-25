//! System-wide shortcuts (Quick Access, Fill best match) through tauri-apps `global-hotkey`.
//!
//! The manager must be created on the thread that runs the event loop (Windows) or on the main
//! thread (macOS): the app creates it on its interface thread. Events arrive on another thread
//! through [`Hotkeys::listen`].

use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shortcut {
    QuickAccess,
    FillBestMatch,
}

/// Registration result for one shortcut, shown next to it in Settings › Autofill.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Off,
    Active,
    /// Already taken by another app, or not a valid combination.
    Unavailable(String),
}

pub struct Hotkeys {
    manager: GlobalHotKeyManager,
    registered: Vec<(HotKey, Shortcut)>,
}

/// `Ctrl+Shift+Space` as stored in preferences → a hotkey. On macOS `Ctrl` means ⌘ (the
/// settings say so), `Alt` is ⌥.
pub fn parse(keys: &str) -> Result<HotKey, String> {
    let keys = if cfg!(target_os = "macos") {
        keys.split('+')
            .map(|k| {
                if k.trim().eq_ignore_ascii_case("ctrl") {
                    "Cmd"
                } else {
                    k.trim()
                }
            })
            .collect::<Vec<_>>()
            .join("+")
    } else {
        keys.to_owned()
    };
    keys.parse::<HotKey>().map_err(|e| e.to_string())
}

impl Hotkeys {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            manager: GlobalHotKeyManager::new().map_err(|e| e.to_string())?,
            registered: vec![],
        })
    }

    /// Replaces the registered shortcuts. `None` = shortcut turned off.
    pub fn set(
        &mut self,
        quick_access: Option<&str>,
        fill_best: Option<&str>,
    ) -> [(Shortcut, Status); 2] {
        for (hk, _) in self.registered.drain(..) {
            let _ = self.manager.unregister(hk);
        }
        let mut one = |keys: Option<&str>, which: Shortcut| -> Status {
            let Some(keys) = keys else {
                return Status::Off;
            };
            match parse(keys) {
                Ok(hk) => match self.manager.register(hk) {
                    Ok(()) => {
                        self.registered.push((hk, which));
                        Status::Active
                    }
                    Err(e) => Status::Unavailable(e.to_string()),
                },
                Err(e) => Status::Unavailable(e),
            }
        };
        let qa = one(quick_access, Shortcut::QuickAccess);
        let best = one(fill_best, Shortcut::FillBestMatch);
        [(Shortcut::QuickAccess, qa), (Shortcut::FillBestMatch, best)]
    }

    /// Calls `f` (on a background thread) each time a registered shortcut is pressed.
    pub fn listen(&self, f: impl Fn(Shortcut) + Send + Sync + 'static) {
        let map: Vec<(u32, Shortcut)> = self
            .registered
            .iter()
            .map(|(hk, s)| (hk.id(), *s))
            .collect();
        GlobalHotKeyEvent::set_event_handler(Some(move |e: GlobalHotKeyEvent| {
            if e.state() != HotKeyState::Pressed {
                return;
            }
            if let Some((_, s)) = map.iter().find(|(id, _)| *id == e.id()) {
                f(*s);
            }
        }));
    }
}

impl Drop for Hotkeys {
    fn drop(&mut self) {
        GlobalHotKeyEvent::set_event_handler(None::<fn(GlobalHotKeyEvent)>);
        for (hk, _) in self.registered.drain(..) {
            let _ = self.manager.unregister(hk);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_default_shortcuts() {
        assert!(parse("Ctrl+Shift+Space").is_ok());
        assert!(parse("Ctrl+Alt+A").is_ok());
        assert_ne!(
            parse("Ctrl+Shift+Space").unwrap().id(),
            parse("Ctrl+Alt+A").unwrap().id()
        );
        assert!(parse("Ctrl+Nope").is_err());
    }
}
