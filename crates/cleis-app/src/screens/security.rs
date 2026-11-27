//! Settings › Security (screen 06) and the verification code asked when Cleis opens (screen 05):
//! opening code with recovery codes, locking after inactivity, clipboard, master password.

use crate::app::{CleisApp, Session};
use crate::i18n::tr;
use crate::theme::*;
use crate::ui::*;
use cleis_core::open_code::{Check, OpenCode};
use gpui_kit::TestSupportExt;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, Entity, SharedString, Window, deferred, div, px};

pub enum SecDialog {
    ChangePassword {
        /// The vault has a master password to check first (none: setting one, D-76).
        has_current: bool,
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
    /// A new recovery key, shown once (D-76).
    RecoveryKey(zeroize::Zeroizing<String>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecMenu {
    Clipboard,
    Lock,
    /// The PIN's wrong tries allowed (mockup 37e).
    PinTries,
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
        crate::trf!("{} min", s / 60)
    }
}

fn lock_label(m: Option<u32>) -> String {
    match m {
        None => tr("Never").into(),
        Some(60) => tr("1 hour").into(),
        Some(m) => crate::trf!("{} min", m),
    }
}

fn tip(
    text: &'static str,
) -> impl Fn(&mut Window, &mut gpui_kit::App) -> gpui_kit::AnyView + 'static {
    move |window, cx| {
        gpui_kit::component::tooltip::Tooltip::new(crate::i18n::tr_dynamic(text)).build(window, cx)
    }
}

fn select_button(id: &'static str, t: &Tokens, label: String, w: f32) -> gpui_kit::Stateful<Div> {
    div().id(id).child(
        value_box(t)
            .w(px(w))
            .cursor_pointer()
            .child(div().flex_1().child(crate::i18n::tr_owned(&label)))
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
                .child(section_title(t, tr("Verification code")))
                .tooltip(tip(
                    "Code asked each time Cleis opens, on top of the master password. This computer only: an organization's two-step verification is in Profile.",
                )),
        )
        .child(
            row(t, true)
                .child(div().flex_1().child(tr("Verification code on open")))
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
                row(t, false).child(div().flex_1().child(tr("Method"))).child(
                    div()
                        .id("code-method")
                        .child(segmented("method", t, &[tr("App"), tr("Email"), "SMS"], 0))
                        .tooltip(tip(tr("Email and SMS need a mail server or a paid service"))),
                ),
            )
            .child(
                row(t, false)
                    .child(div().flex_1().child(tr("Recovery codes")))
                    .child(
                        div()
                            .text_size(TEXT_SM)
                            .text_color(t.text_muted)
                            .child(crate::trf!("{} left", left)),
                    )
                    .child(
                        button("show-recovery", t, BtnKind::Default, false)
                            .child(tr("Show"))
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
        let mut m = menu_box(t).top(px(32.)).right_0().w(px(220.));
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
    let locking =
        section(t).child(section_title(t, tr("Locking"))).child(
            row(t, true)
                .child(div().flex_1().child(tr("After inactivity")))
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
        );
    let secs = p.clipboard_clear_secs;
    let clip_menu = {
        let mut m = menu_box(t).top(px(32.)).right_0().w(px(220.));
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
    let clipboard = section(t).child(section_title(t, tr("Clipboard"))).child(
        row(t, true)
            .child(div().flex_1().child(tr("Clear after copying")))
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
        Some(ts) => crate::trf!("Changed {}", crate::format::date(ts)),
        None => tr("Never changed").into(),
    };
    let _ = changed;
    let unlock = opening_section(s, t, cx);
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            div()
                .text_size(TEXT_XL)
                .line_height(px(26.))
                .font_weight(W_STRONG)
                .child(tr("Security")),
        )
        .child(unlock)
        .child(opening)
        .child(locking)
        .child(clipboard)
}

/// Mockup 35h (D-76): how this computer opens the vault, the recovery key, the master password,
/// and the organization's rule.
fn opening_section(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    use crate::opening::Mode;
    let has_password = s.vault.has_password().unwrap_or(true);
    let recovery = s.vault.recovery_saved().ok().flatten();
    let (rule, by) = s.unlock_rule();
    let hello_ok = crate::opening::hello_available();
    let modes = [
        (
            Mode::Password,
            "key",
            "Master password",
            None::<&'static str>,
        ),
        (
            Mode::Pin,
            "hash",
            "PIN",
            if rule.allows_hello() {
                Some(
                    "Digits only, on this computer; the master password after too many wrong tries",
                )
            } else {
                Some("Not allowed by your organization")
            },
        ),
        (
            Mode::Hello,
            "fingerprint",
            "Windows Hello",
            if !rule.allows_hello() {
                Some("Not allowed by your organization")
            } else if !hello_ok {
                Some("Windows Hello is not set up on this computer")
            } else {
                Some("Face, fingerprint or PIN of this Windows account")
            },
        ),
        (
            Mode::Nothing,
            "laptop",
            "Nothing",
            if rule.allows_nothing() {
                Some("Anyone using this Windows account opens Cleis")
            } else {
                Some("Not allowed by your organization")
            },
        ),
    ];
    let mut seg = div()
        .flex()
        .flex_none()
        .p(px(2.))
        .gap(px(2.))
        .bg(t.bg)
        .border_1()
        .border_color(t.line_strong)
        .rounded(RADIUS_MD);
    for (mode, ico, label, why) in modes {
        let on = s.opening == mode;
        let allowed = match mode {
            Mode::Password => true,
            Mode::Pin => rule.allows_hello(),
            Mode::Hello => rule.allows_hello() && hello_ok,
            Mode::Nothing => rule.allows_nothing(),
        };
        let fg = if on { t.accent_text } else { t.text_muted };
        seg = seg.child(
            div()
                .id(SharedString::from(format!("opening-{mode:?}")))
                .h(px(24.))
                .px(px(10.))
                .flex()
                .items_center()
                .gap(px(6.))
                .rounded(RADIUS_SM)
                .font_weight(W_MEDIUM)
                .text_color(fg)
                .when(on, |d| d.bg(t.accent_soft))
                .when(allowed, |d| d.cursor_pointer())
                .when(!allowed, |d| d.opacity(0.45))
                .child(icon(ico, 14., fg))
                .child(tr(label))
                .when_some(why, |d, w| d.tooltip(tip(w)))
                .when(allowed && !on, |d| {
                    d.on_click(
                        cx.listener(move |this, _, window, cx| this.set_opening(mode, window, cx)),
                    )
                })
                .test_support(),
        );
    }
    let saved = match recovery {
        Some(ms) => crate::trf!("Saved {}", crate::format::date(ms / 1000)).into(),
        None => SharedString::from(tr("None")),
    };
    let changed = match (has_password, s.vault.password_changed().ok().flatten()) {
        (false, _) => SharedString::from(tr("None")),
        (true, Some(ms)) => crate::trf!("Changed {}", crate::format::date(ms / 1000)).into(),
        (true, None) => match s.prefs.password_changed {
            Some(ts) => crate::trf!("Changed {}", crate::format::date(ts)).into(),
            None => SharedString::from(tr("Never changed")),
        },
    };
    // The password goes only with a recovery key and another way in on this computer.
    let removable = has_password
        && recovery.is_some()
        && s.opening != Mode::Password
        && rule.allows_no_password();
    let remove_why = if !rule.allows_no_password() {
        "Not allowed by your organization"
    } else if recovery.is_none() {
        "Make a recovery key first"
    } else if s.opening == Mode::Password {
        "Choose a PIN, Windows Hello or Nothing first"
    } else {
        "The recovery key then opens your vault on another computer"
    };
    let muted = |text: SharedString| {
        div()
            .text_size(TEXT_SM)
            .text_color(t.text_muted)
            .child(text)
    };
    section(t)
        .child(
            div()
                .id("sec-unlock")
                .child(section_title(t, tr("Opening Cleis")))
                .tooltip(tip(tr(
                    "How this computer opens your vault. On another computer, or after reinstalling Windows: the master password or the recovery key.",
                ))),
        )
        .child(
            row(t, true)
                .child(div().flex_1().child(tr("Unlock with")))
                .child(seg),
        )
        // Mockup 37e: the PIN's length, the tries allowed, a new PIN.
        .when_some(crate::opening::pin_info(&s.vault.path().unwrap_or_default()), |d, info| {
            d.children(pin_rows(s, info, t, cx))
        })
        .child(
            row(t, true)
                .child(
                    div()
                        .id("recovery-label")
                        .flex_1()
                        .child(tr("Recovery key"))
                        .tooltip(tip(tr("Opens your vault anywhere; keep it outside Cleis"))),
                )
                .child(muted(saved))
                .child(
                    button("recovery-new", t, BtnKind::Default, false)
                        .child(if recovery.is_some() {
                            tr("Replace")
                        } else {
                            tr("Create")
                        })
                        .when(recovery.is_some(), |d| {
                            d.tooltip(tip(tr("A new key; the previous one stops working")))
                        })
                        .on_click(cx.listener(|this, _, _, cx| this.new_recovery_key(cx)))
                        .test_support(),
                ),
        )
        .child(
            row(t, false)
                .child(div().flex_1().child(tr("Master password")))
                .child(muted(changed))
                .child(
                    button("chpw", t, BtnKind::Default, false)
                        .child(if has_password { tr("Change") } else { tr("Set") })
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.start_change_password(window, cx)
                        }))
                        .test_support(),
                )
                .when(has_password, |d| {
                    d.child(
                        button("rmpw", t, BtnKind::Ghost, false)
                            .child(tr("Remove"))
                            .tooltip(tip(tr(remove_why)))
                            .when(!removable, |b| b.opacity(0.45).cursor_default())
                            .when(removable, |b| {
                                b.on_click(cx.listener(|this, _, _, cx| {
                                    this.ask_action(
                                        crate::app::ConfirmAction::RemovePassword,
                                        cx,
                                    )
                                }))
                            })
                            .test_support(),
                    )
                }),
        )
        .when_some(by, |d, org| {
            let label = match rule {
                cleis_core::org::UnlockRule::PasswordOnly => tr("Master password only"),
                cleis_core::org::UnlockRule::PasswordOrHello => {
                    tr("Master password, PIN or Windows Hello")
                }
                cleis_core::org::UnlockRule::Any => tr("Anything, also nothing"),
            };
            d.child(
                row(t, false)
                    .child(
                        div()
                            .id("unlock-rule")
                            .flex_1()
                            .text_color(t.text_muted)
                            .child(tr("Your organization"))
                            .tooltip(crate::ui::tip(crate::trf!("Set by {}", org))),
                    )
                    .child(muted(label.into())),
            )
        })
}

