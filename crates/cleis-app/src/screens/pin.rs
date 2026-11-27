//! The PIN boxes of mockup 36h, used for the personal vault's PIN (37e-37h): one box per digit,
//! a dot for each digit typed, the next box outlined while it has the keyboard. Digits typed or
//! pasted, Backspace removes the last one, Enter (or the last digit) goes on.

use crate::app::CleisApp;
use crate::theme::*;
use gpui_kit::TestSupportExt;
use gpui_kit::prelude::*;
use gpui_kit::{App, Context, FocusHandle, KeyDownEvent, SharedString, Window, div, px};
use zeroize::Zeroizing;

pub struct PinBoxes {
    pub value: Zeroizing<String>,
    pub len: usize,
    pub focus: FocusHandle,
}

impl PinBoxes {
    pub fn new(len: usize, cx: &mut App) -> Self {
        Self {
            value: Zeroizing::new(String::new()),
            len,
            focus: cx.focus_handle(),
        }
    }

    pub fn full(&self) -> bool {
        self.value.len() == self.len
    }

    pub fn clear(&mut self) {
        self.value = Zeroizing::new(String::new());
    }

    /// Takes the digits of `text` until full; anything else is left out.
    fn push(&mut self, text: &str) {
        for c in text.chars().filter(char::is_ascii_digit) {
            if self.value.len() < self.len {
                self.value.push(c);
            }
        }
    }
}

/// Where the boxes live (the unlock screen, a dialog, the first launch).
pub type Get = fn(&mut CleisApp) -> Option<&mut PinBoxes>;
/// What happens once every box is filled, or on Enter.
pub type Done = fn(&mut CleisApp, &mut Window, &mut Context<CleisApp>);

