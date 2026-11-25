//! The organization server seen from the app (D-26, D-32, D-59).
//!
//! Membership (server address, pinned certificates, member id) is sealed in the vault's
//! settings, so another computer opening the same vault knows it too. The sign-in token, the
//! "remembered device" token and the last contact stay on this computer (`<vault>.org.json`).
//!
//! Network calls run off the interface thread ([`Conn`], blocking); what they bring back is
//! applied to the vault on the interface thread ([`CleisApp::org_done`]).

use crate::app::{CleisApp, Phase, Session};
use cleis_core::org::*;
use cleis_core::{Entry, MemberId, SpaceId};
use cleis_crypto::{KeyPair, PublicKey, SigningKeyPair, SymmetricKey, hex, wrap_key};
use cleis_net::{Client, NetError, Trust};
use cleis_store::{EncryptionMode, SpaceInfo, SpaceKind};
use gpui_kit::component::input::InputState;
use gpui_kit::prelude::*;
use gpui_kit::{Context, Entity, SharedString};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

/// Sealed setting holding the membership.
pub const SETTING: &str = "org";
/// Context of the organization key wrapped to each admin.
pub const ORG_KEY_CONTEXT: &[u8] = b"cleis/org-key";
/// Context of the recovery copies of space keys (same as the members' copies: the space id).
const SYNC_EVERY_SECS: i64 = 30;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Membership {
    pub org: uuid::Uuid,
    pub name: String,
    /// `host:port`.
    pub address: String,
    /// SHA-256 of the certificates trusted, hex.
    pub pins: Vec<String>,
    pub member: MemberId,
}

impl Membership {
    pub fn key(&self) -> String {
        self.org.to_string()
    }

    pub fn trust(&self) -> Trust {
        Trust::Pinned(
            self.pins
                .iter()
                .filter_map(|p| data_encoding::HEXLOWER.decode(p.as_bytes()).ok())
                .filter_map(|b| b.try_into().ok())
                .collect(),
        )
    }
}

/// This computer's part (beside the vault file).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Local {
    pub remembered: Option<String>,
    pub last_ok: Option<i64>,
}

pub fn local_path(vault: &Path) -> PathBuf {
    let mut p = vault.as_os_str().to_owned();
    p.push(".org.json");
    PathBuf::from(p)
}

