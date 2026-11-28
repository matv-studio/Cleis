//! Signing in with another account (D-113, mockup 50b): an item's "Sign in" field, below its
//! password: Password by default, or a label of one's own (Google, Microsoft, an SSO…) whose
//! coloured chip then takes the password's place in the list and in the inspector. The labels
//! are made, coloured (the charter's colours or one's own, on the wheel), renamed and deleted
//! from the field's menu.

use crate::app::{CleisApp, Confirm, ConfirmAction, Session};
use crate::editor::{EntryEditor, Menu};
use crate::i18n::tr;
use crate::screens::wheel::{self, Wheel, WheelFor};
use crate::theme::*;
use crate::ui::*;
use cleis_core::entry::{SIGN_IN_PRESETS, SIGN_IN_TONES, sign_in_labels};
use cleis_core::{Entry, EntryId, SignIn};
use gpui_kit::component::Sizable;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, Entity, SharedString, Subscription, TestSupportExt, Window, div, px};

/// The menu's state, kept by the item's editor (closed with it).
pub struct SignMenu {
    /// The label whose "…" is open (its colours, Rename, Delete), by name.
    pub edit: Option<String>,
    /// "New label" open: the name box below.
    pub new: bool,
    /// The label `edit` being renamed in its box.
    pub renaming: bool,
    /// The name box (new label or renaming), with its subscription.
    pub name: Option<(Entity<InputState>, Subscription)>,
    /// The colour of the label being made.
    pub color: String,
}

impl Default for SignMenu {
    fn default() -> Self {
        Self {
            edit: None,
            new: false,
            renaming: false,
            name: None,
            color: SIGN_IN_TONES[0].to_owned(),
        }
    }
}

/// The labels offered: one's own (the presets until one is changed), then those found on the
/// items (made by another member of an organization).
pub fn labels(s: &Session) -> Vec<SignIn> {
    sign_in_labels(
        s.prefs.sign_in_labels.as_deref(),
        s.entries.iter().filter_map(|e| e.sign_in.as_ref()),
    )
}

