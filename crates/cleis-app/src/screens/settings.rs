//! Settings (screens 06, 07–10, 13, 15, 16): pages of the account, then of the organization.

use crate::app::{CleisApp, Session};
use crate::autofill::AutofillState;
use crate::i18n::tr;
use crate::theme::*;
use crate::ui::*;
use cleis_core::settings::TypingSpeed;
use cleis_core::settings::{ACCENTS, AccentScope, FillMethod, Preferences, Theme};
use gpui_kit::KeyDownEvent;
use gpui_kit::TestSupportExt;
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, SharedString, div, px};

// Account holds what Security and Profile were (27/11/2026, mockups 39a, 39b).
const ACCOUNT: [(&str, &str); 5] = [
    ("cloud", "Account"),
    ("gear", "Preferences"),
    ("fill", "Autofill"),
    ("sync", "Sync & backups"),
    // Mockup 38: every keyboard shortcut, the system-wide ones included.
    ("keyboard", "Shortcuts"),
];

/// Pages of the navigation: `(group, organization key, [(icon, name)])`. Without an
/// organization, Import & export stays under My account with a page to join one (D-54); with
/// several, each has its group (D-69).
type Group = (String, Option<String>, Vec<(&'static str, &'static str)>);

fn groups(s: &Session) -> Vec<Group> {
    let mut account: Vec<_> = ACCOUNT.to_vec();
    // D-99: the personal vault's Import & export stays here; each organization has its own.
    account.push(("upload", "Import & export"));
    if s.org.is_none() {
        account.push(("server", "Organization"));
        return vec![(tr("My account").into(), None, account)];
    }
    account.insert(1, ("laptop", "Devices"));
    let mut out = vec![(tr("My account").into(), None, account)];
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
        org.extend([("server", "Server"), ("upload", super::import::ORG_PAGE)]);
        out.push((o.m.name.clone(), Some(o.m.key()), org));
    }
    out
}

pub fn page(
    s: &Session,
    af: &AutofillState,
    path: &std::path::Path,
    hosts: &crate::cloud::CloudHosts,
    updates: Option<Div>,
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
                    .child(crate::i18n::tr_owned(if name == super::import::ORG_PAGE {
                        "Import & export"
                    } else {
                        name
                    }))
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
                .child(tr("Add an organization"))
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
        "Autofill" => autofill(s, t, cx),
        "Shortcuts" => super::keys::page(s, af, t, cx),
        "Import & export" => super::import::page(s, t, cx),
        p if p == super::import::ORG_PAGE => super::import::page(s, t, cx),
        "Security" | "Profile" => super::account::page(s, s.account_pick, hosts, t, cx),
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
                        .line_height(px(26.))
                        .font_weight(W_STRONG)
                        .child(tr("Sync & backups")),
                )
                .child(super::org::segment(
                    "whose",
                    t,
                    &[tr("Mine"), &name],
                    usize::from(org_view),
                    pick,
                    cx,
                ))
                .child(div().flex_1())
                .when(org_view, |d| {
                    d.child(
                        button("org-backup-now", t, BtnKind::Primary, false)
                            .child(icon("upload", 16., t.primary_ink))
                            .child(tr("Back up now"))
                            .on_click(cx.listener(|this, _, _, cx| this.org_backup_now(cx)))
                            .test_support(),
                    )
                })
                .when(!org_view && s.sync.place.is_some(), |d| {
                    d.child(
                        button("sync-now", t, BtnKind::Primary, false)
                            .child(icon("sync", 16., t.primary_ink))
                            .child(tr("Sync now"))
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
        "Account" => super::account::page(s, s.account_pick, hosts, t, cx),
        "Members" => super::org::members(s, t, cx),
        "Spaces" => super::org::spaces(s, t, cx),
        "Devices" => super::org::devices(s, t, cx),
        "Security policy" => super::org::policy(s, t, cx),
        "Encryption" => super::org::encryption(s, t, cx),
        "Server" | "Organization" => super::org::server(s, t, cx),
        "Add an organization" => super::org::join_page(s, t, cx),
        _ => preferences(s, &s.prefs, s.pref_menu, t, cx).children(updates),
    };
    div().flex_1().min_h_0().flex().child(nav).child(
        div()
            .id("settings-body")
            .flex_1()
            .overflow_y_scroll()
            .px(px(32.))
            .pt(px(24.))
            .pb(px(40.))
            .child(crate::ui::page_in(
                format!("settings-{}", s.settings_page),
                div()
                    .max_w(px(960.))
                    .mx_auto()
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .child(body),
            )),
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
            gpui_kit::component::tooltip::Tooltip::new(crate::i18n::tr_dynamic(text))
                .build(window, cx)
        }
    };
    let has_org = s.all_orgs().next().is_some();
    section(t)
        .child(section_title(t, tr("Vaults")))
        // Without an organization there is one vault: no tab of its own to offer.
        .when(has_org, |d| {
            d.child(
                row(t, true)
                    .child(
                        div()
                            .id("vault-tabs-label")
                            .flex_1()
                            .tooltip(tip(tr("Off: one Vault tab, every vault in its sidebar")))
                            .child(tr("Personal vault in its own tab")),
                    )
                    .child(toggle(
                        "vault-tabs",
                        t,
                        p.vault_tabs,
                        |p| p.vault_tabs = !p.vault_tabs,
                        cx,
                    )),
            )
        })
        .child(shortcut_rows(s, p, t, cx))
        .child(
            row(t, false)
                .child(div().flex_1().child(tr("Personal vault name")))
                .child(
                    div()
                        .w(px(200.))
                        .when_some(s.vault_name.as_ref(), |d, (i, _)| {
                            d.child(gpui_kit::component::input::Input::new(i).id("vault-name"))
                        }),
                ),
        )
}

