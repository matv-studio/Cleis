//! Organization model shared by the app and `cleis-server` (D-26, D-32, D-59): roles and what
//! they allow, the security policy, invitation codes and the messages of the server protocol.
//! Pure data, no I/O.
//!
//! The server never holds a key that opens a record: every space key reaches a member wrapped to
//! that member's account key (X25519), and a copy may be wrapped to the organization key (held by
//! the administrators) or to the offline recovery key. The server checks who may write, keeps
//! versions and writes the activity log.

use crate::{MemberId, SpaceId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Byte strings travel as base64 in the JSON messages.
pub mod b64 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&data_encoding::BASE64.encode(v))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        data_encoding::BASE64
            .decode(s.as_bytes())
            .map_err(serde::de::Error::custom)
    }

    pub mod opt {
        use serde::{Deserialize, Deserializer, Serializer};
        pub fn serialize<S: Serializer>(v: &Option<Vec<u8>>, s: S) -> Result<S::Ok, S::Error> {
            match v {
                Some(v) => s.serialize_some(&data_encoding::BASE64.encode(v)),
                None => s.serialize_none(),
            }
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<u8>>, D::Error> {
            Option::<String>::deserialize(d)?
                .map(|s| {
                    data_encoding::BASE64
                        .decode(s.as_bytes())
                        .map_err(serde::de::Error::custom)
                })
                .transpose()
        }
    }
}

/// Fixed-size keys as base64 too.
pub mod b64_32 {
    use serde::{Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
        super::b64::serialize(v, s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        super::b64::deserialize(d)?
            .try_into()
            .map_err(|_| serde::de::Error::custom("expected 32 bytes"))
    }
}

pub mod b64_64 {
    use serde::{Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        super::b64::serialize(v, s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        super::b64::deserialize(d)?
            .try_into()
            .map_err(|_| serde::de::Error::custom("expected 64 bytes"))
    }
}

// ---------- Roles ----------

/// What a role may do in the spaces it can access (screen 07, "Roles").
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Perm {
    /// Reveal passwords in Cleis. Without it the app only copies and fills (checked by the app:
    /// the device holds the space key either way).
    See,
    /// Copy and fill.
    Fill,
    /// Add and edit items (checked by the server).
    Edit,
    /// Move to the trash and erase (checked by the server).
    Delete,
    /// Members, invitations, roles, spaces, policy, encryption.
    Manage,
    /// Activity log and health report of the organization (delegable to Admins, D-67).
    Audit,
    /// Download server backups: the file also holds the server key and the members'
    /// authenticator keys (Super Admin; delegable to Admins, D-67).
    Backups,
    /// Recovery key and server key rotation (Super Admin; delegable to Admins, D-67).
    Keys,
    /// Sign-in rules (verification, methods, remembered devices) and what the activity log
    /// records and keeps (Super Admin; delegable to Admins, D-67).
    Rules,
    /// Who is Admin or Super Admin, what each role may do, backups put back (Super Admin only,
    /// D-60).
    Admins,
}

/// What a Super Admin may grant the Admin role, or take from it (D-67).
pub const DELEGABLE: [Perm; 4] = [Perm::Audit, Perm::Backups, Perm::Keys, Perm::Rules];

impl Perm {
    pub const ALL: [Perm; 10] = [
        Perm::See,
        Perm::Fill,
        Perm::Edit,
        Perm::Delete,
        Perm::Manage,
        Perm::Audit,
        Perm::Backups,
        Perm::Keys,
        Perm::Rules,
        Perm::Admins,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Perm::See => "See passwords",
            Perm::Fill => "Copy, fill",
            Perm::Edit => "Add, edit",
            Perm::Delete => "Delete",
            Perm::Manage => "Manage members and spaces",
            Perm::Audit => "Activity log, health report",
            Perm::Backups => "Server backups",
            Perm::Keys => "Recovery and server keys",
            Perm::Rules => "Sign-in and log rules",
            Perm::Admins => "Admins, roles",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    /// `admin`, `member`, `read_only`, `intern`, or a UUID for a role made by the organization.
    pub id: String,
    pub name: String,
    pub perms: BTreeSet<Perm>,
}

pub const ADMIN: &str = "admin";
/// Given to the organization's creator; can be transferred to another member or shared (D-60).
pub const SUPER_ADMIN: &str = "super_admin";

/// Admin or Super Admin: holds the organization key, can always export.
pub fn is_admin_role(id: &str) -> bool {
    id == ADMIN || id == SUPER_ADMIN
}

impl Role {
    pub fn can(&self, p: Perm) -> bool {
        self.perms.contains(&p)
    }

    pub fn is_admin(&self) -> bool {
        is_admin_role(&self.id)
    }

    pub fn is_super(&self) -> bool {
        self.id == SUPER_ADMIN
    }

