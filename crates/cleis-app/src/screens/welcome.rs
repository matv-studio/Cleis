//! Screen 03 — First launch: a vault on this computer only, or also an organization's server
//! (address and invitation code), then the master password.

use super::unlock::{logo, stage};
use crate::app::{CleisApp, Phase};
use crate::i18n::tr;
use crate::sync::{Provider, find_vaults};
use crate::theme::*;
use crate::ui::*;
use gpui_kit::TestSupportExt;
use gpui_kit::component::input::Input;
use gpui_kit::component::input::InputState;
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, Entity, PathPromptOptions, SharedString, Window, div, px};
use std::path::PathBuf;

/// Screen 22: an existing vault from a sync place, opened with its master password.
pub struct Restore {
    pub provider: Option<Provider>,
    pub dir: Option<PathBuf>,
    pub found: Vec<PathBuf>,
    pub chosen: usize,
    pub password: Entity<InputState>,
    pub error: Option<SharedString>,
    pub busy: bool,
    /// GitHub / SFTP (D-65): the place being set up, then the one read.
    pub place: Option<super::places::PlaceDialog>,
    pub remote: Option<(crate::remote::RemotePlace, crate::remote::Secret)>,
}

fn tip(
    text: &'static str,
) -> impl Fn(&mut Window, &mut gpui_kit::App) -> gpui_kit::AnyView + 'static {
    move |window, cx| {
        gpui_kit::component::tooltip::Tooltip::new(crate::i18n::tr_dynamic(text)).build(window, cx)
    }
}

fn tile(
    id: &'static str,
    t: &Tokens,
    ico: &str,
    label: &str,
    active: bool,
) -> gpui_kit::Stateful<Div> {
    let fg = if active { t.accent_text } else { t.text_muted };
    div()
        .id(id)
        .flex_1()
        .h(px(64.))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(6.))
        .px(px(4.))
        .bg(if active { t.accent_soft } else { t.bg })
        .border_1()
        .border_color(if active { t.accent_text } else { t.line_strong })
        .rounded(RADIUS_LG)
        .text_size(TEXT_SM)
        .font_weight(W_MEDIUM)
        .text_color(fg)
        .cursor_pointer()
        .child(icon(ico, 18., fg))
        .child(crate::i18n::tr_owned(label))
}

fn restore_card(r: &Restore, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let pick = |id: &'static str,
                ico: &str,
                label: &str,
                p: Provider,
                tip_text: &'static str,
                cx: &mut Context<CleisApp>| {
        tile(id, t, ico, label, r.provider == Some(p))
            .tooltip(tip(tip_text))
            .on_click(cx.listener(move |this, _, _, cx| this.restore_choose(p, cx)))
            .test_support()
    };
    let remote =
        |id: &'static str, ico: &str, label: &str, p: Provider, cx: &mut Context<CleisApp>| {
            tile(id, t, ico, label, r.provider == Some(p))
                .on_click(cx.listener(move |this, _, window, cx| this.open_place(p, window, cx)))
                .test_support()
        };
    let tiles = div()
        .flex()
        .gap(px(8.))
        .child(remote("t-github", "repo", "GitHub", Provider::GitHub, cx))
        .child(pick(
            "t-onedrive",
            "cloud",
            "OneDrive",
            Provider::OneDrive,
            "Folder kept by the OneDrive app",
            cx,
        ))
        .child(pick(
            "t-gdrive",
            "cloud",
            "Google Drive",
            Provider::GoogleDrive,
            "Folder kept by Google Drive for desktop",
            cx,
        ))
        .child(pick(
            "t-folder",
            "folder",
            "Folder",
            Provider::Folder,
            "Network share, Dropbox, USB drive",
            cx,
        ))
        .child(remote("t-sftp", "server", "SFTP", Provider::Sftp, cx));
    let vault_box = match r.found.get(r.chosen) {
        Some(p) => {
            let when = std::fs::metadata(p)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| crate::format::date(d.as_secs() as i64))
                .unwrap_or_default();
            value_box(t)
                .child(icon("file", 14., t.text_muted))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .font_family(FONT_MONO)
                        .text_size(TEXT_SM)
                        .child(p.display().to_string()),
                )
                .child(
                    div()
                        .text_size(TEXT_SM)
                        .text_color(t.text_muted)
                        .child(when),
                )
        }
        None => value_box(t)
            .text_color(t.text_faint)
            .child(match (&r.dir, r.provider) {
                (Some(_), _) => "No Cleis vault in this folder",
                (None, Some(_)) => "Choose the folder",
                (None, None) => "Choose where it is",
            }),
    };
    let busy = r.busy;
    div()
        .w(px(520.))
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(12.))
                .child(logo(t, 40.))
                .child(
                    div()
                        .text_size(TEXT_XL).line_height(px(26.))
                        .font_weight(W_STRONG)
                        .child(tr("Restore my vault")),
                ),
        )
        .child(
            section(t)
                .child(
                    div()
                        .id("restore-where")
                        .child(section_title(t, tr("Where is it?")))
                        .tooltip(tip(tr("Cleis opens your encrypted vault there. The provider never sees your passwords."))),
                )
                .child(tiles)
                .child(field(t, tr("Vault"), vault_box)),
        )
        .child(section(t).child(field(
            t,
            "Master password or recovery key",
            Input::new(&r.password).id("restore-password").mask_toggle(),
        )))
        .when_some(r.error.clone().map(|e| crate::i18n::tr_dynamic(&e)), |d, e| {
            d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e))
        })
        .child(
            div()
                .flex()
                .justify_between()
                .child(
                    button("restore-back", t, BtnKind::Ghost, false)
                        .child(tr("Back"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Phase::Welcome(w) = &mut this.phase {
                                w.restore = None;
                            }
                            cx.notify();
                        }))
                        .test_support(),
                )
                .child(
                    button("restore-go", t, BtnKind::Primary, false)
                        .when(busy || r.found.is_empty(), |b| b.opacity(0.6))
                        .child(if busy { tr("Restoring…") } else { tr("Restore") })
                        .on_click(cx.listener(|this, _, window, cx| this.restore_vault(window, cx)))
                        .test_support(),
                ),
        )
}

