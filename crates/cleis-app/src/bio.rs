//! Biometric unlock (D-66): Windows Hello or Touch ID opens the vault without the master
//! password. A random key kept by the system for this user (DPAPI, login keychain) seals the
//! account secrets in `<vault>.bio`; the system releases it only after checking the user.
//! The master password stays the way in on any other device or after turning this off.

use cleis_crypto::SymmetricKey;
use data_encoding::BASE64;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize)]
struct BioFile {
    version: u8,
    /// The key as the system keeps it (Windows: DPAPI blob).
    key: String,
    /// Account secrets sealed with the key.
    sealed: String,
}

pub fn file(vault: &Path) -> PathBuf {
    let mut p = vault.as_os_str().to_owned();
    p.push(".bio");
    PathBuf::from(p)
}

/// Name of the key in the system's store (one per vault file).
fn id(vault: &Path) -> String {
    let p = vault.canonicalize().unwrap_or_else(|_| vault.to_owned());
    let h = cleis_crypto::sha256(p.to_string_lossy().as_bytes());
    format!("vault-{}", &cleis_crypto::hex(&h)[..16])
}

/// "Windows Hello", "Touch ID", or "" (none on this system).
pub fn name() -> &'static str {
    cleis_autofill::biometric::name()
}

pub fn available() -> bool {
    cleis_autofill::biometric::available()
}

pub fn enabled(vault: &Path) -> bool {
    file(vault).exists()
}

/// Turns it on for this vault, on this device (after a check of the user).
pub fn enable(vault_path: &Path, vault: &cleis_store::Vault) -> Result<(), String> {
    let key = SymmetricKey::generate().map_err(|e| e.to_string())?;
    let sealed = vault
        .seal_account_for_device(&key)
        .map_err(|e| e.to_string())?;
    let kept = cleis_autofill::biometric::protect(&id(vault_path), key.as_bytes())?;
    let f = BioFile {
        version: 1,
        key: BASE64.encode(&kept),
        sealed: BASE64.encode(&sealed),
    };
    std::fs::write(
        file(vault_path),
        serde_json::to_vec(&f).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

pub fn disable(vault: &Path) {
    cleis_autofill::biometric::forget(&id(vault));
    let _ = std::fs::remove_file(file(vault));
}

/// Checks the user (blocking: call off the interface thread).
pub fn verify(window: Option<u64>) -> Result<(), String> {
    cleis_autofill::biometric::verify(
        "unlock your Cleis vault",
        window.map(cleis_autofill::WindowRef),
    )
}

/// After `verify`: opens the vault with the key the system keeps.
pub fn unlock(vault: &Path) -> Result<cleis_store::Vault, String> {
    let f: BioFile =
        serde_json::from_slice(&std::fs::read(file(vault)).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let kept = BASE64.decode(f.key.as_bytes()).map_err(|e| e.to_string())?;
    let sealed = BASE64
        .decode(f.sealed.as_bytes())
        .map_err(|e| e.to_string())?;
    let raw = zeroize::Zeroizing::new(cleis_autofill::biometric::unprotect(&id(vault), &kept)?);
    let bytes: [u8; 32] = raw
        .as_slice()
        .try_into()
        .map_err(|_| "unreadable key".to_owned())?;
    let key = SymmetricKey::from_bytes(bytes);
    cleis_store::LockedVault::open(vault)
        .and_then(|l| l.unlock_with_device(&key, &sealed))
        .map_err(|e| match e {
            cleis_store::StoreError::WrongPassword => {
                "This device's key no longer opens the vault: use the master password".to_owned()
            }
            other => other.to_string(),
        })
}
