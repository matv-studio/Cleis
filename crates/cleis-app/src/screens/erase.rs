//! Sync & backups › Delete (mockups 37l, 37m, 27/11/2026): every backup on this computer, or the
//! account and all its data. Deleting the account asks for the master password (or, without
//! one, the word "delete"), can remove the copy at the sync place too, then Cleis starts again
//! from the first launch. An organization keeps its own spaces: only this member's copy goes.

use crate::app::{CleisApp, ConfirmAction, Session};
use crate::i18n::tr;
use crate::theme::*;
use crate::ui::*;
use gpui_kit::TestSupportExt;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, Entity, SharedString, Window, div, px};
use std::path::{Path, PathBuf};

pub struct EraseAccount {
    pub password: Entity<InputState>,
    /// Also the copy at the sync place (GitHub, SFTP, a synced folder).
    pub remote: bool,
    pub error: Option<SharedString>,
    pub busy: bool,
}

/// The "Delete" section, last on the page.
pub fn section(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let n = crate::sync::backups(&s.vault.path().unwrap_or_default()).len();
    crate::ui::section(t)
        .child(section_title(t, tr("Delete")))
        .child(
            row(t, true)
                .child(
                    div()
                        .id("erase-backups-label")
                        .flex_1()
                        .child(tr("All backups"))
                        .tooltip(tip(crate::trf!(
                            "The {} dated copies on this computer; the vault stays",
                            n
                        ))),
                )
                .child(
                    button("erase-backups", t, BtnKind::DangerGhost, true)
                        .when(n == 0, |b| b.opacity(0.45))
                        .child(icon("trash", 14., t.danger_text))
                        .child(tr("Delete all backups"))
                        .when(n > 0, |b| {
                            b.on_click(cx.listener(|this, _, _, cx| {
                                this.ask_action(ConfirmAction::DeleteBackups, cx)
                            }))
                        })
                        .test_support(),
                ),
        )
        .child(
            row(t, false)
                .child(
                    div()
                        .id("erase-account-label")
                        .flex_1()
                        .child(tr("Account and its data"))
                        .tooltip(tip(tr(
                            "Vault, backups, settings and this computer's unlock; Cleis starts again from the first launch",
                        ))),
                )
                .child(
                    button("erase-account", t, BtnKind::Danger, true)
                        .child(icon("trash", 14., gpui_kit::white()))
                        .child(tr("Delete account"))
                        .on_click(cx.listener(|this, _, window, cx| this.open_erase(window, cx)))
                        .test_support(),
                ),
        )
}

/// Settings › Account, at the bottom (mockup 39b): the account and its data only.
pub fn account_section(t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    crate::ui::section(t)
        .child(section_title(t, tr("Delete")))
        .child(
            row(t, true)
                .child(
                    div()
                        .id("erase-account-label-2")
                        .flex_1()
                        .child(tr("Account and its data"))
                        .tooltip(tip(tr(
                            "Vault, backups, settings and this computer's unlock; Cleis starts again from the first launch",
                        ))),
                )
                .child(
                    button("erase-account-2", t, BtnKind::Danger, true)
                        .child(icon("trash", 14., gpui_kit::white()))
                        .child(tr("Delete account"))
                        .on_click(cx.listener(|this, _, window, cx| this.open_erase(window, cx)))
                        .test_support(),
                ),
        )
}

/// Where the copy at the sync place is, when there is one (named in the dialog).
fn remote_name(s: &Session) -> Option<&'static str> {
    s.sync.place.as_ref().map(|_| s.sync.provider.name())
}

/// The dialog of mockup 37m.
pub fn dialog(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Option<Div> {
    let e = s.erase.as_ref()?;
    let n = crate::sync::backups(&s.vault.path().unwrap_or_default()).len();
    let has_password = s.vault.has_password().unwrap_or(true);
    let card = dialog_box(t)
        .child(
            div()
                .text_size(TEXT_LG)
                .line_height(px(20.))
                .font_weight(W_MEDIUM)
                .child(tr("Delete your account?")),
        )
        .child(
            div()
                .id("erase-what")
                .text_color(t.text_muted)
                .child(crate::trf!(
                    "Vault, {} backups, settings. It cannot be undone.",
                    n
                ))
                .tooltip(tip(tr("Organizations keep their own spaces"))),
        )
        .when_some(remote_name(s), |d, name| {
            let on = e.remote;
            d.child(
                div()
                    .id("erase-remote")
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .cursor_pointer()
                    .child(super::org::check_box(t, on))
                    .child(crate::trf!("Also the copy on {}", name))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(e) = this.session().and_then(|s| s.erase.as_mut()) {
                            e.remote = !e.remote;
                        }
                        cx.notify();
                    }))
                    .test_support(),
            )
        })
        .child(field(
            t,
            if has_password {
                tr("Password")
            } else {
                tr("Type “delete”")
            },
            Input::new(&e.password).id("erase-password"),
        ))
        .when_some(e.error.clone(), |d, err| {
            d.child(
                div()
                    .text_size(TEXT_SM)
                    .text_color(t.danger_text)
                    .child(err),
            )
        })
        .child(
            div()
                .pt(px(8.))
                .flex()
                .justify_end()
                .gap(px(8.))
                .child(
                    button("erase-cancel", t, BtnKind::Default, false)
                        .child(tr("Cancel"))
                        .on_click(cx.listener(|this, _, window, cx| this.close_erase(window, cx)))
                        .test_support(),
                )
                .child(
                    button("erase-ok", t, BtnKind::Danger, false)
                        .when(e.busy, |b| b.opacity(0.6))
                        .child(tr("Delete account"))
                        .on_click(cx.listener(|this, _, window, cx| this.erase_account(window, cx)))
                        .test_support(),
                ),
        );
    Some(
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(t.overlay)
            .child(
                div()
                    .id("erase-dialog")
                    .occlude()
                    .child(card)
                    .test_support(),
            ),
    )
}

