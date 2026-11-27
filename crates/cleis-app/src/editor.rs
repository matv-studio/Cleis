//! Inline editing in the vault inspector (screen 01): each field is a text box bound to the
//! selected item; a change is written after a short pause, on Enter, or when the field loses
//! the focus. A value that cannot be read gets a red border and its message, and nothing is
//! written (charter §6, field).

use crate::app::{CleisApp, Phase};
use crate::i18n::tr;
use cleis_core::column::{ColumnDef, ColumnKind, ColumnScope, Table, Value};
use cleis_core::edit::{self, Field};
use cleis_core::{ColumnId, EntryId};
use gpui_kit::component::input::{InputEvent, InputState, TextareaState};
use gpui_kit::prelude::*;
use gpui_kit::{Context, Entity, SharedString, Subscription, Window};
use std::collections::HashMap;

/// Fields shown as single-line boxes, in inspector order.
pub const LINE_FIELDS: [Field; 7] = [
    Field::Name,
    Field::Website,
    Field::Username,
    Field::Password,
    Field::Price,
    Field::Licenses,
    Field::Renewal,
];

pub struct EntryEditor {
    pub entry: EntryId,
    pub inputs: HashMap<Field, Entity<InputState>>,
    pub totp: Entity<InputState>,
    pub notes: Entity<TextareaState>,
    /// Refused values, by field.
    pub errors: HashMap<Field, SharedString>,
    /// Boxes of the custom columns that apply to the item's space, in column order.
    pub custom: Vec<(ColumnDef, Entity<InputState>)>,
    pub custom_errors: HashMap<ColumnId, SharedString>,
    /// 2FA key box open (always open while the item has no key).
    pub editing_totp: bool,
    pub menu: Option<Menu>,
    /// Boxes typed in since their last save (27/11/2026: saved when the box is left, Enter, or
    /// the editing ends; never at each key).
    pub dirty: std::collections::HashSet<Field>,
    pub dirty_custom: std::collections::HashSet<ColumnId>,
    /// Recovery keys shown in clear (D-102; masked by default).
    pub recovery_shown: bool,
    /// The box where a site's codes are pasted (mockup 47g).
    pub recovery_add: Option<Entity<TextareaState>>,
    /// Redraws while codes are pasted (the "Add n" count).
    pub recovery_sub: Option<Subscription>,
    _subs: Vec<Subscription>,
}

/// Small menus of the inspector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Menu {
    More,
    Subscription,
}

/// The text of a box, its price period in the interface language (the parser reads both).
fn shown_text(e: &cleis_core::Entry, f: Field) -> String {
    let v = edit::text(e, f);
    if f != Field::Price {
        return v;
    }
    [" / month", " / quarter", " / year"]
        .into_iter()
        .find(|p| v.ends_with(p))
        .map_or(v.clone(), |p| {
            format!("{}{}", &v[..v.len() - p.len()], tr(p))
        })
}

fn placeholder(f: Field) -> &'static str {
    match f {
        Field::Name => "Name",
        Field::Website => "example.com",
        Field::Username => "Email or username",
        Field::Password => "Password",
        Field::Totp => "Key or otpauth:// link",
        Field::Notes => "Notes",
        Field::Price => "€15",
        Field::Licenses => "1",
        Field::Renewal => "2026-12-03",
    }
}

impl CleisApp {
    /// Saves what was typed in the item's boxes and not saved yet (the editing ends: another
    /// item, another tab, locking, closing).
    pub fn flush_editor(&mut self, cx: &mut Context<Self>) {
        let Some(ed) = self.editor_mut() else { return };
        let fields: Vec<Field> = ed.dirty.iter().copied().collect();
        let custom: Vec<ColumnId> = ed.dirty_custom.iter().cloned().collect();
        for f in fields {
            self.commit_field(f, cx);
        }
        for id in custom {
            self.commit_custom(&id, cx);
        }
    }

    /// Both inspectors (items, contacts).
    pub fn flush_edits(&mut self, cx: &mut Context<Self>) {
        self.flush_editor(cx);
        self.flush_contact_editor(cx);
    }

