//! Organization screens (mockups 07 Members, 08/18 Security policy, 09/10 Encryption, 11
//! Activity, 13 Sync & backups of the organization) and their dialogs: invitation, new space,
//! new role, re-encryption, verification code asked by the server.

use crate::app::{CleisApp, Session};
use crate::i18n::tr;
use crate::org::{OrgSession, initials};
use crate::theme::*;
use crate::ui::*;
use cleis_core::org::*;
use cleis_core::{MemberId, SpaceId};
use gpui_kit::TestSupportExt;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, Entity, SharedString, Subscription, Window, deferred, div, px};
use std::collections::BTreeSet;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrgMenu {
    Role(MemberId),
    Spaces(MemberId),
    More(MemberId),
    Policy(&'static str),
    Log(&'static str),
    InviteRole,
}

pub struct InviteForm {
    pub email: Entity<InputState>,
    pub name: Entity<InputState>,
    pub role: String,
    pub spaces: BTreeSet<SpaceId>,
    pub error: Option<SharedString>,
}

pub struct SpaceForm {
    pub name: Entity<InputState>,
    pub mode: SpaceMode,
    pub reserved: bool,
}

pub struct RoleForm {
    pub name: Entity<InputState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RekeyForm {
    pub space: SpaceId,
    pub to: SpaceMode,
    pub backup_first: bool,
}

/// Join form of Settings › Organization and of the first launch.
#[derive(Default)]
pub struct JoinForm {
    pub address: Option<Entity<InputState>>,
    pub code: Option<Entity<InputState>>,
    pub name: Option<Entity<InputState>>,
    pub email: Option<Entity<InputState>>,
    /// What the code is for, once checked.
    pub info: Option<InviteInfo>,
    pub checking: bool,
    pub error: Option<SharedString>,
    pub busy: bool,
    /// Create an organization: its name, and where the server runs (D-63).
    pub org_name: Option<Entity<InputState>>,
    /// Your name as the new organization's first member (audit L-10).
    pub creator: Option<Entity<InputState>>,
    pub elsewhere: bool,
}

#[derive(Default)]
pub struct OrgUi {
    pub subs: Vec<Subscription>,
    pub menu: Option<OrgMenu>,
    pub invite: Option<InviteForm>,
    /// Code of the invitation just made, shown with a copy button.
    pub invite_code: Option<String>,
    /// Where the server e-mailed the invitation, if it did.
    pub invite_sent: Option<String>,
    pub new_space: Option<SpaceForm>,
    pub new_role: Option<RoleForm>,
    pub rekey: Option<RekeyForm>,
    /// Encryption page: modes chosen but not applied yet.
    pub changes: Vec<(SpaceId, SpaceMode)>,
    pub search: Option<Entity<InputState>>,
    pub log_search: Option<Entity<InputState>>,
    pub log: Option<LogPage>,
    pub log_busy: bool,
    pub log_member: Option<MemberId>,
    pub log_action: Option<Action>,
    pub log_space: Option<SpaceId>,
    /// Last N days (None: all kept).
    pub log_days: Option<u32>,
    /// Sync & backups: the organization's view rather than "Mine".
    pub org_backups: bool,
    /// Backup asked to be restored (confirmation).
    pub restore: Option<std::path::PathBuf>,
    /// Settings › Devices, as last fetched.
    pub devices: Option<Vec<DeviceView>>,
    pub devices_busy: bool,
    /// Settings › Profile boxes.
    pub profile: Option<(Entity<InputState>, Entity<InputState>)>,
    /// Settings › Spaces: renaming one (its name box), deleting one (confirmation).
    pub rename: Option<(SpaceId, Entity<InputState>)>,
    /// Server page: the address being changed (audit L-6), and its subscription.
    pub address: Option<(Entity<InputState>, Subscription)>,
    /// Space to reserve once its re-encryption to end-to-end is done.
    pub reserve_after: Option<SpaceId>,
    pub delete_space: Option<SpaceId>,
    /// Server hosted on this computer: running, and the addresses members can reach.
    pub host: Option<(bool, Vec<String>)>,
    pub host_busy: bool,
}

impl OrgUi {
    /// Escape: the open menu first, then the open dialog. Returns whether one was open.
    pub fn close_top(&mut self) -> bool {
        if self.menu.take().is_some() || self.address.take().is_some() {
            return true;
        }
        let open = self.invite.is_some()
            || self.invite_code.is_some()
            || self.new_space.is_some()
            || self.new_role.is_some()
            || self.rekey.is_some()
            || self.restore.is_some()
            || self.rename.is_some()
            || self.delete_space.is_some();
        self.invite = None;
        self.invite_code = None;
        self.new_space = None;
        self.new_role = None;
        self.rekey = None;
        self.restore = None;
        self.rename = None;
        self.delete_space = None;
        open
    }

    pub fn log_query(&self) -> LogQuery {
        LogQuery {
            member: self.log_member,
            action: self.log_action,
            space: self.log_space,
            after: self
                .log_days
                .map(|d| cleis_core::now() - i64::from(d) * 86_400),
            before: None,
            limit: 2000,
        }
    }
}

fn tip(
    text: impl Into<SharedString>,
) -> impl Fn(&mut Window, &mut gpui_kit::App) -> gpui_kit::AnyView + 'static {
    let text: SharedString = text.into();
    move |window, cx| {
        gpui_kit::component::tooltip::Tooltip::new(crate::i18n::tr_dynamic(&text)).build(window, cx)
    }
}

pub type Pick = Rc<dyn Fn(&mut CleisApp, usize, &mut Context<CleisApp>)>;

/// Segmented control whose options call `pick(index)`.
pub fn segment(
    id: &str,
    t: &Tokens,
    options: &[&str],
    selected: usize,
    pick: Pick,
    cx: &mut Context<CleisApp>,
) -> Div {
    seg(id, t, options, selected, pick, cx)
}

fn seg(
    id: &str,
    t: &Tokens,
    options: &[&str],
    selected: usize,
    pick: Pick,
    cx: &mut Context<CleisApp>,
) -> Div {
    let mut d = div()
        .flex()
        .flex_none()
        .p(px(2.))
        .gap(px(2.))
        .bg(t.bg)
        .border_1()
        .border_color(t.line_strong)
        .rounded(RADIUS_MD);
    for (i, o) in options.iter().enumerate() {
        let on = i == selected;
        let pick = pick.clone();
        d = d.child(
            div()
                .id(SharedString::from(format!("{id}-{i}")))
                .h(px(22.))
                .px(px(12.))
                .flex()
                .items_center()
                .rounded(RADIUS_SM)
                .text_size(TEXT_MD)
                .font_weight(W_MEDIUM)
                .cursor_pointer()
                .when(on, |d| d.bg(t.accent_soft).text_color(t.accent_text))
                .when(!on, |d| d.text_color(t.text_muted))
                .child(crate::i18n::tr_owned(o))
                .on_click(cx.listener(move |this, _, _, cx| pick(this, i, cx)))
                .test_support(),
        );
    }
    d
}

/// A select box opening a menu of `(label, current)`; `pick(index)` on choice.
#[allow(clippy::too_many_arguments)]
fn select(
    id: &'static str,
    t: &Tokens,
    w: f32,
    open: bool,
    menu: OrgMenu,
    options: Vec<(String, bool)>,
    pick: Pick,
    cx: &mut Context<CleisApp>,
) -> Div {
    let label = options
        .iter()
        .find(|o| o.1)
        .map(|o| o.0.clone())
        .unwrap_or_default();
    let mut m = menu_box(t).top(px(30.)).left_0().min_w(px(w));
    for (i, (name, on)) in options.into_iter().enumerate() {
        let pick = pick.clone();
        m = m.child(
            menu_item(format!("{id}-{i}"), t, None, name, false)
                .when(on, |d| d.text_color(t.accent_text))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(o) = org_mut(this) {
                        o.ui.menu = None;
                    }
                    pick(this, i, cx);
                    cx.notify();
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
                        .w(px(w))
                        .h(px(26.))
                        .cursor_pointer()
                        .child(
                            div()
                                .flex_1()
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .child(crate::i18n::tr_owned(&label)),
                        )
                        .child(icon("chevron", 14., t.text_muted)),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(o) = org_mut(this) {
                        o.ui.menu = if o.ui.menu == Some(menu) {
                            None
                        } else {
                            Some(menu)
                        };
                    }
                    cx.notify();
                }))
                .test_support(),
        )
        .when(open, |d| d.child(deferred(m)))
}

pub fn org_mut(app: &mut CleisApp) -> Option<&mut OrgSession> {
    app.session().and_then(|s| s.org.as_mut())
}

pub fn check_box(t: &Tokens, on: bool) -> Div {
    div()
        .size(px(16.))
        .flex_none()
        .rounded(RADIUS_SM)
        .border_1()
        .border_color(if on { t.accent_text } else { t.line_strong })
        .when(on, |d| d.bg(t.accent_text))
        .flex()
        .items_center()
        .justify_center()
        .when(on, |d| d.child(icon("check", 12., t.primary_ink)))
}

fn head(t: &Tokens, title: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(12.))
        .child(
            div()
                .flex_1()
                .text_size(TEXT_XL)
                .line_height(px(26.))
                .font_weight(W_STRONG)
                .child(crate::i18n::tr_owned(title)),
        )
        .text_color(t.text)
}

fn titled(t: &Tokens, title: &str, info: Option<&'static str>) -> gpui_kit::Stateful<Div> {
    let d = div()
        .id(SharedString::from(format!("st-{title}")))
        .child(section_title(t, title));
    match info {
        Some(i) => d.tooltip(tip(i)),
        None => d,
    }
}

// ---------- Members (07) ----------

