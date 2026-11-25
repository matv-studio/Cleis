//! Requests of the server protocol (`cleis_core::org`). Every request but `hello`, `join`,
//! `challenge` and `session` carries a session token; the caller's role decides what passes.

use crate::{ApiErr, App, INVITE_DAYS, Peer, SESSION_SECS, db};
use axum::extract::{DefaultBodyLimit, FromRequestParts, State};
use axum::http::{StatusCode, request::Parts};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use cleis_core::org::*;
use cleis_core::{MemberId, SpaceId, now};
use cleis_crypto::{VerifyingKey, hex, random_bytes, sha256, verify};
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::BTreeSet;
use std::sync::Arc;
use uuid::Uuid;

type Res<T> = Result<Json<T>, ApiErr>;

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/v1/hello", get(hello))
        .route("/v1/invite-info", post(invite_info))
        .route("/v1/join", post(join))
        .route("/v1/challenge", post(challenge))
        .route("/v1/session", post(session))
        .route("/v1/state", get(state))
        .route("/v1/pull", post(pull))
        .route("/v1/push", post(push))
        .route("/v1/spaces", post(create_space))
        .route("/v1/spaces/rekey", post(rekey))
        .route("/v1/grants", post(grants))
        .route("/v1/admin-keys", post(admin_keys))
        .route("/v1/members", post(update_member))
        .route("/v1/members/revoke", post(revoke))
        .route("/v1/members/transfer", post(transfer))
        .route("/v1/invites", post(invite))
        .route("/v1/invites/revoke", post(revoke_invite))
        .route("/v1/invites/renew", post(renew_invite))
        .route("/v1/roles", post(set_roles))
        .route("/v1/policy", post(set_policy))
        .route("/v1/log", post(report))
        .route("/v1/log/query", post(query_log))
        .route("/v1/log/verify", post(verify_log))
        .route("/v1/me", post(profile))
        .route("/v1/me/devices", post(devices))
        .route("/v1/me/devices/sign-out", post(sign_out))
        .route("/v1/spaces/update", post(update_space))
        .route("/v1/spaces/delete", post(delete_space))
        .route("/v1/me/code", post(code_setup))
        .route("/v1/me/code/confirm", post(code_confirm))
        .route("/v1/versions", post(versions))
        .route("/v1/recovery", post(recovery))
        .route("/v1/rotate", post(rotate))
        .route("/v1/backup", get(backup))
        .route("/v1/restore", post(restore))
        .layer(DefaultBodyLimit::max(256 * 1024 * 1024))
        .with_state(app)
}

// ---------- Members ----------

struct Member {
    id: MemberId,
    name: String,
    email: String,
    role: String,
    account_public: [u8; 32],
    verify_key: [u8; 32],
    export: bool,
    totp: Option<String>,
    totp_pending: Option<String>,
    totp_last: i64,
    last_sign_in: Option<i64>,
    last_device: String,
    revoked: Option<i64>,
}

const MEMBER_COLS: &str = "id, name, email, role, account_public, verify_key, export, totp, totp_pending, totp_last, last_sign_in, last_device, revoked";

fn member_from(r: &rusqlite::Row) -> rusqlite::Result<Member> {
    let id: String = r.get(0)?;
    let key = |i: usize| -> rusqlite::Result<[u8; 32]> {
        let v: Vec<u8> = r.get(i)?;
        Ok(v.try_into().unwrap_or([0; 32]))
    };
    Ok(Member {
        id: MemberId(id.parse().unwrap_or_default()),
        name: r.get(1)?,
        email: r.get(2)?,
        role: r.get(3)?,
        account_public: key(4)?,
        verify_key: key(5)?,
        export: r.get::<_, i64>(6)? != 0,
        totp: r.get(7)?,
        totp_pending: r.get(8)?,
        totp_last: r.get(9)?,
        last_sign_in: r.get(10)?,
        last_device: r.get(11)?,
        revoked: r.get(12)?,
    })
}

fn member(conn: &Connection, id: MemberId) -> rusqlite::Result<Option<Member>> {
    conn.query_row(
        &format!("SELECT {MEMBER_COLS} FROM members WHERE id = ?1"),
        [id.to_string()],
        member_from,
    )
    .optional()
}

fn members(conn: &Connection) -> rusqlite::Result<Vec<Member>> {
    let mut st = conn.prepare(&format!(
        "SELECT {MEMBER_COLS} FROM members WHERE revoked IS NULL ORDER BY created"
    ))?;
    st.query_map([], member_from)?.collect()
}

fn role_of(conn: &Connection, id: &str) -> Role {
    db::roles(conn)
        .into_iter()
        .find(|r| r.id == id)
        .unwrap_or(Role {
            id: id.into(),
            name: id.into(),
            perms: BTreeSet::new(),
        })
}

fn spaces_of(conn: &Connection, member: MemberId) -> rusqlite::Result<Vec<SpaceId>> {
    let mut st = conn.prepare("SELECT space FROM access WHERE member = ?1 ORDER BY space")?;
    st.query_map([member.to_string()], |r| r.get::<_, String>(0))?
        .map(|s| s.map(|s| SpaceId(s.parse().unwrap_or_default())))
        .collect()
}

fn view(conn: &Connection, m: &Member) -> rusqlite::Result<MemberView> {
    Ok(MemberView {
        id: m.id,
        name: m.name.clone(),
        email: m.email.clone(),
        role: m.role.clone(),
        spaces: spaces_of(conn, m.id)?,
        two_fa: m.totp.is_some(),
        export: m.export || is_admin_role(&m.role),
        last_sign_in: m.last_sign_in,
        last_device: m.last_device.clone(),
        revoked: m.revoked.is_some(),
        account_public: m.account_public,
        verify_key: m.verify_key,
    })
}

fn supers(conn: &Connection) -> rusqlite::Result<usize> {
    Ok(members(conn)?
        .iter()
        .filter(|m| m.role == SUPER_ADMIN)
        .count())
}

/// The member a request comes from.
pub struct Caller {
    pub id: MemberId,
    pub role: Role,
    pub device: String,
    pub setup_only: bool,
}

impl Caller {
    fn full(&self) -> Result<(), ApiErr> {
        if self.setup_only {
            Err(ApiErr(
                StatusCode::FORBIDDEN,
                "setup_code",
                "Set up your verification code first.".into(),
            ))
        } else {
            Ok(())
        }
    }

    fn can(&self, p: Perm) -> Result<(), ApiErr> {
        self.full()?;
        if self.role.can(p) {
            Ok(())
        } else {
            Err(ApiErr::forbidden("Your role does not allow this."))
        }
    }

    fn event(&self, action: Action) -> Event {
        Event {
            seq: 0,
            at: now(),
            member: Some(self.id),
            device: self.device.clone(),
            action,
            space: None,
            item: None,
            detail: None,
            note: String::new(),
        }
    }
}

impl FromRequestParts<Arc<App>> for Caller {
    type Rejection = ApiErr;

