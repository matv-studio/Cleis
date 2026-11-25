//! Display formatting (English locale for now; French at release).

use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::{Tone, chip};
use cleis_core::formula::Num;
use cleis_core::{Money, Period, Subscription};
use gpui_kit::Div;

fn group_thousands(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    if n < 0 { format!("-{out}") } else { out }
}

fn symbol(currency: &str) -> &str {
    match currency {
        "EUR" => "€",
        "USD" => "$",
        "GBP" => "£",
        other => other,
    }
}

/// `€1,679.76`.
pub fn money_cents(cents: i64, currency: &str) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let c = cents.abs();
    format!(
        "{sign}{}{}.{:02}",
        symbol(currency),
        group_thousands(c / 100),
        c % 100
    )
}

pub fn money_text(m: &Money) -> String {
    money_cents(m.cents, &m.currency)
}

pub fn money(m: &Money, _short: bool) -> String {
    money_text(m)
}

pub fn period_suffix(p: Period) -> &'static str {
    crate::i18n::tr(match p {
        Period::Month => " /mo",
        Period::Quarter => " /qtr",
        Period::Year => " /yr",
        Period::OneTime => "",
    })
}

pub fn period_long(p: Period) -> &'static str {
    crate::i18n::tr(match p {
        Period::Month => " / month",
        Period::Quarter => " / quarter",
        Period::Year => " / year",
        Period::OneTime => "",
    })
}

pub fn num(n: &Num) -> String {
    match &n.currency {
        Some(c) => money_cents((n.value * 100.0).round() as i64, c),
        None => {
            if n.value.fract() == 0.0 {
                group_thousands(n.value as i64)
            } else {
                format!("{:.2}", n.value)
            }
        }
    }
}

pub fn subscription_chip(t: &Tokens, s: Subscription) -> Option<Div> {
    let (tone, label) = match s {
        Subscription::None => return None,
        Subscription::Active => (Tone::Ok, tr("Active")),
        Subscription::Free => (Tone::Neutral, tr("Free")),
        Subscription::RenewSoon => (Tone::Warn, tr("Renew soon")),
        Subscription::Pending => (Tone::Neutral, tr("Pending")),
        Subscription::Off => (Tone::Neutral, tr("Off")),
    };
    Some(chip(t, tone, label, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_format() {
        assert_eq!(money_cents(167_976, "EUR"), "€1,679.76");
        assert_eq!(money_cents(999, "USD"), "$9.99");
        assert_eq!(money_cents(-50, "EUR"), "-€0.50");
        assert_eq!(
            num(&Num {
                value: 720.0,
                currency: Some("EUR".into()),
                per_year: None
            }),
            "€720.00"
        );
        assert_eq!(num(&Num::plain(1234.0)), "1,234");
    }
}

/// `Nov 25, 2026` from a Unix timestamp (seconds, UTC).
pub fn date(ts: i64) -> String {
    let (y, m, d) = civil(ts.div_euclid(86_400));
    // French (U-9): `25 nov. 2026`.
    if crate::i18n::is_french() {
        const M: [&str; 12] = [
            "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.",
            "nov.", "déc.",
        ];
        return format!("{d} {} {y}", M[(m - 1) as usize]);
    }
    const M: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!("{} {d}, {y}", M[(m - 1) as usize])
}

/// Days since 1970-01-01 → (year, month, day), proleptic Gregorian (H. Hinnant's algorithm).
pub fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod date_tests {
    #[test]
    fn dates() {
        assert_eq!(super::date(0), "Jan 1, 1970");
        assert_eq!(super::date(1_795_579_200), "Nov 25, 2026");
        assert_eq!(super::civil(19_782), (2024, 2, 29));
    }
}

/// Offset of this computer's time zone at `ts`, in seconds (summer time included).
fn local_offset(ts: i64) -> i64 {
    use chrono::{Local, Offset, TimeZone};
    if cfg!(test) {
        return 0;
    }
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map_or(0, |d| i64::from(d.offset().fix().local_minus_utc()))
}

/// `Nov 25, 2026, 9:12 AM` in this computer's time zone.
pub fn date_time(ts: i64) -> String {
    date_time_utc(ts + local_offset(ts))
}

fn date_time_utc(ts: i64) -> String {
    let secs = ts.rem_euclid(86_400);
    let (h, m) = (secs / 3600, secs % 3600 / 60);
    if crate::i18n::is_french() {
        return format!("{}, {h:02}:{m:02}", date(ts));
    }
    let (h12, ap) = match h {
        0 => (12, "AM"),
        1..=11 => (h, "AM"),
        12 => (12, "PM"),
        _ => (h - 12, "PM"),
    };
    format!("{}, {h12}:{m:02} {ap}", date(ts))
}

/// `Today, 9:12 AM`, `Yesterday, 5:30 PM`, else the date and time.
pub fn when(ts: i64, now: i64) -> String {
    let day = |t: i64| (t + local_offset(t)).div_euclid(86_400);
    let full = date_time(ts);
    let time = full
        .rsplit_once(", ")
        .map(|x| x.1.to_owned())
        .unwrap_or_default();
    match day(now) - day(ts) {
        0 => crate::trf!("Today, {}", time),
        1 => crate::trf!("Yesterday, {}", time),
        _ => full,
    }
}

/// `Today, 9:12 AM`, `Yesterday, 5:30 PM`, else the date alone.
pub fn when_short(ts: i64, now: i64) -> String {
    let day = |t: i64| (t + local_offset(t)).div_euclid(86_400);
    if day(now) - day(ts) <= 1 {
        when(ts, now)
    } else {
        date(ts + local_offset(ts))
    }
}

/// `184 KB`, `2.4 MB`.
pub fn size(bytes: u64) -> String {
    if bytes < 1024 * 1024 {
        crate::trf!("{} KB", bytes.div_ceil(1024))
    } else {
        let mb = format!("{:.1}", bytes as f64 / (1024. * 1024.));
        if crate::i18n::is_french() {
            format!("{} Mo", mb.replace('.', ","))
        } else {
            format!("{mb} MB")
        }
    }
}

/// `20261125-091200` for file names.
pub fn stamp(ts: i64) -> String {
    let (y, m, d) = civil(ts.div_euclid(86_400));
    let s = ts.rem_euclid(86_400);
    format!(
        "{y}{m:02}{d:02}-{:02}{:02}{:02}",
        s / 3600,
        s % 3600 / 60,
        s % 60
    )
}

#[cfg(test)]
mod time_tests {
    #[test]
    fn times() {
        assert_eq!(
            super::date_time(1_795_564_800 + 9 * 3600 + 12 * 60),
            "Nov 25, 2026, 9:12 AM"
        );
        assert_eq!(
            super::date_time(1_795_564_800 + 17 * 3600 + 30 * 60),
            "Nov 25, 2026, 5:30 PM"
        );
        let now = 1_795_564_800 + 12 * 3600;
        assert_eq!(super::when(now - 3600, now), "Today, 11:00 AM");
        assert_eq!(super::when(now - 86_400, now), "Yesterday, 12:00 PM");
        assert_eq!(super::size(2048), "2 KB");
        assert_eq!(super::stamp(1_795_564_800), "20261125-000000");
    }
}
