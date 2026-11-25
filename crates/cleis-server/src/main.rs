//! `cleis-server`: the organization server (D-32, D-63). One binary for Linux, Windows and macOS,
//! or a container; any network (VPN address, local network, Internet): the apps trust it by its
//! pinned certificate, not by its name.
//!
//! ```text
//! cleis-server init      --data DIR --org NAME [--listen ADDR:PORT]... [--host NAME]...
//! cleis-server run       --data DIR [--listen ADDR:PORT]...       serve (addresses from cleis-server.toml)
//! cleis-server install   --data DIR [--print]                       run at start-up as a system service
//! cleis-server uninstall
//! cleis-server invite    --data DIR --email E [--name N] [--role ROLE]
//! cleis-server backup    --data DIR [--to DIR]                      copy the server file now
//! cleis-server restore   --data DIR FILE                           put a copy back (server stopped)
//! cleis-server rotate    --data DIR --finish                        switch to the announced certificate now
//! cleis-server status    --data DIR
//! ```

use cleis_server::config::Config;
use cleis_server::{App, ServerError, db, finish_rotation};
use std::path::{Path, PathBuf};

struct Args {
    cmd: String,
    data: PathBuf,
    org: String,
    hosts: Vec<String>,
    listen: Vec<String>,
    email: String,
    name: String,
    role: String,
    to: Option<PathBuf>,
    file: Option<PathBuf>,
    finish: bool,
    print: bool,
}

fn default_data() -> PathBuf {
    if let Ok(d) = std::env::var("CLEIS_DATA") {
        return PathBuf::from(d);
    }
    if cfg!(windows) {
        std::env::var("ProgramData")
            .map(|p| PathBuf::from(p).join("Cleis Server"))
            .unwrap_or_else(|_| PathBuf::from("data"))
    } else if cfg!(target_os = "macos") {
        PathBuf::from("/Library/Application Support/Cleis Server")
    } else {
        PathBuf::from("/var/lib/cleis-server")
    }
}

fn parse() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let cmd = it.next().unwrap_or_else(|| "help".into());
    let mut a = Args {
        cmd,
        data: default_data(),
        org: String::new(),
        hosts: vec![],
        listen: vec![],
        email: String::new(),
        name: String::new(),
        role: "member".into(),
        to: None,
        file: None,
        finish: false,
        print: false,
    };
    while let Some(k) = it.next() {
        let mut val = || it.next().ok_or_else(|| format!("{k} needs a value"));
        match k.as_str() {
            "--data" => a.data = PathBuf::from(val()?),
            "--org" => a.org = val()?,
            "--host" => a.hosts.push(val()?),
            "--listen" => a.listen.push(val()?),
            "--email" => a.email = val()?,
            "--name" => {
                let v = val()?;
                if a.cmd == "init" {
                    a.hosts.push(v);
                } else {
                    a.name = v;
                }
            }
            "--role" => a.role = val()?,
            "--to" => a.to = Some(PathBuf::from(val()?)),
            "--finish" => a.finish = true,
            "--print" => a.print = true,
            f if !f.starts_with("--") && a.file.is_none() => a.file = Some(PathBuf::from(f)),
            _ => return Err(format!("unknown option {k}")),
        }
    }
    Ok(a)
}

const HELP: &str = "Cleis server

  cleis-server init --data DIR --org NAME [--listen ADDR:PORT]... [--host NAME]...
  cleis-server run --data DIR [--listen ADDR:PORT]...
  cleis-server install --data DIR [--print]     start with the system (systemd, launchd, Windows service)
  cleis-server uninstall
  cleis-server invite --data DIR --email EMAIL [--name NAME] [--role ROLE]
  cleis-server backup --data DIR [--to DIR]
  cleis-server restore --data DIR FILE
  cleis-server rotate --data DIR --finish
  cleis-server status --data DIR

The data folder holds server.db and cleis-server.toml (addresses, backups, e-mail, SMS,
Google / Microsoft sign-in). Default: CLEIS_DATA, else the system's usual place.";

