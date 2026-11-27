//! Settings › Shortcuts (mockups 38, 38b, 27/11/2026): every keyboard shortcut in one place.
//! The system-wide ones (Quick Access, Fill best match) move here from Autofill; those inside
//! Cleis (lock, new, search, the selected item's) can be changed too. Each one is changed by
//! pressing the new keys; Esc cancels, Backspace leaves it without keys. Keys used twice are
//! marked "In use" on both rows; the shortcut that comes first in the list is the one that runs.

use crate::app::{CleisApp, Session};
use crate::autofill::AutofillState;
use crate::i18n::tr;
use crate::theme::*;
use crate::ui::*;
use cleis_autofill::hotkeys::{Shortcut, Status};
use cleis_core::settings::{APP_KEYS, Preferences, parse_keys};
use gpui_kit::TestSupportExt;
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, KeyDownEvent, Keystroke, SharedString, div, px};

/// The key pressed matches the shortcut written `spec` (`Ctrl` is ⌘ on macOS).
pub fn pressed(k: &Keystroke, spec: &str) -> bool {
    let Some(c) = parse_keys(spec) else {
        return false;
    };
    let m = &k.modifiers;
    let cmd = if cfg!(target_os = "macos") {
        m.platform
    } else {
        m.control
    };
    if cmd != c.ctrl || m.alt != c.alt || m.shift != c.shift {
        return false;
    }
    let key = k.key.as_str();
    match c.key.to_ascii_lowercase().as_str() {
        "del" | "delete" => key == "delete" || (cfg!(target_os = "macos") && key == "backspace"),
        "space" => key == "space",
        other => key.eq_ignore_ascii_case(other),
    }
}

/// The keys of a shortcut inside Cleis, from a key press: a modifier, or Del / F1-F24 alone.
fn app_keys_from(ev: &KeyDownEvent) -> Option<String> {
    let k = &ev.keystroke;
    let m = &k.modifiers;
    let key = match k.key.as_str() {
        "delete" => "Del".to_owned(),
        "space" => "Space".to_owned(),
        "shift" | "control" | "alt" | "platform" | "function" | "escape" | "backspace" => {
            return None;
        }
        s if s.chars().count() == 1 => s.to_uppercase(),
        s if s.starts_with('f') && s[1..].parse::<u8>().is_ok() => s.to_uppercase(),
        _ => return None,
    };
    let mut parts = vec![];
    if m.control || (cfg!(target_os = "macos") && m.platform) {
        parts.push("Ctrl");
    }
    if m.alt {
        parts.push("Alt");
    }
    if m.shift {
        parts.push("Shift");
    }
    let bare_ok = key == "Del" || (key.starts_with('F') && key.len() > 1);
    if parts.is_empty() && !bare_ok || parts == ["Shift"] && !bare_ok {
        return None;
    }
    parts.push(&key);
    Some(parts.join("+"))
}

/// The label of a shortcut's id (for "In use" tooltips).
fn label_of(id: &str) -> &'static str {
    match id {
        "qa" => "Quick Access",
        "best" => "Fill best match",
        _ => APP_KEYS
            .iter()
            .find(|(k, _, _)| *k == id)
            .map(|(_, l, _)| *l)
            .unwrap_or(""),
    }
}

fn default_of(id: &str) -> String {
    let d = Preferences::default();
    match id {
        "qa" => d.autofill.quick_access.keys,
        "best" => d.autofill.fill_best_match.keys,
        _ => d.app_key(id),
    }
}

fn keys_of(p: &Preferences, id: &str) -> String {
    match id {
        "qa" => p.autofill.quick_access.keys.clone(),
        "best" => p.autofill.fill_best_match.keys.clone(),
        _ => p.app_key(id),
    }
}

fn set_keys(p: &mut Preferences, id: &str, keys: String) {
    match id {
        "qa" => p.autofill.quick_access.keys = keys,
        "best" => p.autofill.fill_best_match.keys = keys,
        _ => {
            if keys == p.app_key(id) {
                return;
            }
            let default = Preferences::default().app_key(id);
            if keys == default {
                p.app_keys.remove(id);
            } else {
                p.app_keys.insert(id.to_owned(), keys);
            }
        }
    }
}

