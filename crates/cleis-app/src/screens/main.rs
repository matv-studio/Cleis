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
    // Updates (D-74), read before the session is borrowed.
    let update_button = super::update::top_button(app, &t, cx);
    let update_dialog = super::update::dialog(app, &t, cx);
    let update_section = matches!(&app.phase, Phase::Main(s) if s.tab == Tab::Settings)
        .then(|| super::update::section_of(app, &t, cx));
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
        Tab::Settings => super::settings::page(s, &app.autofill, &app.path, update_section, &t, cx)
            .into_any_element(),
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
                    // Audit U-3: Ctrl+N, a new item or contact (as the top bar's button).
                    "n" => {
                        cx.stop_propagation();
                        match this.session().map(|s| s.tab) {
                            Some(crate::app::Tab::Vault) => this.new_entry(window, cx),
                            Some(crate::app::Tab::Contacts) => this.new_contact(window, cx),
                            _ => {}
                        }
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
        .child(topbar(s, update_button, &t, window, cx))
        .when_some(s.confirm.as_ref(), |d, c| {
            d.child(super::trash::dialog(c, &t, cx))
        })
        .when_some(s.security.dialog.as_ref(), |d, dlg| {
            d.child(super::security::dialog(s, dlg, &t, cx))
        })
        .when_some(super::org::dialog(s, &t, cx), |d, dlg| d.child(dlg))
        .when_some(super::folders::dialog(s, &t, cx), |d, dlg| d.child(dlg))
        .when_some(super::places::dialog(s, &t, cx), |d, dlg| d.child(dlg))
        .when_some(update_dialog, |d, dlg| d.child(dlg))
        .when_some(super::ctx::outside(s, cx), |d, m| d.child(m))
        .when_some(super::ctx::render(s, &t, cx), |d, m| d.child(m))
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
    if s.prefs.vault_tabs {
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
    let mut nav = div().flex().items_end().h(TOPBAR_H);
    for (tab, scope, id, label) in tabs {
        let active = s.tab == tab && scope.as_ref().is_none_or(|sc| *sc == s.scope);
        let mut el = div()
            .id(SharedString::from(id))
            .h(px(32.))
            .px(px(16.))
            .flex()
            .items_center()
            .font_weight(W_MEDIUM)
            .cursor_pointer()
            // The top bar moves the window except over a control (ui::title_bar).
            .no_drag()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.open_tab(tab, scope.clone(), cx);
            }));
        el = if active {
            // The outline is drawn by a frame that stops 7 px above the bottom; the corners take
            // over from there (a parent's own border would paint over its children).
            el.h(px(33.))
                .mb(px(-1.))
                .relative()
                .bg(t.bg)
                .rounded_t(px(8.))
                .text_color(t.accent_text)
                // Corners first, then the frame over them: its side lines run 2 px into the
                // corners' arcs (vertical there too), so no gap opens at 125 or 150 % scale.
                .child(tab_corner(t, true))
                .child(tab_corner(t, false))
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .bottom(px(7.))
                        .border_t_1()
                        .border_l_1()
                        .border_r_1()
                        .border_color(t.stage_line)
                        .rounded_t(px(8.)),
                )
        } else {
            el.text_color(t.text_muted)
        };
        nav = nav.child(el.child(crate::i18n::tr_owned(&label)).test_support());
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
                        .child(if s.prefs.vault_tabs {
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
                .child(
                    icon_button("lock", t, "lock", false)
                        .no_drag()
                        .tooltip(|window, cx| {
                            gpui_kit::component::tooltip::Tooltip::new(tr("Lock (Ctrl+L)"))
                                .build(window, cx)
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
        None => (
            t.success_text,
            tr("On this computer").into(),
            tr("Personal vault, on this computer").into(),
        ),
        Some(o) if o.busy && o.local.last_ok.is_none() => (
            t.text_faint,
            tr("Syncing…").into(),
            crate::trf!("Reaching {}", o.m.address),
        ),
        Some(o) if o.error.is_some() || o.offline_locked => (
            t.warning_text,
            tr("Offline").into(),
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
            tr("Synced").into(),
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
        .child(div().size(px(6.)).rounded_full().bg(dot))
        .child(crate::i18n::tr_owned(&label))
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
    let count = |f: &dyn Fn(&Entry) -> bool| {
        s.entries
            .iter()
            .filter(|e| s.in_scope(e.space) && f(e))
            .count()
    };
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
                    .child(crate::i18n::tr_owned(&label)),
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
            .child(crate::i18n::tr_owned(label))
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
            tr("All items").into(),
            Some(count(&|_| true)),
            Filter::All,
            cx,
        ))
        .child(item(
            "f-fav",
            "star",
            tr("Favorites").into(),
            Some(count(&|e| e.favorite)),
            Filter::Favorites,
            cx,
        ))
        .child(item(
            "f-renew",
            "clock",
            tr("Renew soon").into(),
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
    use crate::app::Scope;
    let show_personal = !matches!(s.scope, Scope::Org(_));
    // Personal folders (D-62): "+" to add one, "⋯" on hover to rename or delete.
    let personal = if show_personal { personal } else { vec![] };
    let personal_head = div()
        .flex()
        .items_center()
        .child(div().flex_1().child(group(if s.scope == Scope::Personal {
            "Folders"
        } else {
            "Personal"
        })))
        .child(
            div().pt(px(8.)).child(
                icon_button("new-folder-v", t, "plus", true)
                    .tooltip(|window, cx| {
                        gpui_kit::component::tooltip::Tooltip::new(tr("New folder"))
                            .build(window, cx)
                    })
                    .on_click(cx.listener(|this, _, window, cx| this.open_new_folder(window, cx)))
                    .test_support(),
            ),
        );
    if show_personal {
        d = d.child(personal_head);
    }
    let home = s.vault.home_space();
    for sp in personal {
        // "My vault" is the personal section: every personal item, folders included.
        let section = sp.id == home;
        let n = s
            .entries
            .iter()
            .filter(|e| {
                if section {
                    s.scope_has(&Scope::Personal, e.space)
                } else {
                    e.space == sp.id
                }
            })
            .count();
        let gname = SharedString::from(format!("pf-{}", sp.id));
        let menu_open = s.folder_menu == Some(sp.id);
        d = d.child(
            div()
                .id(SharedString::from(format!("pfw-{}", sp.id)))
                .relative()
                .group(gname.clone())
                .on_mouse_down(
                    gpui_kit::MouseButton::Right,
                    super::ctx::on_right(super::ctx::CtxTarget::Space(sp.id), cx),
                )
                .child(item(
                    &format!("sp-{}", sp.id),
                    if section { "user" } else { "folder" },
                    crate::i18n::space(&sp.name),
                    Some(n),
                    if section {
                        Filter::Personal
                    } else {
                        Filter::Space(sp.id)
                    },
                    cx,
                ))
                .child(
                    // The "⋯" alone, centred on the row (no tile behind it).
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .right(px(4.))
                        .flex()
                        .items_center()
                        .when(!menu_open, |d| {
                            d.invisible().group_hover(gname.clone(), |s| s.visible())
                        })
                        .child(super::folders::more_button(sp.id, t, cx)),
                )
                .when(menu_open, |d| {
                    d.child(gpui_kit::deferred(super::folders::menu(s, sp.id, t, cx)))
                }),
        );
    }
    // One group per organization (D-69); with its own tab, only that organization's spaces.
    let shown: Vec<&crate::org::OrgSession> = s
        .all_orgs()
        .filter(|o| match &s.scope {
            Scope::All => true,
            Scope::Personal => false,
            Scope::Org(k) => o.m.key() == *k,
        })
        .collect();
    let shown_key = s.org.as_ref().map(|o| o.m.key());
    for o in &shown {
        let manage = o.can(cleis_core::org::Perm::Manage);
        let key = o.m.key();
        let title = if matches!(s.scope, Scope::Org(_)) {
            tr("Spaces").to_owned()
        } else {
            o.m.name.clone()
        };
        d = d.child(
            div()
                .flex()
                .items_center()
                .child(div().flex_1().child(group(&title)))
                .when(manage, |d| {
                    d.child(
                        div().pt(px(8.)).child(
                            icon_button(
                                // The organization shown keeps the plain id.
                                SharedString::from(if shown_key.as_deref() == Some(key.as_str()) {
                                    "new-space".to_owned()
                                } else {
                                    format!("new-space-{key}")
                                }),
                                t,
                                "plus",
                                true,
                            )
                            .tooltip(|window, cx| {
                                gpui_kit::component::tooltip::Tooltip::new(tr("New space"))
                                    .build(window, cx)
                            })
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.activate_org(&key);
                                this.open_new_space(window, cx)
                            }))
                            .test_support(),
                        ),
                    )
                }),
        );
        for sp in org.iter().filter(|sp| o.names.contains_key(&sp.id)) {
            let n = s.entries.iter().filter(|e| e.space == sp.id).count();
            let reserved = o.is_reserved(sp.id);
            d = d.child(
                div()
                    .id(SharedString::from(format!("osw-{}", sp.id)))
                    .on_mouse_down(
                        gpui_kit::MouseButton::Right,
                        super::ctx::on_right(super::ctx::CtxTarget::Space(sp.id), cx),
                    )
                    .child(item(
                        &format!("sp-{}", sp.id),
                        if reserved { "lock" } else { "folder" },
                        crate::i18n::space(&sp.name),
                        Some(n),
                        Filter::Space(sp.id),
                        cx,
                    )),
            );
        }
    }
    // Organization spaces no membership claims (an organization left elsewhere, the demo
    // without its organization): shown together in the single Vault tab.
    let orphans: Vec<_> = org.iter().filter(|sp| s.org_of(sp.id).is_none()).collect();
    if s.scope == Scope::All && !orphans.is_empty() {
        d = d.child(group(tr("Organization")));
        for sp in orphans {
            let n = s.entries.iter().filter(|e| e.space == sp.id).count();
            d = d.child(item(
                &format!("sp-{}", sp.id),
                "folder",
                crate::i18n::space(&sp.name),
                Some(n),
                Filter::Space(sp.id),
                cx,
            ));
        }
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
                .child(tr("Import"))
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
            tr("Trash").into(),
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
        .when(big, |d| d.text_size(TEXT_LG))
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
                icon_button(SharedString::from(format!("cp-{id}")), t, "copy", true)
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
                .on_mouse_down(
                    gpui_kit::MouseButton::Right,
                    super::ctx::on_right(super::ctx::CtxTarget::Entry(id), cx),
                )
                .on_click(cx.listener(move |this, _, window, cx| {
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
                    d.text_color(t.text)
                        .child(format!("{} ↑", crate::i18n::tr_owned(&c.name)))
                })
                .when(c.id.0 != "name", |d| {
                    d.child(crate::i18n::tr_owned(&c.name))
                }),
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
                        for c in &cols {
                            row = row.child(row_cell(&e, c, &all_cols, &t, now, selected, cx));
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
                    .child(list)
                    .test_support(),
            )
        })
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
                let next = scope.unwrap_or(if s.prefs.vault_tabs {
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
