//! Charter components (design/maquettes/css/ui.css) built from GPUI primitives.

use crate::theme::*;
use gpui_kit::prelude::*;
use gpui_kit::{
    AnyElement, Div, ElementId, FontWeight, Hsla, MouseButton, MouseMoveEvent, SharedString,
    Stateful, Window, WindowControlArea, div, px, svg,
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
        .child(crate::i18n::tr_owned(&label.into()))
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
        // The handle slides to its side when the switch changes (120 ms).
        .child({
            use gpui_kit::AnimationExt;
            handle.with_animation(
                if on { "sw-on" } else { "sw-off" },
                gpui_kit::Animation::new(std::time::Duration::from_millis(TOGGLE_MS))
                    .with_easing(gpui_kit::ease_out_quint()),
                move |d, v| {
                    let away = px((1. - v) * 10.);
                    if on { d.right(away) } else { d.left(away) }
                },
            )
        })
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
        .child(crate::i18n::tr_owned(title))
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
                .child(crate::i18n::tr_owned(o)),
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
                .child(crate::i18n::tr_owned(label)),
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
        .child(crate::i18n::tr_owned(&label.into()))
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

// ---------- Motion (26/11/2026: short and lively; none when the system asks for less) ----------

/// Pages: fade in while rising 8 px, 150 ms.
pub const PAGE_MS: u64 = 150;
/// Menus and popups: fade in while dropping 4 px, 120 ms.
pub const MENU_MS: u64 = 120;
/// Switches: 120 ms.
pub const TOGGLE_MS: u64 = 120;

/// A page shown: fades in and rises into place. `key` changes with the page, which plays it again.
pub fn page_in(key: impl Into<SharedString>, el: Div) -> gpui_kit::AnimationElement<Div> {
    use gpui_kit::AnimationExt;
    el.relative().with_animation(
        ElementId::Name(key.into()),
        gpui_kit::Animation::new(std::time::Duration::from_millis(PAGE_MS))
            .with_easing(gpui_kit::ease_out_quint()),
        |d, v| d.opacity(v).top(px((1. - v) * 8.)),
    )
}

/// A menu or popup opened: fades in and drops into place.
pub fn menu_in(key: impl Into<SharedString>, el: Div) -> gpui_kit::AnimationElement<Div> {
    use gpui_kit::AnimationExt;
    el.with_animation(
        ElementId::Name(key.into()),
        gpui_kit::Animation::new(std::time::Duration::from_millis(MENU_MS))
            .with_easing(gpui_kit::ease_out_quint()),
        |d, v| d.opacity(v).mt(px((v - 1.) * 4.)),
    )
}

/// Customize mode: the handles breathe, all together, to say they can be dragged.
pub fn grip_pulse(id: impl Into<SharedString>, t: &Tokens, size: f32) -> impl IntoElement {
    use gpui_kit::AnimationExt;
    div()
        .flex_none()
        .child(icon("grip", size, t.text_muted))
        .with_animation(
            ElementId::Name(id.into()),
            gpui_kit::Animation::new(std::time::Duration::from_millis(1400))
                .repeat_synced()
                .with_easing(gpui_kit::pulsating_between(0.35, 1.)),
            |d, v| d.opacity(v),
        )
}

/// A pill that is on or off (mockup 35j: the shortcuts of a vault).
pub fn tag_toggle(id: impl Into<ElementId>, t: &Tokens, label: &str, on: bool) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .gap(px(4.))
        .h(px(24.))
        .px(px(8.))
        .rounded_full()
        .text_size(TEXT_SM)
        .cursor_pointer()
        .when(on, |d| d.bg(t.accent_soft).text_color(t.accent_text))
        .when(!on, |d| {
            d.border_1()
                .border_color(t.line_strong)
                .text_color(t.text_muted)
                .hover(|d| d.bg(t.surface_hover))
        })
        .when(on, |d| d.child(icon("check", 12., t.accent_text)))
        .child(crate::i18n::tr_owned(label))
}

/// A tooltip with a text already translated (`.tooltip(tip(tr("…")))`).
pub fn tip(
    text: impl Into<SharedString>,
) -> impl Fn(&mut gpui_kit::Window, &mut gpui_kit::App) -> gpui_kit::AnyView + 'static {
    let text: SharedString = text.into();
    move |window, cx| gpui_kit::component::tooltip::Tooltip::new(text.clone()).build(window, cx)
}

