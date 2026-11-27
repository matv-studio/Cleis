//! The organization server seen from the app (D-26, D-32, D-59).
//!
//! Membership (server address, pinned certificates, member id) is sealed in the vault's
//! settings, so another computer opening the same vault knows it too. The sign-in token, the
//! "remembered device" token and the last contact stay on this computer (`<vault>.org.json`).
//!
//! Network calls run off the interface thread ([`Conn`], blocking); what they bring back is
//! applied to the vault on the interface thread ([`CleisApp::org_done`]).

use crate::app::{CleisApp, Phase, Session};
use crate::i18n::tr;
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

/// `Membership::address` of an organization in a shared folder (D-78): the folder itself is
/// this computer's (`Local::folder`), as it is not at the same place on every computer.
pub const FOLDER: &str = "folder";

impl Membership {
    pub fn key(&self) -> String {
        self.org.to_string()
    }

    pub fn in_folder(&self) -> bool {
        self.address == FOLDER
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
    /// The organization's server runs on this computer, from this folder (D-63).
    pub host_dir: Option<PathBuf>,
    /// The policy's offline access, as last received (audit L-3).
    #[serde(default)]
    pub offline_days: Option<u32>,
    /// The policy's PIN (D-79), as last received: asked again after this many hours.
    pub pin_hours: Option<u32>,
    /// Its length, as last received: the boxes shown before the server answers.
    #[serde(default)]
    pub pin_len: Option<u8>,
    /// The PIN was last given on this computer until then: the organization's spaces open
    /// without asking it again before.
    pub pin_until: Option<i64>,
    /// What the organization asks (D-98), as last received; `None`: nothing.
    pub access_with: Option<AccessWith>,
    /// Asked again after this many minutes without using Cleis (D-98).
    pub idle_mins: Option<u32>,
    /// Wrong tries allowed before this computer locks (shared folder, D-98).
    pub tries: u32,
    /// Shared folder (D-98): the digest checked here, as last received (it is in the folder).
    pub secret: Option<AccessSecret>,
    /// Shared folder: wrong tries in a row on this computer, and when it locked itself.
    pub fails: u32,
    pub locked_at: Option<i64>,
    /// The organization's logo (PNG, base64), shown when it asks (D-98).
    pub logo: Option<String>,
    /// Shared folder (D-78): where the organization's folder is on this computer.
    pub folder: Option<PathBuf>,
}

/// This computer's copy of an organization kept in a shared folder (D-78).
pub fn replica_dir(vault: &Path, key: &str) -> PathBuf {
    let mut p = vault.as_os_str().to_owned();
    p.push(format!(".org-{key}.replica"));
    PathBuf::from(p)
}

/// An organization in a shared folder: what opens its copy (off the interface thread).
#[derive(Clone)]
pub struct FolderSpec {
    pub key: String,
    pub folder: Option<PathBuf>,
    pub replica: PathBuf,
    pub member: Option<MemberId>,
    /// SFTP folder: how this vault signs in there (kept sealed in the vault).
    pub sftp: Option<SftpSecret>,
}

/// An SFTP shared folder's sign-in and the server's key, sealed in the vault (D-78).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SftpSecret {
    pub secret: crate::remote::Secret,
    pub host_key: String,
}

pub fn sftp_setting(key: &str) -> String {
    format!("org_sftp:{key}")
}

/// This computer's file for the organization `key` (one per organization, D-69); the first
/// version kept a single `.org.json`, still read for the first organization.
pub fn local_path(vault: &Path, key: &str) -> PathBuf {
    let mut p = vault.as_os_str().to_owned();
    p.push(format!(".org-{key}.json"));
    PathBuf::from(p)
}

fn legacy_local_path(vault: &Path) -> PathBuf {
    let mut p = vault.as_os_str().to_owned();
    p.push(".org.json");
    PathBuf::from(p)
}

pub fn load_local(vault: &Path, key: &str) -> Local {
    let read = |p: PathBuf| {
        std::fs::read(p)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
    };
    read(local_path(vault, key))
        .or_else(|| read(legacy_local_path(vault)))
        .unwrap_or_default()
}

pub fn save_local(vault: &Path, key: &str, l: &Local) {
    let _ = std::fs::write(
        local_path(vault, key),
        serde_json::to_vec_pretty(l).unwrap_or_default(),
    );
}

/// Vault setting holding this device's "remembered device" token for an organization, sealed
/// like every setting (audit S-14: no longer in clear in the local file).
pub fn remembered_setting(device: &str, key: &str) -> String {
    format!("org_remembered:{device}:{key}")
}

pub fn remove_local(vault: &Path, key: &str) {
    let _ = std::fs::remove_file(local_path(vault, key));
    let _ = std::fs::remove_file(legacy_local_path(vault));
    // Shared folder (D-78): this computer's copy goes too (the folder itself stays).
    let replica = replica_dir(vault, key);
    crate::folder_org::forget(&replica.display().to_string());
    let _ = std::fs::remove_dir_all(replica);
}

/// Setting holding every membership of this vault (D-69); `SETTING` keeps the first, as before.
pub const SETTING_ALL: &str = "orgs";

/// The organizations this vault belongs to, the first one first.
pub fn memberships(vault: &cleis_store::Vault) -> Vec<Membership> {
    match vault.org_setting::<Vec<Membership>>(SETTING_ALL) {
        Ok(Some(all)) => all,
        _ => vault
            .org_setting::<Option<Membership>>(SETTING)
            .ok()
            .flatten()
            .flatten()
            .into_iter()
            .collect(),
    }
}

/// Saves the memberships as the session has them (the one shown first, then the others).
pub fn save_memberships(s: &crate::app::Session) {
    let all: Vec<Membership> = s.all_orgs().map(|o| o.m.clone()).collect();
    let _ = s.vault.set_org_setting(SETTING_ALL, &all);
    let _ = s.vault.set_org_setting(SETTING, &all.first());
}

/// This computer's name (Settings › Organization › Server › This computer).
pub fn computer_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .filter(|h| !h.trim().is_empty())
        .or_else(|| {
            std::fs::read_to_string("/etc/hostname")
                .ok()
                .map(|s| s.trim().to_owned())
        })
        .unwrap_or_else(|| tr("Computer").into())
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
        .unwrap_or_else(|| tr("Computer").into());
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
    /// ID token from Google / Microsoft for this sign-in (D-64).
    pub id_token: Option<String>,
    /// Set when a sign-in gave a new "remembered device" token.
    pub new_remembered: Option<String>,
    pub setup_code: bool,
    /// The organization's PIN typed (D-79), kept for the sign-ins of this job.
    pub pin: Option<Zeroizing<String>>,
    /// Shared folder (D-78): no server, the API answered in the app.
    pub folder: Option<FolderSpec>,
}

/// The shared folder's API, opened if needed (rebuilds the copy: off the interface thread).
pub fn folder_api(
    f: &FolderSpec,
    device: &str,
    signing: &Zeroizing<[u8; 32]>,
) -> Result<cleis_net::LocalApi, NetError> {
    if let Some(api) = crate::folder_org::api_of(&f.key) {
        return Ok(api);
    }
    let folder = f.folder.as_ref().ok_or_else(|| {
        NetError::Unreachable(tr("Choose the organization's shared folder").into())
    })?;
    let store = crate::folder_store::store_for(
        folder,
        f.sftp.as_ref().map(|s| &s.secret),
        f.sftp.as_ref().map_or("", |s| s.host_key.as_str()),
    )
    .map_err(NetError::Unreachable)?;
    let engine = crate::folder_org::Engine::open_store(
        store,
        &folder.display().to_string(),
        &f.replica,
        f.member,
        device,
        SigningKeyPair::from_secret_bytes(**signing),
    )
    .map_err(NetError::Unreachable)?;
    Ok(crate::folder_org::register(&f.key, engine))
}

