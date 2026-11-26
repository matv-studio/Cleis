//! The server's SQLite file. It holds only what the apps sealed or wrapped, plus who may do
//! what: member names and e-mail addresses, roles, public keys, the activity log (item names in
//! it sealed for the admins), authenticator keys of the members who set one up.

use cleis_core::journal::{Chain, DeviceId, Record};
use cleis_core::org::{Event, Integrity, Policy, Role};
use cleis_crypto::SigningKeyPair;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;

pub const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value BLOB NOT NULL) STRICT;
CREATE TABLE IF NOT EXISTS members (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, email TEXT NOT NULL, role TEXT NOT NULL,
    account_public BLOB NOT NULL UNIQUE, verify_key BLOB NOT NULL,
    export INTEGER NOT NULL DEFAULT 0,
    totp TEXT, totp_pending TEXT, totp_last INTEGER NOT NULL DEFAULT 0,
    created INTEGER NOT NULL, last_sign_in INTEGER, last_device TEXT NOT NULL DEFAULT '',
    revoked INTEGER
) STRICT;
CREATE TABLE IF NOT EXISTS invites (
    id TEXT PRIMARY KEY, token_hash BLOB NOT NULL UNIQUE, name TEXT NOT NULL, email TEXT NOT NULL,
    role TEXT NOT NULL, spaces TEXT NOT NULL, created INTEGER NOT NULL, expires INTEGER NOT NULL
) STRICT;
CREATE TABLE IF NOT EXISTS spaces (
    id TEXT PRIMARY KEY, sealed BLOB NOT NULL, mode TEXT NOT NULL, key_gen INTEGER NOT NULL,
    org_wrapped BLOB, recovery_wrapped BLOB, needs_rekey INTEGER NOT NULL DEFAULT 0,
    created INTEGER NOT NULL
) STRICT;
CREATE TABLE IF NOT EXISTS access (
    space TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    member TEXT NOT NULL REFERENCES members(id),
    wrapped BLOB,
    PRIMARY KEY (space, member)
) STRICT;
CREATE TABLE IF NOT EXISTS admin_keys (member TEXT PRIMARY KEY REFERENCES members(id), wrapped BLOB NOT NULL) STRICT;
CREATE TABLE IF NOT EXISTS records (
    kind TEXT NOT NULL, id TEXT NOT NULL, space TEXT NOT NULL REFERENCES spaces(id),
    sealed BLOB NOT NULL, rev INTEGER NOT NULL, updated INTEGER NOT NULL, deleted INTEGER,
    changed INTEGER NOT NULL, key_gen INTEGER NOT NULL, seq INTEGER NOT NULL, author TEXT NOT NULL,
    PRIMARY KEY (kind, id)
) STRICT;
CREATE INDEX IF NOT EXISTS records_seq ON records(seq);
CREATE TABLE IF NOT EXISTS purged (
    kind TEXT NOT NULL, id TEXT NOT NULL, space TEXT NOT NULL, at INTEGER NOT NULL, seq INTEGER NOT NULL,
    PRIMARY KEY (kind, id)
) STRICT;
CREATE TABLE IF NOT EXISTS versions (
    kind TEXT NOT NULL, id TEXT NOT NULL, space TEXT NOT NULL, sealed BLOB NOT NULL, rev INTEGER NOT NULL,
    updated INTEGER NOT NULL, deleted INTEGER, changed INTEGER NOT NULL, key_gen INTEGER NOT NULL,
    author TEXT NOT NULL, saved INTEGER NOT NULL
) STRICT;
CREATE INDEX IF NOT EXISTS versions_item ON versions(kind, id);
CREATE TABLE IF NOT EXISTS sessions (
    token_hash BLOB PRIMARY KEY, member TEXT NOT NULL, device TEXT NOT NULL, expires INTEGER NOT NULL,
    setup_only INTEGER NOT NULL
) STRICT;
CREATE TABLE IF NOT EXISTS remembered (token_hash BLOB PRIMARY KEY, member TEXT NOT NULL, expires INTEGER NOT NULL) STRICT;
CREATE TABLE IF NOT EXISTS challenges (nonce BLOB PRIMARY KEY, expires INTEGER NOT NULL) STRICT;
CREATE TABLE IF NOT EXISTS log (seq INTEGER PRIMARY KEY, at INTEGER NOT NULL, record TEXT NOT NULL) STRICT;
";

