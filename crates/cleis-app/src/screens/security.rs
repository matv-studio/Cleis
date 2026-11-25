//! Settings › Security (screen 06) and the verification code asked when Cleis opens (screen 05):
//! opening code with recovery codes, locking after inactivity, clipboard, master password.

use crate::app::{CleisApp, Session};
use crate::theme::*;
use crate::ui::*;
use cleis_core::open_code::{Check, OpenCode};
use gpui_kit::TestSupportExt;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, Entity, SharedString, Window, deferred, div, px};

pub enum SecDialog {
    ChangePassword {
        current: Entity<InputState>,
        new: Entity<InputState>,
        confirm: Entity<InputState>,
        error: Option<SharedString>,
        busy: bool,
    },
    /// Turning the opening code on: key to add to an authenticator app, then a code to check.
    SetupCode {
        code: OpenCode,
        input: Entity<InputState>,
        error: Option<SharedString>,
    },
    RecoveryCodes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecMenu {
    Clipboard,
    Lock,
}

#[derive(Default)]
pub struct SecurityUi {
    pub dialog: Option<SecDialog>,
    pub menu: Option<SecMenu>,
    pub subs: Vec<gpui_kit::Subscription>,
}

/// Code asked after the master password when the opening code is on.
pub struct Gate {
    pub input: Entity<InputState>,
    pub error: Option<SharedString>,
}

const CLIPBOARD: [u32; 4] = [10, 30, 60, 120];
const LOCK: [Option<u32>; 6] = [None, Some(1), Some(5), Some(15), Some(30), Some(60)];

fn secs_label(s: u32) -> String {
    if s < 60 {
        format!("{s} s")
    } else {
        format!("{} min", s / 60)
    }
}

fn lock_label(m: Option<u32>) -> String {
    match m {
        None => "Never".into(),
        Some(60) => "1 hour".into(),
        Some(m) => format!("{m} min"),
    }
}

fn tip(
    text: &'static str,
) -> impl Fn(&mut Window, &mut gpui_kit::App) -> gpui_kit::AnyView + 'static {
    move |window, cx| gpui_kit::component::tooltip::Tooltip::new(text).build(window, cx)
}

fn select_button(id: &'static str, t: &Tokens, label: String, w: f32) -> gpui_kit::Stateful<Div> {
    div().id(id).child(
        value_box(t)
            .w(px(w))
            .cursor_pointer()
            .child(div().flex_1().child(label))
            .child(icon("chevron", 14., t.text_muted)),
    )
}

