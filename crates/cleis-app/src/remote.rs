//! Sync places reached over the network (D-65): a GitHub repository (contents API, personal
//! access token: no app to register) and an SFTP server. The vault file is downloaded to a local
//! copy, merged there like any other place (rows stay sealed), then sent back when it changed.

use data_encoding::BASE64;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RemotePlace {
    GitHub {
        /// `owner/name`.
        repo: String,
        /// Path of the file in the repository.
        path: String,
        branch: String,
        /// `https://api.github.com`, or a GitHub Enterprise server's `https://host/api/v3`.
        #[serde(default = "github_api")]
        api: String,
    },
    Sftp {
        host: String,
        port: u16,
        user: String,
        path: String,
        /// `SHA256:…` of the server's key, kept at the first connection.
        #[serde(default)]
        host_key: String,
    },
}

fn github_api() -> String {
    "https://api.github.com".into()
}

impl RemotePlace {
    /// Shown in Sync & backups.
    pub fn label(&self) -> String {
        match self {
            RemotePlace::GitHub { repo, path, .. } => format!("{repo} · {path}"),
            RemotePlace::Sftp {
                host, user, path, ..
            } => format!("{user}@{host}:{path}"),
        }
    }
}

/// The sign-in secret of a place, kept sealed in the vault.
pub const SECRET_SETTING: &str = "place_secret";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Secret {
    /// GitHub token, or SFTP password.
    #[serde(default)]
    pub token: String,
    /// SFTP: OpenSSH private key (and its passphrase in `token`).
    #[serde(default)]
    pub key: String,
    /// GitHub App sign-in (D-101): the token lasts 8 hours; this one (6 months) gets the next.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub refresh: String,
    /// When `token` ends (Unix seconds); 0: it does not.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub expires: i64,
}

fn is_zero(v: &i64) -> bool {
    *v == 0
}

impl Secret {
    /// A GitHub token about to end (5 minutes left) that can be renewed.
    pub fn renewable(&self) -> bool {
        self.expires > 0 && !self.refresh.is_empty() && cleis_core::now() >= self.expires - 300
    }
}

/// D-101: a GitHub place's token renewed when it is about to end (device flow: no client secret
/// needed). `None`: nothing to renew, or not a GitHub place.
pub fn renewed(place: &RemotePlace, secret: &Secret) -> Option<Result<Secret, String>> {
    let RemotePlace::GitHub { api, .. } = place else {
        return None;
    };
    if !secret.renewable() {
        return None;
    }
    let hosts = crate::github_login::Hosts::of_api(api);
    Some(
        crate::github_login::refresh(&hosts, &secret.refresh).map(|g| {
            let expires = g.expires_at();
            Secret {
                token: g.token,
                key: String::new(),
                refresh: g.refresh,
                expires,
            }
        }),
    )
}

#[derive(Debug, PartialEq, Eq)]
pub enum RemoteError {
    /// The file changed since it was read: read, merge and send again.
    Conflict,
    /// SFTP: the server presented another key than the one kept.
    HostKey(String),
    Other(String),
}

impl std::fmt::Display for RemoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemoteError::Conflict => f.write_str("the file changed meanwhile"),
            RemoteError::HostKey(k) => write!(f, "the server's key changed ({k})"),
            RemoteError::Other(e) => f.write_str(e),
        }
    }
}

impl From<String> for RemoteError {
    fn from(e: String) -> Self {
        RemoteError::Other(e)
    }
}

/// Version of the file read (GitHub: its blob sha), passed back when sending.
pub type Version = Option<String>;

/// Reads the place's file into `to` (removed when the place has none yet).
pub fn download(place: &RemotePlace, secret: &Secret, to: &Path) -> Result<Version, RemoteError> {
    match place {
        RemotePlace::GitHub {
            repo,
            path,
            branch,
            api,
        } => {
            let gh = GitHub::new(api, repo, path, branch, &secret.token)?;
            match gh.read()? {
                Some((sha, bytes)) => {
                    write_atomic(to, &bytes)?;
                    Ok(Some(sha))
                }
                None => {
                    let _ = std::fs::remove_file(to);
                    Ok(None)
                }
            }
        }
        RemotePlace::Sftp { .. } => crate::sftp::download(place, secret, to),
    }
}