    /// The roles of a new organization (screen 07). Super Admin and Admin cannot be changed or
    /// removed.
    pub fn defaults() -> Vec<Role> {
        let r = |id: &str, name: &str, perms: &[Perm]| Role {
            id: id.into(),
            name: name.into(),
            perms: perms.iter().copied().collect(),
        };
        // D-67: the Admin reads the log; backups, keys and rules stay with the Super Admin until
        // one delegates them.
        let admin = [
            Perm::See,
            Perm::Fill,
            Perm::Edit,
            Perm::Delete,
            Perm::Manage,
            Perm::Audit,
        ];
        vec![
            r(SUPER_ADMIN, "Super Admin", &Perm::ALL),
            r(ADMIN, "Admin", &admin),
            r(
                "member",
                "Member",
                &[Perm::See, Perm::Fill, Perm::Edit, Perm::Delete],
            ),
            r("read_only", "Read only", &[Perm::See, Perm::Fill]),
            r("intern", "Intern", &[Perm::Fill]),
        ]
    }
}

/// Checks a role list sent by a Super Admin: Super Admin first as it is, then Admin with every
/// right but `Admins` save the delegable ones (D-67); names set and unique; other roles hold
/// neither `Admins` nor the delegable rights.
pub fn valid_roles(roles: &[Role]) -> bool {
    let d = Role::defaults();
    let mut names = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let admin_ok = |r: &Role| {
        r.id == d[1].id
            && r.name == d[1].name
            && !r.can(Perm::Admins)
            && Perm::ALL
                .into_iter()
                .filter(|p| *p != Perm::Admins && !DELEGABLE.contains(p))
                .all(|p| r.can(p))
    };
    roles.len() >= 2
        && roles[0] == d[0]
        && admin_ok(&roles[1])
        && roles.iter().all(|r| {
            !r.name.trim().is_empty()
                && names.insert(crate::text::fold(r.name.trim()))
                && ids.insert(r.id.clone())
        })
        && roles.iter().skip(2).all(|r| {
            !is_admin_role(&r.id) && !r.can(Perm::Admins) && !DELEGABLE.iter().any(|p| r.can(*p))
        })
}

/// Roles as kept by an older server, brought to this version: the Super Admin holds every right.
pub fn upgrade_roles(mut roles: Vec<Role>) -> Vec<Role> {
    if let Some(s) = roles.iter_mut().find(|r| r.id == SUPER_ADMIN) {
        s.perms = Perm::ALL.into_iter().collect();
    }
    roles
}

// ---------- Security policy ----------

/// How the members' Cleis may open on their computers (D-76, mockup 35k).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnlockRule {
    /// The master password every time.
    PasswordOnly,
    /// The master password, or Windows Hello on the member's computer.
    PasswordOrHello,
    /// Also nothing to type on the member's computer.
    #[default]
    Any,
}

impl UnlockRule {
    /// Whether Windows Hello may open the vault.
    pub fn allows_hello(self) -> bool {
        self != UnlockRule::PasswordOnly
    }

    /// Whether nothing to type may open the vault.
    pub fn allows_nothing(self) -> bool {
        self == UnlockRule::Any
    }

    /// Whether the master password may be removed (the recovery key then opens elsewhere).
    pub fn allows_no_password(self) -> bool {
        self != UnlockRule::PasswordOnly
    }