pub(crate) fn choice(t: &Tokens, active: bool, title: &str, sub: &str, ico: &str) -> Div {
    choice_with(t, active, title, sub, ico, None)
}

pub(crate) fn choice_with(
    t: &Tokens,
    active: bool,
    title: &str,
    sub: &str,
    ico: &str,
    extra: Option<Div>,
) -> Div {
    div()
        .flex()
        .gap(px(12.))
        .items_start()
        .p(px(16.))
        .rounded(RADIUS_LG)
        .border_1()
        .border_color(if active { t.accent_text } else { t.line_strong })
        .bg(t.surface_raised)
        .child(
            div()
                .mt(px(2.))
                .size(px(16.))
                .rounded_full()
                .border(px(if active { 5. } else { 1.5 }))
                .border_color(if active { t.accent_text } else { t.text_faint }),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .flex_1()
                .child(
                    div()
                        .font_weight(W_MEDIUM)
                        .child(crate::i18n::tr_owned(title)),
                )
                // Settings show no sentence under a choice (charter §1.5): an empty `sub` is left
                // out, the caller puts it in a tooltip (27/11/2026).
                .when(!sub.is_empty(), |d| {
                    d.child(
                        div()
                            .text_size(TEXT_SM)
                            .text_color(t.text_muted)
                            .child(crate::i18n::tr_owned(sub)),
                    )
                })
                .when_some(extra, |d, e| d.child(div().pt(px(12.)).child(e))),
        )
        .child(
            div()
                .size(px(32.))
                .rounded(RADIUS_LG)
                .bg(t.surface_active)
                .flex()
                .items_center()
                .justify_center()
                .child(icon(ico, 16., t.text)),
        )
}

/// Mockup 35i: how this computer will open the vault.
fn opening_choice(
    now: crate::opening::Mode,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    use crate::opening::Mode;
    let hello_ok = crate::opening::hello_available();
    let mut seg = div()
        .flex()
        .flex_none()
        .p(px(2.))
        .gap(px(2.))
        .bg(t.bg)
        .border_1()
        .border_color(t.line_strong)
        .rounded(RADIUS_MD);
    for (mode, label, why) in [
        (Mode::Password, "Password", None),
        (
            Mode::Hello,
            "Windows Hello",
            Some(if hello_ok {
                "Face, fingerprint or PIN of this Windows account"
            } else {
                "Windows Hello is not set up on this computer"
            }),
        ),
        (
            Mode::Nothing,
            "Nothing",
            Some("Anyone using this Windows account opens Cleis"),
        ),
    ] {
        let on = now == mode;
        let allowed = mode != Mode::Hello || hello_ok;
        seg = seg.child(
            div()
                .id(SharedString::from(format!("first-opening-{mode:?}")))
                .h(px(24.))
                .px(px(10.))
                .flex()
                .items_center()
                .rounded(RADIUS_SM)
                .font_weight(W_MEDIUM)
                .when(on, |d| d.bg(t.accent_soft).text_color(t.accent_text))
                .when(!on, |d| d.text_color(t.text_muted))
                .when(allowed, |d| d.cursor_pointer())
                .when(!allowed, |d| d.opacity(0.45))
                .child(tr(label))
                .when_some(why, |d, w| d.tooltip(tip(tr(w))))
                .when(allowed, |d| {
                    d.on_click(cx.listener(move |this, _, _, cx| {
                        if let Phase::Welcome(w) = &mut this.phase {
                            w.opening = mode;
                            w.error = None;
                        }
                        cx.notify();
                    }))
                })
                .test_support(),
        );
    }
    div()
        .flex()
        .items_center()
        .gap(px(12.))
        .child(div().flex_1().child(tr("Unlock with")))
        .child(seg)
}

