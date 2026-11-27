//! Sign in with GitHub (mockup 37n): the code to type on github.com, then the vault's private
//! repository is the sync place, in Settings › Account as on the first launch of a new computer
//! (36d). The rest is the GitHub place of Sync & backups (D-65): the dialog's values are filled
//! and connected as if typed. GitHub App (D-97, D-101, mockups 41): when Cleis does not reach
//! `cleis-vault`, the same dialog shows the two steps on GitHub (make it, install Cleis on it) and
//! Cleis watches for the repository by itself.

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
    /// Signed in, the repository not reached yet: the steps on GitHub (mockups 41).
    pub setup: Option<Setup>,
}

/// Mockups 41a, 41b: what is left to do on GitHub.
#[derive(Clone)]
pub struct Setup {
    pub signed: Signed,
    /// `cleis-vault` made (by Cleis, by the person, or there already).
    pub made: bool,
    /// GitHub's install page opened: "Waiting for GitHub…".
    pub installing: bool,
    /// First launch on a new computer: the vault is looked for, not made.
    pub restoring: bool,
}

/// A GitHub account signed in: what the next step needs.
#[derive(Clone)]
pub struct Signed {
    pub login: String,
    pub token: zeroize::Zeroizing<String>,
    /// D-101: renews the token (8 hours); empty when the app's tokens do not end.
    pub refresh: zeroize::Zeroizing<String>,
    /// When `token` ends (Unix seconds); 0: it does not.
    pub expires: i64,
    pub hosts: Hosts,
}

impl Signed {
    fn of(login: String, g: gl::Grant, hosts: Hosts) -> Self {
        let expires = g.expires_at();
        Self {
            login,
            token: zeroize::Zeroizing::new(g.token),
            refresh: zeroize::Zeroizing::new(g.refresh),
            expires,
            hosts,
        }
    }
}

pub fn dialog(app: &CleisApp, t: &Tokens, cx: &mut Context<CleisApp>) -> Option<AnyElement> {
    let g = app.github.as_ref()?;
    if let Some(st) = &g.setup {
        return Some(overlay(t, setup_card(st, g.error.clone(), t, cx)));
    }
    let restoring = matches!(app.phase, Phase::Welcome(_));
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
        .child(
            div()
                .id("gh-status")
                .child(status)
                .tooltip(tip(if restoring {
                    crate::trf!(
                        "Cleis looks for the private repository {} in your account",
                        gl::REPO_NAME
                    )
                } else {
                    crate::trf!(
                        "A private repository {} is made in your account",
                        gl::REPO_NAME
                    )
                })),
        )
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
                .child(if g.error.is_some() {
                    // The code is spent (expired, refused): a new one.
                    button("gh-retry", t, BtnKind::Primary, false)
                        .child(tr("Try again"))
                        .on_click(cx.listener(|this, _, window, cx| this.start_github(window, cx)))
                        .test_support()
                } else {
                    button("gh-open", t, BtnKind::Primary, false)
                        .when(g.code.is_none() || g.finishing, |b| b.opacity(0.6))
                        .child(icon("external", 14., t.primary_ink))
                        .child(tr("Open GitHub"))
                        .tooltip(tip(tr("github.com/login/device, the code copied")))
                        .on_click(cx.listener(|this, _, _, cx| this.open_github_page(cx)))
                        .test_support()
                }),
        );
    Some(overlay(t, card))
}