/// The PIN's rows under "Unlock with" (mockup 37e).
fn pin_rows(
    s: &Session,
    info: crate::opening::PinInfo,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> Vec<Div> {
    let mut len_seg = div()
        .flex()
        .flex_none()
        .p(px(2.))
        .gap(px(2.))
        .bg(t.bg)
        .border_1()
        .border_color(t.line_strong)
        .rounded(RADIUS_MD);
    for n in crate::opening::PIN_LENGTHS {
        let on = info.len == n;
        len_seg = len_seg.child(
            div()
                .id(SharedString::from(format!("pin-len-{n}")))
                .h(px(24.))
                .w(px(32.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(RADIUS_SM)
                .font_weight(W_MEDIUM)
                .when(on, |d| d.bg(t.accent_soft).text_color(t.accent_text))
                .when(!on, |d| d.text_color(t.text_muted).cursor_pointer())
                .child(n.to_string())
                .when(!on, |d| {
                    d.tooltip(tip(tr("A new PIN of this length"))).on_click(
                        cx.listener(move |this, _, window, cx| this.open_pin_setup(n, window, cx)),
                    )
                })
                .test_support(),
        );
    }
    let open = s.security.menu == Some(SecMenu::PinTries);
    let mut menu = menu_box(t).relative().w(px(160.));
    for n in crate::opening::PIN_TRIES {
        menu = menu.child(
            menu_item(format!("pin-tries-{n}"), t, None, n.to_string(), false)
                .when(n == info.max_tries, |d| d.text_color(t.accent_text))
                .on_click(cx.listener(move |this, _, _, cx| {
                    let path = this.path.clone();
                    if let Some(s) = this.session() {
                        s.security.menu = None;
                        if let Err(e) = crate::opening::set_pin_tries(&path, n) {
                            s.toast = Some(crate::i18n::tr_dynamic(&e).into());
                        }
                    }
                    cx.notify();
                }))
                .test_support(),
        );
    }
    let tries = div()
        .relative()
        .child(
            div()
                .id("pin-tries")
                .child(
                    value_box(t)
                        .w(px(160.))
                        .cursor_pointer()
                        .child(div().flex_1().child(info.max_tries.to_string()))
                        .child(icon("chevron", 14., t.text_muted)),
                )
                .on_click(cx.listener(|this, _, _, cx| this.toggle_sec_menu(SecMenu::PinTries, cx)))
                .test_support(),
        )
        .when(open, |d| d.child(popup_below(menu, px(28.))));
    vec![
        row(t, true)
            .child(div().flex_1().child(tr("Length")))
            .child(len_seg),
        row(t, true)
            .child(
                div()
                    .id("pin-tries-label")
                    .flex_1()
                    .child(tr("Wrong tries before locking"))
                    .tooltip(tip(tr("Then the master password or the recovery key"))),
            )
            .child(tries),
        row(t, true).child(div().flex_1().child(tr("PIN"))).child(
            button("pin-change", t, BtnKind::Default, false)
                .child(tr("Change"))
                .on_click(
                    cx.listener(move |this, _, window, cx| {
                        this.open_pin_setup(info.len, window, cx)
                    }),
                )
                .test_support(),
        ),
    ]
}

fn card(t: &Tokens, title: &str) -> Div {
    dialog_box(t).child(
        div()
            .text_size(TEXT_LG)
            .line_height(px(20.))
            .font_weight(W_MEDIUM)
            .child(crate::i18n::tr_owned(title)),
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
                .child(tr("Cancel"))
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
pub fn qr(t: &Tokens, text: &str) -> Div {
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
            has_current,
            current,
            new,
            confirm,
            error,
            busy,
        } => {
            let typed = new.read(cx).value().to_string();
            card(
                t,
                if *has_current {
                    tr("Change the master password")
                } else {
                    tr("Set a master password")
                },
            )
            .when(*has_current, |d| {
                d.child(field(
                    t,
                    "Current password",
                    Input::new(current).id("pw-current").mask_toggle(),
                ))
            })
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
                    tr("Changing…").into()
                } else {
                    tr("Change").into()
                },
                cx,
            ))
        }
        SecDialog::SetupCode { code, input, error } => card(t, tr("Verification code on open"))
            .child(hint(
                "hint-open-code",
                t,
                "Scan with an authenticator app, then type its code.",
            ))
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
                            .tooltip(tip(tr("Copy the key")))
                            .on_click({
                                let k = code.secret.clone();
                                cx.listener(move |this, _, _, cx| this.copy(k.clone(), "Key", cx))
                            }),
                    ),
            )
            .child(field(t, tr("Code"), Input::new(input).id("code-check")))
            .when_some(error.clone(), |d, e| {
                d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e))
            })
            .child(actions(t, "code-ok", tr("Turn on").into(), cx)),
        SecDialog::RecoveryKey(key) => {
            let text = key.to_string();
            dialog_box(t)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .text_size(TEXT_LG).line_height(px(20.))
                                .font_weight(W_MEDIUM)
                                .child(tr("Recovery key")),
                        )
                        .child(hint(
                            "hint-recovery-key",
                            t,
                            "Opens your vault on any computer, with or without the master password. Keep it outside Cleis: it is shown only now.",
                        )),
                )
                .child(
                    div()
                        .id("recovery-key-text")
                        .p(px(12.))
                        .bg(t.bg)
                        .border_1()
                        .border_color(t.line_strong)
                        .rounded(RADIUS_MD)
                        .font_family(FONT_MONO)
                        .text_size(TEXT_MD)
                        .child(text.clone()),
                )
                .child(
                    div()
                        .pt(px(8.))
                        .flex()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            button("copy-recovery-key", t, BtnKind::Default, false)
                                .child(icon("copy", 14., t.text))
                                .child(tr("Copy"))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.copy(text.clone(), "Recovery key", cx)
                                }))
                                .test_support(),
                        )
                        .child(
                            button("recovery-key-done", t, BtnKind::Primary, false)
                                .child(tr("I saved it"))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.close_sec_dialog(window, cx)
                                }))
                                .test_support(),
                        ),
                )
        }
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
            card(t, tr("Recovery codes"))
                .child(hint(
                    "hint-recovery",
                    t,
                    "Each opens Cleis once. Keep them outside Cleis.",
                ))
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
                                .child(tr("Copy"))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.copy(all.clone(), "Recovery codes", cx)
                                }))
                                .test_support(),
                        )
                        .child(
                            button("recovery-done", t, BtnKind::Primary, false)
                                .child(tr("Done"))
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
        .bg(t.overlay)
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
                                .line_height(px(26.))
                                .font_weight(W_STRONG)
                                .child(tr("Verification code")),
                        )
                        .child(
                            div()
                                .text_color(t.text_muted)
                                .child(tr("Authenticator app")),
                        ),
                )
                .child(
                    section(t)
                        .gap(px(16.))
                        .child(Input::new(&g.input).id("gate-code"))
                        .when_some(
                            g.error.clone().map(|e| crate::i18n::tr_dynamic(&e)),
                            |d, e| {
                                d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e))
                            },
                        )
                        .child(
                            button("gate-verify", t, BtnKind::Primary, false)
                                .h(px(32.))
                                .w_full()
                                .child(tr("Verify"))
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
                        .child(tr("A recovery code works too"))
                        .tooltip(tip(
                            "Shown when the code was turned on, in Settings › Security",
                        )),
                ),
        )
}

