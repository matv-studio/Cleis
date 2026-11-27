//! The profile at the top right (D-101, mockups 40d, 46a–46c): its picture (the organization's
//! logo in its vault, D-98) always opens its menu, signed in to an account or not. "Edit profile"
//! opens a card under it: the photo (the picture is the button), the name and the e-mail, kept for
//! Cleis and sent to every organization, whose members list and activity log show them.

use crate::app::{CleisApp, Session, Tab};
use crate::i18n::tr;
use crate::theme::*;
use crate::ui::*;
use cleis_core::org::{MemberView, ProfileChange};
use cleis_core::settings::Profile;
use gpui_kit::TestSupportExt;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::prelude::*;
use gpui_kit::{
    AnyElement, Context, Div, Entity, PathPromptOptions, SharedString, Window, div, px,
};

/// Side of the profile photo (a 96 px JPEG weighs about 5 KB; the server keeps 24 KB at most).
const PHOTO_SIDE: u32 = 96;

/// The card of mockup 46c, open.
pub struct ProfileEdit {
    pub name: Entity<InputState>,
    pub email: Entity<InputState>,
    /// The photo chosen, or taken away (`None`).
    pub photo: Option<String>,
    pub error: Option<SharedString>,
}

/// The profile shown: the one set in Cleis, else what an organization or the account knows.
pub fn effective(s: &Session) -> Profile {
    let mut p = s.prefs.profile.clone();
    if p.name.trim().is_empty() {
        if let Some(m) = s.all_orgs().find_map(|o| o.me()) {
            p.name = m.name.clone();
            if p.email.is_empty() {
                p.email = m.email.clone();
            }
        } else if let Some(login) = super::account::github_login(s) {
            p.name = login;
        }
    }
    p
}

/// The round picture: the photo, the initials, or the person sign.
pub fn picture(t: &Tokens, p: &Profile, size: f32) -> Div {
    person(t, &p.initials(), p.photo.as_deref(), size)
}

/// A person's picture from initials and an optional photo (members, the profile).
pub fn person(t: &Tokens, initials: &str, photo: Option<&str>, size: f32) -> Div {
    match photo.and_then(photo_image) {
        Some(img) => div()
            .flex_none()
            .size(px(size))
            .rounded_full()
            .overflow_hidden()
            .border_1()
            .border_color(t.line_strong)
            .child(gpui_kit::img(img).size(px(size)).rounded_full()),
        None if initials.is_empty() => {
            avatar(t, "", size).child(icon("user", size * 0.55, t.text_muted))
        }
        None => avatar(t, initials, size),
    }
}

/// A member's picture: its photo (D-101) or its initials.
pub fn member(t: &Tokens, m: &MemberView, size: f32) -> Div {
    person(t, &crate::org::initials(&m.name), m.photo.as_deref(), size)
}

fn status(s: &Session) -> String {
    let synced = || match (&s.sync.last_error, s.sync.last_sync) {
        _ if s.sync.busy => tr("Syncing…").to_owned(),
        (Some(_), _) => tr("Not synced").to_owned(),
        (None, Some(ts)) => crate::trf!("Synced {}", super::sync::ago(ts)),
        (None, None) => tr("Never synced").to_owned(),
    };
    match super::account::github_login(s) {
        Some(login) => format!("@{login} · {}", synced()),
        None if s.sync.place.is_some() => synced(),
        None => tr("Local account").to_owned(),
    }
}

