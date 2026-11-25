//! Settings › Import & export (screen 14): CSV import in three steps — file, columns, review.

use crate::app::{CleisApp, Phase, Session, Tab};
use crate::theme::*;
use crate::ui::*;
use cleis_core::column::{ColumnKind, Table};
use cleis_core::import_csv::{
    self, Mapping, Outcome, Preview, TARGETS, Target, already_there, target_label,
};
use cleis_core::{Entry, SpaceId};
use gpui_kit::TestSupportExt;
use gpui_kit::prelude::*;
use gpui_kit::{
    Context, Div, ExternalPaths, PathPromptOptions, SharedString, Window, deferred, div, px,
};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct ImportState {
    pub file: Option<Loaded>,
    /// Step shown once a file is loaded: 2 = columns, 3 = review.
    pub step: u8,
    /// Row whose target menu is open.
    pub menu: Option<usize>,
    pub space_menu: bool,
    pub space: Option<SpaceId>,
    pub skip_existing: bool,
    pub error: Option<SharedString>,
}

pub struct Loaded {
    pub name: String,
    pub preview: Preview,
    pub mappings: Vec<Mapping>,
}

/// What the review step shows, built from the current mappings.
pub struct Review {
    pub outcome: Outcome,
    pub existing: Vec<bool>,
}

impl Review {
    pub fn new_count(&self, skip_existing: bool) -> usize {
        self.existing
            .iter()
            .filter(|e| !(skip_existing && **e))
            .count()
    }
}

fn steps(t: &Tokens, n: u8) -> Div {
    let step = |k: u8, label: &'static str| {
        let (bg, fg) = if k < n {
            (t.surface_active, t.success_text)
        } else if k == n {
            (t.primary, t.primary_ink)
        } else {
            (t.surface_active, t.text_muted)
        };
        div()
            .flex()
            .items_center()
            .gap(px(6.))
            .text_size(TEXT_SM)
            .text_color(if k == n { t.text } else { t.text_muted })
            .child(
                div()
                    .size(px(18.))
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(bg)
                    .text_size(TEXT_XS)
                    .text_color(fg)
                    .when(k < n, |d| d.child(icon("check", 12., fg)))
                    .when(k >= n, |d| d.child(k.to_string())),
            )
            .child(label)
    };
    let line = || div().w(px(24.)).h(px(1.)).bg(t.line_strong);
    div()
        .flex()
        .items_center()
        .gap(px(16.))
        .child(step(1, "File"))
        .child(line())
        .child(step(2, "Columns"))
        .child(line())
        .child(step(3, "Review"))
}

fn head(t: &Tokens, n: u8) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(12.))
        .min_h(px(32.))
        .child(
            div()
                .flex_1()
                .text_size(TEXT_XL)
                .font_weight(W_STRONG)
                .child("Import"),
        )
        .child(steps(t, n))
}

/// Select-looking box (`.input.select`).
fn select_box(t: &Tokens, label: impl Into<SharedString>, faint: bool) -> Div {
    value_box(t)
        .w_full()
        .cursor_pointer()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .text_ellipsis()
                .when(faint, |d| d.text_color(t.text_faint))
                .child(label.into()),
        )
        .child(icon("chevron", 14., t.text_muted))
}

fn spaces(s: &Session) -> Vec<(SpaceId, String)> {
    s.spaces
        .iter()
        .filter(|sp| !sp.for_contacts)
        .map(|sp| (sp.id, sp.name.clone()))
        .collect()
}

fn chosen_space(s: &Session) -> SpaceId {
    s.import
        .space
        .or(match s.filter {
            crate::app::Filter::Space(sp) => Some(sp),
            _ => None,
        })
        .unwrap_or_else(|| s.vault.home_space())
}

