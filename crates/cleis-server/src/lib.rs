//! Cleis organization server (D-26, D-32, D-59): one binary, one SQLite file, HTTPS with a
//! self-signed certificate that the apps pin.
//!
//! It never sees a key that opens a record. It checks who may read and write each space, keeps
//! the previous versions of each item, writes the activity log (signed and chained), and hands
//! each member the space keys others wrapped for them.

#![forbid(unsafe_code)]

pub mod api;
pub mod config;
pub mod db;
pub mod delivery;
pub mod local;
pub mod oidc;
pub mod tls;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use cleis_core::org::{ApiError, Invitation};
use rusqlite::Connection;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::net::TcpListener;
use tower::ServiceExt;

pub const DB_FILE: &str = "server.db";
/// Invitations expire after a week.
pub const INVITE_DAYS: i64 = 7;
/// Sessions last a working day.
/// Connections served at once (audit S-4).
pub const MAX_CONNECTIONS: usize = 512;

pub const SESSION_SECS: i64 = 12 * 3600;
/// An announced certificate replaces the current one at the first restart after this delay.
pub const ROTATION_DAYS: i64 = 7;

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("{0}")]
    Setup(String),
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Shared state of a running server.
pub struct App {
    pub db: Mutex<Connection>,
    pub log: Mutex<db::Log>,
    pub cert: [u8; 32],
    pub dir: PathBuf,
    failures: Mutex<HashMap<IpAddr, Vec<Instant>>>,
    /// `cleis-server.toml`, read at start.
    pub config: config::Config,
    /// Identity providers' keys (D-64).
    pub keys: oidc::KeyCache,
    /// Codes kept instead of sent (`capture_codes`, tests).
    pub outbox: Mutex<Option<Vec<delivery::Sent>>>,
}

/// Address of the peer, set on each request.
#[derive(Clone, Copy, Debug)]
pub struct Peer(pub SocketAddr);

/// A refused request: status, stable code, message, and the PIN's length with the `pin_*` codes
/// (the app shows that many boxes, D-79).
#[derive(Debug)]
pub struct ApiErr(pub StatusCode, pub &'static str, pub String, pub Option<u8>);

impl ApiErr {
    pub fn forbidden(msg: &str) -> Self {
        Self(StatusCode::FORBIDDEN, "forbidden", msg.into(), None)
    }
    pub fn bad(msg: &str) -> Self {
        Self(StatusCode::BAD_REQUEST, "bad_request", msg.into(), None)
    }
    pub fn not_found(msg: &str) -> Self {
        Self(StatusCode::NOT_FOUND, "not_found", msg.into(), None)
    }
    pub fn unauthorized(code: &'static str, msg: &str) -> Self {
        Self(StatusCode::UNAUTHORIZED, code, msg.into(), None)
    }
    /// A `pin_*` refusal (D-79, D-98): the PIN's length, or 0 when a password is asked.
    pub fn pin(code: &'static str, msg: &str, length: u8) -> Self {
        Self(StatusCode::UNAUTHORIZED, code, msg.into(), Some(length))
    }

    /// A `pin_*` refusal for the organization's rule: its PIN's length, or a password.
    pub fn access(code: &'static str, msg: &str, rule: &cleis_core::org::AccessRule) -> Self {
        let len = match rule.with {
            cleis_core::org::AccessWith::Pin => rule.length,
            _ => 0,
        };
        Self::pin(code, msg, len)
    }
}

/// The digest of an organization's password or PIN (D-79, D-98): the authentication half of
/// Argon2id, in hex. Made by the admin's app, checked by the server or, in a shared folder, by
/// each member's app.
pub fn access_digest(
    secret: &str,
    salt: &[u8; 16],
    params: cleis_crypto::KdfParams,
) -> Result<String, String> {
    let k = cleis_crypto::derive_master_key(secret, salt, params).map_err(|e| e.to_string())?;
    Ok(data_encoding::HEXLOWER.encode(k.authentication.as_bytes()))
}

/// A new digest of `secret`, with a random salt and Cleis' default cost.
pub fn new_access_secret(secret: &str) -> Result<cleis_core::org::AccessSecret, String> {
    let salt: [u8; 16] = cleis_crypto::random_bytes().map_err(|e| e.to_string())?;
    let params = cleis_crypto::KdfParams::default();
    Ok(cleis_core::org::AccessSecret {
        salt: data_encoding::HEXLOWER.encode(&salt),
        hash: access_digest(secret, &salt, params)?,
        params,
    })
}

/// Whether `secret` is the one `stored` was made from.
pub fn access_matches(stored: &cleis_core::org::AccessSecret, secret: &str) -> bool {
    let Some(salt) = data_encoding::HEXLOWER
        .decode(stored.salt.as_bytes())
        .ok()
        .and_then(|v| <[u8; 16]>::try_from(v).ok())
    else {
        return false;
    };
    access_digest(secret, &salt, stored.params).is_ok_and(|h| {
        use subtle_eq::ct_eq;
        ct_eq(h.as_bytes(), stored.hash.as_bytes())
    })
}

/// Comparison that does not stop at the first different byte.
mod subtle_eq {
    pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
        a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
    }
}

