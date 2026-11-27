//! Sign-in with Google or Microsoft (OpenID Connect, D-64): the app obtains an ID token in the
//! member's browser; the server checks it here — signature with the provider's published keys,
//! issuer, audience, expiry, verified e-mail and its domain — and binds or matches the account.

use crate::config::Oidc;
use cleis_core::org::Method;
use data_encoding::BASE64URL_NOPAD as B64;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Claims Cleis reads from an ID token.
#[derive(Clone, Debug, Deserialize)]
pub struct Claims {
    pub iss: String,
    pub sub: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub email_verified: Option<serde_json::Value>,
    /// Microsoft: the account's sign-in name when `email` is absent.
    #[serde(default)]
    pub preferred_username: String,
    pub exp: i64,
    /// Issued at (audit S-15): a token minted long ago is refused even if it has not expired.
    #[serde(default)]
    pub iat: Option<i64>,
    /// Authorized party: when present, the application the token was issued to.
    #[serde(default)]
    pub azp: Option<String>,
    #[serde(default)]
    pub nonce: String,
}

impl Claims {
    /// The account as stored for a member: issuer and subject.
    pub fn account(&self) -> String {
        format!("{}|{}", self.iss, self.sub)
    }

    pub fn address(&self) -> &str {
        if self.email.is_empty() {
            &self.preferred_username
        } else {
            &self.email
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Jwk {
    #[serde(default)]
    pub kid: String,
    pub kty: String,
    #[serde(default)]
    pub n: String,
    #[serde(default)]
    pub e: String,
}

#[derive(Deserialize)]
struct Header {
    alg: String,
    #[serde(default)]
    kid: String,
}

#[derive(Deserialize)]
struct Aud {
    aud: serde_json::Value,
}

/// Keys published by each issuer, fetched at most once an hour (or when a key is unknown).
#[derive(Default)]
pub struct KeyCache(Mutex<HashMap<String, (Instant, Vec<Jwk>)>>);

impl KeyCache {
    /// Sets an issuer's keys (tests, or keys fetched elsewhere).
    pub fn put(&self, issuer: &str, keys: Vec<Jwk>) {
        if let Ok(mut m) = self.0.lock() {
            m.insert(issuer.to_owned(), (Instant::now(), keys));
        }
    }

    fn get(&self, issuer: &str, fresh: bool) -> Option<Vec<Jwk>> {
        let m = self.0.lock().ok()?;
        let (at, keys) = m.get(issuer)?;
        (!fresh || at.elapsed() < Duration::from_secs(3600)).then(|| keys.clone())
    }

    /// Keys for `issuer`; fetched (blocking) when missing, stale, or without `kid`.
    pub fn keys(&self, issuer: &str, kid: &str) -> Result<Vec<Jwk>, String> {
        if let Some(k) = self.get(issuer, true)
            && k.iter().any(|j| j.kid == kid)
        {
            return Ok(k);
        }
        match fetch_keys(issuer) {
            Ok(k) => {
                self.put(issuer, k.clone());
                Ok(k)
            }
            // The provider cannot be reached: keys already known still serve.
            Err(e) => self.get(issuer, false).ok_or(e),
        }
    }
}

fn http_json(url: &str) -> Result<serde_json::Value, String> {
    if !url.starts_with("https://") {
        return Err(format!("{url}: HTTPS required"));
    }
    ureq::get(url)
        .config()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .call()
        .map_err(|e| format!("{url}: {e}"))?
        .body_mut()
        .read_json()
        .map_err(|e| format!("{url}: {e}"))
}

/// The provider's discovery document, then its key set.
fn fetch_keys(issuer: &str) -> Result<Vec<Jwk>, String> {
    let disc = http_json(&format!(
        "{}/.well-known/openid-configuration",
        issuer.trim_end_matches('/')
    ))?;
    let uri = disc["jwks_uri"]
        .as_str()
        .ok_or("no jwks_uri in the discovery document")?;
    let set = http_json(uri)?;
    serde_json::from_value(set["keys"].clone()).map_err(|e| e.to_string())
}

/// Method a provider stands for, by its configured name.
pub fn method_of(p: &Oidc) -> Option<Method> {
    match p.name.to_ascii_lowercase().as_str() {
        "google" => Some(Method::Google),
        "microsoft" => Some(Method::Microsoft),
        _ => None,
    }
}

/// The address an issuer is reached at. A multi-tenant Microsoft issuer (`{tenantid}`) goes
/// through "common": work or school accounts of any tenant and personal Microsoft accounts
/// (outlook.com…), whose sign-in page and keys "organizations" would refuse.
pub fn reach(issuer: &str) -> String {
    issuer.replace("{tenantid}", "common")
}

/// Checks `token` for `provider` at time `now`. Microsoft's "common" issuers carry the tenant in
/// the token: `{tenantid}` in the configured issuer matches any tenant.
pub fn verify(provider: &Oidc, token: &str, now: i64, cache: &KeyCache) -> Result<Claims, String> {
    let mut parts = token.split('.');
    let (Some(h), Some(p), Some(s), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err("malformed token".into());
    };
    let header: Header =
        serde_json::from_slice(&B64.decode(h.as_bytes()).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if header.alg != "RS256" {
        return Err(format!("algorithm {} refused", header.alg));
    }
    let payload = B64.decode(p.as_bytes()).map_err(|e| e.to_string())?;
    let claims: Claims = serde_json::from_slice(&payload).map_err(|e| e.to_string())?;
    if !issuer_matches(&provider.issuer, &claims.iss) {
        return Err("token from another issuer".into());
    }
    let sig = B64.decode(s.as_bytes()).map_err(|e| e.to_string())?;
    // A multi-tenant Microsoft issuer ("{tenantid}") publishes its keys under "common".
    let keys = cache.keys(&reach(&provider.issuer), &header.kid)?;
    let signed = keys
        .iter()
        .filter(|k| k.kty == "RSA" && (header.kid.is_empty() || k.kid == header.kid))
        .any(|k| {
            let (Ok(n), Ok(e)) = (B64.decode(k.n.as_bytes()), B64.decode(k.e.as_bytes())) else {
                return false;
            };
            let input = &token[..h.len() + 1 + p.len()];
            cleis_crypto::verify_rs256(&n, &e, input.as_bytes(), &sig).is_ok()
        });
    if !signed {
        return Err("signature refused".into());
    }
    let aud: Aud = serde_json::from_slice(&payload).map_err(|e| e.to_string())?;
    let ok_aud = match &aud.aud {
        serde_json::Value::String(a) => a == &provider.client_id,
        serde_json::Value::Array(v) => v.iter().any(|a| a.as_str() == Some(&provider.client_id)),
        _ => false,
    };
    if !ok_aud {
        return Err("token for another application".into());
    }
    if claims.exp + 60 < now {
        return Err("token expired".into());
    }
    // Audit S-15: issued recently (1 minute of clock drift), and to Cleis.
    match claims.iat {
        Some(iat) if iat <= now + 60 && now - iat <= provider.max_token_age => {}
        _ => return Err("token too old".into()),
    }
    if claims
        .azp
        .as_deref()
        .is_some_and(|a| a != provider.client_id)
    {
        return Err("token for another application".into());
    }
    let verified = match &claims.email_verified {
        // Microsoft does not send it: its accounts' addresses belong to the tenant.
        None => true,
        Some(serde_json::Value::Bool(b)) => *b,
        Some(serde_json::Value::String(s)) => s == "true",
        Some(_) => false,
    };
    if !provider.domains.is_empty() {
        let domain = claims
            .address()
            .rsplit_once('@')
            .map(|(_, d)| d.to_ascii_lowercase())
            .unwrap_or_default();
        if !verified
            || !provider
                .domains
                .iter()
                .any(|d| d.eq_ignore_ascii_case(&domain))
        {
            return Err("this account's domain is not accepted".into());
        }
    }
    Ok(claims)
}

fn issuer_matches(configured: &str, actual: &str) -> bool {
    let configured = configured.trim_end_matches('/');
    let actual = actual.trim_end_matches('/');
    match configured.split_once("{tenantid}") {
        Some((pre, post)) => {
            actual.len() > pre.len() + post.len()
                && actual.starts_with(pre)
                && actual.ends_with(post)
                && !actual[pre.len()..actual.len() - post.len()].contains('/')
        }
        None => configured == actual,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub const N: &str = "ofgWCuLjybRlzo0tZWJjNiuSfb4p4fAkd_wWJcyQoTbji9k0l8W26mPddxHmfHQp-Vaw-4qPCJrcS2mJPMEzP1Pt0Bm4d4QlL-yRT-SFd2lZS-pCgNMsD1W_YpRPEwOWvG6b32690r2jZ47soMZo9wGzjb_7OMg0LOL-bSf63kpaSHSXndS5z5rexMdbBYUsLA9e-KXBdQOS-UTo7WTBEMa2R2CapHg665xsmtdVMTBQY4uDZlxvb3qCo5ZwKh9kG4LT6_I5IhlJH7aGhyxXFvUK-DWNmoudF8NAco9_h9iaGNj8q2ethFkMLs91kzk2PAcDTW9gb54h4FRWyuXpoQ";

    pub fn token(n: usize) -> String {
        include_str!("../tests/fixtures/oidc-tokens.txt")
            .lines()
            .find_map(|l| l.strip_prefix(&format!("T{n} ")))
            .unwrap()
            .to_owned()
    }

    fn provider(domains: &[&str]) -> Oidc {
        Oidc {
            name: "google".into(),
            issuer: "https://idp.example-idp.com".into(),
            client_id: "cleis-test".into(),
            client_secret: String::new(),
            domains: domains.iter().map(|d| d.to_string()).collect(),
            max_token_age: 600,
        }
    }

    fn cache() -> KeyCache {
        let c = KeyCache::default();
        c.put(
            "https://idp.example-idp.com",
            vec![Jwk {
                kid: "k1".into(),
                kty: "RSA".into(),
                n: N.into(),
                e: "AQAB".into(),
            }],
        );
        c
    }

    #[test]
    fn checks_tokens() {
        let now = 1_795_564_900;
        let c = verify(&provider(&[]), &token(1), now, &cache()).unwrap();
        assert_eq!(c.account(), "https://idp.example-idp.com|user-42");
        assert_eq!(c.nonce, "n1");
        // Domain restriction.
        assert!(verify(&provider(&["example-studio.com"]), &token(2), now, &cache()).is_ok());
        assert!(verify(&provider(&["example-studio.com"]), &token(3), now, &cache()).is_err());
        // Audience, issuer, expiry, tampering.
        let mut p = provider(&[]);
        p.client_id = "other".into();
        assert!(verify(&p, &token(1), now, &cache()).is_err());
        let mut p = provider(&[]);
        p.issuer = "https://other.example-idp.com".into();
        assert!(verify(&p, &token(1), now, &cache()).is_err());
        assert!(verify(&provider(&[]), &token(1), 4_102_444_800 + 120, &cache()).is_err());
        // Audit S-15: a token issued more than 10 minutes ago is refused.
        assert!(verify(&provider(&[]), &token(1), now + 700, &cache()).is_err());
        let t = token(1);
        let (head, rest) = t.split_once('.').unwrap();
        let (body, sig) = rest.split_once('.').unwrap();
        let other = token(3);
        let body3 = other.split('.').nth(1).unwrap();
        assert!(
            verify(
                &provider(&[]),
                &format!("{head}.{body3}.{sig}"),
                now,
                &cache()
            )
            .is_err()
        );
        assert!(verify(&provider(&[]), &format!("{head}.{body}"), now, &cache()).is_err());
    }

    #[test]
    fn tenant_issuers() {
        let p = "https://login.microsoftonline.com/{tenantid}/v2.0";
        assert!(issuer_matches(
            p,
            "https://login.microsoftonline.com/9188040d-6c67/v2.0"
        ));
        assert!(!issuer_matches(
            p,
            "https://login.microsoftonline.com/a/b/v2.0"
        ));
        assert!(!issuer_matches(p, "https://evil.example-idp.com/x/v2.0"));
        assert!(issuer_matches(
            "https://accounts.google.com",
            "https://accounts.google.com"
        ));
        // Personal Microsoft accounts: the consumers tenant, reached through "common".
        assert!(issuer_matches(
            p,
            "https://login.microsoftonline.com/9188040d-6c67-4c5b-b112-36a304b66dad/v2.0"
        ));
        assert_eq!(reach(p), "https://login.microsoftonline.com/common/v2.0");
        assert_eq!(
            reach("https://accounts.google.com"),
            "https://accounts.google.com"
        );
    }
}
