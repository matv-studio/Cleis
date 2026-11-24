//! Prices: exact integer cents, a currency and an optional billing period.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Period {
    Month,
    Quarter,
    Year,
    OneTime,
}

impl Period {
    pub fn per_year(self) -> i64 {
        match self {
            Period::Month => 12,
            Period::Quarter => 4,
            Period::Year | Period::OneTime => 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    /// Amount in minor units (cents).
    pub cents: i64,
    /// ISO 4217 code.
    pub currency: String,
    pub period: Option<Period>,
}

impl Money {
    pub fn new(cents: i64, currency: &str, period: Option<Period>) -> Self {
        Self {
            cents,
            currency: currency.to_owned(),
            period,
        }
    }

    /// Amount over a year (one-time and unknown periods count once).
    pub fn yearly_cents(&self) -> i64 {
        self.cents * self.period.map_or(1, Period::per_year)
    }

    /// Parses inputs as typed by people or found in exports: `15 €`, `€9.99`, `9,99`,
    /// `1 234,50 €/mois`, `29 EUR / an`, `$12/mo`. `default_currency` applies when none
    /// is written; `default_period` when none is written.
    pub fn parse(
        input: &str,
        default_currency: &str,
        default_period: Option<Period>,
    ) -> Option<Self> {
        let s = crate::text::fold(input);
        if s.is_empty() || s == "-" || s == "—" {
            return None;
        }
        let currency = if s.contains('€') || s.contains("eur") {
            "EUR"
        } else if s.contains('$') || s.contains("usd") {
            "USD"
        } else if s.contains('£') || s.contains("gbp") {
            "GBP"
        } else if s.contains("chf") {
            "CHF"
        } else {
            default_currency
        };
        let period = detect_period(&s).or(default_period);
        // Keep digits and separators of the first number.
        let mut num = String::new();
        let mut started = false;
        for c in s.chars() {
            if c.is_ascii_digit()
                || (started
                    && (c == ',' || c == '.' || c == ' ' || c == '\u{a0}' || c == '\u{202f}'))
            {
                started = true;
                num.push(c);
            } else if c == '-' && !started {
                num.push(c);
            } else if started {
                break;
            }
        }
        let num: String = num.chars().filter(|c| !c.is_whitespace()).collect();
        let num = num.trim_end_matches([',', '.']);
        if num.is_empty() || num == "-" {
            return None;
        }
        Some(Self {
            cents: parse_decimal_cents(num)?,
            currency: currency.to_owned(),
            period,
        })
    }

    /// `€15.00` / `15,00 €` style formatting is left to the UI (locale); this is the
    /// neutral form used in exports and tests.
    pub fn to_plain(&self) -> String {
        let sign = if self.cents < 0 { "-" } else { "" };
        let c = self.cents.abs();
        format!("{sign}{}.{:02} {}", c / 100, c % 100, self.currency)
    }
}

fn detect_period(s: &str) -> Option<Period> {
    const MONTH: [&str; 7] = [
        "/mois",
        "par mois",
        "mensuel",
        "/mo",
        "monthly",
        "per month",
        "/month",
    ];
    const YEAR: [&str; 9] = [
        "/an", "par an", "annuel", "/yr", "/year", "yearly", "annual", "per year", "/annee",
    ];
    const QUARTER: [&str; 3] = ["trimestr", "quarter", "/trim"];
    let compact: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if MONTH
        .iter()
        .any(|m| s.contains(m) || compact.contains(&m.replace(' ', "")))
    {
        Some(Period::Month)
    } else if YEAR
        .iter()
        .any(|m| s.contains(m) || compact.contains(&m.replace(' ', "")))
    {
        Some(Period::Year)
    } else if QUARTER.iter().any(|m| s.contains(m)) {
        Some(Period::Quarter)
    } else {
        None
    }
}

/// Header-level hint: "Prix (mensuel)" means monthly prices.
pub fn period_hint(header: &str) -> Option<Period> {
    detect_period(&crate::text::fold(header))
}

/// `1234,5` / `1,234.50` / `1.234,50` → cents. The last separator followed by one or two
/// digits is the decimal separator; others are thousands separators.
fn parse_decimal_cents(num: &str) -> Option<i64> {
    let neg = num.starts_with('-');
    let num = num.trim_start_matches('-');
    let last_sep = num.rfind([',', '.']);
    let (int_part, frac_part) = match last_sep {
        Some(i) if num.len() - i - 1 <= 2 => (&num[..i], &num[i + 1..]),
        _ => (num, ""),
    };
    let int_digits: String = int_part.chars().filter(char::is_ascii_digit).collect();
    let int: i64 = if int_digits.is_empty() {
        0
    } else {
        int_digits.parse().ok()?
    };
    let frac: i64 = match frac_part.len() {
        0 => 0,
        1 => frac_part.parse::<i64>().ok()? * 10,
        _ => frac_part.parse().ok()?,
    };
    let v = int.checked_mul(100)?.checked_add(frac)?;
    Some(if neg { -v } else { v })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_inputs() {
        let m = Money::parse("15 €", "EUR", None).unwrap();
        assert_eq!(
            (m.cents, m.currency.as_str(), m.period),
            (1500, "EUR", None)
        );
        assert_eq!(Money::parse("€9.99", "USD", None).unwrap().cents, 999);
        assert_eq!(Money::parse("9,99", "EUR", None).unwrap().cents, 999);
        assert_eq!(
            Money::parse("1 234,5 €/mois", "EUR", None).unwrap(),
            Money::new(123_450, "EUR", Some(Period::Month))
        );
        assert_eq!(
            Money::parse("1,234.50 USD per year", "EUR", None).unwrap(),
            Money::new(123_450, "USD", Some(Period::Year))
        );
        assert_eq!(
            Money::parse("29 EUR / an", "EUR", None).unwrap().period,
            Some(Period::Year)
        );
        assert_eq!(
            Money::parse("$12/mo", "EUR", None).unwrap(),
            Money::new(1200, "USD", Some(Period::Month))
        );
        assert_eq!(Money::parse("—", "EUR", None), None);
        assert_eq!(Money::parse("gratuit", "EUR", None), None);
        assert_eq!(Money::parse("1.000", "EUR", None).unwrap().cents, 100_000);
    }

    #[test]
    fn yearly() {
        assert_eq!(
            Money::new(1500, "EUR", Some(Period::Month)).yearly_cents(),
            18_000
        );
        assert_eq!(
            Money::new(11_000, "EUR", Some(Period::Year)).yearly_cents(),
            11_000
        );
        assert_eq!(period_hint("Prix (mensuel)"), Some(Period::Month));
    }
}
