//! Sign in with GitHub (mockup 37n, D-97, D-101): the OAuth device flow of the GitHub App
//! "cleis-sync" of matv-studio. Cleis shows a code, the person types it on github.com/login/device,
//! Cleis polls for the token; no token to make by hand, no client secret in the app (the device
//! flow needs none, to sign in or to renew). A GitHub App token reaches only the repositories the
//! app is installed on: the vault's private repository `cleis-vault`, made by Cleis when GitHub lets
//! it, else on GitHub's page filled in by Cleis, then the app installed on it (mockups 41). The
//! token lasts 8 hours and is renewed with its refresh token (6 months); both are kept per
//! computer, sealed in the vault like any place secret (`SyncConfig::secret_key`).
//!
//! Docs: https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/generating-a-user-access-token-for-a-github-app#using-the-device-flow-to-generate-a-user-access-token
//! and …/refreshing-user-access-tokens ("client_secret: required unless the user access token
//! was generated using the device flow").

use serde::Deserialize;
use std::time::Duration;

/// Client ID of the GitHub App "cleis-sync" (matv-studio, App ID 5954662), device flow on.
/// Not a secret. Permissions: Contents read and write, Metadata read.
pub const CLIENT_ID: &str = "Iv23liCw3sj1xvHc6oUh";
/// The app's short name: github.com/apps/cleis-sync.
pub const APP_SLUG: &str = "cleis-sync";
/// The repository made in the account the first time.
pub const REPO_NAME: &str = "cleis-vault";

/// Where GitHub is: github.com, or a stand-in in the tests.
#[derive(Clone, Debug)]
pub struct Hosts {
    pub web: String,
    pub api: String,
}

impl Hosts {
    /// The web side of an API address: api.github.com is github.com; GitHub Enterprise and the
    /// tests' stand-in serve the API under `/api/v3` of the site.
    pub fn of_api(api: &str) -> Self {
        let api = api.trim_end_matches('/');
        let web = if api == "https://api.github.com" {
            "https://github.com".to_owned()
        } else {
            api.trim_end_matches("/api/v3").to_owned()
        };
        Self {
            web,
            api: api.to_owned(),
        }
    }

    fn web(&self) -> &str {
        self.web.trim_end_matches('/')
    }

    /// GitHub's page making the vault's repository, filled in: private, `cleis-vault`.
    pub fn create_page(&self) -> String {
        format!(
            "{}/new?owner=%40me&name={REPO_NAME}&visibility=private&description=Cleis+vault",
            self.web()
        )
    }

    /// GitHub's page installing Cleis (the person picks "Only select repositories › cleis-vault").
    pub fn install_page(&self) -> String {
        format!("{}/apps/{APP_SLUG}/installations/new", self.web())
    }

    /// GitHub's page of the apps the account authorized, where Cleis's access is revoked.
    pub fn authorizations_page(&self) -> String {
        format!("{}/settings/apps/authorizations", self.web())
    }
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

/// What GitHub gives once the code is typed (or a refresh token used).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grant {
    pub token: String,
    /// Empty when the app's tokens do not expire.
    pub refresh: String,
    /// Seconds the token lasts (28800); 0: it does not end.
    pub expires_in: u64,
}

impl Grant {
    /// When the token ends (Unix seconds); 0: it does not.
    pub fn expires_at(&self) -> i64 {
        if self.expires_in == 0 {
            0
        } else {
            cleis_core::now() + self.expires_in as i64
        }
    }

