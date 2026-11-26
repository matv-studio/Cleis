//! An organization in a shared folder (D-78): no server. The folder (SharePoint library synced by
//! the OneDrive app, Google shared drive, NAS share) holds the organization's first state
//! (`genesis.db`) and the operations its members signed (`ops/<signing key>/<number>.json`), each
//! member writing only in their own subfolder, so two computers never write the same file.
//!
//! Each app keeps a copy (`replica`) rebuilt from them in a fixed order (time, signing key,
//! number) through the server's own API (`cleis_server::local`): the same checks of rights, the
//! same results on every copy. The app talks to it as to a server (`cleis_net::Local`).

use crate::folder_store::{LocalStore, Store};
use cleis_core::MemberId;
use cleis_crypto::{SigningKeyPair, VerifyingKey, sha256, verify};
use cleis_net::{Local, LocalApi, Response};
use cleis_server::local::{Answer, Replay};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const OP_DOMAIN: &str = "cleis/folder-op/v1";
/// Marks a folder that holds an organization.
pub const ORG_FILE: &str = "cleis-org.json";
pub const GENESIS: &str = "genesis.db";

const B64: data_encoding::Encoding = data_encoding::BASE64;

/// What `cleis-org.json` says.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrgFile {
    pub id: uuid::Uuid,
    pub name: String,
    pub version: u32,
}

/// One signed operation (an API request).
#[derive(Clone, Debug, Serialize, Deserialize)]
struct OpFile {
    v: u8,
    member: Option<MemberId>,
    device: String,
    seq: u64,
    at: i64,
    seed: String,
    method: String,
    path: String,
    body: String,
    signer: String,
    #[serde(default)]
    sig: String,
}

impl OpFile {
    fn signed(&self) -> Vec<u8> {
        let mut bare = self.clone();
        bare.sig.clear();
        sha256(&serde_json::to_vec(&bare).unwrap_or_default()).to_vec()
    }

    fn key(&self) -> OpKey {
        (self.at, self.signer.clone(), self.seq)
    }

    fn signer_key(&self) -> Option<[u8; 32]> {
        B64.decode(self.signer.as_bytes()).ok()?.try_into().ok()
    }

    fn valid_signature(&self) -> bool {
        let (Some(k), Ok(sig)) = (self.signer_key(), B64.decode(self.sig.as_bytes())) else {
            return false;
        };
        let Ok(sig): Result<[u8; 64], _> = sig.try_into() else {
            return false;
        };
        verify(&VerifyingKey(k), OP_DOMAIN, &self.signed(), &sig).is_ok()
    }
}

/// Order of the operations: time, signing key, number.
type OpKey = (i64, String, u64);

/// Requests that change nothing: answered by the copy, not written to the folder.
const READS: [&str; 6] = [
    "/v1/pull",
    "/v1/log/query",
    "/v1/log/verify",
    "/v1/versions",
    "/v1/invite-info",
    "/v1/me/devices",
];

/// What a shared folder cannot do (no server to send codes, count PINs, hold sessions).
const NOT_HERE: [&str; 10] = [
    "/v1/challenge",
    "/v1/session",
    "/v1/me/code",
    "/v1/me/code/confirm",
    "/v1/rotate",
    "/v1/restore",
    "/v1/members/sign-out",
    "/v1/me/devices/sign-out",
    "/v1/policy/pin",
    "/v1/members/reset-pin",
];

pub struct Engine {
    store: Box<dyn Store>,
    label: String,
    replica: PathBuf,
    me: Option<MemberId>,
    device: String,
    signing: SigningKeyPair,
    app: Option<Arc<cleis_server::App>>,
    applied: Vec<OpKey>,
    cache: HashMap<String, OpFile>,
}

fn answer(status: u16, error: &str, message: &str) -> Answer {
    Answer {
        status,
        body: serde_json::to_vec(&cleis_core::org::ApiError {
            error: error.into(),
            message: message.into(),
        })
        .unwrap_or_default(),
    }
}

