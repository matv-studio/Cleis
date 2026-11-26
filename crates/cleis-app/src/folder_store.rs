//! Where an organization's shared folder is (D-78): a folder of this computer (SharePoint
//! library synced by the OneDrive app, Google shared drive, NAS share), or a folder on an SSH
//! server (SFTP). Paths inside are relative, with `/`.

use crate::remote::Secret;
use russh::client;
use russh::keys::{HashAlg, PrivateKeyWithHashAlg, PublicKeyOrCertificate, decode_secret_key};
use russh_sftp::client::SftpSession;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub trait Store: Send {
    /// A file's bytes, `None` if there is none.
    fn read(&mut self, rel: &str) -> Result<Option<Vec<u8>>, String>;
    /// Names in a folder (none if it does not exist).
    fn list(&mut self, rel: &str) -> Result<Vec<String>, String>;
    /// Writes a whole file at once (folders made as needed): others never see half of it.
    fn write(&mut self, rel: &str, bytes: &[u8]) -> Result<(), String>;
    fn remove(&mut self, rel: &str);
}

/// A folder of this computer.
pub struct LocalStore(pub PathBuf);

impl Store for LocalStore {
    fn read(&mut self, rel: &str) -> Result<Option<Vec<u8>>, String> {
        match std::fs::read(self.0.join(rel)) {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn list(&mut self, rel: &str) -> Result<Vec<String>, String> {
        match std::fs::read_dir(self.0.join(rel)) {
            Ok(r) => Ok(r
                .flatten()
                .filter_map(|e| e.file_name().to_str().map(str::to_owned))
                .collect()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
            Err(e) => Err(e.to_string()),
        }
    }

    fn write(&mut self, rel: &str, bytes: &[u8]) -> Result<(), String> {
        let path = self.0.join(rel);
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
    }

    fn remove(&mut self, rel: &str) {
        let _ = std::fs::remove_file(self.0.join(rel));
    }
}

/// An SFTP place: `sftp://user@host:port/path`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SftpPlace {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub path: String,
}

impl SftpPlace {
    pub fn parse(s: &str) -> Option<SftpPlace> {
        let rest = s.strip_prefix("sftp://")?;
        let (auth, path) = rest.split_once('/').map_or((rest, ""), |(a, p)| (a, p));
        let (user, hostport) = auth.split_once('@')?;
        let (host, port) = match hostport.rsplit_once(':') {
            Some((h, p)) => (h, p.parse().ok()?),
            None => (hostport, 22),
        };
        if user.is_empty() || host.is_empty() {
            return None;
        }
        Some(SftpPlace {
            host: host.into(),
            port,
            user: user.into(),
            path: format!("/{path}"),
        })
    }

    pub fn url(&self) -> String {
        format!(
            "sftp://{}@{}:{}/{}",
            self.user,
            self.host,
            self.port,
            self.path.trim_start_matches('/')
        )
    }
}

struct Keep {
    expected: String,
    seen: Arc<Mutex<String>>,
}

impl client::Handler for Keep {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let fp = key.public_key().fingerprint(HashAlg::Sha256).to_string();
        if let Ok(mut s) = self.seen.lock() {
            *s = fp.clone();
        }
        Ok(self.expected.is_empty() || self.expected == fp)
    }
}

/// A folder on an SSH server; one connection, opened again when it drops.
pub struct SftpStore {
    place: SftpPlace,
    secret: Secret,
    /// The server's key, kept at the first connection.
    pub host_key: String,
    rt: tokio::runtime::Runtime,
    sftp: Option<SftpSession>,
}

impl SftpStore {
    pub fn new(place: SftpPlace, secret: Secret, host_key: String) -> Result<SftpStore, String> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        Ok(SftpStore {
            place,
            secret,
            host_key,
            rt,
            sftp: None,
        })
    }

