//! Menus opened by a right-click (mockups 33 to 33d): an item of the vault, a space or folder of
//! the sidebar, a contact, a member. The menu opens at the pointer, a click outside or Escape
//! closes it, and a "▸" row opens its submenu on hover.

use crate::app::{CleisApp, FolderDialog, Session, Tab};
use crate::i18n::tr;
use crate::theme::*;
use crate::ui::*;
use cleis_core::org::{MemberChange, Perm};
use cleis_core::{ContactId, EntryId, MemberId, SpaceId};
use gpui_kit::TestSupportExt;
use gpui_kit::prelude::*;
use gpui_kit::{
    AnyElement, ClickEvent, Context, Div, MouseButton, MouseDownEvent, Pixels, Point, SharedString,
    Stateful, Window, anchored, deferred, div, point, px,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CtxTarget {
    Entry(EntryId),
    Space(SpaceId),
    Contact(ContactId),
    Member(MemberId),
    /// The list (table or cards) outside any item (mockup 35).
    ListVoid,
    /// The sidebar outside any folder (mockup 35b).
    SidebarVoid,
    /// The contacts (table or cards) outside any contact (26/11/2026, as mockup 35).
    ContactsVoid,
    /// The contacts' sidebar outside any folder.
    ContactsSidebar,
    /// The password box of the panel on the right (mockup 37o).
    PasswordField(EntryId),
    /// The sidebar's Trash (mockup 47e).
    Trash,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sub {
    Move,
    Copy,
    Role,
    Spaces,
    Sort,
    /// A folder's icon (mockup 48g).
    Icon,
}

pub struct CtxMenu {
    pub at: Point<Pixels>,
    pub target: CtxTarget,
    pub sub: Option<Sub>,
}

const ROW: f32 = 28.;
const SEP: f32 = 9.;
const PAD: f32 = 4.;
const WIDTH: f32 = 240.;

fn tip(
    text: &'static str,
) -> impl Fn(&mut Window, &mut gpui_kit::App) -> gpui_kit::AnyView + 'static {
    move |window, cx| {
        gpui_kit::component::tooltip::Tooltip::new(crate::i18n::tr_dynamic(text)).build(window, cx)
    }
}

/// Shortcut as written on this system ("Ctrl+D", "⌘D").
pub fn keys(k: &str) -> String {
    if cfg!(target_os = "macos") {
        k.replace("Ctrl+Shift+", "⇧⌘")
            .replace("Ctrl+Alt+", "⌥⌘")
            .replace("Ctrl+", "⌘")
            .replace("Del", "⌫")
    } else {
        k.to_owned()
    }
}

/// Outline of the row or card whose menu is open (it is not selected: mockup 37o).
pub fn aim_ring(t: &Tokens, radius: Pixels) -> Div {
    div()
        .absolute()
        .inset_0()
        .border_1()
        .border_color(t.focus)
        .rounded(radius)
}

/// Listener for `.on_mouse_down(MouseButton::Right, …)` on a row.
pub fn on_right(
    target: CtxTarget,
    cx: &mut Context<CleisApp>,
) -> impl Fn(&MouseDownEvent, &mut Window, &mut gpui_kit::App) + 'static {
    cx.listener(move |this, ev: &MouseDownEvent, window, cx| {
        cx.stop_propagation();
        this.open_ctx(target, ev.position, window, cx);
    })
}

/// Builds the rows of one menu and keeps their height, to align a submenu with its row.
struct Rows<'a> {
    t: &'a Tokens,
    menu: Div,
    y: f32,
    /// Top of the row that opens the submenu.
    sub_y: f32,
}

impl<'a> Rows<'a> {
    fn new(t: &'a Tokens) -> Self {
        Self {
            t,
            menu: menu_box(t).relative().w(px(WIDTH)),
            y: PAD,
            sub_y: PAD,
        }
    }

    fn row(&mut self, el: impl IntoElement) {
        let m = std::mem::replace(&mut self.menu, div());
        self.menu = m.child(el);
        self.y += ROW;
    }

    fn sep(&mut self) {
        let t = self.t;
        let m = std::mem::replace(&mut self.menu, div());
        self.menu = m.child(div().h(px(1.)).my(px(4.)).bg(t.line));
        self.y += SEP;
    }
}

/// The keys of a shortcut inside Cleis, as chosen (mockup 38); none: not shown.
fn keys_of(s: &Session, name: &str) -> String {
    s.prefs.app_key(name)
}

/// A row with an optional shortcut on the right.
fn item(
    id: impl Into<SharedString>,
    t: &Tokens,
    icon_name: &str,
    label: impl Into<SharedString>,
    shortcut: Option<&str>,
    danger: bool,
) -> Stateful<Div> {
    menu_item(id, t, Some(icon_name), label, danger)
        .child(div().flex_1())
        .when_some(shortcut.filter(|k| !k.is_empty()), |d, k| {
            d.child(
                div()
                    .text_size(TEXT_XS)
                    .text_color(t.text_faint)
                    .child(keys(k)),
            )
        })
}

/// A row that opens a submenu ("Move to ▸").
fn parent(
    id: &str,
    t: &Tokens,
    icon_name: &str,
    label: &'static str,
    sub: Sub,
    open: bool,
    cx: &mut Context<CleisApp>,
) -> AnyElement {
    menu_item(
        SharedString::from(id.to_owned()),
        t,
        Some(icon_name),
        label,
        false,
    )
    .when(open, |d| d.bg(t.surface_hover))
    .child(div().flex_1())
    .child(icon("chevronR", 12., t.text_muted))
    .on_hover(cx.listener(move |this, on: &bool, _, cx| {
        if *on {
            this.ctx_sub(Some(sub), cx);
        }
    }))
    .on_click(cx.listener(move |this, _, _, cx| this.ctx_sub(Some(sub), cx)))
    .test_support()
    .into_any_element()
}

/// Closes the submenu when the pointer reaches another row.
fn plain(el: Stateful<Div>, cx: &mut Context<CleisApp>) -> Stateful<Div> {
    el.on_hover(cx.listener(|this, on: &bool, _, cx| {
        if *on {
            this.ctx_sub(None, cx);
        }
    }))
}

