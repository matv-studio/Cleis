//! Editing a contact from the boxes of its inspector (screen 20): one function reads the text of
//! a box into the contact, refusing what does not fit.

use crate::contact::{Address, Contact, Labeled, PartialDate};
use thiserror::Error;

/// A box of the contact inspector. Multi-valued fields carry their row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Field {
    Name,
    JobTitle,
    Company,
    Email(usize),
    Phone(usize),
    Address(usize),
    Url(usize),
    Birthday,
    Notes,
}

/// Which multi-valued list a row belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum List {
    Email,
    Phone,
    Address,
    Url,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EditError {
    #[error("Not an email address")]
    Email,
    #[error("Not a phone number")]
    Phone,
    #[error("Not a date (Mar 12, 12/03/1990)")]
    Date,
}

/// What a change did.
#[derive(Debug, PartialEq, Eq)]
pub enum Applied {
    Unchanged,
    Changed,
    /// A row was emptied and removed: the boxes must be rebuilt.
    Removed,
}

/// Type labels offered for each list, in the order a click goes through them.
pub fn labels(list: List) -> &'static [&'static str] {
    match list {
        List::Email => &["Work", "Personal", "Other"],
        List::Phone => &["Work", "Mobile", "Home", "Other"],
        List::Address => &["Work", "Home", "Other"],
        List::Url => &["Work", "Personal", "Other"],
    }
}

/// The label after `current` (a custom label goes back to the first one).
pub fn next_label(list: List, current: &str) -> &'static str {
    let l = labels(list);
    let i = l.iter().position(|x| x.eq_ignore_ascii_case(current));
    l[i.map_or(0, |i| (i + 1) % l.len())]
}

/// Text shown in a box.
pub fn text(c: &Contact, f: Field) -> String {
    match f {
        Field::Name => {
            if c.kind == crate::contact::ContactKind::Organization {
                c.company.clone()
            } else {
                c.display_name()
            }
        }
        Field::JobTitle => c.job_title.clone(),
        Field::Company => c.company.clone(),
        Field::Email(i) => c.emails.get(i).map(|x| x.value.clone()).unwrap_or_default(),
        Field::Phone(i) => c.phones.get(i).map(|x| x.value.clone()).unwrap_or_default(),
        Field::Address(i) => c
            .addresses
            .get(i)
            .map(|x| x.value.one_line())
            .unwrap_or_default(),
        Field::Url(i) => c.urls.get(i).map(|x| x.value.clone()).unwrap_or_default(),
        Field::Birthday => c.birthday.map(|b| b.display()).unwrap_or_default(),
        Field::Notes => c.notes.clone(),
    }
}

fn set_row<T: PartialEq>(v: &mut Vec<Labeled<T>>, i: usize, value: Option<T>) -> Applied {
    match value {
        None if i < v.len() => {
            v.remove(i);
            Applied::Removed
        }
        None => Applied::Unchanged,
        Some(x) => match v.get_mut(i) {
            Some(row) if row.value == x => Applied::Unchanged,
            Some(row) => {
                row.value = x;
                Applied::Changed
            }
            None => Applied::Unchanged,
        },
    }
}

/// Reads `value` into the field. An emptied row is removed; a refused value changes nothing.
pub fn apply(c: &mut Contact, f: Field, value: &str) -> Result<Applied, EditError> {
    let v = value.trim();
    let changed = |b: bool| {
        if b {
            Applied::Changed
        } else {
            Applied::Unchanged
        }
    };
    let r = match f {
        Field::Name => {
            if v == text(c, f) {
                Applied::Unchanged
            } else {
                c.set_full_name(v);
                Applied::Changed
            }
        }
        Field::JobTitle => {
            let b = c.job_title != v;
            c.job_title = v.to_owned();
            changed(b)
        }
        Field::Company => {
            let b = c.company != v;
            c.company = v.to_owned();
            changed(b)
        }
        Field::Email(i) => {
            if !v.is_empty() && !(v.contains('@') && !v.contains(' ')) {
                return Err(EditError::Email);
            }
            set_row(&mut c.emails, i, (!v.is_empty()).then(|| v.to_owned()))
        }
        Field::Phone(i) => {
            let digits = v.chars().filter(char::is_ascii_digit).count();
            let allowed = v
                .chars()
                .all(|ch| ch.is_ascii_digit() || " +-().".contains(ch));
            if !v.is_empty() && (digits < 3 || !allowed) {
                return Err(EditError::Phone);
            }
            set_row(&mut c.phones, i, (!v.is_empty()).then(|| v.to_owned()))
        }
        Field::Address(i) => {
            let a = (!v.is_empty()).then(|| Address::parse_line(v));
            // Same text: keep the stored parts (region, PO box…) untouched.
            if a.as_ref().map(Address::one_line).as_deref() == Some(&text(c, f)) {
                Applied::Unchanged
            } else {
                set_row(&mut c.addresses, i, a)
            }
        }
        Field::Url(i) => set_row(&mut c.urls, i, (!v.is_empty()).then(|| v.to_owned())),
        Field::Birthday => {
            let d = if v.is_empty() {
                None
            } else {
                Some(PartialDate::read(v).ok_or(EditError::Date)?)
            };
            let b = c.birthday != d;
            c.birthday = d;
            changed(b)
        }
        Field::Notes => {
            let b = c.notes != value;
            c.notes = value.to_owned();
            changed(b)
        }
    };
    if r != Applied::Unchanged {
        c.updated = crate::now();
    }
    Ok(r)
}