/// The inspector's field: the select, and its menu when open.
pub fn sign_field(
    s: &Session,
    ed: &EntryEditor,
    e: &Entry,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> Div {
    let open = ed.menu == Some(Menu::SignIn);
    let shown: gpui_kit::AnyElement = match &e.sign_in {
        Some(l) => sign_in_chip(t, l).into_any_element(),
        None => div().child(tr("Password")).into_any_element(),
    };
    let select = div()
        .id("sign-in")
        .tooltip(tip(tr("How you sign in to this site")))
        .child(
            value_box(t)
                .cursor_pointer()
                .when(open, |d| d.border_color(t.focus))
                .child(div().flex_1().min_w_0().flex().child(shown))
                .child(icon("chevron", 14., t.text_muted)),
        )
        .on_click(cx.listener(|this, _, _, cx| this.toggle_sign_menu(cx)))
        .test_support();
    let mut d = div().relative().child(select);
    if open {
        d = d.child(popup_below_left(menu(s, ed, e, t, cx), CONTROL_MD));
    }
    field(t, tr("Sign in@@field"), d)
}

/// A row of the menu: the check's place, then what it shows.
fn item(id: impl Into<SharedString>, t: &Tokens) -> gpui_kit::Stateful<Div> {
    div()
        .id(gpui_kit::ElementId::Name(id.into()))
        .h(px(32.))
        .px(px(8.))
        .flex()
        .items_center()
        .gap(px(8.))
        .rounded(RADIUS_SM)
        .cursor_pointer()
        .hover(|d| d.bg(t.surface_hover))
}

fn sep(t: &Tokens) -> Div {
    div().h(px(1.)).my(px(4.)).bg(t.line)
}

/// A colour's swatch: the colour of the label's text, ringed when chosen.
fn swatch(
    id: impl Into<SharedString>,
    fill: gpui_kit::Hsla,
    on: bool,
    t: &Tokens,
) -> gpui_kit::Stateful<Div> {
    div()
        .id(gpui_kit::ElementId::Name(id.into()))
        .size(px(22.))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded_full()
        .cursor_pointer()
        .when(on, |d| d.border(px(1.5)).border_color(t.text_muted))
        .child(div().size(px(16.)).rounded_full().bg(fill))
}

/// The charter's colours, then one's own (the wheel); `pick` gets the colour chosen.
fn swatches(
    prefix: &'static str,
    current: &str,
    target: WheelFor,
    s: &Session,
    t: &Tokens,
    pick: impl Fn(&mut CleisApp, String, &mut Context<CleisApp>) + Clone + 'static,
    cx: &mut Context<CleisApp>,
) -> Div {
    let wheel_here = s.wheel.as_ref().filter(|w| w.target == target);
    let mut row = div()
        .relative()
        .flex()
        .items_center()
        .gap(px(2.))
        .px(px(6.))
        .py(px(4.));
    for tone in SIGN_IN_TONES {
        let fill = match tone {
            "neutral" => t.text_muted,
            _ => label_colors(t, tone).0,
        };
        let p = pick.clone();
        row = row.child(
            swatch(
                format!("{prefix}-{tone}"),
                fill,
                current == tone && wheel_here.is_none(),
                t,
            )
            .tooltip(tip(match tone {
                "set" => "Blue",
                "auto" => "Teal",
                "ok" => "Green",
                "warn" => "Orange",
                "danger" => "Red",
                _ => "Grey",
            }))
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(s) = this.session() {
                    s.wheel = None;
                }
                p(this, tone.to_owned(), cx)
            }))
            .test_support(),
        );
    }
    // One's own colour: shown in its swatch, the wheel live while it is open.
    let own = wheel_here.map(|w| w.rgba()).or_else(|| parse_hex(current));
    let start = parse_hex(current);
    row = row.child(
        div()
            .id(SharedString::from(format!("{prefix}-own")))
            .size(px(22.))
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .rounded_full()
            .cursor_pointer()
            .when(own.is_some() || wheel_here.is_some(), |d| {
                d.border(px(1.5)).border_color(t.text_muted)
            })
            .child(wheel::own_swatch_sized(own, t, 16.))
            .tooltip(tip(tr("Your own color")))
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(s) = this.session() {
                    s.wheel = match s.wheel.take() {
                        Some(w) if w.target == target => None,
                        _ => Some(Wheel::for_target(start, target.clone())),
                    };
                }
                cx.notify();
            }))
            .test_support(),
    );
    if let Some(w) = wheel_here {
        row = row.child(wheel::render_left(w, t, cx));
    }
    row
}

