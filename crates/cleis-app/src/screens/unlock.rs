//! Screen 04 — Unlock.

use crate::app::{CleisApp, Phase, Unlock};
use crate::i18n::tr;
use crate::theme::*;
use crate::ui::*;
use gpui_kit::TestSupportExt;
use gpui_kit::component::input::Input;
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, Window, div, px};

/// Full-window centred stage, shared by the first-launch and unlock screens.
pub fn stage(t: &Tokens, card: impl IntoElement) -> Div {
    div()
        .size_full()
        .bg(t.chrome)
        .font_family(FONT)
        .text_size(TEXT_MD)
        .text_color(t.text)
        .p(px(6.))
        .child(
            div()
                .size_full()
                .bg(t.bg)
                .border_1()
                .border_color(t.stage_line)
                .rounded(px(10.))
                .flex()
                .items_center()
                .justify_center()
                .child(card),
        )
}

/// The Cleis logo (design/logo), in the version of the theme.
pub fn logo(t: &Tokens, size: f32) -> Div {
    use std::sync::{Arc, LazyLock};
    static DARK: LazyLock<Arc<gpui_kit::Image>> = LazyLock::new(|| {
        Arc::new(gpui_kit::Image::from_bytes(
            gpui_kit::ImageFormat::Png,
            include_bytes!("../../assets/logo-dark.png").to_vec(),
        ))
    });
    static LIGHT: LazyLock<Arc<gpui_kit::Image>> = LazyLock::new(|| {
        Arc::new(gpui_kit::Image::from_bytes(
            gpui_kit::ImageFormat::Png,
            include_bytes!("../../assets/logo-light.png").to_vec(),
        ))
    });
    let image = match t.mode {
        crate::theme::Mode::Dark => DARK.clone(),
        crate::theme::Mode::Light => LIGHT.clone(),
    };
    div()
        .flex_none()
        .size(px(size))
        .child(gpui_kit::img(image).size(px(size)))
}

/// The ways into the vault (D-76): Windows Hello or "Open" on this computer, the master password,
/// or the recovery key.
fn ways_in(u: &Unlock, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    use crate::opening::Mode;
    let busy = u.busy;
    let error = u
        .error
        .clone()
        .map(|e| crate::i18n::tr_dynamic(&e))
        .map(|e| div().text_size(TEXT_SM).text_color(t.danger_text).child(e));
    let link = |id: &'static str, label: &'static str, cx: &mut Context<CleisApp>| {
        div()
            .id(id)
            .text_size(TEXT_SM)
            .text_color(t.text_muted)
            .cursor_pointer()
            .hover(|d| d.text_color(t.text))
            .child(tr(label))
            .on_click(cx.listener(move |this, _, window, cx| {
                if let Phase::Unlock(u) = &mut this.phase {
                    u.recovery = !u.recovery;
                    u.error = None;
                    let focus = if u.recovery {
                        &u.recovery_key
                    } else {
                        &u.password
                    };
                    focus.update(cx, |s, cx| s.focus(window, cx));
                }
                cx.notify();
            }))
            .test_support()
    };
    let mut d = div().flex().flex_col().gap(px(12.));
    if u.recovery {
        return d
            .child(field(
                t,
                "Recovery key",
                Input::new(&u.recovery_key).id("recovery-key"),
            ))
            .children(error)
            .child(
                button("unlock-recovery", t, BtnKind::Primary, false)
                    .h(px(32.))
                    .w_full()
                    .when(busy, |b| b.opacity(0.6))
                    .child(tr("Open"))
                    .on_click(cx.listener(|this, _, window, cx| this.unlock_recovery(window, cx)))
                    .test_support(),
            )
            .child(
                div()
                    .flex()
                    .justify_center()
                    .child(link("use-password", "Back", cx)),
            );
    }
    // This computer's way first: one click.
    let quick = match u.mode {
        Mode::Hello => Some(("unlock-hello", "fingerprint", "Unlock with Windows Hello")),
        Mode::Nothing => Some(("unlock-device", "laptop", "Open")),
        Mode::Password => None,
    };
    if let Some((id, ico, label)) = quick {
        let hello = u.mode == Mode::Hello;
        d = d.child(
            button(id, t, BtnKind::Primary, false)
                .h(px(32.))
                .w_full()
                .when(busy, |b| b.opacity(0.6))
                .child(icon(ico, 16., t.primary_ink))
                .child(tr(label))
                .on_click(cx.listener(move |this, _, window, cx| {
                    if hello {
                        this.unlock_hello(window, cx)
                    } else {
                        this.unlock_device(window, cx)
                    }
                }))
                .test_support(),
        );
    }
    if u.has_password {
        d = d
            .child(field(
                t,
                "Master password",
                Input::new(&u.password).id("master-password").mask_toggle(),
            ))
            .child(
                button(
                    "unlock",
                    t,
                    if quick.is_some() {
                        BtnKind::Default
                    } else {
                        BtnKind::Primary
                    },
                    false,
                )
                .h(px(32.))
                .w_full()
                .when(busy && quick.is_none(), |b| b.opacity(0.6))
                .child(if busy && quick.is_none() {
                    tr("Unlocking…")
                } else {
                    tr("Unlock")
                })
                .on_click(cx.listener(|this, _, window, cx| this.unlock(window, cx)))
                .test_support(),
            );
    }
    d.children(error)
        .child(div().flex().justify_center().child(link(
            "use-recovery",
            "Use the recovery key",
            cx,
        )))
}

pub fn render(
    app: &mut CleisApp,
    _window: &mut Window,
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    let t = app.tokens(cx).clone();
    let Phase::Unlock(u) = &app.phase else {
        unreachable!()
    };
    let file = app
        .path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();
    let body = ways_in(u, &t, cx);
    let card = div()
        .w(px(360.))
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
                        .font_weight(W_STRONG)
                        .child(tr("Cleis")),
                ),
        )
        .child(
            section(&t)
                .gap(px(16.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .child(avatar(&t, "", 32.).child(icon("user", 16., t.text_muted)))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(div().font_weight(W_MEDIUM).child(tr("My vault")))
                                .child(
                                    div()
                                        .text_size(TEXT_SM)
                                        .text_color(t.text_muted)
                                        .child(file),
                                ),
                        ),
                )
                .child(body),
        );
    stage(&t, card)
}
