//! Editing an item field by field from text, as typed in the inspector (screen 01). A value that
//! cannot be read is refused and nothing is written (charter: refused field, red border and
//! message); the interface keeps the text so the person can fix it.

use crate::entry::{Entry, Subscription};
use crate::import_csv::parse_date;
use crate::money::{Money, Period};
use crate::totp::Totp;
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Field {
    Name,
    Website,
    Username,
    Password,
    Totp,
    Notes,
    Price,
    Licenses,
    Renewal,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EditError {
    #[error("A name is required")]
    EmptyName,
    #[error("Not a 2FA secret or otpauth:// link")]
    Totp,
    #[error("Not a price, e.g. €15 / month")]
    Price,
    #[error("Not a whole number")]
    Licenses,
    #[error("Not a date, e.g. 2026-12-03 or 3 Dec 2026")]
    Date,
}

/// Text shown in the field for an item.
pub fn text(e: &Entry, f: Field) -> String {
    match f {
        Field::Name => e.name.clone(),
        Field::Website => e.urls.first().cloned().unwrap_or_default(),
        Field::Username => e.username.clone(),
        Field::Password => e.password.clone(),
        Field::Totp => e.totp.as_ref().map(totp_text).unwrap_or_default(),
        Field::Notes => e.notes.clone(),
        // The amount alone: its period is the button beside the box (mockup 48h).
        Field::Price => e.price.as_ref().map(amount_text).unwrap_or_default(),
        Field::Licenses => e.licenses.map(|l| l.to_string()).unwrap_or_default(),
        Field::Renewal => e.renewal.clone().unwrap_or_default(),
    }
}

/// The key as people paste it: base32 for the usual 6 digits / 30 s / SHA-1, else the full
/// `otpauth://` link so nothing is lost.
pub fn totp_text(t: &Totp) -> String {
    let usual = t.digits == 6 && t.period == 30 && t.algorithm == crate::totp::Algorithm::Sha1;
    if usual && t.issuer.is_none() && t.account.is_none() {
        data_encoding::BASE32_NOPAD.encode(&t.secret)
    } else {
        t.to_uri()
    }
}

/// `€15.00 / month` (`15.00 CHF / month` without a symbol): read back by [`apply`] unchanged.
/// `€15.00`: the amount and its currency, without the period.
pub fn amount_text(m: &Money) -> String {
    price_text(&Money {
        period: None,
        ..m.clone()
    })
}

/// The period of a price, as its button says it (mockup 48h), and the one a click gives:
/// month and year in turn; a quarter or a single payment goes to month. No period counts as
/// once a year in the yearly cost, so it reads "/ year".
pub fn period_label(p: Option<Period>) -> (&'static str, Period) {
    match p {
        Some(Period::Month) => ("/ month", Period::Year),
        Some(Period::Year) | None => ("/ year", Period::Month),
        Some(Period::Quarter) => ("/ quarter", Period::Month),
        Some(Period::OneTime) => ("once", Period::Month),
    }
}

pub fn price_text(m: &Money) -> String {
    let c = m.cents.abs();
    let sign = if m.cents < 0 { "-" } else { "" };
    let period = match m.period {
        Some(Period::Month) => " / month",
        Some(Period::Quarter) => " / quarter",
        Some(Period::Year) => " / year",
        Some(Period::OneTime) | None => "",
    };
    let amount = format!("{}.{:02}", c / 100, c % 100);
    match m.currency.as_str() {
        "EUR" => format!("{sign}€{amount}{period}"),
        "USD" => format!("{sign}${amount}{period}"),
        "GBP" => format!("{sign}£{amount}{period}"),
        other => format!("{sign}{amount} {other}{period}"),
    }
}

/// Writes `value` into field `f`. Returns whether the item changed. `currency` applies to a
/// price typed without one.
pub fn apply(e: &mut Entry, f: Field, value: &str, currency: &str) -> Result<bool, EditError> {
    let v = value.trim();
    let before = text(e, f);
    if f != Field::Password && v == before.trim() {
        return Ok(false);
    }
    match f {
        Field::Name => {
            if v.is_empty() {
                return Err(EditError::EmptyName);
            }
            e.name = v.to_owned();
        }
        Field::Website => {
            if v.is_empty() {
                if !e.urls.is_empty() {
                    e.urls.remove(0);
                }
            } else if e.urls.is_empty() {
                e.urls.push(v.to_owned());
            } else {
                e.urls[0] = v.to_owned();
            }
        }
        Field::Username => e.username = v.to_owned(),
        // Passwords keep their spaces: they are part of the secret.
        Field::Password => {
            if value == e.password {
                return Ok(false);
            }
            e.set_password(value.to_owned(), None);
            return Ok(true);
        }
        Field::Totp => {
            e.totp = if v.is_empty() {
                None
            } else {
                Some(Totp::parse(v).map_err(|_| EditError::Totp)?)
            };
        }
        Field::Notes => e.notes = value.trim_end().to_owned(),
        Field::Price => {
            e.price = if v.is_empty() {
                None
            } else {
                let period = e.price.as_ref().and_then(|p| p.period);
                Some(Money::parse(v, currency, period).ok_or(EditError::Price)?)
            };
        }
        Field::Licenses => {
            e.licenses = if v.is_empty() {
                None
            } else {
                Some(v.parse::<u32>().map_err(|_| EditError::Licenses)?)
            };
        }
        Field::Renewal => {
            e.renewal = if v.is_empty() {
                None
            } else {
                Some(parse_date(v).ok_or(EditError::Date)?)
            };
        }
    }
    let changed = text(e, f) != before;
    if changed {
        e.touch(None);
    }
    Ok(changed)
}

/// Subscription states offered in the inspector, in menu order.
pub const SUBSCRIPTIONS: [Subscription; 6] = [
    Subscription::None,
    Subscription::Active,
    Subscription::Free,
    Subscription::RenewSoon,
    Subscription::Pending,
    Subscription::Off,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::SpaceId;

    fn e() -> Entry {
        Entry::new(SpaceId::new(), "Figma")
    }

    #[test]
    fn fields_round_trip() {
        let mut x = e();
        for (f, v) in [
            (Field::Website, "figma.com"),
            (Field::Username, "design@example-studio.com"),
            (Field::Password, " pass with spaces "),
            (Field::Totp, "JBSWY3DPEHPK3PXP"),
            (Field::Notes, "Team plan"),
            (Field::Price, "€15.00"),
            (Field::Licenses, "4"),
            (Field::Renewal, "2026-12-03"),
        ] {
            assert_eq!(apply(&mut x, f, v, "EUR"), Ok(true), "{f:?}");
            assert_eq!(text(&x, f), v, "{f:?}");
            assert_eq!(apply(&mut x, f, v, "EUR"), Ok(false), "{f:?} twice");
        }
        // No period: once a year; the period button (mockup 48h) goes to month, then back.
        assert_eq!(x.price.as_ref().unwrap().yearly_cents(), 1_500);
        let (label, next) = period_label(x.price.as_ref().unwrap().period);
        assert_eq!(label, "/ year");
        x.price.as_mut().unwrap().period = Some(next);
        assert_eq!(x.price.as_ref().unwrap().yearly_cents(), 18_000);
        assert_eq!(period_label(Some(next)), ("/ month", Period::Year));
        assert_eq!(
            text(&x, Field::Price),
            "€15.00",
            "the box keeps the amount alone"
        );
    }

    #[test]
    fn typed_forms_are_read() {
        let mut x = e();
        let per = |x: &Entry| x.price.as_ref().and_then(|p| p.period);
        apply(&mut x, Field::Price, "€9,99/mo", "EUR").unwrap();
        assert_eq!(text(&x, Field::Price), "€9.99");
        assert_eq!(per(&x), Some(Period::Month));
        // A new amount keeps the period already set.
        apply(&mut x, Field::Price, "12", "EUR").unwrap();
        assert_eq!(text(&x, Field::Price), "€12.00");
        assert_eq!(per(&x), Some(Period::Month));
        apply(&mut x, Field::Price, "30 CHF / year", "EUR").unwrap();
        assert_eq!(text(&x, Field::Price), "30.00 CHF");
        assert_eq!(per(&x), Some(Period::Year));
        apply(&mut x, Field::Renewal, "3 décembre 2026", "EUR").unwrap();
        assert_eq!(x.renewal.as_deref(), Some("2026-12-03"));
        apply(
            &mut x,
            Field::Totp,
            "otpauth://totp/Figma:design?secret=JBSWY3DPEHPK3PXP&issuer=Figma",
            "EUR",
        )
        .unwrap();
        assert!(x.totp.is_some());
    }

    #[test]
    fn refused_values_write_nothing() {
        let mut x = e();
        apply(&mut x, Field::Licenses, "2", "EUR").unwrap();
        assert_eq!(
            apply(&mut x, Field::Licenses, "two", "EUR"),
            Err(EditError::Licenses)
        );
        assert_eq!(x.licenses, Some(2));
        assert_eq!(
            apply(&mut x, Field::Name, "  ", "EUR"),
            Err(EditError::EmptyName)
        );
        assert_eq!(x.name, "Figma");
        assert_eq!(
            apply(&mut x, Field::Totp, "not a secret!", "EUR"),
            Err(EditError::Totp)
        );
        assert_eq!(
            apply(&mut x, Field::Renewal, "someday", "EUR"),
            Err(EditError::Date)
        );
        assert_eq!(
            apply(&mut x, Field::Price, "a lot", "EUR"),
            Err(EditError::Price)
        );
    }

    #[test]
    fn password_change_keeps_history() {
        let mut x = e();
        apply(&mut x, Field::Password, "first", "EUR").unwrap();
        apply(&mut x, Field::Password, "second", "EUR").unwrap();
        assert_eq!(x.password_history.len(), 1);
        assert_eq!(x.password_history[0].password, "first");
    }

    #[test]
    fn clearing_fields() {
        let mut x = e();
        apply(&mut x, Field::Website, "figma.com", "EUR").unwrap();
        apply(&mut x, Field::Website, "", "EUR").unwrap();
        assert!(x.urls.is_empty());
        apply(&mut x, Field::Totp, "JBSWY3DPEHPK3PXP", "EUR").unwrap();
        apply(&mut x, Field::Totp, "", "EUR").unwrap();
        assert!(x.totp.is_none());
    }
}