    fn full(&self, rel: &str) -> String {
        let base = self.place.path.trim_end_matches('/');
        if rel.is_empty() {
            base.to_owned()
        } else {
            format!("{base}/{rel}")
        }
    }

    fn ensure(&mut self) -> Result<(), String> {
        if self.sftp.is_none() {
            let (sftp, fp) =
                self.rt
                    .block_on(connect(&self.place, &self.secret, &self.host_key))?;
            if self.host_key.is_empty() {
                self.host_key = fp;
            }
            self.sftp = Some(sftp);
        }
        Ok(())
    }

    /// Runs `f`; a dropped connection is opened again once.
    fn with<R>(
        &mut self,
        f: impl Fn(&SftpSession, &tokio::runtime::Runtime) -> Result<R, String>,
    ) -> Result<R, String> {
        for attempt in 0..2 {
            self.ensure()?;
            let r = match &self.sftp {
                Some(s) => f(s, &self.rt),
                None => Err("SFTP".into()),
            };
            match r {
                Ok(v) => return Ok(v),
                Err(e) if attempt == 1 => return Err(e),
                Err(_) => self.sftp = None,
            }
        }
        Err("SFTP".into())
    }
}

async fn connect(
    place: &SftpPlace,
    secret: &Secret,
    expected: &str,
) -> Result<(SftpSession, String), String> {
    let seen = Arc::new(Mutex::new(String::new()));
    let config = Arc::new(client::Config {
        inactivity_timeout: Some(Duration::from_secs(300)),
        ..Default::default()
    });
    let handler = Keep {
        expected: expected.to_owned(),
        seen: seen.clone(),
    };
    let connect = client::connect(config, (place.host.as_str(), place.port), handler);
    let mut session = match tokio::time::timeout(Duration::from_secs(20), connect).await {
        Err(_) => return Err(format!("{}: no answer", place.host)),
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            let got = seen.lock().map(|s| s.clone()).unwrap_or_default();
            if !expected.is_empty() && !got.is_empty() && got != expected {
                return Err(format!("{}: the server's key changed ({got})", place.host));
            }
            return Err(format!("{}: {e}", place.host));
        }
    };
    let auth = if secret.key.trim().is_empty() {
        session
            .authenticate_password(place.user.clone(), secret.token.clone())
            .await
    } else {
        let pass = (!secret.token.is_empty()).then_some(secret.token.as_str());
        let key =
            decode_secret_key(secret.key.trim(), pass).map_err(|e| format!("private key: {e}"))?;
        let hash = session
            .best_supported_rsa_hash()
            .await
            .ok()
            .flatten()
            .flatten();
        session
            .authenticate_publickey(
                place.user.clone(),
                PrivateKeyWithHashAlg::new(Arc::new(key), hash),
            )
            .await
    }
    .map_err(|e| e.to_string())?;
    if !auth.success() {
        return Err(format!("{} refused the sign-in", place.host));
    }
    let ch = session
        .channel_open_session()
        .await
        .map_err(|e| e.to_string())?;
    ch.request_subsystem(true, "sftp")
        .await
        .map_err(|e| e.to_string())?;
    let sftp = SftpSession::new(ch.into_stream())
        .await
        .map_err(|e| format!("SFTP: {e}"))?;
    let fp = seen.lock().map(|s| s.clone()).unwrap_or_default();
    Ok((sftp, fp))
}

impl Store for SftpStore {
    fn read(&mut self, rel: &str) -> Result<Option<Vec<u8>>, String> {
        let path = self.full(rel);
        self.with(|s, rt| {
            rt.block_on(async {
                if !s.try_exists(path.clone()).await.unwrap_or(false) {
                    return Ok(None);
                }
                s.read(path.clone())
                    .await
                    .map(Some)
                    .map_err(|e| format!("{path}: {e}"))
            })
        })
    }

