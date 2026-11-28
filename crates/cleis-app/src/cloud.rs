//! Sign in with Microsoft or Google to keep the vault in the account's own storage (D-50, D-80,
//! D-104): the app folder of OneDrive (`Files.ReadWrite.AppFolder`) or the hidden app data folder
//! of Google Drive (`drive.appdata`). The browser shows the provider's page; Cleis gets the code
//! on `http://127.0.0.1:<port>` (RFC 8252, PKCE RFC 7636) and keeps the tokens sealed in the
//! vault, like the GitHub ones. Then the file is read and written with the provider's API, merged
//! here like any other place (rows stay sealed).

use crate::oidc::{pct, random_b64, wait_callback};
use crate::remote::{DriveFolder, RemoteError, Version};
use data_encoding::BASE64URL_NOPAD as B64;
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

/// Cleis' registration with Microsoft Entra ("Cleis-Sync", D-103): a public client, no secret.
pub const MICROSOFT_CLIENT_ID: &str = "ab08c5d7-e197-4b7d-beea-105b96bc68b5";
/// Cleis' "Desktop app" client in Google Cloud (D-104).
pub const GOOGLE_CLIENT_ID: &str =
    "1020018847728-29t0jep2co6uge1bdsjau15jdmfp5k1o.apps.googleusercontent.com";
/// Google's desktop clients carry a secret that is not one (it ships in every copy of the app);
/// it is still kept out of git: the build gets it from the CI secret `GOOGLE_CLIENT_SECRET`.
const GOOGLE_SECRET: Option<&str> = option_env!("CLEIS_GOOGLE_CLIENT_SECRET");

const MICROSOFT_API: &str = "https://graph.microsoft.com/v1.0";
const MICROSOFT_LOGIN: &str = "https://login.microsoftonline.com/common/oauth2/v2.0";
const GOOGLE_API: &str = "https://www.googleapis.com";
const GOOGLE_AUTH: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN: &str = "https://oauth2.googleapis.com/token";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cloud {
    Microsoft,
    Google,
}

impl Cloud {
    pub fn name(self) -> &'static str {
        match self {
            Cloud::Microsoft => "Microsoft",
            Cloud::Google => "Google",
        }
    }

    /// Where the vault goes, as the person knows it.
    pub fn storage(self) -> &'static str {
        match self {
            Cloud::Microsoft => "OneDrive",
            Cloud::Google => "Google Drive",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Cloud::Microsoft => "window",
            Cloud::Google => "globe",
        }
    }

    pub fn provider(self) -> crate::sync::Provider {
        match self {
            Cloud::Microsoft => crate::sync::Provider::OneDrive,
            Cloud::Google => crate::sync::Provider::GoogleDrive,
        }
    }

    /// The page of the account where Cleis' access is taken away (every computer).
    pub fn revoke_page(self) -> &'static str {
        match self {
            Cloud::Microsoft => "https://account.live.com/consent/Manage",
            Cloud::Google => "https://myaccount.google.com/connections",
        }
    }

    /// `custom` (D-110): a folder of the person's choice. Microsoft: the whole drive
    /// (`Files.ReadWrite`, to list its folders). Google: only the files Cleis makes or that are
    /// picked in Google's own window (`drive.file`, which that window wants alone).
    fn scopes(self, custom: bool) -> &'static str {
        match (self, custom) {
            (Cloud::Microsoft, false) => {
                "openid profile email offline_access User.Read Files.ReadWrite.AppFolder"
            }
            (Cloud::Microsoft, true) => {
                "openid profile email offline_access User.Read Files.ReadWrite"
            }
            (Cloud::Google, false) => "openid email https://www.googleapis.com/auth/drive.appdata",
            (Cloud::Google, true) => "https://www.googleapis.com/auth/drive.file",
        }
    }
}