pub fn members(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let Some(o) = s.org.as_ref() else {
        return div();
    };
    let Some(st) = o.state.as_ref() else {
        return head(t, tr("Members")).child(chip(t, Tone::Neutral, tr("Not reached yet"), false));
    };
    let q =
        o.ui.search
            .as_ref()
            .map(|i| i.read(cx).value().to_string())
            .unwrap_or_default();
    let ql = cleis_core::text::fold(q.trim());
    let mut page = div().flex().flex_col().gap(px(16.));
    page = page.child(
        head(t, tr("Members"))
            .when_some(o.ui.search.as_ref(), |d, i| {
                d.child(div().w(px(220.)).child(Input::new(i).id("member-search")))
            })
            .child(
                button("invite", t, BtnKind::Primary, false)
                    .child(icon("plus", 16., t.primary_ink))
                    .child(tr("Invite"))
                    .on_click(cx.listener(|this, _, window, cx| this.open_invite(window, cx)))
                    .test_support(),
            ),
    );
    let i_admins = o.can(Perm::Admins);
    let role_name = |id: &str| {
        st.roles
            .iter()
            .find(|r| r.id == id)
            .map(|r| crate::i18n::tr_owned(&r.name))
            .unwrap_or_else(|| id.to_owned())
    };
    let cols = |d: Div| d.flex().items_center().gap(px(12.)).px(px(16.));
    let mut table = div().flex().flex_col().child(
        cols(div())
            .h(px(32.))
            .border_b_1()
            .border_color(t.line)
            .text_size(TEXT_XS)
            .text_color(t.text_muted)
            .font_weight(W_MEDIUM)
            .child(div().w(px(240.)).child(tr("Member")))
            .child(div().w(px(130.)).child(tr("Role")))
            .child(div().flex_1().child(tr("Spaces")))
            .child(div().w(px(80.)).child(tr("2-step")))
            .child(
                div()
                    .id("h-export")
                    .w(px(56.))
                    .child(tr("Export"))
                    .tooltip(tip(tr("Allowed to export the organization's passwords"))),
            )
            .child(div().w(px(140.)).child(tr("Last sign-in")))
            .child(div().w(px(28.))),
    );
    for m in st.members.iter().filter(|m| {
        ql.is_empty()
            || cleis_core::text::fold(&m.name).contains(&ql)
            || cleis_core::text::fold(&m.email).contains(&ql)
    }) {
        let id = m.id;
        let me = m.id == st.me.id;
        let admin = is_admin_role(&m.role);
        // Only a Super Admin gives or takes an admin role (D-60).
        let offered: Vec<&Role> = st
            .roles
            .iter()
            .filter(|r| i_admins || !r.is_admin())
            .collect();
        let roles: Vec<(String, bool)> = offered
            .iter()
            .map(|r| (crate::i18n::tr_owned(&r.name), r.id == m.role))
            .collect();
        let role_ids: Vec<String> = offered.iter().map(|r| r.id.clone()).collect();
        let role_sel = select(
            leak(format!("role-{id}")),
            t,
            120.,
            o.ui.menu == Some(OrgMenu::Role(id)),
            OrgMenu::Role(id),
            roles,
            Rc::new(move |app, i, cx| {
                app.org_member(
                    MemberChange {
                        member: id,
                        role: role_ids.get(i).cloned(),
                        ..Default::default()
                    },
                    cx,
                )
            }),
            cx,
        );
        let mut chips = div().flex().flex_wrap().gap(px(4.));
        if admin {
            chips = chips.child(chip(t, Tone::Neutral, tr("All"), false));
        } else if m.spaces.is_empty() {
            chips = chips.child(div().text_color(t.text_faint).child("—"));
        } else {
            for sp in &m.spaces {
                chips = chips.child(chip(t, Tone::Neutral, o.space_name(*sp), false));
            }
        }
        // Spaces menu: each space with a check.
        let mut sm = menu_box(t).top(px(26.)).left_0().w(px(220.));
        for sv in &st.spaces {
            let on = m.spaces.contains(&sv.id);
            let sid = sv.id;
            let current = m.spaces.clone();
            let locked = sv.reserved && !i_admins;
            sm = sm.child(
                menu_item(
                    format!("msp-{id}-{sid}"),
                    t,
                    sv.reserved.then_some("lock"),
                    o.space_name(sid),
                    false,
                )
                .child(div().flex_1())
                .child(check_box(t, on))
                .when(locked, |d| {
                    d.opacity(0.45)
                        .tooltip(tip(tr("Reserved: a Super Admin changes its members")))
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    if locked {
                        return;
                    }
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
                .test_support(),
            );
        }
        let spaces_cell = div()
            .relative()
            .flex_1()
            .child(
                div()
                    .id(SharedString::from(format!("spaces-{id}")))
                    .cursor_pointer()
                    .when(!admin, |d| d.tooltip(tip(tr("Change the spaces"))))
                    .child(chips)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if admin {
                            return;
                        }
                        if let Some(o) = org_mut(this) {
                            o.ui.menu = if o.ui.menu == Some(OrgMenu::Spaces(id)) {
                                None
                            } else {
                                Some(OrgMenu::Spaces(id))
                            };
                        }
                        cx.notify();
                    }))
                    .test_support(),
            )
            .when(o.ui.menu == Some(OrgMenu::Spaces(id)), |d| {
                d.child(deferred(sm))
            });
        let two_fa = if m.two_fa {
            chip(t, Tone::Ok, tr("On"), true)
        } else {
            chip(t, Tone::Warn, tr("To set up"), true)
        };
        let export = switch(SharedString::from(format!("export-{id}")), t, m.export)
            .when(admin, |d| {
                d.opacity(0.5)
                    .tooltip(tip(tr("Administrators can always export")))
            })
            .when(!admin, |d| {
                let on = m.export;
                d.on_click(cx.listener(move |this, _, _, cx| {
                    this.org_member(
                        MemberChange {
                            member: id,
                            export: Some(!on),
                            ..Default::default()
                        },
                        cx,
                    )
                }))
            })
            .test_support();
        let removable = !admin || i_admins;
        let more_menu = member_menu(o, st, m, t, cx).top(px(28.)).right_0();
        let editable = !me && (!admin || i_admins);
        table = table.child(
            cols(div())
                .id(SharedString::from(format!("member-{id}")))
                .min_h(px(52.))
                .py(px(8.))
                .when(!me, |d| {
                    d.on_mouse_down(
                        gpui_kit::MouseButton::Right,
                        super::ctx::on_right(super::ctx::CtxTarget::Member(id), cx),
                    )
                })
                .border_b_1()
                .border_color(t.line)
                .child(
                    div()
                        .w(px(240.))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(avatar(t, &initials(&m.name), 28.))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .min_w_0()
                                .child(
                                    div()
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .child(m.name.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(TEXT_SM)
                                        .text_color(t.text_muted)
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .child(m.email.clone()),
                                ),
                        ),
                )
                .child(div().w(px(130.)).child(if !editable {
                    div().child(role_name(&m.role)).text_color(t.text_muted)
                } else {
                    role_sel
                }))
                .child(spaces_cell)
                .child(div().w(px(80.)).flex().child(two_fa))
                .child(div().w(px(56.)).child(export))
                .child(div().w(px(140.)).text_color(t.text_muted).child(
                    m.last_sign_in.map_or("—".into(), |ts| {
                        crate::format::when_short(ts, cleis_core::now())
                    }),
                ))
                .child(
                    div()
                        .w(px(28.))
                        .relative()
                        .when(!me && removable, |d| {
                            d.child(
                                icon_button(
                                    SharedString::from(format!("more-{id}")),
                                    t,
                                    "more",
                                    true,
                                )
                                .tooltip(tip(tr("More")))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if let Some(o) = org_mut(this) {
                                        o.ui.menu = if o.ui.menu == Some(OrgMenu::More(id)) {
                                            None
                                        } else {
                                            Some(OrgMenu::More(id))
                                        };
                                    }
                                    cx.notify();
                                }))
                                .test_support(),
                            )
                        })
                        .when(o.ui.menu == Some(OrgMenu::More(id)), |d| {
                            d.child(deferred(more_menu))
                        }),
                )
                .test_support(),
        );
    }
    for inv in &st.invites {
        let id = inv.id;
        let days = (inv.expires - cleis_core::now()).div_euclid(86_400).max(0);
        table = table.child(
            cols(div())
                .min_h(px(52.))
                .py(px(8.))
                .text_color(t.text_muted)
                .child(
                    div()
                        .size(px(28.))
                        .rounded_full()
                        .border_1()
                        .border_dashed()
                        .border_color(t.line_strong)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icon("mail", 14., t.text_muted)),
                )
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .child(div().text_color(t.text).child(inv.email.clone()))
                        .child(div().text_size(TEXT_SM).child(crate::trf!(
                            "Invited {} · expires in {} day{}",
                            crate::format::date(inv.created),
                            days,
                            if days == 1 { "" } else { "s" }
                        ))),
                )
                .child(
                    button(
                        SharedString::from(format!("resend-{id}")),
                        t,
                        BtnKind::Ghost,
                        true,
                    )
                    .child(tr("Resend"))
                    .tooltip(tip(tr("New code, valid 7 days")))
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.org_invite_action(id, true, cx)),
                    )
                    .test_support(),
                )
                .child(
                    button(
                        SharedString::from(format!("unvite-{id}")),
                        t,
                        BtnKind::DangerGhost,
                        true,
                    )
                    .child(tr("Revoke"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.ask_action(crate::app::ConfirmAction::RevokeInvite(id), cx)
                    }))
                    .test_support(),
                ),
        );
    }
    page = page.child(section(t).p_0().overflow_hidden().child(table));

    // Roles matrix.
    let mut matrix = div().flex().flex_col().child(
        div()
            .flex()
            .h(px(32.))
            .items_center()
            .text_size(TEXT_XS)
            .text_color(t.text_muted)
            .font_weight(W_MEDIUM)
            .border_b_1()
            .border_color(t.line)
            .child(div().flex_1())
            .children(st.roles.iter().map(|r| {
                div()
                    .id(SharedString::from(format!("role-head-{}", r.id)))
                    .w(px(110.))
                    .flex()
                    .justify_center()
                    .when(r.id == cleis_core::org::ADMIN, |d| {
                        d.text_color(t.set_text)
                            .tooltip(tip(tr("Set by a Super Admin")))
                    })
                    .child(crate::i18n::tr_owned(&r.name))
            })),
    );
    for p in Perm::ALL {
        let mut line = div()
            .flex()
            .h(px(34.))
            .items_center()
            .border_b_1()
            .border_color(t.line)
            .child(
                div()
                    .id(SharedString::from(format!("perm-label-{p:?}")))
                    .flex_1()
                    .when_some(perm_tip(p), |d, tx| d.tooltip(tip(tx)))
                    .child(tr(p.label())),
            );
        for (ri, r) in st.roles.iter().enumerate() {
            let on = r.can(p);
            // D-67: the Admin column's delegable rights are set by a Super Admin.
            let delegable = cleis_core::org::DELEGABLE.contains(&p);
            let fixed = !i_admins
                || r.is_super()
                || p == Perm::Admins
                || if r.id == cleis_core::org::ADMIN {
                    !delegable
                } else {
                    delegable
                };
            let roles = st.roles.clone();
            line = line.child(
                div().w(px(110.)).flex().justify_center().child(
                    div()
                        .id(SharedString::from(format!("perm-{}-{p:?}", r.id)))
                        .when(!fixed, |d| d.cursor_pointer())
                        .when(fixed, |d| {
                            d.opacity(0.6).tooltip(tip(if r.is_super() {
                                "The Super Admin can do everything"
                            } else if r.is_admin() {
                                "Fixed for Admin"
                            } else if delegable {
                                "Super Admin and Admin only"
                            } else if p == Perm::Admins {
                                "Super Admin only"
                            } else {
                                "A Super Admin changes the roles"
                            }))
                        })
                        .child(check_box(t, on))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if fixed {
                                return;
                            }
                            let mut roles = roles.clone();
                            let set = &mut roles[ri].perms;
                            if !set.remove(&p) {
                                set.insert(p);
                            }
                            this.org_set_roles(roles, cx);
                        }))
                        .test_support(),
                ),
            );
        }
        matrix = matrix.child(line);
    }
    page.child(
        section(t)
            .child(titled(
                t,
                tr("Roles"),
                Some(tr("What each role can do in the spaces it can access.")),
            ))
            .child(matrix)
            .when(i_admins, |d| {
                d.child(
                    div().child(
                        button("new-role", t, BtnKind::Ghost, true)
                            .child(icon("plus", 14., t.text_muted))
                            .child(tr("New role"))
                            .on_click(
                                cx.listener(|this, _, window, cx| this.open_new_role(window, cx)),
                            )
                            .test_support(),
                    ),
                )
            }),
    )
}

/// Actions on a member (row "⋯" and right-click, mockup 33d); the irreversible ones ask first.
fn ask(
    a: crate::app::ConfirmAction,
    cx: &mut Context<CleisApp>,
) -> impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut gpui_kit::App) + 'static {
    cx.listener(
        move |this: &mut CleisApp, _: &gpui_kit::ClickEvent, _, cx| {
            if let Some(o) = org_mut(this) {
                o.ui.menu = None;
            }
            if let Some(s) = this.session() {
                s.ctx = None;
            }
            this.ask_action(a, cx);
        },
    )
}

pub fn member_menu(
    o: &OrgSession,
    st: &State,
    m: &MemberView,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> Div {
    use crate::app::ConfirmAction as A;
    let id = m.id;
    let me = m.id == st.me.id;
    let admin = is_admin_role(&m.role);
    let i_admins = o.can(Perm::Admins);
    let i_super = st.me.role == SUPER_ADMIN;
    let mut menu = menu_box(t).w(px(240.));
    let reach = !me && (!admin || i_admins);
    if reach && o.can(Perm::Rules) {
        menu = menu.child(
            menu_item(
                format!("resetstep-{id}"),
                t,
                Some("key"),
                tr("Reset two-step verification"),
                false,
            )
            .tooltip(tip(tr("Set up again at the next sign-in")))
            .on_click(ask(A::ResetStep(id), cx))
            .test_support(),
        );
    }
    if reach && o.can(Perm::Manage) {
        menu = menu.child(
            menu_item(
                format!("signout-{id}"),
                t,
                Some("logout"),
                tr("Sign out everywhere"),
                false,
            )
            .on_click(ask(A::SignOutMember(id), cx))
            .test_support(),
        );
    }
    if i_super && m.role != SUPER_ADMIN {
        menu = menu
            .child(div().h(px(1.)).my(px(4.)).bg(t.line))
            .child(
                menu_item(
                    format!("mksuper-{id}"),
                    t,
                    Some("shield"),
                    tr("Make Super Admin"),
                    false,
                )
                .on_click(ask(A::MakeSuper(id), cx))
                .test_support(),
            )
            .child(
                menu_item(
                    format!("transfer-{id}"),
                    t,
                    Some("refresh"),
                    tr("Transfer Super Admin…"),
                    false,
                )
                .tooltip(tip(tr("You become Admin")))
                .on_click(ask(A::TransferSuper(id), cx))
                .test_support(),
            );
    }
    if reach {
        menu = menu.child(div().h(px(1.)).my(px(4.)).bg(t.line)).child(
            menu_item(
                format!("revoke-{id}"),
                t,
                Some("trash"),
                tr("Remove…"),
                true,
            )
            .on_click(ask(A::RemoveMember(id), cx))
            .test_support(),
        );
    }
    menu
}

fn perm_tip(p: Perm) -> Option<&'static str> {
    Some(match p {
        Perm::Audit => "Read, verify and export the activity log",
        Perm::Backups => {
            "Download the server file: it also holds the server key and the members' authenticator keys"
        }
        Perm::Keys => "Recovery key, rotate the server key",
        Perm::Rules => "Verification, methods, remembered devices, what the log records and keeps",
        _ => return None,
    })
}

/// Element ids must live long enough: the few dynamic ids of selects are kept for the session.
fn leak(s: String) -> &'static str {
    use std::sync::Mutex;
    static IDS: Mutex<Vec<&'static str>> = Mutex::new(vec![]);
    let mut ids = IDS.lock().expect("ids");
    if let Some(found) = ids.iter().find(|x| **x == s) {
        return found;
    }
    let l: &'static str = Box::leak(s.into_boxed_str());
    ids.push(l);
    l
}

// ---------- Security policy (08, 18) ----------

const REMEMBER: [(u32, &str); 4] = [
    (0, "Never"),
    (7, "7 days"),
    (30, "30 days"),
    (90, "90 days"),
];
const LOCK_AFTER: [(Option<u32>, &str); 6] = [
    (None, "Never"),
    (Some(1), "1 min"),
    (Some(5), "5 min"),
    (Some(15), "15 min"),
    (Some(30), "30 min"),
    (Some(60), "1 hour"),
];
const OFFLINE: [(u32, &str); 4] = [
    (1, "1 day"),
    (7, "7 days"),
    (14, "14 days"),
    (30, "30 days"),
];
const MIN_LEN: [u32; 6] = [8, 10, 12, 14, 16, 20];
/// Mockup 35k (D-76).
const UNLOCK_RULES: [(cleis_core::org::UnlockRule, &str); 3] = [
    (
        cleis_core::org::UnlockRule::PasswordOnly,
        "Master password only",
    ),
    (
        cleis_core::org::UnlockRule::PasswordOrHello,
        "Master password or Windows Hello",
    ),
    (cleis_core::org::UnlockRule::Any, "Anything, also nothing"),
];
/// The members' master password (0: Cleis' own minimum, 10).
const MASTER_MIN: [(u32, &str); 5] = [
    (0, "10 characters"),
    (12, "12 characters"),
    (14, "14 characters"),
    (16, "16 characters"),
    (20, "20 characters"),
];
const MAX_AGE: [(u32, &str); 4] = [
    (182, "6 months"),
    (365, "1 year"),
    (730, "2 years"),
    (0, "Never"),
];
const KEEP: [(u32, &str); 5] = [
    (30, "30 days"),
    (90, "90 days"),
    (365, "1 year"),
    (730, "2 years"),
    (1825, "5 years"),
];