pub fn render(
    app: &mut CleisApp,
    _window: &mut Window,
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    let t = app.tokens(cx).clone();
    let Phase::Welcome(w) = &app.phase else {
        unreachable!()
    };
    if let Some(r) = &w.restore {
        let dlg = r
            .place
            .as_ref()
            .map(|d| super::places::place_dialog(d, &t, cx));
        return stage(&t, restore_card(r, &t, cx))
            .relative()
            .when_some(dlg, |d, x| d.child(x));
    }
    let busy = w.busy;
    let card = div()
        .w(px(460.))
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(12.))
                .child(logo(&t, 40.))
                .child(
                    div()
                        .text_size(TEXT_XL)
                        .line_height(px(26.))
                        .font_weight(W_STRONG)
                        .child(tr("Welcome to Cleis")),
                ),
        )
        .child(
            div()
                .id("local-only")
                .cursor_pointer()
                .child(choice(
                    &t,
                    !w.org,
                    "This computer only",
                    "Personal vault",
                    "laptop",
                ))
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Phase::Welcome(w) = &mut this.phase {
                        w.org = false;
                        w.error = None;
                    }
                    cx.notify();
                }))
                .test_support(),
        )
        // Mockup 36d: the vault from another computer, through an account (D-80).
        .child({
            let acc = |id: &'static str,
                       ico: &str,
                       label: &str,
                       planned: Option<&'static str>,
                       cx: &mut Context<CleisApp>| {
                button(id, &t, BtnKind::Default, true)
                    .child(icon(ico, 14., t.text))
                    .child(crate::i18n::tr_owned(label))
                    .when_some(planned, |b, why| b.opacity(0.45).tooltip(tip(tr(why))))
                    .when(planned.is_none(), |b| {
                        b.on_click(cx.listener(|this, _, window, cx| {
                            this.sign_in_elsewhere(Provider::GitHub, window, cx)
                        }))
                    })
                    .test_support()
            };
            let buttons = div()
                .flex()
                .gap(px(8.))
                .child(acc(
                    "signin-microsoft",
                    "window",
                    "Microsoft",
                    Some("Available once Cleis is registered with Microsoft"),
                    cx,
                ))
                .child(acc(
                    "signin-google",
                    "globe",
                    "Google",
                    Some("Available once Cleis is registered with Google"),
                    cx,
                ))
                .child(acc("signin-github", "repo", "GitHub", None, cx));
            div().id("sign-in").child(choice_with(
                &t,
                false,
                tr("Sign in"),
                "My vault from my other computers",
                "user",
                Some(buttons),
            ))
        })
        .child(
            div()
                .id("org-server")
                .cursor_pointer()
                .child(choice_with(
                    &t,
                    w.org,
                    tr("Join an organization"),
                    "Shared spaces, on top of your personal vault",
                    "server",
                    w.org.then(|| super::org::join_fields(&w.join, &t, cx)),
                ))
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Phase::Welcome(w) = &mut this.phase
                        && !w.org
                    {
                        w.org = true;
                        w.error = None;
                    }
                    cx.notify();
                }))
                .test_support(),
        )
        .child(
            section(&t)
                .child(opening_choice(w.opening, &t, cx))
                .when(w.opening == crate::opening::Mode::Password, |d| {
                    d.child(field(
                        &t,
                        "Master password",
                        Input::new(&w.password).id("new-password").mask_toggle(),
                    ))
                    .child(field(
                        &t,
                        "Confirm",
                        Input::new(&w.confirm).id("confirm-password").mask_toggle(),
                    ))
                })
                .when_some(
                    w.error.clone().map(|e| crate::i18n::tr_dynamic(&e)),
                    |d, e| d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e)),
                ),
        )
        .child(
            div()
                .flex()
                .justify_between()
                .child(
                    button("restore", &t, BtnKind::Ghost, false)
                        .child(icon("sync", 16., t.text_muted))
                        .child(tr("Restore my vault"))
                        .tooltip(tip(tr("From OneDrive, Google Drive or a folder")))
                        .on_click(cx.listener(|this, _, window, cx| this.start_restore(window, cx)))
                        .test_support(),
                )
                .child(
                    button("create", &t, BtnKind::Primary, false)
                        .when(busy, |b| b.opacity(0.6))
                        .tooltip(tip(tr(
                            "A recovery key is shown next: it opens the vault on another computer",
                        )))
                        .child(if busy {
                            tr("Creating…")
                        } else {
                            tr("Create vault")
                        })
                        .on_click(cx.listener(|this, _, window, cx| this.create_vault(window, cx)))
                        .test_support(),
                ),
        );
    stage(&t, card)
}

