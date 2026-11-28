//! Sign in with Microsoft or Google (D-104, mockups 36b–36d): the provider's page opens in the
//! browser; meanwhile the same dialog as GitHub's (37n) says Cleis is waiting, without a code to
//! type. Signed in, the vault in the account's app folder is found (a new computer) or the open
//! vault is sent there, through the place connection of Sync & backups.

use crate::app::{CleisApp, Phase};
use crate::cloud::{self, Cloud};
use crate::i18n::tr;
use crate::remote::{DriveFolder, RemotePlace, Secret};
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
    /// D-110: "Custom folder" ticked: Google's folder picker after its sign-in, OneDrive's
    /// folders listed here (mockups 49j, 49k).
    pub custom: bool,
    /// OneDrive, signed in with "Custom folder": its folders to choose from.
    pub pick: Option<FolderPick>,
}

/// OneDrive's folders shown after signing in (mockup 49k).
pub struct FolderPick {
    pub api: String,
    pub grant: cloud::Grant,
    /// The folder shown, from the drive's top.
    pub path: Vec<String>,
    /// Its folders (`None`: being read).
    pub items: Option<Vec<String>>,
    pub error: Option<SharedString>,
}

/// The folder Cleis suggests at the drive's top, made when the vault is first sent there.
pub const SUGGESTED_FOLDER: &str = "Cleis";

pub fn dialog(
    app: &CleisApp,
    t: &Tokens,
    cx: &mut gpui_kit::Context<CleisApp>,
) -> Option<AnyElement> {
    let c = app.cloud.as_ref()?;
    if let Some(p) = c.pick.as_ref() {
        return Some(overlay(t, folder_dialog(p, t, cx)));
    }
    let name = c.cloud.name();
    let custom = c.custom;
    // D-110: unticked, nothing changes; ticked, the sign-in starts again asking to choose.
    let custom_row = div()
        .id("cloud-custom")
        .flex()
        .items_center()
        .gap(px(8.))
        .cursor_pointer()
        .child(crate::screens::org::check_box(t, custom))
        .child(tr("Custom folder"))
        .tooltip(tip(match c.cloud {
            Cloud::Google => tr("Choose the folder in Google Drive's own window, in your browser"),
            Cloud::Microsoft => tr("Choose a folder of your OneDrive after signing in"),
        }))
        .when(!c.finishing, |d| {
            let which = c.cloud;
            d.on_click(cx.listener(move |this, _, window, cx| {
                this.start_cloud_with(which, !custom, window, cx)
            }))
        })
        .test_support();
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
                .tooltip(tip(if custom {
                    crate::trf!(
                        "Your vault goes to the folder you choose in {}",
                        c.cloud.storage()
                    )
                } else {
                    crate::trf!(
                        "Your vault goes to {}, in a folder only Cleis uses",
                        c.cloud.storage()
                    )
                }))
                .test_support(),
        )
        .child(custom_row)
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
                            this.start_cloud_with(which, custom, window, cx)
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
    Some(overlay(t, card))
}

fn overlay(t: &Tokens, card: impl IntoElement) -> AnyElement {
    div()
        .absolute()
        .inset_0()
        .occlude()
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
        .into_any_element()
}