pub fn policy(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let Some(o) = s.org.as_ref() else {
        return div();
    };
    let Some(st) = o.state.as_ref() else {
        return head(t, tr("Security policy"));
    };
    let p = &st.policy;
    let rules = o.can(Perm::Rules);
    let menu = o.ui.menu;
    let sel = |id: &'static str,
               w: f32,
               options: Vec<(String, bool)>,
               pick: Pick,
               cx: &mut Context<CleisApp>| {
        select(
            id,
            t,
            w,
            menu == Some(OrgMenu::Policy(id)),
            OrgMenu::Policy(id),
            options,
            pick,
            cx,
        )
    };
    let method = |m: Method, cx: &mut Context<CleisApp>| {
        let on = p.methods.contains(&m);
        // Carried out by the server (D-64): `[smtp]`, `[sms]`, `[[oidc]]` in its configuration.
        let ok = m == Method::AuthenticatorApp || st.methods.contains(&m);
        let ico = match m {
            Method::AuthenticatorApp | Method::Sms => "phone",
            Method::Email => "mail",
            _ => "globe",
        };
        div()
            .id(SharedString::from(format!("method-{m:?}")))
            .flex()
            .items_center()
            .gap(px(10.))
            .h(px(32.))
            .when(ok, |d| d.cursor_pointer())
            .when(!ok, |d| {
                d.opacity(0.45).tooltip(tip(match m {
                    Method::Email => "Set [smtp] in the server's cleis-server.toml",
                    Method::Sms => "Set [sms] (an SMS gateway) in the server's cleis-server.toml",
                    Method::Google => {
                        "Add a google [[oidc]] provider in the server's cleis-server.toml"
                    }
                    _ => "Add a microsoft [[oidc]] provider in the server's cleis-server.toml",
                }))
            })
            .child(check_box(t, on && ok))
            .child(icon(ico, 16., t.text_muted))
            .child(tr(m.label()))
            .on_click(cx.listener(move |this, _, _, cx| {
                if ok && m != Method::AuthenticatorApp {
                    this.org_set_policy(
                        |p| {
                            if !p.methods.remove(&m) {
                                p.methods.insert(m);
                            }
                        },
                        cx,
                    );
                }
            }))
            .test_support()
    };
    let opt = |label: &str,
               on: bool,
               id: &'static str,
               flip: fn(&mut Policy),
               cx: &mut Context<CleisApp>| {
        div()
            .id(id)
            .flex()
            .items_center()
            .gap(px(10.))
            .h(px(32.))
            .cursor_pointer()
            .child(check_box(t, on))
            .child(crate::i18n::tr_owned(label))
            .on_click(cx.listener(move |this, _, _, cx| this.org_set_policy(flip, cx)))
            .test_support()
    };

    let signing = section(t)
        .child(titled(
            t,
            tr("Signing in to the server"),
            Some(tr("Applies to every member.")),
        ))
        .child(
            row(t, true)
                .child(div().flex_1().child(tr("Two-step verification")))
                .child(seg(
                    "code-req",
                    t,
                    &["Optional", "Required"],
                    usize::from(p.code_required),
                    Rc::new(|app, i, cx| app.org_set_policy(|p| p.code_required = i == 1, cx)),
                    cx,
                )),
        )
        .child(
            row(t, false)
                .child(div().flex_1().child(tr("Remember device")))
                .child(sel(
                    "remember",
                    160.,
                    REMEMBER
                        .iter()
                        .map(|(d, l)| ((*l).into(), *d == p.remember_days))
                        .collect(),
                    Rc::new(|app, i, cx| {
                        app.org_set_policy(|p| p.remember_days = REMEMBER[i].0, cx)
                    }),
                    cx,
                )),
        );
    let methods = section(t)
        .child(titled(t, tr("Allowed methods"), None))
        .child(
            div()
                .flex()
                .gap(px(24.))
                .child(
                    div()
                        .flex_1()
                        .child(method(Method::AuthenticatorApp, cx))
                        .child(method(Method::Email, cx))
                        .child(method(Method::Sms, cx)),
                )
                .child(
                    div()
                        .flex_1()
                        .child(method(Method::Google, cx))
                        .child(method(Method::Microsoft, cx)),
                ),
        );
    let locking = section(t)
        .child(titled(t, tr("Locking"), None))
        .child(
            row(t, true).child(div().flex_1().child(tr("After inactivity"))).child(sel(
                "pol-lock",
                160.,
                LOCK_AFTER.iter().map(|(m, l)| ((*l).into(), *m == p.lock_after_mins)).collect(),
                Rc::new(|app, i, cx| app.org_set_policy(|p| p.lock_after_mins = LOCK_AFTER[i].0, cx)),
                cx,
            )),
        )
        .child(
            row(t, false)
                .child(
                    div()
                        .id("offline-l")
                        .flex_1()
                        .child(tr("Offline access"))
                        .tooltip(tip(tr("How long the organization's spaces stay readable without reaching the server"))),
                )
                .child(sel(
                    "offline",
                    160.,
                    OFFLINE.iter().map(|(d, l)| ((*l).into(), *d == p.offline_days)).collect(),
                    Rc::new(|app, i, cx| app.org_set_policy(|p| p.offline_days = OFFLINE[i].0, cx)),
                    cx,
                )),
        );
    let opening = section(t)
        .child(titled(
            t,
            tr("Opening Cleis"),
            Some(tr("Applies to every member whose vault holds this organization")),
        ))
        .child(
            row(t, true).child(div().flex_1().child(tr("Unlock with"))).child(sel(
                "pol-unlock",
                280.,
                UNLOCK_RULES
                    .iter()
                    .map(|(r, l)| ((*l).into(), *r == p.unlock))
                    .collect(),
                Rc::new(|app, i, cx| app.org_set_policy(|p| p.unlock = UNLOCK_RULES[i].0, cx)),
                cx,
            )),
        )
        .child(
            row(t, false)
                .child(
                    div()
                        .id("master-min-l")
                        .flex_1()
                        .child(tr("Master password: at least"))
                        .tooltip(tip(tr("Asked when a member sets or changes it, and at the next unlock with a shorter one"))),
                )
                .child(sel(
                    "pol-master-min",
                    160.,
                    MASTER_MIN
                        .iter()
                        .map(|(n, l)| ((*l).into(), *n == p.master_min_length))
                        .collect(),
                    Rc::new(|app, i, cx| {
                        app.org_set_policy(|p| p.master_min_length = MASTER_MIN[i].0, cx)
                    }),
                    cx,
                )),
        );
    let passwords = section(t)
        .child(titled(
            t,
            tr("Passwords"),
            Some(tr("Flagged in the health report.")),
        ))
        .child(
            row(t, true)
                .child(div().flex_1().child(tr("Flag shorter than")))
                .child(sel(
                    "min-len",
                    160.,
                    MIN_LEN
                        .iter()
                        .map(|n| (crate::trf!("{} characters", n), *n == p.min_length))
                        .collect(),
                    Rc::new(|app, i, cx| app.org_set_policy(|p| p.min_length = MIN_LEN[i], cx)),
                    cx,
                )),
        )
        .child(
            row(t, false)
                .child(div().flex_1().child(tr("Flag older than")))
                .child(sel(
                    "max-age",
                    160.,
                    MAX_AGE
                        .iter()
                        .map(|(d, l)| ((*l).into(), *d == p.max_age_days))
                        .collect(),
                    Rc::new(|app, i, cx| app.org_set_policy(|p| p.max_age_days = MAX_AGE[i].0, cx)),
                    cx,
                )),
        );
    let integrity = st.integrity.as_ref();
    let log = section(t)
        .child(titled(
            t,
            tr("Activity log"),
            Some(tr("Written by the server, each event signed and linked to the previous one: an edit or a gap is detected.")),
        ))
        .child(
            row(t, true).child(div().flex_1().child(tr("Record activity"))).child(
                switch("record", t, p.record_activity)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.org_set_policy(|p| p.record_activity = !p.record_activity, cx)
                    }))
                    .test_support(),
            ),
        )
        .child(
            row(t, false).child(div().flex_1().child(tr("Detail"))).child(seg(
                "detail",
                t,
                &["Admin actions", "Changes", "Everything"],
                match p.detail {
                    LogDetail::AdminActions => 0,
                    LogDetail::Changes => 1,
                    LogDetail::Everything => 2,
                },
                Rc::new(|app, i, cx| {
                    app.org_set_policy(
                        |p| {
                            p.detail = [LogDetail::AdminActions, LogDetail::Changes, LogDetail::Everything][i]
                        },
                        cx,
                    )
                }),
                cx,
            )),
        )
        .child(
            row(t, false).child(div().flex_1().child(tr("Keep"))).child(sel(
                "keep",
                160.,
                KEEP.iter().map(|(d, l)| ((*l).into(), *d == p.keep_days)).collect(),
                Rc::new(|app, i, cx| app.org_set_policy(|p| p.keep_days = KEEP[i].0, cx)),
                cx,
            )),
        )
        .child(
            row(t, false)
                .child(div().flex_1().child(tr("Integrity")))
                .child(match integrity {
                    Some(i) if i.intact => chip(
                        t,
                        Tone::Ok,
                        crate::trf!("Verified {}", crate::format::when(i.at, cleis_core::now())),
                        false,
                    ),
                    Some(_) => chip(t, Tone::Danger, tr("Altered"), true),
                    None => chip(t, Tone::Neutral, tr("Not verified yet"), false),
                })
                .child(
                    button("verify-log", t, BtnKind::Default, false)
                        .child(tr("Verify now"))
                        .on_click(cx.listener(|this, _, _, cx| this.org_verify_log(cx)))
                        .test_support(),
                ),
        );
    let autofill = section(t)
        .child(titled(
            t,
            tr("Autofill"),
            Some(tr("Ways members may fill passwords outside Cleis.")),
        ))
        .child(
            div()
                .flex()
                .gap(px(24.))
                .child(
                    div()
                        .flex_1()
                        .child(opt(
                            "Fill fields in apps",
                            p.fill_fields,
                            "pol-fill",
                            |p| p.fill_fields = !p.fill_fields,
                            cx,
                        ))
                        .child(opt(
                            "Type keys",
                            p.type_keys,
                            "pol-type",
                            |p| p.type_keys = !p.type_keys,
                            cx,
                        )),
                )
                .child(div().flex_1().child(opt(
                    "Read browser addresses",
                    p.read_addresses,
                    "pol-addr",
                    |p| p.read_addresses = !p.read_addresses,
                    cx,
                ))),
        );
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(head(t, tr("Security policy")))
        .child(rules_lock("pol-lock-signing", rules, signing))
        .child(rules_lock("pol-lock-methods", rules, methods))
        .child(rules_lock("pol-lock-opening", rules, opening))
        .child(locking)
        .child(passwords)
        .child(rules_lock("pol-lock-log", rules, log))
        .child(autofill)
}

/// D-67: sign-in and log rules seen by an Admin without the Rules right: shown, dimmed, not
/// clickable.
fn rules_lock(id: &'static str, rules: bool, d: Div) -> gpui_kit::AnyElement {
    if rules {
        return d.into_any_element();
    }
    div()
        .relative()
        .child(d.opacity(0.55))
        .child(
            div()
                .id(id)
                .absolute()
                .inset_0()
                .occlude()
                .tooltip(tip(tr("Set by a Super Admin"))),
        )
        .into_any_element()
}

// ---------- Encryption (09, 10) ----------

pub fn encryption(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let Some(o) = s.org.as_ref() else {
        return div();
    };
    let Some(st) = o.state.as_ref() else {
        return head(t, tr("Encryption"));
    };
    let cols = |d: Div| d.flex().items_center().gap(px(12.)).px(px(4.));
    let mut table = div().flex().flex_col().child(
        cols(div())
            .h(px(32.))
            .border_b_1()
            .border_color(t.line)
            .text_size(TEXT_XS)
            .text_color(t.text_muted)
            .font_weight(W_MEDIUM)
            .child(div().flex_1().child(tr("Space")))
            .child(div().w(px(80.)).flex().justify_end().child(tr("Items")))
            .child(div().w(px(80.)).flex().justify_end().child(tr("Members")))
            .child(div().w(px(320.)).child(tr("Mode"))),
    );
    table = table.child(
        cols(div())
            .h(px(40.))
            .border_b_1()
            .border_color(t.line)
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(icon("user", 16., t.text_muted))
                    .child(tr("Personal vaults")),
            )
            .child(
                div()
                    .w(px(80.))
                    .flex()
                    .justify_end()
                    .text_color(t.text_muted)
                    .child("—"),
            )
            .child(
                div()
                    .w(px(80.))
                    .flex()
                    .justify_end()
                    .text_color(t.text_muted)
                    .child(st.members.len().to_string()),
            )
            .child(
                div()
                    .id("personal-mode")
                    .w(px(320.))
                    .flex()
                    .tooltip(tip(
                        "Always end-to-end: personal vaults never reach the server",
                    ))
                    .child(chip(t, Tone::Ok, tr("End-to-end"), false)),
            ),
    );
    for sv in &st.spaces {
        let id = sv.id;
        let pending = o.ui.changes.iter().find(|c| c.0 == id).map(|c| c.1);
        let mode = pending.unwrap_or(sv.mode);
        let holds = s.vault.space_key(id).is_some() && !sv.reserved;
        table = table.child(
            cols(div())
                .h(px(40.))
                .border_b_1()
                .border_color(t.line)
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(icon("folder", 16., t.text_muted))
                        .child(o.space_name(id))
                        .when(sv.needs_rekey, |d| {
                            d.child(
                                div()
                                    .id(SharedString::from(format!("advise-{id}")))
                                    .tooltip(tip(tr("Someone who held its key lost access")))
                                    .child(chip(t, Tone::Warn, tr("Re-encrypt advised"), false)),
                            )
                        }),
                )
                .child(
                    div()
                        .w(px(80.))
                        .flex()
                        .justify_end()
                        .text_color(t.text_muted)
                        .child(sv.items.to_string()),
                )
                .child(
                    div()
                        .w(px(80.))
                        .flex()
                        .justify_end()
                        .text_color(t.text_muted)
                        .child(sv.members.len().to_string()),
                )
                .child(
                    div()
                        .w(px(320.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(if holds {
                            div().child(seg(
                                &format!("mode-{id}"),
                                t,
                                &["Standard", "End-to-end"],
                                usize::from(mode == SpaceMode::EndToEnd),
                                Rc::new(move |app, i, cx| {
                                    if let Some(o) = org_mut(app) {
                                        let want = if i == 0 {
                                            SpaceMode::Standard
                                        } else {
                                            SpaceMode::EndToEnd
                                        };
                                        let cur = o
                                            .state
                                            .as_ref()
                                            .and_then(|s| s.spaces.iter().find(|x| x.id == id))
                                            .map(|x| x.mode);
                                        o.ui.changes.retain(|c| c.0 != id);
                                        if cur != Some(want) {
                                            o.ui.changes.push((id, want));
                                        }
                                    }
                                    cx.notify();
                                }),
                                cx,
                            ))
                        } else {
                            div().child(
                                div()
                                    .id(SharedString::from(format!("nokey-{id}")))
                                    .tooltip(tip(tr("You are not in this space")))
                                    .child(chip(
                                        t,
                                        Tone::Neutral,
                                        match sv.mode {
                                            SpaceMode::Standard => "Standard",
                                            SpaceMode::EndToEnd => "End-to-end",
                                        },
                                        false,
                                    )),
                            )
                        })
                        .when(pending.is_some(), |d| {
                            d.child(chip(t, Tone::Set, tr("Changed"), false))
                        }),
                ),
        );
    }
    let to_rekey: Option<(SpaceId, SpaceMode)> = o.ui.changes.first().copied().or_else(|| {
        st.spaces
            .iter()
            .find(|sv| sv.needs_rekey && s.vault.space_key(sv.id).is_some())
            .map(|sv| (sv.id, sv.mode))
    });
    let modes = section(t)
        .child(titled(
            t,
            tr("Mode per space"),
            Some(tr("Both modes encrypt on the device. Standard: the organization key keeps a copy of the space key, so an admin can restore access. End-to-end: only the members hold the key; the recovery key is essential.")),
        ))
        .child(table)
        .child(
            div()
                .flex()
                .justify_end()
                .gap(px(8.))
                .child(
                    button("enc-cancel", t, BtnKind::Ghost, false)
                        .child(tr("Cancel"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(o) = org_mut(this) {
                                o.ui.changes.clear();
                            }
                            cx.notify();
                        }))
                        .test_support(),
                )
                .child(
                    button("reencrypt", t, BtnKind::Primary, false)
                        .when(to_rekey.is_none(), |b| b.opacity(0.5))
                        .child(icon("refresh", 16., t.primary_ink))
                        .child(tr("Re-encrypt"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let (Some((space, to)), Some(o)) = (to_rekey, org_mut(this)) {
                                o.ui.rekey = Some(RekeyForm {
                                    space,
                                    to,
                                    backup_first: true,
                                });
                            }
                            cx.notify();
                        }))
                        .test_support(),
                ),
        );
    let recovery_on = st.org.recovery_public.is_some();
    let recovery = section(t)
        .child(titled(
            t,
            tr("Recovery key"),
            Some(tr("Restores a space when every member holding it lost their master password. Kept outside Cleis.")),
        ))
        .child(
            row(t, true).child(div().flex_1().child(tr("Admin recovery"))).child(
                switch("recovery", t, recovery_on)
                    .on_click(cx.listener(move |this, _, _, cx| this.org_recovery(!recovery_on, cx)))
                    .test_support(),
            ),
        )
        .when(recovery_on, |d| {
            d.child(
                row(t, false)
                    .child(div().flex_1().child(tr("Recovery key")))
                    .child(chip(
                        t,
                        Tone::Ok,
                        st.org
                            .recovery_saved
                            .map_or(tr("Saved").into(), |ts| crate::trf!("Saved {}", crate::format::date(ts))),
                        true,
                    ))
                    .child(
                        button("recovery-new", t, BtnKind::Default, false)
                            .child(icon("download", 14., t.text))
                            .child(tr("New key"))
                            .tooltip(tip(tr("Makes a new recovery key; the previous one stops working")))
                            .on_click(cx.listener(|this, _, _, cx| this.org_recovery(true, cx)))
                            .test_support(),
                    )
                    .child(
                        button("recovery-use", t, BtnKind::Default, false)
                            .child(tr("Use"))
                            .tooltip(tip(tr("Gives the waiting members their spaces back")))
                            .on_click(cx.listener(|this, _, _, cx| this.org_use_recovery(cx)))
                            .test_support(),
                    ),
            )
        });
    let fp = cleis_crypto::fingerprint_of_hash(&st.org.cert);
    let fp_copy = fp.clone();
    let keys = section(t)
        .child(titled(t, tr("Server keys"), None))
        .child(
            row(t, true)
                .child(div().flex_1().child(tr("Fingerprint")))
                .child(div().font_family(FONT_MONO).text_color(t.text_muted).child(fp))
                .child(
                    icon_button("copy-fp", t, "copy", false)
                        .tooltip(tip(tr("Copy")))
                        .on_click(cx.listener(move |this, _, _, cx| this.copy(fp_copy.clone(), "Fingerprint", cx))),
                ),
        )
        .child(
            row(t, false)
                .child(div().flex_1().child(tr("Rotate server key")))
                .when(st.org.next_cert.is_some(), |d| {
                    d.child(
                        div()
                            .id("rotation")
                            .tooltip(tip(tr("Used after the server's next restart, 7 days after the announcement")))
                            .child(chip(t, Tone::Set, tr("Announced"), false)),
                    )
                })
                .child(
                    button("rotate", t, BtnKind::Default, false)
                        .child(icon("refresh", 14., t.text))
                        .child(tr("Rotate"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.ask_action(crate::app::ConfirmAction::RotateKey, cx)
                        }))
                        .test_support(),
                ),
        );
    // D-67: recovery and server keys need the Keys right (Super Admin, or delegated).
    let can_keys = o.can(Perm::Keys);
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(head(t, tr("Encryption")))
        .child(modes)
        .when(can_keys, |d| d.child(recovery).child(keys))
        .when(!can_keys, |d| {
            d.child(
                section(t)
                    .id("keys-locked")
                    .tooltip(tip(
                        "A Super Admin keeps the recovery key and the server key",
                    ))
                    .child(titled(t, tr("Recovery and server keys"), None))
                    .child(
                        row(t, true)
                            .child(
                                div()
                                    .flex_1()
                                    .text_color(t.text_muted)
                                    .child(tr("Super Admin")),
                            )
                            .child(icon("lock", 14., t.text_muted)),
                    ),
            )
        })
}

