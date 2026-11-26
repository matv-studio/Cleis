//! The vault's sidebar (mockups 35 to 35f, 26/11/2026): the shortcuts chosen for this vault, then
//! one section per vault — clicking its name lists all of it — with its folders below, Import, and
//! Trash beside the Customize button.

use crate::app::{CleisApp, Filter, FolderDialog, Scope, Session, Tab};
use crate::i18n::tr;
use crate::theme::*;
use crate::ui::*;
use cleis_core::{Entry, SpaceId};
use cleis_store::SpaceKind;
use gpui_kit::TestSupportExt;
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, SharedString, div, px};

/// The vault whose shortcuts the sidebar shows (`SidebarShortcuts` keys).
pub fn shortcut_key(s: &Session) -> String {
    match &s.scope {
        Scope::All => "all".into(),
        Scope::Personal => "personal".into(),
        Scope::Org(k) => k.clone(),
    }
}

/// Spaces in the order dragged in the sidebar, then by name.
pub fn ordered<'a>(
    s: &Session,
    spaces: impl Iterator<Item = &'a cleis_store::SpaceInfo>,
) -> Vec<&'a cleis_store::SpaceInfo> {
    let mut v: Vec<_> = spaces.collect();
    let pos = |id: SpaceId| {
        s.prefs
            .space_order
            .iter()
            .position(|x| *x == id)
            .unwrap_or(usize::MAX)
    };
    v.sort_by(|a, b| {
        pos(a.id)
            .cmp(&pos(b.id))
            .then_with(|| cleis_core::text::fold(&a.name).cmp(&cleis_core::text::fold(&b.name)))
    });
    v
}

const SHORTCUTS: [(&str, &str, &str, Filter); 3] = [
    ("f-all", "list", "All items", Filter::All),
    ("f-fav", "star", "Favorites", Filter::Favorites),
    ("f-renew", "clock", "Renew soon", Filter::RenewSoon),
];

