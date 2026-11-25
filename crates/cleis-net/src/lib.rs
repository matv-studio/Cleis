//! Talking to the organization server (D-32, D-59).
//!
//! HTTPS by rustls with the server's certificate pinned: the app accepts the certificates whose
//! SHA-256 it has stored, and on the first connection the one whose hash starts with the 10
//! bytes carried by the invitation code. No certificate authority is involved; the TLS handshake
//! signature is still checked by rustls, so only the holder of the pinned certificate's key can
//! answer.
//!
//! HTTP/1.1 over that stream is written here (one request per connection, `Connection: close`):
//! the server is ours and always sends a length or chunks, so a full HTTP client is not needed.

#![forbid(unsafe_code)]

use cleis_core::org::ApiError;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, WebPkiSupportedAlgorithms};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, ClientConnection, DigitallySignedStruct, SignatureScheme, StreamOwned};
use serde::{Serialize, de::DeserializeOwned};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NetError {
    #[error("the server cannot be reached: {0}")]
    Unreachable(String),
    #[error("this is not your organization's server (certificate not trusted)")]
    Untrusted,
    #[error("secure connection failed: {0}")]
    Tls(String),
    #[error("{}", .1.message)]
    Api(u16, ApiError),
    #[error("unexpected answer from the server: {0}")]
    Bad(String),
}

impl NetError {
    /// The server's error code (`code_required`, `forbidden`…), if it answered with one.
    pub fn code(&self) -> Option<&str> {
        match self {
            NetError::Api(_, e) => Some(&e.error),
            _ => None,
        }
    }
}

/// Which server certificates to accept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Trust {
    /// Full SHA-256 hashes stored after joining (the current one and an announced next one).
    Pinned(Vec<[u8; 32]>),
    /// First contact: the hash must start with these bytes from the invitation code.
    Prefix([u8; 10]),
}

impl Trust {
    fn accepts(&self, hash: &[u8; 32]) -> bool {
        match self {
            Trust::Pinned(p) => p.contains(hash),
            Trust::Prefix(p) => hash[..10] == p[..],
        }
    }
}

#[derive(Debug)]
struct Pinned {
    trust: Trust,
    seen: Mutex<Option<[u8; 32]>>,
    algs: WebPkiSupportedAlgorithms,
}

impl ServerCertVerifier for Pinned {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let hash = cleis_crypto::sha256(end_entity.as_ref());
        if let Ok(mut s) = self.seen.lock() {
            *s = Some(hash);
        }
        if self.trust.accepts(&hash) {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::InvalidCertificate(
                rustls::CertificateError::ApplicationVerificationFailure,
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.algs)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.algs)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algs.supported_schemes()
    }
}

fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// A server reached at `address` (`host:port`), trusted as `trust`.
#[derive(Clone, Debug)]
pub struct Client {
    pub address: String,
    pub trust: Trust,
    /// Session token from `/v1/session`.
    pub token: Option<String>,
    pub timeout: Duration,
}

/// An answer: HTTP status, body and the hash of the certificate the server presented.
#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
    pub cert: [u8; 32],
}

impl Client {
    pub fn new(address: &str, trust: Trust) -> Self {
        Self {
            address: address.to_owned(),
            trust,
            token: None,
            timeout: Duration::from_secs(20),
        }
    }

    fn host(&self) -> &str {
        let a = self.address.as_str();
        if let Some(rest) = a.strip_prefix('[') {
            rest.split(']').next().unwrap_or(rest)
        } else {
            a.rsplit_once(':').map_or(a, |(h, _)| h)
        }
    }

    /// Sends one request and reads the whole answer.
    pub fn send(
        &self,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
    ) -> Result<Response, NetError> {
        let provider = provider();
        let verifier = Arc::new(Pinned {
            trust: self.trust.clone(),
            seen: Mutex::new(None),
            algs: provider.signature_verification_algorithms,
        });
        let config = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|e| NetError::Tls(e.to_string()))?
            .dangerous()
            .with_custom_certificate_verifier(verifier.clone())
            .with_no_client_auth();
        let name = ServerName::try_from(self.host().to_owned())
            .map_err(|e| NetError::Unreachable(e.to_string()))?;
        let conn = ClientConnection::new(Arc::new(config), name)
            .map_err(|e| NetError::Tls(e.to_string()))?;

        let addr = self
            .address
            .to_socket_addrs()
            .map_err(|e| NetError::Unreachable(e.to_string()))?
            .next()
            .ok_or_else(|| NetError::Unreachable(self.address.clone()))?;
        let tcp = TcpStream::connect_timeout(&addr, Duration::from_secs(5).min(self.timeout))
            .map_err(|e| NetError::Unreachable(e.to_string()))?;
        tcp.set_read_timeout(Some(self.timeout)).ok();
        tcp.set_write_timeout(Some(self.timeout)).ok();
        let mut tls = StreamOwned::new(conn, tcp);