/// Mockups 46a–46c: the picture, its menu, the card editing the profile.
pub fn top_right(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> AnyElement {
    let p = effective(s);
    let signed = super::account::github_login(s).is_some();
    let synced = s.sync.place.is_some();
    let open = s.account_menu;
    let editing = s.profile_edit.is_some();
    let tip_text: SharedString = if p.name.is_empty() {
        tr("Profile").into()
    } else {
        p.name.clone().into()
    };
    let sep = || div().my(px(4.)).h(px(1.)).bg(t.line);
    let close = |this: &mut CleisApp| {
        if let Some(s) = this.session() {
            s.account_menu = false;
        }
    };
    let menu = (open && !editing).then(|| {
        let head = div()
            .flex()
            .items_center()
            .gap(px(10.))
            .px(px(8.))
            .pt(px(6.))
            .pb(px(10.))
            .child(picture(t, &p, 32.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .min_w_0()
                    .child(
                        div()
                            .font_weight(W_MEDIUM)
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(if p.name.is_empty() {
                                tr("Your profile").to_owned()
                            } else {
                                p.name.clone()
                            }),
                    )
                    .child(
                        div()
                            .text_size(TEXT_SM)
                            .text_color(t.text_muted)
                            .child(status(s)),
                    ),
            );
        let lock_keys = super::ctx::keys(&s.prefs.app_key("lock"));
        gpui_kit::deferred(
            menu_box(t)
                .id("account-menu")
                .top(px(32.))
                .right_0()
                .w(px(260.))
                .child(head)
                .child(sep())
                .child(
                    menu_item(
                        "acc-menu-profile",
                        t,
                        Some("user"),
                        tr("Edit profile"),
                        false,
                    )
                    .on_click(cx.listener(|this, _, window, cx| this.open_profile_edit(window, cx)))
                    .test_support(),
                )
                .child(if synced {
                    menu_item("acc-menu-sync", t, Some("sync"), tr("Sync now"), false)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            close(this);
                            this.sync_now(cx);
                        }))
                        .test_support()
                } else {
                    menu_item(
                        "acc-menu-signin",
                        t,
                        Some("cloud"),
                        tr("Sign in to sync…"),
                        false,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        close(this);
                        if let Some(s) = this.session() {
                            s.tab = Tab::Settings;
                            s.settings_page = "Account";
                        }
                        cx.notify();
                    }))
                    .test_support()
                })
                .child(
                    menu_item("acc-menu-account", t, Some("gear"), tr("Account"), false)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            close(this);
                            if let Some(s) = this.session() {
                                s.tab = Tab::Settings;
                                s.settings_page = "Account";
                            }
                            cx.notify();
                        }))
                        .test_support(),
                )
                .child(sep())
                .child(if signed {
                    menu_item("acc-menu-signout", t, Some("logout"), tr("Sign out"), false)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            close(this);
                            this.ask_action(crate::app::ConfirmAction::DisconnectPlace, cx)
                        }))
                        .test_support()
                } else {
                    menu_item("acc-menu-lock", t, Some("lock"), tr("Lock"), false)
                        .child(div().flex_1())
                        .child(
                            div()
                                .text_size(TEXT_SM)
                                .text_color(t.text_faint)
                                .child(lock_keys),
                        )
                        .on_click(cx.listener(move |this, _, window, cx| {
                            close(this);
                            this.lock(window, cx);
                        }))
                        .test_support()
                })
                .test_support(),
        )
        .with_priority(2)
    });
    let card = s.profile_edit.as_ref().map(|e| edit_card(s, e, t, cx));
    div()
        .relative()
        .child(
            div()
                .id("acc-avatar")
                .occlude()
                .no_drag()
                .cursor_pointer()
                .rounded_full()
                .when(open || editing, |d| d.border_1().border_color(t.focus))
                // D-98: in an organization's vault, its logo; the profile again elsewhere.
                .child(match super::org::org_in_view(s) {
                    Some(o) => {
                        org_logo(t, o.local.logo.as_deref(), &o.m.name, 24.).into_any_element()
                    }
                    None => picture(t, &p, 24.).into_any_element(),
                })
                .tooltip(move |window, cx| {
                    gpui_kit::component::tooltip::Tooltip::new(tip_text.clone()).build(window, cx)
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(s) = this.session() {
                        s.profile_edit = None;
                        s.account_menu = !s.account_menu;
                    }
                    cx.notify();
                }))
                .test_support(),
        )
        .when_some(menu, |d, m| d.child(m))
        .when_some(card, |d, c| d.child(c))
        .into_any_element()
}

/// Mockup 46c: the photo is the button (the upload sign under the pointer), then the name and the
/// e-mail; the organizations it goes to, by their logos.
fn edit_card(s: &Session, e: &ProfileEdit, t: &Tokens, cx: &mut Context<CleisApp>) -> AnyElement {
    let shown = Profile {
        name: e.name.read(cx).value().to_string(),
        email: String::new(),
        photo: e.photo.clone(),
    };
    let has_photo = e.photo.is_some();
    let veil = t.overlay;
    let photo = div()
        .id("profile-photo")
        .relative()
        .group("profile-photo")
        .cursor_pointer()
        .rounded_full()
        .child(picture(t, &shown, 56.))
        .child(
            div()
                .absolute()
                .inset_0()
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .invisible()
                .group_hover("profile-photo", move |s| s.visible().bg(veil))
                .child(icon("upload", 18., gpui_kit::white())),
        )
        .tooltip(tip(tr("Change the photo")))
        .on_click(cx.listener(|this, _, _, cx| this.pick_profile_photo(cx)))
        .test_support();
    let mut orgs = div().flex().items_center().gap(px(4.));
    let mut names = vec![];
    for o in s.all_orgs() {
        names.push(o.m.name.clone());
        orgs = orgs.child(org_logo(t, o.local.logo.as_deref(), &o.m.name, 20.));
    }
    let orgs_tip: SharedString = names.join(", ").into();
    let card = div()
        .w(px(320.))
        .p(px(16.))
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(
            div().flex().items_center().gap(px(12.)).child(photo).child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(div().font_weight(W_MEDIUM).child(tr("Profile")))
                    .when(!names.is_empty(), |d| {
                        d.child(
                            div()
                                .id("profile-orgs")
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_size(TEXT_SM)
                                .text_color(t.text_muted)
                                .child(tr("Shown to"))
                                .child(orgs)
                                .tooltip(tip(orgs_tip))
                                .test_support(),
                        )
                    })
                    .when(has_photo, |d| {
                        d.child(
                            div()
                                .id("profile-photo-remove")
                                .text_size(TEXT_SM)
                                .text_color(t.text_muted)
                                .cursor_pointer()
                                .hover(|s| s.underline())
                                .child(tr("Remove the photo"))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(e) =
                                        this.session().and_then(|s| s.profile_edit.as_mut())
                                    {
                                        e.photo = None;
                                    }
                                    cx.notify();
                                }))
                                .test_support(),
                        )
                    }),
            ),
        )
        .child(field(
            t,
            tr("Name"),
            Input::new(&e.name).id("profile-me-name"),
        ))
        .child(field(
            t,
            "E-mail",
            Input::new(&e.email).id("profile-me-email"),
        ))
        .when_some(e.error.clone(), |d, err| {
            d.child(
                div()
                    .text_size(TEXT_SM)
                    .text_color(t.danger_text)
                    .child(err),
            )
        })
        .child(
            div()
                .flex()
                .justify_end()
                .gap(px(8.))
                .child(
                    crate::ui::button("profile-me-cancel", t, BtnKind::Default, true)
                        .child(tr("Cancel"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(s) = this.session() {
                                s.profile_edit = None;
                            }
                            cx.notify();
                        }))
                        .test_support(),
                )
                .child(
                    crate::ui::button("profile-me-save", t, BtnKind::Primary, true)
                        .child(tr("Save"))
                        .on_click(cx.listener(|this, _, _, cx| this.save_profile_edit(cx)))
                        .test_support(),
                ),
        );
    gpui_kit::deferred(
        menu_box(t)
            .id("profile-card")
            .p_0()
            .top(px(32.))
            .right_0()
            .child(card)
            .test_support(),
    )
    .with_priority(2)
    .into_any_element()
}

