//! Main window: top bar with centred tabs (charter §5) and the active tab's page.

use super::unlock::logo;
use crate::app::{CleisApp, Filter, Phase, Session, Tab};
use crate::format::{money, subscription_chip};
use crate::theme::*;
use crate::ui::*;
use cleis_core::column::{ColumnDef, ColumnKind};
use cleis_core::totp::Totp;
use cleis_core::{Entry, EntryId};
use gpui_kit::TestSupportExt;
use gpui_kit::component::TitleBar;
use gpui_kit::component::input::Input;
use gpui_kit::prelude::*;
use gpui_kit::{AnyElement, Context, Div, SharedString, Window, div, px, uniform_list};

pub fn render(
    app: &mut CleisApp,
    window: &mut Window,
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    let t = app.tokens(cx).clone();
    app.ensure_editor(window, cx);
    app.ensure_contact_editor(window, cx);
    app.ensure_org_inputs(window, cx);
    let Phase::Main(s) = &app.phase else {
        unreachable!()
    };
    let tab = s.tab;
    let page: AnyElement = match tab {
        Tab::Vault => vault_page(s, &t, window, cx).into_any_element(),
        Tab::Contacts => super::contacts::page(s, &t, cx).into_any_element(),
        Tab::Health => super::health::page(s, &t, cx).into_any_element(),
        Tab::Activity => super::org::activity(s, &t, cx).into_any_element(),
        Tab::Settings => {
            super::settings::page(s, &app.autofill, &app.path, &t, cx).into_any_element()
        }
    };
    let toast = s.toast.clone();
    div()
        .size_full()
        .flex()
        .flex_col_reverse()
        .bg(t.chrome)
        .font_family(FONT)
        .text_size(TEXT_MD)
        .text_color(t.text)
        .track_focus(&s.main_focus)
        .on_key_down(
            cx.listener(|this, ev: &gpui_kit::KeyDownEvent, window, cx| {
                let k = &ev.keystroke;
                let cmd = if cfg!(target_os = "macos") {
                    k.modifiers.platform
                } else {
                    k.modifiers.control
                };
                if !cmd || k.modifiers.alt || k.modifiers.shift {
                    return;
                }
                match k.key.as_str() {
                    // Tooltip of the lock button (mockup 01): "Lock (Ctrl+L)".
                    "l" => {
                        cx.stop_propagation();
                        this.lock(window, cx);
                    }
                    "f" => {
                        cx.stop_propagation();
                        if let Some(s) = this.session() {
                            let search = s.search.clone();
                            s.tab = crate::app::Tab::Vault;
                            search.update(cx, |st, cx| st.focus(window, cx));
                        }
                        cx.notify();
                    }
                    _ => {}
                }
            }),
        )
        // Stage first, top bar last, laid out in reverse: the top bar paints after the stage, so
        // the active tab covers the stage outline and joins it (charter §5).
        .child(
            div()
                .flex_1()
                .min_h_0()
                .mx(px(6.))
                .mb(px(6.))
                .border_1()
                .border_color(t.stage_line)
                .rounded(px(10.))
                .bg(t.bg)
                .overflow_hidden()
                .relative()
                .flex()
                .flex_col()
                .child(page)
                .when_some(toast, |d, msg| {
                    d.child(
                        div()
                            .absolute()
                            .bottom(px(40.))
                            .left_0()
                            .right_0()
                            .flex()
                            .justify_center()
                            .child(
                                div()
                                    .px(px(12.))
                                    .py(px(6.))
                                    .rounded(RADIUS_MD)
                                    .bg(t.inverse)
                                    .text_color(t.inverse_ink)
                                    .text_size(TEXT_SM)
                                    .font_weight(W_MEDIUM)
                                    .child(msg),
                            ),
                    )
                }),
        )
        .child(topbar(s, &t, cx))
        .when_some(s.confirm.as_ref(), |d, c| {
            d.child(super::trash::dialog(c, &t, cx))
        })
        .when_some(s.security.dialog.as_ref(), |d, dlg| {
            d.child(super::security::dialog(s, dlg, &t, cx))
        })
        .when_some(super::org::dialog(s, &t, cx), |d, dlg| d.child(dlg))
        // Any input counts as activity (Security › Locking).
        .on_mouse_move(cx.listener(|this, _, _, _| this.touch_activity()))
        .capture_key_down(cx.listener(|this, _, _, _| this.touch_activity()))
}

