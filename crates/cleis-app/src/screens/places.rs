//! Sync & backups › GitHub / SFTP (mockups 23b, 23c; D-65): the place's details and its
//! sign-in secret, checked by a first read before they are kept.

use crate::app::{CleisApp, Session};
use crate::i18n::tr;
use crate::remote::{RemotePlace, Secret};
use crate::sync::Provider;
use crate::theme::*;
use crate::ui::*;
use gpui_kit::TestSupportExt;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, Entity, SharedString, Window, div, px};

pub struct PlaceDialog {
    pub kind: Provider,
    /// Copies of the organization's backups rather than the vault's sync (path: a folder).
    pub org_backups: bool,
    /// GitHub: repository, file, branch, token. SFTP: server, port, user, file, password, key.
    pub inputs: Vec<Entity<InputState>>,
    /// SFTP: sign in with a private key rather than a password.
    pub with_key: bool,
    /// SFTP: the server's key seen at the first connection, to confirm.
    pub host_key: Option<String>,
    pub error: Option<SharedString>,
    pub busy: bool,
    /// GitHub's API, when the sign-in (37n) gave it rather than the repository box.
    pub api: Option<String>,
    /// GitHub App sign-in (D-101): the refresh token and when the token ends.
    pub renew: Option<(String, i64)>,
}

fn tip(
    text: &'static str,
) -> impl Fn(&mut Window, &mut gpui_kit::App) -> gpui_kit::AnyView + 'static {
    move |window, cx| {
        gpui_kit::component::tooltip::Tooltip::new(crate::i18n::tr_dynamic(text)).build(window, cx)
    }
}

