//! Sync & backups (screens 13, 22, 23; D-50, D-58): the place where a copy of the vault is kept
//! in step, dated backups, export, emergency kit.
//!
//! This device's settings (place, last sync) live beside the vault file in `<vault>.sync.json`:
//! the folder of a sync place differs from one computer to the next.

use crate::app::{CleisApp, Phase, Session};
use gpui_kit::{Context, PathPromptOptions};
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
}

impl Provider {
    pub fn name(self) -> &'static str {
        match self {
            Provider::Folder => "Folder",
            Provider::OneDrive => "OneDrive",
            Provider::GoogleDrive => "Google Drive",
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
    /// Last try, successful or not: paces automatic syncs (not saved).
    #[serde(skip)]
    pub last_attempt: Option<i64>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            place: None,
            provider: Provider::Folder,
            auto: true,
            keep: 30,
            last_sync: None,
            last_error: None,
            last_backup: None,
            last_attempt: None,
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
    std::fs::read(config_path(vault))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn save_config(vault: &Path, c: &SyncConfig) {
    if let Ok(b) = serde_json::to_vec_pretty(c) {
        let _ = std::fs::write(config_path(vault), b);
    }
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
        if let Some(s) = self.session() {
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
        s.sync.last_attempt = Some(cleis_core::now());
        match s.vault.sync_with(&place) {
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
        let place = s
            .sync
            .place
            .as_ref()
            .map(|p| format!("{} · {}", s.sync.provider.name(), p.display()));
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
