//! Settings (screens 06, 07–10, 13, 15, 16): pages of the account, then of the organization.

use crate::app::{CleisApp, Session};
use crate::autofill::AutofillState;
use crate::theme::*;
use crate::ui::*;
use cleis_autofill::hotkeys::{Shortcut, Status};
use cleis_core::settings::TypingSpeed;
use cleis_core::settings::{ACCENTS, AccentScope, FillMethod, Preferences, Theme};
use gpui_kit::KeyDownEvent;
use gpui_kit::TestSupportExt;
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, SharedString, div, px};

const ACCOUNT: [(&str, &str); 4] = [
    ("lock", "Security"),
    ("gear", "Preferences"),
    ("fill", "Autofill"),
    ("sync", "Sync & backups"),
];

/// Pages of the navigation: `(group, organization key, [(icon, name)])`. Without an
/// organization, Import & export stays under My account with a page to join one (D-54); with
/// several, each has its group (D-69).
type Group = (String, Option<String>, Vec<(&'static str, &'static str)>);

fn groups(s: &Session) -> Vec<Group> {
    let mut account: Vec<_> = ACCOUNT.to_vec();
    if s.org.is_none() {
        account.push(("upload", "Import & export"));
        account.push(("server", "Organization"));
        return vec![("My account".into(), None, account)];
    }
    account.insert(0, ("user", "Profile"));
    account.insert(2, ("laptop", "Devices"));
    let mut out = vec![("My account".into(), None, account)];
    for o in s.all_orgs() {
        let mut org = vec![];
        if o.can(cleis_core::org::Perm::Manage) {
            org.extend([
                ("users", "Members"),
                ("folder", "Spaces"),
                ("shield", "Security policy"),
                ("key", "Encryption"),
            ]);
        }
        org.extend([("server", "Server"), ("upload", "Import & export")]);
        out.push((o.m.name.clone(), Some(o.m.key()), org));
    }
    out
}

