//! Master-password key derivation: Argon2id (RFC 9106), then HKDF-SHA256 to split
//! independent sub-keys (encryption key, server authentication key).

use crate::{CryptoError, SymmetricKey};
use argon2::{Algorithm, Argon2, Params, Version};
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use unicode_normalization::UnicodeNormalization;
use zeroize::Zeroizing;

/// Argon2id cost parameters, stored next to the vault so they can be raised later.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdfParams {
    /// Memory in KiB.
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

impl Default for KdfParams {
    /// 64 MiB, 3 passes, 4 lanes: same order as RFC 9106's second recommended option and
    /// as current password-manager defaults; about half a second on an office laptop.
    fn default() -> Self {
        Self {
            memory_kib: 64 * 1024,
            iterations: 3,
            parallelism: 4,
        }
    }
}

impl KdfParams {
    /// Lowest settings accepted for a real vault (audit S-12): OWASP's minimum for Argon2id,
    /// 19 MiB and 2 passes. A vault below it is brought up to the defaults after unlocking.
    pub const FLOOR_MEMORY_KIB: u32 = 19 * 1024;
    pub const FLOOR_ITERATIONS: u32 = 2;

    pub fn meets_floor(&self) -> bool {
        self.memory_kib >= Self::FLOOR_MEMORY_KIB
            && self.iterations >= Self::FLOOR_ITERATIONS
            && self.parallelism >= 1
    }

    /// Cheap parameters for tests only.
    pub fn insecure_for_tests() -> Self {
        Self {
            memory_kib: 256,
            iterations: 1,
            parallelism: 1,
        }
    }
}

/// Keys derived from the master password. Never stored, never sent.
pub struct MasterKey {
    /// Encrypts the user's private keys.
    pub encryption: SymmetricKey,
    /// Proves knowledge of the password to the server without revealing it.
    pub authentication: SymmetricKey,
}

pub fn derive_master_key(
    password: &str,
    salt: &[u8; 16],
    params: KdfParams,
) -> Result<MasterKey, CryptoError> {
    let argon = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(
            params.memory_kib,
            params.iterations,
            params.parallelism,
            Some(32),
        )
        .map_err(|_| CryptoError::Kdf)?,
    );
    // NFC first: "é" typed on macOS (often decomposed) and on Windows (composed) must give
    // the same key, or a vault created on one could not be opened on the other.
    let normalized = Zeroizing::new(password.nfc().collect::<String>());
    let mut stretched = Zeroizing::new([0u8; 32]);
    argon
        .hash_password_into(normalized.as_bytes(), salt, stretched.as_mut())
        .map_err(|_| CryptoError::Kdf)?;
    let hk = Hkdf::<Sha256>::new(None, stretched.as_ref());
    let mut enc = Zeroizing::new([0u8; 32]);
    let mut auth = Zeroizing::new([0u8; 32]);
    hk.expand(b"cleis/master/enc/v1", enc.as_mut())
        .map_err(|_| CryptoError::Kdf)?;
    hk.expand(b"cleis/master/auth/v1", auth.as_mut())
        .map_err(|_| CryptoError::Kdf)?;
    Ok(MasterKey {
        encryption: SymmetricKey::from_bytes(*enc),
        authentication: SymmetricKey::from_bytes(*auth),
    })
}

/// The key a PIN opens the vault with on one computer (mockups 37e-37h, 27/11/2026): Argon2id of
/// the PIN, then HKDF-SHA256 salted with a secret of this computer (sealed by the system for the
/// account: DPAPI on Windows). A PIN has few digits: without that secret, a copy of the files
/// taken elsewhere could be tried PIN after PIN; with it, trying needs this account on this
/// computer, where the app counts the wrong tries.
pub fn derive_pin_key(
    pin: &str,
    salt: &[u8; 16],
    device_secret: &[u8; 32],
    params: KdfParams,
) -> Result<Zeroizing<[u8; 32]>, CryptoError> {
    let argon = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(
            params.memory_kib,
            params.iterations,
            params.parallelism,
            Some(32),
        )
        .map_err(|_| CryptoError::Kdf)?,
    );
    let mut stretched = Zeroizing::new([0u8; 32]);
    argon
        .hash_password_into(pin.as_bytes(), salt, stretched.as_mut())
        .map_err(|_| CryptoError::Kdf)?;
    hkdf_sha256(device_secret, stretched.as_ref(), b"cleis/pin/v1")
}