/// Sends `from` to the place; `version`: what `download` returned.
pub fn upload(
    place: &RemotePlace,
    secret: &Secret,
    from: &Path,
    version: Version,
) -> Result<(), RemoteError> {
    let bytes = std::fs::read(from).map_err(|e| e.to_string())?;
    match place {
        RemotePlace::GitHub {
            repo,
            path,
            branch,
            api,
        } => GitHub::new(api, repo, path, branch, &secret.token)?.write(&bytes, version),
        RemotePlace::Sftp { .. } => crate::sftp::upload(place, secret, &bytes),
    }
}

/// The account key (D-96): a vault without a password opens on a new computer with the key kept
/// next to it in the account (`vault.key` beside `vault.cleis`). Whoever reaches the account
/// reaches the vault: chosen by the person, only while the vault has no password.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountKey {
    pub v: u8,
    /// Hex of the 32-byte key.
    pub key: String,
    /// Base64 of the account secrets sealed with it (`Vault::seal_for_device`).
    pub sealed: String,
}

impl AccountKey {
    pub fn make(v: &cleis_store::Vault) -> Result<Self, String> {
        let key: [u8; 32] = cleis_crypto::random_bytes().map_err(|e| e.to_string())?;
        let sealed = v.seal_for_device(&key).map_err(|e| e.to_string())?;
        Ok(Self {
            v: 1,
            key: cleis_crypto::hex(&key),
            sealed: data_encoding::BASE64.encode(&sealed),
        })
    }

    /// Opens the vault at `path` with it.
    pub fn open(&self, path: &Path) -> Result<cleis_store::Vault, String> {
        let key: [u8; 32] = data_encoding::HEXLOWER
            .decode(self.key.as_bytes())
            .ok()
            .and_then(|k| k.try_into().ok())
            .ok_or("unreadable account key")?;
        let sealed = data_encoding::BASE64
            .decode(self.sealed.as_bytes())
            .map_err(|e| e.to_string())?;
        cleis_store::LockedVault::open(path)
            .and_then(|l| l.unlock_device(&key, &sealed))
            .map_err(|e| e.to_string())
    }
}

/// The place of the account key: the vault's file with `.key` in place of its extension.
pub fn key_place(place: &RemotePlace) -> RemotePlace {
    let swap = |p: &str| match p.rsplit_once('.') {
        Some((stem, _)) => format!("{stem}.key"),
        None => format!("{p}.key"),
    };
    let mut k = place.clone();
    match &mut k {
        RemotePlace::GitHub { path, .. } | RemotePlace::Sftp { path, .. } => *path = swap(path),
    }
    k
}

/// The account key kept beside the vault, if any.
pub fn read_key(place: &RemotePlace, secret: &Secret) -> Result<Option<AccountKey>, RemoteError> {
    let tmp = std::env::temp_dir().join(format!(
        "cleis-key-{}-{}",
        std::process::id(),
        cleis_core::now()
    ));
    let v = download(&key_place(place), secret, &tmp)?;
    if v.is_none() && !tmp.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(&tmp).map_err(|e| e.to_string());
    let _ = std::fs::remove_file(&tmp);
    Ok(serde_json::from_slice(&bytes?).ok())
}

/// Keeps the account key beside the vault (written over any earlier one).
pub fn write_key(place: &RemotePlace, secret: &Secret, k: &AccountKey) -> Result<(), RemoteError> {
    let kp = key_place(place);
    let tmp = std::env::temp_dir().join(format!(
        "cleis-key-{}-{}",
        std::process::id(),
        cleis_core::now()
    ));
    let version = download(&kp, secret, &tmp)?;
    let _ = std::fs::remove_file(&tmp);
    write_atomic(&tmp, &serde_json::to_vec(k).map_err(|e| e.to_string())?)?;
    let r = upload(&kp, secret, &tmp, version);
    let _ = std::fs::remove_file(&tmp);
    r
}

/// Takes the account key away (a password was set, or the account is left).
pub fn remove_key(place: &RemotePlace, secret: &Secret) -> Result<(), RemoteError> {
    remove(&key_place(place), secret)
}

/// Removes the place's file (mockup 37m). GitHub keeps the earlier, sealed versions in the
/// repository's history until it is pruned (D-72) or deleted. Missing: done.
pub fn remove(place: &RemotePlace, secret: &Secret) -> Result<(), RemoteError> {
    match place {
        RemotePlace::GitHub {
            repo,
            path,
            branch,
            api,
        } => GitHub::new(api, repo, path, branch, &secret.token)?.delete(),
        RemotePlace::Sftp { .. } => crate::sftp::remove(place, secret),
    }
}