    /// Builds the boxes for the selected item when the selection changed.
    pub fn ensure_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // The editing of the previous item ends: what was typed is saved first.
        let changed = match &self.phase {
            Phase::Main(s) => s.editor.as_ref().is_some_and(|ed| {
                s.selected != Some(ed.entry) && !ed.dirty.is_empty()
                    || s.selected != Some(ed.entry) && !ed.dirty_custom.is_empty()
            }),
            _ => false,
        };
        if changed {
            self.flush_editor(cx);
        }
        let Phase::Main(s) = &self.phase else { return };
        let Some(e) = s.selected_entry() else {
            if let Phase::Main(s) = &mut self.phase {
                s.editor = None;
            }
            return;
        };
        if s.editor.as_ref().is_some_and(|ed| ed.entry == e.id) {
            return;
        }
        let id = e.id;
        let reveal = s.reveal;
        let values: Vec<(Field, String)> = LINE_FIELDS
            .iter()
            .chain([Field::Totp, Field::Notes].iter())
            .map(|f| (*f, shown_text(e, *f)))
            .collect();
        let has_totp = e.totp.is_some();
        let space = e.space;
        let custom_cols: Vec<(ColumnDef, String)> =
            editable_custom(&s.columns, Table::Vault, space)
                .into_iter()
                .map(|c| {
                    let v = e
                        .cell(&c)
                        .map(|v| field_text(&v, &c.kind))
                        .unwrap_or_default();
                    (c, v)
                })
                .collect();
        let mut inputs = HashMap::new();
        let mut subs = vec![];
        let mut totp = None;
        let mut notes = None;
        for (f, v) in values {
            if f == Field::Notes {
                let st = cx.new(|cx| {
                    TextareaState::new(window, cx)
                        .auto_grow(2, 10)
                        .placeholder(tr(placeholder(f)))
                        .default_value(v)
                });
                subs.push(
                    cx.subscribe_in(&st, window, move |this, _, ev: &InputEvent, _, cx| {
                        this.on_field_event(f, ev, cx);
                    }),
                );
                notes = Some(st);
                continue;
            }
            let masked = f == Field::Password && !reveal;
            let st = cx.new(|cx| {
                InputState::new(window, cx)
                    .masked(masked)
                    .placeholder(tr(placeholder(f)))
                    .default_value(v)
            });
            subs.push(
                cx.subscribe_in(&st, window, move |this, _, ev: &InputEvent, _, cx| {
                    this.on_field_event(f, ev, cx);
                }),
            );
            if f == Field::Totp {
                totp = Some(st);
            } else {
                inputs.insert(f, st);
            }
        }
        let mut custom = vec![];
        for (c, v) in custom_cols {
            let masked = matches!(c.kind, ColumnKind::Secret) && !reveal;
            let st = cx.new(|cx| {
                InputState::new(window, cx)
                    .masked(masked)
                    .placeholder(tr(custom_placeholder(&c.kind)))
                    .default_value(v)
            });
            let cid = c.id.clone();
            subs.push(
                cx.subscribe_in(&st, window, move |this, _, ev: &InputEvent, _, cx| {
                    this.on_custom_event(cid.clone(), ev, cx);
                }),
            );
            custom.push((c, st));
        }
        let (Some(totp), Some(notes)) = (totp, notes) else {
            return;
        };
        if let Phase::Main(s) = &mut self.phase {
            s.editor = Some(EntryEditor {
                entry: id,
                inputs,
                totp,
                notes,
                errors: HashMap::new(),
                custom,
                custom_errors: HashMap::new(),
                editing_totp: !has_totp,
                menu: None,
                dirty: Default::default(),
                dirty_custom: Default::default(),
                recovery_shown: false,
                recovery_add: None,
                recovery_sub: None,
                _subs: subs,
            });
        }
    }

    fn on_field_event(&mut self, f: Field, ev: &InputEvent, cx: &mut Context<Self>) {
        match ev {
            InputEvent::Change => {
                if let Some(ed) = self.editor_mut() {
                    ed.dirty.insert(f);
                }
            }
            InputEvent::PressEnter { .. } | InputEvent::Blur => self.commit_field(f, cx),
            InputEvent::Focus => {}
        }
    }

    fn on_custom_event(&mut self, id: ColumnId, ev: &InputEvent, cx: &mut Context<Self>) {
        match ev {
            InputEvent::Change => {
                if let Some(ed) = self.editor_mut() {
                    ed.dirty_custom.insert(id);
                }
            }
            InputEvent::PressEnter { .. } | InputEvent::Blur => self.commit_custom(&id, cx),
            InputEvent::Focus => {}
        }
    }

    /// Writes one custom field of the selected item from its box.
    pub fn commit_custom(&mut self, id: &ColumnId, cx: &mut Context<Self>) {
        let Phase::Main(s) = &mut self.phase else {
            return;
        };
        if refuse_edit(s) {
            cx.notify();
            return;
        }
        let Some(ed) = s.editor.as_mut() else { return };
        ed.dirty_custom.remove(id);
        let Some((col, st)) = ed.custom.iter().find(|(c, _)| &c.id == id) else {
            return;
        };
        let text = st.read(cx).value().to_string();
        let col = col.clone();
        let entry = ed.entry;
        let Some(e) = s.entries.iter_mut().find(|e| e.id == entry) else {
            return;
        };
        match Value::parse(&col.kind, &text) {
            Ok(v) => {
                ed.custom_errors.remove(id);
                let old = e.fields.get(id).cloned();
                if old == v {
                    cx.notify();
                    return;
                }
                match v {
                    Some(v) => e.fields.insert(id.clone(), v),
                    None => e.fields.remove(id),
                };
                e.touch(None);
                let r = s.vault.put_entry(e);
                s.toast = Some(match r {
                    Ok(()) => tr("Saved").into(),
                    Err(err) => crate::trf!("Not saved: {}", err).into(),
                });
            }
            Err(_) => {
                ed.custom_errors
                    .insert(id.clone(), refused(&col.kind).into());
            }
        }
        cx.notify();
    }

    fn editor_mut(&mut self) -> Option<&mut EntryEditor> {
        match &mut self.phase {
            Phase::Main(s) => s.editor.as_mut(),
            _ => None,
        }
    }

    /// Writes one field of the selected item from its box.
    pub fn commit_field(&mut self, f: Field, cx: &mut Context<Self>) {
        let Phase::Main(s) = &mut self.phase else {
            return;
        };
        if refuse_edit(s) {
            cx.notify();
            return;
        }
        let Some(ed) = s.editor.as_mut() else { return };
        ed.dirty.remove(&f);
        let value = match f {
            Field::Notes => ed.notes.read(cx).value().to_string(),
            Field::Totp => ed.totp.read(cx).value().to_string(),
            _ => match ed.inputs.get(&f) {
                Some(i) => i.read(cx).value().to_string(),
                None => return,
            },
        };
        let id = ed.entry;
        let currency = s.prefs.currency.clone();
        let Some(e) = s.entries.iter_mut().find(|e| e.id == id) else {
            return;
        };
        match edit::apply(e, f, &value, &currency) {
            Ok(changed) => {
                ed.errors.remove(&f);
                if f == Field::Totp && e.totp.is_some() {
                    ed.editing_totp = false;
                }
                if changed {
                    let r = s.vault.put_entry(e);
                    s.toast = Some(match r {
                        Ok(()) => tr("Saved").into(),
                        Err(err) => crate::trf!("Not saved: {}", err).into(),
                    });
                    if matches!(f, Field::Name | Field::Website | Field::Username) {
                        s.refilter();
                    }
                }
            }
            Err(err) => {
                ed.errors
                    .insert(f, crate::i18n::tr_dynamic(&err.to_string()).into());
            }
        }
        cx.notify();
    }

    /// Puts an item's current value back into its box (after a change made elsewhere, such as
    /// a generated password).
    pub fn refresh_field(&mut self, f: Field, window: &mut Window, cx: &mut Context<Self>) {
        let Phase::Main(s) = &mut self.phase else {
            return;
        };
        let Some(ed) = s.editor.as_ref() else { return };
        let Some(e) = s.entries.iter().find(|e| e.id == ed.entry) else {
            return;
        };
        let v = shown_text(e, f);
        if let Some(i) = ed.inputs.get(&f) {
            i.update(cx, |st, cx| st.set_value(v, window, cx));
        }
    }

    /// Shows or hides the password in its box.
    pub fn set_reveal(&mut self, reveal: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Phase::Main(s) = &mut self.phase else {
            return;
        };
        let entry = s.selected_entry().cloned();
        if reveal && entry.as_ref().is_some_and(|e| !s.may_see(e)) {
            s.toast = Some(tr("Your role copies and fills, without showing").into());
            cx.notify();
            return;
        }
        if reveal && let Some(e) = &entry {
            self.org_report(cleis_core::org::Action::Viewed, e, "password", cx);
        }
        let Phase::Main(s) = &mut self.phase else {
            return;
        };
        s.reveal = reveal;
        if let Some(i) = s
            .editor
            .as_ref()
            .and_then(|ed| ed.inputs.get(&Field::Password))
        {
            i.update(cx, |st, cx| st.set_masked(!reveal, window, cx));
        }
        if let Some(ed) = s.editor.as_ref() {
            for (c, i) in &ed.custom {
                if matches!(c.kind, ColumnKind::Secret) {
                    i.update(cx, |st, cx| st.set_masked(!reveal, window, cx));
                }
            }
        }
        cx.notify();
    }
}

