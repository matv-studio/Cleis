//! Contacts (D-39): people and organisations, multi-valued typed fields, folders (spaces),
//! labels, custom columns, links to vault entries.

use crate::column::{ColumnDef, Value};
use crate::ids::{ColumnId, ContactId, EntryId, MemberId, SpaceId, Timestamp, now};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ContactKind {
    #[default]
    Person,
    Organization,
}

/// A value with a type label: `Work`, `Mobile`, `Personal`, or anything the user types.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Labeled<T> {
    pub label: String,
    pub value: T,
    /// Preferred value of its kind (vCard PREF=1).
    pub preferred: bool,
}

impl<T> Labeled<T> {
    pub fn new(label: &str, value: T) -> Self {
        Self {
            label: label.to_owned(),
            value,
            preferred: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Address {
    pub street: String,
    pub extended: String,
    pub po_box: String,
    pub city: String,
    pub region: String,
    pub postal_code: String,
    pub country: String,
}

impl Address {
    /// Reads an address typed on one line: `12 rue de l'Exemple, 69002 Lyon, France`. The first
    /// part is the street; a part starting with digits is the postal code and city; a last part
    /// after that is the country.
    pub fn parse_line(s: &str) -> Self {
        let parts: Vec<&str> = s
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .collect();
        let mut a = Address::default();
        let Some((first, rest)) = parts.split_first() else {
            return a;
        };
        a.street = (*first).to_owned();
        let mut after_city = false;
        for p in rest {
            let starts_digit = p.chars().next().is_some_and(|c| c.is_ascii_digit());
            if !after_city && starts_digit {
                let (code, city) = p.split_once(' ').unwrap_or((p, ""));
                a.postal_code = code.to_owned();
                a.city = city.trim().to_owned();
                after_city = true;
            } else if after_city {
                if a.country.is_empty() {
                    a.country = (*p).to_owned();
                } else {
                    a.country = format!("{}, {p}", a.country);
                }
            } else if a.city.is_empty() && rest.len() == 1 {
                a.city = (*p).to_owned();
            } else if a.extended.is_empty() {
                a.extended = (*p).to_owned();
            } else {
                a.city = (*p).to_owned();
            }
        }
        a
    }

    pub fn one_line(&self) -> String {
        let city = [self.postal_code.as_str(), self.city.as_str()]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(" ");
        [
            self.street.as_str(),
            self.extended.as_str(),
            city.as_str(),
            self.region.as_str(),
            self.country.as_str(),
        ]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(", ")
    }
}

/// A date possibly without year (vCard `--0312`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartialDate {
    pub year: Option<i32>,
    pub month: u8,
    pub day: u8,
}

impl PartialDate {
    /// `1990-03-12`, `19900312`, `--03-12`, `--0312`.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let (year, rest) = if let Some(r) = s.strip_prefix("--") {
            (None, r.replace('-', ""))
        } else {
            let digits = s.replace('-', "");
            let d = digits.split('T').next()?.to_owned();
            if d.len() != 8 {
                return None;
            }
            (Some(d[..4].parse().ok()?), d[4..].to_owned())
        };
        if rest.len() != 4 {
            return None;
        }
        let month: u8 = rest[..2].parse().ok()?;
        let day: u8 = rest[2..].parse().ok()?;
        ((1..=12).contains(&month) && (1..=31).contains(&day)).then_some(Self { year, month, day })
    }

