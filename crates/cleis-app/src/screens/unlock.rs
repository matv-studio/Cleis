//! Screen 04 — Unlock.

use crate::app::{CleisApp, Phase};
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

pub fn logo(t: &Tokens, size: f32) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .size(px(size))
        .rounded(px(if size > 30. { 10. } else { 6. }))
        .bg(t.inverse)
        .child(icon(
            "key",
            if size > 30. { 20. } else { 14. },
            t.inverse_ink,
        ))
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
    let busy = u.busy;
    let bio = u.bio;
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
                        .child("Cleis"),
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
                                .child(div().font_weight(W_MEDIUM).child("My vault"))
                                .child(
                                    div()
                                        .text_size(TEXT_SM)
                                        .text_color(t.text_muted)
                                        .child(file),
                                ),
                        ),
                )
                .child(field(
                    &t,
                    "Master password",
                    Input::new(&u.password).id("master-password").mask_toggle(),
                ))
                .when_some(u.error.clone(), |d, e| {
                    d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e))
                })
                .child(
                    button("unlock", &t, BtnKind::Primary, false)
                        .h(px(32.))
                        .w_full()
                        .when(busy, |b| b.opacity(0.6))
                        .child(if busy { "Unlocking…" } else { "Unlock" })
                        .on_click(cx.listener(|this, _, window, cx| this.unlock(window, cx)))
                        .test_support(),
                )
                .when(bio, |d| {
                    d.child(
                        div().flex().justify_center().gap(px(8.)).child(
                            button("bio", &t, BtnKind::Ghost, true)
                                .child(icon("fingerprint", 16., t.text_muted))
                                .child(crate::bio::name())
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.unlock_bio(window, cx)),
                                )
                                .test_support(),
                        ),
                    )
                }),
        );
    stage(&t, card)
}
