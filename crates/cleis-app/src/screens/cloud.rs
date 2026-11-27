//! Sign in with Microsoft or Google (D-104, mockups 36b–36d): the provider's page opens in the
//! browser; meanwhile the same dialog as GitHub's (37n) says Cleis is waiting, without a code to
//! type. Signed in, the vault in the account's app folder is found (a new computer) or the open
//! vault is sent there, through the place connection of Sync & backups.

use crate::app::{CleisApp, Phase};
use crate::cloud::{self, Cloud};
use crate::i18n::tr;
use crate::remote::{RemotePlace, Secret};
use crate::theme::*;
use crate::ui::*;
use gpui_kit::TestSupportExt;
use gpui_kit::prelude::*;
use gpui_kit::{AnyElement, Context, SharedString, Window, div, px};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// The file's name in the account, the same on every computer.
pub const VAULT_FILE: &str = "vault.cleis";

pub struct CloudSignIn {
    pub cloud: Cloud,
    /// The provider's page (opened again by the button).
    pub url: Option<String>,
    pub cancel: Option<Arc<AtomicBool>>,
    pub error: Option<SharedString>,
    /// Signed in: the vault is being looked for or sent.
    pub finishing: bool,
    /// Counts the sign-ins: an answer to a cancelled one is dropped.
    pub round: u64,
}

pub fn dialog(
    app: &CleisApp,
    t: &Tokens,
    cx: &mut gpui_kit::Context<CleisApp>,
) -> Option<AnyElement> {
    let c = app.cloud.as_ref()?;
    let name = c.cloud.name();
    let status = match (&c.error, c.finishing) {
        (Some(e), _) => div()
            .text_size(TEXT_SM)
            .text_color(t.danger_text)
            .child(e.clone()),
        (None, true) => waiting(t, tr("Finding your vault…").to_owned()),
        (None, false) => waiting(t, crate::trf!("Waiting for {}…", name)),
    };
    let card = dialog_box(t)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .child(icon(c.cloud.icon(), 18., t.text_muted))
                .child(
                    div()
                        .text_size(TEXT_LG)
                        .line_height(px(20.))
                        .font_weight(W_MEDIUM)
                        .child(crate::trf!("Sign in with {}", name)),
                ),
        )
        .child(
            div()
                .id("cloud-status")
                .child(status)
                .tooltip(tip(crate::trf!(
                    "Your vault goes to {}, in a folder only Cleis uses",
                    c.cloud.storage()
                )))
                .test_support(),
        )
        .child(
            div()
                .pt(px(8.))
                .flex()
                .justify_end()
                .gap(px(8.))
                .child(
                    button("cloud-cancel", t, BtnKind::Default, false)
                        .child(tr("Cancel"))
                        .on_click(cx.listener(|this, _, _, cx| this.cancel_cloud(cx)))
                        .test_support(),
                )
                .child(if c.error.is_some() {
                    let which = c.cloud;
                    button("cloud-retry", t, BtnKind::Primary, false)
                        .child(tr("Try again"))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.start_cloud(which, window, cx)
                        }))
                        .test_support()
                } else {
                    button("cloud-open", t, BtnKind::Primary, false)
                        .when(c.url.is_none() || c.finishing, |b| b.opacity(0.6))
                        .child(icon("external", 14., t.primary_ink))
                        .child(crate::trf!("Open {}", name))
                        .tooltip(tip(tr("Opens the sign-in page again in your browser")))
                        .on_click(cx.listener(|this, _, _, cx| this.open_cloud_page(cx)))
                        .test_support()
                }),
        );
    Some(
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(t.overlay)
            .child(
                div()
                    .id("cloud-dialog")
                    .occlude()
                    .child(card)
                    .test_support(),
            )
            .into_any_element(),
    )
}

fn waiting(t: &Tokens, text: String) -> gpui_kit::Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .text_color(t.text_muted)
        .child(icon("sync", 14., t.accent_text))
        .child(text)
}

/// The place of a signed-in account: the vault's file in its app folder.
pub fn place_of(c: Cloud, api: &str, account: &str) -> RemotePlace {
    match c {
        Cloud::Microsoft => RemotePlace::OneDrive {
            path: VAULT_FILE.into(),
            api: api.into(),
            account: account.into(),
        },
        Cloud::Google => RemotePlace::GoogleDrive {
            name: VAULT_FILE.into(),
            api: api.into(),
            account: account.into(),
        },
    }
}

