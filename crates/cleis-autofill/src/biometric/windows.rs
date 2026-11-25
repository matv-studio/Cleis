//! Windows Hello through `UserConsentVerifier` (over the app's window when Windows allows it),
//! the key sealed with DPAPI for the signed-in user.
#![allow(unsafe_code)]

use windows::Security::Credentials::UI::{
    UserConsentVerificationResult, UserConsentVerifier, UserConsentVerifierAvailability,
};
use windows::Win32::Foundation::{HLOCAL, HWND, LocalFree};
use windows::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};
use windows::Win32::System::WinRT::IUserConsentVerifierInterop;
use windows::core::{HSTRING, factory};
use windows_future::IAsyncOperation;

pub const NAME: &str = "Windows Hello";

pub fn available() -> bool {
    UserConsentVerifier::CheckAvailabilityAsync()
        .and_then(|op| op.join())
        .is_ok_and(|a| a == UserConsentVerifierAvailability::Available)
}

pub fn verify(reason: &str, window: Option<crate::WindowRef>) -> Result<(), String> {
    let msg = HSTRING::from(reason);
    // Over the app's window (Windows 11); otherwise the plain call (the prompt may open behind).
    let by_window = window.and_then(|w| {
        let interop = factory::<UserConsentVerifier, IUserConsentVerifierInterop>().ok()?;
        // SAFETY: `w` is the handle of the app's own top-level window.
        unsafe {
            interop
                .RequestVerificationForWindowAsync::<IAsyncOperation<UserConsentVerificationResult>>(
                    HWND(w.0 as *mut core::ffi::c_void),
                    &msg,
                )
                .ok()
        }
    });
    let op = match by_window {
        Some(op) => op,
        None => UserConsentVerifier::RequestVerificationAsync(&msg).map_err(|e| e.to_string())?,
    };
    match op.join().map_err(|e| e.to_string())? {
        UserConsentVerificationResult::Verified => Ok(()),
        UserConsentVerificationResult::Canceled => Err("Canceled".into()),
        UserConsentVerificationResult::DeviceNotPresent
        | UserConsentVerificationResult::NotConfiguredForUser => {
            Err("Windows Hello is not set up".into())
        }
        UserConsentVerificationResult::DisabledByPolicy => {
            Err("Windows Hello is turned off".into())
        }
        UserConsentVerificationResult::RetriesExhausted => Err("Too many tries".into()),
        _ => Err("Windows Hello is busy".into()),
    }
}

fn blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
    CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    }
}

/// Copies and frees a blob returned by DPAPI.
fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
    // SAFETY: DPAPI filled `out` with `cbData` bytes it allocated with LocalAlloc.
    unsafe {
        let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(out.pbData as *mut core::ffi::c_void)));
        v
    }
}

pub fn protect(_id: &str, secret: &[u8]) -> Result<Vec<u8>, String> {
    let input = blob(secret);
    let mut out = CRYPT_INTEGER_BLOB::default();
    // SAFETY: the input blob points at `secret`, alive for the call; DPAPI allocates `out`.
    unsafe {
        CryptProtectData(
            &input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out,
        )
    }
    .map_err(|e| e.to_string())?;
    Ok(take(out))
}

pub fn unprotect(_id: &str, stored: &[u8]) -> Result<Vec<u8>, String> {
    let input = blob(stored);
    let mut out = CRYPT_INTEGER_BLOB::default();
    // SAFETY: as in `protect`.
    unsafe {
        CryptUnprotectData(
            &input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out,
        )
    }
    .map_err(|e| e.to_string())?;
    Ok(take(out))
}

pub fn forget(_id: &str) {}
