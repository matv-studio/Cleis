//! Scroll bars and middle-click scrolling (0.2.4, mockup 48j).
//!
//! Every long list or page of Cleis has a bar on its right, drawn as the charter's sliders (§ 4:
//! a 2 px line, the visible part as a pill): visible whenever there is more than fits, dragged or
//! clicked above or below the pill. A press on the mouse wheel in it leaves an anchor where it
//! was pressed: moving the pointer away from it scrolls, faster the farther; released at once,
//! any click or Esc stops it; held and dragged, it stops when released (as in Windows' browsers).

use crate::app::CleisApp;
use crate::theme::Tokens;
use gpui_kit::TestSupportExt;
use gpui_kit::base::{Scrollbar, ScrollbarMode};
use gpui_kit::prelude::*;
use gpui_kit::{
    Context, Div, MouseButton, MouseDownEvent, Pixels, Point, ScrollHandle, Stateful, Task,
    UniformListScrollHandle, div, point, px,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;

/// The scroll positions of the areas, kept from one render to the next.
#[derive(Default)]
pub struct Scrolls {
    areas: RefCell<HashMap<&'static str, ScrollHandle>>,
    /// The vault's list (a uniform list).
    pub list: UniformListScrollHandle,
}

impl Scrolls {
    /// The handle of area `key` (made at its first use).
    pub fn handle(&self, key: &'static str) -> ScrollHandle {
        self.areas.borrow_mut().entry(key).or_default().clone()
    }
}

/// Middle-click scrolling under way.
pub struct Auto {
    handle: ScrollHandle,
    /// Where the wheel was pressed (window coordinates): the anchor.
    pub origin: Point<Pixels>,
    pointer: Point<Pixels>,
    /// The wheel is still held down.
    held: bool,
    /// The pointer went away from the anchor while the wheel was held: releasing it stops.
    dragged: bool,
    _task: Task<()>,
}

/// No scrolling this close to the anchor.
const DEAD: f32 = 10.;
/// Frame time of the scrolling.
const FRAME: Duration = Duration::from_millis(16);

/// Pixels per frame for a pointer `d` pixels from the anchor (signed): slow near it, faster away.
pub fn speed(d: f32) -> f32 {
    let a = (d.abs() - DEAD).max(0.);
    d.signum() * (a * 0.08 + (a / 60.).powi(2)).min(90.)
}

/// The vertical bar of `handle`, over the right edge of its area.
pub fn bar<H: gpui_kit::base::ScrollbarHandle + Clone>(handle: &H, t: &Tokens) -> Scrollbar {
    let (faint, muted, line) = (t.text_faint, t.text_muted, t.range_track_bg);
    Scrollbar::vertical(handle)
        .mode(ScrollbarMode::Always)
        .styles(move |s| {
            s.track(|t| t.bg(gpui_kit::transparent_black()).width(px(10.)))
                .track_hover(|t| t.bg(line.opacity(0.5)).width(px(10.)))
                .thumb(|th| {
                    th.bg(faint)
                        .width(px(4.))
                        .inset(px(3.))
                        .radius(px(2.))
                        .min_length(px(24.))
                })
                .thumb_hover(|th| th.bg(muted).width(px(4.)).inset(px(3.)).radius(px(2.)))
                .thumb_active(|th| th.bg(muted).width(px(4.)).inset(px(3.)).radius(px(2.)))
        })
}

/// `el` (an `overflow_y_scroll` area, `key` naming its position) with its bar and the middle
/// click. The returned box takes `el`'s place: it grows as a column child (`flex_1`).
pub fn area(
    key: &'static str,
    scrolls: &Scrolls,
    el: Stateful<Div>,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> Div {
    let h = scrolls.handle(key);
    let start = h.clone();
    div()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .relative()
        .flex()
        .flex_col()
        .on_mouse_down(
            MouseButton::Middle,
            cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
                this.start_autoscroll(start.clone(), e.position, cx)
            }),
        )
        .child(el.track_scroll(&h).flex_1().min_h_0().test_support())
        .child(
            div()
                .absolute()
                .top_0()
                .right_0()
                .bottom_0()
                .w(px(10.))
                .child(bar(&h, t)),
        )
}