fn topbar(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> TitleBar {
    let mut tabs = vec![(Tab::Vault, "Vault")];
    if s.prefs.tabs.contacts {
        tabs.push((Tab::Contacts, "Contacts"));
    }
    if s.prefs.tabs.health {
        tabs.push((Tab::Health, "Health"));
    }
    // Activity exists only for an organisation (D-38): a personal vault has no log.
    if s.org
        .as_ref()
        .is_some_and(|o| o.can(cleis_core::org::Perm::Audit))
    {
        tabs.push((Tab::Activity, "Activity"));
    }
    tabs.push((Tab::Settings, "Settings"));
    let mut nav = div().flex().items_end().h(TOPBAR_H);
    for (tab, label) in tabs {
        let active = s.tab == tab;
        let mut el = div()
            .id(SharedString::from(format!("tab-{label}")))
            .h(px(32.))
            .px(px(16.))
            .flex()
            .items_center()
            .font_weight(W_MEDIUM)
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(s) = this.session() {
                    s.tab = tab;
                }
                if tab == Tab::Activity {
                    this.org_query_log(cx);
                }
                cx.notify();
            }));
        el = if active {
            // The outline is drawn by a frame that stops 9 px above the bottom; the corners take
            // over from there (a parent's own border would paint over its children).
            el.h(px(33.))
                .mb(px(-1.))
                .relative()
                .bg(t.bg)
                .rounded_t(px(8.))
                .text_color(t.accent_text)
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .bottom(px(9.))
                        .border_t_1()
                        .border_l_1()
                        .border_r_1()
                        .border_color(t.stage_line)
                        .rounded_t(px(8.)),
                )
                .child(tab_corner(t, true))
                .child(tab_corner(t, false))
        } else {
            el.text_color(t.text_muted)
        };
        nav = nav.child(el.child(label).test_support());
    }
    let primary = match s.tab {
        Tab::Vault => Some(("new-item", "New item")),
        Tab::Contacts => Some(("new-contact", "New contact")),
        _ => None,
    };
    // The top bar is the window's title bar (charter §5: the frame is the top bar): it moves the
    // window and carries the system's window buttons (Windows, Linux) or leaves room for the
    // traffic lights (macOS).
    let bar = div()
        .flex_1()
        .h_full()
        .flex()
        .items_end()
        .child(
            div()
                .flex_1()
                .h(TOPBAR_H)
                .flex()
                .items_center()
                .gap(px(8.))
                .child(logo(t, 22.))
                .child(
                    div()
                        .h(CONTROL_MD)
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .font_weight(W_MEDIUM)
                        .child(
                            s.org
                                .as_ref()
                                .map_or("My vault".to_owned(), |o| o.m.name.clone()),
                        ),
                ),
        )
        .child(nav)
        .child(
            div()
                .flex_1()
                .h(TOPBAR_H)
                .flex()
                .items_center()
                .justify_end()
                .gap(px(8.))
                .child(sync_status(s, t, cx))
                .child(
                    icon_button("lock", t, "lock", false)
                        .on_click(cx.listener(|this, _, window, cx| this.lock(window, cx)))
                        .test_support(),
                )
                .when_some(primary, |d, (id, label)| {
                    d.child(
                        button(id, t, BtnKind::Primary, false)
                            .child(icon("plus", 16., t.primary_ink))
                            .child(label)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                if id == "new-item" {
                                    this.new_entry(window, cx);
                                } else {
                                    this.new_contact(window, cx);
                                }
                            }))
                            .test_support(),
                    )
                }),
        );
    TitleBar::new()
        .h(TOPBAR_H)
        .bg(t.chrome)
        .border_b_0()
        .pr(px(8.))
        .child(bar)
}

