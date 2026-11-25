//! Quick Access (screen 17): a small window over the app in front, pre-filtered by what was
//! detected there. Enter fills, Shift+Enter types, Ctrl+C copies the password, Ctrl+Alt+C the
//! 2FA code, Escape closes. It also closes when it loses the focus.

use crate::app::{CleisApp, Phase};
use crate::autofill::standard;
use crate::theme::*;
use crate::ui::*;
use cleis_autofill::Foreground;
use cleis_core::autofill::{self as logic, Reason};
use cleis_core::settings::FillMethod;
use cleis_core::text::{fold, matches};
use cleis_core::totp::Totp;
use cleis_core::{EntryId, now};
use gpui_kit::TestSupportExt;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::{
    Context, Entity, KeyDownEvent, SharedString, Subscription, Task, WeakEntity, Window, div, px,
};
use std::time::Duration;

pub struct QuickAccess {
    app: WeakEntity<CleisApp>,
    fg: Foreground,
    search: Entity<InputState>,
    query: String,
    selected: usize,
    _subs: Vec<Subscription>,
    _ticker: Task<()>,
}

/// One line of the list.
#[derive(Clone)]
pub struct Item {
    pub id: EntryId,
    pub name: String,
    pub sub: String,
    pub matched: Option<Reason>,
    pub totp: Option<Totp>,
}

/// Items for the window in front: matches first (by reason), then the others, all filtered
/// by the search text. Public for the end-to-end tests.
pub fn items(app: &CleisApp, fg: &Foreground, query: &str) -> Vec<Item> {
    let Phase::Main(s) = &app.phase else {
        return vec![];
    };
    let ranked = logic::rank(&s.entries, &fg.context);
    let space_name = |e: &cleis_core::Entry| {
        s.spaces
            .iter()
            .find(|sp| sp.id == e.space)
            .map(|sp| sp.name.clone())
            .unwrap_or_default()
    };
    let item = |e: &cleis_core::Entry, matched| Item {
        id: e.id,
        name: e.name.clone(),
        sub: [e.username.clone(), space_name(e)]
            .into_iter()
            .filter(|x| !x.is_empty())
            .collect::<Vec<_>>()
            .join(" · "),
        matched,
        totp: e.totp.clone(),
    };
    let q = query.trim();
    let keep = |e: &cleis_core::Entry| q.is_empty() || matches(&e.search_text(), q);
    let mut out: Vec<Item> = ranked
        .iter()
        .filter_map(|c| {
            s.entries
                .iter()
                .find(|e| e.id == c.entry)
                .map(|e| (e, c.reason))
        })
        .filter(|(e, _)| keep(e))
        .map(|(e, r)| item(e, Some(r)))
        .collect();
    let mut others: Vec<&cleis_core::Entry> = s
        .entries
        .iter()
        .filter(|e| !ranked.iter().any(|c| c.entry == e.id) && keep(e))
        .collect();
    others.sort_by_key(|e| (!e.favorite, fold(&e.name)));
    out.extend(others.into_iter().map(|e| item(e, None)));
    out
}

impl QuickAccess {
    pub fn new(
        app: WeakEntity<CleisApp>,
        fg: Foreground,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search"));
        let subs = vec![
            cx.subscribe_in(&search, window, |this, input, ev: &InputEvent, _, cx| {
                if let InputEvent::Change = ev {
                    this.query = input.read(cx).value().to_string();
                    this.selected = 0;
                    cx.notify();
                }
            }),
            cx.observe_window_activation(window, |this, window, cx| {
                if !window.is_window_active() {
                    this.close(window, cx);
                }
            }),
        ];
        search.update(cx, |s, cx| s.focus(window, cx));
        let ticker = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        });
        Self {
            app,
            fg,
            search,
            query: String::new(),
            selected: 0,
            _subs: subs,
            _ticker: ticker,
        }
    }

    fn list(&self, cx: &Context<Self>) -> Vec<Item> {
        self.app
            .upgrade()
            .map(|a| items(a.read(cx), &self.fg, &self.query))
            .unwrap_or_default()
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(a) = self.app.upgrade() {
            a.update(cx, |a, _| a.quick_access_closed());
        }
        window.remove_window();
    }

    /// Fills the selected item into the window that was in front.
    fn fill(&mut self, method: Option<FillMethod>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(it) = self.list(cx).get(self.selected).cloned() else {
            return;
        };
        let fg = self.fg.clone();
        self.close(window, cx);
        if let Some(a) = self.app.upgrade() {
            a.update(cx, |a, cx| {
                a.start_fill_with(it.id, fg, true, method, standard, cx);
            });
        }
    }

    fn copy(&mut self, totp: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(it) = self.list(cx).get(self.selected).cloned() else {
            return;
        };
        let Some(a) = self.app.upgrade() else { return };
        let (text, what) = {
            let app = a.read(cx);
            let Phase::Main(s) = &app.phase else { return };
            let Some(e) = s.entries.iter().find(|e| e.id == it.id) else {
                return;
            };
            if totp {
                match &e.totp {
                    Some(t) => (t.code_at(now()), "2FA code"),
                    None => return,
                }
            } else {
                (e.password.clone(), "Password")
            }
        };
        a.update(cx, |a, cx| a.copy(text, what, cx));
        self.close(window, cx);
    }

    fn on_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let k = &ev.keystroke;
        let m = &k.modifiers;
        let cmd = if cfg!(target_os = "macos") {
            m.platform
        } else {
            m.control
        };
        let n = self.list(cx).len();
        match k.key.as_str() {
            "escape" => self.close(window, cx),
            "down" => {
                self.selected = (self.selected + 1).min(n.saturating_sub(1));
                cx.notify();
            }
            "up" => {
                self.selected = self.selected.saturating_sub(1);
                cx.notify();
            }
            "enter" if m.shift => self.fill(Some(FillMethod::Type), window, cx),
            "enter" => self.fill(None, window, cx),
            "c" if cmd && m.alt => self.copy(true, window, cx),
            "c" if cmd => self.copy(false, window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }
}

