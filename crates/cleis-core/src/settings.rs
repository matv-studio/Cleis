//! User preferences (screens 06, 15, 16, 19) and organisation policy (screens 08, 18).

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    Dark,
    Light,
    System,
}

/// Default accent colour (D-81, 27/11/2026): bronze, calmer than the ochre of D-51. It keeps
/// 4:1 on the dark page and 3.8:1 on the light one, above the 3.6:1 every accent is held to
/// (WCAG 1.4.11 ≥ 3:1), so both themes show it as is.
pub const DEFAULT_ACCENT: &str = "#977327";

/// The default before D-81: a vault that still has it (never chosen, only defaulted) moves to the
/// new one.
pub const OLD_DEFAULT_ACCENT: &str = "#e0a93b";

/// Accent colours offered in Preferences, after "None".
pub const ACCENTS: [(&str, &str); 6] = [
    ("#977327", "Bronze"),
    ("#3b82f6", "Blue"),
    ("#8b5cf6", "Violet"),
    ("#10b981", "Green"),
    ("#14b8a6", "Teal"),
    ("#ef4444", "Red"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AccentScope {
    Minimal,
    Standard,
    /// Cleis default (D-48).
    #[default]
    Extended,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tabs {
    pub contacts: bool,
    pub health: bool,
    pub activity: bool,
}

impl Default for Tabs {
    fn default() -> Self {
        Self {
            contacts: true,
            health: true,
            activity: true,
        }
    }
}

/// Shortcuts shown until changed: All items and Favorites; Renew soon is off (27/11/2026).
pub const DEFAULT_SHORTCUTS: [bool; 3] = [true, true, false];

/// The sidebar's shortcuts (All items, Favorites, Renew soon) of each vault: `"personal"`, an
/// organization's key, or `"all"` (the single Vault tab, or every vault when `same`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SidebarShortcuts {
    /// Vaults in their own tabs: one set for every tab.
    pub same: bool,
    /// The single Vault tab (mockups 37i, 37j, 27/11/2026): each vault has its own shortcuts,
    /// under its name. Off by default: one set at the top, as before.
    pub split: bool,
    pub sets: std::collections::BTreeMap<String, [bool; 3]>,
}

impl SidebarShortcuts {
    /// Whether each vault has its own set, with vaults in their own tabs or not.
    pub fn per_vault(&self, vault_tabs: bool) -> bool {
        if vault_tabs { !self.same } else { self.split }
    }

    /// The key of the set `vault` uses.
    pub fn key_for(&self, vault: &str, vault_tabs: bool) -> String {
        if self.per_vault(vault_tabs) {
            vault.to_owned()
        } else {
            "all".to_owned()
        }
    }

    /// The shortcuts of the set `key` (`DEFAULT_SHORTCUTS` until changed).
    pub fn of(&self, key: &str) -> [bool; 3] {
        self.sets.get(key).copied().unwrap_or(DEFAULT_SHORTCUTS)
    }

    pub fn toggle(&mut self, key: &str, which: usize) {
        let mut v = self.of(key);
        v[which] = !v[which];
        self.sets.insert(key.to_owned(), v);
    }

    /// "Same for every vault" switched, in the mode shown.
    pub fn toggle_same(&mut self, vault_tabs: bool) {
        if vault_tabs {
            self.same = !self.same;
        } else {
            self.split = !self.split;
        }
    }
}

/// Shortcuts inside Cleis (mockup 38, 27/11/2026): `(id, label, default)`. Changed ones are
/// kept in `Preferences::app_keys`; written like the global ones (`Ctrl+Shift+C`, `Ctrl` is ⌘ on
/// macOS), a bare key allowed only for `Del`, `Backspace` and F1-F24.
pub const APP_KEYS: [(&str, &str, &str); 8] = [
    ("lock", "Lock", "Ctrl+L"),
    ("new", "New password", "Ctrl+N"),
    ("search", "Search", "Ctrl+F"),
    ("copy_user", "Copy username", "Ctrl+C"),
    ("copy_pass", "Copy password", "Ctrl+Shift+C"),
    ("copy_totp", "Copy 2FA code", "Ctrl+Alt+C"),
    ("duplicate", "Duplicate", "Ctrl+D"),
    ("trash", "Move to trash", "Del"),
];

/// A shortcut written `Ctrl+Alt+Shift+K`, read for comparing: modifiers and the key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct KeyCombo<'a> {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    /// Lower case: `k`, `del`, `f5`, `space`.
    pub key: &'a str,
}

/// Reads `Ctrl+Shift+C`; `None` for an empty text.
pub fn parse_keys(s: &str) -> Option<KeyCombo<'_>> {
    let mut c = KeyCombo::default();
    let mut key = None;
    for part in s.split('+').map(str::trim).filter(|p| !p.is_empty()) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "cmd" | "⌘" => c.ctrl = true,
            "alt" | "option" | "⌥" => c.alt = true,
            "shift" | "⇧" => c.shift = true,
            _ => key = Some(part),
        }
    }
    c.key = key?;
    Some(c)
}