pub fn load_local(vault: &Path) -> Local {
    std::fs::read(local_path(vault))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn save_local(vault: &Path, l: &Local) {
    let _ = std::fs::write(
        local_path(vault),
        serde_json::to_vec_pretty(l).unwrap_or_default(),
    );
}

/// Name shown in the activity log: computer name and system.
pub fn device_name() -> String {
    let host = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .filter(|h| !h.trim().is_empty())
        .or_else(|| {
            std::fs::read_to_string("/etc/hostname")
                .ok()
                .map(|s| s.trim().to_owned())
        })
        .unwrap_or_else(|| "Computer".into());
    let os = match std::env::consts::OS {
        "windows" => "Windows",
        "macos" => "macOS",
        "linux" => "Linux",
        o => o,
    };
    format!("{} · {os}", host.to_uppercase())
}

// ---------- Network (off the interface thread) ----------

/// Everything a request needs, copied out of the session.
#[derive(Clone)]
pub struct Conn {
    pub client: Client,
    pub member: MemberId,
    pub signing: Zeroizing<[u8; 32]>,
    pub device: String,
    pub remembered: Option<String>,
    /// Verification code typed for this sign-in.
    pub code: Option<String>,
    /// Set when a sign-in gave a new "remembered device" token.
    pub new_remembered: Option<String>,
    pub setup_code: bool,
}

impl Conn {
    pub fn sign_in(&mut self) -> Result<(), NetError> {
        let (ch, cert): (Challenge, [u8; 32]) = self.client.call_cert("/v1/challenge", &())?;
        let key = SigningKeyPair::from_secret_bytes(*self.signing);
        let reply: SessionReply = self.client.call(
            "/v1/session",
            &SessionRequest {
                member: self.member,
                signature: key.sign(SIGN_IN_DOMAIN, &sign_in_message(&ch.nonce, &cert)),
                nonce: ch.nonce,
                device: self.device.clone(),
                code: self.code.take(),
                remembered: self.remembered.clone(),
            },
        )?;
        self.client.token = Some(reply.token);
        self.setup_code = reply.setup_code;
        if reply.remembered.is_some() {
            self.new_remembered = reply.remembered.clone();
            self.remembered = reply.remembered;
        }
        Ok(())
    }

    fn retry<R>(&mut self, f: impl Fn(&Client) -> Result<R, NetError>) -> Result<R, NetError> {
        if self.client.token.is_none() {
            self.sign_in()?;
        }
        match f(&self.client) {
            Err(e) if e.code() == Some("unauthorized") => {
                self.client.token = None;
                self.sign_in()?;
                f(&self.client)
            }
            r => r,
        }
    }

    pub fn call<Q: Serialize, R: serde::de::DeserializeOwned>(
        &mut self,
        path: &str,
        req: &Q,
    ) -> Result<R, NetError> {
        self.retry(|c| c.call(path, req))
    }

    pub fn get<R: serde::de::DeserializeOwned>(&mut self, path: &str) -> Result<R, NetError> {
        self.retry(|c| c.get(path))
    }

    pub fn send(
        &mut self,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
    ) -> Result<Vec<u8>, NetError> {
        self.retry(|c| {
            let r = c.send(method, path, body)?;
            if (200..300).contains(&r.status) {
                Ok(r.body)
            } else {
                Err(NetError::Api(
                    r.status,
                    serde_json::from_slice(&r.body).unwrap_or(ApiError {
                        error: "http".into(),
                        message: format!("HTTP {}", r.status),
                    }),
                ))
            }
        })
    }
}

/// What a sync brought back.
pub enum Outcome {
    Done {
        conn: Box<Conn>,
        changes: Changes,
        state: Box<State>,
    },
    /// The server wants a verification code (`wrong`: the one typed was refused).
    NeedCode {
        wrong: bool,
    },
    /// Signed in, but the policy wants a verification code set up first.
    SetupCode {
        conn: Box<Conn>,
        secret: String,
    },
    /// This member was removed from the organization.
    Revoked,
    Failed(String),
}

/// Rows to send, pulled from `since`, then the state.
pub fn sync_job(mut conn: Conn, push: Changes, since: u64, full: Vec<SpaceId>) -> Outcome {
    let run = |conn: &mut Conn| -> Result<Outcome, NetError> {
        if conn.client.token.is_none() {
            conn.sign_in()?;
        }
        if conn.setup_code {
            let setup: CodeSetup = conn.call("/v1/me/code", &())?;
            return Ok(Outcome::SetupCode {
                conn: Box::new(conn.clone()),
                secret: setup.secret,
            });
        }
        if !push.rows.is_empty() || !push.purged.is_empty() {
            let _: PushReply = conn.call("/v1/push", &push)?;
        }
        let mut changes = Changes {
            seq: since,
            ..Default::default()
        };
        let mut full = full.clone();
        for _ in 0..100 {
            let ch: Changes = conn.call(
                "/v1/pull",
                &Pull {
                    since: changes.seq,
                    full: std::mem::take(&mut full),
                },
            )?;
            let done = ch.seq == changes.seq || ch.rows.is_empty() && ch.purged.is_empty();
            changes.rows.extend(ch.rows);
            changes.purged.extend(ch.purged);
            changes.seq = ch.seq;
            if done {
                break;
            }
        }
        let state: State = conn.get("/v1/state")?;
        Ok(Outcome::Done {
            conn: Box::new(conn.clone()),
            changes,
            state: Box::new(state),
        })
    };
    match run(&mut conn) {
        Ok(o) => o,
        Err(e) => match e.code() {
            Some("code_required") => Outcome::NeedCode { wrong: false },
            Some("wrong_code") => Outcome::NeedCode { wrong: true },
            Some("revoked") => Outcome::Revoked,
            Some("setup_code") => Outcome::Failed("Set up your verification code".into()),
            _ => Outcome::Failed(e.to_string()),
        },
    }
}

/// What an invitation code is for (first launch, Settings › Organization).
pub fn lookup(address: &str, code: &str) -> Result<InviteInfo, String> {
    let addr = server_address(address).ok_or("Enter the server address")?;
    let inv = Invitation::parse(code).ok_or("Invitation code incomplete")?;
    Client::new(&addr, Trust::Prefix(inv.pin))
        .call(
            "/v1/invite-info",
            &InviteLookup {
                token: inv.token.to_vec(),
            },
        )
        .map_err(|e| e.to_string())
}

/// Joins with the vault's account keys; returns the membership to keep.
pub fn join(
    address: &str,
    code: &str,
    name: &str,
    email: &str,
    account_public: [u8; 32],
    signing: &SigningKeyPair,
) -> Result<Membership, String> {
    let addr = server_address(address).ok_or("Enter the server address")?;
    let inv = Invitation::parse(code).ok_or("Invitation code incomplete")?;
    let mut req = JoinRequest {
        token: inv.token.to_vec(),
        name: name.trim().into(),
        email: email.trim().into(),
        device: device_name(),
        account_public,
        verify_key: signing.verifying_key().0,
        signature: [0; 64],
    };
    req.signature = signing.sign(JOIN_DOMAIN, &req.signed_bytes());
    let (reply, cert): (JoinReply, [u8; 32]) = Client::new(&addr, Trust::Prefix(inv.pin))
        .call_cert("/v1/join", &req)
        .map_err(|e| e.to_string())?;
    Ok(Membership {
        org: reply.org.id,
        name: reply.org.name,
        address: addr,
        pins: vec![hex(&cert)],
        member: reply.member,
    })
}

// ---------- Session state ----------

/// Verification code asked by the server (dialog over the main window).
pub struct CodePrompt {
    pub input: Option<Entity<InputState>>,
    pub error: Option<SharedString>,
    /// Setting a code up: the key to add to an authenticator app.
    pub secret: Option<String>,
    pub busy: bool,
}

/// The organization as this session knows it.
pub struct OrgSession {
    pub m: Membership,
    pub token: Option<String>,
    pub local: Local,
    pub state: Option<State>,
    pub busy: bool,
    pub again: bool,
    pub last_attempt: Option<i64>,
    pub error: Option<String>,
    /// Admins: the organization key, opened.
    pub org_key: Option<KeyPair>,
    /// Spaces to pull whole at the next sync.
    pub full: Vec<SpaceId>,
    pub code: Option<CodePrompt>,
    /// Names of the organization's spaces this session can open.
    pub names: BTreeMap<SpaceId, String>,
    /// The organization's spaces are hidden: not reached for longer than the policy allows.
    pub offline_locked: bool,
    pub ui: crate::screens::org::OrgUi,
}

impl OrgSession {
    pub fn me(&self) -> Option<&MemberView> {
        self.state.as_ref().map(|s| &s.me)
    }

    pub fn role(&self) -> Option<&Role> {
        let st = self.state.as_ref()?;
        st.roles.iter().find(|r| r.id == st.me.role)
    }

    pub fn can(&self, p: Perm) -> bool {
        self.role().is_some_and(|r| r.can(p))
    }

    pub fn policy(&self) -> Option<&Policy> {
        self.state.as_ref().map(|s| &s.policy)
    }

    pub fn member_name(&self, id: MemberId) -> String {
        self.state
            .as_ref()
            .and_then(|s| s.members.iter().find(|m| m.id == id))
            .map(|m| m.name.clone())
            .unwrap_or_else(|| "Former member".into())
    }

    pub fn space_name(&self, id: SpaceId) -> String {
        self.names
            .get(&id)
            .cloned()
            .unwrap_or_else(|| "Space".into())
    }

    /// Opens a space key through the organization key (admins, Standard spaces).
    pub fn key_via_org(&self, space: &SpaceView) -> Option<SymmetricKey> {
        let k = self.org_key.as_ref()?;
        cleis_crypto::unwrap_key(k, space.org_wrapped.as_ref()?, space.id.0.as_bytes()).ok()
    }
}

/// A space key for `space`, from this vault or through the organization key.
fn space_key(s: &Session, org: &OrgSession, sv: &SpaceView) -> Option<SymmetricKey> {
    s.vault
        .space_key(sv.id)
        .cloned()
        .or_else(|| org.key_via_org(sv))
}

pub fn initials(name: &str) -> String {
    let mut it = name
        .split_whitespace()
        .filter_map(|w| w.chars().next())
        .collect::<Vec<_>>()
        .into_iter();
    let a = it.next().unwrap_or('?');
    let b = it.next_back();
    let mut s = a.to_uppercase().to_string();
    if let Some(b) = b {
        s.extend(b.to_uppercase());
    }
    s
}

impl CleisApp {
    /// Loads the membership when the vault opens.
    pub fn load_org(&mut self, cx: &mut Context<Self>) {
        let path = self.path.clone();
        let demo = self.demo;
        let Some(s) = self.session() else { return };
        let m: Option<Membership> = s.vault.org_setting(SETTING).ok().flatten();
        s.org = m.map(|m| {
            let local = load_local(&path);
            let offline_days = 14;
            let offline_locked = !demo
                && local
                    .last_ok
                    .is_some_and(|t| cleis_core::now() - t > offline_days * 86_400);
            OrgSession {
                m,
                token: None,
                local,
                state: None,
                busy: false,
                again: true,
                last_attempt: None,
                error: None,
                org_key: None,
                full: vec![],
                code: None,
                names: BTreeMap::new(),
                offline_locked,
                ui: Default::default(),
            }
        });
        if let Some(o) = &s.org {
            let key = o.m.key();
            for (id, _) in s.vault.org_spaces(&key).unwrap_or_default() {
                if let Some(sp) = s.spaces.iter().find(|x| x.id == id) {
                    let name = sp.name.clone();
                    if let Some(o) = s.org.as_mut() {
                        o.names.insert(id, name);
                    }
                }
            }
        }
        if s.org.as_ref().is_some_and(|o| o.offline_locked) {
            hide_org_entries(s);
        }
        cx.notify();
    }

    fn org_conn(&self) -> Option<Conn> {
        let Phase::Main(s) = &self.phase else {
            return None;
        };
        let o = s.org.as_ref()?;
        let mut client = Client::new(&o.m.address, o.m.trust());
        client.token = o.token.clone();
        Some(Conn {
            client,
            member: o.m.member,
            signing: Zeroizing::new(*s.vault.signing_key().secret_bytes()),
            device: device_name(),
            remembered: o.local.remembered.clone(),
            code: None,
            new_remembered: None,
            setup_code: false,
        })
    }

    /// Called every second: sends local changes soon, and pulls every 30 s.
    pub fn org_sync_due(&self) -> bool {
        let Phase::Main(s) = &self.phase else {
            return false;
        };
        let Some(o) = &s.org else { return false };
        if o.busy || o.code.is_some() || s.gate.is_some() || self.demo {
            return false;
        }
        let since = o.last_attempt.map_or(i64::MAX, |t| cleis_core::now() - t);
        if o.error.is_some() {
            return since >= SYNC_EVERY_SECS;
        }
        o.again && since >= 1
            || since >= SYNC_EVERY_SECS
            || since >= 2 && s.vault.org_dirty(&o.m.key()).is_ok_and(|d| !d.is_empty())
    }

    /// Sends this vault's changes to the server, takes the others', refreshes the state.
    pub fn org_sync(&mut self, code: Option<String>, cx: &mut Context<Self>) {
        let Some(mut conn) = self.org_conn() else {
            return;
        };
        conn.code = code;
        let Some(s) = self.session() else { return };
        let Some(o) = s.org.as_mut() else { return };
        let key = o.m.key();
        let push = Changes {
            rows: s.vault.org_dirty(&key).unwrap_or_default(),
            purged: s.vault.org_unsent_purges(&key).unwrap_or_default(),
            seq: 0,
        };
        let since = s.vault.org_cursor(&key).unwrap_or(0);
        let full = std::mem::take(&mut o.full);
        o.busy = true;
        o.again = false;
        o.last_attempt = Some(cleis_core::now());
        cx.notify();
        let sent = push.clone();
        let task = cx.background_spawn(async move { sync_job(conn, push, since, full) });
        cx.spawn(async move |this, cx| {
            let out = task.await;
            this.update(cx, |app, cx| app.org_done(out, sent, cx)).ok();
        })
        .detach();
    }

    fn org_done(&mut self, out: Outcome, sent: Changes, cx: &mut Context<Self>) {
        let path = self.path.clone();
        let Some(s) = self.session() else { return };
        let Some(o) = s.org.as_mut() else { return };
        o.busy = false;
        match out {
            Outcome::Done {
                conn,
                changes,
                state,
            } => {
                o.token = conn.client.token.clone();
                if conn.new_remembered.is_some() {
                    o.local.remembered = conn.new_remembered.clone();
                }
                o.code = None;
                o.error = None;
                o.local.last_ok = Some(cleis_core::now());
                save_local(&path, &o.local);
                let _ = s.vault.org_pushed(&sent.rows, &sent.purged);
                apply_state(s, *state, &changes);
            }
            Outcome::NeedCode { wrong } => {
                let already = o.code.is_some();
                if let Some(c) = o.code.as_mut() {
                    c.busy = false;
                    c.error = Some(
                        if wrong {
                            "Wrong code"
                        } else {
                            "Enter the code"
                        }
                        .into(),
                    );
                }
                o.again = false;
                if !already {
                    self.org_code_prompt(None, wrong, cx);
                }
            }
            Outcome::SetupCode { conn, secret } => {
                o.token = conn.client.token.clone();
                o.again = false;
                self.org_code_prompt(Some(secret), false, cx);
            }
            Outcome::Revoked => {
                let name = o.m.name.clone();
                let key = o.m.key();
                let _ = s.vault.leave_org(&key);
                let _ = s.vault.set_org_setting(SETTING, &None::<Membership>);
                s.org = None;
                let _ = std::fs::remove_file(local_path(&path));
                let _ = crate::sync::reload(s);
                s.toast = Some(format!("Removed from {name}").into());
            }
            Outcome::Failed(e) => {
                o.error = Some(e);
                if let Some(c) = o.code.as_mut() {
                    c.busy = false;
                }
            }
        }
        self.org_followups(cx);
        cx.notify();
    }

    /// Keys this app can hand on: spaces given to members who have no key yet, the organization
    /// key for new admins, the organization key itself for its first admin.
    fn org_followups(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(o) = s.org.as_ref() else { return };
        let Some(st) = o.state.as_ref() else { return };
        let mut wraps = vec![];
        for p in &st.pending {
            let Some(sv) = st.spaces.iter().find(|x| x.id == p.space) else {
                continue;
            };
            if let Some(k) = space_key(s, o, sv)
                && let Ok(w) = wrap_key(&PublicKey(p.account_public), &k, p.space.0.as_bytes())
            {
                wraps.push(Wrap {
                    space: p.space,
                    member: p.member,
                    wrapped: w,
                });
            }
        }
        let mut admin = None;
        if st.me.role == ADMIN && st.org.org_public.is_none() {
            // First admin: the organization key is made here.
            if let Ok(kp) = KeyPair::generate() {
                let secret = SymmetricKey::from_bytes(*kp.secret_bytes());
                if let Ok(w) = wrap_key(&s.vault.account_public(), &secret, ORG_KEY_CONTEXT) {
                    admin = Some((
                        AdminKeys {
                            org_public: Some(kp.public().0.to_vec()),
                            wraps: vec![Wrap {
                                space: SpaceId(uuid::Uuid::nil()),
                                member: st.me.id,
                                wrapped: w,
                            }],
                        },
                        Some(kp),
                    ));
                }
            }
        } else if let Some(k) = &o.org_key
            && !st.pending_admins.is_empty()
        {
            let secret = SymmetricKey::from_bytes(*k.secret_bytes());
            let wraps = st
                .pending_admins
                .iter()
                .filter_map(|p| {
                    wrap_key(&PublicKey(p.account_public), &secret, ORG_KEY_CONTEXT)
                        .ok()
                        .map(|w| Wrap {
                            space: SpaceId(uuid::Uuid::nil()),
                            member: p.member,
                            wrapped: w,
                        })
                })
                .collect();
            admin = Some((
                AdminKeys {
                    org_public: None,
                    wraps,
                },
                None,
            ));
        }
        if !wraps.is_empty() {
            self.org_request::<_, PushReply>("/v1/grants", Grants { wraps }, |_, _, _| {}, cx);
        }
        if let Some((req, kp)) = admin {
            self.org_request::<_, PushReply>(
                "/v1/admin-keys",
                req,
                move |app, r, _| {
                    if r.is_ok()
                        && let Some(kp) = kp
                        && let Some(o) = app.session().and_then(|s| s.org.as_mut())
                    {
                        o.org_key = Some(kp);
                    }
                },
                cx,
            );
        }
    }

    /// One request in the background; `then` runs on the interface thread, then a sync follows.
    pub fn org_request<Q, R>(
        &mut self,
        path: &'static str,
        req: Q,
        then: impl FnOnce(&mut CleisApp, Result<R, String>, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) where
        Q: Serialize + Send + 'static,
        R: serde::de::DeserializeOwned + Send + 'static,
    {
        let Some(mut conn) = self.org_conn() else {
            return;
        };
        let task = cx.background_spawn(async move {
            let r: Result<R, NetError> = conn.call(path, &req);
            (r.map_err(|e| e.to_string()), conn.client.token.clone())
        });
        cx.spawn(async move |this, cx| {
            let (r, token) = task.await;
            this.update(cx, |app, cx| {
                if let Some(o) = app.session().and_then(|s| s.org.as_mut()) {
                    o.token = token;
                    o.again = true;
                }
                if let Err(e) = &r {
                    app.toast_for(e.clone(), 6, cx);
                }
                then(app, r, cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Shows the dialog asking the server's verification code (its box is made at the next
    /// render, which has the window).
    fn org_code_prompt(&mut self, secret: Option<String>, wrong: bool, cx: &mut Context<Self>) {
        if let Some(o) = self.session().and_then(|s| s.org.as_mut()) {
            o.code = Some(CodePrompt {
                input: None,
                error: wrong.then(|| "Wrong code".into()),
                secret,
                busy: false,
            });
        }
        cx.notify();
    }

    pub fn org_submit_code(&mut self, cx: &mut Context<Self>) {
        let Some(o) = self.session().and_then(|s| s.org.as_mut()) else {
            return;
        };
        let Some(c) = o.code.as_mut() else { return };
        let Some(input) = c.input.as_ref() else {
            return;
        };
        let typed = input.read(cx).value().to_string();
        if typed.chars().filter(char::is_ascii_digit).count() != 6 {
            c.error = Some("Six digits".into());
            cx.notify();
            return;
        }
        c.busy = true;
        if c.secret.is_some() {
            self.org_request::<_, SessionReply>(
                "/v1/me/code/confirm",
                CodeCheck { code: typed },
                |app, r, cx| {
                    if let Some(o) = app.session().and_then(|s| s.org.as_mut()) {
                        match r {
                            // The session stays: the server lifted its set-up restriction.
                            Ok(_) => o.code = None,
                            Err(_) => {
                                if let Some(c) = o.code.as_mut() {
                                    c.busy = false;
                                    c.error = Some("Wrong code".into());
                                }
                            }
                        }
                    }
                    cx.notify();
                },
                cx,
            );
        } else {
            self.org_sync(Some(typed), cx);
        }
        cx.notify();
    }

    pub fn org_cancel_code(&mut self, cx: &mut Context<Self>) {
        if let Some(o) = self.session().and_then(|s| s.org.as_mut()) {
            o.code = None;
            o.error = Some("Not signed in".into());
        }
        cx.notify();
    }

    /// Joins an organization from an open vault (Settings › Organization).
    pub fn org_join(
        &mut self,
        address: String,
        code: String,
        name: String,
        email: String,
        cx: &mut Context<Self>,
    ) {
        let Some(s) = self.session() else { return };
        let public = s.vault.account_public().0;
        let signing = SigningKeyPair::from_secret_bytes(*s.vault.signing_key().secret_bytes());
        let task =
            cx.background_spawn(
                async move { join(&address, &code, &name, &email, public, &signing) },
            );
        cx.spawn(async move |this, cx| {
            let r = task.await;
            this.update(cx, |app, cx| app.org_joined(r, cx)).ok();
        })
        .detach();
    }

    pub fn org_joined(&mut self, r: Result<Membership, String>, cx: &mut Context<Self>) {
        match r {
            Ok(m) => {
                let name = m.name.clone();
                if let Some(s) = self.session() {
                    let _ = s.vault.set_org_setting(SETTING, &Some(m));
                }
                self.load_org(cx);
                self.toast(format!("Joined {name}"), cx);
            }
            Err(e) => {
                if let Some(s) = self.session() {
                    s.join.error = Some(e.into());
                    s.join.busy = false;
                }
            }
        }
        cx.notify();
    }

    /// Leaves the organization on this vault (its spaces are forgotten here).
    pub fn org_leave(&mut self, cx: &mut Context<Self>) {
        let path = self.path.clone();
        if let Some(s) = self.session()
            && let Some(o) = s.org.take()
        {
            let _ = s.vault.leave_org(&o.m.key());
            let _ = s.vault.set_org_setting(SETTING, &None::<Membership>);
            let _ = std::fs::remove_file(local_path(&path));
            let _ = crate::sync::reload(s);
            s.toast = Some(format!("Left {}", o.m.name).into());
        }
        cx.notify();
    }

    // ---------- Admin actions ----------

    pub fn org_invite(&mut self, req: NewInvite, cx: &mut Context<Self>) {
        self.org_request::<_, InviteReply>(
            "/v1/invites",
            req,
            |app, r, cx| {
                if let (Ok(r), Some(o)) = (r, app.session().and_then(|s| s.org.as_mut())) {
                    o.ui.invite_code = Some(r.code);
                }
                cx.notify();
            },
            cx,
        );
    }

    pub fn org_member(&mut self, change: MemberChange, cx: &mut Context<Self>) {
        self.org_request::<_, MemberView>("/v1/members", change, |_, _, _| {}, cx);
    }

    pub fn org_revoke(&mut self, member: MemberId, cx: &mut Context<Self>) {
        self.org_request::<_, PushReply>(
            "/v1/members/revoke",
            IdOnly { id: member.0 },
            |app, r, cx| {
                if r.is_ok() {
                    app.toast("Removed · re-encrypt its spaces", cx);
                }
            },
            cx,
        );
    }

    pub fn org_invite_action(&mut self, id: uuid::Uuid, renew: bool, cx: &mut Context<Self>) {
        if renew {
            self.org_request::<_, InviteReply>(
                "/v1/invites/renew",
                IdOnly { id },
                |app, r, cx| {
                    if let (Ok(r), Some(o)) = (r, app.session().and_then(|s| s.org.as_mut())) {
                        o.ui.invite_code = Some(r.code);
                    }
                    cx.notify();
                },
                cx,
            );
        } else {
            self.org_request::<_, PushReply>("/v1/invites/revoke", IdOnly { id }, |_, _, _| {}, cx);
        }
    }

    pub fn org_set_roles(&mut self, roles: Vec<Role>, cx: &mut Context<Self>) {
        if let Some(st) = self
            .session()
            .and_then(|s| s.org.as_mut())
            .and_then(|o| o.state.as_mut())
        {
            st.roles = roles.clone();
        }
        self.org_request::<_, Vec<Role>>("/v1/roles", roles, |_, _, _| {}, cx);
    }

    pub fn org_set_policy(&mut self, f: impl FnOnce(&mut Policy), cx: &mut Context<Self>) {
        let Some(st) = self
            .session()
            .and_then(|s| s.org.as_mut())
            .and_then(|o| o.state.as_mut())
        else {
            return;
        };
        f(&mut st.policy);
        let p = st.policy.clone();
        self.org_request::<_, Policy>("/v1/policy", p, |_, _, _| {}, cx);
        cx.notify();
    }

    /// A new organization space, its key kept here and wrapped for the organization key
    /// (Standard) and the recovery key (if on).
    pub fn org_new_space(&mut self, name: String, mode: SpaceMode, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(o) = s.org.as_ref() else { return };
        let Some(st) = o.state.as_ref() else { return };
        let info = SpaceInfo {
            id: SpaceId::new(),
            name: name.trim().into(),
            kind: SpaceKind::Organization,
            mode: match mode {
                SpaceMode::Standard => EncryptionMode::Standard,
                SpaceMode::EndToEnd => EncryptionMode::EndToEnd,
            },
            for_contacts: false,
        };
        let org_public = st.org.org_public.clone();
        let recovery = st.org.recovery_public.clone();
        let key = o.m.key();
        let Ok((sealed, wrapped, k)) = s.vault.create_org_space(&key, info.clone()) else {
            return;
        };
        let wrap_to = |p: &Option<Vec<u8>>| {
            p.as_ref()
                .and_then(|p| p.clone().try_into().ok())
                .and_then(|p: [u8; 32]| wrap_key(&PublicKey(p), &k, info.id.0.as_bytes()).ok())
        };
        let req = NewSpace {
            id: info.id,
            sealed,
            mode,
            wrapped,
            org_wrapped: if mode == SpaceMode::Standard {
                wrap_to(&org_public)
            } else {
                None
            },
            recovery_wrapped: wrap_to(&recovery),
        };
        if let Some(o) = s.org.as_mut() {
            o.names.insert(info.id, info.name.clone());
        }
        let _ = crate::sync::reload(s);
        let id = info.id;
        self.org_request::<_, PushReply>(
            "/v1/spaces",
            req,
            move |app, r, cx| {
                if r.is_err()
                    && let Some(s) = app.session()
                {
                    let _ = s.vault.remove_org_space(id);
                    let _ = crate::sync::reload(s);
                } else if let Some(s) = app.session() {
                    s.filter = crate::app::Filter::Space(id);
                    s.refilter();
                }
                cx.notify();
            },
            cx,
        );
    }

    /// Re-encrypts a space (screen 10): new key, every item sealed again, the key wrapped for
    /// each member who has the space.
    pub fn org_rekey(&mut self, space: SpaceId, mode: SpaceMode, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(o) = s.org.as_ref() else { return };
        let Some(st) = o.state.as_ref() else { return };
        let Some(sv) = st.spaces.iter().find(|x| x.id == space) else {
            return;
        };
        let Some(mut info) = s.spaces.iter().find(|x| x.id == space).cloned() else {
            self.toast("Sync first: this space's key is not here yet", cx);
            return;
        };
        info.mode = match mode {
            SpaceMode::Standard => EncryptionMode::Standard,
            SpaceMode::EndToEnd => EncryptionMode::EndToEnd,
        };
        let Ok(r) = s.vault.org_prepare_rekey(space, &info) else {
            return;
        };
        let public_of = |id: MemberId| {
            st.members
                .iter()
                .find(|m| m.id == id)
                .map(|m| m.account_public)
        };
        let wraps: Vec<Wrap> = sv
            .members
            .iter()
            .filter_map(|m| {
                let p = public_of(*m)?;
                Some(Wrap {
                    space,
                    member: *m,
                    wrapped: wrap_key(&PublicKey(p), &r.key, space.0.as_bytes()).ok()?,
                })
            })
            .collect();
        let wrap_to = |p: &Option<Vec<u8>>| {
            p.as_ref()
                .and_then(|p| p.clone().try_into().ok())
                .and_then(|p: [u8; 32]| wrap_key(&PublicKey(p), &r.key, space.0.as_bytes()).ok())
        };
        let req = Rekey {
            space,
            key_gen: r.key_gen,
            sealed: r.sealed.clone(),
            mode,
            wraps,
            org_wrapped: if mode == SpaceMode::Standard {
                wrap_to(&st.org.org_public)
            } else {
                None
            },
            recovery_wrapped: wrap_to(&st.org.recovery_public),
            rows: r.rows.clone(),
        };
        let key = o.m.key();
        let r = std::sync::Arc::new(r);
        self.org_request::<_, PushReply>(
            "/v1/spaces/rekey",
            req,
            move |app, res, cx| {
                if res.is_ok()
                    && let Some(s) = app.session()
                {
                    let _ = s.vault.org_rekeyed(&key, &r);
                    let _ = crate::sync::reload(s);
                    if let Some(o) = s.org.as_mut() {
                        o.ui.rekey = None;
                    }
                    s.toast = Some("Re-encrypted".into());
                }
                cx.notify();
            },
            cx,
        );
    }

    /// Recovery key on: a new key pair, its private half saved to a file chosen by the admin,
    /// every space the app can open wrapped to it. Off: the copies are dropped.
    pub fn org_recovery(&mut self, on: bool, cx: &mut Context<Self>) {
        if !on {
            self.org_request::<_, OrgInfo>(
                "/v1/recovery",
                Recovery {
                    public: vec![],
                    wraps: vec![],
                },
                |_, _, _| {},
                cx,
            );
            return;
        }
        let name = self
            .session()
            .and_then(|s| s.org.as_ref())
            .map(|o| o.m.name.clone())
            .unwrap_or_default();
        let dir = directories::UserDirs::new()
            .and_then(|u| u.document_dir().map(Path::to_path_buf))
            .unwrap_or_else(std::env::temp_dir);
        let rx = cx.prompt_for_new_path(&dir, Some(&format!("{name} recovery key.txt")));
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(path))) = rx.await {
                this.update(cx, |app, cx| app.org_recovery_to(&path, cx))
                    .ok();
            }
        })
        .detach();
    }

    pub fn org_recovery_to(&mut self, path: &Path, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(o) = s.org.as_ref() else { return };
        let Some(st) = o.state.as_ref() else { return };
        let Ok(kp) = KeyPair::generate() else { return };
        let text = format!(
            "Cleis recovery key — {}\nKeep it outside Cleis. Anyone holding it can open the organization's spaces.\n\n{}\n",
            o.m.name,
            recovery_text(&kp)
        );
        if let Err(e) = std::fs::write(path, text) {
            s.toast = Some(format!("Not saved: {e}").into());
            cx.notify();
            return;
        }
        let wraps = st
            .spaces
            .iter()
            .filter_map(|sv| {
                let k = space_key(s, o, sv)?;
                Some(Wrap {
                    space: sv.id,
                    member: o.m.member,
                    wrapped: wrap_key(&kp.public(), &k, sv.id.0.as_bytes()).ok()?,
                })
            })
            .collect();
        self.org_request::<_, OrgInfo>(
            "/v1/recovery",
            Recovery {
                public: kp.public().0.to_vec(),
                wraps,
            },
            |app, r, cx| {
                if r.is_ok() {
                    app.toast("Recovery key saved", cx);
                }
            },
            cx,
        );
    }

    /// Restores members' access with the recovery key file: every space it opens is wrapped
    /// again for the members waiting for it.
    pub fn org_use_recovery(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(gpui_kit::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Use recovery key".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = rx.await
                && let Some(p) = paths.into_iter().next()
            {
                this.update(cx, |app, cx| app.org_recovery_from(&p, cx))
                    .ok();
            }
        })
        .detach();
    }

    pub fn org_recovery_from(&mut self, path: &Path, cx: &mut Context<Self>) {
        let Some(kp) = std::fs::read_to_string(path)
            .ok()
            .and_then(|t| parse_recovery(&t))
        else {
            self.toast("Not a Cleis recovery key", cx);
            return;
        };
        let Some(s) = self.session() else { return };
        let Some(o) = s.org.as_ref() else { return };
        let Some(st) = o.state.as_ref() else { return };
        let mut wraps = vec![];
        for p in &st.pending {
            let Some(sv) = st.spaces.iter().find(|x| x.id == p.space) else {
                continue;
            };
            let Some(k) = sv
                .recovery_wrapped
                .as_ref()
                .and_then(|w| cleis_crypto::unwrap_key(&kp, w, sv.id.0.as_bytes()).ok())
            else {
                continue;
            };
            if let Ok(w) = wrap_key(&PublicKey(p.account_public), &k, sv.id.0.as_bytes()) {
                wraps.push(Wrap {
                    space: p.space,
                    member: p.member,
                    wrapped: w,
                });
            }
        }
        let n = wraps.len();
        if n == 0 {
            self.toast("Nobody is waiting for a space this key opens", cx);
            return;
        }
        self.org_request::<_, PushReply>(
            "/v1/grants",
            Grants { wraps },
            move |app, r, cx| {
                if r.is_ok() {
                    app.toast(format!("Access restored · {n}"), cx);
                }
            },
            cx,
        );
    }

    pub fn org_rotate(&mut self, cx: &mut Context<Self>) {
        self.org_request::<_, OrgInfo>(
            "/v1/rotate",
            (),
            |app, r, cx| {
                if let Ok(info) = r {
                    app.org_trust_next(&info);
                    app.toast("New server key announced", cx);
                }
            },
            cx,
        );
    }

    /// Trusts a certificate announced over the pinned connection.
    fn org_trust_next(&mut self, info: &OrgInfo) {
        let Some(s) = self.session() else { return };
        let Some(o) = s.org.as_mut() else { return };
        let mut changed = false;
        for c in [Some(info.cert.to_vec()), info.next_cert.clone()]
            .into_iter()
            .flatten()
        {
            let h = hex(&c);
            if c.len() == 32 && !o.m.pins.contains(&h) {
                o.m.pins.push(h);
                changed = true;
            }
        }
        if changed {
            let m = o.m.clone();
            let _ = s.vault.set_org_setting(SETTING, &Some(m));
        }
    }

    // ---------- Activity reported by the app ----------

    /// Views, copies, fills of an organization's item (written if the policy records them).
    pub fn org_report(
        &mut self,
        action: Action,
        entry: &Entry,
        field: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(s) = self.session() else { return };
        let Some(o) = s.org.as_ref() else { return };
        let Some(st) = o.state.as_ref() else { return };
        if !s.vault.is_org_space(entry.space).unwrap_or(false) || !st.policy.records(action) {
            return;
        }
        let detail = st
            .org
            .org_public
            .as_ref()
            .and_then(|p| p.clone().try_into().ok())
            .and_then(|p: [u8; 32]| {
                let d = serde_json::to_vec(&EventDetail {
                    item: entry.name.clone(),
                    field: field.to_owned(),
                })
                .ok()?;
                cleis_crypto::seal_to(&PublicKey(p), DETAIL_CONTEXT, &d).ok()
            });
        let req = Report {
            action,
            space: Some(entry.space),
            item: Some(format!("entry/{}", entry.id)),
            detail,
        };
        let Some(mut conn) = self.org_conn() else {
            return;
        };
        // Fire and forget: a lost report is not worth an error message.
        cx.background_spawn(async move {
            let _: Result<PushReply, _> = conn.call("/v1/log", &req);
        })
        .detach();
    }

    pub fn org_query_log(&mut self, cx: &mut Context<Self>) {
        let Some(o) = self.session().and_then(|s| s.org.as_mut()) else {
            return;
        };
        let q = o.ui.log_query();
        o.ui.log_busy = true;
        self.org_request::<_, LogPage>(
            "/v1/log/query",
            q,
            |app, r, cx| {
                if let Some(o) = app.session().and_then(|s| s.org.as_mut()) {
                    o.ui.log_busy = false;
                    if let Ok(p) = r {
                        o.ui.log = Some(p);
                    }
                }
                cx.notify();
            },
            cx,
        );
    }

    pub fn org_verify_log(&mut self, cx: &mut Context<Self>) {
        self.org_request::<_, Integrity>(
            "/v1/log/verify",
            (),
            |app, r, cx| {
                if let Ok(i) = r {
                    if let Some(st) = app
                        .session()
                        .and_then(|s| s.org.as_mut())
                        .and_then(|o| o.state.as_mut())
                    {
                        st.integrity = Some(i.clone());
                    }
                    if i.intact {
                        app.toast(format!("Log intact · {} events", i.checked), cx);
                    } else {
                        app.toast_for(
                            format!("Log altered: {}", i.problem.unwrap_or_default()),
                            8,
                            cx,
                        );
                    }
                }
            },
            cx,
        );
    }

    /// Opens an event's sealed detail (admins).
    pub fn org_detail(&self, e: &Event) -> Option<EventDetail> {
        let Phase::Main(s) = &self.phase else {
            return None;
        };
        let k = s.org.as_ref()?.org_key.as_ref()?;
        let plain = cleis_crypto::open_from(k, DETAIL_CONTEXT, e.detail.as_ref()?).ok()?;
        serde_json::from_slice(&plain).ok()
    }

    /// Writes the events shown to a CSV file.
    pub fn org_export_log(&mut self, cx: &mut Context<Self>) {
        let Phase::Main(s) = &self.phase else { return };
        let Some(o) = s.org.as_ref() else { return };
        let Some(page) = o.ui.log.as_ref() else {
            return;
        };
        let mut csv = String::from("Date,Member,Action,Item,Space,Device\n");
        for e in &page.events {
            let item = self
                .org_detail(e)
                .map(|d| {
                    if d.field.is_empty() {
                        d.item
                    } else {
                        format!("{} · {}", d.item, d.field)
                    }
                })
                .unwrap_or_else(|| e.note.clone());
            let row = [
                crate::format::date_time(e.at),
                e.member.map(|m| o.member_name(m)).unwrap_or_default(),
                e.action.label().into(),
                item,
                e.space.map(|x| o.space_name(x)).unwrap_or_default(),
                e.device.clone(),
            ];
            let cells: Vec<String> = row
                .iter()
                .map(|c| format!("\"{}\"", c.replace('"', "\"\"")))
                .collect();
            csv.push_str(&cells.join(","));
            csv.push('\n');
        }
        let dir = directories::UserDirs::new()
            .and_then(|u| u.document_dir().map(Path::to_path_buf))
            .unwrap_or_else(std::env::temp_dir);
        let name = format!("{} activity.csv", o.m.name);
        let rx = cx.prompt_for_new_path(&dir, Some(&name));
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(path))) = rx.await {
                let r = std::fs::write(&path, csv);
                this.update(cx, |app, cx| {
                    app.toast(
                        match r {
                            Ok(_) => "Exported".to_owned(),
                            Err(e) => format!("Not exported: {e}"),
                        },
                        cx,
                    )
                })
                .ok();
            }
        })
        .detach();
    }

    // ---------- Organization backups (screen 13) ----------

    /// Copies the server's file, sealed with the backup key, into this computer's backups.
    pub fn org_backup_now(&mut self, cx: &mut Context<Self>) {
        let vault = self.path.clone();
        let Some(s) = self.session() else { return };
        let Some(o) = s.org.as_ref() else { return };
        let key = match backup_key(&s.vault) {
            Ok(k) => k,
            Err(e) => {
                s.toast = Some(e.into());
                cx.notify();
                return;
            }
        };
        let dir = org_backups_dir(&vault, &o.m);
        let Some(mut conn) = self.org_conn() else {
            return;
        };
        let task = cx.background_spawn(async move {
            let bytes = conn
                .send("GET", "/v1/backup", None)
                .map_err(|e| e.to_string())?;
            let sealed =
                cleis_crypto::seal(&key, BACKUP_CONTEXT, &bytes).map_err(|e| e.to_string())?;
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let name = format!(
                "server-{}.cleisorg",
                crate::format::stamp(cleis_core::now())
            );
            std::fs::write(dir.join(name), sealed).map_err(|e| e.to_string())?;
            Ok::<usize, String>(bytes.len())
        });
        cx.spawn(async move |this, cx| {
            let r = task.await;
            this.update(cx, |app, cx| {
                app.toast(
                    match r {
                        Ok(n) => {
                            format!("Organization backed up · {}", crate::format::size(n as u64))
                        }
                        Err(e) => format!("Not backed up: {e}"),
                    },
                    cx,
                );
            })
            .ok();
        })
        .detach();
    }

    /// Opens a backup with the backup key and checks it is a readable server file.
    pub fn org_verify_backup(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let r = backup_key(&s.vault)
            .and_then(|k| open_backup(&k, &path))
            .map(|b| b.len());
        let msg = match r {
            Ok(n) => format!("Backup readable · {}", crate::format::size(n as u64)),
            Err(e) => format!("Backup unreadable: {e}"),
        };
        self.toast(msg, cx);
    }

    pub fn org_restore_backup(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let bytes = match backup_key(&s.vault).and_then(|k| open_backup(&k, &path)) {
            Ok(b) => b,
            Err(e) => {
                s.toast = Some(format!("Backup unreadable: {e}").into());
                cx.notify();
                return;
            }
        };
        let Some(mut conn) = self.org_conn() else {
            return;
        };
        conn.client.timeout = std::time::Duration::from_secs(120);
        let task = cx.background_spawn(async move {
            conn.send("POST", "/v1/restore", Some(&bytes))
                .map_err(|e| e.to_string())
        });
        cx.spawn(async move |this, cx| {
            let r = task.await;
            this.update(cx, |app, cx| {
                if let Some(o) = app.session().and_then(|s| s.org.as_mut()) {
                    o.token = None;
                    o.again = true;
                }
                app.toast(
                    match r {
                        Ok(_) => "Organization restored".to_owned(),
                        Err(e) => format!("Not restored: {e}"),
                    },
                    cx,
                );
            })
            .ok();
        })
        .detach();
    }

    /// Saves the backup key to a file chosen by the admin.
    pub fn org_download_backup_key(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let key = match backup_key(&s.vault) {
            Ok(k) => k,
            Err(e) => {
                s.toast = Some(e.into());
                return;
            }
        };
        let name = s.org.as_ref().map(|o| o.m.name.clone()).unwrap_or_default();
        let text = format!(
            "Cleis backup key — {name}\nWithout it, no organization backup can be restored. Keep it outside Cleis.\n\n{}\n",
            data_encoding::BASE32_NOPAD.encode(key.as_bytes())
        );
        let dir = directories::UserDirs::new()
            .and_then(|u| u.document_dir().map(Path::to_path_buf))
            .unwrap_or_else(std::env::temp_dir);
        let rx = cx.prompt_for_new_path(&dir, Some(&format!("{name} backup key.txt")));
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(path))) = rx.await {
                let r = std::fs::write(&path, text);
                this.update(cx, |app, cx| {
                    if r.is_ok()
                        && let Some(s) = app.session()
                    {
                        let mut p = s.prefs.clone();
                        p.org_backup_key_saved = Some(cleis_core::now());
                        let _ = s.vault.set_preferences(&p);
                        s.prefs = p;
                    }
                    app.toast(
                        if r.is_ok() {
                            "Backup key saved"
                        } else {
                            "Not saved"
                        },
                        cx,
                    );
                })
                .ok();
            }
        })
        .detach();
    }
}

