//! Screen 12 — Health report: weak, reused, old passwords and items without 2FA, by space;
//! a figure filters the list, "Change" opens the item with a new password ready.

use crate::app::{CleisApp, Filter, Session, Tab};
use crate::i18n::tr;
use crate::theme::*;
use crate::ui::*;
use cleis_core::health::{Issue, Policy, scan_by};
use cleis_core::{EntryId, SpaceId};
use gpui_kit::TestSupportExt;
use gpui_kit::prelude::*;
use gpui_kit::{Context, Hsla, SharedString, Window, deferred, div, px};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IssueKind {
    #[default]
    Any,
    Weak,
    Reused,
    Old,
    NoTwoFactor,
}

#[derive(Default)]
pub struct HealthUi {
    /// Vault shown: the personal vault or one organization (mockup 32e); `None`: all.
    pub vault: Option<crate::app::Scope>,
    pub vault_menu: bool,
    pub space: Option<SpaceId>,
    pub only: IssueKind,
    pub space_menu: bool,
}

fn matches(i: &Issue, k: IssueKind) -> bool {
    matches!(
        (i, k),
        (_, IssueKind::Any)
            | (Issue::Weak, IssueKind::Weak)
            | (Issue::Reused { .. }, IssueKind::Reused)
            | (Issue::Old, IssueKind::Old)
            | (Issue::NoTwoFactor, IssueKind::NoTwoFactor)
    )
}

fn tip(
    text: &'static str,
) -> impl Fn(&mut Window, &mut gpui_kit::App) -> gpui_kit::AnyView + 'static {
    move |window, cx| {
        gpui_kit::component::tooltip::Tooltip::new(crate::i18n::tr_dynamic(text)).build(window, cx)
    }
}