impl CleisApp {
    /// "Edit profile": the card under the picture, filled with the profile shown.
    pub fn open_profile_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let p = effective(s);
        let name = cx.new(|cx| InputState::new(window, cx));
        let email = cx.new(|cx| InputState::new(window, cx));
        let (n, e) = (p.name.clone(), p.email.clone());
        name.update(cx, |i, cx| {
            i.set_value(n, window, cx);
            i.focus(window, cx);
        });
        email.update(cx, |i, cx| i.set_value(e, window, cx));
        if let Some(s) = self.session() {
            s.account_menu = false;
            s.profile_edit = Some(ProfileEdit {
                name,
                email,
                photo: p.photo,
                error: None,
            });
        }
        cx.notify();
    }

    /// The photo: a picture file, centred square, 96 px.
    pub fn pick_profile_photo(&mut self, cx: &mut Context<Self>) {
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
                this.update(cx, |this, cx| this.profile_photo_from(&p, cx))
                    .ok();
            }
        })
        .detach();
    }

    pub fn profile_photo_from(&mut self, path: &std::path::Path, cx: &mut Context<Self>) {
        let r = std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|b| super::contacts::portrait_of(&b, PHOTO_SIDE));
        if let Some(e) = self.session().and_then(|s| s.profile_edit.as_mut()) {
            match r {
                Ok(url) => {
                    e.photo = Some(url);
                    e.error = None;
                }
                Err(err) => e.error = Some(crate::trf!("Photo not read: {}", err).into()),
            }
        }
        cx.notify();
    }

    /// Save: kept in Cleis (synced with the settings), then sent to every organization.
    pub fn save_profile_edit(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(e) = s.profile_edit.as_ref() else {
            return;
        };
        let p = Profile {
            name: e.name.read(cx).value().trim().to_owned(),
            email: e.email.read(cx).value().trim().to_owned(),
            photo: e.photo.clone(),
        };
        // The e-mail is optional (27/11/2026), with or without an organization.
        let error = if p.name.is_empty() {
            Some(tr("Enter your name."))
        } else if !p.email.is_empty() && !p.email.contains('@') {
            Some(tr("Enter an e-mail address."))
        } else {
            None
        };
        if let Some(err) = error {
            if let Some(e) = s.profile_edit.as_mut() {
                e.error = Some(err.into());
            }
            cx.notify();
            return;
        }
        s.profile_edit = None;
        let kept = p.clone();
        self.update_prefs(move |prefs| prefs.profile = kept, cx);
        self.profile_to_orgs(&p, cx);
        self.toast(tr("Saved"), cx);
    }

    /// Each organization gets the profile (its own name and e-mail for this member).
    /// No e-mail in the profile: each organization keeps the one it has (sent back as it is, so a
    /// server of an earlier version, which wants one, takes the change too).
    pub fn profile_to_orgs(&mut self, p: &Profile, cx: &mut Context<Self>) {
        for k in self.org_keys() {
            let mut req = ProfileChange {
                name: p.name.clone(),
                email: p.email.clone(),
                photo: Some(p.photo.clone().unwrap_or_default()),
            };
            self.in_org(&k, |app| {
                if req.email.is_empty()
                    && let Some(m) = app.session().and_then(|s| s.org.as_ref()?.me())
                {
                    req.email = m.email.clone();
                }
                app.org_request::<_, MemberView>(
                    "/v1/me",
                    req,
                    |app, r, cx| {
                        if let Err(e) = r {
                            app.toast(crate::trf!("Profile not sent: {}", e), cx);
                        }
                    },
                    cx,
                )
            });
        }
    }
}
