//! Design-charter tokens (design/maquettes/css/tokens.css) as GPUI colours.
//! Keep both files in step: the mockups are the reference.

use gpui_kit::component::theme::{Theme, ThemeMode};
use gpui_kit::{App, Global, Hsla, Pixels, Rgba, px, rgb, rgba};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Dark,
    Light,
}

#[derive(Clone, Debug)]
pub struct Tokens {
    pub mode: Mode,
    pub bg: Hsla,
    pub surface: Hsla,
    pub chrome: Hsla,
    pub stage_line: Hsla,
    pub surface_raised: Hsla,
    pub surface_hover: Hsla,
    pub surface_active: Hsla,
    pub line: Hsla,
    pub line_strong: Hsla,
    pub text: Hsla,
    pub text_muted: Hsla,
    pub text_faint: Hsla,
    pub inverse: Hsla,
    pub inverse_ink: Hsla,
    pub danger_text: Hsla,
    pub danger_bg: Hsla,
    pub success_text: Hsla,
    pub set_text: Hsla,
    pub set_bg: Hsla,
    pub warning_text: Hsla,
    pub warning_bg: Hsla,
    pub auto_text: Hsla,
    pub auto_bg: Hsla,
    pub overlay: Hsla,
    pub range_track_bg: Hsla,
    pub selection: Hsla,
    pub primary: Hsla,
    pub primary_ink: Hsla,
    pub focus: Hsla,
}

impl Global for Tokens {}

fn c(v: Rgba) -> Hsla {
    v.into()
}

impl Tokens {
    pub fn dark() -> Self {
        let text = c(rgb(0xededed));
        Self {
            mode: Mode::Dark,
            bg: c(rgb(0x191919)),
            surface: c(rgb(0x1f1f1f)),
            chrome: c(rgb(0x2a2a2a)),
            stage_line: c(rgb(0x3a3a3a)),
            surface_raised: c(rgb(0x262626)),
            surface_hover: c(rgb(0x2e2e2e)),
            surface_active: c(rgb(0x383838)),
            line: c(rgb(0x2e2e2e)),
            line_strong: c(rgb(0x383838)),
            text,
            text_muted: c(rgb(0x9d9d9d)),
            text_faint: c(rgb(0x8a8a8a)),
            inverse: c(rgb(0xf2f2f2)),
            inverse_ink: c(rgb(0x191919)),
            danger_text: c(rgb(0xff9b92)),
            danger_bg: c(rgb(0x3b2120)),
            success_text: c(rgb(0x7fd6a3)),
            set_text: c(rgb(0x7fb0ff)),
            set_bg: c(rgba(0x4084ff33)),
            warning_text: c(rgb(0xf3aa55)),
            warning_bg: c(rgba(0xf3a04024)),
            auto_text: c(rgb(0x4fc3d1)),
            auto_bg: c(rgba(0x4fc3d129)),
            overlay: c(rgba(0x0000008c)),
            range_track_bg: c(rgb(0x0c0c0c)),
            selection: text.opacity(0.14),
            primary: c(rgb(0xf2f2f2)),
            primary_ink: c(rgb(0x191919)),
            focus: c(rgb(0x9d9d9d)),
        }
    }