pub fn page(
    s: &Session,
    af: &AutofillState,
    path: &std::path::Path,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> Div {
    let mut nav = div()
        .w(px(220.))
        .flex_none()
        .h_full()
        .bg(t.surface)
        .border_r_1()
        .border_color(t.line)
        .px(px(8.))
        .py(px(12.))
        .flex()
        .flex_col()
        .gap(px(2.));
    let shown_key = s.org.as_ref().map(|o| o.m.key());
    for (gi, (group, key, pages)) in groups(s).into_iter().enumerate() {
        // The organization shown keeps the plain ids; the others' pages carry their key.
        let shown = key.is_none() || key == shown_key;
        nav = nav.child(
            div()
                .px(px(8.))
                .pb(px(4.))
                .when(gi > 0, |d| d.pt(px(16.)))
                .text_size(TEXT_XS)
                .font_weight(W_MEDIUM)
                .text_color(t.text_muted)
                .child(group),
        );
        for (ico, name) in pages {
            let active = shown && s.settings_page == name;
            let key = key.clone();
            nav = nav.child(
                div()
                    .id(SharedString::from(match (&key, shown) {
                        (Some(k), false) => format!("set-{k}-{name}"),
                        _ => format!("set-{name}"),
                    }))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .h(CONTROL_MD)
                    .px(px(8.))
                    .rounded(RADIUS_MD)
                    .cursor_pointer()
                    .text_color(if active { t.accent_text } else { t.text_muted })
                    .when(active, |d| d.bg(t.accent_soft))
                    .hover(|d| d.bg(t.surface_hover))
                    .child(icon(
                        ico,
                        16.,
                        if active { t.accent_text } else { t.text_muted },
                    ))
                    .child(name)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(k) = &key {
                            this.activate_org(k);
                        }
                        if let Some(s) = this.session() {
                            s.settings_page = name;
                        }
                        cx.notify();
                    }))
                    .test_support(),
            );
        }
    }
    // Mockup 32c: another organization, joined or created from here.
    if s.org.is_some() {
        let active = s.settings_page == "Add an organization";
        nav = nav.child(div().h(px(12.))).child(
            div()
                .id("set-add-org")
                .flex()
                .items_center()
                .gap(px(8.))
                .h(CONTROL_MD)
                .px(px(8.))
                .rounded(RADIUS_MD)
                .cursor_pointer()
                .text_color(if active { t.accent_text } else { t.text_muted })
                .when(active, |d| d.bg(t.accent_soft))
                .hover(|d| d.bg(t.surface_hover))
                .tooltip(|window, cx| {
                    gpui_kit::component::tooltip::Tooltip::new(
                        "Join or create another organization",
                    )
                    .build(window, cx)
                })
                .child(icon("plus", 16., t.text_muted))
                .child("Add an organization")
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(s) = this.session() {
                        s.settings_page = "Add an organization";
                    }
                    cx.notify();
                }))
                .test_support(),
        );
    }
    // D-67: the organization's backups need the Backups right.
    let manage = s
        .org
        .as_ref()
        .is_some_and(|o| o.can(cleis_core::org::Perm::Backups));
    let body = match s.settings_page {
        "Autofill" => autofill(s, af, t, cx),
        "Import & export" => super::import::page(s, t, cx),
        "Security" => super::security::page(s, t, cx),
        "Sync & backups" if manage => {
            let org_view = s.org.as_ref().is_some_and(|o| o.ui.org_backups);
            let name = s.org.as_ref().map(|o| o.m.name.clone()).unwrap_or_default();
            let pick: super::org::Pick = std::rc::Rc::new(|app, i, cx| {
                if let Some(o) = super::org::org_mut(app) {
                    o.ui.org_backups = i == 1;
                }
                cx.notify();
            });
            // Audit U-15: the same head in both views: title, then the choice, then the action.
            let whose = div()
                .flex()
                .items_center()
                .gap(px(12.))
                .min_h(px(32.))
                .child(
                    div()
                        .text_size(TEXT_XL)
                        .font_weight(W_STRONG)
                        .child("Sync & backups"),
                )
                .child(super::org::segment(
                    "whose",
                    t,
                    &["Mine", &name],
                    usize::from(org_view),
                    pick,
                    cx,
                ))
                .child(div().flex_1())
                .when(org_view, |d| {
                    d.child(
                        button("org-backup-now", t, BtnKind::Primary, false)
                            .child(icon("upload", 16., t.primary_ink))
                            .child("Back up now")
                            .on_click(cx.listener(|this, _, _, cx| this.org_backup_now(cx)))
                            .test_support(),
                    )
                })
                .when(!org_view && s.sync.place.is_some(), |d| {
                    d.child(
                        button("sync-now", t, BtnKind::Primary, false)
                            .child(icon("sync", 16., t.primary_ink))
                            .child("Sync now")
                            .on_click(cx.listener(|this, _, _, cx| this.sync_now(cx)))
                            .test_support(),
                    )
                });
            let inner = if org_view {
                super::org::backups(s, path, t, cx)
            } else {
                super::sync::page_with(s, t, s.sync_menu, false, cx)
            };
            div()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(whose)
                .child(inner)
        }
        "Sync & backups" => super::sync::page(s, t, s.sync_menu, cx),
        "Members" => super::org::members(s, t, cx),
        "Spaces" => super::org::spaces(s, t, cx),
        "Profile" => super::org::profile(s, t, cx),
        "Devices" => super::org::devices(s, t, cx),
        "Security policy" => super::org::policy(s, t, cx),
        "Encryption" => super::org::encryption(s, t, cx),
        "Server" | "Organization" => super::org::server(s, t, cx),
        "Add an organization" => super::org::join_page(s, t, cx),
        _ => preferences(s, &s.prefs, s.pref_menu, t, cx),
    };
    div().flex_1().min_h_0().flex().child(nav).child(
        div()
            .id("settings-body")
            .flex_1()
            .overflow_y_scroll()
            .px(px(32.))
            .pt(px(24.))
            .pb(px(40.))
            .child(
                div()
                    .max_w(px(960.))
                    .mx_auto()
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .child(body),
            ),
    )
}

fn toggle(
    id: &'static str,
    t: &Tokens,
    on: bool,
    f: fn(&mut Preferences),
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    switch(id, t, on)
        .on_click(cx.listener(move |this, _, _, cx| this.update_prefs(f, cx)))
        .test_support()
}

/// Mockup 32c: the personal vault in its own tab, and its name (D-69).
fn vaults_section(s: &Session, p: &Preferences, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let tip = |text: &'static str| {
        move |window: &mut gpui_kit::Window, cx: &mut gpui_kit::App| {
            gpui_kit::component::tooltip::Tooltip::new(text).build(window, cx)
        }
    };
    section(t)
        .child(section_title(t, "Vaults"))
        .child(
            row(t, true)
                .child(
                    div()
                        .id("vault-tabs-label")
                        .flex_1()
                        .tooltip(tip("Off: one Vault tab, every vault in its sidebar"))
                        .child("Personal vault in its own tab"),
                )
                .child(toggle(
                    "vault-tabs",
                    t,
                    p.vault_tabs,
                    |p| p.vault_tabs = !p.vault_tabs,
                    cx,
                )),
        )
        .child(
            row(t, false)
                .child(div().flex_1().child("Personal vault name"))
                .child(
                    div()
                        .w(px(200.))
                        .when_some(s.vault_name.as_ref(), |d, (i, _)| {
                            d.child(gpui_kit::component::input::Input::new(i).id("vault-name"))
                        }),
                ),
        )
}