/// The organization's backup copies (D-65): the place and its secret, sealed in the vault.
pub const ORG_BACKUP_SETTING: &str = "org_backup_place";

impl RemotePlace {
    /// The same place, for a file `name` in its folder (`path` is a folder here).
    pub fn file(&self, name: &str) -> RemotePlace {
        let join = |dir: &str| {
            let d = dir.trim_end_matches('/');
            if d.is_empty() {
                name.to_owned()
            } else {
                format!("{d}/{name}")
            }
        };
        match self.clone() {
            RemotePlace::GitHub {
                repo,
                path,
                branch,
                api,
            } => RemotePlace::GitHub {
                repo,
                path: join(&path),
                branch,
                api,
            },
            RemotePlace::Sftp {
                host,
                port,
                user,
                path,
                host_key,
            } => RemotePlace::Sftp {
                host,
                port,
                user,
                path: join(&path),
                host_key,
            },
        }
    }
}

/// Checks a folder place can be reached with its secret (GitHub: the repository is visible).
pub fn check(place: &RemotePlace, secret: &Secret) -> Result<(), RemoteError> {
    match place {
        RemotePlace::GitHub { repo, api, .. } => {
            let gh = GitHub::new(api, repo, "", "main", &secret.token)?;
            let url = format!(
                "{}/repos/{}",
                api.trim_end_matches('/'),
                repo.trim_matches('/')
            );
            let mut r = gh
                .agent
                .get(&url)
                .header("Authorization", &format!("Bearer {}", gh.token))
                .header("Accept", "application/vnd.github+json")
                .header("User-Agent", "Cleis")
                .call()
                .map_err(|e| e.to_string())?;
            let status = r.status().as_u16();
            let body = r.body_mut().read_to_vec().unwrap_or_default();
            if status == 200 {
                Ok(())
            } else {
                Err(github_error(status, &body))
            }
        }
        RemotePlace::Sftp { .. } => crate::sftp::server_key(place, secret).map(|_| ()),
    }
}

pub fn write_atomic(to: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut tmp = to.as_os_str().to_owned();
    tmp.push(".part");
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, to).map_err(|e| e.to_string())
}

// ---------- GitHub ----------

struct GitHub {
    url: String,
    /// `{api}/repos/{owner}/{name}`.
    root: String,
    branch: String,
    token: String,
    agent: ureq::Agent,
}

#[derive(Deserialize)]
struct Object {
    sha: String,
    #[serde(default)]
    r#type: String,
}

