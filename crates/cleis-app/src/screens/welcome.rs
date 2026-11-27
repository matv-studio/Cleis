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
use gpui_kit::{
    AnyElement, Context, Div, ElementId, Entity, PathPromptOptions, SharedString, Window, div, px,
};
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
    /// Signed in from the first screen (mockup 40c): the account's vault is opened on the same
    /// card, not on this restore screen.
    pub signed: Option<String>,
    /// The key kept in the account beside a vault without a password (D-96): it opens here with
    /// nothing to type.
    pub account_key: Option<crate::remote::AccountKey>,
}

/// What the first screen starts with (mockups 40).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Start {
    Local,
    Account,
    Org,
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
            "Password or recovery key",
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
    // A name alone is centred on its icon; with a line or boxes under it, it stays at the top.
    let alone = sub.is_empty() && extra.is_none();
    div()
        .flex()
        .gap(px(12.))
        .when(alone, |d| d.items_center())
        .when(!alone, |d| d.items_start())
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
    let mut modes = vec![
        (Mode::Password, "Password", None),
        (
            Mode::Pin,
            "PIN",
            Some("Digits only, on this computer; the recovery key after too many wrong tries"),
        ),
    ];
    if let Some(name) = crate::opening::hello_name() {
        modes.push((Mode::Hello, name, Some(crate::opening::hello_tip(hello_ok))));
    }
    modes.push((
        Mode::Nothing,
        "Nothing",
        Some(crate::opening::nothing_tip()),
    ));
    for (mode, label, why) in modes {
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
                    d.on_click(cx.listener(move |this, _, window, cx| {
                        if let Phase::Welcome(w) = &mut this.phase {
                            w.opening = mode;
                            w.error = None;
                            if mode == Mode::Pin {
                                window.focus(&w.pin.focus, cx);
                            }
                        }
                        cx.notify();
                    }))
                })
                .test_support(),
        );
    }
    // The choices go under the label when both do not fit on one line (longer labels in French).
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_x(px(12.))
        .gap_y(px(10.))
        .child(div().flex_1().child(tr("Unlock Cleis with")))
        .child(seg)
}

