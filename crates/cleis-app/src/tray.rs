//! Notification area icon (Windows) / menu bar icon (macOS): Cleis keeps running with its
//! shortcuts when the main window is closed (D-55). Menu: Open Cleis, Lock, Quit Cleis; a click
//! on the icon opens the window (Windows).

use crate::app::CleisApp;
use gpui_kit::{AppContext, Context, Window};

/// Keeps the icon alive (removed from the notification area when dropped).
pub struct Tray {
    #[cfg(any(windows, target_os = "macos"))]
    _icon: tray_icon::TrayIcon,
    _task: gpui_kit::Task<()>,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(any(windows, target_os = "macos")), allow(dead_code))]
enum TrayAction {
    Open,
    Lock,
    Quit,
}

impl CleisApp {
    /// Adds the icon. Not on Linux (development only) nor in screenshots and tests.
    pub fn install_tray(&mut self, cx: &mut Context<Self>) {
        #[cfg(any(windows, target_os = "macos"))]
        {
            use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
            use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

            let open = MenuItem::new("Open Cleis", true, None);
            let lock = MenuItem::new("Lock", true, None);
            let quit = MenuItem::new("Quit Cleis", true, None);
            let menu = Menu::new();
            let sep = PredefinedMenuItem::separator();
            let items: [&dyn tray_icon::menu::IsMenuItem; 4] = [&open, &lock, &sep, &quit];
            if menu.append_items(&items).is_err() {
                return;
            }
            let Ok(icon) =
                Icon::from_rgba(include_bytes!("../assets/tray-32.rgba").to_vec(), 32, 32)
            else {
                return;
            };
            let built = TrayIconBuilder::new()
                .with_tooltip("Cleis")
                .with_icon(icon)
                .with_menu(Box::new(menu))
                // Windows: a click opens the window, the menu comes with a right click.
                .with_menu_on_left_click(cfg!(target_os = "macos"))
                .build();
            let Ok(icon) = built else { return };
            let (tx, rx) = async_channel::unbounded::<TrayAction>();
            let ids = [
                (open.id().clone(), TrayAction::Open),
                (lock.id().clone(), TrayAction::Lock),
                (quit.id().clone(), TrayAction::Quit),
            ];
            let menu_tx = tx.clone();
            MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
                if let Some((_, a)) = ids.iter().find(|(id, _)| *id == e.id) {
                    let _ = menu_tx.try_send(*a);
                }
            }));
            TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = e
                {
                    let _ = tx.try_send(TrayAction::Open);
                }
            }));
            let Some(main) = self.main_window else { return };
            let task = cx.spawn(async move |this, cx| {
                while let Ok(a) = rx.recv().await {
                    let done = cx.update_window(main, |_, window, cx| {
                        this.update(cx, |this, cx| this.tray_action(a, window, cx))
                    });
                    if !matches!(done, Ok(Ok(()))) {
                        break;
                    }
                }
            });
            self.tray = Some(Tray {
                _icon: icon,
                _task: task,
            });
        }
        #[cfg(not(any(windows, target_os = "macos")))]
        let _ = cx;
    }

    #[cfg_attr(not(any(windows, target_os = "macos")), allow(dead_code))]
    fn tray_action(&mut self, a: TrayAction, window: &mut Window, cx: &mut Context<Self>) {
        match a {
            TrayAction::Open => self.show_main(cx),
            TrayAction::Lock => self.lock(window, cx),
            TrayAction::Quit => cx.quit(),
        }
    }

    /// Writes or removes the login item to match *Preferences › Open at login*. Skipped for the
    /// demo vault, screenshots and tests, so trial builds never register themselves.
    pub fn sync_open_at_login(&mut self) {
        if self.demo || self.no_shortcuts {
            return;
        }
        let Some(on) = self.session().map(|s| s.prefs.open_at_login) else {
            return;
        };
        let Ok(exe) = std::env::current_exe() else {
            return;
        };
        if let Err(e) = cleis_autofill::set_open_at_login(on, &exe, &["--background"])
            && let Some(s) = self.session()
        {
            s.toast = Some(crate::trf!("Open at login not changed: {}", e).into());
        }
    }

    /// D-75: the other launches of Cleis bring this one in front; the installer makes it quit.
    pub fn serve_instance(&mut self, g: crate::instance::Guard, cx: &mut Context<Self>) {
        use crate::instance::Request;
        let (tx, rx) = async_channel::unbounded();
        g.serve(move |r| {
            let _ = tx.try_send(r);
        });
        cx.spawn(async move |this, cx| {
            while let Ok(r) = rx.recv().await {
                let done = this.update(cx, |app, cx| match r {
                    Request::Show => app.show_main(cx),
                    Request::Quit => cx.quit(),
                });
                if done.is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    /// Main window back in front (tray, shortcut while locked).
    pub fn show_main(&mut self, cx: &mut Context<Self>) {
        match self.main_hwnd {
            // Shown and brought in front by Cleis (no simulated Alt, see quick_access.rs).
            Some(h) => cx
                .background_spawn(async move {
                    cleis_autofill::set_window_visible(cleis_autofill::WindowRef(h), true)
                })
                .detach(),
            None => {
                cx.activate(true);
                if let Some(w) = self.main_window {
                    w.update(cx, |_, window, _| window.activate_window()).ok();
                }
            }
        }
    }

    /// Closing the main window while the tray icon exists hides it: shortcuts keep working.
    /// Returns whether the window may really close.
    pub fn on_main_close(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.tray.is_none() {
            return true;
        }
        if let Some(h) = self.main_hwnd {
            cleis_autofill::set_window_visible(cleis_autofill::WindowRef(h), false);
        } else {
            cx.hide();
        }
        let _ = window;
        false
    }
}

/// Windows: the window's `HWND`, for Cleis's own show / hide / bring-in-front calls.
#[cfg(windows)]
pub fn hwnd_of(window: &Window) -> Option<u64> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match HasWindowHandle::window_handle(window).map(|h| h.as_raw()) {
        Ok(RawWindowHandle::Win32(h)) => Some(h.hwnd.get() as u64),
        _ => None,
    }
}

#[cfg(not(windows))]
pub fn hwnd_of(_: &Window) -> Option<u64> {
    None
}
