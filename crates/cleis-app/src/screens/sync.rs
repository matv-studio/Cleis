//! Settings › Sync & backups (screen 23, with the backups list of screen 13).

use crate::app::{CleisApp, Session};
use crate::i18n::tr;
use crate::sync::{Provider, backups};
use crate::theme::*;
use crate::ui::*;
use gpui_kit::TestSupportExt;
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, SharedString, Window, deferred, div, px};

const KEEP: [u32; 3] = [10, 30, 100];

fn tip(
    text: &'static str,
) -> impl Fn(&mut Window, &mut gpui_kit::App) -> gpui_kit::AnyView + 'static {
    move |window, cx| {
        gpui_kit::component::tooltip::Tooltip::new(crate::i18n::tr_dynamic(text)).build(window, cx)
    }
}

fn ago(ts: i64) -> String {
    let d = cleis_core::now() - ts;
    match d {
        ..=59 => tr("just now").into(),
        60..=3599 => crate::trf!("{} min ago", d / 60),
        3600..=86_399 => crate::trf!("{} h ago", d / 3600),
        _ => crate::format::date(ts),
    }
}

fn place_row(
    id: &'static str,
    t: &Tokens,
    ico: &str,
    name: &str,
    sub: &str,
    action: impl IntoElement,
    first: bool,
) -> Div {
    row(t, first)
        .child(
            div()
                .size(px(28.))
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .rounded(RADIUS_MD)
                .bg(t.surface_active)
                .child(icon(ico, 16., t.text)),
        )
        .child(
            div()
                .id(id)
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(crate::i18n::tr_owned(name))
                .child(
                    div()
                        .text_size(TEXT_SM)
                        .text_color(t.text_muted)
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(crate::i18n::tr_owned(sub)),
                ),
        )
        .child(action)
}

pub fn page(s: &Session, t: &Tokens, keep_open: bool, cx: &mut Context<CleisApp>) -> Div {
    page_with(s, t, keep_open, true, cx)
}

