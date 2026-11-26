//! An organization without a server (D-78, "Shared folder"): each app runs this server's API on
//! its own copy of the organization, rebuilt from the operations its members signed in a shared
//! folder (SharePoint, Google shared drive, NAS share, SFTP).
//!
//! An operation is an API request with who made it, when, and a seed: replaying it here with
//! the same clock and the same "random" numbers gives the same result on every copy, and the
//! request goes through the same checks of rights as on a server, so an operation a member had
//! no right to make is refused on every copy alike.
//!
//! What must stay secret never comes from the seed (everyone with the folder reads it): an
//! invitation's code is made by the inviting app, which hands the replay only its hash and the
//! public key derived from it (`invite_proof`); the new member proves the code with a signature.

use crate::{App, Peer};
use axum::body::Body;
use axum::http::Request;
use cleis_core::MemberId;
use cleis_crypto::{CryptoError, SigningKeyPair, sha256};
use std::cell::RefCell;
use std::sync::Arc;
use tower::ServiceExt;

/// Who replays, when, with which numbers.
#[derive(Clone, Debug)]
pub struct Replay {
    /// The member who made the operation (`None`: a new member joining).
    pub member: Option<MemberId>,
    pub device: String,
    /// The operation's time (seconds).
    pub at: i64,
    /// Source of the operation's "random" numbers.
    pub seed: [u8; 32],
}

struct Current {
    replay: Replay,
    counter: u64,
}

thread_local! {
    static CURRENT: RefCell<Option<Current>> = const { RefCell::new(None) };
}

/// The time: the operation's while replaying, the clock's otherwise.
pub fn now() -> i64 {
    CURRENT
        .with(|c| c.borrow().as_ref().map(|c| c.replay.at))
        .unwrap_or_else(cleis_core::now)
}

/// Random bytes: drawn from the operation's seed while replaying (SHA-256 of seed and counter),
/// from the system otherwise.
pub fn random_bytes<const N: usize>() -> Result<[u8; N], CryptoError> {
    let drawn = CURRENT.with(|c| {
        let mut c = c.borrow_mut();
        let c = c.as_mut()?;
        let mut out = [0u8; N];
        let mut filled = 0;
        while filled < N {
            c.counter += 1;
            let block = sha256(&[&c.replay.seed[..], &c.counter.to_be_bytes()[..]].concat());
            let n = (N - filled).min(32);
            out[filled..filled + n].copy_from_slice(&block[..n]);
            filled += n;
        }
        Some(out)
    });
    match drawn {
        Some(b) => Ok(b),
        None => cleis_crypto::random_bytes(),
    }
}

/// A random id (UUID v4) from `random_bytes`.
pub fn uuid() -> uuid::Uuid {
    let b: [u8; 16] = random_bytes().unwrap_or([0; 16]);
    uuid::Builder::from_random_bytes(b).into_uuid()
}

/// A time-ordered id (UUID v7) from `now` and `random_bytes`.
pub fn uuid_v7() -> uuid::Uuid {
    let b: [u8; 10] = random_bytes().unwrap_or([0; 10]);
    let ms = u64::try_from(now()).unwrap_or(0) * 1000;
    uuid::Builder::from_unix_timestamp_millis(ms, &b).into_uuid()
}

/// While replaying: the member the operation is from (and their device).
pub fn replaying() -> Option<(Option<MemberId>, String)> {
    CURRENT.with(|c| {
        c.borrow()
            .as_ref()
            .map(|c| (c.replay.member, c.replay.device.clone()))
    })
}

/// Domain of the signature that proves an invitation code (D-78).
pub const INVITE_PROOF_DOMAIN: &str = "cleis/invite-proof/v1";

/// The key an invitation code gives: its public half is kept with the invitation, its private
/// half signs the new member's request.
pub fn invite_proof(token: &[u8]) -> SigningKeyPair {
    SigningKeyPair::from_secret_bytes(sha256(&[INVITE_PROOF_DOMAIN.as_bytes(), token].concat()))
}

/// An answer of the API.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Runs one request on `app` as `replay` says (no network, no TLS, no session).
pub fn call(app: &Arc<App>, replay: Replay, method: &str, path: &str, body: Vec<u8>) -> Answer {
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            return Answer {
                status: 500,
                body: e.to_string().into_bytes(),
            };
        }
    };
    CURRENT.with(|c| {
        *c.borrow_mut() = Some(Current { replay, counter: 0 });
    });
    let router = crate::api::router(app.clone());
    let answer = rt.block_on(async move {
        let mut req = Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(body))
            .expect("request");
        req.extensions_mut()
            .insert(Peer(std::net::SocketAddr::from(([127, 0, 0, 1], 0))));
        let Ok(r) = router.oneshot(req).await;
        let status = r.status().as_u16();
        let body = axum::body::to_bytes(r.into_body(), 256 * 1024 * 1024)
            .await
            .map(|b| b.to_vec())
            .unwrap_or_default();
        Answer { status, body }
    });
    CURRENT.with(|c| *c.borrow_mut() = None);
    answer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replayed_numbers_repeat() {
        let r = Replay {
            member: None,
            device: "PC".into(),
            at: 1_790_000_000,
            seed: [7; 32],
        };
        let draw = || {
            CURRENT.with(|c| {
                *c.borrow_mut() = Some(Current {
                    replay: r.clone(),
                    counter: 0,
                })
            });
            let out = (now(), random_bytes::<40>().unwrap(), uuid(), uuid_v7());
            CURRENT.with(|c| *c.borrow_mut() = None);
            out
        };
        let (a, b) = (draw(), draw());
        assert_eq!(a, b);
        assert_eq!(a.0, 1_790_000_000);
        assert_ne!(a.1[..32], a.1[8..40]);
        // Outside a replay: the system's.
        assert_ne!(random_bytes::<32>().unwrap(), random_bytes::<32>().unwrap());
    }
}