/// The vault's list (a uniform list) with its bar and the middle click.
pub fn list_area(
    scrolls: &Scrolls,
    list: impl IntoElement,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> Div {
    let h = scrolls.list.clone();
    let base = h.0.borrow().base_handle.clone();
    div()
        .flex_1()
        .min_h_0()
        .relative()
        .flex()
        .flex_col()
        .on_mouse_down(
            MouseButton::Middle,
            cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
                this.start_autoscroll(base.clone(), e.position, cx)
            }),
        )
        .child(list)
        .child(
            div()
                .absolute()
                .top_0()
                .right_0()
                .bottom_0()
                .w(px(10.))
                .child(bar(&h, t)),
        )
}

impl CleisApp {
    /// The wheel was pressed in an area scrolled by `handle` (mockup 48j). A second press
    /// while scrolling stops it.
    pub fn start_autoscroll(
        &mut self,
        handle: ScrollHandle,
        at: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if self.autoscroll.take().is_some() {
            cx.notify();
            return;
        }
        // Nothing to scroll: no anchor.
        if handle.max_offset().y <= px(0.) {
            return;
        }
        let task = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(FRAME).await;
                let go_on = this
                    .update(cx, |app, cx| {
                        let Some(a) = app.autoscroll.as_ref() else {
                            return false;
                        };
                        let v = speed(f32::from(a.pointer.y - a.origin.y));
                        if v != 0. {
                            let max = a.handle.max_offset().y;
                            let o = a.handle.offset();
                            let y = (o.y - px(v)).clamp(-max, px(0.));
                            if y != o.y {
                                a.handle.set_offset(point(o.x, y));
                                cx.notify();
                            }
                        }
                        true
                    })
                    .unwrap_or(false);
                if !go_on {
                    break;
                }
            }
        });
        self.autoscroll = Some(Auto {
            handle,
            origin: at,
            pointer: at,
            held: true,
            dragged: false,
            _task: task,
        });
        cx.notify();
    }

    /// The pointer moved (the window's root): the speed follows its distance to the anchor.
    pub fn autoscroll_move(&mut self, at: Point<Pixels>) {
        if let Some(a) = self.autoscroll.as_mut() {
            a.pointer = at;
            if a.held && (f32::from(at.y - a.origin.y)).abs() > DEAD {
                a.dragged = true;
            }
        }
    }

    /// The wheel released: held and dragged, the scrolling stops; released at once, it goes on
    /// until the next click.
    pub fn autoscroll_release(&mut self, cx: &mut Context<Self>) {
        let stop = match self.autoscroll.as_mut() {
            Some(a) => {
                a.held = false;
                a.dragged
            }
            None => false,
        };
        if stop {
            self.autoscroll = None;
            cx.notify();
        }
    }

    /// Any press stops it (and does nothing else); `true` when it was running.
    pub fn stop_autoscroll(&mut self, cx: &mut Context<Self>) -> bool {
        let was = self.autoscroll.take().is_some();
        if was {
            cx.notify();
        }
        was
    }
}

/// The anchor drawn where the wheel was pressed (mockup 48j).
pub fn anchor(a: &Auto, t: &Tokens) -> impl IntoElement {
    let size = 28.;
    div()
        .absolute()
        .left(a.origin.x - px(size / 2.))
        .top(a.origin.y - px(size / 2.))
        .size(px(size))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(t.surface_raised)
        .border_1()
        .border_color(t.line_strong)
        .shadow_lg()
        .child(crate::ui::icon("autoscroll", 20., t.text))
}

#[cfg(test)]
mod tests {
    use super::speed;

    #[test]
    fn faster_farther() {
        assert_eq!(speed(0.), 0.);
        assert_eq!(speed(8.), 0.);
        assert_eq!(speed(-8.), 0.);
        assert!(speed(40.) > 0. && speed(-40.) < 0.);
        assert!(speed(200.) > speed(60.));
        assert!(speed(5000.) <= 90.);
    }
}