const BACKUP_CONTEXT: &[u8] = b"cleis/org-backup/v1";

/// The organization backups' key, made on first use and sealed in this vault's settings.
fn backup_key(vault: &cleis_store::Vault) -> Result<SymmetricKey, String> {
    let stored: Option<Vec<u8>> = vault
        .org_setting("org_backup_key")
        .map_err(|e| e.to_string())?;
    if let Some(b) = stored {
        return SymmetricKey::from_slice(&b).map_err(|e| e.to_string());
    }
    let k = SymmetricKey::generate().map_err(|e| e.to_string())?;
    vault
        .set_org_setting("org_backup_key", &k.as_bytes().to_vec())
        .map_err(|e| e.to_string())?;
    Ok(k)
}

fn open_backup(key: &SymmetricKey, path: &Path) -> Result<Vec<u8>, String> {
    let sealed = std::fs::read(path).map_err(|e| e.to_string())?;
    let bytes = cleis_crypto::open(key, BACKUP_CONTEXT, &sealed)
        .map_err(|_| "wrong backup key".to_owned())?;
    if !bytes.starts_with(b"SQLite format 3") {
        return Err("not a server file".into());
    }
    Ok(bytes)
}

pub fn org_backups_dir(vault: &Path, m: &Membership) -> PathBuf {
    crate::sync::backups_dir(vault).join(format!("org-{}", m.org))
}