/// Mockups 41a, 41b: the account signed in, then each step a page of GitHub opened by its button.
fn setup_card(
    st: &Setup,
    error: Option<SharedString>,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> Div {
    let login = st.signed.login.clone();
    let initial = login
        .chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_default();
    let account = div()
        .flex()
        .items_center()
        .gap(px(10.))
        .px(px(12.))
        .py(px(10.))
        .bg(t.bg)
        .border_1()
        .border_color(t.line_strong)
        .rounded(RADIUS_LG)
        .child(avatar(t, &initial, 20.))
        .child(div().flex_1().child(format!("@{login}")))
        .child(
            div()
                .text_size(TEXT_SM)
                .text_color(t.text_muted)
                .child(tr("Signed in")),
        );
    let create = step(
        "gh-step-create",
        crate::trf!("Create the private repository {}", gl::REPO_NAME),
        if st.made { Step::Done } else { Step::Next },
        tr("Create on GitHub"),
        tr("The page opens filled in: Private, cleis-vault. Click Create repository."),
        t,
    )
    .when(!st.made, |d| {
        d.child(
            button("gh-create", t, BtnKind::Primary, true)
                .child(icon("external", 14., t.primary_ink))
                .child(tr("Create on GitHub"))
                .tooltip(tip(tr(
                    "The page opens filled in: Private, cleis-vault. Click Create repository.",
                )))
                .on_click(cx.listener(|this, _, _, cx| this.github_create_page(cx)))
                .test_support(),
        )
    });
    let install = step(
        "gh-step-install",
        tr("Install Cleis").into(),
        Step::Next,
        tr("Install on GitHub"),
        tr("Only select repositories › cleis-vault › Install"),
        t,
    )
    .child(
        button(
            "gh-install",
            t,
            if st.installing {
                BtnKind::Default
            } else {
                BtnKind::Primary
            },
            true,
        )
        .child(icon(
            "external",
            14.,
            if st.installing { t.text } else { t.primary_ink },
        ))
        .child(tr("Install on GitHub"))
        .tooltip(tip(crate::trf!(
            "Only select repositories › {} › Install",
            gl::REPO_NAME
        )))
        .on_click(cx.listener(|this, _, _, cx| this.github_install_page(cx)))
        .test_support(),
    );
    dialog_box(t)
        .child(
            div()
                .text_size(TEXT_LG)
                .line_height(px(20.))
                .font_weight(W_MEDIUM)
                .child(tr("Your vault on GitHub")),
        )
        .child(account)
        .child(
            div()
                .id("gh-steps")
                .flex()
                .flex_col()
                .child(create)
                .when(st.made, |d| d.child(install))
                .test_support(),
        )
        .when_some(error, |d, e| {
            d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e))
        })
        .when(st.installing, |d| {
            d.child(
                div()
                    .id("gh-waiting")
                    .child(waiting(t, tr("Waiting for GitHub…")))
                    .test_support(),
            )
        })
        .child(
            div()
                .pt(px(8.))
                .flex()
                .gap(px(8.))
                // A new computer, and no vault on the account: make one here.
                .when(st.restoring, |d| {
                    d.child(
                        button("gh-new-vault", t, BtnKind::Ghost, false)
                            .child(tr("Make a new vault"))
                            .tooltip(tip(tr("No vault on this GitHub account yet")))
                            .on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.github_new_vault(window, cx)
                                }),
                            )
                            .test_support(),
                    )
                })
                .child(div().flex_1())
                .child(
                    button("gh-cancel", t, BtnKind::Default, false)
                        .child(tr("Cancel"))
                        .on_click(cx.listener(|this, _, _, cx| this.cancel_github(cx)))
                        .test_support(),
                ),
        )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Step {
    Next,
    Done,
}

/// A step of mockup 41: the check once done, the title (its tip says what to do on GitHub).
fn step(
    id: &'static str,
    title: String,
    state: Step,
    _button: &str,
    tip_text: &'static str,
    t: &Tokens,
) -> Div {
    let done = state == Step::Done;
    div()
        .flex()
        .items_center()
        .gap(px(10.))
        .min_h(px(44.))
        .py(px(6.))
        .when(done, |d| {
            d.child(
                div()
                    .size(px(20.))
                    .flex_none()
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(t.primary)
                    .child(icon("check", 12., t.primary_ink)),
            )
        })
        .child(
            div()
                .id(id)
                .flex_1()
                .child(title)
                .when(!done, |d| d.tooltip(tip(tr(tip_text))))
                .test_support(),
        )
}

