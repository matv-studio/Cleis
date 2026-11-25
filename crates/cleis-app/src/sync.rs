//! Sync & backups (screens 13, 22, 23; D-50, D-58): the place where a copy of the vault is kept
//! in step, dated backups, export, emergency kit.
//!
//! This device's settings (place, last sync) live beside the vault file in `<vault>.sync.json`:
//! the folder of a sync place differs from one computer to the next.

use crate::app::{CleisApp, Phase, Session};
use gpui_kit::{AppContext, Context, PathPromptOptions};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    /// Any folder (NAS, Dropbox…).
    #[default]
    Folder,
    /// Folder kept by the OneDrive app.
    OneDrive,
    /// Folder kept by Google Drive for desktop.
    GoogleDrive,
    /// File in a GitHub repository (D-65).
    #[serde(rename = "github")]
    GitHub,
    /// File on an SSH server (D-65).
    Sftp,
}

impl Provider {
    pub fn name(self) -> &'static str {
        match self {
            Provider::Folder => "Folder",
            Provider::OneDrive => "OneDrive",
            Provider::GoogleDrive => "Google Drive",
            Provider::GitHub => "GitHub",
            Provider::Sftp => "SFTP",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncConfig {
    /// The copy kept in step: `<folder>/<vault file name>`.
    pub place: Option<PathBuf>,
    pub provider: Provider,
    pub auto: bool,
    /// Dated backups kept on this computer.
    pub keep: u32,
    pub last_sync: Option<i64>,
    pub last_error: Option<String>,
    pub last_backup: Option<i64>,
    /// GitHub or SFTP: where the file is; `place` is then its local copy (D-65).
    pub remote: Option<crate::remote::RemotePlace>,
    /// This device: its place secret is its own (audit L-8).
    pub device: String,
    /// GitHub: when this device last pruned the branch's history (audit L-9).
    pub last_prune: Option<i64>,
    /// Last try, successful or not: paces automatic syncs (not saved).
    #[serde(skip)]
    pub last_attempt: Option<i64>,
    /// A network sync is under way (not saved).
    #[serde(skip)]
    pub busy: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            place: None,
            provider: Provider::Folder,
            auto: true,
            last_prune: None,
            keep: 30,
            last_sync: None,
            last_error: None,
            last_backup: None,
            remote: None,
            device: uuid::Uuid::new_v4().to_string(),
            last_attempt: None,
            busy: false,
        }
    }
}

/// Minutes between automatic syncs while Cleis is open.
pub const AUTO_EVERY_SECS: i64 = 5 * 60;

pub fn config_path(vault: &Path) -> PathBuf {
    let mut p = vault.as_os_str().to_owned();
    p.push(".sync.json");
    PathBuf::from(p)
}

pub fn load_config(vault: &Path) -> SyncConfig {
    let mut c: SyncConfig = std::fs::read(config_path(vault))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    if c.device.is_empty() {
        c.device = uuid::Uuid::new_v4().to_string();
        save_config(vault, &c);
    }
    c
}

impl SyncConfig {
    /// Name of this device's place secret in the vault (audit L-8).
    pub fn secret_key(&self) -> String {
        format!("{}:{}", crate::remote::SECRET_SETTING, self.device)
    }

    /// This device's place secret (or the one shared by an earlier version).
    pub fn secret(&self, vault: &cleis_store::Vault) -> crate::remote::Secret {
        vault
            .org_setting::<Option<crate::remote::Secret>>(&self.secret_key())
            .ok()
            .flatten()
            .flatten()
            .or_else(|| {
                vault
                    .org_setting::<crate::remote::Secret>(crate::remote::SECRET_SETTING)
                    .ok()
                    .flatten()
            })
            .unwrap_or_default()
    }
}

pub fn save_config(vault: &Path, c: &SyncConfig) {
    if let Ok(b) = serde_json::to_vec_pretty(c) {
        let _ = std::fs::write(config_path(vault), b);
    }
}

/// Local copy of a GitHub / SFTP place's file.
pub fn remote_cache(vault: &Path) -> PathBuf {
    let mut p = vault.as_os_str().to_owned();
    p.push(".remote");
    PathBuf::from(p)
}

pub fn backups_dir(vault: &Path) -> PathBuf {
    vault
        .parent()
        .map(|d| d.join("backups"))
        .unwrap_or_else(|| PathBuf::from("backups"))
}

/// Dated backups, newest first.
pub fn backups(vault: &Path) -> Vec<(PathBuf, u64, std::time::SystemTime)> {
    let mut v: Vec<_> = std::fs::read_dir(backups_dir(vault))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "cleis"))
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            Some((e.path(), m.len(), m.modified().ok()?))
        })
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0));
    v
}