fn preferences(
    s: &Session,
    p: &Preferences,
    menu_open: Option<&'static str>,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> Div {
    let theme_ix = match p.theme {
        Theme::Dark => 0,
        Theme::Light => 1,
        Theme::System => 2,
    };
    let mut seg = segmented("theme", t, &["Dark", "Light", "System"], theme_ix);
    // Make each option clickable (segmented() lays them out; we attach handlers here).
    seg = div().child(seg);
    let theme_btns = div().flex().gap(px(2.)).children(
        [Theme::Dark, Theme::Light, Theme::System]
            .into_iter()
            .enumerate()
            .map(|(i, th)| {
                let label = ["Dark", "Light", "System"][i];
                let on = i == theme_ix;
                div()
                    .id(SharedString::from(format!("theme-{label}")))
                    .h(px(22.))
                    .px(px(12.))
                    .flex()
                    .items_center()
                    .rounded(RADIUS_SM)
                    .font_weight(W_MEDIUM)
                    .cursor_pointer()
                    .when(on, |d| d.bg(t.accent_soft).text_color(t.accent_text))
                    .when(!on, |d| d.text_color(t.text_muted))
                    .child(label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.update_prefs(move |p| p.theme = th, cx);
                    }))
            }),
    );
    let _ = seg;
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            div()
                .text_size(TEXT_XL)
                .font_weight(W_STRONG)
                .child("Preferences"),
        )
        .child(vaults_section(s, p, t, cx))
        .child(
            section(t)
                .child(section_title(t, "Appearance"))
                .child(
                    row(t, true).child(div().flex_1().child("Theme")).child(
                        div()
                            .p(px(2.))
                            .bg(t.bg)
                            .border_1()
                            .border_color(t.line_strong)
                            .rounded(RADIUS_MD)
                            .child(theme_btns),
                    ),
                )
                .child(
                    row(t, false)
                        .child(div().flex_1().child("Accent color"))
                        .child(swatches(p, t, cx)),
                )
                .child(
                    row(t, false)
                        .child(div().flex_1().child("Accent scope"))
                        .child(scope_buttons(p, t, cx)),
                ),
        )
        .child(
            section(t)
                .child(section_title(t, "Tabs"))
                .child(
                    row(t, true)
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(icon("users", 16., t.text_muted))
                                .child("Contacts"),
                        )
                        .child(toggle(
                            "tab-contacts",
                            t,
                            p.tabs.contacts,
                            |p| p.tabs.contacts = !p.tabs.contacts,
                            cx,
                        )),
                )
                .child(
                    row(t, false)
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(icon("shield", 16., t.text_muted))
                                .child("Health"),
                        )
                        .child(toggle(
                            "tab-health",
                            t,
                            p.tabs.health,
                            |p| p.tabs.health = !p.tabs.health,
                            cx,
                        )),
                )
                // Audit L-18 (D-40): Activity can be hidden too, when an organization shows it.
                .when(s.org.is_some(), |d| {
                    d.child(
                        row(t, false)
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(icon("journal", 16., t.text_muted))
                                    .child("Activity"),
                            )
                            .child(toggle(
                                "tab-activity",
                                t,
                                p.tabs.activity,
                                |p| p.tabs.activity = !p.tabs.activity,
                                cx,
                            )),
                    )
                }),
        )
        .child(
            section(t)
                .child(section_title(t, "Language and region"))
                .child(
                    row(t, true).child(div().flex_1().child("Language")).child(
                        div()
                            .id("lang")
                            .child(value_box(t).w(px(200.)).child("English"))
                            .tooltip(|window, cx| {
                                gpui_kit::component::tooltip::Tooltip::new(
                                    "French comes with a later version",
                                )
                                .build(window, cx)
                            }),
                    ),
                )
                .child(
                    row(t, false)
                        .child(div().flex_1().child("Currency"))
                        .child(pref_select(
                            "currency",
                            t,
                            menu_open == Some("currency"),
                            CURRENCIES
                                .iter()
                                .map(|(code, name)| (*code, *name, p.currency == *code))
                                .collect(),
                            |p, v| p.currency = v.to_owned(),
                            cx,
                        )),
                ),
        )
        .child(
            section(t)
                .child(section_title(t, "Startup"))
                .child(
                    row(t, true)
                        .child(div().flex_1().child("Open at login"))
                        .child(toggle(
                            "login",
                            t,
                            p.open_at_login,
                            |p| p.open_at_login = !p.open_at_login,
                            cx,
                        )),
                )
                .child(
                    row(t, false)
                        .child(div().flex_1().child("Start in"))
                        .child(pref_select(
                            "start",
                            t,
                            menu_open == Some("start"),
                            START_IN
                                .iter()
                                .map(|(k, name)| (*k, *name, p.start_in == *k))
                                .collect(),
                            |p, v| p.start_in = v.to_owned(),
                            cx,
                        )),
                ),
        )
}

