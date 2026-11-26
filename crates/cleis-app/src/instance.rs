//! One Cleis per user (D-75). The first copy holds a lock on `instance.lock` and listens on a
//! loopback port written, with a random token, to `instance.port` (in the user's own profile).
//! A second launch asks it to come in front and exits; `Cleis --quit` (the installer) asks it to
//! quit. The lock goes with the process, so a crash never leaves Cleis refusing to start.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// What another launch asks of the running copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    Show,
    Quit,
}

/// Held by the running copy for its whole life.
pub struct Guard {
    _lock: File,
    listener: TcpListener,
    token: String,
}

fn dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "matv-studio", "Cleis").map(|d| d.data_local_dir().into())
}

/// Takes the lock in `dir`; `None` when another copy holds it.
pub fn claim_in(dir: &Path) -> std::io::Result<Option<Guard>> {
    std::fs::create_dir_all(dir)?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join("instance.lock"))?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => return Ok(None),
        Err(std::fs::TryLockError::Error(e)) => return Err(e),
    }
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let raw: [u8; 16] =
        cleis_crypto::random_bytes().map_err(|e| std::io::Error::other(e.to_string()))?;
    let token: String = raw.iter().map(|b| format!("{b:02x}")).collect();
    std::fs::write(
        dir.join("instance.port"),
        format!("{} {token}", listener.local_addr()?.port()),
    )?;
    Ok(Some(Guard {
        _lock: lock,
        listener,
        token,
    }))
}

/// [`claim_in`] the user's data folder (`Ok(None)`: another copy runs).
pub fn claim() -> std::io::Result<Option<Guard>> {
    let dir = dir().ok_or_else(|| std::io::Error::other("no data folder"))?;
    claim_in(&dir)
}

/// Sends `r` to the running copy of `dir`; `false` if none answered.
pub fn ask_in(dir: &Path, r: Request) -> bool {
    let Ok(text) = std::fs::read_to_string(dir.join("instance.port")) else {
        return false;
    };
    let mut parts = text.split_whitespace();
    let (Some(port), Some(token)) = (parts.next(), parts.next()) else {
        return false;
    };
    let Ok(port) = port.parse::<u16>() else {
        return false;
    };
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let Ok(mut s) = TcpStream::connect_timeout(&addr, Duration::from_secs(2)) else {
        return false;
    };
    let word = match r {
        Request::Show => "show",
        Request::Quit => "quit",
    };
    let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
    if writeln!(s, "{token} {word}").is_err() {
        return false;
    }
    let mut reply = String::new();
    BufReader::new(s).read_line(&mut reply).is_ok() && reply.trim() == "ok"
}

pub fn ask(r: Request) -> bool {
    dir().is_some_and(|d| ask_in(&d, r))
}

/// Waits until no copy holds the lock of `dir` (after [`Request::Quit`]).
pub fn wait_gone_in(dir: &Path, limit: Duration) -> bool {
    let end = Instant::now() + limit;
    loop {
        if let Ok(f) = File::open(dir.join("instance.lock"))
            && f.try_lock().is_err()
        {
            if Instant::now() >= end {
                return false;
            }
            std::thread::sleep(Duration::from_millis(100));
            continue;
        }
        return true;
    }
}

pub fn wait_gone(limit: Duration) -> bool {
    dir().is_none_or(|d| wait_gone_in(&d, limit))
}

impl Guard {
    /// Answers the other launches on a thread of its own; each request goes to `f`.
    pub fn serve(self, f: impl Fn(Request) + Send + 'static) {
        std::thread::spawn(move || {
            let _lock = self._lock;
            for conn in self.listener.incoming().flatten() {
                let _ = conn.set_read_timeout(Some(Duration::from_secs(2)));
                let mut line = String::new();
                let mut reader = BufReader::new(&conn);
                if reader.read_line(&mut line).is_err() {
                    continue;
                }
                let mut parts = line.split_whitespace();
                if parts.next() != Some(self.token.as_str()) {
                    continue;
                }
                let r = match parts.next() {
                    Some("show") => Request::Show,
                    Some("quit") => Request::Quit,
                    _ => continue,
                };
                let _ = writeln!(&conn, "ok");
                f(r);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn one_copy_answers_the_others() {
        let dir = tempfile::tempdir().unwrap();
        let first = claim_in(dir.path()).unwrap().expect("the first copy runs");
        assert!(
            claim_in(dir.path()).unwrap().is_none(),
            "a second copy is refused"
        );
        let (tx, rx) = mpsc::channel();
        first.serve(move |r| tx.send(r).unwrap());
        assert!(ask_in(dir.path(), Request::Show));
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            Request::Show
        );
        assert!(ask_in(dir.path(), Request::Quit));
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            Request::Quit
        );
        // A wrong token is ignored.
        std::fs::write(dir.path().join("instance.port"), {
            let t = std::fs::read_to_string(dir.path().join("instance.port")).unwrap();
            format!(
                "{} {}",
                t.split_whitespace().next().unwrap(),
                "0".repeat(32)
            )
        })
        .unwrap();
        assert!(!ask_in(dir.path(), Request::Show));
        assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());
    }

    #[test]
    fn nobody_running() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!ask_in(dir.path(), Request::Show));
        assert!(wait_gone_in(dir.path(), Duration::from_millis(10)));
    }
}