/// A row that runs `f` then closes the menu.
fn act(
    el: Stateful<Div>,
    cx: &mut Context<CleisApp>,
    f: impl Fn(&mut CleisApp, &mut Window, &mut Context<CleisApp>) + 'static,
) -> AnyElement {
    plain(el, cx)
        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
            this.close_ctx(cx);
            f(this, window, cx);
        }))
        .test_support()
        .into_any_element()
}

/// A greyed row with the reason in its tooltip.
fn off(el: Stateful<Div>, why: &'static str, cx: &mut Context<CleisApp>) -> AnyElement {
    plain(el, cx)
        .opacity(0.45)
        .cursor_default()
        .tooltip(tip(why))
        .into_any_element()
}

/// Whether a floating menu other than a right-click one is open (audit U-2).
pub fn menu_open(s: &Session) -> bool {
    s.filter_menu
        || s.calendar.is_some()
        || s.wheel.is_some()
        || s.sync_menu
        || s.account_menu
        || s.profile_edit.is_some()
        || s.pref_menu.is_some()
        || s.folder_menu.is_some()
        || s.security.menu.is_some()
        || s.contacts_ui.menu.is_some()
        || s.import.menu.is_some()
        || s.import.space_menu
        || s.health.vault_menu
        || s.health.space_menu
        || s.health.change.is_some()
        || s.editor.as_ref().is_some_and(|e| e.menu.is_some())
        || s.org.as_ref().is_some_and(|o| o.ui.menu.is_some())
}

/// Audit U-2: below the open menu (menus are painted last), a layer that closes it when the
/// pointer goes down anywhere else.
pub fn outside(s: &Session, cx: &mut Context<CleisApp>) -> Option<AnyElement> {
    menu_open(s).then(|| {
        div()
            .id("menu-outside")
            .absolute()
            .inset_0()
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.close_menus(cx)),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, _, _, cx| this.close_menus(cx)),
            )
            .into_any_element()
    })
}

pub fn render(
    s: &Session,
    t: &Tokens,
    window: &Window,
    cx: &mut Context<CleisApp>,
) -> Option<AnyElement> {
    let m = s.ctx.as_ref()?;
    let (main, sub) = match m.target {
        CtxTarget::Entry(id) => entry_menu(s, id, m.sub, t, cx)?,
        CtxTarget::Space(id) => space_menu(s, id, m.sub, t, cx)?,
        CtxTarget::Contact(id) => contact_menu(s, id, m.sub, t, cx)?,
        CtxTarget::Member(id) => member_menu(s, id, m.sub, t, cx)?,
        CtxTarget::ListVoid => list_void_menu(s, m.sub, t, cx),
        CtxTarget::SidebarVoid => (sidebar_void_menu(s, t, cx), None),
        CtxTarget::ContactsVoid => (contacts_void_menu(s, t, cx), None),
        CtxTarget::PasswordField(id) => (password_field_menu(s, id, t, cx)?, None),
        // Mockup 47e: open it, or empty it (asked first, as from the Trash page).
        CtxTarget::Trash => {
            let mut r = Rows::new(t);
            r.row(act(
                item("cx-trash-open", t, "trash", tr("Open"), None, false),
                cx,
                |this, _, cx| {
                    if let Some(s) = this.session() {
                        s.filter = crate::app::Filter::Trash;
                        s.refilter();
                    }
                    cx.notify();
                },
            ));
            r.sep();
            r.row(if s.trash.is_empty() {
                plain(
                    item("cx-trash-empty", t, "trash", tr("Empty trash…"), None, true)
                        .opacity(0.45),
                    cx,
                )
                .into_any_element()
            } else {
                act(
                    item("cx-trash-empty", t, "trash", tr("Empty trash…"), None, true),
                    cx,
                    |this, _, cx| this.ask_empty_trash(cx),
                )
            });
            (r.menu, None)
        }
        CtxTarget::ContactsSidebar => {
            let mut r = Rows::new(t);
            r.row(act(
                item(
                    "cx-new-contact-folder",
                    t,
                    "folder",
                    tr("New folder"),
                    None,
                    false,
                ),
                cx,
                |this, window, cx| this.start_new_folder(window, cx),
            ));
            (r.menu, None)
        }
    };
    // The submenu hangs off the menu without changing its size: opening it never moves the
    // menu under the pointer (the window edge would push both up).
    // Near the right edge, the submenu opens on the left.
    let room = window.viewport_size().width - m.at.x;
    let left = room < px(WIDTH + 240. + 16.);
    // Near the bottom, it ends with the menu instead of starting at its row.
    let up = window.viewport_size().height - m.at.y < px(340.);
    let body = main.when_some(sub, |d, (y, menu)| {
        d.child(
            menu.absolute()
                .when(!left, |d| d.left(px(WIDTH - 4.)))
                .when(left, |d| d.right(px(WIDTH - 4.)))
                .when(!up, |d| d.top(px(y - PAD)))
                .when(up, |d| d.bottom(px(-1.))),
        )
    });
    Some(
        deferred(
            div()
                .id("ctx-overlay")
                .absolute()
                .inset_0()
                .occlude()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| this.close_ctx(cx)),
                )
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(|this, _, _, cx| this.close_ctx(cx)),
                )
                .child(
                    anchored()
                        .position(m.at + point(px(2.), px(2.)))
                        .snap_to_window_with_margin(px(8.))
                        .child(crate::ui::menu_in(format!("ctx-{:?}", m.target), body)),
                ),
        )
        .with_priority(2)
        .into_any_element(),
    )
}

type Built = (Div, Option<(f32, Div)>);