fn hex_key(k: &[u8; 32]) -> String {
    cleis_crypto::hex(k)
}

impl Engine {
    /// A folder of this computer that already holds an organization.
    pub fn open(
        folder: &Path,
        replica: &Path,
        me: Option<MemberId>,
        device: &str,
        signing: SigningKeyPair,
    ) -> Result<Engine, String> {
        Engine::open_store(
            Box::new(LocalStore(folder.to_owned())),
            &folder.display().to_string(),
            replica,
            me,
            device,
            signing,
        )
    }

    /// A folder (of this computer, or on an SSH server) that already holds an organization.
    pub fn open_store(
        mut store: Box<dyn Store>,
        label: &str,
        replica: &Path,
        me: Option<MemberId>,
        device: &str,
        signing: SigningKeyPair,
    ) -> Result<Engine, String> {
        if store.read(ORG_FILE)?.is_none() || store.read(GENESIS)?.is_none() {
            return Err(crate::trf!("No Cleis organization in {}", label));
        }
        let mut e = Engine {
            store,
            label: label.to_owned(),
            replica: replica.to_owned(),
            me,
            device: device.to_owned(),
            signing,
            app: None,
            applied: vec![],
            cache: HashMap::new(),
        };
        e.rebuild()?;
        e.refresh();
        Ok(e)
    }

    pub fn org_file(folder: &Path) -> Option<OrgFile> {
        Engine::org_file_in(&mut LocalStore(folder.to_owned()))
    }

    pub fn org_file_in(store: &mut dyn Store) -> Option<OrgFile> {
        serde_json::from_slice(&store.read(ORG_FILE).ok()??).ok()
    }

