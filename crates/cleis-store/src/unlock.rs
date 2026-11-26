//! Other ways in than the master password (D-76, 26/11/2026):
//! - the recovery key: 32 random bytes shown once as text, which seal the account secrets in the
//!   vault (`account_recovery`); it opens the vault anywhere, master password or not;
//! - a key of this computer (Windows Hello, or nothing): 32 bytes kept outside the vault by the
//!   application, which seal the account secrets in a blob it keeps next to them;
//! - no master password at all, once a recovery key exists.
//!
//! The keys are random, so they seal directly (AES-256-GCM, as everywhere else): no derivation.

use crate::{
    AccountSecrets, LockedVault, Result, StoreError, Vault, meta_get, meta_set, now_ms,
};
use cleis_crypto::{SymmetricKey, open, random_bytes, seal};
use zeroize::Zeroizing;

const RECOVERY_AAD: &[u8] = b"cleis/account/recovery";
const DEVICE_AAD: &[u8] = b"cleis/account/device";

/// Crockford's base32 alphabet: no I, L, O, U, so a key copied by hand reads back.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// The recovery key as shown: 52 characters in groups of four.
pub fn format_recovery(key: &[u8; 32]) -> String {
    let mut bits = 0u32;
    let mut n = 0;
    let mut out = String::new();
    for b in key {
        bits = (bits << 8) | u32::from(*b);
        n += 8;
        while n >= 5 {
            n -= 5;
            out.push(ALPHABET[((bits >> n) & 31) as usize] as char);
        }
    }
    if n > 0 {
        out.push(ALPHABET[((bits << (5 - n)) & 31) as usize] as char);
    }
    out.as_bytes()
        .chunks(4)
        .map(|c| std::str::from_utf8(c).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("-")
}

/// Reads a recovery key typed back: spaces, dashes and case do not matter; O reads as 0, I and L
/// as 1.
pub fn parse_recovery(text: &str) -> Option<Zeroizing<[u8; 32]>> {
    let mut bits = 0u64;
    let mut n = 0;
    let mut out = Zeroizing::new([0u8; 32]);
    let mut len = 0;
    for c in text.chars().filter(|c| !c.is_whitespace() && *c != '-') {
        let c = match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            c => c,
        };
        let v = ALPHABET.iter().position(|a| *a as char == c)? as u64;
        bits = (bits << 5) | v;
        n += 5;
        if n >= 8 {
            n -= 8;
            if len == 32 {
                return None;
            }
            out[len] = (bits >> n) as u8;
            len += 1;
        }
    }
    (len == 32).then_some(out)
}

impl Vault {
    fn account_plain(&self) -> Result<Zeroizing<Vec<u8>>> {
        let secrets = AccountSecrets {
            x25519: *self.account.secret_bytes(),
            ed25519: *self.signing.secret_bytes(),
        };
        Ok(Zeroizing::new(serde_json::to_vec(&secrets)?))
    }

    /// Makes a new recovery key (the previous one stops working) and returns it as text, to show
    /// once. It travels with the vault to the other devices.
    pub fn new_recovery_key(&mut self) -> Result<Zeroizing<String>> {
        let key: Zeroizing<[u8; 32]> = Zeroizing::new(random_bytes()?);
        let sealed = seal(
            &SymmetricKey::from_bytes(*key),
            RECOVERY_AAD,
            &self.account_plain()?,
        )?;
        let tx = self.conn.unchecked_transaction()?;
        meta_set(&self.conn, "account_recovery", &sealed)?;
        meta_set(&self.conn, "recovery_saved", &now_ms().to_be_bytes())?;
        tx.commit()?;
        Ok(Zeroizing::new(format_recovery(&key)))
    }

    /// When the recovery key was made (milliseconds), if there is one.
    pub fn recovery_saved(&self) -> Result<Option<i64>> {
        if meta_get(&self.conn, "account_recovery")?.is_none() {
            return Ok(None);
        }
        Ok(meta_get(&self.conn, "recovery_saved")?
            .and_then(|b| b.try_into().ok())
            .map(i64::from_be_bytes))
    }

    /// Whether a master password opens this vault.
    pub fn has_password(&self) -> Result<bool> {
        Ok(meta_get(&self.conn, "account")?.is_some())
    }

    /// When the master password was last changed (milliseconds), if known.
    pub fn password_changed(&self) -> Result<Option<i64>> {
        Ok(meta_get(&self.conn, "password_changed")?
            .and_then(|b| b.try_into().ok())
            .map(i64::from_be_bytes))
    }

