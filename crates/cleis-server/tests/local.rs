//! Shared folder (D-78): the server's API replayed in the app. Two copies that replay the same
//! operations from the same starting file end in the same state; rights are checked as on a
//! server; an invitation is proven without its code ever being in an operation.

use cleis_core::org::*;
use cleis_crypto::{KeyPair, SigningKeyPair, sha256};
use cleis_server::App;
use cleis_server::local::{Replay, call, invite_proof};
use std::path::Path;
use std::sync::Arc;

struct Op {
    member: Option<cleis_core::MemberId>,
    at: i64,
    seed: [u8; 32],
    path: &'static str,
    body: Vec<u8>,
}

fn run(app: &Arc<App>, op: &Op) -> cleis_server::local::Answer {
    call(
        app,
        Replay {
            member: op.member,
            device: "PC".into(),
            at: op.at,
            seed: op.seed,
        },
        "POST",
        op.path,
        op.body.clone(),
    )
}

fn state(app: &Arc<App>, member: cleis_core::MemberId) -> State {
    let a = call(
        app,
        Replay {
            member: Some(member),
            device: "PC".into(),
            at: 1_800_000_000,
            seed: [0; 32],
        },
        "GET",
        "/v1/state",
        vec![],
    );
    assert_eq!(a.status, 200, "{}", String::from_utf8_lossy(&a.body));
    serde_json::from_slice(&a.body).unwrap()
}

fn copy_db(from: &Arc<App>, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    let conn = from.db.lock().unwrap();
    conn.execute(
        "VACUUM INTO ?1",
        [to.join(cleis_server::DB_FILE).to_string_lossy().as_ref()],
    )
    .unwrap();
}

#[test]
fn copies_agree_and_rights_hold() {
    let dir = std::env::temp_dir().join(format!("cleis-local-test-{}", uuid::Uuid::new_v4()));
    let a_dir = dir.join("a");
    let code = App::init(&a_dir, "Example Studio", &[]).unwrap();
    let a = App::open(&a_dir).unwrap();

    // Lea joins with the first code (on her copy), then the file becomes everyone's start.
    let lea_account = KeyPair::generate().unwrap();
    let lea_sign = SigningKeyPair::generate().unwrap();
    let inv = Invitation::parse(&code).unwrap();
    let mut join = JoinRequest {
        token: inv.token.to_vec(),
        name: "Lea Martin".into(),
        email: "lea@example-studio.com".into(),
        device: "PC".into(),
        account_public: lea_account.public().0,
        verify_key: lea_sign.verifying_key().0,
        signature: [0; 64],
        proof: vec![],
    };
    join.signature = lea_sign.sign(JOIN_DOMAIN, &join.signed_bytes());
    let r = run(
        &a,
        &Op {
            member: None,
            at: 1_790_000_000,
            seed: [1; 32],
            path: "/v1/join",
            body: serde_json::to_vec(&join).unwrap(),
        },
    );
    assert_eq!(r.status, 200, "{}", String::from_utf8_lossy(&r.body));
    let lea = serde_json::from_slice::<JoinReply>(&r.body).unwrap().member;
    let b_dir = dir.join("b");
    copy_db(&a, &b_dir);
    let b = App::open(&b_dir).unwrap();

    // Lea invites Hugo: only the hash and the proof key go into the operation.
    let token = [42u8; 10];
    let mut policy = state(&a, lea).policy;
    policy.min_length = 16;
    let invite = NewInvite {
        name: "Hugo Bernard".into(),
        email: "hugo@example-studio.com".into(),
        role: "member".into(),
        spaces: vec![],
        address: String::new(),
        secret: Some(InviteSecret {
            token_hash: sha256(&token),
            proof_key: invite_proof(&token).verifying_key().0,
        }),
    };
    let hugo_account = KeyPair::generate().unwrap();
    let hugo_sign = SigningKeyPair::generate().unwrap();
    let mut hjoin = JoinRequest {
        token: vec![],
        name: String::new(),
        email: String::new(),
        device: "Laptop".into(),
        account_public: hugo_account.public().0,
        verify_key: hugo_sign.verifying_key().0,
        signature: [0; 64],
        proof: vec![],
    };
    hjoin.signature = hugo_sign.sign(JOIN_DOMAIN, &hjoin.signed_bytes());
    hjoin.proof = invite_proof(&token)
        .sign(
            cleis_server::local::INVITE_PROOF_DOMAIN,
            &hjoin.account_public,
        )
        .to_vec();
    let ops = [
        Op {
            member: Some(lea),
            at: 1_790_000_100,
            seed: [2; 32],
            path: "/v1/policy",
            body: serde_json::to_vec(&policy).unwrap(),
        },
        Op {
            member: Some(lea),
            at: 1_790_000_200,
            seed: [3; 32],
            path: "/v1/invites",
            body: serde_json::to_vec(&invite).unwrap(),
        },
        Op {
            member: None,
            at: 1_790_000_300,
            seed: [4; 32],
            path: "/v1/join",
            body: serde_json::to_vec(&hjoin).unwrap(),
        },
    ];
    let mut hugo = None;
    for op in &ops {
        let (ra, rb) = (run(&a, op), run(&b, op));
        assert_eq!(ra, rb, "both copies answer alike");
        assert_eq!(ra.status, 200, "{}", String::from_utf8_lossy(&ra.body));
        if op.path == "/v1/join" {
            hugo = Some(
                serde_json::from_slice::<JoinReply>(&ra.body)
                    .unwrap()
                    .member,
            );
        }
    }
    let hugo = hugo.unwrap();
    let (sa, sb) = (state(&a, lea), state(&b, lea));
    assert_eq!(
        serde_json::to_value(&sa).unwrap(),
        serde_json::to_value(&sb).unwrap()
    );
    assert_eq!(sa.policy.min_length, 16);
    assert!(
        sa.members
            .iter()
            .any(|m| m.id == hugo && m.name == "Hugo Bernard")
    );
    assert!(sa.invites.is_empty(), "the invitation is used");

    // A member's operation beyond their rights is refused on every copy.
    let mut p = sa.policy.clone();
    p.min_length = 8;
    let bad = Op {
        member: Some(hugo),
        at: 1_790_000_400,
        seed: [5; 32],
        path: "/v1/policy",
        body: serde_json::to_vec(&p).unwrap(),
    };
    assert_eq!(run(&a, &bad).status, 403);
    assert_eq!(run(&b, &bad).status, 403);
    // A proof made with another code joins nobody.
    let mut other = hjoin.clone();
    let stranger = KeyPair::generate().unwrap();
    other.account_public = stranger.public().0;
    other.proof = invite_proof(&[9u8; 10])
        .sign(
            cleis_server::local::INVITE_PROOF_DOMAIN,
            &other.account_public,
        )
        .to_vec();
    let r = run(
        &a,
        &Op {
            member: None,
            at: 1_790_000_500,
            seed: [6; 32],
            path: "/v1/join",
            body: serde_json::to_vec(&other).unwrap(),
        },
    );
    assert_eq!(r.status, 401);
    let _ = std::fs::remove_dir_all(&dir);
}
