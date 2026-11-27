//! Settings › Account (mockups 36b, 36c, D-80): sign in once and the vault follows to the other
//! computers. Under it, the sync place of Sync & backups (D-50, D-65): the vault file, sealed,
//! carries the items, the settings, the organizations joined, and each sync is a version.
//! GitHub works today (a private repository); Microsoft and Google wait for Cleis to be
//! registered with them (D-50, « En suspens » 4).

use crate::app::{CleisApp, Session};
use crate::i18n::tr;
use crate::sync::Provider;
use crate::theme::*;
use crate::ui::*;
use gpui_kit::TestSupportExt;
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, Window, div, px};

/// Is the vault's sync place an account (a provider one signs in to)?
pub fn signed_in(s: &Session) -> bool {
    s.sync.place.is_some() && s.sync.provider == Provider::GitHub
}

/// The GitHub account of the sync place (`owner` of `owner/cleis-vault`).
pub fn github_login(s: &Session) -> Option<String> {
    match s.sync.remote.as_ref()? {
        crate::remote::RemotePlace::GitHub { repo, .. } => {
            repo.split('/').next().map(str::to_owned)
        }
        _ => None,
    }
}

/// Settings › Account (mockups 39a, 39b, 27/11/2026): the account, then what was Security
/// (opening, two-factor authentication, locking, clipboard), what an organization knows of you
/// (was Profile), and deleting the account at the bottom. One page, one menu entry.
pub fn page(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let head = div()
        .text_size(TEXT_XL)
        .line_height(px(26.))
        .font_weight(W_STRONG)
        .child(tr("Account"));
    let account = if signed_in(s) {
        signed(s, t, cx)
    } else {
        sign_in(t, cx)
    };
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(head)
        .child(account)
        .children(super::security::sections(s, t, cx))
        .children(super::org::profile_sections(s, t, cx))
        .child(super::erase::account_section(t, cx))
}

/// Not signed in (36b): the providers, Sign in.
fn sign_in(t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let prov = |id: &'static str,
                ico: &str,
                name: &str,
                sub: &str,
                on: bool,
                planned: Option<&'static str>,
                cx: &mut Context<CleisApp>| {
        // What each one is goes in its tooltip, no line under the name (27/11/2026).
        div()
            .id(id)
            .when(planned.is_some(), |d| d.opacity(0.45))
            .tooltip(tip(crate::i18n::tr_owned(planned.unwrap_or(sub))))
            .when(planned.is_none(), |d| d.cursor_pointer())
            .child(super::welcome::choice(t, on, name, "", ico))
            .when(planned.is_none() && id == "acc-github", |d| {
                d.on_click(cx.listener(|this, _, window, cx| this.start_github(window, cx)))
            })
            .test_support()
    };
    section(t)
            .child(
                div()
                    .id("acc-title")
                    .child(section_title(t, tr("Sign in")))
                    .tooltip(tip(tr(
                        "Your vault, settings, backups and organizations follow you to your other computers. Everything is encrypted on this computer before it leaves: the provider sees only sealed files.",
                    ))),
            )
            .child(prov("acc-local", "laptop", "Local", "This computer only", true, None, cx))
            .child(prov(
                "acc-microsoft",
                "window",
                "Microsoft",
                "Personal, work or school account · OneDrive",
                false,
                Some("Available once Cleis is registered with Microsoft"),
                cx,
            ))
            .child(prov(
                "acc-google",
                "globe",
                "Google",
                "Google Drive",
                false,
                Some("Available once Cleis is registered with Google"),
                cx,
            ))
            .child(prov("acc-github", "repo", "GitHub", "A private repository", false, None, cx))
            .child(
                div().flex().justify_end().child(
                    button("acc-signin", t, BtnKind::Primary, false)
                        .child(tr("Sign in"))
                        .tooltip(tip(tr("With GitHub: a code to type on github.com")))
                        .on_click(cx.listener(|this, _, window, cx| this.start_github(window, cx)))
                        .test_support(),
                ),
            )
}

/// Signed in (39a): the account, its last sync, Sign out and Revoke access.
fn signed(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let c = &s.sync;
    let status = match (&c.last_error, c.last_sync) {
        _ if c.busy => chip(t, Tone::Neutral, tr("Syncing…"), false),
        (Some(_), _) => chip(t, Tone::Danger, tr("Not synced"), true),
        (None, Some(ts)) => chip(
            t,
            Tone::Ok,
            crate::trf!("Synced {}", super::sync::ago(ts)),
            true,
        ),
        (None, None) => chip(t, Tone::Neutral, tr("Never synced"), false),
    };
    let repo = match c.remote.as_ref() {
        Some(crate::remote::RemotePlace::GitHub { repo, .. }) => repo
            .split_once('/')
            .map_or(repo.clone(), |(_, name)| name.to_owned()),
        Some(r) => r.label(),
        None => String::new(),
    };
    let who = github_login(s).map_or_else(|| c.provider.name().to_owned(), |l| format!("@{l}"));
    let is_github = github_login(s).is_some();
    section(t).child(section_title(t, c.provider.name())).child(
        div()
            .id("acc-who")
            .flex()
            .items_center()
            .gap(px(10.))
            .child(icon("repo", 18., t.text_muted))
            .child(div().child(who))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .font_family(FONT_MONO)
                    .text_size(TEXT_SM)
                    .text_color(t.text_muted)
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(repo),
            )
            .tooltip(tip(tr(
                "Your vault, settings, backups and organizations follow you to your other computers",
            )))
            .child(status)
            .child(
                button("acc-signout", t, BtnKind::Ghost, true)
                    .child(tr("Sign out"))
                    .tooltip(tip(tr(
                        "This computer stops following; the copy there stays sealed",
                    )))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.ask_action(crate::app::ConfirmAction::DisconnectPlace, cx)
                    }))
                    .test_support(),
            )
            .when(is_github, |d| {
                d.child(
                    button("acc-revoke", t, BtnKind::DangerGhost, true)
                        .child(tr("Revoke access"))
                        .tooltip(tip(tr(
                            "Removes Cleis' access to your GitHub repositories, on every computer",
                        )))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.ask_action(crate::app::ConfirmAction::RevokeGitHub, cx)
                        }))
                        .test_support(),
                )
            }),
    )
}

impl CleisApp {
    /// The welcome screen's "Sign in" (36d): the vault from another computer, through the account.
    pub fn sign_in_elsewhere(&mut self, p: Provider, window: &mut Window, cx: &mut Context<Self>) {
        self.start_restore(window, cx);
        match p {
            Provider::GitHub => self.start_github(window, cx),
            _ => self.open_place(p, window, cx),
        }
    }
}