impl CleisApp {
    pub fn open_place(&mut self, kind: Provider, window: &mut Window, cx: &mut Context<Self>) {
        let vault_name = self
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "vault.cleis".into());
        let make =
            |ph: &str, value: &str, mask: bool, window: &mut Window, cx: &mut Context<Self>| {
                let (ph, value) = (ph.to_owned(), value.to_owned());
                cx.new(|cx| {
                    let mut i = InputState::new(window, cx).placeholder(ph).masked(mask);
                    i.set_value(value, window, cx);
                    i
                })
            };
        let inputs = match kind {
            Provider::GitHub => vec![
                make("owner/name", "", false, window, cx),
                make("", &vault_name, false, window, cx),
                make("", "main", false, window, cx),
                make("github_pat_…", "", true, window, cx),
            ],
            _ => vec![
                make("files.example-studio.com", "", false, window, cx),
                make("", "22", false, window, cx),
                make("", "", false, window, cx),
                make("", &format!("cleis/{vault_name}"), false, window, cx),
                make("", "", true, window, cx),
                make("-----BEGIN OPENSSH PRIVATE KEY-----", "", false, window, cx),
            ],
        };
        if let Some(first) = inputs.first() {
            first.update(cx, |i, cx| i.focus(window, cx));
        }
        if let Some(slot) = self.place_slot() {
            *slot = Some(PlaceDialog {
                kind,
                org_backups: false,
                inputs,
                with_key: false,
                host_key: None,
                error: None,
                busy: false,
                api: None,
                renew: None,
            });
        }
        cx.notify();
    }

    /// Where the dialog lives: the open vault, or the restore of the first launch.
    pub fn place_slot(&mut self) -> Option<&mut Option<PlaceDialog>> {
        match &mut self.phase {
            Phase::Main(s) => Some(&mut s.place),
            Phase::Welcome(w) => w.restore.as_mut().map(|r| &mut r.place),
            _ => None,
        }
    }

    fn place_mut(&mut self) -> Option<&mut PlaceDialog> {
        self.place_slot().and_then(|p| p.as_mut())
    }

    /// Organization backups: a folder of the place for the sealed copies.
    pub fn open_org_backup_place(
        &mut self,
        kind: Provider,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_place(kind, window, cx);
        let folder = match kind {
            Provider::GitHub => "cleis-backups",
            _ => "cleis-backups",
        };
        if let Some(d) = self.place_mut() {
            d.org_backups = true;
            let n = if kind == Provider::GitHub { 1 } else { 3 };
            let input = d.inputs[n].clone();
            input.update(cx, |i, cx| i.set_value(folder, window, cx));
        }
    }

    fn place_org_backups(&mut self, place: RemotePlace, secret: Secret, cx: &mut Context<Self>) {
        let (p, sec) = (place.clone(), secret.clone());
        let task = cx.background_spawn(async move { crate::remote::check(&p, &sec) });
        cx.spawn(async move |this, cx| {
            let r = task.await;
            this.update(cx, |app, cx| {
                let Some(s) = app.session() else { return };
                match r {
                    Ok(()) => {
                        let _ = s.vault.set_org_setting(
                            crate::remote::ORG_BACKUP_SETTING,
                            &Some((place, secret)),
                        );
                        s.place = None;
                        s.toast = Some(tr("Backups will be copied there too").into());
                    }
                    Err(e) => {
                        if let Some(d) = s.place.as_mut() {
                            d.busy = false;
                            d.error = Some(e.to_string().into());
                        }
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn place_ref(&self) -> Option<&PlaceDialog> {
        match &self.phase {
            Phase::Main(s) => s.place.as_ref(),
            Phase::Welcome(w) => w.restore.as_ref().and_then(|r| r.place.as_ref()),
            _ => None,
        }
    }

    fn place_values(&self, cx: &Context<Self>) -> Option<(RemotePlace, Secret)> {
        let d = self.place_ref()?;
        let v: Vec<String> = d
            .inputs
            .iter()
            .map(|i| i.read(cx).value().trim().to_owned())
            .collect();
        Some(match d.kind {
            Provider::GitHub => {
                let (api, repo) = crate::remote::github_repo(&v[0]);
                let api = d.api.clone().unwrap_or(api);
                (
                    RemotePlace::GitHub {
                        repo,
                        path: v[1].clone(),
                        branch: if v[2].is_empty() {
                            "main".into()
                        } else {
                            v[2].clone()
                        },
                        api,
                    },
                    Secret {
                        token: v[3].clone(),
                        refresh: d.renew.as_ref().map(|r| r.0.clone()).unwrap_or_default(),
                        expires: d.renew.as_ref().map_or(0, |r| r.1),
                        ..Default::default()
                    },
                )
            }
            _ => (
                RemotePlace::Sftp {
                    host: v[0].clone(),
                    port: v[1].parse().unwrap_or(22),
                    user: v[2].clone(),
                    path: v[3].clone(),
                    host_key: d.host_key.clone().unwrap_or_default(),
                },
                Secret {
                    token: v[4].clone(),
                    key: if d.with_key {
                        v[5].clone()
                    } else {
                        String::new()
                    },
                    ..Default::default()
                },
            ),
        })
    }

    /// Connect: SFTP shows the server's key first (connect again to trust it); then a first
    /// read checks the place before it is kept.
    pub fn place_connect(&mut self, cx: &mut Context<Self>) {
        let Some((place, secret)) = self.place_values(cx) else {
            return;
        };
        self.connect_remote(place, secret, cx);
    }

    /// A first read checks the place before it is kept (sign-ins to Microsoft and Google give
    /// the place and its secret directly, D-104).
    pub fn connect_remote(&mut self, place: RemotePlace, secret: Secret, cx: &mut Context<Self>) {
        let kind = match &place {
            RemotePlace::GitHub { .. } => Provider::GitHub,
            RemotePlace::Sftp { .. } => Provider::Sftp,
            RemotePlace::OneDrive { .. } => Provider::OneDrive,
            RemotePlace::GoogleDrive { .. } => Provider::GoogleDrive,
        };
        let first_sftp =
            matches!(&place, RemotePlace::Sftp { host_key, .. } if host_key.is_empty());
        let cache = crate::sync::remote_cache(&self.path);
        let org_backups = self.place_ref().is_some_and(|d| d.org_backups);
        if org_backups && !first_sftp {
            self.place_org_backups(place, secret, cx);
            return;
        }
        let restoring = matches!(self.phase, Phase::Welcome(_));
        let cache = if restoring {
            std::env::temp_dir().join(format!("cleis-restore-{}.cleis", std::process::id()))
        } else {
            cache
        };
        if let Some(d) = self.place_mut() {
            d.busy = true;
            d.error = None;
        }
        cx.notify();
        let (p, sec) = (place.clone(), secret.clone());
        let c2 = cache.clone();
        let task = cx.background_spawn(async move {
            if first_sftp {
                return crate::sftp::server_key(&p, &sec).map(|k| (Some(k), None));
            }
            crate::remote::download(&p, &sec, &c2)?;
            // D-96: restoring, the key kept beside a vault without a password, if any.
            let key = if restoring && c2.exists() {
                crate::remote::read_key(&p, &sec).ok().flatten()
            } else {
                None
            };
            Ok((None, key))
        });
        cx.spawn(async move |this, cx| {
            let r = task.await;
            this.update(cx, |app, cx| {
                let path = app.path.clone();
                let web = app.github_hosts.web.clone();
                if let Phase::Welcome(w) = &mut app.phase {
                    let Some(rs) = w.restore.as_mut() else { return };
                    // Signed in from the first screen, the repository there but empty: a vault
                    // is made here and sent to it (as with no repository, mockup 40b).
                    if let (
                        Ok((None, _)),
                        Some(login),
                        crate::remote::RemotePlace::GitHub { api, .. },
                    ) = (&r, rs.signed.clone(), &place)
                        && !cache.exists()
                    {
                        app.github_after = Some(crate::screens::github::Signed {
                            login,
                            token: zeroize::Zeroizing::new(secret.token.clone()),
                            refresh: zeroize::Zeroizing::new(secret.refresh.clone()),
                            expires: secret.expires,
                            hosts: crate::github_login::Hosts {
                                web,
                                api: api.clone(),
                            },
                        });
                        if let Phase::Welcome(w) = &mut app.phase {
                            w.restore = None;
                            w.choice = Some(crate::screens::welcome::Start::Account);
                        }
                        cx.notify();
                        return;
                    }
                    // D-104: signed in to Microsoft or Google, no vault there yet: a vault is
                    // made here and sent to the account once its recovery key is put away.
                    if matches!(&r, Ok((None, _))) && place.cloud().is_some() && !cache.exists() {
                        app.cloud = None;
                        app.cloud_after = Some((place, secret));
                        if let Phase::Welcome(w) = &mut app.phase {
                            w.restore = None;
                            w.choice = Some(crate::screens::welcome::Start::Account);
                        }
                        cx.notify();
                        return;
                    }
                    if place.cloud().is_some() {
                        // The sign-in's dialog says what went wrong; found, the vault's card shows.
                        match &r {
                            Ok(_) => app.cloud = None,
                            Err(e) => {
                                if let Some(c) = app.cloud.as_mut() {
                                    c.finishing = false;
                                    c.error = Some(crate::i18n::tr_dynamic(&e.to_string()).into());
                                }
                            }
                        }
                    }
                    let Phase::Welcome(w) = &mut app.phase else {
                        return;
                    };
                    let Some(rs) = w.restore.as_mut() else { return };
                    match r {
                        Ok((Some(key), _)) => {
                            if let Some(d) = rs.place.as_mut() {
                                d.busy = false;
                                d.host_key = Some(key);
                            }
                        }
                        Ok((None, account_key)) if cache.exists() => {
                            rs.place = None;
                            rs.provider = Some(kind);
                            rs.found = vec![cache];
                            rs.chosen = 0;
                            rs.dir = None;
                            rs.remote = Some((place, secret));
                            rs.account_key = account_key;
                        }
                        Ok((None, _)) => {
                            if let Some(d) = rs.place.as_mut() {
                                d.busy = false;
                                d.error = Some(tr("No vault at this place yet").into());
                            }
                        }
                        Err(e) => {
                            if let Some(d) = rs.place.as_mut() {
                                d.busy = false;
                                d.error = Some(e.to_string().into());
                            }
                        }
                    }
                    cx.notify();
                    return;
                }
                if place.cloud().is_some() {
                    match &r {
                        Ok(_) => app.cloud = None,
                        Err(e) => {
                            if let Some(c) = app.cloud.as_mut() {
                                c.finishing = false;
                                c.error = Some(crate::i18n::tr_dynamic(&e.to_string()).into());
                            }
                        }
                    }
                }
                let Some(s) = app.session() else { return };
                match r {
                    Ok((Some(key), _)) => {
                        if let Some(d) = s.place.as_mut() {
                            d.busy = false;
                            d.host_key = Some(key);
                        }
                    }
                    Ok((None, _)) => {
                        let key = s.sync.secret_key();
                        let _ = s.vault.set_org_setting(&key, &Some(secret));
                        s.sync.remote = Some(place);
                        s.sync.provider = kind;
                        s.sync.place = Some(crate::sync::remote_cache(&path));
                        s.sync.last_error = None;
                        s.sync.last_sync = None;
                        s.place = None;
                        crate::sync::save_config(&path, &s.sync);
                        app.sync_now(cx);
                    }
                    Err(e) => {
                        if let Some(d) = s.place.as_mut() {
                            d.busy = false;
                            d.error = Some(e.to_string().into());
                        }
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

use crate::app::Phase;

/// `SHA256:pT4bW1x0…9QcE`: enough to compare by eye.
fn short_key(k: &str) -> String {
    let c: Vec<char> = k.chars().collect();
    if c.len() <= 22 {
        return k.to_owned();
    }
    format!(
        "{}…{}",
        c[..15].iter().collect::<String>(),
        c[c.len() - 4..].iter().collect::<String>()
    )
}

pub fn dialog(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Option<Div> {
    s.place.as_ref().map(|d| place_dialog(d, t, cx))
}

pub fn place_dialog(d: &PlaceDialog, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let inp = |n: usize, id: &'static str| Input::new(&d.inputs[n]).id(id);
    let mut body = div().flex().flex_col().gap(px(12.));
    let title = match d.kind {
        Provider::GitHub => {
            body = body
                .child(field(t, tr("Repository"), inp(0, "gh-repo")))
                .child(
                    div()
                        .flex()
                        .gap(px(12.))
                        .child(div().flex_1().child(field(
                            t,
                            if d.org_backups {
                                tr("Folder")
                            } else {
                                tr("File")
                            },
                            inp(1, "gh-path"),
                        )))
                        .child(div().w(px(140.)).child(field(
                            t,
                            tr("Branch"),
                            inp(2, "gh-branch"),
                        ))),
                )
                .child(
                    div()
                        .id("gh-token-field")
                        .tooltip(tip(
                            "Fine-grained token: this repository only, Contents read and write",
                        ))
                        .child(field(
                            t,
                            "Access token",
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(div().flex_1().child(inp(3, "gh-token")))
                                .child(
                                    button("gh-token-new", t, BtnKind::Ghost, true)
                                        .child(tr("Create one"))
                                        .on_click(cx.listener(|_, _, _, cx| {
                                            cx.open_url(crate::remote::github_token_page())
                                        })),
                                ),
                        )),
                );
            "GitHub"
        }
        _ => {
            let pick: super::org::Pick = std::rc::Rc::new(|this, i, cx| {
                if let Some(d) = this.place_mut() {
                    d.with_key = i == 1;
                }
                cx.notify();
            });
            body = body
                .child(
                    div()
                        .flex()
                        .gap(px(12.))
                        .child(div().flex_1().child(field(t, tr("Server"), inp(0, "sftp-host"))))
                        .child(div().w(px(90.)).child(field(t, tr("Port"), inp(1, "sftp-port")))),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(12.))
                        .child(div().flex_1().child(field(t, tr("User"), inp(2, "sftp-user"))))
                        .child(div().flex_1().child(field(
                            t,
                            if d.org_backups { tr("Folder") } else { tr("File") },
                            inp(3, "sftp-path"),
                        ))),
                )
                .child(field(
                    t,
                    "Sign-in",
                    super::org::segment(
                        "sftp-auth",
                        t,
                        &[tr("Password"), tr("Private key")],
                        usize::from(d.with_key),
                        pick,
                        cx,
                    ),
                ))
                .when(d.with_key, |b| {
                    b.child(field(t, tr("OpenSSH private key"), inp(5, "sftp-key")))
                        .child(field(t, tr("Passphrase"), inp(4, "sftp-pass")))
                })
                .when(!d.with_key, |b| b.child(inp(4, "sftp-pass")))
                .when_some(d.host_key.clone(), |b, k| {
                    b.child(
                        row(t, false)
                            .child(
                                div()
                                    .id("sftp-key-label")
                                    .flex_1()
                                    .tooltip(tip(
                                        "Checked at each connection; a change stops the sync. Compare it with the server's.",
                                    ))
                                    .child(tr("Server key")),
                            )
                            .child(
                                div()
                                    .font_family(FONT_MONO)
                                    .text_size(TEXT_SM)
                                    .text_color(t.text_muted)
                                    .child(short_key(&k)),
                            )
                            .child(chip(t, Tone::Neutral, tr("Connect to trust"), false)),
                    )
                });
            "SFTP"
        }
    };
    let card = dialog_box(t)
        .child(
            div()
                .text_size(TEXT_LG)
                .line_height(px(20.))
                .font_weight(W_MEDIUM)
                .child(crate::i18n::tr_owned(title)),
        )
        .child(body)
        .when_some(
            d.error.clone().map(|e| crate::i18n::tr_dynamic(&e)),
            |c, e| c.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e)),
        )
        .child(
            div()
                .pt(px(8.))
                .flex()
                .justify_end()
                .gap(px(8.))
                .child(
                    button("place-cancel", t, BtnKind::Default, false)
                        .child(tr("Cancel"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(slot) = this.place_slot() {
                                *slot = None;
                            }
                            cx.notify();
                        }))
                        .test_support(),
                )
                .child(
                    button("place-connect", t, BtnKind::Primary, false)
                        .when(d.busy, |b| b.opacity(0.6))
                        .child(if d.busy {
                            tr("Connecting…")
                        } else {
                            tr("Connect")
                        })
                        .on_click(cx.listener(|this, _, _, cx| {
                            if this.demo {
                                this.toast(tr("Not in the demo"), cx);
                            } else {
                                this.place_connect(cx);
                            }
                        }))
                        .test_support(),
                ),
        );
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(t.overlay)
        .child(div().id("place-dialog").occlude().child(card))
}
