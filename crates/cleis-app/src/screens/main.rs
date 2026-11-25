//! Main window: top bar with centred tabs (charter §5) and the active tab's page.

use super::unlock::logo;
use crate::app::{CleisApp, Filter, Phase, Session, Tab};
use crate::format::{money, subscription_chip};
use crate::theme::*;
use crate::ui::*;
use cleis_core::column::{ColumnDef, ColumnKind};
use cleis_core::totp::Totp;
use cleis_core::{Entry, EntryId};
use gpui_kit::TestSupportExt;
use gpui_kit::component::input::Input;
use gpui_kit::prelude::*;
use gpui_kit::{AnyElement, Context, Div, Hsla, SharedString, Window, div, px, uniform_list};

pub fn render(
    app: &mut CleisApp,
    window: &mut Window,
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    let t = app.tokens(cx).clone();
    let Phase::Main(s) = &app.phase else {
        unreachable!()
    };
    let tab = s.tab;
    let page: AnyElement = match tab {
        Tab::Vault => vault_page(s, &t, window, cx).into_any_element(),
        Tab::Contacts => super::contacts::page(s, &t, cx).into_any_element(),
        Tab::Health => super::health::page(s, &t, cx).into_any_element(),
        Tab::Settings => super::settings::page(s, &t, cx).into_any_element(),
    };
    let toast = s.toast.clone();
    div()
        .size_full()
        .flex()
        .flex_col()
        .bg(t.chrome)
        .font_family(FONT)
        .text_size(TEXT_MD)
        .text_color(t.text)
        .child(topbar(s, &t, cx))
        .child(
            div()
                .flex_1()
                .min_h_0()
                .mx(px(6.))
                .mb(px(6.))
                .border_1()
                .border_color(t.stage_line)
                .rounded(px(10.))
                .bg(t.bg)
                .overflow_hidden()
                .relative()
                .flex()
                .flex_col()
                .child(page)
                .when_some(toast, |d, msg| {
                    d.child(
                        div()
                            .absolute()
                            .bottom(px(40.))
                            .left_0()
                            .right_0()
                            .flex()
                            .justify_center()
                            .child(
                                div()
                                    .px(px(12.))
                                    .py(px(6.))
                                    .rounded(RADIUS_MD)
                                    .bg(t.inverse)
                                    .text_color(t.inverse_ink)
                                    .text_size(TEXT_SM)
                                    .font_weight(W_MEDIUM)
                                    .child(msg),
                            ),
                    )
                }),
        )
}

fn topbar(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let mut tabs = vec![(Tab::Vault, "Vault")];
    if s.prefs.tabs.contacts {
        tabs.push((Tab::Contacts, "Contacts"));
    }
    if s.prefs.tabs.health {
        tabs.push((Tab::Health, "Health"));
    }
    // Activity exists only for an organisation (D-38): a personal vault has no log.
    tabs.push((Tab::Settings, "Settings"));
    let mut nav = div().flex().items_end().h(TOPBAR_H);
    for (tab, label) in tabs {
        let active = s.tab == tab;
        let mut el = div()
            .id(SharedString::from(format!("tab-{label}")))
            .h(px(32.))
            .px(px(16.))
            .flex()
            .items_center()
            .font_weight(W_MEDIUM)
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(s) = this.session() {
                    s.tab = tab;
                }
                cx.notify();
            }));
        el = if active {
            el.h(px(33.))
                .mb(px(-1.))
                .bg(t.bg)
                .text_color(t.accent_text)
                .border_1()
                .border_b_0()
                .border_color(t.stage_line)
                .rounded_t(px(8.))
        } else {
            el.text_color(t.text_muted)
        };
        nav = nav.child(el.child(label).test_support());
    }
    let primary = match s.tab {
        Tab::Vault => Some(("new-item", "New item")),
        Tab::Contacts => Some(("new-contact", "New contact")),
        _ => None,
    };
    div()
        .h(TOPBAR_H)
        .flex_none()
        .flex()
        .items_end()
        .pl(px(12.))
        .pr(px(8.))
        .child(
            div()
                .flex_1()
                .h(TOPBAR_H)
                .flex()
                .items_center()
                .gap(px(8.))
                .child(logo(t, 22.))
                .child(
                    div()
                        .h(CONTROL_MD)
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .font_weight(W_MEDIUM)
                        .child("My vault"),
                ),
        )
        .child(nav)
        .child(
            div()
                .flex_1()
                .h(TOPBAR_H)
                .flex()
                .items_center()
                .justify_end()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(TEXT_SM)
                        .text_color(t.text_muted)
                        .child(div().size(px(6.)).rounded_full().bg(t.success_text))
                        .child("On this computer"),
                )
                .child(
                    icon_button("lock", t, "lock", false)
                        .on_click(cx.listener(|this, _, window, cx| this.lock(window, cx)))
                        .test_support(),
                )
                .when_some(primary, |d, (id, label)| {
                    d.child(
                        button(id, t, BtnKind::Primary, false)
                            .child(icon("plus", 16., t.primary_ink))
                            .child(label)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                if id == "new-item" {
                                    this.new_entry(window, cx);
                                } else {
                                    this.new_contact(cx);
                                }
                            }))
                            .test_support(),
                    )
                }),
        )
}

