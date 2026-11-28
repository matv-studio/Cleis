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

pub(crate) fn ago(ts: i64) -> String {
    let d = cleis_core::now() - ts;
    match d {
        ..=59 => tr("just now").into(),
        60..=3599 => crate::trf!("{} min ago", d / 60),
        3600..=86_399 => crate::trf!("{} h ago", d / 3600),
        _ => crate::format::date(ts),
    }
}

fn place_row(
    id: impl Into<SharedString>,
    t: &Tokens,
    ico: &str,
    name: &str,
    sub: &str,
    action: impl IntoElement,
    first: bool,
) -> Div {
    // What the place is goes in a tooltip: no sentence under the name (27/11/2026, charter §1.5).
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
                .id(id.into())
                .flex_1()
                .min_w_0()
                .child(crate::i18n::tr_owned(name))
                .tooltip(crate::ui::tip(crate::i18n::tr_owned(sub)))
                .test_support(),
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
                .line_height(px(26.))
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
                "Your encrypted vault, kept the same on every device using these places. Your password stays on your devices.",
            )),
    );
    let ico = |p: Provider| match p {
        Provider::GitHub => "repo",
        Provider::Sftp => "server",
        Provider::Folder => "folder",
        _ => "cloud",
    };
    let status = |busy: bool, err: &Option<String>, last: Option<i64>| match (err, last) {
        _ if busy => chip(t, Tone::Neutral, tr("Syncing…"), false),
        (Some(_), _) => chip(t, Tone::Danger, tr("Not synced"), true),
        (None, Some(ts)) => chip(t, Tone::Ok, crate::trf!("Synced {}", ago(ts)), true),
        (None, None) => chip(t, Tone::Neutral, tr("Never synced"), false),
    };
    let open = s.place_menu.clone();
    // A place's "…" and its menu (Make main for a copy, Disconnect).
    let more = |key: String, copy: bool, cx: &mut Context<CleisApp>| {
        let shown = open.as_deref() == Some(key.as_str());
        let k = key.clone();
        let mut menu = menu_box(t).top(px(28.)).right_0().w(px(200.));
        if copy {
            let id = key.clone();
            menu = menu.child(
                menu_item(
                    SharedString::from(format!("place-main-{key}")),
                    t,
                    Some("star"),
                    tr("Make main").to_owned(),
                    false,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(s) = this.session() {
                        s.place_menu = None;
                    }
                    this.make_main(&id, cx)
                }))
                .test_support(),
            );
        }
        let id = key.clone();
        menu = menu.child(
            menu_item(
                SharedString::from(format!("place-off-{key}")),
                t,
                Some("x"),
                tr("Disconnect").to_owned(),
                false,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(s) = this.session() {
                    s.place_menu = None;
                }
                if copy {
                    this.disconnect_copy(&id, cx)
                } else {
                    this.ask_action(crate::app::ConfirmAction::DisconnectPlace, cx)
                }
            }))
            .test_support(),
        );
        div()
            .relative()
            .child(
                icon_button(
                    SharedString::from(format!("place-more-{key}")),
                    t,
                    "more",
                    true,
                )
                .tooltip(tip(if copy {
                    "Make main, disconnect"
                } else {
                    "Disconnect"
                }))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(s) = this.session() {
                        s.place_menu = if s.place_menu.as_deref() == Some(k.as_str()) {
                            None
                        } else {
                            Some(k.clone())
                        };
                    }
                    cx.notify();
                }))
                .test_support(),
            )
            .when(shown, |d| d.child(deferred(menu)))
    };
    // "Add a place" and its menu (mockup 49g).
    let add = |first: bool, cx: &mut Context<CleisApp>| {
        let shown = open.as_deref() == Some("add");
        let item = |id: &'static str, icon_name: &'static str, label: &'static str, p: Provider| {
            menu_item(id, t, Some(icon_name), tr(label).to_owned(), false)
                .on_click(cx.listener(move |this, _, window, cx| {
                    if let Some(s) = this.session() {
                        s.place_menu = None;
                    }
                    match p {
                        Provider::GoogleDrive => {
                            this.start_cloud(crate::cloud::Cloud::Google, window, cx)
                        }
                        Provider::OneDrive => {
                            this.start_cloud(crate::cloud::Cloud::Microsoft, window, cx)
                        }
                        Provider::Folder => this.choose_sync_folder(Provider::Folder, cx),
                        Provider::GitHub | Provider::Sftp => this.open_place(p, window, cx),
                    }
                }))
                .test_support()
        };
        let menu = menu_box(t)
            .top(px(32.))
            .right_0()
            .w(px(220.))
            .child(item(
                "add-gdrive",
                "cloud",
                "Google Drive",
                Provider::GoogleDrive,
            ))
            .child(item(
                "add-onedrive",
                "cloud",
                "OneDrive",
                Provider::OneDrive,
            ))
            .child(item("add-folder", "folder", "Folder", Provider::Folder))
            .child(item("add-github", "repo", "GitHub", Provider::GitHub))
            .child(item("add-sftp", "server", "SFTP", Provider::Sftp));
        div()
            .relative()
            .child(
                button("place-add", t, BtnKind::Default, true)
                    .child(icon("plus", 14., t.text))
                    .child(tr("Add a place"))
                    .tooltip(tip(if first {
                        "Where your vault is kept in step"
                    } else {
                        "Gets a copy of the main place at each sync"
                    }))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(s) = this.session() {
                            s.place_menu = if s.place_menu.as_deref() == Some("add") {
                                None
                            } else {
                                Some("add".into())
                            };
                        }
                        cx.notify();
                    }))
                    .test_support(),
            )
            .when(shown, |d| d.child(deferred(menu)))
    };
    match &c.place {
        Some(p) => {
            let err = c.last_error.clone().map(|e| crate::i18n::tr_dynamic(&e));
            sync = sync
                .child(place_row(
                    "sync-place",
                    t,
                    ico(c.provider),
                    c.provider.name(),
                    &c.remote
                        .as_ref()
                        .map_or_else(|| p.display().to_string(), |r| r.label()),
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .when(!c.copies.is_empty(), |d| {
                            d.child(
                                div()
                                    .id("place-main-chip")
                                    .child(chip(t, Tone::Set, tr("Main"), false))
                                    .tooltip(tip(
                                        "Merged with this computer at each sync; the other places hold a copy of it",
                                    ))
                                    .test_support(),
                            )
                        })
                        .child(status(c.busy, &c.last_error, c.last_sync))
                        .child(more("main".into(), false, cx)),
                    true,
                ))
                .when_some(err, |d, e| {
                    d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e))
                });
            let main_name = c.provider.name();
            for (n, cp) in c.copies.iter().enumerate() {
                let err = cp.last_error.clone().map(|e| crate::i18n::tr_dynamic(&e));
                sync = sync
                    .child(place_row(
                        format!("sync-copy-{n}"),
                        t,
                        ico(cp.provider),
                        cp.provider.name(),
                        &cp.label(),
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .id(SharedString::from(format!("copy-chip-{n}")))
                                    .child(chip(t, Tone::Neutral, tr("Copy@@place"), false))
                                    .tooltip(crate::ui::tip(crate::trf!(
                                        "Gets a copy of {} at each sync",
                                        main_name
                                    )))
                                    .test_support(),
                            )
                            .child(status(cp.busy, &cp.last_error, cp.last_sync))
                            .child(more(cp.id.clone(), true, cx)),
                        false,
                    ))
                    .when_some(err, |d, e| {
                        d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e))
                    });
            }
            sync = sync
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
                )
                .child(row(t, false).child(div().flex_1()).child(add(false, cx)));
        }
        None => {
            sync = sync.child(
                row(t, true)
                    .child(
                        div()
                            .size(px(28.))
                            .flex()
                            .flex_none()
                            .items_center()
                            .justify_center()
                            .rounded(RADIUS_MD)
                            .bg(t.surface_active)
                            .child(icon("laptop", 16., t.text)),
                    )
                    .child(
                        div()
                            .id("sync-local")
                            .flex_1()
                            .child(tr("Local"))
                            .tooltip(tip(tr("This vault stays on this computer")))
                            .test_support(),
                    )
                    .child(add(true, cx)),
            );
        }
    }
    let device = section(t)
        .child(
            div()
                .id("device-title")
                .child(section_title(t, tr("New device")))
                .tooltip(tip(tr("What a new computer needs besides your password."))),
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
                            "PDF with where your vault is and a space to write your password. Print it and keep it safe.",
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
    // Mockup 49i: the 5 latest, the others under "Show N more"; each one can be deleted.
    const SHOWN: usize = 5;
    let more = list.len().saturating_sub(SHOWN);
    let mut history = div().flex().flex_col();
    for (i, (file, size, _)) in list
        .iter()
        .take(if s.backups_all { usize::MAX } else { SHOWN })
        .enumerate()
    {
        let f = file.clone();
        let gone = file.clone();
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
                )
                .child(
                    icon_button(
                        SharedString::from(format!("del-backup-{i}")),
                        t,
                        "trash",
                        true,
                    )
                    .tooltip(tip("Delete this backup"))
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.ask_delete_backup(gone.clone(), cx)),
                    )
                    .test_support(),
                ),
        );
    }
    if more > 0 {
        history = history.child(
            row(t, false).child(div().flex_1()).child(
                button("backups-more", t, BtnKind::Ghost, true)
                    .child(
                        gpui_kit::svg()
                            .path("icons/chevron.svg")
                            .size(px(14.))
                            .flex_none()
                            .text_color(t.text_muted)
                            .when(s.backups_all, |d| {
                                d.with_transformation(gpui_kit::Transformation::rotate(
                                    gpui_kit::radians(std::f32::consts::PI),
                                ))
                            }),
                    )
                    .child(if s.backups_all {
                        tr("Show fewer").to_owned()
                    } else {
                        crate::trf!("Show {} more", more)
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(s) = this.session() {
                            s.backups_all = !s.backups_all;
                        }
                        cx.notify();
                    }))
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
                        .tooltip(tip(tr("Copy of your vault, opened with your password"))),
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
        .child(device)
        .child(backups_sec)
        // Mockup 37l: the deletions, at the very bottom (not on the organization's page).
        .when(with_head, |d| d.child(super::erase::section(s, t, cx)))
}