/// `with_head: false` when Settings puts the title and the Mine / organization choice above.
pub fn page_with(
    s: &Session,
    t: &Tokens,
    keep_open: bool,
    with_head: bool,
    cx: &mut Context<CleisApp>,
) -> Div {
    let c = &s.sync;
    let head = div()
        .flex()
        .items_center()
        .gap(px(12.))
        .min_h(px(32.))
        .child(
            div()
                .flex_1()
                .text_size(TEXT_XL)
                .font_weight(W_STRONG)
                .child(tr("Sync & backups")),
        )
        .when(c.place.is_some(), |d| {
            d.child(
                button("sync-now", t, BtnKind::Primary, false)
                    .child(icon("sync", 16., t.primary_ink))
                    .child(tr("Sync now"))
                    .on_click(cx.listener(|this, _, _, cx| this.sync_now(cx)))
                    .test_support(),
            )
        });
    let mut sync = section(t).child(
        div()
            .id("sync-title")
            .child(section_title(t, tr("Sync")))
            .tooltip(tip(
                "Your encrypted vault, kept the same on every device using this place. Your master password stays on your devices.",
            )),
    );
    match &c.place {
        Some(p) => {
            let status = match (&c.last_error, c.last_sync) {
                _ if c.busy => chip(t, Tone::Neutral, tr("Syncing…"), false),
                (Some(_), _) => chip(t, Tone::Danger, tr("Not synced"), true),
                (None, Some(ts)) => chip(t, Tone::Ok, crate::trf!("Synced {}", ago(ts)), true),
                (None, None) => chip(t, Tone::Neutral, tr("Never synced"), false),
            };
            let err = c.last_error.clone().map(|e| crate::i18n::tr_dynamic(&e));
            sync = sync
                .child(place_row(
                    "sync-place",
                    t,
                    match c.provider {
                        Provider::GitHub => "repo",
                        Provider::Sftp => "server",
                        Provider::Folder => "folder",
                        _ => "cloud",
                    },
                    c.provider.name(),
                    &c.remote
                        .as_ref()
                        .map_or_else(|| p.display().to_string(), |r| r.label()),
                    div().flex().items_center().gap(px(8.)).child(status).child(
                        button("sync-disconnect", t, BtnKind::Ghost, true)
                            .child(tr("Disconnect"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.ask_action(crate::app::ConfirmAction::DisconnectPlace, cx)
                            }))
                            .test_support(),
                    ),
                    true,
                ))
                .when_some(err, |d, e| {
                    d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e))
                })
                .child(
                    row(t, false)
                        .child(div().flex_1().child(tr("Sync automatically")))
                        .child(
                            switch("sync-auto", t, c.auto)
                                .tooltip(tip(tr(
                                    "When Cleis opens, every 5 minutes and when it locks",
                                )))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(s) = this.session() {
                                        s.sync.auto = !s.sync.auto;
                                    }
                                    let p = this.path.clone();
                                    if let Some(s) = this.session() {
                                        crate::sync::save_config(&p, &s.sync);
                                    }
                                    cx.notify();
                                }))
                                .test_support(),
                        ),
                );
        }
        None => {
            sync = sync.child(
                row(t, true).child(
                    div()
                        .flex_1()
                        .text_color(t.text_muted)
                        .child(tr("This vault stays on this computer")),
                ),
            );
        }
    }
    let choose = |id: &'static str, p: Provider, cx: &mut Context<CleisApp>| {
        button(id, t, BtnKind::Default, true)
            .child(tr("Choose folder"))
            .on_click(cx.listener(move |this, _, _, cx| this.choose_sync_folder(p, cx)))
            .test_support()
    };
    let places =
        section(t)
            .child(
                div()
                    .id("places-title")
                    .child(section_title(
                        t,
                        if c.place.is_some() {
                            "Other places"
                        } else {
                            "Places"
                        },
                    ))
                    .tooltip(tip(tr(
                        "One place at a time; choosing another moves the copy.",
                    ))),
            )
            .child(place_row(
                "p-onedrive",
                t,
                "cloud",
                "OneDrive",
                "Folder kept by the OneDrive app",
                choose("sync-onedrive", Provider::OneDrive, cx),
                true,
            ))
            .child(place_row(
                "p-gdrive",
                t,
                "cloud",
                "Google Drive",
                "Folder kept by Google Drive for desktop",
                choose("sync-gdrive", Provider::GoogleDrive, cx),
                false,
            ))
            .child(place_row(
                "p-folder",
                t,
                "folder",
                "Folder",
                "Network share, Dropbox, USB drive",
                choose("sync-folder", Provider::Folder, cx),
                false,
            ))
            .child(place_row(
                "p-github",
                t,
                "repo",
                "GitHub",
                "Private repository",
                button("sync-github", t, BtnKind::Default, true)
                    .child(tr("Connect"))
                    .tooltip(tip(
                        "A file in one of your private repositories, with an access token",
                    ))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_place(Provider::GitHub, window, cx)
                    }))
                    .test_support(),
                false,
            ))
            .child(place_row(
                "p-sftp",
                t,
                "server",
                "SFTP",
                "Your own server",
                button("sync-sftp", t, BtnKind::Default, true)
                    .child(tr("Set up"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_place(Provider::Sftp, window, cx)
                    }))
                    .test_support(),
                false,
            ));
    let device = section(t)
        .child(
            div()
                .id("device-title")
                .child(section_title(t, tr("New device")))
                .tooltip(tip(tr("What a new computer needs besides your master password."))),
        )
        .child(
            row(t, true)
                .child(
                    div()
                        .id("kit-label")
                        .flex_1()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(icon("file", 16., t.text))
                        .child(tr("Emergency kit"))
                        .tooltip(tip(
                            "PDF with where your vault is and a space to write your master password. Print it and keep it safe.",
                        )),
                )
                .child(
                    button("kit", t, BtnKind::Default, true)
                        .child(icon("download", 14., t.text))
                        .child(tr("Download"))
                        .on_click(cx.listener(|this, _, _, cx| this.save_emergency_kit(cx)))
                        .test_support(),
                ),
        );
    let mut keep_menu = menu_box(t).top(px(32.)).right_0().w(px(220.));
    for k in KEEP {
        keep_menu = keep_menu.child(
            menu_item(
                format!("keep-{k}"),
                t,
                None,
                crate::trf!("{} versions", k),
                false,
            )
            .when(c.keep == k, |d| d.text_color(t.accent_text))
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(s) = this.session() {
                    s.sync.keep = k;
                    s.sync_menu = false;
                }
                let p = this.path.clone();
                if let Some(s) = this.session() {
                    crate::sync::save_config(&p, &s.sync);
                }
                cx.notify();
            }))
            .test_support(),
        );
    }
    let list = backups(&s.vault.path().unwrap_or_default());
    let mut history = div().flex().flex_col();
    for (i, (file, size, _)) in list.iter().take(5).enumerate() {
        let f = file.clone();
        let name = file
            .file_stem()
            .map(|n| n.to_string_lossy().trim_start_matches("vault-").to_owned())
            .unwrap_or_default();
        history = history.child(
            row(t, i == 0)
                .child(div().flex_1().child(crate::i18n::tr_owned(&name)))
                .child(
                    div()
                        .text_size(TEXT_SM)
                        .text_color(t.text_muted)
                        .child(crate::trf!("{} KB", size.div_ceil(1024))),
                )
                .child(
                    button(
                        SharedString::from(format!("restore-{i}")),
                        t,
                        BtnKind::Default,
                        true,
                    )
                    .child(tr("Restore"))
                    .tooltip(tip(
                        "Brings back what this backup holds; nothing newer is lost",
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| this.restore_backup(f.clone(), cx)))
                    .test_support(),
                ),
        );
    }
    let backups_sec = section(t)
        .child(
            div()
                .id("backups-title")
                .child(section_title(t, tr("Backups")))
                .tooltip(tip(tr("Dated copies on this computer you can go back to."))),
        )
        .child(
            row(t, true)
                .child(div().flex_1().child(tr("Keep")))
                .child(
                    div()
                        .relative()
                        .child(
                            div()
                                .id("keep")
                                .child(
                                    value_box(t)
                                        .w(px(160.))
                                        .cursor_pointer()
                                        .child(
                                            div()
                                                .flex_1()
                                                .child(crate::trf!("{} versions", c.keep)),
                                        )
                                        .child(icon("chevron", 14., t.text_muted)),
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(s) = this.session() {
                                        s.sync_menu = !s.sync_menu;
                                    }
                                    cx.notify();
                                }))
                                .test_support(),
                        )
                        .when(keep_open, |d| d.child(deferred(keep_menu))),
                )
                .child(
                    button("backup-now", t, BtnKind::Default, true)
                        .child(icon("upload", 14., t.text))
                        .child(tr("Back up now"))
                        .on_click(cx.listener(|this, _, _, cx| this.backup_now(cx)))
                        .test_support(),
                ),
        )
        .when(!list.is_empty(), |d| d.child(history))
        .child(
            row(t, false)
                .child(
                    div()
                        .id("export-label")
                        .flex_1()
                        .child(tr("Encrypted export"))
                        .tooltip(tip(tr(
                            "Copy of your vault, opened with your master password",
                        ))),
                )
                .child(
                    button("export", t, BtnKind::Default, true)
                        .child(icon("download", 14., t.text))
                        .child(tr("Export"))
                        .on_click(cx.listener(|this, _, _, cx| this.export_vault(cx)))
                        .test_support(),
                ),
        );
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .when(with_head, |d| d.child(head))
        .child(sync)
        .child(places)
        .child(device)
        .child(backups_sec)
}
