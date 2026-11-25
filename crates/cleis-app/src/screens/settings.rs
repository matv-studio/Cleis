//! Settings (screens 06, 15, 16): personal pages. Organisation pages come with the server.

use crate::app::{CleisApp, Session};
use crate::theme::*;
use crate::ui::*;
use cleis_core::settings::{ACCENTS, AccentScope, FillMethod, Preferences, Theme};
use gpui_kit::TestSupportExt;
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, SharedString, div, px};

const PAGES: [(&str, &str); 4] = [
    ("lock", "Security"),
    ("gear", "Preferences"),
    ("fill", "Autofill"),
    ("sync", "Sync & backups"),
];

pub fn page(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
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
    nav = nav.child(
        div()
            .px(px(8.))
            .pb(px(4.))
            .text_size(TEXT_XS)
            .font_weight(W_MEDIUM)
            .text_color(t.text_muted)
            .child("My account"),
    );
    for (ico, name) in PAGES {
        let active = s.settings_page == name;
        nav = nav.child(
            div()
                .id(SharedString::from(format!("set-{name}")))
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
                    if let Some(s) = this.session() {
                        s.settings_page = name;
                    }
                    cx.notify();
                })),
        );
    }
    let body = match s.settings_page {
        "Autofill" => autofill(&s.prefs, t, cx),
        "Security" => security(&s.prefs, t, cx),
        "Sync & backups" => div()
            .child(
                div()
                    .text_size(TEXT_XL)
                    .font_weight(W_STRONG)
                    .child("Sync & backups"),
            )
            .child(
                section(t).mt(px(16.)).opacity(0.45).child(
                    row(t, true)
                        .child(div().flex_1().child("Encrypted export"))
                        .child(button("export", t, BtnKind::Default, false).child("Export")),
                ),
            ),
        _ => preferences(&s.prefs, t, cx),
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

fn preferences(p: &Preferences, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
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
                ),
        )
        .child(
            section(t)
                .child(section_title(t, "Language and region"))
                .child(
                    row(t, true)
                        .child(div().flex_1().child("Language"))
                        .child(value_box(t).w(px(200.)).child("English")),
                )
                .child(
                    row(t, false)
                        .child(div().flex_1().child("Currency"))
                        .child(value_box(t).w(px(200.)).child("Euro (€)")),
                ),
        )
        .child(
            section(t).child(section_title(t, "Startup")).child(
                row(t, true)
                    .child(div().flex_1().child("Open at login"))
                    .child(toggle(
                        "login",
                        t,
                        p.open_at_login,
                        |p| p.open_at_login = !p.open_at_login,
                        cx,
                    )),
            ),
        )
}

/// Accent swatches (`.swatches`): "None" then the palette; the ring marks the current one.
fn swatches(p: &Preferences, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let current = p.accent.as_deref().map(str::to_ascii_lowercase);
    let mut d = div().flex().gap(px(6.));
    let items =
        std::iter::once((None, "None")).chain(ACCENTS.iter().map(|&(hex, name)| (Some(hex), name)));
    for (i, (hex, name)) in items.enumerate() {
        let on = current.as_deref() == hex;
        let fill = hex.and_then(parse_hex);
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

fn security(p: &Preferences, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let secs = p.clipboard_clear_secs;
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            div()
                .text_size(TEXT_XL)
                .font_weight(W_STRONG)
                .child("Security"),
        )
        .child(
            section(t).child(section_title(t, "Clipboard")).child(
                row(t, true)
                    .child(div().flex_1().child("Clear after copying"))
                    .child(
                        value_box(t).w(px(120.)).child(
                            secs.map(|s| format!("{s} s"))
                                .unwrap_or_else(|| "Never".into()),
                        ),
                    )
                    .child(toggle(
                        "clip",
                        t,
                        secs.is_some(),
                        |p| {
                            p.clipboard_clear_secs = if p.clipboard_clear_secs.is_some() {
                                None
                            } else {
                                Some(30)
                            }
                        },
                        cx,
                    )),
            ),
        )
        .child(
            section(t).child(section_title(t, "Master password")).child(
                row(t, true)
                    .child(
                        div()
                            .flex_1()
                            .text_color(t.text_muted)
                            .child("Stored nowhere; derived with Argon2id"),
                    )
                    .child(
                        button("chpw", t, BtnKind::Default, false)
                            .opacity(0.45)
                            .child("Change"),
                    ),
            ),
        )
}

fn autofill(p: &Preferences, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let a = &p.autofill;
    let method = match a.method {
        FillMethod::Automatic => 0,
        FillMethod::Type => 1,
        FillMethod::Paste => 2,
    };
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
                    row(t, true)
                        .child(div().flex_1().child("Quick Access"))
                        .child(keys(t, &["Ctrl", "Shift", "Space"]))
                        .child(toggle(
                            "af-qa",
                            t,
                            a.quick_access.enabled,
                            |p| p.autofill.quick_access.enabled = !p.autofill.quick_access.enabled,
                            cx,
                        )),
                )
                .child(
                    row(t, false)
                        .child(div().flex_1().child("Fill best match"))
                        .child(keys(t, &["Ctrl", "Alt", "A"]))
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
                        .child(segmented(
                            "method",
                            t,
                            &["Automatic", "Type", "Paste"],
                            method,
                        )),
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
}

impl CleisApp {
    pub fn update_prefs(&mut self, f: impl FnOnce(&mut Preferences), cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        f(&mut s.prefs);
        if let Err(e) = s.vault.set_preferences(&s.prefs) {
            s.toast = Some(format!("Not saved: {e}").into());
        }
        let prefs = s.prefs.clone();
        self.apply_appearance(&prefs, cx);
    }
}