const CURRENCIES: [(&str, &str); 5] = [
    ("EUR", "Euro (€)"),
    ("USD", "US dollar ($)"),
    ("GBP", "Pound sterling (£)"),
    ("CHF", "Swiss franc (CHF)"),
    ("CAD", "Canadian dollar (CA$)"),
];

const START_IN: [(&str, &str); 3] = [
    ("all", "All items"),
    ("favorites", "Favorites"),
    ("contacts", "Contacts"),
];

/// A select box of the preferences: its menu lists `(value, label, current)`.
fn pref_select(
    id: &'static str,
    t: &Tokens,
    open: bool,
    options: Vec<(&'static str, &'static str, bool)>,
    set: fn(&mut Preferences, &str),
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    let label = options
        .iter()
        .find(|o| o.2)
        .map(|o| o.1)
        .unwrap_or_default();
    let mut menu = menu_box(t).top(px(32.)).right_0().w(px(220.));
    for (value, name, on) in options {
        menu = menu.child(
            menu_item(format!("{id}-{value}"), t, None, name, false)
                .when(on, |d| d.text_color(t.accent_text))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(s) = this.session() {
                        s.pref_menu = None;
                    }
                    this.update_prefs(|p| set(p, value), cx);
                }))
                .test_support(),
        );
    }
    div()
        .relative()
        .child(
            div()
                .id(id)
                .child(
                    value_box(t)
                        .w(px(200.))
                        .cursor_pointer()
                        .child(div().flex_1().child(label))
                        .child(icon("chevron", 14., t.text_muted)),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(s) = this.session() {
                        s.pref_menu = if s.pref_menu == Some(id) {
                            None
                        } else {
                            Some(id)
                        };
                    }
                    cx.notify();
                }))
                .test_support(),
        )
        .when(open, |d| d.child(gpui_kit::deferred(menu)))
}

