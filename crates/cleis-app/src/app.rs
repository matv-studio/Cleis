//! Application state: first launch → unlock → main window (vault, contacts, health, settings).

use crate::theme::{self, Appearance, Mode, Tokens};
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

/// Dialogs of the personal folders (D-62).
pub enum FolderDialog {
    New(Entity<InputState>),
    Rename(cleis_core::SpaceId, Entity<InputState>),
    Delete(cleis_core::SpaceId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Vault,
    Contacts,
    Health,
    /// Organization activity log (screen 11): members who may audit.
    Activity,
    Settings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    All,
    Favorites,
    RenewSoon,
    Space(cleis_core::SpaceId),
    Trash,
}

/// A question asked before an action that cannot be undone.
pub struct Confirm {
    pub title: SharedString,
    pub body: SharedString,
    pub button: SharedString,
    pub action: ConfirmAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmAction {
    PurgeEntry(EntryId),
    EmptyTrash,
    DeleteContact(cleis_core::ContactId),
}

pub struct Welcome {
    pub step: u8,
    pub password: Entity<InputState>,
    pub confirm: Entity<InputState>,
    pub error: Option<SharedString>,
    pub busy: bool,
    /// "Restore my vault" (screen 22) instead of creating one.
    pub restore: Option<crate::screens::welcome::Restore>,
    /// "Add an organization server" chosen (screen 03): joined once the vault exists.
    pub org: bool,
    pub join: crate::screens::org::JoinForm,
}

pub struct Unlock {
    pub password: Entity<InputState>,
    pub error: Option<SharedString>,
    pub busy: bool,
}

pub struct Session {
    pub vault: Vault,
    pub entries: Vec<Entry>,
    /// Items in the trash (restorable until emptied).
    pub trash: Vec<Entry>,
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
    /// Shortcut being recorded in Settings › Autofill (`"qa"` or `"best"`).
    pub recording: Option<&'static str>,
    /// Takes the keyboard while a shortcut is recorded.
    pub keys_focus: gpui_kit::FocusHandle,
    /// Main window shortcuts (Ctrl+L lock, Ctrl+F search) when no field has the focus.
    pub main_focus: gpui_kit::FocusHandle,
    /// Boxes of the selected item (inspector).
    pub editor: Option<crate::editor::EntryEditor>,
    /// Inspector on its History tab.
    pub history_tab: bool,
    /// Question shown over the window.
    pub confirm: Option<Confirm>,
    /// Columns menu of the vault (screen 02).
    pub columns_menu: Option<crate::screens::columns::ColumnsMenu>,
    /// Settings › Import & export, kept while moving between pages.
    pub import: crate::screens::import::ImportState,
    /// Contacts tab: filter, search, inspector boxes, menus (screens 20, 21).
    pub contacts_ui: crate::screens::contacts::ContactsUi,
    /// Settings › Security dialogs and menus.
    pub security: crate::screens::security::SecurityUi,
    /// Verification code still to type before the vault is shown (screen 05).
    pub gate: Option<crate::screens::security::Gate>,
    /// Last mouse or key input, for locking after inactivity.
    pub last_activity: std::time::Instant,
    /// This device's sync place and backups (Settings › Sync & backups).
    pub sync: crate::sync::SyncConfig,
    pub sync_menu: bool,
    /// Health tab: space shown and issue filter.
    pub health: crate::screens::health::HealthUi,
    /// Open select of Settings › Preferences.
    pub pref_menu: Option<&'static str>,
    /// Vault toolbar: Filter menu (screen 01b) and Cards view.
    pub vault_filter: VaultFilter,
    pub filter_menu: bool,
    pub vault_cards: bool,
    /// The organization this vault belongs to, if any (D-59).
    pub org: Option<crate::org::OrgSession>,
    /// Personal folders (D-62): the dialog open, the folder whose menu is open.
    pub folder: Option<FolderDialog>,
    pub folder_menu: Option<cleis_core::SpaceId>,
    /// Settings › Organization: joining one from an open vault.
    pub join: crate::screens::org::JoinForm,
    pub join_subs: Vec<Subscription>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VaultFilter {
    pub subs: std::collections::BTreeSet<cleis_core::Subscription>,
    /// `Some(true)`: with a 2FA code, `Some(false)`: without.
    pub two_fa: Option<bool>,
}

impl VaultFilter {
    pub fn count(&self) -> usize {
        self.subs.len() + usize::from(self.two_fa.is_some())
    }
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
                Filter::Trash => false,
            })
            .filter(|(_, e)| q.is_empty() || matches(&e.search_text(), &q))
            .filter(|(_, e)| {
                let f = &self.vault_filter;
                (f.subs.is_empty() || f.subs.contains(&e.subscription))
                    && f.two_fa.is_none_or(|w| e.totp.is_some() == w)
            })
            .map(|(i, _)| i)
            .collect();
        v.sort_by_key(|i| cleis_core::text::fold(&self.entries[*i].name));
        self.visible = v;
        // The trash view has its own list: the vault selection waits for the return.
        if self.filter == Filter::Trash {
            return;
        }
        if self
            .selected
            .is_none_or(|s| !self.visible.iter().any(|i| self.entries[*i].id == s))
        {
            self.selected = self.visible.first().map(|i| self.entries[*i].id);
        }
    }