pub fn page(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let p = &s.prefs;
    let ui = &s.security;
    let code_on = p.open_code.is_some();
    let left = p.open_code.as_ref().map_or(0, |c| c.recovery.len());
    let opening = section(t)
        .child(
            div()
                .id("sec-opening")
                .child(section_title(t, "Opening the app"))
                .tooltip(tip(
                    "Code asked each time Cleis opens, on top of the master password.",
                )),
        )
        .child(
            row(t, true)
                .child(div().flex_1().child("Verification code on open"))
                .child(
                    switch("open-code", t, code_on)
                        .on_click(
                            cx.listener(|this, _, window, cx| this.toggle_open_code(window, cx)),
                        )
                        .test_support(),
                ),
        )
        .when(code_on, |d| {
            d.child(
                row(t, false).child(div().flex_1().child("Method")).child(
                    div()
                        .id("code-method")
                        .child(segmented("method", t, &["App", "Email", "SMS", "Key"], 0))
                        .tooltip(tip("Email, SMS and security keys come with the server")),
                ),
            )
            .child(
                row(t, false)
                    .child(div().flex_1().child("Recovery codes"))
                    .child(
                        div()
                            .text_size(TEXT_SM)
                            .text_color(t.text_muted)
                            .child(format!("{left} left")),
                    )
                    .child(
                        button("show-recovery", t, BtnKind::Default, false)
                            .child("Show")
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(s) = this.session() {
                                    s.security.dialog = Some(SecDialog::RecoveryCodes);
                                }
                                cx.notify();
                            }))
                            .test_support(),
                    ),
            )
        });
    let lock_menu = {
        let mut m = menu_box(t).top(px(32.)).right_0().w(px(140.));
        for (i, v) in LOCK.into_iter().enumerate() {
            m = m.child(
                menu_item(format!("lock-{i}"), t, None, lock_label(v), false)
                    .when(p.lock_after_mins == v, |d| d.text_color(t.accent_text))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(s) = this.session() {
                            s.security.menu = None;
                        }
                        this.update_prefs(|p| p.lock_after_mins = v, cx);
                    }))
                    .test_support(),
            );
        }
        m
    };
    let locking = section(t)
        .child(section_title(t, "Locking"))
        .child(
            row(t, true)
                .child(div().flex_1().child("After inactivity"))
                .child(
                    div()
                        .relative()
                        .child(
                            select_button("lock-after", t, lock_label(p.lock_after_mins), 140.)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.toggle_sec_menu(SecMenu::Lock, cx)
                                }))
                                .test_support(),
                        )
                        .when(ui.menu == Some(SecMenu::Lock), |d| {
                            d.child(deferred(lock_menu))
                        }),
                ),
        )
        .child(
            row(t, false)
                .opacity(0.45)
                .child(
                    div()
                        .id("bio-row")
                        .flex_1()
                        .child("Biometrics (Windows Hello, Touch ID)")
                        .tooltip(tip("Planned")),
                )
                .child(switch("bio", t, false)),
        );
    let secs = p.clipboard_clear_secs;
    let clip_menu = {
        let mut m = menu_box(t).top(px(32.)).right_0().w(px(120.));
        for v in CLIPBOARD {
            m = m.child(
                menu_item(format!("clip-{v}"), t, None, secs_label(v), false)
                    .when(secs == Some(v), |d| d.text_color(t.accent_text))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(s) = this.session() {
                            s.security.menu = None;
                        }
                        this.update_prefs(|p| p.clipboard_clear_secs = Some(v), cx);
                    }))
                    .test_support(),
            );
        }
        m
    };
    let clipboard = section(t).child(section_title(t, "Clipboard")).child(
        row(t, true)
            .child(div().flex_1().child("Clear after copying"))
            .child(
                div()
                    .relative()
                    .child(
                        select_button("clip-after", t, secs_label(secs.unwrap_or(30)), 120.)
                            .when(secs.is_none(), |d| d.opacity(0.45))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_sec_menu(SecMenu::Clipboard, cx)
                            }))
                            .test_support(),
                    )
                    .when(ui.menu == Some(SecMenu::Clipboard), |d| {
                        d.child(deferred(clip_menu))
                    }),
            )
            .child(
                switch("clip", t, secs.is_some())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.update_prefs(
                            |p| {
                                p.clipboard_clear_secs = match p.clipboard_clear_secs {
                                    Some(_) => None,
                                    None => Some(30),
                                }
                            },
                            cx,
                        )
                    }))
                    .test_support(),
            ),
    );
    let changed = match p.password_changed {
        Some(ts) => format!("Changed {}", crate::format::date(ts)),
        None => "Never changed".into(),
    };
    let master = section(t).child(section_title(t, "Master password")).child(
        row(t, true)
            .child(div().flex_1().text_color(t.text_muted).child(changed))
            .child(
                button("chpw", t, BtnKind::Default, false)
                    .child("Change")
                    .on_click(
                        cx.listener(|this, _, window, cx| this.start_change_password(window, cx)),
                    )
                    .test_support(),
            ),
    );
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            div()
                .text_size(TEXT_XL)
                .font_weight(W_STRONG)
                .child("Security"),
        )
        .child(opening)
        .child(locking)
        .child(clipboard)
        .child(master)
}

fn card(t: &Tokens, title: &str) -> Div {
    div()
        .w(px(420.))
        .p(px(20.))
        .flex()
        .flex_col()
        .gap(px(12.))
        .bg(t.surface_raised)
        .border_1()
        .border_color(t.line_strong)
        .rounded(RADIUS_LG)
        .shadow_lg()
        .child(
            div()
                .text_size(TEXT_LG)
                .font_weight(W_MEDIUM)
                .child(title.to_owned()),
        )
}

