//! Screen 20 — Contacts: folders, labels, search, table or cards, inspector with inline editing;
//! screen 21 — Merge & fix. vCard import and export.

use crate::app::{CleisApp, Confirm, ConfirmAction, Session};
use crate::contact_editor::ContactEditor;
use crate::i18n::tr;
use crate::theme::*;
use crate::ui::*;
use cleis_core::column::{ColumnDef, Table};
use cleis_core::contact_edit::{Field, List};
use cleis_core::dedupe::{self, DuplicateGroup, Reason};
use cleis_core::text::{fold, matches};
use cleis_core::{Contact, ContactId, SpaceId};
use cleis_store::SpaceKind;
use gpui_kit::TestSupportExt;
use gpui_kit::component::Sizable;
use gpui_kit::component::input::{InputState, Textarea};
use gpui_kit::prelude::*;
use gpui_kit::{
    AnyElement, Context, Div, Entity, PathPromptOptions, SharedString, Subscription, Window,
    deferred, div, px,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ContactFilter {
    #[default]
    All,
    Favorites,
    /// Birthdays in the next 30 days.
    Birthdays,
    Folder(SpaceId),
    Label(String),
    Merge,
    /// "My contacts": every personal contact (27/11/2026, as My vault).
    Mine,
    /// An organization's section: all its contacts (its key).
    Org(String),
    /// Contacts outside any folder: the personal ones not in a folder of their own, and each
    /// organization's "Uncategorized" (a shortcut, with All contacts).
    Uncategorized,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContactMenu {
    More,
    /// The inspector's avatar: choose or remove the photo (mockup 36e).
    Photo,
    AddLabel,
    Link,
}

#[derive(Default)]
pub struct ContactsUi {
    pub filter: ContactFilter,
    pub search: Option<Entity<InputState>>,
    pub query: String,
    pub editor: Option<ContactEditor>,
    /// 0 Details, 1 Linked, 2 History.
    pub tab: u8,
    pub menu: Option<ContactMenu>,
    pub cards: bool,
    /// Box of the new folder being named (sidebar +).
    pub new_folder: Option<Entity<InputState>>,
    /// The organization the folder being named goes to (`None`: personal).
    pub new_folder_org: Option<String>,
    /// Merge & fix: group opened for review, and the contact kept.
    pub review: Option<(Vec<ContactId>, ContactId)>,
    pub focus_next: Option<Field>,
    pub subs: Vec<Subscription>,
}

fn label_tone(l: &str) -> Tone {
    match l {
        "Client" => Tone::Set,
        "VIP" => Tone::Warn,
        "Prospect" => Tone::Auto,
        "Team" => Tone::Ok,
        _ => Tone::Neutral,
    }
}

fn tip(
    text: &'static str,
) -> impl Fn(&mut Window, &mut gpui_kit::App) -> gpui_kit::AnyView + 'static {
    move |window, cx| {
        gpui_kit::component::tooltip::Tooltip::new(crate::i18n::tr_dynamic(text)).build(window, cx)
    }
}

fn today_md() -> (u8, u8) {
    let (_, m, d) = crate::format::civil(cleis_core::now().div_euclid(86_400));
    (m as u8, d as u8)
}

fn birthday_soon(c: &Contact) -> bool {
    let (m, d) = today_md();
    c.birthday.is_some_and(|b| b.days_until(m, d) <= 30)
}

pub fn duplicates(s: &Session) -> Vec<DuplicateGroup> {
    dedupe::find_duplicates(&s.contacts, &s.prefs.not_duplicates)
}

/// Contacts listed with the current folder, label and search, sorted by name (birthdays: by
/// date).
pub fn listed(s: &Session) -> Vec<&Contact> {
    let ui = &s.contacts_ui;
    let q = ui.query.trim();
    let mut v: Vec<&Contact> = s
        .contacts
        .iter()
        .filter(|c| match &ui.filter {
            ContactFilter::All | ContactFilter::Merge => true,
            ContactFilter::Favorites => c.favorite,
            ContactFilter::Birthdays => birthday_soon(c),
            ContactFilter::Folder(sp) => c.space == *sp,
            ContactFilter::Label(l) => c.labels.contains(l),
            ContactFilter::Mine => s.org_of(c.space).is_none() && is_personal(s, c.space),
            ContactFilter::Org(k) => s.org_of(c.space).is_some_and(|o| o.m.key() == *k),
            ContactFilter::Uncategorized => is_uncategorized(s, c.space),
        })
        .filter(|c| q.is_empty() || matches(&c.search_text(), q))
        .collect();
    if ui.filter == ContactFilter::Birthdays {
        let (m, d) = today_md();
        v.sort_by_key(|c| c.birthday.map(|b| b.days_until(m, d)).unwrap_or(999));
    } else {
        v.sort_by_key(|c| fold(&c.display_name()));
    }
    v
}

/// A personal contacts space.
fn is_personal(s: &Session, sp: SpaceId) -> bool {
    s.spaces
        .iter()
        .any(|x| x.id == sp && x.kind == SpaceKind::Personal)
}

/// An organization's "Uncategorized" contacts space.
fn is_unfiled(s: &Session, sp: SpaceId) -> bool {
    s.spaces.iter().any(|x| x.id == sp && x.unfiled)
}

/// Outside any folder: the default personal contacts space, or an organization's
/// "Uncategorized".
fn is_uncategorized(s: &Session, sp: SpaceId) -> bool {
    Some(sp) == personal_default(s) || is_unfiled(s, sp)
}

/// The personal contacts space new contacts go to by default ("My contacts").
fn personal_default(s: &Session) -> Option<SpaceId> {
    s.spaces
        .iter()
        .find(|sp| sp.for_contacts && sp.kind == SpaceKind::Personal)
        .map(|sp| sp.id)
}

/// The organization's "Uncategorized" space, if made.
pub fn unfiled_of(s: &Session, org: &str) -> Option<SpaceId> {
    s.spaces
        .iter()
        .find(|sp| sp.unfiled && s.org_of(sp.id).is_some_and(|o| o.m.key() == org))
        .map(|sp| sp.id)
}

/// The organization a contacts filter shows (its section, "Uncategorized", one of its
/// folders).
fn filter_org(s: &Session, f: &ContactFilter) -> Option<String> {
    match f {
        ContactFilter::Org(k) => Some(k.clone()),
        ContactFilter::Folder(sp) => s.org_of(*sp).map(|o| o.m.key()),
        _ => None,
    }
}

fn all_labels(s: &Session) -> Vec<(String, usize)> {
    let mut m: std::collections::BTreeMap<String, usize> = Default::default();
    for c in &s.contacts {
        for l in &c.labels {
            *m.entry(l.clone()).or_default() += 1;
        }
    }
    m.into_iter().collect()
}

fn side_item(
    id: impl Into<SharedString>,
    t: &Tokens,
    ico: Option<&str>,
    label: impl Into<SharedString>,
    count: Option<usize>,
    active: bool,
) -> gpui_kit::Stateful<Div> {
    let fg = if active { t.accent_text } else { t.text_muted };
    div()
        .id(gpui_kit::ElementId::Name(id.into()))
        .flex()
        .items_center()
        .gap(px(8.))
        .h(CONTROL_MD)
        .px(px(8.))
        .rounded(RADIUS_MD)
        .cursor_pointer()
        .text_color(fg)
        .when(active, |d| d.bg(t.accent_soft))
        .when(!active, |d| d.hover(|s| s.bg(t.surface_hover)))
        .when_some(ico, |d, i| d.child(icon(i, 16., fg)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .child(crate::i18n::tr_owned(&label.into())),
        )
        .when_some(count, |d, n| {
            d.child(
                div()
                    .text_size(TEXT_SM)
                    .text_color(t.text_faint)
                    .child(n.to_string()),
            )
        })
}

fn group_head(t: &Tokens, label: &str) -> Div {
    div()
        .flex()
        .items_center()
        .h(px(28.))
        .px(px(8.))
        .mt(px(8.))
        .text_size(TEXT_XS)
        .font_weight(W_MEDIUM)
        .text_color(t.text_muted)
        .child(div().flex_1().child(crate::i18n::tr_owned(label)))
}

fn sidebar(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> gpui_kit::Stateful<Div> {
    let ui = &s.contacts_ui;
    let f = &ui.filter;
    let set = |filter: ContactFilter| {
        move |this: &mut CleisApp,
              _: &gpui_kit::ClickEvent,
              _: &mut Window,
              cx: &mut Context<CleisApp>| {
            if let Some(s) = this.session() {
                s.contacts_ui.filter = filter.clone();
                s.contacts_ui.review = None;
            }
            cx.notify();
        }
    };
    let mut side = div()
        .id("contacts-sidebar")
        .on_mouse_down(
            gpui_kit::MouseButton::Right,
            super::ctx::on_right(super::ctx::CtxTarget::ContactsSidebar, cx),
        )
        .w(SIDEBAR_W)
        .flex_none()
        .h_full()
        .bg(t.surface)
        .border_r_1()
        .border_color(t.line)
        .p(px(8.))
        .flex()
        .flex_col()
        .gap(px(2.));
    // 27/11/2026: shown or hidden in Preferences › Shortcuts › Contacts.
    let on = s.prefs.contact_shortcuts;
    if on[0] {
        side = side.child(
            side_item(
                "cf-all",
                t,
                Some("users"),
                tr("All contacts"),
                Some(s.contacts.len()),
                *f == ContactFilter::All,
            )
            .on_click(cx.listener(set(ContactFilter::All)))
            .test_support(),
        );
    }
    if on[1] {
        side = side.child(
            side_item(
                "cf-fav",
                t,
                Some("star"),
                tr("Favorites"),
                Some(s.contacts.iter().filter(|c| c.favorite).count()),
                *f == ContactFilter::Favorites,
            )
            .on_click(cx.listener(set(ContactFilter::Favorites)))
            .test_support(),
        );
    }
    if on[2] {
        side = side.child(
            side_item(
                "cf-bday",
                t,
                Some("calendar"),
                tr("Birthdays soon"),
                Some(s.contacts.iter().filter(|c| birthday_soon(c)).count()),
                *f == ContactFilter::Birthdays,
            )
            .tooltip(tip(tr("Next 30 days")))
            .on_click(cx.listener(set(ContactFilter::Birthdays)))
            .test_support(),
        );
    }
    if on[3] {
        side = side.child(
            side_item(
                "cf-unfiled",
                t,
                Some("list"),
                tr("Uncategorized"),
                Some(
                    s.contacts
                        .iter()
                        .filter(|c| is_uncategorized(s, c.space))
                        .count(),
                ),
                *f == ContactFilter::Uncategorized,
            )
            .tooltip(tip(tr("Contacts outside any folder")))
            .on_click(cx.listener(set(ContactFilter::Uncategorized)))
            .test_support(),
        );
    }
    let count = |sp: SpaceId| s.contacts.iter().filter(|c| c.space == sp).count();
    let new_folder_row = |side: gpui_kit::Stateful<Div>, org: Option<&str>| match &ui.new_folder {
        Some(input) if ui.new_folder_org.as_deref() == org => side.child(
            div()
                .pl(px(20.))
                .pr(px(4.))
                .child(Input::new(input).id("new-folder-name").small()),
        ),
        _ => side,
    };
    // 27/11/2026 (mockups 44): sections as in the Vault, their folders below.
    let section = |id: String,
                   ico: &'static str,
                   label: SharedString,
                   n: usize,
                   filter: ContactFilter,
                   add: Option<(String, Option<String>)>,
                   cx: &mut Context<CleisApp>| {
        let active = *f == filter;
        side_item(id, t, Some(ico), label, Some(n), active)
            .font_weight(W_MEDIUM)
            .when(!active, |d| d.text_color(t.text))
            .when_some(add, |d, (add_id, org)| {
                d.child(
                    icon_button(SharedString::from(add_id), t, "plus", true)
                        .tooltip(tip(tr("New folder")))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.start_new_folder_in(org.clone(), window, cx);
                        }))
                        .test_support(),
                )
            })
            .on_click(cx.listener(set(filter)))
            .test_support()
    };
    let sub = |id: String,
               ico: &'static str,
               label: SharedString,
               n: usize,
               filter: ContactFilter,
               cx: &mut Context<CleisApp>| {
        side_item(id, t, Some(ico), label, Some(n), *f == filter)
            .pl(px(24.))
            .on_click(cx.listener(set(filter)))
            .test_support()
    };
    // My contacts: every personal contact; its other folders below.
    let mine = s
        .contacts
        .iter()
        .filter(|c| s.org_of(c.space).is_none() && is_personal(s, c.space))
        .count();
    side = side.child(div().h(px(10.))).child(section(
        "cf-mine".into(),
        "user",
        tr("My contacts").into(),
        mine,
        ContactFilter::Mine,
        Some(("new-folder".into(), None)),
        cx,
    ));
    let default = personal_default(s);
    for sp in s
        .spaces
        .iter()
        .filter(|sp| sp.for_contacts && sp.kind == SpaceKind::Personal && Some(sp.id) != default)
    {
        side = side.child(sub(
            format!("cf-folder-{}", sp.id),
            "folder",
            crate::i18n::space(&sp.name).into(),
            count(sp.id),
            ContactFilter::Folder(sp.id),
            cx,
        ));
    }
    side = new_folder_row(side, None);
    // One section per organization (D-69): all its contacts, "Uncategorized", its folders.
    for o in s.all_orgs() {
        let k = o.m.key();
        let n = s
            .contacts
            .iter()
            .filter(|c| s.org_of(c.space).is_some_and(|x| x.m.key() == k))
            .count();
        let manage = o.can(cleis_core::org::Perm::Manage);
        side = side.child(div().h(px(10.))).child(section(
            format!("cf-org-{k}"),
            if o.pin.is_some() { "lock" } else { "users" },
            o.m.name.clone().into(),
            n,
            ContactFilter::Org(k.clone()),
            manage.then(|| (format!("new-folder-{k}"), Some(k.clone()))),
            cx,
        ));
        for sp in s
            .spaces
            .iter()
            .filter(|sp| sp.for_contacts && !sp.unfiled && o.names.contains_key(&sp.id))
        {
            side = side.child(sub(
                format!("cf-folder-{}", sp.id),
                "folder",
                crate::i18n::space(&sp.name).into(),
                count(sp.id),
                ContactFilter::Folder(sp.id),
                cx,
            ));
        }
        side = new_folder_row(side, Some(k.as_str()));
    }
    // Folders no membership claims (an organization left, the demo without its organization).
    let orphans: Vec<_> = s
        .spaces
        .iter()
        .filter(|sp| {
            sp.for_contacts && sp.kind == SpaceKind::Organization && s.org_of(sp.id).is_none()
        })
        .collect();
    if !orphans.is_empty() {
        side = side.child(group_head(t, tr("Shared")));
        for sp in orphans {
            side = side.child(sub(
                format!("cf-folder-{}", sp.id),
                "folder",
                crate::i18n::space(&sp.name).into(),
                count(sp.id),
                ContactFilter::Folder(sp.id),
                cx,
            ));
        }
    }
    let labels = all_labels(s);
    if !labels.is_empty() {
        side = side.child(group_head(t, tr("Labels")));
        for (l, n) in labels {
            let tone = label_tone(&l);
            let dot = match tone {
                Tone::Set => t.set_text,
                Tone::Warn => t.warning_text,
                Tone::Auto => t.auto_text,
                Tone::Ok => t.success_text,
                _ => t.text_faint,
            };
            let active = *f == ContactFilter::Label(l.clone());
            let fg = if active { t.accent_text } else { t.text_muted };
            side = side.child(
                div()
                    .id(SharedString::from(format!("cf-label-{l}")))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .h(CONTROL_MD)
                    .px(px(8.))
                    .rounded(RADIUS_MD)
                    .cursor_pointer()
                    .text_color(fg)
                    .when(active, |d| d.bg(t.accent_soft))
                    .when(!active, |d| d.hover(|s| s.bg(t.surface_hover)))
                    .child(
                        div()
                            .w(px(16.))
                            .flex()
                            .justify_center()
                            .child(div().size(px(8.)).rounded_full().bg(dot)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(l.clone()),
                    )
                    .child(
                        div()
                            .text_size(TEXT_SM)
                            .text_color(t.text_faint)
                            .child(n.to_string()),
                    )
                    .on_click(cx.listener(set(ContactFilter::Label(l.clone()))))
                    .test_support(),
            );
        }
    }
    let dups = duplicates(s).len();
    side.child(div().flex_1())
        .child(
            side_item(
                "cf-merge",
                t,
                Some("users"),
                tr("Merge & fix"),
                None,
                *f == ContactFilter::Merge,
            )
            .when(dups > 0, |d| {
                d.child(chip(t, Tone::Auto, dups.to_string(), false))
            })
            .tooltip(tip(tr("Possible duplicates")))
            .on_click(cx.listener(set(ContactFilter::Merge)))
            .test_support(),
        )
        .child(
            side_item("cf-import", t, Some("upload"), tr("Import"), None, false)
                .tooltip(tip(tr("vCard (.vcf) or CSV (Google, Outlook)")))
                .on_click(cx.listener(|this, _, _, cx| this.choose_vcard(cx)))
                .test_support(),
        )
}

fn cell(w: f32) -> Div {
    let d = div()
        .h_full()
        .px(px(12.))
        .flex()
        .items_center()
        .overflow_hidden()
        .whitespace_nowrap();
    col_sized(d, (w > 0.).then_some(w))
}

fn labels_row(t: &Tokens, c: &Contact) -> Div {
    let mut d = div().flex().gap(px(4.)).overflow_hidden();
    for l in &c.labels {
        d = d.child(chip(t, label_tone(l), l.clone(), false));
    }
    d
}

fn select(
    id: ContactId,
) -> impl Fn(&mut CleisApp, &gpui_kit::ClickEvent, &mut Window, &mut Context<CleisApp>) {
    move |this, _, window, cx| {
        // The list below must not take it as a click in the void.
        cx.stop_propagation();
        if let Some(s) = this.session() {
            s.selected_contact = Some(id);
            s.contacts_ui.menu = None;
            let f = s.main_focus.clone();
            window.focus(&f, cx);
        }
        cx.notify();
    }
}

/// Width of a contact column: dragged, kept, or as in mockup 09 (0: the rest of the row).
fn contact_col_width(s: &Session, c: &ColumnDef) -> f32 {
    let w = default_width(c);
    super::dnd::width(s, c, (w > 0.).then_some(w)).unwrap_or(0.)
}

fn default_width(c: &ColumnDef) -> f32 {
    match c.id.0.as_str() {
        "name" => 210.,
        "company" | "phone" | "job_title" | "folder" => 150.,
        "labels" | "website" => 160.,
        "city" => 120.,
        "birthday" => 100.,
        "email" | "notes" => 0.,
        _ => 150.,
    }
}

fn contact_cell(
    s: &Session,
    c: &Contact,
    col: &ColumnDef,
    selected: bool,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> AnyElement {
    let id = c.id;
    let base = cell(contact_col_width(s, col));
    match col.id.0.as_str() {
        "name" => base
            .gap(px(8.))
            .child(contact_avatar(t, c, 22.))
            .child(
                div()
                    .font_weight(W_MEDIUM)
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(c.display_name()),
            )
            .when(c.favorite, |d| d.child(icon("star", 12., t.warning_text)))
            .into_any_element(),
        "email" => {
            let email = c.primary_email().unwrap_or_default();
            base.text_color(t.text_muted)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(email.clone()),
                )
                .when(!email.is_empty(), |d| {
                    d.child(
                        cell_button(format!("ce-{id}"), t, "copy")
                            .tooltip(|window, cx| {
                                gpui_kit::component::tooltip::Tooltip::new(tr("Copy e-mail"))
                                    .build(window, cx)
                            })
                            .when(!selected, |d| {
                                d.invisible().group_hover("crow", |s| s.visible())
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.copy(email.clone(), "Email", cx);
                            })),
                    )
                })
                .into_any_element()
        }
        // Mockup 46d: the site opened from the list, as in the vault.
        "website" => {
            let url = c
                .cell(col)
                .map(|v| v.display(&col.kind))
                .unwrap_or_default();
            if url.is_empty() {
                return base.into_any_element();
            }
            let shown = url
                .split_once("://")
                .map_or(url.as_str(), |(_, rest)| rest)
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
                    cell_button(format!("cw-{id}"), t, "external")
                        .tooltip(move |window, cx| {
                            gpui_kit::component::tooltip::Tooltip::new(tip_text.clone())
                                .build(window, cx)
                        })
                        .when(!selected, |d| {
                            d.invisible().group_hover("crow", |s| s.visible())
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.open_site(&url, cx);
                        }))
                        .test_support(),
                )
                .into_any_element()
        }
        "labels" => base.child(labels_row(t, c)).into_any_element(),
        "company" => base
            .child(if c.company.is_empty() {
                "—".to_owned()
            } else {
                c.company.clone()
            })
            .into_any_element(),
        "birthday" => base
            .text_color(t.text_muted)
            .child(c.birthday.map(|b| b.display()).unwrap_or_default())
            .into_any_element(),
        "folder" => base
            .text_color(t.text_muted)
            .child(
                s.spaces
                    .iter()
                    .find(|sp| sp.id == c.space)
                    .map(|sp| crate::i18n::space(&sp.name))
                    .unwrap_or_default(),
            )
            .into_any_element(),
        _ => {
            let text = c
                .cell(col)
                .map(|v| v.display(&col.kind))
                .unwrap_or_default();
            let text =
                if matches!(col.kind, cleis_core::column::ColumnKind::Secret) && !text.is_empty() {
                    "••••••".to_owned()
                } else {
                    text
                };
            base.text_color(t.text_muted)
                .child(div().overflow_hidden().text_ellipsis().child(text))
                .into_any_element()
        }
    }
}

