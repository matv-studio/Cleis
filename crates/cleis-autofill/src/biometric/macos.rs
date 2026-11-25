//! Touch ID (or the Mac's password) through LocalAuthentication; the key kept in the login
//! keychain under the service "Cleis".
#![allow(unsafe_code)]

use block2::RcBlock;
use objc2::runtime::Bool;
use objc2_foundation::{NSError, NSString};
use objc2_local_authentication::{LAContext, LAPolicy};
use std::sync::mpsc;

pub const NAME: &str = "Touch ID";
const SERVICE: &str = "Cleis";

pub fn available() -> bool {
    // SAFETY: a fresh context, queried on this thread.
    unsafe {
        let ctx = LAContext::new();
        ctx.canEvaluatePolicy_error(LAPolicy::DeviceOwnerAuthenticationWithBiometrics)
            .is_ok()
    }
}

pub fn verify(reason: &str, _window: Option<crate::WindowRef>) -> Result<(), String> {
    let (tx, rx) = mpsc::channel::<Result<(), String>>();
    let reply = RcBlock::new(move |ok: Bool, err: *mut NSError| {
        let r = if ok.as_bool() {
            Ok(())
        } else if err.is_null() {
            Err("Not verified".to_owned())
        } else {
            // SAFETY: LocalAuthentication passes a valid NSError when it fails.
            Err(unsafe { (*err).localizedDescription() }.to_string())
        };
        let _ = tx.send(r);
    });
    // SAFETY: the context lives until the reply came (we wait for it below).
    let ctx = unsafe { LAContext::new() };
    unsafe {
        ctx.evaluatePolicy_localizedReason_reply(
            LAPolicy::DeviceOwnerAuthentication,
            &NSString::from_str(reason),
            &reply,
        );
    }
    rx.recv().map_err(|e| e.to_string())?
}

pub fn protect(id: &str, secret: &[u8]) -> Result<Vec<u8>, String> {
    security_framework::passwords::set_generic_password(SERVICE, id, secret)
        .map_err(|e| e.to_string())?;
    // The file only says the key is in the keychain.
    Ok(b"keychain".to_vec())
}

pub fn unprotect(id: &str, _stored: &[u8]) -> Result<Vec<u8>, String> {
    security_framework::passwords::get_generic_password(SERVICE, id).map_err(|e| e.to_string())
}

pub fn forget(id: &str) {
    let _ = security_framework::passwords::delete_generic_password(SERVICE, id);
}