fn sidebar(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let count = |f: &dyn Fn(&Entry) -> bool| s.entries.iter().filter(|e| f(e)).count();
    let item = |id: &str,
                ico: &str,
                label: String,
                n: Option<usize>,
                filter: Filter,
                cx: &mut Context<CleisApp>| {
        let active = s.filter == filter;
        div()
            .id(SharedString::from(id.to_owned()))
            .flex()
            .items_center()
            .gap(px(8.))
            .h(CONTROL_MD)
            .px(px(8.))
            .rounded(RADIUS_MD)
            .cursor_pointer()
            .text_color(if active { t.accent_text } else { t.text_muted })
            .when(active, |d| d.bg(t.accent_soft))
            .hover(|d| d.bg(t.surface_hover))
            .child(icon(
                ico,
                16.,
                if active { t.accent_text } else { t.text_muted },
            ))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(label),
            )
            .when_some(n, |d, n| {
                d.child(
                    div()
                        .text_size(TEXT_SM)
                        .text_color(t.text_faint)
                        .child(n.to_string()),
                )
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(s) = this.session() {
                    s.filter = filter;
                    s.refilter();
                }
                cx.notify();
            }))
    };
    let group = |label: &str| {
        div()
            .pt(px(12.))
            .pb(px(4.))
            .px(px(8.))
            .text_size(TEXT_XS)
            .text_color(t.text_muted)
            .font_weight(W_MEDIUM)
            .child(label.to_owned())
    };
    let mut d = div()
        .w(SIDEBAR_W)
        .flex_none()
        .h_full()
        .bg(t.surface)
        .border_r_1()
        .border_color(t.line)
        .p(px(8.))
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(item(
            "f-all",
            "list",
            "All items".into(),
            Some(s.entries.len()),
            Filter::All,
            cx,
        ))
        .child(item(
            "f-fav",
            "star",
            "Favorites".into(),
            Some(count(&|e| e.favorite)),
            Filter::Favorites,
            cx,
        ))
        .child(item(
            "f-renew",
            "clock",
            "Renew soon".into(),
            Some(count(&|e| {
                e.subscription == cleis_core::Subscription::RenewSoon
            })),
            Filter::RenewSoon,
            cx,
        ));
    let personal: Vec<_> = s
        .spaces
        .iter()
        .filter(|sp| !sp.for_contacts && sp.kind == cleis_store::SpaceKind::Personal)
        .collect();
    let org: Vec<_> = s
        .spaces
        .iter()
        .filter(|sp| !sp.for_contacts && sp.kind == cleis_store::SpaceKind::Organization)
        .collect();
    if !personal.is_empty() {
        d = d.child(group("Personal"));
        for sp in personal {
            let n = s.entries.iter().filter(|e| e.space == sp.id).count();
            d = d.child(item(
                &format!("sp-{}", sp.id),
                "user",
                sp.name.clone(),
                Some(n),
                Filter::Space(sp.id),
                cx,
            ));
        }
    }
    if !org.is_empty() {
        d = d.child(group("Organization"));
        for sp in org {
            let n = s.entries.iter().filter(|e| e.space == sp.id).count();
            d = d.child(item(
                &format!("sp-{}", sp.id),
                "folder",
                sp.name.clone(),
                Some(n),
                Filter::Space(sp.id),
                cx,
            ));
        }
    }
    d.child(div().flex_1())
}