fn table(
    s: &Session,
    list: &[&Contact],
    sel: Option<ContactId>,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> Div {
    let folder = match s.contacts_ui.filter {
        ContactFilter::Folder(f) => Some(f),
        _ => None,
    };
    let cols: Vec<ColumnDef> =
        cleis_core::column::visible_columns(&s.columns, Table::Contacts, folder)
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
        .text_size(TEXT_XS)
        .font_weight(W_MEDIUM)
        .text_color(t.text_muted);
    let ncols = cols.len();
    for (ci, c) in cols.iter().enumerate() {
        head = head.child(
            cell(contact_col_width(s, c))
                .relative()
                .on_drag_move(super::dnd::on_resize_move(c.table, &c.id, cx))
                .child(super::dnd::col_grip(c.table, &c.id, ci + 1 < ncols, t, cx))
                .when(c.id.0 == "name", |d| {
                    d.text_color(t.text)
                        .child(format!("{} ↑", crate::i18n::tr_owned(&c.name)))
                })
                .when(c.id.0 != "name", |d| {
                    d.child(crate::i18n::tr_owned(&c.name))
                }),
        );
    }
    // A click below the rows lets go of the contact and closes its panel.
    let mut rows = div()
        .id("contacts")
        .flex_1()
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .on_click(cx.listener(|this, _, _, cx| this.deselect_contact(cx)))
        .on_mouse_down(
            gpui_kit::MouseButton::Right,
            super::ctx::on_right(super::ctx::CtxTarget::ContactsVoid, cx),
        );
    for c in list {
        let id = c.id;
        let selected = sel == Some(id);
        let mut row = div()
            .id(SharedString::from(format!("c-{id}")))
            .group("crow")
            .w_full()
            .h(ROW_H)
            .flex_none()
            .flex()
            .border_b_1()
            .border_color(t.line)
            .cursor_pointer()
            .when(selected, |d| d.bg(t.selection))
            .when(!selected, |d| d.hover(|s| s.bg(t.surface_hover)))
            .on_mouse_down(
                gpui_kit::MouseButton::Right,
                super::ctx::on_right(super::ctx::CtxTarget::Contact(id), cx),
            )
            .on_click(cx.listener(select(id)));
        for col in &cols {
            row = row.child(contact_cell(s, c, col, selected, t, cx));
        }
        if s.aimed(super::ctx::CtxTarget::Contact(id)) {
            row = row.relative().child(super::ctx::aim_ring(t, RADIUS_SM));
        }
        rows = rows.child(row.test_support());
    }
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(head)
        // Another folder or section: its contacts come in, as the vault's items (27/11/2026).
        .child(crate::ui::page_in(
            format!("crows-{:?}", s.contacts_ui.filter),
            div().flex_1().min_h_0().flex().flex_col().child(rows),
        ))
}

fn cards(
    list: &[&Contact],
    sel: Option<ContactId>,
    aimed: Option<ContactId>,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> Div {
    let mut grid = div().flex().flex_wrap().gap(px(12.)).p(px(12.));
    for c in list {
        let id = c.id;
        let selected = sel == Some(id);
        let sub = [c.job_title.as_str(), c.company.as_str()]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(" · ");
        grid = grid.child(
            div()
                .id(SharedString::from(format!("card-{id}")))
                .relative()
                .when(aimed == Some(id), |d| {
                    d.child(super::ctx::aim_ring(t, RADIUS_LG))
                })
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
                    super::ctx::on_right(super::ctx::CtxTarget::Contact(id), cx),
                )
                .on_click(cx.listener(select(id)))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(contact_avatar(t, c, 32.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .font_weight(W_MEDIUM)
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .child(c.display_name()),
                                )
                                .child(
                                    div()
                                        .text_size(TEXT_SM)
                                        .text_color(t.text_muted)
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .child(sub),
                                ),
                        )
                        .when(c.favorite, |d| d.child(icon("star", 12., t.warning_text))),
                )
                .child(
                    div()
                        .text_size(TEXT_SM)
                        .text_color(t.text_muted)
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(c.primary_email().unwrap_or_default()),
                )
                .child(
                    div()
                        .text_size(TEXT_SM)
                        .text_color(t.text_muted)
                        .child(c.primary_phone().unwrap_or_default()),
                )
                .child(labels_row(t, c))
                .test_support(),
        );
    }
    div().flex_1().min_h_0().flex().flex_col().child(
        div()
            .id("contact-cards")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .on_click(cx.listener(|this, _, _, cx| this.deselect_contact(cx)))
            .on_mouse_down(
                gpui_kit::MouseButton::Right,
                super::ctx::on_right(super::ctx::CtxTarget::ContactsVoid, cx),
            )
            .child(grid)
            .test_support(),
    )
}