/// Accent swatches (`.swatches`): "None" then the palette; the ring marks the current one.
fn swatches(p: &Preferences, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let current = p.accent.as_deref().map(str::to_ascii_lowercase);
    let mut d = div().flex().gap(px(6.));
    let items =
        std::iter::once((None, "None")).chain(ACCENTS.iter().map(|&(hex, name)| (Some(hex), name)));
    for (i, (hex, name)) in items.enumerate() {
        let on = current.as_deref() == hex;
        // Shown as the current theme will draw it (the light theme deepens pale accents).
        let fill = hex
            .and_then(parse_hex)
            .map(|c| fit_contrast(c, t.bg.into(), MIN_ACCENT_CONTRAST));
        let tip: SharedString = if i == 1 {
            format!("{name} (default)").into()
        } else {
            name.into()
        };
        let mut sw = div()
            .id(SharedString::from(format!("accent-{name}")))
            .size(px(22.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .border_1()
            .border_color(t.line_strong)
            .cursor_pointer()
            .bg(fill.map(Into::into).unwrap_or(t.surface_active));
        if fill.is_none() {
            sw = sw.child(icon("x", 12., t.text_muted));
        }
        // Ring = 2 px gap in the page colour, then 1.5 px in muted text (`.swatch.on`).
        let ring = div()
            .size(px(29.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .when(on, |d| d.border(px(1.5)).border_color(t.text_muted));
        d = d.child(
            ring.child(
                sw.tooltip(move |window, cx| {
                    gpui_kit::component::tooltip::Tooltip::new(tip.clone()).build(window, cx)
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.update_prefs(move |p| p.accent = hex.map(str::to_owned), cx);
                }))
                .test_support(),
            ),
        );
    }
    d
}

/// Minimal / Standard / Extended, with the charter's tooltips.
fn scope_buttons(p: &Preferences, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let opts = [
        (AccentScope::Minimal, "Minimal", "Focus and window outline"),
        (
            AccentScope::Standard,
            "Standard",
            "Also the main button, switches and selection",
        ),
        (
            AccentScope::Extended,
            "Extended",
            "Also tabs, choices and active rows",
        ),
    ];
    let disabled = p.accent.is_none();
    let mut d = div().flex().gap(px(2.));
    for (scope, label, tip) in opts {
        let on = p.accent_scope == scope;
        d = d.child(
            div()
                .id(SharedString::from(format!("scope-{label}")))
                .h(px(22.))
                .px(px(12.))
                .flex()
                .items_center()
                .rounded(RADIUS_SM)
                .font_weight(W_MEDIUM)
                .cursor_pointer()
                .when(on, |d| d.bg(t.accent_soft).text_color(t.accent_text))
                .when(!on, |d| d.text_color(t.text_muted))
                .child(label)
                .tooltip(move |window, cx| {
                    gpui_kit::component::tooltip::Tooltip::new(tip).build(window, cx)
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.update_prefs(move |p| p.accent_scope = scope, cx);
                }))
                .test_support(),
        );
    }
    div()
        .p(px(2.))
        .bg(t.bg)
        .border_1()
        .border_color(t.line_strong)
        .rounded(RADIUS_MD)
        .when(disabled, |d| d.opacity(0.45))
        .child(d)
}

/// Status chip of a shortcut: registered, refused by the system, or being recorded.
/// One shortcut line of Settings › Autofill.
struct ShortcutRow<'a> {
    id: &'static str,
    label: &'a str,
    tip: Option<&'static str>,
    keys: &'a str,
    on: bool,
    status: Option<&'a Status>,
    recording: bool,
    first: bool,
}

fn shortcut_row(r: ShortcutRow, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let ShortcutRow {
        id,
        label,
        tip,
        keys: keys_str,
        on,
        status,
        recording,
        first,
    } = r;
    let caps: Vec<String> = keys_str.split('+').map(|k| display_key(k.trim())).collect();
    let caps_ref: Vec<&str> = caps.iter().map(String::as_str).collect();
    let mut name = div().flex_1().child(label.to_owned());
    if let Some(tip) = tip {
        name = name.child(
            div()
                .id(SharedString::from(format!("{id}-tip")))
                .absolute()
                .size_full()
                .tooltip(move |window, cx| {
                    gpui_kit::component::tooltip::Tooltip::new(tip).build(window, cx)
                }),
        );
    }
    let state = match status {
        Some(Status::Unavailable(why)) if on => {
            let why: SharedString = why.clone().into();
            Some(
                div()
                    .id(SharedString::from(format!("{id}-status")))
                    .tooltip(move |window, cx| {
                        gpui_kit::component::tooltip::Tooltip::new(why.clone()).build(window, cx)
                    })
                    .child(chip(t, Tone::Warn, "Unavailable", true))
                    .into_any_element(),
            )
        }
        _ => None,
    };
    row(t, first)
        .child(name.relative())
        .when_some(state, |d, s| d.child(s))
        .child(if recording {
            div()
                .text_size(TEXT_SM)
                .text_color(t.accent_text)
                .child("Press the new shortcut…")
                .into_any_element()
        } else {
            keys(t, &caps_ref).into_any_element()
        })
        .child(
            button(
                SharedString::from(format!("{id}-change")),
                t,
                BtnKind::Ghost,
                true,
            )
            .child(if recording { "Cancel" } else { "Change" })
            .on_click(cx.listener(move |this, _, window, cx| {
                if let Some(s) = this.session() {
                    s.recording = if s.recording == Some(id) {
                        None
                    } else {
                        Some(id)
                    };
                    if s.recording.is_some() {
                        let f = s.keys_focus.clone();
                        window.focus(&f, cx);
                    }
                }
                cx.notify();
            }))
            .test_support(),
        )
}

/// `Ctrl` reads as ⌘ on macOS (settings model).
fn display_key(k: &str) -> String {
    if cfg!(target_os = "macos") {
        match k.to_ascii_lowercase().as_str() {
            "ctrl" => return "⌘".into(),
            "alt" => return "⌥".into(),
            "shift" => return "⇧".into(),
            _ => {}
        }
    }
    k.to_owned()
}

/// Turns a key press into the stored form (`Ctrl+Shift+K`); `None` without a modifier, since a
/// system-wide shortcut on a bare key would swallow that key everywhere.
pub fn shortcut_from(ev: &KeyDownEvent) -> Option<String> {
    let k = &ev.keystroke;
    let m = &k.modifiers;
    let key = match k.key.as_str() {
        "space" => "Space".to_owned(),
        "shift" | "control" | "alt" | "platform" | "function" | "escape" => return None,
        s if s.chars().count() == 1 => s.to_uppercase(),
        s if s.starts_with('f') && s[1..].parse::<u8>().is_ok() => s.to_uppercase(),
        _ => return None,
    };
    let mut parts = vec![];
    // Stored with "Ctrl" for ⌘ on macOS (see `Shortcut` in cleis-core).
    if m.control || (cfg!(target_os = "macos") && m.platform) {
        parts.push("Ctrl");
    }
    if m.alt {
        parts.push("Alt");
    }
    if m.shift {
        parts.push("Shift");
    }
    if parts.is_empty() || (parts == ["Shift"]) {
        return None;
    }
    let mut s = parts.join("+");
    s.push('+');
    s.push_str(&key);
    Some(s)
}

fn autofill(s: &Session, af: &AutofillState, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let p = &s.prefs;
    let a = &p.autofill;
    let status = |which: Shortcut| {
        af.status
            .as_ref()
            .and_then(|st| st.iter().find(|(w, _)| *w == which).map(|(_, s)| s))
    };
    let method_btns = div().flex().gap(px(2.)).children(
        [
            (FillMethod::Automatic, "Automatic", "Writes into the field through the system accessibility interface; types the keys when the app does not allow it"),
            (FillMethod::Type, "Type", "Simulates the keyboard"),
            (FillMethod::Paste, "Paste", "Clipboard, cleared right after"),
        ]
        .into_iter()
        .map(|(m, label, tip)| {
            let on = a.method == m;
            div()
                .id(SharedString::from(format!("method-{label}")))
                .h(px(22.))
                .px(px(12.))
                .flex()
                .items_center()
                .rounded(RADIUS_SM)
                .font_weight(W_MEDIUM)
                .cursor_pointer()
                .when(on, |d| d.bg(t.accent_soft).text_color(t.accent_text))
                .when(!on, |d| d.text_color(t.text_muted))
                .child(label)
                .tooltip(move |window, cx| {
                    gpui_kit::component::tooltip::Tooltip::new(tip).build(window, cx)
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.update_prefs(move |p| p.autofill.method = m, cx);
                }))
                .test_support()
        }),
    );
    let speed_btns = div().flex().gap(px(2.)).children(
        [
            (TypingSpeed::Slow, "Slow"),
            (TypingSpeed::Normal, "Normal"),
            (TypingSpeed::Fast, "Fast"),
        ]
        .into_iter()
        .map(|(v, label)| {
            let on = a.typing_speed == v;
            div()
                .id(SharedString::from(format!("speed-{label}")))
                .h(px(22.))
                .px(px(12.))
                .flex()
                .items_center()
                .rounded(RADIUS_SM)
                .font_weight(W_MEDIUM)
                .cursor_pointer()
                .when(on, |d| d.bg(t.accent_soft).text_color(t.accent_text))
                .when(!on, |d| d.text_color(t.text_muted))
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.update_prefs(move |p| p.autofill.typing_speed = v, cx);
                }))
                .test_support()
        }),
    );
    let seg = |inner: Div| {
        div()
            .p(px(2.))
            .bg(t.bg)
            .border_1()
            .border_color(t.line_strong)
            .rounded(RADIUS_MD)
            .child(inner)
    };

    // Linked apps (screen 16): learned at the first fill from Quick Access.
    let space_name = |id| {
        s.spaces
            .iter()
            .find(|sp| sp.id == id)
            .map(|sp| sp.name.clone())
            .unwrap_or_default()
    };
    let mut links: Vec<(String, String, String, String, cleis_core::EntryId)> = s
        .entries
        .iter()
        .flat_map(|e| {
            e.linked_apps.iter().map(move |l| {
                (
                    l.display_name.clone(),
                    l.program.clone(),
                    e.name.clone(),
                    space_name(e.space),
                    e.id,
                )
            })
        })
        .collect();
    links.sort_by_key(|l| cleis_core::text::fold(&l.0));
    let mut table = div().flex().flex_col();
    let head = |label: &str, w: Option<f32>| {
        let d = div()
            .text_size(TEXT_XS)
            .font_weight(W_MEDIUM)
            .text_color(t.text_muted)
            .child(label.to_owned());
        match w {
            Some(w) => d.w(px(w)).flex_none(),
            None => d.flex_1().min_w_0(),
        }
    };
    table = table.child(
        div()
            .flex()
            .items_center()
            .h(px(28.))
            .gap(px(12.))
            .border_b_1()
            .border_color(t.line)
            .child(head("App", None))
            .child(head("Program", Some(170.)))
            .child(head("Item", Some(170.)))
            .child(head("Space", Some(120.)))
            .child(div().w(px(28.)).flex_none()),
    );
    for (i, (app, program, item, space, id)) in links.into_iter().enumerate() {
        let letter = app.chars().next().unwrap_or('?');
        let prog = program.clone();
        table = table.child(
            div()
                .flex()
                .items_center()
                .h(px(36.))
                .gap(px(12.))
                .border_b_1()
                .border_color(t.line)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(site_icon(t, letter, 20.))
                        .child(app),
                )
                .child(
                    div()
                        .w(px(170.))
                        .flex_none()
                        .font_family(FONT_MONO)
                        .text_size(TEXT_SM)
                        .text_color(t.text_muted)
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(program),
                )
                .child(div().w(px(170.)).flex_none().child(item))
                .child(
                    div()
                        .w(px(120.))
                        .flex_none()
                        .text_color(t.text_muted)
                        .child(space),
                )
                .child(
                    icon_button(SharedString::from(format!("unlink-{i}")), t, "x", true)
                        .tooltip(|window, cx| {
                            gpui_kit::component::tooltip::Tooltip::new("Unlink").build(window, cx)
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.unlink_app(id, prog.clone(), cx);
                        }))
                        .test_support(),
                ),
        );
    }
    let empty_links = s.entries.iter().all(|e| e.linked_apps.is_empty());

    let recording = s.recording;
    div()
        .track_focus(&s.keys_focus)
        .flex()
        .flex_col()
        .gap(px(16.))
        .when(recording.is_some(), |d| {
            d.capture_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                let Some(s) = this.session() else { return };
                let Some(which) = s.recording else { return };
                if ev.keystroke.key == "escape" {
                    s.recording = None;
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                let Some(keys) = shortcut_from(ev) else { return };
                s.recording = None;
                cx.stop_propagation();
                this.update_prefs(
                    move |p| {
                        if which == "qa" {
                            p.autofill.quick_access.keys = keys;
                        } else {
                            p.autofill.fill_best_match.keys = keys;
                        }
                    },
                    cx,
                );
            }))
        })
        .child(
            div()
                .flex()
                .items_center()
                .child(
                    div()
                        .flex_1()
                        .text_size(TEXT_XL)
                        .font_weight(W_STRONG)
                        .child("Autofill"),
                )
                .child(toggle(
                    "af-on",
                    t,
                    a.enabled,
                    |p| p.autofill.enabled = !p.autofill.enabled,
                    cx,
                )),
        )
        .child(
            section(t)
                .when(!a.enabled, |d| d.opacity(0.45))
                .child(section_title(t, "Shortcuts"))
                .child(
                    shortcut_row(
                        ShortcutRow {
                            id: "qa",
                            label: "Quick Access",
                            tip: None,
                            keys: &a.quick_access.keys,
                            on: a.enabled && a.quick_access.enabled,
                            status: status(Shortcut::QuickAccess),
                            recording: recording == Some("qa"),
                            first: true,
                        },
                        t,
                        cx,
                    )
                    .child(toggle(
                        "af-qa",
                        t,
                        a.quick_access.enabled,
                        |p| p.autofill.quick_access.enabled = !p.autofill.quick_access.enabled,
                        cx,
                    )),
                )
                .child(
                    shortcut_row(
                        ShortcutRow {
                            id: "best",
                            label: "Fill best match",
                            tip: Some(
                                "Fills the item linked to the window in front, without opening Quick Access",
                            ),
                            keys: &a.fill_best_match.keys,
                            on: a.enabled && a.fill_best_match.enabled,
                            status: status(Shortcut::FillBestMatch),
                            recording: recording == Some("best"),
                            first: false,
                        },
                        t,
                        cx,
                    )
                    .child(toggle(
                        "af-best",
                        t,
                        a.fill_best_match.enabled,
                        |p| {
                            p.autofill.fill_best_match.enabled =
                                !p.autofill.fill_best_match.enabled
                        },
                        cx,
                    )),
                ),
        )
        .child(
            section(t)
                .when(!a.enabled, |d| d.opacity(0.45))
                .child(section_title(t, "Filling"))
                .child(
                    row(t, true)
                        .child(div().flex_1().child("Method"))
                        .child(seg(method_btns)),
                )
                .child(
                    row(t, false)
                        .child(div().flex_1().child("Then fill the 2FA code"))
                        .child(toggle(
                            "af-totp",
                            t,
                            a.fill_totp_after,
                            |p| p.autofill.fill_totp_after = !p.autofill.fill_totp_after,
                            cx,
                        )),
                )
                .child(
                    row(t, false)
                        .child(div().flex_1().child("Press Enter after filling"))
                        .child(toggle(
                            "af-enter",
                            t,
                            a.press_enter,
                            |p| p.autofill.press_enter = !p.autofill.press_enter,
                            cx,
                        )),
                )
                .child(
                    row(t, false)
                        .child(div().flex_1().child("Typing speed"))
                        .child(seg(speed_btns)),
                ),
        )
        .child(
            section(t)
                .when(!a.enabled, |d| d.opacity(0.45))
                .child(section_title(t, "Detection"))
                .child(
                    row(t, true)
                        .child(div().flex_1().child("Match websites in browsers"))
                        .child(toggle(
                            "af-url",
                            t,
                            a.match_browser_urls,
                            |p| p.autofill.match_browser_urls = !p.autofill.match_browser_urls,
                            cx,
                        )),
                )
                .child(
                    row(t, false)
                        .child(
                            div()
                                .flex_1()
                                .child("Ask before filling an app that is not linked"),
                        )
                        .child(toggle(
                            "af-ask",
                            t,
                            a.ask_unlinked,
                            |p| p.autofill.ask_unlinked = !p.autofill.ask_unlinked,
                            cx,
                        )),
                ),
        )
        .child(
            section(t)
                .child(section_title(t, "Linked apps"))
                .when(empty_links, |d| {
                    d.child(
                        div()
                            .py(px(8.))
                            .text_color(t.text_muted)
                            .child("None yet"),
                    )
                })
                .when(!empty_links, |d| d.child(table)),
        )
        // Paused (D-38): shown greyed, as in the mockup (screen 19).
        .child({
            let br = |name: &str, first: bool| {
                row(t, first)
                    .child(div().flex_1().child(name.to_owned()))
                    .child(chip(t, Tone::Neutral, "Not installed", false))
            };
            div()
                .id("ext-section")
                .opacity(0.45)
                .tooltip(|window, cx| {
                    gpui_kit::component::tooltip::Tooltip::new("Paused for now").build(window, cx)
                })
                .child(
                    section(t)
                        .child(section_title(t, "Browser extension"))
                        .child(br("Chrome", true))
                        .child(br("Edge", false))
                        .child(br("Firefox", false))
                        .child(br("Safari", false)),
                )
        })
        .when(cfg!(target_os = "macos"), |d| {
            let allowed = cleis_autofill::accessibility_allowed();
            d.child(
                section(t).child(section_title(t, "System permissions")).child(
                    row(t, true)
                        .child(div().flex_1().child("Accessibility"))
                        .child(if allowed {
                            chip(t, Tone::Ok, "Allowed", true).into_any_element()
                        } else {
                            button("ax-allow", t, BtnKind::Default, true)
                                .child("Allow")
                                .on_click(cx.listener(|_, _, _, cx| {
                                    cleis_autofill::request_accessibility();
                                    cx.notify();
                                }))
                                .into_any_element()
                        }),
                ),
            )
        })
}