fn actions(
    t: &Tokens,
    ok: &'static str,
    ok_label: String,
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    div()
        .pt(px(8.))
        .flex()
        .justify_end()
        .gap(px(8.))
        .child(
            button("sec-cancel", t, BtnKind::Default, false)
                .child("Cancel")
                .on_click(cx.listener(|this, _, window, cx| this.close_sec_dialog(window, cx)))
                .test_support(),
        )
        .child(
            button(ok, t, BtnKind::Primary, false)
                .child(ok_label)
                .on_click(cx.listener(move |this, _, window, cx| match ok {
                    "chpw-ok" => this.change_password(window, cx),
                    "code-ok" => this.confirm_open_code(cx),
                    _ => {
                        if let Some(s) = this.session() {
                            s.security.dialog = None;
                        }
                        cx.notify();
                    }
                }))
                .test_support(),
        )
}

/// QR code drawn with one square per module (quiet zone included).
fn qr(t: &Tokens, text: &str) -> Div {
    let Ok(code) = qrcode::QrCode::new(text.as_bytes()) else {
        return div();
    };
    let w = code.width();
    let colors = code.to_colors();
    let m = 4.;
    let mut grid = div()
        .p(px(m * 4.))
        .bg(gpui_kit::white())
        .rounded(RADIUS_MD)
        .flex()
        .flex_col();
    for y in 0..w {
        let mut line = div().flex();
        for x in 0..w {
            let dark = colors[y * w + x] == qrcode::Color::Dark;
            line = line.child(div().size(px(m)).when(dark, |d| d.bg(gpui_kit::black())));
        }
        grid = grid.child(line);
    }
    let _ = t;
    grid
}

pub fn dialog(s: &Session, d: &SecDialog, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let body = match d {
        SecDialog::ChangePassword {
            current,
            new,
            confirm,
            error,
            busy,
        } => {
            let typed = new.read(cx).value().to_string();
            card(t, "Change the master password")
                .child(field(
                    t,
                    "Current password",
                    Input::new(current).id("pw-current").mask_toggle(),
                ))
                .child(field(
                    t,
                    "New password",
                    Input::new(new).id("pw-new").mask_toggle(),
                ))
                .child(super::inspector::strength(t, &typed))
                .child(field(
                    t,
                    "Repeat the new password",
                    Input::new(confirm).id("pw-confirm").mask_toggle(),
                ))
                .when_some(error.clone(), |d, e| {
                    d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e))
                })
                .child(actions(
                    t,
                    "chpw-ok",
                    if *busy {
                        "Changing…".into()
                    } else {
                        "Change".into()
                    },
                    cx,
                ))
        }
        SecDialog::SetupCode { code, input, error } => card(t, "Verification code on open")
            .child(
                div()
                    .text_color(t.text_muted)
                    .child("Scan with an authenticator app, then type its code."),
            )
            .child(
                div()
                    .flex()
                    .justify_center()
                    .child(qr(t, &code.totp("My vault").to_uri())),
            )
            .child(
                value_box(t)
                    .child(
                        div()
                            .flex_1()
                            .font_family(FONT_MONO)
                            .text_size(TEXT_SM)
                            .child(code.grouped()),
                    )
                    .child(
                        icon_button("copy-key", t, "copy", true)
                            .tooltip(tip("Copy the key"))
                            .on_click({
                                let k = code.secret.clone();
                                cx.listener(move |this, _, _, cx| this.copy(k.clone(), "Key", cx))
                            }),
                    ),
            )
            .child(field(t, "Code", Input::new(input).id("code-check")))
            .when_some(error.clone(), |d, e| {
                d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e))
            })
            .child(actions(t, "code-ok", "Turn on".into(), cx)),
        SecDialog::RecoveryCodes => {
            let codes = s
                .prefs
                .open_code
                .as_ref()
                .map(|c| c.recovery.clone())
                .unwrap_or_default();
            let all = codes.join("\n");
            let mut grid = div().flex().flex_wrap().gap(px(8.));
            for c in &codes {
                grid = grid.child(div().w(px(180.)).font_family(FONT_MONO).child(c.clone()));
            }
            card(t, "Recovery codes")
                .child(
                    div()
                        .text_color(t.text_muted)
                        .child("Each opens Cleis once. Keep them outside Cleis."),
                )
                .child(grid)
                .child(
                    div()
                        .pt(px(8.))
                        .flex()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            button("copy-recovery", t, BtnKind::Default, false)
                                .child(icon("copy", 14., t.text))
                                .child("Copy")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.copy(all.clone(), "Recovery codes", cx)
                                }))
                                .test_support(),
                        )
                        .child(
                            button("recovery-done", t, BtnKind::Primary, false)
                                .child("Done")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.close_sec_dialog(window, cx)
                                }))
                                .test_support(),
                        ),
                )
        }
    };
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(gpui_kit::black().opacity(0.45))
        .child(div().id("sec-dialog").occlude().child(body))
}