pub fn page(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let ui = &s.contacts_ui;
    let list = listed(s);
    let sel = ui.editor.as_ref().map(|e| e.contact).or(s.selected_contact);
    let center: AnyElement = if ui.filter == ContactFilter::Merge {
        merge_page(s, t, cx).into_any_element()
    } else {
        let toolbar =
            div()
                .h(px(44.))
                .flex_none()
                .flex()
                .items_center()
                .gap(px(8.))
                .px(px(12.))
                .border_b_1()
                .border_color(t.line)
                .when_some(ui.search.as_ref(), |d, st| {
                    d.child(div().w(px(280.)).child(
                        Input::new(st).id("contact-search").prefix(icon(
                            "search",
                            16.,
                            t.text_muted,
                        )),
                    ))
                })
                .child(
                    div()
                        .relative()
                        .child(
                            button("contact-columns", t, BtnKind::Ghost, false)
                                .when(s.columns_menu.is_some(), |d| d.bg(t.surface_hover))
                                .child(icon("columns", 16., t.text_muted))
                                .child(tr("Columns"))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.toggle_columns_menu(Table::Contacts, cx)
                                }))
                                .test_support(),
                        )
                        .when_some(
                            s.columns_menu
                                .as_ref()
                                .filter(|m| m.table == Table::Contacts),
                            |d, m| d.child(deferred(super::columns::menu(s, m, t, cx))),
                        ),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .flex()
                        .p(px(2.))
                        .gap(px(2.))
                        .bg(t.bg)
                        .border_1()
                        .border_color(t.line_strong)
                        .rounded(RADIUS_MD)
                        .children([(false, "list", "Table"), (true, "columns", "Cards")].map(
                            |(cards_view, ico, name)| {
                                let on = ui.cards == cards_view;
                                div()
                                    .id(SharedString::from(format!("view-{name}")))
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
                                    .tooltip(tip(name))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if let Some(s) = this.session() {
                                            s.contacts_ui.cards = cards_view;
                                        }
                                        cx.notify();
                                    }))
                                    .test_support()
                            },
                        )),
                );
        let body = if ui.cards {
            let aimed = s.ctx.as_ref().and_then(|m| match m.target {
                super::ctx::CtxTarget::Contact(id) => Some(id),
                _ => None,
            });
            crate::ui::page_in(
                format!("ccards-{:?}", ui.filter),
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(cards(&list, sel, aimed, t, cx)),
            )
            .into_any_element()
        } else {
            table(s, &list, sel, t, cx).into_any_element()
        };
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(toolbar)
            .child(body)
            .into_any_element()
    };
    let shown = sel.and_then(|id| s.contacts.iter().find(|c| c.id == id));
    let crumb = shown
        .filter(|_| ui.filter != ContactFilter::Merge)
        .map(|c| {
            let folder = s
                .spaces
                .iter()
                .find(|sp| sp.id == c.space)
                .map(|sp| crate::i18n::space(&sp.name))
                .unwrap_or_default();
            format!("{folder} › {}", c.display_name())
        })
        .unwrap_or_else(|| crate::trf!("{} contacts", list.len()));
    let crumb = if ui.filter == ContactFilter::Merge {
        tr("Merge & fix").to_owned()
    } else {
        crumb
    };
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
                .child(center)
                .when(ui.filter != ContactFilter::Merge, |d| {
                    d.when_some(shown.zip(ui.editor.as_ref()), |d, (c, ed)| {
                        d.child(inspector(s, c, ed, t, cx))
                    })
                }),
        )
        .child(
            div()
                .h(STATUSBAR_H)
                .flex_none()
                .flex()
                .items_center()
                .gap(px(12.))
                .px(px(12.))
                .border_t_1()
                .border_color(t.line)
                .bg(t.surface)
                .text_size(TEXT_SM)
                .text_color(t.text_muted)
                .child(div().flex_1().child(crumb))
                .child(
                    div()
                        .id("enc")
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .child(icon("shield", 12., t.text_muted))
                        .child(tr("End-to-end encryption"))
                        .tooltip(tip(tr("Contacts are encrypted like passwords"))),
                ),
        )
}

