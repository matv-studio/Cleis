//! AES-256-GCM with random 96-bit nonces.
//!
//! A random nonce is safe for well under 2^32 messages per key (NIST SP 800-38D §8.3);
//! every item is re-sealed with a fresh nonce, and space keys are rotated on re-encryption,
//! so a single key never comes close to that bound.

use crate::{CryptoError, SymmetricKey, random_bytes};
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};

const VERSION: u8 = 1;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;

/// Encrypts `plaintext`; `aad` is authenticated but not encrypted (bind the ciphertext
/// to its context, e.g. the item and space ids, so it cannot be moved elsewhere).
pub fn seal(key: &SymmetricKey, aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new(key.as_bytes().into());
    let nonce_bytes: [u8; NONCE_LEN] = random_bytes()?;
    let nonce = Nonce::from(nonce_bytes);
    let ct = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| CryptoError::Decrypt)?;
    let mut out = Vec::with_capacity(1 + NONCE_LEN + ct.len());
    out.push(VERSION);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ct);
    Ok(out)
}

/// Decrypts data produced by [`seal`] with the same key and AAD.
pub fn open(key: &SymmetricKey, aad: &[u8], sealed: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let (&version, rest) = sealed.split_first().ok_or(CryptoError::Malformed)?;
    if version != VERSION {
        return Err(CryptoError::Version(version));
    }
    if rest.len() < NONCE_LEN + TAG_LEN {
        return Err(CryptoError::Malformed);
    }
    let (nonce_bytes, ct) = rest.split_at(NONCE_LEN);
    let nonce_arr: [u8; NONCE_LEN] = nonce_bytes.try_into().map_err(|_| CryptoError::Malformed)?;
    let cipher = Aes256Gcm::new(key.as_bytes().into());
    cipher
        .decrypt(&Nonce::from(nonce_arr), Payload { msg: ct, aad })
        .map_err(|_| CryptoError::Decrypt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let k = SymmetricKey::generate().unwrap();
        let s = seal(&k, b"item:1", b"hunter2").unwrap();
        assert_eq!(open(&k, b"item:1", &s).unwrap(), b"hunter2");
    }

    #[test]
    fn nonces_differ() {
        let k = SymmetricKey::generate().unwrap();
        assert_ne!(seal(&k, b"", b"x").unwrap(), seal(&k, b"", b"x").unwrap());
    }

    #[test]
    fn wrong_aad_or_key_or_tamper_fails() {
        let k = SymmetricKey::generate().unwrap();
        let s = seal(&k, b"a", b"secret").unwrap();
        assert_eq!(open(&k, b"b", &s), Err(CryptoError::Decrypt));
        let k2 = SymmetricKey::generate().unwrap();
        assert_eq!(open(&k2, b"a", &s), Err(CryptoError::Decrypt));
        let mut t = s.clone();
        let last = t.len() - 1;
        t[last] ^= 1;
        assert_eq!(open(&k, b"a", &t), Err(CryptoError::Decrypt));
        assert_eq!(open(&k, b"a", &s[..10]), Err(CryptoError::Malformed));
        let mut v = s;
        v[0] = 9;
        assert_eq!(open(&k, b"a", &v), Err(CryptoError::Version(9)));
    }

    /// NIST GCM test vector (gcmEncryptExtIV256, Count 0 with 128-bit tag, empty PT):
    /// checks we are really running AES-256-GCM and not something else.
    #[test]
    fn known_answer_empty() {
        let key = SymmetricKey::from_bytes([0u8; 32]);
        let cipher = Aes256Gcm::new(key.as_bytes().into());
        let tag = cipher
            .encrypt(&Nonce::from([0u8; 12]), b"".as_ref())
            .unwrap();
        assert_eq!(crate::hex(&tag), "530f8afbc74536b9a963b4f1c4cb738b");
    }
}