pub fn render(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> impl IntoElement {
    let editing = s.customize;
    let count = |f: &dyn Fn(&Entry) -> bool| {
        s.entries
            .iter()
            .filter(|e| s.in_scope(e.space) && f(e))
            .count()
    };
    let key = shortcut_key(s);
    let shown = s.prefs.shortcuts.of(&key);
    let mut d = div()
        .id("sidebar")
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
        .on_mouse_down(
            gpui_kit::MouseButton::Right,
            super::ctx::on_right(super::ctx::CtxTarget::SidebarVoid, cx),
        );
    for (ix, (id, ico, label, filter)) in SHORTCUTS.into_iter().enumerate() {
        if !shown[ix] && !editing {
            continue;
        }
        let n = match filter {
            Filter::All => count(&|_| true),
            Filter::Favorites => count(&|e| e.favorite),
            _ => count(&|e| e.subscription == cleis_core::Subscription::RenewSoon),
        };
        let key = key.clone();
        let row = row(s, t, id, ico, tr(label).into(), Some(n), filter, 0., cx)
            .when(editing && !shown[ix], |r| r.opacity(0.45));
        let row = if editing {
            // Customize: the eye shows or hides the shortcut in this vault.
            row.child(
                icon_button(SharedString::from(format!("{id}-eye")), t, "eye", true)
                    .tooltip(tip(if shown[ix] { tr("Hide") } else { tr("Show") }))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        let key = key.clone();
                        this.update_prefs(|p| p.shortcuts.toggle(&key, ix), cx);
                    }))
                    .test_support(),
            )
        } else {
            row
        };
        d = d.child(row.test_support());
    }

    // My vault: the personal section, its folders below.
    let home = s.vault.home_space();
    if !matches!(s.scope, Scope::Org(_)) {
        let n = count(&|e| s.scope_has(&Scope::Personal, e.space));
        let accent = t.accent_soft;
        d = d.child(div().h(px(10.))).child(
            div()
                .id("personal-drop")
                .rounded(RADIUS_MD)
                .drag_over::<super::dnd::DraggedEntry>(move |d, _, _, _| d.bg(accent))
                .on_drop(
                    cx.listener(move |this, d: &super::dnd::DraggedEntry, _, cx| {
                        this.move_entry(d.id, home, cx)
                    }),
                )
                .child(section(
                    s,
                    t,
                    &format!("sp-{home}"),
                    "user",
                    s.personal_name(),
                    n,
                    Filter::Personal,
                    Some(("new-folder-v".into(), tr("New folder"))),
                    cx,
                )),
        );
        let folders = ordered(
            s,
            s.spaces
                .iter()
                .filter(|sp| !sp.for_contacts && sp.kind == SpaceKind::Personal && sp.id != home),
        );
        for sp in folders {
            d = d.child(folder_row(s, t, sp, true, cx));
        }
    }

    // One section per organization (D-69); its own tab shows only that one.
    let orgs: Vec<&crate::org::OrgSession> = s
        .all_orgs()
        .filter(|o| match &s.scope {
            Scope::All => true,
            Scope::Personal => false,
            Scope::Org(k) => o.m.key() == *k,
        })
        .collect();
    let shown_key = s.org.as_ref().map(|o| o.m.key());
    for o in orgs {
        let okey = o.m.key();
        let h = crate::app::org_hash(&okey);
        let n = count(&|e| s.org_of(e.space).is_some_and(|x| x.m.key() == okey));
        // The organization shown keeps the plain id of its "+".
        let add_id = if shown_key.as_deref() == Some(okey.as_str()) {
            "new-space".to_owned()
        } else {
            format!("new-space-{okey}")
        };
        d = d.child(div().h(px(10.))).child(section(
            s,
            t,
            &format!("org-{okey}"),
            "users",
            o.m.name.clone(),
            n,
            Filter::Org(h),
            o.can(cleis_core::org::Perm::Manage)
                .then_some((add_id, tr("New space"))),
            cx,
        ));
        let spaces = ordered(
            s,
            s.spaces.iter().filter(|sp| {
                !sp.for_contacts
                    && sp.kind == SpaceKind::Organization
                    && o.names.contains_key(&sp.id)
            }),
        );
        for sp in spaces {
            d = d.child(folder_row(s, t, sp, false, cx));
        }
    }
    // Spaces no membership claims (an organization left elsewhere, the demo without its
    // organization): together in the single Vault tab.
    let orphans: Vec<_> = s
        .spaces
        .iter()
        .filter(|sp| {
            !sp.for_contacts && sp.kind == SpaceKind::Organization && s.org_of(sp.id).is_none()
        })
        .collect();
    if s.scope == Scope::All && !orphans.is_empty() {
        d = d.child(
            div()
                .pt(px(12.))
                .pb(px(4.))
                .px(px(8.))
                .text_size(TEXT_XS)
                .text_color(t.text_muted)
                .font_weight(W_MEDIUM)
                .child(tr("Organization")),
        );
        for sp in orphans {
            d = d.child(folder_row(s, t, sp, false, cx));
        }
    }

    let import = div()
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
        .test_support();
    // Customize (mockup 35f): beside Trash, its tile centred on the row.
    let custom = div()
        .id("customize")
        .flex_none()
        .size(CONTROL_MD)
        .flex()
        .items_center()
        .justify_center()
        .rounded(RADIUS_MD)
        .cursor_pointer()
        .when(editing, |d| d.bg(t.accent_soft))
        .hover(|d| d.bg(t.surface_hover))
        .child(icon(
            "grip",
            16.,
            if editing { t.accent_text } else { t.text_muted },
        ))
        .tooltip(tip(tr("Customize: reorder and delete")))
        .on_click(cx.listener(|this, _, _, cx| this.toggle_customize(cx)))
        .test_support();
    d.child(div().flex_1())
        .child(import)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(2.))
                .child(
                    div().flex_1().min_w_0().child(
                        row(
                            s,
                            t,
                            "f-trash",
                            "trash",
                            tr("Trash").into(),
                            (!s.trash.is_empty()).then_some(s.trash.len()),
                            Filter::Trash,
                            0.,
                            cx,
                        )
                        .test_support(),
                    ),
                )
                .child(custom),
        )
        .test_support()
}

#[allow(clippy::too_many_arguments)]
fn row(
    s: &Session,
    t: &Tokens,
    id: &str,
    ico: &str,
    label: String,
    n: Option<usize>,
    filter: Filter,
    indent: f32,
    cx: &mut Context<CleisApp>,
) -> gpui_kit::Stateful<Div> {
    let active = s.filter == filter;
    div()
        .id(SharedString::from(id.to_owned()))
        .flex()
        .items_center()
        .gap(px(8.))
        .h(CONTROL_MD)
        .pl(px(8. + indent))
        .pr(px(8.))
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
}

