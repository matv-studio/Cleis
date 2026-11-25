//! Charter components (design/maquettes/css/ui.css) built from GPUI primitives.

use crate::theme::*;
use gpui_kit::prelude::*;
use gpui_kit::{
    AnyElement, Div, ElementId, FontWeight, Hsla, SharedString, Stateful, div, px, svg,
};

pub const W_REGULAR: FontWeight = FontWeight(400.);
pub const W_MEDIUM: FontWeight = FontWeight(500.);
pub const W_STRONG: FontWeight = FontWeight(600.);

pub fn icon(name: &str, size: f32, color: Hsla) -> impl IntoElement {
    svg()
        .path(SharedString::from(format!("icons/{name}.svg")))
        .size(px(size))
        .flex_none()
        .text_color(color)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BtnKind {
    Default,
    Primary,
    Ghost,
    DangerGhost,
    /// Only inside a confirmation (charter §6).
    Danger,
}

/// `.btn`: 28 px, radius 6, medium weight; `small` = 24 px.
pub fn button(id: impl Into<ElementId>, t: &Tokens, kind: BtnKind, small: bool) -> Stateful<Div> {
    let (bg, border, fg, hover) = match kind {
        BtnKind::Default => (t.surface_raised, t.line_strong, t.text, t.surface_hover),
        BtnKind::Primary => (t.primary, t.primary, t.primary_ink, t.primary.opacity(0.92)),
        BtnKind::Ghost => (
            Hsla::transparent_black(),
            Hsla::transparent_black(),
            t.text_muted,
            t.surface_hover,
        ),
        BtnKind::DangerGhost => (
            Hsla::transparent_black(),
            Hsla::transparent_black(),
            t.danger_text,
            t.danger_bg,
        ),
        BtnKind::Danger => (
            t.danger_solid,
            t.danger_solid,
            gpui_kit::white(),
            t.danger_solid.opacity(0.9),
        ),
    };
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .gap(px(6.))
        .h(if small { CONTROL_SM } else { CONTROL_MD })
        .px(px(if small { 8. } else { 12. }))
        .rounded(RADIUS_MD)
        .border_1()
        .border_color(border)
        .bg(bg)
        .text_color(fg)
        .text_size(if small { TEXT_SM } else { TEXT_MD })
        .font_weight(W_MEDIUM)
        .whitespace_nowrap()
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
}

/// `.icon-btn`: square, transparent, muted icon.
pub fn icon_button(id: impl Into<ElementId>, t: &Tokens, name: &str, small: bool) -> Stateful<Div> {
    let size = if small { CONTROL_SM } else { CONTROL_MD };
    let hover = t.surface_hover;
    let text = t.text;
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(size)
        .rounded(RADIUS_MD)
        .cursor_pointer()
        .text_color(t.text_muted)
        .hover(move |s| s.bg(hover).text_color(text))
        .child(icon(name, 16., t.text_muted))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Ok,
    Warn,
    Danger,
    Set,
    Auto,
}

/// `.chip`: 20 px, xs text, tinted by functional colour.
pub fn chip(t: &Tokens, tone: Tone, label: impl Into<SharedString>, dot: bool) -> Div {
    let (fg, bg) = match tone {
        Tone::Neutral => (t.text_muted, t.surface_active),
        Tone::Ok => (t.success_text, t.success_text.opacity(0.14)),
        Tone::Warn => (t.warning_text, t.warning_bg),
        Tone::Danger => (t.danger_text, t.danger_bg),
        Tone::Set => (t.set_text, t.set_bg),
        Tone::Auto => (t.auto_text, t.auto_bg),
    };
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(5.))
        .h(px(20.))
        .px(px(8.))
        .rounded(RADIUS_SM)
        .bg(bg)
        .text_color(fg)
        .text_size(TEXT_XS)
        .font_weight(W_MEDIUM)
        .whitespace_nowrap()
        .when(dot, |d| d.child(div().size(px(6.)).rounded_full().bg(fg)))
        .child(label.into())
}

