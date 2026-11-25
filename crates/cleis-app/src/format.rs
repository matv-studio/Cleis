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