fn main() {
    let args = match parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}\n\n{HELP}");
            std::process::exit(2);
        }
    };
    if let Err(e) = run(args) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run(a: Args) -> Result<(), ServerError> {
    match a.cmd.as_str() {
        "init" => {
            if a.org.trim().is_empty() {
                return Err(ServerError::Setup("--org NAME is required".into()));
            }
            let code = App::init(&a.data, a.org.trim(), &a.hosts)?;
            Config::write_default(&a.data, &a.listen)?;
            let app = App::open(&a.data)?;
            let cfg = Config::load(&a.data).map_err(ServerError::Setup)?;
            println!(
                "Cleis server for {} created in {}",
                a.org.trim(),
                a.data.display()
            );
            println!(
                "Certificate: {}",
                cleis_crypto::fingerprint_of_hash(&app.cert)
            );
            println!("Listens on: {}", cfg.listen.join(", "));
            println!("First administrator's invitation code (valid 7 days):\n\n    {code}\n");
            Ok(())
        }
        "run" => serve_blocking(&a.data, &a.listen, None),
        "service" => windows::run_service(&a.data),
        "install" => install(&a.data, a.print),
        "uninstall" => uninstall(),
        "invite" => {
            let app = App::open(&a.data)?;
            if !a.email.contains('@') {
                return Err(ServerError::Setup("--email is required".into()));
            }
            let code = app.invite(&a.name, &a.email, &a.role)?;
            println!("Invitation for {} (valid 7 days):\n\n    {code}\n", a.email);
            Ok(())
        }
        "backup" => {
            let app = App::open(&a.data)?;
            let cfg = Config::load(&a.data).map_err(ServerError::Setup)?;
            let dir =
                a.to.or_else(|| cfg.backups_dir(&a.data))
                    .unwrap_or_else(|| a.data.join("backups"));
            let p = app.backup_to(&dir, cfg.backups.keep.max(1))?;
            println!("{}", p.display());
            Ok(())
        }
        "restore" => {
            let file = a.file.ok_or_else(|| {
                ServerError::Setup("which file? cleis-server restore --data DIR FILE".into())
            })?;
            restore(&a.data, &file)
        }
        "rotate" if a.finish => {
            let conn = db::open(&a.data.join(cleis_server::DB_FILE))?;
            match (
                db::meta_get(&conn, "next_cert")?,
                db::meta_get(&conn, "next_cert_key")?,
            ) {
                (Some(c), Some(k)) => {
                    finish_rotation(&conn, &c, &k)?;
                    println!(
                        "New certificate in use: {}",
                        cleis_crypto::fingerprint_of_hash(&cleis_crypto::sha256(&c))
                    );
                    Ok(())
                }
                _ => Err(ServerError::Setup(
                    "no rotation announced (Settings › Encryption › Rotate)".into(),
                )),
            }
        }
        "status" => {
            let app = App::open(&a.data)?;
            let cfg = Config::load(&a.data).map_err(ServerError::Setup)?;
            let conn = app.db.lock().expect("database lock");
            let n: i64 = conn.query_row(
                "SELECT count(*) FROM members WHERE revoked IS NULL",
                [],
                |r| r.get(0),
            )?;
            let r: i64 = conn.query_row("SELECT count(*) FROM records", [], |r| r.get(0))?;
            println!(
                "{} members · {} items · certificate {} · listens on {}",
                n,
                r,
                cleis_crypto::fingerprint_of_hash(&app.cert),
                cfg.listen.join(", ")
            );
            Ok(())
        }
        _ => {
            println!("{HELP}");
            Ok(())
        }
    }
}

