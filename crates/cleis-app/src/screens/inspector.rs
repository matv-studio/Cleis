//! Vault inspector (screen 01): the selected item, edited in place. Details (fields,
//! subscription) and History (earlier passwords, dates).

use crate::app::{CleisApp, Session};
use crate::editor::{EntryEditor, Menu};
use crate::format::subscription_chip;
use crate::i18n::tr;
use crate::theme::*;
use crate::ui::*;
use cleis_core::column::ColumnKind;
use cleis_core::edit::{Field, SUBSCRIPTIONS};
use cleis_core::totp::Totp;
use cleis_core::{Entry, Subscription};
use gpui_kit::TestSupportExt;
use gpui_kit::component::Sizable;
use gpui_kit::component::input::{Input, Textarea};
use gpui_kit::prelude::*;
use gpui_kit::{AnyElement, Context, Div, Hsla, SharedString, div, px};

fn tip(
    text: &'static str,
) -> impl Fn(&mut gpui_kit::Window, &mut gpui_kit::App) -> gpui_kit::AnyView + 'static {
    move |window, cx| {
        gpui_kit::component::tooltip::Tooltip::new(crate::i18n::tr_dynamic(text)).build(window, cx)
    }
}

/// Strength bars under the password (charter: four bars).
pub fn strength(t: &Tokens, password: &str) -> Div {
    let bits = cleis_core::generator::entropy_bits(password);
    let level = if password.is_empty() {
        0
    } else if bits >= 90. {
        4
    } else if bits >= 70. {
        3
    } else if bits >= 50. {
        2
    } else {
        1
    };
    let color: Hsla = match level {
        4 | 3 => t.success_text,
        2 => t.warning_text,
        _ => t.danger_text,
    };
    let mut bars = div().flex().gap(px(3.));
    for i in 0..4 {
        bars = bars.child(div().flex_1().h(px(3.)).rounded(px(2.)).bg(if i < level {
            color
        } else {
            t.line_strong
        }));
    }
    bars
}

/// A field's box, with a red border and its message when the value was refused.
fn boxed(t: &Tokens, ed: &EntryEditor, f: Field, input: impl IntoElement) -> Div {
    let err = ed.errors.get(&f).cloned();
    // `min_w_0`: in a row the message wraps inside the panel instead of widening it.
    div()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(
            div()
                .rounded(RADIUS_MD)
                .when(err.is_some(), |d| d.border_1().border_color(t.danger_text))
                .child(input),
        )
        .when_some(err, |d, e| {
            d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e))
        })
}