/// Box with a red border and message when refused (as in the vault inspector).
fn boxed(t: &Tokens, ed: &ContactEditor, f: Field, input: impl IntoElement) -> Div {
    let err = ed.errors.get(&f).cloned();
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(
            div()
                .rounded(RADIUS_MD)
                .when(err.is_some(), |d| d.border_1().border_color(t.danger_text))
                .child(input),
        )
        .when_some(err, |d, e| {
            d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e))
        })
}

fn field_head(t: &Tokens, label: &str, add: Option<impl IntoElement>) -> Div {
    div()
        .flex()
        .items_center()
        .text_size(TEXT_XS)
        .text_color(t.text_muted)
        .font_weight(W_MEDIUM)
        .child(div().flex_1().child(crate::i18n::tr_owned(label)))
        .when_some(add, |d, b| d.child(b))
}

/// The inspector's avatar (mockup 36e): the pointer over it shows the upload sign; a click opens
/// Choose a photo… / Remove photo.
fn photo_button(
    c: &Contact,
    open: bool,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    let id = c.id;
    let has = c.photo.is_some();
    let menu = menu_box(t)
        .top(px(44.))
        .left_0()
        .min_w(px(200.))
        .child(
            menu_item(
                "cm-photo-choose",
                t,
                Some("upload"),
                tr("Choose a photo…"),
                false,
            )
            .on_click(cx.listener(move |this, _, _, cx| this.choose_contact_photo(id, cx)))
            .test_support(),
        )
        .when(has, |d| {
            d.child(
                menu_item("cm-photo-remove", t, Some("x"), tr("Remove photo"), false)
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.set_contact_photo(id, None, cx)),
                    )
                    .test_support(),
            )
        });
    div()
        .relative()
        .flex_none()
        .child(
            div()
                .id("contact-photo")
                .group("contact-photo")
                .relative()
                .size(px(40.))
                .rounded_full()
                .cursor_pointer()
                .child(contact_avatar(t, c, 40.))
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .rounded_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(gpui_kit::black().opacity(0.55))
                        .invisible()
                        .group_hover("contact-photo", |d| d.visible())
                        .when(open, |d| d.visible())
                        .child(icon("upload", 16., gpui_kit::white())),
                )
                .tooltip(tip(tr("Change photo")))
                .on_click(
                    cx.listener(|this, _, _, cx| this.toggle_contact_menu(ContactMenu::Photo, cx)),
                )
                .test_support(),
        )
        .when(open, |d| d.child(deferred(menu_in("photo-menu", menu))))
}

fn inspector(
    s: &Session,
    c: &Contact,
    ed: &ContactEditor,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> Div {
    let id = c.id;
    let ui = &s.contacts_ui;
    let input = |f: Field| ed.inputs.get(&f).cloned();
    let more_open = ui.menu == Some(ContactMenu::More);
    let more = menu_box(t)
        .top(px(32.))
        .right_0()
        .child(
            menu_item("cm-vcard", t, Some("download"), tr("Share vCard"), false)
                .on_click(cx.listener(move |this, _, _, cx| this.export_vcard(vec![id], cx)))
                .test_support(),
        )
        .child(
            menu_item("cm-merge", t, Some("users"), tr("Merge & fix"), false)
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(s) = this.session() {
                        s.contacts_ui.filter = ContactFilter::Merge;
                        s.contacts_ui.menu = None;
                    }
                    cx.notify();
                }))
                .test_support(),
        )
        .child(
            menu_item("cm-delete", t, Some("trash"), tr("Delete"), true)
                .on_click(cx.listener(move |this, _, _, cx| this.ask_delete_contact(id, cx)))
                .test_support(),
        );
    let head = div()
        .p(px(12.))
        .flex()
        .items_start()
        .gap(px(8.))
        .child(photo_button(c, ui.menu == Some(ContactMenu::Photo), t, cx))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(2.))
                .when_some(input(Field::Name), |d, st| {
                    d.child(
                        div()
                            .text_size(TEXT_LG)
                            .line_height(px(20.))
                            .font_weight(W_MEDIUM)
                            .child(Input::new(&st).id("contact-name").appearance(false)),
                    )
                })
                .when_some(input(Field::JobTitle), |d, st| {
                    d.child(
                        div().text_size(TEXT_SM).text_color(t.text_muted).child(
                            Input::new(&st)
                                .id("contact-title")
                                .appearance(false)
                                .small(),
                        ),
                    )
                })
                .when_some(input(Field::Company), |d, st| {
                    d.child(
                        div().text_size(TEXT_SM).text_color(t.text_muted).child(
                            Input::new(&st)
                                .id("contact-company")
                                .appearance(false)
                                .small(),
                        ),
                    )
                }),
        )
        .child(
            icon_button("contact-fav", t, "star", false)
                .when(c.favorite, |d| d.child(div()).text_color(t.warning_text))
                .tooltip(tip(tr("Favorite")))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.update_contact(
                        id,
                        |c| {
                            c.favorite = !c.favorite;
                            None
                        },
                        cx,
                    )
                }))
                .test_support(),
        )
        .child(
            div()
                .relative()
                .child(
                    icon_button("contact-more", t, "more", false)
                        .tooltip(tip(tr("Share vCard, merge, delete")))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.toggle_contact_menu(ContactMenu::More, cx)
                        }))
                        .test_support(),
                )
                .when(more_open, |d| d.child(deferred(more))),
        )
        .child(
            icon_button("close-contact", t, "x", false)
                .tooltip(tip(tr("Close")))
                .on_click(cx.listener(|this, _, _, cx| this.deselect_contact(cx)))
                .test_support(),
        );
    let email = c.primary_email();
    let phone = c.primary_phone();
    let card = c.text_card();
    let acts = div()
        .px(px(12.))
        .pb(px(12.))
        .flex()
        .gap(px(8.))
        .child(
            button("contact-email", t, BtnKind::Default, true)
                .child(icon("mail", 14., t.text))
                .child(tr("Email"))
                .when(email.is_none(), |d| d.opacity(0.45))
                .tooltip(tip(tr("New email")))
                .on_click(cx.listener(move |_, _, _, cx| {
                    if let Some(e) = &email {
                        cx.open_url(&format!("mailto:{e}"));
                    }
                }))
                .test_support(),
        )
        .child(
            button("contact-call", t, BtnKind::Default, true)
                .child(icon("phone", 14., t.text))
                .child(tr("Call"))
                .when(phone.is_none(), |d| d.opacity(0.45))
                .tooltip(tip(tr("Call with the default app")))
                .on_click(cx.listener(move |_, _, _, cx| {
                    if let Some(p) = &phone {
                        let n: String = p
                            .chars()
                            .filter(|c| c.is_ascii_digit() || *c == '+')
                            .collect();
                        cx.open_url(&format!("tel:{n}"));
                    }
                }))
                .test_support(),
        )
        .child(
            button("contact-copy", t, BtnKind::Default, true)
                .child(icon("copy", 14., t.text))
                .child(tr("Copy"))
                .tooltip(tip(tr("Copy as text")))
                .on_click(cx.listener(move |this, _, _, cx| this.copy(card.clone(), "Contact", cx)))
                .test_support(),
        );
    let tabs = div()
        .px(px(12.))
        .flex()
        .gap(px(16.))
        .border_b_1()
        .border_color(t.line)
        .children(
            ["Details", "Linked", "History"]
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let on = ui.tab as usize == i;
                    div()
                        .id(SharedString::from(format!("ctab-{name}")))
                        .h(px(30.))
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .border_b_2()
                        .border_color(if on {
                            t.primary
                        } else {
                            gpui_kit::transparent_black()
                        })
                        .text_color(if on { t.text } else { t.text_muted })
                        .child(tr(name))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(s) = this.session() {
                                s.contacts_ui.tab = i as u8;
                            }
                            cx.notify();
                        }))
                        .test_support()
                }),
        );
    let body = match ui.tab {
        1 => linked(s, c, t, cx),
        2 => history(c, t),
        _ => details(s, c, ed, t, cx),
    };
    div()
        .w(INSPECTOR_W)
        .flex_none()
        .h_full()
        .bg(t.surface)
        .border_l_1()
        .border_color(t.line)
        .flex()
        .flex_col()
        .child(head)
        .child(acts)
        .child(tabs)
        .child(
            div()
                .id("contact-body")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(body),
        )
}