fn stamp(ts: i64) -> String {
    let (y, m, d) = crate::format::civil(ts.div_euclid(86_400));
    let s = ts.rem_euclid(86_400);
    format!(
        "{y:04}-{m:02}-{d:02}-{:02}{:02}{:02}",
        s / 3600,
        s / 60 % 60,
        s % 60
    )
}

/// Vault files in a folder chosen on a new device (screen 22), most recent first.
pub fn find_vaults(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<(PathBuf, std::time::SystemTime)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "cleis"))
        .filter_map(|e| Some((e.path(), e.metadata().ok()?.modified().ok()?)))
        .collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.1));
    v.into_iter().map(|(p, _)| p).collect()
}

/// Reads everything again from the vault (after a sync brought changes).
pub fn reload(s: &mut Session) -> cleis_store::Result<()> {
    s.entries = s.vault.entries()?;
    s.trash = s.vault.trashed_entries()?;
    s.contacts = s.vault.contacts()?;
    s.spaces = s.vault.spaces()?;
    s.columns = s.vault.vault_columns()?;
    s.columns.extend(s.vault.contact_columns()?);
    s.prefs = s.vault.preferences()?;
    s.editor = None;
    s.contacts_ui.editor = None;
    s.refilter();
    Ok(())
}

impl CleisApp {
    fn save_sync(&mut self) {
        let path = self.path.clone();
        if let Some(s) = self.session() {
            save_config(&path, &s.sync);
        }
    }