impl CleisApp {
    /// Renames the personal vault (its first space); an empty name puts the current one back.
    pub fn rename_personal(&mut self, name: &str, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let home = s.vault.home_space();
        if name.is_empty() || name == s.personal_name() {
            return;
        }
        match s.vault.rename_space(home, name) {
            Ok(()) => {
                let _ = crate::sync::reload(s);
                s.toast = Some("Renamed".into());
            }
            Err(e) => s.toast = Some(format!("Not renamed: {e}").into()),
        }
        cx.notify();
    }

    pub fn update_prefs(&mut self, f: impl FnOnce(&mut Preferences), cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        f(&mut s.prefs);
        // D-69: the vault tabs come and go with their setting.
        let scope = match (s.prefs.vault_tabs, &s.scope) {
            (false, _) => crate::app::Scope::All,
            (true, crate::app::Scope::All) => crate::app::Scope::Personal,
            (true, sc) => sc.clone(),
        };
        if s.scope != scope {
            s.scope = scope;
            s.refilter();
        }
        if !s.prefs.tabs.activity && s.tab == crate::app::Tab::Activity {
            s.tab = crate::app::Tab::Vault;
        }
        if let Err(e) = s.vault.set_preferences(&s.prefs) {
            s.toast = Some(format!("Not saved: {e}").into());
        }
        let prefs = s.prefs.clone();
        self.apply_appearance(&prefs, cx);
        self.sync_open_at_login();
        if !self.no_shortcuts {
            self.sync_autofill(cx);
        }
    }
}