// ---------- Server / organization ----------

/// Settings › Server: where the organization is reached, how this vault belongs to it.
pub fn server(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let Some(o) = s.org.as_ref() else {
        return join_page(s, t, cx);
    };
    let status: gpui_kit::AnyElement = if o.busy {
        chip(t, Tone::Neutral, tr("Syncing…"), false).into_any_element()
    } else if let Some(e) = &o.error {
        div()
            .id("srv-err")
            .tooltip(tip(crate::i18n::tr_dynamic(e)))
            .child(chip(t, Tone::Warn, tr("Not reached"), true))
            .into_any_element()
    } else if let Some(ts) = o.local.last_ok {
        chip(
            t,
            Tone::Ok,
            crate::trf!("Synced {}", crate::format::when(ts, cleis_core::now())),
            true,
        )
        .into_any_element()
    } else {
        chip(t, Tone::Neutral, tr("Not reached yet"), false).into_any_element()
    };
    let me = o.me();
    let hosted = o.local.host_dir.is_some();
    let (running, addrs) = o.ui.host.clone().unwrap_or((false, Vec::new()));
    let addr_text = addrs.join(" · ");
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(head(t, tr("Server")))
        .child(
            section(t)
                .child(titled(t, &o.m.name, None))
                .when(hosted, |d| {
                    let copy = addr_text.clone();
                    d.child(
                        row(t, true)
                            .child(div().flex_1().child(tr("Hosted")))
                            .child(tr("This computer"))
                            .child(if o.ui.host.is_none() {
                                chip(t, Tone::Neutral, tr("Checking…"), false)
                            } else if running {
                                chip(t, Tone::Ok, tr("Running"), true)
                            } else {
                                chip(t, Tone::Warn, tr("Stopped"), true)
                            }),
                    )
                    .child(
                        row(t, false)
                            .child(
                                div()
                                    .id("host-addrs")
                                    .flex_1()
                                    .tooltip(tip(tr("Give one of these with each invitation code")))
                                    .child(tr("Addresses for members")),
                            )
                            .child(div().font_family(FONT_MONO).text_color(t.text_muted).child(
                                if addr_text.is_empty() {
                                    tr("No network").to_owned()
                                } else {
                                    addr_text.clone()
                                },
                            ))
                            .child(
                                icon_button("copy-host-addrs", t, "copy", false)
                                    .tooltip(tip(tr("Copy")))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.copy(copy.clone(), "Addresses", cx)
                                    })),
                            ),
                    )
                    .child(
                        row(t, false)
                            .child(
                                div()
                                    .id("host-service")
                                    .flex_1()
                                    .tooltip(tip(tr("Otherwise it runs while Cleis runs")))
                                    .child(tr("Start with the system")),
                            )
                            .child(
                                button("host-install", t, BtnKind::Default, false)
                                    .child(tr("Install"))
                                    .tooltip(tip(tr("Asks for administrator rights")))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if this.demo {
                                            this.toast(tr("Not in the demo"), cx);
                                        } else {
                                            this.org_install_host(cx);
                                        }
                                    }))
                                    .test_support(),
                            ),
                    )
                })
                .when(!hosted, |d| {
                    d.child(
                        row(t, true)
                            .child(div().flex_1().child(tr("Address")))
                            .child(match &o.ui.address {
                                Some((input, _)) => div()
                                    .w(px(260.))
                                    .child(Input::new(input).id("server-address"))
                                    .into_any_element(),
                                None => div()
                                    .id("server-address-value")
                                    .font_family(FONT_MONO)
                                    .text_color(t.text_muted)
                                    .cursor_pointer()
                                    .tooltip(tip(
                                        "Change it when the server moved (same server, same key)",
                                    ))
                                    .child(o.m.address.clone())
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.edit_server_address(window, cx)
                                    }))
                                    .test_support()
                                    .into_any_element(),
                            })
                            .child(status),
                    )
                })
                .child(
                    row(t, false)
                        .child(div().flex_1().child(tr("Certificate")))
                        .child(div().font_family(FONT_MONO).text_color(t.text_muted).child(
                            o.m.pins.first().map_or(String::new(), |p| {
                                data_encoding::HEXLOWER
                                    .decode(p.as_bytes())
                                    .map(|b| cleis_crypto::fingerprint_of_hash(&b))
                                    .unwrap_or_default()
                            }),
                        )),
                )
                .when_some(me, |d, m| {
                    d.child(
                        row(t, false)
                            .child(div().flex_1().child(tr("Signed in as")))
                            .child(avatar(t, &initials(&m.name), 22.))
                            .child(format!(
                                "{} · {}",
                                m.name,
                                o.role()
                                    .map(|r| crate::i18n::tr_owned(&r.name))
                                    .unwrap_or_default()
                            )),
                    )
                })
                .child(
                    row(t, false)
                        .child(div().flex_1())
                        .child(
                            button("org-sync", t, BtnKind::Default, false)
                                .child(icon("sync", 14., t.text))
                                .child(tr("Sync now"))
                                .on_click(cx.listener(|this, _, _, cx| this.org_sync(None, cx)))
                                .test_support(),
                        )
                        .child(
                            button("org-leave", t, BtnKind::DangerGhost, false)
                                .child(tr("Leave"))
                                .tooltip(tip(tr("Forgets the organization's spaces on this vault")))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.ask_action(crate::app::ConfirmAction::LeaveOrg, cx)
                                }))
                                .test_support(),
                        ),
                ),
        )
}

/// Settings › Organization without one: the server address and invitation code (as on the first
/// launch, mockup 03).
pub fn join_page(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(head(t, tr("Organization")))
        .child(create_section(&s.join, t, cx))
        .child(
            section(t)
                .child(titled(
                    t,
                    tr("Join an organization"),
                    Some(tr("Shared spaces, on top of your personal vault")),
                ))
                .child(join_fields(&s.join, t, cx))
                .child(
                    div().flex().justify_end().child(
                        button("join", t, BtnKind::Default, false)
                            .when(s.join.busy, |b| b.opacity(0.6))
                            .child(if s.join.busy {
                                tr("Joining…")
                            } else {
                                tr("Join")
                            })
                            .on_click(cx.listener(|this, _, _, cx| this.join_from_settings(cx)))
                            .test_support(),
                    ),
                ),
        )
}

/// Create an organization (mockup 25b): the server runs on this computer, or the commands to run
/// it on another machine.
fn create_section(j: &JoinForm, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let Some(name) = &j.org_name else {
        return div();
    };
    let pick: Pick = Rc::new(|this, i, cx| {
        if let Some(j) = this.join_form_mut() {
            j.elsewhere = i == 1;
        }
        cx.notify();
    });
    let org = name.read(cx).value().trim().to_owned();
    let org = if org.is_empty() {
        tr("Example Studio").to_owned()
    } else {
        org
    };
    let cmds = [
        crate::trf!("cleis-server init --org \"{}\"", org),
        "cleis-server install".to_owned(),
    ];
    let mut d = section(t)
        .child(titled(
            t,
            tr("Create an organization"),
            Some(tr(
                "Shared spaces for a team: Cleis runs its server for you.",
            )),
        ))
        .child(field(t, tr("Name"), Input::new(name).id("create-name")))
        .when_some(j.creator.as_ref().filter(|_| !j.elsewhere), |d, c| {
            d.child(field(t, tr("Your name"), Input::new(c).id("create-me")))
        })
        .child(
            div()
                .id("create-where")
                .tooltip(tip(if j.elsewhere {
                    "Linux, Windows, macOS or Docker"
                } else {
                    "Runs while Cleis runs; can start with the system"
                }))
                .child(field(
                    t,
                    "Server",
                    segment(
                        "create-where",
                        t,
                        &[tr("This computer"), tr("Another machine")],
                        usize::from(j.elsewhere),
                        pick,
                        cx,
                    ),
                )),
        );
    if j.elsewhere {
        for (n, c) in cmds.into_iter().enumerate() {
            let copy = c.clone();
            d = d.child(
                value_box(t)
                    .child(div().flex_1().font_family(FONT_MONO).child(c))
                    .child(
                        icon_button(SharedString::from(format!("copy-cmd-{n}")), t, "copy", true)
                            .tooltip(tip(tr("Copy")))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.copy(copy.clone(), "Command", cx)
                            })),
                    ),
            );
        }
        d.child(hint(
            "create-then",
            t,
            "Then join below with the code it prints and the machine's address.",
        ))
    } else {
        d.child(
            div().flex().justify_end().child(
                button("create-org", t, BtnKind::Primary, false)
                    .when(j.busy, |b| b.opacity(0.6))
                    .child(if j.busy {
                        tr("Creating…")
                    } else {
                        tr("Create")
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        let Some(name) = this.join_form_mut().and_then(|j| j.org_name.clone())
                        else {
                            return;
                        };
                        let n = name.read(cx).value().trim().to_owned();
                        let me = this
                            .join_form_mut()
                            .and_then(|j| j.creator.clone())
                            .map(|c| c.read(cx).value().trim().to_owned())
                            .unwrap_or_default();
                        let missing = if n.is_empty() {
                            Some(tr("Name the organization"))
                        } else if me.is_empty() {
                            Some(tr("Enter your name"))
                        } else {
                            None
                        };
                        if let Some(why) = missing {
                            if let Some(j) = this.join_form_mut() {
                                j.error = Some(why.into());
                            }
                            cx.notify();
                            return;
                        }
                        if this.demo {
                            this.toast(tr("Not in the demo"), cx);
                            return;
                        }
                        this.org_create_here(n, me, cx);
                    }))
                    .test_support(),
            ),
        )
    }
}

/// Address, invitation code, and name/e-mail when the invitation has none.
pub fn join_fields(j: &JoinForm, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let _ = cx;
    let (Some(address), Some(code)) = (&j.address, &j.code) else {
        return div();
    };
    let chip_el = if j.checking {
        Some(chip(t, Tone::Neutral, tr("Checking…"), false))
    } else {
        j.info
            .as_ref()
            .map(|i| chip(t, Tone::Ok, i.org.clone(), true))
    };
    let need_name = j.info.as_ref().is_some_and(|i| i.name.trim().is_empty());
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .id("addr-field")
                .tooltip(tip(
                    "Cleis server (vault.example.lan, 10.8.0.1:8443), reached through your VPN",
                ))
                .child(field(
                    t,
                    "Server address",
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(div().flex_1().child(Input::new(address).id("org-address")))
                        .when_some(chip_el, |d, c| d.child(c)),
                )),
        )
        .child(field(
            t,
            tr("Invitation code"),
            Input::new(code).id("org-code"),
        ))
        .when(need_name, |d| {
            d.when_some(j.name.as_ref(), |d, n| {
                d.child(field(t, tr("Your name"), Input::new(n).id("org-name")))
            })
            .when_some(j.email.as_ref(), |d, e| {
                d.child(field(t, tr("E-mail"), Input::new(e).id("org-email")))
            })
        })
        .when_some(
            j.error.clone().map(|e| crate::i18n::tr_dynamic(&e)),
            |d, e| d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e)),
        )
}

// ---------- Organization backups (13) ----------

pub fn backups(s: &Session, path: &std::path::Path, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let Some(o) = s.org.as_ref() else {
        return div();
    };
    let list = crate::org::org_backups(path, &o.m);
    let mut hist = div().flex().flex_col().child(
        div()
            .flex()
            .h(px(32.))
            .items_center()
            .gap(px(12.))
            .text_size(TEXT_XS)
            .text_color(t.text_muted)
            .font_weight(W_MEDIUM)
            .border_b_1()
            .border_color(t.line)
            .child(div().flex_1().child(tr("Date")))
            .child(div().w(px(140.)).child(tr("Destination")))
            .child(div().w(px(80.)).flex().justify_end().child(tr("Size")))
            .child(div().w(px(200.))),
    );
    if list.is_empty() {
        hist = hist.child(
            div()
                .py(px(10.))
                .text_color(t.text_faint)
                .child(tr("No backup yet")),
        );
    }
    for (i, (p, size, ts)) in list.into_iter().take(30).enumerate() {
        let pv = p.clone();
        let pr = p.clone();
        hist = hist.child(
            div()
                .flex()
                .min_h(px(36.))
                .items_center()
                .gap(px(12.))
                .border_b_1()
                .border_color(t.line)
                .child(
                    div()
                        .flex_1()
                        .child(crate::format::when(ts, cleis_core::now())),
                )
                .child(
                    div()
                        .w(px(140.))
                        .text_color(t.text_muted)
                        .child(tr("This computer")),
                )
                .child(
                    div()
                        .w(px(80.))
                        .flex()
                        .justify_end()
                        .text_color(t.text_muted)
                        .child(crate::format::size(size)),
                )
                .child(
                    div()
                        .w(px(200.))
                        .flex()
                        .justify_end()
                        .gap(px(6.))
                        .child(
                            button(
                                SharedString::from(format!("obk-verify-{i}")),
                                t,
                                BtnKind::Ghost,
                                true,
                            )
                            .child(tr("Verify"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.org_verify_backup(pv.clone(), cx)
                            }))
                            .test_support(),
                        )
                        .child(
                            button(
                                SharedString::from(format!("obk-restore-{i}")),
                                t,
                                BtnKind::Default,
                                true,
                            )
                            .child(tr("Restore"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(o) = org_mut(this) {
                                    o.ui.restore = Some(pr.clone());
                                }
                                cx.notify();
                            }))
                            .test_support(),
                        ),
                ),
        );
    }
    let saved = s.prefs.org_backup_key_saved;
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            section(t)
                .child(titled(t, tr("This computer"), Some(tr("The server's file, sealed with the backup key, in this computer's backups folder."))))
                .child(
                    row(t, true)
                        .child(div().flex_1().font_family(FONT_MONO).text_size(TEXT_SM).text_color(t.text_muted).child(
                            crate::org::org_backups_dir(path, &o.m).display().to_string(),
                        )),
                ),
        )
        .child({
            let copy: Option<(crate::remote::RemotePlace, crate::remote::Secret)> = s
                .vault
                .org_setting(crate::remote::ORG_BACKUP_SETTING)
                .ok()
                .flatten()
                .flatten();
            let sec = section(t).child(titled(
                t,
                tr("Copies elsewhere"),
                Some(tr("Each backup, sealed, also sent to a repository or a server.")),
            ));
            match copy {
                Some((place, _)) => sec.child(
                    row(t, true)
                        .child(icon(
                            match place {
                                crate::remote::RemotePlace::GitHub { .. } => "repo",
                                _ => "server",
                            },
                            16.,
                            t.text_muted,
                        ))
                        .child(div().flex_1().font_family(FONT_MONO).text_size(TEXT_SM).child(place.label()))
                        .child(
                            button("obk-place-off", t, BtnKind::Ghost, true)
                                .child(tr("Disconnect"))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(s) = this.session() {
                                        let _ = s.vault.set_org_setting(
                                            crate::remote::ORG_BACKUP_SETTING,
                                            &None::<(crate::remote::RemotePlace, crate::remote::Secret)>,
                                        );
                                    }
                                    cx.notify();
                                }))
                                .test_support(),
                        ),
                ),
                None => sec.child(
                    row(t, true)
                        .child(div().flex_1().text_color(t.text_muted).child(tr("Only on this computer")))
                        .child(
                            button("obk-github", t, BtnKind::Default, true)
                                .child(icon("repo", 14., t.text))
                                .child(tr("GitHub"))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_org_backup_place(crate::sync::Provider::GitHub, window, cx)
                                }))
                                .test_support(),
                        )
                        .child(
                            button("obk-sftp", t, BtnKind::Default, true)
                                .child(icon("server", 14., t.text))
                                .child(tr("SFTP"))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_org_backup_place(crate::sync::Provider::Sftp, window, cx)
                                }))
                                .test_support(),
                        ),
                ),
            }
        })
        .child(
            section(t)
                .child(titled(t, tr("Backup key"), Some(tr("Without it, no backup can be restored. Keep it outside Cleis."))))
                .child(
                    row(t, true)
                        .child(match saved {
                            Some(ts) => div()
                                .flex_1()
                                .child(chip(t, Tone::Ok, crate::trf!("Saved {}", crate::format::date(ts)), true)),
                            None => div()
                                .flex_1()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_color(t.warning_text)
                                .child(icon("warn", 14., t.warning_text))
                                .child(tr("Not downloaded yet")),
                        })
                        .child(
                            button("bk-key", t, BtnKind::Default, false)
                                .child(icon("download", 14., t.text))
                                .child(tr("Download"))
                                .on_click(cx.listener(|this, _, _, cx| this.org_download_backup_key(cx)))
                                .test_support(),
                        ),
                ),
        )
        .child(section(t).child(section_title(t, tr("History"))).child(hist))
}