impl CleisApp {
    /// Closes the dialog and gives the keyboard back to the window (Ctrl+L, Ctrl+F).
    /// A new recovery key (D-76), shown once.
    pub fn new_recovery_key(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        match s.vault.new_recovery_key() {
            Ok(key) => s.security.dialog = Some(SecDialog::RecoveryKey(key)),
            Err(e) => s.toast = Some(crate::trf!("Not saved: {}", e).into()),
        }
        cx.notify();
    }

    /// "Unlock with" (mockup 35h). Windows Hello asks for the face, fingerprint or PIN first;
    /// the master password, if the vault has none, is set first.
    pub fn set_opening(
        &mut self,
        mode: crate::opening::Mode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use crate::opening::Mode;
        let Some(s) = self.session() else { return };
        match mode {
            Mode::Password if !s.vault.has_password().unwrap_or(true) => {
                s.after_password = Some(Mode::Password);
                self.start_change_password(window, cx);
            }
            // Mockup 37f: the PIN typed twice first; kept only then.
            Mode::Pin => self.open_pin_setup(crate::opening::DEFAULT_PIN_LEN, window, cx),
            Mode::Hello => {
                let challenge = crate::opening::new_challenge();
                cx.spawn_in(window, async move |this, cx| {
                    let c = challenge.clone();
                    let key = cx
                        .background_executor()
                        .spawn(async move { crate::opening::hello_key(&c) })
                        .await;
                    this.update(cx, |this, cx| {
                        let path = this.path.clone();
                        let Some(s) = this.session() else { return };
                        match key
                            .and_then(|k| crate::opening::set_hello(&path, &s.vault, challenge, &k))
                        {
                            Ok(()) => {
                                s.opening = Mode::Hello;
                                this.toast(tr("Windows Hello opens Cleis on this computer"), cx);
                            }
                            Err(e) => {
                                s.toast = Some(crate::i18n::tr_dynamic(&e).into());
                                cx.notify();
                            }
                        }
                    })
                    .ok();
                })
                .detach();
            }
            other => self.apply_opening(other, cx),
        }
    }

