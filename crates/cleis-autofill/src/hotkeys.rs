//! System-wide shortcuts (Quick Access, Fill best match) through tauri-apps `global-hotkey`.
//!
//! The manager must be created on the thread that runs the event loop (Windows) or on the main
//! thread (macOS): the app creates it on its interface thread. Events arrive on another thread
//! through [`Hotkeys::listen`].

use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::sync::{Arc, Mutex, Once};

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

    /// Calls `f` (on a background thread) each time a registered shortcut is pressed. Called
    /// again after [`Self::set`]: the new shortcuts and `f` replace the previous ones.
    pub fn listen(&self, f: impl Fn(Shortcut) + Send + Sync + 'static) {
        let map = self
            .registered
            .iter()
            .map(|(hk, s)| (hk.id(), *s))
            .collect();
        route(map, f);
    }
}

/// What a pressed shortcut's id leads to, and whom to tell.
type Route = (Vec<(u32, Shortcut)>, Arc<dyn Fn(Shortcut) + Send + Sync>);

/// `global-hotkey` keeps the first event handler it is given for the life of the process (a
/// `OnceCell`): a handler set again after changing a shortcut was ignored, and the first one
/// knew only the old ids and a closed channel, so the shortcut did nothing until Cleis was
/// started again (0.2.4 fix). One handler is installed, and it reads the current route here.
static ROUTE: Mutex<Option<Route>> = Mutex::new(None);
static INSTALLED: Once = Once::new();

fn route(map: Vec<(u32, Shortcut)>, f: impl Fn(Shortcut) + Send + Sync + 'static) {
    if let Ok(mut r) = ROUTE.lock() {
        *r = Some((map, Arc::new(f)));
    }
    INSTALLED.call_once(|| {
        GlobalHotKeyEvent::set_event_handler(Some(|e: GlobalHotKeyEvent| {
            if e.state() == HotKeyState::Pressed && dispatch(e.id()) {
                // The shortcut's key never reaches the app in front; without a mask, releasing
                // its Alt looks like a lone Alt press there and opens the app's menu keys.
                crate::platform::mask_menu_key();
            }
        }));
    });
}

/// Hands a pressed shortcut to the current listener; `false` when the id is not one of ours.
fn dispatch(id: u32) -> bool {
    let target = ROUTE.lock().ok().and_then(|r| {
        let (map, f) = r.as_ref()?;
        let (_, s) = map.iter().find(|(i, _)| *i == id)?;
        Some((*s, Arc::clone(f)))
    });
    match target {
        Some((s, f)) => {
            f(s);
            true
        }
        None => false,
    }
}

impl Drop for Hotkeys {
    fn drop(&mut self) {
        if let Ok(mut r) = ROUTE.lock() {
            *r = None;
        }
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

    /// A changed shortcut reaches the new listener (0.2.4: it needed a restart).
    #[test]
    fn a_changed_shortcut_reaches_the_new_listener() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let old = parse("Ctrl+Shift+Space").unwrap().id();
        let new = parse("Ctrl+Alt+Shift+K").unwrap().id();
        static FIRST: AtomicUsize = AtomicUsize::new(0);
        static SECOND: AtomicUsize = AtomicUsize::new(0);
        route(vec![(old, Shortcut::QuickAccess)], |_| {
            FIRST.fetch_add(1, Ordering::SeqCst);
        });
        assert!(dispatch(old));
        route(vec![(new, Shortcut::QuickAccess)], |s| {
            assert_eq!(s, Shortcut::QuickAccess);
            SECOND.fetch_add(1, Ordering::SeqCst);
        });
        assert!(!dispatch(old), "the old keys no longer do anything");
        assert!(dispatch(new));
        assert_eq!(FIRST.load(Ordering::SeqCst), 1);
        assert_eq!(SECOND.load(Ordering::SeqCst), 1);
    }
}