/// A row with a tick when `on` (Sort by, Shortcuts); the label stays aligned without it.
fn check_item(id: impl Into<SharedString>, t: &Tokens, label: &str, on: bool) -> Stateful<Div> {
    div()
        .id(gpui_kit::ElementId::Name(id.into()))
        .h(px(ROW))
        .px(px(8.))
        .flex()
        .items_center()
        .gap(px(8.))
        .rounded(RADIUS_SM)
        .cursor_pointer()
        .text_color(t.text)
        .hover(|d| d.bg(t.surface_hover))
        .child(
            div()
                .size(px(14.))
                .flex_none()
                .when(on, |d| d.child(icon("check", 14., t.accent_text))),
        )
        .child(crate::i18n::tr_owned(label))
}

/// "New folder" in the personal vault, "New space" in an organization the member manages.
fn new_folder_row(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Option<AnyElement> {
    // The organization shown (its tab, its section, one of its folders, 27/11/2026).
    match crate::screens::org::org_in_view(s).map(|o| (o.m.key(), o.can(Perm::Manage))) {
        Some((k, manage)) => manage.then(|| {
            act(
                item("cx-new-space", t, "folder", tr("New space"), None, false),
                cx,
                move |this, window, cx| {
                    this.activate_org(&k);
                    this.open_new_space(window, cx);
                },
            )
        }),
        _ => Some(act(
            item("cx-new-folder", t, "folder", tr("New folder"), None, false),
            cx,
            |this, window, cx| this.open_new_folder(window, cx),
        )),
    }
}

/// Mockup 35: a right-click in the list outside any item.
fn list_void_menu(s: &Session, sub: Option<Sub>, t: &Tokens, cx: &mut Context<CleisApp>) -> Built {
    use cleis_core::settings::SortBy;
    let mut r = Rows::new(t);
    r.row(act(
        item(
            "cx-new-item",
            t,
            "plus",
            tr("New password"),
            Some(&*keys_of(s, "new")),
            false,
        ),
        cx,
        |this, window, cx| this.new_entry(window, cx),
    ));
    if let Some(row) = new_folder_row(s, t, cx) {
        r.row(row);
    }
    r.sep();
    r.sub_y = r.y;
    r.row(parent(
        "cx-sort",
        t,
        "list",
        "Sort by",
        Sub::Sort,
        sub == Some(Sub::Sort),
        cx,
    ));
    r.row(act(
        item("cx-columns", t, "columns", tr("Columns…"), None, false),
        cx,
        |this, _, cx| this.toggle_columns_menu(cleis_core::column::Table::Vault, cx),
    ));
    let cards = s.vault_cards;
    r.row(act(
        item(
            "cx-view",
            t,
            if cards { "list" } else { "columns" },
            if cards {
                tr("Show as table")
            } else {
                tr("Show as cards")
            },
            None,
            false,
        ),
        cx,
        move |this, _, cx| {
            if let Some(s) = this.session() {
                s.vault_cards = !cards;
                s.refilter();
            }
            cx.notify();
        },
    ));
    r.sep();
    r.row(act(
        item("cx-import", t, "upload", tr("Import…"), None, false),
        cx,
        |this, _, cx| {
            if let Some(s) = this.session() {
                // D-99: into the organization shown, or the personal vault.
                let org = crate::screens::org::org_in_view(s).map(|o| o.m.key());
                s.tab = Tab::Settings;
                s.settings_page = if org.is_some() {
                    crate::screens::import::ORG_PAGE
                } else {
                    "Import & export"
                };
                if let Some(k) = org {
                    this.activate_org(&k);
                }
            }
            cx.notify();
        },
    ));
    let subm = (sub == Some(Sub::Sort)).then(|| {
        let now = s.prefs.sort_by;
        let mut m = menu_box(t).w(px(200.));
        let opts = [
            (SortBy::Name, "Name"),
            (SortBy::Edited, "Last edited"),
            (SortBy::Price, "Price"),
            (SortBy::Renewal, "Renewal"),
            (SortBy::Manual, "My order"),
        ];
        for (by, label) in opts {
            if by == SortBy::Manual {
                m = m.child(div().h(px(1.)).my(px(4.)).bg(t.line));
            }
            let el = check_item(format!("cx-sort-{by:?}"), t, label, now == by)
                .when(by == SortBy::Manual, |d| {
                    d.tooltip(tip(tr("Drag the items to put them in your order")))
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.close_ctx(cx);
                    this.set_sort(by, cx);
                }))
                .test_support();
            m = m.child(el);
        }
        (r.sub_y, m)
    });
    (r.menu, subm)
}

/// A right-click among the contacts outside any contact (as mockup 35 for the vault).
fn contacts_void_menu(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let mut r = Rows::new(t);
    r.row(act(
        item(
            "cx-new-contact",
            t,
            "plus",
            tr("New contact"),
            Some(&*keys_of(s, "new")),
            false,
        ),
        cx,
        |this, window, cx| this.new_contact(window, cx),
    ));
    r.row(act(
        item(
            "cx-new-contact-folder",
            t,
            "folder",
            tr("New folder"),
            None,
            false,
        ),
        cx,
        |this, window, cx| this.start_new_folder(window, cx),
    ));
    r.sep();
    r.row(act(
        item(
            "cx-contact-columns",
            t,
            "columns",
            tr("Columns…"),
            None,
            false,
        ),
        cx,
        |this, _, cx| this.toggle_columns_menu(cleis_core::column::Table::Contacts, cx),
    ));
    let cards = s.contacts_ui.cards;
    r.row(act(
        item(
            "cx-contact-view",
            t,
            if cards { "list" } else { "columns" },
            if cards {
                tr("Show as table")
            } else {
                tr("Show as cards")
            },
            None,
            false,
        ),
        cx,
        move |this, _, cx| {
            if let Some(s) = this.session() {
                s.contacts_ui.cards = !cards;
            }
            cx.notify();
        },
    ));
    r.sep();
    r.row(act(
        item("cx-contact-import", t, "upload", tr("Import…"), None, false),
        cx,
        |this, _, cx| {
            if let Some(s) = this.session() {
                // D-99: into the organization shown, or the personal vault.
                let org = crate::screens::org::org_in_view(s).map(|o| o.m.key());
                s.tab = Tab::Settings;
                s.settings_page = if org.is_some() {
                    crate::screens::import::ORG_PAGE
                } else {
                    "Import & export"
                };
                if let Some(k) = org {
                    this.activate_org(&k);
                }
            }
            cx.notify();
        },
    ));
    r.menu
}

