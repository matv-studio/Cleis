//! Sign in with GitHub as Revel does (mockup 37n, 27/11/2026): the OAuth device flow of the
//! "Cleis" OAuth app of matv-studio. Cleis shows a code, the person types it on
//! github.com/login/device, Cleis polls for the token; no token to make by hand, no client
//! secret in the app (the device flow needs none). The token does not expire (the app's
//! "Expire user access tokens" is off, user's choice of 27/11/2026); it is kept per computer,
//! sealed in the vault like any place secret (`SyncConfig::secret_key`). The vault goes to a
//! private repository `cleis-vault` of the account, made the first time.
//!
//! Docs: https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps#device-flow

use serde::Deserialize;
use std::time::Duration;

/// Client ID of the OAuth app "Cleis" (matv-studio), device flow on. Not a secret.
pub const CLIENT_ID: &str = "Ov23li1POERzptJCQDPI";
/// `repo`: the private repository that keeps the vault (made, read, written).
pub const SCOPE: &str = "repo";
/// The repository made in the account the first time.
pub const REPO_NAME: &str = "cleis-vault";

/// Where GitHub is: github.com, or a stand-in in the tests.
#[derive(Clone, Debug)]
pub struct Hosts {
    pub web: String,
    pub api: String,
}

impl Default for Hosts {
    fn default() -> Self {
        Self {
            web: "https://github.com".into(),
            api: "https://api.github.com".into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct DeviceCode {
    pub device_code: String,
    /// `WDJB-MJHT`: typed by the person on GitHub.
    pub user_code: String,
    pub verification_uri: String,
    /// Seconds before the code expires (900).
    pub expires_in: u64,
    /// Seconds to wait between two polls (5).
    pub interval: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Poll {
    /// Not typed yet: ask again after the interval.
    Pending,
    /// Asked too often: wait 5 seconds more from now on (RFC 8628 §3.5).
    SlowDown,
    Token(String),
    /// The person refused, or the code expired: start again.
    Failed(String),
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .new_agent()
}

/// Plain HTTP only to this computer, and only in debug builds (the tests' stand-in).
fn checked(url: &str) -> Result<(), String> {
    let local = cfg!(debug_assertions) && url.starts_with("http://127.0.0.1:");
    if url.starts_with("https://") || local {
        Ok(())
    } else {
        Err("GitHub is reached by HTTPS".into())
    }
}

fn form(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{k}={}", enc(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn enc(s: &str) -> String {
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

fn post_form(url: &str, body: &str) -> Result<(u16, Vec<u8>), String> {
    checked(url)?;
    let mut r = agent()
        .post(url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("User-Agent", "Cleis")
        .send(body)
        .map_err(|e| e.to_string())?;
    let status = r.status().as_u16();
    let bytes = r.body_mut().read_to_vec().map_err(|e| e.to_string())?;
    Ok((status, bytes))
}

/// Step 1: a code for the person to type on GitHub.
pub fn start(h: &Hosts) -> Result<DeviceCode, String> {
    let url = format!("{}/login/device/code", h.web.trim_end_matches('/'));
    let (status, body) = post_form(&url, &form(&[("client_id", CLIENT_ID), ("scope", SCOPE)]))?;
    if status != 200 {
        return Err(format!("GitHub: HTTP {status}"));
    }
    serde_json::from_slice::<DeviceCode>(&body).map_err(|_| {
        // GitHub answers 200 with an error when the device flow is off for the app.
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
        v["error_description"]
            .as_str()
            .or(v["error"].as_str())
            .unwrap_or("GitHub's answer could not be read")
            .to_owned()
    })
}

/// Step 2, every `interval` seconds: the token once the code is typed.
pub fn poll(h: &Hosts, device_code: &str) -> Result<Poll, String> {
    let url = format!("{}/login/oauth/access_token", h.web.trim_end_matches('/'));
    let (status, body) = post_form(
        &url,
        &form(&[
            ("client_id", CLIENT_ID),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ]),
    )?;
    let v: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|_| format!("GitHub: HTTP {status}, answer not read"))?;
    if let Some(t) = v["access_token"].as_str() {
        return Ok(Poll::Token(t.to_owned()));
    }
    Ok(match v["error"].as_str().unwrap_or_default() {
        "authorization_pending" => Poll::Pending,
        "slow_down" => Poll::SlowDown,
        "expired_token" => Poll::Failed("The code expired: sign in again".into()),
        "access_denied" => Poll::Failed("Access refused on GitHub".into()),
        other => Poll::Failed(
            v["error_description"]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("GitHub: {other}")),
        ),
    })
}

fn api_get(h: &Hosts, token: &str, path: &str) -> Result<(u16, serde_json::Value), String> {
    let url = format!("{}{path}", h.api.trim_end_matches('/'));
    checked(&url)?;
    let mut r = agent()
        .get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "Cleis")
        .call()
        .map_err(|e| e.to_string())?;
    let status = r.status().as_u16();
    let v = r.body_mut().read_json().unwrap_or_default();
    Ok((status, v))
}

/// The account the token belongs to (`login`).
pub fn login(h: &Hosts, token: &str) -> Result<String, String> {
    let (status, v) = api_get(h, token, "/user")?;
    if status != 200 {
        return Err(format!("GitHub refused the token (HTTP {status})"));
    }
    v["login"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "GitHub did not give the account".into())
}

/// The vault's repository `login/cleis-vault` and its branch; made private the first time when
/// `create` (an existing vault is only looked for on a new computer).
/// `vault_repo` found no repository to restore (the account has no vault yet).
pub const NO_VAULT: &str = "No vault on this GitHub account yet";

pub fn vault_repo(
    h: &Hosts,
    token: &str,
    login: &str,
    create: bool,
) -> Result<(String, String), String> {
    let full = format!("{login}/{REPO_NAME}");
    let (status, v) = api_get(h, token, &format!("/repos/{full}"))?;
    match status {
        200 => {
            let branch = v["default_branch"].as_str().unwrap_or("main").to_owned();
            if v["private"].as_bool() == Some(false) {
                return Err(format!("{full} is public: make it private on GitHub first"));
            }
            Ok((full, branch))
        }
        404 if !create => Err(NO_VAULT.into()),
        404 => {
            let url = format!("{}/user/repos", h.api.trim_end_matches('/'));
            checked(&url)?;
            let body = serde_json::json!({
                "name": REPO_NAME,
                "private": true,
                "auto_init": true,
                "description": "Cleis: the vault, sealed on each computer before it leaves",
            });
            let mut r = agent()
                .post(&url)
                .header("Authorization", &format!("Bearer {token}"))
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .header("User-Agent", "Cleis")
                .send_json(&body)
                .map_err(|e| e.to_string())?;
            let status = r.status().as_u16();
            let v: serde_json::Value = r.body_mut().read_json().unwrap_or_default();
            if status != 201 {
                return Err(format!(
                    "GitHub did not make {full} (HTTP {status} {})",
                    v["message"].as_str().unwrap_or_default()
                ));
            }
            let branch = v["default_branch"].as_str().unwrap_or("main").to_owned();
            Ok((full, branch))
        }
        s => Err(format!("GitHub: HTTP {s}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;

    /// A stand-in for github.com and api.github.com: answers each request in turn with
    /// `(status, body)` and records the request lines and bodies.
    fn stand_in(
        answers: Vec<(u16, &'static str)>,
    ) -> (Hosts, std::thread::JoinHandle<Vec<String>>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        let t = std::thread::spawn(move || {
            let mut seen = vec![];
            for (status, body) in answers {
                let (mut s, _) = l.accept().unwrap();
                let mut r = BufReader::new(s.try_clone().unwrap());
                let mut line = String::new();
                r.read_line(&mut line).unwrap();
                let mut len = 0;
                loop {
                    let mut h = String::new();
                    r.read_line(&mut h).unwrap();
                    if h == "\r\n" || h.is_empty() {
                        break;
                    }
                    if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap();
                    }
                }
                let mut b = vec![0; len];
                r.read_exact(&mut b).unwrap();
                seen.push(format!("{} {}", line.trim(), String::from_utf8_lossy(&b)));
                let reply = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                s.write_all(reply.as_bytes()).unwrap();
            }
            seen
        });
        (
            Hosts {
                web: base.clone(),
                api: base,
            },
            t,
        )
    }

    #[test]
    fn device_flow_then_repository_made() {
        let (h, t) = stand_in(vec![
            (
                200,
                r#"{"device_code":"dc1","user_code":"WDJB-MJHT","verification_uri":"https://github.com/login/device","expires_in":900,"interval":5}"#,
            ),
            (200, r#"{"error":"authorization_pending"}"#),
            (200, r#"{"error":"slow_down","interval":10}"#),
            (
                200,
                r#"{"access_token":"gho_x","token_type":"bearer","scope":"repo"}"#,
            ),
            (200, r#"{"login":"lea-example"}"#),
            (404, r#"{"message":"Not Found"}"#),
            (
                201,
                r#"{"full_name":"lea-example/cleis-vault","default_branch":"main","private":true}"#,
            ),
        ]);
        let c = start(&h).unwrap();
        assert_eq!(c.user_code, "WDJB-MJHT");
        assert_eq!(poll(&h, &c.device_code).unwrap(), Poll::Pending);
        assert_eq!(poll(&h, &c.device_code).unwrap(), Poll::SlowDown);
        let Poll::Token(tok) = poll(&h, &c.device_code).unwrap() else {
            panic!()
        };
        let who = login(&h, &tok).unwrap();
        assert_eq!(who, "lea-example");
        let (repo, branch) = vault_repo(&h, &tok, &who, true).unwrap();
        assert_eq!(
            (repo.as_str(), branch.as_str()),
            ("lea-example/cleis-vault", "main")
        );
        let seen = t.join().unwrap();
        assert!(seen[0].starts_with("POST /login/device/code"));
        assert!(
            seen[0].contains(&format!("client_id={CLIENT_ID}")) && seen[0].contains("scope=repo")
        );
        assert!(
            seen[1].contains("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code")
        );
        // (The JSON body is sent chunked: only the request line is read here.)
        assert!(seen[6].starts_with("POST /user/repos"));
    }

    #[test]
    fn refusals_and_public_repository() {
        let (h, t) = stand_in(vec![
            (200, r#"{"error":"access_denied"}"#),
            (200, r#"{"error":"expired_token"}"#),
            (200, r#"{"default_branch":"main","private":false}"#),
            (404, r#"{"message":"Not Found"}"#),
        ]);
        assert!(matches!(poll(&h, "d").unwrap(), Poll::Failed(_)));
        assert!(matches!(poll(&h, "d").unwrap(), Poll::Failed(_)));
        assert!(
            vault_repo(&h, "t", "lea", true)
                .unwrap_err()
                .contains("public")
        );
        // A new computer: nothing made, the absence said.
        assert!(vault_repo(&h, "t", "lea", false).is_err());
        t.join().unwrap();
    }
}