pub fn boxes(
    id: &'static str,
    p: &PinBoxes,
    t: &Tokens,
    window: &Window,
    get: Get,
    done: Done,
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    let focused = p.focus.is_focused(window);
    let n = p.value.len();
    let focus = p.focus.clone();
    let mut row = div()
        .id(id)
        .track_focus(&p.focus)
        .flex()
        .gap(px(6.))
        .justify_center()
        .cursor_text()
        .on_click(move |_, window, cx| window.focus(&focus, cx))
        .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, window, cx| {
            let k = &ev.keystroke;
            let Some(p) = get(this) else { return };
            let cmd = if cfg!(target_os = "macos") {
                k.modifiers.platform
            } else {
                k.modifiers.control
            };
            let was_full = p.full();
            match k.key.as_str() {
                "backspace" => {
                    p.value.pop();
                }
                "enter" => {
                    cx.stop_propagation();
                    done(this, window, cx);
                    cx.notify();
                    return;
                }
                "v" if cmd => {
                    if let Some(text) = cx.read_from_clipboard().and_then(|c| c.text()) {
                        p.push(&text);
                    }
                }
                "escape" | "tab" => return,
                _ => {
                    if let Some(ch) = k.key_char.as_deref()
                        && !cmd
                    {
                        p.push(ch);
                    } else {
                        return;
                    }
                }
            }
            cx.stop_propagation();
            let now_full = p.full();
            if now_full && !was_full {
                done(this, window, cx);
            }
            cx.notify();
        }));
    for i in 0..p.len {
        let current = focused && (i == n || (n == p.len && i == p.len - 1));
        row = row.child(
            div()
                .w(px(40.))
                .h(px(44.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(RADIUS_MD)
                .bg(t.bg)
                .border_1()
                .border_color(t.line_strong)
                .when(current, |d| d.border_2().border_color(t.focus))
                .font_family(FONT_MONO)
                .text_size(px(20.))
                .when(i < n, |d| d.child("•")),
        );
    }
    row.test_support()
}

/// The PIN label under a title: digits count in a tooltip.
pub fn digits_tip(len: usize) -> SharedString {
    crate::trf!("{} digits", len).into()
}

/// Choosing a PIN for this computer (mockup 37f): typed twice, in the same boxes.
pub struct PinSetup {
    pub first: PinBoxes,
    pub confirm: PinBoxes,
    pub max_tries: u8,
    pub error: Option<SharedString>,
    pub busy: bool,
}

pub fn setup_dialog(
    s: &crate::app::Session,
    t: &Tokens,
    window: &Window,
    cx: &mut Context<CleisApp>,
) -> Option<gpui_kit::Div> {
    use crate::i18n::tr;
    use crate::ui::*;
    let d = s.pin_setup.as_ref()?;
    let get_first: Get = |app| app.session()?.pin_setup.as_mut().map(|p| &mut p.first);
    let get_confirm: Get = |app| app.session()?.pin_setup.as_mut().map(|p| &mut p.confirm);
    let first_done: Done = |app, window, cx| {
        if let Some(p) = app.session().and_then(|s| s.pin_setup.as_mut())
            && p.first.full()
        {
            window.focus(&p.confirm.focus, cx);
        }
        cx.notify();
    };
    let confirm_done: Done = |app, window, cx| app.save_pin(window, cx);
    let card = dialog_box(t)
        .child(
            div()
                .text_size(TEXT_LG)
                .line_height(px(20.))
                .font_weight(W_MEDIUM)
                .child(tr("New PIN")),
        )
        .child(boxes(
            "pin-new", &d.first, t, window, get_first, first_done, cx,
        ))
        .child(
            div()
                .id("pin-confirm-label")
                .flex()
                .justify_center()
                .text_color(t.text_muted)
                .child(tr("Confirm"))
                .tooltip(tip(tr("Typed a second time to confirm"))),
        )
        .child(boxes(
            "pin-confirm",
            &d.confirm,
            t,
            window,
            get_confirm,
            confirm_done,
            cx,
        ))
        .when_some(d.error.clone(), |c, e| {
            c.child(
                div()
                    .flex()
                    .justify_center()
                    .text_size(TEXT_SM)
                    .text_color(t.danger_text)
                    .child(e),
            )
        })
        .child(
            div()
                .pt(px(8.))
                .flex()
                .justify_end()
                .gap(px(8.))
                .child(
                    button("pin-cancel", t, BtnKind::Default, false)
                        .child(tr("Cancel"))
                        .on_click(cx.listener(|this, _, window, cx| {
                            if let Some(s) = this.session() {
                                s.pin_setup = None;
                                let f = s.main_focus.clone();
                                window.focus(&f, cx);
                            }
                            cx.notify();
                        }))
                        .test_support(),
                )
                .child(
                    button("pin-save", t, BtnKind::Primary, false)
                        .when(d.busy || !d.confirm.full(), |b| b.opacity(0.6))
                        .child(tr("Save"))
                        .on_click(cx.listener(|this, _, window, cx| this.save_pin(window, cx)))
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
            .child(div().id("pin-dialog").occlude().child(card).test_support()),
    )
}

impl CleisApp {
    /// Opens 37f for a PIN of `len` digits (Security › Opening Cleis: PIN, its length, Change).
    pub fn open_pin_setup(&mut self, len: usize, window: &mut Window, cx: &mut Context<Self>) {
        let path = self.path.clone();
        let tries = crate::opening::pin_info(&path)
            .map_or(crate::opening::DEFAULT_PIN_TRIES, |i| i.max_tries);
        let first = PinBoxes::new(len, cx);
        let confirm = PinBoxes::new(len, cx);
        window.focus(&first.focus, cx);
        if let Some(s) = self.session() {
            s.pin_setup = Some(PinSetup {
                first,
                confirm,
                max_tries: tries,
                error: None,
                busy: false,
            });
        }
        cx.notify();
    }

    /// Both PINs typed: the same one, its key made off the interface thread, then kept.
    pub fn save_pin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(d) = s.pin_setup.as_mut() else {
            return;
        };
        if d.busy || !d.first.full() || !d.confirm.full() {
            return;
        }
        if *d.first.value != *d.confirm.value {
            d.error = Some(crate::i18n::tr("The two PINs differ").into());
            d.confirm.clear();
            window.focus(&d.confirm.focus, cx);
            cx.notify();
            return;
        }
        d.busy = true;
        d.error = None;
        let (pin, len, tries) = (d.first.value.clone(), d.first.len, d.max_tries);
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let prepared = cx
                .background_executor()
                .spawn(async move { crate::opening::prepare_pin(&pin, len, tries) })
                .await;
            this.update_in(cx, |this, window, cx| {
                let path = this.path.clone();
                let Some(s) = this.session() else { return };
                match prepared.and_then(|p| crate::opening::finish_pin(&path, &s.vault, p)) {
                    Ok(()) => {
                        s.pin_setup = None;
                        s.opening = crate::opening::Mode::Pin;
                        let f = s.main_focus.clone();
                        window.focus(&f, cx);
                        this.toast(crate::i18n::tr("A PIN opens Cleis on this computer"), cx);
                    }
                    Err(e) => {
                        if let Some(d) = s.pin_setup.as_mut() {
                            d.busy = false;
                            d.error = Some(crate::i18n::tr_dynamic(&e).into());
                        }
                        cx.notify();
                    }
                }
            })
            .ok();
        })
        .detach();
    }
}