/// Mockup 35b: a right-click in the sidebar outside any folder.
fn sidebar_void_menu(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let mut r = Rows::new(t);
    if let Some(row) = new_folder_row(s, t, cx) {
        r.row(row);
        r.sep();
    }
    // Shortcuts under each vault's name (37j): changed there, with Customize.
    let split = super::sidebar::split_here(s);
    if !split {
        r.row(
            div()
                .id("cx-shortcuts-head")
                .h(px(24.))
                .px(px(8.))
                .flex()
                .items_center()
                .text_size(TEXT_XS)
                .font_weight(W_MEDIUM)
                .text_color(t.text_muted)
                .child(tr("Shortcuts"))
                .tooltip(tip(tr(
                    "For this vault; Preferences › Vaults makes them the same everywhere",
                ))),
        );
    }
    let key = super::sidebar::shortcut_key(s);
    let shown = s.prefs.shortcuts.of(&key);
    for (ix, label) in ["All items", "Favorites", "Renew soon"]
        .into_iter()
        .enumerate()
        .filter(|_| !split)
    {
        let key = key.clone();
        // A tick does not close the menu: several can be changed in a row.
        r.row(
            plain(check_item(format!("cx-sc-{ix}"), t, label, shown[ix]), cx)
                .on_click(cx.listener(move |this, _, _, cx| {
                    let key = key.clone();
                    this.update_prefs(|p| p.shortcuts.toggle(&key, ix), cx);
                    if let Some(s) = this.session() {
                        s.refilter();
                    }
                }))
                .test_support(),
        );
    }
    if !split {
        r.sep();
    }
    r.row(act(
        item("cx-customize", t, "grip", tr("Customize"), None, false),
        cx,
        |this, _, cx| this.toggle_customize(cx),
    ));
    r.menu
}

