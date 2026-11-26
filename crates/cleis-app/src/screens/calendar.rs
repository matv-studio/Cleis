//! Dates (mockup 35g): typed in their box, or picked in a calendar opened from it, with one month
//! or one year from today and Clear.

use crate::app::{CleisApp, Phase, Session};
use crate::i18n::tr;
use crate::theme::*;
use crate::ui::*;
use chrono::{Datelike, Months, NaiveDate};
use cleis_core::ColumnId;
use cleis_core::edit::Field;
use gpui_kit::TestSupportExt;
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, SharedString, Window, div, px};

/// The box a calendar fills.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CalTarget {
    Field(Field),
    Custom(ColumnId),
}

/// The calendar open below a date box, on the month shown.
pub struct Calendar {
    pub target: CalTarget,
    pub year: i32,
    pub month: u32,
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const DAYS: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

pub const WIDTH: f32 = 256.;
/// A day: seven fit inside the padding and the border.
const CELL: f32 = 34.;

fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

/// The date written in a box, whatever its form (ISO, "3 Dec 2026"…).
fn read(text: &str) -> Option<NaiveDate> {
    let iso = cleis_core::import_csv::parse_date(text)?;
    NaiveDate::parse_from_str(&iso, "%Y-%m-%d").ok()
}

/// The text of the box a calendar fills.
fn text_of(s: &Session, target: &CalTarget, cx: &gpui_kit::App) -> String {
    let Some(ed) = s.editor.as_ref() else {
        return String::new();
    };
    match target {
        CalTarget::Field(f) => ed
            .inputs
            .get(f)
            .map(|i| i.read(cx).value().to_string())
            .unwrap_or_default(),
        CalTarget::Custom(id) => ed
            .custom
            .iter()
            .find(|(c, _)| &c.id == id)
            .map(|(_, i)| i.read(cx).value().to_string())
            .unwrap_or_default(),
    }
}

/// The small calendar button inside a date box.
pub fn open_button(
    id: impl Into<SharedString>,
    target: CalTarget,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> gpui_kit::AnyElement {
    let id: SharedString = id.into();
    div()
        .id(id)
        .size(px(20.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(RADIUS_SM)
        .cursor_pointer()
        .hover(|d| d.bg(t.surface_hover))
        .child(icon("calendar", 14., t.text_muted))
        .tooltip(tip(tr("Pick a date")))
        .on_click(cx.listener(move |this, _, _, cx| {
            cx.stop_propagation();
            this.toggle_calendar(target.clone(), cx);
        }))
        .test_support()
        .into_any_element()
}

/// The calendar (put in `deferred` under its box).
pub fn render(
    s: &Session,
    cal: &Calendar,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> gpui_kit::Stateful<Div> {
    let chosen = read(&text_of(s, &cal.target, cx));
    let now = today();
    let first = NaiveDate::from_ymd_opt(cal.year, cal.month, 1).unwrap_or(now);
    let next = first + Months::new(1);
    let days = (next - first).num_days() as u32;
    let lead = first.weekday().num_days_from_monday();
    let nav =
        |id: &'static str, label: &'static str, rot: f32, step: i32, cx: &mut Context<CleisApp>| {
            div()
                .id(id)
                .size(px(24.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(RADIUS_SM)
                .cursor_pointer()
                .hover(|d| d.bg(t.surface_hover))
                .child(
                    gpui_kit::svg()
                        .path("icons/chevron.svg")
                        .size(px(14.))
                        .text_color(t.text_muted)
                        .with_transformation(gpui_kit::Transformation::rotate(gpui_kit::radians(
                            rot,
                        ))),
                )
                .tooltip(tip(tr(label)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.calendar_step(step, cx)
                }))
                .test_support()
        };
    let mut grid = div().flex().flex_wrap().w(px(CELL * 7.));
    for d in DAYS {
        grid = grid.child(
            div()
                .w(px(CELL))
                .h(px(24.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(TEXT_XS)
                .text_color(t.text_faint)
                .child(tr(d)),
        );
    }
    for _ in 0..lead {
        grid = grid.child(div().w(px(CELL)).h(px(30.)));
    }
    for n in 1..=days {
        let date = first.with_day(n).unwrap_or(first);
        let on = chosen == Some(date);
        let is_today = date == now;
        grid = grid.child(
            div()
                .w(px(CELL))
                .h(px(30.))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .id(SharedString::from(format!(
                            "cal-{}",
                            date.format("%Y-%m-%d")
                        )))
                        .size(px(28.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .cursor_pointer()
                        .text_size(TEXT_SM)
                        .when(on, |d| d.bg(t.primary).text_color(t.primary_ink))
                        .when(!on, |d| d.hover(|d| d.bg(t.surface_hover)))
                        .when(is_today && !on, |d| {
                            d.border_1()
                                .border_color(t.line_strong)
                                .text_color(t.accent_text)
                        })
                        .child(n.to_string())
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.pick_date(Some(date), window, cx)
                        }))
                        .test_support(),
                ),
        );
    }
    let foot = |id: &'static str,
                label: &'static str,
                hint: &'static str,
                date: Option<NaiveDate>,
                cx: &mut Context<CleisApp>| {
        button(id, t, BtnKind::Ghost, true)
            .child(tr(label))
            .when(!hint.is_empty(), |d| d.tooltip(tip(tr(hint))))
            .on_click(cx.listener(move |this, _, window, cx| {
                cx.stop_propagation();
                this.pick_date(date, window, cx)
            }))
            .test_support()
    };
    menu_box(t)
        .id("calendar")
        .w(px(WIDTH))
        .p(px(8.))
        .gap(px(4.))
        .child(
            div()
                .flex()
                .items_center()
                .child(nav(
                    "cal-prev",
                    "Previous month",
                    std::f32::consts::FRAC_PI_2,
                    -1,
                    cx,
                ))
                .child(
                    div()
                        .flex_1()
                        .text_center()
                        .font_weight(W_MEDIUM)
                        .child(format!(
                            "{} {}",
                            tr(MONTHS[cal.month as usize - 1]),
                            cal.year
                        )),
                )
                .child(nav(
                    "cal-next",
                    "Next month",
                    -std::f32::consts::FRAC_PI_2,
                    1,
                    cx,
                )),
        )
        .child(grid)
        .child(
            div()
                .pt(px(4.))
                .border_t_1()
                .border_color(t.line)
                .flex()
                .items_center()
                .gap(px(4.))
                .child(foot(
                    "cal-month",
                    "+1 month",
                    "One month from today",
                    Some(now + Months::new(1)),
                    cx,
                ))
                .child(foot(
                    "cal-year",
                    "+1 year",
                    "One year from today",
                    Some(now + Months::new(12)),
                    cx,
                ))
                .child(div().flex_1())
                .child(foot("cal-clear", "Clear", "", None, cx)),
        )
        .on_mouse_down(gpui_kit::MouseButton::Left, |_, _, cx| {
            cx.stop_propagation()
        })
}

/// Below its box, right edges aligned; above it when the window has no room below.
pub fn popup(cal: impl IntoElement, box_h: gpui_kit::Pixels) -> impl IntoElement {
    // A line as tall as the box on its right edge: the calendar hangs from its bottom.
    let target = gpui_kit::Bounds::new(
        gpui_kit::point(px(0.), px(0.)),
        gpui_kit::size(px(0.), box_h),
    );
    div().absolute().top_0().right_0().w(px(0.)).h(box_h).child(
        gpui_kit::deferred(
            gpui_kit::anchored()
                .position_mode(gpui_kit::AnchoredPositionMode::Local)
                .anchor(gpui_kit::Anchor::TopRight)
                .attach(target, gpui_kit::Anchor::BottomRight)
                .offset(gpui_kit::point(px(0.), px(4.)))
                .snap_to_window_with_margin(px(8.))
                .child(cal),
        )
        .with_priority(2),
    )
}

impl CleisApp {
    /// Opens the calendar of a date box on the month of its date (or this month), or closes it.
    pub fn toggle_calendar(&mut self, target: CalTarget, cx: &mut Context<Self>) {
        let Phase::Main(s) = &mut self.phase else {
            return;
        };
        if s.calendar.as_ref().is_some_and(|c| c.target == target) {
            s.calendar = None;
        } else {
            let at = read(&text_of(s, &target, cx)).unwrap_or_else(today);
            s.calendar = Some(Calendar {
                target,
                year: at.year(),
                month: at.month(),
            });
        }
        cx.notify();
    }

    fn calendar_step(&mut self, step: i32, cx: &mut Context<Self>) {
        let Some(c) = self.session().and_then(|s| s.calendar.as_mut()) else {
            return;
        };
        let m = c.year * 12 + c.month as i32 - 1 + step;
        c.year = m.div_euclid(12);
        c.month = m.rem_euclid(12) as u32 + 1;
        cx.notify();
    }

    /// Writes the date picked (or nothing: Clear) in the box, saves it, closes the calendar.
    fn pick_date(&mut self, date: Option<NaiveDate>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(cal) = s.calendar.take() else { return };
        let text = date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default();
        let Some(ed) = s.editor.as_ref() else { return };
        let input = match &cal.target {
            CalTarget::Field(f) => ed.inputs.get(f).cloned(),
            CalTarget::Custom(id) => ed
                .custom
                .iter()
                .find(|(c, _)| &c.id == id)
                .map(|(_, i)| i.clone()),
        };
        if let Some(i) = input {
            i.update(cx, |st, cx| st.set_value(text, window, cx));
        }
        match cal.target {
            CalTarget::Field(f) => self.commit_field(f, cx),
            CalTarget::Custom(id) => self.commit_custom(&id, cx),
        }
        cx.notify();
    }
}