impl GitHub {
    fn new(api: &str, repo: &str, path: &str, branch: &str, token: &str) -> Result<Self, String> {
        // Audit S-13: plain HTTP to this computer only in debug builds (the tests' stand-in);
        // a released app sends the token by HTTPS only.
        let local = cfg!(debug_assertions) && api.starts_with("http://127.0.0.1:");
        if !api.starts_with("https://") && !local {
            return Err("GitHub is reached by HTTPS".into());
        }
        if repo.split('/').filter(|p| !p.is_empty()).count() != 2 {
            return Err("Repository: owner/name".into());
        }
        let path = path.trim_matches('/');
        let enc: Vec<String> = path.split('/').map(pct).collect();
        let root = format!(
            "{}/repos/{}",
            api.trim_end_matches('/'),
            repo.trim_matches('/')
        );
        Ok(Self {
            url: format!("{root}/contents/{}", enc.join("/")),
            root,
            branch: branch.to_owned(),
            token: token.to_owned(),
            agent: ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(60)))
                .http_status_as_error(false)
                .build()
                .new_agent(),
        })
    }

    fn get(&self, accept: &str) -> Result<(u16, Vec<u8>), String> {
        let mut r = self
            .agent
            .get(&format!("{}?ref={}", self.url, pct(&self.branch)))
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", accept)
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "Cleis")
            .call()
            .map_err(|e| e.to_string())?;
        let status = r.status().as_u16();
        let body = r
            .body_mut()
            .with_config()
            .limit(100 * 1024 * 1024)
            .read_to_vec()
            .map_err(|e| e.to_string())?;
        Ok((status, body))
    }

    /// Sha and bytes of the file; `None` when there is none yet.
    fn read(&self) -> Result<Option<(String, Vec<u8>)>, RemoteError> {
        let (status, body) = self.get("application/vnd.github.object")?;
        match status {
            200 => {}
            404 => return Ok(None),
            s => return Err(github_error(s, &body)),
        }
        let o: Object = serde_json::from_slice(&body).map_err(|e| e.to_string())?;
        if o.r#type != "file" {
            return Err(RemoteError::Other("this path is not a file".into()));
        }
        let (status, bytes) = self.get("application/vnd.github.raw")?;
        if status != 200 {
            return Err(github_error(status, &bytes));
        }
        Ok(Some((o.sha, bytes)))
    }

    fn delete(&self) -> Result<(), RemoteError> {
        let Some((sha, _)) = self.read()? else {
            return Ok(());
        };
        let body = serde_json::json!({
            "message": "Cleis: account deleted",
            "sha": sha,
            "branch": self.branch,
            "committer": {"name": "Cleis", "email": "cleis@users.noreply.github.com"},
        });
        let mut r = self
            .agent
            .delete(&self.url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "Cleis")
            .force_send_body()
            .send_json(&body)
            .map_err(|e| e.to_string())?;
        let status = r.status().as_u16();
        let body = r.body_mut().read_to_vec().unwrap_or_default();
        match status {
            200 | 404 => Ok(()),
            s => Err(github_error(s, &body)),
        }
    }

    fn write(&self, bytes: &[u8], sha: Version) -> Result<(), RemoteError> {
        let mut body = serde_json::json!({
            "message": "Cleis sync",
            "content": BASE64.encode(bytes),
            "branch": self.branch,
            "committer": {"name": "Cleis", "email": "cleis@users.noreply.github.com"},
        });
        if let Some(s) = sha {
            body["sha"] = s.into();
        }
        let mut r = self
            .agent
            .put(&self.url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "Cleis")
            .send_json(&body)
            .map_err(|e| e.to_string())?;
        let status = r.status().as_u16();
        let body = r.body_mut().read_to_vec().unwrap_or_default();
        match status {
            200 | 201 => Ok(()),
            // Another device sent its copy meanwhile (sha no longer current).
            409 => Err(RemoteError::Conflict),
            422 if String::from_utf8_lossy(&body).contains("sha") => Err(RemoteError::Conflict),
            s => Err(github_error(s, &body)),
        }
    }
}

// ---------- Pruning the GitHub history (audit L-9, D-72) ----------

/// Versions of the branch kept when it is pruned.
pub const KEEP_VERSIONS: usize = 30;
/// Days between two prunings.
pub const PRUNE_EVERY_DAYS: i64 = 7;

/// What pruning needs from a Git host.
trait GitHistory {
    fn head(&self) -> Result<String, RemoteError>;
    /// Newest first: `(sha, tree, message)`.
    fn history(&self, n: usize) -> Result<Vec<(String, String, String)>, RemoteError>;
    fn commit(
        &self,
        tree: &str,
        message: &str,
        parent: Option<&str>,
    ) -> Result<String, RemoteError>;
    fn force(&self, sha: &str) -> Result<(), RemoteError>;
}

/// Rewrites the branch to its last `keep` commits (same trees and messages, no older parent):
/// each sync adds the whole vault, so the repository would grow without end. Nothing happens
/// when the branch is short, or when another device wrote meanwhile (next time).
fn prune_with(git: &impl GitHistory, keep: usize) -> Result<bool, RemoteError> {
    let head = git.head()?;
    let hist = git.history(keep + 1)?;
    if keep == 0 || hist.len() <= keep || hist.first().map(|c| &c.0) != Some(&head) {
        return Ok(false);
    }
    let mut parent: Option<String> = None;
    for (_, tree, message) in hist[..keep].iter().rev() {
        parent = Some(git.commit(tree, message, parent.as_deref())?);
    }
    if git.head()? != head {
        return Ok(false);
    }
    match parent {
        Some(p) => git.force(&p).map(|()| true),
        None => Ok(false),
    }
}

/// Prunes a GitHub place's branch (other places: nothing to do).
pub fn prune(place: &RemotePlace, secret: &Secret, keep: usize) -> Result<bool, RemoteError> {
    match place {
        RemotePlace::GitHub {
            repo,
            path,
            branch,
            api,
        } => prune_with(&GitHub::new(api, repo, path, branch, &secret.token)?, keep),
        RemotePlace::Sftp { .. } => Ok(false),
    }
}