impl CleisApp {
    pub fn start_restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let password = cx.new(|cx| InputState::new(window, cx).masked(true));
        if let Phase::Welcome(w) = &mut self.phase {
            w.restore = Some(Restore {
                provider: None,
                dir: None,
                found: vec![],
                chosen: 0,
                password,
                error: None,
                busy: false,
                place: None,
                remote: None,
            });
        }
        cx.notify();
    }

    fn restore_choose(&mut self, p: Provider, cx: &mut Context<Self>) {
        if let Phase::Welcome(w) = &mut self.phase
            && let Some(r) = w.restore.as_mut()
        {
            r.provider = Some(p);
        }
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(tr("Open").into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = rx.await
                && let Some(dir) = paths.into_iter().next()
            {
                this.update(cx, |this, cx| this.restore_found(dir, cx)).ok();
            }
        })
        .detach();
        cx.notify();
    }

    /// Lists the vaults of the chosen folder (most recent first).
    pub fn restore_found(&mut self, dir: PathBuf, cx: &mut Context<Self>) {
        if let Phase::Welcome(w) = &mut self.phase
            && let Some(r) = w.restore.as_mut()
        {
            r.found = find_vaults(&dir);
            r.dir = Some(dir);
            r.chosen = 0;
            r.error = None;
        }
        cx.notify();
    }

    /// Copies the vault from the place to this computer, opens it, and keeps the place as its
    /// sync place. A wrong password leaves nothing behind.
    pub fn restore_vault(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Phase::Welcome(w) = &mut self.phase else {
            return;
        };
        let Some(r) = w.restore.as_mut() else { return };
        let Some(src) = r.found.get(r.chosen).cloned() else {
            return;
        };
        if r.busy {
            return;
        }
        let pw = r.password.read(cx).value().to_string();
        let provider = r.provider.unwrap_or_default();
        let remote = r.remote.clone();
        r.busy = true;
        r.error = None;
        cx.notify();
        let dest = self.path.clone();
        cx.spawn_in(window, async move |this, cx| {
            let d2 = dest.clone();
            let res = cx
                .background_executor()
                .spawn(async move {
                    if let Some(dir) = d2.parent() {
                        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
                    }
                    if d2.exists() {
                        return Err(tr("A vault already exists on this computer").to_owned());
                    }
                    std::fs::copy(&src, &d2).map_err(|e| e.to_string())?;
                    // The master password, or the recovery key typed in its place (D-76).
                    let v = cleis_store::LockedVault::open(&d2)
                        .and_then(|l| {
                            if cleis_store::parse_recovery(&pw).is_some() {
                                l.unlock_recovery(&pw).or_else(|_| {
                                    cleis_store::LockedVault::open(&d2).and_then(|l| l.unlock(&pw))
                                })
                            } else {
                                l.unlock(&pw)
                            }
                        })
                        .map_err(|e| e.to_string());
                    if v.is_err() {
                        let _ = std::fs::remove_file(&d2);
                    }
                    v.map(|v| (v, src))
                })
                .await;
            this.update_in(cx, |this, window, cx| match res {
                Ok((v, src)) => {
                    let mut cfg = crate::sync::SyncConfig {
                        place: Some(src.clone()),
                        provider,
                        ..Default::default()
                    };
                    if let Some((place, secret)) = remote {
                        let cache = crate::sync::remote_cache(&dest);
                        let _ = std::fs::rename(&src, &cache);
                        let _ = v.set_org_setting(&cfg.secret_key(), &Some(secret));
                        cfg.place = Some(cache);
                        cfg.remote = Some(place);
                    }
                    crate::sync::save_config(&dest, &cfg);
                    this.enter(v, window, cx);
                }
                Err(e) => {
                    if let Phase::Welcome(w) = &mut this.phase
                        && let Some(r) = w.restore.as_mut()
                    {
                        r.busy = false;
                        r.error = Some(e.into());
                    }
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }
}