/// Screen 05: code asked after the master password.
pub fn gate(g: &Gate, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(t.bg)
        .child(
            div()
                .w(px(360.))
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .size(px(40.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(10.))
                                .bg(t.inverse)
                                .child(icon("shield", 20., t.inverse_ink)),
                        )
                        .child(
                            div()
                                .text_size(TEXT_XL)
                                .font_weight(W_STRONG)
                                .child("Verification code"),
                        )
                        .child(div().text_color(t.text_muted).child("Authenticator app")),
                )
                .child(
                    section(t)
                        .gap(px(16.))
                        .child(Input::new(&g.input).id("gate-code"))
                        .when_some(g.error.clone(), |d, e| {
                            d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e))
                        })
                        .child(
                            button("gate-verify", t, BtnKind::Primary, false)
                                .h(px(32.))
                                .w_full()
                                .child("Verify")
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.check_gate(window, cx)),
                                )
                                .test_support(),
                        ),
                )
                .child(
                    div()
                        .id("gate-hint")
                        .flex()
                        .justify_center()
                        .text_size(TEXT_SM)
                        .text_color(t.text_muted)
                        .child("A recovery code works too")
                        .tooltip(tip(
                            "Shown when the code was turned on, in Settings › Security",
                        )),
                ),
        )
}