fn hint(t: &Tokens, ks: &[&str], label: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(6.))
        .text_size(TEXT_SM)
        .text_color(t.text_muted)
        .child(keys(t, ks))
        .child(label.to_owned())
}

use gpui_kit::Div;

impl Render for QuickAccess {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx).clone();
        let list = self.list(cx);
        self.selected = self.selected.min(list.len().saturating_sub(1));
        let ctx = &self.fg.context;
        let detected: Option<String> = ctx
            .host()
            .or_else(|| (!ctx.app_name.is_empty()).then(|| ctx.app_name.clone()));
        let tip: SharedString =
            format!("Detected from the window in front: {}", ctx.program).into();
        let cmd = if cfg!(target_os = "macos") {
            "⌘"
        } else {
            "Ctrl"
        };
        let alt = if cfg!(target_os = "macos") {
            "⌥"
        } else {
            "Alt"
        };

        let mut rows = div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .px(px(8.))
            .pt(px(4.))
            .pb(px(8.));
        let head = |label: &str| {
            div()
                .px(px(8.))
                .pt(px(8.))
                .pb(px(4.))
                .text_size(TEXT_XS)
                .font_weight(W_MEDIUM)
                .text_color(t.text_muted)
                .child(label.to_owned())
        };
        let mut last_section: Option<Option<Reason>> = None;
        for (i, it) in list.iter().enumerate().take(9) {
            if last_section != Some(it.matched) {
                let label = match it.matched {
                    Some(Reason::Linked) => "Linked to this app",
                    Some(Reason::Website) => "Saved for this site",
                    Some(Reason::Name) => "Suggested",
                    None => "Other",
                };
                rows = rows.child(head(label));
                last_section = Some(it.matched);
            }
            let sel = i == self.selected;
            let letter = it.name.chars().next().unwrap_or('?');
            rows = rows.child(
                div()
                    .id(SharedString::from(format!("qa-item-{i}")))
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .h(px(44.))
                    .px(px(8.))
                    .rounded(RADIUS_MD)
                    .cursor_pointer()
                    .when(sel, |d| d.bg(t.selection))
                    .hover(|d| d.bg(t.surface_hover))
                    .child(site_icon(&t, letter, 28.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .min_w_0()
                            .flex_1()
                            .child(
                                div()
                                    .font_weight(W_MEDIUM)
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .child(it.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(TEXT_SM)
                                    .text_color(t.text_muted)
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .child(it.sub.clone()),
                            ),
                    )
                    .when(sel, |d| d.child(hint(&t, &["↵"], "Fill")))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.selected = i;
                        this.fill(None, window, cx);
                    }))
                    .test_support(),
            );
        }
        if list.is_empty() {
            rows = rows.child(
                div()
                    .px(px(8.))
                    .py(px(16.))
                    .text_color(t.text_muted)
                    .child("No item"),
            );
        }

        let totp = list.get(self.selected).and_then(|it| it.totp.clone());
        let foot = div()
            .h(px(40.))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(16.))
            .px(px(16.))
            .border_t_1()
            .border_color(t.line)
            .bg(t.surface)
            .child(hint(&t, &["Shift", "↵"], "Type"))
            .child(hint(&t, &[cmd, "C"], "Password"))
            .when(totp.is_some(), |d| {
                d.child(hint(&t, &[cmd, alt, "C"], "2FA code"))
            })
            .child(div().flex_1())
            .when_some(totp, |d, totp| {
                let n = now();
                let fraction = totp.remaining(n) as f32 / totp.period.max(1) as f32;
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .font_family(FONT_MONO)
                        .child(totp_ring(&t, fraction))
                        .child(Totp::display(&totp.code_at(n))),
                )
            });

        div()
            .id("quick-access")
            .size_full()
            .flex()
            .flex_col()
            .bg(t.surface_raised)
            .border_1()
            .border_color(t.line_strong)
            .text_color(t.text)
            .text_size(TEXT_MD)
            .font_family(FONT)
            .capture_key_down(cx.listener(Self::on_key))
            .child(
                div()
                    .h(px(48.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .px(px(16.))
                    .border_b_1()
                    .border_color(t.line)
                    .child(icon("search", 16., t.text_muted))
                    .child(
                        div()
                            .flex_1()
                            .child(Input::new(&self.search).id("qa-search").appearance(false)),
                    )
                    .when_some(detected, |d, label| {
                        d.child(
                            div()
                                .id("qa-context")
                                .tooltip(move |window, cx| {
                                    gpui_kit::component::tooltip::Tooltip::new(tip.clone())
                                        .build(window, cx)
                                })
                                .child(chip(&t, Tone::Auto, label, false)),
                        )
                    }),
            )
            .child(div().id("qa-list").flex_1().overflow_y_scroll().child(rows))
            .child(foot)
    }
}