/// `.switch`: flat track with a thin handle (charter §6).
pub fn switch(id: impl Into<ElementId>, t: &Tokens, on: bool) -> Stateful<Div> {
    let track = div()
        .absolute()
        .left_0()
        .right_0()
        .top(px(if on { 6. } else { 7. }))
        .h(px(if on { 4. } else { 2. }))
        .rounded(px(2.))
        .bg(if on { t.primary } else { t.range_track_bg });
    let handle = if on {
        div()
            .absolute()
            .right_0()
            .top(px(1.))
            .w(px(3.))
            .h(px(14.))
            .rounded_full()
            .bg(gpui_kit::white())
            .border_1()
            .border_color(t.overlay)
    } else {
        div()
            .absolute()
            .left_0()
            .top(px(5.))
            .w(px(12.))
            .h(px(6.))
            .rounded_full()
            .bg(t.text_faint)
    };
    div()
        .id(id)
        .relative()
        .w(px(26.))
        .h(px(16.))
        .flex_none()
        .cursor_pointer()
        .child(track)
        .child(handle)
}

/// `.section`: raised card with title and rows.
pub fn section(t: &Tokens) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .px(px(20.))
        .py(px(16.))
        .bg(t.surface_raised)
        .border_1()
        .border_color(t.line)
        .rounded(RADIUS_LG)
}

pub fn section_title(t: &Tokens, title: &str) -> Div {
    div()
        .text_size(TEXT_LG)
        .font_weight(W_MEDIUM)
        .text_color(t.text)
        .child(title.to_owned())
}

/// `.row`: label on the left, controls on the right, divider above (except the first).
pub fn row(t: &Tokens, first: bool) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(12.))
        .min_h(CONTROL_MD)
        .when(!first, |d| d.border_t_1().border_color(t.line).pt(px(12.)))
}

/// `.segmented`.
pub fn segmented(id: &str, t: &Tokens, options: &[&str], selected: usize) -> Div {
    let mut d = div()
        .flex()
        .flex_none()
        .p(px(2.))
        .gap(px(2.))
        .bg(t.bg)
        .border_1()
        .border_color(t.line_strong)
        .rounded(RADIUS_MD);
    for (i, o) in options.iter().enumerate() {
        let on = i == selected;
        d = d.child(
            div()
                .id(SharedString::from(format!("{id}-{i}")))
                .h(px(22.))
                .px(px(12.))
                .flex()
                .items_center()
                .rounded(RADIUS_SM)
                .text_size(TEXT_MD)
                .font_weight(W_MEDIUM)
                .cursor_pointer()
                .when(on, |d| d.bg(t.accent_soft).text_color(t.accent_text))
                .when(!on, |d| d.text_color(t.text_muted))
                .child((*o).to_owned()),
        );
    }
    d
}

/// Label above a control, `.field > .label`.
pub fn field(t: &Tokens, label: &str, control: impl IntoElement) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(4.))
        .min_w_0()
        .child(
            div()
                .text_size(TEXT_XS)
                .text_color(t.text_muted)
                .font_weight(W_MEDIUM)
                .child(label.to_owned()),
        )
        .child(control)
}

/// Static, input-looking box for read-only values (`.input`).
pub fn value_box(t: &Tokens) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .h(CONTROL_MD)
        .px(px(8.))
        .min_w_0()
        .bg(t.bg)
        .border_1()
        .border_color(t.line_strong)
        .rounded(RADIUS_MD)
}

/// Square letter icon used for sites (`.site-ico`).
pub fn site_icon(t: &Tokens, letter: char, size: f32) -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(size))
        .rounded(px(if size > 24. { 8. } else { 5. }))
        .bg(t.surface_active)
        .text_color(t.text)
        .text_size(px(if size > 24. { 13. } else { 10. }))
        .font_weight(W_STRONG)
        .child(letter.to_uppercase().to_string())
}