impl From<rusqlite::Error> for ApiErr {
    fn from(e: rusqlite::Error) -> Self {
        eprintln!("database error: {e}");
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            "server",
            "The server could not save this change.".into(),
            None,
        )
    }
}

impl IntoResponse for ApiErr {
    fn into_response(self) -> Response {
        (
            self.0,
            axum::Json(ApiError {
                error: self.1.into(),
                message: self.2,
                pin_length: self.3.filter(|l| *l > 0),
                password: self.3 == Some(0),
            }),
        )
            .into_response()
    }
}

impl App {
    /// Copies the server file into `dir` (`server-YYYYMMDD-HHMMSS.db`) and keeps the `keep` most
    /// recent copies (D-63).
    pub fn backup_to(&self, dir: &Path, keep: u32) -> Result<PathBuf, ServerError> {
        std::fs::create_dir_all(dir)?;
        restrict(dir);
        let now = cleis_core::now();
        let (y, mo, d) = civil(now.div_euclid(86_400));
        let s = now.rem_euclid(86_400);
        let name = format!(
            "server-{y}{mo:02}{d:02}-{:02}{:02}{:02}.db",
            s / 3600,
            s % 3600 / 60,
            s % 60
        );
        let dest = dir.join(name);
        let tmp = dir.join(".backup.tmp");
        let _ = std::fs::remove_file(&tmp);
        {
            let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
            conn.execute("VACUUM INTO ?1", [tmp.to_string_lossy().as_ref()])?;
        }
        std::fs::rename(&tmp, &dest)?;
        let mut old: Vec<PathBuf> = std::fs::read_dir(dir)?
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("server-") && n.ends_with(".db"))
            })
            .collect();
        old.sort();
        let keep = keep.max(1) as usize;
        if old.len() > keep {
            for p in &old[..old.len() - keep] {
                let _ = std::fs::remove_file(p);
            }
        }
        Ok(dest)
    }

    /// Creates the data folder's database and certificate. Returns the invitation code of the
    /// first administrator.
    pub fn init(dir: &Path, org: &str, names: &[String]) -> Result<String, ServerError> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(DB_FILE);
        if path.exists() {
            return Err(ServerError::Setup(format!(
                "{} already holds a Cleis server",
                dir.display()
            )));
        }
        let conn = db::open(&path)?;
        let id = tls::Identity::generate(names).map_err(ServerError::Setup)?;
        db::meta_set(&conn, "cert", &id.cert)?;
        db::meta_set(&conn, "cert_key", &id.key)?;
        db::meta_set(&conn, "org_id", uuid::Uuid::new_v4().as_bytes())?;
        db::meta_set(&conn, "org_name", org.as_bytes())?;
        db::meta_set_json(&conn, "roles", &cleis_core::org::Role::defaults())?;
        db::meta_set_json(&conn, "policy", &cleis_core::org::Policy::default())?;
        let _ = db::Log::load(&conn)?;
        let code = api::create_invite(
            &conn,
            &id.hash(),
            "",
            "",
            cleis_core::org::SUPER_ADMIN,
            &[],
            None,
            cleis_core::now(),
            None,
        )?
        .1;
        restrict(dir);
        Ok(code)
    }

    /// Opens an initialized data folder; switches to an announced certificate once its delay has
    /// passed.
    pub fn open(dir: &Path) -> Result<Arc<App>, ServerError> {
        let path = dir.join(DB_FILE);
        if !path.exists() {
            return Err(ServerError::Setup(format!(
                "no Cleis server in {} (run `cleis-server init` first)",
                dir.display()
            )));
        }
        let conn = db::open(&path)?;
        let now = cleis_core::now();
        if let (Some(at), Some(cert), Some(key)) = (
            db::meta_json::<i64>(&conn, "next_cert_at"),
            db::meta_get(&conn, "next_cert")?,
            db::meta_get(&conn, "next_cert_key")?,
        ) && now - at >= ROTATION_DAYS * 86_400
        {
            finish_rotation(&conn, &cert, &key)?;
        }
        db::prune_log(&conn, now)?;
        db::migrate_roles(&conn)?;
        let log = db::Log::load(&conn)?;
        let cert = cleis_crypto::sha256(
            &db::meta_get(&conn, "cert")?
                .ok_or_else(|| ServerError::Setup("no certificate".into()))?,
        );
        Ok(Arc::new(App {
            db: Mutex::new(conn),
            log: Mutex::new(log),
            cert,
            dir: dir.to_owned(),
            failures: Mutex::new(HashMap::new()),
            config: config::Config::load(dir).map_err(ServerError::Setup)?,
            keys: oidc::KeyCache::default(),
            outbox: Mutex::new(None),
        }))
    }

    pub fn identity(&self) -> Result<tls::Identity, ServerError> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        Ok(tls::Identity {
            cert: db::meta_get(&conn, "cert")?.unwrap_or_default(),
            key: db::meta_get(&conn, "cert_key")?.unwrap_or_default(),
        })
    }

    /// Counts a failed sign-in or join from `ip`; true when it should be refused (more than 20
    /// failures in 10 minutes).
    pub fn too_many_failures(&self, ip: IpAddr, failed: bool) -> bool {
        let mut f = self.failures.lock().unwrap_or_else(|e| e.into_inner());
        // Addresses with no recent failure are forgotten (the table does not grow forever).
        if f.len() > 1024 {
            f.retain(|_, l| l.iter().any(|t| t.elapsed().as_secs() < 600));
        }
        let list = f.entry(ip).or_default();
        list.retain(|t| t.elapsed().as_secs() < 600);
        if failed {
            list.push(Instant::now());
        }
        list.len() > 20
    }

    /// Writes an event if the policy records it.
    pub fn record(&self, conn: &Connection, event: cleis_core::org::Event) -> Result<(), ApiErr> {
        if !db::policy(conn).records(event.action) {
            return Ok(());
        }
        self.log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .append(conn, &event)?;
        Ok(())
    }

    /// A new invitation code for the server's current certificate (command line).
    pub fn invite(&self, name: &str, email: &str, role: &str) -> Result<String, ServerError> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        Ok(api::create_invite(
            &conn,
            &self.cert,
            name,
            email,
            role,
            &[],
            None,
            cleis_core::now(),
            None,
        )?
        .1)
    }
}