impl GitHub {
    fn call(
        &self,
        method: &str,
        url: &str,
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, RemoteError> {
        let auth = format!("Bearer {}", self.token);
        let res = match method {
            "GET" => self
                .agent
                .get(url)
                .header("Authorization", &auth)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .header("User-Agent", "Cleis")
                .call(),
            m => {
                let r = if m == "POST" {
                    self.agent.post(url)
                } else {
                    self.agent.patch(url)
                };
                r.header("Authorization", &auth)
                    .header("Accept", "application/vnd.github+json")
                    .header("X-GitHub-Api-Version", "2022-11-28")
                    .header("User-Agent", "Cleis")
                    .send_json(body.unwrap_or_default())
            }
        };
        let mut r = res.map_err(|e| e.to_string())?;
        let status = r.status().as_u16();
        let bytes = r.body_mut().read_to_vec().unwrap_or_default();
        if !(200..300).contains(&status) {
            return Err(github_error(status, &bytes));
        }
        serde_json::from_slice(&bytes).map_err(|e| RemoteError::Other(e.to_string()))
    }
}

impl GitHistory for GitHub {
    fn head(&self) -> Result<String, RemoteError> {
        let v = self.call(
            "GET",
            &format!("{}/git/ref/heads/{}", self.root, pct(&self.branch)),
            None,
        )?;
        Ok(v["object"]["sha"].as_str().unwrap_or_default().to_owned())
    }

