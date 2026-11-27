//! Drag and drop (mockups 35c to 35f, 26/11/2026): an item onto a folder, folders and items into
//! the user's order, and the width of a column from the edge of its heading.

use crate::app::{CleisApp, Session};
use crate::theme::*;
use crate::ui::*;
use cleis_core::ColumnId;
use cleis_core::column::{ColumnDef, Table};
use cleis_core::{EntryId, SpaceId};
use gpui_kit::TestSupportExt;
use gpui_kit::prelude::*;
use gpui_kit::{Context, SharedString, Window, div, px};

/// An item of the vault being dragged.
#[derive(Clone)]
pub struct DraggedEntry {
    pub id: EntryId,
    pub name: SharedString,
    /// Its place in the list: dropped further down it goes after the target, else before.
    pub ix: usize,
}

/// A folder or space of the sidebar being dragged.
#[derive(Clone)]
pub struct DraggedSpace {
    pub id: SpaceId,
    pub name: SharedString,
    /// Its place in the sidebar (see `DraggedEntry::ix`).
    pub ix: usize,
}

/// The edge of a column's heading being dragged.
#[derive(Clone)]
pub struct ColResize {
    pub table: Table,
    pub id: ColumnId,
}

/// A column's width while its edge is dragged.
pub struct Resize {
    pub table: Table,
    pub id: ColumnId,
    pub width: f32,
}

/// Where a dragged row or folder will land: a line under the target when it comes from above.
pub fn drop_line(
    d: gpui_kit::StyleRefinement,
    from: usize,
    to: usize,
    line: gpui_kit::Hsla,
) -> gpui_kit::StyleRefinement {
    if from < to {
        d.border_b_2().border_color(line)
    } else {
        d.border_t_2().border_color(line)
    }
}

pub const MIN_COL: f32 = 48.;
pub const MAX_COL: f32 = 640.;

/// What follows the pointer (mockup 35c): the item's letter or a folder, and its name.
pub struct Ghost {
    pub letter: Option<char>,
    pub label: SharedString,
}

impl Render for Ghost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx).clone();
        div()
            .h(px(32.))
            .px(px(10.))
            .flex()
            .items_center()
            .gap(px(8.))
            .bg(t.surface_raised)
            .border_1()
            .border_color(t.line_strong)
            .rounded(RADIUS_MD)
            .shadow_lg()
            .opacity(0.92)
            .text_color(t.text)
            .font_weight(W_MEDIUM)
            .child(match self.letter {
                Some(c) => site_icon(&t, c, 20.).into_any_element(),
                None => icon("folder", 16., t.text_muted).into_any_element(),
            })
            .child(self.label.clone())
    }
}

/// Nothing follows the pointer (a column's edge).
pub struct NoGhost;

impl Render for NoGhost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

pub fn entry_ghost(
    d: &DraggedEntry,
    _: gpui_kit::Point<gpui_kit::Pixels>,
    _: &mut Window,
    cx: &mut gpui_kit::App,
) -> gpui_kit::Entity<Ghost> {
    let letter = d.name.chars().next().unwrap_or('?');
    let label = d.name.clone();
    cx.new(|_| Ghost {
        letter: Some(letter),
        label,
    })
}

pub fn space_ghost(
    d: &DraggedSpace,
    _: gpui_kit::Point<gpui_kit::Pixels>,
    _: &mut Window,
    cx: &mut gpui_kit::App,
) -> gpui_kit::Entity<Ghost> {
    let label = d.name.clone();
    cx.new(|_| Ghost {
        letter: None,
        label,
    })
}

pub fn no_ghost(
    _: &ColResize,
    _: gpui_kit::Point<gpui_kit::Pixels>,
    _: &mut Window,
    cx: &mut gpui_kit::App,
) -> gpui_kit::Entity<NoGhost> {
    cx.new(|_| NoGhost)
}

/// The width of a column: the one being dragged, the one kept, or `default`.
pub fn width(s: &Session, c: &ColumnDef, default: Option<f32>) -> Option<f32> {
    match &s.resize {
        Some(r) if r.table == c.table && r.id == c.id => Some(r.width),
        _ => c.width.map(f32::from).or(default),
    }
}

/// The edge of a heading: drag to resize, double-click to fit (mockup 35e). Mockup 37c: a 1 px
/// grey bar shows where it is, between two headings (`bar`: not after the last one); the accent
/// line takes its place under the pointer.
pub fn col_grip(
    table: Table,
    id: &ColumnId,
    bar: bool,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> impl IntoElement + use<> {
    let drag = ColResize {
        table,
        id: id.clone(),
    };
    let fit = id.clone();
    let line = t.primary;
    let grey = t.line_strong;
    div()
        .id(SharedString::from(format!("colgrip-{}", id.0)))
        .absolute()
        .top_0()
        .bottom_0()
        .right_0()
        .w(px(8.))
        .cursor_col_resize()
        .group("colgrip")
        .when(bar, |d| {
            d.child(
                div()
                    .absolute()
                    .top(px(8.))
                    .bottom(px(8.))
                    .right(px(0.))
                    .w(px(1.))
                    .bg(grey)
                    .group_hover("colgrip", |d| d.invisible()),
            )
        })
        // On the grey bar's own place, at the heading's right edge (not in the middle of the
        // grip, a few pixels to its left).
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .right_0()
                .w(px(2.))
                .invisible()
                .group_hover("colgrip", |d| d.visible())
                .bg(line),
        )
        .tooltip(tip(crate::i18n::tr("Drag to resize · double-click to fit")))
        .on_drag(drag, no_ghost)
        .on_click(cx.listener(move |this, e: &gpui_kit::ClickEvent, _, cx| {
            cx.stop_propagation();
            if e.click_count() >= 2 {
                this.fit_column(table, &fit, cx);
            }
        }))
        .test_support()
}