// ---------- Activity (11) ----------

pub fn activity(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let Some(o) = s.org.as_ref() else {
        return div();
    };
    let Some(st) = o.state.as_ref() else {
        return div();
    };
    let ui = &o.ui;
    let menu = ui.menu;
    let sel =
        |id: &'static str, options: Vec<(String, bool)>, pick: Pick, cx: &mut Context<CleisApp>| {
            select(
                id,
                t,
                150.,
                menu == Some(OrgMenu::Log(id)),
                OrgMenu::Log(id),
                options,
                pick,
                cx,
            )
        };
    let audited: Vec<&OrgSession> = s.all_orgs().filter(|x| x.can(Perm::Audit)).collect();
    let org_keys: Vec<String> = audited.iter().map(|x| x.m.key()).collect();
    let org_opts: Vec<(String, bool)> = audited
        .iter()
        .map(|x| (x.m.name.clone(), x.m.key() == o.m.key()))
        .collect();
    let members: Vec<MemberId> = st.members.iter().map(|m| m.id).collect();
    let mut mopts = vec![(tr("All members").to_owned(), ui.log_member.is_none())];
    mopts.extend(
        st.members
            .iter()
            .map(|m| (m.name.clone(), ui.log_member == Some(m.id))),
    );
    const ACTIONS: [Action; 9] = [
        Action::SignedIn,
        Action::Viewed,
        Action::Copied,
        Action::Filled,
        Action::Created,
        Action::Edited,
        Action::Deleted,
        Action::Exported,
        Action::Revoked,
    ];
    let mut aopts = vec![(tr("All actions").to_owned(), ui.log_action.is_none())];
    aopts.extend(
        ACTIONS
            .iter()
            .map(|a| (tr(a.label()).to_owned(), ui.log_action == Some(*a))),
    );
    let spaces: Vec<SpaceId> = st.spaces.iter().map(|x| x.id).collect();
    let mut sopts = vec![(tr("All spaces").to_owned(), ui.log_space.is_none())];
    sopts.extend(
        spaces
            .iter()
            .map(|x| (o.space_name(*x), ui.log_space == Some(*x))),
    );
    const DAYS: [(Option<u32>, &str); 4] = [
        (Some(1), "Today"),
        (Some(7), "Last 7 days"),
        (Some(30), "Last 30 days"),
        (None, "Everything kept"),
    ];
    let dopts = DAYS
        .iter()
        .map(|(d, l)| ((*l).to_owned(), ui.log_days == *d))
        .collect();
    let toolbar = div()
        .flex()
        .items_center()
        .gap(px(8.))
        .px(px(16.))
        .h(px(48.))
        .border_b_1()
        .border_color(t.line)
        .bg(t.bg)
        .when_some(ui.log_search.as_ref(), |d, i| {
            d.child(div().w(px(240.)).child(Input::new(i).id("log-search")))
        })
        // Mockup 32d: with several organizations, the organization is the first filter.
        .when(org_keys.len() > 1, |d| {
            d.child(sel(
                "log-org",
                org_opts,
                Rc::new(move |app, i, cx| {
                    if let Some(k) = org_keys.get(i) {
                        app.activate_org(k);
                    }
                    app.org_query_log(cx);
                }),
                cx,
            ))
        })
        .child(sel(
            "log-member",
            mopts,
            Rc::new(move |app, i, cx| {
                if let Some(o) = org_mut(app) {
                    o.ui.log_member = i.checked_sub(1).and_then(|i| members.get(i).copied());
                }
                app.org_query_log(cx);
            }),
            cx,
        ))
        .child(sel(
            "log-action",
            aopts,
            Rc::new(|app, i, cx| {
                if let Some(o) = org_mut(app) {
                    o.ui.log_action = i.checked_sub(1).map(|i| ACTIONS[i]);
                }
                app.org_query_log(cx);
            }),
            cx,
        ))
        .child(sel(
            "log-space",
            sopts,
            Rc::new(move |app, i, cx| {
                if let Some(o) = org_mut(app) {
                    o.ui.log_space = i.checked_sub(1).and_then(|i| spaces.get(i).copied());
                }
                app.org_query_log(cx);
            }),
            cx,
        ))
        .child(sel(
            "log-days",
            dopts,
            Rc::new(|app, i, cx| {
                if let Some(o) = org_mut(app) {
                    o.ui.log_days = DAYS[i].0;
                }
                app.org_query_log(cx);
            }),
            cx,
        ))
        .child(div().flex_1())
        .child(
            button("log-verify", t, BtnKind::Ghost, false)
                .child(icon("chain", 16., t.text_muted))
                .child(tr("Verify"))
                .tooltip(tip(tr("Check every signature and link")))
                .on_click(cx.listener(|this, _, _, cx| this.org_verify_log(cx)))
                .test_support(),
        )
        .child(
            button("log-export", t, BtnKind::Default, false)
                .child(icon("download", 16., t.text))
                .child(tr("Export"))
                .on_click(cx.listener(|this, _, _, cx| this.org_export_log(cx)))
                .test_support(),
        );
    let q = ui
        .log_search
        .as_ref()
        .map(|i| cleis_core::text::fold(i.read(cx).value().trim()))
        .unwrap_or_default();
    let cols = |d: Div| d.flex().items_center().gap(px(12.)).px(px(16.));
    let mut table = div().flex().flex_col().child(
        cols(div())
            .h(px(32.))
            .border_b_1()
            .border_color(t.line)
            .text_size(TEXT_XS)
            .text_color(t.text_muted)
            .font_weight(W_MEDIUM)
            .child(div().w(px(170.)).child(tr("Date ↓")))
            .child(div().w(px(170.)).child(tr("Member")))
            .child(div().w(px(130.)).child(tr("Action")))
            .child(div().flex_1().child(tr("Item")))
            .child(div().w(px(140.)).child(tr("Space")))
            .child(div().w(px(220.)).child(tr("Device"))),
    );
    let events = ui
        .log
        .as_ref()
        .map(|p| p.events.as_slice())
        .unwrap_or_default();
    let mut shown = 0;
    for e in events {
        let detail = o.open_detail(e);
        let item = match &detail {
            Some(d) if d.field.is_empty() => d.item.clone(),
            Some(d) => format!("{} · {}", d.item, crate::i18n::tr_owned(&d.field)),
            None if !e.note.is_empty() => e.note.clone(),
            None => "—".into(),
        };
        let who = e.member.map(|m| o.member_name(m)).unwrap_or_default();
        let space = e.space.map_or("—".into(), |x| o.space_name(x));
        if !q.is_empty() {
            let hay = cleis_core::text::fold(&format!(
                "{who} {item} {space} {} {}",
                e.device,
                tr(e.action.label())
            ));
            if !hay.contains(&q) {
                continue;
            }
        }
        shown += 1;
        let tone = match e.action {
            Action::Filled => Tone::Auto,
            Action::Edited
            | Action::MemberChanged
            | Action::RolesChanged
            | Action::PolicyChanged => Tone::Set,
            Action::Created | Action::Joined | Action::SpaceCreated => Tone::Ok,
            Action::Deleted | Action::Erased | Action::Revoked => Tone::Danger,
            Action::Exported | Action::SpaceReencrypted | Action::KeyRotated => Tone::Warn,
            _ => Tone::Neutral,
        };
        table = table.child(
            cols(div())
                .h(px(34.))
                .border_b_1()
                .border_color(t.line)
                .child(
                    div()
                        .w(px(170.))
                        .text_color(t.text_muted)
                        .child(crate::format::when(e.at, cleis_core::now())),
                )
                .child(
                    div()
                        .w(px(170.))
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(who),
                )
                .child(
                    div()
                        .w(px(130.))
                        .flex()
                        .child(chip(t, tone, tr(e.action.label()), false)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(item),
                )
                .child(div().w(px(140.)).text_color(t.text_muted).child(space))
                .child(
                    div()
                        .w(px(220.))
                        .text_color(t.text_muted)
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(e.device.clone()),
                ),
        );
    }
    if events.is_empty() {
        table = table.child(
            div()
                .p(px(16.))
                .text_color(t.text_faint)
                .child(if ui.log_busy {
                    "Loading…"
                } else {
                    "No event"
                }),
        );
    }
    let keep = KEEP
        .iter()
        .find(|k| k.0 == st.policy.keep_days)
        .map_or(crate::trf!("{} days", st.policy.keep_days), |k| {
            crate::i18n::tr_owned(k.1)
        });
    let integrity = match &st.integrity {
        Some(i) if i.intact => (
            Tone::Ok,
            crate::trf!(
                "Signed and chained · verified {}",
                crate::format::when(i.at, cleis_core::now())
            ),
        ),
        Some(_) => (Tone::Danger, tr("Log altered").to_owned()),
        None => (
            Tone::Neutral,
            tr("Signed and chained · not verified yet").to_owned(),
        ),
    };
    let status = div()
        .h(STATUSBAR_H)
        .flex()
        .items_center()
        .gap(px(16.))
        .px(px(16.))
        .border_t_1()
        .border_color(t.line)
        .bg(t.surface)
        .text_size(TEXT_SM)
        .text_color(t.text_muted)
        .child(crate::trf!(
            "{} of {} events",
            shown,
            ui.log.as_ref().map_or(st.events, |p| p.total)
        ))
        .child(div().flex_1())
        .child(
            div()
                .id("chain")
                .flex()
                .items_center()
                .gap(px(6.))
                .text_color(match integrity.0 {
                    Tone::Ok => t.success_text,
                    Tone::Danger => t.danger_text,
                    _ => t.text_muted,
                })
                .tooltip(tip(tr(
                    "Each event is signed by the server and linked to the previous one",
                )))
                .child(icon("chain", 14., t.text_muted))
                .child(integrity.1),
        )
        .child(crate::trf!("Kept {}", keep));
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .bg(t.bg)
        .child(toolbar)
        .child(
            div()
                .id("log-table")
                .flex_1()
                .overflow_y_scroll()
                .child(table),
        )
        .child(status)
}

// ---------- Dialogs ----------

fn card(t: &Tokens, title: &str) -> Div {
    dialog_box(t).child(
        div()
            .text_size(TEXT_LG)
            .line_height(px(20.))
            .font_weight(W_MEDIUM)
            .child(crate::i18n::tr_owned(title)),
    )
}

fn actions(
    t: &Tokens,
    ok_id: &'static str,
    ok: &str,
    on_ok: Pick,
    cx: &mut Context<CleisApp>,
) -> Div {
    div()
        .pt(px(8.))
        .flex()
        .justify_end()
        .gap(px(8.))
        .child(
            button("org-cancel", t, BtnKind::Default, false)
                .child(tr("Cancel"))
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(o) = org_mut(this) {
                        o.ui.invite = None;
                        o.ui.invite_code = None;
                        o.ui.new_space = None;
                        o.ui.new_role = None;
                        o.ui.rekey = None;
                        o.ui.restore = None;
                        o.ui.rename = None;
                        o.ui.delete_space = None;
                    }
                    cx.notify();
                }))
                .test_support(),
        )
        .child(
            button(ok_id, t, BtnKind::Primary, false)
                .child(crate::i18n::tr_owned(ok))
                .on_click(cx.listener(move |this, _, _, cx| on_ok(this, 0, cx)))
                .test_support(),
        )
}

fn overlay(t: &Tokens, body: Div) -> Div {
    scrim(t).child(div().id("org-dialog").occlude().child(body))
}

fn method_short(m: Method) -> &'static str {
    match m {
        Method::AuthenticatorApp => "App",
        Method::Email => "E-mail",
        Method::Sms => "SMS",
        Method::Google => "Google",
        Method::Microsoft => "Microsoft",
    }
}