pub fn configure(conn: &Connection) -> rusqlite::Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "FULL")?;
    conn.pragma_update(None, "secure_delete", "ON")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    conn.execute_batch(SCHEMA)?;
    // Devices page (D-60): when a session was last used, which device a remembered code is for.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS super_keys (member TEXT PRIMARY KEY REFERENCES members(id), wrapped BLOB NOT NULL) STRICT;",
    )?;
    for (t, col, def) in [
        ("sessions", "last", "INTEGER NOT NULL DEFAULT 0"),
        ("remembered", "device", "TEXT NOT NULL DEFAULT ''"),
        // Reserved spaces (D-61): who gave each access, who made each invitation.
        ("spaces", "reserved", "INTEGER NOT NULL DEFAULT 0"),
        ("access", "granted_by", "TEXT"),
        ("invites", "created_by", "TEXT"),
        // Second sign-in step (D-64): method and its target (phone, identity account).
        ("members", "factor", "TEXT"),
        ("members", "factor_target", "TEXT"),
        // Organization PIN (D-79): wrong PINs in a row.
        ("members", "pin_fails", "INTEGER NOT NULL DEFAULT 0"),
    ] {
        let has: bool = conn
            .prepare(&format!(
                "SELECT 1 FROM pragma_table_info('{t}') WHERE name = ?1"
            ))?
            .exists([col])?;
        if !has {
            conn.execute_batch(&format!("ALTER TABLE {t} ADD COLUMN {col} {def}"))?;
        }
    }
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS codes (member TEXT PRIMARY KEY REFERENCES members(id), purpose TEXT NOT NULL, method TEXT NOT NULL, target TEXT NOT NULL, hash BLOB NOT NULL, expires INTEGER NOT NULL, tries INTEGER NOT NULL DEFAULT 0, sent INTEGER NOT NULL) STRICT;
         CREATE TABLE IF NOT EXISTS used_tokens (hash BLOB PRIMARY KEY, expires INTEGER NOT NULL) STRICT;
         CREATE TABLE IF NOT EXISTS code_sends (member TEXT NOT NULL, at INTEGER NOT NULL) STRICT;
         CREATE TABLE IF NOT EXISTS pin_ok (member TEXT NOT NULL, device TEXT NOT NULL, expires INTEGER NOT NULL, PRIMARY KEY (member, device)) STRICT;
         UPDATE members SET factor = 'authenticator_app' WHERE totp IS NOT NULL AND factor IS NULL;",
    )?;
    Ok(())
}

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    configure(&conn)?;
    Ok(conn)
}

pub fn meta_get(conn: &Connection, key: &str) -> rusqlite::Result<Option<Vec<u8>>> {
    conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
        .optional()
}

pub fn meta_set(conn: &Connection, key: &str, value: &[u8]) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn meta_del(conn: &Connection, key: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM meta WHERE key = ?1", [key])?;
    Ok(())
}

pub fn meta_json<T: DeserializeOwned>(conn: &Connection, key: &str) -> Option<T> {
    meta_get(conn, key)
        .ok()
        .flatten()
        .and_then(|b| serde_json::from_slice(&b).ok())
}

pub fn meta_set_json<T: Serialize>(conn: &Connection, key: &str, v: &T) -> rusqlite::Result<()> {
    meta_set(
        conn,
        key,
        &serde_json::to_vec(v).expect("serializable meta value"),
    )
}

pub fn policy(conn: &Connection) -> Policy {
    meta_json(conn, "policy").unwrap_or_default()
}

pub fn roles(conn: &Connection) -> Vec<Role> {
    meta_json(conn, "roles")
        .map(cleis_core::org::upgrade_roles)
        .unwrap_or_else(Role::defaults)
}

/// Next change number (records, purges, re-encryptions).
pub fn next_seq(conn: &Connection) -> rusqlite::Result<u64> {
    let cur = seq(conn)?;
    meta_set(conn, "seq", &(cur + 1).to_be_bytes())?;
    Ok(cur + 1)
}

pub fn seq(conn: &Connection) -> rusqlite::Result<u64> {
    Ok(meta_get(conn, "seq")?
        .and_then(|b| b.try_into().ok())
        .map(u64::from_be_bytes)
        .unwrap_or(0))
}

/// The activity log: one chain written by the server, signed with its own Ed25519 key.
pub struct Log {
    pub device: DeviceId,
    secret: [u8; 32],
    chain: Chain,
}

impl Log {
    pub fn load(conn: &Connection) -> rusqlite::Result<Log> {
        let device = match meta_get(conn, "log_device")? {
            Some(b) => DeviceId(b.try_into().unwrap_or([0; 16])),
            None => {
                let d: [u8; 16] = cleis_crypto::random_bytes().expect("random");
                meta_set(conn, "log_device", &d)?;
                DeviceId(d)
            }
        };
        let secret: [u8; 32] = match meta_get(conn, "log_key")? {
            Some(b) => b.try_into().unwrap_or([0; 32]),
            None => {
                let k = SigningKeyPair::generate().expect("random");
                let s = *k.secret_bytes();
                meta_set(conn, "log_key", &s)?;
                s
            }
        };
        let mut log = Log {
            device,
            secret,
            chain: Chain::resume(device, SigningKeyPair::from_secret_bytes(secret), None),
        };
        log.reload(conn)?;
        Ok(log)
    }