/// Targets offered for a file column: built-in ones, then the vault's custom columns.
fn targets(s: &Session) -> Vec<(Target, String)> {
    let mut v: Vec<(Target, String)> = TARGETS
        .iter()
        .filter(|t| **t != Target::Skip)
        .map(|t| (t.clone(), target_label(t).to_owned()))
        .collect();
    let mut custom: Vec<_> = s
        .columns
        .iter()
        .filter(|c| {
            c.table == Table::Vault && !c.builtin && !matches!(c.kind, ColumnKind::Formula { .. })
        })
        .collect();
    custom.sort_by_key(|c| c.order);
    v.extend(
        custom
            .into_iter()
            .map(|c| (Target::Custom(c.id.clone()), c.name.clone())),
    );
    v.push((Target::Skip, "Skip".into()));
    v
}

fn label_of(s: &Session, t: &Target) -> String {
    match t {
        Target::Custom(id) => s
            .columns
            .iter()
            .find(|c| &c.id == id)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "Custom".into()),
        _ => target_label(t).to_owned(),
    }
}

pub fn review(s: &Session, l: &Loaded) -> Review {
    let outcome = import_csv::build(
        &l.preview,
        &l.mappings,
        chosen_space(s),
        &s.prefs.currency,
        &s.columns,
    );
    let existing = already_there(&outcome.entries, &s.entries);
    Review { outcome, existing }
}

pub fn page(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let st = &s.import;
    match &st.file {
        None => step_file(st, t, cx),
        Some(l) if st.step >= 3 => step_review(s, l, t, cx),
        Some(l) => step_columns(s, l, t, cx),
    }
}

fn step_file(st: &ImportState, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let drop_bg = t.surface_hover;
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(head(t, 1))
        .child(
            div()
                .id("import-drop")
                .py(px(32.))
                .px(px(16.))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(12.))
                .border_1()
                .border_dashed()
                .border_color(t.line_strong)
                .rounded(RADIUS_LG)
                .drag_over::<ExternalPaths>(move |d, _, _, _| d.bg(drop_bg))
                .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                    if let Some(p) = paths.paths().first() {
                        this.load_import(p, cx);
                    }
                }))
                .child(icon("upload", 16., t.text))
                .child(
                    div()
                        .id("import-hint")
                        .text_color(t.text_muted)
                        .child("Drop a CSV file here")
                        .tooltip(tip(
                            "Notion, Excel, Google Sheets, Bitwarden, 1Password, Chrome, Firefox (CSV)",
                        )),
                )
                .child(
                    button("import-choose", t, BtnKind::Default, false)
                        .child("Choose a file")
                        .on_click(cx.listener(|this, _, _, cx| this.choose_import_file(cx)))
                        .test_support(),
                ),
        )
        .when_some(st.error.clone(), |d, e| {
            d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e))
        })
        .child(
            section(t)
                .child(section_title(t, "Export"))
                .child(
                    row(t, true)
                        .opacity(0.45)
                        .child(
                            div()
                                .id("export-label")
                                .flex_1()
                                .child("Encrypted export")
                                .tooltip(tip("Comes with Sync & backups")),
                        )
                        .child(
                            button("export", t, BtnKind::Default, true)
                                .child(icon("download", 14., t.text))
                                .child("Export"),
                        ),
                ),
        )
}

fn tip(
    text: &'static str,
) -> impl Fn(&mut Window, &mut gpui_kit::App) -> gpui_kit::AnyView + 'static {
    move |window, cx| gpui_kit::component::tooltip::Tooltip::new(text).build(window, cx)
}

