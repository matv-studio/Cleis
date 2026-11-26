//! "Server on the NAS" (D-78, mockup 36f): Cleis signs in to the NAS over SSH, finds what it
//! runs, puts Cleis Server there and creates the organization.
//!
//! - QNAP (QTS, QuTS hero): the Cleis Server package (.qpkg) is sent and installed with
//!   `sh CleisServer.qpkg`; it shows in App Center and QTS starts it with the NAS. The
//!   organization's data lives on the default volume (`CleisServer.sh data`).
//! - Another Linux with systemd (Synology, a server): the static `cleis-server` goes to
//!   `/usr/local/bin`, its service is installed (`cleis-server install`, a system account).
//! - Otherwise: in the account's home, started now (not at boot): said to the user.
//!
//! Nothing is added to the NAS for Cleis (no Docker, no package manager). The password is used
//! for this connection only (and `sudo`), never kept.

use crate::remote::Secret;
use russh::ChannelMsg;
use russh::client;
use russh::keys::{HashAlg, PrivateKeyWithHashAlg, PublicKeyOrCertificate, decode_secret_key};
use russh_sftp::client::SftpSession;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Where Cleis Server is put.
#[derive(Clone, Debug)]
pub struct Nas {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub secret: Secret,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum System {
    Qnap,
    Systemd,
    /// No way to start with the system that Cleis knows: started now, in the account's home.
    Plain,
}

/// What the NAS runs (read before anything is installed).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub system: System,
    /// `x86_64` or `arm_64` (QNAP's names; the packages are named after them).
    pub arch: &'static str,
    pub root: bool,
    /// The SSH server's key, as shown to the user (SHA256:…).
    pub host_key: String,
}

impl Found {
    /// File to send: the QNAP package, or the static server.
    pub fn package(&self, version: &str) -> String {
        match self.system {
            System::Qnap => format!("CleisServer_{version}_{}.qpkg", self.arch),
            _ => format!("cleis-server-{}", self.arch),
        }
    }
}

/// Port Cleis Server listens on (its default).
pub const PORT: u16 = 8443;

struct Keep {
    seen: Arc<Mutex<String>>,
}

impl client::Handler for Keep {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        if let Ok(mut s) = self.seen.lock() {
            *s = key.public_key().fingerprint(HashAlg::Sha256).to_string();
        }
        Ok(true)
    }
}

fn runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())
}

async fn connect(nas: &Nas) -> Result<(client::Handle<Keep>, String), String> {
    let seen = Arc::new(Mutex::new(String::new()));
    let config = Arc::new(client::Config {
        inactivity_timeout: Some(Duration::from_secs(120)),
        ..Default::default()
    });
    let connect = client::connect(
        config,
        (nas.host.as_str(), nas.port),
        Keep { seen: seen.clone() },
    );
    let mut session = match tokio::time::timeout(Duration::from_secs(20), connect).await {
        Err(_) => return Err(format!("{}: no answer", nas.host)),
        Ok(Err(e)) => return Err(format!("{}: {e}", nas.host)),
        Ok(Ok(s)) => s,
    };
    let auth = if nas.secret.key.trim().is_empty() {
        session
            .authenticate_password(nas.user.clone(), nas.secret.token.clone())
            .await
    } else {
        let pass = (!nas.secret.token.is_empty()).then_some(nas.secret.token.as_str());
        let key = decode_secret_key(nas.secret.key.trim(), pass)
            .map_err(|e| format!("private key: {e}"))?;
        let hash = session
            .best_supported_rsa_hash()
            .await
            .ok()
            .flatten()
            .flatten();
        session
            .authenticate_publickey(
                nas.user.clone(),
                PrivateKeyWithHashAlg::new(Arc::new(key), hash),
            )
            .await
    }
    .map_err(|e| e.to_string())?;
    if !auth.success() {
        return Err(format!("{} refused the sign-in", nas.host));
    }
    let fp = seen.lock().map(|s| s.clone()).unwrap_or_default();
    Ok((session, fp))
}