/// Where a provider is: its own addresses, or a stand-in in the tests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoints {
    pub auth: String,
    pub token: String,
    /// Graph (`…/v1.0`) or `https://www.googleapis.com` (Drive and its upload path).
    pub api: String,
    pub client_id: String,
    /// Google only; empty for Microsoft.
    pub secret: String,
    /// Google's revocation address (Microsoft has none for a desktop app).
    pub revoke: String,
}

impl Endpoints {
    pub fn official(c: Cloud) -> Self {
        match c {
            Cloud::Microsoft => Self {
                auth: format!("{MICROSOFT_LOGIN}/authorize"),
                token: format!("{MICROSOFT_LOGIN}/token"),
                api: MICROSOFT_API.into(),
                client_id: MICROSOFT_CLIENT_ID.into(),
                secret: String::new(),
                revoke: String::new(),
            },
            Cloud::Google => Self {
                auth: GOOGLE_AUTH.into(),
                token: GOOGLE_TOKEN.into(),
                api: GOOGLE_API.into(),
                client_id: GOOGLE_CLIENT_ID.into(),
                secret: GOOGLE_SECRET.unwrap_or_default().into(),
                revoke: "https://oauth2.googleapis.com/revoke".into(),
            },
        }
    }

    /// A stand-in serving every path from one address (`/authorize`, `/token`, the API).
    pub fn stand_in(c: Cloud, base: &str) -> Self {
        Self {
            auth: format!("{base}/authorize"),
            token: format!("{base}/token"),
            api: base.into(),
            client_id: format!("cleis-test-{}", c.name().to_lowercase()),
            secret: if c == Cloud::Google {
                "test-secret".into()
            } else {
                String::new()
            },
            revoke: format!("{base}/revoke"),
        }
    }

    /// Google needs its secret, which only the release builds carry.
    pub fn usable(&self, c: Cloud) -> bool {
        !self.client_id.is_empty() && (c == Cloud::Microsoft || !self.secret.is_empty())
    }

    /// The endpoints of a place's `api` (kept in the place; the token address follows from it).
    pub fn of_api(c: Cloud, api: &str) -> Self {
        let official = Self::official(c);
        if api.trim_end_matches('/') == official.api {
            official
        } else {
            Self::stand_in(c, api.trim_end_matches('/'))
        }
    }
}

/// Both providers (tests point them at a stand-in).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloudHosts {
    pub microsoft: Endpoints,
    pub google: Endpoints,
}

impl Default for CloudHosts {
    fn default() -> Self {
        Self {
            microsoft: Endpoints::official(Cloud::Microsoft),
            google: Endpoints::official(Cloud::Google),
        }
    }
}

impl CloudHosts {
    pub fn of(&self, c: Cloud) -> &Endpoints {
        match c {
            Cloud::Microsoft => &self.microsoft,
            Cloud::Google => &self.google,
        }
    }

    pub fn stand_in(base: &str) -> Self {
        Self {
            microsoft: Endpoints::stand_in(Cloud::Microsoft, base),
            google: Endpoints::stand_in(Cloud::Google, base),
        }
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
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
        Err("The provider is reached by HTTPS".into())
    }
}

/// A sign-in waiting for the browser.
pub struct Pending {
    pub cloud: Cloud,
    /// The provider's page, opened in the browser.
    pub url: String,
    listener: TcpListener,
    redirect: String,
    verifier: String,
    state: String,
    endpoints: Endpoints,
    /// Set when the person cancels: the wait stops.
    pub cancel: Arc<AtomicBool>,
}