/// The dialog open over the main window, if any.
pub fn dialog(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Option<Div> {
    let o = s.org.as_ref()?;
    let st = o.state.as_ref();
    if let Some(c) = &o.code {
        let title = format!(
            "{} · {}",
            o.m.name,
            tr(if c.setup {
                "set up two-step verification"
            } else {
                "two-step verification"
            })
        );
        let mut body = card(t, &title);
        if c.setup && c.methods.len() > 1 {
            let methods = c.methods.clone();
            let labels: Vec<&str> = methods.iter().map(|m| method_short(*m)).collect();
            let at = methods.iter().position(|m| *m == c.method).unwrap_or(0);
            let pick: Pick = Rc::new(move |this, i, cx| {
                if let Some(m) = methods.get(i) {
                    this.org_setup_pick(*m, cx);
                }
            });
            body = body.child(segment("srv-method", t, &labels, at, pick, cx));
        }
        if c.setup && c.methods.is_empty() {
            body = body.child(div().text_color(t.text_muted).child(tr(
                "The security policy allows no method this server can carry out.",
            )));
        }
        let identity = c.method.is_identity() && !c.asked;
        let to_send = c.setup && !c.asked && matches!(c.method, Method::Email | Method::Sms);
        if identity {
            let label = crate::trf!("Continue with {}", method_short(c.method));
            body = body
                .child(div().text_color(t.text_muted).child(if c.setup {
                    crate::trf!("Your {} will confirm each sign-in.", tr(c.method.label()))
                } else {
                    crate::trf!("Sign in with your {}.", tr(c.method.label()))
                }))
                .child(
                    div().flex().justify_center().py(px(4.)).child(
                        button("srv-identity", t, BtnKind::Primary, false)
                            .when(c.busy, |b| b.opacity(0.6))
                            .child(if c.busy {
                                tr("Waiting for the browser…").to_owned()
                            } else {
                                label
                            })
                            .tooltip(tip(tr("Opens the sign-in page in your browser")))
                            .on_click(cx.listener(|this, _, _, cx| this.org_identity(cx)))
                            .test_support(),
                    ),
                );
        } else if to_send {
            if c.method == Method::Email {
                let email = o.me().map(|m| m.email.clone()).unwrap_or_default();
                body = body.child(hint(
                    "hint-email-code",
                    t,
                    crate::trf!("A code goes to {} at each sign-in.", email),
                ));
            } else if let Some(p) = &c.phone {
                body = body.child(field(t, tr("Phone number"), Input::new(p).id("srv-phone")));
            }
            body = body.child(
                div().flex().justify_end().child(
                    button("srv-send", t, BtnKind::Default, false)
                        .when(c.busy, |b| b.opacity(0.6))
                        .child(tr("Send code"))
                        .on_click(cx.listener(|this, _, _, cx| this.org_setup_send(cx)))
                        .test_support(),
                ),
            );
        }
        if let Some(secret) = &c.secret {
            let uri = cleis_core::totp::Totp::from_base32(secret)
                .map(|mut x| {
                    x.issuer = Some(o.m.name.clone());
                    x.account = o.me().map(|m| m.email.clone());
                    x.to_uri()
                })
                .unwrap_or_default();
            let grouped = secret
                .as_bytes()
                .chunks(4)
                .map(|c| String::from_utf8_lossy(c).into_owned())
                .collect::<Vec<_>>()
                .join(" ");
            let copy = secret.clone();
            body = body
                .child(hint(
                    "hint-srv-scan",
                    t,
                    "Scan with your authenticator app, then type the code.",
                ))
                .child(
                    div()
                        .flex()
                        .justify_center()
                        .child(super::security::qr(t, &uri)),
                )
                .child(
                    value_box(t)
                        .child(
                            div()
                                .flex_1()
                                .font_family(FONT_MONO)
                                .text_size(TEXT_SM)
                                .child(grouped),
                        )
                        .child(
                            icon_button("copy-srv-key", t, "copy", true)
                                .tooltip(tip(tr("Copy the key")))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.copy(copy.clone(), "Key", cx)
                                })),
                        ),
                );
        }
        let ask = c.asked && !identity && !to_send;
        body = body
            .when_some(c.message.clone().filter(|_| ask), |d, m| {
                d.child(div().text_color(t.text_muted).child(m))
            })
            .when_some(c.input.as_ref().filter(|_| ask), |d, i| {
                d.child(field(t, tr("Code"), Input::new(i).id("srv-code")))
            })
            .when_some(
                c.error.clone().map(|e| crate::i18n::tr_dynamic(&e)),
                |d, e| d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e)),
            )
            .child(
                div()
                    .pt(px(8.))
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        button("srv-code-later", t, BtnKind::Default, false)
                            .child(tr("Later"))
                            .on_click(cx.listener(|this, _, _, cx| this.org_cancel_code(cx)))
                            .test_support(),
                    )
                    .when(ask, |d| {
                        d.child(
                            button("srv-code-ok", t, BtnKind::Primary, false)
                                .when(c.busy, |b| b.opacity(0.6))
                                .child(tr("Continue"))
                                .on_click(cx.listener(|this, _, _, cx| this.org_submit_code(cx)))
                                .test_support(),
                        )
                    }),
            );
        return Some(overlay(t, body));
    }
    if let Some(code) = &o.ui.invite_code {
        let copy = code.clone();
        let body = card(t, tr("Invitation"))
            .child(
                div()
                    .text_color(t.text_muted)
                    .child(match &o.ui.invite_sent {
                        // Audit L-17: the server sent it.
                        Some(to) => {
                            crate::trf!(
                                "Sent to {} with the server address. Valid 7 days, once.",
                                to
                            )
                        }
                        None => {
                            tr("Send this code with the server address. Valid 7 days, once.").into()
                        }
                    }),
            )
            .child(
                value_box(t)
                    .child(div().flex_1().font_family(FONT_MONO).child(code.clone()))
                    .child(
                        icon_button("copy-invite", t, "copy", true)
                            .tooltip(tip(tr("Copy")))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.copy(copy.clone(), "Code", cx)
                            }))
                            .test_support(),
                    ),
            )
            .child(
                div().flex().justify_end().pt(px(8.)).child(
                    button("invite-done", t, BtnKind::Primary, false)
                        .child(tr("Done"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(o) = org_mut(this) {
                                o.ui.invite_code = None;
                                o.ui.invite = None;
                            }
                            cx.notify();
                        }))
                        .test_support(),
                ),
            );
        return Some(overlay(t, body));
    }
    if let (Some(f), Some(st)) = (&o.ui.invite, st) {
        let i_admins = o.can(Perm::Admins);
        let offered: Vec<&Role> = st
            .roles
            .iter()
            .filter(|r| i_admins || !r.is_admin())
            .collect();
        let roles: Vec<(String, bool)> = offered
            .iter()
            .map(|r| (crate::i18n::tr_owned(&r.name), r.id == f.role))
            .collect();
        let ids: Vec<String> = offered.iter().map(|r| r.id.clone()).collect();
        let mut spaces = div().flex().flex_wrap().gap(px(6.));
        for sv in &st.spaces {
            let on = f.spaces.contains(&sv.id);
            let id = sv.id;
            spaces = spaces.child(
                div()
                    .id(SharedString::from(format!("inv-sp-{id}")))
                    .cursor_pointer()
                    .child(chip(
                        t,
                        if on { Tone::Set } else { Tone::Neutral },
                        o.space_name(id),
                        on,
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(f) = org_mut(this).and_then(|o| o.ui.invite.as_mut())
                            && !f.spaces.remove(&id)
                        {
                            f.spaces.insert(id);
                        }
                        cx.notify();
                    }))
                    .test_support(),
            );
        }
        let body = card(t, tr("Invite"))
            .child(field(t, tr("E-mail"), Input::new(&f.email).id("inv-email")))
            .child(field(t, tr("Name"), Input::new(&f.name).id("inv-name")))
            .child(field(
                t,
                "Role",
                select(
                    "inv-role",
                    t,
                    200.,
                    o.ui.menu == Some(OrgMenu::InviteRole),
                    OrgMenu::InviteRole,
                    roles,
                    Rc::new(move |app, i, cx| {
                        if let Some(f) = org_mut(app).and_then(|o| o.ui.invite.as_mut()) {
                            f.role = ids[i].clone();
                        }
                        cx.notify();
                    }),
                    cx,
                ),
            ))
            .child(field(t, tr("Spaces"), spaces))
            .when_some(
                f.error.clone().map(|e| crate::i18n::tr_dynamic(&e)),
                |d, e| d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e)),
            )
            .child(actions(
                t,
                "inv-send",
                "Create code",
                Rc::new(|app, _, cx| app.send_invite(cx)),
                cx,
            ));
        return Some(overlay(t, body));
    }
    if let Some(f) = &o.ui.new_space {
        let i_super = st.is_some_and(|s| s.me.role == SUPER_ADMIN);
        let reserved = f.reserved;
        let body = card(t, tr("New space"))
            .child(field(t, tr("Name"), Input::new(&f.name).id("space-name")))
            .child(field(
                t,
                "Encryption",
                div().when(reserved, |d| d.opacity(0.5)).child(seg(
                    "space-mode",
                    t,
                    &["Standard", "End-to-end"],
                    usize::from(f.mode == SpaceMode::EndToEnd || reserved),
                    Rc::new(|app, i, cx| {
                        if let Some(f) = org_mut(app).and_then(|o| o.ui.new_space.as_mut()) {
                            f.mode = if i == 0 {
                                SpaceMode::Standard
                            } else {
                                SpaceMode::EndToEnd
                            };
                        }
                        cx.notify();
                    }),
                    cx,
                )),
            ))
            .when(i_super, |d| {
                d.child(
                    div()
                        .id("space-reserved")
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .cursor_pointer()
                        .tooltip(tip(
                            "End-to-end; its members are changed by a Super Admin only, other admins do not see it",
                        ))
                        .child(check_box(t, reserved))
                        .child(tr("Reserved"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(f) = org_mut(this).and_then(|o| o.ui.new_space.as_mut()) {
                                f.reserved = !f.reserved;
                            }
                            cx.notify();
                        }))
                        .test_support(),
                )
            })
            .child(actions(
                t,
                "space-create",
                "Create",
                Rc::new(|app, _, cx| app.create_space_from_dialog(cx)),
                cx,
            ));
        return Some(overlay(t, body));
    }
    if let Some(f) = &o.ui.new_role {
        let body = card(t, tr("New role"))
            .child(field(t, tr("Name"), Input::new(&f.name).id("role-name")))
            .child(hint(
                "hint-new-role",
                t,
                "Starts with Copy, fill; tick the rest in the table.",
            ))
            .child(actions(
                t,
                "role-create",
                "Create",
                Rc::new(|app, _, cx| app.create_role_from_dialog(cx)),
                cx,
            ));
        return Some(overlay(t, body));
    }
    if let (Some(r), Some(st)) = (&o.ui.rekey, st) {
        let sv = st.spaces.iter().find(|x| x.id == r.space)?;
        let name = o.space_name(r.space);
        let label = |m: SpaceMode| match m {
            SpaceMode::Standard => "Standard",
            SpaceMode::EndToEnd => "End-to-end",
        };
        let names: Vec<String> = sv
            .members
            .iter()
            .map(|m| {
                o.member_name(*m)
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_owned()
            })
            .collect();
        let kv = |k: &str, v: Div| {
            div()
                .flex()
                .gap(px(12.))
                .child(
                    div()
                        .w(px(110.))
                        .text_color(t.text_muted)
                        .child(crate::i18n::tr_owned(k)),
                )
                .child(v)
        };
        let backup_first = r.backup_first;
        let body = card(t, &crate::trf!("Re-encrypt “{}”", name))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(chip(t, Tone::Neutral, label(sv.mode), false))
                    .child(icon("arrowR", 14., t.text_muted))
                    .child(chip(t, Tone::Set, label(r.to), false)),
            )
            .child(kv("Items", div().child(sv.items.to_string())))
            .child(kv("Members", div().child(names.join(", "))))
            .child(kv(
                "Recovery key",
                if st.org.recovery_public.is_some() {
                    div().text_color(t.success_text).child(tr("Included"))
                } else {
                    div().text_color(t.text_muted).child(tr("Not set"))
                },
            ))
            .child(
                div()
                    .id("rk-backup")
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .cursor_pointer()
                    .child(check_box(t, backup_first))
                    .child(tr("Back up first"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(r) = org_mut(this).and_then(|o| o.ui.rekey.as_mut()) {
                            r.backup_first = !r.backup_first;
                        }
                        cx.notify();
                    }))
                    .test_support(),
            )
            .child(actions(
                t,
                "rk-go",
                "Re-encrypt",
                Rc::new(|app, _, cx| app.confirm_rekey(cx)),
                cx,
            ));
        return Some(overlay(t, body));
    }
    if let Some((id, input)) = &o.ui.rename {
        let body = card(t, &crate::trf!("Rename “{}”", o.space_name(*id)))
            .child(field(t, tr("Name"), Input::new(input).id("rename-space")))
            .child(actions(
                t,
                "rename-go",
                "Rename",
                Rc::new(|app, _, cx| app.rename_from_dialog(cx)),
                cx,
            ));
        return Some(overlay(t, body));
    }
    if let (Some(id), Some(st)) = (o.ui.delete_space, st) {
        let n = st.spaces.iter().find(|x| x.id == id).map_or(0, |x| x.items);
        let body = card(t, &crate::trf!("Delete “{}”?", o.space_name(id)))
            .child(div().text_color(t.text_muted).child({
                // Audit L-16: say whether a backup really keeps them.
                let last = s
                    .vault
                    .path()
                    .and_then(|p| crate::org::org_backups(&p, &o.m).first().map(|b| b.2));
                crate::trf!(
                    "Its {} item{} are erased for every member. {}",
                    n,
                    if n == 1 { "" } else { "s" },
                    match last {
                        Some(ts) => crate::trf!(
                            "The last backup of the organization, {}, keeps them.",
                            crate::format::date(ts)
                        ),
                        None =>
                            tr("No backup of the organization was made on this computer.").into(),
                    }
                )
            }))
            .child(
                div()
                    .pt(px(8.))
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        button("del-space-cancel", t, BtnKind::Default, false)
                            .child(tr("Cancel"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(o) = org_mut(this) {
                                    o.ui.delete_space = None;
                                }
                                cx.notify();
                            }))
                            .test_support(),
                    )
                    .child(
                        button("del-space-go", t, BtnKind::Danger, false)
                            .child(tr("Delete"))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(o) = org_mut(this) {
                                    o.ui.delete_space = None;
                                }
                                this.org_delete_space(id, cx);
                            }))
                            .test_support(),
                    ),
            );
        return Some(overlay(t, body));
    }
    if let Some(p) = &o.ui.restore {
        let p = p.clone();
        let body = card(t, tr("Restore the organization"))
            .child(div().text_color(t.text_muted).child(
                "The server goes back to this backup. Changes made since are lost for every member.",
            ))
            .child(actions(
                t,
                "obk-restore-go",
                "Restore",
                Rc::new(move |app, _, cx| {
                    if let Some(o) = org_mut(app) {
                        o.ui.restore = None;
                    }
                    app.org_restore_backup(p.clone(), cx);
                }),
                cx,
            ));
        return Some(overlay(t, body));
    }
    None
}

// ---------- Actions behind the dialogs ----------

/// A join form with its boxes; changes of the address or code look the invitation up.
pub fn join_form(window: &mut Window, cx: &mut Context<CleisApp>) -> (JoinForm, Vec<Subscription>) {
    let make = |ph: &str, window: &mut Window, cx: &mut Context<CleisApp>| {
        let ph = crate::i18n::tr_owned(ph);
        cx.new(|cx| InputState::new(window, cx).placeholder(ph))
    };
    let address = make("vault.example-studio.lan", window, cx);
    let code = make("XXXX-XXXX-…", window, cx);
    let name = make("", window, cx);
    let email = make("", window, cx);
    let org_name = make("Example Studio", window, cx);
    let creator = make("Lea Martin", window, cx);
    let subs = [&address, &code]
        .into_iter()
        .map(|e| {
            cx.subscribe_in(e, window, |this: &mut CleisApp, _, ev, _, cx| {
                if let gpui_kit::component::input::InputEvent::Change = ev {
                    this.check_join_code(cx);
                }
            })
        })
        .collect();
    (
        JoinForm {
            address: Some(address),
            code: Some(code),
            name: Some(name),
            email: Some(email),
            org_name: Some(org_name),
            creator: Some(creator),
            ..Default::default()
        },
        subs,
    )
}