/// A command's output and exit status.
struct Out {
    stdout: String,
    stderr: String,
    status: u32,
}

async fn exec(s: &client::Handle<Keep>, cmd: &str, stdin: Option<&str>) -> Result<Out, String> {
    let mut ch = s.channel_open_session().await.map_err(|e| e.to_string())?;
    ch.exec(true, cmd).await.map_err(|e| e.to_string())?;
    if let Some(input) = stdin {
        ch.data(input.as_bytes()).await.map_err(|e| e.to_string())?;
    }
    ch.eof().await.map_err(|e| e.to_string())?;
    let (mut out, mut err, mut status) = (Vec::new(), Vec::new(), None);
    while let Some(msg) = ch.wait().await {
        match msg {
            ChannelMsg::Data { data } => out.extend_from_slice(&data),
            ChannelMsg::ExtendedData { data, .. } => err.extend_from_slice(&data),
            ChannelMsg::ExitStatus { exit_status } => status = Some(exit_status),
            _ => {}
        }
    }
    Ok(Out {
        stdout: String::from_utf8_lossy(&out).into_owned(),
        stderr: String::from_utf8_lossy(&err).into_owned(),
        status: status.unwrap_or(255),
    })
}

/// A word for the remote shell, quoted.
pub fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Runs `script` with administrator rights: as is for root, else through `sudo` (QTS 5, DSM),
/// the password given on its input.
async fn as_root(
    s: &client::Handle<Keep>,
    nas: &Nas,
    root: bool,
    script: &str,
) -> Result<Out, String> {
    if root {
        exec(s, &format!("sh -c {}", quote(script)), None).await
    } else {
        let pw = format!("{}\n", nas.secret.token);
        exec(
            s,
            &format!("sudo -S -p '' sh -c {}", quote(script)),
            Some(&pw),
        )
        .await
    }
}

/// What the probe prints, read (one word per line).
pub fn read_probe(text: &str) -> Result<(System, &'static str, bool), String> {
    let words: Vec<&str> = text.lines().map(str::trim).collect();
    let arch = match words.first().copied().unwrap_or_default() {
        "x86_64" | "amd64" => "x86_64",
        "aarch64" | "arm64" => "arm_64",
        other => {
            return Err(format!(
                "Cleis Server does not run on this processor ({other})"
            ));
        }
    };
    let root = words.get(1).copied() == Some("0");
    let system = if words.contains(&"QNAP") {
        System::Qnap
    } else if words.contains(&"SYSTEMD") {
        System::Systemd
    } else {
        System::Plain
    };
    Ok((system, arch, root))
}

const PROBE: &str = "uname -m; id -u; \
    [ -f /etc/config/qpkg.conf ] && echo QNAP; \
    [ -d /run/systemd/system ] && command -v systemctl >/dev/null && echo SYSTEMD; true";

/// Signs in and looks at the NAS; nothing is changed.
pub fn probe(nas: &Nas) -> Result<Found, String> {
    runtime()?.block_on(async {
        let (s, host_key) = connect(nas).await?;
        let out = exec(&s, PROBE, None).await?;
        let (system, arch, root) = read_probe(&out.stdout)?;
        Ok(Found {
            system,
            arch,
            root,
            host_key,
        })
    })
}

/// The first administrator's invitation code, in what `cleis-server init` printed.
pub fn code_in(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|l| cleis_core::org::Invitation::parse(l).is_some())
        .map(str::to_owned)
}

