//! Cryptographic building blocks for Cleis.
//!
//! No primitive is implemented here: everything delegates to audited, widely used
//! crates (RustCrypto `aes-gcm`, `argon2`, `sha2`, `hkdf`; dalek `x25519-dalek`,
//! `ed25519-dalek`; `ring` for RS256 checks of identity-provider tokens). This crate only fixes the formats and the ways they are combined,
//! so the rest of the application cannot misuse them. See `THIRD_PARTY.md`.
//!
//! Formats (all versioned by a leading byte so they can evolve):
//! - sealed data: `0x01 | nonce(12) | ciphertext | tag(16)` — AES-256-GCM, caller-supplied AAD.
//! - wrapped key: `0x01 | ephemeral X25519 public(32) | sealed key` — X25519 + HKDF-SHA256 + AES-256-GCM.
//! - signature: Ed25519 (64 bytes) over a domain-separated message.
//! - sealed to a public key: `0x01 | len u16 | wrapped key | sealed data` (`seal_to`).

#![forbid(unsafe_code)]

mod aead;
mod error;
mod kdf;
mod keys;
#[cfg(feature = "rs256")]
mod rsa;
mod sign;
mod wrap;

pub use aead::{open, seal};
pub use error::CryptoError;
pub use kdf::{KdfParams, MasterKey, derive_master_key};
pub use keys::{SymmetricKey, random_bytes};
#[cfg(feature = "rs256")]
pub use rsa::verify_rs256;
pub use sign::{SigningKeyPair, VerifyingKey, verify};
pub use wrap::{KeyPair, PublicKey, open_from, seal_to, unwrap_key, wrap_key};

/// SHA-256 of `data`, used for content hashes and hash chains.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::digest(data).into()
}

/// Lower-case hexadecimal encoding, for fingerprints and logs.
pub fn hex(bytes: &[u8]) -> String {
    data_encoding::HEXLOWER.encode(bytes)
}

/// Short, human-comparable fingerprint of a public key (`SHA256:abcd·ef01·…`).
pub fn fingerprint(public: &[u8]) -> String {
    fingerprint_of_hash(&sha256(public))
}

/// The same display for a SHA-256 already computed (a pinned certificate's hash).
pub fn fingerprint_of_hash(hash: &[u8]) -> String {
    let h = hex(hash);
    let groups: Vec<&str> = (0..4).filter_map(|i| h.get(i * 4..i * 4 + 4)).collect();
    format!("SHA256:{}", groups.join("·"))
}
