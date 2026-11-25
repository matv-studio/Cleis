//! Screen 12 — Health report.

use crate::app::{CleisApp, Session};
use crate::theme::*;
use crate::ui::*;
use cleis_core::health::{Issue, Policy, scan};
use gpui_kit::prelude::*;
use gpui_kit::{Context, Hsla, div, px};

pub fn page(s: &Session, t: &Tokens, _cx: &mut Context<CleisApp>) -> impl IntoElement + use<> {
    let (findings, sum) = scan(&s.entries, &Policy::default(), cleis_core::now());
    let stat = |n: usize, label: &str, color: Hsla| {
        section(t)
            .gap(px(2.))
            .flex_1()
            .child(
                div()
                    .text_size(TEXT_XL)
                    .font_weight(W_STRONG)
                    .text_color(color)
                    .child(n.to_string()),
            )
            .child(
                div()
                    .text_size(TEXT_SM)
                    .text_color(t.text_muted)
                    .child(label.to_owned()),
            )
    };
    let mut table = div().flex().flex_col();
    table = table.child(
        div()
            .h(px(30.))
            .flex()
            .items_center()
            .px(px(12.))
            .border_b_1()
            .border_color(t.line)
            .text_size(TEXT_XS)
            .font_weight(W_MEDIUM)
            .text_color(t.text_muted)
            .child(div().w(px(260.)).child("Item"))
            .child(div().flex_1().child("Issues")),
    );
    for f in findings.iter().take(40) {
        let Some(e) = s.entries.iter().find(|e| e.id == f.entry) else {
            continue;
        };
        let mut chips = div().flex().gap(px(4.));
        for i in &f.issues {
            chips = chips.child(match i {
                Issue::Weak => chip(t, Tone::Danger, "Weak", false),
                Issue::Reused { count } => chip(t, Tone::Warn, format!("Reused ×{count}"), false),
                Issue::Old => chip(t, Tone::Warn, "Over a year", false),
                Issue::NoTwoFactor => chip(t, Tone::Neutral, "No 2FA", false),
            });
        }
        table = table.child(
            div()
                .h(ROW_H)
                .flex()
                .items_center()
                .px(px(12.))
                .border_b_1()
                .border_color(t.line)
                .child(
                    div()
                        .w(px(260.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(site_icon(t, e.name.chars().next().unwrap_or('?'), 20.))
                        .child(div().font_weight(W_MEDIUM).child(e.name.clone())),
                )
                .child(chips),
        );
    }
    div()
        .id("health")
        .flex_1()
        .overflow_y_scroll()
        .px(px(32.))
        .py(px(24.))
        .child(
            div()
                .max_w(px(1100.))
                .mx_auto()
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(
                    div()
                        .text_size(TEXT_XL)
                        .font_weight(W_STRONG)
                        .child("Health report"),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(12.))
                        .child(stat(sum.weak, "Weak", t.danger_text))
                        .child(stat(sum.reused, "Reused", t.warning_text))
                        .child(stat(sum.old, "Over a year old", t.warning_text))
                        .child(stat(sum.no_two_factor, "No 2FA", t.text))
                        .child(stat(sum.ok, "No issue", t.success_text)),
                )
                .child(section(t).p_0().overflow_hidden().child(table)),
        )
}
