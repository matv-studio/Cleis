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
    /// Server on the NAS: the commands to run by hand instead (no SSH).
    pub elsewhere: bool,
    /// Where the organization lives (D-78, mockup 36f).
    pub place: OrgPlace,
    pub nas: Option<NasForm>,
    /// Shared folder: the place chosen (0 SharePoint, 1 Google, 2 NAS share) and its folder.
    pub folder: Option<(usize, std::path::PathBuf)>,
    /// A folder chosen to join, put in the address box at the next render.
    pub address_fill: Option<String>,
    /// Shared folder on an SSH server: where, and how to sign in.
    pub sftp: Option<SftpForm>,
    /// Join an organization whose folder is on an SSH server (`sftp://…` address): its password.
    pub sftp_password: Option<Entity<InputState>>,
}

/// A shared folder on an SSH server (D-78).
pub struct SftpForm {
    pub host: Entity<InputState>,
    pub port: Entity<InputState>,
    pub user: Entity<InputState>,
    pub password: Entity<InputState>,
    pub path: Entity<InputState>,
}

/// Where a new organization lives (D-78; D-102: nothing chosen at first, then Server — on the
/// NAS, a Linux server or this computer — or Shared folder).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OrgPlace {
    #[default]
    None,
    Nas,
    /// Any Linux machine reached over SSH (a VPS, a mini PC): installed as on a NAS.
    Linux,
    Here,
    Folder,
}

impl OrgPlace {
    /// A server Cleis runs (not a shared folder).
    pub fn is_server(self) -> bool {
        matches!(self, OrgPlace::Nas | OrgPlace::Linux | OrgPlace::Here)
    }

    /// Installed over SSH.
    pub fn over_ssh(self) -> bool {
        matches!(self, OrgPlace::Nas | OrgPlace::Linux)
    }
}

/// Server on the NAS: how Cleis signs in to it, and what it found there.
pub struct NasForm {
    pub address: Entity<InputState>,
    pub port: Entity<InputState>,
    pub user: Entity<InputState>,
    pub password: Entity<InputState>,
    pub found: Option<crate::nas::Found>,
    /// What is being done (sending, creating), while it runs.
    pub step: Option<&'static str>,
}

#[derive(Default)]
pub struct OrgUi {
    pub subs: Vec<Subscription>,
    /// Security policy › Organization PIN: the new PIN's box (D-79).
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
    /// Spaces: the space unfolded, its members' rights shown (mockup 46f).
    pub space_open: Option<SpaceId>,
    /// Server hosted on this computer: running, and the addresses members can reach.
    pub host: Option<(bool, Vec<String>)>,
    pub host_busy: bool,
    /// Security policy › Accessing › Password › Set (mockup 42f): typed twice.
    pub access_pw: Option<AccessPwForm>,
    /// Server › Delete (mockup 42j, Super Admin).
    pub delete_org: Option<DeleteOrgForm>,
}

/// The organization's password typed twice (mockup 42f, D-98).
pub struct AccessPwForm {
    pub first: Entity<InputState>,
    pub confirm: Entity<InputState>,
    pub error: Option<SharedString>,
    pub busy: bool,
}

/// Deleting the organization: its name typed again, an export first (mockup 42j, D-98).
pub struct DeleteOrgForm {
    pub name: Entity<InputState>,
    pub export: bool,
    pub busy: bool,
    pub error: Option<SharedString>,
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
            || self.delete_space.is_some()
            || self.access_pw.is_some()
            || self.delete_org.as_ref().is_some_and(|d| !d.busy);
        self.invite = None;
        self.invite_code = None;
        self.new_space = None;
        self.new_role = None;
        self.rekey = None;
        self.restore = None;
        self.rename = None;
        self.delete_space = None;
        self.access_pw = None;
        if self.delete_org.as_ref().is_some_and(|d| !d.busy) {
            self.delete_org = None;
        }
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
        let two_fa = if m.pin_locked {
            // D-79: too many wrong PINs; "Unlock PIN" in the member's menu.
            chip(t, Tone::Danger, tr("PIN locked"), true)
        } else if m.two_fa {
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
                        .child(super::profile::member(t, m, 28.))
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
    // D-79: locked out by wrong PINs.
    if m.pin_locked && o.can(Perm::Rules) {
        menu = menu.child(
            menu_item(
                format!("unlockpin-{id}"),
                t,
                Some("lock"),
                tr("Unlock PIN"),
                false,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(o) = this.session().and_then(|s| s.org.as_mut()) {
                    o.ui.menu = None;
                }
                this.close_ctx(cx);
                this.org_reset_pin(id, cx);
            }))
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
const OFFLINE: [(u32, &str); 4] = [
    (1, "1 day"),
    (7, "7 days"),
    (14, "14 days"),
    (30, "30 days"),
];
const MIN_LEN: [u32; 6] = [8, 10, 12, 14, 16, 20];
/// Mockup 36g (D-79).
const PIN_AGAIN: [(u32, &str); 5] = [
    (1, "1 hour"),
    (4, "4 hours"),
    (8, "8 hours"),
    (24, "1 day"),
    (168, "7 days"),
];
const PIN_TRIES: [u32; 3] = [3, 5, 10];
const PIN_LENGTHS: [u8; 3] = [4, 6, 8];
/// Mockup 42c (D-98): what the organization asks.
const ACCESS_WITH: [AccessWith; 3] = [AccessWith::Password, AccessWith::Pin, AccessWith::Hello];
/// Mockup 42b: asked again after this long without using Cleis.
const ACCESS_IDLE: [(Option<u32>, &str); 6] = [
    (None, "Never"),
    (Some(5), "5 minutes"),
    (Some(15), "15 minutes"),
    (Some(30), "30 minutes"),
    (Some(60), "1 hour"),
    (Some(240), "4 hours"),
];

/// The name of an access way in the list (Windows Hello is Touch ID on a Mac).
pub fn access_label(w: AccessWith) -> &'static str {
    match w {
        AccessWith::Password => "Password",
        AccessWith::Pin => "PIN",
        AccessWith::Hello => {
            if cfg!(target_os = "macos") {
                "Touch ID"
            } else {
                "Windows Hello"
            }
        }
    }
}

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
    // Mockups 42a-42f (D-98): what the organization asks before its spaces show. How each
    // member opens Cleis and their own vault stays theirs.
    let acc = p.access;
    let mut access = section(t)
        .child(titled(
            t,
            &crate::trf!("Accessing {}", o.m.name),
            Some(tr(
                "Asked before the organization's spaces show. How each member opens Cleis and their own vault stays their choice.",
            )),
        ))
        .child(
            row(t, true)
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
        )
        .child(
            row(t, false)
                .child(div().flex_1().child(tr("Ask for authentication")))
                .child(
                    switch("pol-access", t, acc.on)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.org_set_policy(|p| p.access.on = !p.access.on, cx)
                        }))
                        .test_support(),
                ),
        );
    if acc.on {
        access = access.child(
            row(t, false)
                .child(div().flex_1().child(tr("With")))
                .child(sel(
                    "pol-access-with",
                    220.,
                    ACCESS_WITH
                        .iter()
                        .map(|w| (access_label(*w).into(), *w == acc.with))
                        .collect(),
                    Rc::new(|app, i, cx| {
                        app.org_set_policy(|p| p.access.with = ACCESS_WITH[i], cx)
                    }),
                    cx,
                )),
        );
        if acc.with == AccessWith::Pin {
            let len_ix = PIN_LENGTHS
                .iter()
                .position(|l| *l == acc.length)
                .unwrap_or(1);
            access = access.child(
                row(t, false)
                    .child(
                        div()
                            .id("pin-len-l")
                            .flex_1()
                            .child(tr("Length"))
                            .tooltip(tip(tr("Another length asks for a new PIN"))),
                    )
                    .child(seg(
                        "pin-len",
                        t,
                        &["4", "6", "8"],
                        len_ix,
                        Rc::new(|app, i, cx| {
                            app.org_set_policy(|p| p.access.length = PIN_LENGTHS[i], cx)
                        }),
                        cx,
                    )),
            );
        }
        if acc.with != AccessWith::Hello {
            let what = if acc.with == AccessWith::Pin {
                "PIN"
            } else {
                "Password"
            };
            access =
                access.child(
                    row(t, false)
                        .child(
                            div()
                                .id("pin-new-l")
                                .flex_1()
                                .child(tr(what))
                                .tooltip(tip(tr(
                                    "Give it to the members; every device is asked the new one",
                                ))),
                        )
                        .when(!st.pin_set && st.access_secret.is_none(), |d| {
                            d.child(chip(t, Tone::Warn, tr("Not set yet"), true))
                        })
                        .child(
                            button("pin-set", t, BtnKind::Default, false)
                                .child(tr(if st.pin_set || st.access_secret.is_some() {
                                    "Change"
                                } else {
                                    "Set"
                                }))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_access_setup(window, cx)
                                }))
                                .test_support(),
                        ),
                );
        }
        access = access
            .child(
                row(t, false)
                    .child(
                        div()
                            .id("pin-again-l")
                            .flex_1()
                            .child(tr("Ask again after"))
                            .tooltip(tip(tr("Since it was last given on the device"))),
                    )
                    .child(sel(
                        "pol-pin-again",
                        160.,
                        PIN_AGAIN
                            .iter()
                            .map(|(h, l)| ((*l).into(), *h == acc.again_hours))
                            .collect(),
                        Rc::new(|app, i, cx| {
                            app.org_set_policy(|p| p.access.again_hours = PIN_AGAIN[i].0, cx)
                        }),
                        cx,
                    )),
            )
            .child(
                row(t, false)
                    .child(
                        div()
                            .id("pin-idle-l")
                            .flex_1()
                            .child(tr("After inactivity"))
                            .tooltip(tip(tr("Without using Cleis"))),
                    )
                    .child(sel(
                        "pol-pin-idle",
                        160.,
                        ACCESS_IDLE
                            .iter()
                            .map(|(m, l)| ((*l).into(), *m == acc.idle_mins))
                            .collect(),
                        Rc::new(|app, i, cx| {
                            app.org_set_policy(|p| p.access.idle_mins = ACCESS_IDLE[i].0, cx)
                        }),
                        cx,
                    )),
            );
        if acc.with != AccessWith::Hello {
            access = access.child(
                row(t, false)
                    .child(
                        div()
                            .id("pin-tries-l")
                            .flex_1()
                            .child(tr("Wrong tries before locking"))
                            .tooltip(tip(tr(
                                "The member's access is suspended until an admin resets it",
                            ))),
                    )
                    .child(sel(
                        "pol-pin-tries",
                        160.,
                        PIN_TRIES
                            .iter()
                            .map(|n| (n.to_string(), *n == acc.tries))
                            .collect(),
                        Rc::new(|app, i, cx| {
                            app.org_set_policy(|p| p.access.tries = PIN_TRIES[i], cx)
                        }),
                        cx,
                    )),
            );
        }
    }
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
        .child(rules_lock("pol-lock-access", rules, access))
        .child(rules_lock("pol-lock-signing", rules, signing))
        .child(rules_lock("pol-lock-methods", rules, methods))
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
    // D-99: only the organization's spaces; the personal vaults are not the organization's.
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
            Some(tr("Restores a space when every member holding it lost their password. Kept outside Cleis.")),
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
                .child(logo_row(o, t, cx))
                .when(hosted, |d| {
                    let copy = addr_text.clone();
                    d.child(
                        row(t, false)
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
                        row(t, false)
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
        // Mockups 42i, 42j (D-98): the Super Admin deletes the organization.
        .when(o.role().is_some_and(|r| r.is_super()), |d| {
            d.child(
                section(t).child(titled(t, tr("Delete"), None)).child(
                    row(t, true)
                        .child(
                            div()
                                .id("del-org-l")
                                .flex_1()
                                .child(tr("Server and its data"))
                                .tooltip(tip(tr(
                                    "Its spaces, members, log and backups. Members' copies go at their next sync.",
                                ))),
                        )
                        .child(
                            button("del-org", t, BtnKind::Danger, true)
                                .child(icon("trash", 14., gpui_kit::white()))
                                .child(tr("Delete server"))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_delete_org(window, cx)
                                }))
                                .test_support(),
                        ),
                ),
            )
        })
}