/// `Ctrl` reads as ⌘ on macOS.
fn caps(keys: &str) -> Vec<String> {
    keys.split('+')
        .map(|k| {
            let k = k.trim();
            if cfg!(target_os = "macos") {
                match k.to_ascii_lowercase().as_str() {
                    "ctrl" => return "⌘".into(),
                    "alt" => return "⌥".into(),
                    "shift" => return "⇧".into(),
                    _ => {}
                }
            }
            k.to_owned()
        })
        .collect()
}

/// A system-wide shortcut's switch: on, and what turns it over.
type Switch = (bool, fn(&mut Preferences));

struct Row {
    id: &'static str,
    label: &'static str,
    tip: Option<&'static str>,
    first: bool,
    /// System-wide: the switch and the system's answer.
    switch: Option<Switch>,
    status: Option<Status>,
}

fn shortcut_row(s: &Session, r: Row, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let p = &s.prefs;
    let id = r.id;
    let recording = s.recording == Some(id);
    let keys_now = keys_of(p, id);
    let changed = keys_now != default_of(id);
    let clash = p.clash(id);
    let name = div()
        .id(SharedString::from(format!("{id}-label")))
        .flex_1()
        .child(tr(r.label))
        .when_some(r.tip, |d, tp| d.tooltip(tip(tr(tp))));
    let unavailable = match (&r.status, r.switch) {
        (Some(Status::Unavailable(why)), Some((true, _))) => Some(why.clone()),
        _ => None,
    };
    row(t, r.first)
        .child(name)
        .when_some(unavailable, |d, why| {
            d.child(
                div()
                    .id(SharedString::from(format!("{id}-status")))
                    .child(chip(t, Tone::Warn, tr("Unavailable"), true))
                    .tooltip(crate::ui::tip(why))
                    .test_support(),
            )
        })
        .when_some(clash, |d, other| {
            d.child(
                div()
                    .id(SharedString::from(format!("{id}-clash")))
                    .child(chip(t, Tone::Warn, tr("In use"), true))
                    .tooltip(crate::ui::tip(crate::trf!("Also {}", tr(label_of(&other)))))
                    .test_support(),
            )
        })
        .child(if recording {
            div()
                .w(px(150.))
                .h(CONTROL_SM)
                .flex()
                .items_center()
                .justify_center()
                .rounded(RADIUS_MD)
                .bg(t.bg)
                .border_2()
                .border_color(t.focus)
                .text_size(TEXT_SM)
                .text_color(t.text_muted)
                .child(tr("Press the keys…"))
                .into_any_element()
        } else if keys_now.is_empty() {
            div()
                .text_size(TEXT_SM)
                .text_color(t.text_faint)
                .child(tr("None"))
                .into_any_element()
        } else {
            let c = caps(&keys_now);
            let r: Vec<&str> = c.iter().map(String::as_str).collect();
            keys(t, &r).into_any_element()
        })
        // The reset, or its room: the buttons stay in one column.
        .child(
            div()
                .w(CONTROL_SM)
                .flex_none()
                .when(changed && !recording, |d| {
                    d.child(
                        icon_button(
                            SharedString::from(format!("{id}-reset")),
                            t,
                            "refresh",
                            true,
                        )
                        .tooltip(crate::ui::tip(crate::trf!("Back to {}", default_of(id))))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let d = default_of(id);
                            this.update_prefs(move |p| set_keys(p, id, d), cx);
                        }))
                        .test_support(),
                    )
                }),
        )
        .child(
            button(
                SharedString::from(format!("{id}-change")),
                t,
                BtnKind::Ghost,
                true,
            )
            .w(px(72.))
            .justify_center()
            .child(if recording {
                tr("Cancel")
            } else {
                tr("Change")
            })
            .when(recording, |b| {
                b.tooltip(tip(tr("Esc: cancel · Backspace: none")))
            })
            .on_click(cx.listener(move |this, _, window, cx| {
                if let Some(s) = this.session() {
                    s.recording = if s.recording == Some(id) {
                        None
                    } else {
                        Some(id)
                    };
                    if s.recording.is_some() {
                        let f = s.keys_focus.clone();
                        window.focus(&f, cx);
                    }
                }
                cx.notify();
            }))
            .test_support(),
        )
        .child(match r.switch {
            Some((on, flip)) => switch(if id == "qa" { "af-qa" } else { "af-best" }, t, on)
                .on_click(cx.listener(move |this, _, _, cx| this.update_prefs(flip, cx)))
                .test_support()
                .into_any_element(),
            None => div().w(px(26.)).flex_none().into_any_element(),
        })
}