/// Top bar: where the vault lives and whether it is in step (mockup: "Synced").
fn sync_status(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> impl IntoElement {
    let (dot, label, tip_text): (gpui_kit::Hsla, String, String) = match &s.org {
        None => (
            t.success_text,
            "On this computer".into(),
            "Personal vault, on this computer".into(),
        ),
        Some(o) if o.busy && o.local.last_ok.is_none() => (
            t.text_faint,
            "Syncing…".into(),
            format!("Reaching {}", o.m.address),
        ),
        Some(o) if o.error.is_some() || o.offline_locked => (
            t.warning_text,
            "Offline".into(),
            format!(
                "{} not reached{}",
                o.m.address,
                o.error
                    .as_ref()
                    .map(|e| format!(": {e}"))
                    .unwrap_or_default()
            ),
        ),
        Some(o) => (
            t.success_text,
            "Synced".into(),
            o.local.last_ok.map_or(String::new(), |ts| {
                format!(
                    "Synced with {} · {}",
                    o.m.address,
                    crate::format::when(ts, cleis_core::now())
                )
            }),
        ),
    };
    let tip_text: SharedString = tip_text.into();
    div()
        .id("sync-status")
        .flex()
        .items_center()
        .gap(px(6.))
        .text_size(TEXT_SM)
        .text_color(t.text_muted)
        .cursor_pointer()
        .tooltip(move |window, cx| {
            gpui_kit::component::tooltip::Tooltip::new(tip_text.clone()).build(window, cx)
        })
        .child(div().size(px(6.)).rounded_full().bg(dot))
        .child(label)
        .when(s.org.is_some(), |d| {
            d.on_click(cx.listener(|this, _, _, cx| this.org_sync(None, cx)))
        })
        .when_some(s.org.as_ref().and_then(|o| o.me()), |d, m| {
            d.child(
                div()
                    .ml(px(4.))
                    .child(avatar(t, &crate::org::initials(&m.name), 24.)),
            )
        })
}

fn sidebar(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let count = |f: &dyn Fn(&Entry) -> bool| s.entries.iter().filter(|e| f(e)).count();
    let item = |id: &str,
                ico: &str,
                label: String,
                n: Option<usize>,
                filter: Filter,
                cx: &mut Context<CleisApp>| {
        let active = s.filter == filter;
        div()
            .id(SharedString::from(id.to_owned()))
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
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(label),
            )
            .when_some(n, |d, n| {
                d.child(
                    div()
                        .text_size(TEXT_SM)
                        .text_color(t.text_faint)
                        .child(n.to_string()),
                )
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(s) = this.session() {
                    s.filter = filter;
                    s.refilter();
                }
                cx.notify();
            }))
            .test_support()
    };
    let group = |label: &str| {
        div()
            .pt(px(12.))
            .pb(px(4.))
            .px(px(8.))
            .text_size(TEXT_XS)
            .text_color(t.text_muted)
            .font_weight(W_MEDIUM)
            .child(label.to_owned())
    };
    let mut d = div()
        .w(SIDEBAR_W)
        .flex_none()
        .h_full()
        .bg(t.surface)
        .border_r_1()
        .border_color(t.line)
        .p(px(8.))
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(item(
            "f-all",
            "list",
            "All items".into(),
            Some(s.entries.len()),
            Filter::All,
            cx,
        ))
        .child(item(
            "f-fav",
            "star",
            "Favorites".into(),
            Some(count(&|e| e.favorite)),
            Filter::Favorites,
            cx,
        ))
        .child(item(
            "f-renew",
            "clock",
            "Renew soon".into(),
            Some(count(&|e| {
                e.subscription == cleis_core::Subscription::RenewSoon
            })),
            Filter::RenewSoon,
            cx,
        ));
    let personal: Vec<_> = s
        .spaces
        .iter()
        .filter(|sp| !sp.for_contacts && sp.kind == cleis_store::SpaceKind::Personal)
        .collect();
    let org: Vec<_> = s
        .spaces
        .iter()
        .filter(|sp| !sp.for_contacts && sp.kind == cleis_store::SpaceKind::Organization)
        .collect();
    if !personal.is_empty() {
        d = d.child(group("Personal"));
        for sp in personal {
            let n = s.entries.iter().filter(|e| e.space == sp.id).count();
            d = d.child(item(
                &format!("sp-{}", sp.id),
                "user",
                sp.name.clone(),
                Some(n),
                Filter::Space(sp.id),
                cx,
            ));
        }
    }
    if let Some(o) = &s.org {
        let manage = o.can(cleis_core::org::Perm::Manage);
        d = d.child(
            div()
                .flex()
                .items_center()
                .child(div().flex_1().child(group(&o.m.name)))
                .when(manage, |d| {
                    d.child(
                        div().pt(px(8.)).child(
                            icon_button("new-space", t, "plus", true)
                                .tooltip(|window, cx| {
                                    gpui_kit::component::tooltip::Tooltip::new("New space")
                                        .build(window, cx)
                                })
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_new_space(window, cx)
                                }))
                                .test_support(),
                        ),
                    )
                }),
        );
    } else if !org.is_empty() {
        d = d.child(group("Organization"));
    }
    for sp in org {
        let n = s.entries.iter().filter(|e| e.space == sp.id).count();
        d = d.child(item(
            &format!("sp-{}", sp.id),
            "folder",
            sp.name.clone(),
            Some(n),
            Filter::Space(sp.id),
            cx,
        ));
    }
    d.child(div().flex_1())
        .child(
            div()
                .id("f-import")
                .flex()
                .items_center()
                .gap(px(8.))
                .h(CONTROL_MD)
                .px(px(8.))
                .rounded(RADIUS_MD)
                .cursor_pointer()
                .text_color(t.text_muted)
                .hover(|d| d.bg(t.surface_hover))
                .child(icon("upload", 16., t.text_muted))
                .child("Import")
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(s) = this.session() {
                        s.tab = Tab::Settings;
                        s.settings_page = "Import & export";
                    }
                    cx.notify();
                }))
                .test_support(),
        )
        .child(item(
            "f-trash",
            "trash",
            "Trash".into(),
            (!s.trash.is_empty()).then_some(s.trash.len()),
            Filter::Trash,
            cx,
        ))
}