/// Audit U-14: an explanation kept out of sight: an info mark whose tooltip holds the sentence.
pub fn hint(id: &str, t: &Tokens, text: impl Into<SharedString>) -> Stateful<Div> {
    let text: SharedString = text.into();
    div()
        .id(SharedString::from(id.to_owned()))
        .flex()
        .child(icon("info", 14., t.text_faint))
        .tooltip(move |window, cx| {
            gpui_kit::component::tooltip::Tooltip::new(crate::i18n::tr_dynamic(&text))
                .build(window, cx)
        })
}

thread_local! {
    /// A press on the bar's free part, not yet a move (Linux, macOS).
    static BAR_PRESSED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The window's top bar (charter §5: the frame is the top bar), in place of gpui-component's
/// `TitleBar`: it moves the window and carries window buttons drawn like the charter's icon
/// buttons (Windows, Linux; macOS keeps its traffic lights). On Windows the bar is a caption
/// area for the system (drag, double-click, snap); every control in `content` must call
/// [`NoDrag::no_drag`], or its click becomes a window move.
pub fn title_bar(t: &Tokens, window: &Window, content: impl IntoElement) -> Stateful<Div> {
    let windows = cfg!(target_os = "windows");
    let macos = cfg!(target_os = "macos");
    let maximized = window.is_maximized();
    let control = |id: &'static str, icon_name: &'static str, close: bool| {
        let (hover_bg, hover_fg) = if close {
            (t.danger_solid, t.primary_ink)
        } else {
            (t.surface_hover, t.text)
        };
        let muted = t.text_muted;
        div()
            .id(id)
            .group(id)
            .flex_none()
            .size(CONTROL_MD)
            .flex()
            .items_center()
            .justify_center()
            .rounded(RADIUS_MD)
            .text_color(muted)
            .hover(move |s| s.bg(hover_bg).text_color(hover_fg))
            .child(
                svg()
                    .path(SharedString::from(format!("icons/{icon_name}.svg")))
                    .size(px(16.))
                    .text_color(muted)
                    .group_hover(id, move |s| s.text_color(hover_fg)),
            )
    };
    let buttons = (!macos).then(|| {
        let min = control("win-min", "winMin", false);
        let max = control(
            "win-max",
            if maximized { "winRestore" } else { "winMax" },
            false,
        );
        let close = control("win-close", "winClose", true);
        let (min, max, close) = if windows {
            // The system answers these areas itself (clicks, Windows 11 snap layouts).
            (
                min.window_control_area(WindowControlArea::Min),
                max.window_control_area(WindowControlArea::Max),
                close.window_control_area(WindowControlArea::Close),
            )
        } else {
            (
                min.no_drag()
                    .on_click(|_, window, _| window.minimize_window()),
                max.no_drag().on_click(|_, window, _| window.zoom_window()),
                close
                    .no_drag()
                    .on_click(|_, window, _| window.remove_window()),
            )
        };
        div()
            .flex()
            .items_center()
            .gap(px(2.))
            .pl(px(8.))
            .child(min)
            .child(max)
            .child(close)
    });
    let bar = div()
        .id("title-bar")
        .flex_none()
        .h(TOPBAR_H)
        .w_full()
        .flex()
        .items_center()
        .bg(t.chrome)
        // macOS: room for the traffic lights.
        .when(macos, |d| d.pl(px(78.)))
        .when(!macos, |d| d.pl(px(12.)).pr(px(8.)))
        .child(
            div()
                .id("title-drag")
                .flex_1()
                .h_full()
                .flex()
                .when(windows, |d| d.window_control_area(WindowControlArea::Drag))
                .child(content),
        )
        .children(buttons);
    if windows {
        return bar;
    }
    bar.on_mouse_down(MouseButton::Left, |_, window, _| {
        BAR_PRESSED.set(!window.default_prevented());
    })
    .on_mouse_up(MouseButton::Left, |_, _, _| BAR_PRESSED.set(false))
    .on_mouse_move(|e: &MouseMoveEvent, window, _| {
        if e.pressed_button != Some(MouseButton::Left) {
            BAR_PRESSED.set(false);
        } else if BAR_PRESSED.get() {
            BAR_PRESSED.set(false);
            window.start_window_move();
        }
    })
    .on_click(move |e, window, _| {
        if e.click_count() == 2 {
            if macos {
                window.titlebar_double_click();
            } else {
                window.zoom_window();
            }
        }
    })
}

/// A control inside [`title_bar`]: its press stays a click (not a window move).
pub trait NoDrag: InteractiveElement + Sized {
    fn no_drag(self) -> Self {
        self.occlude()
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
    }
}

impl<E: InteractiveElement> NoDrag for E {}