/// The loopback listener and the provider's address (no network). `custom` (D-110): the vault
/// goes to a folder the person chooses (Google: its folder picker opens after the sign-in).
pub fn prepare(cloud: Cloud, e: &Endpoints, hint: &str, custom: bool) -> Result<Pending, String> {
    checked(&e.auth)?;
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect = format!("http://127.0.0.1:{port}");
    let verifier = random_b64(32)?;
    let challenge = B64.encode(&cleis_crypto::sha256(verifier.as_bytes()));
    let state = random_b64(16)?;
    let mut url = format!(
        "{}{}response_type=code&client_id={}&redirect_uri={}&scope={}&code_challenge={challenge}&code_challenge_method=S256&state={state}",
        e.auth,
        if e.auth.contains('?') { '&' } else { '?' },
        pct(&e.client_id),
        pct(&redirect),
        pct(cloud.scopes(custom)),
    );
    match cloud {
        // A refresh token every time (Google gives one only when asked to consent).
        Cloud::Google => url.push_str("&access_type=offline&prompt=consent%20select_account"),
        Cloud::Microsoft => url.push_str("&prompt=select_account"),
    }
    // Google Picker for desktop apps: shown after the consent, folders only.
    if cloud == Cloud::Google && custom {
        url.push_str(&format!(
            "&trigger_onepick=true&allow_folder_selection=true&mimetypes={}",
            pct(FOLDER_MIME)
        ));
    }
    if !hint.is_empty() {
        url.push_str(&format!("&login_hint={}", pct(hint)));
    }
    Ok(Pending {
        cloud,
        url,
        listener,
        redirect,
        verifier,
        state,
        endpoints: e.clone(),
        cancel: Arc::new(AtomicBool::new(false)),
    })
}

const FOLDER_MIME: &str = "application/vnd.google-apps.folder";

/// What a sign-in or a renewal gives.
#[derive(Clone, Debug, Default)]
pub struct Grant {
    pub token: String,
    pub refresh: String,
    /// Unix seconds when `token` ends.
    pub expires: i64,
    /// The account's e-mail address (from the ID token), when it came with the grant.
    pub account: String,
    /// Google's folder picker (D-110): the ids of what was picked.
    pub picked: Vec<String>,
}

fn grant_of(v: &serde_json::Value, old_refresh: &str) -> Result<Grant, String> {
    if let Some(e) = v["error"].as_str() {
        let why = v["error_description"].as_str().unwrap_or(e);
        return Err(if e == "invalid_grant" {
            "Signed out by the provider: sign in again".into()
        } else {
            why.lines().next().unwrap_or(e).to_owned()
        });
    }
    let token = v["access_token"]
        .as_str()
        .ok_or("the provider gave no access token")?
        .to_owned();
    // Microsoft gives a new refresh token each time; Google keeps the first one.
    let refresh = v["refresh_token"]
        .as_str()
        .unwrap_or(old_refresh)
        .to_owned();
    let expires = cleis_core::now() + v["expires_in"].as_i64().unwrap_or(3600);
    let account = v["id_token"]
        .as_str()
        .and_then(|t| t.split('.').nth(1))
        .and_then(|b| B64.decode(b.trim_end_matches('=').as_bytes()).ok())
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|c| {
            c["email"]
                .as_str()
                .or(c["preferred_username"].as_str())
                .map(str::to_owned)
        })
        .unwrap_or_default();
    Ok(Grant {
        token,
        refresh,
        expires,
        account,
        picked: vec![],
    })
}

fn post_token(e: &Endpoints, form: String) -> Result<serde_json::Value, String> {
    checked(&e.token)?;
    let mut form = form;
    form.push_str(&format!("&client_id={}", pct(&e.client_id)));
    if !e.secret.is_empty() {
        form.push_str(&format!("&client_secret={}", pct(&e.secret)));
    }
    let mut r = agent()
        .post(&e.token)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .send(form.as_bytes())
        .map_err(|err| format!("{}: {err}", e.token))?;
    r.body_mut()
        .read_json::<serde_json::Value>()
        .map_err(|err| err.to_string())
}

/// Waits for the browser (5 minutes at most), then exchanges the code for the tokens.
pub fn finish(p: Pending) -> Result<Grant, String> {
    let (code, extra) = wait_callback(&p.listener, &p.state, &p.cancel)?;
    let picked: Vec<String> = extra
        .iter()
        .filter(|(k, _)| k == "picked_file_ids")
        .flat_map(|(_, v)| v.split(','))
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .collect();
    let form = format!(
        "grant_type=authorization_code&code={}&redirect_uri={}&code_verifier={}",
        pct(&code),
        pct(&p.redirect),
        pct(&p.verifier)
    );
    let mut g = grant_of(&post_token(&p.endpoints, form)?, "")?;
    if g.refresh.is_empty() {
        return Err("The provider gave no lasting access: sign in again".into());
    }
    g.picked = picked;
    Ok(g)
}

