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
        // D-102: unused codes left / codes kept; shown from Columns (the table keeps its room).
        col(t, "recovery", "Recovery keys", ColumnKind::Text, false, 12),
    ]
}

/// Built-in columns added since a vault saved its own (D-102: Recovery keys), at the end.
pub fn with_new_builtins(mut saved: Vec<ColumnDef>, defaults: Vec<ColumnDef>) -> Vec<ColumnDef> {
    let mut next = saved.iter().map(|c| c.order).max().map_or(0, |o| o + 1);
    for d in defaults {
        if d.builtin && !saved.iter().any(|c| c.id == d.id && c.table == d.table) {
            let mut d = d;
            d.order = next;
            next += 1;
            saved.push(d);
        }
    }
    saved
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

/// Kinds offered when creating a column (screen 02), in menu order.
pub const NEW_KINDS: [(&str, &str); 9] = [
    ("text", "Text"),
    ("secret", "Secret text"),
    ("number", "Number"),
    ("price", "Price"),
    ("date", "Date"),
    ("choice", "Choice list"),
    ("totp", "2FA code"),
    ("formula", "Formula"),
    ("url", "Web address"),
];

/// Builds the kind chosen in the "New column" form. `extra` holds the formula, or the choice
/// options separated by commas.
pub fn kind_from(key: &str, extra: &str, currency: &str) -> Result<ColumnKind, ColumnError> {
    Ok(match key {
        "text" => ColumnKind::Text,
        "secret" => ColumnKind::Secret,
        "number" => ColumnKind::Number { decimals: 2 },
        "price" => ColumnKind::Price {
            currency: currency.to_owned(),
            default_period: None,
        },
        "date" => ColumnKind::Date,
        "choice" => {
            let options: Vec<ChoiceOption> = extra
                .split(',')
                .map(str::trim)
                .filter(|o| !o.is_empty())
                .map(|o| ChoiceOption {
                    id: crate::text::fold(o),
                    label: o.to_owned(),
                    tone: None,
                })
                .collect();
            if options.is_empty() {
                return Err(ColumnError::NoOptions);
            }
            ColumnKind::Choice { options }
        }
        "totp" => ColumnKind::Totp,
        "formula" => {
            if extra.trim().is_empty() {
                return Err(ColumnError::NoFormula);
            }
            ColumnKind::Formula {
                expr: extra.trim().to_owned(),
            }
        }
        "url" => ColumnKind::Url,
        _ => ColumnKind::Text,
    })
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ColumnError {
    #[error("A name is required")]
    NoName,
    #[error("A column with this name exists")]
    Taken,
    #[error("Give at least one option")]
    NoOptions,
    #[error("Write the formula, e.g. [Price] * 12")]
    NoFormula,
    #[error("Built-in columns can be hidden, not deleted")]
    Builtin,
}

/// Adds a custom column at the end of its table's shown columns.
pub fn add_custom(
    cols: &mut Vec<ColumnDef>,
    table: Table,
    name: &str,
    kind: ColumnKind,
    scope: ColumnScope,
) -> Result<ColumnId, ColumnError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ColumnError::NoName);
    }
    let folded = crate::text::fold(name);
    if cols
        .iter()
        .any(|c| c.table == table && crate::text::fold(&c.name) == folded)
    {
        return Err(ColumnError::Taken);
    }
    let order = cols
        .iter()
        .filter(|c| c.table == table)
        .map(|c| c.order)
        .max()
        .unwrap_or(0)
        + 1;
    let id = ColumnId::custom();
    cols.push(ColumnDef {
        id: id.clone(),
        table,
        name: name.to_owned(),
        kind,
        scope,
        builtin: false,
        visible: true,
        width: None,
        order,
    });
    Ok(id)
}

/// Deletes a custom column (its values stay in the items until they are next edited).
pub fn remove_custom(cols: &mut Vec<ColumnDef>, id: &ColumnId) -> Result<(), ColumnError> {
    match cols.iter().position(|c| &c.id == id) {
        Some(i) if cols[i].builtin => Err(ColumnError::Builtin),
        Some(i) => {
            cols.remove(i);
            Ok(())
        }
        None => Ok(()),
    }
}

/// Moves `id` to position `to` among its table's columns and renumbers them.
pub fn reorder(cols: &mut [ColumnDef], id: &ColumnId, to: usize) {
    let Some(table) = cols.iter().find(|c| &c.id == id).map(|c| c.table) else {
        return;
    };
    let mut ids: Vec<(i32, ColumnId)> = cols
        .iter()
        .filter(|c| c.table == table)
        .map(|c| (c.order, c.id.clone()))
        .collect();
    ids.sort();
    let mut ids: Vec<ColumnId> = ids.into_iter().map(|(_, i)| i).collect();
    let Some(from) = ids.iter().position(|i| i == id) else {
        return;
    };
    let moved = ids.remove(from);
    ids.insert(to.min(ids.len()), moved);
    for c in cols.iter_mut().filter(|c| c.table == table) {
        if let Some(p) = ids.iter().position(|i| i == &c.id) {
            c.order = p as i32;
        }
    }
}

