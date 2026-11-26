//! Updates (D-74): the top bar's "Version x.y.z" button, its confirmation, and the Updates
//! section of Preferences (installed version, pre-releases, check now). Design: Revel's.

use std::time::Instant;

use gpui_kit::TestSupportExt;
use gpui_kit::prelude::*;
use gpui_kit::{AnyElement, Context, Div, SharedString, div, px};

use crate::app::CleisApp;
use crate::i18n::tr;
use crate::theme::*;
use crate::ui::*;
use crate::update::{self, Release};

/// What Cleis knows of the published versions.
#[derive(Default)]
pub struct UpdateState {
    /// Looking at GitHub is allowed (not in the demo, the screenshots or the tests).
    pub enabled: bool,
    /// Pre-releases offered (Preferences, kept in `appearance.json` too).
    pub pre: bool,
    pub available: Option<Release>,
    pub checking: bool,
    /// The last answer of a check asked by hand: "Up to date" or the error.
    pub status: Option<String>,
    pub next: Option<Instant>,
    /// "Install … and restart?" shown.
    pub confirm: bool,
    pub installing: bool,
}

impl CleisApp {
    /// From the one-second tick: at start, then every six hours.
    pub fn update_due(&mut self, cx: &mut Context<Self>) {
        let u = &self.update;
        if !u.enabled || u.checking || u.installing {
            return;
        }
        if u.next.is_none_or(|n| Instant::now() >= n) {
            self.check_update(false, cx);
        }
    }

