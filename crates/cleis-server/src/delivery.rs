//! Verification codes sent by e-mail (SMTP) or SMS (any HTTP gateway), D-64. Configured in
//! `cleis-server.toml`; a test server keeps them in its outbox instead.

use crate::config::{Sms, Smtp};
use std::time::Duration;

/// Where a code goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum To {
    Email(String),
    Phone(String),
}

/// A code as sent (kept by a test server).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sent {
    pub to: To,
    pub code: String,
}

pub fn text(org: &str, code: &str) -> String {
    format!("{code} is your Cleis code for {org}. It expires in 10 minutes.")
}

pub fn send_email(cfg: &Smtp, org: &str, to: &str, code: &str) -> Result<(), String> {
    send_mail(cfg, to, &format!("Cleis code: {code}"), text(org, code))
}

/// The invitation's text: what to type in Cleis (audit L-17).
pub fn invite_text(org: &str, address: &str, code: &str) -> String {
    format!(
        "You are invited to {org} in Cleis.\n\n\
         In Cleis, choose \"Join an organization\" and enter:\n\
         Server: {address}\n\
         Invitation code: {code}\n\n\
         The code works once and expires in 7 days."
    )
}

pub fn send_invite(
    cfg: &Smtp,
    org: &str,
    to: &str,
    address: &str,
    code: &str,
) -> Result<(), String> {
    send_mail(
        cfg,
        to,
        &format!("Invitation to {org} (Cleis)"),
        invite_text(org, address, code),
    )
}

fn send_mail(cfg: &Smtp, to: &str, subject: &str, body: String) -> Result<(), String> {
    use lettre::message::header::ContentType;
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{Message, SmtpTransport, Transport};
    let msg = Message::builder()
        .from(cfg.from.parse().map_err(|e| format!("from: {e}"))?)
        .to(to.parse().map_err(|e| format!("{to}: {e}"))?)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body)
        .map_err(|e| e.to_string())?;
    let builder = match cfg.security.as_str() {
        "tls" => SmtpTransport::relay(&cfg.host),
        "none" => Ok(SmtpTransport::builder_dangerous(&cfg.host)),
        _ => SmtpTransport::starttls_relay(&cfg.host),
    }
    .map_err(|e| e.to_string())?;
    let mut builder = builder
        .port(cfg.port)
        .timeout(Some(Duration::from_secs(15)));
    if !cfg.username.is_empty() {
        builder = builder.credentials(Credentials::new(cfg.username.clone(), cfg.password.clone()));
    }
    builder
        .build()
        .send(&msg)
        .map(|_| ())
        .map_err(|e| format!("e-mail not sent: {e}"))
}

/// Replaces `{to}`, `{code}` and `{text}`: percent-encoded in the URL; in the body, JSON-escaped
/// when it is JSON, percent-encoded when it is a form, as is otherwise.
pub fn sms_request(cfg: &Sms, org: &str, to: &str, code: &str) -> (String, String) {
    let text = text(org, code);
    let vars = [("{to}", to), ("{code}", code), ("{text}", text.as_str())];
    let mut url = cfg.url.clone();
    for (k, v) in vars {
        url = url.replace(k, &pct(v));
    }
    let json = cfg.body.trim_start().starts_with('{');
    let form = cfg
        .headers
        .iter()
        .any(|h| h.to_ascii_lowercase().contains("x-www-form-urlencoded"));
    let mut body = cfg.body.clone();
    for (k, v) in vars {
        let v = if json {
            let s = serde_json::to_string(v).unwrap_or_default();
            s[1..s.len() - 1].to_owned()
        } else if form {
            pct(v)
        } else {
            v.to_owned()
        };
        body = body.replace(k, &v);
    }
    (url, body)
}

pub fn send_sms(cfg: &Sms, org: &str, to: &str, code: &str) -> Result<(), String> {
    let (url, body) = sms_request(cfg, org, to, code);
    if !url.starts_with("https://") {
        return Err("the SMS gateway must be reached by HTTPS".into());
    }
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .new_agent();
    let mut req = http_request(&cfg.method, &url)?;
    for h in &cfg.headers {
        if let Some((k, v)) = h.split_once(':') {
            req = req.header(k.trim(), v.trim());
        }
    }
    let req = req.body(body.into_bytes()).map_err(|e| e.to_string())?;
    agent
        .run(req)
        .map(|_| ())
        .map_err(|e| format!("SMS not sent: {e}"))
}

fn http_request(method: &str, url: &str) -> Result<ureq::http::request::Builder, String> {
    let m = ureq::http::Method::from_bytes(method.to_ascii_uppercase().as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(ureq::http::Request::builder().method(m).uri(url))
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

/// `l•••@example-studio.com`, `•••• 00 12`.
pub fn masked(to: &To) -> String {
    match to {
        To::Email(e) => match e.split_once('@') {
            Some((u, d)) => format!("{}•••@{d}", u.chars().next().unwrap_or('•')),
            None => "•••".into(),
        },
        To::Phone(p) => {
            let digits: Vec<char> = p.chars().filter(char::is_ascii_digit).collect();
            let tail: String = digits[digits.len().saturating_sub(4)..].iter().collect();
            format!(
                "•••• {} {}",
                &tail[..tail.len().min(2)],
                &tail[tail.len().min(2)..]
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sms_templates() {
        let mut cfg = Sms {
            url: "https://sms.example-gateway.com/send?to={to}".into(),
            method: "POST".into(),
            headers: vec!["Content-Type: application/json".into()],
            body: r#"{"to":"{to}","message":"{text}"}"#.into(),
        };
        let (url, body) = sms_request(&cfg, "Example \"Studio\"", "+33 6 99 00 00 12", "123456");
        assert_eq!(
            url,
            "https://sms.example-gateway.com/send?to=%2B33%206%2099%2000%2000%2012"
        );
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["to"], "+33 6 99 00 00 12");
        assert!(
            v["message"]
                .as_str()
                .unwrap()
                .contains("Example \"Studio\"")
        );
        cfg.headers = vec!["Content-Type: application/x-www-form-urlencoded".into()];
        cfg.body = "To={to}&Body={code}".into();
        let (_, body) = sms_request(&cfg, "X", "+33699000012", "123456");
        assert_eq!(body, "To=%2B33699000012&Body=123456");
    }

    #[test]
    fn masks() {
        assert_eq!(
            masked(&To::Email("lea@example-studio.com".into())),
            "l•••@example-studio.com"
        );
        assert_eq!(masked(&To::Phone("+33 6 99 00 00 12".into())), "•••• 00 12");
    }
}