/// Adds an empty row with the list's first label; returns its field.
pub fn add_row(c: &mut Contact, list: List) -> Field {
    let label = labels(list)[0];
    match list {
        List::Email => {
            c.emails.push(Labeled::new(label, String::new()));
            Field::Email(c.emails.len() - 1)
        }
        List::Phone => {
            let label = if c.phones.is_empty() { label } else { "Mobile" };
            c.phones.push(Labeled::new(label, String::new()));
            Field::Phone(c.phones.len() - 1)
        }
        List::Address => {
            c.addresses.push(Labeled::new(label, Address::default()));
            Field::Address(c.addresses.len() - 1)
        }
        List::Url => {
            c.urls.push(Labeled::new(label, String::new()));
            Field::Url(c.urls.len() - 1)
        }
    }
}

/// Moves a row to the next type label.
pub fn cycle_label(c: &mut Contact, list: List, i: usize) {
    let cur = match list {
        List::Email => c.emails.get_mut(i).map(|x| &mut x.label),
        List::Phone => c.phones.get_mut(i).map(|x| &mut x.label),
        List::Address => c.addresses.get_mut(i).map(|x| &mut x.label),
        List::Url => c.urls.get_mut(i).map(|x| &mut x.label),
    };
    if let Some(l) = cur {
        *l = next_label(list, l).to_owned();
        c.updated = crate::now();
    }
}

/// Removes rows left empty (a row added and never filled).
pub fn drop_empty_rows(c: &mut Contact) {
    c.emails.retain(|x| !x.value.trim().is_empty());
    c.phones.retain(|x| !x.value.trim().is_empty());
    c.urls.retain(|x| !x.value.trim().is_empty());
    c.addresses
        .retain(|x| !x.value.one_line().trim().is_empty());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::SpaceId;

    #[test]
    fn edit_rows_and_refusals() {
        let mut c = Contact::person(SpaceId::new(), "Camille", "Dubois");
        let f = add_row(&mut c, List::Email);
        assert_eq!(apply(&mut c, f, "camille"), Err(EditError::Email));
        assert_eq!(
            apply(&mut c, f, "camille@example-print.com"),
            Ok(Applied::Changed)
        );
        let p = add_row(&mut c, List::Phone);
        assert_eq!(apply(&mut c, p, "call me"), Err(EditError::Phone));
        assert_eq!(apply(&mut c, p, "+33 1 99 00 12 34"), Ok(Applied::Changed));
        cycle_label(&mut c, List::Phone, 0);
        assert_eq!(c.phones[0].label, "Mobile");
        assert_eq!(apply(&mut c, Field::Email(0), ""), Ok(Applied::Removed));
        assert!(c.emails.is_empty());
        assert_eq!(
            apply(&mut c, Field::Birthday, "12 mars"),
            Ok(Applied::Changed)
        );
        assert_eq!(text(&c, Field::Birthday), "Mar 12");
        assert_eq!(apply(&mut c, Field::Birthday, "soon"), Err(EditError::Date));
        assert_eq!(
            apply(&mut c, Field::Name, "Camille Dubois"),
            Ok(Applied::Unchanged)
        );
        assert_eq!(
            apply(&mut c, Field::Name, "Camille Martin"),
            Ok(Applied::Changed)
        );
        assert_eq!(c.family, "Martin");
        let a = add_row(&mut c, List::Address);
        apply(&mut c, a, "12 rue de l'Exemple, 69002 Lyon").unwrap();
        assert_eq!(c.city().as_deref(), Some("Lyon"));
        add_row(&mut c, List::Url);
        drop_empty_rows(&mut c);
        assert!(c.urls.is_empty());
    }
}
