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
}

/// Address of the peer, set on each request.
#[derive(Clone, Copy, Debug)]
pub struct Peer(pub SocketAddr);

/// A refused request.
#[derive(Debug)]
pub struct ApiErr(pub StatusCode, pub &'static str, pub String);

impl ApiErr {
    pub fn forbidden(msg: &str) -> Self {
        Self(StatusCode::FORBIDDEN, "forbidden", msg.into())
    }
    pub fn bad(msg: &str) -> Self {
        Self(StatusCode::BAD_REQUEST, "bad_request", msg.into())
    }
    pub fn not_found(msg: &str) -> Self {
        Self(StatusCode::NOT_FOUND, "not_found", msg.into())
    }
    pub fn unauthorized(code: &'static str, msg: &str) -> Self {
        Self(StatusCode::UNAUTHORIZED, code, msg.into())
    }
}

impl From<rusqlite::Error> for ApiErr {
    fn from(e: rusqlite::Error) -> Self {
        eprintln!("database error: {e}");
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            "server",
            "The server could not save this change.".into(),
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
            let conn = self.db.lock().expect("database lock");
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
        }))
    }

    pub fn identity(&self) -> Result<tls::Identity, ServerError> {
        let conn = self.db.lock().expect("database lock");
        Ok(tls::Identity {
            cert: db::meta_get(&conn, "cert")?.unwrap_or_default(),
            key: db::meta_get(&conn, "cert_key")?.unwrap_or_default(),
        })
    }

    /// Counts a failed sign-in or join from `ip`; true when it should be refused (more than 20
    /// failures in 10 minutes).
    pub fn too_many_failures(&self, ip: IpAddr, failed: bool) -> bool {
        let mut f = self.failures.lock().expect("failures lock");
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
        self.log.lock().expect("log lock").append(conn, &event)?;
        Ok(())
    }

    /// A new invitation code for the server's current certificate (command line).
    pub fn invite(&self, name: &str, email: &str, role: &str) -> Result<String, ServerError> {
        let conn = self.db.lock().expect("database lock");
        Ok(api::create_invite(
            &conn,
            &self.cert,
            name,
            email,
            role,
            &[],
            None,
            cleis_core::now(),
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
    #[cfg(not(unix))]
    let _ = dir;
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
    loop {
        let (tcp, peer) = match listener.accept().await {
            Ok(x) => x,
            Err(e) => {
                eprintln!("accept: {e}");
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                continue;
            }
        };
        let acceptor = acceptor.clone();
        let router = router.clone();
        tokio::spawn(async move {
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
            let _ = hyper::server::conn::http1::Builder::new()
                .serve_connection(hyper_util::rt::TokioIo::new(tls), svc)
                .await;
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