        let body = body.unwrap_or_default();
        let mut head = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nUser-Agent: Cleis/{}\r\nAccept: application/json\r\nContent-Length: {}\r\n",
            self.address,
            env!("CARGO_PKG_VERSION"),
            body.len()
        );
        if !body.is_empty() {
            head.push_str("Content-Type: application/json\r\n");
        }
        if let Some(t) = &self.token {
            head.push_str(&format!("Authorization: Bearer {t}\r\n"));
        }
        head.push_str("\r\n");
        let written = tls
            .write_all(head.as_bytes())
            .and_then(|_| tls.write_all(body))
            .and_then(|_| tls.flush());
        let seen = verifier.seen.lock().ok().and_then(|s| *s);
        if let Err(e) = written {
            return Err(tls_error(e, seen.is_some_and(|h| !self.trust.accepts(&h))));
        }
        let cert = seen.ok_or_else(|| NetError::Tls("no certificate".into()))?;
        read_response(BufReader::new(tls), cert)
    }

    /// POST a JSON body, read a JSON answer. A refusal becomes [`NetError::Api`].
    pub fn call<Q: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        req: &Q,
    ) -> Result<R, NetError> {
        let body = serde_json::to_vec(req).map_err(|e| NetError::Bad(e.to_string()))?;
        let r = self.send("POST", path, Some(&body))?;
        decode(r)
    }

    /// [`Client::call`], also giving the hash of the certificate the server presented (signed
    /// into the sign-in, stored after joining).
    pub fn call_cert<Q: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        req: &Q,
    ) -> Result<(R, [u8; 32]), NetError> {
        let body = serde_json::to_vec(req).map_err(|e| NetError::Bad(e.to_string()))?;
        let r = self.send("POST", path, Some(&body))?;
        let cert = r.cert;
        Ok((decode(r)?, cert))
    }

    pub fn get<R: DeserializeOwned>(&self, path: &str) -> Result<R, NetError> {
        decode(self.send("GET", path, None)?)
    }
}

fn decode<R: DeserializeOwned>(r: Response) -> Result<R, NetError> {
    if (200..300).contains(&r.status) {
        serde_json::from_slice(&r.body).map_err(|e| NetError::Bad(e.to_string()))
    } else {
        let e = serde_json::from_slice::<ApiError>(&r.body).unwrap_or(ApiError {
            error: "http".into(),
            message: format!("HTTP {}", r.status),
        });
        Err(NetError::Api(r.status, e))
    }
}

fn tls_error(e: std::io::Error, untrusted: bool) -> NetError {
    if untrusted {
        return NetError::Untrusted;
    }
    match e.kind() {
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => {
            NetError::Unreachable("no answer".into())
        }
        _ => NetError::Tls(e.to_string()),
    }
}

const MAX_BODY: usize = 256 * 1024 * 1024;

fn read_response<R: Read>(mut r: BufReader<R>, cert: [u8; 32]) -> Result<Response, NetError> {
    let io = |e: std::io::Error| NetError::Bad(e.to_string());
    let mut line = String::new();
    r.read_line(&mut line).map_err(io)?;
    let status: u16 = line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| NetError::Bad(format!("status line {line:?}")))?;
    let mut length = None;
    let mut chunked = false;
    loop {
        line.clear();
        if r.read_line(&mut line).map_err(io)? == 0 {
            break;
        }
        let l = line.trim_end();
        if l.is_empty() {
            break;
        }
        if let Some((k, v)) = l.split_once(':') {
            let v = v.trim();
            if k.eq_ignore_ascii_case("content-length") {
                length = v.parse::<usize>().ok();
            } else if k.eq_ignore_ascii_case("transfer-encoding") {
                chunked = v.eq_ignore_ascii_case("chunked");
            }
        }
    }
    let mut body = vec![];
    if chunked {
        loop {
            line.clear();
            r.read_line(&mut line).map_err(io)?;
            let size = usize::from_str_radix(line.trim().split(';').next().unwrap_or(""), 16)
                .map_err(|_| NetError::Bad("chunk size".into()))?;
            if size == 0 {
                break;
            }
            if body.len() + size > MAX_BODY {
                return Err(NetError::Bad("answer too large".into()));
            }
            let start = body.len();
            body.resize(start + size, 0);
            r.read_exact(&mut body[start..]).map_err(io)?;
            line.clear();
            r.read_line(&mut line).map_err(io)?;
        }
    } else if let Some(n) = length {
        if n > MAX_BODY {
            return Err(NetError::Bad("answer too large".into()));
        }
        body.resize(n, 0);
        r.read_exact(&mut body).map_err(io)?;
    } else {
        // Ends with the connection; a TLS close without close_notify still ends the body.
        let _ = r.take(MAX_BODY as u64).read_to_end(&mut body);
    }
    Ok(Response { status, body, cert })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_length_and_chunked_bodies() {
        let r = read_response(
            BufReader::new(&b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\n{}"[..]),
            [0; 32],
        )
        .unwrap();
        assert_eq!((r.status, &r.body[..]), (200, &b"{}"[..]));
        let r = read_response(
            BufReader::new(
                &b"HTTP/1.1 403 Forbidden\r\nTransfer-Encoding: chunked\r\n\r\n3\r\nabc\r\n2;x=y\r\nde\r\n0\r\n\r\n"[..],
            ),
            [0; 32],
        )
        .unwrap();
        assert_eq!((r.status, &r.body[..]), (403, &b"abcde"[..]));
        assert!(read_response(BufReader::new(&b"garbage\r\n"[..]), [0; 32]).is_err());
    }

    #[test]
    fn hosts() {
        let c = |a: &str| Client::new(a, Trust::Pinned(vec![])).host().to_owned();
        assert_eq!(
            c("vault.example-studio.lan:8443"),
            "vault.example-studio.lan"
        );
        assert_eq!(c("[fd00::1]:8443"), "fd00::1");
        assert_eq!(c("10.8.0.1:9000"), "10.8.0.1");
    }

    #[test]
    fn trust() {
        let mut h = [1u8; 32];
        assert!(Trust::Pinned(vec![h]).accepts(&h));
        assert!(Trust::Prefix([1; 10]).accepts(&h));
        h[0] = 2;
        assert!(!Trust::Prefix([1; 10]).accepts(&h));
        assert!(!Trust::Pinned(vec![[1; 32]]).accepts(&h));
    }
}