    /// Minutes of inactivity before locking: this vault's choice, never longer than the
    /// organization's policy.
    pub fn lock_after(&self) -> Option<u32> {
        let own = self.prefs.lock_after_mins;
        let org = self
            .org
            .as_ref()
            .and_then(|o| o.policy())
            .and_then(|p| p.lock_after_mins);
        match (own, org) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    /// Whether this item's password may be shown (an organization role may only copy and fill).
    pub fn may_see(&self, e: &Entry) -> bool {
        match &self.org {
            Some(o) if o.names.contains_key(&e.space) => o.can(cleis_core::org::Perm::See),
            _ => true,
        }
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        self.selected
            .and_then(|id| self.entries.iter().find(|e| e.id == id))
    }
}

pub enum Phase {
    Welcome(Box<Welcome>),
    Unlock(Unlock),
    Main(Box<Session>),
}

pub struct CleisApp {
    pub path: PathBuf,
    pub phase: Phase,
    /// Theme forced on the command line (screenshots); otherwise preferences decide.
    pub forced_mode: Option<Mode>,
    /// Main window, brought back when a shortcut is pressed while the vault is locked.
    pub main_window: Option<gpui_kit::AnyWindowHandle>,
    pub autofill: crate::autofill::AutofillState,
    /// Screenshots and tests: never take the system shortcuts.
    pub no_shortcuts: bool,
    /// Notification area icon; while it exists, closing the window hides it (D-55).
    pub tray: Option<crate::tray::Tray>,
    /// Windows: handle of the main window, hidden and shown from the tray.
    pub main_hwnd: Option<u64>,
    /// Demo vault (`--demo`, `--try-autofill`): nothing is registered on the computer.
    pub demo: bool,
    _ticker: Task<()>,
    pub(crate) _subs: Vec<Subscription>,
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
    pub fn new(
        path: PathBuf,
        forced_mode: Option<Mode>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // One tick per second keeps 2FA codes and their countdown current.
        let ticker = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let Ok(idle) = this.update(cx, |app, cx| {
                    cx.notify();
                    if app.auto_sync_due() {
                        app.sync(true, cx);
                    }
                    if app.auto_backup_due() {
                        app.backup(true, cx);
                    }
                    if app.org_sync_due() {
                        app.org_sync(None, cx);
                    }
                    app.idle_lock_due().then_some(app.main_window).flatten()
                }) else {
                    break;
                };
                // Security › Locking › After inactivity.
                if let Some(w) = idle {
                    let _ = cx.update_window(w, |_, window, cx| {
                        let _ = this.update(cx, |app, cx| app.lock(window, cx));
                    });
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
            forced_mode,
            main_window: Some(window.window_handle()),
            autofill: Default::default(),
            no_shortcuts: false,
            tray: None,
            main_hwnd: crate::tray::hwnd_of(window),
            demo: false,
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
        let (join, subs) = crate::screens::org::join_form(window, cx);
        self._subs =
            vec![
                cx.subscribe_in(&confirm, window, |this, _, ev: &InputEvent, window, cx| {
                    if let InputEvent::PressEnter { .. } = ev {
                        this.create_vault(window, cx);
                    }
                }),
            ];
        self._subs.extend(subs);
        self.phase = Phase::Welcome(Box::new(Welcome {
            step: 0,
            password,
            confirm,
            error: None,
            busy: false,
            restore: None,
            org: false,
            join,
        }));
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
        // Joining: the code must be checked and, without a name in the invitation, one typed.
        let join = if w.org {
            let read = |e: &Option<Entity<InputState>>| {
                e.as_ref()
                    .map(|i| i.read(cx).value().to_string())
                    .unwrap_or_default()
            };
            let j = &w.join;
            let Some(info) = j.info.as_ref() else {
                w.error = Some(if j.checking {
                    "Checking the invitation…".into()
                } else {
                    j.error
                        .clone()
                        .unwrap_or("Enter the server address and invitation code".into())
                });
                cx.notify();
                return;
            };
            let name = read(&j.name);
            if info.name.trim().is_empty() && name.trim().is_empty() {
                w.error = Some("Enter your name".into());
                cx.notify();
                return;
            }
            Some((read(&j.address), read(&j.code), name, read(&j.email)))
        } else {
            None
        };
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
            let res = match (res, join) {
                (Ok(v), Some((address, code, name, email))) => {
                    let public = v.account_public().0;
                    let signing = cleis_crypto::SigningKeyPair::from_secret_bytes(
                        *v.signing_key().secret_bytes(),
                    );
                    let joined = cx
                        .background_executor()
                        .spawn(async move {
                            crate::org::join(&address, &code, &name, &email, public, &signing)
                        })
                        .await;
                    Ok((v, Some(joined)))
                }
                (r, _) => r.map(|v| (v, None)),
            };
            this.update_in(cx, |this, window, cx| match res {
                Ok((v, joined)) => {
                    let failed = match joined {
                        Some(Ok(m)) => {
                            let _ = v.set_org_setting(crate::org::SETTING, &Some(m));
                            None
                        }
                        Some(Err(e)) => Some(e),
                        None => None,
                    };
                    this.enter(v, window, cx);
                    if let Some(e) = failed {
                        this.toast_for(format!("Not joined: {e}"), 8, cx);
                        if let Some(s) = this.session() {
                            s.tab = Tab::Settings;
                            s.settings_page = "Organization";
                        }
                    }
                }
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
                vault.trashed_entries()?,
                vault.contacts()?,
                vault.spaces()?,
                {
                    let mut c = vault.vault_columns()?;
                    c.extend(vault.contact_columns()?);
                    c
                },
                vault.preferences()?,
            ))
        };
        let (entries, trash, contacts, spaces, columns, prefs) = match load() {
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
        self.apply_appearance(&prefs, cx);
        let mut s = Session {
            vault,
            entries,
            trash,
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
            recording: None,
            keys_focus: cx.focus_handle(),
            main_focus: cx.focus_handle(),
            editor: None,
            history_tab: false,
            confirm: None,
            columns_menu: None,
            import: Default::default(),
            contacts_ui: Default::default(),
            security: Default::default(),
            gate: None,
            last_activity: std::time::Instant::now(),
            sync: crate::sync::load_config(&self.path),
            sync_menu: false,
            health: Default::default(),
            pref_menu: None,
            vault_filter: Default::default(),
            filter_menu: false,
            vault_cards: false,
            org: None,
            folder: None,
            folder_menu: None,
            join: Default::default(),
            join_subs: vec![],
        };
        match s.prefs.start_in.as_str() {
            "favorites" => s.filter = Filter::Favorites,
            "contacts" if s.prefs.tabs.contacts => s.tab = Tab::Contacts,
            _ => {}
        }
        s.refilter();
        window.focus(&s.main_focus.clone(), cx);
        self.phase = Phase::Main(Box::new(s));
        self.load_org(cx);
        self.open_gate(window, cx);
        if !cfg!(test) && !self.no_shortcuts {
            self.sync_autofill(cx);
        }
        self.sync_open_at_login();
        cx.notify();
    }

    pub fn lock(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Last changes sent before the keys are wiped.
        if let Phase::Main(s) = &self.phase
            && s.sync.auto
            && s.sync.place.is_some()
            && s.gate.is_none()
        {
            self.sync(true, cx);
        }
        if let Phase::Main(_) = self.phase {
            // Dropping the session drops the vault and wipes its keys.
            self.show_unlock(window, cx);
        }
    }

    /// Applies theme and accent from the preferences (and remembers them for the unlock screen).
    pub fn apply_appearance(&mut self, prefs: &Preferences, cx: &mut Context<Self>) {
        let mut a = Appearance::from_prefs(prefs);
        if let Some(m) = self.forced_mode {
            a.mode = m;
        }
        theme::apply(&a, cx);
        save_appearance(prefs);
        cx.notify();
    }

    /// Short message at the bottom of the main window.
    pub fn toast(&mut self, msg: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.toast_for(msg, 4, cx);
    }

    /// [`Self::toast`] kept on screen `secs` seconds (errors stay longer).
    pub fn toast_for(&mut self, msg: impl Into<SharedString>, secs: u64, cx: &mut Context<Self>) {
        let Phase::Main(s) = &mut self.phase else {
            return;
        };
        let msg: SharedString = msg.into();
        s.toast = Some(msg.clone());
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_secs(secs))
                .await;
            this.update(cx, |this, cx| {
                if let Phase::Main(s) = &mut this.phase
                    && s.toast.as_ref() == Some(&msg)
                {
                    s.toast = None;
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
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

    /// [`Self::copy`] of an item's field; an organization's item is written to its log.
    pub fn copy_of(&mut self, entry: EntryId, text: String, what: &str, cx: &mut Context<Self>) {
        if let Some(e) = self
            .session()
            .and_then(|s| s.entries.iter().find(|e| e.id == entry).cloned())
        {
            self.org_report(
                cleis_core::org::Action::Copied,
                &e,
                &what.to_lowercase(),
                cx,
            );
        }
        self.copy(text, what, cx);
    }

    pub fn touch_activity(&mut self) {
        if let Phase::Main(s) = &mut self.phase {
            s.last_activity = std::time::Instant::now();
        }
    }

    fn idle_lock_due(&self) -> bool {
        match &self.phase {
            Phase::Main(s) => s.lock_after().is_some_and(|m| {
                s.last_activity.elapsed() >= Duration::from_secs(u64::from(m) * 60)
            }),
            _ => false,
        }
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
        let screen = match &self.phase {
            Phase::Welcome(_) => {
                crate::screens::welcome::render(self, window, cx).into_any_element()
            }
            Phase::Unlock(_) => crate::screens::unlock::render(self, window, cx).into_any_element(),
            Phase::Main(s) if s.gate.is_some() => {
                let Some(g) = s.gate.as_ref() else {
                    unreachable!()
                };
                crate::screens::security::gate(g, &self.tokens(cx).clone(), cx).into_any_element()
            }
            Phase::Main(_) => {
                return crate::screens::main::render(self, window, cx).into_any_element();
            }
        };
        // First launch and unlock: an empty title bar in the frame colour, so the window still
        // moves and keeps its system buttons.
        let t = self.tokens(cx).clone();
        gpui_kit::div()
            .size_full()
            .flex()
            .flex_col()
            .bg(t.chrome)
            .child(
                gpui_kit::component::TitleBar::new()
                    .h(crate::theme::TOPBAR_H)
                    .bg(t.chrome)
                    .border_b_0(),
            )
            .child(gpui_kit::div().flex_1().min_h_0().child(screen))
            .into_any_element()
    }
}

fn appearance_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "matv-studio", "Cleis")
        .map(|d| d.config_dir().join("appearance.json"))
}

/// Theme and accent are not secret: kept in clear so the unlock screen already looks right.
fn save_appearance(p: &Preferences) {
    if cfg!(test) {
        return;
    }
    let Some(path) = appearance_path() else {
        return;
    };
    let subset =
        serde_json::json!({ "theme": p.theme, "accent": p.accent, "accent_scope": p.accent_scope });
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, subset.to_string());
}

/// Appearance remembered from the last session, or the defaults (dark, bronze, extended).
pub fn load_appearance() -> Appearance {
    appearance_path()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice::<Preferences>(&b).ok())
        .map(|p| Appearance::from_prefs(&p))
        .unwrap_or_default()
}