/// Two shortcuts are the same keys, written any way.
pub fn same_keys(a: &str, b: &str) -> bool {
    match (parse_keys(a), parse_keys(b)) {
        (Some(x), Some(y)) => {
            x.ctrl == y.ctrl
                && x.alt == y.alt
                && x.shift == y.shift
                && x.key.eq_ignore_ascii_case(y.key)
        }
        _ => false,
    }
}

/// How the vault list is sorted (26/11/2026); `Manual` follows `Preferences::item_order`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    #[default]
    Name,
    Edited,
    Price,
    Renewal,
    Manual,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shortcut {
    pub enabled: bool,
    /// e.g. `"Ctrl+Shift+Space"`; on macOS `Ctrl` reads as `Cmd`.
    pub keys: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FillMethod {
    /// Accessibility interface first, typing as a fallback.
    #[default]
    Automatic,
    Type,
    Paste,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TypingSpeed {
    Slow,
    #[default]
    Normal,
    Fast,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Autofill {
    pub enabled: bool,
    pub quick_access: Shortcut,
    pub fill_best_match: Shortcut,
    pub method: FillMethod,
    pub fill_totp_after: bool,
    pub press_enter: bool,
    pub typing_speed: TypingSpeed,
    pub match_browser_urls: bool,
    pub ask_unlinked: bool,
}

impl Default for Autofill {
    fn default() -> Self {
        Self {
            enabled: true,
            quick_access: Shortcut {
                enabled: true,
                keys: "Ctrl+Shift+Space".into(),
            },
            fill_best_match: Shortcut {
                enabled: true,
                keys: "Ctrl+Alt+A".into(),
            },
            method: FillMethod::Automatic,
            fill_totp_after: true,
            press_enter: false,
            typing_speed: TypingSpeed::Normal,
            match_browser_urls: true,
            ask_unlinked: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub theme: Theme,
    /// `None` = no accent, or a `#rrggbb` colour. Cleis default: bronze `#977327` (D-81).
    pub accent: Option<String>,
    pub accent_scope: AccentScope,
    /// BCP 47 tag: `en`, `fr`.
    pub language: String,
    /// ISO 4217.
    pub currency: String,
    pub tabs: Tabs,
    pub autofill: Autofill,
    /// Clipboard cleared after this many seconds; `None` = never.
    pub clipboard_clear_secs: Option<u32>,
    pub open_at_login: bool,
    /// View shown after unlocking: `all`, `favorites` or `contacts`.
    pub start_in: String,
    /// Verification code asked when Cleis opens (Security › Opening the app).
    pub open_code: Option<crate::open_code::OpenCode>,
    /// Lock after this many minutes without use; `None` = never (personal vault; an
    /// organisation's policy decides for its members, D-02).
    pub lock_after_mins: Option<u32>,
    /// When the master password was last changed (seconds since 1970).
    pub password_changed: Option<i64>,
    /// Contact pairs marked "Not a duplicate" in *Merge & fix*.
    pub not_duplicates: std::collections::BTreeSet<(crate::ContactId, crate::ContactId)>,
    /// When the organization's backup key was saved to a file (screen 13).
    pub org_backup_key_saved: Option<i64>,
    /// The personal vault in its own tab, beside one tab per organization (D-69).
    pub vault_tabs: bool,
    /// Updates (D-74): pre-releases offered too (every 0.x version is one).
    pub update_prereleases: bool,
    /// The dark version of the logo in the app (26/11/2026: the light one by default, both themes).
    pub dark_logo: bool,
    /// Sidebar shortcuts per vault (26/11/2026).
    pub shortcuts: SidebarShortcuts,
    /// Shortcuts inside Cleis changed from `APP_KEYS` (mockup 38); `""`: none.
    pub app_keys: std::collections::BTreeMap<String, String>,
    /// Order of the folders and spaces in the sidebar (dragged); the others follow by name.
    pub space_order: Vec<crate::SpaceId>,
    pub sort_by: SortBy,
    /// "My order" of the items (dragged).
    pub item_order: Vec<crate::EntryId>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            accent: Some(DEFAULT_ACCENT.into()),
            accent_scope: AccentScope::Extended,
            language: "en".into(),
            currency: "EUR".into(),
            tabs: Tabs::default(),
            autofill: Autofill::default(),
            clipboard_clear_secs: Some(30),
            open_at_login: true,
            not_duplicates: Default::default(),
            start_in: "all".into(),
            open_code: None,
            lock_after_mins: None,
            password_changed: None,
            org_backup_key_saved: None,
            vault_tabs: false,
            update_prereleases: true,
            dark_logo: false,
            shortcuts: SidebarShortcuts::default(),
            app_keys: Default::default(),
            space_order: Vec::new(),
            sort_by: SortBy::Name,
            item_order: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LogDetail {
    AdminActions,
    Changes,
    #[default]
    Everything,
}

/// Organisation policy, set by administrators. A personal vault has none (no log, D-38).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Policy {
    pub require_verification_code: bool,
    pub remember_device_days: u32,
    /// `None` = never lock on inactivity (default, D-02).
    pub lock_after_minutes: Option<u32>,
    pub offline_days: u32,
    pub min_password_length: usize,
    pub password_max_age_days: u32,
    pub record_activity: bool,
    pub log_detail: LogDetail,
    pub log_keep_days: u32,
    pub allow_fill_fields: bool,
    pub allow_type_keys: bool,
    pub allow_browser_extension: bool,
    pub allow_read_browser_urls: bool,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            require_verification_code: true,
            remember_device_days: 30,
            lock_after_minutes: None,
            offline_days: 14,
            min_password_length: 12,
            password_max_age_days: 365,
            record_activity: true,
            log_detail: LogDetail::Everything,
            log_keep_days: 365,
            allow_fill_fields: true,
            allow_type_keys: true,
            // Paused (D-38).
            allow_browser_extension: false,
            allow_read_browser_urls: true,
        }
    }
}

impl Preferences {
    /// The keys of the shortcut `id` of `APP_KEYS` (its default until changed).
    pub fn app_key(&self, id: &str) -> String {
        self.app_keys.get(id).cloned().unwrap_or_else(|| {
            APP_KEYS
                .iter()
                .find(|(k, _, _)| *k == id)
                .map(|(_, _, d)| (*d).to_owned())
                .unwrap_or_default()
        })
    }

    /// Every shortcut's id and keys, in Cleis and system-wide (autofill), for finding clashes.
    pub fn all_keys(&self) -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = APP_KEYS
            .iter()
            .map(|(id, _, _)| ((*id).to_owned(), self.app_key(id)))
            .collect();
        if self.autofill.quick_access.enabled {
            v.push(("qa".into(), self.autofill.quick_access.keys.clone()));
        }
        if self.autofill.fill_best_match.enabled {
            v.push(("best".into(), self.autofill.fill_best_match.keys.clone()));
        }
        v
    }

    /// The other shortcut using the same keys as `id`, if any.
    pub fn clash(&self, id: &str) -> Option<String> {
        let all = self.all_keys();
        let mine = all.iter().find(|(k, _)| k == id)?.1.clone();
        if mine.is_empty() {
            return None;
        }
        all.into_iter()
            .find(|(k, keys)| k != id && same_keys(keys, &mine))
            .map(|(k, _)| k)
    }

    /// Brings preferences saved by an older Cleis up to date (D-81: the old default accent).
    pub fn upgraded(mut self) -> Self {
        if self.accent.as_deref() == Some(OLD_DEFAULT_ACCENT) {
            self.accent = Some(DEFAULT_ACCENT.into());
        }
        self
    }

    /// What the user actually gets once the organisation policy is applied.
    pub fn effective_autofill(&self, policy: Option<&Policy>) -> Autofill {
        let mut a = self.autofill.clone();
        if let Some(p) = policy {
            a.match_browser_urls &= p.allow_read_browser_urls;
            if !p.allow_fill_fields && a.method == FillMethod::Automatic {
                a.method = FillMethod::Type;
            }
            if !p.allow_type_keys && !p.allow_fill_fields {
                a.method = FillMethod::Paste;
            }
        }
        if !a.enabled {
            a.quick_access.enabled = false;
            a.fill_best_match.enabled = false;
        }
        a
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_policy() {
        let mut p = Preferences::default();
        let pol = Policy {
            allow_fill_fields: false,
            allow_read_browser_urls: false,
            ..Policy::default()
        };
        let a = p.effective_autofill(Some(&pol));
        assert_eq!(a.method, FillMethod::Type);
        assert!(!a.match_browser_urls);
        p.autofill.enabled = false;
        assert!(!p.effective_autofill(None).quick_access.enabled);
        // Old files without new fields still load.
        let old: Preferences = serde_json::from_str(r#"{"theme":"light"}"#).unwrap();
        assert_eq!(old.theme, Theme::Light);
        assert_eq!(old.accent.as_deref(), Some(DEFAULT_ACCENT));
        // D-81: the ochre that was the default becomes the new default; another colour stays.
        let ochre: Preferences = serde_json::from_str(r##"{"accent":"#e0a93b"}"##).unwrap();
        assert_eq!(ochre.upgraded().accent.as_deref(), Some(DEFAULT_ACCENT));
        let blue: Preferences = serde_json::from_str(r##"{"accent":"#3b82f6"}"##).unwrap();
        assert_eq!(blue.upgraded().accent.as_deref(), Some("#3b82f6"));
        assert_eq!(DEFAULT_SHORTCUTS, [true, true, false]);
        // Mockup 38: defaults, a change, a clash found both ways, written any way.
        let mut k = Preferences::default();
        assert_eq!(k.app_key("copy_pass"), "Ctrl+Shift+C");
        assert!(k.clash("lock").is_none());
        k.app_keys.insert("copy_pass".into(), "ctrl+l".into());
        assert_eq!(k.clash("lock").as_deref(), Some("copy_pass"));
        assert_eq!(k.clash("copy_pass").as_deref(), Some("lock"));
        assert!(same_keys("Ctrl+Shift+C", "shift+ctrl+c"));
        assert!(!same_keys("Ctrl+C", "Ctrl+Shift+C"));
        assert_eq!(parse_keys("Del").unwrap().key, "Del");
        assert_eq!(
            SidebarShortcuts::default().of("personal"),
            [true, true, false]
        );
        assert_eq!(Preferences::default().accent_scope, AccentScope::Extended);
        assert_eq!(Preferences::default().theme, Theme::Dark);
        assert!(old.tabs.activity);
    }
}