pub fn page(s: &Session, af: &AutofillState, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let p = &s.prefs;
    let a = &p.autofill;
    let status = |which: Shortcut| {
        af.status
            .as_ref()
            .and_then(|st| st.iter().find(|(w, _)| *w == which).map(|(_, s)| s.clone()))
    };
    let head = div()
        .flex()
        .items_center()
        .child(
            div()
                .flex_1()
                .text_size(TEXT_XL)
                .line_height(px(26.))
                .font_weight(W_STRONG)
                .child(tr("Shortcuts")),
        )
        .child(
            button("keys-reset-all", t, BtnKind::Ghost, true)
                .child(tr("Reset all"))
                .tooltip(tip(tr("Every shortcut back to Cleis's")))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.update_prefs(
                        |p| {
                            let d = Preferences::default();
                            p.app_keys.clear();
                            p.autofill.quick_access.keys = d.autofill.quick_access.keys;
                            p.autofill.fill_best_match.keys = d.autofill.fill_best_match.keys;
                        },
                        cx,
                    )
                }))
                .test_support(),
        );
    let everywhere = section(t)
        .when(!a.enabled, |d| d.opacity(0.45))
        .child(
            div()
                .id("keys-everywhere")
                .child(section_title(t, tr("Everywhere")))
                .tooltip(tip(tr(
                    "Work in any app, even when Cleis is in the background",
                ))),
        )
        .child(shortcut_row(
            s,
            Row {
                id: "qa",
                label: "Quick Access",
                tip: None,
                first: true,
                switch: Some((a.quick_access.enabled, |p| {
                    p.autofill.quick_access.enabled = !p.autofill.quick_access.enabled
                })),
                status: status(Shortcut::QuickAccess),
            },
            t,
            cx,
        ))
        .child(shortcut_row(
            s,
            Row {
                id: "best",
                label: "Fill best match",
                tip: Some("Fills the item linked to the window in front"),
                first: false,
                switch: Some((a.fill_best_match.enabled, |p| {
                    p.autofill.fill_best_match.enabled = !p.autofill.fill_best_match.enabled
                })),
                status: status(Shortcut::FillBestMatch),
            },
            t,
            cx,
        ));
    let group = |title: &'static str, ids: &[&'static str], cx: &mut Context<CleisApp>| {
        let mut sec = section(t).child(section_title(t, tr(title)));
        for (i, id) in ids.iter().enumerate() {
            let (_, label, _) = APP_KEYS
                .iter()
                .find(|(k, _, _)| k == id)
                .copied()
                .unwrap_or_default();
            let tip = (*id == "new").then_some("New contact in Contacts");
            sec = sec.child(shortcut_row(
                s,
                Row {
                    id,
                    label,
                    tip,
                    first: i == 0,
                    switch: None,
                    status: None,
                },
                t,
                cx,
            ));
        }
        sec
    };
    let in_cleis = group("In Cleis", &["lock", "new", "search"], cx);
    let item = group(
        "Selected item",
        &["copy_user", "copy_pass", "copy_totp", "duplicate", "trash"],
        cx,
    );
    let recording = s.recording;
    div()
        .track_focus(&s.keys_focus)
        .flex()
        .flex_col()
        .gap(px(16.))
        .when(recording.is_some(), |d| {
            d.capture_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                let Some(s) = this.session() else { return };
                let Some(which) = s.recording else { return };
                cx.stop_propagation();
                match ev.keystroke.key.as_str() {
                    "escape" => {
                        s.recording = None;
                        cx.notify();
                        return;
                    }
                    // No keys for it (not for the system-wide ones: switched off instead).
                    "backspace" if which != "qa" && which != "best" => {
                        s.recording = None;
                        this.update_prefs(move |p| set_keys(p, which, String::new()), cx);
                        return;
                    }
                    _ => {}
                }
                let keys = if which == "qa" || which == "best" {
                    super::settings::shortcut_from(ev)
                } else {
                    app_keys_from(ev)
                };
                let Some(keys) = keys else { return };
                s.recording = None;
                this.update_prefs(move |p| set_keys(p, which, keys), cx);
            }))
        })
        .child(head)
        .child(everywhere)
        .child(in_cleis)
        .child(item)
}