impl Conn {
    pub fn sign_in(&mut self) -> Result<(), NetError> {
        if let Some(f) = self.folder.clone() {
            self.client.local = Some(folder_api(&f, &self.device, &self.signing)?);
            self.client.token = Some("local".into());
            self.setup_code = false;
            return Ok(());
        }
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
                id_token: self.id_token.take(),
                pin: self.pin.as_ref().map(|p| p.to_string()),
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
            // D-79: past "Ask again after" the session needs the PIN again, given if typed.
            Err(e)
                if e.code() == Some("unauthorized")
                    || e.code() == Some("pin_required") && self.pin.is_some() =>
            {
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
                        password: false,
                        error: "http".into(),
                        message: crate::trf!("HTTP {}", r.status),
                        pin_length: None,
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
        /// Changes the server refused (`(kind, id)`): the server's version comes back.
        refused: Vec<(String, String)>,
    },
    /// The server wants a verification code (`wrong`: the one typed was refused; `message`:
    /// where a code was sent).
    NeedCode {
        wrong: bool,
        message: Option<String>,
    },
    /// The member signs in with Google / Microsoft.
    NeedIdentity {
        method: Method,
        providers: Vec<Provider>,
    },
    /// Signed in, but the policy wants a second step set up first: the methods allowed.
    SetupCode {
        conn: Box<Conn>,
        hello: Box<Hello>,
    },
    /// The organization's PIN is asked (D-79): `message` after a wrong one, `locked` after
    /// too many.
    NeedPin {
        message: Option<String>,
        locked: bool,
        /// The PIN's length, as the server says (D-79).
        length: Option<u8>,
        /// A password is asked, not a PIN (D-98).
        password: bool,
    },
    /// The organization was deleted by its Super Admin (D-98).
    Deleted,
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
            let hello: Hello = conn.client.get("/v1/hello")?;
            return Ok(Outcome::SetupCode {
                conn: Box::new(conn.clone()),
                hello: Box::new(hello),
            });
        }
        let mut refused = vec![];
        if !push.rows.is_empty() || !push.purged.is_empty() {
            let r: PushReply = conn.call("/v1/push", &push)?;
            refused = r.refused;
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
            refused,
        })
    };
    match run(&mut conn) {
        Ok(o) => o,
        Err(e) => match e.code() {
            Some("code_required") => Outcome::NeedCode {
                wrong: false,
                message: None,
            },
            Some("code_sent") => Outcome::NeedCode {
                wrong: false,
                message: match &e {
                    NetError::Api(_, a) => Some(a.message.clone()),
                    _ => None,
                },
            },
            Some(c @ ("identity_google" | "identity_microsoft")) => Outcome::NeedIdentity {
                method: if c == "identity_google" {
                    Method::Google
                } else {
                    Method::Microsoft
                },
                providers: conn
                    .client
                    .get::<Hello>("/v1/hello")
                    .map(|h| h.providers)
                    .unwrap_or_default(),
            },
            Some("wrong_code") => match &e {
                NetError::Api(_, a) if a.message != "Wrong code." => {
                    Outcome::Failed(a.message.clone())
                }
                _ => Outcome::NeedCode {
                    wrong: true,
                    message: None,
                },
            },
            Some(c @ ("pin_required" | "wrong_pin" | "pin_locked")) => Outcome::NeedPin {
                message: match &e {
                    NetError::Api(_, a) if c != "pin_required" => Some(a.message.clone()),
                    _ => None,
                },
                locked: c == "pin_locked",
                length: match &e {
                    NetError::Api(_, a) => a.pin_length,
                    _ => None,
                },
                password: matches!(&e, NetError::Api(_, a) if a.password),
            },
            Some("revoked") => Outcome::Revoked,
            Some("deleted") => Outcome::Deleted,
            Some("setup_code") => Outcome::Failed(tr("Set up your verification code").into()),
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
        proof: vec![],
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
    /// Setting the second step up (rather than signing in).
    pub setup: bool,
    /// Where the code was sent (e-mail, SMS).
    pub message: Option<String>,
    /// Set-up: the methods the member may choose.
    pub methods: Vec<Method>,
    pub providers: Vec<Provider>,
    /// Set-up: the method chosen; sign-in: the identity provider asked.
    pub method: Method,
    /// SMS set-up: the phone number box.
    pub phone: Option<Entity<InputState>>,
    /// A code was sent or a key given: the code box is shown.
    pub asked: bool,
}

impl CodePrompt {
    pub fn new(method: Method) -> Self {
        Self {
            input: None,
            error: None,
            secret: None,
            busy: false,
            setup: false,
            message: None,
            methods: vec![],
            providers: vec![],
            method,
            phone: None,
            asked: true,
        }
    }
}

/// What the organization asks before its spaces show (mockups 36h, 42g, 42h): a PIN in boxes, a
/// password in a field, or Windows Hello / Touch ID.
pub struct PinPrompt {
    pub with: AccessWith,
    /// Checked on this computer (shared folder, Windows Hello), not by the server.
    pub here: bool,
    /// The password's field (made at the next render).
    pub field: Option<Entity<InputState>>,
    /// Made at the next render (they need the keyboard focus).
    pub boxes: Option<crate::screens::pin::PinBoxes>,
    /// Digits, from the server's refusal or the policy last received (D-79).
    pub len: u8,
    pub error: Option<String>,
    pub busy: bool,
    /// Too many wrong PINs: an admin has to unlock this member.
    pub locked: bool,
    /// The boxes took the keyboard once shown.
    pub focused: bool,
}

impl PinPrompt {
    pub fn new(len: u8) -> Self {
        Self::asking(AccessWith::Pin, len, false)
    }

    pub fn asking(with: AccessWith, len: u8, here: bool) -> Self {
        Self {
            with,
            here,
            field: None,
            boxes: None,
            len: if crate::opening::PIN_LENGTHS.contains(&usize::from(len)) {
                len
            } else {
                PIN_DEFAULT_LEN
            },
            error: None,
            busy: false,
            locked: false,
            focused: false,
        }
    }
}

/// The organization PIN's length when nothing says otherwise (`PinRule::default`).
pub const PIN_DEFAULT_LEN: u8 = 6;

/// Progress of "Server on the NAS", from its background task.
enum NasEvent {
    Step(&'static str),
    Found(crate::nas::Found),
    Done(Result<Membership, String>),
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
    /// Super Admins: the Super Admin key, opened (details of reserved spaces, D-61).
    pub super_key: Option<KeyPair>,
    /// Spaces to pull whole at the next sync.
    pub full: Vec<SpaceId>,
    pub code: Option<CodePrompt>,
    /// The organization's PIN is asked (D-79): its spaces stay closed until it is given.
    pub pin: Option<PinPrompt>,
    /// Names of the organization's spaces this session can open.
    pub names: BTreeMap<SpaceId, String>,
    /// The organization's spaces are hidden: not reached for longer than the policy allows.
    pub offline_locked: bool,
    pub ui: crate::screens::org::OrgUi,
    /// D-101: the profile set in Cleis sent this session (once, when the organization shows
    /// another name, e-mail or photo).
    pub profile_sent: bool,
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
            .unwrap_or_else(|| tr("Former member").into())
    }

    pub fn space_name(&self, id: SpaceId) -> String {
        self.names
            .get(&id)
            .cloned()
            .unwrap_or_else(|| tr("Space").into())
    }

    /// Opens an activity detail: with the organization key, or the Super Admin key for reserved
    /// spaces.
    pub fn open_detail(&self, e: &Event) -> Option<EventDetail> {
        let d = e.detail.as_ref()?;
        let plain = [self.org_key.as_ref(), self.super_key.as_ref()]
            .into_iter()
            .flatten()
            .find_map(|k| cleis_crypto::open_from(k, DETAIL_CONTEXT, d).ok())?;
        serde_json::from_slice(&plain).ok()
    }

    pub fn is_reserved(&self, space: SpaceId) -> bool {
        self.state
            .as_ref()
            .and_then(|s| s.spaces.iter().find(|x| x.id == space))
            .is_some_and(|x| x.reserved)
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
    /// Loads the memberships when the vault opens (D-69: several organizations). Sessions already
    /// open are kept; `active` becomes the one shown.
    pub fn load_org(&mut self, cx: &mut Context<Self>) {
        self.load_orgs(None, cx);
    }

    pub fn load_orgs(&mut self, active: Option<String>, cx: &mut Context<Self>) {
        let path = self.path.clone();
        let demo = self.demo;
        let Some(s) = self.session() else { return };
        let mut open: Vec<OrgSession> = s.org.take().into_iter().collect();
        open.append(&mut s.orgs_idle);
        let mut sessions = vec![];
        for m in memberships(&s.vault) {
            let key = m.key();
            if let Some(i) = open.iter().position(|o| o.m.key() == key) {
                sessions.push(open.remove(i));
                continue;
            }
            let mut local = load_local(&path, &key);
            let rk = remembered_setting(&s.sync.device, &key);
            match local.remembered.take() {
                // Older versions kept it in the file: moved into the vault.
                Some(t) => {
                    let _ = s.vault.set_org_setting(&rk, &Some(t.clone()));
                    local.remembered = Some(t);
                    let mut bare = local.clone();
                    bare.remembered = None;
                    save_local(&path, &key, &bare);
                }
                None => {
                    local.remembered = s
                        .vault
                        .org_setting::<Option<String>>(&rk)
                        .ok()
                        .flatten()
                        .flatten();
                }
            }
            let offline_days = i64::from(local.offline_days.unwrap_or(14));
            let offline_locked = !demo
                && local
                    .last_ok
                    .is_some_and(|t| cleis_core::now() - t > offline_days * 86_400);
            // D-79, D-98: given here too long ago (or never): asked before the spaces open.
            let pin = (!demo
                && local.pin_hours.is_some()
                && local.pin_until.is_none_or(|t| cleis_core::now() >= t))
            .then(|| prompt_for(&local, m.in_folder()));
            let mut o = OrgSession {
                m,
                token: None,
                local,
                state: None,
                busy: false,
                again: true,
                last_attempt: None,
                error: None,
                org_key: None,
                super_key: None,
                full: vec![],
                code: None,
                pin,
                names: BTreeMap::new(),
                offline_locked,
                ui: Default::default(),
                profile_sent: false,
            };
            for (id, _) in s.vault.org_spaces(&key).unwrap_or_default() {
                if let Some(sp) = s.spaces.iter().find(|x| x.id == id) {
                    o.names.insert(id, sp.name.clone());
                }
            }
            sessions.push(o);
        }
        if let Some(k) = active
            && let Some(i) = sessions.iter().position(|o| o.m.key() == k)
        {
            let o = sessions.remove(i);
            sessions.insert(0, o);
        }
        let mut it = sessions.into_iter();
        s.org = it.next();
        s.orgs_idle = it.collect();
        hide_org_entries(s);
        // Hosted here: start the server if nothing answers yet.
        let host = s.org.as_ref().and_then(|o| {
            let port = crate::host::port_of(&o.m.address);
            o.local.host_dir.clone().map(|d| (d, port))
        });
        if let (Some((dir, port)), Some(exe), false) = (host, crate::host::server_exe(), demo) {
            cx.background_spawn(async move {
                if let Err(e) = crate::host::ensure_running(&exe, &dir, port) {
                    eprintln!("hosted server: {e}");
                }
            })
            .detach();
        }
        cx.notify();
    }

    /// Keys of the vault's organizations, the one shown first.
    pub fn org_keys(&self) -> Vec<String> {
        match &self.phase {
            Phase::Main(s) => s.all_orgs().map(|o| o.m.key()).collect(),
            _ => vec![],
        }
    }

    pub fn active_org_key(&self) -> Option<String> {
        match &self.phase {
            Phase::Main(s) => s.org.as_ref().map(|o| o.m.key()),
            _ => None,
        }
    }

    /// Runs `f` with the organization `key` in `Session::org`, then puts the one shown back
    /// (D-69): a sync or an answer of the server reaches its own organization.
    pub fn in_org<R>(&mut self, key: &str, f: impl FnOnce(&mut Self) -> R) -> Option<R> {
        let Phase::Main(s) = &mut self.phase else {
            return None;
        };
        if s.org.as_ref().is_some_and(|o| o.m.key() == key) {
            return Some(f(self));
        }
        let i = s.orgs_idle.iter().position(|o| o.m.key() == key)?;
        let other = s.orgs_idle.remove(i);
        let shown = s.org.replace(other);
        let r = f(self);
        if let Phase::Main(s) = &mut self.phase {
            let back = std::mem::replace(&mut s.org, shown);
            if let Some(o) = back {
                let at = i.min(s.orgs_idle.len());
                s.orgs_idle.insert(at, o);
            }
            if s.org.is_none() && !s.orgs_idle.is_empty() {
                s.org = Some(s.orgs_idle.remove(0));
            }
        }
        Some(r)
    }

    /// Shows the organization `key` (its tab, its settings, its activity).
    pub fn activate_org(&mut self, key: &str) {
        let Phase::Main(s) = &mut self.phase else {
            return;
        };
        if s.org.as_ref().is_some_and(|o| o.m.key() == key) {
            return;
        }
        let Some(i) = s.orgs_idle.iter().position(|o| o.m.key() == key) else {
            return;
        };
        let o = s.orgs_idle.remove(i);
        if let Some(prev) = s.org.replace(o) {
            s.orgs_idle.insert(i, prev);
        }
    }

    /// Creates an organization whose server runs on this computer, and joins it as its first
    /// administrator (D-63).
    pub fn org_create_here(&mut self, name: String, me: String, cx: &mut Context<Self>) {
        let Some(exe) = crate::host::server_exe() else {
            if let Some(s) = self.session() {
                s.join.error = Some("cleis-server is not beside Cleis".into());
            }
            cx.notify();
            return;
        };
        let Some(s) = self.session() else { return };
        let public = s.vault.account_public().0;
        let signing = SigningKeyPair::from_secret_bytes(*s.vault.signing_key().secret_bytes());
        s.join.busy = true;
        s.join.error = None;
        let dir = crate::host::default_dir();
        let task = cx.background_spawn(async move {
            let port = crate::host::free_port();
            let code = crate::host::create(&exe, &dir, name.trim(), port)?;
            crate::host::ensure_running(&exe, &dir, port)?;
            let addr = format!("127.0.0.1:{port}");
            let m = join(&addr, &code, &me, "", public, &signing)?;
            Ok::<_, String>((m, dir))
        });
        cx.spawn(async move |this, cx| {
            let r = task.await;
            this.update(cx, |app, cx| {
                let path = app.path.clone();
                match r {
                    Ok((m, dir)) => {
                        let key = m.key();
                        let mut local = load_local(&path, &key);
                        local.host_dir = Some(dir);
                        save_local(&path, &key, &local);
                        app.org_joined(Ok(m), cx);
                        if let Some(s) = app.session() {
                            s.settings_page = "Server";
                        }
                    }
                    Err(e) => app.org_joined(Err(e), cx),
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// Shared folder (D-78): the organization's start written in `folder`, this vault its first
    /// administrator.
    pub fn org_create_in_folder(
        &mut self,
        name: String,
        me: String,
        folder: PathBuf,
        sftp: Option<SftpSecret>,
        cx: &mut Context<Self>,
    ) {
        let path = self.path.clone();
        let Some(s) = self.session() else { return };
        let public = s.vault.account_public().0;
        let signing = SigningKeyPair::from_secret_bytes(*s.vault.signing_key().secret_bytes());
        s.join.busy = true;
        s.join.error = None;
        let task = cx.background_spawn(async move {
            // The copy is made under a temporary name: the organization's id is not known yet.
            let tmp = replica_dir(&path, &format!("new-{}", uuid::Uuid::new_v4()));
            let mut sftp = sftp;
            let store: Box<dyn crate::folder_store::Store> =
                match crate::folder_store::SftpPlace::parse(&folder.to_string_lossy()) {
                    Some(place) => {
                        let sec = sftp.clone().unwrap_or_default();
                        let mut st =
                            crate::folder_store::SftpStore::new(place, sec.secret, sec.host_key)?;
                        // The server's key, kept from this first connection.
                        use crate::folder_store::Store;
                        st.list("")?;
                        if let Some(s) = sftp.as_mut() {
                            s.host_key = st.host_key.clone();
                        }
                        Box::new(st)
                    }
                    None => {
                        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
                        Box::new(crate::folder_store::LocalStore(folder.clone()))
                    }
                };
            let (engine, org, member) = crate::folder_org::Engine::create_in(
                store,
                &folder.display().to_string(),
                &tmp,
                name.trim(),
                me.trim(),
                public,
                signing,
                &device_name(),
            )?;
            drop(engine);
            let key = org.to_string();
            let replica = replica_dir(&path, &key);
            let _ = std::fs::remove_dir_all(&replica);
            let _ = std::fs::remove_dir_all(&tmp);
            let m = Membership {
                org,
                name: name.trim().to_owned(),
                address: FOLDER.into(),
                pins: vec![],
                member,
            };
            Ok::<_, String>((m, folder, sftp))
        });
        cx.spawn(async move |this, cx| {
            let r = task.await;
            this.update(cx, |app, cx| app.folder_joined(r, cx)).ok();
        })
        .detach();
        cx.notify();
    }

    /// A shared-folder organization created or joined: where its folder is on this computer and,
    /// for SFTP, how to sign in there (sealed in the vault).
    fn folder_joined(
        &mut self,
        r: Result<(Membership, PathBuf, Option<SftpSecret>), String>,
        cx: &mut Context<Self>,
    ) {
        let path = self.path.clone();
        match r {
            Ok((m, folder, sftp)) => {
                let key = m.key();
                let mut local = load_local(&path, &key);
                local.folder = Some(folder);
                save_local(&path, &key, &local);
                if let (Some(sec), Some(s)) = (sftp, self.session()) {
                    let _ = s.vault.set_org_setting(&sftp_setting(&key), &sec);
                }
                self.org_joined(Ok(m), cx);
            }
            Err(e) => self.org_joined(Err(e), cx),
        }
    }

    /// Shared folder (D-78): joins the organization in `folder` with an invitation code.
    pub fn org_join_folder(
        &mut self,
        folder: PathBuf,
        sftp: Option<SftpSecret>,
        code: String,
        name: String,
        email: String,
        cx: &mut Context<Self>,
    ) {
        let path = self.path.clone();
        let Some(s) = self.session() else { return };
        let public = s.vault.account_public().0;
        let secret = Zeroizing::new(*s.vault.signing_key().secret_bytes());
        s.join.busy = true;
        s.join.error = None;
        let task = cx.background_spawn(async move {
            let mut sftp = sftp;
            let info = {
                let mut store =
                    crate::folder_store::store_for(&folder, sftp.as_ref().map(|s| &s.secret), "")?;
                crate::folder_org::Engine::org_file_in(store.as_mut())
                    .ok_or_else(|| crate::trf!("No Cleis organization in {}", folder.display()))?
            };
            if let (Some(sec), Some(place)) = (
                sftp.as_mut(),
                crate::folder_store::SftpPlace::parse(&folder.to_string_lossy()),
            ) {
                let mut st =
                    crate::folder_store::SftpStore::new(place, sec.secret.clone(), String::new())?;
                use crate::folder_store::Store;
                st.list("")?;
                sec.host_key = st.host_key.clone();
            }
            let replica = replica_dir(&path, &info.id.to_string());
            let key = replica.display().to_string();
            let spec = FolderSpec {
                key: key.clone(),
                folder: Some(folder.clone()),
                replica,
                member: None,
                sftp: sftp.clone(),
            };
            let device = device_name();
            let mut client = Client::new(FOLDER, Trust::Pinned(vec![]));
            client.local = Some(folder_api(&spec, &device, &secret).map_err(|e| e.to_string())?);
            client.token = Some("local".into());
            let signing = SigningKeyPair::from_secret_bytes(*secret);
            let req =
                crate::folder_org::proven_join(&code, &name, &email, &device, public, &signing)?;
            let reply: JoinReply = client.call("/v1/join", &req).map_err(|e| e.to_string())?;
            crate::folder_org::set_member(&key, reply.member);
            Ok::<_, String>((
                Membership {
                    org: reply.org.id,
                    name: reply.org.name,
                    address: FOLDER.into(),
                    pins: vec![],
                    member: reply.member,
                },
                folder,
                sftp,
            ))
        });
        cx.spawn(async move |this, cx| {
            let r = task.await;
            this.update(cx, |app, cx| app.folder_joined(r, cx)).ok();
        })
        .detach();
    }

    /// Server on the NAS (D-78): signs in over SSH, finds the system, sends Cleis Server, creates
    /// the organization there and joins it as its first administrator.
    pub fn org_create_on_nas(&mut self, name: String, me: String, cx: &mut Context<Self>) {
        let Some(j) = self.join_form_mut() else {
            return;
        };
        let Some(n) = j.nas.as_ref() else { return };
        let value = |e: &Entity<InputState>| e.read(cx).value().trim().to_owned();
        let host = value(&n.address);
        let port = value(&n.port).parse::<u16>().unwrap_or(22);
        let user = value(&n.user);
        let password = n.password.read(cx).value().to_string();
        if host.is_empty() || user.is_empty() {
            j.error = Some(tr("Enter the NAS address and its administrator").into());
            cx.notify();
            return;
        }
        j.busy = true;
        j.error = None;
        let nas = crate::nas::Nas {
            host: host.clone(),
            port,
            user,
            secret: crate::remote::Secret {
                token: password,
                ..Default::default()
            },
        };
        let Some(s) = self.session() else { return };
        let public = s.vault.account_public().0;
        let signing = SigningKeyPair::from_secret_bytes(*s.vault.signing_key().secret_bytes());
        let (tx, rx) = async_channel::unbounded::<NasEvent>();
        cx.background_spawn(async move {
            let run = || -> Result<Membership, String> {
                let _ = tx.send_blocking(NasEvent::Step("Looking at the NAS"));
                let found = crate::nas::probe(&nas)?;
                let _ = tx.send_blocking(NasEvent::Found(found.clone()));
                let _ = tx.send_blocking(NasEvent::Step("Getting Cleis Server"));
                let file = found.package(env!("CARGO_PKG_VERSION"));
                let bytes = crate::nas::package_bytes(&file)?;
                let code = crate::nas::install(&nas, &found, name.trim(), &bytes, |s| {
                    let _ = tx.send_blocking(NasEvent::Step(s));
                })?;
                let _ = tx.send_blocking(NasEvent::Step("Starting the server"));
                let up = (0..30).any(|_| {
                    if crate::nas::answers(&nas.host) {
                        return true;
                    }
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    false
                });
                if !up {
                    return Err(crate::trf!(
                        "The server does not answer on {}:{}",
                        nas.host,
                        crate::nas::PORT
                    ));
                }
                let addr = format!("{}:{}", nas.host, crate::nas::PORT);
                join(&addr, &code, &me, "", public, &signing)
            };
            let r = run();
            let _ = tx.send_blocking(NasEvent::Done(r));
        })
        .detach();
        cx.spawn(async move |this, cx| {
            while let Ok(ev) = rx.recv().await {
                let done = matches!(ev, NasEvent::Done(_));
                this.update(cx, |app, cx| {
                    match ev {
                        NasEvent::Step(s) => {
                            if let Some(n) = app.join_form_mut().and_then(|j| j.nas.as_mut()) {
                                n.step = Some(s);
                            }
                        }
                        NasEvent::Found(f) => {
                            if let Some(n) = app.join_form_mut().and_then(|j| j.nas.as_mut()) {
                                n.found = Some(f);
                            }
                        }
                        NasEvent::Done(r) => {
                            if let Some(n) = app.join_form_mut().and_then(|j| j.nas.as_mut()) {
                                n.step = None;
                            }
                            app.org_joined(r, cx);
                        }
                    }
                    cx.notify();
                })
                .ok();
                if done {
                    break;
                }
            }
        })
        .detach();
        let _ = host;
        cx.notify();
    }

    /// Makes the hosted server start with the system (administrator rights asked).
    pub fn org_install_host(&mut self, cx: &mut Context<Self>) {
        let Some((dir, port)) = self.session().and_then(|s| s.org.as_ref()).and_then(|o| {
            let port = crate::host::port_of(&o.m.address);
            o.local.host_dir.clone().map(|d| (d, port))
        }) else {
            return;
        };
        let Some(exe) = crate::host::server_exe() else {
            return;
        };
        let task = cx.background_spawn(async move {
            // The service takes the port: the copy started by Cleis gives it up first.
            crate::host::stop();
            let r = crate::host::install_service(&exe, &dir);
            for _ in 0..30 {
                if crate::host::listening(port) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            if r.is_err() {
                let _ = crate::host::ensure_running(&exe, &dir, port);
            }
            r
        });
        cx.spawn(async move |this, cx| {
            let r = task.await;
            this.update(cx, |app, cx| {
                app.toast(
                    match r {
                        Ok(()) => tr("The server now starts with the system").to_owned(),
                        Err(e) => crate::trf!("Not installed: {}", e),
                    },
                    cx,
                )
            })
            .ok();
        })
        .detach();
    }

    fn org_conn(&self) -> Option<Conn> {
        let Phase::Main(s) = &self.phase else {
            return None;
        };
        let o = s.org.as_ref()?;
        let mut client = Client::new(&o.m.address, o.m.trust());
        client.token = o.token.clone();
        // The copy is this vault's: its folder names it (one vault per run, several in tests).
        let replica = replica_dir(&self.path, &o.m.key());
        let rkey = replica.display().to_string();
        let sftp = s
            .vault
            .org_setting::<SftpSecret>(&sftp_setting(&o.m.key()))
            .ok()
            .flatten();
        let folder = o.m.in_folder().then(|| FolderSpec {
            key: rkey.clone(),
            folder: o.local.folder.clone(),
            replica: replica.clone(),
            member: Some(o.m.member),
            sftp,
        });
        if folder.is_some() {
            client.local = crate::folder_org::api_of(&rkey);
            if client.local.is_none() {
                client.token = None;
            }
        }
        Some(Conn {
            client,
            member: o.m.member,
            signing: Zeroizing::new(*s.vault.signing_key().secret_bytes()),
            device: device_name(),
            remembered: o.local.remembered.clone(),
            code: None,
            id_token: None,
            new_remembered: None,
            setup_code: false,
            pin: None,
            folder,
        })
    }

    /// Called every second: sends local changes soon, and pulls every 30 s.
    pub fn org_sync_due(&self) -> bool {
        let Phase::Main(s) = &self.phase else {
            return false;
        };
        let Some(o) = &s.org else { return false };
        // Asked here (shared folder, Windows Hello): the copy keeps syncing behind the card.
        let asked = o.pin.as_ref().is_some_and(|p| !p.here);
        if o.busy || o.code.is_some() || asked || s.gate.is_some() || self.demo {
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
        self.org_sync_with(code, None, cx);
    }

    /// `org_sync` with an ID token from Google / Microsoft.
    pub fn org_sync_with(
        &mut self,
        code: Option<String>,
        id_token: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let Some(mut conn) = self.org_conn() else {
            return;
        };
        conn.code = code;
        conn.id_token = id_token;
        self.org_run(conn, cx);
    }

    /// One sync with this connection (sign-in, push, pull, state) off the interface thread.
    fn org_run(&mut self, conn: Conn, cx: &mut Context<Self>) {
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
            this.update(cx, |app, cx| {
                app.in_org(&key, |app| app.org_done(out, sent, cx));
            })
            .ok();
        })
        .detach();
    }

    fn org_done(&mut self, out: Outcome, sent: Changes, cx: &mut Context<Self>) {
        let path = self.path.clone();
        let Some(s) = self.session() else { return };
        let Some(o) = s.org.as_mut() else { return };
        o.busy = false;
        let mut profile = None;
        match out {
            Outcome::Done {
                conn,
                changes,
                state,
                refused,
            } => {
                o.token = conn.client.token.clone();
                if conn.new_remembered.is_some() {
                    o.local.remembered = conn.new_remembered.clone();
                }
                o.code = None;
                o.error = None;
                o.local.last_ok = Some(cleis_core::now());
                o.local.offline_days = Some(state.policy.offline_days);
                // D-79, D-98: what the organization asks, kept for the next start.
                let rule = state.policy.access;
                let folder = o.m.in_folder();
                let secret_known = if folder {
                    state.access_secret.is_some()
                } else {
                    state.pin_set
                };
                // Something to check: a password or PIN once set, or Windows Hello.
                let asks = rule.on && (rule.with == AccessWith::Hello || secret_known);
                let here = folder || rule.with == AccessWith::Hello;
                o.local.pin_hours = asks.then_some(rule.again_hours);
                o.local.pin_len = (asks && rule.with == AccessWith::Pin).then_some(rule.length);
                o.local.access_with = asks.then_some(rule.with);
                o.local.idle_mins = rule.idle_mins.filter(|_| asks);
                o.local.tries = rule.tries;
                o.local.secret = state.access_secret.clone();
                o.local.logo = state
                    .org
                    .logo
                    .as_ref()
                    .map(|b| data_encoding::BASE64.encode(b));
                // An admin unlocked this member since this computer locked itself.
                if let (Some(at), Some(reset)) = (o.local.locked_at, state.me.pin_reset)
                    && reset > at
                {
                    o.local.locked_at = None;
                    o.local.fails = 0;
                    if let Some(p) = o.pin.as_mut() {
                        p.locked = false;
                    }
                }
                let was_asked = if !asks {
                    o.local.pin_until = None;
                    o.pin.take().is_some()
                } else if here {
                    // Checked on this computer: a sync opens nothing; asked if never given.
                    if o.pin.is_none() && o.local.pin_until.is_none_or(|t| cleis_core::now() >= t) {
                        o.pin = Some(prompt_for(&o.local, folder));
                    }
                    false
                } else {
                    // The server checked it: given (or not asked) until "Ask again after".
                    let was = o.pin.take().is_some();
                    if was || o.local.pin_until.is_none() {
                        o.local.pin_until =
                            Some(cleis_core::now() + i64::from(rule.again_hours.max(1)) * 3600);
                    }
                    was
                };
                let key = o.m.key();
                let _ = s.vault.set_org_setting(
                    &remembered_setting(&s.sync.device, &key),
                    &o.local.remembered,
                );
                let mut bare = o.local.clone();
                bare.remembered = None;
                save_local(&path, &key, &bare);
                let _ = s.vault.org_pushed(&sent.rows, &sent.purged);
                // Audit L-1: what the server refused is put back as the server has it.
                if !refused.is_empty() {
                    let spaces = s.vault.org_refused(&refused).unwrap_or_default();
                    o.full.extend(spaces);
                    o.again = true;
                    s.toast = Some(
                        format!(
                            "{} change{} refused by your role: put back",
                            refused.len(),
                            if refused.len() == 1 { "" } else { "s" }
                        )
                        .into(),
                    );
                }
                apply_state(s, *state, &changes);
                // D-101: the profile set in Cleis, sent once to an organization showing another.
                let mine = s.prefs.profile.clone();
                if let Some(o) = s.org.as_mut()
                    && !o.profile_sent
                    && !mine.name.is_empty()
                    && mine.email.contains('@')
                    && o.me().is_some_and(|m| {
                        m.name != mine.name || m.email != mine.email || m.photo != mine.photo
                    })
                {
                    o.profile_sent = true;
                    profile = Some(mine);
                }
                if was_asked {
                    let _ = crate::sync::reload(s);
                } else {
                    hide_org_entries(s);
                }
            }
            Outcome::NeedCode { wrong, message } => {
                o.again = false;
                match o.code.as_mut() {
                    Some(c) => {
                        c.busy = false;
                        c.error = Some(
                            if wrong {
                                "Wrong code"
                            } else {
                                "Enter the code"
                            }
                            .into(),
                        );
                        if message.is_some() {
                            c.message = message;
                        }
                    }
                    None => {
                        let mut c = CodePrompt::new(Method::AuthenticatorApp);
                        c.error = wrong.then(|| tr("Wrong code").into());
                        c.message = message;
                        o.code = Some(c);
                    }
                }
            }
            Outcome::NeedIdentity { method, providers } => {
                o.again = false;
                let mut c = CodePrompt::new(method);
                c.providers = providers;
                c.asked = false;
                o.code = Some(c);
            }
            Outcome::SetupCode { conn, hello } => {
                o.token = conn.client.token.clone();
                o.again = false;
                let mut c = CodePrompt::new(Method::AuthenticatorApp);
                c.setup = true;
                c.asked = false;
                c.methods = hello.methods.clone();
                c.providers = hello.providers.clone();
                o.code = Some(c);
                let first = hello
                    .methods
                    .first()
                    .copied()
                    .unwrap_or(Method::AuthenticatorApp);
                self.org_setup_pick(first, cx);
            }
            Outcome::NeedPin {
                message,
                locked,
                length,
                password,
            } => {
                o.again = false;
                if length.is_some() {
                    o.local.pin_len = length;
                }
                let with = if password {
                    AccessWith::Password
                } else {
                    AccessWith::Pin
                };
                o.local.access_with = Some(with);
                let len = o.local.pin_len.unwrap_or(PIN_DEFAULT_LEN);
                let p = o
                    .pin
                    .get_or_insert_with(|| PinPrompt::asking(with, len, false));
                if p.with != with {
                    *p = PinPrompt::asking(with, len, false);
                }
                p.busy = false;
                p.error = message;
                p.locked = locked;
                // Another length: new boxes; otherwise emptied for the next try.
                if p.len != len && crate::opening::PIN_LENGTHS.contains(&usize::from(len)) {
                    p.len = len;
                    p.boxes = None;
                } else if let Some(b) = p.boxes.as_mut() {
                    b.clear();
                }
                p.focused = false;
                if let Some(s) = self.session() {
                    hide_org_entries(s);
                }
                self.org_followups(cx);
                cx.notify();
                return;
            }
            Outcome::Deleted => {
                let name = o.m.name.clone();
                let key = o.m.key();
                let _ = s.vault.leave_org(&key);
                s.org = None;
                save_memberships(s);
                remove_local(&path, &key);
                let _ = crate::sync::reload(s);
                s.toast = Some(crate::trf!("{} was deleted", name).into());
            }
            Outcome::Revoked => {
                let name = o.m.name.clone();
                let key = o.m.key();
                let _ = s.vault.leave_org(&key);
                s.org = None;
                save_memberships(s);
                remove_local(&path, &key);
                let _ = crate::sync::reload(s);
                s.toast = Some(crate::trf!("Removed from {}", name).into());
            }
            Outcome::Failed(e) => {
                o.error = Some(e);
                if let Some(c) = o.code.as_mut() {
                    c.busy = false;
                }
            }
        }
        if let Some(p) = profile {
            let req = ProfileChange {
                name: p.name,
                email: p.email,
                photo: Some(p.photo.unwrap_or_default()),
            };
            self.org_request::<_, MemberView>("/v1/me", req, |_, _, _| {}, cx);
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
        let is_super = |m: Option<MemberId>| {
            m.is_some_and(|m| {
                st.members
                    .iter()
                    .any(|x| x.id == m && x.role == SUPER_ADMIN)
            })
        };
        for p in &st.pending {
            let Some(sv) = st.spaces.iter().find(|x| x.id == p.space) else {
                continue;
            };
            // A reserved space's key goes only to someone a Super Admin gave it to (D-61).
            if sv.reserved && !is_super(p.granted_by) {
                continue;
            }
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
        if is_admin_role(&st.me.role) && st.org.org_public.is_none() {
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
        // The Super Admin key, the same way (D-61).
        let mut sup = None;
        if st.me.role == SUPER_ADMIN && st.org.super_public.is_none() {
            if let Ok(kp) = KeyPair::generate() {
                let secret = SymmetricKey::from_bytes(*kp.secret_bytes());
                if let Ok(w) = wrap_key(&s.vault.account_public(), &secret, SUPER_KEY_CONTEXT) {
                    sup = Some((
                        SuperKeys {
                            super_public: Some(kp.public().0.to_vec()),
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
        } else if let Some(k) = &o.super_key
            && !st.pending_supers.is_empty()
        {
            let secret = SymmetricKey::from_bytes(*k.secret_bytes());
            let wraps = st
                .pending_supers
                .iter()
                .filter_map(|p| {
                    wrap_key(&PublicKey(p.account_public), &secret, SUPER_KEY_CONTEXT)
                        .ok()
                        .map(|w| Wrap {
                            space: SpaceId(uuid::Uuid::nil()),
                            member: p.member,
                            wrapped: w,
                        })
                })
                .collect();
            sup = Some((
                SuperKeys {
                    super_public: None,
                    wraps,
                },
                None,
            ));
        }
        if let Some((req, kp)) = sup {
            self.org_request::<_, PushReply>(
                "/v1/super-keys",
                req,
                move |app, r, _| {
                    if r.is_ok()
                        && let Some(kp) = kp
                        && let Some(o) = app.session().and_then(|s| s.org.as_mut())
                    {
                        o.super_key = Some(kp);
                    }
                },
                cx,
            );
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
        let Some(key) = self.active_org_key() else {
            return;
        };
        let task = cx.background_spawn(async move {
            let r: Result<R, NetError> = conn.call(path, &req);
            (r.map_err(|e| e.to_string()), conn.client.token.clone())
        });
        cx.spawn(async move |this, cx| {
            let (r, token) = task.await;
            this.update(cx, |app, cx| {
                app.in_org(&key, |app| {
                    if let Some(o) = app.session().and_then(|s| s.org.as_mut()) {
                        o.token = token;
                        o.again = true;
                    }
                    if let Err(e) = &r {
                        app.toast_for(e.clone(), 6, cx);
                    }
                    then(app, r, cx);
                });
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Opens the set-up of the second sign-in step (Settings › Profile, or asked by the policy).
    pub fn org_setup_prompt(&mut self, cx: &mut Context<Self>) {
        let Some(mut conn) = self.org_conn() else {
            return;
        };
        let task = cx.background_spawn(async move { conn.get::<Hello>("/v1/hello") });
        cx.spawn(async move |this, cx| {
            let r = task.await;
            this.update(cx, |app, cx| {
                let Some(o) = app.session().and_then(|s| s.org.as_mut()) else {
                    return;
                };
                match r {
                    Ok(hello) => {
                        let mut c = CodePrompt::new(Method::AuthenticatorApp);
                        c.setup = true;
                        c.asked = false;
                        c.methods = hello.methods.clone();
                        c.providers = hello.providers;
                        o.code = Some(c);
                        let first = hello
                            .methods
                            .first()
                            .copied()
                            .unwrap_or(Method::AuthenticatorApp);
                        app.org_setup_pick(first, cx);
                    }
                    Err(e) => app.toast(e.to_string(), cx),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Set-up: a method chosen. The authenticator app's key is asked at once; e-mail and SMS wait
    /// for "Send code", Google and Microsoft for "Continue".
    pub fn org_setup_pick(&mut self, method: Method, cx: &mut Context<Self>) {
        let Some(c) = self
            .session()
            .and_then(|s| s.org.as_mut())
            .and_then(|o| o.code.as_mut())
        else {
            return;
        };
        c.method = method;
        c.secret = None;
        c.message = None;
        c.error = None;
        c.asked = false;
        if method == Method::AuthenticatorApp {
            self.org_setup_send(cx);
        }
        cx.notify();
    }

    /// Set-up: asks the server for the key (app) or to send a code (e-mail, SMS).
    pub fn org_setup_send(&mut self, cx: &mut Context<Self>) {
        let Some(c) = self
            .session()
            .and_then(|s| s.org.as_mut())
            .and_then(|o| o.code.as_mut())
        else {
            return;
        };
        let target = c
            .phone
            .as_ref()
            .filter(|_| c.method == Method::Sms)
            .map(|p| p.read(cx).value().to_string())
            .unwrap_or_default();
        let method = c.method;
        c.busy = true;
        c.error = None;
        self.org_request::<_, CodeSetup>(
            "/v1/me/code",
            CodeStart { method, target },
            move |app, r, cx| {
                if let Some(c) = app
                    .session()
                    .and_then(|s| s.org.as_mut())
                    .and_then(|o| o.code.as_mut())
                    .filter(|c| c.method == method)
                {
                    c.busy = false;
                    match r {
                        Ok(setup) => {
                            c.asked = true;
                            if method == Method::AuthenticatorApp {
                                c.secret = Some(setup.secret);
                            } else {
                                c.message = Some(crate::trf!("Code sent to {}", setup.sent_to));
                            }
                        }
                        Err(e) => c.error = Some(e.into()),
                    }
                }
                cx.notify();
            },
            cx,
        );
    }

    /// Google / Microsoft: the provider's page in the browser, then the ID token to the server
    /// (sign-in, or set-up).
    pub fn org_identity(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(o) = s.org.as_mut() else { return };
        let email = o.me().map(|m| m.email.clone()).unwrap_or_default();
        let Some(c) = o.code.as_mut() else { return };
        let Some(p) = c.providers.iter().find(|p| p.method == c.method).cloned() else {
            c.error = Some(crate::trf!("{} is not set up on the server", c.method.label()).into());
            cx.notify();
            return;
        };
        let setup = c.setup;
        c.busy = true;
        c.error = None;
        let key = o.m.key();
        cx.notify();
        let task = cx.background_spawn(async move { crate::oidc::start(&p, &email) });
        cx.spawn(async move |this, cx| {
            let pending = task.await;
            let pending = match pending {
                Ok(p) => p,
                Err(e) => {
                    this.update(cx, |app, cx| {
                        app.in_org(&key, |app| app.org_identity_failed(e, cx));
                    })
                    .ok();
                    return;
                }
            };
            this.update(cx, |_, cx| cx.open_url(&pending.url)).ok();
            let finish = cx
                .background_spawn(async move { crate::oidc::finish(pending) })
                .await;
            this.update(cx, |app, cx| {
                app.in_org(&key, |app| app.org_identity_done(finish, setup, cx));
            })
            .ok();
        })
        .detach();
    }

    fn org_identity_done(
        &mut self,
        finish: Result<String, String>,
        setup: bool,
        cx: &mut Context<Self>,
    ) {
        let app = self;
        match finish {
            Ok(token) if setup => {
                let method = app
                    .session()
                    .and_then(|s| s.org.as_ref())
                    .and_then(|o| o.code.as_ref())
                    .map_or(Method::Google, |c| c.method);
                app.org_request::<_, CodeSetup>(
                    "/v1/me/code",
                    CodeStart {
                        method,
                        target: token,
                    },
                    |app, r, cx| match r {
                        Ok(_) => {
                            if let Some(o) = app.session().and_then(|s| s.org.as_mut()) {
                                o.code = None;
                            }
                            app.toast(tr("Two-step verification set up"), cx);
                            app.org_sync(None, cx);
                        }
                        Err(e) => app.org_identity_failed(e, cx),
                    },
                    cx,
                );
            }
            Ok(token) => app.org_sync_with(None, Some(token), cx),
            Err(e) => app.org_identity_failed(e, cx),
        }
    }

    fn org_identity_failed(&mut self, e: String, cx: &mut Context<Self>) {
        if let Some(c) = self
            .session()
            .and_then(|s| s.org.as_mut())
            .and_then(|o| o.code.as_mut())
        {
            c.busy = false;
            c.error = Some(e.into());
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
            c.error = Some(tr("Six digits").into());
            cx.notify();
            return;
        }
        c.busy = true;
        if c.setup {
            self.org_request::<_, SessionReply>(
                "/v1/me/code/confirm",
                CodeCheck { code: typed },
                |app, r, cx| {
                    if let Some(o) = app.session().and_then(|s| s.org.as_mut()) {
                        match r {
                            // The session stays: the server lifted its set-up restriction.
                            Ok(_) => {
                                o.code = None;
                                app.toast(tr("Two-step verification set up"), cx);
                            }
                            Err(_) => {
                                if let Some(c) = o.code.as_mut() {
                                    c.busy = false;
                                    c.error = Some(tr("Wrong code").into());
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
            o.error = Some(tr("Not signed in").into());
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
                let key = m.key();
                if let Some(s) = self.session() {
                    let mut all = memberships(&s.vault);
                    all.retain(|x| x.key() != key);
                    all.push(m);
                    let _ = s.vault.set_org_setting(SETTING_ALL, &all);
                    let _ = s.vault.set_org_setting(SETTING, &all.first());
                    s.join = Default::default();
                }
                self.load_orgs(Some(key), cx);
                if let Some(s) = self.session()
                    && s.settings_page == "Add an organization"
                {
                    s.settings_page = "Server";
                }
                self.toast(crate::trf!("Joined {}", name), cx);
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
            // Another organization of this vault, if any, is shown instead.
            s.org = (!s.orgs_idle.is_empty()).then(|| s.orgs_idle.remove(0));
            save_memberships(s);
            remove_local(&path, &o.m.key());
            let _ = crate::sync::reload(s);
            if s.tab == crate::app::Tab::Settings && s.org.is_none() {
                s.settings_page = "Organization";
            }
            s.toast = Some(crate::trf!("Left {}", o.m.name).into());
        }
        cx.notify();
    }

    // ---------- Admin actions ----------

    pub fn org_invite(&mut self, mut req: NewInvite, cx: &mut Context<Self>) {
        let to = req.email.clone();
        // The organization's "Uncategorized" contacts are everyone's (27/11/2026).
        if let Some(s) = self.session()
            && let Some(k) = s.org.as_ref().map(|o| o.m.key())
            && let Some(sp) = crate::screens::contacts::unfiled_of(s, &k)
            && !req.spaces.contains(&sp)
        {
            req.spaces.push(sp);
        }
        // Shared folder (D-78): the code is made here and never goes to the folder.
        let folder = self
            .session()
            .and_then(|s| s.org.as_ref())
            .is_some_and(|o| o.m.in_folder());
        let mut made = None;
        if folder {
            match crate::folder_org::new_invite_secret() {
                Ok((token, secret)) => {
                    req.secret = Some(secret);
                    made = Some(cleis_server::code_for(token, &[0; 32]));
                }
                Err(e) => {
                    self.toast(e, cx);
                    return;
                }
            }
        }
        self.org_request::<_, InviteReply>(
            "/v1/invites",
            req,
            move |app, r, cx| {
                if let (Ok(mut r), Some(o)) = (r, app.session().and_then(|s| s.org.as_mut())) {
                    if let Some(c) = made {
                        r.code = c;
                    }
                    o.ui.invite_code = Some(r.code);
                    o.ui.invite_sent = r.emailed.then_some(to);
                }
                cx.notify();
            },
            cx,
        );
    }

    pub fn org_member(&mut self, change: MemberChange, cx: &mut Context<Self>) {
        self.org_request::<_, MemberView>("/v1/members", change, |_, _, _| {}, cx);
    }

    /// Hands the Super Admin role to `member`; this member becomes Admin (D-60).
    pub fn org_transfer(&mut self, member: MemberId, cx: &mut Context<Self>) {
        self.org_request::<_, MemberView>(
            "/v1/members/transfer",
            IdOnly { id: member.0 },
            |app, r, cx| {
                if r.is_ok() {
                    app.toast(tr("Super Admin transferred"), cx);
                }
            },
            cx,
        );
    }

    pub fn org_devices(&mut self, cx: &mut Context<Self>) {
        if let Some(o) = self.session().and_then(|s| s.org.as_mut()) {
            o.ui.devices_busy = true;
        }
        self.org_request::<_, Vec<DeviceView>>(
            "/v1/me/devices",
            (),
            |app, r, cx| {
                if let Some(o) = app.session().and_then(|s| s.org.as_mut()) {
                    o.ui.devices_busy = false;
                    o.ui.devices = Some(r.unwrap_or_default());
                }
                cx.notify();
            },
            cx,
        );
    }

    pub fn org_sign_out(&mut self, device: String, cx: &mut Context<Self>) {
        self.org_request::<_, PushReply>(
            "/v1/me/devices/sign-out",
            DeviceRef { device },
            |app, _, cx| app.org_devices(cx),
            cx,
        );
    }

    /// Renames an organization space: its info sealed again with its key.
    pub fn org_rename_space(&mut self, id: SpaceId, name: String, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(mut info) = s.spaces.iter().find(|x| x.id == id).cloned() else {
            return;
        };
        let Some(key) = s.vault.space_key(id).cloned() else {
            return;
        };
        info.name = name.clone();
        let Ok(sealed) = cleis_store::seal_space_info(&key, &info) else {
            return;
        };
        if let Some(o) = s.org.as_mut() {
            o.names.insert(id, name);
        }
        self.org_request::<_, PushReply>(
            "/v1/spaces/update",
            SpaceUpdate { id, sealed },
            |_, _, _| {},
            cx,
        );
    }

    /// A folder's icon (mockup 48g): a personal folder's is kept in the vault; an organization
    /// space's is sealed again with its key and sent, so every member sees it.
    pub fn set_space_icon(&mut self, id: SpaceId, icon: Option<&str>, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        if let Err(e) = s.vault.set_space_icon(id, icon) {
            s.toast = Some(crate::trf!("Not done: {}", e).into());
            cx.notify();
            return;
        }
        let _ = crate::sync::reload(s);
        let org = s.org_of(id).map(|o| o.m.key());
        let info = s.spaces.iter().find(|x| x.id == id).cloned();
        let key = s.vault.space_key(id).cloned();
        if let (Some(org), Some(info), Some(key)) = (org, info, key)
            && let Ok(sealed) = cleis_store::seal_space_info(&key, &info)
        {
            self.in_org(&org, |app| {
                app.org_request::<_, PushReply>(
                    "/v1/spaces/update",
                    SpaceUpdate { id, sealed },
                    |_, _, _| {},
                    cx,
                )
            });
        }
        cx.notify();
    }

    /// Deletes an organization space for every member (its items with it).
    pub fn org_delete_space(&mut self, id: SpaceId, cx: &mut Context<Self>) {
        self.org_request::<_, PushReply>(
            "/v1/spaces/delete",
            IdOnly { id: id.0 },
            move |app, r, cx| {
                if r.is_ok()
                    && let Some(s) = app.session()
                {
                    let _ = s.vault.remove_org_space(id);
                    let _ = crate::sync::reload(s);
                    if s.filter == crate::app::Filter::Space(id) {
                        s.filter = crate::app::Filter::All;
                        s.refilter();
                    }
                }
                cx.notify();
            },
            cx,
        );
    }

    pub fn org_revoke(&mut self, member: MemberId, cx: &mut Context<Self>) {
        self.org_request::<_, PushReply>(
            "/v1/members/revoke",
            IdOnly { id: member.0 },
            |app, r, cx| {
                if r.is_ok() {
                    app.toast(tr("Removed · re-encrypt its spaces"), cx);
                }
            },
            cx,
        );
    }

    /// Clears a member's second sign-in step: set up again at the next sign-in (mockup 33d).
    pub fn org_reset_step(&mut self, member: MemberId, cx: &mut Context<Self>) {
        self.org_request::<_, PushReply>(
            "/v1/members/reset-step",
            IdOnly { id: member.0 },
            |app, r, cx| {
                if r.is_ok() {
                    app.toast(tr("Two-step verification reset"), cx);
                }
            },
            cx,
        );
    }

    /// Unlocks a member locked out by wrong PINs (D-79).
    pub fn org_reset_pin(&mut self, member: MemberId, cx: &mut Context<Self>) {
        self.org_request::<_, PushReply>(
            "/v1/members/reset-pin",
            IdOnly { id: member.0 },
            move |app, r, cx| {
                if r.is_ok() {
                    if let Some(m) = app
                        .session()
                        .and_then(|s| s.org.as_mut())
                        .and_then(|o| o.state.as_mut())
                        .and_then(|st| st.members.iter_mut().find(|m| m.id == member))
                    {
                        m.pin_locked = false;
                    }
                    app.toast(tr("PIN unlocked"), cx);
                }
            },
            cx,
        );
    }

    /// Security policy › Accessing: the password or PIN typed twice becomes the organization's
    /// (D-79, D-98). Only its digest leaves this app; the dialog closes once it is kept.
    pub fn org_set_access(&mut self, secret: Zeroizing<String>, cx: &mut Context<Self>) {
        let Some(o) = self.session().and_then(|s| s.org.as_mut()) else {
            return;
        };
        let Some(rule) = o.state.as_ref().map(|st| st.policy.access) else {
            return;
        };
        if !rule.accepts(&secret) {
            let why = match rule.with {
                AccessWith::Pin => crate::trf!("The PIN has {} digits", rule.length),
                _ => crate::trf!("At least {} characters", ACCESS_PASSWORD_MIN),
            };
            self.access_setup_failed(why.into(), cx);
            return;
        }
        let Some(key) = self.active_org_key() else {
            return;
        };
        let task = cx.background_spawn(async move { cleis_server::new_access_secret(&secret) });
        cx.spawn(async move |this, cx| {
            let digest = task.await;
            this.update(cx, |app, cx| {
                app.in_org(&key, |app| match digest {
                    Err(e) => app.access_setup_failed(e.into(), cx),
                    Ok(d) => app.org_request::<_, PushReply>(
                        "/v1/policy/access",
                        d,
                        |app, r, cx| {
                            let path = app.path.clone();
                            let Some(s) = app.session() else { return };
                            match r {
                                Ok(_) => {
                                    s.pin_setup = None;
                                    if let Some(o) = s.org.as_mut() {
                                        o.ui.access_pw = None;
                                        if let Some(st) = o.state.as_mut() {
                                            st.pin_set = true;
                                        }
                                        // The one who set it is not asked it right away here.
                                        let hours = o.local.pin_hours.unwrap_or(8).max(1);
                                        o.local.pin_until =
                                            Some(cleis_core::now() + i64::from(hours) * 3600);
                                        save_local_bare(&path, &o.m.key(), &o.local);
                                        o.again = true;
                                    }
                                    app.toast(tr("Saved"), cx);
                                }
                                Err(e) => app.access_setup_failed(e.into(), cx),
                            }
                        },
                        cx,
                    ),
                });
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// The set-up dialog (PIN boxes or password fields) shows why it was refused.
    fn access_setup_failed(&mut self, why: SharedString, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        if let Some(d) = s.pin_setup.as_mut() {
            d.busy = false;
            d.error = Some(crate::i18n::tr_dynamic(&why).into());
        }
        if let Some(f) = s.org.as_mut().and_then(|o| o.ui.access_pw.as_mut()) {
            f.busy = false;
            f.error = Some(why);
        }
        cx.notify();
    }

    /// Ends a member's sessions on every device.
    pub fn org_sign_out_member(&mut self, member: MemberId, cx: &mut Context<Self>) {
        self.org_request::<_, PushReply>(
            "/v1/members/sign-out",
            IdOnly { id: member.0 },
            |app, r, cx| {
                if r.is_ok() {
                    app.toast(tr("Signed out everywhere"), cx);
                }
            },
            cx,
        );
    }

    pub fn org_invite_action(&mut self, id: uuid::Uuid, renew: bool, cx: &mut Context<Self>) {
        if renew {
            let (address, to) = self
                .session()
                .and_then(|s| s.org.as_ref())
                .map(|o| {
                    let to = o
                        .state
                        .as_ref()
                        .and_then(|st| st.invites.iter().find(|i| i.id == id))
                        .map(|i| i.email.clone())
                        .unwrap_or_default();
                    (o.m.address.clone(), to)
                })
                .unwrap_or_default();
            // Shared folder (D-78): the new code is made here.
            let (secret, made) = if address == FOLDER {
                match crate::folder_org::new_invite_secret() {
                    Ok((token, secret)) => {
                        (Some(secret), Some(cleis_server::code_for(token, &[0; 32])))
                    }
                    Err(e) => {
                        self.toast(e, cx);
                        return;
                    }
                }
            } else {
                (None, None)
            };
            self.org_request::<_, InviteReply>(
                "/v1/invites/renew",
                RenewInvite {
                    id,
                    address,
                    secret,
                },
                move |app, r, cx| {
                    if let (Ok(mut r), Some(o)) = (r, app.session().and_then(|s| s.org.as_mut())) {
                        if let Some(c) = made {
                            r.code = c;
                        }
                        o.ui.invite_code = Some(r.code);
                        o.ui.invite_sent = r.emailed.then_some(to);
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
    pub fn org_new_space(
        &mut self,
        name: String,
        mode: SpaceMode,
        reserved: bool,
        cx: &mut Context<Self>,
    ) {
        self.org_create_space(name, mode, reserved, (false, false), true, cx);
    }

    /// A space made by an import (D-102): usable at once, the list stays where it is.
    pub fn org_new_space_quiet(&mut self, name: String, cx: &mut Context<Self>) -> Option<SpaceId> {
        self.org_create_space(name, SpaceMode::Standard, false, (false, false), false, cx)
    }

    /// A contacts folder of the organization (27/11/2026), or its "Uncategorized" space
    /// (`unfiled`, given to every member). Returns the new space, usable at once.
    pub fn org_new_contacts_space(
        &mut self,
        name: String,
        unfiled: bool,
        cx: &mut Context<Self>,
    ) -> Option<SpaceId> {
        self.org_create_space(name, SpaceMode::Standard, false, (true, unfiled), true, cx)
    }

    fn org_create_space(
        &mut self,
        name: String,
        mode: SpaceMode,
        reserved: bool,
        (for_contacts, unfiled): (bool, bool),
        show: bool,
        cx: &mut Context<Self>,
    ) -> Option<SpaceId> {
        // A reserved space is end-to-end (D-61).
        let mode = if reserved { SpaceMode::EndToEnd } else { mode };
        let s = self.session()?;
        let o = s.org.as_ref()?;
        let st = o.state.as_ref()?;
        let info = SpaceInfo {
            id: SpaceId::new(),
            name: name.trim().into(),
            kind: SpaceKind::Organization,
            mode: match mode {
                SpaceMode::Standard => EncryptionMode::Standard,
                SpaceMode::EndToEnd => EncryptionMode::EndToEnd,
            },
            for_contacts,
            unfiled,
            icon: None,
        };
        // "Uncategorized" is everyone's: its key wrapped for each member right away.
        let everyone: Vec<(MemberId, [u8; 32])> = if unfiled {
            st.members
                .iter()
                .filter(|m| !m.revoked && m.id != st.me.id)
                .map(|m| (m.id, m.account_public))
                .collect()
        } else {
            vec![]
        };
        let org_public = st.org.org_public.clone();
        let recovery = st.org.recovery_public.clone();
        let key = o.m.key();
        let Ok((sealed, wrapped, k)) = s.vault.create_org_space(&key, info.clone()) else {
            return None;
        };
        let grants: Vec<Wrap> = everyone
            .iter()
            .filter_map(|(m, p)| {
                wrap_key(&PublicKey(*p), &k, info.id.0.as_bytes())
                    .ok()
                    .map(|w| Wrap {
                        space: info.id,
                        member: *m,
                        wrapped: w,
                    })
            })
            .collect();
        let wrap_to = |p: &Option<Vec<u8>>| {
            p.as_ref()
                .and_then(|p| p.clone().try_into().ok())
                .and_then(|p: [u8; 32]| wrap_key(&PublicKey(p), &k, info.id.0.as_bytes()).ok())
        };
        let req = NewSpace {
            id: info.id,
            sealed,
            mode,
            reserved,
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
        if for_contacts && show {
            s.contacts_ui.filter = if unfiled {
                crate::screens::contacts::ContactFilter::Org(key.clone())
            } else {
                crate::screens::contacts::ContactFilter::Folder(id)
            };
        }
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
                    if !for_contacts && show {
                        s.filter = crate::app::Filter::Space(id);
                        s.refilter();
                    }
                    if !grants.is_empty() {
                        app.org_request::<_, PushReply>(
                            "/v1/grants",
                            Grants { wraps: grants },
                            |_, _, _| {},
                            cx,
                        );
                    }
                }
                cx.notify();
            },
            cx,
        );
        Some(id)
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
            self.toast(tr("Sync first: this space's key is not here yet"), cx);
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
                    let reserve = s.org.as_mut().and_then(|o| {
                        o.ui.rekey = None;
                        o.ui.reserve_after.take()
                    });
                    s.toast = Some(tr("Re-encrypted").into());
                    if let Some(id) = reserve {
                        app.org_reserve(id, true, cx);
                    }
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
        let rx = cx.prompt_for_new_path(&dir, Some(&crate::trf!("{} recovery key.txt", name)));
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
            s.toast = Some(crate::trf!("Not saved: {}", e).into());
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
                    app.toast(tr("Recovery key saved"), cx);
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
            prompt: Some(tr("Use recovery key").into()),
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
            self.toast(tr("Not a Cleis recovery key"), cx);
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
            self.toast(tr("Nobody is waiting for a space this key opens"), cx);
            return;
        }
        self.org_request::<_, PushReply>(
            "/v1/grants",
            Grants { wraps },
            move |app, r, cx| {
                if r.is_ok() {
                    app.toast(crate::trf!("Access restored · {}", n), cx);
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
                    app.toast(tr("New server key announced"), cx);
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
            save_memberships(s);
        }
    }

    // ---------- Activity reported by the app ----------

    /// Views, copies, fills of an organization's item (written if the policy records them).
    /// Writes an event without an item (an export of several contacts).
    pub fn org_log(
        &mut self,
        action: Action,
        space: Option<SpaceId>,
        note: String,
        cx: &mut Context<Self>,
    ) {
        // To the organization of `space` when given (D-69).
        if let Some(key) =
            space.and_then(|sp| self.session().and_then(|s| s.org_of(sp).map(|o| o.m.key())))
            && self.active_org_key().as_deref() != Some(key.as_str())
        {
            self.in_org(&key, |app| app.org_log(action, space, note, cx));
            return;
        }
        let detail = self.session().and_then(|s| {
            let o = s.org.as_ref()?;
            let st = o.state.as_ref()?;
            let p: [u8; 32] = st.org.org_public.clone()?.try_into().ok()?;
            let d = serde_json::to_vec(&EventDetail {
                item: note,
                field: String::new(),
            })
            .ok()?;
            cleis_crypto::seal_to(&PublicKey(p), DETAIL_CONTEXT, &d).ok()
        });
        let req = Report {
            action,
            space,
            item: None,
            detail,
        };
        let Some(mut conn) = self.org_conn() else {
            return;
        };
        cx.background_spawn(async move {
            let _: Result<PushReply, _> = conn.call("/v1/log", &req);
        })
        .detach();
    }

    /// Reports an action on an item to the organization it belongs to (D-69).
    pub fn org_report(
        &mut self,
        action: Action,
        entry: &Entry,
        field: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(key) = self
            .session()
            .and_then(|s| s.org_of(entry.space).map(|o| o.m.key()))
        else {
            return;
        };
        self.in_org(&key, |app| app.org_report_here(action, entry, field, cx));
    }

    fn org_report_here(
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
        // Reserved spaces: readable by the Super Admins only (D-61).
        let reserved = o.is_reserved(entry.space);
        let detail = if reserved {
            st.org.super_public.as_ref()
        } else {
            st.org.org_public.as_ref()
        };
        let detail = detail
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
                        app.toast(crate::trf!("Log intact · {} events", i.checked), cx);
                    } else {
                        app.toast_for(
                            crate::trf!("Log altered: {}", i.problem.unwrap_or_default()),
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
        s.org.as_ref()?.open_detail(e)
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
                        format!("{} · {}", d.item, crate::i18n::tr_owned(&d.field))
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
        let name = crate::trf!("{} activity.csv", o.m.name);
        let rx = cx.prompt_for_new_path(&dir, Some(&name));
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(path))) = rx.await {
                let r = std::fs::write(&path, csv);
                this.update(cx, |app, cx| {
                    app.toast(
                        match r {
                            Ok(_) => tr("Exported").to_owned(),
                            Err(e) => crate::trf!("Not exported: {}", e),
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
        let copy: Option<(crate::remote::RemotePlace, crate::remote::Secret)> = s
            .vault
            .org_setting(crate::remote::ORG_BACKUP_SETTING)
            .ok()
            .flatten()
            .flatten();
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
            let file = dir.join(&name);
            std::fs::write(&file, &sealed).map_err(|e| e.to_string())?;
            // A copy at the place chosen (GitHub, SFTP): sealed, like the local one.
            if let Some((place, secret)) = copy {
                crate::remote::upload(&place.file(&name), &secret, &file, None)
                    .map_err(|e| crate::trf!("kept here, not copied ({})", e))?;
            }
            Ok::<usize, String>(bytes.len())
        });
        cx.spawn(async move |this, cx| {
            let r = task.await;
            this.update(cx, |app, cx| {
                app.toast(
                    match r {
                        Ok(n) => {
                            crate::trf!(
                                "Organization backed up · {}",
                                crate::format::size(n as u64)
                            )
                        }
                        Err(e) => crate::trf!("Not backed up: {}", e),
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
            Ok(n) => crate::trf!("Backup readable · {}", crate::format::size(n as u64)),
            Err(e) => crate::trf!("Backup unreadable: {}", e),
        };
        self.toast(msg, cx);
    }

    pub fn org_restore_backup(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let bytes = match backup_key(&s.vault).and_then(|k| open_backup(&k, &path)) {
            Ok(b) => b,
            Err(e) => {
                s.toast = Some(crate::trf!("Backup unreadable: {}", e).into());
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
                        Ok(_) => tr("Organization restored").to_owned(),
                        Err(e) => crate::trf!("Not restored: {}", e),
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
        let rx = cx.prompt_for_new_path(&dir, Some(&crate::trf!("{} backup key.txt", name)));
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
        .map_err(|_| tr("wrong backup key").to_owned())?;
    if !bytes.starts_with(b"SQLite format 3") {
        return Err(tr("not a server file").into());
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

/// Hides the organization's entries and contacts: not reached for longer than the policy
/// allows, or its PIN not given yet (D-79).
pub fn hide_org_entries(s: &mut Session) {
    // Only those organizations (D-69: each has its own policy).
    let org_spaces: Vec<SpaceId> = s
        .all_orgs()
        .filter(|o| o.offline_locked || o.pin.is_some())
        .flat_map(|o| {
            s.vault
                .org_spaces(&o.m.key())
                .unwrap_or_default()
                .into_iter()
                .map(|x| x.0)
                .chain(o.names.keys().copied())
        })
        .collect();
    if org_spaces.is_empty() {
        return;
    }
    if s.selected_entry()
        .is_some_and(|e| org_spaces.contains(&e.space))
    {
        s.selected = None;
    }
    s.entries.retain(|e| !org_spaces.contains(&e.space));
    s.contacts.retain(|c| !org_spaces.contains(&c.space));
    s.spaces.retain(|sp| !org_spaces.contains(&sp.id));
    s.refilter();
}

/// Applies a sync: keys of the spaces given or taken away, rows, the organization key.
fn apply_state(s: &mut Session, st: State, changes: &Changes) {
    let Some(o) = s.org.as_mut() else { return };
    let key = o.m.key();
    let mut changed = false;
    let mut pins_changed = false;
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
            // Renamed (or its icon changed, 0.2.4) elsewhere: same key, new sealed info.
            let renamed = s.vault.space_key(sv.id).is_some_and(|k| {
                cleis_store::open_space_info(k, sv.id, &sv.sealed).is_ok_and(|i| {
                    s.spaces
                        .iter()
                        .find(|x| x.id == sv.id)
                        .is_none_or(|x| x.name != i.name || x.icon != i.icon)
                })
            });
            if renamed
                && s.vault
                    .put_org_space(&key, sv.id, &sv.sealed, w, sv.key_gen)
                    .is_ok()
            {
                changed = true;
            }
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
    if st.super_key.is_none() {
        o.super_key = None;
    } else if o.super_key.is_none()
        && let Some(w) = &st.super_key
    {
        o.super_key = s.vault.unwrap_own(w, SUPER_KEY_CONTEXT).ok();
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
            pins_changed = true;
        }
    }
    let renamed = o.m.name != st.org.name;
    o.m.name.clone_from(&st.org.name);
    let was_locked = o.offline_locked;
    o.offline_locked = false;
    o.state = Some(st);
    if pins_changed || renamed {
        save_memberships(s);
    }
    if changed || was_locked {
        let _ = crate::sync::reload(s);
    }
}

impl CleisApp {
    /// The password or PIN typed, or Windows Hello asked (mockups 36h, 42g, 42h). A server
    /// checks the password or PIN at the next sign-in; a shared folder's is checked here, and
    /// Windows Hello by this computer.
    pub fn org_pin_submit(&mut self, cx: &mut Context<Self>) {
        let conn = self.org_conn();
        let Some(o) = self.session().and_then(|s| s.org.as_mut()) else {
            return;
        };
        let key = o.m.key();
        let Some(p) = o.pin.as_mut() else { return };
        if p.busy || p.locked {
            return;
        }
        let typed = match p.with {
            AccessWith::Hello => None,
            AccessWith::Pin => {
                let Some(b) = p.boxes.as_ref().filter(|b| b.full()) else {
                    p.error = Some(crate::trf!("The PIN has {} digits", p.len));
                    cx.notify();
                    return;
                };
                Some(Zeroizing::new(b.value.to_string()))
            }
            AccessWith::Password => {
                let v = p
                    .field
                    .as_ref()
                    .map(|f| f.read(cx).value().to_string())
                    .unwrap_or_default();
                if v.is_empty() {
                    p.error = Some(tr("Enter the organization's password").into());
                    cx.notify();
                    return;
                }
                Some(Zeroizing::new(v))
            }
        };
        p.busy = true;
        p.error = None;
        cx.notify();
        if !p.here {
            let (Some(mut conn), Some(typed)) = (conn, typed) else {
                return;
            };
            // A new sign-in: the session (if any) was opened before it was asked.
            conn.client.token = None;
            conn.pin = Some(typed);
            self.org_run(conn, cx);
            return;
        }
        let secret = o.local.secret.clone();
        let task = cx.background_spawn(async move {
            match typed {
                None => crate::opening::hello_check()
                    .map(|_| true)
                    .map_err(|e| e.to_string()),
                Some(t) => Ok(secret.is_some_and(|s| cleis_server::access_matches(&s, &t))),
            }
        });
        cx.spawn(async move |this, cx| {
            let r = task.await;
            this.update(cx, |app, cx| {
                app.in_org(&key, |app| app.org_checked_here(r, cx));
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// What this computer found (shared folder, Windows Hello): the spaces open until "Ask again
    /// after", or a wrong try is counted.
    fn org_checked_here(&mut self, r: Result<bool, String>, cx: &mut Context<Self>) {
        let path = self.path.clone();
        let Some(s) = self.session() else { return };
        let Some(o) = s.org.as_mut() else { return };
        let key = o.m.key();
        let Some(p) = o.pin.as_mut() else { return };
        p.busy = false;
        match r {
            Ok(true) => {
                o.pin = None;
                o.local.fails = 0;
                o.local.pin_until = Some(
                    cleis_core::now() + i64::from(o.local.pin_hours.unwrap_or(8).max(1)) * 3600,
                );
                save_local_bare(&path, &key, &o.local);
                let _ = crate::sync::reload(s);
            }
            Ok(false) => {
                o.local.fails += 1;
                let tries = o.local.tries;
                if tries > 0 && o.local.fails >= tries {
                    o.local.locked_at = Some(cleis_core::now());
                    p.locked = true;
                } else {
                    let left = tries.saturating_sub(o.local.fails);
                    p.error = Some(if tries == 0 {
                        tr("Wrong, try again").into()
                    } else if left == 1 {
                        tr("Wrong. 1 try left.").into()
                    } else {
                        crate::trf!("Wrong. {} tries left.", left)
                    });
                }
                if let Some(b) = p.boxes.as_mut() {
                    b.clear();
                }
                p.focused = false;
                save_local_bare(&path, &key, &o.local);
            }
            Err(e) => p.error = Some(crate::i18n::tr_dynamic(&e)),
        }
        cx.notify();
    }

    /// Called every second (D-98): an organization asks again after its own inactivity delay, or
    /// once "Ask again after" has passed on this computer.
    pub fn org_idle_check(&mut self, cx: &mut Context<Self>) {
        if self.demo {
            return;
        }
        let path = self.path.clone();
        let Some(s) = self.session() else { return };
        let idle = s.last_activity.elapsed().as_secs();
        let now = cleis_core::now();
        let mut hid = false;
        for o in s.org.iter_mut().chain(s.orgs_idle.iter_mut()) {
            if o.pin.is_some() || o.local.access_with.is_none() {
                continue;
            }
            let idle_due = o.local.idle_mins.is_some_and(|m| idle >= u64::from(m) * 60);
            let hours_due = o.local.pin_until.is_some_and(|t| now >= t);
            if idle_due || hours_due {
                o.local.pin_until = None;
                o.pin = Some(prompt_for(&o.local, o.m.in_folder()));
                save_local_bare(&path, &o.m.key(), &o.local);
                hid = true;
            }
        }
        if hid {
            hide_org_entries(s);
            cx.notify();
        }
    }

    /// Reserves a space or frees it (Super Admin, D-61). A Standard space is re-encrypted
    /// end-to-end first: the dialog opens for that.
    pub fn org_reserve(&mut self, id: SpaceId, reserved: bool, cx: &mut Context<Self>) {
        let Some(o) = self.session().and_then(|s| s.org.as_mut()) else {
            return;
        };
        let standard = o
            .state
            .as_ref()
            .and_then(|s| s.spaces.iter().find(|x| x.id == id))
            .is_some_and(|x| x.mode == SpaceMode::Standard);
        if reserved && standard {
            o.ui.rekey = Some(crate::screens::org::RekeyForm {
                space: id,
                to: SpaceMode::EndToEnd,
                backup_first: true,
            });
            o.ui.reserve_after = Some(id);
            cx.notify();
            return;
        }
        if let Some(sv) = o
            .state
            .as_mut()
            .and_then(|s| s.spaces.iter_mut().find(|x| x.id == id))
        {
            sv.reserved = reserved;
        }
        self.org_request::<_, PushReply>(
            "/v1/spaces/reserve",
            Reserve { id, reserved },
            |_, _, _| {},
            cx,
        );
    }
}

/// The card an organization shows before its spaces (D-98): what it asks, checked by the server
/// or here.
pub fn prompt_for(local: &Local, folder: bool) -> PinPrompt {
    let with = local.access_with.unwrap_or(AccessWith::Pin);
    let mut p = PinPrompt::asking(
        with,
        local.pin_len.unwrap_or(PIN_DEFAULT_LEN),
        folder || with == AccessWith::Hello,
    );
    p.locked = folder && local.locked_at.is_some();
    p
}

/// This computer's file, without the remembered-device token (kept in the vault).
pub fn save_local_bare(vault: &Path, key: &str, l: &Local) {
    let mut bare = l.clone();
    bare.remembered = None;
    save_local(vault, key, &bare);
}

/// Side of the organization's logo, in pixels (sharp at the 48 px of the access card, D-98).
const LOGO_SIDE: u32 = 128;

/// A picture as the organization's logo: the centred square at [`LOGO_SIDE`] px, PNG.
pub fn logo_png(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let side = img.width().min(img.height());
    if side == 0 {
        return Err("Empty picture".into());
    }
    let sq = img.crop_imm(
        (img.width() - side) / 2,
        (img.height() - side) / 2,
        side,
        side,
    );
    let small = sq.resize_exact(
        LOGO_SIDE.min(side),
        LOGO_SIDE.min(side),
        image::imageops::FilterType::Lanczos3,
    );
    let mut out = std::io::Cursor::new(Vec::new());
    small
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    let out = out.into_inner();
    if out.len() > LOGO_MAX {
        return Err("The logo is too large".into());
    }
    Ok(out)
}

impl CleisApp {
    /// Server › Logo (D-98): set, or taken away.
    pub fn org_set_logo(&mut self, png: Option<Vec<u8>>, cx: &mut Context<Self>) {
        let shown = png.as_ref().map(|b| data_encoding::BASE64.encode(b));
        self.org_request::<_, PushReply>(
            "/v1/org/logo",
            LogoSet { png },
            move |app, r, cx| {
                let path = app.path.clone();
                if r.is_ok()
                    && let Some(o) = app.session().and_then(|s| s.org.as_mut())
                {
                    o.local.logo = shown;
                    if let Some(st) = o.state.as_mut() {
                        st.org.logo = o
                            .local
                            .logo
                            .as_ref()
                            .and_then(|b| data_encoding::BASE64.decode(b.as_bytes()).ok());
                    }
                    save_local_bare(&path, &o.m.key(), &o.local);
                }
                cx.notify();
            },
            cx,
        );
    }

    /// Server › Delete (42j, D-98): an export first if asked, then the server erases everything;
    /// a server on this computer stops and its folder goes, a shared folder keeps only the mark
    /// that it was deleted. The organization then leaves this vault.
    pub fn org_delete_server(&mut self, cx: &mut Context<Self>) {
        let vault = self.path.clone();
        let Some(mut conn) = self.org_conn() else {
            return;
        };
        let Some(s) = self.session() else { return };
        let backup = backup_key(&s.vault).ok();
        let Some(o) = s.org.as_mut() else { return };
        let Some(f) = o.ui.delete_org.as_mut() else {
            return;
        };
        let name = f.name.read(cx).value().trim().to_string();
        let export = f.export;
        f.busy = true;
        f.error = None;
        let key = o.m.key();
        let dir = org_backups_dir(&vault, &o.m);
        let host_dir = o.local.host_dir.clone();
        let folder =
            o.m.in_folder()
                .then(|| replica_dir(&vault, &key).display().to_string());
        cx.notify();
        let task = cx.background_spawn(async move {
            if export {
                let k = backup.ok_or_else(|| "No backup key".to_string())?;
                let bytes = conn
                    .send("GET", "/v1/backup", None)
                    .map_err(|e| e.to_string())?;
                let sealed =
                    cleis_crypto::seal(&k, BACKUP_CONTEXT, &bytes).map_err(|e| e.to_string())?;
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                let file = dir.join(format!(
                    "server-{}.cleisorg",
                    crate::format::stamp(cleis_core::now())
                ));
                std::fs::write(&file, &sealed).map_err(|e| e.to_string())?;
            }
            let _: PushReply = conn
                .call("/v1/org/delete", &OrgDelete { name })
                .map_err(|e| e.to_string())?;
            if let Some(rkey) = folder {
                crate::folder_org::mark_deleted(&rkey);
            }
            if let Some(d) = host_dir {
                crate::host::stop();
                let _ = std::fs::remove_dir_all(d);
            }
            Ok::<(), String>(())
        });
        let key2 = o.m.key();
        cx.spawn(async move |this, cx| {
            let r = task.await;
            this.update(cx, |app, cx| {
                app.in_org(&key2, |app| match r {
                    Ok(()) => {
                        let name = app
                            .session()
                            .and_then(|s| s.org.as_ref())
                            .map(|o| o.m.name.clone())
                            .unwrap_or_default();
                        app.org_leave(cx);
                        app.toast(crate::trf!("{} was deleted", name), cx);
                    }
                    Err(e) => {
                        if let Some(f) = app
                            .session()
                            .and_then(|s| s.org.as_mut())
                            .and_then(|o| o.ui.delete_org.as_mut())
                        {
                            f.busy = false;
                            f.error = Some(crate::i18n::tr_dynamic(&e).into());
                        }
                        cx.notify();
                    }
                });
            })
            .ok();
        })
        .detach();
    }
}