fn multi(
    c: &Contact,
    ed: &ContactEditor,
    list: List,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> Div {
    let id = c.id;
    let (label, rows): (&str, Vec<(String, Field)>) = match list {
        List::Email => (
            "Email",
            c.emails
                .iter()
                .enumerate()
                .map(|(i, x)| (crate::i18n::tr_owned(&x.label), Field::Email(i)))
                .collect(),
        ),
        List::Phone => (
            "Phone",
            c.phones
                .iter()
                .enumerate()
                .map(|(i, x)| (crate::i18n::tr_owned(&x.label), Field::Phone(i)))
                .collect(),
        ),
        List::Address => (
            "Address",
            c.addresses
                .iter()
                .enumerate()
                .map(|(i, x)| (crate::i18n::tr_owned(&x.label), Field::Address(i)))
                .collect(),
        ),
        List::Url => (
            "Website",
            c.urls
                .iter()
                .enumerate()
                .map(|(i, x)| (crate::i18n::tr_owned(&x.label), Field::Url(i)))
                .collect(),
        ),
    };
    let key = label.to_lowercase();
    let add = icon_button(SharedString::from(format!("add-{key}")), t, "plus", true)
        .tooltip(tip(tr("Add")))
        .on_click(cx.listener(move |this, _, _, cx| this.add_contact_row(id, list, cx)))
        .test_support();
    let mut d = div()
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(field_head(t, label, Some(add)));
    for (i, (kind, f)) in rows.into_iter().enumerate() {
        let Some(st) = ed.inputs.get(&f).cloned() else {
            continue;
        };
        let value = cleis_core::contact_edit::text(c, f);
        let action = match list {
            List::Address => {
                let q = value.clone();
                icon_button(
                    SharedString::from(format!("map-{key}-{i}")),
                    t,
                    "external",
                    true,
                )
                .tooltip(tip(tr("Open in maps")))
                .on_click(cx.listener(move |_, _, _, cx| {
                    let enc: String = q
                        .chars()
                        .map(|ch| {
                            if ch.is_ascii_alphanumeric() {
                                ch.to_string()
                            } else {
                                format!("%{:02X}", ch as u32 & 0xff)
                            }
                        })
                        .collect();
                    cx.open_url(&format!("https://www.openstreetmap.org/search?query={enc}"));
                }))
            }
            _ => {
                let what = label;
                icon_button(
                    SharedString::from(format!("copy-{key}-{i}")),
                    t,
                    "copy",
                    true,
                )
                .tooltip(tip(tr("Copy")))
                .on_click(cx.listener(move |this, _, _, cx| this.copy(value.clone(), what, cx)))
            }
        };
        d = d.child(
            div()
                .flex()
                .items_start()
                .gap(px(4.))
                .child(
                    div()
                        .id(SharedString::from(format!("kind-{key}-{i}")))
                        .w(px(64.))
                        .flex_none()
                        .h(CONTROL_MD)
                        .flex()
                        .items_center()
                        .px(px(4.))
                        .rounded(RADIUS_SM)
                        .cursor_pointer()
                        .text_size(TEXT_SM)
                        .text_color(t.text_muted)
                        .hover(|s| s.bg(t.surface_hover))
                        .tooltip(tip(tr("Change type")))
                        .child(kind)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.update_contact(
                                id,
                                |c| {
                                    cleis_core::contact_edit::cycle_label(c, list, i);
                                    None
                                },
                                cx,
                            )
                        }))
                        .test_support(),
                )
                .child(boxed(
                    t,
                    ed,
                    f,
                    Input::new(&st)
                        .id(SharedString::from(format!("edit-{key}-{i}")))
                        .suffix(action),
                )),
        );
    }
    d
}

fn details(
    s: &Session,
    c: &Contact,
    ed: &ContactEditor,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> Div {
    let id = c.id;
    let ui = &s.contacts_ui;
    let mut labels = div().flex().flex_wrap().items_center().gap(px(4.));
    for l in &c.labels {
        let l2 = l.clone();
        labels = labels.child(
            div()
                .id(SharedString::from(format!("label-{l}")))
                .group("lbl")
                .flex()
                .items_center()
                .gap(px(2.))
                .child(chip(t, label_tone(l), l.clone(), false))
                .child(
                    icon_button(SharedString::from(format!("unlabel-{l}")), t, "x", true)
                        .invisible()
                        .group_hover("lbl", |s| s.visible())
                        .tooltip(tip(tr("Remove label")))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let l = l2.clone();
                            this.update_contact(
                                id,
                                move |c| {
                                    c.labels.remove(&l);
                                    None
                                },
                                cx,
                            )
                        }))
                        .test_support(),
                ),
        );
    }
    let label_open = ui.menu == Some(ContactMenu::AddLabel);
    let mut label_menu = menu_box(t).top(px(28.)).left_0().w(px(220.)).child(
        div()
            .p(px(4.))
            .child(Input::new(&ed.label).id("label-input").small()),
    );
    for (l, _) in all_labels(s)
        .into_iter()
        .filter(|(l, _)| !c.labels.contains(l))
    {
        let l2 = l.clone();
        label_menu = label_menu.child(
            menu_item(format!("pick-label-{l}"), t, None, l.clone(), false)
                .on_click(cx.listener(move |this, _, _, cx| this.add_label(&l2, cx)))
                .test_support(),
        );
    }
    labels = labels.child(
        div()
            .relative()
            .child(
                icon_button("add-label", t, "plus", true)
                    .tooltip(tip(tr("Add label")))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.toggle_contact_menu(ContactMenu::AddLabel, cx);
                        if let Some(ed) = this.session().and_then(|s| s.contacts_ui.editor.as_ref())
                        {
                            let l = ed.label.clone();
                            l.update(cx, |st, cx| st.focus(window, cx));
                        }
                    }))
                    .test_support(),
            )
            .when(label_open, |d| d.child(deferred(label_menu))),
    );
    let input = |f: Field| ed.inputs.get(&f).cloned();
    div()
        .p(px(12.))
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(multi(c, ed, List::Email, t, cx))
        .child(multi(c, ed, List::Phone, t, cx))
        .child(multi(c, ed, List::Address, t, cx))
        .child(multi(c, ed, List::Url, t, cx))
        .when_some(input(Field::Birthday), |d, st| {
            d.child(field(
                t,
                "Birthday",
                boxed(t, ed, Field::Birthday, Input::new(&st).id("edit-birthday")),
            ))
        })
        .children(ed.custom.iter().map(|(col, st)| {
            let err = ed.custom_errors.get(&col.id).cloned();
            field(
                t,
                &col.name,
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .rounded(RADIUS_MD)
                            .when(err.is_some(), |d| d.border_1().border_color(t.danger_text))
                            .child(
                                Input::new(st)
                                    .id(SharedString::from(format!("ccustom-{}", col.id.0))),
                            ),
                    )
                    .when_some(err, |d, e| {
                        d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e))
                    }),
            )
        }))
        .child(field(t, tr("Labels"), labels))
        .when(!c.linked_entries.is_empty(), |d| {
            d.child(field(t, tr("Linked items"), linked_list(s, c, t, cx)))
        })
        .child(field(t, tr("Notes"), Textarea::new(&ed.notes)))
}