/// Width of each built-in column, as in mockup 01.
fn col_width(c: &ColumnDef) -> Option<f32> {
    match c.id.0.as_str() {
        "name" => Some(190.),
        "username" | "notes" => None,
        "password" => Some(116.),
        "totp" => Some(100.),
        "subscription" => Some(120.),
        "price" => Some(120.),
        "licenses" => Some(72.),
        "yearly_cost" => Some(100.),
        "website" => Some(160.),
        "renewal" => Some(110.),
        _ => Some(140.),
    }
}

fn is_num(c: &ColumnDef) -> bool {
    matches!(
        c.kind,
        ColumnKind::Price { .. } | ColumnKind::Number { .. } | ColumnKind::Formula { .. }
    )
}

fn cell_box(c: &ColumnDef) -> Div {
    let d = div()
        .h_full()
        .px(px(12.))
        .flex()
        .items_center()
        .overflow_hidden()
        .whitespace_nowrap()
        .min_w_0();
    let d = match col_width(c) {
        Some(w) => d.w(px(w)).flex_none(),
        None => d.flex_1(),
    };
    if is_num(c) { d.justify_end() } else { d }
}

fn totp_view(t: &Tokens, totp: &Totp, now: i64, big: bool) -> Div {
    let code = Totp::display(&totp.code_at(now));
    let frac = totp.remaining(now) as f32 / totp.period as f32;
    div()
        .flex()
        .items_center()
        .gap(px(6.))
        .font_family(FONT_MONO)
        .when(big, |d| d.text_size(px(17.)))
        .child(totp_ring(t, frac))
        .child(code)
}

fn faint_dash(t: &Tokens) -> Div {
    div().text_color(t.text_faint).child("—")
}

fn row_cell(
    e: &Entry,
    c: &ColumnDef,
    cols: &[ColumnDef],
    t: &Tokens,
    now: i64,
    selected: bool,
    cx: &mut Context<CleisApp>,
) -> AnyElement {
    let id = e.id;
    let base = cell_box(c);
    match c.id.0.as_str() {
        "name" => base
            .gap(px(8.))
            .child(site_icon(t, e.name.chars().next().unwrap_or('?'), 20.))
            .child(
                div()
                    .font_weight(W_MEDIUM)
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(e.name.clone()),
            )
            .into_any_element(),
        "username" => {
            let u = e.username.clone();
            base.text_color(t.text_muted)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(e.username.clone()),
                )
                .child(
                    icon_button(SharedString::from(format!("cu-{id}")), t, "copy", true)
                        .when(!selected, |d| {
                            d.invisible().group_hover("row", |s| s.visible())
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.copy_of(id, u.clone(), "Username", cx);
                        }))
                        .test_support(),
                )
                .into_any_element()
        }
        "password" => {
            let p = e.password.clone();
            base.child(
                div()
                    .flex_1()
                    .text_color(t.text_muted)
                    .child(if e.password.is_empty() {
                        ""
                    } else {
                        "••••••"
                    }),
            )
            .child(
                icon_button(SharedString::from(format!("cp-{id}")), t, "copy", true)
                    .when(!selected, |d| {
                        d.invisible().group_hover("row", |s| s.visible())
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.copy_of(id, p.clone(), "Password", cx);
                    })),
            )
            .into_any_element()
        }
        "totp" => match &e.totp {
            Some(totp) => base
                .child(totp_view(t, totp, now, false))
                .into_any_element(),
            None => base.child(faint_dash(t)).into_any_element(),
        },
        "subscription" => base
            .when_some(subscription_chip(t, e.subscription), |d, c| d.child(c))
            .into_any_element(),
        "price" => match &e.price {
            Some(m) => base
                .child(money(m, true))
                .when_some(m.period.filter(|_| true), |d, p| {
                    d.child(
                        div()
                            .text_color(t.text_faint)
                            .child(crate::format::period_suffix(p)),
                    )
                })
                .into_any_element(),
            None => base.child(faint_dash(t)).into_any_element(),
        },
        "licenses" => base
            .child(
                e.licenses
                    .map(|l| l.to_string())
                    .unwrap_or_else(|| "—".into()),
            )
            .into_any_element(),
        _ => {
            if let ColumnKind::Formula { expr } = &c.kind {
                let v = e.compute(expr, cols).ok().flatten();
                return base
                    .child(
                        v.map(|n| crate::format::num(&n))
                            .unwrap_or_else(|| "—".into()),
                    )
                    .into_any_element();
            }
            let Some(v) = e.cell(c) else {
                return base.into_any_element();
            };
            if matches!(c.kind, ColumnKind::Secret) {
                return base
                    .text_color(t.text_muted)
                    .child("••••••")
                    .into_any_element();
            }
            if let (cleis_core::column::Value::Choice(id), ColumnKind::Choice { options }) =
                (&v, &c.kind)
            {
                let o = options.iter().find(|o| &o.id == id);
                let tone = match o.and_then(|o| o.tone.as_deref()) {
                    Some("ok") => Tone::Ok,
                    Some("warn") => Tone::Warn,
                    Some("danger") => Tone::Danger,
                    Some("set") => Tone::Set,
                    Some("auto") => Tone::Auto,
                    _ => Tone::Neutral,
                };
                return base
                    .child(chip(t, tone, v.display(&c.kind), false))
                    .into_any_element();
            }
            let text = v.display(&c.kind);
            base.text_color(t.text_muted)
                .child(div().overflow_hidden().text_ellipsis().child(text))
                .into_any_element()
        }
    }
}