    /// Reads a date as people type it: ISO and vCard forms, `12/03` or `12/03/1990` (day first),
    /// `Mar 12`, `12 mars`, `March 12, 1990`.
    pub fn read(s: &str) -> Option<Self> {
        let s = s.trim();
        if let Some(d) = Self::parse(s) {
            return Some(d);
        }
        let ok = |y: Option<i32>, m: u32, d: u32| {
            ((1..=12).contains(&m) && (1..=31).contains(&d)).then_some(Self {
                year: y,
                month: m as u8,
                day: d as u8,
            })
        };
        let year = |w: &str| w.parse::<i32>().ok().filter(|y| (1000..=9999).contains(y));
        let nums: Vec<&str> = s.split(['/', '.', '-']).map(str::trim).collect();
        if nums.len() >= 2
            && nums
                .iter()
                .all(|n| n.chars().all(|c| c.is_ascii_digit()) && !n.is_empty())
        {
            let d = nums[0].parse().ok()?;
            let m = nums[1].parse().ok()?;
            let y = match nums.get(2) {
                Some(w) => Some(year(w)?),
                None => None,
            };
            return ok(y, m, d);
        }
        let words: Vec<&str> = s
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|w| !w.is_empty())
            .collect();
        let m = words.iter().find_map(|w| crate::import_csv::month(w))?;
        let mut digits = words
            .iter()
            .filter(|w| w.chars().all(|c| c.is_ascii_digit()) || w.ends_with("er"));
        let d = digits.next()?.trim_end_matches("er").parse().ok()?;
        let y = digits.next().and_then(|w| year(w));
        ok(y, m, d)
    }

    /// `Mar 12` or `Mar 12, 1990`.
    pub fn display(&self) -> String {
        const M: [&str; 12] = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        let m = M[usize::from(self.month.clamp(1, 12) - 1)];
        match self.year {
            Some(y) => format!("{m} {}, {y}", self.day),
            None => format!("{m} {}", self.day),
        }
    }

    pub fn to_vcard(&self) -> String {
        match self.year {
            Some(y) => format!("{y:04}{:02}{:02}", self.month, self.day),
            None => format!("--{:02}{:02}", self.month, self.day),
        }
    }

    /// Days from (month, day) `today` to the next occurrence (0 = today), ignoring leap days.
    pub fn days_until(&self, today_month: u8, today_day: u8) -> u32 {
        const CUM: [u32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
        let doy = |m: u8, d: u8| CUM[usize::from(m.clamp(1, 12) - 1)] + u32::from(d);
        let (a, b) = (doy(today_month, today_day), doy(self.month, self.day));
        if b >= a { b - a } else { 365 - a + b }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Contact {
    pub id: ContactId,
    /// Folder: a personal or organisation space.
    pub space: SpaceId,
    pub kind: ContactKind,
    pub prefix: String,
    pub given: String,
    pub additional: String,
    pub family: String,
    pub suffix: String,
    pub nickname: String,
    pub company: String,
    pub department: String,
    pub job_title: String,
    pub emails: Vec<Labeled<String>>,
    pub phones: Vec<Labeled<String>>,
    pub addresses: Vec<Labeled<Address>>,
    pub urls: Vec<Labeled<String>>,
    pub birthday: Option<PartialDate>,
    pub anniversary: Option<PartialDate>,
    pub labels: BTreeSet<String>,
    pub favorite: bool,
    pub notes: String,
    pub linked_entries: Vec<EntryId>,
    pub fields: BTreeMap<ColumnId, Value>,
    pub created: Timestamp,
    pub updated: Timestamp,
    pub updated_by: Option<MemberId>,
}

impl Contact {
    pub fn new(space: SpaceId) -> Self {
        let t = now();
        Self {
            id: ContactId::new(),
            space,
            kind: ContactKind::Person,
            prefix: String::new(),
            given: String::new(),
            additional: String::new(),
            family: String::new(),
            suffix: String::new(),
            nickname: String::new(),
            company: String::new(),
            department: String::new(),
            job_title: String::new(),
            emails: vec![],
            phones: vec![],
            addresses: vec![],
            urls: vec![],
            birthday: None,
            anniversary: None,
            labels: BTreeSet::new(),
            favorite: false,
            notes: String::new(),
            linked_entries: vec![],
            fields: BTreeMap::new(),
            created: t,
            updated: t,
            updated_by: None,
        }
    }

    pub fn person(space: SpaceId, given: &str, family: &str) -> Self {
        let mut c = Self::new(space);
        c.given = given.to_owned();
        c.family = family.to_owned();
        c
    }

    /// Sets the name from one box: the last word is the family name, the rest the given names
    /// (`Camille Dubois`, `Jean-Pierre de La Tour` → given `Jean-Pierre de La`, family `Tour`).
    /// An organisation keeps the text as its company name.
    pub fn set_full_name(&mut self, s: &str) {
        let s = s.trim();
        if self.kind == ContactKind::Organization {
            self.company = s.to_owned();
            return;
        }
        self.prefix.clear();
        self.additional.clear();
        self.suffix.clear();
        match s.rsplit_once(char::is_whitespace) {
            Some((g, f)) => {
                self.given = g.trim().to_owned();
                self.family = f.to_owned();
            }
            None => {
                self.given = s.to_owned();
                self.family.clear();
            }
        }
    }

    /// Plain text card (Copy button): name, title, emails, phones, addresses.
    pub fn text_card(&self) -> String {
        let mut lines = vec![self.display_name()];
        let sub = [self.job_title.as_str(), self.company.as_str()]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        if !sub.is_empty() && sub != lines[0] {
            lines.push(sub);
        }
        for e in &self.emails {
            lines.push(format!("{}: {}", e.label, e.value));
        }
        for p in &self.phones {
            lines.push(format!("{}: {}", p.label, p.value));
        }
        for a in &self.addresses {
            lines.push(format!("{}: {}", a.label, a.value.one_line()));
        }
        lines.join("\n")
    }

    /// Name as displayed and sorted: "Given Family", or the company for organisations.
    pub fn display_name(&self) -> String {
        if self.kind == ContactKind::Organization
            || (self.given.is_empty() && self.family.is_empty())
        {
            if !self.company.is_empty() {
                return self.company.clone();
            }
            if !self.nickname.is_empty() {
                return self.nickname.clone();
            }
            return self
                .emails
                .first()
                .map(|e| e.value.clone())
                .unwrap_or_default();
        }
        [
            self.prefix.as_str(),
            self.given.as_str(),
            self.additional.as_str(),
            self.family.as_str(),
            self.suffix.as_str(),
        ]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
    }

    pub fn initials(&self) -> String {
        let n = self.display_name();
        let mut it = n
            .split_whitespace()
            .filter_map(|w| w.chars().find(|c| c.is_alphanumeric()));
        let a = it.next();
        let b = it.next_back();
        a.into_iter()
            .chain(b)
            .flat_map(char::to_uppercase)
            .collect()
    }

    fn preferred<T: Clone>(v: &[Labeled<T>]) -> Option<T> {
        v.iter()
            .find(|x| x.preferred)
            .or_else(|| v.first())
            .map(|x| x.value.clone())
    }

    pub fn primary_email(&self) -> Option<String> {
        Self::preferred(&self.emails)
    }
    pub fn primary_phone(&self) -> Option<String> {
        Self::preferred(&self.phones)
    }
    pub fn city(&self) -> Option<String> {
        Self::preferred(&self.addresses)
            .map(|a| a.city)
            .filter(|c| !c.is_empty())
    }

    pub fn search_text(&self) -> String {
        let mut s = format!(
            "{} {} {} {} {}",
            self.display_name(),
            self.nickname,
            self.company,
            self.job_title,
            self.notes
        );
        for e in &self.emails {
            s.push(' ');
            s.push_str(&e.value);
        }
        for p in &self.phones {
            s.push(' ');
            s.push_str(&p.value);
            s.push(' ');
            s.push_str(
                &p.value
                    .chars()
                    .filter(char::is_ascii_digit)
                    .collect::<String>(),
            );
        }
        for a in &self.addresses {
            s.push(' ');
            s.push_str(&a.value.one_line());
        }
        for l in &self.labels {
            s.push(' ');
            s.push_str(l);
        }
        s
    }

    pub fn cell(&self, col: &ColumnDef) -> Option<Value> {
        if !col.builtin {
            return self.fields.get(&col.id).cloned();
        }
        let t = |s: String| (!s.is_empty()).then_some(Value::Text(s));
        match col.id.0.as_str() {
            "name" => t(self.display_name()),
            "company" => t(self.company.clone()),
            "job_title" => t(self.job_title.clone()),
            "email" => self.primary_email().map(Value::Text),
            "phone" => self.primary_phone().map(Value::Text),
            "labels" => t(self.labels.iter().cloned().collect::<Vec<_>>().join(", ")),
            "city" => self.city().map(Value::Text),
            "birthday" => self.birthday.map(|b| Value::Date(b.to_vcard())),
            "website" => Self::preferred(&self.urls).map(Value::Text),
            "notes" => t(self.notes.clone()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_values() {
        let a = Address::parse_line("12 rue de l'Exemple, 69002 Lyon, France");
        assert_eq!(
            (
                a.street.as_str(),
                a.postal_code.as_str(),
                a.city.as_str(),
                a.country.as_str()
            ),
            ("12 rue de l'Exemple", "69002", "Lyon", "France")
        );
        assert_eq!(a.one_line(), "12 rue de l'Exemple, 69002 Lyon, France");
        let d = |s| PartialDate::read(s).map(|d| (d.year, d.month, d.day));
        assert_eq!(d("12/03"), Some((None, 3, 12)));
        assert_eq!(d("12/03/1990"), Some((Some(1990), 3, 12)));
        assert_eq!(d("Mar 12"), Some((None, 3, 12)));
        assert_eq!(d("12 mars 1990"), Some((Some(1990), 3, 12)));
        assert_eq!(d("1er avril"), Some((None, 4, 1)));
        assert_eq!(d("1990-03-12"), Some((Some(1990), 3, 12)));
        assert_eq!(d("31/02/x"), None);
        assert_eq!(
            PartialDate::read("Mar 12, 1990").unwrap().display(),
            "Mar 12, 1990"
        );
        let mut c = Contact::new(SpaceId::new());
        c.set_full_name("Jean-Pierre de La Tour");
        assert_eq!(
            (c.given.as_str(), c.family.as_str()),
            ("Jean-Pierre de La", "Tour")
        );
    }

    #[test]
    fn names_and_dates() {
        let mut c = Contact::person(SpaceId::new(), "Camille", "Dubois");
        assert_eq!(c.display_name(), "Camille Dubois");
        assert_eq!(c.initials(), "CD");
        c.kind = ContactKind::Organization;
        c.company = "Example Print".into();
        assert_eq!(c.display_name(), "Example Print");
        let b = PartialDate::parse("--0312").unwrap();
        assert_eq!((b.year, b.month, b.day), (None, 3, 12));
        assert_eq!(
            PartialDate::parse("1990-03-12").unwrap().to_vcard(),
            "19900312"
        );
        assert_eq!(PartialDate::parse("1990-13-12"), None);
        assert_eq!(b.days_until(3, 1), 11);
        assert_eq!(b.days_until(3, 13), 364);
    }
}