/// Mockup 37o: the password box's own menu (the row's menu is mockup 33).
fn password_field_menu(
    s: &Session,
    id: EntryId,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> Option<Div> {
    let e = s.entries.iter().find(|e| e.id == id)?;
    let mut r = Rows::new(t);
    if !e.password.is_empty() {
        let v = e.password.clone();
        r.row(act(
            item(
                "cx-pw-copy",
                t,
                "copy",
                tr("Copy"),
                Some(&*keys_of(s, "copy_pass")),
                false,
            ),
            cx,
            move |this, _, cx| this.copy_of(id, v.clone(), "Password", cx),
        ));
    }
    r.row(act(
        item(
            "cx-pw-paste",
            t,
            "upload",
            tr("Paste"),
            Some(tr("Ctrl+V")),
            false,
        ),
        cx,
        move |this, window, cx| this.paste_password(id, window, cx),
    ));
    let shown = s.reveal;
    r.row(act(
        item(
            "cx-pw-reveal",
            t,
            "eye",
            if shown { tr("Hide") } else { tr("Show") },
            None,
            false,
        ),
        cx,
        move |this, window, cx| this.set_reveal(!shown, window, cx),
    ));
    r.sep();
    r.row(act(
        item(
            "cx-pw-gen",
            t,
            "refresh",
            tr("Generate a password"),
            None,
            false,
        ),
        cx,
        move |this, window, cx| this.generate_password(id, window, cx),
    ));
    if !e.password_history.is_empty() {
        r.row(act(
            item(
                "cx-pw-history",
                t,
                "clock",
                tr("Previous passwords"),
                None,
                false,
            ),
            cx,
            |this, _, cx| {
                if let Some(s) = this.session() {
                    s.history_tab = true;
                }
                cx.notify();
            },
        ));
    }
    Some(r.menu)
}

fn entry_menu(
    s: &Session,
    id: EntryId,
    sub: Option<Sub>,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> Option<Built> {
    let e = s.entries.iter().find(|e| e.id == id)?;
    let mut r = Rows::new(t);
    if !e.username.is_empty() {
        let v = e.username.clone();
        r.row(act(
            item(
                "cx-copy-user",
                t,
                "user",
                tr("Copy username"),
                Some(&*keys_of(s, "copy_user")),
                false,
            ),
            cx,
            move |this, _, cx| this.copy_of(id, v.clone(), "Username", cx),
        ));
    }
    if !e.password.is_empty() {
        let v = e.password.clone();
        r.row(act(
            item(
                "cx-copy-pass",
                t,
                "key",
                tr("Copy password"),
                Some(&*keys_of(s, "copy_pass")),
                false,
            ),
            cx,
            move |this, _, cx| this.copy_of(id, v.clone(), "Password", cx),
        ));
    }
    if let Some(totp) = e.totp.clone() {
        r.row(act(
            item(
                "cx-copy-totp",
                t,
                "totp",
                tr("Copy 2FA code"),
                Some(&*keys_of(s, "copy_totp")),
                false,
            ),
            cx,
            move |this, _, cx| {
                let code = totp.code_at(cleis_core::now());
                this.copy_of(id, code, "Code", cx)
            },
        ));
    }
    if let Some(url) = e.urls.first() {
        let url = if url.contains("://") {
            url.clone()
        } else {
            format!("https://{url}")
        };
        r.row(act(
            item("cx-open", t, "external", tr("Open website"), None, false),
            cx,
            move |_, _, cx| cx.open_url(&url),
        ));
    }
    r.sep();
    r.row(act(
        item(
            "cx-fav",
            t,
            "star",
            if e.favorite {
                "Remove from favorites"
            } else {
                "Add to favorites"
            },
            None,
            false,
        ),
        cx,
        move |this, _, cx| this.update_entry(id, |e| e.favorite = !e.favorite, cx),
    ));
    r.row(act(
        item(
            "cx-duplicate",
            t,
            "copy",
            tr("Duplicate"),
            Some(&*keys_of(s, "duplicate")),
            false,
        ),
        cx,
        move |this, _, cx| this.duplicate_entry(id, cx),
    ));
    r.sub_y = r.y;
    r.row(parent(
        "cx-move",
        t,
        "folder",
        "Move to",
        Sub::Move,
        sub == Some(Sub::Move),
        cx,
    ));
    r.sep();
    r.row(act(
        item(
            "cx-trash",
            t,
            "trash",
            tr("Move to trash"),
            Some(&*keys_of(s, "trash")),
            true,
        ),
        cx,
        move |this, _, cx| this.trash_entry(id, cx),
    ));
    let subm = (sub == Some(Sub::Move)).then(|| {
        let from_org = s.org_of(e.space).map(|o| o.m.key());
        let mut m = menu_box(t).w(px(220.));
        for sp in s
            .spaces
            .iter()
            .filter(|sp| !sp.for_contacts && sp.id != e.space)
        {
            let to = sp.id;
            let reserved = s.org.as_ref().is_some_and(|o| o.is_reserved(to));
            let el = menu_item(
                format!("cx-move-{to}"),
                t,
                Some(if reserved { "lock" } else { "folder" }),
                crate::i18n::space(&sp.name),
                false,
            );
            // Audit L-2: an organization's item does not leave the organization.
            let to_org = s.org_of(to).map(|o| o.m.key());
            m = m.child(if from_org.is_some() && from_org != to_org {
                el.opacity(0.45)
                    .cursor_default()
                    .tooltip(tip(tr("An organization's items stay in the organization")))
                    .into_any_element()
            } else {
                el.on_click(cx.listener(move |this, _, _, cx| {
                    this.close_ctx(cx);
                    this.move_entry(id, to, cx);
                }))
                .test_support()
                .into_any_element()
            });
        }
        (r.sub_y, m)
    });
    Some((r.menu, subm))
}

fn space_menu(
    s: &Session,
    id: SpaceId,
    sub: Option<Sub>,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> Option<Built> {
    let sp = s.spaces.iter().find(|x| x.id == id)?;
    let mut r = Rows::new(t);
    // Contacts' folders: their icon only.
    if sp.for_contacts {
        let personal = sp.kind != cleis_store::SpaceKind::Organization;
        let manage = s.org_of(id).is_some_and(|o| o.can(Perm::Manage));
        if !(personal || manage) {
            return None;
        }
        r.sub_y = r.y;
        r.row(parent(
            "cx-icon",
            t,
            "palette",
            "Icon",
            Sub::Icon,
            sub == Some(Sub::Icon),
            cx,
        ));
        let subm = (sub == Some(Sub::Icon)).then(|| (r.sub_y, icon_grid(sp, t, cx)));
        return Some((r.menu, subm));
    }
    let new_here = item("cx-new-here", t, "plus", tr("New item here"), None, false);
    r.row(if s.may(id, Perm::Edit) {
        act(new_here, cx, move |this, window, cx| {
            if let Some(s) = this.session() {
                s.tab = Tab::Vault;
                s.filter = crate::app::Filter::Space(id);
                s.refilter();
            }
            this.new_entry(window, cx);
        })
    } else {
        off(new_here, "Your role does not allow adding here", cx)
    });
    if sp.kind == cleis_store::SpaceKind::Organization {
        let manage = s.org.as_ref().is_some_and(|o| o.can(Perm::Manage));
        if manage {
            r.row(act(
                item("cx-rename", t, "text", tr("Rename"), None, false),
                cx,
                move |this, window, cx| this.open_rename(id, window, cx),
            ));
            r.sub_y = r.y;
            r.row(parent(
                "cx-icon",
                t,
                "palette",
                "Icon",
                Sub::Icon,
                sub == Some(Sub::Icon),
                cx,
            ));
            let page = |label: &'static str| {
                move |this: &mut CleisApp, _: &mut Window, cx: &mut Context<CleisApp>| {
                    if let Some(s) = this.session() {
                        s.tab = Tab::Settings;
                        s.settings_page = label;
                    }
                    cx.notify();
                }
            };
            r.row(act(
                item("cx-members", t, "users", tr("Members…"), None, false),
                cx,
                page("Members"),
            ));
            r.row(act(
                item("cx-encryption", t, "key", tr("Encryption…"), None, false),
                cx,
                page("Encryption"),
            ));
            r.sep();
            r.row(act(
                item(
                    "cx-delete-space",
                    t,
                    "trash",
                    tr("Delete space"),
                    None,
                    true,
                ),
                cx,
                move |this, _, cx| {
                    if let Some(o) = super::org::org_mut(this) {
                        o.ui.delete_space = Some(id);
                    }
                    cx.notify();
                },
            ));
        }
    } else {
        r.row(act(
            item("cx-rename", t, "text", tr("Rename"), None, false),
            cx,
            move |this, window, cx| this.open_folder_rename(id, window, cx),
        ));
        r.sub_y = r.y;
        r.row(parent(
            "cx-icon",
            t,
            "palette",
            "Icon",
            Sub::Icon,
            sub == Some(Sub::Icon),
            cx,
        ));
        if s.vault.home_space() != id {
            r.sep();
            r.row(act(
                item("cx-delete-folder", t, "trash", tr("Delete"), None, true),
                cx,
                move |this, _, cx| {
                    if let Some(s) = this.session() {
                        s.folder = Some(FolderDialog::Delete(id));
                    }
                    cx.notify();
                },
            ));
        }
    }
    let subm = (sub == Some(Sub::Icon)).then(|| (r.sub_y, icon_grid(sp, t, cx)));
    Some((r.menu, subm))
}

/// Mockup 48g: the icons a folder can take, the current one outlined; Default puts the folder
/// icon back.
fn icon_grid(sp: &cleis_store::SpaceInfo, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let id = sp.id;
    let current = crate::ui::folder_icon(sp.icon.as_deref());
    let mut grid = div()
        .flex()
        .flex_wrap()
        .gap(px(4.))
        .w(px(32. * 6. + 4. * 5.));
    for name in crate::ui::FOLDER_ICONS {
        let on = name == current;
        let mut label = name.to_owned();
        if let Some(f) = label.get_mut(0..1) {
            f.make_ascii_uppercase();
        }
        grid = grid.child(
            div()
                .id(SharedString::from(format!("cx-icon-{name}")))
                .size(px(32.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(RADIUS_MD)
                .cursor_pointer()
                .when(on, |d| d.border_2().border_color(t.primary))
                .hover(|d| d.bg(t.surface_hover))
                .child(icon(name, 16., if on { t.text } else { t.text_muted }))
                .tooltip(crate::ui::tip(crate::i18n::tr_owned(&label)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.close_ctx(cx);
                    this.set_space_icon(id, (name != "folder").then_some(name), cx);
                }))
                .test_support(),
        );
    }
    menu_box(t)
        .w(px(32. * 6. + 4. * 5. + 16.))
        .min_w(px(0.))
        .p(px(8.))
        .gap(px(4.))
        .child(grid)
        .child(div().h(px(1.)).my(px(4.)).bg(t.line))
        .child(
            menu_item("cx-icon-default", t, Some("folder"), "Default", false)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.close_ctx(cx);
                    this.set_space_icon(id, None, cx);
                }))
                .test_support(),
        )
}

