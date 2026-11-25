//! Inline editing in the contact inspector (screen 20), like the vault inspector: each value is a
//! box written after a short pause, on Enter or when it loses the focus; refused values keep a
//! red border and their message.

use crate::app::{CleisApp, Phase, Tab};
use cleis_core::ContactId;
use cleis_core::contact_edit::{self, Applied, Field, List};
use gpui_kit::component::input::{InputEvent, InputState, TextareaState};
use gpui_kit::prelude::*;
use gpui_kit::{Context, Entity, SharedString, Subscription, Task, Window};
use std::collections::HashMap;
use std::time::Duration;

pub struct ContactEditor {
    pub contact: ContactId,
    pub inputs: HashMap<Field, Entity<InputState>>,
    pub notes: Entity<TextareaState>,
    /// New label typed in the inspector (Labels › +).
    pub label: Entity<InputState>,
    pub errors: HashMap<Field, SharedString>,
    pending: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

fn placeholder(f: Field) -> &'static str {
    match f {
        Field::Name => "Name",
        Field::JobTitle => "Job title",
        Field::Company => "Company",
        Field::Email(_) => "name@example.com",
        Field::Phone(_) => "+33 1 99 00 00 00",
        Field::Address(_) => "Street, postcode city, country",
        Field::Url(_) => "example.com",
        Field::Birthday => "Mar 12",
        Field::Notes => "Notes",
    }
}

impl CleisApp {
    /// The contact shown in the inspector: the selection, or the first one listed.
    pub fn shown_contact(&self) -> Option<ContactId> {
        let Phase::Main(s) = &self.phase else {
            return None;
        };
        s.selected_contact
            .filter(|id| s.contacts.iter().any(|c| c.id == *id))
            .or_else(|| crate::screens::contacts::listed(s).first().map(|c| c.id))
    }