    /// Asks GitHub (`by_hand`: the answer is shown in Preferences).
    pub fn check_update(&mut self, by_hand: bool, cx: &mut Context<Self>) {
        if self.update.checking {
            return;
        }
        self.update.checking = true;
        self.update.next = Some(Instant::now() + update::EVERY);
        if by_hand {
            self.update.status = None;
        }
        let pre = self.update.pre;
        let task = cx.background_spawn(async move { update::check(pre) });
        cx.spawn(async move |this, cx| {
            let r = task.await;
            this.update(cx, |app, cx| {
                let u = &mut app.update;
                u.checking = false;
                match r {
                    Ok(found) => {
                        if by_hand && found.is_none() {
                            u.status = Some(tr("Up to date").into());
                        }
                        u.available = found;
                    }
                    Err(e) => {
                        if by_hand {
                            u.status = Some(e);
                        }
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// The top bar's button: asks first where Cleis can replace itself, otherwise opens the
    /// release page.
    pub fn open_update(&mut self, cx: &mut Context<Self>) {
        let Some(r) = self.update.available.clone() else {
            return;
        };
        if r.package.is_some() {
            self.update.confirm = true;
            cx.notify();
        } else {
            cx.open_url(&r.page);
        }
    }

    pub fn install_update(&mut self, cx: &mut Context<Self>) {
        let Some(r) = self.update.available.clone() else {
            return;
        };
        if self.update.installing {
            return;
        }
        self.update.installing = true;
        cx.notify();
        let demo = self.demo;
        let task = cx.background_spawn(async move {
            if demo {
                return Err("demo".to_owned());
            }
            update::install(&r)?;
            update::restart()
        });
        cx.spawn(async move |this, cx| {
            let r = task.await;
            this.update(cx, |app, cx| match r {
                Ok(()) => cx.quit(),
                Err(e) => {
                    app.update.installing = false;
                    app.update.confirm = false;
                    app.toast_for(crate::trf!("Update not installed: {}", e), 8, cx);
                }
            })
            .ok();
        })
        .detach();
    }
}

/// Top bar, right: shown while a newer version is published.
pub fn top_button(app: &CleisApp, t: &Tokens, cx: &mut Context<CleisApp>) -> Option<AnyElement> {
    let r = app.update.available.as_ref()?;
    Some(
        button("update", t, BtnKind::Default, false)
            .child(icon("download", 16., t.text_muted))
            .child(crate::trf!("Version {}", r.version))
            .tooltip(|window, cx| {
                gpui_kit::component::tooltip::Tooltip::new(tr("Install the new version"))
                    .build(window, cx)
            })
            .on_click(cx.listener(|this, _, _, cx| this.open_update(cx)))
            .test_support()
            .into_any_element(),
    )
}

/// "Install Cleis x.y.z and restart?"
pub fn dialog(app: &CleisApp, t: &Tokens, cx: &mut Context<CleisApp>) -> Option<Div> {
    if !app.update.confirm {
        return None;
    }
    let r = app.update.available.as_ref()?;
    let busy = app.update.installing;
    Some(
        scrim(t).child(
            dialog_box(t)
                .id("update-dialog")
                .occlude()
                .child(
                    div()
                        .text_size(TEXT_LG)
                        .font_weight(W_MEDIUM)
                        .child(crate::trf!("Install Cleis {} and restart?", r.version)),
                )
                .child(
                    div()
                        .pt(px(8.))
                        .flex()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            button("update-cancel", t, BtnKind::Default, false)
                                .child(tr("Cancel"))
                                .when(!busy, |b| {
                                    b.on_click(cx.listener(|this, _, _, cx| {
                                        this.update.confirm = false;
                                        cx.notify();
                                    }))
                                })
                                .test_support(),
                        )
                        .child(
                            button("update-install", t, BtnKind::Primary, false)
                                .when(busy, |b| b.opacity(0.6))
                                .child(if busy {
                                    tr("Installing…")
                                } else {
                                    tr("Install and restart")
                                })
                                .when(!busy, |b| {
                                    b.on_click(
                                        cx.listener(|this, _, _, cx| this.install_update(cx)),
                                    )
                                })
                                .test_support(),
                        ),
                ),
        ),
    )
}

/// Preferences › Updates.
pub fn section_of(app: &CleisApp, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let u = &app.update;
    let status: Option<SharedString> = if u.checking {
        Some(tr("Checking…").into())
    } else if let Some(r) = &u.available {
        Some(crate::trf!("Version {} available", r.version).into())
    } else {
        u.status.clone().map(|s| crate::i18n::tr_dynamic(&s).into())
    };
    section(t)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(section_title(t, tr("Updates")))
                .child(hint(
                    "updates-hint",
                    t,
                    tr("Cleis looks for a new version on GitHub when it starts, then every six hours"),
                )),
        )
        .child(
            row(t, true)
                .child(div().flex_1().child(tr("Installed version")))
                .child(
                    div()
                        .id("installed-version")
                        .font_weight(W_MEDIUM)
                        .child(update::current()),
                ),
        )
        .child(
            row(t, false)
                .child(div().flex_1().child(tr("Receive pre-releases")))
                .child(
                    switch("update-pre", t, u.pre)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.update_prefs(
                                |p| p.update_prereleases = !p.update_prereleases,
                                cx,
                            );
                            this.update.available = None;
                            this.check_update(true, cx);
                        }))
                        .test_support(),
                ),
        )
        .child(
            row(t, false)
                .child(
                    button("check-update", t, BtnKind::Default, false)
                        .when(u.checking, |b| b.opacity(0.6))
                        .child(tr("Check for updates"))
                        .on_click(cx.listener(|this, _, _, cx| this.check_update(true, cx)))
                        .test_support(),
                )
                .child(div().flex_1())
                .when_some(status, |d, s| {
                    d.child(
                        div()
                            .id("update-status")
                            .text_color(t.text_muted)
                            .child(s),
                    )
                })
                .when(u.available.is_some(), |d| {
                    d.child(
                        button("update-now", t, BtnKind::Primary, false)
                            .child(tr("Install"))
                            .on_click(cx.listener(|this, _, _, cx| this.open_update(cx)))
                            .test_support(),
                    )
                }),
        )
}
