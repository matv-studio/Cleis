//! Organization spaces in the vault file (D-59). They sit beside the personal spaces, sealed the
//! same way; their rows go to and come from the organization server as they are (sealed), never
//! to the personal sync place or backups. The server's copy is the reference: a row changed here
//! and not yet sent (`changed > pushed`) is kept until the server takes or refuses it.

use crate::{Result, SpaceInfo, StoreError, Vault, aad, now_ms};
use cleis_core::SpaceId;
use cleis_core::org::Row;
use cleis_crypto::{KeyPair, PublicKey, SymmetricKey, open, seal, unwrap_key, wrap_key};
use rusqlite::{OptionalExtension, params};

/// Seals a space's name and settings (the `sealed` field of a space, here and on the server).
pub fn seal_space_info(key: &SymmetricKey, info: &SpaceInfo) -> Result<Vec<u8>> {
    let id = info.id.to_string();
    Ok(seal(
        key,
        &aad("space", &id, &info.id),
        &serde_json::to_vec(info)?,
    )?)
}

pub fn open_space_info(key: &SymmetricKey, id: SpaceId, sealed: &[u8]) -> Result<SpaceInfo> {
    let plain = open(key, &aad("space", &id.to_string(), &id), sealed)?;
    Ok(serde_json::from_slice(&plain)?)
}

/// A space key, wrapped for a member (context: the space id, as for the vault's own spaces).
pub fn wrap_for(key: &SymmetricKey, space: SpaceId, to: &PublicKey) -> Result<Vec<u8>> {
    Ok(wrap_key(to, key, space.0.as_bytes())?)
}

const ROW_COLS: &str =
    "r.kind, r.id, r.space, r.sealed, r.rev, r.updated, r.deleted, r.changed, s.key_gen";

fn row_from(r: &rusqlite::Row) -> rusqlite::Result<Row> {
    Ok(Row {
        kind: r.get(0)?,
        id: r.get(1)?,
        space: SpaceId(r.get::<_, String>(2)?.parse().unwrap_or_default()),
        sealed: r.get(3)?,
        rev: r.get(4)?,
        updated: r.get(5)?,
        deleted: r.get(6)?,
        changed: r.get(7)?,
        key_gen: r.get(8)?,
    })
}

/// What a re-encryption prepared: applied here once the server took it.
pub struct Rekeyed {
    pub space: SpaceId,
    pub key: SymmetricKey,
    pub key_gen: u32,
    pub sealed: Vec<u8>,
    pub rows: Vec<Row>,
}

