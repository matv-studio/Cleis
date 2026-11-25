//! `cleis-server`: the organization server (D-32).
//!
//! ```text
//! cleis-server init   --data DIR --org NAME [--name HOST]...   create the database, print the first admin's code
//! cleis-server run    --data DIR --listen ADDR:PORT            serve (listen on the VPN address only)
//! cleis-server invite --data DIR --email E [--name N] [--role admin|member|…]
//! cleis-server rotate --data DIR --finish                       switch to the announced certificate now
//! cleis-server status --data DIR
//! ```

#![forbid(unsafe_code)]

use cleis_server::{App, ServerError, db, finish_rotation};
use std::path::PathBuf;

struct Args {
    cmd: String,
    data: PathBuf,
    org: String,
    names: Vec<String>,
    listen: String,
    email: String,
    name: String,
    role: String,
    finish: bool,
}

fn parse() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let cmd = it.next().unwrap_or_else(|| "help".into());
    let mut a = Args {
        cmd,
        data: PathBuf::from(std::env::var("CLEIS_DATA").unwrap_or_else(|_| "data".into())),
        org: String::new(),
        names: vec![],
        listen: String::new(),
        email: String::new(),
        name: String::new(),
        role: "member".into(),
        finish: false,
    };
    while let Some(k) = it.next() {
        let mut val = || it.next().ok_or_else(|| format!("{k} needs a value"));
        match k.as_str() {
            "--data" => a.data = PathBuf::from(val()?),
            "--org" => a.org = val()?,
            "--host" => a.names.push(val()?),
            "--listen" => a.listen = val()?,
            "--email" => a.email = val()?,
            "--name" => {
                let v = val()?;
                if a.cmd == "init" {
                    a.names.push(v);
                } else {
                    a.name = v;
                }
            }
            "--role" => a.role = val()?,
            "--finish" => a.finish = true,
            _ => return Err(format!("unknown option {k}")),
        }
    }
    Ok(a)
}

const HELP: &str = "cleis-server init --data DIR --org NAME [--name HOST]
cleis-server run --data DIR --listen ADDRESS:PORT
cleis-server invite --data DIR --email EMAIL [--name NAME] [--role ROLE]
cleis-server rotate --data DIR --finish
cleis-server status --data DIR";

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
            let code = App::init(&a.data, a.org.trim(), &a.names)?;
            let app = App::open(&a.data)?;
            println!(
                "Cleis server for {} created in {}",
                a.org.trim(),
                a.data.display()
            );
            println!(
                "Certificate: {}",
                cleis_crypto::fingerprint_of_hash(&app.cert)
            );
            println!("First administrator's invitation code (valid 7 days):\n\n    {code}\n");
            Ok(())
        }
        "run" => {
            if a.listen.is_empty() {
                return Err(ServerError::Setup(
                    "--listen ADDRESS:PORT is required (the VPN address, e.g. 10.8.0.1:8443)"
                        .into(),
                ));
            }
            let app = App::open(&a.data)?;
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                let listener = tokio::net::TcpListener::bind(&a.listen).await?;
                println!(
                    "Cleis server listening on {} · certificate {}",
                    a.listen,
                    cleis_crypto::fingerprint_of_hash(&app.cert)
                );
                cleis_server::serve(app, listener, async {
                    let _ = tokio::signal::ctrl_c().await;
                })
                .await
            })
        }
        "invite" => {
            let app = App::open(&a.data)?;
            if !a.email.contains('@') {
                return Err(ServerError::Setup("--email is required".into()));
            }
            let code = app.invite(&a.name, &a.email, &a.role)?;
            println!("Invitation for {} (valid 7 days):\n\n    {code}\n", a.email);
            Ok(())
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
            let conn = app.db.lock().expect("database lock");
            let n: i64 = conn.query_row(
                "SELECT count(*) FROM members WHERE revoked IS NULL",
                [],
                |r| r.get(0),
            )?;
            let r: i64 = conn.query_row("SELECT count(*) FROM records", [], |r| r.get(0))?;
            println!(
                "{} members · {} items · certificate {}",
                n,
                r,
                cleis_crypto::fingerprint_of_hash(&app.cert)
            );
            Ok(())
        }
        _ => {
            println!("{HELP}");
            Ok(())
        }
    }
}