impl CleisApp {
    /// Inputs of the organization screens (they need the window).
    pub fn ensure_org_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        // Preferences › Vaults: the personal vault's name, renamed on Enter or when left.
        if s.tab == crate::app::Tab::Settings && s.vault_name.is_none() {
            let name = s.personal_name();
            let st = cx.new(|cx| InputState::new(window, cx));
            st.update(cx, |i, cx| i.set_value(name, window, cx));
            let sub = cx.subscribe_in(
                &st,
                window,
                |this, input, ev: &gpui_kit::component::input::InputEvent, _, cx| {
                    use gpui_kit::component::input::InputEvent;
                    if matches!(ev, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                        let name = input.read(cx).value().trim().to_string();
                        this.rename_personal(&name, cx);
                    }
                },
            );
            if let Some(s) = self.session() {
                s.vault_name = Some((st, sub));
            }
        }
        let Some(s) = self.session() else { return };
        let joining = s.org.is_none() || s.settings_page == "Add an organization";
        if joining && s.join.address.is_none() {
            let (mut join, subs) = join_form(window, cx);
            join.elsewhere = s.join.elsewhere;
            if let Some(s) = self.session() {
                s.join = join;
                s.join_subs = subs;
            }
        }
        let Some(s) = self.session() else { return };
        if s.org
            .as_ref()
            .and_then(|o| o.code.as_ref())
            .is_some_and(|c| c.input.is_none())
        {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder("123 456"));
            input.update(cx, |i, cx| i.focus(window, cx));
            let sub = cx.subscribe_in(&input, window, |this: &mut CleisApp, _, ev, _, cx| {
                if let gpui_kit::component::input::InputEvent::PressEnter { .. } = ev {
                    this.org_submit_code(cx);
                }
            });
            if let Some(o) = self.session().and_then(|s| s.org.as_mut()) {
                if let Some(c) = o.code.as_mut() {
                    c.input = Some(input);
                }
                o.ui.subs.push(sub);
            }
        }
        if self
            .session()
            .and_then(|s| s.org.as_ref())
            .and_then(|o| o.code.as_ref())
            .is_some_and(|c| c.setup && c.phone.is_none())
        {
            let phone = cx.new(|cx| InputState::new(window, cx).placeholder("+33 6 99 00 00 12"));
            if let Some(c) = org_mut(self).and_then(|o| o.code.as_mut()) {
                c.phone = Some(phone);
            }
        }
        // Profile boxes, filled with the member's name once the state is known.
        let me = self
            .session()
            .and_then(|s| s.org.as_ref())
            .filter(|o| o.ui.profile.is_none())
            .and_then(|o| o.me().map(|m| (m.name.clone(), m.email.clone())));
        if let Some((name, email)) = me {
            let n = cx.new(|cx| InputState::new(window, cx));
            let e = cx.new(|cx| InputState::new(window, cx));
            n.update(cx, |i, cx| i.set_value(name, window, cx));
            e.update(cx, |i, cx| i.set_value(email, window, cx));
            if let Some(o) = org_mut(self) {
                o.ui.profile = Some((n, e));
            }
        }
        // Devices are fetched when their page is shown.
        let want_devices = self.session().is_some_and(|s| {
            s.settings_page == "Devices"
                && s.org
                    .as_ref()
                    .is_some_and(|o| o.ui.devices.is_none() && !o.ui.devices_busy)
        });
        if want_devices && !self.demo {
            self.org_devices(cx);
        }
        // Hosted server: its state and addresses, read when the Server page is shown.
        let want_host = self.session().is_some_and(|s| {
            s.settings_page == "Server"
                && s.org.as_ref().is_some_and(|o| {
                    o.local.host_dir.is_some() && o.ui.host.is_none() && !o.ui.host_busy
                })
        });
        if want_host && !self.demo {
            let mut port = crate::host::PORT;
            if let Some(o) = org_mut(self) {
                o.ui.host_busy = true;
                port = crate::host::port_of(&o.m.address);
            }
            let task = cx.background_spawn(async move {
                (crate::host::listening(port), crate::host::addresses(port))
            });
            cx.spawn(async move |this, cx| {
                let r = task.await;
                this.update(cx, |app, cx| {
                    if let Some(o) = org_mut(app) {
                        o.ui.host = Some(r);
                        o.ui.host_busy = false;
                    }
                    cx.notify();
                })
                .ok();
            })
            .detach();
        }
        let Some(s) = self.session() else { return };
        if let Some(o) = s.org.as_ref()
            && (o.ui.search.is_none() || o.ui.log_search.is_none())
        {
            let search = cx.new(|cx| InputState::new(window, cx).placeholder(tr("Search")));
            let log_search = cx.new(|cx| InputState::new(window, cx).placeholder(tr("Search")));
            let subs = vec![
                cx.observe(&search, |_, _, cx| cx.notify()),
                cx.observe(&log_search, |_, _, cx| cx.notify()),
            ];
            if let Some(o) = self.session().and_then(|s| s.org.as_mut()) {
                o.ui.search = Some(search);
                o.ui.log_search = Some(log_search);
                o.ui.subs.extend(subs);
            }
        }
    }

    /// The join form shown: first launch or Settings › Organization.
    pub(crate) fn join_form_mut(&mut self) -> Option<&mut JoinForm> {
        match &mut self.phase {
            crate::app::Phase::Welcome(w) => Some(&mut w.join),
            crate::app::Phase::Main(s) => Some(&mut s.join),
            _ => None,
        }
    }

    /// Looks the invitation code up as soon as it is complete.
    pub fn check_join_code(&mut self, cx: &mut Context<Self>) {
        if self.demo {
            return;
        }
        let Some(j) = self.join_form_mut() else {
            return;
        };
        let (Some(a), Some(c)) = (j.address.clone(), j.code.clone()) else {
            return;
        };
        let address = a.read(cx).value().to_string();
        let code = c.read(cx).value().to_string();
        let Some(j) = self.join_form_mut() else {
            return;
        };
        j.info = None;
        j.error = None;
        if Invitation::parse(&code).is_none() || server_address(&address).is_none() {
            cx.notify();
            return;
        }
        j.checking = true;
        let task = cx.background_spawn(async move { crate::org::lookup(&address, &code) });
        cx.spawn(async move |this, cx| {
            let r = task.await;
            this.update(cx, |app, cx| {
                if let Some(j) = app.join_form_mut() {
                    j.checking = false;
                    match r {
                        Ok(i) => j.info = Some(i),
                        Err(e) => j.error = Some(e.into()),
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub fn join_from_settings(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        if s.join.busy {
            return;
        }
        let read = |e: &Option<Entity<InputState>>, cx: &Context<Self>| {
            e.as_ref()
                .map(|i| i.read(cx).value().to_string())
                .unwrap_or_default()
        };
        let (address, code, name, email) = (
            read(&s.join.address, cx),
            read(&s.join.code, cx),
            read(&s.join.name, cx),
            read(&s.join.email, cx),
        );
        let need_name = s
            .join
            .info
            .as_ref()
            .is_some_and(|i| i.name.trim().is_empty());
        if need_name && name.trim().is_empty() {
            s.join.error = Some(tr("Enter your name").into());
            cx.notify();
            return;
        }
        s.join.busy = true;
        s.join.error = None;
        self.org_join(address, code, name, email, cx);
        cx.notify();
    }

    pub fn open_invite(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let email = cx.new(|cx| InputState::new(window, cx).placeholder(tr("name@example.com")));
        let name = cx.new(|cx| InputState::new(window, cx));
        email.update(cx, |i, cx| i.focus(window, cx));
        if let Some(o) = org_mut(self) {
            o.ui.invite = Some(InviteForm {
                email,
                name,
                role: "member".into(),
                spaces: BTreeSet::new(),
                error: None,
            });
        }
        cx.notify();
    }

    fn send_invite(&mut self, cx: &mut Context<Self>) {
        let Some(o) = org_mut(self) else { return };
        let Some(f) = o.ui.invite.as_mut() else {
            return;
        };
        let email = f.email.read(cx).value().trim().to_string();
        if !email.contains('@') {
            f.error = Some(tr("Enter an e-mail address").into());
            cx.notify();
            return;
        }
        let req = NewInvite {
            name: f.name.read(cx).value().trim().to_string(),
            email,
            role: f.role.clone(),
            spaces: f.spaces.iter().copied().collect(),
            address: o.m.address.clone(),
        };
        self.org_invite(req, cx);
    }

    pub fn open_new_space(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder(tr("Design")));
        name.update(cx, |i, cx| i.focus(window, cx));
        if let Some(o) = org_mut(self) {
            o.ui.new_space = Some(SpaceForm {
                name,
                mode: SpaceMode::Standard,
                reserved: false,
            });
        }
        cx.notify();
    }

    fn create_space_from_dialog(&mut self, cx: &mut Context<Self>) {
        let Some(o) = org_mut(self) else { return };
        let Some(f) = o.ui.new_space.take() else {
            return;
        };
        let name = f.name.read(cx).value().trim().to_string();
        if name.is_empty() {
            o.ui.new_space = Some(f);
            return;
        }
        self.org_new_space(name, f.mode, f.reserved, cx);
    }

    pub fn open_new_role(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder(tr("Contractor")));
        name.update(cx, |i, cx| i.focus(window, cx));
        if let Some(o) = org_mut(self) {
            o.ui.new_role = Some(RoleForm { name });
        }
        cx.notify();
    }

    fn create_role_from_dialog(&mut self, cx: &mut Context<Self>) {
        let Some(o) = org_mut(self) else { return };
        let Some(f) = o.ui.new_role.take() else {
            return;
        };
        let name = f.name.read(cx).value().trim().to_string();
        let Some(st) = o.state.as_ref() else { return };
        if name.is_empty()
            || st
                .roles
                .iter()
                .any(|r| cleis_core::text::fold(&r.name) == cleis_core::text::fold(&name))
        {
            o.ui.new_role = Some(f);
            return;
        }
        let mut roles = st.roles.clone();
        roles.push(Role {
            id: uuid::Uuid::now_v7().to_string(),
            name,
            perms: [Perm::Fill].into_iter().collect(),
        });
        self.org_set_roles(roles, cx);
    }

    fn confirm_rekey(&mut self, cx: &mut Context<Self>) {
        let Some(o) = org_mut(self) else { return };
        let Some(r) = o.ui.rekey.clone() else { return };
        o.ui.changes.retain(|c| c.0 != r.space);
        if r.backup_first {
            self.org_backup_now(cx);
        }
        self.org_rekey(r.space, r.to, cx);
    }
}

/// Dialogs opened for screenshots (`--menu`).
pub fn demo_dialog(app: &mut CleisApp, m: &str, window: &mut Window, cx: &mut Context<CleisApp>) {
    match m {
        "place-github" | "place-sftp" => {
            let kind = if m == "place-github" {
                crate::sync::Provider::GitHub
            } else {
                crate::sync::Provider::Sftp
            };
            app.open_place(kind, window, cx);
            let values: &[(usize, &str)] = if m == "place-github" {
                &[(0, "lea-example/cleis-vault"), (3, "github_pat_11AEXAMPLE")]
            } else {
                &[(0, "files.example-studio.com"), (2, "lea"), (4, "example")]
            };
            let inputs = app
                .place_slot()
                .and_then(|p| p.as_ref())
                .map(|d| d.inputs.clone())
                .unwrap_or_default();
            for (n, v) in values {
                if let Some(i) = inputs.get(*n) {
                    i.update(cx, |i, cx| i.set_value(*v, window, cx));
                }
            }
            if m == "place-sftp"
                && let Some(Some(d)) = app.place_slot()
            {
                d.host_key = Some(tr("SHA256:pT4bW1x0c9QeRk2v7mZ3yJ5nL8aF6hD4sG1oU9QcE").into());
            }
        }
        "create-elsewhere" => {
            if let Some(j) = app.join_form_mut() {
                j.elsewhere = true;
            }
        }
        "invite" => {
            app.open_invite(window, cx);
            if let Some(f) = org_mut(app).and_then(|o| o.ui.invite.as_mut()) {
                let (e, n) = (f.email.clone(), f.name.clone());
                e.update(cx, |i, cx| {
                    i.set_value("noa@example-studio.com", window, cx)
                });
                n.update(cx, |i, cx| i.set_value("Noa Lambert", window, cx));
            }
            if let Some(o) = org_mut(app) {
                let first = o
                    .state
                    .as_ref()
                    .and_then(|s| s.spaces.first())
                    .map(|s| s.id);
                if let (Some(f), Some(id)) = (o.ui.invite.as_mut(), first) {
                    f.spaces.insert(id);
                }
            }
        }
        "invite-code" => {
            if let Some(o) = org_mut(app) {
                o.ui.invite_code = Some("K7Q2-9MXA-4TRE-LB2C-WQ7N-3HJD-PXYV-6FKA".into());
            }
        }
        "new-space" => {
            app.open_new_space(window, cx);
            if let Some(f) = org_mut(app).and_then(|o| o.ui.new_space.as_ref()) {
                let n = f.name.clone();
                n.update(cx, |i, cx| i.set_value("Marketing", window, cx));
            }
        }
        "new-role" => app.open_new_role(window, cx),
        "new-folder" => app.open_new_folder(window, cx),
        "delete-folder" | "folder-menu" => {
            if let Some(s) = app.session() {
                let id = s.spaces.iter().find(|x| x.name == "Banking").map(|x| x.id);
                if let Some(id) = id {
                    if m == "folder-menu" {
                        s.folder_menu = Some(id);
                    } else {
                        s.folder = Some(crate::app::FolderDialog::Delete(id));
                    }
                }
            }
        }
        "super-admin" | "rename-space" | "delete-space" => {
            let (member, space) = org_mut(app)
                .and_then(|o| o.state.as_ref())
                .map(|s| {
                    (
                        s.members.get(1).map(|m| m.id),
                        s.spaces.get(1).map(|x| x.id),
                    )
                })
                .unwrap_or_default();
            match (m, member, space) {
                ("super-admin", Some(id), _) => {
                    if let Some(o) = org_mut(app) {
                        o.ui.menu = Some(OrgMenu::More(id));
                    }
                }
                ("rename-space", _, Some(id)) => app.open_rename(id, window, cx),
                ("delete-space", _, Some(id)) => {
                    if let Some(o) = org_mut(app) {
                        o.ui.delete_space = Some(id);
                    }
                }
                _ => {}
            }
        }
        "ctx-entry" | "ctx-space" | "ctx-contact" | "ctx-member" | "ctx-list" | "ctx-sidebar"
        | "confirm-transfer" | "confirm-remove" => {
            use super::ctx::{CtxMenu, CtxTarget, Sub};
            use gpui_kit::point;
            let Some(s) = app.session() else { return };
            let member = s
                .org
                .as_ref()
                .and_then(|o| o.state.as_ref())
                .and_then(|st| st.members.get(1).map(|m| m.id));
            // The void menus are opened with nothing selected (the panel on the right closed).
            if matches!(m, "ctx-list" | "ctx-sidebar") {
                s.selected = None;
            }
            let (target, at, sub) = match m {
                "ctx-entry" => {
                    let id = s.visible.get(1).map(|i| s.entries[*i].id);
                    s.selected = id;
                    (
                        id.map(CtxTarget::Entry),
                        point(px(470.), px(196.)),
                        Some(Sub::Move),
                    )
                }
                "ctx-space" => (
                    s.spaces
                        .iter()
                        .find(|x| x.name == "Design")
                        .map(|x| CtxTarget::Space(x.id)),
                    point(px(120.), px(400.)),
                    None,
                ),
                "ctx-contact" => {
                    let id = super::contacts::listed(s).get(1).map(|c| c.id);
                    s.selected_contact = id;
                    (id.map(CtxTarget::Contact), point(px(420.), px(196.)), None)
                }
                "ctx-member" => (
                    member.map(CtxTarget::Member),
                    point(px(640.), px(250.)),
                    Some(Sub::Role),
                ),
                "ctx-list" => (
                    Some(CtxTarget::ListVoid),
                    point(px(560.), px(640.)),
                    Some(Sub::Sort),
                ),
                "ctx-sidebar" => (
                    Some(CtxTarget::SidebarVoid),
                    point(px(120.), px(560.)),
                    None,
                ),
                _ => (None, point(px(0.), px(0.)), None),
            };
            if let Some(target) = target {
                s.ctx = Some(CtxMenu { at, target, sub });
            }
            if let Some(id) = member {
                match m {
                    "confirm-transfer" => {
                        app.ask_action(crate::app::ConfirmAction::TransferSuper(id), cx)
                    }
                    "confirm-remove" => {
                        app.ask_action(crate::app::ConfirmAction::RemoveMember(id), cx)
                    }
                    _ => {}
                }
            }
        }
        "recovery-key" => {
            if let Some(s) = app.session() {
                // A sample, never a real key.
                let sample: [u8; 32] = core::array::from_fn(|i| (i * 29 + 3) as u8);
                s.security.dialog = Some(super::security::SecDialog::RecoveryKey(
                    zeroize::Zeroizing::new(cleis_store::format_recovery(&sample)),
                ));
            }
        }
        "wheel" => {
            if let Some(s) = app.session() {
                s.wheel = Some(super::wheel::Wheel::new(crate::theme::parse_hex("#e06b9a")));
            }
        }
        "calendar" => {
            if let Some(s) = app.session() {
                use chrono::Datelike;
                let at = s
                    .selected_entry()
                    .and_then(|e| e.renewal.clone())
                    .and_then(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
                    .unwrap_or_else(|| chrono::Local::now().date_naive());
                s.calendar = Some(super::calendar::Calendar {
                    target: super::calendar::CalTarget::Field(cleis_core::edit::Field::Renewal),
                    year: at.year(),
                    month: at.month(),
                });
            }
        }
        "customize" => {
            if let Some(s) = app.session() {
                s.customize = true;
                s.selected = None;
            }
        }
        "health-vault" => {
            if let Some(s) = app.session() {
                s.health.vault = s.org.as_ref().map(|o| crate::app::Scope::Org(o.m.key()));
                s.health.vault_menu = true;
            }
        }
        "rekey" => {
            if let Some(o) = org_mut(app) {
                let acc = o
                    .state
                    .as_ref()
                    .and_then(|s| {
                        s.spaces
                            .iter()
                            // Audit L-15: a change the server allows (Design to end-to-end).
                            .find(|x| o.names.get(&x.id).is_some_and(|n| n == "Design"))
                    })
                    .map(|x| x.id);
                if let Some(id) = acc {
                    o.ui.changes = vec![(id, SpaceMode::EndToEnd)];
                    o.ui.rekey = Some(RekeyForm {
                        space: id,
                        to: SpaceMode::EndToEnd,
                        backup_first: true,
                    });
                }
            }
        }
        "server-code" => {
            if let Some(o) = org_mut(app) {
                let mut c = crate::org::CodePrompt::new(Method::AuthenticatorApp);
                c.setup = true;
                c.methods = vec![
                    Method::AuthenticatorApp,
                    Method::Email,
                    Method::Sms,
                    Method::Google,
                ];
                c.secret = Some("JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP".into());
                o.code = Some(c);
            }
        }
        "server-sms" => {
            if let Some(o) = org_mut(app) {
                let mut c = crate::org::CodePrompt::new(Method::Sms);
                c.setup = true;
                c.asked = false;
                c.methods = vec![
                    Method::AuthenticatorApp,
                    Method::Email,
                    Method::Sms,
                    Method::Google,
                ];
                o.code = Some(c);
            }
        }
        "server-email" => {
            if let Some(o) = org_mut(app) {
                let mut c = crate::org::CodePrompt::new(Method::Email);
                c.message = Some(tr("Enter the code sent to l•••@example-studio.com.").into());
                o.code = Some(c);
            }
        }
        "server-identity" => {
            if let Some(o) = org_mut(app) {
                let mut c = crate::org::CodePrompt::new(Method::Google);
                c.asked = false;
                o.code = Some(c);
            }
        }
        _ => {}
    }
}

/// First launch with the server option chosen and filled (screen 03), for screenshots.
pub fn demo_welcome(app: &mut CleisApp, window: &mut Window, cx: &mut Context<CleisApp>) {
    if let crate::app::Phase::Welcome(w) = &mut app.phase {
        w.org = true;
        w.join.info = Some(InviteInfo {
            org: tr("Example Studio").into(),
            name: tr("Lea Martin").into(),
            email: tr("lea@example-studio.com").into(),
        });
        let (a, c) = (w.join.address.clone(), w.join.code.clone());
        if let Some(a) = a {
            a.update(cx, |i, cx| {
                i.set_value("vault.example-studio.lan", window, cx)
            });
        }
        if let Some(c) = c {
            c.update(cx, |i, cx| {
                i.set_value("K7Q2-9MXA-4TRE-LB2C-WQ7N-3HJD-PXYV-6FKA", window, cx)
            });
        }
    }
}

// ---------- Profile, Devices, Spaces (mockups 26–28, D-60) ----------

pub fn profile(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let Some(o) = s.org.as_ref() else {
        return div();
    };
    let Some(me) = o.me() else {
        return head(t, tr("Profile"));
    };
    let role = o.role().map(|r| r.name.clone()).unwrap_or_default();
    let fp = cleis_crypto::fingerprint(&me.verify_key);
    let fp_copy = fp.clone();
    let mut identity = section(t)
        .child(titled(
            t,
            &o.m.name,
            Some(tr("Shown to the other members.")),
        ))
        .child(
            row(t, true)
                .child(avatar(t, &initials(&me.name), 40.))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .child(me.name.clone())
                        .child(
                            div()
                                .text_size(TEXT_SM)
                                .text_color(t.text_muted)
                                .child(role),
                        ),
                ),
        );
    if let Some((n, e)) = &o.ui.profile {
        identity = identity
            .child(
                div()
                    .flex()
                    .gap(px(16.))
                    .child(div().flex_1().child(field(
                        t,
                        tr("Name"),
                        Input::new(n).id("profile-name"),
                    )))
                    .child(div().flex_1().child(field(
                        t,
                        "E-mail",
                        Input::new(e).id("profile-email"),
                    ))),
            )
            .child(
                div().flex().justify_end().child(
                    button("profile-save", t, BtnKind::Primary, false)
                        .child(tr("Save"))
                        .on_click(cx.listener(|this, _, _, cx| this.save_profile(cx)))
                        .test_support(),
                ),
            );
    }
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(head(t, tr("Profile")))
        .child(identity)
        .child(
            section(t)
                .child(titled(
                    t,
                    tr("Account key"),
                    Some(tr("Signs you in to the server; never leaves your vault.")),
                ))
                .child(
                    row(t, true)
                        .child(div().flex_1().child(tr("Fingerprint")))
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(t.text_muted)
                                .child(fp),
                        )
                        .child(
                            icon_button("copy-account-fp", t, "copy", false)
                                .tooltip(tip(tr("Copy")))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.copy(fp_copy.clone(), "Fingerprint", cx)
                                })),
                        ),
                )
                .child(
                    row(t, false)
                        .child(
                            div()
                                .id("second-step")
                                .flex_1()
                                .tooltip(tip(tr("Asked by the server at sign-in; the code on open (Security) is this computer's own")))
                                .child(tr("Two-step verification")),
                        )
                        .child(
                            div().text_color(t.text_muted).child(
                                me.factor
                                    .map_or(tr("None").to_owned(), |m| tr(m.label()).to_owned()),
                            ),
                        )
                        .child(
                            button("second-step-set", t, BtnKind::Default, false)
                                .child(tr(if me.factor.is_some() {
                                    "Change"
                                } else {
                                    "Set up"
                                }))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if this.demo {
                                        this.toast(tr("Not in the demo"), cx);
                                    } else {
                                        this.org_setup_prompt(cx);
                                    }
                                }))
                                .test_support(),
                        ),
                ),
        )
}