    fn history(&self, n: usize) -> Result<Vec<(String, String, String)>, RemoteError> {
        let v = self.call(
            "GET",
            &format!(
                "{}/commits?sha={}&per_page={}",
                self.root,
                pct(&self.branch),
                n.min(100)
            ),
            None,
        )?;
        Ok(v.as_array()
            .map(|a| {
                a.iter()
                    .map(|c| {
                        (
                            c["sha"].as_str().unwrap_or_default().to_owned(),
                            c["commit"]["tree"]["sha"]
                                .as_str()
                                .unwrap_or_default()
                                .to_owned(),
                            c["commit"]["message"]
                                .as_str()
                                .unwrap_or_default()
                                .to_owned(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    fn commit(
        &self,
        tree: &str,
        message: &str,
        parent: Option<&str>,
    ) -> Result<String, RemoteError> {
        let v = self.call(
            "POST",
            &format!("{}/git/commits", self.root),
            Some(serde_json::json!({
                "message": message,
                "tree": tree,
                "parents": parent.into_iter().collect::<Vec<_>>(),
                "author": {"name": "Cleis", "email": "cleis@users.noreply.github.com"},
            })),
        )?;
        Ok(v["sha"].as_str().unwrap_or_default().to_owned())
    }

    fn force(&self, sha: &str) -> Result<(), RemoteError> {
        self.call(
            "PATCH",
            &format!("{}/git/refs/heads/{}", self.root, pct(&self.branch)),
            Some(serde_json::json!({"sha": sha, "force": true})),
        )
        .map(|_| ())
    }
}

fn github_error(status: u16, body: &[u8]) -> RemoteError {
    let msg = serde_json::from_slice::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v["message"].as_str().map(str::to_owned))
        .unwrap_or_default();
    RemoteError::Other(match status {
        401 => "GitHub refused the token".into(),
        403 => format!("GitHub refused access ({msg})"),
        404 => "Repository or branch not found (or the token cannot see it)".into(),
        _ => format!("GitHub: HTTP {status} {msg}"),
    })
}

fn pct(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `owner/name`, or a repository's address: GitHub (`https://github.com/owner/name`) or a GitHub
/// Enterprise server (`https://git.example-studio.com/owner/name`, API under `/api/v3`). Returns
/// the API base and `owner/name`.
pub fn github_repo(text: &str) -> (String, String) {
    let t = text.trim().trim_end_matches('/').trim_end_matches(".git");
    let Some((scheme, rest)) = t.split_once("://") else {
        return (github_api(), t.to_owned());
    };
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let api = if host == "github.com" || host == "www.github.com" {
        github_api()
    } else {
        format!("{scheme}://{host}/api/v3")
    };
    (api, path.to_owned())
}

/// Page creating a fine-grained token limited to one repository's contents.
pub fn github_token_page() -> &'static str {
    "https://github.com/settings/personal-access-tokens/new"
}

#[cfg(test)]
mod tests {
    use super::{GitHistory, RemoteError, prune_with};
    use std::cell::RefCell;

    /// `(sha, tree, message, parent)`.
    type Commit = (String, String, String, Option<String>);

    /// A branch in memory: its commits and head.
    #[derive(Default)]
    struct Repo {
        commits: RefCell<Vec<Commit>>,
        head: RefCell<String>,
        /// Another device writes after this many `head` reads.
        race_after: Option<usize>,
        reads: RefCell<usize>,
    }

    impl Repo {
        fn with(n: usize) -> Self {
            let r = Repo::default();
            let mut parent = None;
            for i in 0..n {
                parent = Some(
                    r.commit(&format!("tree{i}"), "Cleis sync", parent.as_deref())
                        .unwrap(),
                );
            }
            *r.head.borrow_mut() = parent.unwrap_or_default();
            r
        }
        fn chain(&self) -> Vec<String> {
            let c = self.commits.borrow();
            let mut out = vec![];
            let mut at = Some(self.head.borrow().clone());
            while let Some(sha) = at {
                let Some(x) = c.iter().find(|x| x.0 == sha) else {
                    break;
                };
                out.push(x.1.clone());
                at = x.3.clone();
            }
            out
        }
    }

    impl GitHistory for Repo {
        fn head(&self) -> Result<String, RemoteError> {
            *self.reads.borrow_mut() += 1;
            if self.race_after == Some(*self.reads.borrow()) {
                let h = self.head.borrow().clone();
                let new = self.commit("tree-other", "Cleis sync", Some(&h))?;
                *self.head.borrow_mut() = new;
            }
            Ok(self.head.borrow().clone())
        }
        fn history(&self, n: usize) -> Result<Vec<(String, String, String)>, RemoteError> {
            let c = self.commits.borrow();
            let mut out = vec![];
            let mut at = Some(self.head.borrow().clone());
            while let Some(sha) = at {
                if out.len() == n {
                    break;
                }
                let Some(x) = c.iter().find(|x| x.0 == sha) else {
                    break;
                };
                out.push((x.0.clone(), x.1.clone(), x.2.clone()));
                at = x.3.clone();
            }
            Ok(out)
        }
        fn commit(
            &self,
            tree: &str,
            message: &str,
            parent: Option<&str>,
        ) -> Result<String, RemoteError> {
            let mut c = self.commits.borrow_mut();
            let sha = format!("c{}", c.len());
            c.push((
                sha.clone(),
                tree.into(),
                message.into(),
                parent.map(str::to_owned),
            ));
            Ok(sha)
        }
        fn force(&self, sha: &str) -> Result<(), RemoteError> {
            *self.head.borrow_mut() = sha.into();
            Ok(())
        }
    }

    /// Audit L-9: the branch keeps its last versions, the newest first and unchanged.
    #[test]
    fn prune_keeps_the_last_versions() {
        let r = Repo::with(40);
        assert!(prune_with(&r, 30).unwrap());
        let chain = r.chain();
        assert_eq!(chain.len(), 30);
        assert_eq!(chain[0], "tree39");
        assert_eq!(chain[29], "tree10");
        // Short branch: nothing to do.
        let r = Repo::with(5);
        assert!(!prune_with(&r, 30).unwrap());
        assert_eq!(r.chain().len(), 5);
    }

    /// Another device wrote during the pruning: its commit is kept, pruning waits.
    #[test]
    fn prune_steps_back_when_the_branch_moved() {
        let mut r = Repo::with(40);
        r.race_after = Some(2);
        assert!(!prune_with(&r, 30).unwrap());
        let chain = r.chain();
        assert_eq!(chain.len(), 41);
        assert_eq!(chain[0], "tree-other");
    }

    use super::*;

    #[test]
    fn repositories() {
        assert_eq!(
            github_repo("lea-example/cleis-vault"),
            (
                "https://api.github.com".into(),
                "lea-example/cleis-vault".into()
            )
        );
        assert_eq!(
            github_repo("https://github.com/lea-example/cleis-vault.git"),
            (
                "https://api.github.com".into(),
                "lea-example/cleis-vault".into()
            )
        );
        assert_eq!(
            github_repo("https://git.example-studio.com/it/vaults/"),
            (
                "https://git.example-studio.com/api/v3".into(),
                "it/vaults".into()
            )
        );
    }
}