/// Mockup 49k: OneDrive's folders, one level at a time; the file's place below; Use this
/// folder. At the top, "Cleis" is offered (made when the vault is sent) when there is none.
fn folder_dialog(p: &FolderPick, t: &Tokens, cx: &mut Context<CleisApp>) -> gpui_kit::Div {
    let shown = |path: &[String]| {
        if path.is_empty() {
            "OneDrive".to_owned()
        } else {
            path.join(" / ")
        }
    };
    let row_box = |id: SharedString| {
        div()
            .id(id)
            .h(px(32.))
            .px(px(8.))
            .flex()
            .items_center()
            .gap(px(8.))
            .rounded(RADIUS_SM)
            .cursor_pointer()
            .hover(|d| d.bg(t.surface_hover))
    };
    let mut list = div()
        .id("folder-list")
        .h(px(200.))
        .overflow_y_scroll()
        .p(px(4.))
        .bg(t.bg)
        .border_1()
        .border_color(t.line_strong)
        .rounded(RADIUS_MD)
        .flex()
        .flex_col();
    if !p.path.is_empty() {
        list = list.child(
            row_box("folder-up".into())
                .text_color(t.text_muted)
                .child(icon("arrowR", 14., t.text_muted))
                .child(crate::trf!(
                    "Back to {}",
                    shown(&p.path[..p.path.len() - 1])
                ))
                .on_click(cx.listener(|this, _, _, cx| this.folder_pick_up(cx)))
                .test_support(),
        );
    }
    match &p.items {
        None => {
            list = list.child(
                div()
                    .p(px(8.))
                    .text_color(t.text_muted)
                    .child(tr("Reading the folders…")),
            )
        }
        Some(items) => {
            let mut names: Vec<(String, bool)> = items.iter().map(|n| (n.clone(), false)).collect();
            if p.path.is_empty()
                && !items
                    .iter()
                    .any(|n| n.eq_ignore_ascii_case(SUGGESTED_FOLDER))
            {
                names.insert(0, (SUGGESTED_FOLDER.to_owned(), true));
            }
            for (i, (n, new)) in names.into_iter().enumerate() {
                let open = n.clone();
                list = list.child(
                    row_box(SharedString::from(format!("folder-{i}")))
                        .child(icon("folder", 14., t.text_muted))
                        .child(div().flex_1().child(n.clone()))
                        .when(new, |d| {
                            d.child(chip(t, Tone::Neutral, tr("New"), false))
                                .tooltip(tip(tr("Made by Cleis")))
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.folder_pick_open(open.clone(), new, cx)
                        }))
                        .test_support(),
                );
            }
        }
    }
    let file = if p.path.is_empty() {
        VAULT_FILE.to_owned()
    } else {
        format!("{} / {VAULT_FILE}", p.path.join(" / "))
    };
    dialog_box(t)
        .w(px(440.))
        .child(
            div()
                .text_size(TEXT_LG)
                .line_height(px(20.))
                .font_weight(W_MEDIUM)
                .child(tr("OneDrive — where to keep the vault")),
        )
        .child(field_label(t, tr("Account")))
        .child(value_box(t).child(p.grant.account.clone()))
        .child(field_label(t, tr("Folder")))
        .child(list)
        .when_some(p.error.clone(), |d, e| {
            d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e))
        })
        .child(field_label(t, tr("File")))
        .child(value_box(t).child(div().id("folder-file").child(file).test_support()))
        .child(
            div()
                .pt(px(8.))
                .flex()
                .justify_end()
                .gap(px(8.))
                .child(
                    button("folder-cancel", t, BtnKind::Ghost, false)
                        .child(tr("Cancel"))
                        .on_click(cx.listener(|this, _, _, cx| this.cancel_cloud(cx)))
                        .test_support(),
                )
                .child(
                    button("folder-use", t, BtnKind::Primary, false)
                        .child(icon("check", 14., t.primary_ink))
                        .child(tr("Use this folder"))
                        .on_click(cx.listener(|this, _, _, cx| this.folder_pick_use(cx)))
                        .test_support(),
                ),
        )
}