/// Custom vault columns that apply to a space and can be typed into (formulas are computed).
pub fn editable_custom(
    cols: &[ColumnDef],
    table: Table,
    space: cleis_core::SpaceId,
) -> Vec<ColumnDef> {
    let mut v: Vec<ColumnDef> = cols
        .iter()
        .filter(|c| c.table == table && !c.builtin)
        .filter(|c| match c.scope {
            ColumnScope::AllSpaces => true,
            ColumnScope::Space(sp) => sp == space,
        })
        .filter(|c| !matches!(c.kind, ColumnKind::Formula { .. }))
        .cloned()
        .collect();
    v.sort_by_key(|c| c.order);
    v
}

/// Text put in a custom field's box: dates stay ISO so they read back unchanged.
pub fn field_text(v: &Value, kind: &ColumnKind) -> String {
    match v {
        Value::Date(d) => d.clone(),
        _ => v.display(kind),
    }
}

pub fn custom_placeholder(kind: &ColumnKind) -> &'static str {
    match kind {
        ColumnKind::Date => "2026-12-03",
        ColumnKind::Number { .. } => "0",
        ColumnKind::Price { .. } => "€15 / month",
        ColumnKind::Url => "example.com",
        ColumnKind::Email => "name@example.com",
        ColumnKind::Checkbox => "Yes or No",
        ColumnKind::Choice { .. } => "One of the options",
        _ => "",
    }
}

pub fn refused(kind: &ColumnKind) -> String {
    match kind {
        ColumnKind::Date => tr("Not a date").into(),
        ColumnKind::Number { .. } => tr("Not a number").into(),
        ColumnKind::Price { .. } => tr("Not a price").into(),
        ColumnKind::Choice { options } => crate::trf!(
            "One of: {}",
            options
                .iter()
                .map(|o| o.label.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => tr("Not accepted").into(),
    }
}

/// The member's role does not allow editing the selected item (audit L-1): nothing is written,
/// the inspector closes and says why.
fn refuse_edit(s: &mut crate::app::Session) -> bool {
    let Some(id) = s.editor.as_ref().map(|e| e.entry) else {
        return false;
    };
    let space = s.entries.iter().find(|e| e.id == id).map(|e| e.space);
    if space.is_some_and(|sp| !s.may(sp, cleis_core::org::Perm::Edit)) {
        s.toast = Some(tr("Your role does not allow editing here").into());
        s.editor = None;
        return true;
    }
    false
}