/// Width of each built-in column, as in mockup 01.
fn col_width(c: &ColumnDef) -> Option<f32> {
    match c.id.0.as_str() {
        "name" => Some(190.),
        "username" | "notes" => None,
        "password" => Some(116.),
        "totp" => Some(100.),
        "subscription" => Some(120.),
        "price" => Some(120.),
        "licenses" => Some(72.),
        "yearly_cost" => Some(100.),
        "website" => Some(160.),
        "renewal" => Some(110.),
        _ => Some(140.),
    }
}

fn is_num(c: &ColumnDef) -> bool {
    matches!(
        c.kind,
        ColumnKind::Price { .. } | ColumnKind::Number { .. } | ColumnKind::Formula { .. }
    )
}

fn cell_box(c: &ColumnDef) -> Div {
    let d = div()
        .h_full()
        .px(px(12.))
        .flex()
        .items_center()
        .overflow_hidden()
        .whitespace_nowrap()
        .min_w_0();
    let d = match col_width(c) {
        Some(w) => d.w(px(w)).flex_none(),
        None => d.flex_1(),
    };
    if is_num(c) { d.justify_end() } else { d }
}

fn totp_view(t: &Tokens, totp: &Totp, now: i64, big: bool) -> Div {
    let code = Totp::display(&totp.code_at(now));
    let frac = totp.remaining(now) as f32 / totp.period as f32;
    div()
        .flex()
        .items_center()
        .gap(px(6.))
        .font_family(FONT_MONO)
        .when(big, |d| d.text_size(px(17.)))
        .child(totp_ring(t, frac))
        .child(code)
}

fn faint_dash(t: &Tokens) -> Div {
    div().text_color(t.text_faint).child("—")
}

fn row_cell(
    e: &Entry,
    c: &ColumnDef,
    cols: &[ColumnDef],
    t: &Tokens,
    now: i64,
    selected: bool,
    cx: &mut Context<CleisApp>,
) -> AnyElement {
    let id = e.id;
    let base = cell_box(c);
    match c.id.0.as_str() {
        "name" => base
            .gap(px(8.))
            .child(site_icon(t, e.name.chars().next().unwrap_or('?'), 20.))
            .child(
                div()
                    .font_weight(W_MEDIUM)
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(e.name.clone()),
            )
            .into_any_element(),
        "username" => {
            let u = e.username.clone();
            base.text_color(t.text_muted)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(e.username.clone()),
                )
                .child(
                    icon_button(SharedString::from(format!("cu-{id}")), t, "copy", true)
                        .when(!selected, |d| {
                            d.invisible().group_hover("row", |s| s.visible())
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.copy(u.clone(), "Username", cx);
                        }))
                        .test_support(),
                )
                .into_any_element()
        }
        "password" => {
            let p = e.password.clone();
            base.child(
                div()
                    .flex_1()
                    .text_color(t.text_muted)
                    .child(if e.password.is_empty() {
                        ""
                    } else {
                        "••••••"
                    }),
            )
            .child(
                icon_button(SharedString::from(format!("cp-{id}")), t, "copy", true)
                    .when(!selected, |d| {
                        d.invisible().group_hover("row", |s| s.visible())
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.copy(p.clone(), "Password", cx);
                    })),
            )
            .into_any_element()
        }
        "totp" => match &e.totp {
            Some(totp) => base
                .child(totp_view(t, totp, now, false))
                .into_any_element(),
            None => base.child(faint_dash(t)).into_any_element(),
        },
        "subscription" => base
            .when_some(subscription_chip(t, e.subscription), |d, c| d.child(c))
            .into_any_element(),
        "price" => match &e.price {
            Some(m) => base
                .child(money(m, true))
                .when_some(m.period.filter(|_| true), |d, p| {
                    d.child(
                        div()
                            .text_color(t.text_faint)
                            .child(crate::format::period_suffix(p)),
                    )
                })
                .into_any_element(),
            None => base.child(faint_dash(t)).into_any_element(),
        },
        "licenses" => base
            .child(
                e.licenses
                    .map(|l| l.to_string())
                    .unwrap_or_else(|| "—".into()),
            )
            .into_any_element(),
        _ => {
            if let ColumnKind::Formula { expr } = &c.kind {
                let v = e.compute(expr, cols).ok().flatten();
                return base
                    .child(
                        v.map(|n| crate::format::num(&n))
                            .unwrap_or_else(|| "—".into()),
                    )
                    .into_any_element();
            }
            let text = e
                .cell(c)
                .and_then(|v| v.as_text().map(str::to_owned))
                .unwrap_or_default();
            base.text_color(t.text_muted)
                .child(div().overflow_hidden().text_ellipsis().child(text))
                .into_any_element()
        }
    }
}