/// Mockups 45a, 45b (27/11/2026): the sidebar's shortcuts as a grid, one row per shortcut, one
/// column per vault (or every vault), a check box in each cell; Contacts' below, the same way.
fn shortcut_rows(s: &Session, p: &Preferences, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    use crate::screens::org::check_box;
    const COL: f32 = 128.;
    let tabs = s.vault_tabs();
    let per_vault = p.shortcuts.per_vault(tabs);
    let orgs = s.org_tabs();
    let mut vaults: Vec<(String, &'static str, String)> = Vec::new();
    if per_vault {
        vaults.push(("personal".into(), "user", s.personal_name()));
        for (k, name) in orgs.iter().cloned() {
            vaults.push((k, "users", name));
        }
    } else {
        vaults.push(("all".into(), "list", tr("Every vault").to_owned()));
    }
    // One Vault tab, a set per vault (37i): All items is the vault's own name there.
    let first = usize::from(per_vault && !tabs);
    let head = |label: &str, cols: Vec<(Option<&'static str>, String)>| {
        let mut h = div()
            .h(px(32.))
            .flex()
            .items_center()
            .border_b_1()
            .border_color(t.line)
            .text_size(TEXT_XS)
            .font_weight(W_MEDIUM)
            .text_color(t.text_muted)
            .child(div().flex_1().child(crate::i18n::tr_owned(label)));
        for (ico, name) in cols {
            h = h.child(
                div()
                    .w(px(COL))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.))
                    .overflow_hidden()
                    .when_some(ico, |d, i| d.child(icon(i, 14., t.text_muted)))
                    .child(
                        div()
                            .min_w_0()
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .child(name),
                    ),
            );
        }
        h
    };
    let line = |ico: &'static str, label: &str, tip_text: Option<&'static str>| {
        div()
            .h(px(36.))
            .flex()
            .items_center()
            .border_b_1()
            .border_color(t.line)
            .child(
                div()
                    .id(SharedString::from(format!("sc-l-{label}")))
                    .flex_1()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(icon(ico, 16., t.text_muted))
                    .child(crate::i18n::tr_owned(label))
                    .when_some(tip_text, |d, x| d.tooltip(tip(tr(x)))),
            )
    };
    let cell = |id: String, on: bool| {
        div()
            .id(SharedString::from(id))
            .w(px(COL))
            .flex_none()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .child(check_box(t, on))
    };
    let mut grid = div().pl(px(16.)).flex().flex_col().child(head(
        "Vault",
        vaults
            .iter()
            .map(|(_, i, n)| (Some(*i), n.clone()))
            .collect(),
    ));
    for (ix, (ico, label, tip_text)) in [
        ("list", "All items", None),
        ("star", "Favorites", None),
        (
            "clock",
            "Renew soon",
            Some("Subscriptions renewing in the next 30 days"),
        ),
    ]
    .into_iter()
    .enumerate()
    .skip(first)
    {
        let mut r = line(ico, label, tip_text);
        for (key, _, _) in &vaults {
            let k = key.clone();
            r = r.child(
                cell(format!("sc-{key}-{ix}"), p.shortcuts.of(key)[ix])
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let k = k.clone();
                        this.update_prefs(|p| p.shortcuts.toggle(&k, ix), cx);
                        if let Some(s) = this.session() {
                            s.refilter();
                        }
                    }))
                    .test_support(),
            );
        }
        grid = grid.child(r);
    }
    grid = grid
        .child(div().h(px(12.)))
        .child(head("Contacts", vec![(None, tr("Shown").into())]));
    for (ix, (ico, label, tip_text)) in [
        ("users", "All contacts", None),
        ("star", "Favorites", None),
        ("calendar", "Birthdays soon", Some("Next 30 days")),
        ("list", "Uncategorized", Some("Contacts outside any folder")),
    ]
    .into_iter()
    .enumerate()
    {
        grid = grid.child(
            line(ico, label, tip_text).child(
                cell(format!("sc-contacts-{ix}"), p.contact_shortcuts[ix])
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.update_prefs(
                            |p| p.contact_shortcuts[ix] = !p.contact_shortcuts[ix],
                            cx,
                        );
                    }))
                    .test_support(),
            ),
        );
    }
    div()
        .flex()
        .flex_col()
        .child(
            row(t, true)
                .child(
                    div()
                        .id("shortcuts-label")
                        .flex_1()
                        .tooltip(tip(tr("At the top of the sidebar")))
                        .child(tr("Shortcuts")),
                )
                // Mockup 37i: per vault also with one Vault tab, as soon as there is more than one.
                .when(!orgs.is_empty(), |r| {
                    r.child(
                        div()
                            .text_size(TEXT_SM)
                            .text_color(t.text_muted)
                            .child(tr("Same for every vault")),
                    )
                    .child(toggle(
                        "shortcuts-same",
                        t,
                        !per_vault,
                        |p| {
                            let tabs = p.vault_tabs;
                            p.shortcuts.toggle_same(tabs)
                        },
                        cx,
                    ))
                }),
        )
        .child(grid)
}

