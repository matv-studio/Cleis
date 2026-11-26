//! Keeping two copies of the same vault in step (D-50): this device's file and a copy in a sync
//! place (a folder kept by OneDrive, Google Drive, Dropbox or a NAS; later GitHub, SFTP).
//!
//! The merge works on sealed rows and never decrypts them: each record, space and setting keeps
//! the copy changed last (`changed`, milliseconds), and records erased for good on either side
//! stay erased (`purged`). Both copies hold the same account keys, so every row opens on both.
//! The merged file then replaces the other copy in one step (written beside it, then renamed).

use crate::{Result, StoreError, Vault, meta_get, meta_set};
use rusqlite::OptionalExtension;
use rusqlite::{Connection, params};
use std::path::Path;

/// What a sync did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SyncReport {
    /// Rows taken from the other copy.
    pub received: usize,
    /// Rows the other copy did not have, or had older.
    pub sent: usize,
    /// The other copy did not exist yet: it was created.
    pub created: bool,
}

fn count(conn: &Connection, sql: &str) -> Result<usize> {
    Ok(conn.query_row(sql, [], |r| r.get::<_, i64>(0))? as usize)
}

impl Vault {
    /// Path of this vault's file.
    pub fn path(&self) -> Option<std::path::PathBuf> {
        self.conn.path().map(std::path::PathBuf::from)
    }

