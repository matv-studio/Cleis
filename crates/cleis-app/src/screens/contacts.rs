//! Screen 20 — Contacts (read view; editing comes with the inspector forms).

use crate::app::{CleisApp, Session};
use crate::theme::*;
use crate::ui::*;
use cleis_core::Contact;
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, SharedString, div, px};

fn label_tone(l: &str) -> Tone {
    match l {
        "Client" => Tone::Set,
        "VIP" => Tone::Warn,
        "Prospect" => Tone::Auto,
        "Team" => Tone::Ok,
        _ => Tone::Neutral,
    }
}

pub fn page(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let mut list: Vec<&Contact> = s.contacts.iter().collect();
    list.sort_by_key(|c| cleis_core::text::fold(&c.display_name()));
    let sel = s.selected_contact.or_else(|| list.first().map(|c| c.id));
    let w = [210., 150., 0., 150., 160.];
    let cell = |i: usize| {
        let d = div()
            .h_full()
            .px(px(12.))
            .flex()
            .items_center()
            .overflow_hidden()
            .whitespace_nowrap()
            .min_w_0();
        if w[i] > 0. {
            d.w(px(w[i])).flex_none()
        } else {
            d.flex_1()
        }
    };
    let mut head = div()
        .w_full()
        .h(px(30.))
        .flex_none()
        .flex()
        .border_b_1()
        .border_color(t.line)
        .text_size(TEXT_XS)
        .font_weight(W_MEDIUM)
        .text_color(t.text_muted);
    for (i, h) in ["Name ↑", "Company", "Email", "Phone", "Labels"]
        .iter()
        .enumerate()
    {
        head = head.child(
            cell(i)
                .when(i == 0, |d| d.text_color(t.text))
                .child((*h).to_owned()),
        );
    }
    let mut rows = div()
        .id("contacts")
        .flex_1()
        .overflow_y_scroll()
        .flex()
        .flex_col();
    for c in &list {
        let id = c.id;
        let selected = sel == Some(id);
        let mut labels = div().flex().gap(px(4.));
        for l in &c.labels {
            labels = labels.child(chip(t, label_tone(l), l.clone(), false));
        }
        rows = rows.child(
            div()
                .id(SharedString::from(format!("c-{id}")))
                .w_full()
                .h(ROW_H)
                .flex_none()
                .flex()
                .border_b_1()
                .border_color(t.line)
                .cursor_pointer()
                .when(selected, |d| d.bg(t.selection))
                .when(!selected, |d| d.hover(|s| s.bg(t.surface_hover)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(s) = this.session() {
                        s.selected_contact = Some(id);
                    }
                    cx.notify();
                }))
                .child(
                    cell(0)
                        .gap(px(8.))
                        .child(avatar(t, &c.initials(), 22.))
                        .child(
                            div()
                                .font_weight(W_MEDIUM)
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(c.display_name()),
                        )
                        .when(c.favorite, |d| d.child(icon("star", 12., t.warning_text))),
                )
                .child(cell(1).child(if c.company.is_empty() {
                    "—".to_owned()
                } else {
                    c.company.clone()
                }))
                .child(
                    cell(2)
                        .text_color(t.text_muted)
                        .child(c.primary_email().unwrap_or_default()),
                )
                .child(
                    cell(3)
                        .text_color(t.text_muted)
                        .child(c.primary_phone().unwrap_or_default()),
                )
                .child(cell(4).child(labels)),
        );
    }
    let selected = sel.and_then(|id| s.contacts.iter().find(|c| c.id == id));
    let folder_count = |for_folder: cleis_core::SpaceId| {
        s.contacts.iter().filter(|c| c.space == for_folder).count()
    };
    let mut side = div()
        .w(SIDEBAR_W)
        .flex_none()
        .h_full()
        .bg(t.surface)
        .border_r_1()
        .border_color(t.line)
        .p(px(8.))
        .flex()
        .flex_col()
        .gap(px(2.));
    let item = |ico: &str, label: String, n: usize, active: bool| {
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .h(CONTROL_MD)
            .px(px(8.))
            .rounded(RADIUS_MD)
            .text_color(if active { t.text } else { t.text_muted })
            .when(active, |d| d.bg(t.surface_active))
            .child(icon(ico, 16., if active { t.text } else { t.text_muted }))
            .child(div().flex_1().child(label))
            .child(
                div()
                    .text_size(TEXT_SM)
                    .text_color(t.text_faint)
                    .child(n.to_string()),
            )
    };
    side = side
        .child(item("users", "All contacts".into(), s.contacts.len(), true))
        .child(item(
            "star",
            "Favorites".into(),
            s.contacts.iter().filter(|c| c.favorite).count(),
            false,
        ));
    for sp in s.spaces.iter().filter(|sp| sp.for_contacts) {
        side = side.child(item("user", sp.name.clone(), folder_count(sp.id), false));
    }
    let dups = cleis_core::dedupe::find_duplicates(&s.contacts, &Default::default()).len();
    side = side.child(div().flex_1()).when(dups > 0, |d| {
        d.child(item("users", "Merge & fix".into(), dups, false))
    });
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .child(side)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(head)
                        .child(rows),
                )
                .when_some(selected, |d, c| d.child(inspector(c, t, cx))),
        )
        .child(
            div()
                .h(STATUSBAR_H)
                .flex_none()
                .flex()
                .items_center()
                .px(px(12.))
                .border_t_1()
                .border_color(t.line)
                .bg(t.surface)
                .text_size(TEXT_SM)
                .text_color(t.text_muted)
                .child(format!("{} contacts", s.contacts.len())),
        )
}

