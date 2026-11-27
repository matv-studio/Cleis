//! How this computer opens the vault (D-76, mockups 35h, 35i): the master password, Windows
//! Hello, or nothing. For the last two, a key of this computer seals the account secrets in
//! `<vault>.unlock`, next to the vault but never synced with it:
//! - Windows Hello: the key is the SHA-256 of the Windows Hello signature of a random challenge
//!   kept in that file; only the user, through the prompt, gets the signature;
//! - nothing: a random key, sealed by DPAPI for the Windows account;
//! - a PIN (mockups 37e-37h, D-82): Argon2id of the PIN with a secret of this computer sealed by
//!   DPAPI (`cleis_crypto::derive_pin_key`); after `max_tries` wrong PINs in a row the file is
//!   removed and the master password (or the recovery key) is needed again.

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
    /// A PIN of 4, 6 or 8 digits, on this computer only.
    Pin,
}

/// PIN lengths offered, and the default.
pub const PIN_LENGTHS: [usize; 3] = [4, 6, 8];
pub const DEFAULT_PIN_LEN: usize = 6;
/// Wrong PINs in a row before the master password is needed again: offered, and the default.
pub const PIN_TRIES: [u8; 3] = [3, 5, 10];
pub const DEFAULT_PIN_TRIES: u8 = 5;

#[derive(Serialize, Deserialize)]
struct LocalFile {
    mode: Mode,
    /// The account secrets sealed with this computer's key.
    sealed: Vec<u8>,
    /// Windows Hello: the challenge signed.
    #[serde(default)]
    challenge: Vec<u8>,
    /// Nothing: the key, sealed for the Windows account. PIN: this computer's secret, sealed
    /// the same way.
    #[serde(default)]
    key: Vec<u8>,
    /// PIN: Argon2id's salt and costs, its length, the tries allowed and left.
    #[serde(default)]
    salt: Vec<u8>,
    #[serde(default)]
    kdf: Option<cleis_crypto::KdfParams>,
    #[serde(default)]
    pin_len: usize,
    #[serde(default)]
    max_tries: u8,
    #[serde(default)]
    tries_left: u8,
}

/// The PIN settings of this computer, when it opens with a PIN.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PinInfo {
    pub len: usize,
    pub max_tries: u8,
    pub tries_left: u8,
}

pub fn pin_info(vault: &Path) -> Option<PinInfo> {
    read(vault)
        .filter(|f| f.mode == Mode::Pin)
        .map(|f| PinInfo {
            len: f.pin_len,
            max_tries: f.max_tries,
            tries_left: f.tries_left,
        })
}

/// Why a PIN did not open the vault.
#[derive(Debug, PartialEq, Eq)]
pub enum PinError {
    /// Wrong PIN; tries left before the master password is needed.
    Wrong(u8),
    /// Too many wrong PINs: the PIN is gone from this computer.
    Locked,
    Other(String),
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
            sealed,
            challenge,
            ..LocalFile::empty(Mode::Hello)
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
            sealed,
            key: protected,
            ..LocalFile::empty(Mode::Nothing)
        },
    )
}

impl LocalFile {
    fn empty(mode: Mode) -> Self {
        Self {
            mode,
            sealed: Vec::new(),
            challenge: Vec::new(),
            key: Vec::new(),
            salt: Vec::new(),
            kdf: None,
            pin_len: 0,
            max_tries: 0,
            tries_left: 0,
        }
    }
}

/// Costs of the PIN's Argon2id: the vault's defaults (about half a second), cheap in tests.
fn pin_kdf() -> cleis_crypto::KdfParams {
    if cfg!(test) || cfg!(debug_assertions) && std::env::var_os("CLEIS_FAST_KDF").is_some() {
        cleis_crypto::KdfParams::insecure_for_tests()
    } else {
        cleis_crypto::KdfParams::default()
    }
}

fn pin_key(
    pin: &str,
    salt: &[u8],
    protected: &[u8],
    kdf: cleis_crypto::KdfParams,
) -> Result<Zeroizing<[u8; 32]>, String> {
    let salt: [u8; 16] = salt
        .try_into()
        .map_err(|_| "This computer's PIN file is damaged".to_string())?;
    let raw = Zeroizing::new(cleis_autofill::unlock::unprotect(protected)?);
    let secret: Zeroizing<[u8; 32]> = Zeroizing::new(
        raw.as_slice()
            .try_into()
            .map_err(|_| "This computer's key is damaged".to_string())?,
    );
    cleis_crypto::derive_pin_key(pin, &salt, &secret, kdf).map_err(|e| e.to_string())
}

/// A new PIN, its key made (the slow part, any thread): written by `finish_pin`.
pub struct PreparedPin {
    key: Zeroizing<[u8; 32]>,
    file: LocalFile,
}