    fn list(&mut self, rel: &str) -> Result<Vec<String>, String> {
        let path = self.full(rel);
        self.with(|s, rt| {
            rt.block_on(async {
                if !s.try_exists(path.clone()).await.unwrap_or(false) {
                    return Ok(vec![]);
                }
                let dir = s
                    .read_dir(path.clone())
                    .await
                    .map_err(|e| format!("{path}: {e}"))?;
                Ok(dir
                    .map(|e| e.file_name())
                    .filter(|n| n != "." && n != "..")
                    .collect())
            })
        })
    }

    fn write(&mut self, rel: &str, bytes: &[u8]) -> Result<(), String> {
        let path = self.full(rel);
        self.with(|s, rt| {
            rt.block_on(async {
                // Folders on the way (the place itself too), made if missing.
                let parent = path.rsplit_once('/').map_or("", |(p, _)| p);
                let mut at = String::new();
                for p in parent.split('/').filter(|p| !p.is_empty()) {
                    at = format!("{at}/{p}");
                    if !s.try_exists(at.clone()).await.unwrap_or(false) {
                        s.create_dir(at.clone())
                            .await
                            .map_err(|e| format!("{at}: {e}"))?;
                    }
                }
                let tmp = format!("{path}.tmp");
                {
                    use tokio::io::AsyncWriteExt;
                    let mut f = s
                        .create(tmp.clone())
                        .await
                        .map_err(|e| format!("{tmp}: {e}"))?;
                    f.write_all(bytes)
                        .await
                        .map_err(|e| format!("{tmp}: {e}"))?;
                    f.shutdown().await.map_err(|e| format!("{tmp}: {e}"))?;
                }
                if s.try_exists(path.clone()).await.unwrap_or(false) {
                    let _ = s.remove_file(path.clone()).await;
                }
                s.rename(tmp.clone(), path.clone())
                    .await
                    .map_err(|e| format!("{path}: {e}"))
            })
        })
    }

    fn remove(&mut self, rel: &str) {
        let path = self.full(rel);
        let _ = self.with(|s, rt| {
            rt.block_on(async { s.remove_file(path.clone()).await.map_err(|e| e.to_string()) })
        });
    }
}