/// Table / Cards switch of the vault toolbar (mockup 01).
fn view_toggle(cards: bool, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    div()
        .flex()
        .p(px(2.))
        .gap(px(2.))
        .bg(t.bg)
        .border_1()
        .border_color(t.line_strong)
        .rounded(RADIUS_MD)
        .children(
            [(false, "list", "Table"), (true, "columns", "Cards")].map(|(c, ico, name)| {
                let on = cards == c;
                div()
                    .id(SharedString::from(format!("vview-{name}")))
                    .h(px(22.))
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .rounded(RADIUS_SM)
                    .cursor_pointer()
                    .when(on, |d| d.bg(t.accent_soft))
                    .child(icon(
                        ico,
                        14.,
                        if on { t.accent_text } else { t.text_muted },
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(s) = this.session() {
                            s.vault_cards = c;
                        }
                        cx.notify();
                    }))
                    .test_support()
            }),
        )
}

/// Cards view of the vault: one card per listed item.
fn vault_cards(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> impl IntoElement {
    let mut grid = div().flex().flex_wrap().gap(px(12.)).p(px(12.));
    for i in &s.visible {
        let e = &s.entries[*i];
        let id = e.id;
        let selected = s.selected == Some(id);
        grid = grid.child(
            div()
                .id(SharedString::from(format!("vcard-{id}")))
                .w(px(240.))
                .p(px(12.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .bg(t.surface_raised)
                .border_1()
                .border_color(if selected { t.primary } else { t.line })
                .rounded(RADIUS_LG)
                .cursor_pointer()
                .hover(|d| d.bg(t.surface_hover))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(s) = this.session() {
                        s.selected = Some(id);
                        s.reveal = false;
                    }
                    cx.notify();
                }))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(site_icon(t, e.name.chars().next().unwrap_or('?'), 24.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .font_weight(W_MEDIUM)
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(e.name.clone()),
                        )
                        .when(e.favorite, |d| d.child(icon("star", 12., t.warning_text))),
                )
                .child(
                    div()
                        .text_size(TEXT_SM)
                        .text_color(t.text_muted)
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(e.username.clone()),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .when_some(subscription_chip(t, e.subscription), |d, c| d.child(c))
                        .child(div().flex_1())
                        .when_some(e.price.as_ref(), |d, m| d.child(money(m, true))),
                )
                .test_support(),
        );
    }
    div().flex_1().min_h_0().flex().flex_col().child(
        div()
            .id("vault-cards")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .child(grid),
    )
}