/// Server › Logo (mockups 42i, 42k, D-98): the logo itself is the button (its hover says so);
/// × takes it away. Admins with the Rules right; the others see it.
fn logo_row(o: &OrgSession, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let rules = o.can(Perm::Rules);
    let logo = o.local.logo.clone();
    let mut r = row(t, true).child(
        div()
            .id("logo-l")
            .flex_1()
            .child(tr("Logo"))
            .tooltip(tip(tr(
                "Shown to the members when the organization asks for authentication. PNG or JPEG, square.",
            ))),
    );
    match (&logo, rules) {
        (None, false) => r = r.child(div().text_color(t.text_muted).child(tr("None"))),
        (None, true) => {
            r = r.child(
                button("logo-add", t, BtnKind::Default, true)
                    .child(icon("plus", 14., t.text))
                    .child(tr("Add"))
                    .on_click(cx.listener(|this, _, _, cx| this.org_pick_logo(cx)))
                    .test_support(),
            )
        }
        (Some(b), _) => {
            r = r.child(
                div()
                    .id("logo-change")
                    .relative()
                    .rounded(px(6.))
                    .child(org_logo(t, Some(b), &o.m.name, 32.))
                    .when(rules, |d| {
                        d.cursor_pointer()
                            .group("logo")
                            .child(
                                div()
                                    .absolute()
                                    .inset_0()
                                    .rounded(px(6.))
                                    .border_1()
                                    .border_color(t.focus)
                                    .bg(t.overlay)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .invisible()
                                    .group_hover("logo", |d| d.visible())
                                    .child(icon("upload", 14., gpui_kit::white())),
                            )
                            .tooltip(tip(tr("Change")))
                            .on_click(cx.listener(|this, _, _, cx| this.org_pick_logo(cx)))
                    })
                    .test_support(),
            );
            if rules {
                r = r.child(
                    icon_button("logo-remove", t, "x", true)
                        .tooltip(tip(tr("Remove")))
                        .on_click(cx.listener(|this, _, _, cx| this.org_set_logo(None, cx)))
                        .test_support(),
                );
            }
        }
    }
    r
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

/// Create an organization (mockups 47a–47d, D-102): two choices, none chosen at first — Server
/// (where: NAS, Linux server over SSH, this computer) or Shared folder. What each means is in its
/// tooltip, never a line under it.
fn create_section(j: &JoinForm, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    use super::welcome::choice_with;
    let Some(name) = &j.org_name else {
        return div();
    };
    let server = j.place.is_server();
    let server_extra = server.then(|| server_fields(j, t, cx));
    let folder_extra = (j.place == OrgPlace::Folder).then(|| folder_places(j, t, cx));
    let opt = |id: &'static str,
               on: bool,
               p: OrgPlace,
               title: &str,
               tip_text: &'static str,
               ico: &str,
               extra: Option<Div>,
               cx: &mut Context<CleisApp>| {
        div()
            .id(id)
            .cursor_pointer()
            .child(choice_with(t, on, title, "", ico, extra))
            .tooltip(tip(tr(tip_text)))
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(j) = this.join_form_mut() {
                    // Server again: its place stays as chosen.
                    if !(p.is_server() && j.place.is_server()) {
                        j.place = p;
                    }
                    j.error = None;
                }
                cx.notify();
            }))
            .test_support()
    };
    let busy = j.busy;
    let label = match (j.place, busy, j.nas.as_ref().and_then(|n| n.step)) {
        (_, true, Some(step)) => crate::i18n::tr_owned(step),
        (_, true, None) => tr("Creating…").to_owned(),
        (p, false, _) if p.over_ssh() && !j.elsewhere => tr("Install and create").to_owned(),
        _ => tr("Create").to_owned(),
    };
    let unchosen = j.place == OrgPlace::None;
    let planned = unchosen || (j.place == OrgPlace::Folder && j.folder.is_none());
    let by_hand = j.place.over_ssh() && j.elsewhere;
    section(t)
        .child(titled(t, tr("Create an organization"), None))
        .child(
            div()
                .flex()
                .gap(px(12.))
                .child(div().flex_1().child(field(
                    t,
                    tr("Name"),
                    Input::new(name).id("create-name"),
                )))
                .when_some(j.creator.as_ref().filter(|_| !by_hand), |d, c| {
                    d.child(div().flex_1().child(field(
                        t,
                        tr("Your name"),
                        Input::new(c).id("create-me"),
                    )))
                }),
        )
        .child(opt(
            "create-server",
            server,
            OrgPlace::Nas,
            "Server",
            "Access changes at once; members reach it over the network or your VPN",
            "server",
            server_extra,
            cx,
        ))
        .child(opt(
            "create-folder",
            j.place == OrgPlace::Folder,
            OrgPlace::Folder,
            "Shared folder",
            "No server: SharePoint, Google shared drive, NAS share or SFTP; access changes at the next sync",
            "folder",
            folder_extra,
            cx,
        ))
        .when_some(
            j.error.clone().map(|e| crate::i18n::tr_dynamic(&e)),
            |d, e| d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e)),
        )
        .when(!by_hand, |d| {
            d.child(
                div().flex().justify_end().child(
                    button("create-org", t, BtnKind::Primary, false)
                        .when(busy || planned, |b| b.opacity(0.45))
                        .when(planned, |b| {
                            b.tooltip(tip(tr(if unchosen {
                                "Choose Server or Shared folder"
                            } else {
                                "Choose the shared folder first"
                            })))
                        })
                        .child(label)
                        .when(!planned, |b| {
                            b.on_click(cx.listener(|this, _, _, cx| this.create_org_clicked(cx)))
                        })
                        .test_support(),
                ),
            )
        })
}

