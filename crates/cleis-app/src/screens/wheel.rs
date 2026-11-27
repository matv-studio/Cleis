//! An accent colour of one's own (mockup 36, 26/11/2026): the last swatch opens a wheel (hue
//! around, saturation from the centre), a brightness bar and a hexadecimal field. The colour is
//! shown live while it changes and saved with "Use"; Escape or a click elsewhere puts the saved
//! one back.

use crate::app::{CleisApp, Session};
use crate::i18n::tr;
use crate::theme::*;
use crate::ui::*;
use cleis_core::settings::ACCENTS;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::{
    Bounds, Context, Div, Entity, MouseButton, Pixels, Point, Rgba, Subscription, TestSupportExt,
    Window, div, px,
};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

/// Diameter of the wheel (mockup: 216 px).
pub const WHEEL: f32 = 216.;
const BAR_H: f32 = 10.;
const WIDTH: f32 = 248.;

/// The wheel open in Preferences › Appearance.
pub struct Wheel {
    /// Hue in degrees, 0 = red at the top, clockwise.
    pub h: f32,
    /// Saturation, 0 at the centre to 1 at the rim.
    pub s: f32,
    /// Brightness, 0 (black) to 1.
    pub v: f32,
    /// The hexadecimal field (made at the next render: it needs the window).
    pub hex: Option<(Entity<InputState>, Subscription)>,
    area: Rc<Cell<Bounds<Pixels>>>,
    bar: Rc<Cell<Bounds<Pixels>>>,
}

impl Wheel {
    pub fn new(start: Option<Rgba>) -> Self {
        let (h, s, v) = start.map(rgb_to_hsv).unwrap_or((330., 0.52, 0.88));
        Self {
            h,
            s,
            v,
            hex: None,
            area: Rc::default(),
            bar: Rc::default(),
        }
    }

    pub fn rgba(&self) -> Rgba {
        hsv_to_rgb(self.h, self.s, self.v)
    }

    pub fn hex_text(&self) -> String {
        to_hex(self.rgba())
    }

    /// A point of the wheel (window coordinates) chosen: hue from its angle, saturation from its
    /// distance to the centre (the rim beyond counts as the rim).
    fn pick_wheel(&mut self, p: Point<Pixels>) {
        let b = self.area.get();
        let r = f32::from(b.size.width) / 2.;
        if r <= 0. {
            return;
        }
        let dx = f32::from(p.x - b.origin.x) - r;
        let dy = f32::from(p.y - b.origin.y) - r;
        self.h = hue_of(dx, dy);
        self.s = ((dx * dx + dy * dy).sqrt() / r).min(1.);
    }

    fn pick_bar(&mut self, p: Point<Pixels>) {
        let b = self.bar.get();
        let w = f32::from(b.size.width);
        if w > 0. {
            self.v = (f32::from(p.x - b.origin.x) / w).clamp(0., 1.);
        }
    }
}

/// Which part is being dragged (nothing follows the pointer).
#[derive(Clone, Copy)]
pub enum WheelDrag {
    Wheel,
    Bar,
}

/// Degrees clockwise from the top, as `conic-gradient` lays its colours.
fn hue_of(dx: f32, dy: f32) -> f32 {
    let a = dx.atan2(-dy).to_degrees();
    if a < 0. { a + 360. } else { a }
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Rgba {
    let h = h.rem_euclid(360.) / 60.;
    let c = v * s;
    let x = c * (1. - (h % 2. - 1.).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.),
        1 => (x, c, 0.),
        2 => (0., c, x),
        3 => (0., x, c),
        4 => (x, 0., c),
        _ => (c, 0., x),
    };
    let m = v - c;
    Rgba {
        r: r + m,
        g: g + m,
        b: b + m,
        a: 1.,
    }
}