fn linked_list(s: &Session, c: &Contact, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let id = c.id;
    let mut d = div().flex().flex_col().gap(px(4.));
    for e in s
        .entries
        .iter()
        .filter(|e| c.linked_entries.contains(&e.id))
    {
        let eid = e.id;
        let pw = e.password.clone();
        d = d.child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .h(px(32.))
                .px(px(8.))
                .bg(t.bg)
                .border_1()
                .border_color(t.line)
                .rounded(RADIUS_MD)
                .child(site_icon(t, e.name.chars().next().unwrap_or('?'), 20.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(e.name.clone()),
                )
                .when(!pw.is_empty(), |d| {
                    d.child(
                        icon_button(SharedString::from(format!("lk-pw-{eid}")), t, "key", true)
                            .tooltip(tip(tr("Copy password")))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.copy(pw.clone(), "Password", cx)
                            })),
                    )
                })
                .child(
                    icon_button(SharedString::from(format!("unlink-{eid}")), t, "x", true)
                        .tooltip(tip(tr("Unlink")))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.update_contact(
                                id,
                                move |c| {
                                    c.linked_entries.retain(|x| *x != eid);
                                    None
                                },
                                cx,
                            )
                        }))
                        .test_support(),
                ),
        );
    }
    d
}

fn linked(s: &Session, c: &Contact, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let id = c.id;
    let open = s.contacts_ui.menu == Some(ContactMenu::Link);
    let mut pick = menu_box(t)
        .top(px(32.))
        .left_0()
        .w(px(260.))
        .max_h(px(320.))
        .overflow_hidden();
    let mut entries: Vec<_> = s
        .entries
        .iter()
        .filter(|e| !c.linked_entries.contains(&e.id))
        .collect();
    entries.sort_by_key(|e| fold(&e.name));
    for e in entries {
        let eid = e.id;
        pick = pick.child(
            menu_item(format!("link-{eid}"), t, None, e.name.clone(), false)
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(s) = this.session() {
                        s.contacts_ui.menu = None;
                    }
                    this.update_contact(
                        id,
                        move |c| {
                            c.linked_entries.push(eid);
                            None
                        },
                        cx,
                    )
                }))
                .test_support(),
        );
    }
    div()
        .p(px(12.))
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(field_head(t, "Vault items for this contact", None::<Div>))
        .child(linked_list(s, c, t, cx))
        .child(
            div()
                .relative()
                .child(
                    button("link-item", t, BtnKind::Default, true)
                        .child(icon("link", 14., t.text))
                        .child(tr("Link an item"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.toggle_contact_menu(ContactMenu::Link, cx)
                        }))
                        .test_support(),
                )
                .when(open, |d| d.child(deferred(pick))),
        )
}

fn history(c: &Contact, t: &Tokens) -> Div {
    let row = |k: &str, v: String| {
        div()
            .flex()
            .text_size(TEXT_SM)
            .child(
                div()
                    .w(px(80.))
                    .text_color(t.text_muted)
                    .child(crate::i18n::tr_owned(k)),
            )
            .child(div().flex_1().child(v))
    };
    div()
        .p(px(12.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(row("Created", crate::format::date(c.created)))
        .child(row("Edited", crate::format::date(c.updated)))
}

// ---------- 21. Merge & fix ----------

fn reason_text(r: Reason) -> &'static str {
    match r {
        Reason::SameEmail => "Same email",
        Reason::SamePhone => "Same phone",
        Reason::SimilarName => "Similar name",
    }
}

fn merge_card(
    s: &Session,
    c: &Contact,
    keep: bool,
    other: &Contact,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    let id = c.id;
    let folder = s
        .spaces
        .iter()
        .find(|sp| sp.id == c.space)
        .map(|sp| crate::i18n::space(&sp.name))
        .unwrap_or_default();
    let rows = [
        ("Company", c.company.clone(), other.company.clone()),
        (
            "Email",
            c.primary_email().unwrap_or_default(),
            other.primary_email().unwrap_or_default(),
        ),
        (
            "Phone",
            c.primary_phone().unwrap_or_default(),
            other.primary_phone().unwrap_or_default(),
        ),
        (
            "Birthday",
            c.birthday.map(|b| b.display()).unwrap_or_default(),
            other.birthday.map(|b| b.display()).unwrap_or_default(),
        ),
        ("Folder", folder, String::new()),
    ];
    let mut kv = div().flex().flex_col().gap(px(4.));
    for (k, v, o) in rows {
        let diff = !v.is_empty() && k != "Folder" && v != o;
        kv = kv.child(
            div()
                .flex()
                .text_size(TEXT_SM)
                .child(div().w(px(80.)).text_color(t.text_muted).child(tr(k)))
                .child(
                    div()
                        .flex_1()
                        .when(diff, |d| d.text_color(t.auto_text))
                        .child(if v.is_empty() { "—".to_owned() } else { v }),
                ),
        );
    }
    div()
        .id(SharedString::from(format!("merge-card-{id}")))
        .flex_1()
        .p(px(12.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .bg(t.bg)
        .border_1()
        .border_color(if keep { t.primary } else { t.line })
        .rounded(RADIUS_LG)
        .cursor_pointer()
        .tooltip(tip(tr("Keep this one")))
        .on_click(cx.listener(move |this, _, _, cx| {
            if let Some(s) = this.session() {
                let group = duplicates(s)
                    .into_iter()
                    .find(|g| g.contacts.contains(&id))
                    .map(|g| g.contacts);
                if let Some(g) = group {
                    s.contacts_ui.review = Some((g, id));
                }
            }
            cx.notify();
        }))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(contact_avatar(t, c, 32.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(div().font_weight(W_MEDIUM).child(c.display_name()))
                        .child(div().text_size(TEXT_SM).text_color(t.text_muted).child(tr(
                            if keep {
                                "Kept"
                            } else {
                                "Merged into the other"
                            },
                        ))),
                ),
        )
        .child(kv)
        .test_support()
}

fn most_complete(members: &[&Contact]) -> Option<ContactId> {
    members
        .iter()
        .max_by_key(|c| c.search_text().len())
        .map(|c| c.id)
}

fn merge_page(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let groups = duplicates(s);
    let by_id = |id: &ContactId| s.contacts.iter().find(|c| c.id == *id);
    let mut page = div()
        .max_w(px(960.))
        .mx_auto()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(12.))
                .min_h(px(32.))
                .child(
                    div()
                        .flex_1()
                        .text_size(TEXT_XL)
                        .line_height(px(26.))
                        .font_weight(W_STRONG)
                        .child(tr("Merge & fix")),
                )
                .when(!groups.is_empty(), |d| {
                    d.child(chip(
                        t,
                        Tone::Auto,
                        crate::trf!(
                            "{} possible duplicate{}",
                            groups.len(),
                            if groups.len() == 1 { "" } else { "s" }
                        ),
                        false,
                    ))
                    .child(
                        button("merge-all", t, BtnKind::Primary, false)
                            .child(tr("Merge all"))
                            .on_click(cx.listener(|this, _, _, cx| this.merge_all(cx)))
                            .test_support(),
                    )
                }),
        );
    if groups.is_empty() {
        page = page.child(
            div()
                .py(px(48.))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(8.))
                .border_1()
                .border_dashed()
                .border_color(t.line_strong)
                .rounded(RADIUS_LG)
                .text_color(t.text_muted)
                .child(icon("check", 16., t.success_text))
                .child(tr("No duplicates")),
        );
    }
    for (gi, g) in groups.iter().enumerate() {
        let members: Vec<&Contact> = g.contacts.iter().filter_map(by_id).collect();
        let Some(first) = members.first() else {
            continue;
        };
        let ids = g.contacts.clone();
        let reviewing = s
            .contacts_ui
            .review
            .as_ref()
            .filter(|(r, _)| *r == ids)
            .map(|(_, k)| *k)
            // The first group opens by itself, keeping its most complete contact.
            .or_else(|| {
                (gi == 0 && s.contacts_ui.review.is_none())
                    .then(|| most_complete(&members))
                    .flatten()
            });
        let ids_dismiss = ids.clone();
        let ids_merge = ids.clone();
        let ids_review = ids.clone();
        let mut sec = section(t).child(
            row(t, true)
                .child(
                    div()
                        .flex_1()
                        .font_weight(W_MEDIUM)
                        .child(first.display_name()),
                )
                .child(chip(t, Tone::Neutral, reason_text(g.reason), false))
                .when(members.len() > 2 || reviewing.is_none(), |d| {
                    d.child(
                        div()
                            .text_size(TEXT_SM)
                            .text_color(t.text_muted)
                            .child(crate::trf!("{} contacts", members.len())),
                    )
                })
                .child(
                    button(
                        SharedString::from(format!("not-dup-{gi}")),
                        t,
                        BtnKind::Ghost,
                        true,
                    )
                    .child(tr("Not a duplicate"))
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.not_duplicate(&ids_dismiss, cx)),
                    )
                    .test_support(),
                )
                .child(match reviewing {
                    Some(keep) => button(
                        SharedString::from(format!("merge-{gi}")),
                        t,
                        BtnKind::Default,
                        true,
                    )
                    .child(tr("Merge"))
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.merge_group(&ids_merge, keep, cx)),
                    )
                    .test_support(),
                    None => button(
                        SharedString::from(format!("review-{gi}")),
                        t,
                        BtnKind::Default,
                        true,
                    )
                    .child(tr("Review"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(s) = this.session() {
                            s.contacts_ui.review = Some((ids_review.clone(), ids_review[0]));
                        }
                        cx.notify();
                    }))
                    .test_support(),
                }),
        );
        if let Some(keep) = reviewing {
            let kept = members
                .iter()
                .find(|c| c.id == keep)
                .copied()
                .unwrap_or(first);
            let mut cards = div().flex().gap(px(12.));
            for c in &members {
                let other = if c.id == kept.id {
                    members
                        .iter()
                        .find(|o| o.id != kept.id)
                        .copied()
                        .unwrap_or(kept)
                } else {
                    kept
                };
                cards = cards.child(merge_card(s, c, c.id == keep, other, t, cx));
            }
            sec = sec.child(cards);
        }
        page = page.child(sec);
    }
    div().flex_1().min_w_0().flex().child(
        div()
            .id("merge-page")
            .flex_1()
            .min_w_0()
            .overflow_y_scroll()
            .px(px(32.))
            .pt(px(24.))
            .pb(px(40.))
            .child(page),
    )
}