/// Listener for a heading cell: the dragged edge follows the pointer.
pub fn on_resize_move(
    table: Table,
    id: &ColumnId,
    cx: &mut Context<CleisApp>,
) -> impl Fn(&gpui_kit::DragMoveEvent<ColResize>, &mut Window, &mut gpui_kit::App) + 'static {
    let id = id.clone();
    cx.listener(move |this, e: &gpui_kit::DragMoveEvent<ColResize>, _, cx| {
        let d = e.drag(cx);
        if d.table != table || d.id != id {
            return;
        }
        let w = f32::from(e.event.position.x - e.bounds.left()).clamp(MIN_COL, MAX_COL);
        if let Some(s) = this.session() {
            s.resize = Some(Resize {
                table,
                id: id.clone(),
                width: w.round(),
            });
        }
        cx.notify();
    })
}

/// Rough width of a text in the list (13 px Inter): enough to fit a column.
fn text_width(text: &str, bold: bool) -> f32 {
    let per = if bold { 7.6 } else { 7.1 };
    text.chars().count() as f32 * per
}

impl CleisApp {
    /// The drag ended over the window: the width is kept with the column.
    pub fn end_resize(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(r) = s.resize.take() else { return };
        if let Some(c) = cleis_core::column::find_mut(&mut s.columns, r.table, &r.id) {
            c.width = Some(r.width as u16);
        }
        self.save_columns(cx);
    }

    /// Double-click on the edge: as wide as the longest text shown.
    pub fn fit_column(&mut self, table: Table, id: &ColumnId, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(c) = s
            .columns
            .iter()
            .find(|c| c.table == table && &c.id == id)
            .cloned()
        else {
            return;
        };
        let head = text_width(&crate::i18n::tr_owned(&c.name), true) + 16.;
        let longest = match c.table {
            cleis_core::column::Table::Vault => s
                .visible
                .iter()
                .map(|i| {
                    let e = &s.entries[*i];
                    match c.id.0.as_str() {
                        "name" => text_width(&e.name, true) + 28.,
                        "password" => 70.,
                        _ => e
                            .cell(&c)
                            .map_or(0., |v| text_width(&v.display(&c.kind), false)),
                    }
                })
                .fold(0., f32::max),
            cleis_core::column::Table::Contacts => s
                .contacts
                .iter()
                .map(|x| {
                    x.cell(&c)
                        .map_or(0., |v| text_width(&v.display(&c.kind), false))
                })
                .fold(0., f32::max),
        };
        let w = (head.max(longest) + 24.).clamp(MIN_COL, MAX_COL).round();
        if let Some(c) = cleis_core::column::find_mut(&mut s.columns, table, id) {
            c.width = Some(w as u16);
        }
        self.save_columns(cx);
    }

    /// An item dropped on another one (My order, mockup 35d): it takes that place.
    pub fn reorder_entry(&mut self, moved: EntryId, onto: EntryId, cx: &mut Context<Self>) {
        use cleis_core::settings::SortBy;
        if moved == onto {
            return;
        }
        if self
            .session()
            .is_some_and(|s| s.prefs.sort_by != SortBy::Manual)
        {
            self.set_sort(SortBy::Manual, cx);
        }
        let Some(s) = self.session() else { return };
        let mut order = s.prefs.item_order.clone();
        // Items never placed join at the end, in the order shown.
        for i in &s.visible {
            let id = s.entries[*i].id;
            if !order.contains(&id) {
                order.push(id);
            }
        }
        let from = order.iter().position(|x| *x == moved);
        let to = order.iter().position(|x| *x == onto);
        let (Some(from), Some(mut to)) = (from, to) else {
            return;
        };
        let id = order.remove(from);
        // Dropped on an item further down, it goes after it.
        if from < to {
            to = (to).min(order.len());
            order.insert(to, id);
        } else {
            order.insert(to, id);
        }
        self.update_prefs(|p| p.item_order = order, cx);
        if let Some(s) = self.session() {
            s.refilter();
        }
        cx.notify();
    }

    /// A folder dropped on another of the same vault (mockup 35d): it takes that place.
    pub fn reorder_space(&mut self, moved: SpaceId, onto: SpaceId, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        if moved == onto {
            return;
        }
        let group = |id: SpaceId| {
            let kind = s.spaces.iter().find(|x| x.id == id).map(|x| x.kind);
            (kind, s.org_of(id).map(|o| o.m.key()))
        };
        if group(moved) != group(onto) {
            return;
        }
        let mut order: Vec<SpaceId> =
            super::sidebar::ordered(s, s.spaces.iter().filter(|x| !x.for_contacts))
                .iter()
                .map(|x| x.id)
                .collect();
        let (Some(from), Some(to)) = (
            order.iter().position(|x| *x == moved),
            order.iter().position(|x| *x == onto),
        ) else {
            return;
        };
        let id = order.remove(from);
        order.insert(to.min(order.len()), id);
        self.update_prefs(|p| p.space_order = order, cx);
        cx.notify();
    }
}