fn field_label(t: &Tokens, text: &str) -> gpui_kit::Div {
    div()
        .text_size(TEXT_XS)
        .font_weight(W_MEDIUM)
        .text_color(t.text_muted)
        .child(crate::i18n::tr_owned(text))
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

/// The place of a signed-in account: the vault's file in its app folder, or in the folder the
/// person chose (D-110).
pub fn place_of(c: Cloud, api: &str, account: &str, folder: Option<DriveFolder>) -> RemotePlace {
    match c {
        Cloud::Microsoft => RemotePlace::OneDrive {
            path: VAULT_FILE.into(),
            api: api.into(),
            account: account.into(),
            folder,
        },
        Cloud::Google => RemotePlace::GoogleDrive {
            name: VAULT_FILE.into(),
            api: api.into(),
            account: account.into(),
            folder,
        },
    }
}

impl CleisApp {
    /// Settings › Account or the first launch's "Sign in" (36b, 36d): the provider's page in the
    /// browser, Cleis waiting on this computer for its answer.
    pub fn start_cloud(&mut self, c: Cloud, window: &mut Window, cx: &mut Context<Self>) {
        self.start_cloud_with(c, false, window, cx)
    }

    /// `custom` (D-110): the person chooses the folder (Google's picker, OneDrive's folders).
    pub fn start_cloud_with(
        &mut self,
        c: Cloud,
        custom: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(old) = self.cloud.as_ref().and_then(|s| s.cancel.clone()) {
            old.store(true, Ordering::Relaxed);
        }
        let e = self.cloud_hosts.of(c).clone();
        if !e.usable(c) {
            self.toast(tr("Not available in this build"), cx);
            return;
        }
        let round = self.cloud.as_ref().map_or(0, |s| s.round) + 1;
        let pending = match cloud::prepare(c, &e, "", custom) {
            Ok(p) => p,
            Err(err) => {
                self.cloud = Some(CloudSignIn {
                    cloud: c,
                    url: None,
                    cancel: None,
                    error: Some(crate::i18n::tr_dynamic(&err).into()),
                    finishing: false,
                    round,
                    custom,
                    pick: None,
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
            custom,
            pick: None,
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
                    Ok(g) if custom => {
                        s.finishing = true;
                        cx.notify();
                        this.cloud_signed_custom(c, &api, g, cx);
                    }
                    Ok(g) => {
                        s.finishing = true;
                        cx.notify();
                        this.cloud_signed(c, &api, g, None, cx);
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
    fn cloud_signed(
        &mut self,
        c: Cloud,
        api: &str,
        g: cloud::Grant,
        folder: Option<DriveFolder>,
        cx: &mut Context<Self>,
    ) {
        let place = place_of(c, api, &g.account, folder);
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

    /// D-110: Google: the folder picked in its window is read (name, account), then used;
    /// OneDrive: its folders are listed to choose from (mockup 49k).
    fn cloud_signed_custom(
        &mut self,
        c: Cloud,
        api: &str,
        g: cloud::Grant,
        cx: &mut Context<Self>,
    ) {
        match c {
            Cloud::Google => {
                let Some(id) = g.picked.first().cloned() else {
                    if let Some(s) = self.cloud.as_mut() {
                        s.finishing = false;
                        s.error = Some(tr("No folder chosen in Google Drive").into());
                    }
                    cx.notify();
                    return;
                };
                let (a, token) = (api.to_owned(), g.token.clone());
                let task =
                    cx.background_spawn(async move { cloud::gdrive_picked(&a, &id, &token) });
                let api = api.to_owned();
                cx.spawn(async move |this, cx| {
                    let r = task.await;
                    this.update(cx, |this, cx| match r {
                        Ok((folder, account)) => {
                            let mut g = g;
                            if g.account.is_empty() {
                                g.account = account;
                            }
                            this.cloud_signed(c, &api, g, Some(folder), cx);
                        }
                        Err(e) => {
                            if let Some(s) = this.cloud.as_mut() {
                                s.finishing = false;
                                s.error = Some(crate::i18n::tr_dynamic(&e.to_string()).into());
                            }
                            cx.notify();
                        }
                    })
                    .ok();
                })
                .detach();
            }
            Cloud::Microsoft => {
                if let Some(s) = self.cloud.as_mut() {
                    s.pick = Some(FolderPick {
                        api: api.to_owned(),
                        grant: g,
                        path: vec![],
                        items: None,
                        error: None,
                    });
                }
                self.folder_pick_load(cx);
            }
        }
    }

    /// Reads the folders of the folder shown (mockup 49k).
    fn folder_pick_load(&mut self, cx: &mut Context<Self>) {
        let Some(p) = self.cloud.as_mut().and_then(|s| s.pick.as_mut()) else {
            return;
        };
        p.items = None;
        p.error = None;
        let (api, path, token) = (p.api.clone(), p.path.join("/"), p.grant.token.clone());
        let shown = path.clone();
        let task = cx.background_spawn(async move { cloud::onedrive_folders(&api, &path, &token) });
        cx.notify();
        cx.spawn(async move |this, cx| {
            let r = task.await;
            this.update(cx, |this, cx| {
                let Some(p) = this.cloud.as_mut().and_then(|s| s.pick.as_mut()) else {
                    return;
                };
                // An answer for a folder no longer shown is dropped.
                if p.path.join("/") != shown {
                    return;
                }
                match r {
                    Ok(v) => p.items = Some(v),
                    Err(e) => {
                        p.items = Some(vec![]);
                        p.error = Some(crate::i18n::tr_dynamic(&e.to_string()).into());
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn folder_pick_open(&mut self, name: String, new: bool, cx: &mut Context<Self>) {
        let Some(p) = self.cloud.as_mut().and_then(|s| s.pick.as_mut()) else {
            return;
        };
        p.path.push(name);
        if new {
            // Not there yet: made when the vault is sent; nothing in it to list.
            p.items = Some(vec![]);
            p.error = None;
            cx.notify();
        } else {
            self.folder_pick_load(cx);
        }
    }

    pub fn folder_pick_up(&mut self, cx: &mut Context<Self>) {
        let Some(p) = self.cloud.as_mut().and_then(|s| s.pick.as_mut()) else {
            return;
        };
        p.path.pop();
        self.folder_pick_load(cx);
    }

    /// "Use this folder": the vault goes there (or is found there).
    pub fn folder_pick_use(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.cloud.as_mut() else { return };
        let Some(p) = s.pick.take() else { return };
        s.finishing = true;
        let folder = DriveFolder {
            id: p.path.join("/"),
            name: if p.path.is_empty() {
                "OneDrive".into()
            } else {
                p.path.join(" / ")
            },
        };
        self.cloud_signed(Cloud::Microsoft, &p.api, p.grant, Some(folder), cx);
        cx.notify();
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