/// A new access token from the refresh token (no browser); `custom`: the place is in a folder
/// the person chose (D-110), with the wider access asked for it.
pub fn refresh(
    c: Cloud,
    e: &Endpoints,
    refresh_token: &str,
    custom: bool,
) -> Result<Grant, String> {
    let mut form = format!(
        "grant_type=refresh_token&refresh_token={}",
        pct(refresh_token)
    );
    if c == Cloud::Microsoft {
        form.push_str(&format!("&scope={}", pct(c.scopes(custom))));
    }
    grant_of(&post_token(e, form)?, refresh_token)
}

/// Takes Cleis' access away from the account (Google; every computer).
pub fn revoke(e: &Endpoints, token: &str) -> Result<(), String> {
    if e.revoke.is_empty() || token.is_empty() {
        return Ok(());
    }
    checked(&e.revoke)?;
    let r = agent()
        .post(&e.revoke)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send(format!("token={}", pct(token)).as_bytes())
        .map_err(|err| err.to_string())?;
    if r.status().is_success() {
        Ok(())
    } else {
        Err(format!("{} ({})", e.revoke, r.status()))
    }
}

// ---------- The vault's file in the account ----------

fn status_error(c: Cloud, status: u16, body: &[u8]) -> RemoteError {
    let msg = serde_json::from_slice::<serde_json::Value>(body)
        .ok()
        .and_then(|v| {
            v["error"]["message"]
                .as_str()
                .or(v["error_description"].as_str())
                .map(str::to_owned)
        })
        .unwrap_or_default();
    match status {
        401 => RemoteError::Other(format!("{}: sign in again", c.storage())),
        403 => RemoteError::Other(format!("{}: access refused {msg}", c.storage())),
        412 | 409 => RemoteError::Conflict,
        507 => RemoteError::Other(format!("{}: no space left", c.storage())),
        _ => RemoteError::Other(format!("{} ({status}) {msg}", c.storage())),
    }
}

/// `a/b c.cleis` → `a/b%20c.cleis` (slashes kept: OneDrive paths).
fn path_enc(p: &str) -> String {
    p.trim_matches('/')
        .split('/')
        .map(pct)
        .collect::<Vec<_>>()
        .join("/")
}

fn call(
    c: Cloud,
    method: &str,
    url: &str,
    token: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
) -> Result<(u16, Vec<u8>, Option<String>), RemoteError> {
    checked(url)?;
    let a = agent();
    let auth = format!("Bearer {token}");
    macro_rules! with_headers {
        ($r:expr) => {{
            let mut r = $r
                .header("Authorization", &auth)
                .header("User-Agent", "Cleis");
            for (k, v) in headers {
                r = r.header(*k, *v);
            }
            r
        }};
    }
    let resp = match (method, body) {
        ("GET", _) => with_headers!(a.get(url)).call(),
        ("DELETE", _) => with_headers!(a.delete(url)).call(),
        ("PUT", Some(b)) => with_headers!(a.put(url)).send(b),
        ("PATCH", Some(b)) => with_headers!(a.patch(url)).send(b),
        ("POST", Some(b)) => with_headers!(a.post(url)).send(b),
        _ => return Err(RemoteError::Other("bad request".into())),
    };
    let mut r = resp.map_err(|e| RemoteError::Other(format!("{}: {e}", c.storage())))?;
    let status = r.status().as_u16();
    let etag = r
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let bytes = r
        .body_mut()
        .with_config()
        .limit(512 * 1024 * 1024)
        .read_to_vec()
        .unwrap_or_default();
    Ok((status, bytes, etag))
}