/// One option of the first screen (mockups 40): radio, name, icon, and what it holds. A card
/// that can be chosen takes the click and the hover on its whole surface; once chosen, only its
/// head does, so the boxes and buttons it holds work as such.
fn option(
    id: &'static str,
    t: &Tokens,
    (active, clickable): (bool, bool),
    (title, tip_text, ico): (&str, &'static str, &str),
    extra: Option<AnyElement>,
    on_head: impl Fn(&mut CleisApp, &mut Window, &mut Context<CleisApp>) + 'static,
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    let line = if active { t.accent_text } else { t.line_strong };
    let whole = clickable && !active;
    let on_head = std::rc::Rc::new(on_head);
    let on_card = on_head.clone();
    div()
        .id(SharedString::from(format!("{id}-card")))
        .flex()
        .flex_col()
        .rounded(RADIUS_LG)
        .border_1()
        .border_color(line)
        .bg(t.surface_raised)
        // "Sign in" in the middle does nothing by itself (its providers do): no hover there.
        .when(whole, |d| {
            d.cursor_pointer()
                .hover(|d| d.bg(t.surface_hover))
                .on_click(cx.listener(move |this, _, window, cx| on_card(this, window, cx)))
        })
        .child(
            div()
                .id(id)
                .flex()
                .gap(px(12.))
                .items_center()
                .px(px(16.))
                .py(px(12.))
                .rounded(RADIUS_LG)
                .when(clickable, |d| d.cursor_pointer())
                .tooltip(tip(tr(tip_text)))
                .child(
                    div()
                        .size(px(16.))
                        .rounded_full()
                        .border(px(if active { 5. } else { 1.5 }))
                        .border_color(if active { t.accent_text } else { t.text_faint }),
                )
                .child(
                    div()
                        .flex_1()
                        .font_weight(W_MEDIUM)
                        .child(crate::i18n::tr_owned(title)),
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
                .when(clickable && active, |d| {
                    d.on_click(cx.listener(move |this, _, window, cx| on_head(this, window, cx)))
                })
                .test_support(),
        )
        .when_some(extra, |d, e| {
            d.child(div().px(px(16.)).pb(px(16.)).pl(px(44.)).child(e))
        })
        .test_support()
}

pub fn render(
    app: &mut CleisApp,
    window: &mut Window,
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    let t = app.tokens(cx).clone();
    let Phase::Welcome(w) = &app.phase else {
        unreachable!()
    };
    // Restoring from a folder or SFTP keeps its own card; signing in to an account stays here.
    if let Some(r) = w
        .restore
        .as_ref()
        .filter(|r| r.signed.is_none() && app.github.is_none())
    {
        let dlg = r
            .place
            .as_ref()
            .map(|d| super::places::place_dialog(d, &t, cx));
        return stage(&t, restore_card(r, &t, cx))
            .relative()
            .when_some(dlg, |d, x| d.child(x));
    }
    let opened = w.restore.as_ref().filter(|r| r.signed.is_some());
    let login = opened
        .and_then(|r| r.signed.clone())
        .or_else(|| app.github_after.as_ref().map(|n| n.login.clone()));
    let chosen = if opened.is_some() || app.github_after.is_some() {
        Some(Start::Account)
    } else {
        w.choice
    };
    let signing = app.github.is_some() && chosen.is_none();
    let busy = w.busy;
    let step2 = chosen.is_some();
    // The head of an option: in the middle it chooses; once up, it brings the options back down.
    let head = |c: Option<Start>| {
        move |this: &mut CleisApp, window: &mut Window, cx: &mut Context<CleisApp>| {
            let up = matches!(&this.phase, Phase::Welcome(w) if w.choice.is_some())
                || this.github_after.is_some()
                || matches!(&this.phase, Phase::Welcome(w) if w.restore.is_some());
            if up {
                this.welcome_back(cx);
            } else if let Some(c) = c {
                this.welcome_choose(c, window, cx);
            }
        }
    };
    let providers = |cx: &mut Context<CleisApp>| {
        let acc = |id: &'static str,
                   ico: &str,
                   label: &str,
                   planned: Option<&'static str>,
                   cx: &mut Context<CleisApp>| {
            button(id, &t, BtnKind::Default, true)
                .child(icon(ico, 14., t.text))
                .child(crate::i18n::tr_owned(label))
                .when(id == "signin-github" && signing, |b| {
                    b.border_color(t.focus)
                })
                .when_some(planned, |b, why| b.opacity(0.45).tooltip(tip(tr(why))))
                // The card under it also takes clicks once it is up: the button keeps its own.
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    if planned.is_none() {
                        this.sign_in_elsewhere(Provider::GitHub, window, cx)
                    }
                }))
                .test_support()
        };
        div()
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
            .child(acc("signin-github", "repo", "GitHub", None, cx))
            .into_any_element()
    };
    let account_row = |login: &str, cx: &mut Context<CleisApp>| {
        div()
            .id("signed-as")
            .flex()
            .items_center()
            .gap(px(8.))
            .child(avatar(
                &t,
                &login
                    .chars()
                    .next()
                    .unwrap_or('?')
                    .to_uppercase()
                    .to_string(),
                20.,
            ))
            .child(format!("@{login}"))
            .child(chip(&t, Tone::Ok, "GitHub", true))
            .child(div().flex_1())
            .child(
                button("signed-change", &t, BtnKind::Ghost, true)
                    .child(tr("Change"))
                    .tooltip(tip(tr("Another account or provider")))
                    .on_click(cx.listener(|this, _, _, cx| this.welcome_back(cx)))
                    .test_support(),
            )
            .test_support()
            .into_any_element()
    };
    let options = div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(option(
            "local-only",
            &t,
            (chosen == Some(Start::Local), true),
            (
                "Local account",
                "Your vault stays on this computer",
                "laptop",
            ),
            None,
            head(Some(Start::Local)),
            cx,
        ))
        .child(option(
            "sign-in",
            &t,
            (chosen == Some(Start::Account) || signing, step2),
            (
                "Sign in",
                "Your vault follows you to your other computers",
                "user",
            ),
            Some(match &login {
                Some(l) => account_row(l, cx),
                None => providers(cx),
            }),
            head(None),
            cx,
        ))
        .child(option(
            "org-server",
            &t,
            (chosen == Some(Start::Org), true),
            (
                "Join an organization",
                "Shared spaces, on top of your personal vault",
                "server",
            ),
            (chosen == Some(Start::Org))
                .then(|| super::org::join_fields(&w.join, &t, cx).into_any_element()),
            head(Some(Start::Org)),
            cx,
        ));
    // Moving up or down from where they were (mockups 40): half the height of what appears.
    let shift = (w.below_h.get().max(160.) + 16.) / 2.;
    let from = if w.up { shift } else { -shift };
    let options: AnyElement = if w.round == 0 {
        options.into_any_element()
    } else {
        use gpui_kit::AnimationExt;
        options
            .relative()
            .with_animation(
                ElementId::Name(format!("welcome-move-{}", w.round).into()),
                gpui_kit::Animation::new(std::time::Duration::from_millis(PAGE_MS + 40))
                    .with_easing(gpui_kit::ease_out_quint()),
                move |d, v| d.top(px((1. - v) * from)),
            )
            .into_any_element()
    };
    // Joining: how the vault opens appears once the invitation is checked, as after GitHub.
    let ready = chosen != Some(Start::Org) || w.join.info.is_some();
    let below: Option<AnyElement> = (step2 && ready).then(|| {
        let measure = w.below_h.clone();
        let body = match opened {
            Some(r) => open_here(r, login.as_deref().unwrap_or(""), &t, cx),
            None => create_here(w, app.github_after.is_some(), &t, window, cx),
        };
        crate::ui::page_in(
            format!("welcome-below-{}-{}", w.round, ready),
            div().flex().flex_col().gap(px(16.)).child(body).child(
                gpui_kit::canvas(
                    move |b, _, _| measure.set(f32::from(b.size.height)),
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            ),
        )
        .into_any_element()
    });
    let _ = busy;
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
        .child(options)
        .when_some(below, |d, b| d.child(b))
        .when(!step2, |d| {
            d.child(
                div().flex().child(
                    button("restore", &t, BtnKind::Ghost, false)
                        .child(icon("sync", 16., t.text_muted))
                        .child(tr("Restore my vault"))
                        .tooltip(tip(tr("From OneDrive, Google Drive, a folder or SFTP")))
                        .on_click(cx.listener(|this, _, window, cx| this.start_restore(window, cx)))
                        .test_support(),
                ),
            )
        });
    stage(&t, card)
}

