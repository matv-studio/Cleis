//! SFTP sync place (D-65), over `russh` / `russh-sftp` (pure Rust). The server's key is kept at
//! the first connection (shown to the user) and checked at every later one.

use crate::remote::{RemoteError, RemotePlace, Secret, Version, write_atomic};
use russh::client;
use russh::keys::{HashAlg, PrivateKeyWithHashAlg, PublicKeyOrCertificate, decode_secret_key};
use russh_sftp::client::SftpSession;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct Checker {
    expected: String,
    seen: Arc<Mutex<String>>,
}

impl client::Handler for Checker {
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

fn runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())
}

/// Connects and signs in; returns the session and the server key seen.
async fn open(place: &RemotePlace, secret: &Secret) -> Result<(SftpSession, String), RemoteError> {
    let RemotePlace::Sftp {
        host,
        port,
        user,
        host_key,
        ..
    } = place
    else {
        return Err(RemoteError::Other("not an SFTP place".into()));
    };
    let seen = Arc::new(Mutex::new(String::new()));
    let config = Arc::new(client::Config {
        inactivity_timeout: Some(Duration::from_secs(30)),
        ..Default::default()
    });
    let handler = Checker {
        expected: host_key.clone(),
        seen: seen.clone(),
    };
    let connect = client::connect(config, (host.as_str(), *port), handler);
    let mut session = match tokio::time::timeout(Duration::from_secs(20), connect).await {
        Err(_) => return Err(RemoteError::Other(format!("{host}: no answer"))),
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            let got = seen.lock().map(|s| s.clone()).unwrap_or_default();
            if !host_key.is_empty() && !got.is_empty() && &got != host_key {
                return Err(RemoteError::HostKey(got));
            }
            return Err(RemoteError::Other(format!("{host}: {e}")));
        }
    };
    let auth = if secret.key.trim().is_empty() {
        session
            .authenticate_password(user.clone(), secret.token.clone())
            .await
    } else {
        let pass = (!secret.token.is_empty()).then_some(secret.token.as_str());
        let key = decode_secret_key(secret.key.trim(), pass)
            .map_err(|e| RemoteError::Other(format!("private key: {e}")))?;
        let hash = session
            .best_supported_rsa_hash()
            .await
            .ok()
            .flatten()
            .flatten();
        session
            .authenticate_publickey(
                user.clone(),
                PrivateKeyWithHashAlg::new(Arc::new(key), hash),
            )
            .await
    }
    .map_err(|e| RemoteError::Other(e.to_string()))?;
    if !auth.success() {
        return Err(RemoteError::Other(format!("{host} refused the sign-in")));
    }
    let channel = session
        .channel_open_session()
        .await
        .map_err(|e| RemoteError::Other(e.to_string()))?;
    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(|e| RemoteError::Other(e.to_string()))?;
    let sftp = SftpSession::new(channel.into_stream())
        .await
        .map_err(|e| RemoteError::Other(format!("SFTP: {e}")))?;
    let fp = seen.lock().map(|s| s.clone()).unwrap_or_default();
    Ok((sftp, fp))
}

fn path_of(place: &RemotePlace) -> String {
    match place {
        RemotePlace::Sftp { path, .. } => path.clone(),
        _ => String::new(),
    }
}

/// First connection: the server's key, to show and keep.
pub fn server_key(place: &RemotePlace, secret: &Secret) -> Result<String, RemoteError> {
    runtime()?.block_on(async {
        let (sftp, fp) = open(place, secret).await?;
        let _ = sftp.close().await;
        Ok(fp)
    })
}

pub fn download(place: &RemotePlace, secret: &Secret, to: &Path) -> Result<Version, RemoteError> {
    let path = path_of(place);
    runtime()?.block_on(async {
        let (sftp, _) = open(place, secret).await?;
        let r = if sftp.try_exists(path.clone()).await.unwrap_or(false) {
            let bytes = sftp
                .read(path.clone())
                .await
                .map_err(|e| RemoteError::Other(format!("{path}: {e}")))?;
            write_atomic(to, &bytes)?;
            Ok(None)
        } else {
            let _ = std::fs::remove_file(to);
            Ok(None)
        };
        let _ = sftp.close().await;
        r
    })
}