/// Serves until Ctrl+C (or `stop` resolves, for the Windows service).
fn serve_blocking(
    data: &Path,
    listen: &[String],
    stop: Option<tokio::sync::oneshot::Receiver<()>>,
) -> Result<(), ServerError> {
    let cfg = Config::load(data).map_err(ServerError::Setup)?;
    let addrs = if listen.is_empty() {
        cfg.listen.clone()
    } else {
        listen.to_vec()
    };
    let app = App::open(data)?;
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let mut listeners = vec![];
        for addr in &addrs {
            match tokio::net::TcpListener::bind(addr).await {
                Ok(l) => listeners.push(l),
                // "[::]:8443" may already cover IPv4 on some systems ("0.0.0.0:8443" then fails).
                Err(e) if addrs.len() > 1 => eprintln!("{addr}: {e}"),
                Err(e) => return Err(ServerError::Setup(format!("{addr}: {e}"))),
            }
        }
        if listeners.is_empty() {
            return Err(ServerError::Setup("no address to listen on".into()));
        }
        let bound: Vec<String> = listeners
            .iter()
            .filter_map(|l| l.local_addr().ok())
            .map(|a| a.to_string())
            .collect();
        println!(
            "Cleis server listening on {} · certificate {}",
            bound.join(", "),
            cleis_crypto::fingerprint_of_hash(&app.cert)
        );
        if let Some(dir) = cfg.backups_dir(data) {
            tokio::spawn(cleis_server::backup_loop(
                app.clone(),
                dir,
                cfg.backups.every_hours,
                cfg.backups.keep,
            ));
        }
        cleis_server::serve_all(app, listeners, async move {
            match stop {
                Some(rx) => {
                    let _ = rx.await;
                }
                None => {
                    let _ = tokio::signal::ctrl_c().await;
                }
            }
        })
        .await
    })
}

/// Puts a copy made by `backup` (or the app's organization backup, opened) in place.
fn restore(data: &Path, file: &Path) -> Result<(), ServerError> {
    let other = rusqlite::Connection::open(file)?;
    let ok: String = other.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    if ok != "ok" || db::meta_get(&other, "org_id")?.is_none() {
        return Err(ServerError::Setup("not a Cleis server file".into()));
    }
    drop(other);
    let path = data.join(cleis_server::DB_FILE);
    if path.exists() {
        let keep = data.join(format!("{}.before-restore", cleis_server::DB_FILE));
        std::fs::rename(&path, &keep)?;
        println!("Previous file kept as {}", keep.display());
    }
    for s in ["-wal", "-shm"] {
        let _ = std::fs::remove_file(data.join(format!("{}{s}", cleis_server::DB_FILE)));
    }
    std::fs::create_dir_all(data)?;
    std::fs::copy(file, &path)?;
    println!("Restored. Start the server again.");
    Ok(())
}

// ---------- System services ----------

fn data_abs(data: &Path) -> Result<PathBuf, ServerError> {
    std::fs::create_dir_all(data)?;
    Ok(std::fs::canonicalize(data)?)
}

/// systemd unit (Linux).
/// `user`: the owner of a data folder kept in a home (server hosted by the Cleis app), so the
/// files stay theirs.
pub fn systemd_unit(exe: &Path, data: &Path, user: Option<u32>) -> String {
    let user = user.map_or(String::new(), |u| format!("User={u}\n"));
    format!(
        "[Unit]\n\
         Description=Cleis organization server\n\
         After=network-online.target\n\
         Wants=network-online.target\n\n\
         [Service]\n\
         {}ExecStart=\"{}\" run --data \"{}\"\n\
         Restart=on-failure\n\
         RestartSec=5\n\
         NoNewPrivileges=yes\n\
         PrivateTmp=yes\n\
         ProtectSystem=strict\n\
         ProtectHome=read-only\n\
         ReadWritePaths=\"{}\"\n\n\
         [Install]\n\
         WantedBy=multi-user.target\n",
        user,
        exe.display(),
        data.display(),
        data.display()
    )
}

/// Owner of the data folder when it is not the administrator's.
fn data_owner(data: &Path) -> Option<u32> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        std::fs::metadata(data)
            .ok()
            .map(|m| m.uid())
            .filter(|u| *u != 0)
    }
    #[cfg(not(unix))]
    {
        let _ = data;
        None
    }
}

