//! Personal folders (D-62): spaces of the personal vault to sort passwords. "All items" shows
//! every folder together; deleting a folder moves its items to My vault.

use crate::app::{CleisApp, Filter, FolderDialog, Session};
use crate::i18n::tr;
use crate::theme::*;
use crate::ui::*;
use cleis_core::SpaceId;
use cleis_store::{EncryptionMode, SpaceKind};
use gpui_kit::TestSupportExt;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, SharedString, Window, div, px};

fn tip(
    text: &'static str,
) -> impl Fn(&mut Window, &mut gpui_kit::App) -> gpui_kit::AnyView + 'static {
    move |window, cx| {
        gpui_kit::component::tooltip::Tooltip::new(crate::i18n::tr_dynamic(text)).build(window, cx)
    }
}

/// The menu of a folder in the sidebar: rename, delete (not My vault).
pub fn menu(s: &Session, id: SpaceId, t: &Tokens, cx: &mut Context<CleisApp>) -> Div {
    let home = s.vault.home_space() == id;
    let mut m = menu_box(t).top(px(28.)).right_0().w(px(220.)).child(
        menu_item(
            format!("frename-{id}"),
            t,
            Some("text"),
            tr("Rename"),
            false,
        )
        .on_click(cx.listener(move |this, _, window, cx| this.open_folder_rename(id, window, cx)))
        .test_support(),
    );
    if !home {
        m = m.child(
            menu_item(
                format!("fdelete-{id}"),
                t,
                Some("trash"),
                tr("Delete"),
                true,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(s) = this.session() {
                    s.folder_menu = None;
                    s.folder = Some(FolderDialog::Delete(id));
                }
                cx.notify();
            }))
            .test_support(),
        );
    }
    m
}

/// The "⋯" button of a folder row, shown on hover.
pub fn more_button(id: SpaceId, t: &Tokens, cx: &mut Context<CleisApp>) -> impl IntoElement {
    let (muted, text) = (t.text_muted, t.text);
    div()
        .id(SharedString::from(format!("fmore-{id}")))
        .size(CONTROL_SM)
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .text_color(muted)
        .hover(move |s| s.text_color(text))
        .child(icon("more", 16., muted))
        .tooltip(tip(tr("Folder")))
        .on_click(cx.listener(move |this, _, _, cx| {
            if let Some(s) = this.session() {
                s.folder_menu = if s.folder_menu == Some(id) {
                    None
                } else {
                    Some(id)
                };
            }
            cx.stop_propagation();
            cx.notify();
        }))
        .test_support()
}

pub fn dialog(s: &Session, t: &Tokens, cx: &mut Context<CleisApp>) -> Option<Div> {
    let d = s.folder.as_ref()?;
    let name_of = |id: SpaceId| {
        s.spaces
            .iter()
            .find(|x| x.id == id)
            .map(|x| crate::i18n::space(&x.name))
            .unwrap_or_default()
    };
    let (title, body, ok, danger): (String, Div, &str, bool) = match d {
        // Mockups 37a, 37b: a new folder and a renamed one are named in place, in the sidebar.
        FolderDialog::New(_) | FolderDialog::Rename(..) => return None,
        FolderDialog::Delete(id) => {
            let n = s.entries.iter().filter(|e| e.space == *id).count();
            let home = name_of(s.vault.home_space());
            (
                crate::trf!("Delete “{}”?", name_of(*id)),
                div().text_color(t.text_muted).child(if n == 0 {
                    tr("It is empty.").to_owned()
                } else {
                    crate::trf!(
                        "Its {} item{} move to {}.",
                        n,
                        if n == 1 { "" } else { "s" },
                        home
                    )
                }),
                "Delete",
                true,
            )
        }
    };
    let card = dialog_box(t)
        .child(
            div()
                .text_size(TEXT_LG)
                .line_height(px(20.))
                .font_weight(W_MEDIUM)
                .child(crate::i18n::tr_owned(&title)),
        )
        .child(body)
        .child(
            div()
                .pt(px(8.))
                .flex()
                .justify_end()
                .gap(px(8.))
                .child(
                    button("folder-cancel", t, BtnKind::Default, false)
                        .child(tr("Cancel"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(s) = this.session() {
                                s.folder = None;
                            }
                            cx.notify();
                        }))
                        .test_support(),
                )
                .child(
                    button(
                        "folder-ok",
                        t,
                        if danger {
                            BtnKind::Danger
                        } else {
                            BtnKind::Primary
                        },
                        false,
                    )
                    .child(crate::i18n::tr_owned(ok))
                    .on_click(cx.listener(|this, _, _, cx| this.folder_confirm(cx)))
                    .test_support(),
                ),
        );
    Some(
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(t.overlay)
            .child(div().id("folder-dialog").occlude().child(card)),
    )
}