pub fn render(s: &Session, e: &Entry, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let Some(ed) = s.editor.as_ref().filter(|ed| ed.entry == e.id) else {
        return div().w(INSPECTOR_W).flex_none().h_full().bg(t.surface);
    };
    let id = e.id;
    let now = cleis_core::now();
    let space = s
        .spaces
        .iter()
        .find(|sp| sp.id == e.space)
        .map(|sp| crate::i18n::space(&sp.name))
        .unwrap_or_default();
    let copy = |key: &str, value: String, what: &'static str, cx: &mut Context<CleisApp>| {
        icon_button(SharedString::from(format!("{key}-{id}")), t, "copy", true)
            .tooltip(tip(tr("Copy")))
            .on_click(cx.listener(move |this, _, _, cx| this.copy_of(id, value.clone(), what, cx)))
    };
    let input = |f: Field| ed.inputs.get(&f).cloned();

    // Header: icon, editable name, space, favorite, menu.
    let name_box = input(Field::Name).map(|st| {
        Input::new(&st)
            .id("edit-name")
            .appearance(false)
            .text_size(TEXT_LG)
            .font_weight(W_MEDIUM)
    });
    let more_open = ed.menu == Some(Menu::More);
    let more_menu = menu_box(t)
        .top(px(30.))
        .right_0()
        .child(
            menu_item("menu-duplicate", t, Some("copy"), tr("Duplicate"), false)
                .on_click(cx.listener(move |this, _, _, cx| this.duplicate_entry(id, cx)))
                .test_support(),
        )
        .children(
            s.spaces
                .iter()
                .filter(|sp| sp.id != e.space && !sp.for_contacts)
                // Audit L-2: an organization's item does not leave the organization.
                .filter(|sp| {
                    let key = |id| s.org_of(id).map(|o| o.m.key());
                    !s.is_org_space(e.space) || key(sp.id) == key(e.space)
                })
                .map(|sp| {
                    let to = sp.id;
                    menu_item(
                        format!("menu-move-{}", sp.id),
                        t,
                        Some("folder"),
                        crate::trf!("Move to {}", sp.name),
                        false,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| this.move_entry(id, to, cx)))
                    .test_support()
                }),
        )
        .child(div().h(px(1.)).my(px(4.)).bg(t.line))
        .child(
            menu_item("menu-trash", t, Some("trash"), tr("Move to trash"), true)
                .on_click(cx.listener(move |this, _, _, cx| this.trash_entry(id, cx)))
                .test_support(),
        );
    let head = div()
        .p(px(12.))
        .pb(px(8.))
        .flex()
        .items_center()
        .gap(px(8.))
        .child(site_icon(t, e.name.chars().next().unwrap_or('?'), 32.))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .when_some(name_box, |d, b| d.child(boxed(t, ed, Field::Name, b)))
                .child(
                    div()
                        .text_size(TEXT_SM)
                        .text_color(t.text_muted)
                        .child(space),
                ),
        )
        .child(
            icon_button("fav", t, "star", false)
                .tooltip(tip(tr("Favorite")))
                .when(e.favorite, |d| d.text_color(t.warning_text))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.update_entry(id, |e| e.favorite = !e.favorite, cx)
                }))
                .test_support(),
        )
        .child(
            div()
                .relative()
                .child(
                    icon_button("more", t, "more", false)
                        .tooltip(tip(tr("More")))
                        .on_click(cx.listener(|this, _, _, cx| this.toggle_menu(Menu::More, cx)))
                        .test_support(),
                )
                .when(more_open, |d| d.child(gpui_kit::deferred(more_menu))),
        )
        .child(
            icon_button("close-inspector", t, "x", false)
                .tooltip(tip(tr("Close")))
                .on_click(cx.listener(|this, _, _, cx| this.deselect(cx)))
                .test_support(),
        );

    let tab = |label: &'static str, on: bool, cx: &mut Context<CleisApp>| {
        div()
            .id(SharedString::from(format!("insp-{label}")))
            .h(px(32.))
            .px(px(8.))
            .flex()
            .items_center()
            .font_weight(W_MEDIUM)
            .cursor_pointer()
            .when(on, |d| {
                d.border_b_2()
                    .border_color(t.accent_text)
                    .text_color(t.accent_text)
            })
            .when(!on, |d| d.text_color(t.text_muted))
            .child(crate::i18n::tr_owned(label))
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(s) = this.session() {
                    s.history_tab = label == "History";
                }
                cx.notify();
            }))
            .test_support()
    };
    let tabs = div()
        .flex()
        .gap(px(2.))
        .px(px(12.))
        .border_b_1()
        .border_color(t.line)
        .child(tab("Details", !s.history_tab, cx))
        .child(tab("History", s.history_tab, cx));

    let body = if s.history_tab {
        history(s, e, t, cx).into_any_element()
    } else {
        details(s, ed, e, t, now, &copy, cx).into_any_element()
    };

    div()
        .w(INSPECTOR_W)
        .flex_none()
        .h_full()
        .bg(t.surface)
        .border_l_1()
        .border_color(t.line)
        .flex()
        .flex_col()
        .child(head)
        .child(tabs)
        .child(
            div()
                .id("inspector-body")
                .flex_1()
                .overflow_y_scroll()
                .p(px(12.))
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(body),
        )
}

