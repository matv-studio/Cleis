//! Trash (vault sidebar): deleted items, restorable; deleting for good asks first, since the
//! file erases them for real (`secure_delete`).

use crate::app::{CleisApp, Confirm, ConfirmAction, Session};
use crate::theme::*;
use crate::ui::*;
use gpui_kit::TestSupportExt;
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, SharedString, div, px};

pub fn page(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let mut items: Vec<_> = s.trash.iter().collect();
    items.sort_by_key(|e| cleis_core::text::fold(&e.name));
    let mut list = div().flex().flex_col();
    for (i, e) in items.iter().enumerate() {
        let id = e.id;
        let name = e.name.clone();
        list = list.child(
            div()
                .h(ROW_H)
                .px(px(12.))
                .flex()
                .items_center()
                .gap(px(12.))
                .border_b_1()
                .border_color(t.line)
                .child(site_icon(t, e.name.chars().next().unwrap_or('?'), 20.))
                .child(div().flex_1().min_w_0().child(e.name.clone()))
                .child(
                    div()
                        .w(px(220.))
                        .text_color(t.text_muted)
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(e.username.clone()),
                )
                .child(
                    button(
                        SharedString::from(format!("restore-{i}")),
                        t,
                        BtnKind::Default,
                        true,
                    )
                    .child("Restore")
                    .on_click(cx.listener(move |this, _, _, cx| this.restore_entry(id, cx)))
                    .test_support(),
                )
                .child(
                    button(
                        SharedString::from(format!("purge-{i}")),
                        t,
                        BtnKind::DangerGhost,
                        true,
                    )
                    .child("Delete")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.ask(
                            Confirm {
                                title: format!("Delete “{name}” for good?").into(),
                                body: "It is erased from this computer and cannot be restored."
                                    .into(),
                                button: "Delete".into(),
                                action: ConfirmAction::PurgeEntry(id),
                            },
                            cx,
                        )
                    }))
                    .test_support(),
                ),
        );
    }
    let n = items.len();
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(44.))
                .flex_none()
                .px(px(12.))
                .flex()
                .items_center()
                .gap(px(8.))
                .border_b_1()
                .border_color(t.line)
                .child(div().flex_1().font_weight(W_MEDIUM).child("Trash"))
                .when(n > 0, |d| {
                    d.child(
                        button("empty-trash", t, BtnKind::DangerGhost, true)
                            .child("Empty trash")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.ask(
                                    Confirm {
                                        title: format!(
                                            "Delete {n} item{} for good?",
                                            if n == 1 { "" } else { "s" }
                                        )
                                        .into(),
                                        body: "They are erased from this computer and cannot be restored."
                                            .into(),
                                        button: "Empty trash".into(),
                                        action: ConfirmAction::EmptyTrash,
                                    },
                                    cx,
                                )
                            }))
                            .test_support(),
                    )
                }),
        )
        .child(
            div()
                .id("trash-list")
                .flex_1()
                .overflow_y_scroll()
                .child(list)
                .when(n == 0, |d| {
                    d.child(
                        div()
                            .p(px(24.))
                            .text_color(t.text_muted)
                            .child("The trash is empty"),
                    )
                }),
        )
}

/// Confirmation over the window (charter: the destructive button only appears here).
pub fn dialog(c: &Confirm, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(t.overlay)
        .child(
            dialog_box(t)
                .id("confirm")
                .child(
                    div()
                        .text_size(TEXT_LG)
                        .font_weight(W_MEDIUM)
                        .child(c.title.clone()),
                )
                .child(div().text_color(t.text_muted).child(c.body.clone()))
                .child(
                    div()
                        .pt(px(8.))
                        .flex()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            // Mockup 34: Cancel holds the focus, Enter or Escape cancels.
                            button("confirm-cancel", t, BtnKind::Default, false)
                                .border_2()
                                .border_color(t.focus)
                                .child("Cancel")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(s) = this.session() {
                                        s.confirm = None;
                                    }
                                    cx.notify();
                                }))
                                .test_support(),
                        )
                        .child(
                            button("confirm-ok", t, BtnKind::Danger, false)
                                .child(c.button.clone())
                                .on_click(cx.listener(|this, _, _, cx| this.confirmed(cx)))
                                .test_support(),
                        ),
                ),
        )
}

impl CleisApp {
    pub fn ask(&mut self, c: Confirm, cx: &mut Context<Self>) {
        if let Some(s) = self.session() {
            s.confirm = Some(c);
        }
        cx.notify();
    }