    /// The stricter of two rules (a vault in several organizations follows all of them).
    pub fn stricter(self, o: UnlockRule) -> UnlockRule {
        use UnlockRule::*;
        match (self, o) {
            (PasswordOnly, _) | (_, PasswordOnly) => PasswordOnly,
            (PasswordOrHello, _) | (_, PasswordOrHello) => PasswordOrHello,
            _ => Any,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogDetail {
    /// Members, roles, spaces, policy, exports.
    AdminActions,
    /// Also creations, edits and deletions.
    Changes,
    /// Also views, copies and fills.
    Everything,
}

/// Second sign-in step an organization may allow (screen 08). E-mail and SMS need the server's
/// `[smtp]` / `[sms]`, Google and Microsoft an `[[oidc]]` provider (D-64); the server says which it
/// can carry out (`Hello::methods`). Security keys are left out (D-60).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    AuthenticatorApp,
    Email,
    Sms,
    Google,
    Microsoft,
}

impl Method {
    pub const ALL: [Method; 5] = [
        Method::AuthenticatorApp,
        Method::Email,
        Method::Sms,
        Method::Google,
        Method::Microsoft,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Method::AuthenticatorApp => "Authenticator app",
            Method::Email => "Email",
            Method::Sms => "SMS",
            Method::Google => "Google account",
            Method::Microsoft => "Microsoft account",
        }
    }

    /// Signs in through an identity provider (browser) rather than with a typed code.
    pub fn is_identity(self) -> bool {
        matches!(self, Method::Google | Method::Microsoft)
    }
}

/// An identity provider the apps sign in with (OpenID Connect, authorization code with PKCE and a
/// loopback redirect, RFC 8252).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provider {
    pub method: Method,
    pub issuer: String,
    pub client_id: String,
    /// Google desktop clients only (not a secret, see the server configuration).
    #[serde(default)]
    pub client_secret: String,
}

/// Starts setting up the second step: the method, and the phone number (SMS) or the ID token
/// (Google, Microsoft). E-mail uses the member's address.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodeStart {
    pub method: Method,
    #[serde(default)]
    pub target: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Policy {
    /// A verification code is required to sign in to the server.
    pub code_required: bool,
    /// Days a device is remembered after a code (0: ask every time).
    pub remember_days: u32,
    pub methods: BTreeSet<Method>,
    /// Lock after this many minutes without activity (None: never). Members cannot set longer.
    pub lock_after_mins: Option<u32>,
    /// How long the organization's spaces stay readable without reaching the server.
    pub offline_days: u32,
    /// Health report: flag passwords shorter than this.
    pub min_length: u32,
    /// Health report: flag passwords older than this.
    pub max_age_days: u32,
    pub record_activity: bool,
    pub detail: LogDetail,
    pub keep_days: u32,
    pub fill_fields: bool,
    pub type_keys: bool,
    pub read_addresses: bool,
    /// How Cleis may open on the members' computers (D-76).
    pub unlock: UnlockRule,
    /// The members' master password: at least this many characters (0: Cleis' own minimum).
    pub master_min_length: u32,
    /// The organization's PIN, asked before its spaces open (D-79).
    pub pin: PinRule,
}

/// The organization's PIN (D-79, mockups 36g/36h): one PIN set by an admin, checked by the
/// server before it opens a session; the organization's spaces stay closed in the app until it
/// is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PinRule {
    pub on: bool,
    /// Digits: 4, 6 or 8.
    pub length: u8,
    /// Asked again after this many hours on a device.
    pub again_hours: u32,
    /// Wrong PINs in a row before the member is locked out (until an admin unlocks).
    pub tries: u32,
}

impl Default for PinRule {
    fn default() -> Self {
        Self {
            on: false,
            length: 6,
            again_hours: 8,
            tries: 5,
        }
    }
}

impl PinRule {
    /// A PIN of the policy's length, digits only.
    pub fn accepts(&self, pin: &str) -> bool {
        pin.len() == usize::from(self.length) && pin.bytes().all(|b| b.is_ascii_digit())
    }
}

/// Sets the organization's PIN (`/v1/policy/pin`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PinSet {
    pub pin: String,
}

impl Policy {
    /// Same sign-in and log rules (what `Perm::Rules` guards, D-67).
    pub fn same_rules(&self, o: &Policy) -> bool {
        self.code_required == o.code_required
            && self.remember_days == o.remember_days
            && self.methods == o.methods
            && self.record_activity == o.record_activity
            && self.detail == o.detail
            && self.keep_days == o.keep_days
            && self.unlock == o.unlock
            && self.master_min_length == o.master_min_length
            && self.pin == o.pin
    }
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            code_required: false,
            remember_days: 30,
            methods: [Method::AuthenticatorApp].into_iter().collect(),
            lock_after_mins: None,
            offline_days: 14,
            min_length: 12,
            max_age_days: 365,
            record_activity: true,
            detail: LogDetail::Changes,
            keep_days: 365,
            fill_fields: true,
            type_keys: true,
            read_addresses: true,
            unlock: UnlockRule::Any,
            master_min_length: 0,
            pin: PinRule::default(),
        }
    }
}

impl Policy {
    /// Whether an event of this kind is written, given the chosen detail.
    pub fn records(&self, action: Action) -> bool {
        self.record_activity
            && match action.level() {
                LogDetail::AdminActions => true,
                LogDetail::Changes => self.detail != LogDetail::AdminActions,
                LogDetail::Everything => self.detail == LogDetail::Everything,
            }
    }
}