    pub fn light() -> Self {
        let text = c(rgb(0x1a1a1a));
        Self {
            mode: Mode::Light,
            bg: c(rgb(0xefefef)),
            surface: c(rgb(0xffffff)),
            chrome: c(rgb(0xe3e3e3)),
            stage_line: c(rgb(0xcfcfcf)),
            surface_raised: c(rgb(0xffffff)),
            surface_hover: c(rgb(0xf7f7f7)),
            surface_active: c(rgb(0xe6e6e6)),
            line: c(rgb(0xe6e6e6)),
            line_strong: c(rgb(0xdadada)),
            text,
            text_muted: c(rgb(0x626262)),
            text_faint: c(rgb(0x767676)),
            inverse: c(rgb(0x1a1a1a)),
            inverse_ink: c(rgb(0xffffff)),
            danger_text: c(rgb(0xb3261e)),
            danger_bg: c(rgb(0xfdeceb)),
            success_text: c(rgb(0x1b7a4b)),
            set_text: c(rgb(0x1f5fd1)),
            set_bg: c(rgba(0x4084ff24)),
            warning_text: c(rgb(0xb25b00)),
            warning_bg: c(rgba(0xf3a04029)),
            auto_text: c(rgb(0x0d7f8c)),
            auto_bg: c(rgba(0x0d7f8c1f)),
            overlay: c(rgba(0x00000047)),
            range_track_bg: c(rgb(0xc9c9c9)),
            selection: text.opacity(0.14),
            primary: c(rgb(0x1a1a1a)),
            primary_ink: c(rgb(0xffffff)),
            focus: c(rgb(0x626262)),
        }
    }

    pub fn for_mode(mode: Mode) -> Self {
        match mode {
            Mode::Dark => Self::dark(),
            Mode::Light => Self::light(),
        }
    }
}

// Sizes from the charter.
pub const TEXT_XS: Pixels = px(11.);
pub const TEXT_SM: Pixels = px(12.);
pub const TEXT_MD: Pixels = px(13.);
pub const TEXT_LG: Pixels = px(15.);
pub const TEXT_XL: Pixels = px(20.);
pub const CONTROL_SM: Pixels = px(24.);
pub const CONTROL_MD: Pixels = px(28.);
pub const RADIUS_SM: Pixels = px(4.);
pub const RADIUS_MD: Pixels = px(6.);
pub const RADIUS_LG: Pixels = px(8.);
pub const TOPBAR_H: Pixels = px(44.);
pub const ROW_H: Pixels = px(32.);
pub const SIDEBAR_W: Pixels = px(220.);
pub const INSPECTOR_W: Pixels = px(280.);
pub const STATUSBAR_H: Pixels = px(28.);

pub const FONT: &str = "Geist";
pub const FONT_MONO: &str = "Geist Mono";

/// Installs the tokens and maps them onto gpui-component's theme so its inputs,
/// scrollbars and popovers follow the charter too.
pub fn apply(mode: Mode, cx: &mut App) {
    let t = Tokens::for_mode(mode);
    Theme::change(
        match mode {
            Mode::Dark => ThemeMode::Dark,
            Mode::Light => ThemeMode::Light,
        },
        None,
        cx,
    );
    let th = Theme::global_mut(cx);
    th.font_family = FONT.into();
    th.font_size = TEXT_MD;
    th.mono_font_family = FONT_MONO.into();
    th.mono_font_size = TEXT_MD;
    th.radius = RADIUS_MD;
    th.radius_lg = RADIUS_LG;
    th.shadow = false;
    let col = &mut th.colors;
    col.background = t.bg;
    col.foreground = t.text;
    col.border = t.line_strong;
    col.input = t.line_strong;
    col.ring = t.focus;
    col.caret = t.text;
    col.selection = t.selection;
    col.muted = t.surface_active;
    col.muted_foreground = t.text_muted;
    col.popover = t.surface_raised;
    col.popover_foreground = t.text;
    col.primary = t.primary;
    col.primary_foreground = t.primary_ink;
    col.primary_hover = t.primary.opacity(0.92);
    col.primary_active = t.primary.opacity(0.85);
    col.accent = t.surface_hover;
    col.accent_foreground = t.text;
    col.list = t.bg;
    col.list_hover = t.surface_hover;
    col.list_active = t.selection;
    col.scrollbar = Hsla::transparent_black();
    col.scrollbar_thumb = t.text_faint;
    col.scrollbar_thumb_hover = t.text_muted;
    col.overlay = t.overlay;
    cx.set_global(t);
}

pub fn tokens(cx: &App) -> &Tokens {
    cx.global::<Tokens>()
}