pub fn devices(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let Some(o) = s.org.as_ref() else {
        return div();
    };
    let cols = |d: Div| d.flex().items_center().gap(px(12.)).px(px(16.));
    let mut table = div().flex().flex_col().child(
        cols(div())
            .h(px(32.))
            .border_b_1()
            .border_color(t.line)
            .text_size(TEXT_XS)
            .text_color(t.text_muted)
            .font_weight(W_MEDIUM)
            .child(div().flex_1().child(tr("Device")))
            .child(div().w(px(200.)).child(tr("Last active")))
            .child(div().w(px(180.)).child(tr("Two-step verification")))
            .child(div().w(px(100.))),
    );
    match &o.ui.devices {
        None => {
            table = table.child(
                div()
                    .p(px(16.))
                    .text_color(t.text_faint)
                    .child(tr("Loading…")),
            );
        }
        Some(list) => {
            for (i, d) in list.iter().enumerate() {
                let name = d.device.clone();
                let code = if d.current {
                    chip(t, Tone::Ok, tr("This device"), true)
                } else if let Some(until) = d.remembered_until {
                    div()
                        .child(chip(t, Tone::Neutral, tr("Remembered"), false))
                        .id(SharedString::from(format!("dev-rem-{i}")))
                        .tooltip(tip(crate::trf!(
                            "Code not asked until {}",
                            crate::format::date(until)
                        )))
                        .into_any_element()
                        .into_div()
                } else {
                    chip(t, Tone::Neutral, tr("Asked"), false)
                };
                table = table.child(
                    cols(div())
                        .h(px(44.))
                        .border_b_1()
                        .border_color(t.line)
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(icon("laptop", 16., t.text_muted))
                                .child(d.device.clone()),
                        )
                        .child(
                            div()
                                .w(px(200.))
                                .text_color(t.text_muted)
                                .child(if d.current {
                                    tr("Now").to_owned()
                                } else if d.last_active == 0 {
                                    "—".to_owned()
                                } else {
                                    crate::format::when_short(d.last_active, cleis_core::now())
                                }),
                        )
                        .child(div().w(px(180.)).flex().child(code))
                        .child(
                            div()
                                .w(px(100.))
                                .flex()
                                .justify_end()
                                .when(!d.current, |x| {
                                    x.child(
                                        button(
                                            SharedString::from(format!("signout-{i}")),
                                            t,
                                            BtnKind::Ghost,
                                            true,
                                        )
                                        .child(tr("Sign out"))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.org_sign_out(name.clone(), cx)
                                        }))
                                        .test_support(),
                                    )
                                }),
                        ),
                );
            }
        }
    }
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(head(t, tr("Devices")))
        .child(section(t).p_0().overflow_hidden().child(table))
}

trait IntoDiv {
    fn into_div(self) -> Div;
}
impl IntoDiv for gpui_kit::AnyElement {
    fn into_div(self) -> Div {
        div().child(self)
    }
}

pub fn spaces(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let Some(o) = s.org.as_ref() else {
        return div();
    };
    let Some(st) = o.state.as_ref() else {
        return head(t, tr("Spaces"));
    };
    let cols = |d: Div| d.flex().items_center().gap(px(12.)).px(px(16.));
    let mut table = div().flex().flex_col().child(
        cols(div())
            .h(px(32.))
            .border_b_1()
            .border_color(t.line)
            .text_size(TEXT_XS)
            .text_color(t.text_muted)
            .font_weight(W_MEDIUM)
            .child(div().flex_1().child(tr("Space")))
            .child(div().w(px(80.)).flex().justify_end().child(tr("Items")))
            .child(div().w(px(200.)).child(tr("Members")))
            .child(div().w(px(140.)).child(tr("Encryption")))
            .child(
                div()
                    .id("h-reserved")
                    .w(px(80.))
                    .child(tr("Reserved"))
                    .tooltip(tip(
                        "Members changed by a Super Admin only; other admins do not see it",
                    )),
            )
            .child(div().w(px(64.))),
    );
    let i_super = st.me.role == SUPER_ADMIN;
    for sv in &st.spaces {
        let id = sv.id;
        let reserved = sv.reserved;
        let holds = s.vault.space_key(id).is_some() && (!reserved || i_super);
        let mut faces = div().flex().items_center();
        for (k, m) in sv.members.iter().take(5).enumerate() {
            faces = faces.child(
                div()
                    .when(k > 0, |d| d.ml(px(-4.)))
                    .rounded_full()
                    .border_2()
                    .border_color(t.surface)
                    .child(avatar(t, &initials(&o.member_name(*m)), 22.)),
            );
        }
        faces = faces.child(
            div()
                .ml(px(8.))
                .text_color(t.text_muted)
                .child(sv.members.len().to_string()),
        );
        table = table.child(
            cols(div())
                .h(px(44.))
                .border_b_1()
                .border_color(t.line)
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(icon("folder", 16., t.text_muted))
                        .child(o.space_name(id)),
                )
                .child(
                    div()
                        .w(px(80.))
                        .flex()
                        .justify_end()
                        .text_color(t.text_muted)
                        .child(sv.items.to_string()),
                )
                .child(div().w(px(200.)).child(faces))
                .child(div().w(px(140.)).flex().child(match sv.mode {
                    SpaceMode::Standard => chip(t, Tone::Neutral, tr("Standard"), false),
                    SpaceMode::EndToEnd => chip(t, Tone::Ok, tr("End-to-end"), false),
                }))
                .child(div().w(px(80.)).flex().child(if i_super {
                    div()
                        .id(SharedString::from(format!("reserve-{id}")))
                        .cursor_pointer()
                        .child(check_box(t, sv.reserved))
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.org_reserve(id, !reserved, cx)),
                        )
                        .test_support()
                        .into_any_element()
                } else if sv.reserved {
                    icon("lock", 14., t.text_muted).into_any_element()
                } else {
                    div().into_any_element()
                }))
                .child(
                    div()
                        .w(px(64.))
                        .flex()
                        .justify_end()
                        .child(
                            icon_button(
                                SharedString::from(format!("rename-{id}")),
                                t,
                                "text",
                                true,
                            )
                            .when(!holds, |d| d.opacity(0.4))
                            .tooltip(tip(if holds {
                                "Rename"
                            } else {
                                "Only a member of the space renames it"
                            }))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                if holds {
                                    this.open_rename(id, window, cx);
                                }
                            }))
                            .test_support(),
                        )
                        .child(
                            icon_button(
                                SharedString::from(format!("delspace-{id}")),
                                t,
                                "trash",
                                true,
                            )
                            .tooltip(tip(tr("Delete")))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(o) = org_mut(this) {
                                    o.ui.delete_space = Some(id);
                                }
                                cx.notify();
                            }))
                            .test_support(),
                        ),
                ),
        );
    }
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            head(t, tr("Spaces")).child(
                button("spaces-new", t, BtnKind::Primary, false)
                    .child(icon("plus", 16., t.primary_ink))
                    .child(tr("New space"))
                    .on_click(cx.listener(|this, _, window, cx| this.open_new_space(window, cx)))
                    .test_support(),
            ),
        )
        .child(section(t).p_0().overflow_hidden().child(table))
}

impl CleisApp {
    /// Audit L-6: the server's address changes (new IP, new name); its pinned key does not, so
    /// another server at that address is refused.
    pub fn edit_server_address(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(o) = org_mut(self) else { return };
        let current = o.m.address.clone();
        let input = cx.new(|cx| InputState::new(window, cx));
        input.update(cx, |i, cx| {
            i.set_value(current, window, cx);
            i.focus(window, cx);
        });
        let sub = cx.subscribe_in(
            &input,
            window,
            |this, input, ev: &gpui_kit::component::input::InputEvent, _, cx| {
                if let gpui_kit::component::input::InputEvent::PressEnter { .. } = ev {
                    let value = input.read(cx).value().trim().to_string();
                    this.set_server_address(&value, cx);
                }
            },
        );
        if let Some(o) = org_mut(self) {
            o.ui.address = Some((input, sub));
        }
        cx.notify();
    }

    pub fn set_server_address(&mut self, value: &str, cx: &mut Context<Self>) {
        let ok = value
            .rsplit_once(':')
            .is_some_and(|(h, p)| !h.is_empty() && !h.contains('/') && p.parse::<u16>().is_ok());
        if !ok {
            self.toast(tr("host:port, e.g. vault.example-studio.lan:8443"), cx);
            return;
        }
        let Some(s) = self.session() else { return };
        if let Some(o) = s.org.as_mut() {
            o.ui.address = None;
            if o.m.address != value {
                o.m.address = value.to_owned();
                o.token = None;
                o.error = None;
                o.again = true;
                crate::org::save_memberships(s);
            }
        }
        cx.notify();
    }

    pub fn open_rename(&mut self, id: SpaceId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(o) = org_mut(self) else { return };
        let name = o.space_name(id);
        let input = cx.new(|cx| InputState::new(window, cx));
        input.update(cx, |i, cx| {
            i.set_value(name, window, cx);
            i.focus(window, cx);
        });
        if let Some(o) = org_mut(self) {
            o.ui.rename = Some((id, input));
        }
        cx.notify();
    }

    fn rename_from_dialog(&mut self, cx: &mut Context<Self>) {
        let Some(o) = org_mut(self) else { return };
        let Some((id, input)) = o.ui.rename.take() else {
            return;
        };
        let name = input.read(cx).value().trim().to_string();
        if name.is_empty() {
            o.ui.rename = Some((id, input));
            return;
        }
        self.org_rename_space(id, name, cx);
    }

    fn save_profile(&mut self, cx: &mut Context<Self>) {
        let Some(o) = org_mut(self) else { return };
        let Some((n, e)) = o.ui.profile.clone() else {
            return;
        };
        let req = ProfileChange {
            name: n.read(cx).value().trim().to_string(),
            email: e.read(cx).value().trim().to_string(),
        };
        self.org_request::<_, MemberView>(
            "/v1/me",
            req,
            |app, r, cx| {
                if r.is_ok() {
                    app.toast(tr("Saved"), cx);
                }
            },
            cx,
        );
    }
}