/// The store of a folder as `Local::folder` keeps it: a path, or an `sftp://` place.
pub fn store_for(
    folder: &Path,
    secret: Option<&Secret>,
    host_key: &str,
) -> Result<Box<dyn Store>, String> {
    let text = folder.to_string_lossy();
    if let Some(place) = SftpPlace::parse(&text) {
        let secret = secret.cloned().ok_or("Enter the SFTP password")?;
        return Ok(Box::new(SftpStore::new(
            place,
            secret,
            host_key.to_owned(),
        )?));
    }
    Ok(Box::new(LocalStore(folder.to_owned())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sftp_places() {
        let p = SftpPlace::parse("sftp://lea@nas.example-studio.lan:2222/share/cleis").unwrap();
        assert_eq!(
            (p.host.as_str(), p.port, p.user.as_str(), p.path.as_str()),
            ("nas.example-studio.lan", 2222, "lea", "/share/cleis")
        );
        assert_eq!(
            p.url(),
            "sftp://lea@nas.example-studio.lan:2222/share/cleis"
        );
        assert_eq!(SftpPlace::parse("sftp://lea@nas/x").unwrap().port, 22);
        assert!(SftpPlace::parse("C:\\Users\\lea\\SharePoint").is_none());
        assert!(SftpPlace::parse("sftp://nas/x").is_none());
    }

    #[test]
    fn local_store() {
        let d = tempfile::tempdir().unwrap();
        let mut s = LocalStore(d.path().to_owned());
        assert_eq!(s.read("ops/a/1.json").unwrap(), None);
        assert!(s.list("ops").unwrap().is_empty());
        s.write("ops/a/1.json", b"x").unwrap();
        assert_eq!(s.read("ops/a/1.json").unwrap().as_deref(), Some(&b"x"[..]));
        assert_eq!(s.list("ops").unwrap(), vec!["a".to_owned()]);
        s.remove("ops/a/1.json");
        assert_eq!(s.read("ops/a/1.json").unwrap(), None);
    }

    /// Against a real OpenSSH server on 127.0.0.1 (skipped without one): files go and come
    /// back, then an organization is made in a folder there and opened by a second copy.
    #[test]
    fn openssh_folder() {
        use std::process::{Command, Stdio};
        let skip = |why: &str| {
            assert!(
                std::env::var_os("CLEIS_REQUIRE_ALL").is_none(),
                "not run: {why}"
            );
            eprintln!("skipped: {why}");
        };
        let sshd = Path::new("/usr/sbin/sshd");
        if !sshd.exists() {
            return skip("no OpenSSH server");
        }
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        for name in ["host", "user"] {
            assert!(
                Command::new("ssh-keygen")
                    .args(["-q", "-t", "ed25519", "-N", "", "-f"])
                    .arg(d.join(name))
                    .status()
                    .unwrap()
                    .success()
            );
        }
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let cfg = d.join("sshd.conf");
        std::fs::write(
            &cfg,
            format!(
                "Port {port}\nListenAddress 127.0.0.1\nHostKey {}\nAuthorizedKeysFile {}\nPidFile {}\nStrictModes no\nUsePAM no\nPasswordAuthentication no\nPermitRootLogin yes\nSubsystem sftp internal-sftp\n",
                d.join("host").display(),
                d.join("user.pub").display(),
                d.join("sshd.pid").display(),
            ),
        )
        .unwrap();
        let mut child = Command::new(sshd)
            .args(["-D", "-e", "-f"])
            .arg(&cfg)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let up = (0..50).any(|_| {
            std::thread::sleep(Duration::from_millis(100));
            std::net::TcpStream::connect(("127.0.0.1", port)).is_ok()
        });
        if !up {
            let _ = child.kill();
            let _ = child.wait();
            return skip("the OpenSSH server did not start");
        }
        let user = String::from_utf8(Command::new("id").arg("-un").output().unwrap().stdout)
            .unwrap()
            .trim()
            .to_owned();
        let remote = d.join("remote/Example Studio");
        let place = SftpPlace {
            host: "127.0.0.1".into(),
            port,
            user,
            path: remote.display().to_string(),
        };
        let secret = Secret {
            token: String::new(),
            key: std::fs::read_to_string(d.join("user")).unwrap(),
        };
        let mut st = SftpStore::new(place.clone(), secret.clone(), String::new()).unwrap();
        assert_eq!(st.read("ops/a/1.json").unwrap(), None);
        st.write("ops/a/1.json", b"sealed").unwrap();
        assert_eq!(
            st.read("ops/a/1.json").unwrap().as_deref(),
            Some(&b"sealed"[..])
        );
        assert_eq!(st.list("ops").unwrap(), vec!["a".to_owned()]);
        st.remove("ops/a/1.json");
        assert!(st.host_key.starts_with("SHA256:"));
        let key = st.host_key.clone();

        let sign = cleis_crypto::SigningKeyPair::generate().unwrap();
        let acc = cleis_crypto::KeyPair::generate().unwrap();
        let made = crate::folder_org::Engine::create_in(
            Box::new(st),
            &place.url(),
            &d.join("replica-a"),
            "Example Studio",
            "Lea Martin",
            acc.public().0,
            cleis_crypto::SigningKeyPair::from_secret_bytes(*sign.secret_bytes()),
            "PC",
        );
        let other = SftpStore::new(place.clone(), secret, key).unwrap();
        let opened = crate::folder_org::Engine::open_store(
            Box::new(other),
            &place.url(),
            &d.join("replica-b"),
            None,
            "LAPTOP",
            cleis_crypto::SigningKeyPair::generate().unwrap(),
        );
        child.kill().unwrap();
        child.wait().unwrap();
        made.unwrap();
        opened.unwrap();
        assert!(remote.join("genesis.db").is_file());
    }
}
