//! Cleis domain model. Pure logic, no I/O: everything here is unit-tested and shared by
//! the desktop app, the server and the command-line tools.

#![forbid(unsafe_code)]

pub mod autofill;
pub mod column;
pub mod contact;
pub mod dedupe;
pub mod entry;
pub mod formula;
pub mod generator;
pub mod health;
pub mod ids;
pub mod import_csv;
pub mod journal;
pub mod money;
pub mod settings;
pub mod text;
pub mod totp;
pub mod vcard;

pub use column::{ColumnDef, ColumnKind, ColumnScope, Value};
pub use contact::{Address, Contact, Labeled, PartialDate};
pub use entry::{Entry, LinkedApp, Subscription};
pub use ids::{ColumnId, ContactId, EntryId, MemberId, SpaceId, now};
pub use money::{Money, Period};
