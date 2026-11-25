//! `cleis-server.toml`, in the data folder (D-63). Every setting has a default; the command line
//! overrides the listening addresses. Written once by `init`, edited by hand afterwards.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const FILE: &str = "cleis-server.toml";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Addresses to listen on: a VPN address (`10.8.0.1:8443`), every interface (`0.0.0.0:8443`,
    /// `[::]:8443`), or several.
    pub listen: Vec<String>,
    pub backups: Backups,
    /// Codes sent by e-mail (sign-in method "Email").
    pub smtp: Option<Smtp>,
    /// Codes sent by SMS through any HTTP gateway.
    pub sms: Option<Sms>,
    /// Sign-in with Google or Microsoft accounts (OpenID Connect), by provider name.
    pub oidc: Vec<Oidc>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: vec!["0.0.0.0:8443".into(), "[::]:8443".into()],
            backups: Backups::default(),
            smtp: None,
            sms: None,
            oidc: vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Backups {
    /// Folder for the automatic copies (relative to the data folder); empty: none.
    pub dir: String,
    pub every_hours: u32,
    pub keep: u32,
}

impl Default for Backups {
    fn default() -> Self {
        Self {
            dir: "backups".into(),
            every_hours: 24,
            keep: 30,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Smtp {
    pub host: String,
    #[serde(default = "smtp_port")]
    pub port: u16,
    /// `starttls` (default), `tls` or `none` (only for a relay on the same machine).
    #[serde(default = "starttls")]
    pub security: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    /// `Cleis <vault@example-studio.com>`.
    pub from: String,
}

fn smtp_port() -> u16 {
    587
}
fn starttls() -> String {
    "starttls".into()
}

/// Any SMS gateway reached by HTTPS: `{to}` and `{code}` are replaced in the URL and the body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sms {
    pub url: String,
    #[serde(default = "post")]
    pub method: String,
    /// `["Authorization: Basic …", "Content-Type: application/x-www-form-urlencoded"]`.
    #[serde(default)]
    pub headers: Vec<String>,
    #[serde(default)]
    pub body: String,
}

fn post() -> String {
    "POST".into()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Oidc {
    /// `google`, `microsoft`, or any name.
    pub name: String,
    /// `https://accounts.google.com`, `https://login.microsoftonline.com/<tenant>/v2.0`.
    pub issuer: String,
    pub client_id: String,
    /// Desktop-app ("installed") clients of Google carry a secret that is not one: it is given to
    /// the apps with the client id. Empty for Microsoft.
    #[serde(default)]
    pub client_secret: String,
    /// E-mail domains accepted (`example-studio.com`); empty: any verified address of a member.
    #[serde(default)]
    pub domains: Vec<String>,
    /// Oldest ID token accepted, in seconds after it was issued (default 600, audit S-15).
    #[serde(default = "default_token_age")]
    pub max_token_age: i64,
}

fn default_token_age() -> i64 {
    600
}

impl Config {
    /// Sign-in methods this server can carry out (the authenticator app always).
    pub fn methods(&self) -> Vec<cleis_core::org::Method> {
        use cleis_core::org::Method;
        let mut m = vec![Method::AuthenticatorApp];
        if self.smtp.is_some() {
            m.push(Method::Email);
        }
        if self.sms.is_some() {
            m.push(Method::Sms);
        }
        for p in &self.oidc {
            if let Some(x) = crate::oidc::method_of(p)
                && !m.contains(&x)
            {
                m.push(x);
            }
        }
        m
    }

    pub fn provider(&self, method: cleis_core::org::Method) -> Option<&Oidc> {
        self.oidc
            .iter()
            .find(|p| crate::oidc::method_of(p) == Some(method))
    }

    pub fn path(data: &Path) -> PathBuf {
        data.join(FILE)
    }

    /// Reads the data folder's configuration; defaults when there is none.
    pub fn load(data: &Path) -> Result<Config, String> {
        match std::fs::read_to_string(Self::path(data)) {
            Ok(s) => toml::from_str(&s).map_err(|e| format!("{FILE}: {e}")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(format!("{FILE}: {e}")),
        }
    }

    /// Writes a commented configuration (first `init`).
    pub fn write_default(data: &Path, listen: &[String]) -> std::io::Result<()> {
        let mut c = Config::default();
        if !listen.is_empty() {
            c.listen = listen.to_vec();
        }
        let body = toml::to_string_pretty(&c).unwrap_or_default();
        let text = format!(
            "# Cleis server configuration (D-63). Restart the server after a change.\n\
             # listen: addresses to serve on (a VPN address, 0.0.0.0:8443 for every interface…).\n\
             # [backups]: automatic copies of the server file; dir relative to this folder.\n\
             #\n\
             # Optional sections:\n\
             # [smtp]  host = \"smtp.example-studio.com\"  port = 587  security = \"starttls\"\n\
             #         username = \"…\"  password = \"…\"  from = \"Cleis <vault@example-studio.com>\"\n\
             # [sms]   url = \"https://sms.example-gateway.com/send?to={{to}}\"  method = \"POST\"\n\
             #         headers = [\"Authorization: Bearer …\"]  body = \"Cleis code: {{code}}\"\n\
             #         ({{to}}, {{code}}, {{text}} are replaced; JSON or form bodies are escaped)\n\
             # [[oidc]] name = \"google\"  issuer = \"https://accounts.google.com\"\n\
             #          client_id = \"….apps.googleusercontent.com\"  client_secret = \"…\"\n\
             #          domains = [\"example-studio.com\"]\n\
             # [[oidc]] name = \"microsoft\"\n\
             #          issuer = \"https://login.microsoftonline.com/<tenant id>/v2.0\"  client_id = \"…\"\n\
             # Redirect address to register with the provider: http://127.0.0.1 (desktop app).\n\n{body}"
        );
        std::fs::write(Self::path(data), text)
    }

    pub fn backups_dir(&self, data: &Path) -> Option<PathBuf> {
        if self.backups.dir.trim().is_empty() || self.backups.every_hours == 0 {
            return None;
        }
        let p = PathBuf::from(&self.backups.dir);
        Some(if p.is_absolute() { p } else { data.join(p) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_round_trip() {
        let d = std::env::temp_dir().join(format!("cleis-cfg-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        assert_eq!(Config::load(&d).unwrap(), Config::default());
        Config::write_default(&d, &["10.8.0.1:8443".into()]).unwrap();
        let c = Config::load(&d).unwrap();
        assert_eq!(c.listen, vec!["10.8.0.1:8443".to_owned()]);
        assert_eq!(c.backups_dir(&d), Some(d.join("backups")));
        let t: Config = toml::from_str(
            "listen = [\"[::]:9000\"]\n[smtp]\nhost = \"smtp.example-studio.com\"\nfrom = \"Cleis <v@example-studio.com>\"\n[[oidc]]\nname = \"google\"\nissuer = \"https://accounts.google.com\"\nclient_id = \"x\"\n",
        )
        .unwrap();
        assert_eq!(t.smtp.unwrap().port, 587);
        assert_eq!(t.oidc[0].name, "google");
        assert!(toml::from_str::<Config>("listen = 3").is_err());
        let _ = std::fs::remove_dir_all(d);
    }
}