// ---------- Activity log ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    SignedIn,
    Viewed,
    Copied,
    Filled,
    Created,
    Edited,
    Deleted,
    Restored,
    Erased,
    Exported,
    Invited,
    Joined,
    MemberChanged,
    Revoked,
    RolesChanged,
    PolicyChanged,
    SpaceCreated,
    SpaceReencrypted,
    KeyRotated,
    BackedUp,
    SpaceRenamed,
    SpaceDeleted,
    SignedOut,
    SpaceReserved,
}

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Action::SignedIn => "Signed in",
            Action::Viewed => "Viewed",
            Action::Copied => "Copied",
            Action::Filled => "Filled",
            Action::Created => "Created",
            Action::Edited => "Edited",
            Action::Deleted => "Deleted",
            Action::Restored => "Restored",
            Action::Erased => "Erased",
            Action::Exported => "Exported",
            Action::Invited => "Invited",
            Action::Joined => "Joined",
            Action::MemberChanged => "Member changed",
            Action::Revoked => "Revoked",
            Action::RolesChanged => "Roles changed",
            Action::PolicyChanged => "Policy changed",
            Action::SpaceCreated => "Space created",
            Action::SpaceReencrypted => "Re-encrypted",
            Action::KeyRotated => "Key rotated",
            Action::BackedUp => "Backed up",
            Action::SpaceRenamed => "Space renamed",
            Action::SpaceDeleted => "Space deleted",
            Action::SignedOut => "Signed out",
            Action::SpaceReserved => "Space reserved",
        }
    }

    pub fn level(self) -> LogDetail {
        match self {
            Action::Viewed | Action::Copied | Action::Filled => LogDetail::Everything,
            Action::Created
            | Action::Edited
            | Action::Deleted
            | Action::Restored
            | Action::Erased
            | Action::SignedIn => LogDetail::Changes,
            _ => LogDetail::AdminActions,
        }
    }

    /// Events a member's app may report (the server writes the others itself).
    pub fn from_client(self) -> bool {
        matches!(
            self,
            Action::Viewed | Action::Copied | Action::Filled | Action::Exported
        )
    }
}

/// One line of the activity log as the server keeps it. `detail` (item name, field) is sealed
/// to the organization key by the app that reported it: only administrators read it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub seq: u64,
    pub at: i64,
    pub member: Option<MemberId>,
    pub device: String,
    pub action: Action,
    pub space: Option<SpaceId>,
    /// `kind/id` of the item concerned, if any.
    pub item: Option<String>,
    #[serde(with = "b64::opt", default)]
    pub detail: Option<Vec<u8>>,
    /// Words the server writes itself (a member's name, a role…): never secret.
    #[serde(default)]
    pub note: String,
}

/// Item name and field, sealed in `Event::detail`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventDetail {
    pub item: String,
    #[serde(default)]
    pub field: String,
}

pub const DETAIL_CONTEXT: &[u8] = b"cleis/log-detail/v1";
/// Context of the Super Admin key wrapped to each Super Admin.
pub const SUPER_KEY_CONTEXT: &[u8] = b"cleis/super-key";

// ---------- Invitation codes ----------

/// What an invitation code carries (D-59): a one-time token and the first 10 bytes of the
/// SHA-256 of the server's certificate, so the app trusts that server and no other from the
/// first connection. 20 bytes, typed as 8 groups of 4 base32 characters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invitation {
    pub token: [u8; 10],
    pub pin: [u8; 10],
}

const CODE_ALPHABET: data_encoding::Encoding = data_encoding::BASE32_NOPAD;

impl Invitation {
    pub fn code(&self) -> String {
        let mut b = Vec::with_capacity(20);
        b.extend_from_slice(&self.token);
        b.extend_from_slice(&self.pin);
        let s = CODE_ALPHABET.encode(&b);
        s.as_bytes()
            .chunks(4)
            .map(|c| String::from_utf8_lossy(c).into_owned())
            .collect::<Vec<_>>()
            .join("-")
    }

    /// Reads a typed code: case, spaces, dashes and the look-alikes 0/O, 1/I, 8/B are forgiven.
    pub fn parse(code: &str) -> Option<Self> {
        let s: String = code
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| match c.to_ascii_uppercase() {
                '0' => 'O',
                '1' => 'I',
                '8' => 'B',
                c => c,
            })
            .collect();
        let b = CODE_ALPHABET.decode(s.as_bytes()).ok()?;
        if b.len() != 20 {
            return None;
        }
        Some(Self {
            token: b[..10].try_into().ok()?,
            pin: b[10..].try_into().ok()?,
        })
    }

    /// Whether a certificate hash matches the pin of this code.
    pub fn matches(&self, cert_sha256: &[u8; 32]) -> bool {
        cert_sha256[..10] == self.pin
    }
}

/// `host:port` from what the person typed (`https://vault.example-studio.lan`,
/// `vault.example-studio.lan:8443`, `10.8.0.1`). Default port 8443.
pub fn server_address(input: &str) -> Option<String> {
    let s = input.trim();
    let s = s.strip_prefix("https://").unwrap_or(s);
    let s = s.split('/').next()?.trim();
    if s.is_empty() || s.contains(char::is_whitespace) || s.starts_with('\\') {
        return None;
    }
    let has_port = s.rsplit_once(':').is_some_and(|(h, p)| {
        !h.is_empty() && !h.ends_with(':') && p.parse::<u16>().is_ok_and(|p| p > 0)
    });
    if has_port && !s.starts_with('[') || s.starts_with('[') && s.contains("]:") {
        Some(s.to_owned())
    } else if s.contains(':') && !s.starts_with('[') {
        // Bare IPv6 address.
        Some(format!("[{s}]:8443"))
    } else {
        Some(format!("{s}:8443"))
    }
}

