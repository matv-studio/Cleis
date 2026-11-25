//! Screen 03 — First launch: choose local vault (server option comes with cleis-server),
//! then create the master password.

use super::unlock::{logo, stage};
use crate::app::{CleisApp, Phase};
use crate::theme::*;
use crate::ui::*;
use gpui_kit::TestSupportExt;
use gpui_kit::component::input::Input;
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, Window, div, px};

fn choice(t: &Tokens, active: bool, title: &str, sub: &str, ico: &str) -> Div {
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
                .child(div().font_weight(W_MEDIUM).child(title.to_owned()))
                .child(
                    div()
                        .text_size(TEXT_SM)
                        .text_color(t.text_muted)
                        .child(sub.to_owned()),
                ),
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

pub fn render(
    app: &mut CleisApp,
    _window: &mut Window,
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    let t = app.tokens(cx).clone();
    let Phase::Welcome(w) = &app.phase else {
        unreachable!()
    };
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
                        .font_weight(W_STRONG)
                        .child("Welcome to Cleis"),
                ),
        )
        .child(
            choice(&t, true, "This computer only", "Personal vault", "laptop").child(div()), // selected
        )
        .child(
            div()
                .id("org-server")
                .opacity(0.45)
                .tooltip(|window, cx| {
                    gpui_kit::component::tooltip::Tooltip::new("Coming with the Cleis server")
                        .build(window, cx)
                })
                .child(choice(
                    &t,
                    false,
                    "Add an organization server",
                    "Shared spaces, on top of your personal vault",
                    "server",
                )),
        )
        .child(
            section(&t)
                .child(field(
                    &t,
                    "Master password",
                    Input::new(&w.password).id("new-password").mask_toggle(),
                ))
                .child(field(
                    &t,
                    "Confirm",
                    Input::new(&w.confirm).id("confirm-password").mask_toggle(),
                ))
                .when_some(w.error.clone(), |d, e| {
                    d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e))
                }),
        )
        .child(
            div().flex().justify_end().child(
                button("create", &t, BtnKind::Primary, false)
                    .when(busy, |b| b.opacity(0.6))
                    .child(if busy { "Creating…" } else { "Create vault" })
                    .on_click(cx.listener(|this, _, window, cx| this.create_vault(window, cx)))
                    .test_support(),
            ),
        );
    stage(&t, card)
}