/// Round avatar with initials.
pub fn avatar(t: &Tokens, initials: &str, size: f32) -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(size))
        .rounded_full()
        .bg(t.surface_active)
        .border_1()
        .border_color(t.line_strong)
        .text_color(t.text)
        .text_size(px(if size > 30. { 12. } else { 11. }))
        .font_weight(W_STRONG)
        .child(initials.to_owned())
}

/// TOTP countdown ring: a small conic fill approximated by a pie made of the
/// remaining fraction (GPUI has no conic gradient; a bar reads just as well at 14 px).
pub fn totp_ring(t: &Tokens, fraction: f32) -> AnyElement {
    let low = fraction < 0.25;
    let color = if low { t.warning_text } else { t.text_muted };
    div()
        .flex_none()
        .size(px(14.))
        .rounded_full()
        .border_2()
        .border_color(t.line_strong)
        .relative()
        .child(
            div()
                .absolute()
                .left(px(-2.))
                .bottom(px(-2.))
                .w(px(14.))
                .h(px(14. * fraction))
                .rounded_b_full()
                .bg(color)
                .opacity(0.85),
        )
        .overflow_hidden()
        .into_any_element()
}

/// `.kbd` key caps.
pub fn keys(t: &Tokens, ks: &[&str]) -> Div {
    let mut d = div().flex().gap(px(3.));
    for k in ks {
        d = d.child(
            div()
                .min_w(px(20.))
                .h(px(20.))
                .px(px(5.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(TEXT_XS)
                .font_weight(W_MEDIUM)
                .text_color(t.text_muted)
                .bg(t.bg)
                .border_1()
                .border_b_2()
                .border_color(t.line_strong)
                .rounded(RADIUS_SM)
                .child((*k).to_owned()),
        );
    }
    d
}

/// `.menu`: floating list (absolute; place it under its button).
pub fn menu_box(t: &Tokens) -> Div {
    div()
        .absolute()
        // Clicks on the menu must not reach what lies under it.
        .occlude()
        .min_w(px(220.))
        .p(px(4.))
        .flex()
        .flex_col()
        .bg(t.surface_raised)
        .border_1()
        .border_color(t.line_strong)
        .rounded(RADIUS_MD)
        .shadow_lg()
}

/// `.menu-item`.
pub fn menu_item(
    id: impl Into<SharedString>,
    t: &Tokens,
    icon_name: Option<&str>,
    label: impl Into<SharedString>,
    danger: bool,
) -> Stateful<Div> {
    let fg = if danger { t.danger_text } else { t.text };
    div()
        .id(ElementId::Name(id.into()))
        .h(px(28.))
        .px(px(8.))
        .flex()
        .items_center()
        .gap(px(8.))
        .rounded(RADIUS_SM)
        .cursor_pointer()
        .text_color(fg)
        .hover(|d| d.bg(t.surface_hover))
        .when_some(icon_name, |d, n| d.child(icon(n, 14., fg)))
        .child(label.into())
}

/// Audit U-12: the one dialog of the charter (§ 6): 420 px, raised surface, title then body.
pub fn dialog_box(t: &Tokens) -> Div {
    div()
        .w(DIALOG_W)
        .p(px(20.))
        .flex()
        .flex_col()
        .gap(px(12.))
        .bg(t.surface_raised)
        .border_1()
        .border_color(t.line_strong)
        .rounded(RADIUS_LG)
        .shadow_lg()
}

/// Audit U-11: the veil under a dialog, from the theme (55 % dark, 28 % light).
pub fn scrim(t: &Tokens) -> Div {
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(t.overlay)
}

pub const DIALOG_W: gpui_kit::Pixels = px(420.);

/// Audit U-14: an explanation kept out of sight: an info mark whose tooltip holds the sentence.
pub fn hint(id: &str, t: &Tokens, text: impl Into<SharedString>) -> Stateful<Div> {
    let text: SharedString = text.into();
    div()
        .id(SharedString::from(id.to_owned()))
        .flex()
        .child(icon("info", 14., t.text_faint))
        .tooltip(move |window, cx| {
            gpui_kit::component::tooltip::Tooltip::new(text.clone()).build(window, cx)
        })
}
