//! Main window: top bar with centred tabs (charter §5) and the active tab's page.

use super::unlock::logo;
use crate::app::{CleisApp, Filter, Phase, Session, Tab};
use crate::format::{money, subscription_chip};
use crate::i18n::tr;
use crate::theme::*;
use crate::ui::*;
use cleis_core::column::{ColumnDef, ColumnKind};
use cleis_core::totp::Totp;
use cleis_core::{Entry, EntryId};
use gpui_kit::TestSupportExt;
use gpui_kit::component::input::Input;
use gpui_kit::prelude::*;
use gpui_kit::{AnyElement, Context, Div, SharedString, Window, div, px, uniform_list};

pub fn render(
    app: &mut CleisApp,
    window: &mut Window,
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    let t = app.tokens(cx).clone();
    app.refresh_language(window, cx);
    app.watch_toast(cx);
    app.ensure_editor(window, cx);
    app.ensure_contact_editor(window, cx);
    app.ensure_org_inputs(window, cx);
    app.ensure_wheel_input(window, cx);
    // Updates (D-74), read before the session is borrowed.
    let update_button = super::update::top_button(app, &t, cx);
    let update_dialog = super::update::dialog(app, &t, cx);
    let update_section = matches!(&app.phase, Phase::Main(s) if s.tab == Tab::Settings)
        .then(|| super::update::section_of(app, &t, cx));
    if let Phase::Main(s) = &mut app.phase {
        crate::nav::track(s);
    }
    let Phase::Main(s) = &app.phase else {
        unreachable!()
    };
    // A question takes the keyboard: Enter and Escape cancel it (mockup 34).
    if s.confirm.is_some() && !s.main_focus.is_focused(window) {
        window.focus(&s.main_focus.clone(), cx);
    }
    let tab = s.tab;
    let page: AnyElement = match tab {
        Tab::Vault => vault_page(s, &t, window, cx).into_any_element(),
        Tab::Contacts => super::contacts::page(s, &t, cx).into_any_element(),
        Tab::Health => super::health::page(s, &t, cx).into_any_element(),
        Tab::Activity => super::org::activity(s, &t, cx).into_any_element(),
        Tab::Settings => super::settings::page(
            s,
            &app.autofill,
            &app.path,
            &app.cloud_hosts,
            update_section,
            &t,
            cx,
        )
        .into_any_element(),
    };
    let toast = s.toast.clone();
    div()
        .size_full()
        .flex()
        .flex_col_reverse()
        .bg(t.chrome)
        .font_family(FONT)
        // Whole-pixel line boxes (26/11/2026): a label is exactly centred in its control at any
        // scale (GPUI's default, 1.618 × 13 px, left half pixels to round).
        .line_height(px(18.))
        .text_size(TEXT_MD)
        .text_color(t.text)
        .track_focus(&s.main_focus)
        .child(nav_buttons(cx))
        .on_key_down(
            cx.listener(|this, ev: &gpui_kit::KeyDownEvent, window, cx| {
                let k = &ev.keystroke;
                // Audit U-2: Escape closes the topmost menu or dialog.
                if k.key == "escape" {
                    if this.close_top(cx) {
                        cx.stop_propagation();
                    }
                    return;
                }
                if k.key == "enter" && this.session().is_some_and(|s| s.confirm.is_some()) {
                    cx.stop_propagation();
                    this.cancel_confirm(cx);
                    return;
                }
                // Item shortcuts (mockup 33) while no box has the keyboard.
                if this
                    .session()
                    .is_some_and(|s| s.main_focus.is_focused(window) && s.confirm.is_none())
                    && this.item_key(k, cx)
                {
                    cx.stop_propagation();
                    return;
                }
                // Back and forward, as in a browser: Alt+← / Alt+→, ⌘[ / ⌘] on macOS.
                let m = &k.modifiers;
                let nav = if cfg!(target_os = "macos") {
                    match (
                        m.platform && !m.alt && !m.shift && !m.control,
                        k.key.as_str(),
                    ) {
                        (true, "[") => Some(gpui_kit::NavigationDirection::Back),
                        (true, "]") => Some(gpui_kit::NavigationDirection::Forward),
                        _ => None,
                    }
                } else {
                    match (
                        m.alt && !m.control && !m.shift && !m.platform,
                        k.key.as_str(),
                    ) {
                        (true, "left") => Some(gpui_kit::NavigationDirection::Back),
                        (true, "right") => Some(gpui_kit::NavigationDirection::Forward),
                        _ => None,
                    }
                };
                if let Some(dir) = nav
                    && this.navigate(dir, cx)
                {
                    cx.stop_propagation();
                    return;
                }
                // Mockup 38: the keys chosen in Settings › Shortcuts.
                let Some(p) = this.session().map(|s| s.prefs.clone()) else {
                    return;
                };
                use super::keys::pressed;
                if pressed(k, &p.app_key("lock")) {
                    cx.stop_propagation();
                    this.lock(window, cx);
                } else if pressed(k, &p.app_key("new")) {
                    // Audit U-3: a new item or contact (as the top bar's button).
                    cx.stop_propagation();
                    match this.session().map(|s| s.tab) {
                        Some(crate::app::Tab::Vault) => this.new_entry(window, cx),
                        Some(crate::app::Tab::Contacts) => this.new_contact(window, cx),
                        _ => {}
                    }
                } else if pressed(k, &p.app_key("search")) {
                    cx.stop_propagation();
                    if let Some(s) = this.session() {
                        let search = s.search.clone();
                        s.tab = crate::app::Tab::Vault;
                        search.update(cx, |st, cx| st.focus(window, cx));
                    }
                    cx.notify();
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
                .child(crate::ui::tab_in(
                    format!("page-{tab:?}-{:?}", s.scope),
                    s.tab_dir,
                    div().flex_1().min_h_0().flex().flex_col().child(page),
                ))
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
        .child(topbar(s, update_button, &t, window, cx))
        .when_some(s.confirm.as_ref(), |d, c| {
            d.child(super::trash::dialog(c, &t, cx))
        })
        .when_some(s.security.dialog.as_ref(), |d, dlg| {
            d.child(super::security::dialog(s, dlg, &t, cx))
        })
        .when_some(super::org::dialog(s, &t, cx), |d, dlg| d.child(dlg))
        .when_some(super::folders::dialog(s, &t, cx), |d, dlg| d.child(dlg))
        .when_some(super::erase::dialog(s, &t, cx), |d, dlg| d.child(dlg))
        .when_some(super::pin::setup_dialog(s, &t, window, cx), |d, dlg| {
            d.child(dlg)
        })
        .when_some(super::places::dialog(s, &t, cx), |d, dlg| d.child(dlg))
        .when_some(update_dialog, |d, dlg| d.child(dlg))
        .when_some(super::ctx::outside(s, cx), |d, m| d.child(m))
        .when_some(super::ctx::render(s, &t, window, cx), |d, m| d.child(m))
        // A column's edge let go anywhere (vault, contacts): its width is kept.
        .on_drop(cx.listener(|this, _: &super::dnd::ColResize, _, cx| this.end_resize(cx)))
        // Any input counts as activity (Security › Locking).
        .on_mouse_move(cx.listener(|this, _, _, _| this.touch_activity()))
        .capture_key_down(cx.listener(|this, _, _, _| this.touch_activity()))
}

fn topbar(
    s: &Session,
    update: Option<AnyElement>,
    t: &Tokens,
    window: &Window,
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    use crate::app::Scope;
    // D-69: with Preferences › Vaults › "Personal vault in its own tab", the personal vault comes
    // first, then one tab per organization; otherwise a single Vault tab shows them all.
    let mut tabs: Vec<(Tab, Option<Scope>, String, String)> = vec![];
    if s.vault_tabs() {
        tabs.push((
            Tab::Vault,
            Some(Scope::Personal),
            "tab-vault-personal".into(),
            s.personal_name(),
        ));
        for (key, name) in s.org_tabs() {
            tabs.push((
                Tab::Vault,
                Some(Scope::Org(key.clone())),
                format!("tab-vault-{key}"),
                name,
            ));
        }
    } else {
        tabs.push((Tab::Vault, None, "tab-Vault".into(), tr("Vault").into()));
    }
    if s.prefs.tabs.contacts {
        tabs.push((
            Tab::Contacts,
            None,
            "tab-Contacts".into(),
            tr("Contacts").into(),
        ));
    }
    if s.prefs.tabs.health {
        tabs.push((Tab::Health, None, "tab-Health".into(), tr("Health").into()));
    }
    // Activity exists only for an organisation (D-38): a personal vault has no log.
    if s.prefs.tabs.activity && s.all_orgs().any(|o| o.can(cleis_core::org::Perm::Audit)) {
        tabs.push((
            Tab::Activity,
            None,
            "tab-Activity".into(),
            tr("Activity").into(),
        ));
    }
    tabs.push((
        Tab::Settings,
        None,
        "tab-Settings".into(),
        tr("Settings").into(),
    ));
    let mut nav = div().relative().flex().items_end().h(TOPBAR_H);
    // The active tab is drawn again in a layer above the others (27/11/2026): its outline curves
    // 8 px into its neighbours, and a neighbour's hover light (its whole clickable area) passes
    // under that curve, never over it. Same boxes in both layers, so they line up exactly.
    let mut top = div()
        .absolute()
        .top_0()
        .left_0()
        .bottom_0()
        .flex()
        .items_end();
    let is_active = |tab: Tab, scope: &Option<crate::app::Scope>| {
        s.tab == tab && scope.as_ref().is_none_or(|sc| *sc == s.scope)
    };
    let active_ix = tabs
        .iter()
        .position(|(tab, scope, _, _)| is_active(*tab, scope));
    let shown_ix = active_ix.unwrap_or(0);
    let clear = gpui_kit::transparent_black();
    for (ix, (tab, scope, id, label)) in tabs.into_iter().enumerate() {
        let active = is_active(tab, &scope);
        let label = crate::i18n::tr_owned(&label);
        // The page comes from the side of the tab clicked (26/11/2026).
        let dir = (ix as f32 - shown_ix as f32).signum();
        let shape = || {
            div()
                .h(px(32.))
                .px(px(16.))
                .flex()
                .items_center()
                .font_weight(W_MEDIUM)
        };
        let mut el = shape()
            .id(SharedString::from(id))
            .cursor_pointer()
            // The top bar moves the window except over a control (ui::title_bar).
            .no_drag()
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(s) = this.session() {
                    s.tab_dir = dir;
                }
                this.open_tab(tab, scope.clone(), cx);
            }));
        el = if active {
            // Drawn in the layer above; here only its room and its click.
            el.h(px(33.)).mb(px(-1.)).text_color(clear)
        } else {
            // Hover (26/11/2026): the whole tab lights up under the pointer.
            let group = SharedString::from(format!("tabhover-{ix}"));
            el.relative()
                .group(group.clone())
                .text_color(t.text_muted)
                .hover(|d| d.text_color(t.text))
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .rounded_t(px(8.))
                        .group_hover(group, |d| d.bg(t.surface_hover)),
                )
        };
        top = top.child(if active {
            // The tab and its outline are one drawn shape (26/11/2026): no seam between pieces
            // at 125 or 150 % scale, where separate boxes and arcs landed on different pixels.
            shape()
                .h(px(33.))
                .mb(px(-1.))
                .relative()
                .text_color(t.accent_text)
                .child(tab_shape(t))
                .child(label.clone())
        } else {
            shape().text_color(clear).child(label.clone())
        });
        nav = nav.child(el.child(label).test_support());
    }
    let nav = nav.child(top);
    let primary = match s.tab {
        Tab::Vault => Some(("new-item", "New password")),
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
                        .child(if s.vault_tabs() {
                            s.org
                                .as_ref()
                                .and_then(|o| o.me().map(|m| m.name.clone()))
                                .unwrap_or_else(|| s.personal_name())
                        } else {
                            s.org
                                .as_ref()
                                .map_or_else(|| s.personal_name(), |o| o.m.name.clone())
                        }),
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
                .children(update)
                .child(sync_status(s, t, cx))
                .child(super::profile::top_right(s, t, cx))
                .child(
                    // Opening with nothing (D-76): the padlock is drawn open (27/11/2026).
                    icon_button(
                        "lock",
                        t,
                        if s.opening == crate::opening::Mode::Nothing {
                            "unlock"
                        } else {
                            "lock"
                        },
                        false,
                    )
                    .no_drag()
                    .tooltip({
                        let k = super::ctx::keys(&s.prefs.app_key("lock"));
                        let text: SharedString = crate::trf!("Lock ({})", k).into();
                        move |window, cx| {
                            gpui_kit::component::tooltip::Tooltip::new(text.clone())
                                .build(window, cx)
                        }
                    })
                    .on_click(cx.listener(|this, _, window, cx| this.lock(window, cx)))
                    .test_support(),
                )
                .when_some(primary, |d, (id, label)| {
                    d.child(
                        button(id, t, BtnKind::Primary, false)
                            .no_drag()
                            .child(icon("plus", 16., t.primary_ink))
                            .child(crate::i18n::tr_owned(label))
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
    title_bar(t, window, bar)
}

/// Top bar: where the vault lives and whether it is in step (mockup: "Synced").
fn sync_status(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> impl IntoElement {
    let (dot, label, tip_text): (gpui_kit::Hsla, String, String) = match &s.org {
        // Mockup 37d: the dot alone; its tooltip names the connection ("Local", not "On this
        // computer"), or the place the personal vault syncs with.
        None => (
            t.success_text,
            String::new(),
            match (&s.sync.place, &s.sync.remote) {
                (Some(_), Some(r)) => format!("{} · {}", s.sync.provider.name(), r.label()),
                (Some(p), None) => format!("{} · {}", s.sync.provider.name(), p.display()),
                _ => tr("Local").into(),
            },
        ),
        Some(o) if o.busy && o.local.last_ok.is_none() => (
            t.text_faint,
            String::new(),
            crate::trf!("Reaching {}", o.m.address),
        ),
        Some(o) if o.error.is_some() || o.offline_locked => (
            t.warning_text,
            String::new(),
            crate::trf!(
                "{} not reached{}",
                o.m.address,
                o.error
                    .as_ref()
                    .map(|e| format!(": {}", crate::i18n::tr_dynamic(e)))
                    .unwrap_or_default()
            ),
        ),
        Some(o) => (
            t.success_text,
            String::new(),
            o.local.last_ok.map_or(String::new(), |ts| {
                crate::trf!(
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
        .no_drag()
        .flex()
        .items_center()
        .gap(px(6.))
        .text_size(TEXT_SM)
        .text_color(t.text_muted)
        .cursor_pointer()
        .tooltip(move |window, cx| {
            gpui_kit::component::tooltip::Tooltip::new(tip_text.clone()).build(window, cx)
        })
        // A target large enough to point at, the dot in its middle.
        .child(
            div()
                .size(px(20.))
                .flex()
                .items_center()
                .justify_center()
                .child(div().size(px(6.)).rounded_full().bg(dot)),
        )
        .when(!label.is_empty(), |d| d.child(label))
        .when(s.org.is_some(), |d| {
            d.on_click(cx.listener(|this, _, _, cx| this.org_sync(None, cx)))
        })
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
        "recovery" => Some(120.),
        _ => Some(140.),
    }
}

fn is_num(c: &ColumnDef) -> bool {
    matches!(
        c.kind,
        ColumnKind::Price { .. } | ColumnKind::Number { .. } | ColumnKind::Formula { .. }
    )
}

/// Width of a vault column: dragged, kept, or as in mockup 01; in customize mode the name also
/// holds the handle and the cross.
fn vault_width(s: &Session, c: &ColumnDef) -> Option<f32> {
    let w = super::dnd::width(s, c, col_width(c));
    if s.customize && c.id.0 == "name" {
        w.map(|w| w + 48.)
    } else {
        w
    }
}

/// A cell of column `c`, `w` wide (or sharing the rest of the row when `None`).
fn cell_box(c: &ColumnDef, w: Option<f32>) -> Div {
    let d = div()
        .h_full()
        .px(px(12.))
        .flex()
        .items_center()
        .overflow_hidden()
        .whitespace_nowrap()
        .min_w_0();
    let d = match w {
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
        .when(big, |d| d.text_size(TEXT_LG).line_height(px(20.)))
        .child(totp_ring(t, frac))
        .child(code)
}

fn faint_dash(t: &Tokens) -> Div {
    div().text_color(t.text_faint).child("—")
}

#[allow(clippy::too_many_arguments)]
fn row_cell(
    e: &Entry,
    c: &ColumnDef,
    cols: &[ColumnDef],
    t: &Tokens,
    now: i64,
    selected: bool,
    w: Option<f32>,
    editing: bool,
    cx: &mut Context<CleisApp>,
) -> AnyElement {
    let id = e.id;
    let base = cell_box(c, w);
    match c.id.0.as_str() {
        // Customize (mockup 35f): a handle, then the cross that moves the item to the trash;
        // the column grows by their width so that the name keeps its room.
        "name" => base
            .gap(px(8.))
            .when(editing, |d| {
                d.child(crate::ui::grip_pulse(format!("grip-{id}"), t, 14.))
                    .child(super::sidebar::remove_badge(
                        &format!("rm-item-{id}"),
                        t,
                        tr("Move to trash"),
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.trash_entry(id, cx);
                        }),
                    ))
            })
            .child(site_icon(t, e.name.chars().next().unwrap_or('?'), 20.))
            .child(
                div()
                    .id(SharedString::from(format!("rname-{id}")))
                    .min_w_0()
                    .font_weight(W_MEDIUM)
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(e.name.clone())
                    // A name cut short shows whole on hover.
                    .when(e.name.chars().count() > 18, |d| {
                        d.tooltip(tip(e.name.clone()))
                    }),
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
                    cell_button(format!("cu-{id}"), t, "copy")
                        .tooltip(|window, cx| {
                            gpui_kit::component::tooltip::Tooltip::new(tr("Copy username"))
                                .build(window, cx)
                        })
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
                cell_button(format!("cp-{id}"), t, "copy")
                    .tooltip(|window, cx| {
                        gpui_kit::component::tooltip::Tooltip::new(tr("Copy password"))
                            .build(window, cx)
                    })
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
        // D-102 (mockup 47f): unused recovery keys / all, orange at 2 or fewer.
        "recovery" => {
            let (left, all) = e.recovery_left();
            if all == 0 {
                return base.child(faint_dash(t)).into_any_element();
            }
            let text = format!("{left} / {all}");
            if left <= 2 {
                base.child(chip(t, Tone::Warn, text, false))
                    .into_any_element()
            } else {
                base.text_color(t.text_muted).child(text).into_any_element()
            }
        }
        // Mockup 46d: the site's address, and the button opening it.
        "website" => match e.urls.first() {
            Some(u) => {
                let url = u.clone();
                let shown = u
                    .split_once("://")
                    .map_or(u.as_str(), |(_, rest)| rest)
                    .trim_end_matches('/')
                    .to_owned();
                let tip_text: SharedString = crate::trf!("Open {}", shown).into();
                base.text_color(t.text_muted)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(shown),
                    )
                    .child(
                        cell_button(format!("ow-{id}"), t, "external")
                            .tooltip(move |window, cx| {
                                gpui_kit::component::tooltip::Tooltip::new(tip_text.clone())
                                    .build(window, cx)
                            })
                            .when(!selected, |d| {
                                d.invisible().group_hover("row", |s| s.visible())
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.open_site(&url, cx);
                            }))
                            .test_support(),
                    )
                    .into_any_element()
            }
            None => base.child(faint_dash(t)).into_any_element(),
        },
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

/// The mouse's side buttons go to the view before or after (27/11/2026), wherever the pointer
/// is: listened to for the whole window, not through an element's hitbox (the top bar and the
/// menus occlude what is under them).
fn nav_buttons(cx: &mut Context<CleisApp>) -> impl IntoElement {
    let app = cx.entity().downgrade();
    gpui_kit::canvas(
        |_, _, _| {},
        move |_, _, window, _| {
            window.on_mouse_event(move |ev: &gpui_kit::MouseDownEvent, phase, _, cx| {
                if phase != gpui_kit::DispatchPhase::Bubble {
                    return;
                }
                if let gpui_kit::MouseButton::Navigate(dir) = ev.button {
                    let _ = app.update(cx, |this, cx| this.navigate(dir, cx));
                }
            });
        },
    )
    .absolute()
    .size_0()
}

/// Cards view of the vault: one card per listed item.
fn vault_cards(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> impl IntoElement {
    let mut grid = div().flex().flex_wrap().gap(px(12.)).p(px(12.));
    for i in &s.visible {
        let e = &s.entries[*i];
        let id = e.id;
        let selected = s.selected == Some(id);
        let aimed = s.aimed(super::ctx::CtxTarget::Entry(id));
        grid = grid.child(
            div()
                .id(SharedString::from(format!("vcard-{id}")))
                .relative()
                .when(aimed, |d| d.child(super::ctx::aim_ring(t, RADIUS_LG)))
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
                .on_mouse_down(
                    gpui_kit::MouseButton::Right,
                    super::ctx::on_right(super::ctx::CtxTarget::Entry(id), cx),
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    if let Some(s) = this.session() {
                        s.selected = Some(id);
                        s.reveal = false;
                        let f = s.main_focus.clone();
                        window.focus(&f, cx);
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
            .on_click(cx.listener(|this, _, _, cx| this.deselect(cx)))
            .on_mouse_down(
                gpui_kit::MouseButton::Right,
                super::ctx::on_right(super::ctx::CtxTarget::ListVoid, cx),
            )
            .child(grid)
            .test_support(),
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
            .child(crate::i18n::tr_owned(label))
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
        menu_item("vf-clear", t, Some("x"), tr("Clear filters"), false)
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

/// Customize mode (mockup 35f): in place of the toolbar, a low banner and Done.
fn edit_bar(t: &Tokens, cx: &mut Context<CleisApp>) -> impl IntoElement {
    div()
        .h(px(36.))
        .flex_none()
        .flex()
        .items_center()
        .gap(px(8.))
        .px(px(12.))
        .bg(t.accent_soft)
        .border_b_1()
        .border_color(t.line)
        .child(
            div()
                .id("edit-bar-info")
                .size(px(20.))
                .flex()
                .items_center()
                .justify_center()
                .child(icon("grip", 16., t.accent_text))
                .tooltip(tip(tr(
                    "Drag the handles to reorder; the cross moves to the trash",
                ))),
        )
        .child(div().flex_1())
        .child(
            button("customize-done", t, BtnKind::Primary, true)
                .child(tr("Done"))
                .on_click(cx.listener(|this, _, _, cx| this.toggle_customize(cx)))
                .test_support(),
        )
}

fn vault_page(s: &Session, t: &Tokens, window: &mut Window, cx: &mut Context<CleisApp>) -> Div {
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
    // The column the list is sorted by (Sort by ▸ in the list's menu) carries the arrow.
    let sorted = match s.prefs.sort_by {
        cleis_core::settings::SortBy::Name => Some(("name", "↑")),
        cleis_core::settings::SortBy::Price => Some(("price", "↓")),
        cleis_core::settings::SortBy::Renewal => Some(("renewal", "↑")),
        _ => None,
    };
    let ncols = cols.len();
    for (ci, c) in cols.iter().enumerate() {
        let arrow = sorted.filter(|(id, _)| c.id.0 == *id).map(|(_, a)| a);
        head = head.child(
            cell_box(c, vault_width(s, c))
                .relative()
                .on_drag_move(super::dnd::on_resize_move(&c.id, cx))
                .when_some(arrow, |d, a| {
                    d.text_color(t.text)
                        .child(format!("{} {a}", crate::i18n::tr_owned(&c.name)))
                })
                .when(arrow.is_none(), |d| d.child(crate::i18n::tr_owned(&c.name)))
                .child(super::dnd::col_grip(&c.id, ci + 1 < ncols, t, cx)),
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
                let rows: Vec<(usize, EntryId, Entry)> = range
                    .filter_map(|ix| {
                        s.visible
                            .get(ix)
                            .map(|i| (ix, s.entries[*i].id, s.entries[*i].clone()))
                    })
                    .collect();
                let sel = s.selected;
                let aimed_at = s.ctx.as_ref().and_then(|m| match m.target {
                    super::ctx::CtxTarget::Entry(id) => Some(id),
                    _ => None,
                });
                let widths: Vec<Option<f32>> = cols.iter().map(|c| vault_width(s, c)).collect();
                let editing = s.customize;
                // Items take each other's place in "My order", or while customizing.
                let reorder = editing || s.prefs.sort_by == cleis_core::settings::SortBy::Manual;
                rows.into_iter()
                    .map(|(ix, id, e)| {
                        let selected = sel == Some(id);
                        let dragged = super::dnd::DraggedEntry {
                            id,
                            name: e.name.clone().into(),
                            ix,
                        };
                        let line = t.primary;
                        let mut row = div()
                            .id(SharedString::from(format!("row-{id}")))
                            .on_drag(dragged, super::dnd::entry_ghost)
                            .when(reorder, |d| {
                                d.drag_over::<super::dnd::DraggedEntry>(move |d, from, _, _| {
                                    super::dnd::drop_line(d, from.ix, ix, line)
                                })
                                .on_drop(cx.listener(
                                    move |this, d: &super::dnd::DraggedEntry, _, cx| {
                                        this.reorder_entry(d.id, id, cx)
                                    },
                                ))
                            })
                            .group("row")
                            .w_full()
                            .h(ROW_H)
                            .flex()
                            .border_b_1()
                            .border_color(t.line)
                            .cursor_pointer()
                            .when(selected, |d| d.bg(t.selection))
                            .when(!selected, |d| d.hover(|s| s.bg(t.surface_hover)))
                            .on_mouse_down(
                                gpui_kit::MouseButton::Right,
                                super::ctx::on_right(super::ctx::CtxTarget::Entry(id), cx),
                            )
                            .on_click(cx.listener(move |this, _, window, cx| {
                                // The list below must not take it as a click in the void.
                                cx.stop_propagation();
                                if let Some(s) = this.session() {
                                    s.selected = Some(id);
                                    s.reveal = false;
                                    let f = s.main_focus.clone();
                                    window.focus(&f, cx);
                                }
                                cx.notify();
                            }));
                        for (c, w) in cols.iter().zip(&widths) {
                            row = row.child(row_cell(
                                &e, c, &all_cols, &t, now, selected, *w, editing, cx,
                            ));
                        }
                        if aimed_at == Some(id) {
                            row = row.relative().child(super::ctx::aim_ring(&t, RADIUS_SM));
                        }
                        row.test_support()
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
        .when(s.customize, |d| d.child(edit_bar(t, cx)))
        .when(!s.customize, |d| {
            d.child(
                div()
                    .h(px(44.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(12.))
                    .border_b_1()
                    .border_color(t.line)
                    .child(div().w(px(280.)).child(
                        Input::new(&s.search).id("search").prefix(icon(
                            "search",
                            16.,
                            t.text_muted,
                        )),
                    ))
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
                                        0 => tr("Filter").to_owned(),
                                        n => crate::trf!("Filter · {}", n),
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
                                    .child(tr("Columns"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.toggle_columns_menu(
                                            cleis_core::column::Table::Vault,
                                            cx,
                                        )
                                    }))
                                    .test_support(),
                            )
                            .when_some(
                                s.columns_menu
                                    .as_ref()
                                    .filter(|m| m.table == cleis_core::column::Table::Vault),
                                |d, m| {
                                    d.child(gpui_kit::deferred(super::columns::menu(s, m, t, cx)))
                                },
                            ),
                    )
                    .child(div().flex_1())
                    .child(view_toggle(s.vault_cards, t, cx)),
            )
        })
        .when(!s.vault_cards, |d| {
            d.child(head).child(
                // A click below the rows lets go of the item and closes the panel on the right.
                div()
                    .id("entries-void")
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .on_click(cx.listener(|this, _, _, cx| this.deselect(cx)))
                    .on_mouse_down(
                        gpui_kit::MouseButton::Right,
                        super::ctx::on_right(super::ctx::CtxTarget::ListVoid, cx),
                    )
                    // Another folder or section: its items come in (26/11/2026).
                    .child(crate::ui::page_in(
                        format!("rows-{:?}-{:?}", s.filter, s.scope),
                        div().flex_1().min_h_0().flex().flex_col().child(list),
                    ))
                    .test_support(),
            )
        })
        .when(s.vault_cards, |d| {
            d.child(crate::ui::page_in(
                format!("cards-{:?}-{:?}", s.filter, s.scope),
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(vault_cards(s, t, cx)),
            ))
        })
        .child(
            div()
                .h(ROW_H)
                .flex_none()
                .flex()
                .items_center()
                .px(px(12.))
                .text_size(TEXT_SM)
                .text_color(t.text_muted)
                .child(div().flex_1().child(crate::trf!(
                    "{} item{}",
                    count,
                    if count == 1 { "" } else { "s" }
                )))
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
                .child(super::sidebar::render(s, t, cx))
                .when(s.filter == Filter::Trash, |d| {
                    d.child(super::trash::page(s, t, cx))
                })
                .when(s.filter != Filter::Trash, |d| {
                    // D-79: an organization whose PIN is asked shows the PIN, not its items.
                    match super::org::pin_shown(s) {
                        Some(o) => d.child(super::org::pin_card(o, t, window, cx)),
                        None => d
                            .child(center)
                            .when_some(s.selected_entry().cloned(), |d, e| {
                                d.child(super::inspector::render(s, &e, t, cx))
                            }),
                    }
                }),
        )
        .child(statusbar(s, t))
}

/// The active tab (charter §5, like an index divider): its fill flares into the stage with two
/// concave 8 px corners, and one outline runs from the stage's top line, up the tab, over its
/// rounded top and back down. Drawn as two paths over the tab's box widened by 8 px each side.
fn tab_shape(t: &Tokens) -> impl IntoElement {
    let (fill, line) = (t.bg, t.stage_line);
    gpui_kit::canvas(
        |_, _, _| (),
        move |b, _, window, _| {
            use gpui_kit::{PathBuilder, point};
            let r = 8.;
            let (x0, y0) = (f32::from(b.origin.x), f32::from(b.origin.y));
            let (w, h) = (f32::from(b.size.width), f32::from(b.size.height));
            // The stage's top line is the box's last pixel row: the outline meets it there.
            let base = y0 + h - 0.5;
            let (l, rt) = (x0 + r + 0.5, x0 + w - r - 0.5);
            let top = y0 + 0.5;
            let p = |x: f32, y: f32| point(px(x), px(y));
            let outline = |mut pb: PathBuilder| {
                pb.move_to(p(x0, base));
                pb.curve_to(p(l, base - r), p(l, base));
                pb.line_to(p(l, top + r));
                pb.curve_to(p(l + r, top), p(l, top));
                pb.line_to(p(rt - r, top));
                pb.curve_to(p(rt, top + r), p(rt, top));
                pb.line_to(p(rt, base - r));
                pb.curve_to(p(x0 + w, base), p(rt, base));
                pb
            };
            let mut area = outline(PathBuilder::fill());
            area.line_to(p(x0 + w, y0 + h));
            area.line_to(p(x0, y0 + h));
            area.close();
            if let Ok(path) = area.build() {
                window.paint_path(path, fill);
            }
            if let Ok(path) = outline(PathBuilder::stroke(px(1.))).build() {
                window.paint_path(path, line);
            }
        },
    )
    .absolute()
    .top_0()
    .bottom_0()
    .left(px(-8.))
    .right(px(-8.))
}

fn statusbar(s: &Session, t: &Tokens) -> Div {
    let crumb = s
        .selected_entry()
        .map(|e| {
            let sp = s
                .spaces
                .iter()
                .find(|sp| sp.id == e.space)
                .map(|sp| crate::i18n::space(&sp.name))
                .unwrap_or_default();
            format!("{sp} › {}", e.name)
        })
        .unwrap_or_default();
    // Standard spaces keep a copy of their key for the admins (D-21); everything else is
    // end-to-end.
    // Audit L-14: nothing selected, nothing claimed.
    let mode = s
        .selected_entry()
        .and_then(|e| s.spaces.iter().find(|sp| sp.id == e.space))
        .map(|sp| sp.mode == cleis_store::EncryptionMode::Standard);
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
        .when_some(mode, |d, standard| {
            d.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(icon("shield", 14., t.text_muted))
                    .child(tr(if standard {
                        "Standard encryption"
                    } else {
                        "End-to-end encryption"
                    })),
            )
        })
        .child(tr("Saved"))
}

impl CleisApp {
    /// Shows a tab; a vault tab also sets what the vault shows and the organization shown.
    pub fn open_tab(&mut self, tab: Tab, scope: Option<crate::app::Scope>, cx: &mut Context<Self>) {
        use crate::app::Scope;
        self.flush_edits(cx);
        if let Some(Scope::Org(k)) = &scope {
            self.activate_org(k);
        }
        if tab == Tab::Activity
            && let Some(s) = self.session()
            && !s
                .org
                .as_ref()
                .is_some_and(|o| o.can(cleis_core::org::Perm::Audit))
            && let Some(k) = s
                .orgs_idle
                .iter()
                .find(|o| o.can(cleis_core::org::Perm::Audit))
                .map(|o| o.m.key())
        {
            self.activate_org(&k);
        }
        if let Some(s) = self.session() {
            s.tab = tab;
            s.ctx = None;
            if tab == Tab::Vault {
                let next = scope.unwrap_or(if s.vault_tabs() {
                    s.scope.clone()
                } else {
                    Scope::All
                });
                if s.scope != next {
                    s.scope = next;
                    // A folder of another vault is not shown here.
                    if let Filter::Space(sp) = s.filter
                        && !s.in_scope(sp)
                    {
                        s.filter = Filter::All;
                    }
                    s.refilter();
                }
            }
        }
        if tab == Tab::Activity {
            self.org_query_log(cx);
        }
        cx.notify();
    }

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
            Ok(()) => tr("Saved").into(),
            Err(err) => crate::trf!("Not saved: {}", err).into(),
        });
        s.refilter();
        cx.notify();
    }

    /// New item, selected, its name box focused and selected so typing replaces it.
    pub fn new_entry(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        // In the organization shown, never the personal vault (27/11/2026).
        let Some(space) = s.target_space() else {
            s.toast = Some(tr("Your role does not allow adding here").into());
            cx.notify();
            return;
        };
        let e = Entry::new(space, "New password");
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
        if !s.may(src.space, cleis_core::org::Perm::Edit) {
            s.toast = Some(tr("Your role does not allow adding here").into());
            cx.notify();
            return;
        }
        let mut e = src.clone();
        e.id = EntryId::new();
        e.name = crate::trf!("{} copy", src.name);
        e.password_history.clear();
        e.linked_apps.clear();
        let t = cleis_core::now();
        e.created = t;
        e.updated = t;
        if let Err(err) = s.vault.put_entry(&e) {
            s.toast = Some(crate::trf!("Not saved: {}", err).into());
            return;
        }
        s.selected = Some(e.id);
        s.entries.push(e);
        s.refilter();
        cx.notify();
    }

    /// Moves an item to another space (re-sealed with that space's key by the store).
    pub fn move_entry(&mut self, id: EntryId, to: cleis_core::SpaceId, cx: &mut Context<Self>) {
        use cleis_core::org::Perm;
        let Some(s) = self.session() else { return };
        let Some(from) = s.entries.iter().find(|e| e.id == id).map(|e| e.space) else {
            return;
        };
        // Audit L-2: an organization's item stays in the organization; between two of its spaces
        // it is a new item there and an erasure here, so the members of the space left lose it.
        let org_key = |sp| s.org_of(sp).map(|o| o.m.key());
        let refusal = if s.is_org_space(from) && org_key(from) != org_key(to) {
            Some(tr("An organization's items stay in the organization"))
        } else if !s.may(to, Perm::Edit) || !s.may(from, Perm::Delete) {
            Some(tr("Your role does not allow moving it"))
        } else {
            None
        };
        if let Some(why) = refusal {
            s.toast = Some(why.into());
            cx.notify();
            return;
        }
        if s.is_org_space(from) {
            let Some(src) = s.entries.iter().find(|e| e.id == id).cloned() else {
                return;
            };
            let mut copy = src.clone();
            copy.id = EntryId::new();
            copy.space = to;
            copy.touch(None);
            let r = s
                .vault
                .put_entry(&copy)
                .and_then(|()| s.vault.purge_entry(id).map(|_| ()));
            match r {
                Ok(()) => {
                    s.entries.retain(|e| e.id != id);
                    s.selected = Some(copy.id);
                    s.entries.push(copy);
                    s.editor = None;
                    s.toast = Some(tr("Moved").into());
                }
                Err(err) => s.toast = Some(crate::trf!("Not moved: {}", err).into()),
            }
            s.refilter();
            cx.notify();
            return;
        }
        let Some(e) = s.entries.iter_mut().find(|e| e.id == id) else {
            return;
        };
        e.space = to;
        e.touch(None);
        // Same record, sealed again with the destination space's key.
        match s.vault.put_entry(e) {
            Ok(()) => s.toast = Some(tr("Moved").into()),
            Err(err) => {
                e.space = from;
                s.toast = Some(crate::trf!("Not moved: {}", err).into());
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
        if s.entries
            .iter()
            .find(|e| e.id == id)
            .is_some_and(|e| !s.may(e.space, cleis_core::org::Perm::Delete))
        {
            s.toast = Some(tr("Your role does not allow deleting here").into());
            cx.notify();
            return;
        }
        match s.vault.delete_entry(id) {
            Ok(_) => {
                if let Some(i) = s.entries.iter().position(|e| e.id == id) {
                    let e = s.entries.remove(i);
                    s.trash.push(e);
                }
                s.editor = None;
                s.toast = Some(tr("Moved to trash").into());
            }
            Err(err) => s.toast = Some(crate::trf!("Not deleted: {}", err).into()),
        }
        s.refilter();
        cx.notify();
    }
}