/// Writes beside the file, then renames it over (the copy is never seen half written).
pub fn upload(place: &RemotePlace, secret: &Secret, bytes: &[u8]) -> Result<(), RemoteError> {
    let path = path_of(place);
    runtime()?.block_on(async {
        let (sftp, _) = open(place, secret).await?;
        if let Some((dir, _)) = path.rsplit_once('/')
            && !dir.is_empty()
            && !sftp.try_exists(dir.to_owned()).await.unwrap_or(true)
        {
            sftp.create_dir(dir.to_owned())
                .await
                .map_err(|e| RemoteError::Other(format!("{dir}: {e}")))?;
        }
        let part = format!("{path}.part");
        {
            use tokio::io::AsyncWriteExt;
            let err = |e: &dyn std::fmt::Display| RemoteError::Other(format!("{part}: {e}"));
            let mut f = sftp.create(part.clone()).await.map_err(|e| err(&e))?;
            f.write_all(bytes).await.map_err(|e| err(&e))?;
            f.shutdown().await.map_err(|e| err(&e))?;
        }
        // SFTP v3 does not rename over an existing file.
        let _ = sftp.remove_file(path.clone()).await;
        sftp.rename(part.clone(), path.clone())
            .await
            .map_err(|e| RemoteError::Other(format!("{path}: {e}")))?;
        let _ = sftp.close().await;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    /// Audit X-3: a skipped test fails where everything must run (`CLEIS_REQUIRE_ALL`, Linux CI).
    fn skip(why: &str) {
        assert!(
            std::env::var_os("CLEIS_REQUIRE_ALL").is_none(),
            "not run: {why}"
        );
        eprintln!("skipped: {why}");
    }

    /// Against a real OpenSSH server on 127.0.0.1, when one is installed (skipped otherwise):
    /// first connection gives the key, files go and come back, another key is refused.
    #[test]
    fn openssh_round_trip() {
        let sshd = Path::new("/usr/sbin/sshd");
        if !sshd.exists() || Command::new("ssh-keygen").arg("-?").output().is_err() {
            skip("no OpenSSH server");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let keygen = |name: &str| {
            let ok = Command::new("ssh-keygen")
                .args(["-q", "-t", "ed25519", "-N", "", "-f"])
                .arg(d.join(name))
                .status()
                .unwrap()
                .success();
            assert!(ok);
        };
        keygen("host");
        keygen("user");
        keygen("other_host");
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let user = String::from_utf8(Command::new("id").arg("-un").output().unwrap().stdout)
            .unwrap()
            .trim()
            .to_owned();
        let conf = |host: &str| {
            format!(
                "Port {port}\nListenAddress 127.0.0.1\nHostKey {}\nAuthorizedKeysFile {}\nPidFile {}\nStrictModes no\nUsePAM no\nPasswordAuthentication no\nPermitRootLogin yes\nSubsystem sftp internal-sftp\n",
                d.join(host).display(),
                d.join("user.pub").display(),
                d.join(format!("{host}.pid")).display(),
            )
        };
        let start = |host: &str| {
            let cfg = d.join(format!("{host}.conf"));
            std::fs::write(&cfg, conf(host)).unwrap();
            let child = Command::new(sshd)
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
            (child, up)
        };
        let (mut server, up) = start("host");
        if !up {
            // No privilege separation folder (/run/sshd), or no right to run sshd here.
            let _ = server.kill();
            skip("the OpenSSH server did not start");
            return;
        }
        let target = d.join("remote/vault.cleis");
        let mut place = RemotePlace::Sftp {
            host: "127.0.0.1".into(),
            port,
            user,
            path: target.display().to_string(),
            host_key: String::new(),
        };
        let secret = Secret {
            token: String::new(),
            key: std::fs::read_to_string(d.join("user")).unwrap(),
        };
        let key = server_key(&place, &secret).unwrap();
        assert!(key.starts_with("SHA256:"), "{key}");
        if let RemotePlace::Sftp { host_key, .. } = &mut place {
            *host_key = key;
        }
        let local = d.join("local.cleis");
        assert_eq!(download(&place, &secret, &local), Ok(None));
        assert!(!local.exists());
        upload(&place, &secret, b"sealed rows").unwrap();
        upload(&place, &secret, b"sealed rows, again").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"sealed rows, again");
        download(&place, &secret, &local).unwrap();
        assert_eq!(std::fs::read(&local).unwrap(), b"sealed rows, again");
        server.kill().unwrap();
        server.wait().unwrap();
        // Same address, another key: refused.
        let (mut other, _) = start("other_host");
        let r = download(&place, &secret, &local);
        other.kill().unwrap();
        other.wait().unwrap();
        assert!(matches!(r, Err(RemoteError::HostKey(_))), "{r:?}");
    }
}
