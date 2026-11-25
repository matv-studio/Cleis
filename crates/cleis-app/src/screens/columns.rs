//! Columns menu of the vault (screen 02): shown and hidden columns (click to switch, drag to
//! reorder), and the "New column" form (name, type, all spaces).

use crate::app::{CleisApp, Phase, Session};
use crate::i18n::tr;
use crate::theme::*;
use crate::ui::*;
use cleis_core::column::{
    self, ColumnDef, ColumnKind, ColumnScope, NEW_KINDS, Table, add_custom, kind_from,
};
use cleis_core::ids::ColumnId;
use gpui_kit::TestSupportExt;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, Entity, SharedString, Window, div, px};

pub struct ColumnsMenu {
    pub creating: Option<NewColumn>,
    /// Vault or contacts (the same menu serves both tabs).
    pub table: Table,
}

pub struct NewColumn {
    pub name: Entity<InputState>,
    /// Formula, or choice options separated by commas.
    pub extra: Entity<InputState>,
    pub kind: usize,
    pub all_spaces: bool,
    pub error: Option<SharedString>,
}

/// Dragged column (reorder).
#[derive(Clone)]
pub struct DraggedColumn {
    pub id: ColumnId,
    pub name: SharedString,
}

pub struct DragGhost(SharedString);

impl Render for DragGhost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx).clone();
        div()
            .px(px(8.))
            .h(px(28.))
            .flex()
            .items_center()
            .bg(t.surface_raised)
            .border_1()
            .border_color(t.line_strong)
            .rounded(RADIUS_MD)
            .shadow_lg()
            .child(self.0.clone())
    }
}

fn col_icon(c: &ColumnDef) -> &'static str {
    match c.id.0.as_str() {
        "name" => "text",
        "username" => "user",
        "password" => "key",
        "totp" => "totp",
        "subscription" => "list",
        "price" => "euro",
        "licenses" => "hash",
        "yearly" | "yearly_cost" => "sigma",
        "website" | "urls" => "link",
        "renewal" => "calendar",
        "notes" => "note",
        "space" => "folder",
        _ => kind_icon(&c.kind),
    }
}

fn kind_icon(k: &ColumnKind) -> &'static str {
    match k {
        ColumnKind::Text => "text",
        ColumnKind::Secret => "lock",
        ColumnKind::Number { .. } => "hash",
        ColumnKind::Price { .. } => "euro",
        ColumnKind::Date => "calendar",
        ColumnKind::Choice { .. } => "list",
        ColumnKind::Totp => "totp",
        ColumnKind::Formula { .. } => "sigma",
        ColumnKind::Url => "link",
        ColumnKind::Email => "mail",
        ColumnKind::Phone => "phone",
        ColumnKind::Checkbox => "check",
    }
}

const KIND_ICONS: [&str; 9] = [
    "text", "lock", "hash", "euro", "calendar", "list", "totp", "sigma", "link",
];

fn head(t: &Tokens, label: &str) -> Div {
    div()
        .px(px(8.))
        .pt(px(6.))
        .pb(px(2.))
        .text_size(TEXT_XS)
        .font_weight(W_MEDIUM)
        .text_color(t.text_muted)
        .child(crate::i18n::tr_owned(label))
}