fn step_columns(s: &Session, l: &Loaded, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let st = &s.import;
    let space_id = chosen_space(s);
    let space_name = spaces(s)
        .into_iter()
        .find(|(id, _)| *id == space_id)
        .map(|(_, n)| n)
        .unwrap_or_default();
    let mut space_list = menu_box(t).top(px(32.)).left_0().w_full();
    for (i, (id, name)) in spaces(s).into_iter().enumerate() {
        space_list = space_list.child(
            menu_item(format!("import-space-{i}"), t, None, name, false)
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(s) = this.session() {
                        s.import.space = Some(id);
                        s.import.space_menu = false;
                    }
                    cx.notify();
                }))
                .test_support(),
        );
    }
    let file_box = div()
        .flex()
        .items_center()
        .gap(px(12.))
        .p(px(16.))
        .border_1()
        .border_dashed()
        .border_color(t.line_strong)
        .rounded(RADIUS_LG)
        .child(icon("file", 16., t.text))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(l.name.clone())
                .child(
                    div()
                        .text_size(TEXT_SM)
                        .text_color(t.text_muted)
                        .child(format!(
                            "{} rows · {} columns",
                            l.preview.rows.len(),
                            l.preview.headers.len()
                        )),
                ),
        )
        .child(
            div().w(px(200.)).child(field(
                t,
                "Into space",
                div()
                    .relative()
                    .child(
                        div()
                            .id("import-space")
                            .child(select_box(t, space_name, false))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(s) = this.session() {
                                    s.import.space_menu = !s.import.space_menu;
                                    s.import.menu = None;
                                }
                                cx.notify();
                            }))
                            .test_support(),
                    )
                    .when(st.space_menu, |d| d.child(deferred(space_list))),
            )),
        )
        .child(
            button("import-change", t, BtnKind::Ghost, true)
                .child("Change")
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(s) = this.session() {
                        s.import = ImportState::default();
                    }
                    cx.notify();
                }))
                .test_support(),
        );

    let cell = || div().flex_1().min_w_0();
    let mut table = section(t).child(
        div()
            .flex()
            .items_center()
            .gap(px(12.))
            .text_size(TEXT_XS)
            .font_weight(W_MEDIUM)
            .text_color(t.text_muted)
            .child(cell().child("File column"))
            .child(div().w(px(24.)))
            .child(cell().child("Cleis column"))
            .child(cell().child("Sample")),
    );
    let all_targets = targets(s);
    for (i, m) in l.mappings.iter().enumerate() {
        let skip = m.target == Target::Skip;
        let mut list = menu_box(t)
            .top(px(32.))
            .left_0()
            .w_full()
            .max_h(px(320.))
            .overflow_hidden();
        for (k, (target, name)) in all_targets.iter().enumerate() {
            let target2 = target.clone();
            let on = *target == m.target;
            list = list.child(
                menu_item(format!("map-{i}-{k}"), t, None, name.clone(), false)
                    .when(on, |d| {
                        d.bg(t.surface_hover).child(div().flex_1()).child(icon(
                            "check",
                            14.,
                            t.accent_text,
                        ))
                    })
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.set_mapping(i, target2.clone(), cx)),
                    )
                    .test_support(),
            );
        }
        table = table.child(
            div()
                .flex()
                .items_center()
                .gap(px(12.))
                .child(
                    cell()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(icon("columns", 14., t.text_muted))
                        .child(
                            div()
                                .min_w_0()
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(m.header.clone()),
                        ),
                )
                .child(div().w(px(24.)).flex().justify_center().child(icon(
                    "arrowR",
                    14.,
                    t.text_faint,
                )))
                .child(
                    cell()
                        .relative()
                        .child(
                            div()
                                .id(SharedString::from(format!("map-{i}")))
                                .child(select_box(t, label_of(s, &m.target), skip))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if let Some(s) = this.session() {
                                        s.import.menu = if s.import.menu == Some(i) {
                                            None
                                        } else {
                                            Some(i)
                                        };
                                        s.import.space_menu = false;
                                    }
                                    cx.notify();
                                }))
                                .test_support(),
                        )
                        .when(st.menu == Some(i), |d| d.child(deferred(list))),
                )
                .child(
                    cell()
                        .text_size(TEXT_SM)
                        .text_color(t.text_muted)
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(m.sample.clone()),
                ),
        );
    }
    let matched = l
        .mappings
        .iter()
        .filter(|m| m.automatic && m.target != Target::Skip)
        .count();
    let skipped = l
        .mappings
        .iter()
        .filter(|m| m.target == Target::Skip)
        .count();
    let has_name = l
        .mappings
        .iter()
        .any(|m| matches!(m.target, Target::Name | Target::Website | Target::Username));
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(head(t, 2))
        .child(file_box)
        .child(table)
        .child(
            div()
                .flex()
                .gap(px(8.))
                .when(matched > 0, |d| {
                    d.child(chip(
                        t,
                        Tone::Auto,
                        format!(
                            "{matched} column{} matched",
                            if matched == 1 { "" } else { "s" }
                        ),
                        false,
                    ))
                })
                .when(skipped > 0, |d| {
                    d.child(chip(t, Tone::Neutral, format!("{skipped} skipped"), false))
                }),
        )
        .child(
            div()
                .flex()
                .justify_end()
                .gap(px(8.))
                .child(
                    button("import-back", t, BtnKind::Ghost, false)
                        .child("Back")
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(s) = this.session() {
                                s.import = ImportState::default();
                            }
                            cx.notify();
                        }))
                        .test_support(),
                )
                .child(
                    button("import-continue", t, BtnKind::Primary, false)
                        .when(!has_name, |d| d.opacity(0.45))
                        .child("Continue")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(s) = this.session()
                                && has_name
                            {
                                s.import.step = 3;
                                s.import.menu = None;
                                s.import.space_menu = false;
                            }
                            cx.notify();
                        }))
                        .test_support(),
                ),
        )
}