impl CleisApp {
    /// Choose a photo… : a picture file, scaled down and kept in the contact.
    pub fn choose_contact_photo(&mut self, id: ContactId, cx: &mut Context<Self>) {
        if let Some(s) = self.session() {
            s.contacts_ui.menu = None;
        }
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(tr("Choose a photo…").into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = rx.await
                && let Some(p) = paths.into_iter().next()
            {
                this.update(cx, |this, cx| this.contact_photo_from(id, &p, cx))
                    .ok();
            }
        })
        .detach();
        cx.notify();
    }

    /// A picture file becomes the contact's photo: centred square, 256 px, JPEG.
    pub fn contact_photo_from(
        &mut self,
        id: ContactId,
        path: &std::path::Path,
        cx: &mut Context<Self>,
    ) {
        match std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|b| portrait(&b))
        {
            Ok(url) => self.set_contact_photo(id, Some(url), cx),
            Err(e) => {
                if let Some(s) = self.session() {
                    s.toast = Some(crate::trf!("Photo not read: {}", e).into());
                }
                cx.notify();
            }
        }
    }

    pub fn set_contact_photo(
        &mut self,
        id: ContactId,
        photo: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let Some(s) = self.session() else { return };
        s.contacts_ui.menu = None;
        let Some(i) = s.contacts.iter().position(|c| c.id == id) else {
            return;
        };
        if !s.may(s.contacts[i].space, cleis_core::org::Perm::Edit) {
            s.toast = Some(tr("Your role does not allow changing it").into());
            cx.notify();
            return;
        }
        let mut c = s.contacts[i].clone();
        c.photo = photo;
        c.updated = cleis_core::now();
        match s.vault.put_contact(&c) {
            Ok(()) => s.contacts[i] = c,
            Err(e) => s.toast = Some(crate::trf!("Not saved: {}", e).into()),
        }
        cx.notify();
    }

    pub fn toggle_contact_menu(&mut self, m: ContactMenu, cx: &mut Context<Self>) {
        if let Some(s) = self.session() {
            s.contacts_ui.menu = if s.contacts_ui.menu == Some(m) {
                None
            } else {
                Some(m)
            };
        }
        cx.notify();
    }

    pub fn new_contact(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        // Where the list is (27/11/2026): the folder, the organization shown (its
        // "Uncategorized", made the first time by an admin), or the personal contacts.
        let space = match &s.contacts_ui.filter {
            ContactFilter::Folder(sp) => Some(*sp),
            ContactFilter::Org(k) => {
                let k = k.clone();
                match unfiled_of(s, &k) {
                    Some(sp) => Some(sp),
                    None => {
                        let manage = s
                            .all_orgs()
                            .find(|o| o.m.key() == k)
                            .is_some_and(|o| o.can(cleis_core::org::Perm::Manage));
                        if manage {
                            self.in_org(&k, |app| {
                                app.org_new_contacts_space("Uncategorized".into(), true, cx)
                            })
                            .flatten()
                        } else {
                            None
                        }
                    }
                }
            }
            _ => personal_default(s)
                .or_else(|| s.spaces.iter().find(|sp| sp.for_contacts).map(|sp| sp.id))
                .or(Some(s.vault.home_space())),
        };
        let Some(s) = self.session() else { return };
        let Some(space) = space.filter(|sp| s.may(*sp, cleis_core::org::Perm::Edit)) else {
            s.toast = Some(tr("Your role does not allow adding here").into());
            cx.notify();
            return;
        };
        let mut c = Contact::new(space);
        c.given = tr("New contact").into();
        if let ContactFilter::Label(l) = &s.contacts_ui.filter {
            c.labels.insert(l.clone());
        }
        if matches!(
            s.contacts_ui.filter,
            ContactFilter::Merge | ContactFilter::Birthdays
        ) {
            s.contacts_ui.filter = ContactFilter::All;
        }
        if s.contacts_ui.filter == ContactFilter::Favorites {
            c.favorite = true;
        }
        match s.vault.put_contact(&c) {
            Ok(()) => {
                s.selected_contact = Some(c.id);
                s.contacts.push(c);
                s.contacts_ui.editor = None;
                s.contacts_ui.tab = 0;
                if let Some(st) = s.contacts_ui.search.clone() {
                    st.update(cx, |st, cx| st.set_value("", window, cx));
                }
                s.contacts_ui.query.clear();
            }
            Err(e) => s.toast = Some(crate::trf!("Not saved: {}", e).into()),
        }
        // Name box focused, its text selected, once the boxes exist.
        self.ensure_contact_editor(window, cx);
        if let Some(st) = self
            .session()
            .and_then(|s| s.contacts_ui.editor.as_ref())
            .and_then(|e| e.inputs.get(&Field::Name).cloned())
        {
            st.update(cx, |st, cx| {
                st.focus(window, cx);
                st.select_all(window, cx);
            });
        }
        cx.notify();
    }

    pub fn ask_delete_contact(&mut self, id: ContactId, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        s.contacts_ui.menu = None;
        let name = s
            .contacts
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.display_name())
            .unwrap_or_default();
        s.confirm = Some(Confirm {
            title: crate::trf!("Delete {}?", name).into(),
            body: tr("The contact is removed from every device. This cannot be undone.").into(),
            button: tr("Delete").into(),
            action: ConfirmAction::DeleteContact(id),
        });
        cx.notify();
    }

    /// Moves a contact to another contacts space; an organization's contact stays in it and,
    /// between two of its spaces, becomes a new contact there (as items, audit L-2).
    pub fn move_contact(&mut self, id: ContactId, to: cleis_core::SpaceId, cx: &mut Context<Self>) {
        use cleis_core::org::Perm;
        let Some(s) = self.session() else { return };
        let Some(from) = s.contacts.iter().find(|c| c.id == id).map(|c| c.space) else {
            return;
        };
        let org_key = |sp| s.org_of(sp).map(|o| o.m.key());
        let refusal = if s.is_org_space(from) && org_key(from) != org_key(to) {
            Some(tr("An organization's contacts stay in the organization"))
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
        let Some(i) = s.contacts.iter().position(|c| c.id == id) else {
            return;
        };
        let mut c = s.contacts[i].clone();
        c.space = to;
        c.updated = cleis_core::now();
        let r = if s.is_org_space(from) {
            c.id = ContactId::new();
            s.vault
                .put_contact(&c)
                .and_then(|()| s.vault.purge_contact(id).map(|_| ()))
        } else {
            s.vault.put_contact(&c)
        };
        match r {
            Ok(()) => {
                s.selected_contact = Some(c.id);
                s.contacts[i] = c;
                s.contacts_ui.editor = None;
                s.toast = Some(tr("Moved").into());
            }
            Err(e) => s.toast = Some(crate::trf!("Not moved: {}", e).into()),
        }
        cx.notify();
    }

    pub fn delete_contact(&mut self, id: ContactId) {
        let Some(s) = self.session() else { return };
        match s.vault.purge_contact(id) {
            Ok(_) => {
                s.contacts.retain(|c| c.id != id);
                if s.selected_contact == Some(id) {
                    s.selected_contact = None;
                }
                s.contacts_ui.editor = None;
                s.toast = Some(tr("Contact deleted").into());
            }
            Err(e) => s.toast = Some(crate::trf!("Not deleted: {}", e).into()),
        }
    }

    /// "New folder" from a menu: in the organization shown if this member manages it, else
    /// personal.
    pub fn start_new_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let org = self.session().and_then(|s| {
            filter_org(s, &s.contacts_ui.filter).filter(|k| {
                s.all_orgs()
                    .find(|o| o.m.key() == *k)
                    .is_some_and(|o| o.can(cleis_core::org::Perm::Manage))
            })
        });
        self.start_new_folder_in(org, window, cx);
    }

    /// A folder named in place under its section: personal, or the organization's (`org`).
    pub fn start_new_folder_in(
        &mut self,
        org: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(s) = self.session() {
            s.contacts_ui.new_folder_org = org;
        }
        let st = cx.new(|cx| InputState::new(window, cx).placeholder(tr("Folder name")));
        st.update(cx, |st, cx| st.focus(window, cx));
        let sub = cx.subscribe_in(
            &st,
            window,
            |this, input, ev: &gpui_kit::component::input::InputEvent, _, cx| {
                use gpui_kit::component::input::InputEvent;
                match ev {
                    InputEvent::PressEnter { .. } => {
                        let name = input.read(cx).value().to_string();
                        this.create_folder(&name, cx);
                    }
                    InputEvent::Blur => {
                        if let Some(s) = this.session() {
                            s.contacts_ui.new_folder = None;
                        }
                        cx.notify();
                    }
                    _ => {}
                }
            },
        );
        if let Some(s) = self.session() {
            s.contacts_ui.new_folder = Some(st);
            s.contacts_ui.subs.push(sub);
        }
        cx.notify();
    }

    pub fn create_folder(&mut self, name: &str, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        s.contacts_ui.new_folder = None;
        let name = name.trim();
        if name.is_empty() {
            cx.notify();
            return;
        }
        // An organization's folder goes through it (its members get it as a space).
        if let Some(k) = s.contacts_ui.new_folder_org.take() {
            let name = name.to_owned();
            self.in_org(&k, |app| app.org_new_contacts_space(name, false, cx));
            cx.notify();
            return;
        }
        let r = s.vault.create_space(
            name,
            SpaceKind::Personal,
            cleis_store::EncryptionMode::EndToEnd,
            true,
        );
        match r.and_then(|id| Ok((id, s.vault.spaces()?))) {
            Ok((id, spaces)) => {
                s.spaces = spaces;
                s.contacts_ui.filter = ContactFilter::Folder(id);
            }
            Err(e) => s.toast = Some(crate::trf!("Folder not created: {}", e).into()),
        }
        cx.notify();
    }

    fn not_duplicate(&mut self, ids: &[ContactId], cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        for (i, a) in ids.iter().enumerate() {
            for b in &ids[i + 1..] {
                let pair = if a < b { (*a, *b) } else { (*b, *a) };
                s.prefs.not_duplicates.insert(pair);
            }
        }
        if let Err(e) = s.vault.set_preferences(&s.prefs) {
            s.toast = Some(crate::trf!("Not saved: {}", e).into());
        }
        s.contacts_ui.review = None;
        cx.notify();
    }

    /// Merges a group into `keep` (D-39): empty fields filled, lists united, others deleted.
    pub fn merge_group(&mut self, ids: &[ContactId], keep: ContactId, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let others: Vec<Contact> = s
            .contacts
            .iter()
            .filter(|c| ids.contains(&c.id) && c.id != keep)
            .cloned()
            .collect();
        let Some(k) = s.contacts.iter_mut().find(|c| c.id == keep) else {
            return;
        };
        for o in &others {
            dedupe::merge(k, o);
        }
        k.updated = cleis_core::now();
        let saved = s.vault.put_contact(k);
        if let Err(e) = saved {
            s.toast = Some(crate::trf!("Not merged: {}", e).into());
            cx.notify();
            return;
        }
        for o in &others {
            let _ = s.vault.purge_contact(o.id);
        }
        s.contacts.retain(|c| !others.iter().any(|o| o.id == c.id));
        s.contacts_ui.review = None;
        s.contacts_ui.editor = None;
        s.selected_contact = Some(keep);
        s.toast = Some(tr("Merged").into());
        cx.notify();
    }

    fn merge_all(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let groups = duplicates(s);
        for g in groups {
            // Keep the most complete contact of each group.
            let keep = {
                let Some(s) = self.session() else { return };
                let members: Vec<&Contact> = g
                    .contacts
                    .iter()
                    .filter_map(|id| s.contacts.iter().find(|c| c.id == *id))
                    .collect();
                most_complete(&members)
            };
            if let Some(k) = keep {
                self.merge_group(&g.contacts, k, cx);
            }
        }
    }

    pub fn choose_vcard(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some(tr("Import").into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = rx.await {
                this.update(cx, |this, cx| {
                    for p in paths {
                        this.import_vcard(&p, cx);
                    }
                })
                .ok();
            }
        })
        .detach();
    }

    /// Imports a .vcf or .csv file into the current folder (or the personal one).
    pub fn import_vcard(&mut self, path: &std::path::Path, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let space = match s.contacts_ui.filter {
            ContactFilter::Folder(sp) => sp,
            _ => s
                .spaces
                .iter()
                .find(|sp| sp.for_contacts && sp.kind == SpaceKind::Personal)
                .map(|sp| sp.id)
                .unwrap_or_else(|| s.vault.home_space()),
        };
        let text = match std::fs::read(path) {
            Ok(b) => cleis_core::import_csv::decode(&b),
            Err(e) => {
                s.toast = Some(crate::trf!("Not imported: {}", e).into());
                cx.notify();
                return;
            }
        };
        let is_csv = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("csv"));
        let new = if is_csv {
            match cleis_core::import_contacts::contacts_from_csv(&text, space) {
                Ok(r) => r.contacts,
                Err(e) => {
                    s.toast = Some(crate::trf!("Not imported: {}", e).into());
                    cx.notify();
                    return;
                }
            }
        } else {
            cleis_core::vcard::parse(&text, space)
        };
        let n = new.len();
        match s.vault.put_contacts(&new) {
            Ok(()) => {
                s.contacts.extend(new);
                s.toast = Some(if n == 0 {
                    tr("No contact found in this file").into()
                } else {
                    format!("{n} contact{} imported", if n == 1 { "" } else { "s" }).into()
                });
            }
            Err(e) => s.toast = Some(crate::trf!("Not imported: {}", e).into()),
        }
        cx.notify();
    }

    /// Saves contacts as a vCard 4.0 file chosen by the user.
    pub fn export_vcard(&mut self, ids: Vec<ContactId>, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        s.contacts_ui.menu = None;
        let list: Vec<&Contact> = s.contacts.iter().filter(|c| ids.contains(&c.id)).collect();
        // Audit L-4 (D-19): an organization's contacts leave Cleis only with the Export right,
        // and the export is written to its log.
        let org_spaces: std::collections::BTreeSet<cleis_core::SpaceId> = list
            .iter()
            .map(|c| c.space)
            .filter(|sp| s.is_org_space(*sp))
            .collect();
        if !org_spaces.is_empty() {
            let allowed = s
                .org
                .as_ref()
                .and_then(|o| o.me())
                .is_some_and(|m| m.export || cleis_core::org::is_admin_role(&m.role));
            if !allowed {
                s.toast = Some(tr("Your organization does not let you export its contacts").into());
                cx.notify();
                return;
            }
        }
        let Some(first) = list.first() else { return };
        let name = format!(
            "{}.vcf",
            first.display_name().replace(['/', '\\', ':'], "-")
        );
        let data = cleis_core::vcard::write(&list);
        let count = list.len();
        for sp in org_spaces {
            self.org_log(
                cleis_core::org::Action::Exported,
                Some(sp),
                format!("{count} contacts"),
                cx,
            );
        }
        let Some(_s) = self.session() else { return };
        let dir = directories::UserDirs::new()
            .and_then(|d| d.download_dir().map(|p| p.to_owned()))
            .unwrap_or_else(std::env::temp_dir);
        let rx = cx.prompt_for_new_path(&dir, Some(&name));
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(path))) = rx.await {
                let r = std::fs::write(&path, data);
                this.update(cx, |this, cx| {
                    let msg = match r {
                        Ok(()) => tr("vCard saved").to_owned(),
                        Err(e) => crate::trf!("Not saved: {}", e),
                    };
                    this.toast(msg, cx);
                })
                .ok();
            }
        })
        .detach();
    }
}

/// Side of a contact's photo, in pixels: sharp at twice the inspector's 40 px, and more.
const PORTRAIT: u32 = 256;

/// A picture (any format the `image` crate reads) as a contact photo: the centred square, scaled
/// to [`PORTRAIT`] px, JPEG, as a `data:` URL.
pub fn portrait(bytes: &[u8]) -> Result<String, String> {
    portrait_of(bytes, PORTRAIT)
}

/// A picture as a centred square of `side` pixels (at most its own), JPEG, as a `data:` URL.
pub fn portrait_of(bytes: &[u8], side_max: u32) -> Result<String, String> {
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let side = img.width().min(img.height());
    let sq = img.crop_imm(
        (img.width() - side) / 2,
        (img.height() - side) / 2,
        side,
        side,
    );
    let small = sq
        .resize_exact(
            side_max.min(side),
            side_max.min(side),
            image::imageops::FilterType::Lanczos3,
        )
        .to_rgb8();
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85)
        .encode_image(&small)
        .map_err(|e| e.to_string())?;
    Ok(cleis_core::contact::photo_url("image/jpeg", &out))
}