/// The files of this vault on this computer: the vault (and SQLite's journal), the files kept
/// beside it under its name (unlock, sync, organizations), and the backups folder.
pub fn local_files(vault: &Path) -> Vec<PathBuf> {
    let mut v = vec![];
    let name = vault
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if let Some(dir) = vault.parent()
        && let Ok(rd) = std::fs::read_dir(dir)
    {
        for f in rd.flatten() {
            let n = f.file_name().to_string_lossy().into_owned();
            if !name.is_empty() && n.starts_with(&name) && f.path().is_file() {
                v.push(f.path());
            }
        }
    }
    v.sort();
    v
}

impl CleisApp {
    pub fn open_erase(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let has_password = self
            .session()
            .is_some_and(|s| s.vault.has_password().unwrap_or(true));
        let password = cx.new(|cx| {
            let st = InputState::new(window, cx);
            if has_password { st.masked(true) } else { st }
        });
        password.update(cx, |i, cx| i.focus(window, cx));
        let sub = cx.subscribe_in(&password, window, |this, _, ev: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = ev {
                this.erase_account(window, cx);
            }
        });
        self._subs.push(sub);
        if let Some(s) = self.session() {
            // The copy elsewhere goes too unless unticked: what "delete my account" means.
            let remote = s.sync.place.is_some();
            s.erase = Some(EraseAccount {
                password,
                remote,
                error: None,
                busy: false,
            });
        }
        cx.notify();
    }

    pub fn close_erase(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(s) = self.session() {
            s.erase = None;
            let f = s.main_focus.clone();
            window.focus(&f, cx);
        }
        cx.notify();
    }

    /// Every dated copy on this computer (37l); the vault itself stays.
    pub fn delete_backups(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let mut failed = 0;
        for (p, _, _) in crate::sync::backups(&s.vault.path().unwrap_or_default()) {
            if std::fs::remove_file(&p).is_err() {
                failed += 1;
            }
        }
        s.toast = Some(if failed == 0 {
            tr("Backups deleted").into()
        } else {
            crate::trf!("{} backup(s) could not be deleted", failed).into()
        });
        cx.notify();
    }

    /// Mockup 37m: checked off the interface thread (the master password takes an unlock's
    /// time), then the copy elsewhere, then everything here; Cleis starts from the first launch.
    pub fn erase_account(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(e) = s.erase.as_mut() else { return };
        if e.busy {
            return;
        }
        let typed = e.password.read(cx).value().to_string();
        let has_password = s.vault.has_password().unwrap_or(true);
        if !has_password {
            if typed.trim().to_lowercase() != tr("delete").to_lowercase() {
                e.error = Some(tr("Type “delete” to confirm").into());
                cx.notify();
                return;
            }
        } else if typed.is_empty() {
            e.error = Some(tr("Wrong password").into());
            cx.notify();
            return;
        }
        e.busy = true;
        e.error = None;
        let remote = e.remote;
        // What to remove elsewhere, read now (the vault closes before the files go).
        let place = s.sync.place.clone();
        let remote_place = s.sync.remote.clone();
        let secret = s.sync.secret(&s.vault);
        let path = s.vault.path().unwrap_or_default();
        let check = if has_password {
            match s.vault.password_check() {
                Ok(c) => Some(c),
                Err(err) => {
                    if let Some(e) = s.erase.as_mut() {
                        e.busy = false;
                        e.error = Some(crate::trf!("Not done: {}", err).into());
                    }
                    cx.notify();
                    return;
                }
            }
        } else {
            None
        };
        cx.notify();
        let task = cx.background_spawn(async move {
            if let Some(check) = check
                && !check.check(&typed).unwrap_or(false)
            {
                return Err(tr("Wrong password").to_owned());
            }
            if remote {
                match (&remote_place, &place) {
                    (Some(r), _) => {
                        // D-101: an ended GitHub App token renewed for this last call.
                        let secret = match crate::remote::renewed(r, &secret) {
                            Some(next) => next?,
                            None => secret,
                        };
                        crate::remote::remove(r, &secret).map_err(|e| e.to_string())?
                    }
                    (None, Some(file)) if file != &path && file.exists() => {
                        std::fs::remove_file(file).map_err(|e| e.to_string())?;
                    }
                    _ => {}
                }
            }
            Ok(())
        });
        cx.spawn_in(window, async move |this, cx| {
            let r = task.await;
            this.update_in(cx, |this, window, cx| match r {
                Ok(()) => this.erase_here(window, cx),
                Err(err) => {
                    if let Some(e) = this.session().and_then(|s| s.erase.as_mut()) {
                        e.busy = false;
                        e.error = Some(crate::i18n::tr_dynamic(&err).into());
                    }
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    /// The vault closed, its files on this computer removed, Cleis back at the first launch.
    fn erase_here(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let path = self.path.clone();
        // The open vault (and its SQLite connection) closes first: the first launch replaces it.
        self.show_welcome(window, cx);
        crate::opening::forget(&path);
        for f in local_files(&path) {
            let _ = std::fs::remove_file(f);
        }
        let _ = std::fs::remove_dir_all(crate::sync::backups_dir(&path));
        crate::app::forget_appearance();
        cx.notify();
    }
}