fn col_row(c: &ColumnDef, i: usize, t: &Tokens, cx: &mut Context<CleisApp>) -> impl IntoElement {
    let id = c.id.clone();
    let locked = c.id.0 == "name";
    let on = c.visible;
    let dragged = DraggedColumn {
        id: c.id.clone(),
        name: c.name.clone().into(),
    };
    let drop_id = c.id.clone();
    let del_id = c.id.clone();
    div()
        .id(SharedString::from(format!("col-{}", c.id.0)))
        .group("colrow")
        .h(px(28.))
        .px(px(6.))
        .flex()
        .items_center()
        .gap(px(8.))
        .rounded(RADIUS_SM)
        .cursor_pointer()
        .hover(|d| d.bg(t.surface_hover))
        .on_drag(dragged, |d: &DraggedColumn, _, _, cx| {
            let name = d.name.clone();
            cx.new(|_| DragGhost(name))
        })
        .drag_over::<DraggedColumn>(|d, _, _, cx| d.bg(tokens(cx).selection))
        .on_drop(cx.listener(move |this, d: &DraggedColumn, _, cx| {
            this.reorder_column(&d.id, &drop_id, cx);
        }))
        .child(icon("grip", 14., t.text_faint))
        .child(
            div()
                .size(px(16.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(RADIUS_SM)
                .border_1()
                .border_color(if on { t.primary } else { t.line_strong })
                .when(on, |d| d.bg(t.primary))
                .when(on, |d| d.child(icon("check", 12., t.primary_ink))),
        )
        .child(icon(col_icon(c), 14., t.text_muted))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .text_ellipsis()
                .child(crate::i18n::tr_owned(&c.name)),
        )
        .when(locked, |d| d.child(icon("lock", 12., t.text_faint)))
        .when(!c.builtin, |d| {
            d.child(
                icon_button(SharedString::from(format!("col-del-{i}")), t, "trash", true)
                    .tooltip(|window, cx| {
                        gpui_kit::component::tooltip::Tooltip::new(tr("Delete column"))
                            .build(window, cx)
                    })
                    .invisible()
                    .group_hover("colrow", |s| s.visible())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.delete_column(&del_id, cx);
                    }))
                    .test_support(),
            )
        })
        .when(!locked, |d| {
            d.on_click(cx.listener(move |this, _, _, cx| this.toggle_column(&id, cx)))
        })
        .test_support()
}

pub fn menu(s: &Session, m: &ColumnsMenu, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let mut cols: Vec<&ColumnDef> = s.columns.iter().filter(|c| c.table == m.table).collect();
    cols.sort_by_key(|c| c.order);
    let mut list = div()
        .w(px(260.))
        .p(px(4.))
        .flex()
        .flex_col()
        .bg(t.surface_raised)
        .border_1()
        .border_color(t.line_strong)
        .rounded(RADIUS_MD)
        .shadow_lg()
        .child(head(t, tr("Shown")));
    for (i, c) in cols.iter().filter(|c| c.visible).enumerate() {
        list = list.child(col_row(c, i, t, cx));
    }
    list = list
        .child(div().h(px(1.)).my(px(4.)).bg(t.line))
        .child(head(t, tr("Hidden")));
    for (i, c) in cols.iter().filter(|c| !c.visible).enumerate() {
        list = list.child(col_row(c, 100 + i, t, cx));
    }
    list = list.child(div().h(px(1.)).my(px(4.)).bg(t.line)).child(
        div()
            .id("col-new")
            .h(px(28.))
            .px(px(6.))
            .flex()
            .items_center()
            .gap(px(8.))
            .rounded(RADIUS_SM)
            .cursor_pointer()
            .hover(|d| d.bg(t.surface_hover))
            .when(m.creating.is_some(), |d| d.bg(t.surface_hover))
            .child(icon("plus", 14., t.text_muted))
            .child(tr("New column"))
            .on_click(cx.listener(|this, _, window, cx| this.start_new_column(window, cx)))
            .test_support(),
    );
    let mut row = div().flex().items_start().gap(px(8.)).child(list);
    if let Some(n) = &m.creating {
        let mut kinds = div().flex().flex_col().mx(px(-4.));
        for (i, (_, label)) in NEW_KINDS.iter().enumerate() {
            let sel = i == n.kind;
            kinds = kinds.child(
                div()
                    .id(SharedString::from(format!("kind-{label}")))
                    .h(px(28.))
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .rounded(RADIUS_SM)
                    .cursor_pointer()
                    .when(sel, |d| d.bg(t.surface_hover))
                    .hover(|d| d.bg(t.surface_hover))
                    .child(icon(KIND_ICONS[i], 14., t.text_muted))
                    .child(div().flex_1().child(crate::i18n::tr_owned(label)))
                    .when(sel, |d| d.child(icon("check", 14., t.accent_text)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(n) = this.new_column_form() {
                            n.kind = i;
                            n.error = None;
                        }
                        cx.notify();
                    }))
                    .test_support(),
            );
        }
        let key = NEW_KINDS[n.kind].0;
        let extra_label = match key {
            "formula" => Some(tr("Formula")),
            "choice" => Some(tr("Options, separated by commas")),
            _ => None,
        };
        let form = div()
            .w(px(260.))
            .p(px(12.))
            .flex()
            .flex_col()
            .gap(px(12.))
            .bg(t.surface_raised)
            .border_1()
            .border_color(t.line_strong)
            .rounded(RADIUS_MD)
            .shadow_lg()
            .child(field(t, tr("Name"), Input::new(&n.name).id("col-name")))
            .child(field(t, tr("Type"), kinds))
            .when_some(extra_label, |d, l| {
                d.child(field(t, l, Input::new(&n.extra).id("col-extra")))
            })
            .child(
                row_div(t)
                    .child(div().flex_1().child(tr("All spaces")))
                    .child(
                        div()
                            .id("col-all-spaces")
                            .cursor_pointer()
                            .child(switch_visual(t, n.all_spaces))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(n) = this.new_column_form() {
                                    n.all_spaces = !n.all_spaces;
                                }
                                cx.notify();
                            }))
                            .test_support(),
                    ),
            )
            .when_some(
                n.error.clone().map(|e| crate::i18n::tr_dynamic(&e)),
                |d, e| d.child(div().text_size(TEXT_SM).text_color(t.danger_text).child(e)),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        button("col-cancel", t, BtnKind::Ghost, false)
                            .child(tr("Cancel"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(m) =
                                    this.session().and_then(|s| s.columns_menu.as_mut())
                                {
                                    m.creating = None;
                                }
                                cx.notify();
                            }))
                            .test_support(),
                    )
                    .child(
                        button("col-create", t, BtnKind::Primary, false)
                            .child(tr("Create"))
                            .on_click(cx.listener(|this, _, _, cx| this.create_column(cx)))
                            .test_support(),
                    ),
            );
        row = row.child(form);
    }
    div()
        .absolute()
        .occlude()
        .top(px(36.))
        .left_0()
        .child(row)
        .on_mouse_down_out(cx.listener(|this, _, _, cx| {
            if let Some(s) = this.session() {
                s.columns_menu = None;
            }
            cx.notify();
        }))
}