impl CleisApp {
    /// Closes the dialog and gives the keyboard back to the window (Ctrl+L, Ctrl+F).
    fn close_sec_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(s) = self.session() {
            s.security.dialog = None;
            window.focus(&s.main_focus.clone(), cx);
        }
        cx.notify();
    }

    fn toggle_sec_menu(&mut self, m: SecMenu, cx: &mut Context<Self>) {
        if let Some(s) = self.session() {
            s.security.menu = if s.security.menu == Some(m) {
                None
            } else {
                Some(m)
            };
        }
        cx.notify();
    }

    pub fn toggle_open_code(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        if s.prefs.open_code.is_some() {
            self.update_prefs(|p| p.open_code = None, cx);
            self.toast("Verification code turned off", cx);
            return;
        }
        let code = match OpenCode::generate() {
            Ok(c) => c,
            Err(e) => {
                s.toast = Some(format!("No key: {e}").into());
                cx.notify();
                return;
            }
        };
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("123 456"));
        input.update(cx, |st, cx| st.focus(window, cx));
        let sub = cx.subscribe_in(&input, window, |this, _, ev: &InputEvent, _, cx| {
            if let InputEvent::PressEnter { .. } = ev {
                this.confirm_open_code(cx);
            }
        });
        if let Some(s) = self.session() {
            s.security.dialog = Some(SecDialog::SetupCode {
                code,
                input,
                error: None,
            });
            s.security.subs.push(sub);
        }
        cx.notify();
    }

    /// Turns the opening code on once the app shows a matching code.
    pub fn confirm_open_code(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(SecDialog::SetupCode { code, input, error }) = s.security.dialog.as_mut() else {
            return;
        };
        let typed = input.read(cx).value().to_string();
        let mut probe = code.clone();
        probe.recovery.clear();
        if probe.check(&typed, cleis_core::now()) != Check::Code {
            *error = Some("Wrong code: check the time of this computer and of the phone".into());
            cx.notify();
            return;
        }
        let code = code.clone();
        s.security.dialog = Some(SecDialog::RecoveryCodes);
        self.update_prefs(move |p| p.open_code = Some(code), cx);
    }

    fn start_change_password(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mk = |cx: &mut Context<Self>, window: &mut Window| {
            cx.new(|cx| InputState::new(window, cx).masked(true))
        };
        let current = mk(cx, window);
        let new = mk(cx, window);
        let confirm = mk(cx, window);
        current.update(cx, |st, cx| st.focus(window, cx));
        let sub = cx.observe(&new, |_, _, cx| cx.notify());
        if let Some(s) = self.session() {
            s.security.dialog = Some(SecDialog::ChangePassword {
                current,
                new,
                confirm,
                error: None,
                busy: false,
            });
            s.security.subs.push(sub);
        }
        cx.notify();
    }

    /// Checks the current password, then re-seals the vault under the new one. Argon2id takes
    /// about half a second each time: the dialog shows "Changing…" meanwhile.
    pub fn change_password(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(SecDialog::ChangePassword {
            current,
            new,
            confirm,
            error,
            busy,
        }) = s.security.dialog.as_mut()
        else {
            return;
        };
        if *busy {
            return;
        }
        let cur = current.read(cx).value().to_string();
        let pw = new.read(cx).value().to_string();
        let again = confirm.read(cx).value().to_string();
        let refuse = if pw.chars().count() < 10 {
            Some("At least 10 characters")
        } else if pw != again {
            Some("The new passwords differ")
        } else if pw == cur {
            Some("The new password is the current one")
        } else {
            None
        };
        if let Some(e) = refuse {
            *error = Some(e.into());
            cx.notify();
            return;
        }
        *busy = true;
        *error = None;
        cx.notify();
        // Let "Changing…" be drawn before the work.
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(30))
                .await;
            this.update(cx, |this, cx| {
                let Some(s) = this.session() else { return };
                let result = s.vault.check_password(&cur).and_then(|ok| {
                    if !ok {
                        return Ok(false);
                    }
                    let kdf = s.vault.kdf()?;
                    s.vault.change_password(&pw, kdf).map(|()| true)
                });
                match result {
                    Ok(true) => {
                        s.security.dialog = None;
                        this.update_prefs(|p| p.password_changed = Some(cleis_core::now()), cx);
                        this.toast("Master password changed", cx);
                    }
                    other => {
                        if let Some(SecDialog::ChangePassword { error, busy, .. }) =
                            s.security.dialog.as_mut()
                        {
                            *busy = false;
                            *error = Some(match other {
                                Ok(_) => "Wrong current password".into(),
                                Err(e) => format!("Not changed: {e}").into(),
                            });
                        }
                        cx.notify();
                    }
                }
            })
            .ok();
        })
        .detach();
    }

    /// Opens the gate after the master password when the opening code is on.
    pub fn open_gate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        if s.prefs.open_code.is_none() {
            return;
        }
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("123 456"));
        input.update(cx, |st, cx| st.focus(window, cx));
        let sub = cx.subscribe_in(&input, window, |this, _, ev: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = ev {
                this.check_gate(window, cx);
            }
        });
        if let Some(s) = self.session() {
            s.gate = Some(Gate { input, error: None });
            s.security.subs.push(sub);
        }
    }

    pub fn check_gate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(g) = s.gate.as_mut() else { return };
        let typed = g.input.read(cx).value().to_string();
        let Some(code) = s.prefs.open_code.as_mut() else {
            s.gate = None;
            cx.notify();
            return;
        };
        match code.check(&typed, cleis_core::now()) {
            Check::Wrong => {
                g.error = Some("Wrong code".into());
                cx.notify();
            }
            Check::Code => {
                s.gate = None;
                window.focus(&s.main_focus.clone(), cx);
                cx.notify();
            }
            Check::Recovery => {
                s.gate = None;
                let left = s.prefs.open_code.as_ref().map_or(0, |c| c.recovery.len());
                // The used code is removed from the saved list.
                self.update_prefs(|_| {}, cx);
                self.toast_for(format!("Recovery code used · {left} left"), 8, cx);
            }
        }
    }
}