/// A value that does not fit its column's type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BadValue;

impl Value {
    /// Reads a typed value for a custom column; empty text clears it.
    pub fn parse(kind: &ColumnKind, text: &str) -> Result<Option<Value>, BadValue> {
        let v = text.trim();
        if v.is_empty() {
            return Ok(None);
        }
        Ok(Some(match kind {
            ColumnKind::Text
            | ColumnKind::Secret
            | ColumnKind::Url
            | ColumnKind::Email
            | ColumnKind::Phone
            | ColumnKind::Totp => Value::Text(v.to_owned()),
            ColumnKind::Number { .. } => Value::Number(
                v.replace(',', ".")
                    .replace([' ', '\u{a0}', '\u{202f}'], "")
                    .parse::<f64>()
                    .map_err(|_| BadValue)?,
            ),
            ColumnKind::Price {
                currency,
                default_period,
            } => Value::Money(Money::parse(v, currency, *default_period).ok_or(BadValue)?),
            ColumnKind::Date => Value::Date(crate::import_csv::parse_date(v).ok_or(BadValue)?),
            ColumnKind::Choice { options } => {
                let f = crate::text::fold(v);
                let o = options
                    .iter()
                    .find(|o| crate::text::fold(&o.label) == f || o.id == f)
                    .ok_or(BadValue)?;
                Value::Choice(o.id.clone())
            }
            ColumnKind::Checkbox => Value::Bool(matches!(
                crate::text::fold(v).as_str(),
                "yes" | "oui" | "true" | "vrai" | "1" | "x"
            )),
            ColumnKind::Formula { .. } => return Err(BadValue),
        }))
    }

    /// Text shown in a cell or a box.
    pub fn display(&self, kind: &ColumnKind) -> String {
        match (self, kind) {
            (Value::Choice(id), ColumnKind::Choice { options }) => options
                .iter()
                .find(|o| &o.id == id)
                .map(|o| o.label.clone())
                .unwrap_or_else(|| id.clone()),
            (Value::Text(s) | Value::Date(s) | Value::Choice(s), _) => s.clone(),
            (Value::Number(n), ColumnKind::Number { decimals }) => {
                format!("{n:.*}", usize::from(*decimals))
            }
            (Value::Number(n), _) => n.to_string(),
            (Value::Money(m), _) => crate::edit::price_text(m),
            (Value::Bool(b), _) => if *b { "Yes" } else { "No" }.into(),
        }
    }
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
    fn custom_columns_add_reorder_remove() {
        let mut cols = default_vault_columns("EUR");
        let id = add_custom(
            &mut cols,
            Table::Vault,
            "Cancellation date",
            kind_from("date", "", "EUR").unwrap(),
            ColumnScope::AllSpaces,
        )
        .unwrap();
        assert_eq!(
            add_custom(
                &mut cols,
                Table::Vault,
                "cancellation DATE",
                ColumnKind::Text,
                ColumnScope::AllSpaces
            ),
            Err(ColumnError::Taken)
        );
        assert_eq!(
            add_custom(
                &mut cols,
                Table::Vault,
                " ",
                ColumnKind::Text,
                ColumnScope::AllSpaces
            ),
            Err(ColumnError::NoName)
        );
        reorder(&mut cols, &id, 1);
        let v = visible_columns(&cols, Table::Vault, None);
        assert_eq!(v[1].name, "Cancellation date");
        assert_eq!(v[0].name, "Name");
        let name = cols.iter().find(|c| c.id.0 == "name").unwrap().id.clone();
        assert_eq!(remove_custom(&mut cols, &name), Err(ColumnError::Builtin));
        remove_custom(&mut cols, &id).unwrap();
        assert!(cols.iter().all(|c| c.id != id));
        assert_eq!(
            kind_from("choice", " , ", "EUR"),
            Err(ColumnError::NoOptions)
        );
        assert_eq!(kind_from("formula", "", "EUR"), Err(ColumnError::NoFormula));
    }

    #[test]
    fn values_parse_and_display() {
        let choice = kind_from("choice", "Monthly, Yearly", "EUR").unwrap();
        let v = Value::parse(&choice, "yearly").unwrap().unwrap();
        assert_eq!(v.display(&choice), "Yearly");
        assert!(Value::parse(&choice, "weekly").is_err());
        let n = ColumnKind::Number { decimals: 2 };
        assert_eq!(
            Value::parse(&n, "1 234,5").unwrap().unwrap().display(&n),
            "1234.50"
        );
        assert_eq!(
            Value::parse(&ColumnKind::Date, "3 Dec 2026").unwrap(),
            Some(Value::Date("2026-12-03".into()))
        );
        assert_eq!(Value::parse(&ColumnKind::Text, "  "), Ok(None));
    }

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
