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
    /// A contact's birthday (mockup 48f): a day may be kept without its year.
    Birthday,
    /// A date column of the contacts.
    ContactCustom(ColumnId),
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
    if let Some(i) = contact_input(s, target) {
        return i.read(cx).value().to_string();
    }
    let Some(ed) = s.editor.as_ref() else {
        return String::new();
    };
    match target {
        CalTarget::Birthday | CalTarget::ContactCustom(_) => String::new(),
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

/// The box of a contact date.
fn contact_input(
    s: &Session,
    target: &CalTarget,
) -> Option<gpui_kit::Entity<gpui_kit::component::input::InputState>> {
    let ed = s.contacts_ui.editor.as_ref()?;
    match target {
        CalTarget::Birthday => ed
            .inputs
            .get(&cleis_core::contact_edit::Field::Birthday)
            .cloned(),
        CalTarget::ContactCustom(id) => ed
            .custom
            .iter()
            .find(|(c, _)| &c.id == id)
            .map(|(_, i)| i.clone()),
        _ => None,
    }
}

/// A birthday as typed, possibly without its year.
fn read_birthday(text: &str) -> Option<cleis_core::contact::PartialDate> {
    cleis_core::contact::PartialDate::read(text)
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
    let text = text_of(s, &cal.target, cx);
    let birthday = cal.target == CalTarget::Birthday;
    let born = birthday.then(|| read_birthday(&text)).flatten();
    let chosen = read(&text);
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
        // A birthday without its year is marked in every year shown.
        let on = match born {
            Some(b) => {
                date.month() == u32::from(b.month)
                    && date.day() == u32::from(b.day)
                    && b.year.is_none_or(|y| y == date.year())
            }
            None => chosen == Some(date),
        };
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
                .when(!birthday, |d| {
                    d.child(foot(
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
                })
                // Mockup 48f: keep the day and the month only.
                .when(born.is_some_and(|b| b.year.is_some()), |d| {
                    d.child(
                        button("cal-no-year", t, BtnKind::Ghost, true)
                            .child(tr("No year"))
                            .tooltip(tip(tr("Keep the day and the month only")))
                            .on_click(cx.listener(|this, _, window, cx| {
                                cx.stop_propagation();
                                this.birthday_without_year(window, cx)
                            }))
                            .test_support(),
                    )
                })
                .child(div().flex_1())
                .child(foot("cal-clear", "Clear", "", None, cx)),
        )
        .on_mouse_down(gpui_kit::MouseButton::Left, |_, _, cx| {
            cx.stop_propagation()
        })
}

/// Below its box, right edges aligned; above it when the window has no room below.
pub fn popup(cal: impl IntoElement, box_h: gpui_kit::Pixels) -> impl IntoElement {
    crate::ui::popup_below(cal, box_h)
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
            let text = text_of(s, &target, cx);
            let born = (target == CalTarget::Birthday)
                .then(|| read_birthday(&text))
                .flatten()
                .and_then(|b| {
                    NaiveDate::from_ymd_opt(
                        b.year.unwrap_or_else(|| today().year()),
                        u32::from(b.month),
                        u32::from(b.day),
                    )
                });
            let at = born.or_else(|| read(&text)).unwrap_or_else(today);
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
        let text = date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default();
        self.write_date(text, window, cx);
    }

    /// The birthday shown keeps its day and month, not its year (mockup 48f).
    fn birthday_without_year(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(cal) = s.calendar.as_ref() else {
            return;
        };
        let Some(b) = read_birthday(&text_of(s, &cal.target, cx)) else {
            return;
        };
        let text = cleis_core::contact::PartialDate { year: None, ..b }.display();
        self.write_date(text, window, cx);
    }

    /// Writes `text` in the box of the open calendar, saves it, closes the calendar.
    fn write_date(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(cal) = s.calendar.take() else { return };
        if let Some(i) = contact_input(s, &cal.target) {
            i.update(cx, |st, cx| st.set_value(text, window, cx));
            match cal.target {
                CalTarget::Birthday => {
                    self.commit_contact_field(cleis_core::contact_edit::Field::Birthday, cx)
                }
                CalTarget::ContactCustom(id) => self.commit_contact_custom(&id, cx),
                _ => {}
            }
            cx.notify();
            return;
        }
        let Some(ed) = s.editor.as_ref() else { return };
        let input = match &cal.target {
            CalTarget::Birthday | CalTarget::ContactCustom(_) => None,
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
            CalTarget::Birthday | CalTarget::ContactCustom(_) => {}
        }
        cx.notify();
    }
}