    /// Writes (or removes) this computer's key for `mode` (not Windows Hello: see `set_opening`).
    pub fn apply_opening(&mut self, mode: crate::opening::Mode, cx: &mut Context<Self>) {
        use crate::opening::Mode;
        let path = self.path.clone();
        let Some(s) = self.session() else { return };
        let done = match mode {
            Mode::Password => {
                crate::opening::forget(&path);
                Ok(tr("The master password opens Cleis on this computer"))
            }
            Mode::Nothing => crate::opening::set_nothing(&path, &s.vault)
                .map(|()| tr("Cleis opens without a password on this computer")),
            Mode::Hello | Mode::Pin => return,
        };
        match done {
            Ok(msg) => {
                s.opening = mode;
                self.toast(msg, cx);
            }
            Err(e) => {
                s.toast = Some(crate::i18n::tr_dynamic(&e).into());
                cx.notify();
            }
        }
    }

    /// Confirmed (D-76): the recovery key and this computer's key open the vault from now on.
    pub fn remove_password(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        match s.vault.remove_password() {
            Ok(()) => self.toast(tr("Master password removed"), cx),
            Err(e) => {
                s.toast = Some(crate::trf!("Not changed: {}", e).into());
                cx.notify();
            }
        }
    }

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
            self.toast(tr("Verification code turned off"), cx);
            return;
        }
        let code = match OpenCode::generate() {
            Ok(c) => c,
            Err(e) => {
                s.toast = Some(crate::trf!("No key: {}", e).into());
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
            *error =
                Some(tr("Wrong code: check the time of this computer and of the phone").into());
            cx.notify();
            return;
        }
        let code = code.clone();
        s.security.dialog = Some(SecDialog::RecoveryCodes);
        self.update_prefs(move |p| p.open_code = Some(code), cx);
    }

    pub fn start_change_password(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mk = |cx: &mut Context<Self>, window: &mut Window| {
            cx.new(|cx| InputState::new(window, cx).masked(true))
        };
        let current = mk(cx, window);
        let new = mk(cx, window);
        let confirm = mk(cx, window);
        let has_current = self
            .session()
            .is_none_or(|s| s.vault.has_password().unwrap_or(true));
        if has_current {
            current.update(cx, |st, cx| st.focus(window, cx));
        } else {
            new.update(cx, |st, cx| st.focus(window, cx));
        }
        let sub = cx.observe(&new, |_, _, cx| cx.notify());
        if let Some(s) = self.session() {
            s.security.dialog = Some(SecDialog::ChangePassword {
                has_current,
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
        let min = s.master_min();
        let Some(SecDialog::ChangePassword {
            has_current,
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
        let has_current = *has_current;
        let cur = current.read(cx).value().to_string();
        let pw = new.read(cx).value().to_string();
        let again = confirm.read(cx).value().to_string();
        let refuse = if pw.chars().count() < min {
            Some(crate::trf!("At least {} characters", min))
        } else if pw != again {
            Some(tr("The new passwords differ").into())
        } else if has_current && pw == cur {
            Some(tr("The new password is the current one").into())
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
                let result = if has_current {
                    s.vault.check_password(&cur)
                } else {
                    Ok(true)
                }
                .and_then(|ok| {
                    if !ok {
                        return Ok(false);
                    }
                    // No password yet (D-76): the default derivation settings.
                    let kdf = s.vault.kdf().unwrap_or_default();
                    s.vault.change_password(&pw, kdf).map(|()| true)
                });
                match result {
                    Ok(true) => {
                        s.security.dialog = None;
                        this.update_prefs(|p| p.password_changed = Some(cleis_core::now()), cx);
                        this.toast(
                            if has_current {
                                tr("Master password changed")
                            } else {
                                tr("Master password set")
                            },
                            cx,
                        );
                        if let Some(then) = this.session().and_then(|s| s.after_password.take()) {
                            this.apply_opening(then, cx);
                        }
                    }
                    other => {
                        if let Some(SecDialog::ChangePassword { error, busy, .. }) =
                            s.security.dialog.as_mut()
                        {
                            *busy = false;
                            *error = Some(match other {
                                Ok(_) => tr("Wrong current password").into(),
                                Err(e) => crate::trf!("Not changed: {}", e).into(),
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
                g.error = Some(tr("Wrong code").into());
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
                self.toast_for(crate::trf!("Recovery code used · {} left", left), 8, cx);
            }
        }
    }
}

impl CleisApp {}