/// Mockups 47b, 47c: where the server runs, then what that place needs.
fn server_fields(j: &JoinForm, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    const WHERE: [(OrgPlace, &str, &str, &str); 3] = [
        (
            OrgPlace::Nas,
            "server",
            "NAS",
            "Cleis installs the server on your NAS over SSH (QNAP, Synology, other Linux NAS)",
        ),
        (
            OrgPlace::Linux,
            "window",
            "Linux server",
            "Any Linux machine you reach over SSH: a VPS, a mini PC…",
        ),
        (
            OrgPlace::Here,
            "laptop",
            "This computer",
            "The server runs here; members reach it only while this computer is on",
        ),
    ];
    let mut seg = div()
        .flex()
        .flex_none()
        .p(px(2.))
        .gap(px(2.))
        .bg(t.bg)
        .border_1()
        .border_color(t.line_strong)
        .rounded(RADIUS_MD);
    for (n, (p, ico, label, tip_text)) in WHERE.into_iter().enumerate() {
        let on = j.place == p;
        let fg = if on { t.accent_text } else { t.text_muted };
        let hover = t.surface_hover;
        seg = seg.child(
            div()
                .id(SharedString::from(format!("create-where-{n}")))
                .h(px(22.))
                .px(px(10.))
                .flex()
                .items_center()
                .gap(px(6.))
                .rounded(RADIUS_SM)
                .cursor_pointer()
                .text_color(fg)
                .font_weight(W_MEDIUM)
                .when(on, |d| d.bg(t.accent_soft))
                .when(!on, |d| d.hover(move |d| d.bg(hover)))
                .child(icon(ico, 14., fg))
                .child(tr(label))
                .tooltip(tip(tr(tip_text)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    if let Some(j) = this.join_form_mut() {
                        j.place = p;
                        j.error = None;
                    }
                    cx.notify();
                }))
                .test_support(),
        );
    }
    let body = if j.place.over_ssh() {
        nas_fields(j, t, cx)
    } else {
        div()
            .flex()
            .items_center()
            .gap(px(12.))
            .child(
                div()
                    .id("create-here-tip")
                    .flex_1()
                    .text_color(t.text_muted)
                    .child(tr("This computer"))
                    .tooltip(tip(tr(
                        "Reachable by the other members only while this computer is on",
                    )))
                    .test_support(),
            )
            .child(chip(t, Tone::Neutral, crate::org::computer_name(), false))
    };
    div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(12.))
                .child(div().flex_1().child(tr("Where")))
                .child(seg),
        )
        .child(body)
}

/// Server on the NAS: address, SSH port, administrator and password (used once, never kept);
/// what was found there; or the commands to run by hand.
fn nas_fields(j: &JoinForm, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let org = j
        .org_name
        .as_ref()
        .map(|n| n.read(cx).value().trim().to_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| tr("Example Studio").to_owned());
    let toggle = div()
        .id("create-by-hand")
        .text_size(TEXT_SM)
        .text_color(t.text_muted)
        .cursor_pointer()
        .hover(|d| d.text_color(t.text))
        .child(tr(if j.elsewhere {
            "Let Cleis install it"
        } else {
            "Install it by hand"
        }))
        .tooltip(tip(tr("Linux, Windows, macOS or Docker")))
        .on_click(cx.listener(|this, _, _, cx| {
            if let Some(j) = this.join_form_mut() {
                j.elsewhere = !j.elsewhere;
            }
            cx.notify();
        }))
        .test_support();
    if j.elsewhere {
        let cmds = [
            crate::trf!("cleis-server init --org \"{}\"", org),
            "cleis-server install".to_owned(),
        ];
        let mut d = div().flex().flex_col().gap(px(8.));
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
        return d
            .child(hint(
                "create-then",
                t,
                "Then join below with the code it prints and the machine's address.",
            ))
            .child(toggle);
    }
    let Some(n) = j.nas.as_ref() else {
        return div().child(toggle);
    };
    let found = n.found.as_ref().map(|f| {
        let what = match f.system {
            crate::nas::System::Qnap => tr("QNAP · App Center").to_owned(),
            crate::nas::System::Systemd => tr("Linux · service").to_owned(),
            crate::nas::System::Plain => tr("Linux · not started with the NAS").to_owned(),
        };
        row(t, false)
            .child(div().flex_1().text_color(t.text_muted).child(tr("Found")))
            .child(
                div()
                    .id("nas-found")
                    .tooltip(tip(crate::trf!("SSH key {}", f.host_key)))
                    .child(chip(
                        t,
                        if f.system == crate::nas::System::Plain {
                            Tone::Warn
                        } else {
                            Tone::Ok
                        },
                        what,
                        true,
                    )),
            )
    });
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .flex()
                .gap(px(12.))
                .child(div().flex_1().child(field(
                    t,
                    tr(if j.place == OrgPlace::Linux {
                        "Server address"
                    } else {
                        "NAS address"
                    }),
                    Input::new(&n.address).id("nas-address"),
                )))
                .child(div().w(px(90.)).child(field(
                    t,
                    tr("SSH port"),
                    Input::new(&n.port).id("nas-port"),
                ))),
        )
        .child(
            div()
                .flex()
                .gap(px(12.))
                .child(div().flex_1().child(field(
                    t,
                    tr("Administrator"),
                    Input::new(&n.user).id("nas-user"),
                )))
                .child(div().flex_1().child(field(
                    t,
                    tr("Password"),
                    Input::new(&n.password).id("nas-password").mask_toggle(),
                ))),
        )
        .children(found)
        .child(toggle)
}

/// Shared folder (mockup 36i): SharePoint, Google shared drive, NAS share or SFTP; the one chosen
/// shows its folder.
fn folder_places(j: &JoinForm, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    const PLACES: [(&str, &str, &str); 3] = [
        ("cloud", "SharePoint", "Library synced by the OneDrive app"),
        (
            "cloud",
            "Google shared drive",
            "Folder kept by Google Drive for desktop",
        ),
        ("folder", "NAS share", "Network folder"),
    ];
    let mut list = div()
        .px(px(12.))
        .bg(t.bg)
        .border_1()
        .border_color(t.line_strong)
        .rounded(RADIUS_MD);
    for (i, (ico, name, sub)) in PLACES.into_iter().enumerate() {
        let chosen = j.folder.as_ref().filter(|(k, _)| *k == i);
        let on = chosen.is_some();
        // What it is goes in the tooltip; under the name, only the folder chosen (D-102).
        let path = chosen.map(|(_, p)| p.display().to_string());
        list = list.child(
            row(t, i > 0)
                .id(SharedString::from(format!("folder-place-{i}")))
                .tooltip(tip(tr(sub)))
                .py(px(10.))
                .gap(px(10.))
                .child(
                    div()
                        .flex_none()
                        .size(px(12.))
                        .rounded_full()
                        .border(px(if on { 4. } else { 1.5 }))
                        .border_color(if on { t.accent_text } else { t.text_faint }),
                )
                .child(icon(ico, 16., t.text_muted))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(crate::i18n::tr_owned(name))
                        .when_some(path, |d, p| {
                            d.child(
                                div()
                                    .text_size(TEXT_SM)
                                    .text_color(t.text_muted)
                                    .font_family(FONT_MONO)
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .child(p),
                            )
                        }),
                )
                .child(
                    button(
                        SharedString::from(format!("folder-pick-{i}")),
                        t,
                        BtnKind::Default,
                        true,
                    )
                    .child(tr(if on { "Change" } else { "Choose folder" }))
                    .on_click(cx.listener(move |this, _, _, cx| this.pick_org_folder(i, cx)))
                    .test_support(),
                ),
        );
    }
    let on = j.folder.as_ref().is_some_and(|(k, _)| *k == 3);
    let fields = j.sftp.as_ref().filter(|_| on).map(|f| {
        div()
            .pb(px(12.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .gap(px(12.))
                    .child(div().flex_1().child(field(
                        t,
                        tr("Server"),
                        Input::new(&f.host).id("sftp-host"),
                    )))
                    .child(div().w(px(90.)).child(field(
                        t,
                        tr("SSH port"),
                        Input::new(&f.port).id("sftp-port"),
                    ))),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.))
                    .child(div().flex_1().child(field(
                        t,
                        tr("User"),
                        Input::new(&f.user).id("sftp-user"),
                    )))
                    .child(div().flex_1().child(field(
                        t,
                        tr("Password"),
                        Input::new(&f.password).id("sftp-password").mask_toggle(),
                    ))),
            )
            .child(field(t, tr("Folder"), Input::new(&f.path).id("sftp-path")))
    });
    list.child(
        row(t, true)
            .id("folder-place-sftp")
            .tooltip(tip(tr("A folder on an SSH server")))
            .py(px(10.))
            .gap(px(10.))
            .child(
                div()
                    .flex_none()
                    .size(px(12.))
                    .rounded_full()
                    .border(px(if on { 4. } else { 1.5 }))
                    .border_color(if on { t.accent_text } else { t.text_faint }),
            )
            .child(icon("server", 16., t.text_muted))
            .child(div().flex_1().child(tr("SFTP")))
            .when(!on, |d| {
                d.child(
                    button("folder-pick-sftp", t, BtnKind::Default, true)
                        .child(tr("Set up"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(j) = this.join_form_mut() {
                                j.folder = Some((3, std::path::PathBuf::from("sftp://")));
                                j.place = OrgPlace::Folder;
                            }
                            cx.notify();
                        }))
                        .test_support(),
                )
            }),
    )
    .children(fields)
}

