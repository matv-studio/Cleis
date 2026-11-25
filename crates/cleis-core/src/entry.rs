//! Vault entries (logins, subscriptions, secrets).

use crate::column::{ColumnDef, ColumnKind, Value};
use crate::formula::{FormulaError, Num, Row};
use crate::ids::{ColumnId, EntryId, MemberId, SpaceId, Timestamp, now};
use crate::money::Money;
use crate::totp::Totp;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Subscription {
    #[default]
    None,
    Active,
    Free,
    RenewSoon,
    Pending,
    Off,
}

impl Subscription {
    pub fn id(self) -> &'static str {
        match self {
            Subscription::None => "",
            Subscription::Active => "active",
            Subscription::Free => "free",
            Subscription::RenewSoon => "renew_soon",
            Subscription::Pending => "pending",
            Subscription::Off => "off",
        }
    }
}

/// A desktop application recognised for autofill (D-31).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedApp {
    /// Executable file name on Windows (`Figma.exe`) or bundle id on macOS.
    pub program: String,
    pub display_name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct PasswordHistory {
    pub password: String,
    #[zeroize(skip)]
    pub replaced_at: Timestamp,
    #[zeroize(skip)]
    pub replaced_by: Option<MemberId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub id: EntryId,
    pub space: SpaceId,
    pub name: String,
    pub username: String,
    pub password: String,
    pub totp: Option<Totp>,
    pub urls: Vec<String>,
    pub notes: String,
    pub favorite: bool,
    pub subscription: Subscription,
    pub price: Option<Money>,
    pub licenses: Option<u32>,
    /// ISO date `YYYY-MM-DD`.
    pub renewal: Option<String>,
    pub linked_apps: Vec<LinkedApp>,
    /// Values of custom columns.
    pub fields: BTreeMap<ColumnId, Value>,
    pub password_history: Vec<PasswordHistory>,
    pub password_changed: Timestamp,
    pub created: Timestamp,
    pub updated: Timestamp,
    pub updated_by: Option<MemberId>,
}

impl Drop for Entry {
    fn drop(&mut self) {
        self.password.zeroize();
    }
}

impl Entry {
    pub fn new(space: SpaceId, name: impl Into<String>) -> Self {
        let t = now();
        Self {
            id: EntryId::new(),
            space,
            name: name.into(),
            username: String::new(),
            password: String::new(),
            totp: None,
            urls: vec![],
            notes: String::new(),
            favorite: false,
            subscription: Subscription::None,
            price: None,
            licenses: None,
            renewal: None,
            linked_apps: vec![],
            fields: BTreeMap::new(),
            password_history: vec![],
            password_changed: t,
            created: t,
            updated: t,
            updated_by: None,
        }
    }

    /// Changes the password and keeps the previous one in the history (Inspector › History).
    pub fn set_password(&mut self, new: String, by: Option<MemberId>) {
        if new == self.password {
            return;
        }
        let t = now();
        if !self.password.is_empty() {
            self.password_history.push(PasswordHistory {
                password: std::mem::take(&mut self.password),
                replaced_at: t,
                replaced_by: by,
            });
        }
        self.password = new;
        self.password_changed = t;
        self.touch(by);
    }

    pub fn touch(&mut self, by: Option<MemberId>) {
        self.updated = now();
        self.updated_by = by;
    }

    /// Main website host, for matching browser tabs and icons.
    pub fn host(&self) -> Option<String> {
        self.urls.iter().find_map(|u| {
            let with_scheme = if u.contains("://") {
                u.clone()
            } else {
                format!("https://{u}")
            };
            url::Url::parse(&with_scheme)
                .ok()?
                .host_str()
                .map(|h| h.trim_start_matches("www.").to_owned())
        })
    }

    /// Text searched by the vault search box.
    pub fn search_text(&self) -> String {
        let mut s = format!("{} {} {}", self.name, self.username, self.notes);
        for u in &self.urls {
            s.push(' ');
            s.push_str(u);
        }
        for v in self.fields.values() {
            if let Some(t) = v.as_text() {
                s.push(' ');
                s.push_str(t);
            }
        }
        s
    }

    /// Value of a column for formulas and sorting.
    pub fn cell(&self, col: &ColumnDef) -> Option<Value> {
        if !col.builtin {
            return self.fields.get(&col.id).cloned();
        }
        match col.id.0.as_str() {
            "name" => Some(Value::Text(self.name.clone())),
            "username" => Some(Value::Text(self.username.clone())),
            "subscription" => (self.subscription != Subscription::None)
                .then(|| Value::Choice(self.subscription.id().into())),
            "price" => self.price.clone().map(Value::Money),
            "licenses" => self.licenses.map(|l| Value::Number(f64::from(l))),
            "renewal" => self.renewal.clone().map(Value::Date),
            "website" => self.urls.first().cloned().map(Value::Text),
            "notes" => Some(Value::Text(self.notes.clone())),
            _ => None,
        }
    }

    /// Computes a formula column for this entry.
    pub fn compute(&self, expr: &str, columns: &[ColumnDef]) -> Result<Option<Num>, FormulaError> {
        struct R<'a>(&'a Entry, &'a [ColumnDef]);
        impl Row for R<'_> {
            fn get(&self, name: &str) -> Result<Option<Num>, FormulaError> {
                let f = crate::text::fold(name);
                let col = self
                    .1
                    .iter()
                    .find(|c| crate::text::fold(&c.name) == f)
                    .ok_or_else(|| FormulaError::UnknownColumn(name.to_owned()))?;
                if let ColumnKind::Formula { .. } = col.kind {
                    return Err(FormulaError::UnknownColumn(name.to_owned()));
                }
                Ok(match self.0.cell(col) {
                    Some(Value::Number(n)) => Some(Num::plain(n)),
                    Some(Value::Money(m)) => Some(Num {
                        value: m.cents as f64 / 100.0,
                        currency: Some(m.currency.clone()),
                        per_year: m.period.map(|p| p.per_year() as f64),
                    }),
                    Some(Value::Text(t)) => t.trim().replace(',', ".").parse().ok().map(Num::plain),
                    _ => None,
                })
            }
        }
        crate::formula::eval(expr, &R(self, columns))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::default_vault_columns;
    use crate::money::Period;

    #[test]
    fn yearly_cost_column() {
        let cols = default_vault_columns("EUR");
        let mut e = Entry::new(SpaceId::new(), "Figma");
        e.price = Some(Money::new(1500, "EUR", Some(Period::Month)));
        e.licenses = Some(4);
        let ColumnKind::Formula { expr } =
            &cols.iter().find(|c| c.id.0 == "yearly_cost").unwrap().kind
        else {
            panic!()
        };
        let v = e.compute(expr, &cols).unwrap().unwrap();
        assert_eq!(v.value, 720.0);
        e.licenses = None;
        assert_eq!(e.compute(expr, &cols).unwrap().unwrap().value, 180.0);
        e.price = None;
        assert_eq!(e.compute(expr, &cols).unwrap(), None);
    }

    #[test]
    fn password_history_and_host() {
        let mut e = Entry::new(SpaceId::new(), "OVHcloud");
        e.set_password("first".into(), None);
        e.set_password("second".into(), None);
        e.set_password("second".into(), None);
        assert_eq!(e.password_history.len(), 1);
        assert_eq!(e.password_history[0].password, "first");
        e.urls = vec!["www.ovhcloud.com/fr/".into()];
        assert_eq!(e.host().as_deref(), Some("ovhcloud.com"));
    }
}