// OneDrive: the app folder, `/me/drive/special/approot:/<path>`, or a folder chosen in the
// drive (D-110), `/me/drive/root:/<folder>/<path>` (its parents are made when the file is).

fn od_item(api: &str, folder: Option<&DriveFolder>, path: &str) -> String {
    match folder {
        Some(f) if f.id.trim_matches('/').is_empty() => format!(
            "{}/me/drive/root:/{}",
            api.trim_end_matches('/'),
            path_enc(path)
        ),
        Some(f) => format!(
            "{}/me/drive/root:/{}/{}",
            api.trim_end_matches('/'),
            path_enc(&f.id),
            path_enc(path)
        ),
        None => format!(
            "{}/me/drive/special/approot:/{}",
            api.trim_end_matches('/'),
            path_enc(path)
        ),
    }
}

/// The folders in a folder of the drive (`path` from the drive's top; empty: the top), by name
/// (D-110, mockup 49k).
pub fn onedrive_folders(api: &str, path: &str, token: &str) -> Result<Vec<String>, RemoteError> {
    let c = Cloud::Microsoft;
    let api = api.trim_end_matches('/');
    let base = if path.trim_matches('/').is_empty() {
        format!("{api}/me/drive/root/children")
    } else {
        format!("{api}/me/drive/root:/{}:/children", path_enc(path))
    };
    let mut url = format!("{base}?$select=name,folder&$top=200");
    let mut out = vec![];
    // A few pages at most: a folder's own folders, not the whole drive.
    for _ in 0..10 {
        let (st, body, _) = call(c, "GET", &url, token, &[], None)?;
        match st {
            200 => {}
            404 => return Ok(out),
            _ => return Err(status_error(c, st, &body)),
        }
        let v: serde_json::Value =
            serde_json::from_slice(&body).map_err(|e| RemoteError::Other(e.to_string()))?;
        for it in v["value"].as_array().into_iter().flatten() {
            if it.get("folder").is_some_and(|f| !f.is_null())
                && let Some(n) = it["name"].as_str()
            {
                out.push(n.to_owned());
            }
        }
        match v["@odata.nextLink"].as_str() {
            Some(next) => url = next.to_owned(),
            None => break,
        }
    }
    out.sort_by_key(|n| cleis_core::text::fold(n));
    Ok(out)
}

pub fn onedrive_read(
    api: &str,
    folder: Option<&DriveFolder>,
    path: &str,
    token: &str,
) -> Result<Option<(String, Vec<u8>)>, RemoteError> {
    let c = Cloud::Microsoft;
    let (st, body, _) = call(c, "GET", &od_item(api, folder, path), token, &[], None)?;
    match st {
        404 => return Ok(None),
        200 => {}
        _ => return Err(status_error(c, st, &body)),
    }
    let meta: serde_json::Value =
        serde_json::from_slice(&body).map_err(|e| RemoteError::Other(e.to_string()))?;
    let etag = meta["eTag"].as_str().unwrap_or_default().to_owned();
    // The download address is signed: no token sent to it.
    let (st, bytes) = match meta["@microsoft.graph.downloadUrl"].as_str() {
        Some(u) => {
            checked(u)?;
            let mut r = agent()
                .get(u)
                .call()
                .map_err(|e| RemoteError::Other(format!("OneDrive: {e}")))?;
            let st = r.status().as_u16();
            (
                st,
                r.body_mut()
                    .with_config()
                    .limit(512 * 1024 * 1024)
                    .read_to_vec()
                    .unwrap_or_default(),
            )
        }
        None => {
            let (st, b, _) = call(
                c,
                "GET",
                &format!("{}:/content", od_item(api, folder, path)),
                token,
                &[],
                None,
            )?;
            (st, b)
        }
    };
    if st != 200 {
        return Err(status_error(c, st, &bytes));
    }
    Ok(Some((etag, bytes)))
}

