//! Local vault file: a single SQLite database where every sensitive value is sealed
//! (AES-256-GCM) before it is written. Works fully offline; the server and the shared
//! folder modes reuse the same sealed records.
//!
//! Key hierarchy:
//! - master password → Argon2id → `MasterKey.encryption`
//! - which seals the account secrets (X25519 + Ed25519 private keys)
//! - each space key is wrapped (X25519) to the account's public key
//! - each record is sealed with its space key, AAD = `kind:id:space`.
//!
//! What stays in clear in the file: ids, space of each record, revision numbers and
//! timestamps (needed to sync). Names, usernames, passwords, notes, contacts, columns and
//! preferences are all sealed.

#![forbid(unsafe_code)]

use cleis_core::column::{ColumnDef, default_contact_columns, default_vault_columns};
use cleis_core::settings::Preferences;
use cleis_core::{Contact, ContactId, Entry, EntryId, SpaceId};
use cleis_crypto::{
    CryptoError, KdfParams, KeyPair, SigningKeyPair, SymmetricKey, derive_master_key, open,
    random_bytes, seal, unwrap_key, wrap_key,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::BTreeMap;
use std::path::Path;
use thiserror::Error;
use zeroize::Zeroizing;

const FORMAT: i64 = 1;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("wrong master password")]
    WrongPassword,
    #[error("this file is not a Cleis vault")]
    NotAVault,
    #[error("this vault was created by a newer version of Cleis")]
    TooNew,
    #[error("a file already exists there")]
    Exists,
    #[error("file error: {0}")]
    Io(String),
    #[error("unknown space")]
    UnknownSpace,
    #[error("the vault is damaged: {0}")]
    Corrupt(&'static str),
    #[error(transparent)]
    Crypto(#[from] CryptoError),
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpaceKind {
    Personal,
    Organization,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EncryptionMode {
    /// The organisation server keeps a recovery copy of the space key.
    Standard,
    /// Only members hold the key.
    EndToEnd,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceInfo {
    pub id: SpaceId,
    pub name: String,
    pub kind: SpaceKind,
    pub mode: EncryptionMode,
    /// Contacts folder rather than a vault space.
    pub for_contacts: bool,
}

#[derive(Serialize, Deserialize, zeroize::Zeroize, zeroize::ZeroizeOnDrop)]
struct AccountSecrets {
    x25519: [u8; 32],
    ed25519: [u8; 32],
}

/// A vault file that has not been unlocked yet.
pub struct LockedVault {
    conn: Connection,
}

/// An unlocked vault. Keys live only in this value and are wiped when it is dropped.
pub struct Vault {
    conn: Connection,
    account: KeyPair,
    signing: SigningKeyPair,
    space_keys: BTreeMap<SpaceId, SymmetricKey>,
    /// Seals vault-level data (columns, preferences): the personal space key.
    home: SpaceId,
}

fn configure(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "FULL")?;
    // Freed pages are overwritten, so deleted records do not linger in the file.
    conn.pragma_update(None, "secure_delete", "ON")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(())
}

const SCHEMA: &str = "
CREATE TABLE meta (key TEXT PRIMARY KEY, value BLOB NOT NULL) STRICT;
CREATE TABLE spaces (
    id TEXT PRIMARY KEY,
    wrapped_key BLOB NOT NULL,
    sealed BLOB NOT NULL,
    rev INTEGER NOT NULL DEFAULT 1
) STRICT;
CREATE TABLE records (
    kind TEXT NOT NULL CHECK (kind IN ('entry','contact')),
    id TEXT NOT NULL,
    space TEXT NOT NULL REFERENCES spaces(id),
    sealed BLOB NOT NULL,
    rev INTEGER NOT NULL,
    updated INTEGER NOT NULL,
    deleted INTEGER,
    PRIMARY KEY (kind, id)
) STRICT;
CREATE INDEX records_space ON records(space, kind);
CREATE TABLE settings (key TEXT PRIMARY KEY, sealed BLOB NOT NULL) STRICT;
";

fn meta_get(conn: &Connection, key: &str) -> Result<Option<Vec<u8>>> {
    Ok(conn
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
        .optional()?)
}

fn meta_set(conn: &Connection, key: &str, value: &[u8]) -> Result<()> {
    conn.execute("INSERT INTO meta(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value", params![key, value])?;
    Ok(())
}

fn aad(kind: &str, id: &str, space: &SpaceId) -> Vec<u8> {
    format!("cleis/{kind}/{id}/{space}").into_bytes()
}

impl Vault {
    /// Creates a new vault file with one personal space for passwords and one for contacts.
    ///
    /// The file is built under a temporary name next to `path` and renamed once complete, so
    /// an interruption (crash, power loss, full disk) never leaves a half-made vault behind that
    /// the next launch would take for a real one. An existing file at `path` is never replaced.
    pub fn create(path: &Path, password: &str, kdf: KdfParams) -> Result<Self> {
        if path.exists() {
            return Err(StoreError::Exists);
        }
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(format!(".creating-{}", std::process::id()));
        let tmp = std::path::PathBuf::from(tmp);
        let cleanup = |p: &Path| {
            for suffix in ["", "-wal", "-shm", "-journal"] {
                let mut f = p.as_os_str().to_owned();
                f.push(suffix);
                let _ = std::fs::remove_file(std::path::PathBuf::from(f));
            }
        };
        cleanup(&tmp);
        let built = Self::build(&tmp, password, kdf);
        let mut v = match built {
            Ok(v) => v,
            Err(e) => {
                cleanup(&tmp);
                return Err(e);
            }
        };
        // Close (checkpoints the WAL into the main file), rename, reopen with the same keys.
        let conn = std::mem::replace(&mut v.conn, Connection::open_in_memory()?);
        if let Err((_, e)) = conn.close() {
            cleanup(&tmp);
            return Err(e.into());
        }
        if let Err(e) = std::fs::rename(&tmp, path) {
            cleanup(&tmp);
            return Err(StoreError::Io(e.to_string()));
        }
        cleanup(&tmp);
        let conn = Connection::open(path)?;
        configure(&conn)?;
        v.conn = conn;
        Ok(v)
    }

    fn build(path: &Path, password: &str, kdf: KdfParams) -> Result<Self> {
        let conn = Connection::open(path)?;
        configure(&conn)?;
        conn.execute_batch(SCHEMA)?;
        let salt: [u8; 16] = random_bytes()?;
        meta_set(&conn, "format", &FORMAT.to_be_bytes())?;
        meta_set(&conn, "kdf", &serde_json::to_vec(&kdf)?)?;
        meta_set(&conn, "salt", &salt)?;
        let account = KeyPair::generate()?;
        let signing = SigningKeyPair::generate()?;
        let mut v = Vault {
            conn,
            account,
            signing,
            space_keys: BTreeMap::new(),
            home: SpaceId::new(),
        };
        v.seal_account(password, &salt, kdf)?;
        let home = v.create_space(
            "My vault",
            SpaceKind::Personal,
            EncryptionMode::EndToEnd,
            false,
        )?;
        v.home = home;
        meta_set(&v.conn, "home", home.0.as_bytes())?;
        v.create_space(
            "My contacts",
            SpaceKind::Personal,
            EncryptionMode::EndToEnd,
            true,
        )?;
        v.set_setting("vault_columns", &default_vault_columns("EUR"))?;
        v.set_setting("contact_columns", &default_contact_columns())?;
        v.set_setting("preferences", &Preferences::default())?;
        Ok(v)
    }

    fn seal_account(&self, password: &str, salt: &[u8; 16], kdf: KdfParams) -> Result<()> {
        let mk = derive_master_key(password, salt, kdf)?;
        let secrets = AccountSecrets {
            x25519: *self.account.secret_bytes(),
            ed25519: *self.signing.secret_bytes(),
        };
        let plain = Zeroizing::new(serde_json::to_vec(&secrets)?);
        drop(secrets);
        let sealed = seal(&mk.encryption, b"cleis/account", &plain)?;
        meta_set(&self.conn, "account", &sealed)?;
        meta_set(&self.conn, "account_public", &self.account.public().0)?;
        Ok(())
    }

    /// Re-seals the account secrets under a new master password. Records are untouched.
    pub fn change_password(&mut self, new_password: &str, kdf: KdfParams) -> Result<()> {
        let salt: [u8; 16] = random_bytes()?;
        let tx = self.conn.unchecked_transaction()?;
        meta_set(&self.conn, "kdf", &serde_json::to_vec(&kdf)?)?;
        meta_set(&self.conn, "salt", &salt)?;
        self.seal_account(new_password, &salt, kdf)?;
        tx.commit()?;
        Ok(())
    }

    pub fn create_space(
        &mut self,
        name: &str,
        kind: SpaceKind,
        mode: EncryptionMode,
        for_contacts: bool,
    ) -> Result<SpaceId> {
        let id = SpaceId::new();
        let key = SymmetricKey::generate()?;
        let info = SpaceInfo {
            id,
            name: name.to_owned(),
            kind,
            mode,
            for_contacts,
        };
        let wrapped = wrap_key(&self.account.public(), &key, id.0.as_bytes())?;
        let sealed = seal(
            &key,
            &aad("space", &id.to_string(), &id),
            &serde_json::to_vec(&info)?,
        )?;
        self.conn.execute(
            "INSERT INTO spaces(id, wrapped_key, sealed) VALUES (?1, ?2, ?3)",
            params![id.to_string(), wrapped, sealed],
        )?;
        self.space_keys.insert(id, key);
        Ok(id)
    }

    pub fn rename_space(&mut self, id: SpaceId, name: &str) -> Result<()> {
        let mut info = self
            .spaces()?
            .into_iter()
            .find(|s| s.id == id)
            .ok_or(StoreError::UnknownSpace)?;
        info.name = name.to_owned();
        let key = self.key(id)?;
        let sealed = seal(
            key,
            &aad("space", &id.to_string(), &id),
            &serde_json::to_vec(&info)?,
        )?;
        self.conn.execute(
            "UPDATE spaces SET sealed = ?2, rev = rev + 1 WHERE id = ?1",
            params![id.to_string(), sealed],
        )?;
        Ok(())
    }

    pub fn spaces(&self) -> Result<Vec<SpaceInfo>> {
        let mut st = self
            .conn
            .prepare("SELECT id, sealed FROM spaces ORDER BY rowid")?;
        let rows = st.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?))
        })?;
        let mut out = vec![];
        for row in rows {
            let (id, sealed) = row?;
            let sid = SpaceId(id.parse().map_err(|_| StoreError::Corrupt("space id"))?);
            let plain = open(self.key(sid)?, &aad("space", &id, &sid), &sealed)?;
            out.push(serde_json::from_slice(&plain)?);
        }
        Ok(out)
    }

    fn key(&self, space: SpaceId) -> Result<&SymmetricKey> {
        self.space_keys.get(&space).ok_or(StoreError::UnknownSpace)
    }

    /// Ed25519 identity of this account (activity log, signed writes).
    pub fn signing_key(&self) -> &SigningKeyPair {
        &self.signing
    }

    pub fn account_public(&self) -> cleis_crypto::PublicKey {
        self.account.public()
    }

    fn put<T: Serialize>(
        &self,
        kind: &str,
        id: &str,
        space: SpaceId,
        updated: i64,
        value: &T,
    ) -> Result<()> {
        let plain = Zeroizing::new(serde_json::to_vec(value)?);
        let sealed = seal(self.key(space)?, &aad(kind, id, &space), &plain)?;
        self.conn.execute(
            "INSERT INTO records(kind, id, space, sealed, rev, updated, deleted) VALUES (?1, ?2, ?3, ?4, 1, ?5, NULL)
             ON CONFLICT(kind, id) DO UPDATE SET space = excluded.space, sealed = excluded.sealed,
               rev = records.rev + 1, updated = excluded.updated, deleted = NULL",
            params![kind, id, space.to_string(), sealed, updated],
        )?;
        Ok(())
    }

    fn load_all<T: DeserializeOwned>(&self, kind: &str, deleted: bool) -> Result<Vec<T>> {
        let sql = if deleted {
            "SELECT id, space, sealed FROM records WHERE kind = ?1 AND deleted IS NOT NULL ORDER BY id"
        } else {
            "SELECT id, space, sealed FROM records WHERE kind = ?1 AND deleted IS NULL ORDER BY id"
        };
        let mut st = self.conn.prepare(sql)?;
        let rows = st.query_map([kind], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Vec<u8>>(2)?,
            ))
        })?;
        let mut out = vec![];
        for row in rows {
            let (id, space, sealed) = row?;
            let sid = SpaceId(
                space
                    .parse()
                    .map_err(|_| StoreError::Corrupt("record space"))?,
            );
            let Ok(key) = self.key(sid) else { continue }; // space not shared with us
            let plain = Zeroizing::new(open(key, &aad(kind, &id, &sid), &sealed)?);
            out.push(serde_json::from_slice(&plain)?);
        }
        Ok(out)
    }

    fn soft_delete(&self, kind: &str, id: &str, at: i64) -> Result<bool> {
        Ok(self.conn.execute("UPDATE records SET deleted = ?3, rev = rev + 1 WHERE kind = ?1 AND id = ?2 AND deleted IS NULL", params![kind, id, at])? == 1)
    }

    fn restore(&self, kind: &str, id: &str) -> Result<bool> {
        Ok(self.conn.execute("UPDATE records SET deleted = NULL, rev = rev + 1 WHERE kind = ?1 AND id = ?2 AND deleted IS NOT NULL", params![kind, id])? == 1)
    }

    /// Erases a record for good (emptying the trash).
    fn purge(&self, kind: &str, id: &str) -> Result<bool> {
        Ok(self.conn.execute(
            "DELETE FROM records WHERE kind = ?1 AND id = ?2",
            params![kind, id],
        )? == 1)
    }

    pub fn put_entry(&self, e: &Entry) -> Result<()> {
        self.put("entry", &e.id.to_string(), e.space, e.updated, e)
    }
    pub fn entries(&self) -> Result<Vec<Entry>> {
        self.load_all("entry", false)
    }
    pub fn trashed_entries(&self) -> Result<Vec<Entry>> {
        self.load_all("entry", true)
    }
    pub fn delete_entry(&self, id: EntryId) -> Result<bool> {
        self.soft_delete("entry", &id.to_string(), cleis_core::now())
    }
    pub fn restore_entry(&self, id: EntryId) -> Result<bool> {
        self.restore("entry", &id.to_string())
    }
    pub fn purge_entry(&self, id: EntryId) -> Result<bool> {
        self.purge("entry", &id.to_string())
    }

    pub fn put_contact(&self, c: &Contact) -> Result<()> {
        self.put("contact", &c.id.to_string(), c.space, c.updated, c)
    }
    pub fn contacts(&self) -> Result<Vec<Contact>> {
        self.load_all("contact", false)
    }
    pub fn delete_contact(&self, id: ContactId) -> Result<bool> {
        self.soft_delete("contact", &id.to_string(), cleis_core::now())
    }
    pub fn restore_contact(&self, id: ContactId) -> Result<bool> {
        self.restore("contact", &id.to_string())
    }

    /// Writes many records in one transaction (imports).
    pub fn put_entries(&mut self, es: &[Entry]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for e in es {
            self.put_entry(e)?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn put_contacts(&mut self, cs: &[Contact]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for c in cs {
            self.put_contact(c)?;
        }
        tx.commit()?;
        Ok(())
    }

    fn set_setting<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let sealed = seal(
            self.key(self.home)?,
            &aad("setting", key, &self.home),
            &serde_json::to_vec(value)?,
        )?;
        self.conn.execute("INSERT INTO settings(key, sealed) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET sealed = excluded.sealed", params![key, sealed])?;
        Ok(())
    }

    fn setting<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let Some(sealed): Option<Vec<u8>> = self
            .conn
            .query_row("SELECT sealed FROM settings WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .optional()?
        else {
            return Ok(None);
        };
        let plain = open(
            self.key(self.home)?,
            &aad("setting", key, &self.home),
            &sealed,
        )?;
        Ok(Some(serde_json::from_slice(&plain)?))
    }

    pub fn vault_columns(&self) -> Result<Vec<ColumnDef>> {
        Ok(self
            .setting("vault_columns")?
            .unwrap_or_else(|| default_vault_columns("EUR")))
    }
    pub fn set_vault_columns(&self, c: &[ColumnDef]) -> Result<()> {
        self.set_setting("vault_columns", &c)
    }
    pub fn contact_columns(&self) -> Result<Vec<ColumnDef>> {
        Ok(self
            .setting("contact_columns")?
            .unwrap_or_else(default_contact_columns))
    }
    pub fn set_contact_columns(&self, c: &[ColumnDef]) -> Result<()> {
        self.set_setting("contact_columns", &c)
    }
    pub fn preferences(&self) -> Result<Preferences> {
        Ok(self.setting("preferences")?.unwrap_or_default())
    }
    pub fn set_preferences(&self, p: &Preferences) -> Result<()> {
        self.set_setting("preferences", p)
    }

    /// Personal space where new entries go by default.
    pub fn home_space(&self) -> SpaceId {
        self.home
    }

    /// Drops every key; the file stays as it is.
    pub fn lock(self) -> LockedVault {
        LockedVault { conn: self.conn }
    }
}