fn menu(
    s: &Session,
    ed: &EntryEditor,
    e: &Entry,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    let id = e.id;
    let st = &ed.sign;
    let check = |on: bool| {
        div()
            .w(px(14.))
            .flex_none()
            .when(on, |d| d.child(icon("check", 14., t.text)))
    };
    let mut m = menu_box(t).w(INSPECTOR_W - px(24.)).child(
        item("sign-password", t)
            .child(check(e.sign_in.is_none()))
            .child(tr("Password"))
            .on_click(cx.listener(move |this, _, _, cx| this.set_sign_in(id, None, cx)))
            .test_support(),
    );
    m = m.child(sep(t));
    for (i, l) in labels(s).into_iter().enumerate() {
        let on = e.sign_in.as_ref().is_some_and(|x| x.same(&l.label));
        let editing = st.edit.as_deref().is_some_and(|n| l.same(n));
        let group = SharedString::from(format!("sign-row-{i}"));
        let name = l.label.clone();
        let chosen = l.clone();
        m =
            m.child(
                item(format!("sign-l-{i}"), t)
                    .group(group.clone())
                    .when(editing, |d| d.bg(t.surface_hover))
                    .child(check(on))
                    .child(sign_in_chip(t, &l))
                    .child(div().flex_1())
                    .child(
                        div()
                            .when(!editing, |d| {
                                d.invisible().group_hover(group.clone(), |s| s.visible())
                            })
                            .child(
                                icon_button(
                                    SharedString::from(format!("sign-more-{i}")),
                                    t,
                                    "more",
                                    true,
                                )
                                .tooltip(tip(tr("Color, rename, delete")))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.edit_sign_label(&name, cx)
                                }))
                                .test_support(),
                            ),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_sign_in(id, Some(chosen.clone()), cx)
                    }))
                    .test_support(),
            );
        if editing {
            m = m.child(edit_panel(s, st, &l, t, cx));
        }
    }
    m = m.child(sep(t));
    if st.new {
        let color = st.color.clone();
        let preview = SignIn::new(
            &st.name
                .as_ref()
                .map(|(i, _)| i.read(cx).value().to_string())
                .unwrap_or_default(),
            &s.wheel
                .as_ref()
                .filter(|w| w.target == WheelFor::NewLabel)
                .map_or(color.clone(), |w| w.hex_text()),
        );
        m = m.child(
            div()
                .id("sign-new-form")
                .flex()
                .flex_col()
                .gap(px(4.))
                .p(px(4.))
                .when_some(st.name.as_ref(), |d, (i, _)| {
                    d.child(Input::new(i).id("sign-new-name").small())
                })
                .child(swatches(
                    "sign-new-sw",
                    &color,
                    WheelFor::NewLabel,
                    s,
                    t,
                    |this, c, cx| {
                        if let Some(ed) = this.session().and_then(|s| s.editor.as_mut()) {
                            ed.sign.color = c;
                        }
                        cx.notify();
                    },
                    cx,
                ))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .px(px(4.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .when(!preview.label.is_empty(), |d| {
                                    d.child(sign_in_chip(t, &preview))
                                }),
                        )
                        .child(
                            button("sign-new-cancel", t, BtnKind::Ghost, true)
                                .child(tr("Cancel"))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(ed) = this.session().and_then(|s| s.editor.as_mut())
                                    {
                                        ed.sign = SignMenu::default();
                                    }
                                    if let Some(s) = this.session() {
                                        s.wheel = None;
                                    }
                                    cx.notify();
                                }))
                                .test_support(),
                        )
                        .child(
                            button("sign-add", t, BtnKind::Primary, true)
                                .child(tr("Add"))
                                .on_click(
                                    cx.listener(move |this, _, _, cx| this.add_sign_label(id, cx)),
                                )
                                .test_support(),
                        ),
                )
                .test_support(),
        );
    } else {
        m = m.child(
            menu_item("sign-new", t, Some("plus"), tr("New label"), false)
                .text_color(t.text_muted)
                .on_click(cx.listener(|this, _, window, cx| this.start_new_sign_label(window, cx)))
                .test_support(),
        );
    }
    menu_in("sign-menu-in", m)
}

/// Under a label whose "…" is open: its colours, Rename (its name in a box), Delete.
fn edit_panel(
    s: &Session,
    st: &SignMenu,
    l: &SignIn,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> Div {
    let name = l.label.clone();
    let n2 = name.clone();
    let n3 = name.clone();
    let mut d = div()
        .flex()
        .flex_col()
        .gap(px(2.))
        .pl(px(22.))
        .pr(px(4.))
        .pb(px(4.));
    if st.renaming
        && let Some((i, _)) = st.name.as_ref()
    {
        d = d.child(
            div()
                .px(px(4.))
                .py(px(2.))
                .child(Input::new(i).id("sign-rename").small()),
        );
    }
    d.child(swatches(
        "sign-sw",
        &l.color,
        WheelFor::Label(name.clone()),
        s,
        t,
        move |this, c, cx| this.recolor_sign_label(&name, &c, cx),
        cx,
    ))
    .when(!st.renaming, |d| {
        d.child(
            menu_item("sign-rename-start", t, Some("text"), tr("Rename"), false)
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.start_rename_sign_label(&n2, window, cx)
                }))
                .test_support(),
        )
    })
    .child(
        menu_item("sign-delete", t, Some("trash"), tr("Delete label"), true)
            .on_click(cx.listener(move |this, _, _, cx| this.ask_delete_sign_label(&n3, cx)))
            .test_support(),
    )
}

impl CleisApp {
    pub fn toggle_sign_menu(&mut self, cx: &mut Context<Self>) {
        if let Some(s) = self.session() {
            s.wheel = None;
            if let Some(ed) = s.editor.as_mut() {
                ed.sign = SignMenu::default();
            }
        }
        self.toggle_menu(Menu::SignIn, cx);
    }

    fn close_sign_menu(&mut self) {
        if let Some(s) = self.session() {
            s.wheel = None;
            if let Some(ed) = s.editor.as_mut() {
                ed.sign = SignMenu::default();
                if ed.menu == Some(Menu::SignIn) {
                    ed.menu = None;
                }
            }
        }
    }