pub fn onedrive_write(
    api: &str,
    folder: Option<&DriveFolder>,
    path: &str,
    token: &str,
    bytes: &[u8],
    version: Version,
) -> Result<(), RemoteError> {
    let c = Cloud::Microsoft;
    // Sent only over what was read: a copy written meanwhile is merged first.
    let cond = match &version {
        Some(etag) => ("If-Match", etag.as_str()),
        None => ("If-None-Match", "*"),
    };
    let (st, body, _) = call(
        c,
        "PUT",
        &format!("{}:/content", od_item(api, folder, path)),
        token,
        &[("Content-Type", "application/octet-stream"), cond],
        Some(bytes),
    )?;
    match st {
        200 | 201 => Ok(()),
        _ => Err(status_error(c, st, &body)),
    }
}

pub fn onedrive_delete(
    api: &str,
    folder: Option<&DriveFolder>,
    path: &str,
    token: &str,
) -> Result<(), RemoteError> {
    let c = Cloud::Microsoft;
    let (st, body, _) = call(c, "DELETE", &od_item(api, folder, path), token, &[], None)?;
    match st {
        200 | 204 | 404 => Ok(()),
        _ => Err(status_error(c, st, &body)),
    }
}

pub fn onedrive_check(
    api: &str,
    folder: Option<&DriveFolder>,
    token: &str,
) -> Result<(), RemoteError> {
    let c = Cloud::Microsoft;
    let url = match folder {
        Some(_) => format!("{}/me/drive/root", api.trim_end_matches('/')),
        None => format!("{}/me/drive/special/approot", api.trim_end_matches('/')),
    };
    let (st, body, _) = call(c, "GET", &url, token, &[], None)?;
    if st == 200 {
        Ok(())
    } else {
        Err(status_error(c, st, &body))
    }
}

// Google Drive: the hidden app data folder, or a folder picked in Google's window (D-110,
// `drive.file`: Cleis sees only that folder and the files it made); files found by name.

fn gd_quote(v: &str) -> String {
    v.replace('\\', "\\\\").replace('\'', "\\'")
}

fn gd_find(
    api: &str,
    folder: Option<&DriveFolder>,
    name: &str,
    token: &str,
) -> Result<Option<(String, String)>, RemoteError> {
    let c = Cloud::Google;
    let mut q = format!("name = '{}' and trashed = false", gd_quote(name));
    let space = match folder {
        Some(f) => {
            q.push_str(&format!(" and '{}' in parents", gd_quote(&f.id)));
            "drive"
        }
        None => "appDataFolder",
    };
    let url = format!(
        "{}/drive/v3/files?spaces={space}&fields={}&q={}",
        api.trim_end_matches('/'),
        pct("files(id,version)"),
        pct(&q)
    );
    let (st, body, _) = call(c, "GET", &url, token, &[], None)?;
    if st != 200 {
        return Err(status_error(c, st, &body));
    }
    let v: serde_json::Value =
        serde_json::from_slice(&body).map_err(|e| RemoteError::Other(e.to_string()))?;
    Ok(v["files"].as_array().and_then(|f| f.first()).map(|f| {
        (
            f["id"].as_str().unwrap_or_default().to_owned(),
            f["version"].as_str().unwrap_or_default().to_owned(),
        )
    }))
}

pub fn gdrive_read(
    api: &str,
    folder: Option<&DriveFolder>,
    name: &str,
    token: &str,
) -> Result<Option<(String, Vec<u8>)>, RemoteError> {
    let c = Cloud::Google;
    let Some((id, version)) = gd_find(api, folder, name, token)? else {
        return Ok(None);
    };
    let url = format!(
        "{}/drive/v3/files/{}?alt=media",
        api.trim_end_matches('/'),
        pct(&id)
    );
    let (st, bytes, _) = call(c, "GET", &url, token, &[], None)?;
    if st != 200 {
        return Err(status_error(c, st, &bytes));
    }
    Ok(Some((format!("{id}:{version}"), bytes)))
}

