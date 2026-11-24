use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            /// Time-ordered id (UUIDv7): sorts by creation, merges well across devices.
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

id_type!(EntryId);
id_type!(ContactId);
id_type!(SpaceId);
id_type!(MemberId);

/// Columns have stable string ids: built-in ones are readable (`"username"`),
/// custom ones are UUIDs.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ColumnId(pub String);

impl ColumnId {
    pub fn custom() -> Self {
        Self(Uuid::now_v7().to_string())
    }
    pub fn builtin(name: &str) -> Self {
        Self(name.to_owned())
    }
}

/// Unix time in seconds.
pub type Timestamp = i64;

pub fn now() -> Timestamp {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