fn vault_page(s: &Session, t: &Tokens, _window: &mut Window, cx: &mut Context<CleisApp>) -> Div {
    let cols: Vec<ColumnDef> =
        cleis_core::column::visible_columns(&s.columns, cleis_core::column::Table::Vault, None)
            .into_iter()
            .cloned()
            .collect();
    let mut head = div()
        .w_full()
        .h(px(30.))
        .flex_none()
        .flex()
        .border_b_1()
        .border_color(t.line)
        .bg(t.bg)
        .text_size(TEXT_XS)
        .font_weight(W_MEDIUM)
        .text_color(t.text_muted);
    for c in &cols {
        head = head.child(
            cell_box(c)
                .when(c.id.0 == "name", |d| {
                    d.text_color(t.text).child(format!("{} ↑", c.name))
                })
                .when(c.id.0 != "name", |d| d.child(c.name.clone())),
        );
    }
    let count = s.visible.len();
    let total: i64 = s
        .visible
        .iter()
        .filter_map(|i| {
            s.entries[*i]
                .price
                .as_ref()
                .map(|p| p.yearly_cents() * i64::from(s.entries[*i].licenses.unwrap_or(1)))
        })
        .sum();
    let list = uniform_list(
        "entries",
        count,
        cx.processor(
            move |this: &mut CleisApp, range: std::ops::Range<usize>, _window, cx| {
                let t = this.tokens(cx).clone();
                let now = cleis_core::now();
                let Phase::Main(s) = &this.phase else {
                    return vec![];
                };
                let cols: Vec<ColumnDef> = cleis_core::column::visible_columns(
                    &s.columns,
                    cleis_core::column::Table::Vault,
                    None,
                )
                .into_iter()
                .cloned()
                .collect();
                let all_cols = s.columns.clone();
                let rows: Vec<(EntryId, Entry)> = range
                    .filter_map(|ix| {
                        s.visible
                            .get(ix)
                            .map(|i| (s.entries[*i].id, s.entries[*i].clone()))
                    })
                    .collect();
                let sel = s.selected;
                rows.into_iter()
                    .map(|(id, e)| {
                        let selected = sel == Some(id);
                        let mut row = div()
                            .id(SharedString::from(format!("row-{id}")))
                            .group("row")
                            .w_full()
                            .h(ROW_H)
                            .flex()
                            .border_b_1()
                            .border_color(t.line)
                            .cursor_pointer()
                            .when(selected, |d| d.bg(t.selection))
                            .when(!selected, |d| d.hover(|s| s.bg(t.surface_hover)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(s) = this.session() {
                                    s.selected = Some(id);
                                    s.reveal = false;
                                }
                                cx.notify();
                            }));
                        for c in &cols {
                            row = row.child(row_cell(&e, c, &all_cols, &t, now, selected, cx));
                        }
                        row
                    })
                    .collect()
            },
        ),
    )
    .flex_1();
    let center =
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(44.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(12.))
                    .border_b_1()
                    .border_color(t.line)
                    .child(div().w(px(280.)).child(
                        Input::new(&s.search).id("search").prefix(icon(
                            "search",
                            16.,
                            t.text_muted,
                        )),
                    ))
                    .child(
                        button("filter", t, BtnKind::Ghost, false)
                            .child(icon("filter", 16., t.text_muted))
                            .child("Filter"),
                    )
                    .child(
                        button("columns", t, BtnKind::Ghost, false)
                            .child(icon("columns", 16., t.text_muted))
                            .child("Columns"),
                    ),
            )
            .child(head)
            .child(list)
            .child(
                div()
                    .h(ROW_H)
                    .flex_none()
                    .flex()
                    .items_center()
                    .px(px(12.))
                    .text_size(TEXT_SM)
                    .text_color(t.text_muted)
                    .child(div().flex_1().child(format!("{count} items")))
                    .child(
                        div()
                            .font_weight(W_MEDIUM)
                            .text_color(t.text)
                            .child(crate::format::money_cents(total, "EUR")),
                    ),
            );
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
                .child(sidebar(s, t, cx))
                .child(center)
                .when_some(s.selected_entry().cloned(), |d, e| {
                    d.child(inspector(s, &e, t, cx))
                }),
        )
        .child(statusbar(s, t))
}