fn contact_menu(
    s: &Session,
    id: ContactId,
    sub: Option<Sub>,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> Option<Built> {
    let c = s.contacts.iter().find(|c| c.id == id)?;
    let mut r = Rows::new(t);
    let email = c.primary_email();
    let phone = c.primary_phone();
    let mail = item("cx-mail", t, "mail", tr("Send e-mail"), None, false);
    r.row(match email.clone() {
        Some(e) => act(mail, cx, move |_, _, cx| {
            cx.open_url(&format!("mailto:{e}"))
        }),
        None => off(mail, "No e-mail address", cx),
    });
    let call = item("cx-call", t, "phone", tr("Call"), None, false);
    r.row(match phone.clone() {
        Some(p) => act(call, cx, move |_, _, cx| {
            let n: String = p
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '+')
                .collect();
            // RFC 3966: no spaces in a tel: address (dots and dashes are allowed).
            cx.open_url(&format!("tel:{}", n.replace(' ', "")));
        }),
        None => off(call, "No phone number", cx),
    });
    let copy_y = r.y;
    r.row(parent(
        "cx-copy",
        t,
        "copy",
        "Copy",
        Sub::Copy,
        sub == Some(Sub::Copy),
        cx,
    ));
    r.sep();
    r.row(act(
        item(
            "cx-fav",
            t,
            "star",
            if c.favorite {
                "Remove from favorites"
            } else {
                "Add to favorites"
            },
            None,
            false,
        ),
        cx,
        move |this, _, cx| {
            this.update_contact(
                id,
                |c| {
                    c.favorite = !c.favorite;
                    None
                },
                cx,
            )
        },
    ));
    r.row(act(
        item("cx-vcard", t, "download", tr("Export vCard"), None, false),
        cx,
        move |this, _, cx| this.export_vcard(vec![id], cx),
    ));
    let move_y = r.y;
    r.row(parent(
        "cx-move",
        t,
        "folder",
        "Move to",
        Sub::Move,
        sub == Some(Sub::Move),
        cx,
    ));
    r.sep();
    r.row(act(
        item(
            "cx-delete-contact",
            t,
            "trash",
            tr("Delete"),
            Some(&*keys_of(s, "trash")),
            true,
        ),
        cx,
        move |this, _, cx| this.ask_delete_contact(id, cx),
    ));
    let subm = match sub {
        Some(Sub::Copy) => {
            let mut m = menu_box(t).w(px(220.));
            let mut add =
                |key: &str, label: &'static str, value: Option<String>, what: &'static str| {
                    let el = menu_item(
                        SharedString::from(format!("cx-copy-{key}")),
                        t,
                        None,
                        label,
                        false,
                    );
                    m = std::mem::replace(&mut m, div()).child(match value {
                        Some(v) => el
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.close_ctx(cx);
                                this.copy(v.clone(), what, cx);
                            }))
                            .test_support()
                            .into_any_element(),
                        None => el.opacity(0.45).cursor_default().into_any_element(),
                    });
                };
            add("email", "E-mail", email, "E-mail");
            add("phone", "Phone", phone, "Phone");
            add("card", "Card as text", Some(c.text_card()), "Card");
            Some((copy_y, m))
        }
        Some(Sub::Move) => {
            let from_org = s.org_of(c.space).map(|o| o.m.key());
            let mut m = menu_box(t).w(px(220.));
            for sp in s
                .spaces
                .iter()
                .filter(|sp| sp.for_contacts && sp.id != c.space)
            {
                let to = sp.id;
                let el = menu_item(
                    format!("cx-move-{to}"),
                    t,
                    Some("folder"),
                    crate::i18n::space(&sp.name),
                    false,
                );
                let to_org = s.org_of(to).map(|o| o.m.key());
                m = m.child(if from_org.is_some() && from_org != to_org {
                    el.opacity(0.45)
                        .cursor_default()
                        .tooltip(tip(tr(
                            "An organization's contacts stay in the organization",
                        )))
                        .into_any_element()
                } else {
                    el.on_click(cx.listener(move |this, _, _, cx| {
                        this.close_ctx(cx);
                        this.move_contact(id, to, cx);
                    }))
                    .test_support()
                    .into_any_element()
                });
            }
            Some((move_y, m))
        }
        _ => None,
    };
    Some((r.menu, subm))
}