/// The slow half of setting a PIN (Argon2id): no vault needed, run off the interface thread.
pub fn prepare_pin(pin: &str, len: usize, max_tries: u8) -> Result<PreparedPin, String> {
    if pin.len() != len || !pin.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("The PIN has {len} digits"));
    }
    let secret: Zeroizing<[u8; 32]> =
        Zeroizing::new(cleis_crypto::random_bytes().map_err(|e| e.to_string())?);
    let salt: [u8; 16] = cleis_crypto::random_bytes().map_err(|e| e.to_string())?;
    let protected = cleis_autofill::unlock::protect(&*secret)?;
    let kdf = pin_kdf();
    let key = pin_key(pin, &salt, &protected, kdf)?;
    Ok(PreparedPin {
        key,
        file: LocalFile {
            key: protected,
            salt: salt.to_vec(),
            kdf: Some(kdf),
            pin_len: len,
            max_tries,
            tries_left: max_tries,
            ..LocalFile::empty(Mode::Pin)
        },
    })
}

/// The quick half: the account secrets sealed with the PIN's key, the file written.
pub fn finish_pin(path: &Path, v: &Vault, mut p: PreparedPin) -> Result<(), String> {
    p.file.sealed = v.seal_for_device(&p.key).map_err(|e| e.to_string())?;
    write(path, &p.file)
}

/// A PIN opens the vault on this computer from now on (`len` digits, `max_tries` wrong ones in
/// a row allowed).
pub fn set_pin(path: &Path, v: &Vault, pin: &str, len: usize, max_tries: u8) -> Result<(), String> {
    finish_pin(path, v, prepare_pin(pin, len, max_tries)?)
}

/// The tries allowed changed (Security › Opening Cleis): counted again from full.
pub fn set_pin_tries(path: &Path, max_tries: u8) -> Result<(), String> {
    let mut f = read(path)
        .filter(|f| f.mode == Mode::Pin)
        .ok_or_else(|| "No PIN on this computer".to_string())?;
    f.max_tries = max_tries;
    f.tries_left = max_tries;
    write(path, &f)
}

/// Opens the vault with the PIN; a wrong one is counted, the last allowed one removes the PIN.
pub fn open_pin(path: &Path, pin: &str) -> Result<Vault, PinError> {
    let mut f = read(path)
        .filter(|f| f.mode == Mode::Pin)
        .ok_or(PinError::Locked)?;
    if f.tries_left == 0 {
        forget(path);
        return Err(PinError::Locked);
    }
    let locked = LockedVault::open(path).map_err(|e| PinError::Other(e.to_string()))?;
    let kdf = f.kdf.unwrap_or_else(pin_kdf);
    let key = pin_key(pin, &f.salt, &f.key, kdf).map_err(PinError::Other)?;
    match locked.unlock_device(&key, &f.sealed) {
        Ok(v) => {
            if f.tries_left != f.max_tries {
                f.tries_left = f.max_tries;
                let _ = write(path, &f);
            }
            Ok(v)
        }
        Err(StoreError::WrongDeviceKey) => {
            f.tries_left = f.tries_left.saturating_sub(1);
            if f.tries_left == 0 {
                forget(path);
                return Err(PinError::Locked);
            }
            write(path, &f).map_err(PinError::Other)?;
            Err(PinError::Wrong(f.tries_left))
        }
        Err(other) => Err(PinError::Other(other.to_string())),
    }
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
        (Mode::Pin, _) => return Err("The PIN is needed".into()),
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
    fn pin_opens_counts_and_locks() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v.cleis");
        let v = Vault::create(&path, "pw", cleis_crypto::KdfParams::insecure_for_tests()).unwrap();
        let home = v.home_space();
        assert!(set_pin(&path, &v, "12345", 6, 3).is_err(), "length checked");
        assert!(set_pin(&path, &v, "12a456", 6, 3).is_err(), "digits only");
        set_pin(&path, &v, "123456", 6, 3).unwrap();
        drop(v);
        assert_eq!(mode(&path), Mode::Pin);
        assert!(open(&path, None).is_err(), "no PIN, no opening");
        assert_eq!(open_pin(&path, "000000").err(), Some(PinError::Wrong(2)));
        // The right one counts from full again.
        assert_eq!(open_pin(&path, "123456").unwrap().home_space(), home);
        assert_eq!(pin_info(&path).unwrap().tries_left, 3);
        assert_eq!(open_pin(&path, "111111").err(), Some(PinError::Wrong(2)));
        assert_eq!(open_pin(&path, "222222").err(), Some(PinError::Wrong(1)));
        assert_eq!(open_pin(&path, "333333").err(), Some(PinError::Locked));
        // Gone: the master password again.
        assert_eq!(mode(&path), Mode::Password);
        assert_eq!(open_pin(&path, "123456").err(), Some(PinError::Locked));
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