fn row_div(_t: &Tokens) -> Div {
    div().flex().items_center().gap(px(8.)).min_h(px(24.))
}

/// The charter's switch drawn without its own click handler (the row handles it).
fn switch_visual(t: &Tokens, on: bool) -> Div {
    div()
        .w(px(22.))
        .h(px(14.))
        .relative()
        .child(
            div()
                .absolute()
                .top(px(6.))
                .left_0()
                .right_0()
                .h(px(2.))
                .bg(t.line_strong),
        )
        .when(on, |d| {
            d.child(
                div()
                    .absolute()
                    .top(px(5.))
                    .left_0()
                    .right(px(3.))
                    .h(px(4.))
                    .bg(t.primary),
            )
        })
        .child(
            div()
                .absolute()
                .top_0()
                .when(on, |d| d.right_0())
                .when(!on, |d| d.left_0())
                .w(px(3.))
                .h(px(14.))
                .rounded(px(1.))
                .bg(if on { t.primary } else { t.text_muted }),
        )
}

impl CleisApp {
    fn new_column_form(&mut self) -> Option<&mut NewColumn> {
        self.session()
            .and_then(|s| s.columns_menu.as_mut())
            .and_then(|m| m.creating.as_mut())
    }

    pub fn toggle_columns_menu(&mut self, table: Table, cx: &mut Context<Self>) {
        if let Some(s) = self.session() {
            s.columns_menu = match s.columns_menu {
                Some(_) => None,
                None => Some(ColumnsMenu {
                    creating: None,
                    table,
                }),
            };
            s.filter_menu = false;
        }
        cx.notify();
    }

