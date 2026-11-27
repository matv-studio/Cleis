//! D-104: the vault kept in a Microsoft (OneDrive app folder) or Google (Drive app data folder)
//! account, against a stand-in of both: sign-in through the browser (simulated on the loopback
//! address), the vault sent, found on a new computer, made when the account has none, tokens
//! renewed, access revoked.

use cleis_app::app::{CleisApp, Phase};
use cleis_app::theme;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AppContext, Bounds, Entity, Point, TestAppContext, WindowBounds, WindowHandle, WindowOptions,
    base::Root, px, size,
};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

fn open(cx: &mut TestAppContext, path: &Path) -> (WindowHandle<Root>, Entity<CleisApp>) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        cx.set_reduce_motion(true);
        theme::apply(&theme::Appearance::default(), cx);
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: Point::default(),
                size: size(px(1440.), px(900.)),
            })),
            ..Default::default()
        };
        let path = path.to_owned();
        let (w, app) = gpui_kit::open_window(opts, cx, move |window, cx| {
            cx.new(|cx| {
                let mut a = CleisApp::new(path, None, window, cx);
                a.no_shortcuts = true;
                a.autofill.dry_run = Some(vec![]);
                a
            })
        })
        .expect("window");
        (w.downcast::<Root>().expect("root"), app)
    })
}

fn act(
    cx: &mut TestAppContext,
    h: WindowHandle<Root>,
    f: impl FnOnce(&mut gpui_kit::Window, &mut gpui_kit::App),
) {
    cx.update_window(h.into(), |_, w, cx| f(w, cx)).unwrap();
    cx.run_until_parked();
}

fn window_has(cx: &mut TestAppContext, h: WindowHandle<Root>, id: &str) -> bool {
    let id = gpui_kit::SharedString::from(id.to_owned());
    cx.update_window(h.into(), move |_, w, _| w.try_find(id).is_some())
        .unwrap()
}