fn statusbar(s: &Session, t: &Tokens) -> Div {
    let crumb = s
        .selected_entry()
        .map(|e| {
            let sp = s
                .spaces
                .iter()
                .find(|sp| sp.id == e.space)
                .map(|sp| sp.name.clone())
                .unwrap_or_default();
            format!("{sp} › {}", e.name)
        })
        .unwrap_or_default();
    div()
        .h(STATUSBAR_H)
        .flex_none()
        .flex()
        .items_center()
        .gap(px(16.))
        .px(px(12.))
        .border_t_1()
        .border_color(t.line)
        .bg(t.surface)
        .text_size(TEXT_SM)
        .text_color(t.text_muted)
        .child(div().flex_1().child(crumb))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(icon("shield", 14., t.text_muted))
                .child("End-to-end encryption"),
        )
        .child("Saved")
}

fn inspector(s: &Session, e: &Entry, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let id = e.id;
    let now = cleis_core::now();
    let space = s
        .spaces
        .iter()
        .find(|sp| sp.id == e.space)
        .map(|sp| sp.name.clone())
        .unwrap_or_default();
    let copy_btn = |key: &str, value: String, what: &'static str, cx: &mut Context<CleisApp>| {
        icon_button(SharedString::from(format!("{key}-{id}")), t, "copy", true)
            .on_click(cx.listener(move |this, _, _, cx| this.copy(value.clone(), what, cx)))
    };
    let strength = cleis_core::generator::entropy_bits(&e.password);
    let level = if e.password.is_empty() {
        0
    } else if strength >= 90. {
        4
    } else if strength >= 70. {
        3
    } else if strength >= 50. {
        2
    } else {
        1
    };
    let bar_color: Hsla = match level {
        4 | 3 => t.success_text,
        2 => t.warning_text,
        _ => t.danger_text,
    };
    let mut bars = div().flex().gap(px(3.));
    for i in 0..4 {
        bars = bars.child(div().flex_1().h(px(3.)).rounded(px(2.)).bg(if i < level {
            bar_color
        } else {
            t.line_strong
        }));
    }
    let website = e.urls.first().cloned();
    let yearly = cleis_core::column::default_vault_columns("EUR")
        .iter()
        .find(|c| c.id.0 == "yearly_cost")
        .and_then(|c| {
            if let ColumnKind::Formula { expr } = &c.kind {
                e.compute(expr, &s.columns).ok().flatten()
            } else {
                None
            }
        });
    let fav = e.favorite;
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
                        .child(
                            div()
                                .text_size(TEXT_LG)
                                .font_weight(W_MEDIUM)
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(e.name.clone()),
                        )
                        .child(
                            div()
                                .text_size(TEXT_SM)
                                .text_color(t.text_muted)
                                .child(space),
                        ),
                )
                .child(
                    icon_button("fav", t, "star", false)
                        .when(fav, |d| d.text_color(t.warning_text))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.update_entry(id, |e| e.favorite = !e.favorite, cx)
                        }))
                        .test_support(),
                )
                .child(icon_button("more", t, "more", false)),
        )
        .child(
            div()
                .flex()
                .gap(px(2.))
                .px(px(12.))
                .border_b_1()
                .border_color(t.line)
                .child(
                    div()
                        .h(px(32.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .font_weight(W_MEDIUM)
                        .border_b_2()
                        .border_color(t.accent_text)
                        .text_color(t.accent_text)
                        .child("Details"),
                )
                .child(
                    div()
                        .h(px(32.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .font_weight(W_MEDIUM)
                        .text_color(t.text_muted)
                        .child("History"),
                ),
        )
        .child(
            div()
                .id("inspector-body")
                .flex_1()
                .overflow_y_scroll()
                .p(px(12.))
                .flex()
                .flex_col()
                .gap(px(12.))
                .when_some(website, |d, w| {
                    let url = if w.contains("://") {
                        w.clone()
                    } else {
                        format!("https://{w}")
                    };
                    d.child(field(
                        t,
                        "Website",
                        value_box(t)
                            .child(div().flex_1().overflow_hidden().text_ellipsis().child(w))
                            .child(
                                icon_button("open", t, "external", true)
                                    .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url))),
                            ),
                    ))
                })
                .child(field(
                    t,
                    "Username",
                    value_box(t)
                        .child(
                            div()
                                .flex_1()
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(e.username.clone()),
                        )
                        .child(copy_btn("iu", e.username.clone(), "Username", cx)),
                ))
                .child(
                    field(
                        t,
                        "Password",
                        value_box(t)
                            .child(
                                div()
                                    .flex_1()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .when(!s.reveal, |d| d.text_color(t.text_muted))
                                    .when(s.reveal, |d| d.font_family(FONT_MONO))
                                    .child(if s.reveal {
                                        e.password.clone()
                                    } else {
                                        "••••••••••••••".into()
                                    }),
                            )
                            .child(icon_button("reveal", t, "eye", true).on_click(cx.listener(
                                |this, _, _, cx| {
                                    if let Some(s) = this.session() {
                                        s.reveal = !s.reveal;
                                    }
                                    cx.notify();
                                },
                            )))
                            .child(copy_btn("ip", e.password.clone(), "Password", cx))
                            .child(icon_button("gen", t, "refresh", true).on_click(cx.listener(
                                move |this, _, _, cx| {
                                    if let Ok(p) =
                                        cleis_core::generator::generate(&Default::default())
                                    {
                                        this.update_entry(id, |e| e.set_password(p, None), cx);
                                    }
                                },
                            ))),
                    )
                    .child(bars),
                )
                .when_some(e.totp.clone(), |d, totp| {
                    let code = totp.code_at(now);
                    d.child(field(
                        t,
                        "2FA code",
                        value_box(t)
                            .h(px(36.))
                            .child(div().flex_1().child(totp_view(t, &totp, now, true)))
                            .child(copy_btn("it", code, "Code", cx)),
                    ))
                })
                .when(!e.notes.is_empty(), |d| {
                    d.child(field(
                        t,
                        "Notes",
                        value_box(t)
                            .h_auto()
                            .min_h(px(48.))
                            .py(px(6.))
                            .items_start()
                            .text_color(t.text_muted)
                            .child(e.notes.clone()),
                    ))
                })
                .when(
                    e.price.is_some() || e.subscription != cleis_core::Subscription::None,
                    |d| {
                        let kv = |k: &str, v: String| {
                            div()
                                .flex()
                                .text_size(TEXT_SM)
                                .child(div().flex_1().text_color(t.text_muted).child(k.to_owned()))
                                .child(v)
                        };
                        d.child(
                            div()
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
                                                .child("Subscription"),
                                        )
                                        .when_some(subscription_chip(t, e.subscription), |d, c| {
                                            d.child(c)
                                        }),
                                )
                                .when_some(e.price.clone(), |d, m| {
                                    d.child(kv(
                                        "Price",
                                        format!(
                                            "{}{}",
                                            crate::format::money_text(&m),
                                            m.period
                                                .map(crate::format::period_long)
                                                .unwrap_or_default()
                                        ),
                                    ))
                                })
                                .when_some(e.licenses, |d, l| {
                                    d.child(kv("Licenses", l.to_string()))
                                })
                                .when_some(yearly.clone(), |d, y| {
                                    d.child(kv("Yearly cost", crate::format::num(&y)))
                                })
                                .when_some(e.renewal.clone(), |d, r| d.child(kv("Renewal", r))),
                        )
                    },
                ),
        )
}

impl CleisApp {
    pub fn update_entry(
        &mut self,
        id: EntryId,
        f: impl FnOnce(&mut Entry),
        cx: &mut Context<Self>,
    ) {
        let Some(s) = self.session() else { return };
        let Some(e) = s.entries.iter_mut().find(|e| e.id == id) else {
            return;
        };
        f(e);
        e.touch(None);
        let result = s.vault.put_entry(e);
        s.toast = Some(match result {
            Ok(()) => "Saved".into(),
            Err(err) => format!("Not saved: {err}").into(),
        });
        s.refilter();
        cx.notify();
    }

    pub fn new_entry(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let space = match s.filter {
            Filter::Space(sp) => sp,
            _ => s.vault.home_space(),
        };
        let e = Entry::new(space, "New item");
        if s.vault.put_entry(&e).is_ok() {
            s.selected = Some(e.id);
            s.entries.push(e);
            s.refilter();
        }
        cx.notify();
    }
}