    /// Password (`None`) or a label chosen for the item; the menu closes.
    pub fn set_sign_in(&mut self, id: EntryId, l: Option<SignIn>, cx: &mut Context<Self>) {
        self.close_sign_menu();
        let same = self
            .session()
            .and_then(|s| s.entries.iter().find(|e| e.id == id))
            .is_some_and(|e| e.sign_in == l);
        if !same {
            self.update_entry(id, move |e| e.sign_in = l, cx);
        }
        cx.notify();
    }

    /// One's own list, made from the presets the first time it changes.
    fn own_labels(&mut self) -> Vec<SignIn> {
        self.session()
            .map(|s| {
                s.prefs.sign_in_labels.clone().unwrap_or_else(|| {
                    SIGN_IN_PRESETS
                        .iter()
                        .map(|(n, c)| SignIn::new(n, c))
                        .collect()
                })
            })
            .unwrap_or_default()
    }

    /// Changes every item with label `name` that one may edit; returns how many.
    fn relabel(&mut self, name: &str, to: Option<SignIn>, cx: &mut Context<Self>) -> usize {
        let Some(s) = self.session() else { return 0 };
        let mut n = 0;
        let mut failed = false;
        for i in 0..s.entries.len() {
            let e = &s.entries[i];
            if !e.sign_in.as_ref().is_some_and(|l| l.same(name))
                || !s.may(e.space, cleis_core::org::Perm::Edit)
            {
                continue;
            }
            let e = &mut s.entries[i];
            e.sign_in = to.clone();
            e.touch(None);
            failed |= s.vault.put_entry(e).is_err();
            n += 1;
        }
        if failed {
            s.toast = Some(tr("Not saved").into());
        }
        s.refilter();
        cx.notify();
        n
    }

    fn start_name_box(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let st = cx.new(|cx| InputState::new(window, cx).placeholder(tr("Label name")));
        st.update(cx, |i, cx| {
            i.set_value(text.to_owned(), window, cx);
            i.focus(window, cx);
        });
        let sub = cx.subscribe_in(&st, window, |this, _, ev: &InputEvent, _, cx| match ev {
            InputEvent::PressEnter { .. } => this.name_box_enter(cx),
            InputEvent::Change => cx.notify(),
            _ => {}
        });
        if let Some(ed) = self.session().and_then(|s| s.editor.as_mut()) {
            ed.sign.name = Some((st, sub));
        }
    }

    fn name_box_enter(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(ed) = s.editor.as_ref() else { return };
        let id = ed.entry;
        if ed.sign.new {
            self.add_sign_label(id, cx);
        } else if ed.sign.renaming {
            self.rename_sign_label(cx);
        }
    }

    pub fn start_new_sign_label(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // The next of the charter's colours not yet taken (blue when all are).
        let taken: Vec<String> = self
            .session()
            .map(|s| labels(s).into_iter().map(|l| l.color).collect())
            .unwrap_or_default();
        let color = SIGN_IN_TONES
            .iter()
            .find(|c| !taken.iter().any(|t| t == *c))
            .unwrap_or(&SIGN_IN_TONES[0]);
        if let Some(s) = self.session() {
            s.wheel = None;
            if let Some(ed) = s.editor.as_mut() {
                ed.sign = SignMenu {
                    new: true,
                    color: (*color).to_owned(),
                    ..Default::default()
                };
            }
        }
        self.start_name_box("", window, cx);
        cx.notify();
    }

    /// "Add" (or Enter): the label made, kept in one's list and given to the item. A name
    /// already there takes that label.
    pub fn add_sign_label(&mut self, id: EntryId, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(ed) = s.editor.as_ref() else { return };
        let name = ed
            .sign
            .name
            .as_ref()
            .map(|(i, _)| i.read(cx).value().trim().to_owned())
            .unwrap_or_default();
        if name.is_empty() {
            return;
        }
        // The wheel still open: its colour is the one seen.
        let color = s
            .wheel
            .as_ref()
            .filter(|w| w.target == WheelFor::NewLabel)
            .map_or_else(|| ed.sign.color.clone(), |w| w.hex_text());
        let label = match labels(s).into_iter().find(|l| l.same(&name)) {
            Some(l) => l,
            None => {
                let l = SignIn::new(&name, &color);
                let mut own = self.own_labels();
                own.push(l.clone());
                self.update_prefs(move |p| p.sign_in_labels = Some(own), cx);
                l
            }
        };
        self.set_sign_in(id, Some(label), cx);
    }

