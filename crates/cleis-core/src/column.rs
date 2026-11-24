//! Columns: shared by the vault and the contacts. Built-in columns map to typed fields;
//! custom columns store a [`Value`] per row. Users show, hide, reorder, add and delete them.

use crate::ids::{ColumnId, SpaceId};
use crate::money::{Money, Period};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ColumnKind {
    Text,
    /// Masked in the UI, copied without showing, never exported in clear without permission.
    Secret,
    Number {
        decimals: u8,
    },
    Price {
        currency: String,
        default_period: Option<Period>,
    },
    Date,
    Choice {
        options: Vec<ChoiceOption>,
    },
    Totp,
    Formula {
        expr: String,
    },
    Url,
    Email,
    Phone,
    Checkbox,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChoiceOption {
    pub id: String,
    pub label: String,
    /// Functional colour from the design charter: `ok`, `warn`, `danger`, `set`, `auto` or none.
    pub tone: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "scope", content = "space", rename_all = "snake_case")]
pub enum ColumnScope {
    /// Shown in every space of the vault (or of the contact book).
    AllSpaces,
    Space(SpaceId),
}

/// What a column belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Table {
    Vault,
    Contacts,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColumnDef {
    pub id: ColumnId,
    pub table: Table,
    pub name: String,
    pub kind: ColumnKind,
    pub scope: ColumnScope,
    /// Built-in columns can be hidden but not deleted.
    pub builtin: bool,
    pub visible: bool,
    pub width: Option<u16>,
    pub order: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", content = "v", rename_all = "snake_case")]
pub enum Value {
    Text(String),
    Number(f64),
    Money(Money),
    /// ISO date `YYYY-MM-DD`.
    Date(String),
    Choice(String),
    Bool(bool),
}

impl Value {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(s) | Value::Date(s) | Value::Choice(s) => Some(s),
            _ => None,
        }
    }
}

fn col(
    table: Table,
    id: &str,
    name: &str,
    kind: ColumnKind,
    visible: bool,
    order: i32,
) -> ColumnDef {
    ColumnDef {
        id: ColumnId::builtin(id),
        table,
        name: name.to_owned(),
        kind,
        scope: ColumnScope::AllSpaces,
        builtin: true,
        visible,
        width: None,
        order,
    }
}

pub fn subscription_options() -> Vec<ChoiceOption> {
    let o = |id: &str, label: &str, tone: Option<&str>| ChoiceOption {
        id: id.to_owned(),
        label: label.to_owned(),
        tone: tone.map(str::to_owned),
    };
    vec![
        o("active", "Active", Some("ok")),
        o("free", "Free", None),
        o("renew_soon", "Renew soon", Some("warn")),
        o("pending", "Pending", None),
        o("off", "Off", None),
    ]
}

/// Default vault columns, as in the mockups (02-colonnes).
pub fn default_vault_columns(currency: &str) -> Vec<ColumnDef> {
    let t = Table::Vault;
    vec![
        col(t, "name", "Name", ColumnKind::Text, true, 0),
        col(t, "username", "Username", ColumnKind::Text, true, 1),
        col(t, "password", "Password", ColumnKind::Secret, true, 2),
        col(t, "totp", "2FA code", ColumnKind::Totp, true, 3),
        col(
            t,
            "subscription",
            "Subscription",
            ColumnKind::Choice {
                options: subscription_options(),
            },
            true,
            4,
        ),
        col(
            t,
            "price",
            "Price",
            ColumnKind::Price {
                currency: currency.to_owned(),
                default_period: Some(Period::Month),
            },
            true,
            5,
        ),
        col(
            t,
            "licenses",
            "Licenses",
            ColumnKind::Number { decimals: 0 },
            false,
            6,
        ),
        col(
            t,
            "yearly_cost",
            "Yearly cost",
            ColumnKind::Formula {
                expr: "yearly([Price]) * default([Licenses], 1)".into(),
            },
            true,
            7,
        ),
        col(t, "website", "Website", ColumnKind::Url, false, 8),
        col(t, "renewal", "Renewal", ColumnKind::Date, false, 9),
        col(t, "notes", "Notes", ColumnKind::Text, false, 10),
        col(t, "space", "Space", ColumnKind::Text, false, 11),
    ]
}

/// Default contact columns, as in the mockups (20-contacts).
pub fn default_contact_columns() -> Vec<ColumnDef> {
    let t = Table::Contacts;
    vec![
        col(t, "name", "Name", ColumnKind::Text, true, 0),
        col(t, "company", "Company", ColumnKind::Text, true, 1),
        col(t, "email", "Email", ColumnKind::Email, true, 2),
        col(t, "phone", "Phone", ColumnKind::Phone, true, 3),
        col(t, "labels", "Labels", ColumnKind::Text, true, 4),
        col(t, "job_title", "Job title", ColumnKind::Text, false, 5),
        col(t, "city", "City", ColumnKind::Text, false, 6),
        col(t, "birthday", "Birthday", ColumnKind::Date, false, 7),
        col(t, "website", "Website", ColumnKind::Url, false, 8),
        col(t, "notes", "Notes", ColumnKind::Text, false, 9),
        col(t, "folder", "Folder", ColumnKind::Text, false, 10),
    ]
}

/// Visible columns of one table for one space, in display order.
pub fn visible_columns(
    cols: &[ColumnDef],
    table: Table,
    space: Option<SpaceId>,
) -> Vec<&ColumnDef> {
    let mut v: Vec<&ColumnDef> = cols
        .iter()
        .filter(|c| c.table == table && c.visible)
        .filter(|c| match (&c.scope, space) {
            (ColumnScope::AllSpaces, _) => true,
            (ColumnScope::Space(s), Some(cur)) => *s == cur,
            (ColumnScope::Space(_), None) => true,
        })
        .collect();
    v.sort_by_key(|c| c.order);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_filtering() {
        let mut cols = default_vault_columns("EUR");
        let space = SpaceId::new();
        cols.push(ColumnDef {
            id: ColumnId::custom(),
            table: Table::Vault,
            name: "Cancellation date".into(),
            kind: ColumnKind::Date,
            scope: ColumnScope::Space(space),
            builtin: false,
            visible: true,
            width: None,
            order: 20,
        });
        let other = SpaceId::new();
        let names = |s| {
            visible_columns(&cols, Table::Vault, Some(s))
                .iter()
                .map(|c| c.name.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(space).last().unwrap(), "Cancellation date");
        assert!(!names(other).contains(&"Cancellation date".to_string()));
        assert!(!names(other).contains(&"Licenses".to_string()));
        assert_eq!(
            visible_columns(&default_contact_columns(), Table::Contacts, None).len(),
            5
        );
    }
}