    /// No more master password: the recovery key (required) and this computer's key open the
    /// vault. The other devices follow at their next sync.
    pub fn remove_password(&mut self) -> Result<()> {
        if self.recovery_saved()?.is_none() {
            return Err(StoreError::NoRecoveryKey);
        }
        let tx = self.conn.unchecked_transaction()?;
        self.conn.execute(
            "DELETE FROM meta WHERE key IN ('account', 'kdf', 'salt')",
            [],
        )?;
        meta_set(&self.conn, "password_changed", &now_ms().to_be_bytes())?;
        tx.commit()?;
        Ok(())
    }

    /// The account secrets sealed with a key of this computer, for the application to keep
    /// outside the vault (Windows Hello, or nothing).
    pub fn seal_for_device(&self, key: &[u8; 32]) -> Result<Vec<u8>> {
        Ok(seal(
            &SymmetricKey::from_bytes(*key),
            DEVICE_AAD,
            &self.account_plain()?,
        )?)
    }
}

impl LockedVault {
    /// Whether a master password opens this vault (otherwise: the recovery key or this
    /// computer's key).
    pub fn has_password(&self) -> Result<bool> {
        Ok(meta_get(&self.conn, "account")?.is_some())
    }

    /// Opens the vault with its recovery key, typed as shown.
    pub fn unlock_recovery(self, text: &str) -> Result<Vault> {
        let key = parse_recovery(text).ok_or(StoreError::WrongRecoveryKey)?;
        let sealed = meta_get(&self.conn, "account_recovery")?.ok_or(StoreError::NoRecoveryKey)?;
        let plain = Zeroizing::new(
            open(&SymmetricKey::from_bytes(*key), RECOVERY_AAD, &sealed)
                .map_err(|_| StoreError::WrongRecoveryKey)?,
        );
        self.unlock_with_secrets(&plain)
    }

    /// Opens the vault with this computer's key and the blob `seal_for_device` made.
    pub fn unlock_device(self, key: &[u8; 32], sealed: &[u8]) -> Result<Vault> {
        let plain = Zeroizing::new(
            open(&SymmetricKey::from_bytes(*key), DEVICE_AAD, sealed)
                .map_err(|_| StoreError::WrongDeviceKey)?,
        );
        self.unlock_with_secrets(&plain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cleis_crypto::KdfParams;

    #[test]
    fn recovery_text_round_trip() {
        let key: [u8; 32] = core::array::from_fn(|i| (i * 37 + 11) as u8);
        let text = format_recovery(&key);
        assert_eq!(text.len(), 52 + 12);
        assert_eq!(*parse_recovery(&text).unwrap(), key);
        // Typed by hand: lower case, spaces, O for 0.
        let typed = text.to_lowercase().replace('-', " ").replace('0', "o");
        assert_eq!(*parse_recovery(&typed).unwrap(), key);
        assert!(parse_recovery("ABCD").is_none());
        assert!(parse_recovery(&format!("{text}A")).is_none());
        assert!(parse_recovery(&text.replace(&text[0..1], "U")).is_none());
    }

    #[test]
    fn recovery_device_and_no_password() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v.cleis");
        let mut v = Vault::create(&path, "pw", KdfParams::insecure_for_tests()).unwrap();
        let home = v.home_space();
        assert!(v.has_password().unwrap());
        assert_eq!(v.recovery_saved().unwrap(), None);
        // Without a recovery key the password stays.
        assert!(matches!(v.remove_password(), Err(StoreError::NoRecoveryKey)));
        let text = v.new_recovery_key().unwrap();
        let device = [7u8; 32];
        let blob = v.seal_for_device(&device).unwrap();
        v.remove_password().unwrap();
        drop(v);

        let locked = LockedVault::open(&path).unwrap();
        assert!(!locked.has_password().unwrap());
        assert!(matches!(
            LockedVault::open(&path).unwrap().unlock("pw"),
            Err(StoreError::NoPassword)
        ));
        assert!(matches!(
            LockedVault::open(&path)
                .unwrap()
                .unlock_device(&[8u8; 32], &blob),
            Err(StoreError::WrongDeviceKey)
        ));
        let v = locked.unlock_device(&device, &blob).unwrap();
        assert_eq!(v.home_space(), home);
        drop(v);
        let mut v = LockedVault::open(&path)
            .unwrap()
            .unlock_recovery(&text.to_lowercase())
            .unwrap();
        assert_eq!(v.home_space(), home);
        // A master password again; the recovery key still opens.
        v.change_password("new", KdfParams::insecure_for_tests()).unwrap();
        drop(v);
        assert!(LockedVault::open(&path).unwrap().unlock("new").is_ok());
        assert!(LockedVault::open(&path).unwrap().unlock_recovery(&text).is_ok());
        assert!(matches!(
            LockedVault::open(&path)
                .unwrap()
                .unlock_recovery(&format_recovery(&[0u8; 32])),
            Err(StoreError::WrongRecoveryKey)
        ));
    }
}
