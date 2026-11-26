//! Personal folders (D-62): spaces of the personal vault to sort passwords. "All items" shows
//! every folder together; deleting a folder moves its items to My vault.

use crate::app::{CleisApp, Filter, FolderDialog, Session};
use crate::i18n::tr;
use crate::theme::*;
use crate::ui::*;
use cleis_core::SpaceId;
use cleis_store::{EncryptionMode, SpaceKind};
use gpui_kit::TestSupportExt;
use gpui_kit::component::input::{Input, InputEvent, InputState};
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
        FolderDialog::New(input) => (
            tr("New folder").into(),
            field(t, tr("Name"), Input::new(input).id("folder-name")),
            "Create",
            false,
        ),
        FolderDialog::Rename(id, input) => (
            crate::trf!("Rename “{}”", name_of(*id)),
            field(t, tr("Name"), Input::new(input).id("folder-name")),
            "Rename",
            false,
        ),
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
        let sub = cx.subscribe_in(&input, window, |this: &mut CleisApp, _, ev, _, cx| {
            if let InputEvent::PressEnter { .. } = ev {
                this.folder_confirm(cx);
            }
        });
        self._subs.push(sub);
        input
    }

    pub fn open_new_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = self.folder_input("", window, cx);
        if let Some(s) = self.session() {
            s.folder = Some(FolderDialog::New(input));
        }
        cx.notify();
    }

    pub fn open_folder_rename(&mut self, id: SpaceId, window: &mut Window, cx: &mut Context<Self>) {
        let name = self
            .session()
            .and_then(|s| s.spaces.iter().find(|x| x.id == id).map(|x| x.name.clone()))
            .unwrap_or_default();
        let input = self.folder_input(&name, window, cx);
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
                if name.is_empty() {
                    s.folder = Some(d);
                    return;
                }
                match &d {
                    FolderDialog::New(_) => s
                        .vault
                        .create_space(&name, SpaceKind::Personal, EncryptionMode::EndToEnd, false)
                        .map(|id| {
                            s.filter = Filter::Space(id);
                        }),
                    FolderDialog::Rename(id, _) => s.vault.rename_space(*id, &name),
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