/// Under the options once one is chosen (mockups 40, 40b): how this computer opens the vault,
/// then Create vault.
fn create_here(
    w: &crate::app::Welcome,
    to_account: bool,
    t: &Tokens,
    window: &Window,
    cx: &mut Context<CleisApp>,
) -> Div {
    let busy = w.busy;
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            section(t)
                .child(opening_choice(w.opening, t, cx))
                // Mockup 37h: the PIN's boxes, then the same again.
                .when(w.opening == crate::opening::Mode::Pin, |d| {
                    let get: super::pin::Get = |app| match &mut app.phase {
                        Phase::Welcome(w) => Some(&mut w.pin),
                        _ => None,
                    };
                    let get2: super::pin::Get = |app| match &mut app.phase {
                        Phase::Welcome(w) => Some(&mut w.pin_confirm),
                        _ => None,
                    };
                    let next: super::pin::Done = |app, window, cx| {
                        if let Phase::Welcome(w) = &app.phase {
                            window.focus(&w.pin_confirm.focus, cx);
                        }
                    };
                    let done: super::pin::Done = |app, window, cx| app.create_vault(window, cx);
                    d.child(super::pin::boxes(
                        "first-pin",
                        &w.pin,
                        t,
                        window,
                        get,
                        next,
                        cx,
                    ))
                    .child(
                        div()
                            .id("first-pin-confirm-label")
                            .flex()
                            .justify_center()
                            .text_size(TEXT_SM)
                            .text_color(t.text_muted)
                            .child(tr("Confirm"))
                            .tooltip(tip(tr("Typed a second time to confirm"))),
                    )
                    .child(super::pin::boxes(
                        "first-pin-confirm",
                        &w.pin_confirm,
                        t,
                        window,
                        get2,
                        done,
                        cx,
                    ))
                })
                .when(w.opening == crate::opening::Mode::Password, |d| {
                    d.child(field(
                        t,
                        "Password",
                        Input::new(&w.password).id("new-password").mask_toggle(),
                    ))
                    .child(field(
                        t,
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
            div().flex().justify_end().child(
                button("create", t, BtnKind::Primary, false)
                    .when(busy, |b| b.opacity(0.6))
                    .tooltip(tip(if to_account {
                        tr("Then saved to a private repository cleis-vault on your account")
                    } else {
                        tr("A recovery key is shown next: it opens the vault on another computer")
                    }))
                    .child(if busy {
                        tr("Creating…")
                    } else {
                        tr("Create vault")
                    })
                    .on_click(cx.listener(|this, _, window, cx| this.create_vault(window, cx)))
                    .test_support(),
            ),
        )
}

/// Signed in, a vault on the account (mockup 40c): opened here with its password or recovery
/// key; while it is looked for, a line says so.
fn open_here(r: &Restore, _login: &str, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let busy = r.busy;
    let place_err = r.place.as_ref().and_then(|p| p.error.clone());
    let label = r
        .remote
        .as_ref()
        .map(|(p, _)| match p {
            crate::remote::RemotePlace::GitHub { repo, .. } => repo.clone(),
            other => other.label(),
        })
        .unwrap_or_default();
    let found = !r.found.is_empty();
    let mut sec = section(t);
    if !found {
        sec = sec.child(match place_err {
            Some(e) => div()
                .text_size(TEXT_SM)
                .text_color(t.danger_text)
                .child(crate::i18n::tr_dynamic(&e))
                .into_any_element(),
            None => div()
                .id("finding-vault")
                .flex()
                .items_center()
                .gap(px(8.))
                .text_color(t.text_muted)
                .child(icon("sync", 14., t.accent_text))
                .child(tr("Finding your vault…"))
                .into_any_element(),
        });
        return div().flex().flex_col().child(sec);
    }
    sec = sec
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(icon("repo", 16., t.text_muted))
                .child(
                    div()
                        .flex_1()
                        .font_family(FONT_MONO)
                        .text_size(TEXT_SM)
                        .child(label),
                ),
        )
        // D-96: no password on this vault, its key kept in the account: nothing to type.
        .when(r.account_key.is_none(), |d| {
            d.child(field(
                t,
                "Password or recovery key",
                Input::new(&r.password).id("restore-password").mask_toggle(),
            ))
        })
        .when_some(r.error.clone(), |d, e| {
            d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e))
        });
    div().flex().flex_col().gap(px(16.)).child(sec).child(
        div().flex().justify_end().child(
            button("restore-go", t, BtnKind::Primary, false)
                .when(busy, |b| b.opacity(0.6))
                .child(if busy {
                    tr("Opening…")
                } else {
                    tr("Open vault")
                })
                .on_click(cx.listener(|this, _, window, cx| this.restore_vault(window, cx)))
                .test_support(),
        ),
    )
}