pub fn gdrive_write(
    api: &str,
    folder: Option<&DriveFolder>,
    name: &str,
    token: &str,
    bytes: &[u8],
    version: Version,
) -> Result<(), RemoteError> {
    let c = Cloud::Google;
    let api = api.trim_end_matches('/');
    let found = gd_find(api, folder, name, token)?;
    match (version, found) {
        // Drive has no conditional upload: the file's version is compared just before.
        (Some(read), Some((id, now))) => {
            if read != format!("{id}:{now}") {
                return Err(RemoteError::Conflict);
            }
            let url = format!("{api}/upload/drive/v3/files/{}?uploadType=media", pct(&id));
            let (st, body, _) = call(
                c,
                "PATCH",
                &url,
                token,
                &[("Content-Type", "application/octet-stream")],
                Some(bytes),
            )?;
            if st == 200 {
                Ok(())
            } else {
                Err(status_error(c, st, &body))
            }
        }
        (None, None) => {
            let boundary = format!("cleis{}", cleis_core::now());
            let parent = folder.map_or("appDataFolder", |f| f.id.as_str());
            let meta = serde_json::json!({ "name": name, "parents": [parent] });
            let mut body = format!(
                "--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{meta}\r\n--{boundary}\r\nContent-Type: application/octet-stream\r\n\r\n"
            )
            .into_bytes();
            body.extend_from_slice(bytes);
            body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
            let url = format!("{api}/upload/drive/v3/files?uploadType=multipart");
            let ct = format!("multipart/related; boundary={boundary}");
            let (st, rb, _) = call(
                c,
                "POST",
                &url,
                token,
                &[("Content-Type", &ct)],
                Some(&body),
            )?;
            if st == 200 {
                Ok(())
            } else {
                Err(status_error(c, st, &rb))
            }
        }
        // Made or removed by another computer since it was read.
        _ => Err(RemoteError::Conflict),
    }
}

pub fn gdrive_delete(
    api: &str,
    folder: Option<&DriveFolder>,
    name: &str,
    token: &str,
) -> Result<(), RemoteError> {
    let c = Cloud::Google;
    let Some((id, _)) = gd_find(api, folder, name, token)? else {
        return Ok(());
    };
    let url = format!("{}/drive/v3/files/{}", api.trim_end_matches('/'), pct(&id));
    let (st, body, _) = call(c, "DELETE", &url, token, &[], None)?;
    match st {
        200 | 204 | 404 => Ok(()),
        _ => Err(status_error(c, st, &body)),
    }
}

pub fn gdrive_check(
    api: &str,
    folder: Option<&DriveFolder>,
    token: &str,
) -> Result<(), RemoteError> {
    gd_find(api, folder, "vault.cleis", token).map(|_| ())
}

