//! Master-password key derivation: Argon2id (RFC 9106), then HKDF-SHA256 to split
//! independent sub-keys (encryption key, server authentication key).

use crate::{CryptoError, SymmetricKey};
use argon2::{Algorithm, Argon2, Params, Version};
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
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
    let mut stretched = Zeroizing::new([0u8; 32]);
    argon
        .hash_password_into(password.as_bytes(), salt, stretched.as_mut())
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

#[cfg(test)]
mod tests {
    use super::*;

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