    /// Picks the folder of a sync place; the copy is `<folder>/<vault file name>`.
    pub fn choose_sync_folder(&mut self, provider: Provider, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(format!("Sync with {}", provider.name()).into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = rx.await
                && let Some(dir) = paths.into_iter().next()
            {
                this.update(cx, |this, cx| this.set_sync_folder(&dir, provider, cx))
                    .ok();
            }
        })
        .detach();
    }

    pub fn set_sync_folder(&mut self, dir: &Path, provider: Provider, cx: &mut Context<Self>) {
        let name = self
            .path
            .file_name()
            .map(|n| n.to_owned())
            .unwrap_or_else(|| "vault.cleis".into());
        if let Some(s) = self.session() {
            s.sync.place = Some(dir.join(name));
            s.sync.provider = provider;
            s.sync.last_error = None;
        }
        self.save_sync();
        self.sync_now(cx);
    }

    pub fn disconnect_sync(&mut self, cx: &mut Context<Self>) {
        let cache = remote_cache(&self.path);
        if let Some(s) = self.session() {
            if s.sync.remote.take().is_some() {
                let _ = std::fs::remove_file(&cache);
                let key = s.sync.secret_key();
                let _ = s
                    .vault
                    .set_org_setting(&key, &None::<crate::remote::Secret>);
            }
            s.sync.place = None;
            s.sync.last_sync = None;
            s.sync.last_error = None;
        }
        self.save_sync();
        cx.notify();
    }

    /// Merges with the place now. Takes milliseconds: the rows stay sealed.
    pub fn sync_now(&mut self, cx: &mut Context<Self>) {
        self.sync(false, cx);
    }

    /// `quiet`: automatic sync, only errors and received changes are shown.
    pub fn sync(&mut self, quiet: bool, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(place) = s.sync.place.clone() else {
            return;
        };
        if s.sync.remote.is_some() {
            self.sync_remote(quiet, cx);
            return;
        }
        s.sync.last_attempt = Some(cleis_core::now());
        let r = s.vault.sync_with(&place).map_err(|e| e.to_string());
        self.sync_done(quiet, r, cx);
    }

    /// GitHub / SFTP (D-65): the file is read into a local copy, merged there, and sent back when
    /// this vault had something new. Another device's copy sent meanwhile: read and merge again.
    fn sync_remote(&mut self, quiet: bool, cx: &mut Context<Self>) {
        let cache = remote_cache(&self.path);
        let Some(s) = self.session() else { return };
        if s.sync.busy {
            return;
        }
        let Some(remote) = s.sync.remote.clone() else {
            return;
        };
        let secret = s.sync.secret(&s.vault);
        s.sync.busy = true;
        s.sync.last_attempt = Some(cleis_core::now());
        cx.notify();
        cx.spawn(async move |this, cx| {
            let mut outcome: Result<cleis_store::SyncReport, String> = Err(String::new());
            for _ in 0..3 {
                let (r, c, sec) = (remote.clone(), cache.clone(), secret.clone());
                let version = match cx
                    .background_spawn(async move { crate::remote::download(&r, &sec, &c) })
                    .await
                {
                    Ok(v) => v,
                    Err(e) => {
                        outcome = Err(e.to_string());
                        break;
                    }
                };
                let merged = this
                    .update(cx, |app, _| {
                        app.session()
                            .map(|s| s.vault.sync_with(&cache).map_err(|e| e.to_string()))
                    })
                    .ok()
                    .flatten();
                let Some(merged) = merged else { return };
                let rep = match merged {
                    Ok(r) => r,
                    Err(e) => {
                        outcome = Err(e);
                        break;
                    }
                };
                if rep.sent == 0 && !rep.created {
                    outcome = Ok(rep);
                    break;
                }
                let (r, c, sec) = (remote.clone(), cache.clone(), secret.clone());
                match cx
                    .background_spawn(async move { crate::remote::upload(&r, &sec, &c, version) })
                    .await
                {
                    Ok(()) => {
                        outcome = Ok(rep);
                        break;
                    }
                    Err(crate::remote::RemoteError::Conflict) => {
                        outcome = Err("the file kept changing; try again".into());
                        continue;
                    }
                    Err(e) => {
                        outcome = Err(e.to_string());
                        break;
                    }
                }
            }
            let ok = outcome.is_ok();
            this.update(cx, |app, cx| {
                if let Some(s) = app.session() {
                    s.sync.busy = false;
                }
                app.sync_done(quiet, outcome, cx);
                if ok {
                    app.prune_place(cx);
                }
            })
            .ok();
        })
        .detach();
    }

    /// Audit L-9: once a week, the GitHub branch keeps only its last versions.
    fn prune_place(&mut self, cx: &mut Context<Self>) {
        let path = self.path.clone();
        let Some(s) = self.session() else { return };
        let now = cleis_core::now();
        let due = s
            .sync
            .last_prune
            .is_none_or(|t| now - t >= crate::remote::PRUNE_EVERY_DAYS * 86_400);
        let Some(remote) = s.sync.remote.clone().filter(|_| due) else {
            return;
        };
        if !matches!(remote, crate::remote::RemotePlace::GitHub { .. }) {
            return;
        }
        let secret = s.sync.secret(&s.vault);
        s.sync.last_prune = Some(now);
        save_config(&path, &s.sync);
        cx.background_spawn(async move {
            if let Err(e) = crate::remote::prune(&remote, &secret, crate::remote::KEEP_VERSIONS) {
                eprintln!("GitHub history not pruned: {e}");
            }
        })
        .detach();
    }

    fn sync_done(
        &mut self,
        quiet: bool,
        r: Result<cleis_store::SyncReport, String>,
        cx: &mut Context<Self>,
    ) {
        let Some(s) = self.session() else { return };
        match r {
            Ok(r) => {
                s.sync.last_sync = Some(cleis_core::now());
                s.sync.last_error = None;
                if r.received > 0
                    && let Err(e) = reload(s)
                {
                    s.sync.last_error = Some(e.to_string());
                }
                let msg: Option<String> = if r.created {
                    Some("Copy created in the sync place".into())
                } else if r.received > 0 {
                    Some(format!("Synced · {} received", r.received))
                } else if quiet {
                    None
                } else if r.sent > 0 {
                    Some(format!("Synced · {} sent", r.sent))
                } else {
                    Some("Up to date".into())
                };
                if let Some(m) = msg {
                    s.toast = Some(m.into());
                }
            }
            Err(e) => {
                s.sync.last_error = Some(e.to_string());
                s.toast = Some(format!("Not synced: {e}").into());
            }
        }
        self.save_sync();
        cx.notify();
    }

    /// Called every second by the window ticker.
    pub fn auto_sync_due(&self) -> bool {
        let Phase::Main(s) = &self.phase else {
            return false;
        };
        s.sync.auto
            && s.sync.place.is_some()
            && s.gate.is_none()
            // A failing place is retried at the same pace, not every second.
            && s
                .sync
                .last_attempt
                .is_none_or(|t| cleis_core::now() - t >= AUTO_EVERY_SECS)
    }

    pub fn backup_now(&mut self, cx: &mut Context<Self>) {
        self.backup(false, cx);
    }

    pub fn backup(&mut self, quiet: bool, cx: &mut Context<Self>) {
        let vault_path = self.path.clone();
        let Some(s) = self.session() else { return };
        let dir = backups_dir(&vault_path);
        let now = cleis_core::now();
        let file = dir.join(format!("vault-{}.cleis", stamp(now)));
        let r = std::fs::create_dir_all(&dir)
            .map_err(|e| e.to_string())
            .and_then(|()| s.vault.copy_to(&file).map_err(|e| e.to_string()));
        match r {
            Ok(()) => {
                s.sync.last_backup = Some(now);
                let keep = s.sync.keep.max(1) as usize;
                for (old, _, _) in backups(&vault_path).into_iter().skip(keep) {
                    let _ = std::fs::remove_file(old);
                }
                if !quiet {
                    s.toast = Some("Backed up".into());
                }
            }
            Err(e) => s.toast = Some(format!("Not backed up: {e}").into()),
        }
        self.save_sync();
        cx.notify();
    }

    /// Daily backup while Cleis is open.
    pub fn auto_backup_due(&self) -> bool {
        let Phase::Main(s) = &self.phase else {
            return false;
        };
        !self.demo
            && !self.no_shortcuts
            && s.gate.is_none()
            && s.sync
                .last_backup
                .is_none_or(|t| cleis_core::now() - t >= 86_400)
    }

    /// Restores a backup: it is merged into the vault (nothing newer is lost).
    pub fn restore_backup(&mut self, file: PathBuf, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        // Merge from a copy so the backup itself stays as it was.
        let tmp = std::env::temp_dir().join(format!(
            "cleis-restore-{}-{}.cleis",
            std::process::id(),
            cleis_core::now()
        ));
        let r = std::fs::copy(&file, &tmp)
            .map_err(|e| e.to_string())
            .and_then(|_| s.vault.sync_with(&tmp).map_err(|e| e.to_string()));
        let _ = std::fs::remove_file(&tmp);
        match r {
            Ok(rep) => {
                let _ = reload(s);
                s.toast = Some(format!("{} restored from the backup", rep.received).into());
            }
            Err(e) => s.toast = Some(format!("Not restored: {e}").into()),
        }
        cx.notify();
    }

    /// Saves a copy of the vault (opened with the master password) where the user chooses.
    pub fn export_vault(&mut self, cx: &mut Context<Self>) {
        let dir = directories::UserDirs::new()
            .and_then(|d| d.download_dir().map(|p| p.to_owned()))
            .unwrap_or_else(std::env::temp_dir);
        let name = format!("Cleis export {}.cleis", stamp(cleis_core::now()));
        let rx = cx.prompt_for_new_path(&dir, Some(&name));
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(path))) = rx.await {
                this.update(cx, |this, cx| {
                    let msg = match this.session().map(|s| s.vault.copy_to(&path)) {
                        Some(Ok(())) => "Export saved".to_owned(),
                        Some(Err(e)) => format!("Not exported: {e}"),
                        None => return,
                    };
                    this.toast(msg, cx);
                })
                .ok();
            }
        })
        .detach();
    }

    /// Emergency kit (PDF to print): where the vault is and room for the master password.
    pub fn save_emergency_kit(&mut self, cx: &mut Context<Self>) {
        let vault = self.path.clone();
        let Some(s) = self.session() else { return };
        let place = s.sync.place.as_ref().map(|p| {
            // Audit L-7: GitHub / SFTP: the repository or server, not the local copy.
            let at = s
                .sync
                .remote
                .as_ref()
                .map_or_else(|| p.display().to_string(), |r| r.label());
            format!("{} · {at}", s.sync.provider.name())
        });
        let pdf = crate::kit::emergency_kit(&vault, place.as_deref(), cleis_core::now());
        let dir = directories::UserDirs::new()
            .and_then(|d| d.download_dir().map(|p| p.to_owned()))
            .unwrap_or_else(std::env::temp_dir);
        let rx = cx.prompt_for_new_path(&dir, Some("Cleis emergency kit.pdf"));
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(path))) = rx.await {
                let r = std::fs::write(&path, pdf);
                this.update(cx, |this, cx| {
                    let msg = match r {
                        Ok(()) => "Emergency kit saved".to_owned(),
                        Err(e) => format!("Not saved: {e}"),
                    };
                    this.toast(msg, cx);
                })
                .ok();
            }
        })
        .detach();
    }
}