    fn save_columns(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let of = |t: Table| -> Vec<ColumnDef> {
            s.columns.iter().filter(|c| c.table == t).cloned().collect()
        };
        let (vault_cols, contact_cols) = (of(Table::Vault), of(Table::Contacts));
        // The inspectors' boxes follow the custom columns.
        s.editor = None;
        s.contacts_ui.editor = None;
        let r = s
            .vault
            .set_vault_columns(&vault_cols)
            .and_then(|()| s.vault.set_contact_columns(&contact_cols));
        if let Err(e) = r {
            s.toast = Some(crate::trf!("Columns not saved: {}", e).into());
        }
        cx.notify();
    }

    pub fn toggle_column(&mut self, id: &ColumnId, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        if let Some(c) = s.columns.iter_mut().find(|c| &c.id == id) {
            c.visible = !c.visible;
        }
        self.save_columns(cx);
    }

    pub fn reorder_column(&mut self, moved: &ColumnId, onto: &ColumnId, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(table) = s.columns.iter().find(|c| &c.id == moved).map(|c| c.table) else {
            return;
        };
        let mut ordered: Vec<&ColumnDef> = s.columns.iter().filter(|c| c.table == table).collect();
        ordered.sort_by_key(|c| c.order);
        let Some(to) = ordered.iter().position(|c| &c.id == onto) else {
            return;
        };
        // Dropping onto a column of the other group also shows or hides it.
        let target_visible = ordered[to].visible;
        column::reorder(&mut s.columns, moved, to);
        if let Some(c) = s.columns.iter_mut().find(|c| &c.id == moved)
            && c.id.0 != "name"
        {
            c.visible = target_visible;
        }
        self.save_columns(cx);
    }

    pub fn delete_column(&mut self, id: &ColumnId, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        if let Err(e) = column::remove_custom(&mut s.columns, id) {
            s.toast = Some(e.to_string().into());
        }
        self.save_columns(cx);
    }

    pub fn start_new_column(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder(tr("Cancellation date")));
        let extra = cx.new(|cx| InputState::new(window, cx));
        name.update(cx, |st, cx| st.focus(window, cx));
        if let Some(m) = self.session().and_then(|s| s.columns_menu.as_mut()) {
            m.creating = Some(NewColumn {
                name,
                extra,
                kind: 0,
                all_spaces: true,
                error: None,
            });
        }
        cx.notify();
    }

    /// Fills the open form as in the mockup (screenshots only).
    pub fn demo_new_column(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(n) = self.new_column_form() {
            n.kind = 4;
            let name = n.name.clone();
            name.update(cx, |st, cx| {
                st.set_value(tr("Cancellation date"), window, cx)
            });
        }
    }

    fn create_column(&mut self, cx: &mut Context<Self>) {
        let Phase::Main(s) = &mut self.phase else {
            return;
        };
        let Some(table) = s.columns_menu.as_ref().map(|m| m.table) else {
            return;
        };
        let contacts_folder = match &s.contacts_ui.filter {
            crate::screens::contacts::ContactFilter::Folder(f) => Some(*f),
            _ => None,
        };
        let Some(n) = s.columns_menu.as_mut().and_then(|m| m.creating.as_mut()) else {
            return;
        };
        let name = n.name.read(cx).value().to_string();
        let extra = n.extra.read(cx).value().to_string();
        let scope = if n.all_spaces {
            ColumnScope::AllSpaces
        } else if table == Table::Contacts {
            ColumnScope::Space(contacts_folder.unwrap_or_else(|| s.vault.home_space()))
        } else {
            match s.filter {
                crate::app::Filter::Space(sp) => ColumnScope::Space(sp),
                _ => ColumnScope::Space(s.vault.home_space()),
            }
        };
        let result = kind_from(NEW_KINDS[n.kind].0, &extra, &s.prefs.currency)
            .and_then(|k| add_custom(&mut s.columns, table, &name, k, scope));
        match result {
            Ok(_) => {
                if let Some(m) = s.columns_menu.as_mut() {
                    m.creating = None;
                }
                self.save_columns(cx);
            }
            Err(e) => {
                n.error = Some(e.to_string().into());
                cx.notify();
            }
        }
    }
}