    /// Creates the organization `org` in `folder` (empty, or new) with its first administrator:
    /// this vault (`name`, keys). Returns the engine, the organization's id and the member.
    pub fn create(
        folder: &Path,
        replica: &Path,
        org: &str,
        name: &str,
        account_public: [u8; 32],
        signing: SigningKeyPair,
        device: &str,
    ) -> Result<(Engine, uuid::Uuid, MemberId), String> {
        std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
        Engine::create_in(
            Box::new(LocalStore(folder.to_owned())),
            &folder.display().to_string(),
            replica,
            org,
            name,
            account_public,
            signing,
            device,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_in(
        mut store: Box<dyn Store>,
        label: &str,
        replica: &Path,
        org: &str,
        name: &str,
        account_public: [u8; 32],
        signing: SigningKeyPair,
        device: &str,
    ) -> Result<(Engine, uuid::Uuid, MemberId), String> {
        if store.read(ORG_FILE)?.is_some() {
            return Err(crate::trf!("{} already holds an organization", label));
        }
        let _ = std::fs::remove_dir_all(replica);
        let code = cleis_server::App::init(replica, org, &[]).map_err(|e| e.to_string())?;
        let app = cleis_server::App::open(replica).map_err(|e| e.to_string())?;
        let inv = cleis_core::org::Invitation::parse(&code).ok_or("no code")?;
        let mut req = cleis_core::org::JoinRequest {
            token: inv.token.to_vec(),
            name: name.into(),
            email: String::new(),
            device: device.into(),
            account_public,
            verify_key: signing.verifying_key().0,
            signature: [0; 64],
            proof: vec![],
        };
        req.signature = signing.sign(cleis_core::org::JOIN_DOMAIN, &req.signed_bytes());
        let seed: [u8; 32] = cleis_crypto::random_bytes().map_err(|e| e.to_string())?;
        let a = cleis_server::local::call(
            &app,
            Replay {
                member: None,
                device: device.into(),
                at: cleis_core::now(),
                seed,
            },
            "POST",
            "/v1/join",
            serde_json::to_vec(&req).map_err(|e| e.to_string())?,
        );
        if a.status != 200 {
            return Err(String::from_utf8_lossy(&a.body).into_owned());
        }
        let reply: cleis_core::org::JoinReply =
            serde_json::from_slice(&a.body).map_err(|e| e.to_string())?;
        // The copy as it is now becomes everyone's start.
        let snap = replica.with_extension("genesis");
        let _ = std::fs::remove_file(&snap);
        {
            let conn = app.db.lock().unwrap_or_else(|e| e.into_inner());
            conn.execute("VACUUM INTO ?1", [snap.to_string_lossy().as_ref()])
                .map_err(|e| e.to_string())?;
        }
        let bytes = std::fs::read(&snap).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(&snap);
        store.write(GENESIS, &bytes)?;
        let info = OrgFile {
            id: reply.org.id,
            name: reply.org.name.clone(),
            version: 1,
        };
        store.write(
            ORG_FILE,
            &serde_json::to_vec_pretty(&info).map_err(|e| e.to_string())?,
        )?;
        drop(app);
        let e = Engine::open_store(store, label, replica, Some(reply.member), device, signing)?;
        Ok((e, reply.org.id, reply.member))
    }

    fn my_dir(&self) -> String {
        format!("ops/{}", hex_key(&self.signing.verifying_key().0))
    }

    /// Every operation in the folder, sorted (files being written are skipped till complete).
    fn scan(&mut self) -> Vec<OpFile> {
        let mut out = vec![];
        let Ok(dirs) = self.store.list("ops") else {
            return out;
        };
        for d in dirs {
            let Ok(files) = self.store.list(&format!("ops/{d}")) else {
                continue;
            };
            for f in files {
                if !f.ends_with(".json") {
                    continue;
                }
                let rel = format!("ops/{d}/{f}");
                if let Some(op) = self.cache.get(&rel) {
                    out.push(op.clone());
                    continue;
                }
                let Some(op) = self
                    .store
                    .read(&rel)
                    .ok()
                    .flatten()
                    .and_then(|b| serde_json::from_slice::<OpFile>(&b).ok())
                else {
                    continue;
                };
                self.cache.insert(rel, op.clone());
                out.push(op);
            }
        }
        out.sort_by_key(OpFile::key);
        out
    }

    /// The copy from the folder's start, all operations replayed.
    fn rebuild(&mut self) -> Result<(), String> {
        self.app = None;
        let _ = std::fs::remove_dir_all(&self.replica);
        std::fs::create_dir_all(&self.replica).map_err(|e| e.to_string())?;
        let genesis = self
            .store
            .read(GENESIS)?
            .ok_or_else(|| crate::trf!("No Cleis organization in {}", self.label))?;
        std::fs::write(self.replica.join(cleis_server::DB_FILE), genesis)
            .map_err(|e| e.to_string())?;
        self.app = Some(cleis_server::App::open(&self.replica).map_err(|e| e.to_string())?);
        self.applied.clear();
        Ok(())
    }

    /// Applies what is new in the folder; returns the answers of the operations applied now.
    fn refresh(&mut self) -> Vec<(OpKey, Answer)> {
        let ops = self.scan();
        let keys: Vec<OpKey> = ops.iter().map(OpFile::key).collect();
        if !keys.starts_with(&self.applied) {
            // An operation arrived late, before others already applied: start again.
            if self.rebuild().is_err() {
                return vec![];
            }
        }
        let new: Vec<OpFile> = ops[self.applied.len()..].to_vec();
        let mut answers = vec![];
        for op in new {
            let a = self.apply(&op);
            self.applied.push(op.key());
            answers.push((op.key(), a));
        }
        answers
    }

    /// One operation, if its signature and its signer hold.
    fn apply(&self, op: &OpFile) -> Answer {
        let Some(app) = &self.app else {
            return answer(500, "server", "no copy");
        };
        if op.v != 1 || !op.valid_signature() {
            return answer(400, "bad_request", "Unsigned operation.");
        }
        let signer = op.signer_key().unwrap_or_default();
        let Ok(body) = B64.decode(op.body.as_bytes()) else {
            return answer(400, "bad_request", "Unreadable operation.");
        };
        let ok = match op.member {
            Some(m) => {
                let conn = app.db.lock().unwrap_or_else(|e| e.into_inner());
                conn.query_row(
                    "SELECT verify_key FROM members WHERE id = ?1",
                    [m.to_string()],
                    |r| r.get::<_, Vec<u8>>(0),
                )
                .is_ok_and(|k| k == signer)
            }
            // A new member: their own keys sign their request.
            None => {
                op.path == "/v1/join"
                    && serde_json::from_slice::<cleis_core::org::JoinRequest>(&body)
                        .is_ok_and(|j| j.verify_key == signer)
            }
        };
        if !ok {
            return answer(403, "forbidden", "Not signed by this member.");
        }
        let seed: [u8; 32] = B64
            .decode(op.seed.as_bytes())
            .ok()
            .and_then(|s| s.try_into().ok())
            .unwrap_or([0; 32]);
        cleis_server::local::call(
            app,
            Replay {
                member: op.member,
                device: op.device.clone(),
                at: op.at,
                seed,
            },
            &op.method,
            &op.path,
            body,
        )
    }

    /// A change: signed into this member's subfolder, then replayed with the others'.
    fn write_op(&mut self, method: &str, path: &str, body: &[u8]) -> Answer {
        let dir = self.my_dir();
        let seq = self
            .store
            .list(&dir)
            .unwrap_or_default()
            .iter()
            .filter_map(|f| f.strip_suffix(".json").and_then(|s| s.parse::<u64>().ok()))
            .max()
            .unwrap_or(0)
            + 1;
        let seed: [u8; 32] = cleis_crypto::random_bytes().unwrap_or([0; 32]);
        // After the last operation applied here (another computer's clock may run ahead), so that
        // it comes after what this computer saw.
        let at = cleis_core::now().max(self.applied.last().map_or(0, |k| k.0 + 1));
        let mut op = OpFile {
            v: 1,
            member: if path == "/v1/join" { None } else { self.me },
            device: self.device.clone(),
            seq,
            at,
            seed: B64.encode(&seed),
            method: method.into(),
            path: path.into(),
            body: B64.encode(body),
            signer: B64.encode(&self.signing.verifying_key().0),
            sig: String::new(),
        };
        op.sig = B64.encode(&self.signing.sign(OP_DOMAIN, &op.signed()));
        let file = format!("{dir}/{seq:010}.json");
        let bytes = serde_json::to_vec(&op).unwrap_or_default();
        if let Err(e) = self.store.write(&file, &bytes) {
            return answer(500, "folder", &e);
        }
        let key = op.key();
        let mine = self
            .refresh()
            .into_iter()
            .find(|(k, _)| *k == key)
            .map(|(_, a)| a);
        let a = mine.unwrap_or_else(|| answer(500, "folder", "The change was not applied"));
        if a.status >= 400 {
            // Refused here, so refused on every copy: nothing to keep.
            self.store.remove(&file);
            self.cache.remove(&file);
            self.applied.retain(|k| *k != key);
        }
        a
    }

    pub fn set_member(&mut self, me: MemberId) {
        self.me = Some(me);
    }

    fn handle(&mut self, method: &str, path: &str, body: &[u8]) -> Answer {
        if NOT_HERE.contains(&path) {
            return answer(400, "folder", "Not with an organization in a shared folder");
        }
        if method == "POST" && !READS.contains(&path) {
            return self.write_op(method, path, body);
        }
        self.refresh();
        let Some(app) = &self.app else {
            return answer(500, "server", "no copy");
        };
        let seed: [u8; 32] = cleis_crypto::random_bytes().unwrap_or([0; 32]);
        cleis_server::local::call(
            app,
            Replay {
                member: self.me,
                device: self.device.clone(),
                at: cleis_core::now(),
                seed,
            },
            method,
            path,
            body.to_vec(),
        )
    }
}

/// The engine as the network client's API.
pub struct Shared(pub Mutex<Engine>);

impl Local for Shared {
    fn send(&self, method: &str, path: &str, body: &[u8]) -> Response {
        let a = match self.0.lock() {
            Ok(mut e) => e.handle(method, path, body),
            Err(_) => answer(500, "server", "busy"),
        };
        Response {
            status: a.status,
            body: a.body,
            cert: [0; 32],
        }
    }
}

static OPEN: Mutex<Vec<(String, Arc<Shared>)>> = Mutex::new(Vec::new());

/// The engine of the organization `key` (opened once per run).
pub fn api_of(key: &str) -> Option<LocalApi> {
    let open = OPEN.lock().ok()?;
    open.iter()
        .find(|(k, _)| k == key)
        .map(|(_, e)| LocalApi(e.clone() as Arc<dyn Local>))
}

pub fn register(key: &str, engine: Engine) -> LocalApi {
    let shared = Arc::new(Shared(Mutex::new(engine)));
    if let Ok(mut open) = OPEN.lock() {
        open.retain(|(k, _)| k != key);
        open.push((key.to_owned(), shared.clone()));
    }
    LocalApi(shared as Arc<dyn Local>)
}

/// The member this computer is, once joined.
pub fn set_member(key: &str, member: MemberId) {
    if let Ok(open) = OPEN.lock()
        && let Some((_, e)) = open.iter().find(|(k, _)| k == key)
        && let Ok(mut e) = e.0.lock()
    {
        e.set_member(member);
    }
}

pub fn forget(key: &str) {
    if let Ok(mut open) = OPEN.lock() {
        open.retain(|(k, _)| k != key);
    }
}

/// The invitation's secret parts (D-78): the code stays with the inviting app.
pub fn new_invite_secret() -> Result<([u8; 10], cleis_core::org::InviteSecret), String> {
    let token: [u8; 10] = cleis_crypto::random_bytes().map_err(|e| e.to_string())?;
    let secret = cleis_core::org::InviteSecret {
        token_hash: sha256(&token),
        proof_key: cleis_server::local::invite_proof(&token).verifying_key().0,
    };
    Ok((token, secret))
}

/// A join request proven by the code (no code in it).
pub fn proven_join(
    code: &str,
    name: &str,
    email: &str,
    device: &str,
    account_public: [u8; 32],
    signing: &SigningKeyPair,
) -> Result<cleis_core::org::JoinRequest, String> {
    let inv = cleis_core::org::Invitation::parse(code).ok_or("Invitation code incomplete")?;
    let mut req = cleis_core::org::JoinRequest {
        token: vec![],
        name: name.trim().into(),
        email: email.trim().into(),
        device: device.into(),
        account_public,
        verify_key: signing.verifying_key().0,
        signature: [0; 64],
        proof: vec![],
    };
    req.signature = signing.sign(cleis_core::org::JOIN_DOMAIN, &req.signed_bytes());
    req.proof = cleis_server::local::invite_proof(&inv.token)
        .sign(cleis_server::local::INVITE_PROOF_DOMAIN, &account_public)
        .to_vec();
    Ok(req)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cleis_core::org::*;
    use cleis_net::{Client, Trust};

    fn client(api: LocalApi) -> Client {
        let mut c = Client::new("folder", Trust::Pinned(vec![]));
        c.local = Some(api);
        c.token = Some("local".into());
        c
    }

    /// Lea creates the organization in a folder, invites Hugo; Hugo's app (its own copy of the
    /// same folder) joins with the code; both copies agree; a change Hugo has no right to make
    /// is refused; Lea removes Hugo, and his copy says so.
    #[test]
    fn two_computers_one_folder() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("SharePoint/Example Studio");
        let lea_sign = SigningKeyPair::generate().unwrap();
        let lea_acc = cleis_crypto::KeyPair::generate().unwrap();
        let (lea_engine, org, lea) = Engine::create(
            &folder,
            &dir.path().join("lea-replica"),
            "Example Studio",
            "Lea Martin",
            lea_acc.public().0,
            SigningKeyPair::from_secret_bytes(*lea_sign.secret_bytes()),
            "PC-LEA",
        )
        .unwrap();
        assert_eq!(Engine::org_file(&folder).unwrap().id, org);
        let lea_c = client(register("lea", lea_engine));
        let st: State = lea_c.get("/v1/state").unwrap();
        assert_eq!(st.me.id, lea);
        assert_eq!(st.me.role, SUPER_ADMIN);

        let (token, secret) = new_invite_secret().unwrap();
        let reply: InviteReply = lea_c
            .call(
                "/v1/invites",
                &NewInvite {
                    name: "Hugo Bernard".into(),
                    email: "hugo@example-studio.com".into(),
                    role: "member".into(),
                    spaces: vec![],
                    address: String::new(),
                    secret: Some(secret),
                },
            )
            .unwrap();
        assert!(reply.code.is_empty(), "the code is never made by a replay");
        let code = cleis_server::code_for(token, &[0; 32]);
        // The operations in the folder hold no code.
        let all: String = walk(&folder.join("ops"));
        assert!(!all.contains(&code));

        let hugo_sign = SigningKeyPair::generate().unwrap();
        let hugo_acc = cleis_crypto::KeyPair::generate().unwrap();
        let hugo_engine = Engine::open(
            &folder,
            &dir.path().join("hugo-replica"),
            None,
            "LAPTOP-HUGO",
            SigningKeyPair::from_secret_bytes(*hugo_sign.secret_bytes()),
        )
        .unwrap();
        let hugo_c = client(register("hugo", hugo_engine));
        let req = proven_join(
            &code,
            "",
            "",
            "LAPTOP-HUGO",
            hugo_acc.public().0,
            &hugo_sign,
        )
        .unwrap();
        let joined: JoinReply = hugo_c.call("/v1/join", &req).unwrap();
        {
            let open = OPEN.lock().unwrap();
            let (_, e) = open.iter().find(|(k, _)| k == "hugo").unwrap();
            e.0.lock().unwrap().set_member(joined.member);
        }
        let hs: State = hugo_c.get("/v1/state").unwrap();
        assert_eq!(hs.me.name, "Hugo Bernard");
        // Lea's copy sees him at its next read.
        let ls: State = lea_c.get("/v1/state").unwrap();
        assert!(ls.members.iter().any(|m| m.id == joined.member));

        // Not his right: refused, and nothing left in the folder.
        let mut p = hs.policy.clone();
        p.min_length = 8;
        let r: Result<Policy, _> = hugo_c.call("/v1/policy", &p);
        assert_eq!(r.unwrap_err().code(), Some("forbidden"));

        // Removed by Lea: his copy refuses him.
        let _: PushReply = lea_c
            .call(
                "/v1/members/revoke",
                &IdOnly {
                    id: joined.member.0,
                },
            )
            .unwrap();
        let r: Result<State, _> = hugo_c.get("/v1/state");
        assert_eq!(r.unwrap_err().code(), Some("revoked"));
        forget("lea");
        forget("hugo");
    }

    fn walk(p: &Path) -> String {
        let mut s = String::new();
        if let Ok(r) = std::fs::read_dir(p) {
            for e in r.flatten() {
                if e.path().is_dir() {
                    s.push_str(&walk(&e.path()));
                } else if let Ok(t) = std::fs::read_to_string(e.path()) {
                    s.push_str(&t);
                }
            }
        }
        s
    }
}