/// Address, invitation code, and name/e-mail when the invitation has none.
pub fn join_fields(j: &JoinForm, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
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
                        .when_some(chip_el, |d, c| d.child(c))
                        // D-78: an organization in a shared folder joins from its folder.
                        .child(
                            icon_button("join-folder", t, "folder", false)
                                .tooltip(tip(tr("Organization in a shared folder")))
                                .on_click(cx.listener(|this, _, _, cx| this.pick_join_folder(cx)))
                                .test_support(),
                        ),
                )),
        )
        .when(
            address.read(cx).value().trim().starts_with("sftp://"),
            |d| {
                d.when_some(j.sftp_password.as_ref(), |d, p| {
                    d.child(field(
                        t,
                        tr("SFTP password"),
                        Input::new(p).id("org-sftp-password").mask_toggle(),
                    ))
                })
            },
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

/// The organization the vault shows (its tab, or its section in the sidebar), if any (D-98: its
/// logo then takes the profile's place at the top right).
pub fn org_in_view(s: &Session) -> Option<&OrgSession> {
    use crate::app::{Filter, Scope, Tab};
    match s.tab {
        Tab::Vault => {
            // One of its folders counts too (27/11/2026).
            if let Filter::Space(sp) = s.filter
                && let Some(o) = s.org_of(sp)
            {
                return Some(o);
            }
            s.all_orgs().find(|o| match (&s.scope, &s.filter) {
                (Scope::Org(k), _) => *k == o.m.key(),
                (_, Filter::Org(h)) => *h == crate::app::org_hash(&o.m.key()),
                _ => false,
            })
        }
        Tab::Contacts => match &s.contacts_ui.filter {
            super::contacts::ContactFilter::Folder(sp) => s.org_of(*sp),
            super::contacts::ContactFilter::Org(k) => s.all_orgs().find(|o| o.m.key() == *k),
            _ => match &s.scope {
                Scope::Org(k) => s.all_orgs().find(|o| o.m.key() == *k),
                _ => None,
            },
        },
        _ => None,
    }
}

/// The organization whose PIN is asked, when the vault shows it (its tab, or its section in the
/// sidebar) (D-79).
pub fn pin_shown(s: &Session) -> Option<&OrgSession> {
    use crate::app::{Filter, Scope};
    s.all_orgs().find(|o| {
        o.pin.is_some()
            && match (&s.scope, &s.filter) {
                (Scope::Org(k), _) => *k == o.m.key(),
                (_, Filter::Org(h)) => *h == crate::app::org_hash(&o.m.key()),
                _ => false,
            }
    })
}

/// Mockups 36h, 42g, 42h (D-98): what the organization asks, in the place of its items, under
/// its logo.
pub fn pin_card(o: &OrgSession, t: &Tokens, window: &Window, cx: &mut Context<CleisApp>) -> Div {
    let Some(p) = o.pin.as_ref() else {
        return div();
    };
    let key = o.m.key();
    let busy = p.busy;
    let label = match p.with {
        AccessWith::Pin => Some(tr("Organization PIN")),
        AccessWith::Password => Some(tr("Organization password")),
        AccessWith::Hello => None,
    };
    let mut body = section(t).gap(px(12.)).child(
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(8.))
            .child(
                div()
                    .relative()
                    .child(org_logo(t, o.local.logo.as_deref(), &o.m.name, 48.))
                    .child(
                        div()
                            .absolute()
                            .right(px(-6.))
                            .bottom(px(-6.))
                            .size(px(20.))
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(t.surface_raised)
                            .border_1()
                            .border_color(t.line_strong)
                            .child(icon("lock", 11., t.text)),
                    ),
            )
            .child(
                div()
                    .pt(px(4.))
                    .text_size(TEXT_XL)
                    .line_height(px(26.))
                    .font_weight(W_STRONG)
                    .child(o.m.name.clone()),
            )
            .when_some(label, |d, l| {
                d.child(
                    div()
                        .id("pin-label")
                        .text_color(t.text_muted)
                        .child(l)
                        .tooltip(tip(if p.with == AccessWith::Pin {
                            crate::trf!("Set by your organization · {} digits", p.len)
                        } else {
                            tr("Set by your organization").into()
                        })),
                )
            }),
    );
    if p.locked {
        body = body.child(
            div()
                .id("pin-locked")
                .text_align(gpui_kit::TextAlign::Center)
                .text_color(t.danger_text)
                .child(tr(
                    "Too many wrong tries. Ask an admin to unlock your access.",
                )),
        );
    } else {
        body = match p.with {
            AccessWith::Pin => body.when_some(p.boxes.as_ref(), |d, b| {
                // The boxes of the organization shown (only one card at a time).
                let get: super::pin::Get = |app| {
                    let s = app.session()?;
                    let key = pin_shown(s)?.m.key();
                    s.all_orgs_mut()
                        .find(|o| o.m.key() == key)?
                        .pin
                        .as_mut()?
                        .boxes
                        .as_mut()
                };
                let done: super::pin::Done = |app, _, cx| {
                    let Some(key) = app.session().and_then(|s| pin_shown(s).map(|o| o.m.key()))
                    else {
                        return;
                    };
                    app.in_org(&key, |app| app.org_pin_submit(cx));
                };
                d.child(super::pin::boxes("org-pin", b, t, window, get, done, cx))
            }),
            AccessWith::Password => body.when_some(p.field.as_ref(), |d, f| {
                d.child(Input::new(f).id("org-password"))
            }),
            AccessWith::Hello => body,
        };
        let ready = match p.with {
            AccessWith::Pin => p.boxes.as_ref().is_some_and(|b| b.full()),
            _ => true,
        };
        let (open_label, ico) = match p.with {
            AccessWith::Hello => (
                if cfg!(target_os = "macos") {
                    tr("Open with Touch ID")
                } else {
                    tr("Open with Windows Hello")
                },
                Some("fingerprint"),
            ),
            _ => (tr("Open"), None),
        };
        body = body
            .when_some(p.error.clone(), |d, e| {
                d.child(
                    div()
                        .id("pin-error")
                        .text_size(TEXT_SM)
                        .text_color(t.danger_text)
                        .child(crate::i18n::tr_dynamic(&e)),
                )
            })
            .child(
                button("pin-open", t, BtnKind::Primary, false)
                    .h(px(32.))
                    .w_full()
                    .when(busy || !ready, |b| b.opacity(0.6))
                    .when_some(ico, |b, i| b.child(icon(i, 16., t.primary_ink)))
                    .child(open_label)
                    .when(p.with == AccessWith::Hello, |b| {
                        b.tooltip(tip(tr(crate::opening::hello_tip(
                            crate::opening::hello_available(),
                        ))))
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let key = key.clone();
                        this.in_org(&key, |app| app.org_pin_submit(cx));
                    }))
                    .test_support(),
            );
    }
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .items_center()
        .justify_center()
        .child(div().w(px(340.)).child(body))
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
                        // D-78: the folder, shared with the member, takes the place of an address.
                        None if o.m.in_folder() => tr(
                            "Send this code, and share the organization's folder with this person. Valid 7 days, once.",
                        )
                        .into(),
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
    if let Some(f) = &o.ui.access_pw {
        let body = card(t, &crate::trf!("{} password", o.m.name))
            .child(
                div()
                    .id("access-pw-l")
                    .tooltip(tip(crate::trf!(
                        "At least {} characters",
                        ACCESS_PASSWORD_MIN
                    )))
                    .child(field(
                        t,
                        tr("Password"),
                        Input::new(&f.first).id("access-pw"),
                    )),
            )
            .child(field(
                t,
                tr("Confirm"),
                Input::new(&f.confirm).id("access-pw-confirm"),
            ))
            .when_some(f.error.clone(), |d, e| {
                d.child(
                    div()
                        .text_size(TEXT_SM)
                        .text_color(t.danger_text)
                        .child(crate::i18n::tr_dynamic(&e)),
                )
            })
            .child(
                div()
                    .pt(px(8.))
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        button("access-pw-cancel", t, BtnKind::Default, false)
                            .child(tr("Cancel"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(o) = org_mut(this) {
                                    o.ui.access_pw = None;
                                }
                                cx.notify();
                            }))
                            .test_support(),
                    )
                    .child(
                        button("access-pw-save", t, BtnKind::Primary, false)
                            .when(f.busy, |b| b.opacity(0.6))
                            .child(tr("Save"))
                            .on_click(cx.listener(|this, _, _, cx| this.save_access_password(cx)))
                            .test_support(),
                    ),
            );
        return Some(overlay(t, body));
    }
    if let Some(f) = &o.ui.delete_org {
        let members = st.map_or(0, |x| x.members.iter().filter(|m| !m.revoked).count());
        let spaces = st.map_or(0, |x| x.spaces.len());
        let typed = f.name.read(cx).value().trim() == o.m.name.trim();
        let export = f.export;
        let body = card(t, tr("Delete the server?"))
            .child(
                div()
                    .id("del-org-what")
                    .text_color(t.text_muted)
                    .tooltip(tip(tr(if o.local.host_dir.is_some() || o.m.in_folder() {
                        "Its members' copies are removed at their next sync"
                    } else {
                        "Server on another machine: its data is erased, the program stays installed there"
                    })))
                    .child(crate::trf!(
                        "{}, {} members, {} spaces. It cannot be undone.",
                        o.m.name,
                        members,
                        spaces
                    )),
            )
            .child(
                div()
                    .id("del-org-export")
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .h(px(32.))
                    .cursor_pointer()
                    .child(check_box(t, export))
                    .child(
                        div()
                            .id("del-org-export-l")
                            .child(tr("Export the spaces first"))
                            .tooltip(tip(tr(
                                "A backup of the organization, sealed, kept on this computer",
                            ))),
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(d) = org_mut(this).and_then(|o| o.ui.delete_org.as_mut()) {
                            d.export = !d.export;
                        }
                        cx.notify();
                    }))
                    .test_support(),
            )
            .child(field(
                t,
                &crate::trf!("Type {} to confirm", o.m.name),
                Input::new(&f.name).id("del-org-name"),
            ))
            .when_some(f.error.clone(), |d, e| {
                d.child(
                    div()
                        .text_size(TEXT_SM)
                        .text_color(t.danger_text)
                        .child(crate::i18n::tr_dynamic(&e)),
                )
            })
            .child(
                div()
                    .pt(px(8.))
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        button("del-org-cancel", t, BtnKind::Default, false)
                            .when(f.busy, |b| b.opacity(0.45))
                            .child(tr("Cancel"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(o) = org_mut(this)
                                    && !o.ui.delete_org.as_ref().is_some_and(|d| d.busy)
                                {
                                    o.ui.delete_org = None;
                                }
                                cx.notify();
                            }))
                            .test_support(),
                    )
                    .child(
                        button("del-org-go", t, BtnKind::Danger, false)
                            .when(!typed || f.busy, |b| b.opacity(0.45))
                            .child(if f.busy {
                                tr("Deleting…")
                            } else {
                                tr("Delete server")
                            })
                            .when(typed && !f.busy, |b| {
                                b.on_click(
                                    cx.listener(|this, _, _, cx| this.org_delete_server(cx)),
                                )
                            })
                            .test_support(),
                    ),
            );
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
    let valued = |v: &str, window: &mut Window, cx: &mut Context<CleisApp>| {
        let v = v.to_owned();
        let st = cx.new(|cx| InputState::new(window, cx));
        st.update(cx, |i, cx| i.set_value(v, window, cx));
        st
    };
    let sftp = SftpForm {
        host: make("files.example-studio.lan", window, cx),
        port: valued("22", window, cx),
        user: make("lea", window, cx),
        password: cx.new(|cx| InputState::new(window, cx).masked(true)),
        path: make("/home/lea/cleis", window, cx),
    };
    let sftp_password = cx.new(|cx| InputState::new(window, cx).masked(true));
    let nas = NasForm {
        address: make("nas.example-studio.lan", window, cx),
        port: valued("22", window, cx),
        user: valued("admin", window, cx),
        password: cx.new(|cx| InputState::new(window, cx).masked(true)),
        found: None,
        step: None,
    };
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
            nas: Some(nas),
            sftp: Some(sftp),
            sftp_password: Some(sftp_password),
            ..Default::default()
        },
        subs,
    )
}

