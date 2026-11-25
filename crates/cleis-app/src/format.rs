//! Display formatting (English locale for now; French at release).

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
    match p {
        Period::Month => " /mo",
        Period::Quarter => " /qtr",
        Period::Year => " /yr",
        Period::OneTime => "",
    }
}

pub fn period_long(p: Period) -> &'static str {
    match p {
        Period::Month => " / month",
        Period::Quarter => " / quarter",
        Period::Year => " / year",
        Period::OneTime => "",
    }
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
        Subscription::Active => (Tone::Ok, "Active"),
        Subscription::Free => (Tone::Neutral, "Free"),
        Subscription::RenewSoon => (Tone::Warn, "Renew soon"),
        Subscription::Pending => (Tone::Neutral, "Pending"),
        Subscription::Off => (Tone::Neutral, "Off"),
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