/// Waits (real time: the network runs on threads) until `f` holds.
fn until(cx: &mut TestAppContext, what: &str, mut f: impl FnMut(&mut TestAppContext) -> bool) {
    for _ in 0..200 {
        cx.run_until_parked();
        if f(cx) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    panic!("timed out: {what}");
}

/// The stand-in's state: files of both providers, the valid token, what was asked.
#[derive(Default)]
struct Cloud {
    /// Name → (version, bytes); one store per provider.
    onedrive: Mutex<HashMap<String, (u32, Vec<u8>)>>,
    gdrive: Mutex<HashMap<String, (u32, Vec<u8>)>>,
    /// The access token the API accepts now.
    token: Mutex<String>,
    tokens: AtomicU32,
    refreshes: AtomicU32,
    revoked: AtomicU32,
    /// Tokens that end at once (`expires_in: 1`).
    expiring: AtomicBool,
    /// The account named in the ID token.
    email: Mutex<String>,
}

fn id_token(email: &str) -> String {
    use data_encoding::BASE64URL_NOPAD as B;
    format!(
        "{}.{}.sig",
        B.encode(br#"{"alg":"RS256"}"#),
        B.encode(format!(r#"{{"email":"{email}"}}"#).as_bytes())
    )
}

fn unpct(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(if b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn reply(stream: &mut std::net::TcpStream, status: u16, headers: &[(&str, String)], body: &[u8]) {
    let mut head = format!(
        "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    for (k, v) in headers {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str("\r\n");
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
}

fn fake_cloud() -> (String, Arc<Cloud>) {
    let state = Arc::new(Cloud::default());
    *state.email.lock().unwrap() = "lea@example-studio.com".into();
    let st = state.clone();
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    let b2 = base.clone();
    std::thread::spawn(move || {
        for stream in l.incoming() {
            let Ok(mut stream) = stream else { continue };
            let st = st.clone();
            let base = b2.clone();
            std::thread::spawn(move || {
                let mut r = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                if r.read_line(&mut line).is_err() {
                    return;
                }
                let mut parts = line.split_whitespace();
                let method = parts.next().unwrap_or("").to_owned();
                let target = parts.next().unwrap_or("").to_owned();
                let mut headers: HashMap<String, String> = HashMap::new();
                loop {
                    let mut h = String::new();
                    if r.read_line(&mut h).is_err() || h == "\r\n" || h.is_empty() {
                        break;
                    }
                    if let Some((k, v)) = h.split_once(':') {
                        headers.insert(k.trim().to_lowercase(), v.trim().to_owned());
                    }
                }
                let len: usize = headers
                    .get("content-length")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                let mut body = vec![0u8; len];
                let _ = r.read_exact(&mut body);
                let (path, query) = target.split_once('?').unwrap_or((&target, ""));
                let q: HashMap<String, String> = query
                    .split('&')
                    .filter_map(|kv| kv.split_once('='))
                    .map(|(k, v)| (k.to_owned(), unpct(v)))
                    .collect();
                let json = |v: serde_json::Value| v.to_string().into_bytes();
                // The token endpoint.
                if method == "POST" && path == "/token" {
                    let form: HashMap<String, String> = String::from_utf8_lossy(&body)
                        .split('&')
                        .filter_map(|kv| kv.split_once('='))
                        .map(|(k, v)| (k.to_owned(), unpct(v)))
                        .collect();
                    let n = st.tokens.fetch_add(1, Ordering::SeqCst) + 1;
                    match form.get("grant_type").map(String::as_str) {
                        Some("authorization_code") => {
                            assert!(form.contains_key("code_verifier"), "PKCE");
                        }
                        Some("refresh_token") => {
                            st.refreshes.fetch_add(1, Ordering::SeqCst);
                        }
                        _ => {}
                    }
                    let token = format!("at{n}");
                    *st.token.lock().unwrap() = token.clone();
                    let expires = if st.expiring.load(Ordering::SeqCst) {
                        1
                    } else {
                        3600
                    };
                    let email = st.email.lock().unwrap().clone();
                    let v = serde_json::json!({
                        "access_token": token,
                        "refresh_token": format!("rt{n}"),
                        "expires_in": expires,
                        "id_token": id_token(&email),
                    });
                    reply(&mut stream, 200, &[], &json(v));
                    return;
                }
                if method == "POST" && path == "/revoke" {
                    st.revoked.fetch_add(1, Ordering::SeqCst);
                    reply(&mut stream, 200, &[], b"");
                    return;
                }
                // The download address OneDrive gives is signed: no token.
                if let Some(name) = path.strip_prefix("/dl/") {
                    let name = unpct(name);
                    match st.onedrive.lock().unwrap().get(&name) {
                        Some((_, b)) => reply(&mut stream, 200, &[], b),
                        None => reply(&mut stream, 404, &[], b""),
                    }
                    return;
                }
                let ok = headers
                    .get("authorization")
                    .is_some_and(|a| *a == format!("Bearer {}", st.token.lock().unwrap()));
                if !ok {
                    reply(&mut stream, 401, &[], b"{}");
                    return;
                }
                // OneDrive's app folder.
                if path == "/me/drive/special/approot" {
                    reply(&mut stream, 200, &[], b"{}");
                    return;
                }
                if let Some(rest) = path.strip_prefix("/me/drive/special/approot:/") {
                    let (name, content) = match rest.strip_suffix(":/content") {
                        Some(n) => (unpct(n), true),
                        None => (unpct(rest), false),
                    };
                    let mut files = st.onedrive.lock().unwrap();
                    match (method.as_str(), content) {
                        ("GET", false) => match files.get(&name) {
                            Some((v, _)) => {
                                let meta = serde_json::json!({
                                    "eTag": format!("\"e{v}\""),
                                    "@microsoft.graph.downloadUrl": format!("{base}/dl/{name}"),
                                });
                                reply(&mut stream, 200, &[], &json(meta));
                            }
                            None => reply(&mut stream, 404, &[], b"{}"),
                        },
                        ("PUT", true) => {
                            let now = files.get(&name).map(|(v, _)| format!("\"e{v}\""));
                            let fine = match (headers.get("if-match"), &now) {
                                (Some(m), Some(n)) => m == n,
                                (Some(_), None) => false,
                                (None, None) => headers.contains_key("if-none-match"),
                                (None, Some(_)) => false,
                            };
                            if !fine {
                                reply(&mut stream, 412, &[], b"{}");
                                return;
                            }
                            let v = files.get(&name).map_or(1, |(v, _)| v + 1);
                            files.insert(name, (v, body));
                            reply(&mut stream, 201, &[], b"{}");
                        }
                        ("DELETE", false) => {
                            files.remove(&name);
                            reply(&mut stream, 204, &[], b"");
                        }
                        _ => reply(&mut stream, 400, &[], b"{}"),
                    }
                    return;
                }
                // Google Drive's app data folder (files by id = their name here).
                if path == "/drive/v3/files" && method == "GET" {
                    assert_eq!(q.get("spaces").map(String::as_str), Some("appDataFolder"));
                    let want = q.get("q").cloned().unwrap_or_default();
                    let name = want.split('\'').nth(1).unwrap_or_default().to_owned();
                    let files = st.gdrive.lock().unwrap();
                    let list: Vec<_> = files
                        .get(&name)
                        .map(|(v, _)| serde_json::json!({"id": name, "version": v.to_string()}))
                        .into_iter()
                        .collect();
                    reply(
                        &mut stream,
                        200,
                        &[],
                        &json(serde_json::json!({ "files": list })),
                    );
                    return;
                }
                if let Some(id) = path.strip_prefix("/drive/v3/files/") {
                    let id = unpct(id);
                    let mut files = st.gdrive.lock().unwrap();
                    match method.as_str() {
                        "GET" => match files.get(&id) {
                            Some((_, b)) => reply(&mut stream, 200, &[], b),
                            None => reply(&mut stream, 404, &[], b"{}"),
                        },
                        "DELETE" => {
                            files.remove(&id);
                            reply(&mut stream, 204, &[], b"");
                        }
                        _ => reply(&mut stream, 400, &[], b"{}"),
                    }
                    return;
                }
                if path == "/upload/drive/v3/files" && method == "POST" {
                    // multipart/related: the metadata, then the file.
                    let ct = headers.get("content-type").cloned().unwrap_or_default();
                    let boundary = ct.split("boundary=").nth(1).unwrap_or_default().to_owned();
                    let sep = format!("--{boundary}");
                    let text = body.clone();
                    let s = String::from_utf8_lossy(&text);
                    let meta_start = s.find("{").unwrap();
                    let meta_end = s[meta_start..].find("}\r\n").unwrap() + meta_start + 1;
                    let meta: serde_json::Value =
                        serde_json::from_str(&s[meta_start..meta_end]).unwrap();
                    assert_eq!(meta["parents"][0], "appDataFolder");
                    let name = meta["name"].as_str().unwrap().to_owned();
                    // The file: after the second part's blank line, up to the last separator.
                    let second = find(&text, sep.as_bytes(), meta_end).unwrap();
                    let start = find(&text, b"\r\n\r\n", second).unwrap() + 4;
                    let end = rfind(&text, format!("\r\n{sep}--").as_bytes()).unwrap();
                    let mut files = st.gdrive.lock().unwrap();
                    files.insert(name, (1, text[start..end].to_vec()));
                    reply(&mut stream, 200, &[], b"{}");
                    return;
                }
                if let Some(id) = path.strip_prefix("/upload/drive/v3/files/") {
                    let id = unpct(id);
                    let mut files = st.gdrive.lock().unwrap();
                    let v = files.get(&id).map_or(1, |(v, _)| v + 1);
                    files.insert(id, (v, body));
                    reply(&mut stream, 200, &[], b"{}");
                    return;
                }
                reply(&mut stream, 404, &[], b"{}");
            });
        }
    });
    (base, state)
}

fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    hay[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

fn rfind(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).rposition(|w| w == needle)
}

/// Clicks `id`, which opens the provider's page, and the browser comes back with the code. The
/// browser answers before the test executor runs the wait (it would run it to its end).
fn sign_in_with(
    cx: &mut TestAppContext,
    h: WindowHandle<Root>,
    app: &Entity<CleisApp>,
    id: &'static str,
) {
    cx.update_window(h.into(), |_, w, cx| w.click(id, cx))
        .unwrap();
    browser_returns(cx, app);
    std::thread::sleep(std::time::Duration::from_millis(50));
    cx.run_until_parked();
}

/// The browser: it comes back to Cleis' loopback address with the code and the state.
fn browser_returns(cx: &mut TestAppContext, app: &Entity<CleisApp>) {
    let url = app.read_with(cx, |a, _| {
        a.cloud
            .as_ref()
            .and_then(|c| c.url.clone())
            .expect("the provider's page")
    });
    let q: HashMap<String, String> = url
        .split_once('?')
        .unwrap()
        .1
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.to_owned(), unpct(v)))
        .collect();
    let redirect = q["redirect_uri"].clone();
    assert!(redirect.starts_with("http://127.0.0.1:"));
    assert_eq!(q["code_challenge_method"], "S256");
    if q["scope"].contains("drive.appdata") {
        assert_eq!(q["access_type"], "offline");
    } else {
        assert!(
            q["scope"].contains("Files.ReadWrite.AppFolder")
                && q["scope"].contains("offline_access")
        );
    }
    let state = q["state"].clone();
    let addr = redirect.trim_start_matches("http://").to_owned();
    std::thread::spawn(move || {
        let mut s = std::net::TcpStream::connect(addr).unwrap();
        let _ = s.write_all(
            format!("GET /?code=4%2Fabc&state={state} HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes(),
        );
        let mut out = String::new();
        let _ = s.read_to_string(&mut out);
        assert!(out.contains("Signed in"), "{out}");
    });
}

fn unlock(cx: &mut TestAppContext, h: WindowHandle<Root>) {
    act(cx, h, |w, cx| {
        w.click("master-password", cx);
        w.input(cleis_app::demo::PASSWORD, cx);
        w.press("enter", cx);
    });
}

/// Scenario: Settings › Account › Microsoft › Sign in; the browser comes back; the open vault goes
/// to OneDrive's app folder and syncs there. On a new computer, Sign in › Microsoft finds it; the
/// master password opens it there.
#[gpui_kit::test]
fn onedrive_account_and_new_computer(cx: &mut TestAppContext) {
    let (base, st) = fake_cloud();
    let hosts = cleis_app::cloud::CloudHosts::stand_in(&base);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("demo.cleis");
    cleis_app::demo::create(&path).unwrap();
    let (h, app) = open(cx, &path);
    app.update(cx, |a, _| a.cloud_hosts = hosts.clone());
    cx.run_until_parked();
    unlock(cx, h);
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Account", cx));
    // Choosing does not sign in yet; Sign in opens Microsoft's page.
    act(cx, h, |w, cx| w.click("acc-microsoft", cx));
    app.read_with(cx, |a, _| assert!(a.cloud.is_none() && a.github.is_none()));
    sign_in_with(cx, h, &app, "acc-signin");
    until(cx, "the vault sent to OneDrive", |_| {
        st.onedrive.lock().unwrap().contains_key("vault.cleis")
    });
    until(cx, "signed in", |cx| {
        app.read_with(cx, |a, _| {
            let Phase::Main(s) = &a.phase else {
                return false;
            };
            a.cloud.is_none() && cleis_app::screens::account::signed_in(s)
        })
    });
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(s.sync.provider, cleis_app::sync::Provider::OneDrive);
        let r = s.sync.remote.as_ref().unwrap();
        assert_eq!(r.account(), Some("lea@example-studio.com"));
        // The tokens stay sealed in the vault, never in the settings file beside it.
        let conf = std::fs::read_to_string(cleis_app::sync::config_path(&path)).unwrap();
        assert!(!conf.contains("at1") && !conf.contains("rt1"));
        assert_eq!(s.sync.secret(&s.vault).refresh, "rt1");
    });
    assert!(window_has(cx, h, "acc-revoke"));

    // A new computer: Sign in › Microsoft, the vault of the account comes; its password opens it.
    let (h2, app2) = open(cx, &dir.path().join("new/vault.cleis"));
    app2.update(cx, |a, _| a.cloud_hosts = hosts.clone());
    cx.run_until_parked();
    sign_in_with(cx, h2, &app2, "signin-microsoft");
    until(cx, "the vault found", |cx| {
        app2.read_with(cx, |a, _| match &a.phase {
            Phase::Welcome(w) => w.restore.as_ref().is_some_and(|r| !r.found.is_empty()),
            _ => false,
        })
    });
    assert!(window_has(cx, h2, "signed-as"));
    act(cx, h2, |w, cx| {
        w.click("restore-password", cx);
        w.input(cleis_app::demo::PASSWORD, cx);
        w.click("restore-go", cx);
    });
    until(cx, "the vault open on the new computer", |cx| {
        app2.read_with(cx, |a, _| matches!(a.phase, Phase::Main(_)))
    });
    app2.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(!s.entries.is_empty());
        assert_eq!(s.sync.provider, cleis_app::sync::Provider::OneDrive);
    });
}

/// Scenario: a new computer signs in with Google and the account has no vault yet: the vault is
/// made here and goes to Drive's app data folder once its recovery key is put away. Tokens that
/// end are renewed with the refresh token; Revoke access takes Cleis' access away on Google.
#[gpui_kit::test]
fn google_drive_new_vault_renewal_and_revoke(cx: &mut TestAppContext) {
    let (base, st) = fake_cloud();
    let hosts = cleis_app::cloud::CloudHosts::stand_in(&base);
    let dir = tempfile::tempdir().unwrap();
    let (h, app) = open(cx, &dir.path().join("new/vault.cleis"));
    app.update(cx, |a, _| a.cloud_hosts = hosts.clone());
    cx.run_until_parked();
    sign_in_with(cx, h, &app, "signin-google");
    until(cx, "no vault on the account: made here", |cx| {
        app.read_with(cx, |a, _| a.cloud_after.is_some())
    });
    app.read_with(cx, |a, _| {
        assert!(a.cloud.is_none());
        let Phase::Welcome(w) = &a.phase else {
            panic!()
        };
        assert_eq!(w.choice, Some(cleis_app::screens::welcome::Start::Account));
    });
    assert!(window_has(cx, h, "signed-as") && window_has(cx, h, "create"));
    act(cx, h, |w, cx| {
        w.click("new-password", cx);
        w.input("correct horse battery staple", cx);
        w.click("confirm-password", cx);
        w.input("correct horse battery staple", cx);
        w.click("create", cx);
    });
    until(cx, "the vault made", |cx| {
        app.read_with(cx, |a, _| matches!(a.phase, Phase::Main(_)))
    });
    // Nothing sent before the recovery key is put away.
    assert!(st.gdrive.lock().unwrap().is_empty());
    act(cx, h, |w, cx| w.click("recovery-key-done", cx));
    until(cx, "the vault sent to Google Drive", |_| {
        st.gdrive.lock().unwrap().contains_key("vault.cleis")
    });
    until(cx, "synced", |cx| {
        app.read_with(cx, |a, _| {
            let Phase::Main(s) = &a.phase else {
                return false;
            };
            !s.sync.busy && s.sync.last_sync.is_some()
        })
    });
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert_eq!(s.sync.provider, cleis_app::sync::Provider::GoogleDrive);
        assert!(cleis_app::screens::account::signed_in(s));
    });

    // The token ends: renewed before the next sync, with the refresh token, and kept.
    st.expiring.store(true, Ordering::SeqCst);
    app.update(cx, |a, _| {
        let Phase::Main(s) = &mut a.phase else {
            panic!()
        };
        let key = s.sync.secret_key();
        let mut sec = s.sync.secret(&s.vault);
        sec.expires = cleis_core::now() - 10;
        let _ = s.vault.set_org_setting(&key, &Some(sec));
    });
    let before = st.gdrive.lock().unwrap()["vault.cleis"].0;
    app.update(cx, |a, cx| a.sync_now(cx));
    until(cx, "renewed and synced", |cx| {
        st.refreshes.load(Ordering::SeqCst) >= 1
            && app.read_with(cx, |a, _| {
                let Phase::Main(s) = &a.phase else {
                    return false;
                };
                !s.sync.busy && s.sync.last_error.is_none()
            })
    });
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        let sec = s.sync.secret(&s.vault);
        assert_eq!(sec.token, *st.token.lock().unwrap(), "the new token kept");
        assert!(!sec.refresh.is_empty());
    });
    assert!(st.gdrive.lock().unwrap()["vault.cleis"].0 >= before);

    // Revoke access: confirmed, Google's grant taken away, this computer signed out.
    act(cx, h, |w, cx| w.click("tab-Settings", cx));
    act(cx, h, |w, cx| w.click("set-Account", cx));
    act(cx, h, |w, cx| w.click("acc-revoke", cx));
    act(cx, h, |w, cx| w.click("confirm-ok", cx));
    until(cx, "revoked", |_| st.revoked.load(Ordering::SeqCst) == 1);
    app.read_with(cx, |a, _| {
        let Phase::Main(s) = &a.phase else { panic!() };
        assert!(!cleis_app::screens::account::signed_in(s));
        assert!(s.sync.remote.is_none());
    });
}