fn stat(t: &Tokens, label: &str, value: usize, warn: bool) -> Div {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(
            div()
                .text_size(TEXT_XS)
                .font_weight(W_MEDIUM)
                .text_color(t.text_muted)
                .child(label.to_owned()),
        )
        .child(
            div()
                .text_size(TEXT_XL)
                .font_weight(W_STRONG)
                .when(warn && value > 0, |d| d.text_color(t.warning_text))
                .child(value.to_string()),
        )
}

fn step_review(s: &Session, l: &Loaded, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let st = &s.import;
    let r = review(s, l);
    let existing = r.existing.iter().filter(|e| **e).count();
    let n = r.new_count(st.skip_existing);
    let space_id = chosen_space(s);
    let space_name = spaces(s)
        .into_iter()
        .find(|(id, _)| *id == space_id)
        .map(|(_, n)| n)
        .unwrap_or_default();
    let summary = section(t)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(icon("file", 16., t.text))
                .child(div().flex_1().child(l.name.clone()))
                .child(
                    div()
                        .text_size(TEXT_SM)
                        .text_color(t.text_muted)
                        .child(format!("into {space_name}")),
                ),
        )
        .child(
            div()
                .flex()
                .gap(px(12.))
                .child(stat(t, "New items", n, false))
                .child(stat(t, "Already in the vault", existing, false))
                .child(stat(t, "Warnings", r.outcome.warnings.len(), true)),
        )
        .when(existing > 0, |d| {
            d.child(
                row(t, false)
                    .child(div().flex_1().child("Skip items already in the vault"))
                    .child(
                        switch("import-skip", t, st.skip_existing)
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(s) = this.session() {
                                    s.import.skip_existing = !s.import.skip_existing;
                                }
                                cx.notify();
                            }))
                            .test_support(),
                    ),
            )
        });
    let mut warnings = section(t).child(section_title(t, "Warnings"));
    let mut table = div().flex().flex_col().child(
        div()
            .flex()
            .h(px(30.))
            .items_center()
            .bg(t.bg)
            .border_b_1()
            .border_color(t.line)
            .text_size(TEXT_XS)
            .font_weight(W_MEDIUM)
            .text_color(t.text_muted)
            .child(div().w(px(64.)).px(px(12.)).child("Row"))
            .child(div().flex_1().px(px(12.)).child("Message")),
    );
    for (line, msg) in r.outcome.warnings.iter().take(200) {
        table = table.child(
            div()
                .flex()
                .h(ROW_H)
                .items_center()
                .border_b_1()
                .border_color(t.line)
                .child(
                    div()
                        .w(px(64.))
                        .px(px(12.))
                        .text_right()
                        .text_color(t.text_muted)
                        .child(line.to_string()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .px(px(12.))
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(msg.clone()),
                ),
        );
    }
    warnings = warnings.child(table);
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(head(t, 3))
        .child(summary)
        .when(!r.outcome.warnings.is_empty(), |d| d.child(warnings))
        .child(
            div()
                .flex()
                .justify_end()
                .gap(px(8.))
                .child(
                    button("import-back", t, BtnKind::Ghost, false)
                        .child("Back")
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(s) = this.session() {
                                s.import.step = 2;
                            }
                            cx.notify();
                        }))
                        .test_support(),
                )
                .child(
                    button("import-run", t, BtnKind::Primary, false)
                        .when(n == 0, |d| d.opacity(0.45))
                        .child(format!("Import {n} item{}", if n == 1 { "" } else { "s" }))
                        .on_click(cx.listener(|this, _, _, cx| this.run_import(cx)))
                        .test_support(),
                ),
        )
}