pub fn rgb_to_hsv(c: Rgba) -> (f32, f32, f32) {
    let max = c.r.max(c.g).max(c.b);
    let min = c.r.min(c.g).min(c.b);
    let d = max - min;
    let h = if d == 0. {
        0.
    } else if max == c.r {
        60. * ((c.g - c.b) / d).rem_euclid(6.)
    } else if max == c.g {
        60. * ((c.b - c.r) / d + 2.)
    } else {
        60. * ((c.r - c.g) / d + 4.)
    };
    let s = if max == 0. { 0. } else { d / max };
    (h, s, max)
}

pub fn to_hex(c: Rgba) -> String {
    let b = |x: f32| (x.clamp(0., 1.) * 255.).round() as u8;
    format!("#{:02x}{:02x}{:02x}", b(c.r), b(c.g), b(c.b))
}

/// The wheel at full brightness, drawn once at twice its size (sharp at 200 %).
fn wheel_image() -> Arc<gpui_kit::Image> {
    thread_local! {
        static IMG: Arc<gpui_kit::Image> = Arc::new(gpui_kit::Image::from_bytes(
            gpui_kit::ImageFormat::Png,
            wheel_png(2 * WHEEL as u32),
        ));
    }
    IMG.with(Arc::clone)
}

/// PNG of a hue-saturation wheel `n` pixels wide, antialiased at the rim.
pub fn wheel_png(n: u32) -> Vec<u8> {
    let r = n as f32 / 2.;
    let mut img = image::RgbaImage::new(n, n);
    for (x, y, px) in img.enumerate_pixels_mut() {
        let dx = x as f32 + 0.5 - r;
        let dy = y as f32 + 0.5 - r;
        let d = (dx * dx + dy * dy).sqrt();
        let alpha = (r - d + 0.5).clamp(0., 1.);
        let c = hsv_to_rgb(hue_of(dx, dy), (d / r).min(1.), 1.);
        let b = |x: f32| (x * 255.).round() as u8;
        *px = image::Rgba([b(c.r), b(c.g), b(c.b), b(alpha)]);
    }
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png)
        .expect("PNG in memory");
    out.into_inner()
}

/// The last swatch: your own colour when it is the accent, otherwise a small wheel.
pub fn own_swatch(own: Option<Rgba>, t: &Tokens) -> Div {
    let d = div()
        .size(px(22.))
        .rounded_full()
        .overflow_hidden()
        .flex()
        .items_center()
        .justify_center();
    match own {
        Some(c) => d.bg(c).border_1().border_color(t.line_strong),
        None => d.child(gpui_kit::img(wheel_image()).size(px(22.)).rounded_full()),
    }
}

/// Is this accent one of the palette's?
pub fn in_palette(hex: &str) -> bool {
    ACCENTS.iter().any(|(h, _)| h.eq_ignore_ascii_case(hex))
}

