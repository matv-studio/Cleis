pub const NAME: &str = "";

pub fn available() -> bool {
    false
}

pub fn verify(_: &str, _: Option<crate::WindowRef>) -> Result<(), String> {
    Err("No biometric unlock on this system".into())
}

pub fn protect(_: &str, _: &[u8]) -> Result<Vec<u8>, String> {
    Err("No biometric unlock on this system".into())
}

pub fn unprotect(_: &str, _: &[u8]) -> Result<Vec<u8>, String> {
    Err("No biometric unlock on this system".into())
}

pub fn forget(_: &str) {}