/// Makes the new certificate the current one.
pub fn finish_rotation(conn: &Connection, cert: &[u8], key: &[u8]) -> rusqlite::Result<()> {
    db::meta_set(conn, "cert", cert)?;
    db::meta_set(conn, "cert_key", key)?;
    db::meta_del(conn, "next_cert")?;
    db::meta_del(conn, "next_cert_key")?;
    db::meta_del(conn, "next_cert_at")
}

/// The data folder is for the service account alone.
fn restrict(dir: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    }
    // Audit S-7: on Windows, the folder stops inheriting (ProgramData is readable by every
    // account) and keeps SYSTEM, the Administrators and the account that made it.
    #[cfg(windows)]
    {
        let me = match (std::env::var("USERDOMAIN"), std::env::var("USERNAME")) {
            (Ok(d), Ok(u)) if !u.ends_with('$') => Some(format!("{d}\\{u}:(OI)(CI)F")),
            _ => None,
        };
        let mut args = vec![
            dir.as_os_str().to_owned(),
            "/inheritance:r".into(),
            "/grant:r".into(),
            "*S-1-5-18:(OI)(CI)F".into(),
            "/grant:r".into(),
            "*S-1-5-32-544:(OI)(CI)F".into(),
        ];
        if let Some(me) = me {
            args.push("/grant:r".into());
            args.push(me.into());
        }
        let _ = std::process::Command::new("icacls")
            .args(&args)
            .stdout(std::process::Stdio::null())
            .status();
    }
}

/// Invitation code for a token and certificate.
pub fn code_for(token: [u8; 10], cert: &[u8; 32]) -> String {
    Invitation {
        token,
        pin: cert[..10].try_into().expect("10 bytes"),
    }
    .code()
}

/// Serves HTTPS on `listener` until `shutdown` resolves.
pub async fn serve(
    app: Arc<App>,
    listener: TcpListener,
    shutdown: impl std::future::Future<Output = ()>,
) -> Result<(), ServerError> {
    serve_all(app, vec![listener], shutdown).await
}

/// Serves HTTPS on every listener until `shutdown` resolves (D-63).
pub async fn serve_all(
    app: Arc<App>,
    listeners: Vec<TcpListener>,
    shutdown: impl std::future::Future<Output = ()>,
) -> Result<(), ServerError> {
    let config = app
        .identity()?
        .server_config()
        .map_err(ServerError::Setup)?;
    let acceptor = tokio_rustls::TlsAcceptor::from(config);
    let router = api::router(app.clone());
    let tasks: Vec<_> = listeners
        .into_iter()
        .map(|l| tokio::spawn(accept_loop(l, acceptor.clone(), router.clone())))
        .collect();
    shutdown.await;
    for t in tasks {
        t.abort();
    }
    Ok(())
}