/// A vault's section row: its name lists all of it; "+" adds a folder or a space.
#[allow(clippy::too_many_arguments)]
fn section(
    s: &Session,
    t: &Tokens,
    id: &str,
    ico: &str,
    label: String,
    n: usize,
    filter: Filter,
    add: Option<(String, &'static str)>,
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    let org_key = id.strip_prefix("org-").map(str::to_owned);
    let active = s.filter == filter;
    row(s, t, id, ico, label, Some(n), filter, 0., cx)
        .font_weight(W_MEDIUM)
        .when(!active, |d| d.text_color(t.text))
        .when_some(add, |d, (add_id, tipt)| {
            d.child(
                icon_button(SharedString::from(add_id), t, "plus", true)
                    .tooltip(tip(tipt))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        match &org_key {
                            Some(k) => {
                                this.activate_org(k);
                                this.open_new_space(window, cx);
                            }
                            None => this.open_new_folder(window, cx),
                        }
                    }))
                    .test_support(),
            )
        })
        .test_support()
}

/// A folder (personal) or a space (organization), below its section.
fn folder_row(
    s: &Session,
    t: &Tokens,
    sp: &cleis_store::SpaceInfo,
    personal: bool,
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    let id = sp.id;
    let n = s.entries.iter().filter(|e| e.space == id).count();
    let reserved = s.org_of(id).is_some_and(|o| o.is_reserved(id));
    let gname = SharedString::from(format!("pf-{id}"));
    let menu_open = personal && s.folder_menu == Some(id);
    let editing = s.customize;
    let label = crate::i18n::space(&sp.name);
    let inner = row(
        s,
        t,
        &format!("sp-{id}"),
        if reserved { "lock" } else { "folder" },
        label,
        (!editing).then_some(n),
        Filter::Space(id),
        16.,
        cx,
    )
    .when(editing, |r| r.pl(px(4.)));
    let inner = if editing {
        // Customize: a handle to drag, a cross to delete (personal folders).
        inner
            .child(div().flex_none().text_color(t.text_faint).child(icon(
                "grip",
                14.,
                t.text_faint,
            )))
            .when(personal, |r| {
                r.child(remove_badge(
                    &format!("rm-folder-{id}"),
                    t,
                    tr("Delete folder"),
                    cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        if let Some(s) = this.session() {
                            s.folder = Some(FolderDialog::Delete(id));
                        }
                        cx.notify();
                    }),
                ))
            })
    } else {
        inner
    };
    let accent = t.accent_soft;
    let line = t.primary;
    let ix = ordered(s, s.spaces.iter().filter(|x| !x.for_contacts))
        .iter()
        .position(|x| x.id == id)
        .unwrap_or(0);
    let dragged = super::dnd::DraggedSpace {
        id,
        name: sp.name.clone().into(),
        ix,
    };
    div()
        .id(SharedString::from(format!("pfw-{id}")))
        .relative()
        .group(gname.clone())
        .rounded(RADIUS_MD)
        // Mockups 35c, 35d: an item dropped here moves into it; a folder takes its place.
        .on_drag(dragged, super::dnd::space_ghost)
        .drag_over::<super::dnd::DraggedEntry>(move |d, _, _, _| d.bg(accent))
        .drag_over::<super::dnd::DraggedSpace>(move |d, from, _, _| {
            super::dnd::drop_line(d, from.ix, ix, line)
        })
        .on_drop(
            cx.listener(move |this, d: &super::dnd::DraggedEntry, _, cx| {
                this.move_entry(d.id, id, cx)
            }),
        )
        .on_drop(
            cx.listener(move |this, d: &super::dnd::DraggedSpace, _, cx| {
                this.reorder_space(d.id, id, cx)
            }),
        )
        .on_mouse_down(
            gpui_kit::MouseButton::Right,
            super::ctx::on_right(super::ctx::CtxTarget::Space(id), cx),
        )
        .child(inner.test_support())
        .when(personal && !editing, |d| {
            d.child(
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
                    .child(super::folders::more_button(id, t, cx)),
            )
        })
        .when(menu_open, |d| {
            d.child(gpui_kit::deferred(super::folders::menu(s, id, t, cx)))
        })
        .test_support()
}

/// The red cross of the customize mode, centred in its disc.
pub fn remove_badge(
    id: &str,
    t: &Tokens,
    tooltip: &'static str,
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.to_owned()))
        .flex_none()
        .size(px(18.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(t.danger_solid)
        .cursor_pointer()
        .child(icon("x", 10., gpui_kit::white()))
        .tooltip(tip(tooltip))
        .on_click(on_click)
        .test_support()
}