/// Organization backups, most recent first.
pub fn org_backups(vault: &Path, m: &Membership) -> Vec<(PathBuf, u64, i64)> {
    let mut v: Vec<(PathBuf, u64, i64)> = std::fs::read_dir(org_backups_dir(vault, m))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "cleisorg"))
        .filter_map(|e| {
            let md = e.metadata().ok()?;
            let t = md
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_secs() as i64;
            Some((e.path(), md.len(), t))
        })
        .collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.2));
    v
}

/// The recovery key as typed or printed: base32 of the X25519 private key, in groups.
pub fn recovery_text(kp: &KeyPair) -> String {
    let s = data_encoding::BASE32_NOPAD.encode(&kp.secret_bytes()[..]);
    s.as_bytes()
        .chunks(4)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect::<Vec<_>>()
        .join("-")
}

pub fn parse_recovery(text: &str) -> Option<KeyPair> {
    text.lines().find_map(|l| {
        let s: String = l.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        let b = data_encoding::BASE32_NOPAD
            .decode(s.to_ascii_uppercase().as_bytes())
            .ok()?;
        let arr: [u8; 32] = b.try_into().ok()?;
        Some(KeyPair::from_secret_bytes(arr))
    })
}

/// Hides the organization's entries (offline longer than the policy allows).
fn hide_org_entries(s: &mut Session) {
    let org_spaces: Vec<SpaceId> = s
        .org
        .as_ref()
        .map(|o| {
            s.vault
                .org_spaces(&o.m.key())
                .unwrap_or_default()
                .into_iter()
                .map(|x| x.0)
                .collect()
        })
        .unwrap_or_default();
    s.entries.retain(|e| !org_spaces.contains(&e.space));
    s.spaces.retain(|sp| !org_spaces.contains(&sp.id));
    s.refilter();
}