    async fn from_request_parts(parts: &mut Parts, app: &Arc<App>) -> Result<Self, ApiErr> {
        let token = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| ApiErr::unauthorized("unauthorized", "Sign in again."))?;
        let hash = sha256(token.as_bytes());
        let conn = app.db.lock().expect("database lock");
        let found: Option<(String, String, i64)> = conn
            .query_row(
                "SELECT s.member, s.device, s.setup_only FROM sessions s JOIN members m ON m.id = s.member
                 WHERE s.token_hash = ?1 AND s.expires > ?2 AND m.revoked IS NULL",
                params![&hash[..], now()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let (member, device, setup_only) =
            found.ok_or_else(|| ApiErr::unauthorized("unauthorized", "Sign in again."))?;
        conn.execute(
            "UPDATE sessions SET last = ?2 WHERE token_hash = ?1 AND last < ?2 - 60",
            params![&hash[..], now()],
        )?;
        let id = MemberId(member.parse().unwrap_or_default());
        let role_id: String =
            conn.query_row("SELECT role FROM members WHERE id = ?1", [&member], |r| {
                r.get(0)
            })?;
        Ok(Caller {
            id,
            role: role_of(&conn, &role_id),
            device,
            setup_only: setup_only != 0,
        })
    }
}

// ---------- Spaces ----------

fn space_exists(conn: &Connection, s: SpaceId) -> rusqlite::Result<bool> {
    conn.prepare("SELECT 1 FROM spaces WHERE id = ?1")?
        .exists([s.to_string()])
}

/// The member was given the space and holds its key.
fn has_key(conn: &Connection, s: SpaceId, m: MemberId) -> rusqlite::Result<bool> {
    conn.prepare("SELECT 1 FROM access WHERE space = ?1 AND member = ?2 AND wrapped IS NOT NULL")?
        .exists([s.to_string(), m.to_string()])
}

fn in_space(conn: &Connection, s: SpaceId, m: MemberId) -> rusqlite::Result<bool> {
    conn.prepare("SELECT 1 FROM access WHERE space = ?1 AND member = ?2")?
        .exists([s.to_string(), m.to_string()])
}

fn key_gen(conn: &Connection, s: SpaceId) -> rusqlite::Result<Option<u32>> {
    conn.query_row(
        "SELECT key_gen FROM spaces WHERE id = ?1",
        [s.to_string()],
        |r| r.get(0),
    )
    .optional()
}

fn mode_str(m: SpaceMode) -> &'static str {
    match m {
        SpaceMode::Standard => "standard",
        SpaceMode::EndToEnd => "end_to_end",
    }
}

fn org_info(conn: &Connection, cert: &[u8; 32]) -> rusqlite::Result<OrgInfo> {
    let id = db::meta_get(conn, "org_id")?
        .and_then(|b| Uuid::from_slice(&b).ok())
        .unwrap_or_default();
    Ok(OrgInfo {
        id,
        name: String::from_utf8(db::meta_get(conn, "org_name")?.unwrap_or_default())
            .unwrap_or_default(),
        cert: *cert,
        next_cert: db::meta_get(conn, "next_cert")?.map(|c| sha256(&c).to_vec()),
        org_public: db::meta_get(conn, "org_public")?,
        recovery_public: db::meta_get(conn, "recovery_public")?,
        recovery_saved: db::meta_json(conn, "recovery_saved"),
    })
}

// ---------- Sign-in ----------