    /// Builds the contact boxes (and the search box) when the contact changed.
    pub fn ensure_contact_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let shown = self.shown_contact();
        let Phase::Main(s) = &mut self.phase else {
            return;
        };
        if s.tab != Tab::Contacts {
            return;
        }
        if s.contacts_ui.search.is_none() {
            let st =
                cx.new(|cx| InputState::new(window, cx).placeholder("Search name, company, phone"));
            let sub = cx.subscribe_in(&st, window, |this, input, ev: &InputEvent, _, cx| {
                if let (InputEvent::Change, Phase::Main(s)) = (ev, &mut this.phase) {
                    s.contacts_ui.query = input.read(cx).value().to_string();
                    cx.notify();
                }
            });
            s.contacts_ui.search = Some(st);
            s.contacts_ui.subs.push(sub);
        }
        let Some(id) = shown else {
            s.contacts_ui.editor = None;
            return;
        };
        if s.contacts_ui
            .editor
            .as_ref()
            .is_some_and(|e| e.contact == id)
        {
            return;
        }
        let Some(c) = s.contacts.iter().find(|c| c.id == id) else {
            return;
        };
        let mut fields = vec![
            Field::Name,
            Field::JobTitle,
            Field::Company,
            Field::Birthday,
        ];
        fields.extend((0..c.emails.len()).map(Field::Email));
        fields.extend((0..c.phones.len()).map(Field::Phone));
        fields.extend((0..c.addresses.len()).map(Field::Address));
        fields.extend((0..c.urls.len()).map(Field::Url));
        let values: Vec<(Field, String)> = fields
            .into_iter()
            .map(|f| (f, contact_edit::text(c, f)))
            .collect();
        let notes_text = c.notes.clone();
        let focus = s.contacts_ui.focus_next.take();
        let mut inputs = HashMap::new();
        let mut subs = vec![];
        for (f, v) in values {
            let st = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(placeholder(f))
                    .default_value(v)
            });
            subs.push(
                cx.subscribe_in(&st, window, move |this, _, ev: &InputEvent, _, cx| {
                    this.on_contact_event(f, ev, cx);
                }),
            );
            if focus == Some(f) {
                st.update(cx, |st, cx| st.focus(window, cx));
            }
            inputs.insert(f, st);
        }
        let notes = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(2, 10)
                .placeholder("Notes")
                .default_value(notes_text)
        });
        subs.push(
            cx.subscribe_in(&notes, window, |this, _, ev: &InputEvent, _, cx| {
                this.on_contact_event(Field::Notes, ev, cx);
            }),
        );
        let label = cx.new(|cx| InputState::new(window, cx).placeholder("New label"));
        subs.push(
            cx.subscribe_in(&label, window, |this, input, ev: &InputEvent, _, cx| {
                if let InputEvent::PressEnter { .. } = ev {
                    let text = input.read(cx).value().to_string();
                    this.add_label(&text, cx);
                }
            }),
        );
        if let Phase::Main(s) = &mut self.phase {
            s.contacts_ui.editor = Some(ContactEditor {
                contact: id,
                inputs,
                notes,
                label,
                errors: HashMap::new(),
                pending: None,
                _subs: subs,
            });
        }
    }

    fn on_contact_event(&mut self, f: Field, ev: &InputEvent, cx: &mut Context<Self>) {
        match ev {
            InputEvent::Change => {
                let task = cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(Duration::from_millis(700))
                        .await;
                    this.update(cx, |this, cx| this.commit_contact_field(f, cx))
                        .ok();
                });
                if let Phase::Main(s) = &mut self.phase
                    && let Some(ed) = s.contacts_ui.editor.as_mut()
                {
                    ed.pending = Some(task);
                }
            }
            InputEvent::PressEnter { .. } | InputEvent::Blur => self.commit_contact_field(f, cx),
            InputEvent::Focus => {}
        }
    }

    /// Writes one box of the shown contact.
    pub fn commit_contact_field(&mut self, f: Field, cx: &mut Context<Self>) {
        let Phase::Main(s) = &mut self.phase else {
            return;
        };
        let Some(ed) = s.contacts_ui.editor.as_mut() else {
            return;
        };
        let value = match f {
            Field::Notes => ed.notes.read(cx).value().to_string(),
            _ => match ed.inputs.get(&f) {
                Some(i) => i.read(cx).value().to_string(),
                None => return,
            },
        };
        let id = ed.contact;
        let Some(c) = s.contacts.iter_mut().find(|c| c.id == id) else {
            return;
        };
        match contact_edit::apply(c, f, &value) {
            Ok(Applied::Unchanged) => {
                ed.errors.remove(&f);
            }
            Ok(r) => {
                ed.errors.remove(&f);
                let saved = s.vault.put_contact(c);
                s.toast = Some(match saved {
                    Ok(()) => "Saved".into(),
                    Err(e) => format!("Not saved: {e}").into(),
                });
                if r == Applied::Removed {
                    // Rows moved up: rebuild the boxes.
                    s.contacts_ui.editor = None;
                }
            }
            Err(e) => {
                ed.errors.insert(f, e.to_string().into());
            }
        }
        cx.notify();
    }

    /// Applies a change to the shown contact, saves it and rebuilds the boxes.
    pub fn update_contact(
        &mut self,
        id: ContactId,
        f: impl FnOnce(&mut cleis_core::Contact) -> Option<Field>,
        cx: &mut Context<Self>,
    ) {
        let Phase::Main(s) = &mut self.phase else {
            return;
        };
        let Some(c) = s.contacts.iter_mut().find(|c| c.id == id) else {
            return;
        };
        let focus = f(c);
        c.updated = cleis_core::now();
        if let Err(e) = s.vault.put_contact(c) {
            s.toast = Some(format!("Not saved: {e}").into());
        }
        s.contacts_ui.editor = None;
        s.contacts_ui.focus_next = focus;
        cx.notify();
    }

    /// Adds an empty row to a list and puts the cursor in it.
    pub fn add_contact_row(&mut self, id: ContactId, list: List, cx: &mut Context<Self>) {
        self.update_contact(id, |c| Some(contact_edit::add_row(c, list)), cx);
    }

    pub fn add_label(&mut self, text: &str, cx: &mut Context<Self>) {
        let label = text.trim().to_owned();
        let Some(id) = self.shown_contact() else {
            return;
        };
        if let Phase::Main(s) = &mut self.phase {
            s.contacts_ui.menu = None;
        }
        if label.is_empty() {
            cx.notify();
            return;
        }
        self.update_contact(
            id,
            |c| {
                c.labels.insert(label);
                None
            },
            cx,
        );
    }
}