/// A key from a secret kept by the system and a challenge (D-98, Touch ID): HKDF-SHA256 of the
/// secret, salted by the challenge, so the same challenge always gives the same key, as Windows
/// Hello's deterministic signature does.
pub fn derive_device_key(
    device_secret: &[u8; 32],
    challenge: &[u8],
) -> Result<Zeroizing<[u8; 32]>, CryptoError> {
    hkdf_sha256(challenge, device_secret, b"cleis/device-check/v1")
}

/// HKDF-SHA256 (RFC 5869), 32 bytes out.
fn hkdf_sha256(salt: &[u8], ikm: &[u8], info: &[u8]) -> Result<Zeroizing<[u8; 32]>, CryptoError> {
    let hk = Hkdf::<Sha256>::new(Some(salt), ikm);
    let mut out = Zeroizing::new([0u8; 32]);
    hk.expand(info, out.as_mut())
        .map_err(|_| CryptoError::Kdf)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 5869, test case 1 (the first 32 bytes of its 42-byte OKM).
    #[test]
    fn hkdf_official_vector() {
        let ikm = [0x0bu8; 22];
        let salt: Vec<u8> = (0x00u8..=0x0c).collect();
        let info: Vec<u8> = (0xf0u8..=0xf9).collect();
        let okm = hkdf_sha256(&salt, &ikm, &info).unwrap();
        assert_eq!(
            crate::hex(okm.as_ref()),
            "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf"
        );
    }

    #[test]
    fn device_key_is_stable_per_challenge() {
        let a = derive_device_key(&[3; 32], b"challenge").unwrap();
        assert_eq!(a, derive_device_key(&[3; 32], b"challenge").unwrap());
        assert_ne!(a, derive_device_key(&[3; 32], b"another").unwrap());
        assert_ne!(a, derive_device_key(&[4; 32], b"challenge").unwrap());
    }

    #[test]
    fn pin_key_needs_pin_and_computer() {
        let p = KdfParams::insecure_for_tests();
        let a = derive_pin_key("123456", &[1; 16], &[7; 32], p).unwrap();
        assert_eq!(a, derive_pin_key("123456", &[1; 16], &[7; 32], p).unwrap());
        assert_ne!(a, derive_pin_key("123457", &[1; 16], &[7; 32], p).unwrap());
        assert_ne!(a, derive_pin_key("123456", &[1; 16], &[8; 32], p).unwrap());
        assert_ne!(a, derive_pin_key("123456", &[2; 16], &[7; 32], p).unwrap());
    }

    #[test]
    fn deterministic_and_salted() {
        let p = KdfParams::insecure_for_tests();
        let a = derive_master_key("correct horse", &[1; 16], p).unwrap();
        let b = derive_master_key("correct horse", &[1; 16], p).unwrap();
        let c = derive_master_key("correct horse", &[2; 16], p).unwrap();
        let d = derive_master_key("correct horsf", &[1; 16], p).unwrap();
        assert_eq!(a.encryption, b.encryption);
        assert_ne!(a.encryption, c.encryption);
        assert_ne!(a.encryption, d.encryption);
        assert_ne!(a.encryption, a.authentication);
    }

    #[test]
    fn composed_and_decomposed_passwords_match() {
        let p = KdfParams::insecure_for_tests();
        let composed = derive_master_key("Caf\u{e9} cr\u{e8}me", &[1; 16], p).unwrap();
        let decomposed = derive_master_key("Cafe\u{301} cre\u{300}me", &[1; 16], p).unwrap();
        assert_eq!(composed.encryption, decomposed.encryption);
    }

    /// RFC 9106 §5.3 Argon2id test vector, run against the argon2 crate configured the
    /// same way as `derive_master_key` (Argon2id, v0x13), to pin algorithm and version.
    #[test]
    fn rfc9106_argon2id_vector() {
        let params = argon2::ParamsBuilder::new()
            .m_cost(32)
            .t_cost(3)
            .p_cost(4)
            .output_len(32)
            .data(argon2::AssociatedData::new(&[4u8; 12]).unwrap())
            .build()
            .unwrap();
        let argon = Argon2::new_with_secret(&[3u8; 8], Algorithm::Argon2id, Version::V0x13, params)
            .unwrap();
        let mut out = [0u8; 32];
        argon
            .hash_password_into(&[1u8; 32], &[2u8; 16], &mut out)
            .unwrap();
        assert_eq!(
            crate::hex(&out),
            "0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659"
        );
    }
}

#[cfg(test)]
mod floor_tests {
    use super::KdfParams;

    #[test]
    fn floor() {
        assert!(KdfParams::default().meets_floor());
        assert!(!KdfParams::insecure_for_tests().meets_floor());
    }
}
