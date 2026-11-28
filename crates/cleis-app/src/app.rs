//! Application state: first launch → unlock → main window (vault, contacts, health, settings).

use crate::i18n::tr;
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
    /// "My vault": the whole personal vault, its folders included (a section, not a folder).
    Personal,
    /// An organization's section: all its spaces (`org_hash` of its key).
    Org(u64),
    /// One vault's favorites or items to renew, under its name (mockup 37j): `0` is the
    /// personal vault, otherwise `org_hash` of the organization's key.
    FavoritesOf(u64),
    RenewSoonOf(u64),
    Trash,
}

/// A stable number for an organization key, so that `Filter` stays `Copy`.
pub fn org_hash(key: &str) -> u64 {
    // FNV-1a: stable across runs (the filter is not saved, but tests compare it).
    key.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// A question asked before an action that cannot be undone.
pub struct Confirm {
    pub title: SharedString,
    pub body: SharedString,
    pub button: SharedString,
    pub action: ConfirmAction,
    /// A second, harmless way out between Cancel and the destructive button (mockup 49l):
    /// its label, icon and action.
    pub alt: Option<(SharedString, &'static str, ConfirmAction)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmAction {
    PurgeEntry(EntryId),
    EmptyTrash,
    DeleteContact(cleis_core::ContactId),
    // Audit U-1: the organization's actions that cannot be undone from this window.
    RemoveMember(cleis_core::MemberId),
    TransferSuper(cleis_core::MemberId),
    MakeSuper(cleis_core::MemberId),
    ResetStep(cleis_core::MemberId),
    SignOutMember(cleis_core::MemberId),
    RevokeInvite(uuid::Uuid),
    LeaveOrg,
    RotateKey,
    DisconnectPlace,
    /// Settings › Account › Revoke access (mockup 39c): Cleis forgets GitHub, then GitHub's page
    /// for Cleis opens to remove the authorization everywhere.
    RevokeGitHub,
    /// D-104: the same for a Microsoft or Google account (Google revoked at once).
    RevokeCloud,
    /// D-76: no more master password.
    RemovePassword,
    /// Mockup 37l: every dated copy on this computer.
    DeleteBackups,
    /// Mockup 49i: one dated copy (`Session::backup_to_delete`).
    DeleteBackup,
    /// Mockup 49l: leaving refused, Settings › Members instead.
    GoMembers,
    /// Mockup 49l: leaving refused, the organization's deletion instead (Server page).
    OpenDeleteOrg,
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
    /// The option chosen (mockups 40, 27/11/2026): none, the three options alone in the middle;
    /// one, they move up and how the vault opens appears under them.
    pub choice: Option<crate::screens::welcome::Start>,
    /// Counts the moves up and down (each one replays the animation), and the way of the last.
    pub round: u32,
    pub up: bool,
    /// Height of what appears under the options, measured: how far they move.
    pub below_h: std::rc::Rc<std::cell::Cell<f32>>,
    pub join: crate::screens::org::JoinForm,
    /// How this computer will open the vault (mockup 35i, D-76).
    pub opening: crate::opening::Mode,
    /// Mockup 37h: the PIN, typed twice, when the vault opens with one.
    pub pin: crate::screens::pin::PinBoxes,
    pub pin_confirm: crate::screens::pin::PinBoxes,
}

pub struct Unlock {
    pub password: Entity<InputState>,
    pub error: Option<SharedString>,
    pub busy: bool,
    /// Whether a master password opens the vault (D-76).
    pub has_password: bool,
    /// How this computer opens it.
    pub mode: crate::opening::Mode,
    /// The recovery key is being typed.
    pub recovery: bool,
    pub recovery_key: Entity<InputState>,
    /// The PIN's boxes (mockup 37g), when this computer opens with a PIN.
    pub pin: Option<crate::screens::pin::PinBoxes>,
    /// "Use the password" chosen over the PIN.
    pub pin_off: bool,
}

impl Unlock {
    pub fn new(path: &std::path::Path, window: &mut Window, cx: &mut Context<CleisApp>) -> Self {
        Self {
            password: password_input(window, cx, ""),
            error: None,
            busy: false,
            has_password: LockedVault::open(path)
                .and_then(|l| l.has_password())
                .unwrap_or(true),
            mode: crate::opening::mode(path),
            recovery: false,
            recovery_key: cx
                .new(|cx| InputState::new(window, cx).placeholder("XXXX-XXXX-XXXX-XXXX-…")),
            pin: crate::opening::pin_info(path)
                .map(|i| crate::screens::pin::PinBoxes::new(i.len, cx)),
            pin_off: false,
        }
    }
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
    /// The backup the confirmation is about (mockup 49i).
    pub backup_to_delete: Option<std::path::PathBuf>,
    /// Sync & backups: every backup listed, not only the 5 latest (mockup 49i).
    pub backups_all: bool,
    /// Columns menu of the vault (screen 02).
    pub columns_menu: Option<crate::screens::columns::ColumnsMenu>,
    /// Settings › Import & export, kept while moving between pages.
    pub import: crate::screens::import::ImportState,
    /// Contacts tab: filter, search, inspector boxes, menus (screens 20, 21).
    pub contacts_ui: crate::screens::contacts::ContactsUi,
    /// Positions of the scrolled areas (their bars, the middle click).
    pub scrolls: crate::scroll::Scrolls,
    /// The language the boxes were made in; a change remakes their placeholders (U-9).
    pub lang_fr: bool,
    /// Settings › Security dialogs and menus.
    pub security: crate::screens::security::SecurityUi,
    /// Verification code still to type before the vault is shown (screen 05).
    pub gate: Option<crate::screens::security::Gate>,
    /// Last mouse or key input, for locking after inactivity.
    pub last_activity: std::time::Instant,
    /// This device's sync place and backups (Settings › Sync & backups).
    pub sync: crate::sync::SyncConfig,
    pub sync_menu: bool,
    /// Sync & backups (mockups 49f, 49g): `add`, `main` or a copy place's id, its menu open.
    pub place_menu: Option<String>,
    /// The account's menu, under its picture at the top right (mockup 40d).
    pub account_menu: bool,
    /// Mockup 46c: the profile edited under its picture.
    pub profile_edit: Option<crate::screens::profile::ProfileEdit>,
    /// Health tab: space shown and issue filter.
    pub health: crate::screens::health::HealthUi,
    /// Open select of Settings › Preferences.
    pub pref_menu: Option<&'static str>,
    /// Vault toolbar: Filter menu (screen 01b) and Cards view.
    pub vault_filter: VaultFilter,
    pub filter_menu: bool,
    pub vault_cards: bool,
    /// The organization shown (tab, settings, activity), if any (D-59). The vault's other
    /// organizations wait in `orgs_idle` and take its place while they are shown or synced (D-69).
    pub org: Option<crate::org::OrgSession>,
    pub orgs_idle: Vec<crate::org::OrgSession>,
    /// Preferences › Vaults: the personal vault's name box, and its subscription.
    pub vault_name: Option<(Entity<InputState>, Subscription)>,
    /// What the Vault tab shows when the personal vault has its own tab (D-69).
    pub scope: Scope,
    /// Personal folders (D-62): the dialog open, the folder whose menu is open.
    pub folder: Option<FolderDialog>,
    /// Sync & backups: GitHub / SFTP place being set up (D-65).
    pub place: Option<crate::screens::places::PlaceDialog>,
    /// Settings › Account: the account chosen before "Sign in" (36b; none: Local).
    pub account_pick: Option<crate::sync::Provider>,
    pub folder_menu: Option<cleis_core::SpaceId>,
    /// Customize mode (mockup 35f): handles to reorder, crosses to delete, shortcuts shown or hidden.
    pub customize: bool,
    /// A column's edge being dragged (mockup 35e).
    pub resize: Option<crate::screens::dnd::Resize>,
    /// The calendar open below a date box (mockup 35g).
    pub calendar: Option<crate::screens::calendar::Calendar>,
    /// Preferences › Appearance: your own accent colour being chosen (mockup 36).
    pub wheel: Option<crate::screens::wheel::Wheel>,
    /// Where the last tab clicked was, relative to the one shown before (-1 left, +1 right): the
    /// page slides from that side (26/11/2026).
    pub tab_dir: f32,
    /// How this computer opens the vault (D-76).
    pub opening: crate::opening::Mode,
    /// Chosen while the vault had no master password: applied once it is set.
    pub after_password: Option<crate::opening::Mode>,
    /// Menu opened by a right-click (mockups 33 to 33d).
    pub ctx: Option<crate::screens::ctx::CtxMenu>,
    /// Settings › Organization: joining one from an open vault.
    pub join: crate::screens::org::JoinForm,
    pub join_subs: Vec<Subscription>,
    /// Sync & backups › Delete account (mockup 37m).
    pub erase: Option<crate::screens::erase::EraseAccount>,
    /// Security › Opening Cleis › PIN: the new PIN typed twice (mockup 37f).
    pub pin_setup: Option<crate::screens::pin::PinSetup>,
    /// Views behind and ahead: the mouse's side buttons, Alt+← / Alt+→ (27/11/2026).
    pub history: crate::nav::History,
}

/// Vaults shown by the Vault tab: everything (one tab), the personal vault, or one organization
/// (its key), when Preferences › Vaults gives each its tab (D-69).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Scope {
    All,
    Personal,
    Org(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VaultFilter {
    pub subs: std::collections::BTreeSet<cleis_core::Subscription>,
    /// `Some(true)`: with a 2FA code, `Some(false)`: without.
    pub two_fa: Option<bool>,
    /// Mockup 49a, "Show": favorites only.
    pub favorites: bool,
    /// Mockup 49a, "Show": items holding recovery codes only.
    pub recovery: bool,
}

impl VaultFilter {
    pub fn count(&self) -> usize {
        self.subs.len()
            + usize::from(self.two_fa.is_some())
            + usize::from(self.favorites)
            + usize::from(self.recovery)
    }

    /// Whether `e` passes the filters that are on.
    pub fn keeps(&self, e: &cleis_core::Entry) -> bool {
        (self.subs.is_empty() || self.subs.contains(&e.subscription))
            && self.two_fa.is_none_or(|w| e.totp.is_some() == w)
            && (!self.favorites || e.favorite)
            && (!self.recovery || !e.recovery.is_empty())
    }
}

impl Session {
    /// The order chosen in the list's menu (Sort by ▸, 26/11/2026); by name within ties.
    fn sort(&self, v: &mut [usize]) {
        use cleis_core::settings::SortBy;
        use std::cmp::Reverse;
        let name = |i: &usize| cleis_core::text::fold(&self.entries[*i].name);
        match self.prefs.sort_by {
            SortBy::Name => v.sort_by_key(name),
            SortBy::Edited => v.sort_by_key(|i| (Reverse(self.entries[*i].updated), name(i))),
            SortBy::Price => v.sort_by_key(|i| {
                let e = &self.entries[*i];
                (Reverse(e.price.as_ref().map(|m| m.yearly_cents())), name(i))
            }),
            // The nearest renewal first; without a date, last.
            SortBy::Renewal => v.sort_by_key(|i| {
                let e = &self.entries[*i];
                (e.renewal.is_none(), e.renewal.clone(), name(i))
            }),
            SortBy::Manual => {
                let pos = |i: &usize| {
                    let id = self.entries[*i].id;
                    self.prefs
                        .item_order
                        .iter()
                        .position(|x| *x == id)
                        .unwrap_or(usize::MAX)
                };
                v.sort_by_key(|i| (pos(i), name(i)));
            }
        }
    }

    /// Whether the vault list shows its folder lines (mockup 48a): All items, My vault, an
    /// organization's section, as a table, when Preferences › Lists › Folder lines is on.
    pub fn folder_lines(&self) -> bool {
        self.prefs.folder_lines
            && !self.vault_cards
            && matches!(self.filter, Filter::All | Filter::Personal | Filter::Org(_))
    }

    /// Where each space's items come in a list with folder lines: the personal vault first (its
    /// root, then its folders as in the sidebar), then each organization as its tabs, its
    /// spaces as in the sidebar. `contacts`: the contacts' folders instead of the vault's.
    pub fn space_ranks(
        &self,
        contacts: bool,
    ) -> std::collections::HashMap<cleis_core::SpaceId, (usize, usize)> {
        use cleis_store::SpaceKind;
        let mut out = std::collections::HashMap::new();
        let mine = crate::screens::sidebar::ordered(
            self,
            self.spaces
                .iter()
                .filter(|sp| sp.for_contacts == contacts && sp.kind == SpaceKind::Personal),
        );
        let home = if contacts {
            mine.first().map(|sp| sp.id)
        } else {
            Some(self.vault.home_space())
        };
        if let Some(h) = home {
            out.insert(h, (0, 0));
        }
        for (i, sp) in mine.iter().filter(|sp| Some(sp.id) != home).enumerate() {
            out.insert(sp.id, (0, i + 1));
        }
        let tabs = self.org_tabs();
        for (j, (key, _)) in tabs.iter().enumerate() {
            let Some(o) = self.all_orgs().find(|o| &o.m.key() == key) else {
                continue;
            };
            // As the sidebar: the spaces the organization names (contacts: its folders).
            let mut theirs = crate::screens::sidebar::ordered(
                self,
                self.spaces.iter().filter(|sp| {
                    sp.for_contacts == contacts
                        && sp.kind == SpaceKind::Organization
                        && (o.names.contains_key(&sp.id)
                            || self.org_of(sp.id).is_some_and(|x| &x.m.key() == key))
                }),
            );
            // An organization's contacts outside any folder come first, as in the sidebar.
            theirs.sort_by_key(|sp| !sp.unfiled);
            for (i, sp) in theirs.iter().enumerate() {
                out.insert(sp.id, (j + 1, i));
            }
        }
        // Spaces no membership claims (an organization left, the demo without its server):
        // last, together, as in the sidebar.
        let orphans = crate::screens::sidebar::ordered(
            self,
            self.spaces.iter().filter(|sp| {
                sp.for_contacts == contacts
                    && sp.kind == SpaceKind::Organization
                    && !out.contains_key(&sp.id)
            }),
        );
        for (i, sp) in orphans.iter().enumerate() {
            out.insert(sp.id, (tabs.len() + 1, i));
        }
        out
    }

    /// The name on a folder line: the folder, after its vault's name when the list holds
    /// several vaults (`with_vault`); a vault's root (or an organization's contacts outside any
    /// folder) is named after the vault.
    /// Whether these spaces belong to more than one vault (the personal one, an organization):
    /// only then do the folder lines name their vault (mockup 49b, 49c).
    pub fn mixes_vaults(&self, spaces: impl IntoIterator<Item = cleis_core::SpaceId>) -> bool {
        let mut first: Option<Option<String>> = None;
        for sp in spaces {
            let v = self.org_of(sp).map(|o| o.m.key());
            match &first {
                None => first = Some(v),
                Some(f) if *f != v => return true,
                Some(_) => {}
            }
        }
        false
    }

    pub fn folder_label(&self, space: cleis_core::SpaceId, with_vault: bool) -> (String, String) {
        let sp = self.spaces.iter().find(|x| x.id == space);
        let vault = match self.org_of(space) {
            Some(o) => o.m.name.clone(),
            None if sp.is_some_and(|x| x.for_contacts) => crate::i18n::tr("My contacts").to_owned(),
            None => self.personal_name(),
        };
        let root = space == self.vault.home_space()
            || sp.is_some_and(|x| {
                x.unfiled
                    || (x.for_contacts
                        && x.kind == cleis_store::SpaceKind::Personal
                        && self
                            .spaces
                            .iter()
                            .find(|y| y.for_contacts && y.kind == cleis_store::SpaceKind::Personal)
                            .is_some_and(|y| y.id == x.id))
            });
        let name = sp.map(|x| crate::i18n::space(&x.name)).unwrap_or_default();
        match (root, with_vault) {
            (true, _) => (
                String::new(),
                if sp.is_some_and(|x| x.unfiled) {
                    format!("{vault} · {}", crate::i18n::tr("Uncategorized"))
                } else {
                    vault
                },
            ),
            (false, true) => (vault, name),
            (false, false) => (String::new(), name),
        }
    }

    /// Whether a space belongs to vault `h` (`0`: personal, else `org_hash` of the key).
    pub fn in_vault(&self, h: u64, space: cleis_core::SpaceId) -> bool {
        if h == 0 {
            self.scope_has(&Scope::Personal, space)
        } else {
            self.org_of(space)
                .is_some_and(|o| org_hash(&o.m.key()) == h)
        }
    }

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
                Filter::Personal => self.scope_has(&Scope::Personal, e.space),
                Filter::Org(h) => self
                    .org_of(e.space)
                    .is_some_and(|o| org_hash(&o.m.key()) == h),
                Filter::FavoritesOf(h) => e.favorite && self.in_vault(h, e.space),
                Filter::RenewSoonOf(h) => {
                    e.subscription == cleis_core::Subscription::RenewSoon
                        && self.in_vault(h, e.space)
                }
                Filter::Trash => false,
            })
            .filter(|(_, e)| q.is_empty() || matches(&e.search_text(), &q))
            .filter(|(_, e)| self.in_scope(e.space))
            .filter(|(_, e)| self.vault_filter.keeps(e))
            .map(|(i, _)| i)
            .collect();
        self.sort(&mut v);
        // Mockup 48a: grouped by folder, each folder keeping the sort within it.
        if self.folder_lines() {
            let ranks = self.space_ranks(false);
            v.sort_by_key(|i| {
                ranks
                    .get(&self.entries[*i].space)
                    .copied()
                    .unwrap_or((usize::MAX, usize::MAX))
            });
        }
        self.visible = v;
        // The trash view has its own list: the vault selection waits for the return.
        if self.filter == Filter::Trash {
            return;
        }
        // An item no longer listed is let go; nothing is chosen in its place (the panel on the
        // right opens on a click or a search, 26/11/2026).
        if self
            .selected
            .is_some_and(|s| !self.visible.iter().any(|i| self.entries[*i].id == s))
        {
            self.selected = None;
        }
        // A search shows its best match.
        if self.selected.is_none() && !q.is_empty() {
            self.selected = self.visible.first().map(|i| self.entries[*i].id);
        }
    }

    /// Minutes of inactivity before Cleis locks: this vault's own choice. An organization only
    /// hides its own spaces after its own delay (D-98).
    pub fn lock_after(&self) -> Option<u32> {
        self.prefs.lock_after_mins
    }

    /// Whether this item's password may be shown (an organization role may only copy and fill).
    pub fn may_see(&self, e: &Entry) -> bool {
        self.may(e.space, cleis_core::org::Perm::See)
    }

    /// What the member's role allows in `space`, narrowed there (D-101); everything in a
    /// personal space (audit L-1, L-4).
    pub fn may(&self, space: cleis_core::SpaceId, p: cleis_core::org::Perm) -> bool {
        self.org_of(space).is_none_or(|o| {
            o.can(p)
                && o.me()
                    .and_then(|m| m.limits.get(&space))
                    .is_none_or(|l| l.allows(p))
        })
    }

    /// Whether `space` belongs to an organization.
    pub fn is_org_space(&self, space: cleis_core::SpaceId) -> bool {
        self.org_of(space).is_some()
    }

    /// Every organization of this vault, the one shown first.
    pub fn all_orgs(&self) -> impl Iterator<Item = &crate::org::OrgSession> {
        self.org.iter().chain(self.orgs_idle.iter())
    }

    pub fn all_orgs_mut(&mut self) -> impl Iterator<Item = &mut crate::org::OrgSession> {
        self.org.iter_mut().chain(self.orgs_idle.iter_mut())
    }

    /// The organizations in a stable order for the tabs: as joined, then by name.
    pub fn org_tabs(&self) -> Vec<(String, String)> {
        let joined: Vec<String> = crate::org::memberships(&self.vault)
            .iter()
            .map(|m| m.key())
            .collect();
        let mut v: Vec<(String, String)> = self
            .all_orgs()
            .map(|o| (o.m.key(), o.m.name.clone()))
            .collect();
        v.sort_by_key(|(k, n)| {
            (
                joined.iter().position(|x| x == k).unwrap_or(usize::MAX),
                n.clone(),
            )
        });
        v
    }

    /// Name of the personal vault (its first space; "My vault" unless renamed, D-69).
    pub fn personal_name(&self) -> String {
        let home = self.vault.home_space();
        self.spaces.iter().find(|x| x.id == home).map_or_else(
            || tr("My vault").to_owned(),
            |x| crate::i18n::space(&x.name),
        )
    }

    /// Where something new goes (27/11/2026): the folder shown; in an organization shown (its
    /// tab, its section), its first folder this member may write in; otherwise the personal
    /// vault. `None`: nowhere this member may add.
    pub fn target_space(&self) -> Option<cleis_core::SpaceId> {
        if let Filter::Space(sp) = self.filter {
            return self.may(sp, cleis_core::org::Perm::Edit).then_some(sp);
        }
        match crate::screens::org::org_in_view(self) {
            Some(o) => self
                .spaces
                .iter()
                .filter(|sp| !sp.for_contacts && o.names.contains_key(&sp.id))
                .map(|sp| sp.id)
                .find(|sp| {
                    o.can(cleis_core::org::Perm::Edit) && self.may(*sp, cleis_core::org::Perm::Edit)
                }),
            None => Some(self.vault.home_space()),
        }
    }

    /// The organization a space belongs to.
    pub fn org_of(&self, space: cleis_core::SpaceId) -> Option<&crate::org::OrgSession> {
        self.all_orgs().find(|o| o.names.contains_key(&space))
    }

    /// The shortest password allowed for the vault: Cleis' own minimum. Organizations no longer
    /// rule how Cleis opens (D-98).
    pub fn master_min(&self) -> usize {
        10
    }

    /// Whether an item of `space` is shown by the current scope (D-69).
    pub fn in_scope(&self, space: cleis_core::SpaceId) -> bool {
        self.scope_has(&self.scope, space)
    }

    /// Whether `space` is in the vault(s) `scope` names (vault tabs, Health filter).
    pub fn scope_has(&self, scope: &Scope, space: cleis_core::SpaceId) -> bool {
        match scope {
            Scope::All => true,
            Scope::Personal => self
                .spaces
                .iter()
                .find(|x| x.id == space)
                .is_some_and(|x| x.kind == cleis_store::SpaceKind::Personal),
            Scope::Org(k) => self.org_of(space).is_some_and(|o| o.m.key() == *k),
        }
    }

    /// Whether a right-click menu is open on this row (outlined, not selected).
    pub fn aimed(&self, target: crate::screens::ctx::CtxTarget) -> bool {
        self.ctx.as_ref().is_some_and(|m| m.target == target)
    }

    /// Vaults in their own tabs: only with an organization, the personal vault alone needs no
    /// tab of its own (27/11/2026).
    pub fn vault_tabs(&self) -> bool {
        self.prefs.vault_tabs && self.all_orgs().next().is_some()
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
    /// Sign in with GitHub under way (mockup 37n), whatever the phase.
    pub github: Option<crate::screens::github::GitHubSignIn>,
    /// Signed in on the first launch with no vault on the account (mockup 37r): the vault made
    /// next is saved to it.
    pub github_after: Option<crate::screens::github::Signed>,
    /// Where GitHub is: github.com, or the tests' stand-in.
    pub github_hosts: crate::github_login::Hosts,
    /// Sign in with Microsoft or Google under way (D-104), whatever the phase.
    pub cloud: Option<crate::screens::cloud::CloudSignIn>,
    /// Signed in on the first launch with no vault in the account: the vault made next goes
    /// there (the place and its secret).
    pub cloud_after: Option<(crate::remote::RemotePlace, crate::remote::Secret)>,
    /// Where Microsoft and Google are: their own addresses, or the tests' stand-in.
    pub cloud_hosts: crate::cloud::CloudHosts,
    /// Theme forced on the command line (screenshots); otherwise preferences decide.
    pub forced_mode: Option<Mode>,
    /// The colour wheel's field to rewrite at the next render (it needs the window).
    pub wheel_field: Option<(Entity<InputState>, String)>,
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
    /// Middle-click scrolling under way (mockup 48j).
    pub autoscroll: Option<crate::scroll::Auto>,
    /// Pages opened from the vault (a site's address), kept instead of opened in screenshots
    /// and tests (`no_shortcuts`).
    pub opened: Vec<String>,
    /// Published versions (D-74).
    pub update: crate::screens::update::UpdateState,
    /// The message at the bottom whose removal is planned (see [`Self::watch_toast`]).
    toast_seen: Option<SharedString>,
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
    /// An item's website in the browser; `https://` added to a bare address. Screenshots and
    /// tests keep it in `opened` instead.
    pub fn open_site(&mut self, url: &str, cx: &mut Context<Self>) {
        let url = if url.contains("://") {
            url.to_owned()
        } else {
            format!("https://{url}")
        };
        if self.no_shortcuts || self.demo {
            self.opened.push(url);
        } else {
            cx.open_url(&url);
        }
    }

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
                    app.update_due(cx);
                    if app.auto_sync_due() {
                        app.sync(true, cx);
                    }
                    if app.auto_backup_due() {
                        app.backup(true, cx);
                    }
                    // Every organization of the vault, each in turn (D-69).
                    for key in app.org_keys() {
                        app.in_org(&key, |app| {
                            if app.org_sync_due() {
                                app.org_sync(None, cx);
                            }
                        });
                    }
                    // An organization asks again after its own inactivity delay (D-98).
                    app.org_idle_check(cx);
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
        let first = Unlock::new(&path, window, cx);
        let mut app = Self {
            path,
            phase: Phase::Unlock(first),
            forced_mode,
            wheel_field: None,
            main_window: Some(window.window_handle()),
            autofill: Default::default(),
            no_shortcuts: false,
            opened: vec![],
            tray: None,
            main_hwnd: crate::tray::hwnd_of(window),
            demo: false,
            autoscroll: None,
            update: crate::screens::update::UpdateState {
                pre: true,
                ..Default::default()
            },
            toast_seen: None,
            github: None,
            github_after: None,
            github_hosts: Default::default(),
            cloud: None,
            cloud_after: None,
            cloud_hosts: Default::default(),
            _ticker: ticker,
            _subs: vec![],
        };
        // A server started by Cleis stops with it (an installed service does not).
        cx.on_app_quit(|_, _| {
            crate::host::stop();
            async {}
        })
        .detach();
        if app.path.exists() {
            app.show_unlock(window, cx);
            // "Nothing" on this computer (D-76): the vault opens by itself at launch.
            if crate::opening::mode(&app.path) == crate::opening::Mode::Nothing {
                app.unlock_device(window, cx);
            }
        } else {
            app.show_welcome(window, cx);
        }
        app
    }

    /// No contact shown: its panel closes (click in the void, its "×").
    pub fn deselect_contact(&mut self, cx: &mut Context<Self>) {
        if let Some(s) = self.session()
            && s.selected_contact.take().is_some()
        {
            s.contacts_ui.menu = None;
            cx.notify();
        }
    }

    /// No item selected: the inspector closes (click in the void, its "×").
    pub fn deselect(&mut self, cx: &mut Context<Self>) {
        if let Some(s) = self.session()
            && s.selected.take().is_some()
        {
            s.reveal = false;
            cx.notify();
        }
    }

    /// Customize mode on or off (the button beside Trash, the banner's Done, Escape).
    pub fn toggle_customize(&mut self, cx: &mut Context<Self>) {
        if let Some(s) = self.session() {
            s.customize = !s.customize;
            s.folder_menu = None;
        }
        cx.notify();
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
            choice: None,
            round: 0,
            up: true,
            below_h: Default::default(),
            join,
            opening: crate::opening::Mode::Password,
            pin: crate::screens::pin::PinBoxes::new(crate::opening::DEFAULT_PIN_LEN, cx),
            pin_confirm: crate::screens::pin::PinBoxes::new(crate::opening::DEFAULT_PIN_LEN, cx),
        }));
        cx.notify();
    }

    pub fn show_unlock(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_unlock_taking(window, cx);
    }

    /// [`Self::show_unlock`], returning the phase it replaced.
    pub fn show_unlock_taking(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Phase> {
        let u = Unlock::new(&self.path, window, cx);
        self._subs = vec![
            cx.subscribe_in(
                &u.password,
                window,
                |this, _, ev: &InputEvent, window, cx| {
                    if let InputEvent::PressEnter { .. } = ev {
                        this.unlock(window, cx);
                    }
                },
            ),
            cx.subscribe_in(
                &u.recovery_key,
                window,
                |this, _, ev: &InputEvent, window, cx| {
                    if let InputEvent::PressEnter { .. } = ev {
                        this.unlock_recovery(window, cx);
                    }
                },
            ),
        ];
        if let Some(p) = u.pin.as_ref() {
            window.focus(&p.focus, cx);
        } else if u.has_password {
            u.password.update(cx, |s, cx| s.focus(window, cx));
        }
        let old = std::mem::replace(&mut self.phase, Phase::Unlock(u));
        cx.notify();
        Some(old)
    }

    pub fn create_vault(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Phase::Welcome(w) = &mut self.phase else {
            return;
        };
        if w.busy {
            return;
        }
        let opening = w.opening;
        let with_password = opening == crate::opening::Mode::Password;
        let pw = if with_password {
            w.password.read(cx).value().to_string()
        } else {
            // Windows Hello or nothing (D-76): a password nobody knows, removed once the
            // recovery key and this computer's key exist.
            cleis_crypto::hex(&cleis_crypto::random_bytes::<32>().unwrap_or([7; 32]))
        };
        let confirm = w.confirm.read(cx).value().to_string();
        if with_password && pw.chars().count() < 10 {
            w.error = Some(tr("At least 10 characters").into());
            cx.notify();
            return;
        }
        if with_password && pw != confirm {
            w.error = Some(tr("The passwords differ").into());
            cx.notify();
            return;
        }
        // Mockup 37h: a PIN, the same twice.
        let pin = if opening == crate::opening::Mode::Pin {
            if !w.pin.full() || !w.pin_confirm.full() {
                w.error = Some(crate::trf!("The PIN has {} digits", w.pin.len).into());
                cx.notify();
                return;
            }
            if *w.pin.value != *w.pin_confirm.value {
                w.error = Some(tr("The two PINs differ").into());
                w.pin_confirm.clear();
                cx.notify();
                return;
            }
            Some((w.pin.value.clone(), w.pin.len))
        } else {
            None
        };
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
                    tr("Checking the invitation…").into()
                } else {
                    j.error
                        .clone()
                        .unwrap_or(tr("Enter the server address and invitation code").into())
                });
                cx.notify();
                return;
            };
            let name = read(&j.name);
            if info.name.trim().is_empty() && name.trim().is_empty() {
                w.error = Some(tr("Enter your name").into());
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
        let challenge = crate::opening::new_challenge();
        cx.spawn_in(window, async move |this, cx| {
            let res = cx
                .background_executor()
                .spawn(async move {
                    // Windows Hello first: nothing is created if the prompt is turned down.
                    let hello = match opening {
                        crate::opening::Mode::Hello => Some(crate::opening::hello_key(&challenge)?),
                        _ => None,
                    };
                    if let Some(dir) = path.parent() {
                        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
                    }
                    let mut v = Vault::create(&path, &pw, KdfParams::default())
                        .map_err(|e| e.to_string())?;
                    let finish = || -> Result<_, String> {
                        // D-96: no password, no recovery key forced either: this computer's key
                        // (and the account's, once synced) open it; one can be made later.
                        let key = if opening == crate::opening::Mode::Password {
                            Some(v.new_recovery_key().map_err(|e| e.to_string())?)
                        } else {
                            None
                        };
                        match (opening, hello) {
                            (crate::opening::Mode::Hello, Some(k)) => {
                                crate::opening::set_hello(&path, &v, challenge, &k)?
                            }
                            (crate::opening::Mode::Nothing, _) => {
                                crate::opening::set_nothing(&path, &v)?
                            }
                            (crate::opening::Mode::Pin, _) => {
                                if let Some((pin, len)) = &pin {
                                    crate::opening::set_pin(
                                        &path,
                                        &v,
                                        pin,
                                        *len,
                                        crate::opening::DEFAULT_PIN_TRIES,
                                    )?
                                }
                            }
                            _ => {}
                        }
                        if opening != crate::opening::Mode::Password {
                            v.remove_password_only().map_err(|e| e.to_string())?;
                        }
                        Ok(key)
                    };
                    match finish() {
                        Ok(key) => Ok::<_, String>((v, key)),
                        // A vault nobody could open is not left behind.
                        Err(e) => {
                            drop(v);
                            let _ = std::fs::remove_file(&path);
                            crate::opening::forget(&path);
                            Err(e)
                        }
                    }
                })
                .await;
            let (res, recovery) = match res {
                Ok((v, key)) => (Ok(v), key),
                Err(e) => (Err(e), None),
            };
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
                    // The recovery key, shown once (mockup 35i: "shown next"); without one, the
                    // account (if signed in) is reached at once.
                    match recovery {
                        Some(key) => {
                            if let Some(s) = this.session() {
                                s.security.dialog =
                                    Some(crate::screens::security::SecDialog::RecoveryKey(key));
                            }
                        }
                        None => this.resume_github(window, cx),
                    }
                    if let Some(e) = failed {
                        this.toast_for(crate::trf!("Not joined: {}", e), 8, cx);
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
        let demo = self.demo;
        cx.spawn_in(window, async move |this, cx| {
            let res = cx
                .background_executor()
                .spawn(async move {
                    let mut v = LockedVault::open(&path).and_then(|l| l.unlock(&pw))?;
                    // Audit S-12: weak key derivation settings are raised to the defaults.
                    if !demo && v.kdf().is_ok_and(|k| !k.meets_floor()) {
                        let _ = v.upgrade_kdf(&pw, cleis_crypto::KdfParams::default());
                    }
                    Ok::<_, cleis_store::StoreError>(v)
                })
                .await;
            this.update_in(cx, |this, window, cx| match res {
                Ok(v) => {
                    this.enter(v, window, cx);
                }
                Err(e) => {
                    if let Phase::Unlock(u) = &mut this.phase {
                        u.busy = false;
                        u.error = Some(match e {
                            cleis_store::StoreError::WrongPassword => tr("Wrong password").into(),
                            cleis_store::StoreError::NoPassword => {
                                tr("This vault has no password: use the recovery key").into()
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

    /// The unlock failed: the message, the boxes emptied.
    fn unlock_failed(&mut self, msg: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        if let Phase::Unlock(u) = &mut self.phase {
            u.busy = false;
            u.error = Some(msg);
            for i in [&u.password, &u.recovery_key] {
                i.update(cx, |s, cx| s.set_value("", window, cx));
            }
        }
        cx.notify();
    }

    /// Windows Hello (D-76): the prompt, then the vault opens with this computer's key.
    pub fn unlock_hello(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Phase::Unlock(u) = &mut self.phase else {
            return;
        };
        if u.busy {
            return;
        }
        let Some(challenge) = crate::opening::hello_challenge(&self.path) else {
            return;
        };
        u.busy = true;
        u.error = None;
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let key = cx
                .background_executor()
                .spawn(async move { crate::opening::hello_key(&challenge) })
                .await;
            this.update_in(cx, |this, window, cx| {
                match key.and_then(|k| crate::opening::open(&this.path, Some(&k))) {
                    Ok(v) => this.enter(v, window, cx),
                    Err(e) => this.unlock_failed(crate::i18n::tr_dynamic(&e).into(), window, cx),
                }
            })
            .ok();
        })
        .detach();
    }

    /// Nothing to type on this computer (D-76): the key kept for the Windows account opens it.
    pub fn unlock_device(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match crate::opening::open(&self.path, None) {
            Ok(v) => self.enter(v, window, cx),
            Err(e) => self.unlock_failed(crate::i18n::tr_dynamic(&e).into(), window, cx),
        }
    }

    /// Mockup 37g: the PIN typed opens the vault on this computer (Argon2id: off the interface
    /// thread); a wrong one is counted, the last allowed one sends back to the master password.
    pub fn unlock_pin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Phase::Unlock(u) = &mut self.phase else {
            return;
        };
        let Some(p) = u.pin.as_ref() else { return };
        if u.busy || !p.full() {
            return;
        }
        let pin = p.value.clone();
        u.busy = true;
        u.error = None;
        cx.notify();
        let path = self.path.clone();
        cx.spawn_in(window, async move |this, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { crate::opening::open_pin(&path, &pin) })
                .await;
            this.update_in(cx, |this, window, cx| {
                use crate::opening::PinError;
                let msg: SharedString = match res {
                    Ok(v) => return this.enter(v, window, cx),
                    Err(PinError::Wrong(n)) => crate::trf!("Wrong PIN · {} tries left", n).into(),
                    Err(PinError::Locked) => {
                        if let Phase::Unlock(u) = &mut this.phase {
                            u.mode = crate::opening::Mode::Password;
                            u.pin = None;
                            if u.has_password {
                                u.password.update(cx, |s, cx| s.focus(window, cx));
                            }
                        }
                        tr("Too many wrong PINs: use the password").into()
                    }
                    Err(PinError::Other(e)) => crate::i18n::tr_dynamic(&e).into(),
                };
                if let Phase::Unlock(u) = &mut this.phase
                    && let Some(p) = u.pin.as_mut()
                {
                    p.clear();
                }
                this.unlock_failed(msg, window, cx);
            })
            .ok();
        })
        .detach();
    }

    /// The recovery key typed (D-76) opens the vault, master password or not.
    pub fn unlock_recovery(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Phase::Unlock(u) = &mut self.phase else {
            return;
        };
        let text = zeroize::Zeroizing::new(u.recovery_key.read(cx).value().to_string());
        if u.busy || text.trim().is_empty() {
            return;
        }
        u.busy = true;
        u.error = None;
        cx.notify();
        let path = self.path.clone();
        cx.spawn_in(window, async move |this, cx| {
            let res = cx
                .background_executor()
                .spawn(
                    async move { LockedVault::open(&path).and_then(|l| l.unlock_recovery(&text)) },
                )
                .await;
            this.update_in(cx, |this, window, cx| match res {
                Ok(v) => this.enter(v, window, cx),
                Err(e) => {
                    let msg = match e {
                        cleis_store::StoreError::WrongRecoveryKey => tr("Wrong recovery key"),
                        cleis_store::StoreError::NoRecoveryKey => {
                            tr("This vault has no recovery key")
                        }
                        _ => tr("Wrong recovery key"),
                    };
                    this.unlock_failed(msg.into(), window, cx)
                }
            })
            .ok();
        })
        .detach();
    }

    /// Opens the main window on an unlocked vault.
    pub fn enter(&mut self, vault: Vault, window: &mut Window, cx: &mut Context<Self>) {
        // Biometric unlock was removed (audit S-9): a device key left by an earlier build goes.
        let mut bio = self.path.as_os_str().to_owned();
        bio.push(".bio");
        let _ = std::fs::remove_file(std::path::PathBuf::from(bio));
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
                // A new vault keeps the language already shown (system's, or chosen) (U-9).
                vault.saved_preferences()?.unwrap_or_else(|| Preferences {
                    language: crate::i18n::current_tag().into(),
                    ..Preferences::default()
                }),
            ))
        };
        let (entries, trash, contacts, spaces, columns, prefs) = match load() {
            Ok(x) => x,
            Err(e) => {
                let mut u = Unlock::new(&self.path, window, cx);
                u.error = Some(crate::i18n::tr_dynamic(&e.to_string()).into());
                self.phase = Phase::Unlock(u);
                cx.notify();
                return;
            }
        };
        // The language first, so the boxes below get their placeholders in it.
        self.apply_appearance(&prefs, cx);
        let search = cx.new(|cx| InputState::new(window, cx).placeholder(tr("Search")));
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
            backup_to_delete: None,
            backups_all: false,
            columns_menu: None,
            import: Default::default(),
            contacts_ui: Default::default(),
            scrolls: Default::default(),
            lang_fr: crate::i18n::is_french(),
            security: Default::default(),
            gate: None,
            last_activity: std::time::Instant::now(),
            sync: crate::sync::load_config(&self.path),
            sync_menu: false,
            place_menu: None,
            account_menu: false,
            profile_edit: None,
            health: Default::default(),
            pref_menu: None,
            vault_filter: Default::default(),
            filter_menu: false,
            vault_cards: false,
            org: None,
            folder: None,
            place: None,
            account_pick: None,
            folder_menu: None,
            customize: false,
            resize: None,
            calendar: None,
            wheel: None,
            tab_dir: 0.,
            opening: crate::opening::mode(&self.path),
            after_password: None,
            ctx: None,
            orgs_idle: vec![],
            vault_name: None,
            scope: Scope::All,
            join: Default::default(),
            join_subs: vec![],
            erase: None,
            pin_setup: None,
            history: Default::default(),
        };
        match s.prefs.start_in.as_str() {
            "favorites" => s.filter = Filter::Favorites,
            "contacts" if s.prefs.tabs.contacts => s.tab = Tab::Contacts,
            // Mockup 49e: one Vault tab, My vault's section (all its folders).
            "vault:personal" if !s.vault_tabs() => s.filter = Filter::Personal,
            _ => {}
        }
        s.refilter();
        window.focus(&s.main_focus.clone(), cx);
        self.phase = Phase::Main(Box::new(s));
        self.load_org(cx);
        // Start in a vault tab (mockup 37k): known once the organizations are loaded.
        let start = self.session().and_then(|s| {
            let key = s.prefs.start_in.strip_prefix("vault:")?;
            if !s.vault_tabs() {
                return None;
            }
            if key == "personal" {
                Some(Scope::Personal)
            } else {
                s.org_tabs()
                    .iter()
                    .any(|(k, _)| k == key)
                    .then(|| Scope::Org(key.to_owned()))
            }
        });
        if let Some(scope) = start {
            self.open_tab(Tab::Vault, Some(scope), cx);
        }
        self.open_gate(window, cx);
        if !cfg!(test) && !self.no_shortcuts {
            self.sync_autofill(cx);
        }
        self.sync_open_at_login();
        cx.notify();
    }

    pub fn lock(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // What was typed and not saved yet goes first.
        self.flush_edits(cx);
        // Last changes sent before the keys are wiped.
        // A remote place, main or copy (mockup 49f): sent off the interface after locking.
        let remote = matches!(&self.phase, Phase::Main(s)
            if s.sync.auto && s.sync.place.is_some() && s.gate.is_none()
                && (s.sync.remote.is_some() || s.sync.copies.iter().any(|c| c.remote.is_some())));
        if let Phase::Main(s) = &self.phase
            && s.sync.auto
            && s.sync.place.is_some()
            && s.gate.is_none()
            && !remote
        {
            self.sync(true, cx);
        }
        if let Phase::Main(_) = self.phase {
            let old = self.show_unlock_taking(window, cx);
            // GitHub / SFTP (audit L-5): the vault lives on in the background until its last
            // changes are sent, then it is dropped and its keys wiped.
            if remote && let Some(Phase::Main(s)) = old {
                let s = *s;
                let path = self.path.clone();
                cx.background_spawn(async move {
                    if let Err(e) = crate::sync::remote_sync_detached(s.vault, s.sync, &path) {
                        cleis_autofill::log(&crate::trf!("sync on lock: {}", e));
                    }
                })
                .detach();
            }
        }
    }

    /// After a language change: the placeholders of the boxes that stay, and the inspector
    /// boxes made again (U-9).
    pub fn refresh_language(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let fr = crate::i18n::is_french();
        let Phase::Main(s) = &mut self.phase else {
            return;
        };
        if s.lang_fr == fr {
            return;
        }
        s.lang_fr = fr;
        s.editor = None;
        s.contacts_ui.editor = None;
        let mut boxes = vec![(s.search.clone(), "Search")];
        if let Some(i) = &s.contacts_ui.search {
            boxes.push((i.clone(), "Search name, company, phone"));
        }
        for o in s.org.iter().chain(s.orgs_idle.iter()) {
            boxes.extend(o.ui.search.iter().map(|i| (i.clone(), "Search")));
            boxes.extend(o.ui.log_search.iter().map(|i| (i.clone(), "Search")));
        }
        for (i, ph) in boxes {
            i.update(cx, |i, cx| i.set_placeholder(tr(ph), window, cx));
        }
    }

    /// Applies theme and accent from the preferences (and remembers them for the unlock screen).
    pub fn apply_appearance(&mut self, prefs: &Preferences, cx: &mut Context<Self>) {
        crate::i18n::set_language(&prefs.language);
        self.update.pre = prefs.update_prereleases;
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
        let msg: SharedString = crate::i18n::tr_dynamic(&msg.into()).into();
        s.toast = Some(msg.clone());
        self.clear_toast_after(msg, Duration::from_secs(secs), cx);
    }

    /// Plans the removal of a message set directly in `Session::toast` (render calls it): a
    /// confirmation such as "Saved" goes after 1.5 s, anything else after 4 s.
    pub fn watch_toast(&mut self, cx: &mut Context<Self>) {
        let Phase::Main(s) = &self.phase else { return };
        let Some(msg) = s.toast.clone() else {
            self.toast_seen = None;
            return;
        };
        if self.toast_seen.as_ref() == Some(&msg) {
            return;
        }
        let short = [tr("Saved"), tr("Copied")].contains(&msg.as_ref());
        let after = Duration::from_millis(if short { 1500 } else { 4000 });
        self.clear_toast_after(msg, after, cx);
    }

    fn clear_toast_after(&mut self, msg: SharedString, after: Duration, cx: &mut Context<Self>) {
        self.toast_seen = Some(msg.clone());
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(after).await;
            this.update(cx, |this, cx| {
                if let Phase::Main(s) = &mut this.phase
                    && s.toast.as_ref() == Some(&msg)
                {
                    s.toast = None;
                    this.toast_seen = None;
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
            s.toast = Some(crate::trf!("{} copied", crate::i18n::tr_owned(what)).into());
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
        // Audit L-4: the role's "Copy, fill" right.
        if let Some(s) = self.session()
            && s.entries
                .iter()
                .find(|e| e.id == entry)
                .is_some_and(|e| !s.may(e.space, cleis_core::org::Perm::Fill))
        {
            s.toast = Some(tr("Your role does not allow copying this item").into());
            cx.notify();
            return;
        }
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
                let t = self.tokens(cx).clone();
                let gh = crate::screens::github::dialog(self, &t, cx)
                    .or_else(|| crate::screens::cloud::dialog(self, &t, cx));
                let main = crate::screens::main::render(self, window, cx).into_any_element();
                // Mockup 48j: the middle click's anchor; the pointer's moves set its speed; any
                // press stops it (and does nothing else), releasing the held wheel too.
                let anchor = self
                    .autoscroll
                    .as_ref()
                    .map(|a| crate::scroll::anchor(a, &t));
                return gpui_kit::div()
                    .size_full()
                    .relative()
                    .on_mouse_move(cx.listener(|this, e: &gpui_kit::MouseMoveEvent, _, _| {
                        this.autoscroll_move(e.position)
                    }))
                    .capture_any_mouse_down(cx.listener(|this, _, _, cx| {
                        if this.stop_autoscroll(cx) {
                            cx.stop_propagation();
                        }
                    }))
                    .on_mouse_up(
                        gpui_kit::MouseButton::Middle,
                        cx.listener(|this, _, _, cx| this.autoscroll_release(cx)),
                    )
                    .child(main)
                    .children(gh)
                    .children(anchor)
                    .into_any_element();
            }
        };
        let gh = {
            let t = self.tokens(cx).clone();
            crate::screens::github::dialog(self, &t, cx)
                .or_else(|| crate::screens::cloud::dialog(self, &t, cx))
        };
        // First launch and unlock: an empty title bar in the frame colour, so the window still
        // moves and keeps its system buttons.
        let t = self.tokens(cx).clone();
        gpui_kit::div()
            .size_full()
            .flex()
            .flex_col()
            .bg(t.chrome)
            .child(crate::ui::title_bar(&t, window, gpui_kit::div()))
            .child(gpui_kit::div().flex_1().min_h_0().child(screen))
            .when_some(gh, |d, dlg| d.relative().child(dlg))
            .into_any_element()
    }
}

/// The appearance kept in clear, removed with the account (mockup 37m).
pub fn forget_appearance() {
    if cfg!(test) {
        return;
    }
    if let Some(p) = appearance_path() {
        let _ = std::fs::remove_file(p);
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
    let subset = serde_json::json!({
        "theme": p.theme,
        "accent": p.accent,
        "accent_scope": p.accent_scope,
        "language": p.language,
        "update_prereleases": p.update_prereleases,
        "dark_logo": p.dark_logo,
    });
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, subset.to_string());
}

/// Appearance remembered from the last session, or the defaults (dark, bronze, extended).
pub fn load_appearance() -> Appearance {
    let p = appearance_path()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice::<Preferences>(&b).ok())
        .map(Preferences::upgraded)
        .unwrap_or_else(|| Preferences {
            // First launch: the system's language (U-9).
            language: sys_locale::get_locale().unwrap_or_default(),
            ..Preferences::default()
        });
    // The language too, so the unlock screen speaks it (U-9).
    crate::i18n::set_language(&p.language);
    Appearance::from_prefs(&p)
}