impl CleisApp {
    pub fn choose_import_file(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Import".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = rx.await
                && let Some(p) = paths.into_iter().next()
            {
                this.update(cx, |this, cx| this.load_import(&p, cx)).ok();
            }
        })
        .detach();
    }

    /// Reads a CSV file and opens the Columns step with the guessed mapping.
    pub fn load_import(&mut self, path: &Path, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let result = std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|b| import_csv::preview(&import_csv::decode(&b)).map_err(|e| e.to_string()));
        match result {
            Ok(preview) => {
                let mappings = preview.mappings.clone();
                s.import = ImportState {
                    file: Some(Loaded {
                        name,
                        preview,
                        mappings,
                    }),
                    step: 2,
                    skip_existing: true,
                    ..Default::default()
                };
            }
            Err(e) => {
                s.import.error = Some(format!("{name}: {e}").into());
            }
        }
        cx.notify();
    }

    /// Points a file column at a Cleis column; a column already used elsewhere is skipped there,
    /// so each one receives a single file column.
    pub fn set_mapping(&mut self, row: usize, target: Target, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        s.import.menu = None;
        if let Some(l) = s.import.file.as_mut() {
            if target != Target::Skip {
                for (i, m) in l.mappings.iter_mut().enumerate() {
                    if i != row && m.target == target {
                        m.target = Target::Skip;
                        m.automatic = false;
                    }
                }
            }
            if let Some(m) = l.mappings.get_mut(row) {
                m.target = target;
                m.automatic = false;
            }
        }
        cx.notify();
    }

    pub fn run_import(&mut self, cx: &mut Context<Self>) {
        let Phase::Main(s) = &mut self.phase else {
            return;
        };
        let Some(l) = s.import.file.as_ref() else {
            return;
        };
        let r = review(s, l);
        let entries: Vec<Entry> = r
            .outcome
            .entries
            .into_iter()
            .zip(r.existing)
            .filter(|(_, ex)| !(s.import.skip_existing && *ex))
            .map(|(e, _)| e)
            .collect();
        if entries.is_empty() {
            return;
        }
        let n = entries.len();
        match s.vault.put_entries(&entries) {
            Ok(()) => {
                s.entries.extend(entries);
                s.import = ImportState::default();
                s.tab = Tab::Vault;
                s.filter = crate::app::Filter::All;
                s.refilter();
                self.toast(
                    format!("{n} item{} imported", if n == 1 { "" } else { "s" }),
                    cx,
                );
            }
            Err(e) => {
                s.import.error = Some(format!("Not imported: {e}").into());
                cx.notify();
            }
        }
    }
}

/// Loads a file given on the command line (`--import FILE`, screenshots).
pub fn preload(app: &mut CleisApp, path: PathBuf, cx: &mut Context<CleisApp>) {
    app.load_import(&path, cx);
}