    /// Resumes after the last record kept (also after a failed write).
    pub fn reload(&mut self, conn: &Connection) -> rusqlite::Result<()> {
        let last: Option<String> = conn
            .query_row(
                "SELECT record FROM log ORDER BY seq DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?;
        let last: Option<Record> = last.and_then(|s| serde_json::from_str(&s).ok());
        let key = SigningKeyPair::from_secret_bytes(self.secret);
        self.chain = match last {
            Some(r) => Chain::resume(self.device, key, Some(&r)),
            None => {
                // Everything was pruned: continue after the anchor.
                let anchor = anchor(conn);
                if anchor.0 == 0 {
                    Chain::resume(self.device, key, None)
                } else {
                    Chain::resume_at(self.device, key, anchor.0, anchor.1)
                }
            }
        };
        Ok(())
    }

    pub fn verifying_key(&self) -> cleis_crypto::VerifyingKey {
        SigningKeyPair::from_secret_bytes(self.secret).verifying_key()
    }

    pub fn append(&mut self, conn: &Connection, event: &Event) -> rusqlite::Result<u64> {
        let payload = serde_json::to_vec(event).expect("serializable event");
        let rec = self.chain.append(event.at, payload, vec![]);
        let written = conn.execute(
            "INSERT INTO log(seq, at, record) VALUES (?1, ?2, ?3)",
            params![
                rec.seq as i64,
                rec.at,
                serde_json::to_string(&rec).expect("serializable record")
            ],
        );
        match written {
            Ok(_) => Ok(rec.seq),
            Err(e) => {
                self.reload(conn)?;
                Err(e)
            }
        }
    }

    /// Checks every signature and link of what is kept.
    pub fn verify(&self, conn: &Connection, now: i64) -> rusqlite::Result<Integrity> {
        let mut st = conn.prepare("SELECT record FROM log ORDER BY seq")?;
        let recs: Vec<Record> = st
            .query_map([], |r| r.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .filter_map(|s| serde_json::from_str(&s).ok())
            .collect();
        let total: i64 = conn.query_row("SELECT count(*) FROM log", [], |r| r.get(0))?;
        let mut hashes = Default::default();
        let (n, problems) = cleis_core::journal::verify_chain(
            self.device,
            &recs,
            &self.verifying_key(),
            anchor(conn),
            &mut hashes,
        );
        let mut problem = problems.first().map(|p| format!("{p:?}"));
        if problem.is_none() && n as i64 != total {
            problem = Some("unreadable record".into());
        }
        Ok(Integrity {
            checked: n as u64,
            intact: problem.is_none(),
            problem,
            at: now,
        })
    }
}

/// `(seq, hash)` of the last record dropped by retention.
pub fn anchor(conn: &Connection) -> (u64, [u8; 32]) {
    let seq = meta_get(conn, "log_anchor_seq")
        .ok()
        .flatten()
        .and_then(|b| b.try_into().ok())
        .map(u64::from_be_bytes)
        .unwrap_or(0);
    let hash = meta_get(conn, "log_anchor_hash")
        .ok()
        .flatten()
        .and_then(|b| b.try_into().ok())
        .unwrap_or([0; 32]);
    (seq, hash)
}

/// Drops events older than the policy keeps; the hash of the last one dropped becomes the
/// anchor, so what remains still verifies.
pub fn prune_log(conn: &Connection, now: i64) -> rusqlite::Result<usize> {
    let keep = i64::from(policy(conn).keep_days.max(1)) * 86_400;
    let last: Option<(i64, String)> = conn
        .query_row(
            "SELECT seq, record FROM log WHERE at < ?1 ORDER BY seq DESC LIMIT 1",
            [now - keep],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((seq, rec)) = last else {
        return Ok(0);
    };
    let Ok(rec) = serde_json::from_str::<Record>(&rec) else {
        return Ok(0);
    };
    let n = conn.execute("DELETE FROM log WHERE seq <= ?1", [seq])?;
    meta_set(conn, "log_anchor_seq", &(seq as u64).to_be_bytes())?;
    meta_set(conn, "log_anchor_hash", &rec.hash())?;
    Ok(n)
}

/// Servers made before the Super Admin role (D-60): the role list gets it, and the earliest admin
/// becomes Super Admin if nobody is.
pub fn migrate_roles(conn: &Connection) -> rusqlite::Result<()> {
    use cleis_core::org::{ADMIN, SUPER_ADMIN};
    let mut roles = roles(conn);
    if !roles.iter().any(|r| r.id == SUPER_ADMIN) {
        let d = Role::defaults();
        roles.retain(|r| r.id != ADMIN);
        roles.splice(0..0, d[..2].iter().cloned());
        meta_set_json(conn, "roles", &roles)?;
    }
    let has_super: bool = conn
        .prepare("SELECT 1 FROM members WHERE role = ?1 AND revoked IS NULL")?
        .exists([SUPER_ADMIN])?;
    if !has_super {
        conn.execute(
            "UPDATE members SET role = ?1 WHERE id = (SELECT id FROM members WHERE role = ?2 AND revoked IS NULL ORDER BY created LIMIT 1)",
            [SUPER_ADMIN, ADMIN],
        )?;
    }
    Ok(())
}