pub fn page(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> impl IntoElement + use<> {
    let ui = &s.health;
    let entries: Vec<_> = s
        .entries
        .iter()
        .filter(|e| ui.vault.as_ref().is_none_or(|v| s.scope_has(v, e.space)))
        .filter(|e| ui.space.is_none_or(|sp| e.space == sp))
        .cloned()
        .collect();
    // Audit L-3: an organization's items are judged by its policy.
    let own = Policy::default();
    // D-69: each organization's items by its own policy.
    let orgs: Vec<(String, Policy)> = s
        .all_orgs()
        .filter_map(|o| {
            o.policy().map(|p| {
                (
                    o.m.key(),
                    Policy {
                        min_length: p.min_length as usize,
                        max_age_days: p.max_age_days,
                        ..Policy::default()
                    },
                )
            })
        })
        .collect();
    let (findings, sum) = scan_by(
        &entries,
        |e| {
            s.org_of(e.space)
                .and_then(|o| orgs.iter().find(|(k, _)| *k == o.m.key()))
                .map_or(&own, |(_, p)| p)
        },
        cleis_core::now(),
    );
    let space_name = |id: SpaceId| {
        s.spaces
            .iter()
            .find(|sp| sp.id == id)
            .map(|sp| crate::i18n::space(&sp.name))
            .unwrap_or_default()
    };
    let stat = |id: &'static str,
                n: usize,
                label: &str,
                color: Hsla,
                kind: IssueKind,
                cx: &mut Context<CleisApp>| {
        let on = ui.only == kind && kind != IssueKind::Any;
        section(t)
            .gap(px(2.))
            .when(on, |d| d.border_color(t.accent_text))
            .child(
                div()
                    .text_size(TEXT_XL)
                    .line_height(px(26.))
                    .font_weight(W_STRONG)
                    .text_color(color)
                    .child(n.to_string()),
            )
            .child(
                div()
                    .text_size(TEXT_SM)
                    .text_color(t.text_muted)
                    .child(crate::i18n::tr_owned(label)),
            )
            .id(id)
            .flex_1()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(s) = this.session() {
                    s.health.only = if s.health.only == kind {
                        IssueKind::Any
                    } else {
                        kind
                    };
                }
                cx.notify();
            }))
            .test_support()
    };
    let head_cell = |w: f32, label: &str| {
        let d = div().px(px(12.)).child(crate::i18n::tr_owned(label));
        if w > 0. {
            d.w(px(w)).flex_none()
        } else {
            d.flex_1()
        }
    };
    let mut table = div().flex().flex_col().child(
        div()
            .h(px(30.))
            .flex()
            .items_center()
            .border_b_1()
            .border_color(t.line)
            .text_size(TEXT_XS)
            .font_weight(W_MEDIUM)
            .text_color(t.text_muted)
            .child(head_cell(260., "Item"))
            .child(head_cell(0., "Issues"))
            .child(head_cell(140., "Space"))
            .child(head_cell(160., "Last changed"))
            .child(head_cell(100., "")),
    );
    let shown: Vec<_> = findings
        .iter()
        .filter(|f| f.issues.iter().any(|i| matches(i, ui.only)))
        .collect();
    for f in shown.iter().take(200) {
        let Some(e) = entries.iter().find(|e| e.id == f.entry) else {
            continue;
        };
        let id = e.id;
        let mut chips = div().flex().gap(px(4.));
        for i in &f.issues {
            chips = chips.child(match i {
                Issue::Weak => chip(t, Tone::Danger, tr("Weak"), false),
                Issue::Reused { count } => {
                    chip(t, Tone::Warn, crate::trf!("Reused ×{}", count), false)
                }
                Issue::Old => chip(t, Tone::Warn, tr("Over a year"), false),
                Issue::NoTwoFactor => chip(t, Tone::Neutral, tr("No 2FA"), false),
            });
        }
        let needs_password = f.issues.iter().any(|i| !matches!(i, Issue::NoTwoFactor));
        table = table.child(
            div()
                .h(ROW_H)
                .flex()
                .items_center()
                .border_b_1()
                .border_color(t.line)
                .child(
                    div()
                        .w(px(260.))
                        .flex_none()
                        .px(px(12.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(site_icon(t, e.name.chars().next().unwrap_or('?'), 20.))
                        .child(
                            div()
                                .font_weight(W_MEDIUM)
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(e.name.clone()),
                        ),
                )
                .child(div().flex_1().px(px(12.)).child(chips))
                .child(
                    div()
                        .w(px(140.))
                        .flex_none()
                        .px(px(12.))
                        .text_color(t.text_muted)
                        .child(space_name(e.space)),
                )
                .child(
                    div()
                        .w(px(160.))
                        .flex_none()
                        .px(px(12.))
                        .text_color(t.text_muted)
                        .child(crate::format::date(e.password_changed)),
                )
                .child(
                    div().w(px(100.)).flex_none().px(px(12.)).child(
                        button(
                            SharedString::from(format!("fix-{id}")),
                            t,
                            BtnKind::Default,
                            true,
                        )
                        .child(if needs_password {
                            tr("Change")
                        } else {
                            tr("Add 2FA")
                        })
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.fix_from_health(id, needs_password, window, cx)
                        }))
                        .test_support(),
                    ),
                ),
        );
    }
    if shown.is_empty() {
        table = table.child(
            div()
                .p(px(24.))
                .flex()
                .justify_center()
                .gap(px(8.))
                .text_color(t.text_muted)
                .child(icon("check", 16., t.success_text))
                .child(tr("Nothing to fix")),
        );
    }
    let label = ui
        .space
        .map(space_name)
        .unwrap_or_else(|| tr("All spaces").into());
    let mut menu = menu_box(t).top(px(32.)).right_0().w(px(220.)).child(
        menu_item("hs-all", t, None, tr("All spaces"), false)
            .on_click(cx.listener(|this, _, _, cx| this.health_space(None, cx)))
            .test_support(),
    );
    for sp in s
        .spaces
        .iter()
        .filter(|sp| !sp.for_contacts && ui.vault.as_ref().is_none_or(|v| s.scope_has(v, sp.id)))
    {
        let id = sp.id;
        menu = menu.child(
            menu_item(
                format!("hs-{id}"),
                t,
                None,
                crate::i18n::space(&sp.name),
                false,
            )
            .on_click(cx.listener(move |this, _, _, cx| this.health_space(Some(id), cx)))
            .test_support(),
        );
    }
    // Mockup 32e: with an organization, the vault comes first (personal, then each one).
    use crate::app::Scope;
    let mut vaults: Vec<(Option<Scope>, String, Option<&str>, String)> =
        vec![(None, tr("All vaults").into(), None, "hv-all".into())];
    vaults.push((
        Some(Scope::Personal),
        s.personal_name(),
        Some("user"),
        "hv-personal".into(),
    ));
    for (key, name) in s.org_tabs() {
        vaults.push((
            Some(Scope::Org(key.clone())),
            name,
            Some("folder"),
            format!("hv-{key}"),
        ));
    }
    let vault_label = vaults
        .iter()
        .find(|v| v.0 == ui.vault)
        .map(|v| v.1.clone())
        .unwrap_or_default();
    let mut vmenu = menu_box(t).top(px(32.)).left_0().w(px(220.));
    for (scope, name, ico, id) in vaults {
        let on = scope == ui.vault;
        vmenu = vmenu.child(
            menu_item(id, t, ico, name, false)
                .child(div().flex_1())
                .when(on, |d| d.child(icon("check", 14., t.text)))
                .on_click(cx.listener(move |this, _, _, cx| this.health_vault(scope.clone(), cx)))
                .test_support(),
        );
    }
    let vault_select = (s.org.is_some()).then(|| {
        div()
            .relative()
            .child(
                div()
                    .id("health-vault")
                    .tooltip(tip(tr("Vault")))
                    .child(
                        value_box(t)
                            .w(px(170.))
                            .cursor_pointer()
                            .child(
                                div()
                                    .flex_1()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .child(vault_label),
                            )
                            .child(icon("chevron", 14., t.text_muted)),
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(s) = this.session() {
                            s.health.vault_menu = !s.health.vault_menu;
                            s.health.space_menu = false;
                        }
                        cx.notify();
                    }))
                    .test_support(),
            )
            .when(ui.vault_menu, |d| d.child(deferred(vmenu)))
    });
    div()
        .id("health")
        .flex_1()
        .overflow_y_scroll()
        .px(px(32.))
        .py(px(24.))
        .child(
            div()
                .max_w(px(1100.))
                .mx_auto()
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .child(
                            div()
                                .flex_1()
                                .text_size(TEXT_XL)
                                .line_height(px(26.))
                                .font_weight(W_STRONG)
                                .child(tr("Health report")),
                        )
                        .children(vault_select)
                        .child(
                            div()
                                .relative()
                                .child(
                                    div()
                                        .id("health-space")
                                        .child(
                                            value_box(t)
                                                .w(px(170.))
                                                .cursor_pointer()
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .overflow_hidden()
                                                        .text_ellipsis()
                                                        .child(crate::i18n::tr_owned(&label)),
                                                )
                                                .child(icon("chevron", 14., t.text_muted)),
                                        )
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            if let Some(s) = this.session() {
                                                s.health.space_menu = !s.health.space_menu;
                                                s.health.vault_menu = false;
                                            }
                                            cx.notify();
                                        }))
                                        .test_support(),
                                )
                                .when(ui.space_menu, |d| d.child(deferred(menu))),
                        )
                        .child(
                            button("health-scan", t, BtnKind::Default, false)
                                .child(icon("refresh", 16., t.text))
                                .child(tr("Scan"))
                                .tooltip(tip(tr("Check again (the report also updates by itself)")))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.toast(tr("Scanned"), cx);
                                }))
                                .test_support(),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(12.))
                        .child(stat(
                            "st-weak",
                            sum.weak,
                            "Weak",
                            t.danger_text,
                            IssueKind::Weak,
                            cx,
                        ))
                        .child(stat(
                            "st-reused",
                            sum.reused,
                            "Reused",
                            t.warning_text,
                            IssueKind::Reused,
                            cx,
                        ))
                        .child(stat(
                            "st-old",
                            sum.old,
                            "Over a year old",
                            t.warning_text,
                            IssueKind::Old,
                            cx,
                        ))
                        .child(stat(
                            "st-2fa",
                            sum.no_two_factor,
                            "No 2FA",
                            t.text,
                            IssueKind::NoTwoFactor,
                            cx,
                        ))
                        .child(stat(
                            "st-ok",
                            sum.ok,
                            "No issue",
                            t.success_text,
                            IssueKind::Any,
                            cx,
                        )),
                )
                .child(section(t).p_0().overflow_hidden().child(table)),
        )
}

impl CleisApp {
    fn health_vault(&mut self, v: Option<crate::app::Scope>, cx: &mut Context<Self>) {
        if let Some(s) = self.session() {
            // A space of another vault is no longer shown.
            if let (Some(sp), Some(scope)) = (s.health.space, v.as_ref())
                && !s.scope_has(scope, sp)
            {
                s.health.space = None;
            }
            s.health.vault = v;
            s.health.vault_menu = false;
        }
        cx.notify();
    }

    fn health_space(&mut self, sp: Option<SpaceId>, cx: &mut Context<Self>) {
        if let Some(s) = self.session() {
            s.health.space = sp;
            s.health.space_menu = false;
        }
        cx.notify();
    }

    /// Opens the item in the vault: with a new generated password (the old one stays in its
    /// history) when the password is the problem, on the 2FA key box otherwise.
    pub fn fix_from_health(
        &mut self,
        id: EntryId,
        password: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(s) = self.session() else { return };
        s.tab = Tab::Vault;
        s.filter = Filter::All;
        s.selected = Some(id);
        s.history_tab = false;
        s.refilter();
        cx.notify();
        if password {
            self.ensure_editor(window, cx);
            self.generate_password(id, window, cx);
        }
    }
}