/// launchd daemon (macOS).
pub fn launchd_plist(exe: &Path, data: &Path) -> String {
    let esc = |p: &Path| {
        p.display()
            .to_string()
            .replace('&', "&amp;")
            .replace('<', "&lt;")
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\"><dict>\n\
         <key>Label</key><string>{LAUNCHD_LABEL}</string>\n\
         <key>ProgramArguments</key><array><string>{}</string><string>run</string><string>--data</string><string>{}</string></array>\n\
         <key>RunAtLoad</key><true/>\n\
         <key>KeepAlive</key><true/>\n\
         <key>StandardOutPath</key><string>{}/server.log</string>\n\
         <key>StandardErrorPath</key><string>{}/server.log</string>\n\
         </dict></plist>\n",
        esc(exe),
        esc(data),
        esc(data),
        esc(data)
    )
}

const LAUNCHD_LABEL: &str = "studio.matv.cleis-server";
const SYSTEMD_PATH: &str = "/etc/systemd/system/cleis-server.service";
const LAUNCHD_PATH: &str = "/Library/LaunchDaemons/studio.matv.cleis-server.plist";

fn sh(cmd: &str, args: &[&str]) -> Result<(), ServerError> {
    let st = std::process::Command::new(cmd).args(args).status()?;
    if st.success() {
        Ok(())
    } else {
        Err(ServerError::Setup(format!(
            "{cmd} {} failed",
            args.join(" ")
        )))
    }
}

fn install(data: &Path, print: bool) -> Result<(), ServerError> {
    let exe = std::env::current_exe()?;
    let data = data_abs(data)?;
    if !data.join(cleis_server::DB_FILE).exists() {
        return Err(ServerError::Setup(format!(
            "no Cleis server in {} (run `cleis-server init` first)",
            data.display()
        )));
    }
    if cfg!(windows) {
        if print {
            println!(
                "sc.exe create CleisServer binPath= \"\\\"{}\\\" service --data \\\"{}\\\"\" start= auto",
                exe.display(),
                data.display()
            );
            return Ok(());
        }
        return windows::install(&exe, &data);
    }
    let (path, text) = if cfg!(target_os = "macos") {
        (LAUNCHD_PATH, launchd_plist(&exe, &data))
    } else {
        (SYSTEMD_PATH, systemd_unit(&exe, &data, data_owner(&data)))
    };
    if print {
        println!("# {path}\n{text}");
        return Ok(());
    }
    if let Err(e) = std::fs::write(path, &text) {
        println!("# {path}\n{text}");
        return Err(ServerError::Setup(format!(
            "{path}: {e}. Run as administrator (sudo), or save the text above there yourself."
        )));
    }
    if cfg!(target_os = "macos") {
        sh("launchctl", &["bootstrap", "system", path])?;
    } else {
        sh("systemctl", &["daemon-reload"])?;
        sh("systemctl", &["enable", "--now", "cleis-server"])?;
    }
    println!("Installed: Cleis server starts with the system.");
    Ok(())
}

fn uninstall() -> Result<(), ServerError> {
    if cfg!(windows) {
        return windows::uninstall();
    }
    if cfg!(target_os = "macos") {
        let _ = sh(
            "launchctl",
            &["bootout", &format!("system/{LAUNCHD_LABEL}")],
        );
        std::fs::remove_file(LAUNCHD_PATH)?;
    } else {
        let _ = sh("systemctl", &["disable", "--now", "cleis-server"]);
        std::fs::remove_file(SYSTEMD_PATH)?;
        let _ = sh("systemctl", &["daemon-reload"]);
    }
    println!("Uninstalled. The data folder is kept.");
    Ok(())
}

