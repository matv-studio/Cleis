//! An organization hosted on this computer (D-63): Cleis runs the `cleis-server` shipped beside
//! it, in the user's data folder. It serves while Cleis runs (the tray keeps Cleis open), or
//! always once installed as a system service.

use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

pub const PORT: u16 = 8443;

/// The server process started by this app, stopped when Cleis quits.
static CHILD: Mutex<Option<Child>> = Mutex::new(None);

/// Folder chosen instead of the user's data folder (tests).
static DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn use_dir(dir: PathBuf) {
    if let Ok(mut d) = DIR.lock() {
        *d = Some(dir);
    }
}

/// `cleis-server` beside the app (portable zip, macOS bundle, development build).
pub fn server_exe() -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "cleis-server.exe"
    } else {
        "cleis-server"
    };
    let me = std::env::current_exe().ok()?;
    // Beside the app, or up to the build folder (tests are built further down).
    me.ancestors()
        .skip(1)
        .take(5)
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

/// Where this computer's server keeps its file: the first folder not used by an earlier one.
pub fn default_dir() -> PathBuf {
    if let Some(d) = DIR.lock().ok().and_then(|d| d.clone()) {
        return d;
    }
    let base = directories::ProjectDirs::from("com", "matv-studio", "Cleis")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(std::env::temp_dir);
    (1..)
        .map(|n| {
            base.join(if n == 1 {
                "server".into()
            } else {
                format!("server-{n}")
            })
        })
        .find(|d| !d.join("server.db").exists())
        .unwrap_or_else(|| base.join("server"))
}

/// First port from 8443 that nothing answers on.
pub fn free_port() -> u16 {
    (PORT..PORT + 20).find(|p| !listening(*p)).unwrap_or(PORT)
}

/// Port of an address "host:port" (8443 when absent).
pub fn port_of(address: &str) -> u16 {
    address
        .rsplit_once(':')
        .and_then(|(_, p)| p.parse().ok())
        .unwrap_or(PORT)
}

fn command(exe: &Path) -> Command {
    let mut c = Command::new(exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // No console window.
        c.creation_flags(0x0800_0000);
    }
    c.stdin(Stdio::null());
    c
}

/// Creates the server's file; returns the first administrator's invitation code.
pub fn create(exe: &Path, dir: &Path, org: &str, port: u16) -> Result<String, String> {
    if dir.join("server.db").exists() {
        return Err(format!("A server already exists in {}", dir.display()));
    }
    let out = command(exe)
        .args(["init", "--data"])
        .arg(dir)
        .args(["--org", org])
        .args(["--listen", &format!("0.0.0.0:{port}")])
        .args(["--listen", &format!("[::]:{port}")])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_owned());
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .find(|l| cleis_core::org::Invitation::parse(l).is_some())
        .map(str::to_owned)
        .ok_or_else(|| "no invitation code printed".into())
}

/// Whether something answers on this computer's port.
pub fn listening(port: u16) -> bool {
    TcpStream::connect_timeout(
        &SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(300),
    )
    .is_ok()
}

/// Starts the server unless one already answers (a system service, an earlier start).
pub fn ensure_running(exe: &Path, dir: &Path, port: u16) -> Result<(), String> {
    if listening(port) {
        return Ok(());
    }
    let log = std::fs::File::create(dir.join("server.log")).map_err(|e| e.to_string())?;
    let child = command(exe)
        .args(["run", "--data"])
        .arg(dir)
        .stdout(log.try_clone().map_err(|e| e.to_string())?)
        .stderr(log)
        .spawn()
        .map_err(|e| e.to_string())?;
    if let Ok(mut c) = CHILD.lock() {
        *c = Some(child);
    }
    for _ in 0..50 {
        if listening(port) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(std::fs::read_to_string(dir.join("server.log"))
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "the server did not start".into()))
}

/// Stops the server started by this app (not a system service).
pub fn stop() {
    if let Ok(mut c) = CHILD.lock()
        && let Some(mut child) = c.take()
    {
        let _ = child.kill();
        let _ = child.wait();
    }
}

pub fn started_here() -> bool {
    CHILD.lock().is_ok_and(|c| c.is_some())
}

/// Addresses other computers reach this one at: the main local address (the one used for the
/// way out), and the loopback for this computer.
pub fn addresses(port: u16) -> Vec<String> {
    let mut out = vec![];
    for probe in ["10.255.255.255:1", "[fd00::1]:1"] {
        let bind = if probe.starts_with('[') {
            "[::]:0"
        } else {
            "0.0.0.0:0"
        };
        if let Ok(s) = UdpSocket::bind(bind)
            && s.connect(probe).is_ok()
            && let Ok(a) = s.local_addr()
            && !a.ip().is_unspecified()
            && !a.ip().is_loopback()
        {
            out.push(SocketAddr::new(a.ip(), port).to_string());
        }
    }
    out
}

/// Installs the server as a system service, asking for administrator rights.
pub fn install_service(exe: &Path, dir: &Path) -> Result<(), String> {
    let status = if cfg!(windows) {
        let arg = |p: &Path| p.display().to_string().replace('\'', "''");
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Start-Process -FilePath '{}' -ArgumentList 'install','--data','\"{}\"' -Verb RunAs -Wait -WindowStyle Hidden",
                    arg(exe),
                    arg(dir)
                ),
            ])
            .status()
    } else if cfg!(target_os = "macos") {
        let q = |p: &Path| p.display().to_string().replace('"', "\\\"");
        Command::new("osascript")
            .args([
                "-e",
                &format!(
                    "do shell script \"'{}' install --data '{}'\" with administrator privileges",
                    q(exe),
                    q(dir)
                ),
            ])
            .status()
    } else {
        Command::new("pkexec")
            .arg(exe)
            .args(["install", "--data"])
            .arg(dir)
            .status()
    }
    .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("not installed (administrator rights refused?)".into())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn addresses_are_not_loopback() {
        for a in super::addresses(8443) {
            assert!(a.ends_with(":8443"));
            assert!(!a.starts_with("127."));
        }
    }
}
