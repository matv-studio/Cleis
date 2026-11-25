//! Menus opened by a right-click (mockups 33 to 33d): an item of the vault, a space or folder of
//! the sidebar, a contact, a member. The menu opens at the pointer, a click outside or Escape
//! closes it, and a "▸" row opens its submenu on hover.

use crate::app::{CleisApp, FolderDialog, Session, Tab};
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sub {
    Move,
    Copy,
    Role,
    Spaces,
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
    move |window, cx| gpui_kit::component::tooltip::Tooltip::new(text).build(window, cx)
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
        .when_some(shortcut, |d, k| {
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
        || s.sync_menu
        || s.pref_menu.is_some()
        || s.folder_menu.is_some()
        || s.security.menu.is_some()
        || s.contacts_ui.menu.is_some()
        || s.import.menu.is_some()
        || s.import.space_menu
        || s.health.vault_menu
        || s.health.space_menu
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

pub fn render(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Option<AnyElement> {
    let m = s.ctx.as_ref()?;
    let (main, sub) = match m.target {
        CtxTarget::Entry(id) => entry_menu(s, id, m.sub, t, cx)?,
        CtxTarget::Space(id) => (space_menu(s, id, t, cx)?, None),
        CtxTarget::Contact(id) => contact_menu(s, id, m.sub, t, cx)?,
        CtxTarget::Member(id) => member_menu(s, id, m.sub, t, cx)?,
    };
    let body = div()
        .flex()
        .items_start()
        .child(main)
        .when_some(sub, |d, (y, menu)| {
            d.child(menu.relative().ml(px(-4.)).mt(px(y - PAD)))
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
                        .child(body),
                ),
        )
        .with_priority(2)
        .into_any_element(),
    )
}

type Built = (Div, Option<(f32, Div)>);

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
                "Copy username",
                Some("Ctrl+C"),
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
                "Copy password",
                Some("Ctrl+Shift+C"),
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
                "Copy 2FA code",
                Some("Ctrl+Alt+C"),
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
            item("cx-open", t, "external", "Open website", None, false),
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
            "Duplicate",
            Some("Ctrl+D"),
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
        item("cx-trash", t, "trash", "Move to trash", Some("Del"), true),
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
                sp.name.clone(),
                false,
            );
            // Audit L-2: an organization's item does not leave the organization.
            let to_org = s.org_of(to).map(|o| o.m.key());
            m = m.child(if from_org.is_some() && from_org != to_org {
                el.opacity(0.45)
                    .cursor_default()
                    .tooltip(tip("An organization's items stay in the organization"))
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

fn space_menu(s: &Session, id: SpaceId, t: &Tokens, cx: &mut Context<CleisApp>) -> Option<Div> {
    let sp = s.spaces.iter().find(|x| x.id == id)?;
    let mut r = Rows::new(t);
    let new_here = item("cx-new-here", t, "plus", "New item here", None, false);
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
                item("cx-rename", t, "text", "Rename", None, false),
                cx,
                move |this, window, cx| this.open_rename(id, window, cx),
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
                item("cx-members", t, "users", "Members…", None, false),
                cx,
                page("Members"),
            ));
            r.row(act(
                item("cx-encryption", t, "key", "Encryption…", None, false),
                cx,
                page("Encryption"),
            ));
            r.sep();
            r.row(act(
                item("cx-delete-space", t, "trash", "Delete space", None, true),
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
            item("cx-rename", t, "text", "Rename", None, false),
            cx,
            move |this, window, cx| this.open_folder_rename(id, window, cx),
        ));
        if s.vault.home_space() != id {
            r.sep();
            r.row(act(
                item("cx-delete-folder", t, "trash", "Delete", None, true),
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
    Some(r.menu)
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
    let mail = item("cx-mail", t, "mail", "Send e-mail", None, false);
    r.row(match email.clone() {
        Some(e) => act(mail, cx, move |_, _, cx| {
            cx.open_url(&format!("mailto:{e}"))
        }),
        None => off(mail, "No e-mail address", cx),
    });
    let call = item("cx-call", t, "phone", "Call", None, false);
    r.row(match phone.clone() {
        Some(p) => act(call, cx, move |_, _, cx| {
            let n: String = p
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '+')
                .collect();
            cx.open_url(&format!("tel:{n}"));
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
        item("cx-vcard", t, "download", "Export vCard", None, false),
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
        item("cx-delete-contact", t, "trash", "Delete", Some("Del"), true),
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
                    sp.name.clone(),
                    false,
                );
                let to_org = s.org_of(to).map(|o| o.m.key());
                m = m.child(if from_org.is_some() && from_org != to_org {
                    el.opacity(0.45)
                        .cursor_default()
                        .tooltip(tip("An organization's contacts stay in the organization"))
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
                        .tooltip(tip("Reserved: a Super Admin changes its members"))
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
        // The row right-clicked becomes the selection, as in the file managers.
        match target {
            CtxTarget::Entry(id) => {
                if s.selected != Some(id) {
                    s.selected = Some(id);
                    s.reveal = false;
                }
            }
            CtxTarget::Contact(id) => s.selected_contact = Some(id),
            _ => {}
        }
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

    pub fn close_menus(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        s.filter_menu = false;
        s.sync_menu = false;
        s.pref_menu = None;
        s.folder_menu = None;
        s.security.menu = None;
        s.contacts_ui.menu = None;
        s.import.menu = None;
        s.import.space_menu = false;
        s.health.vault_menu = false;
        s.health.space_menu = false;
        if let Some(e) = s.editor.as_mut() {
            e.menu = None;
        }
        if let Some(o) = s.org.as_mut() {
            o.ui.menu = None;
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
        match s.tab {
            Tab::Vault => {
                let Some(e) = s.selected_entry().cloned() else {
                    return false;
                };
                let id = e.id;
                match (cmd, m.shift, m.alt, k.key.as_str()) {
                    (true, false, false, "c") if !e.username.is_empty() => {
                        self.copy_of(id, e.username.clone(), "Username", cx)
                    }
                    (true, true, false, "c") if !e.password.is_empty() => {
                        self.copy_of(id, e.password.clone(), "Password", cx)
                    }
                    (true, false, true, "c") if e.totp.is_some() => {
                        let code = e.totp.as_ref().map(|t| t.code_at(cleis_core::now()));
                        self.copy_of(id, code.unwrap_or_default(), "Code", cx)
                    }
                    (true, false, false, "d") => self.duplicate_entry(id, cx),
                    _ if del => self.trash_entry(id, cx),
                    _ => return false,
                }
                true
            }
            Tab::Contacts if del => {
                let Some(id) = s.selected_contact else {
                    return false;
                };
                self.ask_delete_contact(id, cx);
                true
            }
            _ => false,
        }
    }

    /// Closes the topmost menu or question (Escape, audit U-2). Returns whether one was open.
    pub fn close_top(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(s) = self.session() else {
            return false;
        };
        let closed = if s.ctx.is_some() {
            s.ctx = None;
            true
        } else if s.confirm.is_some() {
            s.confirm = None;
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
        } else if s.health.vault_menu || s.health.space_menu {
            s.health.vault_menu = false;
            s.health.space_menu = false;
            true
        } else if s.contacts_ui.menu.is_some() {
            s.contacts_ui.menu = None;
            true
        } else if let Some(ed) = s.editor.as_mut().filter(|ed| ed.menu.is_some()) {
            ed.menu = None;
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
