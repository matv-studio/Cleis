//! Key wrapping for a recipient: a space key is given to each member by encrypting it
//! to that member's X25519 public key (ephemeral-static Diffie-Hellman, HKDF-SHA256,
//! AES-256-GCM — the same shape as HPKE base mode / libsodium sealed boxes).

use crate::{CryptoError, SymmetricKey, aead, random_bytes};
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use x25519_dalek::StaticSecret;
use zeroize::Zeroizing;

const VERSION: u8 = 1;
const INFO: &[u8] = b"cleis/wrap/v1";

/// A member's or a device's X25519 public key.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PublicKey(pub [u8; 32]);

impl std::fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PublicKey({})", crate::fingerprint(&self.0))
    }
}

/// X25519 key pair. The secret half is wiped on drop by `x25519-dalek`.
pub struct KeyPair {
    secret: StaticSecret,
    public: PublicKey,
}

impl KeyPair {
    pub fn generate() -> Result<Self, CryptoError> {
        Ok(Self::from_secret_bytes(random_bytes()?))
    }

    pub fn from_secret_bytes(bytes: [u8; 32]) -> Self {
        let secret = StaticSecret::from(bytes);
        let public = PublicKey(x25519_dalek::PublicKey::from(&secret).to_bytes());
        Self { secret, public }
    }

    pub fn public(&self) -> PublicKey {
        self.public
    }

    /// Secret bytes, for storage encrypted under the master key only.
    pub fn secret_bytes(&self) -> Zeroizing<[u8; 32]> {
        Zeroizing::new(self.secret.to_bytes())
    }
}

fn derive(
    shared: &[u8; 32],
    eph: &[u8; 32],
    recipient: &[u8; 32],
) -> Result<SymmetricKey, CryptoError> {
    let mut salt = [0u8; 64];
    salt[..32].copy_from_slice(eph);
    salt[32..].copy_from_slice(recipient);
    let hk = Hkdf::<Sha256>::new(Some(&salt), shared);
    let mut okm = Zeroizing::new([0u8; 32]);
    hk.expand(INFO, okm.as_mut())
        .map_err(|_| CryptoError::Kdf)?;
    Ok(SymmetricKey::from_bytes(*okm))
}

/// Encrypts `key` so that only the holder of `recipient`'s secret can recover it.
/// `context` (e.g. the space id) is authenticated.
pub fn wrap_key(
    recipient: &PublicKey,
    key: &SymmetricKey,
    context: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let eph = KeyPair::generate()?;
    let shared = eph
        .secret
        .diffie_hellman(&x25519_dalek::PublicKey::from(recipient.0));
    if !shared.was_contributory() {
        return Err(CryptoError::Malformed);
    }
    let kek = derive(shared.as_bytes(), &eph.public.0, &recipient.0)?;
    let sealed = aead::seal(&kek, context, key.as_bytes())?;
    let mut out = Vec::with_capacity(1 + 32 + sealed.len());
    out.push(VERSION);
    out.extend_from_slice(&eph.public.0);
    out.extend_from_slice(&sealed);
    Ok(out)
}

