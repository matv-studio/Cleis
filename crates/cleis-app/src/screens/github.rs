//! Sign in with GitHub (mockup 37n): the code to type on github.com, then the vault's private
//! repository is the sync place, in Settings › Account as on the first launch of a new computer
//! (36d). The rest is the GitHub place of Sync & backups (D-65): the dialog's values are filled
//! and connected as if typed.

use crate::app::{CleisApp, Phase};
use crate::github_login::{self as gl, DeviceCode, Hosts, Poll};
use crate::i18n::tr;
use crate::sync::Provider;
use crate::theme::*;
use crate::ui::*;
use gpui_kit::TestSupportExt;
use gpui_kit::prelude::*;
use gpui_kit::{AnyElement, ClipboardItem, Context, Div, SharedString, Window, div, px};
use std::time::Duration;

#[derive(Default)]
pub struct GitHubSignIn {
    pub code: Option<DeviceCode>,
    pub error: Option<SharedString>,
    /// The code was typed: the repository is being found or made.
    pub finishing: bool,
    /// Counts the sign-ins: a cancelled one stops polling.
    pub round: u64,
    /// Where GitHub is (a stand-in in the tests).
    pub hosts: Hosts,
}

pub fn dialog(app: &CleisApp, t: &Tokens, cx: &mut Context<CleisApp>) -> Option<AnyElement> {
    let g = app.github.as_ref()?;
    let code: SharedString = g
        .code
        .as_ref()
        .map(|c| c.user_code.clone())
        .unwrap_or_else(|| "····-····".into())
        .into();
    let copy_code = code.clone();
    let status = match (&g.error, g.finishing) {
        (Some(e), _) => div()
            .text_size(TEXT_SM)
            .text_color(t.danger_text)
            .child(e.clone()),
        (None, true) => waiting(t, tr("Finding your vault…")),
        (None, false) => waiting(t, tr("Waiting for GitHub…")),
    };
    let card = dialog_box(t)
        .child(
            div()
                .text_size(TEXT_LG)
                .line_height(px(20.))
                .font_weight(W_MEDIUM)
                .child(tr("Sign in with GitHub")),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(12.))
                .p(px(16.))
                .bg(t.bg)
                .border_1()
                .border_color(t.line_strong)
                .rounded(RADIUS_LG)
                .child(
                    div()
                        .id("gh-code")
                        .font_family(FONT_MONO)
                        .text_size(px(22.))
                        .line_height(px(28.))
                        .child(code),
                )
                .child(
                    icon_button("gh-copy", t, "copy", true)
                        .tooltip(tip(tr("Copy")))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(copy_code.to_string()));
                            this.toast(tr("Copied"), cx);
                        }))
                        .test_support(),
                ),
        )
        .child(div().id("gh-status").child(status).tooltip(tip(crate::trf!(
            "A private repository {} is made in your account",
            gl::REPO_NAME
        ))))
        .child(
            div()
                .pt(px(8.))
                .flex()
                .justify_end()
                .gap(px(8.))
                .child(
                    button("gh-cancel", t, BtnKind::Default, false)
                        .child(tr("Cancel"))
                        .on_click(cx.listener(|this, _, _, cx| this.cancel_github(cx)))
                        .test_support(),
                )
                .child(
                    button("gh-open", t, BtnKind::Primary, false)
                        .when(g.code.is_none(), |b| b.opacity(0.6))
                        .child(icon("external", 14., t.primary_ink))
                        .child(tr("Open GitHub"))
                        .tooltip(tip(tr("github.com/login/device, the code copied")))
                        .on_click(cx.listener(|this, _, _, cx| this.open_github_page(cx)))
                        .test_support(),
                ),
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
                    .id("github-dialog")
                    .occlude()
                    .child(card)
                    .test_support(),
            )
            .into_any_element(),
    )
}

fn waiting(t: &Tokens, text: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .text_color(t.text_muted)
        .child(icon("sync", 14., t.accent_text))
        .child(text.to_owned())
}