    fn of(v: &serde_json::Value) -> Option<Self> {
        Some(Self {
            token: v["access_token"].as_str()?.to_owned(),
            refresh: v["refresh_token"].as_str().unwrap_or_default().to_owned(),
            expires_in: v["expires_in"].as_u64().unwrap_or(0),
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Poll {
    /// Not typed yet: ask again after the interval.
    Pending,
    /// Asked too often: wait 5 seconds more from now on (RFC 8628 §3.5).
    SlowDown,
    Token(Grant),
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
    // A GitHub App takes no scope: its permissions are the app's.
    let (status, body) = post_form(&url, &form(&[("client_id", CLIENT_ID)]))?;
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
    if let Some(g) = Grant::of(&v) {
        return Ok(Poll::Token(g));
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

/// A new token from the refresh token (D-101). The refresh token is spent: the answer has the
/// next one.
pub fn refresh(h: &Hosts, refresh_token: &str) -> Result<Grant, String> {
    let url = format!("{}/login/oauth/access_token", h.web());
    let (status, body) = post_form(
        &url,
        &form(&[
            ("client_id", CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ]),
    )?;
    let v: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|_| format!("GitHub: HTTP {status}, answer not read"))?;
    Grant::of(&v).ok_or_else(|| {
        match v["error"].as_str() {
            // Six months unused, or the access revoked: only a new sign-in helps.
            Some("bad_refresh_token") | Some("unauthorized_client") => {
                "GitHub ended the sign-in: sign in again in Settings › Account".into()
            }
            _ => v["error_description"]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("GitHub: HTTP {status}")),
        }
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

/// Whether the account installed Cleis (on any repository).
fn installed(h: &Hosts, token: &str) -> Result<bool, String> {
    let (status, v) = api_get(h, token, "/user/installations")?;
    if status != 200 {
        return Err(format!("GitHub: HTTP {status}"));
    }
    Ok(v["installations"]
        .as_array()
        .is_some_and(|a| a.iter().any(|i| i["app_slug"] == APP_SLUG)))
}

/// Where the account's vault stands for Cleis.
#[derive(Debug, PartialEq, Eq)]
pub enum Found {
    /// `login/cleis-vault` and its branch: Cleis reaches it.
    Repo(String, String),
    /// Cleis does not see it: to make on GitHub (`made` false), then to install Cleis on
    /// (mockups 41a, 41b). GitHub answers 404 alike for a repository missing and one Cleis is
    /// not installed on.
    Setup { made: bool },
    /// Restoring, and Cleis was never installed on the account: no vault to look for (40b).
    Missing,
}

/// The vault's repository `login/cleis-vault`. When `create`, Cleis tries to make it (private);
/// GitHub refuses that to most GitHub App tokens, so the setup steps follow.
pub fn vault_repo(h: &Hosts, token: &str, login: &str, create: bool) -> Result<Found, String> {
    let full = format!("{login}/{REPO_NAME}");
    let (status, v) = api_get(h, token, &format!("/repos/{full}"))?;
    match status {
        200 => {
            let branch = v["default_branch"].as_str().unwrap_or("main").to_owned();
            if v["private"].as_bool() == Some(false) {
                return Err(format!("{full} is public: make it private on GitHub first"));
            }
            Ok(Found::Repo(full, branch))
        }
        // Restoring: installed somewhere, the vault may be there unreached; never installed,
        // no vault made by Cleis.
        404 if !create => Ok(if installed(h, token)? {
            Found::Setup { made: true }
        } else {
            Found::Missing
        }),
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
            match status {
                // Made, and the token reaches it (the app installed on every repository).
                201 => {
                    let branch = v["default_branch"].as_str().unwrap_or("main").to_owned();
                    let (s2, _) = api_get(h, token, &format!("/repos/{full}"))?;
                    Ok(if s2 == 200 {
                        Found::Repo(full, branch)
                    } else {
                        Found::Setup { made: true }
                    })
                }
                // "name already exists": there, Cleis not installed on it.
                422 => Ok(Found::Setup { made: true }),
                // 403 "Resource not accessible by integration", 404: made on GitHub's page.
                401 => Err(format!("GitHub refused the token (HTTP {status})")),
                _ => Ok(Found::Setup { made: false }),
            }
        }
        401 => Err(format!("GitHub refused the token (HTTP {status})")),
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
                r#"{"access_token":"ghu_x","expires_in":28800,"refresh_token":"ghr_y","refresh_token_expires_in":15897600,"token_type":"bearer","scope":""}"#,
            ),
            (200, r#"{"login":"lea-example"}"#),
            (404, r#"{"message":"Not Found"}"#),
            (
                201,
                r#"{"full_name":"lea-example/cleis-vault","default_branch":"main","private":true}"#,
            ),
            (200, r#"{"default_branch":"main","private":true}"#),
        ]);
        let c = start(&h).unwrap();
        assert_eq!(c.user_code, "WDJB-MJHT");
        assert_eq!(poll(&h, &c.device_code).unwrap(), Poll::Pending);
        assert_eq!(poll(&h, &c.device_code).unwrap(), Poll::SlowDown);
        let Poll::Token(g) = poll(&h, &c.device_code).unwrap() else {
            panic!()
        };
        assert_eq!(
            (g.token.as_str(), g.refresh.as_str(), g.expires_in),
            ("ghu_x", "ghr_y", 28800)
        );
        assert!(g.expires_at() > cleis_core::now() + 28000);
        let who = login(&h, &g.token).unwrap();
        assert_eq!(who, "lea-example");
        assert_eq!(
            vault_repo(&h, &g.token, &who, true).unwrap(),
            Found::Repo("lea-example/cleis-vault".into(), "main".into())
        );
        let seen = t.join().unwrap();
        assert!(seen[0].starts_with("POST /login/device/code"));
        // A GitHub App asks no scope.
        assert!(seen[0].contains(&format!("client_id={CLIENT_ID}")) && !seen[0].contains("scope"));
        assert!(
            seen[1].contains("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code")
        );
        // (The JSON body is sent chunked: only the request line is read here.)
        assert!(seen[6].starts_with("POST /user/repos"));
    }

    /// Mockups 41: GitHub refuses the app to make the repository (403), or it is there already
    /// (422), or the token does not see it yet (restoring): the setup steps.
    #[test]
    fn repository_to_make_or_install() {
        let (h, t) = stand_in(vec![
            (404, r#"{"message":"Not Found"}"#),
            (
                403,
                r#"{"message":"Resource not accessible by integration"}"#,
            ),
            (404, r#"{"message":"Not Found"}"#),
            (422, r#"{"message":"Repository creation failed."}"#),
            (404, r#"{"message":"Not Found"}"#),
            (
                200,
                r#"{"total_count":1,"installations":[{"id":7,"app_slug":"cleis-sync"}]}"#,
            ),
            (404, r#"{"message":"Not Found"}"#),
            (200, r#"{"total_count":0,"installations":[]}"#),
        ]);
        assert_eq!(
            vault_repo(&h, "t", "lea", true).unwrap(),
            Found::Setup { made: false }
        );
        assert_eq!(
            vault_repo(&h, "t", "lea", true).unwrap(),
            Found::Setup { made: true }
        );
        // Restoring: installed, the vault not reached; never installed, none.
        assert_eq!(
            vault_repo(&h, "t", "lea", false).unwrap(),
            Found::Setup { made: true }
        );
        assert_eq!(vault_repo(&h, "t", "lea", false).unwrap(), Found::Missing);
        t.join().unwrap();
        assert_eq!(
            h.create_page(),
            format!(
                "{}/new?owner=%40me&name=cleis-vault&visibility=private&description=Cleis+vault",
                h.web
            )
        );
        assert!(
            h.install_page()
                .ends_with("/apps/cleis-sync/installations/new")
        );
    }

    #[test]
    fn token_renewed_with_its_refresh_token() {
        let (h, t) = stand_in(vec![
            (
                200,
                r#"{"access_token":"ghu_2","expires_in":28800,"refresh_token":"ghr_2","token_type":"bearer"}"#,
            ),
            (200, r#"{"error":"bad_refresh_token"}"#),
        ]);
        let g = refresh(&h, "ghr_1").unwrap();
        assert_eq!((g.token.as_str(), g.refresh.as_str()), ("ghu_2", "ghr_2"));
        assert!(refresh(&h, "ghr_1").unwrap_err().contains("sign in again"));
        let seen = t.join().unwrap();
        assert!(
            seen[0].contains("grant_type=refresh_token") && seen[0].contains("refresh_token=ghr_1")
        );
        // Device flow: no client secret sent.
        assert!(!seen[0].contains("client_secret"));
        assert_eq!(
            Hosts::of_api("https://api.github.com").web,
            "https://github.com"
        );
        assert_eq!(
            Hosts::of_api("https://git.example-studio.com/api/v3").web,
            "https://git.example-studio.com"
        );
    }

    #[test]
    fn refusals_and_public_repository() {
        let (h, t) = stand_in(vec![
            (200, r#"{"error":"access_denied"}"#),
            (200, r#"{"error":"expired_token"}"#),
            (200, r#"{"default_branch":"main","private":false}"#),
        ]);
        assert!(matches!(poll(&h, "d").unwrap(), Poll::Failed(_)));
        assert!(matches!(poll(&h, "d").unwrap(), Poll::Failed(_)));
        assert!(
            vault_repo(&h, "t", "lea", true)
                .unwrap_err()
                .contains("public")
        );
        t.join().unwrap();
    }
}