fn overlay(t: &Tokens, card: Div) -> AnyElement {
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
        .into_any_element()
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

    /// Mockup 39c: Cleis forgets its GitHub token here, then GitHub's page of authorized apps
    /// opens, where "Revoke" next to cleis-sync removes the authorization for every computer. GitHub revokes a grant
    /// through its API only with the application's secret, which a desktop app cannot keep.
    pub fn revoke_github(&mut self, cx: &mut Context<Self>) {
        self.disconnect_sync(cx);
        let url = self.github_hosts.authorizations_page();
        if !self.demo {
            cx.open_url(&url);
        }
        self.toast(
            tr("Click Revoke next to cleis-sync on the page GitHub opened"),
            cx,
        );
    }

    pub fn cancel_github(&mut self, cx: &mut Context<Self>) {
        self.github = None;
        // The first screen: back to its options, nothing chosen.
        if let Phase::Welcome(w) = &mut self.phase
            && w.restore.as_ref().is_some_and(|r| r.signed.is_none())
        {
            w.restore = None;
        }
        cx.notify();
    }

    /// The vault made after 37r: its repository made on the account, then the GitHub place
    /// connected as from Settings (the dialog shows the step).
    pub fn resume_github(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !matches!(self.phase, Phase::Main(_)) {
            return;
        }
        // D-104: the same for a Microsoft or Google account.
        if self.resume_cloud(cx) {
            return;
        }
        let Some(n) = self.github_after.take() else {
            return;
        };
        let round = self.github.as_ref().map_or(0, |g| g.round) + 1;
        self.github = Some(GitHubSignIn {
            round,
            hosts: n.hosts.clone(),
            finishing: true,
            ..Default::default()
        });
        cx.notify();
        self.finish_github(round, n, true, window, cx);
    }

    /// The token given: the account's vault repository found (or made), then connected; not
    /// reached, the steps on GitHub (mockups 41).
    fn finish_github(
        &mut self,
        round: u64,
        signed: Signed,
        create: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (h, tok, who) = (
            signed.hosts.clone(),
            signed.token.clone(),
            signed.login.clone(),
        );
        let task = cx.background_spawn(async move { gl::vault_repo(&h, &tok, &who, create) });
        cx.spawn_in(window, async move |this, cx| {
            let found = task.await;
            this.update_in(cx, |this, window, cx| {
                if !this.github.as_ref().is_some_and(|g| g.round == round) {
                    return;
                }
                match found {
                    Ok(gl::Found::Repo(repo, branch)) => {
                        this.github = None;
                        this.connect_github(&repo, &branch, &signed, window, cx);
                        return;
                    }
                    // Mockup 40b: no vault on the account; the first screen goes on to make
                    // one, saved to the account once its recovery key is put away.
                    Ok(gl::Found::Missing) => {
                        this.github = None;
                        this.github_after = Some(signed);
                        if let Phase::Welcome(w) = &mut this.phase {
                            w.restore = None;
                            w.choice = Some(crate::screens::welcome::Start::Account);
                            w.round += 1;
                            w.up = true;
                            let pw = w.password.clone();
                            pw.update(cx, |s, cx| s.focus(window, cx));
                        }
                    }
                    Ok(gl::Found::Setup { made }) => {
                        let restoring = matches!(this.phase, Phase::Welcome(_));
                        if let Some(g) = this.github.as_mut() {
                            g.finishing = false;
                            g.setup = Some(Setup {
                                signed,
                                made,
                                installing: false,
                                restoring,
                            });
                        }
                        this.watch_github(round, window, cx);
                    }
                    Err(e) => {
                        if let Some(g) = this.github.as_mut() {
                            g.finishing = false;
                            g.error = Some(crate::i18n::tr_dynamic(&e).into());
                        }
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Mockup 41a: GitHub's page making `cleis-vault`, filled in; the next step shows.
    pub fn github_create_page(&mut self, cx: &mut Context<Self>) {
        let Some(st) = self.github.as_mut().and_then(|g| g.setup.as_mut()) else {
            return;
        };
        st.made = true;
        let url = st.signed.hosts.create_page();
        if !cfg!(test) && !self.demo {
            cx.open_url(&url);
        }
        cx.notify();
    }

    /// Mockup 41b: GitHub's page installing Cleis; Cleis waits for the repository.
    pub fn github_install_page(&mut self, cx: &mut Context<Self>) {
        let Some(st) = self.github.as_mut().and_then(|g| g.setup.as_mut()) else {
            return;
        };
        st.installing = true;
        let url = st.signed.hosts.install_page();
        if !cfg!(test) && !self.demo {
            cx.open_url(&url);
        }
        cx.notify();
    }

    /// A new computer, no vault on the account (40b): the first screen goes on to make one,
    /// saved to the account once made (then the steps on GitHub if Cleis cannot reach it).
    pub fn github_new_vault(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(st) = self.github.take().and_then(|g| g.setup) else {
            return;
        };
        self.github_after = Some(st.signed);
        if let Phase::Welcome(w) = &mut self.phase {
            w.restore = None;
            w.choice = Some(crate::screens::welcome::Start::Account);
            w.round += 1;
            w.up = true;
            let pw = w.password.clone();
            pw.update(cx, |s, cx| s.focus(window, cx));
        }
        cx.notify();
    }

    /// Every 5 seconds while the steps show: the repository reached, the place connects.
    fn watch_github(&mut self, round: u64, window: &mut Window, cx: &mut Context<Self>) {
        let exec = cx.background_executor().clone();
        cx.spawn_in(window, async move |this, cx| {
            loop {
                exec.timer(Duration::from_secs(5)).await;
                let Some(signed) = this
                    .update(cx, |this, _| {
                        this.github
                            .as_ref()
                            .filter(|g| g.round == round)
                            .and_then(|g| g.setup.as_ref())
                            .map(|s| s.signed.clone())
                    })
                    .ok()
                    .flatten()
                else {
                    return;
                };
                let (h, tok, who) = (
                    signed.hosts.clone(),
                    signed.token.clone(),
                    signed.login.clone(),
                );
                let r = exec
                    .spawn(async move { gl::vault_repo(&h, &tok, &who, false) })
                    .await;
                let done = this
                    .update_in(cx, |this, window, cx| {
                        if !this.github.as_ref().is_some_and(|g| g.round == round) {
                            return true;
                        }
                        match r {
                            Ok(gl::Found::Repo(repo, branch)) => {
                                this.github = None;
                                this.connect_github(&repo, &branch, &signed, window, cx);
                                true
                            }
                            // A repository made public meanwhile: said, still watched.
                            Err(e) => {
                                if let Some(g) = this.github.as_mut() {
                                    g.error = Some(crate::i18n::tr_dynamic(&e).into());
                                }
                                cx.notify();
                                false
                            }
                            Ok(_) => {
                                if let Some(g) = this.github.as_mut()
                                    && g.error.take().is_some()
                                {
                                    cx.notify();
                                }
                                false
                            }
                        }
                    })
                    .unwrap_or(true);
                if done {
                    return;
                }
            }
        })
        .detach();
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
                    Ok(Poll::Token(grant)) => {
                        // The account first (its login names the repository).
                        let (h, tok) = (hosts.clone(), grant.token.clone());
                        let who = exec.spawn(async move { gl::login(&h, &tok) }).await;
                        let who = match who {
                            Ok(w) => w,
                            Err(e) => {
                                this.update(cx, fail(e)).ok();
                                return;
                            }
                        };
                        this.update_in(cx, |this, window, cx| {
                            if let Some(g) = this.github.as_mut().filter(|g| g.round == round) {
                                g.finishing = true;
                            }
                            cx.notify();
                            let signed = Signed::of(who, grant, hosts);
                            // Restoring and no vault yet: 40b, the vault is made here.
                            this.finish_github(round, signed, !restoring, window, cx);
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
        signed: &Signed,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_place(Provider::GitHub, window, cx);
        // Mockup 40c: on the first screen the account's vault opens on the same card.
        if let Phase::Welcome(w) = &mut self.phase
            && let Some(r) = w.restore.as_mut()
        {
            r.signed = repo.split('/').next().map(str::to_owned);
            w.round += 1;
            w.up = true;
        }
        let Some(d) = self.place_slot().and_then(|p| p.as_mut()) else {
            return;
        };
        d.api = Some(signed.hosts.api.clone());
        d.renew =
            (!signed.refresh.is_empty()).then(|| (signed.refresh.to_string(), signed.expires));
        let inputs = d.inputs.clone();
        // The same file name on every computer, whatever the vault is called here.
        let values = [repo, "vault.cleis", branch, signed.token.as_str()];
        for (i, v) in inputs.iter().zip(values) {
            if !v.is_empty() {
                let v = v.to_owned();
                i.update(cx, |i, cx| i.set_value(v, window, cx));
            }
        }
        self.place_connect(cx);
    }
}