type CopyBtn<'a> =
    dyn Fn(&str, String, &'static str, &mut Context<CleisApp>) -> gpui_kit::Stateful<Div> + 'a;

fn details(
    s: &Session,
    ed: &EntryEditor,
    e: &Entry,
    t: &Tokens,
    now: i64,
    copy: &CopyBtn,
    cx: &mut Context<CleisApp>,
) -> Div {
    let id = e.id;
    let input = |f: Field| ed.inputs.get(&f).cloned();
    let mut d = div().flex().flex_col().gap(px(12.));

    if let Some(st) = input(Field::Website) {
        let url = e.urls.first().map(|w| {
            if w.contains("://") {
                w.clone()
            } else {
                format!("https://{w}")
            }
        });
        let inp = Input::new(&st).id("edit-website").when_some(url, |i, url| {
            i.suffix(
                icon_button("open", t, "external", true)
                    .tooltip(tip(tr("Open website")))
                    .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url))),
            )
        });
        d = d.child(field(t, tr("Website"), boxed(t, ed, Field::Website, inp)));
    }
    if let Some(st) = input(Field::Username) {
        let inp = Input::new(&st).id("edit-username").suffix(copy(
            "iu",
            e.username.clone(),
            "Username",
            cx,
        ));
        d = d.child(field(t, tr("Username"), boxed(t, ed, Field::Username, inp)));
    }
    if let Some(st) = input(Field::Password) {
        let actions = div()
            .flex()
            .child(
                icon_button("reveal", t, "eye", true)
                    .tooltip(tip(tr("Show")))
                    .on_click(cx.listener(|this, _, window, cx| {
                        let r = this.session().map(|s| !s.reveal).unwrap_or(false);
                        this.set_reveal(r, window, cx);
                    }))
                    .test_support(),
            )
            .child(copy("ip", e.password.clone(), "Password", cx))
            .child(
                icon_button("gen", t, "refresh", true)
                    .tooltip(tip(tr("Generate a password")))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.generate_password(id, window, cx)
                    }))
                    .test_support(),
            );
        let inp = Input::new(&st).id("edit-password").suffix(actions);
        d = d.child(
            field(t, tr("Password"), boxed(t, ed, Field::Password, inp))
                .child(strength(t, &e.password)),
        );
    }

    // 2FA: the current code, or the box for its key.
    let totp_key = Input::new(&ed.totp).id("edit-totp");
    d = d.child(match (&e.totp, ed.editing_totp) {
        (Some(totp), false) => {
            let code = totp.code_at(now);
            field(
                t,
                "2FA code",
                value_box(t)
                    .h(px(36.))
                    .child(div().flex_1().child(totp_view(t, totp, now)))
                    .child(copy("it", code, "Code", cx))
                    .child(
                        icon_button("edit-totp-key", t, "key", true)
                            .tooltip(tip(tr("Change the 2FA key")))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(s) = this.session()
                                    && let Some(ed) = s.editor.as_mut()
                                {
                                    ed.editing_totp = true;
                                }
                                cx.notify();
                            }))
                            .test_support(),
                    ),
            )
        }
        _ => field(t, tr("2FA key"), boxed(t, ed, Field::Totp, totp_key)),
    });

    d = d.child(field(t, tr("Notes"), Textarea::new(&ed.notes)));

    // Subscription: state, price, licenses, yearly cost, renewal.
    let yearly = cleis_core::column::default_vault_columns(&s.prefs.currency)
        .iter()
        .find(|c| c.id.0 == "yearly_cost")
        .and_then(|c| match &c.kind {
            ColumnKind::Formula { expr } => e.compute(expr, &s.columns).ok().flatten(),
            _ => None,
        });
    let sub_open = ed.menu == Some(Menu::Subscription);
    let chip_el = subscription_chip(t, e.subscription)
        .unwrap_or_else(|| chip(t, Tone::Neutral, tr("None"), false));
    let sub_menu = menu_box(t)
        .top(px(26.))
        .right_0()
        .children(SUBSCRIPTIONS.iter().map(|st| {
            let st = *st;
            let label = match st {
                Subscription::None => "None",
                Subscription::Active => "Active",
                Subscription::Free => "Free",
                Subscription::RenewSoon => "Renew soon",
                Subscription::Pending => "Pending",
                Subscription::Off => "Off",
            };
            menu_item(format!("sub-{label}"), t, None, label, false)
                .when(st == e.subscription, |d| d.text_color(t.accent_text))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.update_entry(id, |e| e.subscription = st, cx);
                    this.toggle_menu(Menu::Subscription, cx);
                }))
                .test_support()
        }));
    let kv = |label: &str, f: Field, cx: &mut Context<CleisApp>| -> AnyElement {
        let _ = cx;
        match input(f) {
            // Label level with the box, even when a message opens below it.
            Some(st) => div()
                .flex()
                .items_start()
                .gap(px(8.))
                .text_size(TEXT_SM)
                .child(
                    div()
                        .w(px(96.))
                        .h(CONTROL_SM)
                        .flex()
                        .items_center()
                        .text_color(t.text_muted)
                        .child(crate::i18n::tr_owned(label)),
                )
                .child({
                    let target = super::calendar::CalTarget::Field(f);
                    let date = f == Field::Renewal;
                    let open = s.calendar.as_ref().filter(|c| date && c.target == target);
                    let mut inp = Input::new(&st)
                        .id(SharedString::from(format!("edit-{label}")))
                        .small();
                    if date {
                        inp = inp.suffix(super::calendar::open_button(
                            "cal-open-renewal",
                            target,
                            t,
                            cx,
                        ));
                    }
                    div()
                        .flex_1()
                        .min_w_0()
                        .relative()
                        .child(boxed(t, ed, f, inp))
                        .when_some(open, |d, c| {
                            d.child(super::calendar::popup(
                                super::calendar::render(s, c, t, cx),
                                CONTROL_SM,
                            ))
                        })
                })
                .into_any_element(),
            None => div().into_any_element(),
        }
    };
    let sub = div()
        .p(px(12.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .bg(t.surface_raised)
        .border_1()
        .border_color(t.line)
        .rounded(RADIUS_LG)
        .child(
            div()
                .flex()
                .items_center()
                .child(
                    div()
                        .flex_1()
                        .text_size(TEXT_SM)
                        .font_weight(W_MEDIUM)
                        .text_color(t.text_muted)
                        .child(tr("Subscription")),
                )
                .child(
                    div()
                        .relative()
                        .child(
                            div()
                                .id("sub-state")
                                .cursor_pointer()
                                .child(chip_el)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.toggle_menu(Menu::Subscription, cx)
                                }))
                                .test_support(),
                        )
                        .when(sub_open, |d| d.child(gpui_kit::deferred(sub_menu))),
                ),
        )
        .child(kv("Price", Field::Price, cx))
        .child(kv("Licenses", Field::Licenses, cx))
        .when_some(yearly, |d, y| {
            d.child(
                div()
                    .flex()
                    .text_size(TEXT_SM)
                    .child(
                        div()
                            .w(px(96.))
                            .text_color(t.text_muted)
                            .child(tr("Yearly cost")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_right()
                            .font_weight(W_MEDIUM)
                            .child(crate::format::num(&y)),
                    ),
            )
        })
        .child(kv("Renewal", Field::Renewal, cx));
    for (c, st) in &ed.custom {
        let err = ed.custom_errors.get(&c.id).cloned();
        let date = matches!(c.kind, cleis_core::column::ColumnKind::Date);
        let target = super::calendar::CalTarget::Custom(c.id.clone());
        let open = s.calendar.as_ref().filter(|x| date && x.target == target);
        let mut inp = Input::new(st).id(SharedString::from(format!("custom-{}", c.id.0)));
        if date {
            inp = inp.suffix(super::calendar::open_button(
                format!("cal-open-{}", c.id.0),
                target,
                t,
                cx,
            ));
        }
        let cal =
            open.map(|x| super::calendar::popup(super::calendar::render(s, x, t, cx), CONTROL_MD));
        d = d.child(field(
            t,
            &c.name,
            div()
                .relative()
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(
                    div()
                        .rounded(RADIUS_MD)
                        .when(err.is_some(), |d| d.border_1().border_color(t.danger_text))
                        .child(inp),
                )
                .children(cal)
                .when_some(err, |d, e| {
                    d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e))
                }),
        ));
    }
    d = d.child(sub);
    d.child(
        div()
            .text_size(TEXT_SM)
            .text_color(t.text_faint)
            .child(crate::trf!("Edited {}", crate::format::date(e.updated))),
    )
}