/// Filter menu of the vault toolbar (screen 01b): subscription states and 2FA, with counts.
fn filter_menu(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    use cleis_core::Subscription as Sub;
    let f = &s.vault_filter;
    let check = |on: bool| {
        div()
            .size(px(16.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(RADIUS_SM)
            .border_1()
            .border_color(if on { t.primary } else { t.line_strong })
            .when(on, |d| {
                d.bg(t.primary).child(icon("check", 12., t.primary_ink))
            })
    };
    let count = |pred: &dyn Fn(&Entry) -> bool| s.entries.iter().filter(|e| pred(e)).count();
    let head = |label: &str| {
        div()
            .px(px(8.))
            .pt(px(6.))
            .pb(px(2.))
            .text_size(TEXT_XS)
            .font_weight(W_MEDIUM)
            .text_color(t.text_muted)
            .child(label.to_owned())
    };
    let opt = |id: String, label: &str, on: bool, n: usize| {
        menu_item(id, t, None, label.to_owned(), false)
            .child(div().flex_1())
            .child(
                div()
                    .text_size(TEXT_SM)
                    .text_color(t.text_faint)
                    .child(n.to_string()),
            )
            .child(div().absolute().left(px(8.)).child(check(on)))
            .pl(px(32.))
            .relative()
    };
    let mut m = menu_box(t)
        .top(px(36.))
        .left_0()
        .w(px(240.))
        .child(head("Subscription"));
    for (sub, label) in [
        (Sub::Active, "Active"),
        (Sub::RenewSoon, "Renew soon"),
        (Sub::Pending, "Pending"),
        (Sub::Free, "Free"),
        (Sub::Off, "Off"),
    ] {
        let on = f.subs.contains(&sub);
        m = m.child(
            opt(
                format!("vf-{}", sub.id()),
                label,
                on,
                count(&|e| e.subscription == sub),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(s) = this.session() {
                    if !s.vault_filter.subs.remove(&sub) {
                        s.vault_filter.subs.insert(sub);
                    }
                    s.refilter();
                }
                cx.notify();
            }))
            .test_support(),
        );
    }
    m = m
        .child(div().h(px(1.)).my(px(4.)).bg(t.line))
        .child(head("2FA"));
    for (want, label) in [(true, "With a 2FA code"), (false, "Without")] {
        let on = f.two_fa == Some(want);
        m = m.child(
            opt(
                format!("vf-2fa-{want}"),
                label,
                on,
                count(&|e| e.totp.is_some() == want),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(s) = this.session() {
                    s.vault_filter.two_fa = if on { None } else { Some(want) };
                    s.refilter();
                }
                cx.notify();
            }))
            .test_support(),
        );
    }
    m.child(div().h(px(1.)).my(px(4.)).bg(t.line)).child(
        menu_item("vf-clear", t, Some("x"), "Clear filters", false)
            .on_click(cx.listener(|this, _, _, cx| {
                if let Some(s) = this.session() {
                    s.vault_filter = Default::default();
                    s.filter_menu = false;
                    s.refilter();
                }
                cx.notify();
            }))
            .test_support(),
    )
}