    /// Writes a consistent copy of the vault to `dest` (backups, first copy in a sync place). The
    /// copy is made beside `dest` and renamed, so `dest` is never half written.
    pub fn copy_to(&self, dest: &Path) -> Result<()> {
        let tmp = dest.with_extension(format!("part-{}", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        self.conn
            .execute("VACUUM INTO ?1", [tmp.to_string_lossy().as_ref()])?;
        // Organization spaces live on their server, not in personal copies (D-59).
        let stripped = (|| -> Result<()> {
            let c = Connection::open(&tmp)?;
            c.pragma_update(None, "secure_delete", "ON")?;
            c.execute_batch(
                "DELETE FROM records WHERE space IN (SELECT id FROM spaces WHERE org IS NOT NULL);
                 DELETE FROM purged WHERE space IN (SELECT id FROM spaces WHERE org IS NOT NULL);
                 DELETE FROM spaces WHERE org IS NOT NULL;
                 DELETE FROM meta WHERE key LIKE 'org_seq:%';
                 VACUUM;",
            )?;
            Ok(())
        })();
        if let Err(e) = stripped {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
        std::fs::rename(&tmp, dest).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            StoreError::Io(e.to_string())
        })
    }

    /// Merges this vault with the copy at `other`, then writes the result to both.
    pub fn sync_with(&mut self, other: &Path) -> Result<SyncReport> {
        if !other.exists() {
            if let Some(dir) = other.parent() {
                std::fs::create_dir_all(dir).map_err(|e| StoreError::Io(e.to_string()))?;
            }
            self.copy_to(other)?;
            return Ok(SyncReport {
                created: true,
                ..Default::default()
            });
        }
        // Work on a private copy of the other file: the sync tool may rewrite it meanwhile.
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let snapshot = std::env::temp_dir().join(format!(
            "cleis-sync-{}-{}-{}.db",
            std::process::id(),
            crate::now_ms(),
            N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::copy(other, &snapshot).map_err(|e| StoreError::Io(e.to_string()))?;
        let result = self.merge_from(&snapshot);
        let _ = std::fs::remove_file(&snapshot);
        let report = result?;
        self.copy_to(other)?;
        Ok(report)
    }

    fn merge_from(&mut self, snapshot: &Path) -> Result<SyncReport> {
        {
            let o = crate::LockedVault::open(snapshot)?;
            let theirs = meta_get(&o.conn, "account_public")?;
            if theirs.is_none() || theirs != meta_get(&self.conn, "account_public")? {
                return Err(StoreError::OtherVault);
            }
        }
        let c = &self.conn;
        c.execute(
            "ATTACH DATABASE ?1 AS o",
            [snapshot.to_string_lossy().as_ref()],
        )?;
        let r = (|| -> Result<SyncReport> {
            let tx = c.unchecked_transaction()?;
            let sent = count(
                c,
                "SELECT
                   (SELECT count(*) FROM main.records m LEFT JOIN o.records t USING (kind, id)
                      WHERE (t.id IS NULL OR m.changed > t.changed)
                        AND m.space NOT IN (SELECT id FROM main.spaces WHERE org IS NOT NULL))
                 + (SELECT count(*) FROM main.spaces m LEFT JOIN o.spaces t USING (id)
                      WHERE m.org IS NULL AND (t.id IS NULL OR m.changed > t.changed))
                 + (SELECT count(*) FROM main.settings m LEFT JOIN o.settings t USING (key)
                      WHERE t.key IS NULL OR m.changed > t.changed)",
            )?;
            let received = count(
                c,
                "SELECT
                   (SELECT count(*) FROM o.records t LEFT JOIN main.records m USING (kind, id)
                      LEFT JOIN main.purged p USING (kind, id)
                      WHERE (m.id IS NULL OR t.changed > m.changed) AND (p.id IS NULL OR t.changed > p.at))
                 + (SELECT count(*) FROM o.spaces t LEFT JOIN main.spaces m USING (id)
                      WHERE t.org IS NULL AND (m.id IS NULL OR t.changed > m.changed)
                        AND t.id NOT IN (SELECT id FROM main.purged WHERE kind = 'space'))
                 + (SELECT count(*) FROM o.settings t LEFT JOIN main.settings m USING (key)
                      WHERE m.key IS NULL OR t.changed > m.changed)",
            )?;
            // Spaces first: records point at them.
            c.execute_batch(
                "INSERT INTO main.spaces(id, wrapped_key, sealed, rev, changed)
                   SELECT id, wrapped_key, sealed, rev, changed FROM o.spaces t WHERE t.org IS NULL
                     AND t.id NOT IN (SELECT id FROM main.purged WHERE kind = 'space')
                   ON CONFLICT(id) DO UPDATE SET wrapped_key = excluded.wrapped_key,
                     sealed = excluded.sealed, rev = excluded.rev, changed = excluded.changed
                   WHERE excluded.changed > main.spaces.changed AND main.spaces.org IS NULL;
                 INSERT INTO main.purged(kind, id, at)
                   SELECT kind, id, at FROM o.purged WHERE sent = 1
                   ON CONFLICT(kind, id) DO UPDATE SET at = max(main.purged.at, excluded.at);
                 INSERT INTO main.records(kind, id, space, sealed, rev, updated, deleted, changed)
                   SELECT kind, id, space, sealed, rev, updated, deleted, changed FROM o.records t
                   WHERE t.space NOT IN (SELECT id FROM o.spaces WHERE org IS NOT NULL)
                     AND t.space NOT IN (SELECT id FROM main.spaces WHERE org IS NOT NULL)
                   ON CONFLICT(kind, id) DO UPDATE SET space = excluded.space,
                     sealed = excluded.sealed, rev = excluded.rev, updated = excluded.updated,
                     deleted = excluded.deleted, changed = excluded.changed
                   WHERE excluded.changed > main.records.changed;
                 DELETE FROM main.records WHERE EXISTS (
                   SELECT 1 FROM main.purged p WHERE p.kind = records.kind AND p.id = records.id
                     AND p.at >= records.changed);
                 DELETE FROM main.spaces WHERE org IS NULL
                   AND id IN (SELECT id FROM main.purged WHERE kind = 'space')
                   AND NOT EXISTS (SELECT 1 FROM main.records r WHERE r.space = spaces.id);
                 INSERT INTO main.settings(key, sealed, changed)
                   SELECT key, sealed, changed FROM o.settings WHERE true
                   ON CONFLICT(key) DO UPDATE SET sealed = excluded.sealed,
                     changed = excluded.changed
                   WHERE excluded.changed > main.settings.changed;",
            )?;
            // Master password changed on the other device: its sealing of the same account
            // keys is the current one.
            let when = |db: &str, key: &str| -> Result<i64> {
                let v: Option<Vec<u8>> = c
                    .query_row(
                        &format!("SELECT value FROM {db}.meta WHERE key = ?1"),
                        params![key],
                        |r| r.get(0),
                    )
                    .ok();
                Ok(v.and_then(|b| b.try_into().ok())
                    .map(i64::from_be_bytes)
                    .unwrap_or(0))
            };
            // Master password removed there (D-76): removed here too. Same for a newer
            // recovery key.
            let copy = |keys: &[&str]| -> Result<()> {
                for key in keys {
                    let v: Option<Vec<u8>> = c
                        .query_row(
                            "SELECT value FROM o.meta WHERE key = ?1",
                            params![key],
                            |r| r.get(0),
                        )
                        .optional()?;
                    match v {
                        Some(v) => meta_set(c, key, &v)?,
                        None => {
                            c.execute("DELETE FROM main.meta WHERE key = ?1", params![key])?;
                        }
                    }
                }
                Ok(())
            };
            if when("o", "password_changed")? > when("main", "password_changed")? {
                copy(&["kdf", "salt", "account", "password_changed"])?;
            }
            if when("o", "recovery_saved")? > when("main", "recovery_saved")? {
                copy(&["account_recovery", "recovery_saved"])?;
            }
            tx.commit()?;
            Ok(SyncReport {
                received,
                sent,
                created: false,
            })
        })();
        c.execute("DETACH DATABASE o", [])?;
        let report = r?;
        // New spaces came in: load their keys.
        self.reload_space_keys()?;
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use crate::{LockedVault, Vault};
    use cleis_core::{Contact, Entry};
    use cleis_crypto::KdfParams;

    const PW: &str = "correct horse battery";

    #[test]
    fn two_devices_merge() {
        let d = tempfile::tempdir().unwrap();
        let a_path = d.path().join("a.cleis");
        let place = d.path().join("place/vault.cleis");
        let mut a = Vault::create(&a_path, PW, KdfParams::insecure_for_tests()).unwrap();
        let mut gh = Entry::new(a.home_space(), "GitHub");
        a.put_entry(&gh).unwrap();
        let old = Entry::new(a.home_space(), "Old");
        a.put_entry(&old).unwrap();
        let r = a.sync_with(&place).unwrap();
        assert!(r.created);

        // Second device: a copy of the place.
        let b_path = d.path().join("b.cleis");
        std::fs::copy(&place, &b_path).unwrap();
        let mut b = LockedVault::open(&b_path).unwrap().unlock(PW).unwrap();
        let mut c = Contact::person(b.home_space(), "Camille", "Dubois");
        b.put_contact(&c).unwrap();
        gh.username = "dev@example-studio.com".into();
        std::thread::sleep(std::time::Duration::from_millis(5));
        b.put_entry(&gh).unwrap();
        b.purge_entry(old.id).unwrap();
        b.sync_with(&place).unwrap();

        // Meanwhile on A: a newer edit of the contact cannot exist yet; A edits something else.
        let gl = Entry::new(a.home_space(), "GitLab");
        a.put_entry(&gl).unwrap();
        let r = a.sync_with(&place).unwrap();
        assert_eq!(r.received, 2, "{r:?}");
        let names: Vec<String> = a
            .entries()
            .unwrap()
            .iter()
            .map(|e| e.name.clone())
            .collect();
        assert!(names.contains(&"GitLab".to_string()));
        assert!(
            !names.contains(&"Old".to_string()),
            "erased on B stays erased"
        );
        let gh_a = a
            .entries()
            .unwrap()
            .into_iter()
            .find(|e| e.id == gh.id)
            .unwrap();
        assert_eq!(gh_a.username, "dev@example-studio.com");
        assert_eq!(a.contacts().unwrap().len(), 1);

        // B gets GitLab; a later edit on B wins over the older one.
        b.sync_with(&place).unwrap();
        assert_eq!(b.entries().unwrap().len(), 2);
        c.company = "Example Print".into();
        b.put_contact(&c).unwrap();
        b.sync_with(&place).unwrap();
        a.sync_with(&place).unwrap();
        assert_eq!(a.contacts().unwrap()[0].company, "Example Print");

        // Password changed on B: A's next sync takes it, the old one stops opening the place.
        b.change_password("a new pass phrase", KdfParams::insecure_for_tests())
            .unwrap();
        b.sync_with(&place).unwrap();
        a.sync_with(&place).unwrap();
        drop(a);
        assert!(
            LockedVault::open(&a_path)
                .unwrap()
                .unlock("a new pass phrase")
                .is_ok()
        );
    }

    /// D-76: a recovery key and the removal of the master password reach the other device.
    #[test]
    fn recovery_key_and_no_password_travel() {
        let d = tempfile::tempdir().unwrap();
        let (a_path, b_path) = (d.path().join("a.cleis"), d.path().join("b.cleis"));
        let place = d.path().join("place/vault.cleis");
        let mut a = Vault::create(&a_path, PW, KdfParams::insecure_for_tests()).unwrap();
        a.sync_with(&place).unwrap();
        std::fs::copy(&place, &b_path).unwrap();
        let mut b = LockedVault::open(&b_path).unwrap().unlock(PW).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let key = a.new_recovery_key().unwrap();
        a.remove_password().unwrap();
        a.sync_with(&place).unwrap();
        b.sync_with(&place).unwrap();
        assert!(!b.has_password().unwrap());
        drop(b);
        assert!(
            LockedVault::open(&b_path)
                .unwrap()
                .unlock_recovery(&key)
                .is_ok()
        );
    }

    #[test]
    fn refuses_another_vault() {
        let d = tempfile::tempdir().unwrap();
        let mut a =
            Vault::create(&d.path().join("a"), PW, KdfParams::insecure_for_tests()).unwrap();
        let b = Vault::create(&d.path().join("b"), PW, KdfParams::insecure_for_tests()).unwrap();
        drop(b);
        assert!(matches!(
            a.sync_with(&d.path().join("b")),
            Err(crate::StoreError::OtherVault)
        ));
    }
}

#[cfg(test)]
mod folder_tests {
    use crate::{EncryptionMode, SpaceKind, Vault};
    use cleis_core::Entry;
    use cleis_crypto::KdfParams;

    #[test]
    fn deleted_folder_moves_items_and_stays_deleted() {
        let d = tempfile::tempdir().unwrap();
        let place = d.path().join("place.cleis");
        let mut a =
            Vault::create(&d.path().join("a"), "pw", KdfParams::insecure_for_tests()).unwrap();
        let bank = a
            .create_space(
                "Banking",
                SpaceKind::Personal,
                EncryptionMode::EndToEnd,
                false,
            )
            .unwrap();
        let e = Entry::new(bank, "Bank");
        a.put_entry(&e).unwrap();
        let old = Entry::new(bank, "Old card");
        a.put_entry(&old).unwrap();
        a.delete_entry(old.id).unwrap();
        a.sync_with(&place).unwrap();
        // The place's copy still has the folder; deleting here wins at the next sync.
        let home = a.home_space();
        assert_eq!(a.delete_space(bank, home).unwrap(), 2);
        assert!(a.delete_space(home, bank).is_err(), "home stays");
        let moved = a.entries().unwrap();
        assert_eq!(moved.len(), 1);
        assert_eq!(moved[0].space, home);
        assert_eq!(a.trashed_entries().unwrap()[0].space, home);
        a.sync_with(&place).unwrap();
        assert!(a.spaces().unwrap().iter().all(|s| s.id != bank));
        assert_eq!(a.entries().unwrap()[0].space, home);
    }
}
