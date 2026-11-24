use thiserror::Error;

/// Errors are deliberately coarse: an attacker learns nothing from which check failed.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CryptoError {
    #[error("decryption failed")]
    Decrypt,
    #[error("unsupported format version {0}")]
    Version(u8),
    #[error("malformed input")]
    Malformed,
    #[error("invalid signature")]
    Signature,
    #[error("key derivation failed")]
    Kdf,
    #[error("system random generator unavailable")]
    Random,
}