/// The popup below the swatches.
pub fn render(w: &Wheel, t: &Tokens, cx: &mut Context<CleisApp>) -> impl IntoElement {
    use gpui_kit::{linear_color_stop, linear_gradient};
    let c = w.rgba();
    let full = hsv_to_rgb(w.h, w.s, 1.);
    let r = WHEEL / 2.;
    let rad = w.h.to_radians();
    let (mx, my) = (r + w.s * r * rad.sin(), r - w.s * r * rad.cos());
    let area = w.area.clone();
    let bar = w.bar.clone();
    let marker = div()
        .absolute()
        .left(px((mx - 7.).round()))
        .top(px((my - 7.).round()))
        .size(px(14.))
        .rounded_full()
        .border_2()
        .border_color(gpui_kit::white())
        .shadow(vec![gpui_kit::BoxShadow {
            color: gpui_kit::black().opacity(0.5),
            offset: gpui_kit::point(px(0.), px(0.)),
            blur_radius: px(0.),
            spread_radius: px(1.),
            inset: false,
        }]);
    let wheel = div()
        .id("wheel-area")
        .relative()
        .size(px(WHEEL))
        .flex_none()
        .cursor_crosshair()
        .child(gpui_kit::img(wheel_image()).size(px(WHEEL)).rounded_full())
        // Brightness darkens the whole wheel.
        .child(
            div()
                .absolute()
                .inset_0()
                .rounded_full()
                .bg(gpui_kit::black().opacity(1. - w.v)),
        )
        .child(marker)
        .child(
            gpui_kit::canvas(move |b, _, _| area.set(b), |_, _, _, _| {})
                .absolute()
                .inset_0(),
        )
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, e: &gpui_kit::MouseDownEvent, _, cx| {
                this.wheel_pick(WheelDrag::Wheel, e.position, cx);
            }),
        )
        .on_drag(WheelDrag::Wheel, |_, _, _, cx| {
            cx.new(|_| crate::screens::dnd::NoGhost)
        })
        .test_support();
    let handle_x = ((WIDTH - 32.) * w.v - 1.5).round();
    let bright = div()
        .id("wheel-bar")
        .relative()
        .h(px(BAR_H))
        .w_full()
        .rounded(px(BAR_H / 2.))
        .bg(linear_gradient(
            90.,
            linear_color_stop(gpui_kit::black(), 0.),
            linear_color_stop(full, 1.),
        ))
        .cursor_pointer()
        .tooltip(tip(tr("Brightness")))
        .child(
            div()
                .absolute()
                .left(px(handle_x))
                .top(px(-2.))
                .w(px(3.))
                .h(px(14.))
                .rounded_full()
                .bg(gpui_kit::white())
                .border_1()
                .border_color(t.overlay),
        )
        .child(
            gpui_kit::canvas(move |b, _, _| bar.set(b), |_, _, _, _| {})
                .absolute()
                .inset_0(),
        )
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, e: &gpui_kit::MouseDownEvent, _, cx| {
                this.wheel_pick(WheelDrag::Bar, e.position, cx);
            }),
        )
        .on_drag(WheelDrag::Bar, |_, _, _, cx| {
            cx.new(|_| crate::screens::dnd::NoGhost)
        })
        .test_support();
    let field = div().flex_1().when_some(w.hex.as_ref(), |d, (i, _)| {
        d.child(gpui_kit::component::Sizable::small(
            Input::new(i).id("wheel-hex"),
        ))
    });
    let body = div()
        .id("wheel")
        .occlude()
        .w(px(WIDTH))
        .p(px(16.))
        .flex()
        .flex_col()
        .gap(px(12.))
        .bg(t.surface_raised)
        .border_1()
        .border_color(t.line_strong)
        .rounded(RADIUS_LG)
        .shadow_lg()
        .on_drag_move(
            cx.listener(|this, e: &gpui_kit::DragMoveEvent<WheelDrag>, _, cx| {
                let part = *e.drag(cx);
                this.wheel_pick(part, e.event.position, cx);
            }),
        )
        .child(wheel)
        .child(bright)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .flex_none()
                        .size(px(22.))
                        .rounded_full()
                        .border_1()
                        .border_color(t.line_strong)
                        .bg(c),
                )
                .child(field)
                .child(
                    button("wheel-use", t, BtnKind::Primary, true)
                        .child(tr("Use"))
                        .on_click(cx.listener(|this, _, _, cx| this.wheel_use(cx)))
                        .test_support(),
                ),
        );
    popup_below(menu_in("wheel-in", div().child(body)), px(30.))
}

impl CleisApp {
    /// The last swatch clicked: the wheel opens on the current accent (or closes).
    pub fn toggle_wheel(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        if s.wheel.take().is_none() {
            let start = s.prefs.accent.as_deref().and_then(parse_hex);
            s.wheel = Some(Wheel::new(start));
        } else {
            let prefs = s.prefs.clone();
            self.apply_appearance(&prefs, cx);
        }
        cx.notify();
    }

    fn wheel_pick(&mut self, part: WheelDrag, p: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(w) = self.session().and_then(|s| s.wheel.as_mut()) else {
            return;
        };
        match part {
            WheelDrag::Wheel => w.pick_wheel(p),
            WheelDrag::Bar => w.pick_bar(p),
        }
        self.wheel_changed(true, cx);
    }