/// The logo shown in the app: light (default) or dark, whatever the theme (26/11/2026).
fn logo_buttons(p: &Preferences, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    div()
        .p(px(2.))
        .bg(t.bg)
        .border_1()
        .border_color(t.line_strong)
        .rounded(RADIUS_MD)
        .flex()
        .gap(px(2.))
        .children([(false, "Light"), (true, "Dark")].map(|(dark, label)| {
            let on = p.dark_logo == dark;
            div()
                .id(SharedString::from(format!("logo-{label}")))
                .h(px(22.))
                .px(px(8.))
                .flex()
                .items_center()
                .gap(px(6.))
                .rounded(RADIUS_SM)
                .font_weight(W_MEDIUM)
                .cursor_pointer()
                .when(on, |d| d.bg(t.accent_soft).text_color(t.accent_text))
                .when(!on, |d| d.text_color(t.text_muted))
                .child(super::unlock::logo_version(dark, 16.))
                .child(crate::i18n::tr_owned(label))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.update_prefs(move |p| p.dark_logo = dark, cx);
                }))
                .test_support()
        }))
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
    let mut seg = segmented(
        "theme",
        t,
        &[tr("Dark"), tr("Light"), tr("System")],
        theme_ix,
    );
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
                    .child(crate::i18n::tr_owned(label))
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
                .line_height(px(26.))
                .font_weight(W_STRONG)
                .child(tr("Preferences")),
        )
        .child(vaults_section(s, p, t, cx))
        .child(
            section(t)
                .child(section_title(t, tr("Appearance")))
                .child(
                    row(t, true).child(div().flex_1().child(tr("Theme"))).child(
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
                        .child(div().flex_1().child(tr("Logo")))
                        .child(logo_buttons(p, t, cx)),
                )
                .child(
                    row(t, false)
                        .child(div().flex_1().child(tr("Accent color")))
                        .child(swatches(s, p, t, cx)),
                )
                .child(
                    row(t, false)
                        .child(div().flex_1().child(tr("Accent scope")))
                        .child(scope_buttons(p, t, cx)),
                ),
        )
        .child(
            section(t)
                .child(section_title(t, tr("Tabs")))
                .child(
                    row(t, true)
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(icon("users", 16., t.text_muted))
                                .child(tr("Contacts")),
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
                                .child(tr("Health")),
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
                                    .child(tr("Activity")),
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
                .child(section_title(t, tr("Language and region")))
                .child(
                    // Audit U-9: English or French, applied at once.
                    row(t, true)
                        .child(div().flex_1().child(tr("Language")))
                        .child(pref_select(
                            "lang",
                            t,
                            menu_open == Some("lang"),
                            crate::i18n::LANGUAGES
                                .iter()
                                .map(|(tag, name)| (*tag, *name, p.language.starts_with(tag)))
                                .collect(),
                            |p, v| p.language = v.to_owned(),
                            cx,
                        )),
                )
                .child(
                    row(t, false)
                        .child(div().flex_1().child(tr("Currency")))
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
                .child(section_title(t, tr("Startup")))
                .child(
                    row(t, true)
                        .child(div().flex_1().child(tr("Open at login")))
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
                        .child(div().flex_1().child(tr("Start in")))
                        .child(pref_select(
                            "start",
                            t,
                            menu_open == Some("start"),
                            start_options(s, p),
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

/// Start in (mockup 37k): with vaults in their own tabs, each vault tab first, then the views.
fn start_options(s: &Session, p: &Preferences) -> Vec<(String, String, bool)> {
    let mut v = vec![];
    if s.vault_tabs() {
        v.push(("vault:personal".to_owned(), s.personal_name()));
        for (k, name) in s.org_tabs() {
            v.push((format!("vault:{k}"), name));
        }
    }
    let first = v.len();
    let mut out: Vec<(String, String, bool)> = v
        .into_iter()
        .map(|(k, n)| {
            let on = p.start_in == k;
            (k, n, on)
        })
        .collect();
    if first > 0 {
        out.push(("-".into(), String::new(), false));
    }
    for (k, name) in START_IN {
        out.push((k.into(), tr(name).into(), p.start_in == k));
    }
    // A vault tab no longer there (tabs off, organization left): shown as All items.
    if !out.iter().any(|o| o.2)
        && let Some(o) = out.iter_mut().find(|o| o.0 == "all")
    {
        o.2 = true;
    }
    out
}

const START_IN: [(&str, &str); 3] = [
    ("all", "All items"),
    ("favorites", "Favorites"),
    ("contacts", "Contacts"),
];

/// A select box of the preferences: its menu lists `(value, label, current)`; a value `"-"` is a
/// separator line.
fn pref_select<V: Into<SharedString>, L: Into<SharedString>>(
    id: &'static str,
    t: &Tokens,
    open: bool,
    options: Vec<(V, L, bool)>,
    set: fn(&mut Preferences, &str),
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    let options: Vec<(SharedString, SharedString, bool)> = options
        .into_iter()
        .map(|(v, l, on)| (v.into(), l.into(), on))
        .collect();
    let label = options
        .iter()
        .find(|o| o.2)
        .map(|o| o.1.clone())
        .unwrap_or_default();
    let mut menu = menu_box(t).relative().w(px(220.));
    for (value, name, on) in options {
        if value.as_ref() == "-" {
            menu = menu.child(div().h(px(1.)).my(px(4.)).bg(t.line));
            continue;
        }
        menu = menu.child(
            menu_item(format!("{id}-{value}"), t, None, name, false)
                .when(on, |d| d.text_color(t.accent_text))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(s) = this.session() {
                        s.pref_menu = None;
                    }
                    let v = value.clone();
                    this.update_prefs(|p| set(p, &v), cx);
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
                        .child(div().flex_1().child(crate::i18n::tr_owned(&label)))
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
        // Below the box, or above it near the window's bottom (26/11/2026).
        .when(open, |d| d.child(popup_below(menu, px(28.))))
}

/// Accent swatches (`.swatches`): "None" then the palette; the ring marks the current one.
fn swatches(s: &Session, p: &Preferences, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let current = p.accent.as_deref().map(str::to_ascii_lowercase);
    let mut d = div().relative().flex().gap(px(6.));
    let items =
        std::iter::once((None, "None")).chain(ACCENTS.iter().map(|&(hex, name)| (Some(hex), name)));
    for (i, (hex, name)) in items.enumerate() {
        let on = current.as_deref() == hex && s.wheel.is_none();
        // As the buttons will be filled: the colour itself (D-99).
        let fill = hex.and_then(parse_hex);
        let tip: SharedString = if i == 1 {
            crate::trf!("{} (default)", name).into()
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
        // Ring = 2.5 px gap in the page colour, then 1.5 px in muted text (`.swatch.on`). The ring
        // and the swatch differ by an even number of pixels, so the swatch sits at a whole-pixel
        // offset in it: exactly centred at any scale (26/11/2026).
        let ring = div()
            .size(px(30.))
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
    // Your own colour (mockup 36): the last swatch, ringed when it is the accent or the wheel is open.
    use crate::screens::wheel;
    let own = current
        .as_deref()
        .filter(|h| !wheel::in_palette(h))
        .and_then(parse_hex);
    let on = own.is_some() || s.wheel.is_some();
    d = d.child(
        div()
            .size(px(30.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .when(on, |d| d.border(px(1.5)).border_color(t.text_muted))
            .child(
                div()
                    .id("accent-own")
                    .cursor_pointer()
                    .child(wheel::own_swatch(own, t))
                    .tooltip(tip(tr("Your own color")))
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_wheel(cx)))
                    .test_support(),
            ),
    );
    if let Some(w) = s.wheel.as_ref() {
        d = d.child(wheel::render(w, t, cx));
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
                .child(crate::i18n::tr_owned(label))
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

/// Turns a key press into the stored form (`Ctrl+Shift+K`); `None` without a modifier, since a
/// system-wide shortcut on a bare key would swallow that key everywhere.
pub fn shortcut_from(ev: &KeyDownEvent) -> Option<String> {
    let k = &ev.keystroke;
    let m = &k.modifiers;
    let key = match k.key.as_str() {
        "space" => tr("Space").to_owned(),
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

fn autofill(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let p = &s.prefs;
    let a = &p.autofill;
    let method_btns = div().flex().gap(px(2.)).children(
        [
            (FillMethod::Automatic, "Automatic", "Writes into the field through the system accessibility interface; types the keys when the app does not allow it"),
            (FillMethod::Type, "Typing", "Simulates the keyboard"),
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
                .child(crate::i18n::tr_owned(label))
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
                .child(crate::i18n::tr_owned(label))
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
            .map(|sp| crate::i18n::space(&sp.name))
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
            .child(crate::i18n::tr_owned(label));
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
                            gpui_kit::component::tooltip::Tooltip::new(tr("Unlink"))
                                .build(window, cx)
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.unlink_app(id, prog.clone(), cx);
                        }))
                        .test_support(),
                ),
        );
    }
    let empty_links = s.entries.iter().all(|e| e.linked_apps.is_empty());

    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            div()
                .flex()
                .items_center()
                .child(
                    div()
                        .flex_1()
                        .text_size(TEXT_XL)
                        .line_height(px(26.))
                        .font_weight(W_STRONG)
                        .child(tr("Autofill")),
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
                .child(section_title(t, tr("Filling")))
                .child(
                    row(t, true)
                        .child(div().flex_1().child(tr("Method")))
                        .child(seg(method_btns)),
                )
                .child(
                    row(t, false)
                        .child(div().flex_1().child(tr("Then fill the 2FA code")))
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
                        .child(div().flex_1().child(tr("Press Enter after filling")))
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
                        .child(div().flex_1().child(tr("Typing speed")))
                        .child(seg(speed_btns)),
                ),
        )
        .child(
            section(t)
                .when(!a.enabled, |d| d.opacity(0.45))
                .child(section_title(t, tr("Detection")))
                .child(
                    row(t, true)
                        .child(div().flex_1().child(tr("Match websites in browsers")))
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
                                .child(tr("Ask before filling an app that is not linked")),
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
                .child(section_title(t, tr("Linked apps")))
                .when(empty_links, |d| {
                    d.child(
                        div()
                            .py(px(8.))
                            .text_color(t.text_muted)
                            .child(tr("None yet")),
                    )
                })
                .when(!empty_links, |d| d.child(table)),
        )
        // Paused (D-38): shown greyed, as in the mockup (screen 19).
        .when(cfg!(target_os = "macos"), |d| {
            let allowed = cleis_autofill::accessibility_allowed();
            d.child(
                section(t)
                    .child(section_title(t, tr("System permissions")))
                    .child(
                        row(t, true)
                            .child(div().flex_1().child(tr("Accessibility")))
                            .child(if allowed {
                                chip(t, Tone::Ok, tr("Allowed"), true).into_any_element()
                            } else {
                                button("ax-allow", t, BtnKind::Default, true)
                                    .child(tr("Allow"))
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
                s.toast = Some(tr("Renamed").into());
            }
            Err(e) => s.toast = Some(crate::trf!("Not renamed: {}", e).into()),
        }
        cx.notify();
    }

    pub fn update_prefs(&mut self, f: impl FnOnce(&mut Preferences), cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        f(&mut s.prefs);
        // D-69: the vault tabs come and go with their setting.
        let scope = match (s.vault_tabs(), &s.scope) {
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
            s.toast = Some(crate::trf!("Not saved: {}", e).into());
        }
        let prefs = s.prefs.clone();
        self.apply_appearance(&prefs, cx);
        self.sync_open_at_login();
        if !self.no_shortcuts {
            self.sync_autofill(cx);
        }
    }
}