async fn accept_loop(
    listener: TcpListener,
    acceptor: tokio_rustls::TlsAcceptor,
    router: axum::Router,
) {
    let slots = std::sync::Arc::new(tokio::sync::Semaphore::new(MAX_CONNECTIONS));
    loop {
        let (tcp, peer) = match listener.accept().await {
            Ok(x) => x,
            Err(e) => {
                eprintln!("accept: {e}");
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                continue;
            }
        };
        // At most MAX_CONNECTIONS at once (audit S-4); the next ones wait.
        let Ok(permit) = slots.clone().acquire_owned().await else {
            return;
        };
        let acceptor = acceptor.clone();
        let router = router.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let Ok(Ok(tls)) =
                tokio::time::timeout(std::time::Duration::from_secs(10), acceptor.accept(tcp))
                    .await
            else {
                return;
            };
            let svc = hyper::service::service_fn(
                move |mut req: hyper::Request<hyper::body::Incoming>| {
                    req.extensions_mut().insert(Peer(peer));
                    router.clone().oneshot(req)
                },
            );
            // A request's headers must arrive within 15 s; a connection lives 10 min at most.
            let conn = hyper::server::conn::http1::Builder::new()
                .timer(hyper_util::rt::TokioTimer::new())
                .header_read_timeout(std::time::Duration::from_secs(15))
                .serve_connection(hyper_util::rt::TokioIo::new(tls), svc);
            let _ = tokio::time::timeout(std::time::Duration::from_secs(600), conn).await;
        });
    }
}

/// Days since 1970-01-01 → (year, month, day).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// Runs the automatic backups of the configuration until the process ends.
pub async fn backup_loop(app: Arc<App>, dir: PathBuf, every_hours: u32, keep: u32) {
    let every = std::time::Duration::from_secs(u64::from(every_hours.max(1)) * 3600);
    loop {
        let a = app.clone();
        let (d, k) = (dir.clone(), keep);
        match tokio::task::spawn_blocking(move || a.backup_to(&d, k)).await {
            Ok(Ok(p)) => println!("backup: {}", p.display()),
            Ok(Err(e)) => eprintln!("backup failed: {e}"),
            Err(e) => eprintln!("backup failed: {e}"),
        }
        tokio::time::sleep(every).await;
    }
}

impl App {
    /// Keeps verification codes in `outbox` instead of sending them (tests).
    pub fn capture_codes(&self) {
        if let Ok(mut o) = self.outbox.lock() {
            *o = Some(vec![]);
        }
    }

    /// Codes kept since `capture_codes`.
    pub fn sent_codes(&self) -> Vec<delivery::Sent> {
        self.outbox
            .lock()
            .ok()
            .and_then(|o| o.clone())
            .unwrap_or_default()
    }

    /// Sends an invitation by e-mail when `[smtp]` is set up (blocking; audit L-17). A test
    /// server keeps it in its outbox, as codes.
    pub fn deliver_invite(&self, to: &str, address: &str, code: &str) -> Result<(), String> {
        if let Ok(mut o) = self.outbox.lock()
            && let Some(v) = o.as_mut()
        {
            v.push(delivery::Sent {
                to: delivery::To::Email(to.to_owned()),
                code: code.to_owned(),
            });
            return Ok(());
        }
        let cfg = self.config.smtp.as_ref().ok_or("e-mail is not set up")?;
        let org = {
            let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
            String::from_utf8(
                db::meta_get(&conn, "org_name")
                    .ok()
                    .flatten()
                    .unwrap_or_default(),
            )
            .unwrap_or_default()
        };
        delivery::send_invite(cfg, &org, to, address, code)
    }

    pub fn emails_invites(&self) -> bool {
        self.config.smtp.is_some() || self.outbox.lock().is_ok_and(|o| o.is_some())
    }

    /// Sends a code by e-mail or SMS (blocking: call from a blocking task).
    pub fn deliver(&self, to: &delivery::To, code: &str) -> Result<(), String> {
        if let Ok(mut o) = self.outbox.lock()
            && let Some(v) = o.as_mut()
        {
            v.push(delivery::Sent {
                to: to.clone(),
                code: code.to_owned(),
            });
            return Ok(());
        }
        let org = {
            let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
            String::from_utf8(
                db::meta_get(&conn, "org_name")
                    .ok()
                    .flatten()
                    .unwrap_or_default(),
            )
            .unwrap_or_default()
        };
        match to {
            delivery::To::Email(e) => {
                let cfg = self
                    .config
                    .smtp
                    .as_ref()
                    .ok_or("e-mail is not set up on the server")?;
                delivery::send_email(cfg, &org, e, code)
            }
            delivery::To::Phone(p) => {
                let cfg = self
                    .config
                    .sms
                    .as_ref()
                    .ok_or("SMS is not set up on the server")?;
                delivery::send_sms(cfg, &org, p, code)
            }
        }
    }
}