/// Recovers a key produced by [`wrap_key`].
pub fn unwrap_key(
    own: &KeyPair,
    wrapped: &[u8],
    context: &[u8],
) -> Result<SymmetricKey, CryptoError> {
    let (&version, rest) = wrapped.split_first().ok_or(CryptoError::Malformed)?;
    if version != VERSION {
        return Err(CryptoError::Version(version));
    }
    if rest.len() < 32 {
        return Err(CryptoError::Malformed);
    }
    let (eph_bytes, sealed) = rest.split_at(32);
    let eph: [u8; 32] = eph_bytes.try_into().map_err(|_| CryptoError::Malformed)?;
    let shared = own
        .secret
        .diffie_hellman(&x25519_dalek::PublicKey::from(eph));
    if !shared.was_contributory() {
        return Err(CryptoError::Decrypt);
    }
    let kek = derive(shared.as_bytes(), &eph, &own.public.0)?;
    let raw = Zeroizing::new(aead::open(&kek, context, sealed)?);
    SymmetricKey::from_slice(&raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_unwrap() {
        let alice = KeyPair::generate().unwrap();
        let space = SymmetricKey::generate().unwrap();
        let w = wrap_key(&alice.public(), &space, b"space:general").unwrap();
        assert_eq!(unwrap_key(&alice, &w, b"space:general").unwrap(), space);
    }

    #[test]
    fn other_recipient_or_context_fails() {
        let alice = KeyPair::generate().unwrap();
        let bob = KeyPair::generate().unwrap();
        let space = SymmetricKey::generate().unwrap();
        let w = wrap_key(&alice.public(), &space, b"s1").unwrap();
        assert!(unwrap_key(&bob, &w, b"s1").is_err());
        assert!(unwrap_key(&alice, &w, b"s2").is_err());
    }

    #[test]
    fn rejects_low_order_point() {
        let space = SymmetricKey::generate().unwrap();
        assert!(wrap_key(&PublicKey([0u8; 32]), &space, b"").is_err());
    }

    /// RFC 7748 §6.1 Diffie-Hellman vector.
    #[test]
    fn rfc7748_vector() {
        let a = data_encoding::HEXLOWER
            .decode(b"77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")
            .unwrap();
        let b_pub = data_encoding::HEXLOWER
            .decode(b"de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f")
            .unwrap();
        let alice = KeyPair::from_secret_bytes(a.try_into().unwrap());
        assert_eq!(
            crate::hex(&alice.public().0),
            "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a"
        );
        let shared = alice.secret.diffie_hellman(&x25519_dalek::PublicKey::from(
            <[u8; 32]>::try_from(b_pub).unwrap(),
        ));
        assert_eq!(
            crate::hex(shared.as_bytes()),
            "4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742"
        );
    }
}

const SEALED_TO_VERSION: u8 = 1;

/// Seals `plain` so that only the holder of `recipient`'s private key can open it: a fresh
/// symmetric key is wrapped to `recipient` ([`wrap_key`]) and seals the data ([`aead::seal`]).
/// Format: `0x01 | len(wrapped) u16 BE | wrapped key | sealed data`. Used for activity-log details
/// readable by the administrators only, and for the recovery copies of keys.
pub fn seal_to(
    recipient: &PublicKey,
    context: &[u8],
    plain: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let key = SymmetricKey::generate()?;
    let wrapped = wrap_key(recipient, &key, context)?;
    let sealed = aead::seal(&key, context, plain)?;
    let len = u16::try_from(wrapped.len()).map_err(|_| CryptoError::Malformed)?;
    let mut out = Vec::with_capacity(3 + wrapped.len() + sealed.len());
    out.push(SEALED_TO_VERSION);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&wrapped);
    out.extend_from_slice(&sealed);
    Ok(out)
}

/// Opens what [`seal_to`] sealed for `own`.
pub fn open_from(
    own: &KeyPair,
    context: &[u8],
    blob: &[u8],
) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    let (&version, rest) = blob.split_first().ok_or(CryptoError::Malformed)?;
    if version != SEALED_TO_VERSION {
        return Err(CryptoError::Version(version));
    }
    if rest.len() < 2 {
        return Err(CryptoError::Malformed);
    }
    let len = usize::from(u16::from_be_bytes([rest[0], rest[1]]));
    let rest = &rest[2..];
    if rest.len() < len {
        return Err(CryptoError::Malformed);
    }
    let (wrapped, sealed) = rest.split_at(len);
    let key = unwrap_key(own, wrapped, context)?;
    Ok(Zeroizing::new(aead::open(&key, context, sealed)?))
}

#[cfg(test)]
mod sealed_to_tests {
    use super::*;

    #[test]
    fn seal_to_round_trip_and_binding() {
        let admin = KeyPair::generate().unwrap();
        let other = KeyPair::generate().unwrap();
        let blob = seal_to(&admin.public(), b"cleis/log", b"Copied Figma").unwrap();
        assert_eq!(
            &open_from(&admin, b"cleis/log", &blob).unwrap()[..],
            b"Copied Figma"
        );
        // Another key, another context, a changed byte: refused.
        assert!(open_from(&other, b"cleis/log", &blob).is_err());
        assert!(open_from(&admin, b"cleis/other", &blob).is_err());
        let mut bad = blob.clone();
        *bad.last_mut().unwrap() ^= 1;
        assert!(open_from(&admin, b"cleis/log", &bad).is_err());
        assert!(open_from(&admin, b"cleis/log", &blob[..10]).is_err());
    }
}
