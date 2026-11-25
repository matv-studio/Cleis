//! Application state: first launch → unlock → main window (vault, contacts, health, settings).

use crate::theme::{self, Mode, Tokens};
use cleis_core::column::ColumnDef;
use cleis_core::settings::Preferences;
use cleis_core::text::matches;
use cleis_core::{Contact, Entry, EntryId};
use cleis_crypto::KdfParams;
use cleis_store::{LockedVault, SpaceInfo, Vault};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::{App, ClipboardItem, Context, Entity, SharedString, Subscription, Task, Window};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Vault,
    Contacts,
    Health,
    Settings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    All,
    Favorites,
    RenewSoon,
    Space(cleis_core::SpaceId),
}

pub struct Welcome {
    pub step: u8,
    pub password: Entity<InputState>,
    pub confirm: Entity<InputState>,
    pub error: Option<SharedString>,
    pub busy: bool,
}

pub struct Unlock {
    pub password: Entity<InputState>,
    pub error: Option<SharedString>,
    pub busy: bool,
}

pub struct Session {
    pub vault: Vault,
    pub entries: Vec<Entry>,
    pub contacts: Vec<Contact>,
    pub spaces: Vec<SpaceInfo>,
    pub columns: Vec<ColumnDef>,
    pub prefs: Preferences,
    pub tab: Tab,
    pub filter: Filter,
    pub selected: Option<EntryId>,
    pub search: Entity<InputState>,
    pub query: String,
    pub reveal: bool,
    pub toast: Option<SharedString>,
    /// Indices into `entries`, filtered and sorted by name.
    pub visible: Vec<usize>,
    pub selected_contact: Option<cleis_core::ContactId>,
    pub settings_page: &'static str,
}

impl Session {
    pub fn refilter(&mut self) {
        let q = self.query.trim().to_owned();
        let mut v: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| match self.filter {
                Filter::All => true,
                Filter::Favorites => e.favorite,
                Filter::RenewSoon => e.subscription == cleis_core::Subscription::RenewSoon,
                Filter::Space(s) => e.space == s,
            })
            .filter(|(_, e)| q.is_empty() || matches(&e.search_text(), &q))
            .map(|(i, _)| i)
            .collect();
        v.sort_by_key(|i| cleis_core::text::fold(&self.entries[*i].name));
        self.visible = v;
        if self
            .selected
            .is_none_or(|s| !self.visible.iter().any(|i| self.entries[*i].id == s))
        {
            self.selected = self.visible.first().map(|i| self.entries[*i].id);
        }
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        self.selected
            .and_then(|id| self.entries.iter().find(|e| e.id == id))
    }
}

pub enum Phase {
    Welcome(Welcome),
    Unlock(Unlock),
    Main(Box<Session>),
}

pub struct CleisApp {
    pub path: PathBuf,
    pub phase: Phase,
    pub mode: Mode,
    _ticker: Task<()>,
    _subs: Vec<Subscription>,
}

fn password_input(
    window: &mut Window,
    cx: &mut Context<CleisApp>,
    placeholder: &str,
) -> Entity<InputState> {
    let ph = placeholder.to_owned();
    cx.new(|cx| InputState::new(window, cx).masked(true).placeholder(ph))
}