/// What the single address field of the first launch holds (D-25).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaceKind {
    Server(String),
    Folder(String),
    Sftp(String),
    GitHub(String),
}

pub fn detect_place(input: &str) -> Option<PlaceKind> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    let lower = s.to_ascii_lowercase();
    if lower.starts_with("sftp://") {
        return Some(PlaceKind::Sftp(s.to_owned()));
    }
    if lower.starts_with("github.com/") || lower.starts_with("https://github.com/") {
        let repo = s.split("github.com/").nth(1)?.trim_end_matches(".git");
        return Some(PlaceKind::GitHub(repo.trim_end_matches('/').to_owned()));
    }
    if s.starts_with("\\\\") || s.starts_with('/') || s.chars().nth(1) == Some(':') {
        return Some(PlaceKind::Folder(s.to_owned()));
    }
    server_address(s).map(PlaceKind::Server)
}

// ---------- Protocol ----------

/// Domain of the sign-in signature.
pub const SIGN_IN_DOMAIN: &str = "cleis/sign-in/v1";
/// Domain of the signature that proves a joining member holds their account key.
pub const JOIN_DOMAIN: &str = "cleis/join/v1";

/// Bytes a member signs to sign in: the server's challenge and the certificate it presented, so a
/// signature made for one server cannot be replayed to another.
pub fn sign_in_message(nonce: &[u8], cert_sha256: &[u8; 32]) -> Vec<u8> {
    [nonce, cert_sha256].concat()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JoinRequest {
    #[serde(with = "b64")]
    pub token: Vec<u8>,
    pub name: String,
    pub email: String,
    pub device: String,
    #[serde(with = "b64_32")]
    pub account_public: [u8; 32],
    #[serde(with = "b64_32")]
    pub verify_key: [u8; 32],
    /// Signature of `token | account_public` with the account's Ed25519 key.
    #[serde(with = "b64_64")]
    pub signature: [u8; 64],
}

impl JoinRequest {
    pub fn signed_bytes(&self) -> Vec<u8> {
        [&self.token[..], &self.account_public[..]].concat()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JoinReply {
    pub member: MemberId,
    pub org: OrgInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Challenge {
    #[serde(with = "b64")]
    pub nonce: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionRequest {
    pub member: MemberId,
    #[serde(with = "b64")]
    pub nonce: Vec<u8>,
    #[serde(with = "b64_64")]
    pub signature: [u8; 64],
    pub device: String,
    /// Verification code, when the policy asks for one.
    #[serde(default)]
    pub code: Option<String>,
    /// Token given after an earlier code, while the device is remembered.
    #[serde(default)]
    pub remembered: Option<String>,
    /// ID token from the member's identity provider (Google, Microsoft).
    #[serde(default)]
    pub id_token: Option<String>,
    /// The organization's PIN, when its policy asks for one (D-79).
    #[serde(default)]
    pub pin: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionReply {
    pub token: String,
    #[serde(default)]
    pub remembered: Option<String>,
    /// The policy asks for a verification code and this member has not set one up yet: only
    /// the set-up requests are accepted until then.
    #[serde(default)]
    pub setup_code: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrgInfo {
    pub id: uuid::Uuid,
    pub name: String,
    /// SHA-256 of the certificate the server presents now.
    #[serde(with = "b64_32")]
    pub cert: [u8; 32],
    /// Certificate announced by a key rotation: apps accept it too until the switch.
    #[serde(with = "b64::opt", default)]
    pub next_cert: Option<Vec<u8>>,
    /// Organization key (X25519 public): space keys wrapped to it can be opened by any admin.
    #[serde(with = "b64::opt", default)]
    pub org_public: Option<Vec<u8>>,
    /// Offline recovery key (X25519 public), if the admins turned recovery on.
    #[serde(with = "b64::opt", default)]
    pub recovery_public: Option<Vec<u8>>,
    #[serde(default)]
    pub recovery_saved: Option<i64>,
    /// Super Admin key (X25519 public): activity details of reserved spaces are sealed to it
    /// (D-61).
    #[serde(with = "b64::opt", default)]
    pub super_public: Option<Vec<u8>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpaceMode {
    /// A copy of the key is wrapped to the organization key: admins can restore access.
    Standard,
    /// Only the members (and the offline recovery key, if on) hold the key.
    EndToEnd,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberView {
    pub id: MemberId,
    pub name: String,
    pub email: String,
    pub role: String,
    pub spaces: Vec<SpaceId>,
    pub two_fa: bool,
    pub export: bool,
    pub last_sign_in: Option<i64>,
    pub last_device: String,
    pub revoked: bool,
    #[serde(with = "b64_32")]
    pub account_public: [u8; 32],
    #[serde(with = "b64_32")]
    pub verify_key: [u8; 32],
    /// Second sign-in step set up (D-64).
    #[serde(default)]
    pub factor: Option<Method>,
    /// Locked out by wrong PINs, until an admin unlocks (D-79).
    #[serde(default)]
    pub pin_locked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InviteView {
    pub id: uuid::Uuid,
    pub name: String,
    pub email: String,
    pub role: String,
    pub spaces: Vec<SpaceId>,
    pub created: i64,
    pub expires: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceView {
    pub id: SpaceId,
    /// Space name and settings, sealed with the space key (as in the vault file).
    #[serde(with = "b64")]
    pub sealed: Vec<u8>,
    pub mode: SpaceMode,
    /// Raised by each re-encryption.
    pub key_gen: u32,
    /// The space key wrapped to this member, once someone has wrapped it.
    #[serde(with = "b64::opt", default)]
    pub wrapped: Option<Vec<u8>>,
    /// Admins: the copy wrapped to the organization key (Standard spaces).
    #[serde(with = "b64::opt", default)]
    pub org_wrapped: Option<Vec<u8>>,
    /// Admins: the copy wrapped to the recovery key.
    #[serde(with = "b64::opt", default)]
    pub recovery_wrapped: Option<Vec<u8>>,
    pub members: Vec<MemberId>,
    pub items: u32,
    /// Someone who held the key lost access since the last re-encryption.
    #[serde(default)]
    pub needs_rekey: bool,
    /// Reserved (D-61): end-to-end, its members changed by a Super Admin only, hidden from the
    /// other admins.
    #[serde(default)]
    pub reserved: bool,
}

/// A member who was given a space but has no wrapped key for it yet: any app that holds the key
/// (a member of the space, or an admin through the organization key) wraps it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pending {
    pub space: SpaceId,
    pub member: MemberId,
    #[serde(with = "b64_32")]
    pub account_public: [u8; 32],
    /// Who gave the space: an app hands a reserved space's key on only when a Super Admin did.
    #[serde(default)]
    pub granted_by: Option<MemberId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub org: OrgInfo,
    pub me: MemberView,
    pub roles: Vec<Role>,
    pub policy: Policy,
    /// Everyone (names and public keys are needed to share spaces); admins see the details.
    pub members: Vec<MemberView>,
    pub invites: Vec<InviteView>,
    pub spaces: Vec<SpaceView>,
    pub pending: Vec<Pending>,
    /// Admins: the organization private key wrapped to this admin.
    #[serde(with = "b64::opt", default)]
    pub org_key: Option<Vec<u8>>,
    /// Admins promoted since: they wait for an admin app to wrap the organization key for them.
    pub pending_admins: Vec<Pending>,
    /// Super Admins: the Super Admin private key wrapped to this one (D-61).
    #[serde(with = "b64::opt", default)]
    pub super_key: Option<Vec<u8>>,
    /// Super Admins named since: they wait for the Super Admin key.
    #[serde(default)]
    pub pending_supers: Vec<Pending>,
    /// Highest change number on the server.
    pub seq: u64,
    /// Auditors: the last check of the activity log.
    #[serde(default)]
    pub integrity: Option<Integrity>,
    /// Auditors: events kept.
    #[serde(default)]
    pub events: u64,
    /// Second steps the server can carry out (D-64).
    #[serde(default)]
    pub methods: Vec<Method>,
    #[serde(default)]
    pub providers: Vec<Provider>,
    /// The organization's PIN has been set (D-79; the PIN itself never leaves the server).
    #[serde(default)]
    pub pin_set: bool,
}

/// A record as stored on the server: the same sealed row as in the vault file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    pub kind: String,
    pub id: String,
    pub space: SpaceId,
    #[serde(with = "b64")]
    pub sealed: Vec<u8>,
    pub rev: i64,
    pub updated: i64,
    pub deleted: Option<i64>,
    /// Milliseconds: the later change wins.
    pub changed: i64,
    #[serde(default)]
    pub key_gen: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pull {
    pub since: u64,
    /// Spaces to send whole, whatever their change numbers (a space just shared with this
    /// member).
    #[serde(default)]
    pub full: Vec<SpaceId>,
}

/// What an invitation code is for, asked before joining (first launch shows it).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InviteLookup {
    #[serde(with = "b64")]
    pub token: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InviteInfo {
    pub org: String,
    pub name: String,
    pub email: String,
}

/// Unauthenticated greeting: which organization answers at this address.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hello {
    pub name: String,
    pub version: String,
    /// Methods a member may set up: carried out by this server and allowed by the policy.
    #[serde(default)]
    pub methods: Vec<Method>,
    #[serde(default)]
    pub providers: Vec<Provider>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Changes {
    pub rows: Vec<Row>,
    /// `(kind, id)` erased for good.
    pub purged: Vec<(String, String)>,
    pub seq: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PushReply {
    pub seq: u64,
    /// `(kind, id)` the server refused (no right, older than its copy, old key).
    pub refused: Vec<(String, String)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewSpace {
    pub id: SpaceId,
    #[serde(with = "b64")]
    pub sealed: Vec<u8>,
    pub mode: SpaceMode,
    #[serde(default)]
    pub reserved: bool,
    #[serde(with = "b64")]
    pub wrapped: Vec<u8>,
    #[serde(with = "b64::opt", default)]
    pub org_wrapped: Option<Vec<u8>>,
    #[serde(with = "b64::opt", default)]
    pub recovery_wrapped: Option<Vec<u8>>,
}

/// A whole space under a new key (screen 10): every row re-sealed, the key wrapped again for
/// each member.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rekey {
    pub space: SpaceId,
    pub key_gen: u32,
    #[serde(with = "b64")]
    pub sealed: Vec<u8>,
    pub mode: SpaceMode,
    pub wraps: Vec<Wrap>,
    #[serde(with = "b64::opt", default)]
    pub org_wrapped: Option<Vec<u8>>,
    #[serde(with = "b64::opt", default)]
    pub recovery_wrapped: Option<Vec<u8>>,
    pub rows: Vec<Row>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wrap {
    pub space: SpaceId,
    pub member: MemberId,
    #[serde(with = "b64")]
    pub wrapped: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Grants {
    pub wraps: Vec<Wrap>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdminKeys {
    /// The organization private key wrapped to each admin (space field unused).
    pub wraps: Vec<Wrap>,
    /// Set once, by the first admin.
    #[serde(with = "b64::opt", default)]
    pub org_public: Option<Vec<u8>>,
}

/// The Super Admin key: set once by the first Super Admin, then wrapped for each new one.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SuperKeys {
    pub wraps: Vec<Wrap>,
    #[serde(with = "b64::opt", default)]
    pub super_public: Option<Vec<u8>>,
}

/// Reserves a space or frees it (Super Admin).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reserve {
    pub id: SpaceId,
    pub reserved: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewInvite {
    pub name: String,
    pub email: String,
    pub role: String,
    pub spaces: Vec<SpaceId>,
    /// The server's address as members reach it: put in the invitation e-mail (audit L-17).
    #[serde(default)]
    pub address: String,
}

/// A new code for an invitation (Resend).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RenewInvite {
    pub id: uuid::Uuid,
    #[serde(default)]
    pub address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InviteReply {
    pub id: uuid::Uuid,
    pub code: String,
    /// Sent by e-mail by the server (its `[smtp]` is set up).
    #[serde(default)]
    pub emailed: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MemberChange {
    pub member: MemberId,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub spaces: Option<Vec<SpaceId>>,
    #[serde(default)]
    pub export: Option<bool>,
}

/// A member's own name and e-mail (Settings › Profile).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileChange {
    pub name: String,
    pub email: String,
}

/// A computer where this member is signed in or remembered (Settings › Devices).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceView {
    pub device: String,
    pub last_active: i64,
    /// The verification code is not asked until then.
    #[serde(default)]
    pub remembered_until: Option<i64>,
    /// The device making the request.
    pub current: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceRef {
    pub device: String,
}

/// New sealed name of a space (rename).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpaceUpdate {
    pub id: SpaceId,
    #[serde(with = "b64")]
    pub sealed: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdOnly {
    pub id: uuid::Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub action: Action,
    #[serde(default)]
    pub space: Option<SpaceId>,
    #[serde(default)]
    pub item: Option<String>,
    #[serde(with = "b64::opt", default)]
    pub detail: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LogQuery {
    #[serde(default)]
    pub member: Option<MemberId>,
    #[serde(default)]
    pub action: Option<Action>,
    #[serde(default)]
    pub space: Option<SpaceId>,
    /// Unix seconds.
    #[serde(default)]
    pub after: Option<i64>,
    /// Events before this number (paging, newest first).
    #[serde(default)]
    pub before: Option<u64>,
    pub limit: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LogPage {
    pub events: Vec<Event>,
    pub total: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Integrity {
    pub checked: u64,
    pub intact: bool,
    /// First problem found, in words.
    #[serde(default)]
    pub problem: Option<String>,
    pub at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodeSetup {
    /// Base32 key for the authenticator app.
    pub secret: String,
    /// E-mail, SMS: where the code went (masked).
    #[serde(default)]
    pub sent_to: String,
    /// Google, Microsoft: set up already (nothing to confirm).
    #[serde(default)]
    pub done: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodeCheck {
    pub code: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Versions {
    pub kind: String,
    pub id: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct VersionList {
    pub rows: Vec<Row>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recovery {
    #[serde(with = "b64")]
    pub public: Vec<u8>,
    /// Space keys wrapped to the new recovery key.
    pub wraps: Vec<Wrap>,
}

/// Error body of a refused request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiError {
    /// Stable code the app acts on: `code_required`, `code_sent`, `identity_google`,
    /// `identity_microsoft`, `wrong_code`, `setup_code`, `forbidden`, `revoked`, `expired`,
    /// `not_found`, `bad_request`, `unauthorized`.
    pub error: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invitation_code_round_trip() {
        let inv = Invitation {
            token: [7; 10],
            pin: [0xab; 10],
        };
        let code = inv.code();
        assert_eq!(code.len(), 32 + 7);
        assert_eq!(Invitation::parse(&code), Some(inv.clone()));
        // Typed loosely: lower case, spaces, look-alikes.
        let loose = code.to_lowercase().replace('-', " ").replace('o', "0");
        assert_eq!(Invitation::parse(&loose), Some(inv.clone()));
        assert_eq!(Invitation::parse("K7Q2-9MXA"), None);
        let mut cert = [0u8; 32];
        cert[..10].copy_from_slice(&[0xab; 10]);
        assert!(inv.matches(&cert));
        cert[3] = 0;
        assert!(!inv.matches(&cert));
    }

    #[test]
    fn addresses_and_places() {
        assert_eq!(
            server_address("https://vault.example-studio.lan/").as_deref(),
            Some("vault.example-studio.lan:8443")
        );
        assert_eq!(
            server_address("10.8.0.1:9000").as_deref(),
            Some("10.8.0.1:9000")
        );
        assert_eq!(server_address("fd00::1").as_deref(), Some("[fd00::1]:8443"));
        assert_eq!(
            server_address("[fd00::1]:9000").as_deref(),
            Some("[fd00::1]:9000")
        );
        assert_eq!(server_address(""), None);
        assert_eq!(
            detect_place("\\\\nas\\cleis"),
            Some(PlaceKind::Folder("\\\\nas\\cleis".into()))
        );
        assert_eq!(
            detect_place("sftp://nas.example-studio.lan/cleis"),
            Some(PlaceKind::Sftp(
                "sftp://nas.example-studio.lan/cleis".into()
            ))
        );
        assert_eq!(
            detect_place("https://github.com/example-studio/vault.git"),
            Some(PlaceKind::GitHub("example-studio/vault".into()))
        );
        assert_eq!(
            detect_place("vault.example-studio.lan"),
            Some(PlaceKind::Server("vault.example-studio.lan:8443".into()))
        );
    }

    #[test]
    fn roles_and_policy() {
        let roles = Role::defaults();
        assert!(valid_roles(&roles));
        assert!(roles[0].is_super() && roles[0].can(Perm::Admins));
        assert!(roles[1].can(Perm::Manage) && !roles[1].can(Perm::Admins));
        assert!(!roles[4].can(Perm::See) && roles[4].can(Perm::Fill));
        // D-67: a Super Admin delegates or takes back the log, backups, keys and rules.
        let mut delegated = roles.clone();
        delegated[1].perms.remove(&Perm::Audit);
        delegated[1].perms.insert(Perm::Backups);
        assert!(valid_roles(&delegated));
        let mut bad = roles.clone();
        bad[1].perms.remove(&Perm::Manage);
        assert!(!valid_roles(&bad));
        let mut bad = roles.clone();
        bad[1].perms.insert(Perm::Admins);
        assert!(!valid_roles(&bad));
        let mut bad = roles.clone();
        bad[2].perms.insert(Perm::Backups);
        assert!(!valid_roles(&bad));
        let mut bad = roles.clone();
        bad[0].perms.remove(&Perm::Keys);
        assert!(!valid_roles(&bad));
        let mut dup = roles.clone();
        dup[3].name = "member".into();
        assert!(!valid_roles(&dup));
        let mut sneaky = roles.clone();
        sneaky[2].perms.insert(Perm::Admins);
        assert!(!valid_roles(&sneaky));

        let mut p = Policy::default();
        assert!(p.records(Action::Edited) && !p.records(Action::Copied));
        p.detail = LogDetail::AdminActions;
        assert!(p.records(Action::Revoked) && !p.records(Action::Edited));
        p.record_activity = false;
        assert!(!p.records(Action::Revoked));
    }
}
