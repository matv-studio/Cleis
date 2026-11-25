//! Sign-in with Google or Microsoft from the desktop (D-64): OpenID Connect authorization code
//! with PKCE and a loopback redirect (RFC 8252, RFC 7636). The browser shows the provider's own
//! page; Cleis only receives the code on `http://127.0.0.1:<port>` and exchanges it for an ID
//! token, which the organization's server checks.

use cleis_core::org::Provider;
use data_encoding::BASE64URL_NOPAD as B64;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

/// A sign-in waiting for the browser.
pub struct Pending {
    pub url: String,
    listener: TcpListener,
    redirect: String,
    verifier: String,
    state: String,
    nonce: String,
    token_endpoint: String,
    client_id: String,
    client_secret: String,
}

/// `n` (at most 32) random bytes, base64url.
fn random_b64(n: usize) -> Result<String, String> {
    let r: [u8; 32] = cleis_crypto::random_bytes().map_err(|e| e.to_string())?;
    Ok(B64.encode(&r[..n.min(32)]))
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

fn unpct(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() => match u8::from_str_radix(&s[i + 1..i + 3], 16) {
                Ok(v) => {
                    out.push(v);
                    i += 3;
                    continue;
                }
                Err(_) => out.push(b'%'),
            },
            b'+' => out.push(b' '),
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .new_agent()
}

/// Reads the provider's endpoints and opens the loopback listener (network: call off the
/// interface thread). `hint`: the member's e-mail, pre-filled on the provider's page.
pub fn start(p: &Provider, hint: &str) -> Result<Pending, String> {
    let disc: serde_json::Value = agent()
        .get(&format!(
            "{}/.well-known/openid-configuration",
            p.issuer.trim_end_matches('/')
        ))
        .call()
        .map_err(|e| format!("{}: {e}", p.issuer))?
        .body_mut()
        .read_json()
        .map_err(|e| e.to_string())?;
    let auth = disc["authorization_endpoint"]
        .as_str()
        .ok_or("no authorization endpoint")?;
    let token = disc["token_endpoint"].as_str().ok_or("no token endpoint")?;
    prepare(p, auth, token, hint)
}

/// The listener and the authorization address (no network).
pub fn prepare(p: &Provider, auth: &str, token: &str, hint: &str) -> Result<Pending, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect = format!("http://127.0.0.1:{port}");
    let verifier = random_b64(32)?;
    let challenge = B64.encode(&cleis_crypto::sha256(verifier.as_bytes()));
    let state = random_b64(16)?;
    let nonce = random_b64(16)?;
    let mut url = format!(
        "{auth}{}response_type=code&client_id={}&redirect_uri={}&scope={}&code_challenge={challenge}&code_challenge_method=S256&state={state}&nonce={nonce}&prompt=select_account",
        if auth.contains('?') { '&' } else { '?' },
        pct(&p.client_id),
        pct(&redirect),
        pct("openid email profile"),
    );
    if !hint.is_empty() {
        url.push_str(&format!("&login_hint={}", pct(hint)));
    }
    Ok(Pending {
        url,
        listener,
        redirect,
        verifier,
        state,
        nonce,
        token_endpoint: token.to_owned(),
        client_id: p.client_id.clone(),
        client_secret: p.client_secret.clone(),
    })
}

/// `code` from the browser's request line, once `state` matches.
pub fn parse_callback(line: &str, state: &str) -> Result<String, String> {
    let target = line.split_whitespace().nth(1).ok_or("bad request")?;
    let query = target.split_once('?').map(|(_, q)| q).unwrap_or("");
    let mut code = None;
    let mut got_state = None;
    let mut error = None;
    for kv in query.split('&') {
        let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
        match k {
            "code" => code = Some(unpct(v)),
            "state" => got_state = Some(unpct(v)),
            "error" => error = Some(unpct(v)),
            "error_description" => error = Some(unpct(v)),
            _ => {}
        }
    }
    if let Some(e) = error {
        return Err(e);
    }
    if got_state.as_deref() != Some(state) {
        return Err("the sign-in did not come from this request".into());
    }
    code.ok_or_else(|| "no code".into())
}

/// Waits for the browser (5 minutes at most), then gets the ID token.
pub fn finish(p: Pending) -> Result<String, String> {
    p.listener
        .set_nonblocking(true)
        .map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(300);
    let code = loop {
        match p.listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false).ok();
                stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
                let mut reader = BufReader::new(&stream);
                let mut line = String::new();
                if reader.read_line(&mut line).is_err() || !line.starts_with("GET /") {
                    continue;
                }
                // Favicon and other requests of the browser.
                if !line.contains("state=") {
                    let _ = (&stream).write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    );
                    continue;
                }
                let r = parse_callback(&line, &p.state);
                let page = match &r {
                    Ok(_) => "Signed in. You can close this tab and go back to Cleis.",
                    Err(_) => "Sign-in refused. Go back to Cleis to try again.",
                };
                let body = format!(
                    "<!doctype html><meta charset=utf-8><title>Cleis</title><body style=\"font-family:system-ui;background:#1b1b1b;color:#eee;display:grid;place-items:center;height:90vh\"><p>{page}</p>"
                );
                let _ = (&stream).write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                );
                break r?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() > deadline {
                    return Err("No answer from the browser".into());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(e.to_string()),
        }
    };
    let mut form = format!(
        "grant_type=authorization_code&code={}&redirect_uri={}&client_id={}&code_verifier={}",
        pct(&code),
        pct(&p.redirect),
        pct(&p.client_id),
        pct(&p.verifier)
    );
    if !p.client_secret.is_empty() {
        form.push_str(&format!("&client_secret={}", pct(&p.client_secret)));
    }
    let reply: serde_json::Value = agent()
        .post(&p.token_endpoint)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send(form.as_bytes())
        .map_err(|e| format!("token: {e}"))?
        .body_mut()
        .read_json()
        .map_err(|e| e.to_string())?;
    let token = reply["id_token"]
        .as_str()
        .ok_or("the provider gave no ID token")?
        .to_owned();
    // The token answers this request (the server checks its signature).
    let payload = token
        .split('.')
        .nth(1)
        .and_then(|b| B64.decode(b.as_bytes()).ok())
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .ok_or("unreadable ID token")?;
    if payload["nonce"].as_str() != Some(p.nonce.as_str()) {
        return Err("the ID token does not answer this sign-in".into());
    }
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cleis_core::org::Method;

    #[test]
    fn authorization_address_and_callback() {
        let p = Provider {
            method: Method::Google,
            issuer: "https://idp.example-idp.com".into(),
            client_id: "cleis desktop".into(),
            client_secret: String::new(),
        };
        let pending = prepare(
            &p,
            "https://idp.example-idp.com/auth",
            "https://idp.example-idp.com/token",
            "lea@example-studio.com",
        )
        .unwrap();
        let u = &pending.url;
        assert!(u.starts_with("https://idp.example-idp.com/auth?response_type=code"));
        assert!(u.contains("client_id=cleis%20desktop"));
        assert!(u.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A"));
        assert!(u.contains("code_challenge_method=S256"));
        assert!(u.contains("login_hint=lea%40example-studio.com"));
        let st = pending.state.clone();
        assert_eq!(
            parse_callback(&format!("GET /?state={st}&code=4%2F0Ab HTTP/1.1"), &st).unwrap(),
            "4/0Ab"
        );
        assert!(parse_callback("GET /?state=other&code=x HTTP/1.1", &st).is_err());
        assert_eq!(
            parse_callback("GET /?error=access_denied HTTP/1.1", &st).unwrap_err(),
            "access_denied"
        );
    }
}