impl CleisApp {
    /// Settings › Account, Sync & backups, or the first launch's "Sign in" (36d, 37n).
    pub fn start_github(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let round = self.github.as_ref().map_or(0, |g| g.round) + 1;
        let hosts = self
            .github
            .as_ref()
            .map(|g| g.hosts.clone())
            .unwrap_or_else(|| self.github_hosts.clone());
        self.github = Some(GitHubSignIn {
            round,
            hosts: hosts.clone(),
            ..Default::default()
        });
        cx.notify();
        let task = cx.background_spawn(async move { gl::start(&hosts) });
        cx.spawn_in(window, async move |this, cx| {
            let r = task.await;
            this.update_in(cx, |this, window, cx| {
                let Some(g) = this.github.as_mut().filter(|g| g.round == round) else {
                    return;
                };
                match r {
                    Ok(code) => {
                        g.code = Some(code);
                        this.poll_github(round, window, cx);
                    }
                    Err(e) => g.error = Some(crate::i18n::tr_dynamic(&e).into()),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn cancel_github(&mut self, cx: &mut Context<Self>) {
        self.github = None;
        cx.notify();
    }

    /// The page where the code is typed; the code copied, ready to paste.
    pub fn open_github_page(&mut self, cx: &mut Context<Self>) {
        let Some(c) = self.github.as_ref().and_then(|g| g.code.clone()) else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(c.user_code.clone()));
        if !cfg!(test) && !self.demo {
            cx.open_url(&c.verification_uri);
        }
    }

    /// Asks GitHub every `interval` seconds until the code is typed, refused or expired.
    fn poll_github(&mut self, round: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(g) = self.github.as_ref() else {
            return;
        };
        let Some(code) = g.code.clone() else { return };
        let hosts = g.hosts.clone();
        let restoring = matches!(self.phase, Phase::Welcome(_));
        let exec = cx.background_executor().clone();
        cx.spawn_in(window, async move |this, cx| {
            let mut every = Duration::from_secs(code.interval.max(5));
            let end = std::time::Instant::now() + Duration::from_secs(code.expires_in);
            loop {
                exec.timer(every).await;
                let alive = this
                    .update(cx, |this, _| {
                        this.github.as_ref().is_some_and(|g| g.round == round)
                    })
                    .unwrap_or(false);
                if !alive {
                    return;
                }
                let (h, dc) = (hosts.clone(), code.device_code.clone());
                let r = exec.spawn(async move { gl::poll(&h, &dc) }).await;
                let fail = |msg: String| {
                    let m = msg.clone();
                    move |this: &mut CleisApp, cx: &mut Context<CleisApp>| {
                        if let Some(g) = this.github.as_mut().filter(|g| g.round == round) {
                            g.error = Some(crate::i18n::tr_dynamic(&m).into());
                        }
                        cx.notify();
                    }
                };
                match r {
                    Ok(Poll::Pending) if std::time::Instant::now() < end => {}
                    Ok(Poll::Pending) => {
                        this.update(cx, fail("The code expired: sign in again".into()))
                            .ok();
                        return;
                    }
                    Ok(Poll::SlowDown) => every += Duration::from_secs(5),
                    Ok(Poll::Failed(e)) | Err(e) => {
                        this.update(cx, fail(e)).ok();
                        return;
                    }
                    Ok(Poll::Token(token)) => {
                        this.update(cx, |this, cx| {
                            if let Some(g) = this.github.as_mut() {
                                g.finishing = true;
                            }
                            cx.notify();
                        })
                        .ok();
                        let h = hosts.clone();
                        let tok = token.clone();
                        let found = exec
                            .spawn(async move {
                                let who = gl::login(&h, &tok)?;
                                gl::vault_repo(&h, &tok, &who, !restoring)
                            })
                            .await;
                        this.update_in(cx, |this, window, cx| match found {
                            Ok((repo, branch)) => {
                                let api = hosts.api.clone();
                                this.github = None;
                                this.connect_github(&repo, &branch, &api, &token, window, cx);
                            }
                            Err(e) => fail(e)(this, cx),
                        })
                        .ok();
                        return;
                    }
                }
            }
        })
        .detach();
    }

    /// The GitHub place (D-65) filled with what the sign-in gave, then connected: in the open
    /// vault it becomes the sync place; on the first launch it is the vault to restore.
    fn connect_github(
        &mut self,
        repo: &str,
        branch: &str,
        api: &str,
        token: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_place(Provider::GitHub, window, cx);
        let Some(d) = self.place_slot().and_then(|p| p.as_mut()) else {
            return;
        };
        d.api = Some(api.to_owned());
        let inputs = d.inputs.clone();
        // The same file name on every computer, whatever the vault is called here.
        let values = [repo, "vault.cleis", branch, token];
        for (i, v) in inputs.iter().zip(values) {
            if !v.is_empty() {
                let v = v.to_owned();
                i.update(cx, |i, cx| i.set_value(v, window, cx));
            }
        }
        self.place_connect(cx);
    }
}