/// Applies a sync: keys of the spaces given or taken away, rows, the organization key.
fn apply_state(s: &mut Session, st: State, changes: &Changes) {
    let Some(o) = s.org.as_mut() else { return };
    let key = o.m.key();
    let mut changed = false;
    let here: BTreeMap<SpaceId, u32> = s
        .vault
        .org_spaces(&key)
        .unwrap_or_default()
        .into_iter()
        .collect();
    let mut keep = vec![];
    for sv in &st.spaces {
        let Some(w) = &sv.wrapped else { continue };
        if !sv.members.contains(&st.me.id) {
            continue;
        }
        keep.push(sv.id);
        let gen_here = here.get(&sv.id).copied();
        if gen_here != Some(sv.key_gen) || s.vault.space_key(sv.id).is_none() {
            if s.vault
                .put_org_space(&key, sv.id, &sv.sealed, w, sv.key_gen)
                .is_ok()
            {
                changed = true;
                o.full.push(sv.id);
                o.again = true;
            }
        } else {
            // Renamed elsewhere: same key, new sealed info.
            let _ = s
                .vault
                .put_org_space(&key, sv.id, &sv.sealed, w, sv.key_gen);
        }
    }
    for id in here.keys() {
        if !keep.contains(id) {
            let _ = s.vault.remove_org_space(*id);
            changed = true;
        }
    }
    if s.vault
        .org_apply(&changes.rows, &changes.purged)
        .unwrap_or(0)
        > 0
    {
        changed = true;
    }
    let _ = s.vault.set_org_cursor(&key, changes.seq);
    if o.org_key.is_none()
        && let Some(w) = &st.org_key
    {
        o.org_key = s.vault.unwrap_own(w, ORG_KEY_CONTEXT).ok();
    }
    // Names of the spaces this app can open (its own keys, or the organization key).
    let mut names = BTreeMap::new();
    for sv in &st.spaces {
        let k = s
            .vault
            .space_key(sv.id)
            .cloned()
            .or_else(|| o.key_via_org(sv));
        if let Some(k) = k
            && let Ok(info) = cleis_store::open_space_info(&k, sv.id, &sv.sealed)
        {
            names.insert(sv.id, info.name);
        }
    }
    o.names = names;
    // New certificate announced: trusted from now on.
    if let Some(next) = &st.org.next_cert {
        let h = hex(next);
        if next.len() == 32 && !o.m.pins.contains(&h) {
            o.m.pins.push(h);
            let m = o.m.clone();
            let _ = s.vault.set_org_setting(SETTING, &Some(m));
        }
    }
    o.m.name.clone_from(&st.org.name);
    let was_locked = o.offline_locked;
    o.offline_locked = false;
    o.state = Some(st);
    if changed || was_locked {
        let _ = crate::sync::reload(s);
    }
}