impl CleisApp {
    /// Settings › Account or the first launch's "Sign in" (36b, 36d): the provider's page in the
    /// browser, Cleis waiting on this computer for its answer.
    pub fn start_cloud(&mut self, c: Cloud, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(old) = self.cloud.as_ref().and_then(|s| s.cancel.clone()) {
            old.store(true, Ordering::Relaxed);
        }
        let e = self.cloud_hosts.of(c).clone();
        if !e.usable(c) {
            self.toast(tr("Not available in this build"), cx);
            return;
        }
        let round = self.cloud.as_ref().map_or(0, |s| s.round) + 1;
        let pending = match cloud::prepare(c, &e, "") {
            Ok(p) => p,
            Err(err) => {
                self.cloud = Some(CloudSignIn {
                    cloud: c,
                    url: None,
                    cancel: None,
                    error: Some(crate::i18n::tr_dynamic(&err).into()),
                    finishing: false,
                    round,
                });
                cx.notify();
                return;
            }
        };
        let url = pending.url.clone();
        self.cloud = Some(CloudSignIn {
            cloud: c,
            url: Some(url.clone()),
            cancel: Some(pending.cancel.clone()),
            error: None,
            finishing: false,
            round,
        });
        if !cfg!(test) && !self.demo {
            cx.open_url(&url);
        }
        cx.notify();
        let task = cx.background_spawn(async move { cloud::finish(pending) });
        let api = e.api.clone();
        cx.spawn_in(window, async move |this, cx| {
            let r = task.await;
            this.update_in(cx, |this, _window, cx| {
                let Some(s) = this.cloud.as_mut().filter(|s| s.round == round) else {
                    return;
                };
                match r {
                    Ok(g) => {
                        s.finishing = true;
                        cx.notify();
                        this.cloud_signed(c, &api, g, cx);
                    }
                    Err(err) if err == "Cancelled" => {}
                    Err(err) => {
                        s.error = Some(crate::i18n::tr_dynamic(&err).into());
                        cx.notify();
                    }
                }
            })
            .ok();
        })
        .detach();
    }

    /// Signed in: on the first launch the account's vault is looked for; in the open vault it
    /// becomes the sync place (sent there at once).
    fn cloud_signed(&mut self, c: Cloud, api: &str, g: cloud::Grant, cx: &mut Context<Self>) {
        let place = place_of(c, api, &g.account);
        let secret = Secret {
            token: g.token,
            key: String::new(),
            refresh: g.refresh,
            expires: g.expires,
        };
        if let Phase::Welcome(w) = &mut self.phase
            && let Some(r) = w.restore.as_mut()
        {
            r.signed = Some(g.account.clone());
            w.round += 1;
            w.up = true;
        }
        self.connect_remote(place, secret, cx);
    }

    pub fn cancel_cloud(&mut self, cx: &mut Context<Self>) {
        if let Some(c) = self.cloud.take().and_then(|s| s.cancel) {
            c.store(true, Ordering::Relaxed);
        }
        if let Phase::Welcome(w) = &mut self.phase
            && w.restore.as_ref().is_some_and(|r| r.found.is_empty())
        {
            w.restore = None;
        }
        cx.notify();
    }

    pub fn open_cloud_page(&mut self, cx: &mut Context<Self>) {
        let Some(url) = self.cloud.as_ref().and_then(|s| s.url.clone()) else {
            return;
        };
        if !cfg!(test) && !self.demo {
            cx.open_url(&url);
        }
    }

    /// Settings › Account › Revoke access: Cleis forgets the account here; Google's grant is
    /// revoked at once, Microsoft's page of app permissions opens (it has no revoking API for
    /// a desktop app).
    pub fn revoke_cloud(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(place) = s.sync.remote.clone() else {
            return;
        };
        let Some(c) = place.cloud() else { return };
        let secret = s.sync.secret(&s.vault);
        self.disconnect_sync(cx);
        match c {
            Cloud::Google => {
                let e = match &place {
                    RemotePlace::GoogleDrive { api, .. } => cloud::Endpoints::of_api(c, api),
                    _ => return,
                };
                cx.background_spawn(async move {
                    let _ = cloud::revoke(&e, &secret.refresh);
                })
                .detach();
                self.toast(tr("Google access revoked"), cx);
            }
            Cloud::Microsoft => {
                if !self.demo {
                    cx.open_url(c.revoke_page());
                }
                self.toast(
                    tr("Click Remove next to Cleis-Sync on the page Microsoft opened"),
                    cx,
                );
            }
        }
    }

    /// The vault made after signing in with no vault on the account (as 37r for GitHub): it goes
    /// to the account once its recovery key is put away.
    pub fn resume_cloud(&mut self, cx: &mut Context<Self>) -> bool {
        if !matches!(self.phase, Phase::Main(_)) {
            return false;
        }
        let Some((place, secret)) = self.cloud_after.take() else {
            return false;
        };
        self.connect_remote(place, secret, cx);
        true
    }
}