impl CleisApp {
    fn folder_input(
        &mut self,
        value: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui_kit::Entity<InputState> {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(tr("Banking")));
        let value = value.to_owned();
        input.update(cx, |i, cx| {
            i.set_value(value, window, cx);
            i.focus(window, cx);
        });
        // Enter or leaving the field keeps a name; leaving it empty cancels (mockup 37a).
        let sub = cx.subscribe_in(&input, window, |this: &mut CleisApp, _, ev, window, cx| {
            if matches!(ev, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                this.folder_confirm(cx);
                if let Some(s) = this.session()
                    && s.folder.is_none()
                {
                    let f = s.main_focus.clone();
                    window.focus(&f, cx);
                }
            }
        });
        self._subs.push(sub);
        input
    }

    pub fn open_new_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // The row appears under My vault: in an organization's own tab, the personal one opens.
        if self
            .session()
            .is_some_and(|s| matches!(s.scope, crate::app::Scope::Org(_)))
        {
            self.open_tab(
                crate::app::Tab::Vault,
                Some(crate::app::Scope::Personal),
                cx,
            );
        }
        let input = self.folder_input("", window, cx);
        if let Some(s) = self.session() {
            s.folder_menu = None;
            s.folder = Some(FolderDialog::New(input));
        }
        cx.notify();
    }

    /// Esc in a folder named in place: nothing made, nothing renamed.
    pub fn cancel_folder_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(s) = self.session()
            && matches!(
                s.folder,
                Some(FolderDialog::New(_) | FolderDialog::Rename(..))
            )
        {
            s.folder = None;
            let f = s.main_focus.clone();
            window.focus(&f, cx);
        }
        cx.notify();
    }

    pub fn open_folder_rename(&mut self, id: SpaceId, window: &mut Window, cx: &mut Context<Self>) {
        let name = self
            .session()
            .and_then(|s| s.spaces.iter().find(|x| x.id == id).map(|x| x.name.clone()))
            .unwrap_or_default();
        let input = self.folder_input(&name, window, cx);
        input.update(cx, |i, cx| i.select_all(window, cx));
        if let Some(s) = self.session() {
            s.folder_menu = None;
            s.folder = Some(FolderDialog::Rename(id, input));
        }
        cx.notify();
    }

    pub fn folder_confirm(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(d) = s.folder.take() else { return };
        let r = match &d {
            FolderDialog::New(input) | FolderDialog::Rename(_, input) => {
                let name = input.read(cx).value().trim().to_string();
                // Empty: cancelled (a new folder is not made, a renamed one keeps its name).
                if name.is_empty() {
                    cx.notify();
                    return;
                }
                match &d {
                    FolderDialog::New(_) => s
                        .vault
                        .create_space(&name, SpaceKind::Personal, EncryptionMode::EndToEnd, false)
                        .map(|id| {
                            s.filter = Filter::Space(id);
                        }),
                    FolderDialog::Rename(id, _) => {
                        let same = s.spaces.iter().any(|x| x.id == *id && x.name == name);
                        if same {
                            cx.notify();
                            return;
                        }
                        s.vault.rename_space(*id, &name)
                    }
                    FolderDialog::Delete(_) => Ok(()),
                }
            }
            FolderDialog::Delete(id) => {
                let home = s.vault.home_space();
                s.vault.delete_space(*id, home).map(|_| {
                    if s.filter == Filter::Space(*id) {
                        s.filter = Filter::All;
                    }
                })
            }
        };
        match r {
            Ok(()) => {
                let _ = crate::sync::reload(s);
                // Preferences › Vaults shows the personal vault's name: read again.
                s.vault_name = None;
            }
            Err(e) => s.toast = Some(crate::trf!("Not done: {}", e).into()),
        }
        cx.notify();
    }
}

/// The row of a folder named in place (mockups 37a, 37b): the row itself is the field; Esc
/// cancels, Enter or a click elsewhere keeps the name.
pub fn edit_row(
    input: &gpui_kit::Entity<InputState>,
    t: &Tokens,
    cx: &mut Context<CleisApp>,
) -> impl IntoElement {
    div()
        .id("folder-edit")
        .flex()
        .items_center()
        .gap(px(8.))
        .h(CONTROL_MD)
        .pr(px(4.))
        .rounded(RADIUS_MD)
        .bg(t.bg)
        .border_2()
        .border_color(t.focus)
        .pl(px(22.))
        .child(icon("folder", 16., t.text_muted))
        .child(
            div().flex_1().min_w_0().child(
                Input::new(input)
                    .id("folder-name")
                    .appearance(false)
                    .h(px(22.)),
            ),
        )
        // A click anywhere else keeps the name, or cancels when there is none (37a): a click on
        // something that takes no keyboard would not move the focus, so it is caught here.
        .on_mouse_down_out(cx.listener(|this, _, window, cx| {
            this.folder_confirm(cx);
            if let Some(s) = this.session()
                && s.folder.is_none()
            {
                let f = s.main_focus.clone();
                window.focus(&f, cx);
            }
        }))
        .capture_key_down(
            cx.listener(|this, ev: &gpui_kit::KeyDownEvent, window, cx| {
                if ev.keystroke.key == "escape" {
                    cx.stop_propagation();
                    this.cancel_folder_edit(window, cx);
                }
            }),
        )
        .tooltip(tip("Enter or click elsewhere: keep · Esc: cancel"))
        .test_support()
}