impl Vault {
    /// The organization spaces of `org` held here, with their key generation.
    pub fn org_spaces(&self, org: &str) -> Result<Vec<(SpaceId, u32)>> {
        let mut st = self
            .conn
            .prepare("SELECT id, key_gen FROM spaces WHERE org = ?1 ORDER BY rowid")?;
        let rows = st.query_map([org], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?)))?;
        let mut out = vec![];
        for r in rows {
            let (id, g) = r?;
            out.push((
                SpaceId(id.parse().map_err(|_| StoreError::Corrupt("space id"))?),
                g,
            ));
        }
        Ok(out)
    }

    /// Whether `space` belongs to an organization (not this vault's own).
    pub fn is_org_space(&self, space: SpaceId) -> Result<bool> {
        Ok(self
            .conn
            .query_row(
                "SELECT org IS NOT NULL FROM spaces WHERE id = ?1",
                [space.to_string()],
                |r| r.get::<_, bool>(0),
            )
            .optional()?
            .unwrap_or(false))
    }

    /// Adds or updates an organization space from the server: its sealed info and the key
    /// wrapped to this account. Refuses a key that does not open the info.
    pub fn put_org_space(
        &mut self,
        org: &str,
        id: SpaceId,
        sealed: &[u8],
        wrapped: &[u8],
        key_gen: u32,
    ) -> Result<()> {
        let key = unwrap_key(&self.account, wrapped, id.0.as_bytes())?;
        open_space_info(&key, id, sealed)?;
        self.conn.execute(
            "INSERT INTO spaces(id, wrapped_key, sealed, changed, org, key_gen) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET wrapped_key = excluded.wrapped_key, sealed = excluded.sealed,
               org = excluded.org, key_gen = excluded.key_gen",
            params![id.to_string(), wrapped, sealed, now_ms(), org, key_gen],
        )?;
        self.space_keys.insert(id, key);
        Ok(())
    }

    /// Forgets an organization space and everything in it (access removed, organization left).
    pub fn remove_org_space(&mut self, id: SpaceId) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        self.conn
            .execute("DELETE FROM records WHERE space = ?1", [id.to_string()])?;
        self.conn
            .execute("DELETE FROM purged WHERE space = ?1", [id.to_string()])?;
        self.conn.execute(
            "DELETE FROM spaces WHERE id = ?1 AND org IS NOT NULL",
            [id.to_string()],
        )?;
        tx.commit()?;
        self.space_keys.remove(&id);
        Ok(())
    }

    /// A new organization space: its key, wrapped to this account, kept here at once. Returns
    /// the sealed info and the key (to wrap for the organization key and the recovery key).
    pub fn create_org_space(
        &mut self,
        org: &str,
        info: SpaceInfo,
    ) -> Result<(Vec<u8>, Vec<u8>, SymmetricKey)> {
        let key = SymmetricKey::generate()?;
        let sealed = seal_space_info(&key, &info)?;
        let wrapped = wrap_for(&key, info.id, &self.account.public())?;
        self.put_org_space(org, info.id, &sealed, &wrapped, 1)?;
        Ok((sealed, wrapped, key))
    }

    /// A space's key, for wrapping it to another member.
    pub fn space_key(&self, space: SpaceId) -> Option<&SymmetricKey> {
        self.space_keys.get(&space)
    }

    /// Opens something wrapped to this account (the organization key given to an admin).
    pub fn unwrap_own(&self, wrapped: &[u8], context: &[u8]) -> Result<KeyPair> {
        let k = unwrap_key(&self.account, wrapped, context)?;
        Ok(KeyPair::from_secret_bytes(*k.as_bytes()))
    }

    /// Rows of `org`'s spaces changed here since they were last agreed with the server.
    pub fn org_dirty(&self, org: &str) -> Result<Vec<Row>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT {ROW_COLS} FROM records r JOIN spaces s ON s.id = r.space WHERE s.org = ?1 AND r.changed > r.pushed"
        ))?;
        let rows = st
            .query_map([org], row_from)?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    /// Erasures in `org`'s spaces not yet sent.
    pub fn org_unsent_purges(&self, org: &str) -> Result<Vec<(String, String)>> {
        let mut st = self.conn.prepare(
            "SELECT p.kind, p.id FROM purged p JOIN spaces s ON s.id = p.space WHERE s.org = ?1 AND p.sent = 0",
        )?;
        let rows = st
            .query_map([org], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    /// The server answered a push: rows it took are agreed; rows it refused are agreed too (its
    /// own copy comes back with the next pull), erasures are sent.
    pub fn org_pushed(&self, rows: &[Row], purges: &[(String, String)]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for r in rows {
            self.conn.execute(
                "UPDATE records SET pushed = ?3 WHERE kind = ?1 AND id = ?2 AND changed = ?3",
                params![r.kind, r.id, r.changed],
            )?;
        }
        for (k, i) in purges {
            self.conn.execute(
                "UPDATE purged SET sent = 1 WHERE kind = ?1 AND id = ?2",
                params![k, i],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Takes rows and erasures from the server. A row changed here after the server's copy, not
    /// yet sent, is kept; rows of a space whose key here is of another generation wait for the
    /// keys to be refreshed. Returns how many rows changed.
    pub fn org_apply(&mut self, rows: &[Row], purged: &[(String, String)]) -> Result<usize> {
        let tx = self.conn.unchecked_transaction()?;
        let mut n = 0;
        for r in rows {
            let gen_here: Option<u32> = self
                .conn
                .query_row(
                    "SELECT key_gen FROM spaces WHERE id = ?1 AND org IS NOT NULL",
                    [r.space.to_string()],
                    |x| x.get(0),
                )
                .optional()?;
            if gen_here != Some(r.key_gen) || !matches!(r.kind.as_str(), "entry" | "contact") {
                continue;
            }
            let local: Option<(i64, i64)> = self
                .conn
                .query_row(
                    "SELECT changed, pushed FROM records WHERE kind = ?1 AND id = ?2",
                    params![r.kind, r.id],
                    |x| Ok((x.get(0)?, x.get(1)?)),
                )
                .optional()?;
            if let Some((changed, pushed)) = local
                && changed > pushed
                && changed > r.changed
            {
                continue;
            }
            n += self.conn.execute(
                "INSERT INTO records(kind, id, space, sealed, rev, updated, deleted, changed, pushed)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
                 ON CONFLICT(kind, id) DO UPDATE SET space = excluded.space, sealed = excluded.sealed,
                   rev = excluded.rev, updated = excluded.updated, deleted = excluded.deleted,
                   changed = excluded.changed, pushed = excluded.changed",
                params![r.kind, r.id, r.space.to_string(), r.sealed, r.rev, r.updated, r.deleted, r.changed],
            )?;
        }
        for (k, i) in purged {
            n += self.conn.execute(
                "DELETE FROM records WHERE kind = ?1 AND id = ?2",
                params![k, i],
            )?;
        }
        tx.commit()?;
        Ok(n)
    }

    /// Prepares the re-encryption of an organization space: a new key and every row of the space
    /// opened with the old key and sealed with the new one. Nothing changes here until
    /// [`Vault::org_rekeyed`].
    pub fn org_prepare_rekey(&self, space: SpaceId, info: &SpaceInfo) -> Result<Rekeyed> {
        let old = self.key(space)?;
        let key_gen: u32 = self.conn.query_row(
            "SELECT key_gen FROM spaces WHERE id = ?1",
            [space.to_string()],
            |r| r.get(0),
        )?;
        let key = SymmetricKey::generate()?;
        let mut st = self.conn.prepare(&format!(
            "SELECT {ROW_COLS} FROM records r JOIN spaces s ON s.id = r.space WHERE r.space = ?1"
        ))?;
        let rows: Vec<Row> = st
            .query_map([space.to_string()], row_from)?
            .collect::<rusqlite::Result<_>>()?;
        let mut out = Vec::with_capacity(rows.len());
        for mut r in rows {
            let a = aad(&r.kind, &r.id, &space);
            let plain = zeroize::Zeroizing::new(open(old, &a, &r.sealed)?);
            r.sealed = seal(&key, &a, &plain)?;
            r.key_gen = key_gen + 1;
            out.push(r);
        }
        Ok(Rekeyed {
            space,
            sealed: seal_space_info(&key, info)?,
            key,
            key_gen: key_gen + 1,
            rows: out,
        })
    }

    /// The server took a re-encryption: keep the new key and rows.
    pub fn org_rekeyed(&mut self, org: &str, r: &Rekeyed) -> Result<()> {
        let wrapped = wrap_for(&r.key, r.space, &self.account.public())?;
        let tx = self.conn.unchecked_transaction()?;
        self.conn.execute(
            "UPDATE spaces SET wrapped_key = ?2, sealed = ?3, key_gen = ?4, org = ?5 WHERE id = ?1",
            params![r.space.to_string(), wrapped, r.sealed, r.key_gen, org],
        )?;
        for row in &r.rows {
            self.conn.execute(
                "UPDATE records SET sealed = ?3, changed = ?4, pushed = ?4 WHERE kind = ?1 AND id = ?2",
                params![row.kind, row.id, row.sealed, row.changed],
            )?;
        }
        tx.commit()?;
        self.space_keys.insert(r.space, r.key.clone());
        Ok(())
    }

    /// Sync cursor for an organization (this device only).
    pub fn org_cursor(&self, org: &str) -> Result<u64> {
        Ok(crate::meta_get(&self.conn, &format!("org_seq:{org}"))?
            .and_then(|b| b.try_into().ok())
            .map(u64::from_be_bytes)
            .unwrap_or(0))
    }

    pub fn set_org_cursor(&self, org: &str, seq: u64) -> Result<()> {
        crate::meta_set(&self.conn, &format!("org_seq:{org}"), &seq.to_be_bytes())
    }

    /// Forgets an organization entirely (left, or removed by its admins).
    pub fn leave_org(&mut self, org: &str) -> Result<()> {
        for (s, _) in self.org_spaces(org)? {
            self.remove_org_space(s)?;
        }
        self.conn.execute(
            "DELETE FROM meta WHERE key = ?1",
            [format!("org_seq:{org}")],
        )?;
        Ok(())
    }

    /// Settings sealed with the personal key (organization membership, kept across devices).
    pub fn org_setting<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        self.setting(key)
    }

    pub fn set_org_setting<T: serde::Serialize>(&self, key: &str, v: &T) -> Result<()> {
        self.set_setting(key, v)
    }
}

#[cfg(test)]
mod tests {
    use crate::{EncryptionMode, LockedVault, SpaceInfo, SpaceKind, Vault};
    use cleis_core::{Entry, SpaceId};
    use cleis_crypto::KdfParams;

    #[test]
    fn org_rows_round_trip_and_rekey() {
        let d = tempfile::tempdir().unwrap();
        let mut a =
            Vault::create(&d.path().join("a"), "pw", KdfParams::insecure_for_tests()).unwrap();
        let mut b =
            Vault::create(&d.path().join("b"), "pw", KdfParams::insecure_for_tests()).unwrap();
        let info = SpaceInfo {
            id: SpaceId::new(),
            name: "General".into(),
            kind: SpaceKind::Organization,
            mode: EncryptionMode::Standard,
            for_contacts: false,
        };
        let (sealed, _, key) = a.create_org_space("org1", info.clone()).unwrap();
        assert!(a.is_org_space(info.id).unwrap());
        let e = Entry::new(info.id, "Figma");
        a.put_entry(&e).unwrap();
        let dirty = a.org_dirty("org1").unwrap();
        assert_eq!(dirty.len(), 1);
        // Personal sync copies leave organization spaces out.
        let place = d.path().join("place.cleis");
        a.sync_with(&place).unwrap();
        let c = rusqlite::Connection::open(&place).unwrap();
        let n: i64 = c
            .query_row(
                "SELECT count(*) FROM spaces WHERE org IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let m: i64 = c
            .query_row("SELECT count(*) FROM records", [], |r| r.get(0))
            .unwrap();
        assert_eq!((n, m), (0, 0));
        drop(c);
        a.sync_with(&place).unwrap();
        assert_eq!(
            a.entries().unwrap().len(),
            1,
            "the sync keeps the org row here"
        );
        a.org_pushed(&dirty, &[]).unwrap();
        assert!(a.org_dirty("org1").unwrap().is_empty());

        // B is given the space: the key wrapped to B's account, the rows as they are.
        let w = super::wrap_for(&key, info.id, &b.account_public()).unwrap();
        b.put_org_space("org1", info.id, &sealed, &w, 1).unwrap();
        assert_eq!(b.org_apply(&dirty, &[]).unwrap(), 1);
        assert_eq!(b.entries().unwrap()[0].name, "Figma");
        assert!(b.spaces().unwrap().iter().any(|s| s.name == "General"));
        // A wrong key is refused.
        let w2 = super::wrap_for(
            &cleis_crypto::SymmetricKey::generate().unwrap(),
            info.id,
            &b.account_public(),
        )
        .unwrap();
        assert!(b.put_org_space("org1", info.id, &sealed, &w2, 1).is_err());

        // A local edit not yet sent beats an older server row; a newer one replaces it.
        let mut e2 = e.clone();
        e2.username = "local".into();
        std::thread::sleep(std::time::Duration::from_millis(3));
        b.put_entry(&e2).unwrap();
        b.org_apply(&dirty, &[]).unwrap();
        assert_eq!(b.entries().unwrap()[0].username, "local");

        // Re-encryption: same content under a new key and generation.
        let r = a.org_prepare_rekey(info.id, &info).unwrap();
        assert_eq!(r.key_gen, 2);
        a.org_rekeyed("org1", &r).unwrap();
        assert_eq!(a.org_spaces("org1").unwrap(), vec![(info.id, 2)]);
        assert_eq!(a.entries().unwrap()[0].name, "Figma");
        // B still on generation 1: rows of generation 2 wait.
        assert_eq!(b.org_apply(&r.rows, &[]).unwrap(), 0);

        // Erasure in an org space waits to be sent; leaving forgets everything.
        a.purge_entry(e.id).unwrap();
        assert_eq!(a.org_unsent_purges("org1").unwrap().len(), 1);
        a.set_org_cursor("org1", 42).unwrap();
        assert_eq!(a.org_cursor("org1").unwrap(), 42);
        b.leave_org("org1").unwrap();
        assert!(b.entries().unwrap().is_empty());
        assert!(!b.spaces().unwrap().iter().any(|s| s.name == "General"));
        drop(a);
        // Reopens with the org space key.
        let a = LockedVault::open(&d.path().join("a"))
            .unwrap()
            .unlock("pw")
            .unwrap();
        assert!(a.spaces().unwrap().iter().any(|s| s.name == "General"));
    }
}