impl CleisApp {
    /// An option chosen in the middle of the first screen: the options move up and how the vault
    /// opens appears under them (mockups 40).
    pub fn welcome_choose(&mut self, c: Start, window: &mut Window, cx: &mut Context<Self>) {
        if let Phase::Welcome(w) = &mut self.phase {
            w.choice = Some(c);
            w.org = c == Start::Org;
            w.error = None;
            w.round += 1;
            w.up = true;
            if w.opening == crate::opening::Mode::Password && c == Start::Local {
                let pw = w.password.clone();
                pw.update(cx, |s, cx| s.focus(window, cx));
            }
        }
        cx.notify();
    }

    /// Changing one's mind: the options come back down, nothing under them.
    pub fn welcome_back(&mut self, cx: &mut Context<Self>) {
        self.github = None;
        self.github_after = None;
        if let Phase::Welcome(w) = &mut self.phase {
            w.choice = None;
            w.org = false;
            w.restore = None;
            w.error = None;
            w.round += 1;
            w.up = false;
        }
        cx.notify();
    }
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
                signed: None,
                account_key: None,
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
        let account_key = r.account_key.clone();
        let had_key = account_key.is_some();
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
                    // D-96: the key kept in the account; this computer then opens with nothing.
                    if let Some(k) = &account_key {
                        let v = k.open(&d2);
                        let v = v.and_then(|v| {
                            crate::opening::set_nothing(&d2, &v)?;
                            Ok(v)
                        });
                        if v.is_err() {
                            let _ = std::fs::remove_file(&d2);
                            crate::opening::forget(&d2);
                        }
                        return v.map(|v| (v, src));
                    }
                    // The password, or the recovery key typed in its place (D-76).
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
                        key_sent: had_key,
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