    pub fn edit_sign_label(&mut self, name: &str, cx: &mut Context<Self>) {
        if let Some(s) = self.session() {
            s.wheel = None;
            if let Some(ed) = s.editor.as_mut() {
                let open = ed.sign.edit.as_deref().is_some_and(|n| n == name);
                ed.sign = SignMenu {
                    edit: (!open).then(|| name.to_owned()),
                    ..Default::default()
                };
            }
        }
        cx.notify();
    }

    /// A label's colour changed: in one's list and on every item one may edit.
    pub fn recolor_sign_label(&mut self, name: &str, color: &str, cx: &mut Context<Self>) {
        let mut own = self.own_labels();
        let found = self
            .session()
            .and_then(|s| labels(s).into_iter().find(|l| l.same(name)));
        let Some(mut l) = found else { return };
        l.color = color.to_owned();
        match own.iter_mut().find(|o| o.same(name)) {
            Some(o) => o.color = color.to_owned(),
            None => own.push(l.clone()),
        }
        self.update_prefs(move |p| p.sign_in_labels = Some(own), cx);
        self.relabel(name, Some(l), cx);
    }

    pub fn start_rename_sign_label(
        &mut self,
        name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(s) = self.session() {
            s.wheel = None;
            if let Some(ed) = s.editor.as_mut() {
                ed.sign.edit = Some(name.to_owned());
                ed.sign.renaming = true;
            }
        }
        self.start_name_box(name, window, cx);
        cx.notify();
    }

    /// Enter in the rename box: the label's new name, in one's list and on the items. The name
    /// of another label merges the two.
    pub fn rename_sign_label(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(ed) = s.editor.as_ref() else { return };
        let Some(old) = ed.sign.edit.clone() else {
            return;
        };
        let new = ed
            .sign
            .name
            .as_ref()
            .map(|(i, _)| i.read(cx).value().trim().to_owned())
            .unwrap_or_default();
        if new.is_empty() {
            return;
        }
        let all = labels(s);
        let Some(mut l) = all.iter().find(|l| l.same(&old)).cloned() else {
            return;
        };
        let other = all.iter().find(|x| x.same(&new) && !x.same(&old)).cloned();
        let mut own = self.own_labels();
        own.retain(|o| !o.same(&old) || other.is_none());
        let target = match other {
            Some(o) => o,
            None => {
                l.label = new.clone();
                match own.iter_mut().find(|o| o.same(&old)) {
                    Some(o) => *o = l.clone(),
                    None => own.push(l.clone()),
                }
                l
            }
        };
        self.update_prefs(move |p| p.sign_in_labels = Some(own), cx);
        self.relabel(&old, Some(target), cx);
        if let Some(ed) = self.session().and_then(|s| s.editor.as_mut()) {
            ed.sign = SignMenu::default();
        }
        cx.notify();
    }

    /// Delete: asked first when items use the label (they go back to a password).
    pub fn ask_delete_sign_label(&mut self, name: &str, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let n = s
            .entries
            .iter()
            .filter(|e| e.sign_in.as_ref().is_some_and(|l| l.same(name)))
            .count();
        if n == 0 {
            self.delete_sign_label(name, cx);
            return;
        }
        s.label_to_delete = Some(name.to_owned());
        self.close_sign_menu();
        self.ask(
            Confirm {
                title: crate::trf!("Delete the label {}?", name).into(),
                body: if n == 1 {
                    tr("The item that uses it goes back to Password.").to_owned()
                } else {
                    crate::trf!("The {} items that use it go back to Password.", n)
                }
                .into(),
                button: tr("Delete").into(),
                action: ConfirmAction::DeleteSignLabel,
                alt: None,
            },
            cx,
        )
    }

    pub fn delete_sign_label(&mut self, name: &str, cx: &mut Context<Self>) {
        let mut own = self.own_labels();
        own.retain(|o| !o.same(name));
        self.update_prefs(move |p| p.sign_in_labels = Some(own), cx);
        self.relabel(name, None, cx);
        if let Some(ed) = self.session().and_then(|s| s.editor.as_mut()) {
            ed.sign = SignMenu::default();
        }
        cx.notify();
    }
}