#[cfg(windows)]
mod windows {
    //! Windows service through the Service Control Manager (`windows-service`).
    use super::*;
    use std::ffi::OsString;
    use std::sync::Mutex;
    use std::time::Duration;
    use windows_service::service::{
        ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode,
        ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
    use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
    use windows_service::{define_windows_service, service_dispatcher};

    const NAME: &str = "CleisServer";
    static DATA: Mutex<Option<PathBuf>> = Mutex::new(None);

    define_windows_service!(ffi_main, service_main);

    pub fn run_service(data: &Path) -> Result<(), ServerError> {
        *DATA.lock().expect("data lock") = Some(data.to_owned());
        service_dispatcher::start(NAME, ffi_main)
            .map_err(|e| ServerError::Setup(format!("service: {e}")))
    }

    fn service_main(_args: Vec<OsString>) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let tx = Mutex::new(Some(tx));
        let handler = move |c| match c {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                if let Some(tx) = tx.lock().ok().and_then(|mut t| t.take()) {
                    let _ = tx.send(());
                }
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        };
        let Ok(status) = service_control_handler::register(NAME, handler) else {
            return;
        };
        let report = |state, accept| {
            let _ = status.set_service_status(ServiceStatus {
                service_type: ServiceType::OWN_PROCESS,
                current_state: state,
                controls_accepted: accept,
                exit_code: ServiceExitCode::Win32(0),
                checkpoint: 0,
                wait_hint: Duration::from_secs(10),
                process_id: None,
            });
        };
        report(
            ServiceState::Running,
            ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        );
        let data = DATA
            .lock()
            .ok()
            .and_then(|d| d.clone())
            .unwrap_or_else(default_data);
        if let Err(e) = serve_blocking(&data, &[], Some(rx)) {
            let _ = std::fs::write(data.join("service-error.txt"), e.to_string());
        }
        report(ServiceState::Stopped, ServiceControlAccept::empty());
    }

    fn err(e: windows_service::Error) -> ServerError {
        ServerError::Setup(format!("{e} (run as administrator)"))
    }

    pub fn install(exe: &Path, data: &Path) -> Result<(), ServerError> {
        let manager = ServiceManager::local_computer(
            None::<&str>,
            ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
        )
        .map_err(err)?;
        let info = ServiceInfo {
            name: NAME.into(),
            display_name: "Cleis server".into(),
            service_type: ServiceType::OWN_PROCESS,
            start_type: ServiceStartType::AutoStart,
            error_control: ServiceErrorControl::Normal,
            executable: exe.to_owned(),
            launch_arguments: vec![
                "service".into(),
                "--data".into(),
                data.as_os_str().to_owned(),
            ],
            dependencies: vec![],
            account_name: None,
            account_password: None,
        };
        let service = manager
            .create_service(&info, ServiceAccess::START | ServiceAccess::CHANGE_CONFIG)
            .map_err(err)?;
        let _ = service
            .set_description("Cleis organization server (encrypted spaces, members, activity log)");
        service.start::<&str>(&[]).map_err(err)?;
        println!("Installed: Cleis server starts with Windows (service CleisServer).");
        Ok(())
    }

    pub fn uninstall() -> Result<(), ServerError> {
        let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
            .map_err(err)?;
        let service = manager
            .open_service(
                NAME,
                ServiceAccess::STOP | ServiceAccess::DELETE | ServiceAccess::QUERY_STATUS,
            )
            .map_err(err)?;
        let _ = service.stop();
        service.delete().map_err(err)?;
        println!("Uninstalled. The data folder is kept.");
        Ok(())
    }
}

#[cfg(not(windows))]
mod windows {
    use super::*;
    pub fn run_service(_: &Path) -> Result<(), ServerError> {
        Err(ServerError::Setup(
            "`service` is for Windows; use `run`".into(),
        ))
    }
    pub fn install(_: &Path, _: &Path) -> Result<(), ServerError> {
        Err(ServerError::Setup("Windows only".into()))
    }
    pub fn uninstall() -> Result<(), ServerError> {
        Err(ServerError::Setup("Windows only".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_files() {
        let u = systemd_unit(
            Path::new("/usr/local/bin/cleis-server"),
            Path::new("/var/lib/cleis-server"),
            None,
        );
        assert!(!u.contains("User="));
        assert!(
            systemd_unit(
                Path::new("/x"),
                Path::new("/home/lea/.local/share/cleis/server"),
                Some(1000)
            )
            .contains("User=1000\nExecStart")
        );
        assert!(u.contains(
            "ExecStart=\"/usr/local/bin/cleis-server\" run --data \"/var/lib/cleis-server\""
        ));
        assert!(u.contains("ReadWritePaths=\"/var/lib/cleis-server\""));
        let p = launchd_plist(
            Path::new("/usr/local/bin/cleis-server"),
            Path::new("/Library/Application Support/Cleis Server"),
        );
        assert!(p.contains("<string>run</string>"));
        assert!(p.contains("Cleis Server</string>"));
    }
}