/// Two computers write the same OneDrive file: the second, refused (its copy is older), merges
/// what the first sent and sends again; nothing is lost.
#[gpui_kit::test]
fn onedrive_conflict_merges(cx: &mut TestAppContext) {
    let (base, st) = fake_cloud();
    let api = base.clone();
    let place = cleis_app::remote::RemotePlace::OneDrive {
        path: "vault.cleis".into(),
        api,
        account: "lea@example-studio.com".into(),
    };
    *st.token.lock().unwrap() = "t".into();
    let secret = cleis_app::remote::Secret {
        token: "t".into(),
        ..Default::default()
    };
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.bin");
    std::fs::write(&a, b"first").unwrap();
    // Nothing there yet: the version read is none, and sending makes it.
    let tmp = dir.path().join("read.bin");
    let v = cleis_app::remote::download(&place, &secret, &tmp).unwrap();
    assert!(v.is_none());
    cleis_app::remote::upload(&place, &secret, &a, v.clone()).unwrap();
    // A second write over the same "nothing" is refused: the file appeared meanwhile.
    assert_eq!(
        cleis_app::remote::upload(&place, &secret, &a, v),
        Err(cleis_app::remote::RemoteError::Conflict)
    );
    let v = cleis_app::remote::download(&place, &secret, &tmp).unwrap();
    assert_eq!(std::fs::read(&tmp).unwrap(), b"first");
    std::fs::write(&a, b"second").unwrap();
    cleis_app::remote::upload(&place, &secret, &a, v.clone()).unwrap();
    // The old version is refused now.
    assert_eq!(
        cleis_app::remote::upload(&place, &secret, &a, v),
        Err(cleis_app::remote::RemoteError::Conflict)
    );
    // The account key beside it (D-96), then removed with the vault.
    cleis_app::remote::remove(&place, &secret).unwrap();
    assert!(st.onedrive.lock().unwrap().is_empty());
    // Google: the same, with versions compared before sending.
    let g = cleis_app::remote::RemotePlace::GoogleDrive {
        name: "vault.cleis".into(),
        api: base,
        account: String::new(),
    };
    let v = cleis_app::remote::download(&g, &secret, &tmp).unwrap();
    assert!(v.is_none());
    cleis_app::remote::upload(&g, &secret, &a, v.clone()).unwrap();
    assert_eq!(
        cleis_app::remote::upload(&g, &secret, &a, v),
        Err(cleis_app::remote::RemoteError::Conflict)
    );
    let v = cleis_app::remote::download(&g, &secret, &tmp).unwrap();
    assert_eq!(std::fs::read(&tmp).unwrap(), b"second");
    std::fs::write(&a, b"third").unwrap();
    cleis_app::remote::upload(&g, &secret, &a, v.clone()).unwrap();
    assert_eq!(
        cleis_app::remote::upload(&g, &secret, &a, v),
        Err(cleis_app::remote::RemoteError::Conflict)
    );
    assert_eq!(st.gdrive.lock().unwrap()["vault.cleis"].1, b"third");
    // A wrong token: said plainly.
    let bad = cleis_app::remote::Secret {
        token: "nope".into(),
        ..Default::default()
    };
    let e = cleis_app::remote::check(&g, &bad).unwrap_err();
    assert!(e.to_string().contains("sign in again"), "{e}");
    let _ = cx;
}