fn totp_view(t: &Tokens, totp: &Totp, now: i64) -> Div {
    let code = Totp::display(&totp.code_at(now));
    let frac = totp.remaining(now) as f32 / totp.period.max(1) as f32;
    div()
        .flex()
        .items_center()
        .gap(px(6.))
        .font_family(FONT_MONO)
        .text_size(TEXT_LG)
        .child(totp_ring(t, frac))
        .child(code)
}

/// Earlier passwords, newest first, with the dates of the item.
fn history(s: &Session, e: &Entry, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let mut d = div().flex().flex_col().gap(px(8.));
    if e.password_history.is_empty() {
        d = d.child(
            div()
                .text_color(t.text_muted)
                .child(tr("No earlier password")),
        );
    }
    let id = e.id;
    for (i, h) in e.password_history.iter().rev().enumerate() {
        let pw = h.password.clone();
        d = d.child(
            value_box(t)
                .child(
                    div().flex_1().flex().flex_col().min_w_0().child(
                        div()
                            .font_family(FONT_MONO)
                            .overflow_hidden()
                            .text_ellipsis()
                            .when(!s.reveal, |d| d.text_color(t.text_muted))
                            .child(if s.reveal {
                                h.password.clone()
                            } else {
                                "••••••••••".into()
                            }),
                    ),
                )
                .child(
                    div()
                        .text_size(TEXT_SM)
                        .text_color(t.text_muted)
                        .child(crate::format::date(h.replaced_at)),
                )
                .child(
                    icon_button(
                        SharedString::from(format!("hist-copy-{i}")),
                        t,
                        "copy",
                        true,
                    )
                    .tooltip(tip(tr("Copy")))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.copy_of(id, pw.clone(), "Password", cx)
                    })),
                ),
        );
    }
    d.child(
        div()
            .pt(px(8.))
            .flex()
            .flex_col()
            .gap(px(4.))
            .text_size(TEXT_SM)
            .text_color(t.text_muted)
            .child(crate::trf!("Created {}", crate::format::date(e.created)))
            .child(crate::trf!(
                "Password changed {}",
                crate::format::date(e.password_changed)
            ))
            .child(crate::trf!("Edited {}", crate::format::date(e.updated))),
    )
}

impl CleisApp {
    /// New generated password for an item; the old one goes to its history.
    pub fn generate_password(
        &mut self,
        id: cleis_core::EntryId,
        window: &mut gpui_kit::Window,
        cx: &mut Context<CleisApp>,
    ) {
        if let Ok(p) = cleis_core::generator::generate(&Default::default()) {
            self.update_entry(id, |e| e.set_password(p, None), cx);
            self.refresh_field(Field::Password, window, cx);
        }
    }
}
