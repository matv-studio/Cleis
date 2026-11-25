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

/// Default accent colour (D-48).
pub const DEFAULT_ACCENT: &str = "#a57118";

/// Accent colours offered in Preferences, after "None".
pub const ACCENTS: [(&str, &str); 6] = [
    ("#a57118", "Bronze"),
    ("#3b82f6", "Blue"),
    ("#8b5cf6", "Violet"),
    ("#10b981", "Green"),
    ("#f59e0b", "Amber"),
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
    /// `None` = no accent, or a `#rrggbb` colour. Cleis default: bronze `#a57118` (D-48).
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
    pub biometrics: bool,
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
            biometrics: false,
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
        assert_eq!(Preferences::default().accent_scope, AccentScope::Extended);
        assert_eq!(Preferences::default().theme, Theme::Dark);
        assert!(old.tabs.activity);
    }
}