/// Sends `package`, installs it, creates the organization `org` (its address: `host`) and starts
/// the server. Returns the first administrator's invitation code; `step` tells the progress.
pub fn install(
    nas: &Nas,
    found: &Found,
    org: &str,
    package: &[u8],
    step: impl Fn(&'static str),
) -> Result<String, String> {
    runtime()?.block_on(async {
        let (s, _) = connect(nas).await?;
        step("Sending Cleis Server");
        let tmp = "/tmp/cleis-install";
        exec(&s, &format!("mkdir -p {tmp} && chmod 700 {tmp}"), None).await?;
        let file = format!("{tmp}/{}", if found.system == System::Qnap {
            "CleisServer.qpkg"
        } else {
            "cleis-server"
        });
        {
            let ch = s.channel_open_session().await.map_err(|e| e.to_string())?;
            ch.request_subsystem(true, "sftp")
                .await
                .map_err(|e| e.to_string())?;
            let sftp = SftpSession::new(ch.into_stream())
                .await
                .map_err(|e| format!("SFTP: {e}"))?;
            let mut f = sftp
                .create(file.clone())
                .await
                .map_err(|e| format!("{file}: {e}"))?;
            use tokio::io::AsyncWriteExt;
            f.write_all(package)
                .await
                .map_err(|e| format!("{file}: {e}"))?;
            f.shutdown().await.map_err(|e| format!("{file}: {e}"))?;
            let _ = sftp.close().await;
        }
        let name = quote(org);
        let host = quote(&nas.host);
        let script = match found.system {
            System::Qnap => format!(
                "set -e
# A QPKG ends with status 10, installed or not: its start script tells.
sh {file} >/dev/null 2>&1 || true
[ -x /etc/init.d/CleisServer.sh ] || {{ echo 'App Center did not install Cleis Server' >&2; exit 4; }}
D=$(/etc/init.d/CleisServer.sh data)
R=$(/sbin/getcfg CleisServer Install_Path -f /etc/config/qpkg.conf)
if [ -f \"$D/server.db\" ]; then echo 'An organization already lives on this NAS' >&2; exit 3; fi
mkdir -p \"$D\"
\"$R/cleis-server\" init --data \"$D\" --org {name} --host {host}
/etc/init.d/CleisServer.sh start
rm -rf {tmp}"
            ),
            System::Systemd => format!(
                "set -e
install -m 0755 {file} /usr/local/bin/cleis-server
D=/var/lib/cleis-server
if [ -f \"$D/server.db\" ]; then echo 'An organization already lives on this machine' >&2; exit 3; fi
/usr/local/bin/cleis-server init --data \"$D\" --org {name} --host {host}
/usr/local/bin/cleis-server install --data \"$D\"
rm -rf {tmp}"
            ),
            System::Plain => format!(
                "set -e
B=\"${{CLEIS_SERVER_HOME:-$HOME/.cleis-server}}\"; D=\"$B/data\"
mkdir -p \"$B\"
install -m 0755 {file} \"$B/cleis-server\"
if [ -f \"$D/server.db\" ]; then echo 'An organization already lives on this machine' >&2; exit 3; fi
\"$B/cleis-server\" init --data \"$D\" --org {name} --host {host}
nohup \"$B/cleis-server\" run --data \"$D\" >\"$B/cleis-server.log\" 2>&1 </dev/null &
echo $! >\"$B/pid\"
rm -rf {tmp}"
            ),
        };
        step("Creating the organization");
        let root = found.root || found.system == System::Plain;
        let out = as_root(&s, nas, root, &script).await?;
        if out.status != 0 {
            let why = out
                .stderr
                .lines()
                .map(str::trim)
                .rfind(|l| !l.is_empty())
                .unwrap_or("installation failed")
                .to_owned();
            return Err(why);
        }
        code_in(&out.stdout).ok_or_else(|| "no invitation code printed".into())
    })
}

/// The file to send (`Found::package`): from `CLEIS_SERVER_PACKAGES` (a folder, for tests and
/// development builds), beside the app (`server/`), or this version's release on GitHub, checked
/// against its published checksum.
pub fn package_bytes(file: &str) -> Result<Vec<u8>, String> {
    let local = std::env::var_os("CLEIS_SERVER_PACKAGES")
        .map(std::path::PathBuf::from)
        .into_iter()
        .chain(
            std::env::current_exe()
                .ok()
                .and_then(|e| e.parent().map(|d| d.join("server"))),
        )
        .map(|d| d.join(file))
        .find(|p| p.is_file());
    if let Some(p) = local {
        return std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()));
    }
    let base = format!(
        "https://github.com/matv-studio/Cleis/releases/download/v{}/",
        env!("CARGO_PKG_VERSION")
    );
    let bytes = crate::update::download(&format!("{base}{file}"))?;
    let sums = crate::update::download(&format!("{base}{file}.sha256"))?;
    crate::update::verify(&bytes, &String::from_utf8_lossy(&sums))?;
    Ok(bytes)
}

/// Whether the server answers on the NAS (after its start).
pub fn answers(host: &str) -> bool {
    use std::net::ToSocketAddrs;
    let Ok(mut addrs) = (host, PORT).to_socket_addrs() else {
        return false;
    };
    addrs.any(|a| std::net::TcpStream::connect_timeout(&a, Duration::from_secs(3)).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_is_read() {
        assert_eq!(
            read_probe("x86_64\n0\nQNAP\n").unwrap(),
            (System::Qnap, "x86_64", true)
        );
        assert_eq!(
            read_probe("aarch64\n1026\nSYSTEMD\n").unwrap(),
            (System::Systemd, "arm_64", false)
        );
        assert_eq!(
            read_probe("x86_64\n1000\n").unwrap(),
            (System::Plain, "x86_64", false)
        );
        assert!(read_probe("armv7l\n0\n").is_err());
    }

    #[test]
    fn names_and_quotes() {
        let f = Found {
            system: System::Qnap,
            arch: "x86_64",
            root: true,
            host_key: String::new(),
        };
        assert_eq!(f.package("0.2.2"), "CleisServer_0.2.2_x86_64.qpkg");
        assert_eq!(quote("L'atelier"), r"'L'\''atelier'");
    }

    /// Against a real OpenSSH server on 127.0.0.1 (skipped without one, or without a built
    /// `cleis-server`): the probe, then the "plain" way, which needs neither root nor systemd.
    #[test]
    fn openssh_plain_install() {
        use std::process::{Command, Stdio};
        let skip = |why: &str| {
            assert!(
                std::env::var_os("CLEIS_REQUIRE_ALL").is_none(),
                "not run: {why}"
            );
            eprintln!("skipped: {why}");
        };
        let sshd = std::path::Path::new("/usr/sbin/sshd");
        let Some(server) = crate::host::server_exe() else {
            return skip("cleis-server is not built");
        };
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
        let home = d.join("nas-home");
        let cfg = d.join("sshd.conf");
        std::fs::write(
            &cfg,
            format!(
                "Port {port}\nListenAddress 127.0.0.1\nHostKey {}\nAuthorizedKeysFile {}\nPidFile {}\nStrictModes no\nUsePAM no\nPasswordAuthentication no\nPermitRootLogin yes\nSubsystem sftp internal-sftp\nSetEnv CLEIS_SERVER_HOME={}\n",
                d.join("host").display(),
                d.join("user.pub").display(),
                d.join("sshd.pid").display(),
                home.display(),
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
        let nas = Nas {
            host: "127.0.0.1".into(),
            port,
            user,
            secret: Secret {
                token: String::new(),
                key: std::fs::read_to_string(d.join("user")).unwrap(),
            },
        };
        let mut found = probe(&nas).unwrap();
        assert!(found.host_key.starts_with("SHA256:"), "{found:?}");
        assert_eq!(found.arch, "x86_64");
        // This machine may run systemd: the plain way is the one tried here.
        found.system = System::Plain;
        let bin = std::fs::read(server).unwrap();
        let steps = std::cell::RefCell::new(vec![]);
        let code = install(&nas, &found, "L'atelier Example", &bin, |s| {
            steps.borrow_mut().push(s)
        });
        let pid = std::fs::read_to_string(home.join("pid")).unwrap_or_default();
        if !pid.trim().is_empty() {
            let _ = Command::new("kill").arg(pid.trim()).status();
        }
        child.kill().unwrap();
        child.wait().unwrap();
        let code = code.unwrap();
        assert!(cleis_core::org::Invitation::parse(&code).is_some());
        assert!(home.join("data/server.db").exists());
        assert_eq!(steps.into_inner().len(), 2);
    }
}