    /// The colour changed: shown live (not saved), and in the field unless it was typed there.
    fn wheel_changed(&mut self, to_field: bool, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(w) = s.wheel.as_ref() else { return };
        let hex = w.hex_text();
        let mut prefs = s.prefs.clone();
        prefs.accent = Some(hex.clone());
        let field = w.hex.as_ref().map(|(i, _)| i.clone());
        let mut a = Appearance::from_prefs(&prefs);
        if let Some(m) = self.forced_mode {
            a.mode = m;
        }
        crate::theme::apply(&a, cx);
        if to_field && let Some(i) = field {
            self.wheel_field = Some((i, hex.to_uppercase()));
        }
        cx.notify();
    }

    /// Typed in the field: a full `#rrggbb` moves the wheel.
    fn wheel_typed(&mut self, text: &str, cx: &mut Context<Self>) {
        let Some(c) = parse_hex(text.trim()) else {
            return;
        };
        let Some(w) = self.session().and_then(|s| s.wheel.as_mut()) else {
            return;
        };
        if w.hex_text().eq_ignore_ascii_case(text.trim()) {
            return;
        }
        (w.h, w.s, w.v) = rgb_to_hsv(c);
        self.wheel_changed(false, cx);
    }

    /// "Use": the colour becomes the accent (saved).
    pub fn wheel_use(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(w) = s.wheel.take() else { return };
        let hex = w.hex_text();
        self.update_prefs(move |p| p.accent = Some(hex), cx);
        cx.notify();
    }

    /// The field needs the window: made at the next render, kept in step afterwards.
    pub fn ensure_wheel_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((i, text)) = self.wheel_field.take() {
            i.update(cx, |i, cx| i.set_value(text, window, cx));
        }
        let Some(s) = self.session() else { return };
        let Some(w) = s.wheel.as_mut() else { return };
        if w.hex.is_some() {
            return;
        }
        let text = w.hex_text().to_uppercase();
        let st = cx.new(|cx| InputState::new(window, cx));
        st.update(cx, |i, cx| i.set_value(text, window, cx));
        let sub = cx.subscribe_in(
            &st,
            window,
            |this, input, ev: &InputEvent, _, cx| match ev {
                InputEvent::Change => {
                    let text = input.read(cx).value().to_string();
                    this.wheel_typed(&text, cx);
                }
                InputEvent::PressEnter { .. } => this.wheel_use(cx),
                _ => {}
            },
        );
        if let Some(w) = self.session().and_then(|s| s.wheel.as_mut()) {
            w.hex = Some((st, sub));
        }
    }
}

/// Is the wheel open? (`Session` helper for the menus' outside layer.)
pub fn open(s: &Session) -> bool {
    s.wheel.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsv_round_trip() {
        for hex in [
            "#e06b9a", "#3b82f6", "#000000", "#ffffff", "#e0a93b", "#10b981",
        ] {
            let c = parse_hex(hex).unwrap();
            let (h, s, v) = rgb_to_hsv(c);
            assert_eq!(to_hex(hsv_to_rgb(h, s, v)), hex);
        }
    }

    #[test]
    fn hue_is_clockwise_from_the_top() {
        assert_eq!(hue_of(0., -1.).round(), 0.);
        assert_eq!(hue_of(1., 0.).round(), 90.);
        assert_eq!(hue_of(0., 1.).round(), 180.);
        assert_eq!(hue_of(-1., 0.).round(), 270.);
    }

    #[test]
    fn wheel_png_is_round() {
        let png = wheel_png(32);
        let img = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(img.get_pixel(0, 0).0[3], 0);
        assert_eq!(img.get_pixel(16, 16).0[3], 255);
        // Centre: white (no saturation).
        assert!(img.get_pixel(16, 16).0[..3].iter().all(|&c| c > 230));
    }
}
