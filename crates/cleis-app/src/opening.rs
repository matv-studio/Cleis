//! How this computer opens the vault (D-76, mockups 35h, 35i): the master password, Windows
//! Hello, or nothing. For the last two, a key of this computer seals the account secrets in
//! `<vault>.unlock`, next to the vault but never synced with it:
//! - Windows Hello: the key is the SHA-256 of the Windows Hello signature of a random challenge
//!   kept in that file; only the user, through the prompt, gets the signature;
//! - nothing: a random key, sealed by DPAPI for the Windows account.

use cleis_store::{LockedVault, StoreError, Vault};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

/// The Windows Hello key of Cleis (one per Windows account; the challenge is per vault).
const HELLO_KEY: &str = "Cleis";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// The master password only.
    Password,
    Hello,
    Nothing,
}

#[derive(Serialize, Deserialize)]
struct LocalFile {
    mode: Mode,
    /// The account secrets sealed with this computer's key.
    sealed: Vec<u8>,
    /// Windows Hello: the challenge signed.
    #[serde(default)]
    challenge: Vec<u8>,
    /// Nothing: the key, sealed for the Windows account.
    #[serde(default)]
    key: Vec<u8>,
}

fn file_of(vault: &Path) -> PathBuf {
    let mut p = vault.as_os_str().to_owned();
    p.push(".unlock");
    PathBuf::from(p)
}

fn read(vault: &Path) -> Option<LocalFile> {
    serde_json::from_slice(&std::fs::read(file_of(vault)).ok()?).ok()
}

/// How this computer opens `vault`.
pub fn mode(vault: &Path) -> Mode {
    read(vault).map_or(Mode::Password, |f| f.mode)
}

/// Whether Windows Hello can be offered here.
pub fn hello_available() -> bool {
    cleis_autofill::unlock::hello_available()
}

/// The key Windows Hello gives for `challenge` (the prompt is shown). Blocks: run it off the
/// interface thread.
pub fn hello_key(challenge: &[u8]) -> Result<Zeroizing<[u8; 32]>, String> {
    let sig = Zeroizing::new(cleis_autofill::unlock::hello_sign(HELLO_KEY, challenge)?);
    Ok(Zeroizing::new(cleis_crypto::sha256(&sig)))
}

/// A new challenge for Windows Hello.
pub fn new_challenge() -> Vec<u8> {
    cleis_crypto::random_bytes::<32>()
        .map(|b| b.to_vec())
        .unwrap_or_default()
}

fn write(vault: &Path, f: &LocalFile) -> Result<(), String> {
    let path = file_of(vault);
    let bytes = serde_json::to_vec(f).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("unlock-new");
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    // Read by this user only (Linux, macOS; Windows: the profile folder already is).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

/// Windows Hello opens the vault on this computer from now on (`key` from `hello_key`).
pub fn set_hello(path: &Path, v: &Vault, challenge: Vec<u8>, key: &[u8; 32]) -> Result<(), String> {
    let sealed = v.seal_for_device(key).map_err(|e| e.to_string())?;
    write(
        path,
        &LocalFile {
            mode: Mode::Hello,
            sealed,
            challenge,
            key: Vec::new(),
        },
    )
}

/// Nothing to type on this computer from now on.
pub fn set_nothing(path: &Path, v: &Vault) -> Result<(), String> {
    let key: Zeroizing<[u8; 32]> =
        Zeroizing::new(cleis_crypto::random_bytes().map_err(|e| e.to_string())?);
    let sealed = v.seal_for_device(&key).map_err(|e| e.to_string())?;
    let protected = cleis_autofill::unlock::protect(&*key)?;
    write(
        path,
        &LocalFile {
            mode: Mode::Nothing,
            sealed,
            challenge: Vec::new(),
            key: protected,
        },
    )
}

/// Back to the master password on this computer (or a rule of the organization).
pub fn forget(path: &Path) {
    let _ = std::fs::remove_file(file_of(path));
}

/// The Windows Hello challenge of this computer, if it opens with Windows Hello.
pub fn hello_challenge(path: &Path) -> Option<Vec<u8>> {
    read(path)
        .filter(|f| f.mode == Mode::Hello)
        .map(|f| f.challenge)
}

/// Opens the vault without the master password: with the Windows Hello key (`hello`, from
/// `hello_key`) or, for "nothing", with the key kept for the Windows account.
pub fn open(path: &Path, hello: Option<&[u8; 32]>) -> Result<Vault, String> {
    let f = read(path).ok_or_else(|| "This computer has no key for this vault".to_string())?;
    let locked = LockedVault::open(path).map_err(|e| e.to_string())?;
    let key: Zeroizing<[u8; 32]> = match (f.mode, hello) {
        (Mode::Hello, Some(k)) => Zeroizing::new(*k),
        (Mode::Nothing, _) => {
            let raw = Zeroizing::new(cleis_autofill::unlock::unprotect(&f.key)?);
            Zeroizing::new(
                raw.as_slice()
                    .try_into()
                    .map_err(|_| "This computer's key is damaged".to_string())?,
            )
        }
        _ => return Err("Windows Hello is needed".into()),
    };
    locked.unlock_device(&key, &f.sealed).map_err(|e| match e {
        // The vault was changed elsewhere (new account keys) or the file is someone else's.
        StoreError::WrongDeviceKey => {
            "This computer's key no longer opens the vault: use the master password or the recovery key".into()
        }
        other => other.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_opens_and_forget_removes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v.cleis");
        let v = Vault::create(&path, "pw", cleis_crypto::KdfParams::insecure_for_tests()).unwrap();
        let home = v.home_space();
        assert_eq!(mode(&path), Mode::Password);
        set_nothing(&path, &v).unwrap();
        drop(v);
        assert_eq!(mode(&path), Mode::Nothing);
        assert_eq!(open(&path, None).unwrap().home_space(), home);
        forget(&path);
        assert_eq!(mode(&path), Mode::Password);
        assert!(open(&path, None).is_err());
    }

    #[test]
    fn hello_needs_its_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v.cleis");
        let v = Vault::create(&path, "pw", cleis_crypto::KdfParams::insecure_for_tests()).unwrap();
        // The key Windows Hello would give, as a stand-in.
        let key = [9u8; 32];
        set_hello(&path, &v, new_challenge(), &key).unwrap();
        drop(v);
        assert_eq!(mode(&path), Mode::Hello);
        assert_eq!(hello_challenge(&path).unwrap().len(), 32);
        assert!(open(&path, None).is_err());
        assert!(open(&path, Some(&[1u8; 32])).is_err());
        assert!(open(&path, Some(&key)).is_ok());
    }
}
