//! Ed25519 signatures (RFC 8032) with domain separation, used for the activity log,
//! signed writes in shared-folder mode and device identities.

use crate::{CryptoError, random_bytes};
use ed25519_dalek::Signer;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VerifyingKey(pub [u8; 32]);

impl std::fmt::Debug for VerifyingKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VerifyingKey({})", crate::fingerprint(&self.0))
    }
}

pub struct SigningKeyPair {
    inner: ed25519_dalek::SigningKey,
}

fn framed(domain: &str, msg: &[u8]) -> Vec<u8> {
    let mut m = Vec::with_capacity(domain.len() + 1 + msg.len());
    m.extend_from_slice(domain.as_bytes());
    m.push(0);
    m.extend_from_slice(msg);
    m
}

impl SigningKeyPair {
    pub fn generate() -> Result<Self, CryptoError> {
        Ok(Self::from_secret_bytes(random_bytes()?))
    }

    pub fn from_secret_bytes(bytes: [u8; 32]) -> Self {
        Self {
            inner: ed25519_dalek::SigningKey::from_bytes(&bytes),
        }
    }

    pub fn secret_bytes(&self) -> Zeroizing<[u8; 32]> {
        Zeroizing::new(self.inner.to_bytes())
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        VerifyingKey(self.inner.verifying_key().to_bytes())
    }

    /// Signs `msg` under a domain tag such as `"cleis/log/v1"`, so a signature made for
    /// one purpose can never be replayed for another.
    pub fn sign(&self, domain: &str, msg: &[u8]) -> [u8; 64] {
        self.inner.sign(&framed(domain, msg)).to_bytes()
    }
}

pub fn verify(
    key: &VerifyingKey,
    domain: &str,
    msg: &[u8],
    sig: &[u8; 64],
) -> Result<(), CryptoError> {
    let vk = ed25519_dalek::VerifyingKey::from_bytes(&key.0).map_err(|_| CryptoError::Signature)?;
    vk.verify_strict(
        &framed(domain, msg),
        &ed25519_dalek::Signature::from_bytes(sig),
    )
    .map_err(|_| CryptoError::Signature)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_verify_and_domain_separation() {
        let k = SigningKeyPair::generate().unwrap();
        let s = k.sign("cleis/log/v1", b"event");
        assert!(verify(&k.verifying_key(), "cleis/log/v1", b"event", &s).is_ok());
        assert!(verify(&k.verifying_key(), "cleis/write/v1", b"event", &s).is_err());
        assert!(verify(&k.verifying_key(), "cleis/log/v1", b"evenT", &s).is_err());
    }

    /// RFC 8032 §7.1 TEST 1 public key derivation.
    #[test]
    fn rfc8032_public_key() {
        let sk = data_encoding::HEXLOWER
            .decode(b"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
            .unwrap();
        let k = SigningKeyPair::from_secret_bytes(sk.try_into().unwrap());
        assert_eq!(
            crate::hex(&k.verifying_key().0),
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
        );
    }
}
