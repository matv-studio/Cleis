use crate::CryptoError;
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Fills a fixed-size array from the operating system's random generator.
pub fn random_bytes<const N: usize>() -> Result<[u8; N], CryptoError> {
    let mut out = [0u8; N];
    getrandom::fill(&mut out).map_err(|_| CryptoError::Random)?;
    Ok(out)
}

/// A 256-bit symmetric key (space key, item key, backup key…). Wiped from memory on drop.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SymmetricKey([u8; 32]);

impl SymmetricKey {
    pub fn generate() -> Result<Self, CryptoError> {
        Ok(Self(random_bytes()?))
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self, CryptoError> {
        let arr: [u8; 32] = bytes.try_into().map_err(|_| CryptoError::Malformed)?;
        Ok(Self(arr))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl PartialEq for SymmetricKey {
    fn eq(&self, other: &Self) -> bool {
        self.0.ct_eq(&other.0).into()
    }
}
impl Eq for SymmetricKey {}

impl std::fmt::Debug for SymmetricKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SymmetricKey(…)")
    }
}