async fn hello(State(app): State<Arc<App>>) -> Json<Hello> {
    let conn = app.db.lock().expect("database lock");
    Json(Hello {
        name: String::from_utf8(
            db::meta_get(&conn, "org_name")
                .ok()
                .flatten()
                .unwrap_or_default(),
        )
        .unwrap_or_default(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

pub fn create_invite(
    conn: &Connection,
    cert: &[u8; 32],
    name: &str,
    email: &str,
    role: &str,
    spaces: &[SpaceId],
    at: i64,
) -> rusqlite::Result<(Uuid, String)> {
    let token: [u8; 10] = random_bytes().expect("random");
    let id = Uuid::new_v4();
    conn.execute(
        "INSERT INTO invites(id, token_hash, name, email, role, spaces, created, expires) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id.to_string(),
            &sha256(&token)[..],
            name,
            email,
            role,
            serde_json::to_string(spaces).expect("ids"),
            at,
            at + INVITE_DAYS * 86_400
        ],
    )?;
    Ok((id, crate::code_for(token, cert)))
}

fn throttle(app: &App, peer: &Peer, failed: bool) -> Result<(), ApiErr> {
    if app.too_many_failures(peer.0.ip(), failed) {
        Err(ApiErr(
            StatusCode::TOO_MANY_REQUESTS,
            "throttled",
            "Too many attempts. Wait a few minutes.".into(),
        ))
    } else {
        Ok(())
    }
}

async fn invite_info(
    State(app): State<Arc<App>>,
    Extension(peer): Extension<Peer>,
    Json(req): Json<InviteLookup>,
) -> Res<InviteInfo> {
    throttle(&app, &peer, false)?;
    let conn = app.db.lock().expect("database lock");
    let found: Option<(String, String)> = conn
        .query_row(
            "SELECT name, email FROM invites WHERE token_hash = ?1 AND expires > ?2",
            params![&sha256(&req.token)[..], now()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((name, email)) = found else {
        drop(conn);
        throttle(&app, &peer, true)?;
        return Err(ApiErr::unauthorized(
            "expired",
            "This invitation code is not valid any more.",
        ));
    };
    Ok(Json(InviteInfo {
        org: String::from_utf8(db::meta_get(&conn, "org_name")?.unwrap_or_default())
            .unwrap_or_default(),
        name,
        email,
    }))
}

async fn join(
    State(app): State<Arc<App>>,
    Extension(peer): Extension<Peer>,
    Json(req): Json<JoinRequest>,
) -> Res<JoinReply> {
    throttle(&app, &peer, false)?;
    let conn = app.db.lock().expect("database lock");
    let t = now();
    let invite: Option<(String, String, String, String, String)> = conn
        .query_row(
            "SELECT id, name, email, role, spaces FROM invites WHERE token_hash = ?1 AND expires > ?2",
            params![&sha256(&req.token)[..], t],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    let Some((invite_id, name, email, role, spaces)) = invite else {
        drop(conn);
        throttle(&app, &peer, true)?;
        return Err(ApiErr::unauthorized(
            "expired",
            "This invitation code is not valid any more.",
        ));
    };
    if verify(
        &VerifyingKey(req.verify_key),
        JOIN_DOMAIN,
        &req.signed_bytes(),
        &req.signature,
    )
    .is_err()
    {
        return Err(ApiErr::bad("Signature does not match."));
    }
    let name = if name.trim().is_empty() {
        req.name.trim().to_owned()
    } else {
        name
    };
    let email = if email.trim().is_empty() {
        req.email.trim().to_owned()
    } else {
        email
    };
    if name.is_empty() {
        return Err(ApiErr::bad("Enter your name."));
    }
    if conn
        .prepare("SELECT 1 FROM members WHERE account_public = ?1")?
        .exists([&req.account_public[..]])?
    {
        return Err(ApiErr::bad(
            "This vault already belongs to the organization.",
        ));
    }
    let id = MemberId::new();
    let tx = conn.unchecked_transaction()?;
    conn.execute(
        "INSERT INTO members(id, name, email, role, account_public, verify_key, created, last_device) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id.to_string(), name, email, role, &req.account_public[..], &req.verify_key[..], t, req.device],
    )?;
    let spaces: Vec<SpaceId> = serde_json::from_str(&spaces).unwrap_or_default();
    for s in spaces {
        if space_exists(&conn, s)? {
            conn.execute(
                "INSERT OR IGNORE INTO access(space, member) VALUES (?1, ?2)",
                [s.to_string(), id.to_string()],
            )?;
        }
    }
    conn.execute("DELETE FROM invites WHERE id = ?1", [invite_id])?;
    app.record(
        &conn,
        Event {
            seq: 0,
            at: t,
            member: Some(id),
            device: req.device.clone(),
            action: Action::Joined,
            space: None,
            item: None,
            detail: None,
            note: name.clone(),
        },
    )?;
    tx.commit()?;
    Ok(Json(JoinReply {
        member: id,
        org: org_info(&conn, &app.cert)?,
    }))
}

async fn challenge(State(app): State<Arc<App>>) -> Res<Challenge> {
    let nonce: [u8; 32] = random_bytes().map_err(|_| ApiErr::bad("random"))?;
    let conn = app.db.lock().expect("database lock");
    conn.execute("DELETE FROM challenges WHERE expires < ?1", [now()])?;
    conn.execute(
        "INSERT INTO challenges(nonce, expires) VALUES (?1, ?2)",
        params![&nonce[..], now() + 60],
    )?;
    Ok(Json(Challenge {
        nonce: nonce.to_vec(),
    }))
}

/// Checks a 6-digit code against `secret`, refusing a code already used (`last` time step).
fn check_code(secret: &str, code: &str, last: i64, t: i64) -> Option<i64> {
    let digits: String = code.chars().filter(char::is_ascii_digit).collect();
    let totp = cleis_core::totp::Totp::from_base32(secret).ok()?;
    let step = t.div_euclid(30);
    [-1, 0, 1]
        .into_iter()
        .map(|d| step + d)
        .find(|s| *s > last && totp.code_at(s * 30) == digits)
}

fn random_token() -> Result<String, ApiErr> {
    let b: [u8; 32] = random_bytes().map_err(|_| ApiErr::bad("random"))?;
    Ok(hex(&b))
}

async fn session(
    State(app): State<Arc<App>>,
    Extension(peer): Extension<Peer>,
    Json(req): Json<SessionRequest>,
) -> Res<SessionReply> {
    throttle(&app, &peer, false)?;
    let conn = app.db.lock().expect("database lock");
    let t = now();
    if conn.execute(
        "DELETE FROM challenges WHERE nonce = ?1 AND expires >= ?2",
        params![&req.nonce[..], t],
    )? != 1
    {
        return Err(ApiErr::unauthorized("expired", "Try again."));
    }
    let Some(m) = member(&conn, req.member)? else {
        return Err(ApiErr::unauthorized(
            "revoked",
            "This account is not part of the organization.",
        ));
    };
    if m.revoked.is_some() {
        return Err(ApiErr::unauthorized(
            "revoked",
            "This account was removed from the organization.",
        ));
    }
    if verify(
        &VerifyingKey(m.verify_key),
        SIGN_IN_DOMAIN,
        &sign_in_message(&req.nonce, &app.cert),
        &req.signature,
    )
    .is_err()
    {
        drop(conn);
        throttle(&app, &peer, true)?;
        return Err(ApiErr::unauthorized("unauthorized", "Signature refused."));
    }
    let policy = db::policy(&conn);
    let mut remembered = None;
    if let Some(secret) = &m.totp {
        let known = match &req.remembered {
            Some(r) => conn
                .prepare("SELECT 1 FROM remembered WHERE token_hash = ?1 AND member = ?2 AND expires > ?3")?
                .exists(params![&sha256(r.as_bytes())[..], m.id.to_string(), t])?,
            None => false,
        };
        if !known {
            let Some(code) = &req.code else {
                return Err(ApiErr::unauthorized(
                    "code_required",
                    "Enter the code from your authenticator app.",
                ));
            };
            let Some(step) = check_code(secret, code, m.totp_last, t) else {
                drop(conn);
                throttle(&app, &peer, true)?;
                return Err(ApiErr::unauthorized("wrong_code", "Wrong code."));
            };
            conn.execute(
                "UPDATE members SET totp_last = ?2 WHERE id = ?1",
                params![m.id.to_string(), step],
            )?;
            if policy.remember_days > 0 {
                let r = random_token()?;
                conn.execute("DELETE FROM remembered WHERE expires < ?1", [t])?;
                conn.execute(
                    "INSERT INTO remembered(token_hash, member, expires, device) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        &sha256(r.as_bytes())[..],
                        m.id.to_string(),
                        t + i64::from(policy.remember_days) * 86_400,
                        req.device
                    ],
                )?;
                remembered = Some(r);
            }
        }
    }
    let setup_only = policy.code_required && m.totp.is_none();
    let token = random_token()?;
    conn.execute("DELETE FROM sessions WHERE expires < ?1", [t])?;
    conn.execute(
        "INSERT INTO sessions(token_hash, member, device, expires, setup_only, last) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![&sha256(token.as_bytes())[..], m.id.to_string(), req.device, t + SESSION_SECS, setup_only as i64, t],
    )?;
    conn.execute(
        "UPDATE members SET last_sign_in = ?2, last_device = ?3 WHERE id = ?1",
        params![m.id.to_string(), t, req.device],
    )?;
    app.record(
        &conn,
        Event {
            seq: 0,
            at: t,
            member: Some(m.id),
            device: req.device.clone(),
            action: Action::SignedIn,
            space: None,
            item: None,
            detail: None,
            note: String::new(),
        },
    )?;
    Ok(Json(SessionReply {
        token,
        remembered,
        setup_code: setup_only,
    }))
}

async fn code_setup(State(app): State<Arc<App>>, caller: Caller) -> Res<CodeSetup> {
    let key: [u8; 20] = random_bytes().map_err(|_| ApiErr::bad("random"))?;
    let secret = data_encoding::BASE32_NOPAD.encode(&key);
    let conn = app.db.lock().expect("database lock");
    conn.execute(
        "UPDATE members SET totp_pending = ?2 WHERE id = ?1",
        params![caller.id.to_string(), secret],
    )?;
    Ok(Json(CodeSetup { secret }))
}

async fn code_confirm(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(req): Json<CodeCheck>,
) -> Res<SessionReply> {
    let conn = app.db.lock().expect("database lock");
    let m = member(&conn, caller.id)?.ok_or_else(|| ApiErr::not_found("member"))?;
    let pending = m
        .totp_pending
        .ok_or_else(|| ApiErr::bad("Start the set-up again."))?;
    let step = check_code(&pending, &req.code, 0, now())
        .ok_or_else(|| ApiErr::unauthorized("wrong_code", "Wrong code."))?;
    conn.execute(
        "UPDATE members SET totp = totp_pending, totp_pending = NULL, totp_last = ?2 WHERE id = ?1",
        params![caller.id.to_string(), step],
    )?;
    conn.execute(
        "UPDATE sessions SET setup_only = 0 WHERE member = ?1",
        [caller.id.to_string()],
    )?;
    let mut e = caller.event(Action::MemberChanged);
    e.note = "Verification code set up".into();
    app.record(&conn, e)?;
    Ok(Json(SessionReply {
        token: String::new(),
        remembered: None,
        setup_code: false,
    }))
}

// ---------- State ----------

async fn state(State(app): State<Arc<App>>, caller: Caller) -> Res<cleis_core::org::State> {
    let conn = app.db.lock().expect("database lock");
    let me = member(&conn, caller.id)?.ok_or_else(|| ApiErr::not_found("member"))?;
    let manage = caller.role.can(Perm::Manage);
    let audit = caller.role.can(Perm::Audit);
    let all = members(&conn)?;
    let views = all
        .iter()
        .map(|m| view(&conn, m))
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut invites = vec![];
    if manage {
        let mut st = conn.prepare(
            "SELECT id, name, email, role, spaces, created, expires FROM invites WHERE expires > ?1 AND role != '' ORDER BY created",
        )?;
        invites = st
            .query_map([now()], |r| {
                Ok(InviteView {
                    id: r.get::<_, String>(0)?.parse().unwrap_or_default(),
                    name: r.get(1)?,
                    email: r.get(2)?,
                    role: r.get(3)?,
                    spaces: serde_json::from_str(&r.get::<_, String>(4)?).unwrap_or_default(),
                    created: r.get(5)?,
                    expires: r.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        // The first administrator's code has no e-mail: it stays out of the list.
        invites.retain(|i: &InviteView| !i.email.is_empty());
    }

    let sql = if manage {
        "SELECT id, sealed, mode, key_gen, org_wrapped, recovery_wrapped, needs_rekey FROM spaces ORDER BY created".to_owned()
    } else {
        "SELECT id, sealed, mode, key_gen, org_wrapped, recovery_wrapped, needs_rekey FROM spaces WHERE id IN (SELECT space FROM access WHERE member = ?1) ORDER BY created".to_owned()
    };
    let mut st = conn.prepare(&sql)?;
    let map = |r: &rusqlite::Row| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Vec<u8>>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, u32>(3)?,
            r.get::<_, Option<Vec<u8>>>(4)?,
            r.get::<_, Option<Vec<u8>>>(5)?,
            r.get::<_, i64>(6)?,
        ))
    };
    let rows: Vec<_> = if manage {
        st.query_map([], map)?.collect::<rusqlite::Result<_>>()?
    } else {
        st.query_map([caller.id.to_string()], map)?
            .collect::<rusqlite::Result<_>>()?
    };
    let mut spaces = vec![];
    let mut pending = vec![];
    for (id, sealed, mode, key_gen, org_wrapped, recovery_wrapped, needs_rekey) in rows {
        let sid = SpaceId(id.parse().unwrap_or_default());
        let mut st = conn.prepare(
            "SELECT a.member, a.wrapped, m.account_public FROM access a JOIN members m ON m.id = a.member WHERE a.space = ?1 AND m.revoked IS NULL",
        )?;
        let access: Vec<(String, Option<Vec<u8>>, Vec<u8>)> = st
            .query_map([&id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<rusqlite::Result<_>>()?;
        let wrapped = access
            .iter()
            .find(|a| a.0 == caller.id.to_string())
            .and_then(|a| a.1.clone());
        for (m, w, public) in &access {
            if w.is_none() {
                pending.push(Pending {
                    space: sid,
                    member: MemberId(m.parse().unwrap_or_default()),
                    account_public: public.clone().try_into().unwrap_or([0; 32]),
                });
            }
        }
        let items: u32 = conn.query_row(
            "SELECT count(*) FROM records WHERE space = ?1 AND deleted IS NULL",
            [&id],
            |r| r.get(0),
        )?;
        spaces.push(SpaceView {
            id: sid,
            sealed,
            mode: if mode == "standard" {
                SpaceMode::Standard
            } else {
                SpaceMode::EndToEnd
            },
            key_gen,
            wrapped,
            org_wrapped: if manage { org_wrapped } else { None },
            recovery_wrapped: if manage { recovery_wrapped } else { None },
            members: access
                .iter()
                .map(|a| MemberId(a.0.parse().unwrap_or_default()))
                .collect(),
            items,
            needs_rekey: manage && needs_rekey != 0,
        });
    }

    let org_key: Option<Vec<u8>> = if is_admin_role(&me.role) {
        conn.query_row(
            "SELECT wrapped FROM admin_keys WHERE member = ?1",
            [caller.id.to_string()],
            |r| r.get(0),
        )
        .optional()?
    } else {
        None
    };
    let mut pending_admins = vec![];
    if org_key.is_some() {
        for m in all
            .iter()
            .filter(|m| is_admin_role(&m.role) && m.id != caller.id)
        {
            if !conn
                .prepare("SELECT 1 FROM admin_keys WHERE member = ?1")?
                .exists([m.id.to_string()])?
            {
                pending_admins.push(Pending {
                    space: SpaceId(Uuid::nil()),
                    member: m.id,
                    account_public: m.account_public,
                });
            }
        }
    }
    let events: u64 = if audit {
        conn.query_row("SELECT count(*) FROM log", [], |r| r.get::<_, i64>(0))? as u64
    } else {
        0
    };
    Ok(Json(cleis_core::org::State {
        org: org_info(&conn, &app.cert)?,
        me: view(&conn, &me)?,
        roles: db::roles(&conn),
        policy: db::policy(&conn),
        members: views,
        invites,
        spaces,
        pending,
        org_key,
        pending_admins,
        seq: db::seq(&conn)?,
        integrity: if audit {
            db::meta_json(&conn, "integrity")
        } else {
            None
        },
        events,
    }))
}

// ---------- Records ----------

const ROW_COLS: &str = "kind, id, space, sealed, rev, updated, deleted, changed, key_gen";

fn row_from(r: &rusqlite::Row) -> rusqlite::Result<Row> {
    Ok(Row {
        kind: r.get(0)?,
        id: r.get(1)?,
        space: SpaceId(r.get::<_, String>(2)?.parse().unwrap_or_default()),
        sealed: r.get(3)?,
        rev: r.get(4)?,
        updated: r.get(5)?,
        deleted: r.get(6)?,
        changed: r.get(7)?,
        key_gen: r.get(8)?,
    })
}

const PAGE: usize = 1000;

async fn pull(State(app): State<Arc<App>>, caller: Caller, Json(req): Json<Pull>) -> Res<Changes> {
    caller.full()?;
    let conn = app.db.lock().expect("database lock");
    let me = caller.id.to_string();
    let mut st = conn.prepare(&format!(
        "SELECT {ROW_COLS}, seq FROM records WHERE seq > ?1 AND space IN (SELECT space FROM access WHERE member = ?2) ORDER BY seq LIMIT {PAGE}"
    ))?;
    let mut rows: Vec<(Row, u64)> = st
        .query_map(params![req.since as i64, me], |r| {
            Ok((row_from(r)?, r.get::<_, i64>(9)? as u64))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut st = conn.prepare(&format!(
        "SELECT kind, id, seq FROM purged WHERE seq > ?1 AND space IN (SELECT space FROM access WHERE member = ?2) ORDER BY seq LIMIT {PAGE}"
    ))?;
    let mut purged: Vec<(String, String, u64)> = st
        .query_map(params![req.since as i64, me], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? as u64))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut upto = db::seq(&conn)?;
    if rows.len() == PAGE {
        upto = upto.min(rows.last().map_or(upto, |r| r.1));
    }
    if purged.len() == PAGE {
        upto = upto.min(purged.last().map_or(upto, |p| p.2));
    }
    rows.retain(|r| r.1 <= upto);
    purged.retain(|p| p.2 <= upto);
    let mut out: Vec<Row> = rows.into_iter().map(|r| r.0).collect();
    for s in req.full {
        if !in_space(&conn, s, caller.id)? {
            continue;
        }
        let mut st = conn.prepare(&format!("SELECT {ROW_COLS} FROM records WHERE space = ?1"))?;
        for r in st.query_map([s.to_string()], row_from)? {
            let r = r?;
            if !out.iter().any(|o| o.kind == r.kind && o.id == r.id) {
                out.push(r);
            }
        }
    }
    Ok(Json(Changes {
        rows: out,
        purged: purged.into_iter().map(|p| (p.0, p.1)).collect(),
        seq: upto,
    }))
}

fn existing(conn: &Connection, kind: &str, id: &str) -> rusqlite::Result<Option<Row>> {
    conn.query_row(
        &format!("SELECT {ROW_COLS} FROM records WHERE kind = ?1 AND id = ?2"),
        [kind, id],
        row_from,
    )
    .optional()
}

/// Versions kept per item.
const KEEP_VERSIONS: i64 = 20;

async fn push(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(req): Json<Changes>,
) -> Res<PushReply> {
    caller.full()?;
    let conn = app.db.lock().expect("database lock");
    let tx = conn.unchecked_transaction()?;
    let mut refused = vec![];
    let t = now();
    for row in req.rows {
        let refuse = |refused: &mut Vec<(String, String)>| {
            refused.push((row.kind.clone(), row.id.clone()));
        };
        if !matches!(row.kind.as_str(), "entry" | "contact")
            || !has_key(&conn, row.space, caller.id)?
            || key_gen(&conn, row.space)? != Some(row.key_gen)
        {
            refuse(&mut refused);
            continue;
        }
        let ex = existing(&conn, &row.kind, &row.id)?;
        let perm = match &ex {
            Some(ex) if ex.deleted.is_some() != row.deleted.is_some() => Perm::Delete,
            None if row.deleted.is_some() => Perm::Delete,
            _ => Perm::Edit,
        };
        let allowed = caller.role.can(Perm::Edit)
            && caller.role.can(perm)
            && match &ex {
                Some(ex) => {
                    ex.changed < row.changed
                        && (ex.space == row.space || has_key(&conn, ex.space, caller.id)?)
                }
                None => !conn
                    .prepare("SELECT 1 FROM purged WHERE kind = ?1 AND id = ?2")?
                    .exists([&row.kind, &row.id])?,
            };
        if !allowed {
            refuse(&mut refused);
            continue;
        }
        if ex.is_some() {
            conn.execute(
                "INSERT INTO versions(kind, id, space, sealed, rev, updated, deleted, changed, key_gen, author, saved)
                 SELECT kind, id, space, sealed, rev, updated, deleted, changed, key_gen, author, ?3 FROM records WHERE kind = ?1 AND id = ?2",
                params![row.kind, row.id, t],
            )?;
            conn.execute(
                "DELETE FROM versions WHERE kind = ?1 AND id = ?2 AND rowid NOT IN
                 (SELECT rowid FROM versions WHERE kind = ?1 AND id = ?2 ORDER BY changed DESC LIMIT ?3)",
                params![row.kind, row.id, KEEP_VERSIONS],
            )?;
        }
        let seq = db::next_seq(&conn)?;
        conn.execute(
            "INSERT INTO records(kind, id, space, sealed, rev, updated, deleted, changed, key_gen, seq, author)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(kind, id) DO UPDATE SET space = excluded.space, sealed = excluded.sealed, rev = excluded.rev,
               updated = excluded.updated, deleted = excluded.deleted, changed = excluded.changed,
               key_gen = excluded.key_gen, seq = excluded.seq, author = excluded.author",
            params![row.kind, row.id, row.space.to_string(), row.sealed, row.rev, row.updated, row.deleted,
                    row.changed, row.key_gen, seq as i64, caller.id.to_string()],
        )?;
        let action = match (&ex, row.deleted) {
            (None, _) => Action::Created,
            (Some(ex), Some(_)) if ex.deleted.is_none() => Action::Deleted,
            (Some(ex), None) if ex.deleted.is_some() => Action::Restored,
            _ => Action::Edited,
        };
        let mut e = caller.event(action);
        e.space = Some(row.space);
        e.item = Some(format!("{}/{}", row.kind, row.id));
        app.record(&conn, e)?;
    }
    for (kind, id) in req.purged {
        let Some(ex) = existing(&conn, &kind, &id)? else {
            continue;
        };
        if !caller.role.can(Perm::Delete) || !has_key(&conn, ex.space, caller.id)? {
            refused.push((kind, id));
            continue;
        }
        let seq = db::next_seq(&conn)?;
        conn.execute(
            "DELETE FROM records WHERE kind = ?1 AND id = ?2",
            [&kind, &id],
        )?;
        conn.execute(
            "DELETE FROM versions WHERE kind = ?1 AND id = ?2",
            [&kind, &id],
        )?;
        conn.execute(
            "INSERT INTO purged(kind, id, space, at, seq) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(kind, id) DO UPDATE SET at = excluded.at, seq = excluded.seq",
            params![kind, id, ex.space.to_string(), t, seq as i64],
        )?;
        let mut e = caller.event(Action::Erased);
        e.space = Some(ex.space);
        e.item = Some(format!("{kind}/{id}"));
        app.record(&conn, e)?;
    }
    tx.commit()?;
    Ok(Json(PushReply {
        seq: db::seq(&conn)?,
        refused,
    }))
}

async fn versions(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(req): Json<Versions>,
) -> Res<VersionList> {
    caller.full()?;
    let conn = app.db.lock().expect("database lock");
    let Some(ex) = existing(&conn, &req.kind, &req.id)? else {
        return Err(ApiErr::not_found("No such item."));
    };
    if !has_key(&conn, ex.space, caller.id)? {
        return Err(ApiErr::forbidden("Not shared with you."));
    }
    let mut st = conn.prepare(&format!(
        "SELECT {ROW_COLS} FROM versions WHERE kind = ?1 AND id = ?2 ORDER BY changed DESC"
    ))?;
    let rows = st
        .query_map([&req.kind, &req.id], row_from)?
        .collect::<rusqlite::Result<_>>()?;
    Ok(Json(VersionList { rows }))
}

// ---------- Spaces and keys ----------

async fn create_space(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(req): Json<NewSpace>,
) -> Res<PushReply> {
    caller.can(Perm::Manage)?;
    let conn = app.db.lock().expect("database lock");
    if space_exists(&conn, req.id)? {
        return Err(ApiErr::bad("This space already exists."));
    }
    if req.mode == SpaceMode::Standard && req.org_wrapped.is_none() {
        return Err(ApiErr::bad("A Standard space needs its recovery copy."));
    }
    let tx = conn.unchecked_transaction()?;
    conn.execute(
        "INSERT INTO spaces(id, sealed, mode, key_gen, org_wrapped, recovery_wrapped, created) VALUES (?1, ?2, ?3, 1, ?4, ?5, ?6)",
        params![req.id.to_string(), req.sealed, mode_str(req.mode), req.org_wrapped, req.recovery_wrapped, now()],
    )?;
    conn.execute(
        "INSERT INTO access(space, member, wrapped) VALUES (?1, ?2, ?3)",
        params![req.id.to_string(), caller.id.to_string(), req.wrapped],
    )?;
    let mut e = caller.event(Action::SpaceCreated);
    e.space = Some(req.id);
    app.record(&conn, e)?;
    tx.commit()?;
    Ok(Json(PushReply {
        seq: db::seq(&conn)?,
        refused: vec![],
    }))
}

async fn rekey(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(req): Json<Rekey>,
) -> Res<PushReply> {
    caller.can(Perm::Manage)?;
    let conn = app.db.lock().expect("database lock");
    let Some(cur) = key_gen(&conn, req.space)? else {
        return Err(ApiErr::not_found("No such space."));
    };
    if !has_key(&conn, req.space, caller.id)? {
        return Err(ApiErr::forbidden("You do not hold this space's key."));
    }
    if req.key_gen != cur + 1 {
        return Err(ApiErr::bad("The space changed meanwhile; try again."));
    }
    if req.mode == SpaceMode::Standard && req.org_wrapped.is_none() {
        return Err(ApiErr::bad("A Standard space needs its recovery copy."));
    }
    let mut st = conn.prepare("SELECT kind, id FROM records WHERE space = ?1")?;
    let present: BTreeSet<(String, String)> = st
        .query_map([req.space.to_string()], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let sent: BTreeSet<(String, String)> = req
        .rows
        .iter()
        .filter(|r| r.space == req.space)
        .map(|r| (r.kind.clone(), r.id.clone()))
        .collect();
    if present != sent || sent.len() != req.rows.len() {
        return Err(ApiErr::bad("The space changed meanwhile; try again."));
    }
    let tx = conn.unchecked_transaction()?;
    for r in &req.rows {
        let seq = db::next_seq(&conn)?;
        conn.execute(
            "UPDATE records SET sealed = ?3, key_gen = ?4, seq = ?5, changed = ?6, rev = ?7 WHERE kind = ?1 AND id = ?2",
            params![r.kind, r.id, r.sealed, req.key_gen, seq as i64, r.changed, r.rev],
        )?;
    }
    // Earlier versions are sealed with the old key, which members no longer keep.
    conn.execute(
        "DELETE FROM versions WHERE space = ?1",
        [req.space.to_string()],
    )?;
    conn.execute(
        "UPDATE spaces SET sealed = ?2, mode = ?3, key_gen = ?4, org_wrapped = ?5, recovery_wrapped = ?6, needs_rekey = 0 WHERE id = ?1",
        params![req.space.to_string(), req.sealed, mode_str(req.mode), req.key_gen, req.org_wrapped, req.recovery_wrapped],
    )?;
    conn.execute(
        "UPDATE access SET wrapped = NULL WHERE space = ?1",
        [req.space.to_string()],
    )?;
    for w in req.wraps.iter().filter(|w| w.space == req.space) {
        conn.execute(
            "UPDATE access SET wrapped = ?3 WHERE space = ?1 AND member = ?2",
            params![req.space.to_string(), w.member.to_string(), w.wrapped],
        )?;
    }
    let mut e = caller.event(Action::SpaceReencrypted);
    e.space = Some(req.space);
    e.note = match req.mode {
        SpaceMode::Standard => "Standard".into(),
        SpaceMode::EndToEnd => "End-to-end".into(),
    };
    app.record(&conn, e)?;
    tx.commit()?;
    Ok(Json(PushReply {
        seq: db::seq(&conn)?,
        refused: vec![],
    }))
}

async fn grants(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(req): Json<Grants>,
) -> Res<PushReply> {
    caller.full()?;
    let conn = app.db.lock().expect("database lock");
    let admin = caller.role.can(Perm::Manage);
    let mut refused = vec![];
    for w in req.wraps {
        let org_copy = conn
            .prepare("SELECT 1 FROM spaces WHERE id = ?1 AND org_wrapped IS NOT NULL")?
            .exists([w.space.to_string()])?;
        let can = has_key(&conn, w.space, caller.id)? || admin && org_copy;
        let n = if can {
            conn.execute(
                "UPDATE access SET wrapped = ?3 WHERE space = ?1 AND member = ?2 AND wrapped IS NULL",
                params![w.space.to_string(), w.member.to_string(), w.wrapped],
            )?
        } else {
            0
        };
        if n == 0 {
            refused.push((w.space.to_string(), w.member.to_string()));
        }
    }
    Ok(Json(PushReply {
        seq: db::seq(&conn)?,
        refused,
    }))
}

async fn admin_keys(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(req): Json<AdminKeys>,
) -> Res<PushReply> {
    caller.can(Perm::Manage)?;
    if !caller.role.is_admin() {
        return Err(ApiErr::forbidden("Administrators only."));
    }
    let conn = app.db.lock().expect("database lock");
    let tx = conn.unchecked_transaction()?;
    let holds = conn
        .prepare("SELECT 1 FROM admin_keys WHERE member = ?1")?
        .exists([caller.id.to_string()])?;
    match (&req.org_public, db::meta_get(&conn, "org_public")?) {
        (Some(p), None) if p.len() == 32 => db::meta_set(&conn, "org_public", p)?,
        (Some(_), _) => return Err(ApiErr::bad("The organization key is already set.")),
        (None, None) => return Err(ApiErr::bad("No organization key yet.")),
        (None, Some(_)) if !holds => {
            return Err(ApiErr::forbidden("You do not hold the organization key."));
        }
        _ => {}
    }
    let mut refused = vec![];
    for w in req.wraps {
        let ok = member(&conn, w.member)?
            .is_some_and(|m| is_admin_role(&m.role) && m.revoked.is_none())
            && conn.execute(
                "INSERT OR IGNORE INTO admin_keys(member, wrapped) VALUES (?1, ?2)",
                params![w.member.to_string(), w.wrapped],
            )? == 1;
        if !ok {
            refused.push((String::new(), w.member.to_string()));
        }
    }
    tx.commit()?;
    Ok(Json(PushReply {
        seq: db::seq(&conn)?,
        refused,
    }))
}

async fn recovery(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(req): Json<Recovery>,
) -> Res<OrgInfo> {
    caller.can(Perm::Admins)?;
    let conn = app.db.lock().expect("database lock");
    let tx = conn.unchecked_transaction()?;
    if req.public.is_empty() {
        db::meta_del(&conn, "recovery_public")?;
        db::meta_del(&conn, "recovery_saved")?;
        conn.execute("UPDATE spaces SET recovery_wrapped = NULL", [])?;
    } else {
        if req.public.len() != 32 {
            return Err(ApiErr::bad("Bad key."));
        }
        db::meta_set(&conn, "recovery_public", &req.public)?;
        db::meta_set_json(&conn, "recovery_saved", &now())?;
        conn.execute("UPDATE spaces SET recovery_wrapped = NULL", [])?;
        for w in req.wraps {
            conn.execute(
                "UPDATE spaces SET recovery_wrapped = ?2 WHERE id = ?1",
                params![w.space.to_string(), w.wrapped],
            )?;
        }
    }
    let mut e = caller.event(Action::KeyRotated);
    e.note = if req.public.is_empty() {
        "Recovery key off".into()
    } else {
        "Recovery key".into()
    };
    app.record(&conn, e)?;
    tx.commit()?;
    Ok(Json(org_info(&conn, &app.cert)?))
}

async fn rotate(State(app): State<Arc<App>>, caller: Caller) -> Res<OrgInfo> {
    caller.can(Perm::Admins)?;
    let id = crate::tls::Identity::generate(&[]).map_err(|e| ApiErr::bad(&e))?;
    let conn = app.db.lock().expect("database lock");
    db::meta_set(&conn, "next_cert", &id.cert)?;
    db::meta_set(&conn, "next_cert_key", &id.key)?;
    db::meta_set_json(&conn, "next_cert_at", &now())?;
    let mut e = caller.event(Action::KeyRotated);
    e.note = "Server key".into();
    app.record(&conn, e)?;
    Ok(Json(org_info(&conn, &app.cert)?))
}

// ---------- Members, invitations, roles, policy ----------

/// Removes access rows; spaces whose key the member held are marked for re-encryption.
fn drop_access(conn: &Connection, member: MemberId, spaces: &[SpaceId]) -> rusqlite::Result<()> {
    for s in spaces {
        let held = has_key(conn, *s, member)?;
        conn.execute(
            "DELETE FROM access WHERE space = ?1 AND member = ?2",
            [s.to_string(), member.to_string()],
        )?;
        if held {
            conn.execute(
                "UPDATE spaces SET needs_rekey = 1 WHERE id = ?1",
                [s.to_string()],
            )?;
        }
    }
    Ok(())
}

async fn update_member(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(req): Json<MemberChange>,
) -> Res<MemberView> {
    caller.can(Perm::Manage)?;
    let conn = app.db.lock().expect("database lock");
    let Some(m) = member(&conn, req.member)?.filter(|m| m.revoked.is_none()) else {
        return Err(ApiErr::not_found("No such member."));
    };
    let tx = conn.unchecked_transaction()?;
    let mut notes = vec![];
    if let Some(role) = &req.role
        && *role != m.role
    {
        let roles = db::roles(&conn);
        let Some(r) = roles.iter().find(|r| r.id == *role) else {
            return Err(ApiErr::bad("No such role."));
        };
        if (is_admin_role(&m.role) || is_admin_role(role)) && !caller.role.can(Perm::Admins) {
            return Err(ApiErr::forbidden(
                "Only a Super Admin changes who is admin.",
            ));
        }
        if m.role == SUPER_ADMIN && supers(&conn)? <= 1 {
            return Err(ApiErr::bad("The organization needs a Super Admin."));
        }
        if is_admin_role(&m.role) && !is_admin_role(role) {
            conn.execute(
                "DELETE FROM admin_keys WHERE member = ?1",
                [m.id.to_string()],
            )?;
        }
        conn.execute(
            "UPDATE members SET role = ?2 WHERE id = ?1",
            params![m.id.to_string(), role],
        )?;
        notes.push(format!("role {}", r.name));
    }
    if let Some(spaces) = &req.spaces {
        let now_in: BTreeSet<SpaceId> = spaces_of(&conn, m.id)?.into_iter().collect();
        let want: BTreeSet<SpaceId> = spaces.iter().copied().collect();
        let gone: Vec<SpaceId> = now_in.difference(&want).copied().collect();
        drop_access(&conn, m.id, &gone)?;
        for s in want.difference(&now_in) {
            if space_exists(&conn, *s)? {
                conn.execute(
                    "INSERT OR IGNORE INTO access(space, member) VALUES (?1, ?2)",
                    [s.to_string(), m.id.to_string()],
                )?;
            }
        }
        if now_in != want {
            notes.push("spaces".into());
        }
    }
    if let Some(export) = req.export
        && export != m.export
    {
        conn.execute(
            "UPDATE members SET export = ?2 WHERE id = ?1",
            params![m.id.to_string(), export as i64],
        )?;
        notes.push(if export { "export on" } else { "export off" }.into());
    }
    if !notes.is_empty() {
        let mut e = caller.event(Action::MemberChanged);
        e.note = format!("{}: {}", m.name, notes.join(", "));
        app.record(&conn, e)?;
    }
    tx.commit()?;
    let m = member(&conn, req.member)?.ok_or_else(|| ApiErr::not_found("member"))?;
    Ok(Json(view(&conn, &m)?))
}

async fn revoke(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(req): Json<IdOnly>,
) -> Res<PushReply> {
    caller.can(Perm::Manage)?;
    let id = MemberId(req.id);
    if id == caller.id {
        return Err(ApiErr::bad("You cannot remove yourself."));
    }
    let conn = app.db.lock().expect("database lock");
    let Some(m) = member(&conn, id)?.filter(|m| m.revoked.is_none()) else {
        return Err(ApiErr::not_found("No such member."));
    };
    if is_admin_role(&m.role) && !caller.role.can(Perm::Admins) {
        return Err(ApiErr::forbidden("Only a Super Admin removes an admin."));
    }
    if m.role == SUPER_ADMIN && supers(&conn)? <= 1 {
        return Err(ApiErr::bad("The organization needs a Super Admin."));
    }
    let tx = conn.unchecked_transaction()?;
    conn.execute(
        "UPDATE members SET revoked = ?2 WHERE id = ?1",
        params![id.to_string(), now()],
    )?;
    for t in ["sessions", "remembered", "admin_keys"] {
        conn.execute(
            &format!("DELETE FROM {t} WHERE member = ?1"),
            [id.to_string()],
        )?;
    }
    let spaces = spaces_of(&conn, id)?;
    drop_access(&conn, id, &spaces)?;
    let mut e = caller.event(Action::Revoked);
    e.note = m.name.clone();
    app.record(&conn, e)?;
    tx.commit()?;
    Ok(Json(PushReply {
        seq: db::seq(&conn)?,
        refused: vec![],
    }))
}

/// Hands the Super Admin role to another member; the caller becomes Admin (D-60).
async fn transfer(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(req): Json<IdOnly>,
) -> Res<MemberView> {
    caller.can(Perm::Admins)?;
    if !caller.role.is_super() {
        return Err(ApiErr::forbidden("Only a Super Admin transfers the role."));
    }
    let to = MemberId(req.id);
    if to == caller.id {
        return Err(ApiErr::bad("Choose another member."));
    }
    let conn = app.db.lock().expect("database lock");
    let Some(m) = member(&conn, to)?.filter(|m| m.revoked.is_none()) else {
        return Err(ApiErr::not_found("No such member."));
    };
    let tx = conn.unchecked_transaction()?;
    conn.execute(
        "UPDATE members SET role = ?2 WHERE id = ?1",
        params![to.to_string(), SUPER_ADMIN],
    )?;
    conn.execute(
        "UPDATE members SET role = ?2 WHERE id = ?1",
        params![caller.id.to_string(), ADMIN],
    )?;
    let mut e = caller.event(Action::MemberChanged);
    e.note = format!("{}: Super Admin transferred", m.name);
    app.record(&conn, e)?;
    tx.commit()?;
    let m = member(&conn, to)?.ok_or_else(|| ApiErr::not_found("member"))?;
    Ok(Json(view(&conn, &m)?))
}

async fn invite(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(req): Json<NewInvite>,
) -> Res<InviteReply> {
    caller.can(Perm::Manage)?;
    if is_admin_role(&req.role) && !caller.role.can(Perm::Admins) {
        return Err(ApiErr::forbidden("Only a Super Admin invites an admin."));
    }
    let email = req.email.trim();
    if !email.contains('@') {
        return Err(ApiErr::bad("Enter an e-mail address."));
    }
    let conn = app.db.lock().expect("database lock");
    if !db::roles(&conn).iter().any(|r| r.id == req.role) {
        return Err(ApiErr::bad("No such role."));
    }
    if members(&conn)?
        .iter()
        .any(|m| m.email.eq_ignore_ascii_case(email))
    {
        return Err(ApiErr::bad("Already a member."));
    }
    let (id, code) = create_invite(
        &conn,
        &app.cert,
        req.name.trim(),
        email,
        &req.role,
        &req.spaces,
        now(),
    )?;
    let mut e = caller.event(Action::Invited);
    e.note = email.to_owned();
    app.record(&conn, e)?;
    Ok(Json(InviteReply { id, code }))
}

async fn revoke_invite(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(req): Json<IdOnly>,
) -> Res<PushReply> {
    caller.can(Perm::Manage)?;
    let conn = app.db.lock().expect("database lock");
    conn.execute("DELETE FROM invites WHERE id = ?1", [req.id.to_string()])?;
    Ok(Json(PushReply::default()))
}

async fn renew_invite(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(req): Json<IdOnly>,
) -> Res<InviteReply> {
    caller.can(Perm::Manage)?;
    let token: [u8; 10] = random_bytes().map_err(|_| ApiErr::bad("random"))?;
    let conn = app.db.lock().expect("database lock");
    let t = now();
    if conn.execute(
        "UPDATE invites SET token_hash = ?2, created = ?3, expires = ?4 WHERE id = ?1",
        params![
            req.id.to_string(),
            &sha256(&token)[..],
            t,
            t + INVITE_DAYS * 86_400
        ],
    )? != 1
    {
        return Err(ApiErr::not_found("No such invitation."));
    }
    Ok(Json(InviteReply {
        id: req.id,
        code: crate::code_for(token, &app.cert),
    }))
}

async fn set_roles(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(req): Json<Vec<Role>>,
) -> Res<Vec<Role>> {
    caller.can(Perm::Admins)?;
    if !valid_roles(&req) {
        return Err(ApiErr::bad(
            "Each role needs its own name; Admin cannot change.",
        ));
    }
    let conn = app.db.lock().expect("database lock");
    let ids: BTreeSet<&str> = req.iter().map(|r| r.id.as_str()).collect();
    let mut st = conn
        .prepare("SELECT role FROM members WHERE revoked IS NULL UNION SELECT role FROM invites")?;
    let used: Vec<String> = st
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    if used.iter().any(|u| !ids.contains(u.as_str())) {
        return Err(ApiErr::bad(
            "A role still given to someone cannot be removed.",
        ));
    }
    db::meta_set_json(&conn, "roles", &req)?;
    app.record(&conn, caller.event(Action::RolesChanged))?;
    Ok(Json(req))
}

async fn set_policy(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(req): Json<Policy>,
) -> Res<Policy> {
    caller.can(Perm::Manage)?;
    let conn = app.db.lock().expect("database lock");
    let before = db::policy(&conn);
    db::meta_set_json(&conn, "policy", &req)?;
    // Written even when the new policy records nothing, so turning the log off is visible.
    if before.record_activity {
        app.log
            .lock()
            .expect("log lock")
            .append(&conn, &caller.event(Action::PolicyChanged))?;
    } else {
        app.record(&conn, caller.event(Action::PolicyChanged))?;
    }
    Ok(Json(req))
}

// ---------- Activity log ----------

async fn report(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(req): Json<Report>,
) -> Res<PushReply> {
    caller.full()?;
    if !req.action.from_client() {
        return Err(ApiErr::bad("The server writes this event itself."));
    }
    let conn = app.db.lock().expect("database lock");
    if let Some(s) = req.space
        && !in_space(&conn, s, caller.id)?
    {
        return Err(ApiErr::forbidden("Not shared with you."));
    }
    if req.action == Action::Exported {
        let m = member(&conn, caller.id)?.ok_or_else(|| ApiErr::not_found("member"))?;
        if !(m.export || is_admin_role(&m.role)) {
            return Err(ApiErr::forbidden(
                "Your organization does not let you export its passwords.",
            ));
        }
    }
    let mut e = caller.event(req.action);
    e.space = req.space;
    e.item = req.item;
    e.detail = req.detail;
    app.record(&conn, e)?;
    Ok(Json(PushReply::default()))
}

fn events(conn: &Connection) -> rusqlite::Result<Vec<Event>> {
    let mut st = conn.prepare("SELECT record FROM log ORDER BY seq DESC")?;
    let out = st
        .query_map([], |r| r.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .filter_map(|s| serde_json::from_str::<cleis_core::journal::Record>(&s).ok())
        .filter_map(|r| {
            let mut e: Event = serde_json::from_slice(&r.payload).ok()?;
            e.seq = r.seq;
            Some(e)
        })
        .collect();
    Ok(out)
}

async fn query_log(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(q): Json<LogQuery>,
) -> Res<LogPage> {
    caller.can(Perm::Audit)?;
    let conn = app.db.lock().expect("database lock");
    let all: Vec<Event> = events(&conn)?
        .into_iter()
        .filter(|e| q.member.is_none() || e.member == q.member)
        .filter(|e| q.action.is_none() || Some(e.action) == q.action)
        .filter(|e| q.space.is_none() || e.space == q.space)
        .filter(|e| q.after.is_none_or(|a| e.at >= a))
        .collect();
    let total = all.len() as u64;
    let limit = q.limit.clamp(1, 5000) as usize;
    let events = all
        .into_iter()
        .filter(|e| q.before.is_none_or(|b| e.seq < b))
        .take(limit)
        .collect();
    Ok(Json(LogPage { events, total }))
}

async fn verify_log(State(app): State<Arc<App>>, caller: Caller) -> Res<Integrity> {
    caller.can(Perm::Audit)?;
    let conn = app.db.lock().expect("database lock");
    let rep = app.log.lock().expect("log lock").verify(&conn, now())?;
    db::meta_set_json(&conn, "integrity", &rep)?;
    Ok(Json(rep))
}

// ---------- Backups ----------

fn snapshot(conn: &Connection, dir: &std::path::Path) -> Result<Vec<u8>, ApiErr> {
    let tmp = dir.join(format!("snapshot-{}.db", uuid::Uuid::new_v4()));
    conn.execute("VACUUM INTO ?1", [tmp.to_string_lossy().as_ref()])?;
    let bytes = std::fs::read(&tmp).map_err(|e| ApiErr::bad(&e.to_string()));
    let _ = std::fs::remove_file(&tmp);
    bytes
}

/// A copy of the whole server file (only sealed data, but also the server's certificate key and
/// the members' authenticator keys: the app seals it with the backup key before keeping it).
async fn backup(State(app): State<Arc<App>>, caller: Caller) -> Result<Vec<u8>, ApiErr> {
    caller.can(Perm::Manage)?;
    let conn = app.db.lock().expect("database lock");
    app.record(&conn, caller.event(Action::BackedUp))?;
    snapshot(&conn, &app.dir)
}

/// Puts back a copy made by `backup` (same organization only). The current file is kept beside
/// it as `server.db.before-restore`.
async fn restore(
    State(app): State<Arc<App>>,
    caller: Caller,
    body: axum::body::Bytes,
) -> Res<PushReply> {
    caller.can(Perm::Admins)?;
    if !caller.role.is_admin() {
        return Err(ApiErr::forbidden("Administrators only."));
    }
    let tmp = app.dir.join(format!("restore-{}.db", uuid::Uuid::new_v4()));
    std::fs::write(&tmp, &body).map_err(|e| ApiErr::bad(&e.to_string()))?;
    let check = (|| -> Result<(), ApiErr> {
        let other = Connection::open(&tmp)?;
        let ok: String = other.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        let same = db::meta_get(&other, "org_id")?;
        let conn = app.db.lock().expect("database lock");
        if ok != "ok" || same.is_none() || same != db::meta_get(&conn, "org_id")? {
            return Err(ApiErr::bad(
                "This backup is not one of this organization's.",
            ));
        }
        if db::meta_get(&other, "cert")? != db::meta_get(&conn, "cert")? {
            return Err(ApiErr::bad(
                "This backup was made with another server key: restore it with `cleis-server restore`.",
            ));
        }
        Ok(())
    })();
    if let Err(e) = check {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    let mut conn = app.db.lock().expect("database lock");
    let path = app.dir.join(crate::DB_FILE);
    let keep = app.dir.join(format!("{}.before-restore", crate::DB_FILE));
    // Close the current connection before the files move.
    let placeholder = Connection::open_in_memory()?;
    let old = std::mem::replace(&mut *conn, placeholder);
    let _ = old.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)");
    drop(old);
    let _ = std::fs::remove_file(&keep);
    let moved = std::fs::rename(&path, &keep).and_then(|_| std::fs::rename(&tmp, &path));
    for s in ["-wal", "-shm"] {
        let _ = std::fs::remove_file(app.dir.join(format!("{}{s}", crate::DB_FILE)));
    }
    if let Err(e) = moved {
        let _ = std::fs::rename(&keep, &path);
        *conn = db::open(&path)?;
        return Err(ApiErr::bad(&e.to_string()));
    }
    *conn = db::open(&path)?;
    app.log.lock().expect("log lock").reload(&conn)?;
    let mut e = caller.event(Action::BackedUp);
    e.note = "Restored".into();
    app.record(&conn, e)?;
    Ok(Json(PushReply {
        seq: db::seq(&conn)?,
        refused: vec![],
    }))
}

// ---------- Profile, devices, spaces (D-60) ----------

async fn profile(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(req): Json<ProfileChange>,
) -> Res<MemberView> {
    caller.full()?;
    let name = req.name.trim();
    let email = req.email.trim();
    if name.is_empty() {
        return Err(ApiErr::bad("Enter your name."));
    }
    if !email.contains('@') {
        return Err(ApiErr::bad("Enter an e-mail address."));
    }
    let conn = app.db.lock().expect("database lock");
    if members(&conn)?
        .iter()
        .any(|m| m.id != caller.id && m.email.eq_ignore_ascii_case(email))
    {
        return Err(ApiErr::bad("Another member has this e-mail address."));
    }
    conn.execute(
        "UPDATE members SET name = ?2, email = ?3 WHERE id = ?1",
        params![caller.id.to_string(), name, email],
    )?;
    let m = member(&conn, caller.id)?.ok_or_else(|| ApiErr::not_found("member"))?;
    Ok(Json(view(&conn, &m)?))
}

async fn devices(State(app): State<Arc<App>>, caller: Caller) -> Res<Vec<DeviceView>> {
    let conn = app.db.lock().expect("database lock");
    let t = now();
    let me = caller.id.to_string();
    let mut out: Vec<DeviceView> = vec![];
    let mut st = conn.prepare(
        "SELECT device, max(last) FROM sessions WHERE member = ?1 AND expires > ?2 GROUP BY device",
    )?;
    for r in st.query_map(params![me, t], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
    })? {
        let (device, last) = r?;
        out.push(DeviceView {
            current: device == caller.device,
            device,
            last_active: last,
            remembered_until: None,
        });
    }
    let mut st = conn.prepare(
        "SELECT device, max(expires) FROM remembered WHERE member = ?1 AND expires > ?2 AND device != '' GROUP BY device",
    )?;
    for r in st.query_map(params![me, t], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
    })? {
        let (device, until) = r?;
        match out.iter_mut().find(|d| d.device == device) {
            Some(d) => d.remembered_until = Some(until),
            None => out.push(DeviceView {
                current: device == caller.device,
                device,
                last_active: 0,
                remembered_until: Some(until),
            }),
        }
    }
    out.sort_by_key(|d| (!d.current, std::cmp::Reverse(d.last_active)));
    Ok(Json(out))
}

async fn sign_out(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(req): Json<DeviceRef>,
) -> Res<PushReply> {
    let conn = app.db.lock().expect("database lock");
    let me = caller.id.to_string();
    conn.execute(
        "DELETE FROM sessions WHERE member = ?1 AND device = ?2",
        [&me, &req.device],
    )?;
    conn.execute(
        "DELETE FROM remembered WHERE member = ?1 AND device = ?2",
        [&me, &req.device],
    )?;
    let mut e = caller.event(Action::SignedOut);
    e.note = req.device.clone();
    app.record(&conn, e)?;
    Ok(Json(PushReply::default()))
}

async fn update_space(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(req): Json<SpaceUpdate>,
) -> Res<PushReply> {
    caller.can(Perm::Manage)?;
    let conn = app.db.lock().expect("database lock");
    if !has_key(&conn, req.id, caller.id)? {
        return Err(ApiErr::forbidden("You do not hold this space's key."));
    }
    conn.execute(
        "UPDATE spaces SET sealed = ?2 WHERE id = ?1",
        params![req.id.to_string(), req.sealed],
    )?;
    let mut e = caller.event(Action::SpaceRenamed);
    e.space = Some(req.id);
    app.record(&conn, e)?;
    Ok(Json(PushReply {
        seq: db::seq(&conn)?,
        refused: vec![],
    }))
}

/// Erases a space and everything in it, for every member.
async fn delete_space(
    State(app): State<Arc<App>>,
    caller: Caller,
    Json(req): Json<IdOnly>,
) -> Res<PushReply> {
    caller.can(Perm::Manage)?;
    let id = SpaceId(req.id).to_string();
    let conn = app.db.lock().expect("database lock");
    if !space_exists(&conn, SpaceId(req.id))? {
        return Err(ApiErr::not_found("No such space."));
    }
    let tx = conn.unchecked_transaction()?;
    for t in ["versions", "records", "purged", "access"] {
        conn.execute(&format!("DELETE FROM {t} WHERE space = ?1"), [&id])?;
    }
    conn.execute("DELETE FROM spaces WHERE id = ?1", [&id])?;
    let mut e = caller.event(Action::SpaceDeleted);
    e.space = Some(SpaceId(req.id));
    app.record(&conn, e)?;
    tx.commit()?;
    Ok(Json(PushReply {
        seq: db::seq(&conn)?,
        refused: vec![],
    }))
}