    /// Asks before an organization or sync action (audit U-1, mockups 34 and 34b).
    pub fn ask_action(&mut self, action: ConfirmAction, cx: &mut Context<Self>) {
        use ConfirmAction as A;
        let Some(s) = self.session() else { return };
        let member = |id: cleis_core::MemberId| {
            s.org
                .as_ref()
                .and_then(|o| o.state.as_ref())
                .and_then(|st| st.members.iter().find(|m| m.id == id))
                .map(|m| m.name.clone())
                .unwrap_or_default()
        };
        let org = s.org.as_ref().map(|o| o.m.name.clone()).unwrap_or_default();
        let (title, body, button) = match action {
            A::RemoveMember(id) => (
                format!("Remove {}?", member(id)),
                "Their access ends on every device. Re-encrypt the spaces they could open."
                    .to_owned(),
                "Remove",
            ),
            A::TransferSuper(id) => (
                format!("Transfer Super Admin to {}?", member(id)),
                "You become Admin.".to_owned(),
                "Transfer",
            ),
            A::MakeSuper(id) => (
                format!("Make {} Super Admin?", member(id)),
                "They can do everything, including taking this role from you.".to_owned(),
                "Make Super Admin",
            ),
            A::ResetStep(id) => (
                format!("Reset two-step verification for {}?", member(id)),
                "They are signed out and set it up again at their next sign-in.".to_owned(),
                "Reset",
            ),
            A::SignOutMember(id) => (
                format!("Sign {} out everywhere?", member(id)),
                "Every device asks them to sign in again.".to_owned(),
                "Sign out",
            ),
            A::RevokeInvite(id) => (
                format!(
                    "Revoke the invitation to {}?",
                    s.org
                        .as_ref()
                        .and_then(|o| o.state.as_ref())
                        .and_then(|st| st.invites.iter().find(|i| i.id == id))
                        .map(|i| i.email.clone())
                        .unwrap_or_default()
                ),
                "Its code stops working.".to_owned(),
                "Revoke",
            ),
            A::LeaveOrg => (
                format!("Leave {org}?"),
                "Its spaces are removed from this vault. Joining again needs a new invitation."
                    .to_owned(),
                "Leave",
            ),
            A::RotateKey => (
                "Rotate the server key?".to_owned(),
                "The new key is used after the server's next restart, 7 days from now.".to_owned(),
                "Rotate",
            ),
            A::DisconnectPlace => (
                format!("Disconnect {}?", s.sync.provider.name()),
                "This device stops syncing there. The copy already there is kept.".to_owned(),
                "Disconnect",
            ),
            _ => return,
        };
        s.confirm = Some(Confirm {
            title: title.into(),
            body: body.into(),
            button: button.into(),
            action,
        });
        cx.notify();
    }

    /// Closes the question without acting (Cancel, Escape, Enter).
    pub fn cancel_confirm(&mut self, cx: &mut Context<Self>) {
        if let Some(s) = self.session() {
            s.confirm = None;
        }
        cx.notify();
    }

    fn confirmed(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(c) = s.confirm.take() else { return };
        match c.action {
            ConfirmAction::PurgeEntry(id) => {
                if s.vault.purge_entry(id).is_ok() {
                    s.trash.retain(|e| e.id != id);
                    s.toast = Some("Deleted".into());
                }
            }
            ConfirmAction::DeleteContact(id) => {
                self.delete_contact(id);
            }
            ConfirmAction::RemoveMember(id) => self.org_revoke(id, cx),
            ConfirmAction::TransferSuper(id) => self.org_transfer(id, cx),
            ConfirmAction::MakeSuper(id) => self.org_member(
                cleis_core::org::MemberChange {
                    member: id,
                    role: Some(cleis_core::org::SUPER_ADMIN.into()),
                    ..Default::default()
                },
                cx,
            ),
            ConfirmAction::ResetStep(id) => self.org_reset_step(id, cx),
            ConfirmAction::SignOutMember(id) => self.org_sign_out_member(id, cx),
            ConfirmAction::RevokeInvite(id) => self.org_invite_action(id, false, cx),
            ConfirmAction::LeaveOrg => self.org_leave(cx),
            ConfirmAction::RotateKey => self.org_rotate(cx),
            ConfirmAction::DisconnectPlace => self.disconnect_sync(cx),
            ConfirmAction::EmptyTrash => {
                let ids: Vec<_> = s.trash.iter().map(|e| e.id).collect();
                let mut failed = 0;
                for id in ids {
                    match s.vault.purge_entry(id) {
                        Ok(_) => s.trash.retain(|e| e.id != id),
                        Err(_) => failed += 1,
                    }
                }
                s.toast = Some(if failed == 0 {
                    "Trash emptied".into()
                } else {
                    format!("{failed} item(s) could not be deleted").into()
                });
            }
        }
        cx.notify();
    }

    pub fn restore_entry(&mut self, id: cleis_core::EntryId, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        match s.vault.restore_entry(id) {
            Ok(_) => {
                if let Some(i) = s.trash.iter().position(|e| e.id == id) {
                    let e = s.trash.remove(i);
                    s.selected = Some(e.id);
                    s.entries.push(e);
                    s.refilter();
                }
                s.toast = Some("Restored".into());
            }
            Err(err) => s.toast = Some(format!("Not restored: {err}").into()),
        }
        cx.notify();
    }
}