impl LockedVault {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        let format = meta_get(&conn, "format")
            .map_err(|_| StoreError::NotAVault)?
            .ok_or(StoreError::NotAVault)?;
        let format = i64::from_be_bytes(format.try_into().map_err(|_| StoreError::NotAVault)?);
        if format > FORMAT {
            return Err(StoreError::TooNew);
        }
        configure(&conn)?;
        Ok(Self { conn })
    }

    pub fn unlock(self, password: &str) -> Result<Vault> {
        let kdf: KdfParams = serde_json::from_slice(
            &meta_get(&self.conn, "kdf")?.ok_or(StoreError::Corrupt("kdf"))?,
        )?;
        let salt: [u8; 16] = meta_get(&self.conn, "salt")?
            .ok_or(StoreError::Corrupt("salt"))?
            .try_into()
            .map_err(|_| StoreError::Corrupt("salt"))?;
        let sealed = meta_get(&self.conn, "account")?.ok_or(StoreError::Corrupt("account"))?;
        let mk = derive_master_key(password, &salt, kdf)?;
        let plain = Zeroizing::new(
            open(&mk.encryption, b"cleis/account", &sealed)
                .map_err(|_| StoreError::WrongPassword)?,
        );
        let secrets: AccountSecrets = serde_json::from_slice(&plain)?;
        let account = KeyPair::from_secret_bytes(secrets.x25519);
        let signing = SigningKeyPair::from_secret_bytes(secrets.ed25519);
        let home_bytes = meta_get(&self.conn, "home")?.ok_or(StoreError::Corrupt("home"))?;
        let home =
            SpaceId(uuid::Uuid::from_slice(&home_bytes).map_err(|_| StoreError::Corrupt("home"))?);
        let mut space_keys = BTreeMap::new();
        {
            let mut st = self.conn.prepare("SELECT id, wrapped_key FROM spaces")?;
            let rows = st.query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?))
            })?;
            for row in rows {
                let (id, wrapped) = row?;
                let sid = SpaceId(id.parse().map_err(|_| StoreError::Corrupt("space id"))?);
                if let Ok(k) = unwrap_key(&account, &wrapped, sid.0.as_bytes()) {
                    space_keys.insert(sid, k);
                }
            }
        }
        Ok(Vault {
            conn: self.conn,
            account,
            signing,
            space_keys,
            home,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cleis_core::contact::Labeled;
    use cleis_core::totp::Totp;

    fn fresh() -> (tempfile::TempDir, std::path::PathBuf, Vault) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.cleis");
        let v = Vault::create(
            &path,
            "correct horse battery",
            KdfParams::insecure_for_tests(),
        )
        .unwrap();
        (dir, path, v)
    }

    #[test]
    fn create_is_atomic_and_never_overwrites() {
        let (dir, path, v) = fresh();
        drop(v);
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n.contains("creating"))
            .collect();
        assert!(names.is_empty(), "temporary files left: {names:?}");
        assert!(matches!(
            Vault::create(&path, "another password", KdfParams::insecure_for_tests()),
            Err(StoreError::Exists)
        ));
        // The original is intact and still opens with its own password.
        LockedVault::open(&path)
            .unwrap()
            .unlock("correct horse battery")
            .unwrap();
    }

    #[test]
    fn create_lock_unlock_round_trip() {
        let (_d, path, v) = fresh();
        let mut e = Entry::new(v.home_space(), "Figma");
        e.username = "design@example-studio.com".into();
        e.password = "s3cret-Figma!".into();
        e.totp = Some(Totp::from_base32("JBSWY3DPEHPK3PXP").unwrap());
        v.put_entry(&e).unwrap();
        let contacts_space = v
            .spaces()
            .unwrap()
            .into_iter()
            .find(|s| s.for_contacts)
            .unwrap()
            .id;
        let mut c = Contact::person(contacts_space, "Camille", "Dubois");
        c.emails
            .push(Labeled::new("Work", "camille@example-print.com".into()));
        v.put_contact(&c).unwrap();
        drop(v.lock());

        assert!(matches!(
            LockedVault::open(&path).unwrap().unlock("wrong"),
            Err(StoreError::WrongPassword)
        ));
        let v = LockedVault::open(&path)
            .unwrap()
            .unlock("correct horse battery")
            .unwrap();
        let es = v.entries().unwrap();
        assert_eq!(es.len(), 1);
        assert_eq!(es[0], e);
        assert_eq!(
            v.contacts().unwrap()[0].primary_email().as_deref(),
            Some("camille@example-print.com")
        );
        assert_eq!(v.spaces().unwrap().len(), 2);
        assert_eq!(v.vault_columns().unwrap().len(), 12);
        assert!(v.preferences().unwrap().tabs.contacts);
    }

    #[test]
    fn nothing_readable_in_the_file() {
        let (_d, path, v) = fresh();
        let mut e = Entry::new(v.home_space(), "VerySecretServiceName");
        e.password = "PlainPasswordMarker".into();
        v.put_entry(&e).unwrap();
        drop(v.lock());
        // Checkpoint the WAL into the main file, then scan every byte on disk.
        let c = Connection::open(&path).unwrap();
        c.pragma_update(None, "wal_checkpoint", "TRUNCATE").unwrap();
        drop(c);
        let mut all = std::fs::read(&path).unwrap();
        for ext in ["-wal", "-shm"] {
            if let Ok(b) = std::fs::read(format!("{}{ext}", path.display())) {
                all.extend(b);
            }
        }
        let hay = String::from_utf8_lossy(&all);
        assert!(!hay.contains("PlainPasswordMarker"));
        assert!(!hay.contains("VerySecretServiceName"));
    }

    #[test]
    fn trash_restore_purge_and_updates() {
        let (_d, _p, v) = fresh();
        let mut e = Entry::new(v.home_space(), "Slack");
        v.put_entry(&e).unwrap();
        e.set_password("new".into(), None);
        v.put_entry(&e).unwrap();
        assert_eq!(v.entries().unwrap()[0].password, "new");
        assert!(v.delete_entry(e.id).unwrap());
        assert!(v.entries().unwrap().is_empty());
        assert_eq!(v.trashed_entries().unwrap().len(), 1);
        assert!(v.restore_entry(e.id).unwrap());
        assert_eq!(v.entries().unwrap().len(), 1);
        assert!(v.purge_entry(e.id).unwrap());
        assert!(v.entries().unwrap().is_empty() && v.trashed_entries().unwrap().is_empty());
    }

    #[test]
    fn change_password_keeps_data() {
        let (_d, path, mut v) = fresh();
        v.put_entry(&Entry::new(v.home_space(), "GitHub")).unwrap();
        v.change_password("new pass phrase", KdfParams::insecure_for_tests())
            .unwrap();
        drop(v.lock());
        assert!(
            LockedVault::open(&path)
                .unwrap()
                .unlock("correct horse battery")
                .is_err()
        );
        let v = LockedVault::open(&path)
            .unwrap()
            .unlock("new pass phrase")
            .unwrap();
        assert_eq!(v.entries().unwrap()[0].name, "GitHub");
    }

    #[test]
    fn tampered_record_is_rejected() {
        let (_d, path, v) = fresh();
        let e = Entry::new(v.home_space(), "Bank");
        v.put_entry(&e).unwrap();
        drop(v.lock());
        let c = Connection::open(&path).unwrap();
        // Move the record to another space: AAD no longer matches.
        let other: String = c
            .query_row(
                "SELECT id FROM spaces WHERE id != (SELECT space FROM records LIMIT 1)",
                [],
                |r| r.get(0),
            )
            .unwrap();
        c.execute("UPDATE records SET space = ?1", [other]).unwrap();
        drop(c);
        let v = LockedVault::open(&path)
            .unwrap()
            .unlock("correct horse battery")
            .unwrap();
        assert!(matches!(v.entries(), Err(StoreError::Crypto(_))));
    }

    #[test]
    fn rejects_other_files() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("other.db");
        Connection::open(&p)
            .unwrap()
            .execute_batch("CREATE TABLE x(a)")
            .unwrap();
        assert!(matches!(LockedVault::open(&p), Err(StoreError::NotAVault)));
    }
}