impl CleisApp {
    /// Inputs of the organization screens (they need the window).
    pub fn ensure_org_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // D-79 (mockup 36h): the PIN boxes of each organization that asks it; the shown one
        // takes the keyboard the first time it appears.
        let shown = self.session().and_then(|s| pin_shown(s).map(|o| o.m.key()));
        let Some(s) = self.session() else { return };
        for o in s.all_orgs_mut() {
            let here = shown.as_deref() == Some(o.m.key().as_str());
            let Some(p) = o.pin.as_mut() else { continue };
            if p.locked {
                continue;
            }
            match p.with {
                AccessWith::Pin => {
                    if p.boxes.is_none() {
                        p.boxes = Some(super::pin::PinBoxes::new(usize::from(p.len), cx));
                        p.focused = false;
                    }
                    if here
                        && !p.focused
                        && let Some(b) = p.boxes.as_ref()
                    {
                        window.focus(&b.focus, cx);
                        p.focused = true;
                    }
                }
                // Mockup 42g: one field; Enter opens.
                AccessWith::Password => {
                    if p.field.is_none() {
                        let f = cx.new(|cx| InputState::new(window, cx).masked(true));
                        let key = o.m.key();
                        let sub = cx.subscribe_in(
                            &f,
                            window,
                            move |this, _, ev: &gpui_kit::component::input::InputEvent, _, cx| {
                                if matches!(
                                    ev,
                                    gpui_kit::component::input::InputEvent::PressEnter { .. }
                                ) {
                                    this.in_org(&key, |app| app.org_pin_submit(cx));
                                }
                            },
                        );
                        o.ui.subs.push(sub);
                        p.field = Some(f);
                        p.focused = false;
                    }
                    if here
                        && !p.focused
                        && let Some(f) = p.field.as_ref()
                    {
                        f.update(cx, |i, cx| i.focus(window, cx));
                        p.focused = true;
                    }
                }
                AccessWith::Hello => {}
            }
        }
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
        if let Some(j) = self.join_form_mut()
            && let (Some(fill), Some(a)) = (j.address_fill.take(), j.address.clone())
        {
            a.update(cx, |i, cx| i.set_value(fill, window, cx));
        }
        let Some(s) = self.session() else { return };
        let joining = s.org.is_none() || s.settings_page == "Add an organization";
        if joining && s.join.address.is_none() {
            // D-101: you, as your profile says (name, e-mail), when joining or creating.
            let me = super::profile::effective(s);
            let (mut join, subs) = join_form(window, cx);
            for (field, v) in [
                (join.name.clone(), me.name.clone()),
                (join.email.clone(), me.email.clone()),
                (join.creator.clone(), me.name.clone()),
            ] {
                if let Some(f) = field.filter(|_| !v.is_empty()) {
                    f.update(cx, |i, cx| i.set_value(v, window, cx));
                }
            }
            let Some(s) = self.session() else { return };
            join.elsewhere = s.join.elsewhere;
            join.place = s.join.place;
            join.folder = s.join.folder.clone();
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
    /// "Create" / "Install and create" (D-78): checks the boxes, then creates where chosen.
    pub fn create_org_clicked(&mut self, cx: &mut Context<Self>) {
        let Some(j) = self.join_form_mut() else {
            return;
        };
        if j.busy {
            return;
        }
        let read = |e: &Option<Entity<InputState>>, cx: &Context<Self>| {
            e.as_ref()
                .map(|i| i.read(cx).value().trim().to_owned())
                .unwrap_or_default()
        };
        let (org, creator, place) = (j.org_name.clone(), j.creator.clone(), j.place);
        let n = read(&org, cx);
        let me = read(&creator, cx);
        let missing = if n.is_empty() {
            Some(tr("Name the organization"))
        } else if me.is_empty() {
            Some(tr("Enter your name"))
        } else {
            None
        };
        if let Some(why) = missing {
            if let Some(j) = self.join_form_mut() {
                j.error = Some(why.into());
            }
            cx.notify();
            return;
        }
        if self.demo {
            self.toast(tr("Not in the demo"), cx);
            return;
        }
        match place {
            OrgPlace::Here => self.org_create_here(n, me, cx),
            OrgPlace::Nas | OrgPlace::Linux => self.org_create_on_nas(n, me, cx),
            OrgPlace::None => {}
            OrgPlace::Folder => {
                let Some((kind, folder)) = self.join_form_mut().and_then(|j| j.folder.clone())
                else {
                    return;
                };
                if kind != 3 {
                    self.org_create_in_folder(n, me, folder, None, cx);
                    return;
                }
                let Some(f) = self.join_form_mut().and_then(|j| j.sftp.as_ref()) else {
                    return;
                };
                let v = |e: &Entity<InputState>| e.read(cx).value().trim().to_owned();
                let place = crate::folder_store::SftpPlace {
                    host: v(&f.host),
                    port: v(&f.port).parse().unwrap_or(22),
                    user: v(&f.user),
                    path: format!("/{}", v(&f.path).trim_start_matches('/')),
                };
                let password = f.password.read(cx).value().to_string();
                if place.host.is_empty() || place.user.is_empty() {
                    if let Some(j) = self.join_form_mut() {
                        j.error = Some(tr("Enter the server and the user").into());
                    }
                    cx.notify();
                    return;
                }
                let secret = crate::org::SftpSecret {
                    secret: crate::remote::Secret {
                        token: password,
                        ..Default::default()
                    },
                    host_key: String::new(),
                };
                self.org_create_in_folder(
                    n,
                    me,
                    std::path::PathBuf::from(place.url()),
                    Some(secret),
                    cx,
                );
            }
        }
    }

    /// Join: the folder of an organization, in the address box.
    pub fn pick_join_folder(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(gpui_kit::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(tr("Choose folder").into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = rx.await
                && let Some(p) = paths.into_iter().next()
            {
                this.update(cx, |app, cx| {
                    if let Some(j) = app.join_form_mut() {
                        j.address_fill = Some(p.display().to_string());
                    }
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    /// Shared folder: the folder of SharePoint, Google shared drive or the NAS share chosen.
    pub fn pick_org_folder(&mut self, kind: usize, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(gpui_kit::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(tr("Choose folder").into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = rx.await
                && let Some(p) = paths.into_iter().next()
            {
                this.update(cx, |app, cx| {
                    if let Some(j) = app.join_form_mut() {
                        j.folder = Some((kind, p));
                        j.place = OrgPlace::Folder;
                        j.error = None;
                    }
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

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
        let folder = std::path::Path::new(address.trim()).join(crate::folder_org::ORG_FILE);
        if Invitation::parse(&code).is_none()
            || server_address(&address).is_none()
            || folder.is_file()
        {
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
        // D-78: a shared folder holding an organization, in the place of a server address.
        let folder = std::path::PathBuf::from(address.trim());
        if folder.join(crate::folder_org::ORG_FILE).is_file() {
            self.org_join_folder(folder, None, code, name, email, cx);
            cx.notify();
            return;
        }
        // An organization's folder on an SSH server: `sftp://user@host:port/path`.
        if crate::folder_store::SftpPlace::parse(address.trim()).is_some() {
            let password = s
                .join
                .sftp_password
                .as_ref()
                .map(|p| p.read(cx).value().to_string())
                .unwrap_or_default();
            let secret = crate::org::SftpSecret {
                secret: crate::remote::Secret {
                    token: password,
                    ..Default::default()
                },
                host_key: String::new(),
            };
            self.org_join_folder(folder, Some(secret), code, name, email, cx);
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
            secret: None,
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
                j.place = OrgPlace::Nas;
                j.elsewhere = true;
            }
        }
        // Mockups 47b, 47c: Server, on the NAS or on this computer.
        "create-server" | "create-server-here" => {
            if let Some(j) = app.join_form_mut() {
                j.place = if m == "create-server" {
                    OrgPlace::Nas
                } else {
                    OrgPlace::Here
                };
            }
        }
        // D-78: Shared folder, a SharePoint library chosen (mockup 36i).
        "create-folder" => {
            if let Some(j) = app.join_form_mut() {
                j.place = OrgPlace::Folder;
                j.folder = Some((
                    0,
                    std::path::PathBuf::from("C:\\Users\\lea\\Example Studio\\Cleis"),
                ));
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
        // Mockups 37b, 37f, 37m, 37n, 37o (captures).
        "rename-folder" => {
            let id = app
                .session()
                .and_then(|s| s.spaces.iter().find(|x| x.name == "Banking").map(|x| x.id));
            if let Some(id) = id {
                app.open_folder_rename(id, window, cx);
            }
        }
        "pin-setup" => app.open_pin_setup(crate::opening::DEFAULT_PIN_LEN, window, cx),
        // Mockup 40d: signed in to GitHub, the account's menu open (shown only, nothing sent).
        "account" => {
            let path = app.path.clone();
            if let Some(s) = app.session() {
                s.sync.provider = crate::sync::Provider::GitHub;
                s.sync.place = Some(path);
                s.sync.last_sync = Some(cleis_core::now() - 120);
                s.sync.remote = Some(crate::remote::RemotePlace::GitHub {
                    repo: "lea-example/cleis-vault".into(),
                    path: "vault.cleis".into(),
                    branch: "main".into(),
                    api: "https://api.github.com".into(),
                });
                s.account_menu = true;
                // Shown only: nothing is sent from a screenshot.
                s.sync.auto = false;
            }
        }
        // D-104: signed in with Microsoft or Google, the vault in the account (shown only).
        "onedrive" | "gdrive" => {
            let path = app.path.clone();
            if let Some(s) = app.session() {
                let (provider, place) = if m == "onedrive" {
                    (
                        crate::sync::Provider::OneDrive,
                        crate::remote::RemotePlace::OneDrive {
                            path: crate::screens::cloud::VAULT_FILE.into(),
                            api: "https://graph.microsoft.com/v1.0".into(),
                            account: "lea@example-studio.com".into(),
                        },
                    )
                } else {
                    (
                        crate::sync::Provider::GoogleDrive,
                        crate::remote::RemotePlace::GoogleDrive {
                            name: crate::screens::cloud::VAULT_FILE.into(),
                            api: "https://www.googleapis.com".into(),
                            account: "lea@example-studio.com".into(),
                        },
                    )
                };
                s.sync.provider = provider;
                s.sync.place = Some(path);
                s.sync.last_sync = Some(cleis_core::now() - 120);
                s.sync.remote = Some(place);
                s.sync.auto = false;
            }
        }
        // D-104: Microsoft (or Google) chosen in Account, then its sign-in waiting for the browser.
        "pick-microsoft" | "pick-google" => {
            if let Some(s) = app.session() {
                s.account_pick = Some(if m == "pick-microsoft" {
                    crate::sync::Provider::OneDrive
                } else {
                    crate::sync::Provider::GoogleDrive
                });
            }
        }
        "cloud-microsoft" | "cloud-google" => {
            let c = if m == "cloud-microsoft" {
                crate::cloud::Cloud::Microsoft
            } else {
                crate::cloud::Cloud::Google
            };
            if let Some(s) = app.session() {
                s.account_pick = Some(c.provider());
            }
            app.cloud = Some(super::cloud::CloudSignIn {
                cloud: c,
                url: Some("https://login.example-studio.com/authorize".into()),
                cancel: None,
                error: None,
                finishing: false,
                round: 1,
            });
        }
        // Mockup 46f: Spaces, Design unfolded.
        "space-members" => {
            if let Some(o) = org_mut(app) {
                o.ui.space_open = o.state.as_ref().and_then(|st| {
                    st.spaces
                        .iter()
                        .map(|sv| sv.id)
                        .find(|id| o.names.get(id).is_some_and(|n| n == "Design"))
                });
            }
        }
        // Mockup 46d: the Website column, its open button on the selected row.
        "website" | "recovery" => {
            if let Some(s) = app.session() {
                for c in s.columns.iter_mut() {
                    if c.id.0 == m {
                        c.visible = true;
                    }
                    // Mockup 47f: room for Recovery keys.
                    if m == "recovery" && c.id.0 == "yearly_cost" {
                        c.visible = false;
                    }
                }
            }
        }
        // Mockup 46c: the profile edited under its picture.
        "profile-edit" => app.open_profile_edit(window, cx),
        // Mockup 46a: the profile's menu.
        "profile-menu" => {
            if let Some(s) = app.session() {
                s.account_menu = true;
            }
        }
        // Mockups 37n, 41a, 41b: the code to type, then the steps on GitHub (shown only).
        "github" | "github-create" | "github-install" => {
            let signed = super::github::Signed {
                login: "lea-example".into(),
                token: zeroize::Zeroizing::new(String::new()),
                refresh: zeroize::Zeroizing::new(String::new()),
                expires: 0,
                hosts: app.github_hosts.clone(),
            };
            app.github = Some(super::github::GitHubSignIn {
                code: Some(crate::github_login::DeviceCode {
                    device_code: String::new(),
                    user_code: "WDJB-MJHT".into(),
                    verification_uri: "https://github.com/login/device".into(),
                    expires_in: 900,
                    interval: 5,
                }),
                setup: (m != "github").then(|| super::github::Setup {
                    signed,
                    made: m == "github-install",
                    installing: m == "github-install",
                    restoring: false,
                }),
                ..Default::default()
            });
        }
        // Mockup 37q: Security policy › Organization PIN › Change.
        "org-pin-setup" => {
            if let Some(st) = app
                .session()
                .and_then(|s| s.org.as_mut())
                .and_then(|o| o.state.as_mut())
            {
                st.policy.access.on = true;
                st.policy.access.with = AccessWith::Pin;
            }
            app.open_org_pin_setup(window, cx);
        }
        "ctx-password" => {
            if let Some(s) = app.session()
                && let Some(id) = s.selected
            {
                s.ctx = Some(super::ctx::CtxMenu {
                    at: gpui_kit::point(gpui_kit::px(1200.), gpui_kit::px(300.)),
                    target: super::ctx::CtxTarget::PasswordField(id),
                    sub: None,
                });
            }
        }
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
        | "ctx-trash" | "confirm-transfer" | "confirm-remove" => {
            use super::ctx::{CtxMenu, CtxTarget, Sub};
            use gpui_kit::point;
            let Some(s) = app.session() else { return };
            let member = s
                .org
                .as_ref()
                .and_then(|o| o.state.as_ref())
                .and_then(|st| st.members.get(1).map(|m| m.id));
            // The void menus are opened with nothing selected (the panel on the right closed).
            if matches!(m, "ctx-list" | "ctx-sidebar" | "ctx-trash") {
                s.selected = None;
            }
            let (target, at, sub) = match m {
                "ctx-entry" => {
                    // D-88: the row right-clicked is outlined, not selected (the panel stays closed).
                    let id = s.visible.get(1).map(|i| s.entries[*i].id);
                    s.selected = None;
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
                    s.selected_contact = None;
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
                // Mockup 47e: the Trash's menu.
                "ctx-trash" => (Some(CtxTarget::Trash), point(px(40.), px(760.)), None),
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
        "contact-photo" => {
            if let Some(s) = app.session() {
                s.tab = crate::app::Tab::Contacts;
                s.selected_contact = s.contacts.iter().find(|c| c.photo.is_some()).map(|c| c.id);
                s.contacts_ui.menu = Some(super::contacts::ContactMenu::Photo);
            }
        }
        // D-79: the organization asks its PIN (mockup 36h).
        "org-pin" => {
            if let Some(s) = app.session() {
                let key = s.org.as_ref().map(|o| o.m.key());
                if let (Some(o), Some(k)) = (s.org.as_mut(), key) {
                    o.pin = Some(crate::org::PinPrompt::new(crate::org::PIN_DEFAULT_LEN));
                    s.filter = crate::app::Filter::Org(crate::app::org_hash(&k));
                }
                crate::org::hide_org_entries(s);
            }
        }
        "policy-pin" => {
            if let Some(st) = app
                .session()
                .and_then(|s| s.org.as_mut())
                .and_then(|o| o.state.as_mut())
            {
                st.policy.access.on = true;
                st.policy.access.with = AccessWith::Pin;
            }
        }
        // Mockups 42b, 42g (D-98): a password set by the admin; asked of a member.
        "policy-access" => {
            if let Some(st) = app
                .session()
                .and_then(|s| s.org.as_mut())
                .and_then(|o| o.state.as_mut())
            {
                st.policy.access.on = true;
                st.policy.access.with = AccessWith::Password;
                st.policy.access.idle_mins = Some(15);
                st.pin_set = true;
            }
        }
        "org-password" => {
            if let Some(s) = app.session() {
                let key = s.org.as_ref().map(|o| o.m.key());
                if let (Some(o), Some(k)) = (s.org.as_mut(), key) {
                    o.pin = Some(crate::org::PinPrompt::asking(
                        AccessWith::Password,
                        crate::org::PIN_DEFAULT_LEN,
                        false,
                    ));
                    s.filter = crate::app::Filter::Org(crate::app::org_hash(&k));
                }
                crate::org::hide_org_entries(s);
            }
        }
        "delete-org" => {
            app.open_delete_org(window, cx);
        }
        // Mockup 44a: Contacts, the organization's section.
        "contacts-org" => {
            if let Some(s) = app.session()
                && let Some(k) = s.org.as_ref().map(|o| o.m.key())
            {
                s.tab = crate::app::Tab::Contacts;
                s.contacts_ui.filter = super::contacts::ContactFilter::Org(k);
            }
        }
        // Mockup 43a: Health › Change on the first item whose password is the problem.
        "health-change" => {
            let id = app.session().and_then(|s| {
                let (f, _) = cleis_core::health::scan(
                    &s.entries,
                    &cleis_core::health::Policy::default(),
                    cleis_core::now(),
                );
                f.into_iter()
                    .find(|f| {
                        f.issues
                            .iter()
                            .any(|i| matches!(i, cleis_core::health::Issue::Weak))
                    })
                    .map(|f| f.entry)
            });
            if let Some(id) = id {
                if let Some(s) = app.session() {
                    s.tab = crate::app::Tab::Health;
                }
                app.open_health_change(id, window, cx);
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
        w.choice = Some(super::welcome::Start::Org);
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
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(head(t, tr("Profile")))
        .children(profile_sections(s, t, cx))
}

/// What the organization knows of you (was Settings › Profile): in Settings › Account since
/// 27/11/2026 (mockup 39b), one place for the account.
pub fn profile_sections(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Vec<Div> {
    let Some(o) = s.org.as_ref() else {
        return vec![];
    };
    let Some(me) = o.me() else {
        return vec![];
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
                .child(super::profile::member(t, me, 40.))
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
    vec![
        identity,
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
                                .tooltip(tip(tr("Asked by the server at sign-in; two-factor authentication above is this computer's own")))
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
    ]
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

/// What a box of the rights does when clicked.
type RightAct = Box<dyn Fn(&mut CleisApp, &mut Context<CleisApp>)>;

/// Mockup 46f: a space unfolded, each member and what each may do in it. The role set on the
/// server is the ceiling: a space takes a right away, never gives one (a box the role does not
/// allow is greyed, its tip says why). Administrators: everything, greyed.
fn space_members(
    o: &OrgSession,
    st: &State,
    sv: &SpaceView,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    let space = sv.id;
    let manage = o.can(Perm::Manage);
    let i_super = st.me.role == SUPER_ADMIN;
    let locked = sv.reserved && !i_super;
    let grid = |d: Div| d.flex().items_center().gap(px(12.)).pl(px(56.)).pr(px(16.));
    let col = |w: f32| div().w(px(w)).flex().justify_center();
    let mut panel = div()
        .id(SharedString::from(format!("space-members-{space}")))
        .flex()
        .flex_col()
        .pb(px(8.))
        .border_b_1()
        .border_color(t.line)
        .child(
            grid(div())
                .h(px(28.))
                .text_size(TEXT_XS)
                .text_color(t.text_muted)
                .font_weight(W_MEDIUM)
                .child(div().w(px(28.)))
                .child(div().flex_1().child(tr("Member")))
                .child(col(96.).child(tr("Access")))
                .child(
                    col(96.)
                        .id("h-edit")
                        .child(tr("Edit"))
                        .tooltip(tip(tr("Add, change, move to the trash"))),
                )
                .child(
                    col(128.)
                        .id("h-show")
                        .child(tr("Show passwords"))
                        .tooltip(tip(tr("Without: copy and fill only"))),
                ),
        );
    for m in st.members.iter().filter(|m| !m.revoked) {
        let mid = m.id;
        let admin = is_admin_role(&m.role);
        let role = st.roles.iter().find(|r| r.id == m.role);
        let role_name = role.map(|r| r.name.clone()).unwrap_or_default();
        let has = admin || m.spaces.contains(&space);
        let limit = m.limits.get(&space).copied().unwrap_or_default();
        let current = m.spaces.clone();
        // One box: its state, whether it can be clicked, and why not.
        let cell = |key: &str,
                    w: f32,
                    on: bool,
                    why: Option<String>,
                    act: Option<RightAct>,
                    cx: &mut Context<CleisApp>| {
            let off = act.is_none();
            col(w).child(
                div()
                    .id(SharedString::from(format!("sm-{key}-{space}-{mid}")))
                    .when(!off, |d| d.cursor_pointer())
                    .child(check_box(t, on).when(off, |d| d.opacity(0.4)))
                    .when_some(why, |d, w| d.tooltip(tip(w)))
                    .when_some(act, |d, f| {
                        d.on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            f(this, cx)
                        }))
                    })
                    .test_support(),
            )
        };
        let editable = manage && !admin && !locked;
        let why_locked = || {
            if admin {
                Some(tr("Administrators see every space").to_owned())
            } else if locked {
                Some(tr("Reserved: a Super Admin changes its members").to_owned())
            } else {
                None
            }
        };
        let access = cell(
            "access",
            96.,
            has,
            why_locked(),
            editable.then(|| -> RightAct {
                let current = current.clone();
                Box::new(move |app, cx| {
                    let mut next = current.clone();
                    if has {
                        next.retain(|x| *x != space);
                    } else {
                        next.push(space);
                    }
                    app.org_member(
                        MemberChange {
                            member: mid,
                            spaces: Some(next),
                            ..Default::default()
                        },
                        cx,
                    );
                })
            }),
            cx,
        );
        // Edit and Show passwords: within the role, with access.
        let right = |key: &str,
                     w: f32,
                     p: Perm,
                     on_now: bool,
                     flip: fn(&mut SpaceLimit),
                     cx: &mut Context<CleisApp>| {
            let allowed = role.is_some_and(|r| r.can(p));
            let (on, why) = if admin {
                (true, why_locked())
            } else if !allowed {
                (
                    false,
                    Some(crate::trf!("Not allowed by the role {}", role_name)),
                )
            } else if !has {
                (false, Some(tr("No access").to_owned()))
            } else {
                (on_now, why_locked())
            };
            let act = (editable && allowed && has).then(|| -> RightAct {
                Box::new(move |app, cx| {
                    let mut l = limit;
                    flip(&mut l);
                    app.org_member(
                        MemberChange {
                            member: mid,
                            limits: Some([(space, l)].into_iter().collect()),
                            ..Default::default()
                        },
                        cx,
                    );
                })
            });
            cell(key, w, on, why, act, cx)
        };
        let edit = right(
            "edit",
            96.,
            Perm::Edit,
            !limit.no_edit,
            |l| l.no_edit = !l.no_edit,
            cx,
        );
        let show = right(
            "show",
            128.,
            Perm::See,
            !limit.no_show,
            |l| l.no_show = !l.no_show,
            cx,
        );
        panel = panel.child(
            grid(div())
                .h(px(40.))
                .child(super::profile::member(t, m, 28.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(m.name.clone()),
                )
                .child(access)
                .child(edit)
                .child(show),
        );
    }
    panel.test_support()
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
                    .child(match st.members.iter().find(|x| x.id == *m) {
                        Some(x) => super::profile::member(t, x, 22.),
                        None => avatar(t, &initials(&o.member_name(*m)), 22.),
                    }),
            );
        }
        faces = faces.child(
            div()
                .ml(px(8.))
                .text_color(t.text_muted)
                .child(sv.members.len().to_string()),
        );
        let open = o.ui.space_open == Some(id);
        let hover = t.surface_hover;
        table = table.child(
            cols(div())
                .id(SharedString::from(format!("space-row-{id}")))
                .h(px(44.))
                .border_b_1()
                .border_color(t.line)
                .cursor_pointer()
                .when(open, |d| d.bg(hover))
                .hover(move |d| d.bg(hover))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(o) = org_mut(this) {
                        o.ui.space_open = if o.ui.space_open == Some(id) {
                            None
                        } else {
                            Some(id)
                        };
                    }
                    cx.notify();
                }))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            gpui_kit::svg()
                                .path("icons/chevron.svg")
                                .size(px(14.))
                                .flex_none()
                                .text_color(t.text_muted)
                                .with_transformation(gpui_kit::Transformation::rotate(
                                    gpui_kit::radians(if open {
                                        0.
                                    } else {
                                        -std::f32::consts::FRAC_PI_2
                                    }),
                                )),
                        )
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
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.org_reserve(id, !reserved, cx)
                        }))
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
                                cx.stop_propagation();
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
                                cx.stop_propagation();
                                if let Some(o) = org_mut(this) {
                                    o.ui.delete_space = Some(id);
                                }
                                cx.notify();
                            }))
                            .test_support(),
                        ),
                )
                .test_support(),
        );
        if open {
            table = table.child(space_members(o, st, sv, t, cx));
        }
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
            photo: None,
        };
        let (name, email) = (req.name.clone(), req.email.clone());
        self.org_request::<_, MemberView>(
            "/v1/me",
            req,
            move |app, r, cx| {
                if r.is_ok() {
                    // D-101: one profile in Cleis; the other organizations get it too.
                    app.update_prefs(
                        |p| {
                            p.profile.name = name;
                            p.profile.email = email;
                        },
                        cx,
                    );
                    if let Some(s) = app.session() {
                        for o in s.orgs_idle.iter_mut() {
                            o.profile_sent = false;
                        }
                    }
                    app.toast(tr("Saved"), cx);
                }
            },
            cx,
        );
    }
}

impl CleisApp {
    /// Security policy › Accessing › Set / Change (D-98): PIN boxes (37q) or the password typed
    /// twice (42f).
    pub fn open_access_setup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(with) =
            org_mut(self).and_then(|o| o.state.as_ref().map(|st| st.policy.access.with))
        else {
            return;
        };
        match with {
            AccessWith::Pin => self.open_org_pin_setup(window, cx),
            AccessWith::Password => {
                let first = cx.new(|cx| InputState::new(window, cx).masked(true));
                let confirm = cx.new(|cx| InputState::new(window, cx).masked(true));
                first.update(cx, |i, cx| i.focus(window, cx));
                if let Some(o) = org_mut(self) {
                    o.ui.access_pw = Some(AccessPwForm {
                        first,
                        confirm,
                        error: None,
                        busy: false,
                    });
                }
                cx.notify();
            }
            AccessWith::Hello => {}
        }
    }

    /// The password typed twice (42f): the same, long enough, then kept (only its digest goes).
    pub fn save_access_password(&mut self, cx: &mut Context<Self>) {
        let Some(o) = org_mut(self) else { return };
        let Some(f) = o.ui.access_pw.as_mut() else {
            return;
        };
        if f.busy {
            return;
        }
        let a = zeroize::Zeroizing::new(f.first.read(cx).value().to_string());
        let b = zeroize::Zeroizing::new(f.confirm.read(cx).value().to_string());
        if *a != *b {
            f.error = Some(tr("The two passwords differ").into());
            cx.notify();
            return;
        }
        if a.chars().count() < ACCESS_PASSWORD_MIN {
            f.error = Some(crate::trf!("At least {} characters", ACCESS_PASSWORD_MIN).into());
            cx.notify();
            return;
        }
        f.busy = true;
        f.error = None;
        cx.notify();
        self.org_set_access(a, cx);
    }

    /// Server › Delete (42j): the dialog, its name box focused.
    pub fn open_delete_org(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = cx.new(|cx| InputState::new(window, cx));
        name.update(cx, |i, cx| i.focus(window, cx));
        let sub = cx.observe(&name, |_, _, cx| cx.notify());
        if let Some(o) = org_mut(self) {
            o.ui.subs.push(sub);
            o.ui.delete_org = Some(DeleteOrgForm {
                name,
                export: true,
                busy: false,
                error: None,
            });
        }
        cx.notify();
    }

    /// Picks a picture for the logo: the centred square at 128 px, as PNG.
    pub fn org_pick_logo(&mut self, cx: &mut Context<Self>) {
        if self.demo {
            self.toast(tr("Not in the demo"), cx);
            return;
        }
        let rx = cx.prompt_for_paths(gpui_kit::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: None,
        });
        let Some(key) = self.active_org_key() else {
            return;
        };
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else {
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            let png = cx
                .background_spawn(async move {
                    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
                    crate::org::logo_png(&bytes)
                })
                .await;
            this.update(cx, |app, cx| {
                app.in_org(&key, |app| match png {
                    Ok(png) => app.org_set_logo(Some(png), cx),
                    Err(e) => app.toast(crate::i18n::tr_dynamic(&e), cx),
                });
            })
            .ok();
        })
        .detach();
    }
}