fn vault_page(s: &Session, t: &Tokens, _window: &mut Window, cx: &mut Context<CleisApp>) -> Div {
    let cols: Vec<ColumnDef> =
        cleis_core::column::visible_columns(&s.columns, cleis_core::column::Table::Vault, None)
            .into_iter()
            .cloned()
            .collect();
    let mut head = div()
        .w_full()
        .h(px(30.))
        .flex_none()
        .flex()
        .border_b_1()
        .border_color(t.line)
        .bg(t.bg)
        .text_size(TEXT_XS)
        .font_weight(W_MEDIUM)
        .text_color(t.text_muted);
    for c in &cols {
        head = head.child(
            cell_box(c)
                .when(c.id.0 == "name", |d| {
                    d.text_color(t.text).child(format!("{} ↑", c.name))
                })
                .when(c.id.0 != "name", |d| d.child(c.name.clone())),
        );
    }
    let count = s.visible.len();
    let total: i64 = s
        .visible
        .iter()
        .filter_map(|i| {
            s.entries[*i]
                .price
                .as_ref()
                // Yearly total in the currency of the preferences (others are not converted).
                .filter(|p| p.currency == s.prefs.currency)
                .map(|p| p.yearly_cents() * i64::from(s.entries[*i].licenses.unwrap_or(1)))
        })
        .sum();
    let list = uniform_list(
        "entries",
        count,
        cx.processor(
            move |this: &mut CleisApp, range: std::ops::Range<usize>, _window, cx| {
                let t = this.tokens(cx).clone();
                let now = cleis_core::now();
                let Phase::Main(s) = &this.phase else {
                    return vec![];
                };
                let cols: Vec<ColumnDef> = cleis_core::column::visible_columns(
                    &s.columns,
                    cleis_core::column::Table::Vault,
                    None,
                )
                .into_iter()
                .cloned()
                .collect();
                let all_cols = s.columns.clone();
                let rows: Vec<(EntryId, Entry)> = range
                    .filter_map(|ix| {
                        s.visible
                            .get(ix)
                            .map(|i| (s.entries[*i].id, s.entries[*i].clone()))
                    })
                    .collect();
                let sel = s.selected;
                rows.into_iter()
                    .map(|(id, e)| {
                        let selected = sel == Some(id);
                        let mut row = div()
                            .id(SharedString::from(format!("row-{id}")))
                            .group("row")
                            .w_full()
                            .h(ROW_H)
                            .flex()
                            .border_b_1()
                            .border_color(t.line)
                            .cursor_pointer()
                            .when(selected, |d| d.bg(t.selection))
                            .when(!selected, |d| d.hover(|s| s.bg(t.surface_hover)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(s) = this.session() {
                                    s.selected = Some(id);
                                    s.reveal = false;
                                }
                                cx.notify();
                            }));
                        for c in &cols {
                            row = row.child(row_cell(&e, c, &all_cols, &t, now, selected, cx));
                        }
                        row
                    })
                    .collect()
            },
        ),
    )
    .flex_1();
    let center = div()
        .flex_1()
        .min_w_0()
        .relative()
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(44.))
                .flex_none()
                .flex()
                .items_center()
                .gap(px(8.))
                .px(px(12.))
                .border_b_1()
                .border_color(t.line)
                .child(
                    div()
                        .w(px(280.))
                        .child(Input::new(&s.search).id("search").prefix(icon(
                            "search",
                            16.,
                            t.text_muted,
                        ))),
                )
                .child(
                    div()
                        .relative()
                        .child(
                            button("filter", t, BtnKind::Ghost, false)
                                .when(s.filter_menu || s.vault_filter.count() > 0, |d| {
                                    d.bg(t.surface_active)
                                })
                                .child(icon("filter", 16., t.text_muted))
                                .child(match s.vault_filter.count() {
                                    0 => "Filter".to_owned(),
                                    n => format!("Filter · {n}"),
                                })
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(s) = this.session() {
                                        s.filter_menu = !s.filter_menu;
                                        s.columns_menu = None;
                                    }
                                    cx.notify();
                                }))
                                .test_support(),
                        )
                        .when(s.filter_menu, |d| {
                            d.child(gpui_kit::deferred(filter_menu(s, t, cx)))
                        }),
                )
                .child(
                    div()
                        .relative()
                        .child(
                            button("columns", t, BtnKind::Ghost, false)
                                .when(s.columns_menu.is_some(), |d| d.bg(t.surface_hover))
                                .child(icon("columns", 16., t.text_muted))
                                .child("Columns")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.toggle_columns_menu(cleis_core::column::Table::Vault, cx)
                                }))
                                .test_support(),
                        )
                        .when_some(
                            s.columns_menu
                                .as_ref()
                                .filter(|m| m.table == cleis_core::column::Table::Vault),
                            |d, m| d.child(gpui_kit::deferred(super::columns::menu(s, m, t, cx))),
                        ),
                )
                .child(div().flex_1())
                .child(view_toggle(s.vault_cards, t, cx)),
        )
        .when(!s.vault_cards, |d| d.child(head).child(list))
        .when(s.vault_cards, |d| d.child(vault_cards(s, t, cx)))
        .child(
            div()
                .h(ROW_H)
                .flex_none()
                .flex()
                .items_center()
                .px(px(12.))
                .text_size(TEXT_SM)
                .text_color(t.text_muted)
                .child(div().flex_1().child(format!("{count} items")))
                .child(
                    div()
                        .font_weight(W_MEDIUM)
                        .text_color(t.text)
                        .child(crate::format::money_cents(total, &s.prefs.currency)),
                ),
        );
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .child(sidebar(s, t, cx))
                .when(s.filter == Filter::Trash, |d| {
                    d.child(super::trash::page(s, t, cx))
                })
                .when(s.filter != Filter::Trash, |d| {
                    d.child(center)
                        .when_some(s.selected_entry().cloned(), |d, e| {
                            d.child(super::inspector::render(s, &e, t, cx))
                        })
                }),
        )
        .child(statusbar(s, t))
}

/// Concave 8 px corner joining the active tab to the stage outline, like an index divider
/// (charter §5): a 9 × 9 px stage-coloured square, clipped, showing a quarter of a chrome-coloured
/// circle centred on its outer top corner; the circle's border is the outline that leaves the
/// stage top, curves up and follows the tab. (A rounded corner cannot be used: GPUI caps a radius
/// at half the box.)
fn tab_corner(t: &Tokens, left: bool) -> Div {
    let disc = div()
        .absolute()
        .top(px(-9.))
        .size(px(18.))
        .rounded_full()
        .bg(t.chrome)
        .border_1()
        .border_color(t.stage_line);
    let disc = if left {
        disc.left(px(-9.))
    } else {
        disc.right(px(-9.))
    };
    let sq = div()
        .absolute()
        .bottom_0()
        .size(px(9.))
        .overflow_hidden()
        .bg(t.bg)
        .child(disc);
    // The square's last column is the tab's first one, where the frame's side line ends.
    if left {
        sq.left(px(-8.))
    } else {
        sq.right(px(-8.))
    }
}