fn inspector(c: &Contact, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let line = |kind: &str, value: String, key: String, cx: &mut Context<CleisApp>| {
        let v = value.clone();
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .min_h(px(28.))
            .child(
                div()
                    .w(px(60.))
                    .flex_none()
                    .text_size(TEXT_SM)
                    .text_color(t.text_muted)
                    .child(kind.to_owned()),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(value),
            )
            .child(
                icon_button(SharedString::from(key), t, "copy", true).on_click(
                    cx.listener(move |this, _, _, cx| this.copy(v.clone(), "Copied", cx)),
                ),
            )
    };
    let mut emails = div().flex().flex_col();
    for (i, e) in c.emails.iter().enumerate() {
        emails = emails.child(line(&e.label, e.value.clone(), format!("ce{i}"), cx));
    }
    let mut phones = div().flex().flex_col();
    for (i, p) in c.phones.iter().enumerate() {
        phones = phones.child(line(&p.label, p.value.clone(), format!("cp{i}"), cx));
    }
    let sub = [c.job_title.as_str(), c.company.as_str()]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" · ");
    div()
        .w(INSPECTOR_W)
        .flex_none()
        .h_full()
        .bg(t.surface)
        .border_l_1()
        .border_color(t.line)
        .flex()
        .flex_col()
        .child(
            div()
                .p(px(12.))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(avatar(t, &c.initials(), 40.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(TEXT_LG)
                                .font_weight(W_MEDIUM)
                                .child(c.display_name()),
                        )
                        .child(div().text_size(TEXT_SM).text_color(t.text_muted).child(sub)),
                ),
        )
        .child(
            div()
                .p(px(12.))
                .pt_0()
                .flex()
                .flex_col()
                .gap(px(12.))
                .when(!c.emails.is_empty(), |d| d.child(field(t, "Email", emails)))
                .when(!c.phones.is_empty(), |d| d.child(field(t, "Phone", phones)))
                .when_some(c.city(), |d, city| {
                    d.child(field(t, "City", value_box(t).child(city)))
                })
                .when_some(c.birthday, |d, b| {
                    d.child(field(
                        t,
                        "Birthday",
                        value_box(t).child(format!("{:02}/{:02}", b.day, b.month)),
                    ))
                })
                .when(!c.notes.is_empty(), |d| {
                    d.child(field(
                        t,
                        "Notes",
                        value_box(t)
                            .h_auto()
                            .min_h(px(44.))
                            .py(px(6.))
                            .items_start()
                            .text_color(t.text_muted)
                            .child(c.notes.clone()),
                    ))
                }),
        )
}

impl CleisApp {
    pub fn new_contact(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let space = s
            .spaces
            .iter()
            .find(|sp| sp.for_contacts)
            .map(|sp| sp.id)
            .unwrap_or_else(|| s.vault.home_space());
        let c = Contact::person(space, "New", "contact");
        if s.vault.put_contact(&c).is_ok() {
            s.selected_contact = Some(c.id);
            s.contacts.push(c);
        }
        cx.notify();
    }
}