impl CleisApp {
    pub fn new(path: PathBuf, mode: Mode, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // One tick per second keeps 2FA codes and their countdown current.
        let ticker = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        });
        let mut app = Self {
            path,
            phase: Phase::Unlock(Unlock {
                password: password_input(window, cx, ""),
                error: None,
                busy: false,
            }),
            mode,
            _ticker: ticker,
            _subs: vec![],
        };
        if app.path.exists() {
            app.show_unlock(window, cx);
        } else {
            app.show_welcome(window, cx);
        }
        app
    }

    pub fn tokens<'a>(&self, cx: &'a App) -> &'a Tokens {
        theme::tokens(cx)
    }

    pub fn show_welcome(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let password = password_input(window, cx, "");
        let confirm = password_input(window, cx, "");
        self._subs =
            vec![
                cx.subscribe_in(&confirm, window, |this, _, ev: &InputEvent, window, cx| {
                    if let InputEvent::PressEnter { .. } = ev {
                        this.create_vault(window, cx);
                    }
                }),
            ];
        self.phase = Phase::Welcome(Welcome {
            step: 0,
            password,
            confirm,
            error: None,
            busy: false,
        });
        cx.notify();
    }

    pub fn show_unlock(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let password = password_input(window, cx, "");
        self._subs =
            vec![
                cx.subscribe_in(&password, window, |this, _, ev: &InputEvent, window, cx| {
                    if let InputEvent::PressEnter { .. } = ev {
                        this.unlock(window, cx);
                    }
                }),
            ];
        password.update(cx, |s, cx| s.focus(window, cx));
        self.phase = Phase::Unlock(Unlock {
            password,
            error: None,
            busy: false,
        });
        cx.notify();
    }

    pub fn create_vault(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Phase::Welcome(w) = &mut self.phase else {
            return;
        };
        if w.busy {
            return;
        }
        let pw = w.password.read(cx).value().to_string();
        let confirm = w.confirm.read(cx).value().to_string();
        if pw.chars().count() < 10 {
            w.error = Some("At least 10 characters".into());
            cx.notify();
            return;
        }
        if pw != confirm {
            w.error = Some("The passwords differ".into());
            cx.notify();
            return;
        }
        w.busy = true;
        w.error = None;
        cx.notify();
        let path = self.path.clone();
        cx.spawn_in(window, async move |this, cx| {
            let res = cx
                .background_executor()
                .spawn(async move {
                    if let Some(dir) = path.parent() {
                        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
                    }
                    Vault::create(&path, &pw, KdfParams::default()).map_err(|e| e.to_string())
                })
                .await;
            this.update_in(cx, |this, window, cx| match res {
                Ok(v) => this.enter(v, window, cx),
                Err(e) => {
                    if let Phase::Welcome(w) = &mut this.phase {
                        w.busy = false;
                        w.error = Some(e.into());
                    }
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    pub fn unlock(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Phase::Unlock(u) = &mut self.phase else {
            return;
        };
        if u.busy {
            return;
        }
        let pw = u.password.read(cx).value().to_string();
        if pw.is_empty() {
            return;
        }
        u.busy = true;
        u.error = None;
        cx.notify();
        let path = self.path.clone();
        cx.spawn_in(window, async move |this, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { LockedVault::open(&path).and_then(|l| l.unlock(&pw)) })
                .await;
            this.update_in(cx, |this, window, cx| match res {
                Ok(v) => this.enter(v, window, cx),
                Err(e) => {
                    if let Phase::Unlock(u) = &mut this.phase {
                        u.busy = false;
                        u.error = Some(match e {
                            cleis_store::StoreError::WrongPassword => {
                                "Wrong master password".into()
                            }
                            other => other.to_string().into(),
                        });
                        u.password.update(cx, |s, cx| {
                            s.set_value("", window, cx);
                            s.focus(window, cx);
                        });
                    }
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    /// Opens the main window on an unlocked vault.
    pub fn enter(&mut self, vault: Vault, window: &mut Window, cx: &mut Context<Self>) {
        let load = || -> cleis_store::Result<_> {
            Ok((
                vault.entries()?,
                vault.contacts()?,
                vault.spaces()?,
                vault.vault_columns()?,
                vault.preferences()?,
            ))
        };
        let (entries, contacts, spaces, columns, prefs) = match load() {
            Ok(x) => x,
            Err(e) => {
                self.phase = Phase::Unlock(Unlock {
                    password: password_input(window, cx, ""),
                    error: Some(e.to_string().into()),
                    busy: false,
                });
                cx.notify();
                return;
            }
        };
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search"));
        self._subs =
            vec![
                cx.subscribe_in(&search, window, |this, input, ev: &InputEvent, _, cx| {
                    if let (InputEvent::Change, Phase::Main(s)) = (ev, &mut this.phase) {
                        s.query = input.read(cx).value().to_string();
                        s.refilter();
                        cx.notify();
                    }
                }),
            ];
        let mode = match prefs.theme {
            cleis_core::settings::Theme::Light => Mode::Light,
            _ => self.mode,
        };
        if mode != self.mode {
            self.set_mode(mode, cx);
        }
        let mut s = Session {
            vault,
            entries,
            contacts,
            spaces,
            columns,
            prefs,
            tab: Tab::Vault,
            filter: Filter::All,
            selected: None,
            search,
            query: String::new(),
            reveal: false,
            toast: None,
            visible: vec![],
            selected_contact: None,
            settings_page: "Preferences",
        };
        s.refilter();
        self.phase = Phase::Main(Box::new(s));
        cx.notify();
    }

    pub fn lock(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Phase::Main(_) = self.phase {
            // Dropping the session drops the vault and wipes its keys.
            self.show_unlock(window, cx);
        }
    }

    pub fn set_mode(&mut self, mode: Mode, cx: &mut Context<Self>) {
        self.mode = mode;
        theme::apply(mode, cx);
        cx.notify();
    }

    /// Copies a secret and clears the clipboard after the preference delay, if unchanged.
    pub fn copy(&mut self, text: String, what: &str, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
        let delay = if let Phase::Main(s) = &mut self.phase {
            s.toast = Some(format!("{what} copied").into());
            s.prefs.clipboard_clear_secs
        } else {
            None
        };
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(1800))
                .await;
            this.update(cx, |this, cx| {
                if let Phase::Main(s) = &mut this.phase {
                    s.toast = None;
                }
                cx.notify();
            })
            .ok();
            if let Some(secs) = delay {
                cx.background_executor()
                    .timer(Duration::from_secs(u64::from(secs).saturating_sub(2)))
                    .await;
                cx.update(|cx| {
                    if cx.read_from_clipboard().and_then(|c| c.text()).as_deref()
                        == Some(text.as_str())
                    {
                        cx.write_to_clipboard(ClipboardItem::new_string(String::new()));
                    }
                });
            }
        })
        .detach();
    }

    pub fn session(&mut self) -> Option<&mut Session> {
        match &mut self.phase {
            Phase::Main(s) => Some(s),
            _ => None,
        }
    }
}

impl Render for CleisApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match &self.phase {
            Phase::Welcome(_) => {
                crate::screens::welcome::render(self, window, cx).into_any_element()
            }
            Phase::Unlock(_) => crate::screens::unlock::render(self, window, cx).into_any_element(),
            Phase::Main(_) => crate::screens::main::render(self, window, cx).into_any_element(),
        }
    }
}