fn statusbar(s: &Session, t: &Tokens) -> Div {
    let crumb = s
        .selected_entry()
        .map(|e| {
            let sp = s
                .spaces
                .iter()
                .find(|sp| sp.id == e.space)
                .map(|sp| sp.name.clone())
                .unwrap_or_default();
            format!("{sp} › {}", e.name)
        })
        .unwrap_or_default();
    // Standard spaces keep a copy of their key for the admins (D-21); everything else is
    // end-to-end.
    let standard = s
        .selected_entry()
        .and_then(|e| s.spaces.iter().find(|sp| sp.id == e.space))
        .is_some_and(|sp| sp.mode == cleis_store::EncryptionMode::Standard);
    div()
        .h(STATUSBAR_H)
        .flex_none()
        .flex()
        .items_center()
        .gap(px(16.))
        .px(px(12.))
        .border_t_1()
        .border_color(t.line)
        .bg(t.surface)
        .text_size(TEXT_SM)
        .text_color(t.text_muted)
        .child(div().flex_1().child(crumb))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(icon("shield", 14., t.text_muted))
                .child(if standard {
                    "Standard encryption"
                } else {
                    "End-to-end encryption"
                }),
        )
        .child("Saved")
}

impl CleisApp {
    pub fn update_entry(
        &mut self,
        id: EntryId,
        f: impl FnOnce(&mut Entry),
        cx: &mut Context<Self>,
    ) {
        let Some(s) = self.session() else { return };
        let Some(e) = s.entries.iter_mut().find(|e| e.id == id) else {
            return;
        };
        f(e);
        e.touch(None);
        let result = s.vault.put_entry(e);
        s.toast = Some(match result {
            Ok(()) => "Saved".into(),
            Err(err) => format!("Not saved: {err}").into(),
        });
        s.refilter();
        cx.notify();
    }

    /// New item, selected, its name box focused and selected so typing replaces it.
    pub fn new_entry(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let space = match s.filter {
            Filter::Space(sp) => sp,
            _ => s.vault.home_space(),
        };
        let e = Entry::new(space, "New item");
        if s.vault.put_entry(&e).is_err() {
            return;
        }
        s.query.clear();
        s.selected = Some(e.id);
        s.entries.push(e);
        s.history_tab = false;
        s.refilter();
        self.ensure_editor(window, cx);
        if let Some(name) = self
            .session()
            .and_then(|s| s.editor.as_ref())
            .and_then(|ed| ed.inputs.get(&cleis_core::edit::Field::Name).cloned())
        {
            name.update(cx, |st, cx| {
                st.focus(window, cx);
                st.select_all(window, cx);
            });
        }
        cx.notify();
    }

    pub fn toggle_menu(&mut self, m: crate::editor::Menu, cx: &mut Context<Self>) {
        if let Some(s) = self.session()
            && let Some(ed) = s.editor.as_mut()
        {
            ed.menu = if ed.menu == Some(m) { None } else { Some(m) };
        }
        cx.notify();
    }

    /// Copy of an item ("Figma copy"), without its password history or links.
    pub fn duplicate_entry(&mut self, id: EntryId, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(src) = s.entries.iter().find(|e| e.id == id) else {
            return;
        };
        let mut e = src.clone();
        e.id = EntryId::new();
        e.name = format!("{} copy", src.name);
        e.password_history.clear();
        e.linked_apps.clear();
        let t = cleis_core::now();
        e.created = t;
        e.updated = t;
        if let Err(err) = s.vault.put_entry(&e) {
            s.toast = Some(format!("Not saved: {err}").into());
            return;
        }
        s.selected = Some(e.id);
        s.entries.push(e);
        s.refilter();
        cx.notify();
    }

    /// Moves an item to another space (re-sealed with that space's key by the store).
    pub fn move_entry(&mut self, id: EntryId, to: cleis_core::SpaceId, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(e) = s.entries.iter_mut().find(|e| e.id == id) else {
            return;
        };
        let from = e.space;
        e.space = to;
        e.touch(None);
        // Same record, sealed again with the destination space's key.
        match s.vault.put_entry(e) {
            Ok(()) => s.toast = Some("Moved".into()),
            Err(err) => {
                e.space = from;
                s.toast = Some(format!("Not moved: {err}").into());
            }
        }
        if let Some(ed) = s.editor.as_mut() {
            ed.menu = None;
        }
        s.refilter();
        cx.notify();
    }

    /// Moves an item to the trash (restorable from Trash in the sidebar).
    pub fn trash_entry(&mut self, id: EntryId, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        match s.vault.delete_entry(id) {
            Ok(_) => {
                if let Some(i) = s.entries.iter().position(|e| e.id == id) {
                    let e = s.entries.remove(i);
                    s.trash.push(e);
                }
                s.editor = None;
                s.toast = Some("Moved to trash".into());
            }
            Err(err) => s.toast = Some(format!("Not deleted: {err}").into()),
        }
        s.refilter();
        cx.notify();
    }
}