fn member_menu(
    s: &Session,
    id: MemberId,
    sub: Option<Sub>,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> Option<Built> {
    use cleis_core::org::{SUPER_ADMIN, is_admin_role};
    let o = s.org.as_ref()?;
    let st = o.state.as_ref()?;
    let m = st.members.iter().find(|m| m.id == id)?;
    let i_admins = o.can(Perm::Admins);
    let admin = is_admin_role(&m.role);
    let editable = m.id != st.me.id && (!admin || i_admins) && o.can(Perm::Manage);
    // Role and spaces first, then the actions of the row's "⋯" menu.
    let mut head = Rows::new(t);
    let (mut role_y, mut spaces_y) = (PAD, PAD);
    if editable {
        role_y = head.y;
        head.row(parent(
            "cx-role",
            t,
            "shield",
            "Role",
            Sub::Role,
            sub == Some(Sub::Role),
            cx,
        ));
        if !admin {
            spaces_y = head.y;
            head.row(parent(
                "cx-spaces",
                t,
                "folder",
                "Spaces",
                Sub::Spaces,
                sub == Some(Sub::Spaces),
                cx,
            ));
        }
    }
    let actions = super::org::member_menu(o, st, m, t, cx);
    let menu = if editable {
        head.sep();
        head.menu.child(
            actions
                .relative()
                .p_0()
                .border_0()
                .shadow_none()
                .bg(gpui_kit::Hsla::transparent_black()),
        )
    } else {
        actions.relative()
    };
    let subm = match sub {
        Some(Sub::Role) if editable => {
            let mut sm = menu_box(t).w(px(220.));
            for r in st.roles.iter().filter(|r| i_admins || !r.is_admin()) {
                let rid = r.id.clone();
                let on = r.id == m.role;
                sm = sm.child(
                    menu_item(format!("cx-role-{}", r.id), t, None, r.name.clone(), false)
                        .child(div().flex_1())
                        .when(on, |d| d.child(icon("check", 14., t.text)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.close_ctx(cx);
                            if rid == SUPER_ADMIN {
                                this.ask_action(crate::app::ConfirmAction::MakeSuper(id), cx);
                                return;
                            }
                            this.org_member(
                                MemberChange {
                                    member: id,
                                    role: Some(rid.clone()),
                                    ..Default::default()
                                },
                                cx,
                            );
                        }))
                        .test_support(),
                );
            }
            Some((role_y, sm))
        }
        Some(Sub::Spaces) if editable => {
            let mut sm = menu_box(t).w(px(220.));
            for sv in &st.spaces {
                let on = m.spaces.contains(&sv.id);
                let sid = sv.id;
                let current = m.spaces.clone();
                let locked = sv.reserved && !i_admins;
                let el = menu_item(
                    format!("cx-msp-{sid}"),
                    t,
                    sv.reserved.then_some("lock"),
                    o.space_name(sid),
                    false,
                )
                .child(div().flex_1())
                .child(super::org::check_box(t, on));
                sm = sm.child(if locked {
                    el.opacity(0.45)
                        .cursor_default()
                        .tooltip(tip(tr("Reserved: a Super Admin changes its members")))
                        .into_any_element()
                } else {
                    el.on_click(cx.listener(move |this, _, _, cx| {
                        let mut next = current.clone();
                        if on {
                            next.retain(|x| *x != sid);
                        } else {
                            next.push(sid);
                        }
                        this.org_member(
                            MemberChange {
                                member: id,
                                spaces: Some(next),
                                ..Default::default()
                            },
                            cx,
                        );
                    }))
                    .test_support()
                    .into_any_element()
                });
            }
            Some((spaces_y, sm))
        }
        _ => None,
    };
    Some((menu, subm))
}

impl CleisApp {
    pub fn open_ctx(
        &mut self,
        target: CtxTarget,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(s) = self.session() else { return };
        // The right-clicked row keeps the selection and the inspector as they are: only a left
        // click opens an item. The row is outlined while its menu is open (`aimed`).
        s.folder_menu = None;
        s.filter_menu = false;
        if let Some(ed) = s.editor.as_mut() {
            ed.menu = None;
        }
        if let Some(o) = s.org.as_mut() {
            o.ui.menu = None;
        }
        s.ctx = Some(CtxMenu {
            at,
            target,
            sub: None,
        });
        let focus = s.main_focus.clone();
        window.focus(&focus, cx);
        cx.notify();
    }

    /// Sort by ▸ (mockup 35): kept in the preferences, the list sorted again at once.
    pub fn set_sort(&mut self, by: cleis_core::settings::SortBy, cx: &mut Context<Self>) {
        // "My order" keeps the order made before; items never placed follow, as shown now.
        if let Some(s) = self.session()
            && by == cleis_core::settings::SortBy::Manual
            && s.prefs.sort_by != by
        {
            let mut order: Vec<EntryId> = s
                .prefs
                .item_order
                .iter()
                .copied()
                .filter(|x| s.entries.iter().any(|e| e.id == *x))
                .collect();
            for i in &s.visible {
                let id = s.entries[*i].id;
                if !order.contains(&id) {
                    order.push(id);
                }
            }
            s.prefs.item_order = order;
        }
        self.update_prefs(|p| p.sort_by = by, cx);
        if let Some(s) = self.session() {
            s.refilter();
        }
        cx.notify();
    }

    pub fn close_menus(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        s.filter_menu = false;
        s.calendar = None;
        let wheel = s.wheel.take().is_some();
        s.sync_menu = false;
        s.account_menu = false;
        s.profile_edit = None;
        s.pref_menu = None;
        s.folder_menu = None;
        s.security.menu = None;
        s.contacts_ui.menu = None;
        s.import.menu = None;
        s.import.space_menu = false;
        s.health.vault_menu = false;
        s.health.space_menu = false;
        s.health.change = None;
        if let Some(e) = s.editor.as_mut() {
            e.menu = None;
        }
        if let Some(o) = s.org.as_mut() {
            o.ui.menu = None;
        }
        // The wheel's colour was only shown: the saved accent comes back.
        if wheel {
            let prefs = s.prefs.clone();
            self.apply_appearance(&prefs, cx);
        }
        cx.notify();
    }

    pub fn close_ctx(&mut self, cx: &mut Context<Self>) {
        if let Some(s) = self.session()
            && s.ctx.take().is_some()
        {
            cx.notify();
        }
    }