/// GitHub / SFTP sync run off the interface with the vault moved in (on lock, audit L-5): read,
/// merge, send back when this vault had something new; the vault is dropped at the end.
pub fn remote_sync_detached(
    mut vault: cleis_store::Vault,
    mut cfg: SyncConfig,
    vault_path: &Path,
) -> Result<(), String> {
    let Some(remote) = cfg.remote.clone() else {
        return Ok(());
    };
    let secret = cfg.secret(&vault);
    let cache = remote_cache(vault_path);
    let mut result = Err("the file kept changing".to_owned());
    for _ in 0..3 {
        let version =
            crate::remote::download(&remote, &secret, &cache).map_err(|e| e.to_string())?;
        let rep = vault.sync_with(&cache).map_err(|e| e.to_string())?;
        if rep.sent == 0 && !rep.created {
            result = Ok(());
            break;
        }
        match crate::remote::upload(&remote, &secret, &cache, version) {
            Ok(()) => {
                result = Ok(());
                break;
            }
            Err(crate::remote::RemoteError::Conflict) => continue,
            Err(e) => return Err(e.to_string()),
        }
    }
    if result.is_ok() {
        cfg.last_sync = Some(cleis_core::now());
        cfg.last_error = None;
        save_config(vault_path, &cfg);
    }
    result
}