/// The folder picked in Google's window (D-110): its name, and the account's address (the
/// `drive.file` sign-in carries no ID token).
pub fn gdrive_picked(
    api: &str,
    id: &str,
    token: &str,
) -> Result<(DriveFolder, String), RemoteError> {
    let c = Cloud::Google;
    let api = api.trim_end_matches('/');
    let url = format!(
        "{api}/drive/v3/files/{}?fields={}",
        pct(id),
        pct("id,name,mimeType")
    );
    let (st, body, _) = call(c, "GET", &url, token, &[], None)?;
    if st != 200 {
        return Err(status_error(c, st, &body));
    }
    let v: serde_json::Value =
        serde_json::from_slice(&body).map_err(|e| RemoteError::Other(e.to_string()))?;
    if v["mimeType"].as_str() != Some(FOLDER_MIME) {
        return Err(RemoteError::Other("Google Drive: choose a folder".into()));
    }
    let folder = DriveFolder {
        id: id.to_owned(),
        name: v["name"].as_str().unwrap_or("Google Drive").to_owned(),
    };
    let url = format!("{api}/drive/v3/about?fields={}", pct("user(emailAddress)"));
    let (st, body, _) = call(c, "GET", &url, token, &[], None)?;
    let account = if st == 200 {
        serde_json::from_slice::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v["user"]["emailAddress"].as_str().map(str::to_owned))
            .unwrap_or_default()
    } else {
        String::new()
    };
    Ok((folder, account))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorization_addresses() {
        let e = Endpoints::stand_in(Cloud::Google, "http://127.0.0.1:9");
        let p = prepare(Cloud::Google, &e, "lea@example-studio.com", false).unwrap();
        assert!(
            p.url
                .starts_with("http://127.0.0.1:9/authorize?response_type=code")
        );
        assert!(p.url.contains(
            "scope=openid%20email%20https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fdrive.appdata"
        ));
        assert!(p.url.contains("access_type=offline"));
        assert!(p.url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A"));
        assert!(p.url.contains("code_challenge_method=S256"));
        let m = Endpoints::official(Cloud::Microsoft);
        assert!(m.auth.contains("/common/"));
        let p = prepare(Cloud::Microsoft, &m, "", false).unwrap();
        assert!(p.url.contains("Files.ReadWrite.AppFolder"));
        assert!(p.url.contains("offline_access"));
        assert!(!p.url.contains("login_hint"));
        assert!(!p.url.contains("trigger_onepick"));
        // D-110, "Custom folder": the whole OneDrive; Google's picker with drive.file alone.
        let p = prepare(Cloud::Microsoft, &m, "", true).unwrap();
        assert!(p.url.contains("Files.ReadWrite&"));
        assert!(!p.url.contains("AppFolder"));
        let p = prepare(Cloud::Google, &e, "", true).unwrap();
        assert!(
            p.url
                .contains("scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fdrive.file&")
        );
        assert!(!p.url.contains("openid"));
        assert!(p.url.contains("trigger_onepick=true"));
        assert!(p.url.contains("allow_folder_selection=true"));
        assert!(
            p.url
                .contains("mimetypes=application%2Fvnd.google-apps.folder")
        );
        // A place keeps its API; the official one gives back the official addresses.
        assert_eq!(
            Endpoints::of_api(Cloud::Microsoft, MICROSOFT_API),
            Endpoints::official(Cloud::Microsoft)
        );
        assert_eq!(
            Endpoints::of_api(Cloud::Google, "http://127.0.0.1:9/").token,
            "http://127.0.0.1:9/token"
        );
        assert!(checked("http://example-idp.com/token").is_err());
    }

    #[test]
    fn grants() {
        let id = format!(
            "x.{}.y",
            B64.encode(br#"{"email":"lea@example-studio.com"}"#)
        );
        let g = grant_of(
            &serde_json::json!({"access_token":"a","refresh_token":"r","expires_in":3599,"id_token":id}),
            "",
        )
        .unwrap();
        assert_eq!(g.account, "lea@example-studio.com");
        assert_eq!(g.refresh, "r");
        assert!(g.expires > cleis_core::now() + 3500);
        // Google keeps the refresh token it gave first.
        let g = grant_of(&serde_json::json!({"access_token":"b"}), "r").unwrap();
        assert_eq!(g.refresh, "r");
        let e = grant_of(&serde_json::json!({"error":"invalid_grant"}), "r").unwrap_err();
        assert!(e.contains("sign in again"));
        assert_eq!(path_enc("/a b/vault.cleis"), "a%20b/vault.cleis");
    }

    #[test]
    fn chosen_folders() {
        let f = DriveFolder {
            id: "Documents/Admin files".into(),
            name: "Documents / Admin files".into(),
        };
        assert_eq!(
            od_item("https://g/v1.0", Some(&f), "vault.cleis"),
            "https://g/v1.0/me/drive/root:/Documents/Admin%20files/vault.cleis"
        );
        let top = DriveFolder {
            id: String::new(),
            name: "OneDrive".into(),
        };
        assert_eq!(
            od_item("https://g/v1.0", Some(&top), "vault.cleis"),
            "https://g/v1.0/me/drive/root:/vault.cleis"
        );
        assert_eq!(
            od_item("https://g/v1.0", None, "vault.cleis"),
            "https://g/v1.0/me/drive/special/approot:/vault.cleis"
        );
    }
}