    fn ctx_sub(&mut self, sub: Option<Sub>, cx: &mut Context<Self>) {
        if let Some(s) = self.session()
            && let Some(m) = s.ctx.as_mut()
            && m.sub != sub
        {
            m.sub = sub;
            cx.notify();
        }
    }

    /// Shortcuts of the selected item or contact (mockups 33, 33c). Returns whether one ran.
    pub fn item_key(&mut self, k: &gpui_kit::Keystroke, cx: &mut Context<Self>) -> bool {
        let Some(s) = self.session() else {
            return false;
        };
        let m = &k.modifiers;
        let cmd = if cfg!(target_os = "macos") {
            m.platform
        } else {
            m.control
        };
        let del = !cmd
            && !m.alt
            && !m.shift
            && (k.key == "delete" || (cfg!(target_os = "macos") && k.key == "backspace"));
        // Audit U-3: arrows move the selection through the list.
        let step = match (cmd || m.alt || m.shift, k.key.as_str()) {
            (false, "down") => Some(1isize),
            (false, "up") => Some(-1),
            _ => None,
        };
        if let Some(step) = step {
            match s.tab {
                Tab::Vault => {
                    let ids: Vec<EntryId> = s.visible.iter().map(|i| s.entries[*i].id).collect();
                    let at = s.selected.and_then(|id| ids.iter().position(|x| *x == id));
                    let next = match at {
                        Some(i) => (i as isize + step).clamp(0, ids.len() as isize - 1) as usize,
                        None => 0,
                    };
                    if let Some(id) = ids.get(next) {
                        s.selected = Some(*id);
                        s.reveal = false;
                    }
                }
                Tab::Contacts => {
                    let ids: Vec<ContactId> =
                        super::contacts::listed(s).iter().map(|c| c.id).collect();
                    let at = s
                        .selected_contact
                        .and_then(|id| ids.iter().position(|x| *x == id));
                    let next = match at {
                        Some(i) => (i as isize + step).clamp(0, ids.len() as isize - 1) as usize,
                        None => 0,
                    };
                    if let Some(id) = ids.get(next) {
                        s.selected_contact = Some(*id);
                    }
                }
                _ => return false,
            }
            cx.notify();
            return true;
        }
        // With a row's menu open, its keys act on that row, not on the selection.
        let aimed = s.ctx.as_ref().map(|m| m.target);
        match s.tab {
            Tab::Vault => {
                let id = match aimed {
                    Some(CtxTarget::Entry(id)) => Some(id),
                    _ => s.selected,
                };
                let Some(e) = id
                    .and_then(|id| s.entries.iter().find(|e| e.id == id))
                    .cloned()
                else {
                    return false;
                };
                if aimed.is_some() {
                    s.ctx = None;
                }
                let id = e.id;
                // Mockup 38: the keys chosen in Settings › Shortcuts.
                let p = s.prefs.clone();
                let on = |name: &str| super::keys::pressed(k, &p.app_key(name));
                if on("copy_user") && !e.username.is_empty() {
                    self.copy_of(id, e.username.clone(), "Username", cx)
                } else if on("copy_pass") && !e.password.is_empty() {
                    self.copy_of(id, e.password.clone(), "Password", cx)
                } else if on("copy_totp") && e.totp.is_some() {
                    let code = e.totp.as_ref().map(|t| t.code_at(cleis_core::now()));
                    self.copy_of(id, code.unwrap_or_default(), "Code", cx)
                } else if on("duplicate") {
                    self.duplicate_entry(id, cx)
                } else if on("trash") {
                    self.trash_entry(id, cx)
                } else {
                    return false;
                }
                true
            }
            Tab::Contacts if del || super::keys::pressed(k, &s.prefs.app_key("trash")) => {
                let id = match aimed {
                    Some(CtxTarget::Contact(id)) => Some(id),
                    _ => s.selected_contact,
                };
                let Some(id) = id else {
                    return false;
                };
                s.ctx = None;
                self.ask_delete_contact(id, cx);
                true
            }
            _ => false,
        }
    }

    /// Closes the topmost menu or question (Escape, audit U-2). Returns whether one was open.
    pub fn close_top(&mut self, cx: &mut Context<Self>) -> bool {
        // Mockup 48j: Esc stops the middle click's scrolling first.
        if self.stop_autoscroll(cx) {
            return true;
        }
        let Some(s) = self.session() else {
            return false;
        };
        let closed = if s.ctx.is_some() {
            s.ctx = None;
            true
        } else if s.calendar.is_some() {
            s.calendar = None;
            true
        } else if s.wheel.is_some() {
            self.toggle_wheel(cx);
            return true;
        } else if s.confirm.is_some() {
            s.confirm = None;
            true
        } else if s.erase.as_ref().is_some_and(|e| !e.busy) {
            s.erase = None;
            true
        } else if s.pin_setup.as_ref().is_some_and(|p| !p.busy) {
            s.pin_setup = None;
            true
        } else if s.folder.is_some() || s.folder_menu.is_some() {
            s.folder = None;
            s.folder_menu = None;
            true
        } else if s.place.is_some() {
            s.place = None;
            true
        } else if s.security.dialog.is_some() || s.security.menu.is_some() {
            s.security.dialog = None;
            s.security.menu = None;
            true
        } else if s.filter_menu || s.sync_menu || s.pref_menu.is_some() || s.columns_menu.is_some()
        {
            s.filter_menu = false;
            s.sync_menu = false;
            s.pref_menu = None;
            s.columns_menu = None;
            true
        } else if s.health.vault_menu || s.health.space_menu || s.health.change.is_some() {
            s.health.vault_menu = false;
            s.health.space_menu = false;
            s.health.change = None;
            true
        } else if s.contacts_ui.menu.is_some() {
            s.contacts_ui.menu = None;
            true
        } else if let Some(ed) = s.editor.as_mut().filter(|ed| ed.menu.is_some()) {
            ed.menu = None;
            true
        } else if s.customize {
            s.customize = false;
            true
        } else if let Some(o) = s.org.as_mut() {
            o.ui.close_top()
        } else {
            false
        };
        if closed {
            cx.notify();
        }
        closed
    }
}
