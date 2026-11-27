//! Phone numbers shown in each country's own format (0.2.4, user's choice of 28/11/2026): France
//! in pairs (`06 12 34 56 78`, `+33 6 12 34 56 78`), Switzerland `+41 79 123 45 67`, Germany
//! `+49 30 12345678`… from Google's libphonenumber metadata (`rlibphonenumber`).
//!
//! A number typed without a country code is taken as French and stays national; one with `+`
//! (or `00`) is shown with its code. A number the metadata does not accept (too short, a
//! service code, letters) is left as typed.

use rlibphonenumber::{PhoneNumber, PhoneNumberFormat, enums::Region};

/// `number` in its country's format, or as typed when it is not a valid number.
pub fn format(number: &str) -> String {
    let t = number.trim();
    if t.is_empty() {
        return String::new();
    }
    let Ok(n) = PhoneNumber::parse(t, Some(Region::FR)) else {
        return t.to_owned();
    };
    if !n.is_valid() {
        return t.to_owned();
    }
    let international = t.starts_with('+') || t.starts_with("00");
    let style = if international {
        PhoneNumberFormat::International
    } else {
        PhoneNumberFormat::National
    };
    n.format_as(style).to_string()
}

#[cfg(test)]
mod tests {
    use super::format;

    #[test]
    fn each_country_its_format() {
        // France: pairs, national without code, international with it.
        assert_eq!(format("0612345678"), "06 12 34 56 78");
        assert_eq!(format("06.12.34.56.78"), "06 12 34 56 78");
        assert_eq!(format("+33612345678"), "+33 6 12 34 56 78");
        assert_eq!(format("0033 6 12 34 56 78"), "+33 6 12 34 56 78");
        // Other countries: their own groups, not pairs everywhere.
        assert_eq!(format("+41791234567"), "+41 79 123 45 67");
        assert_eq!(format("+49 30 12345678"), "+49 30 12345678");
        assert_eq!(format("+31104081775"), "+31 10 408 1775");
        assert_eq!(format("+32470123456"), "+32 470 12 34 56");
        assert_eq!(format("+390299001122"), "+39 02 9900 1122");
        // Not a number the metadata accepts: as typed.
        assert_eq!(format(" 0612 "), "0612");
        assert_eq!(format("3615"), "3615");
        assert_eq!(format(""), "");
    }
}
