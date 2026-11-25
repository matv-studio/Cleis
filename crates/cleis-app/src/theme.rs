//! Design-charter tokens (design/maquettes/css/tokens.css) as GPUI colours.
//! Keep both files in step: the mockups are the reference.

use cleis_core::settings::{AccentScope, Preferences};
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
    /// Fill of the confirm button of a destructive action (`--danger-solid`).
    pub danger_solid: Hsla,
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
    /// Extended scope: tinted background of active tabs, choices and rows.
    pub accent_soft: Hsla,
    /// Extended scope: text of active tabs, choices and rows.
    pub accent_text: Hsla,
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
            text_muted: c(rgb(0xa3a3a3)),
            text_faint: c(rgb(0x969696)),
            inverse: c(rgb(0xf2f2f2)),
            inverse_ink: c(rgb(0x191919)),
            danger_text: c(rgb(0xff9b92)),
            danger_solid: c(rgb(0xd4382c)),
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
            accent_soft: c(rgb(0x383838)),
            accent_text: text,
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
            danger_solid: c(rgb(0xd4382c)),
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
            accent_soft: c(rgb(0xe6e6e6)),
            accent_text: text,
        }
    }

    pub fn for_mode(mode: Mode) -> Self {
        match mode {
            Mode::Dark => Self::dark(),
            Mode::Light => Self::light(),
        }
    }

    /// Applies an accent following the charter (tokens.css): the accent replaces the primary
    /// action, focus, stage outline and selection; Extended also tints tabs, choices and active
    /// rows; Minimal keeps the primary action in inverse contrast.
    pub fn with_accent(mut self, accent: Option<Rgba>, scope: AccentScope) -> Self {
        let Some(a) = accent else { return self };
        let a = fit_contrast(a, self.bg.into(), MIN_ACCENT_CONTRAST);
        let ah: Hsla = a.into();
        let text: Rgba = self.text.into();
        self.focus = ah;
        self.stage_line = ah;
        if scope != AccentScope::Minimal {
            self.primary = ah;
            self.primary_ink = ink_for(a);
            self.selection = ah.opacity(0.14);
        }
        if scope == AccentScope::Extended {
            self.accent_soft = ah.opacity(0.18);
            self.accent_text = mix(a, text, 0.72).into();
        }
        self
    }
}

/// `color-mix(in srgb, a p%, b)`: interpolation of gamma-encoded sRGB components.
pub fn mix(a: Rgba, b: Rgba, p: f32) -> Rgba {
    Rgba {
        r: a.r * p + b.r * (1. - p),
        g: a.g * p + b.g * (1. - p),
        b: a.b * p + b.b * (1. - p),
        a: 1.,
    }
}

fn linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// WCAG relative luminance.
pub fn luminance(c: Rgba) -> f32 {
    0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
}

/// Black or white, whichever contrasts more with `c` (charter: `--accent-ink`).
pub fn ink_for(c: Rgba) -> Hsla {
    let l = luminance(c);
    let with_white = 1.05 / (l + 0.05);
    let with_black = (l + 0.05) / 0.05;
    if with_white >= with_black {
        gpui_kit::white()
    } else {
        gpui_kit::black()
    }
}

/// Contrast an accent keeps against the page in both themes: above the 3:1 floor of
/// WCAG 1.4.11 so the 72 % text mix and focus rings on raised surfaces still pass (D-51).
pub const MIN_ACCENT_CONTRAST: f32 = 3.6;

/// WCAG contrast ratio.
pub fn contrast(a: Rgba, b: Rgba) -> f32 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// Same hue and saturation, lightness moved away from `bg` until the contrast reaches `min`:
/// a pale folder ochre stays as is on the dark page and deepens on the light one.
pub fn fit_contrast(c: Rgba, bg: Rgba, min: f32) -> Rgba {
    if contrast(c, bg) >= min {
        return c;
    }
    let darker = luminance(bg) > 0.18;
    let mut h: Hsla = c.into();
    for _ in 0..200 {
        h.l = (h.l + if darker { -0.005 } else { 0.005 }).clamp(0., 1.);
        let r: Rgba = h.into();
        if contrast(r, bg) >= min || h.l <= 0. || h.l >= 1. {
            return r;
        }
    }
    h.into()
}

/// `#rrggbb` → colour.
pub fn parse_hex(s: &str) -> Option<Rgba> {
    let h = s.trim().trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    u32::from_str_radix(h, 16).ok().map(rgb)
}

/// Appearance chosen in Preferences (read before unlock from the plain appearance file).
#[derive(Clone, Debug, PartialEq)]
pub struct Appearance {
    pub mode: Mode,
    pub accent: Option<Rgba>,
    pub scope: AccentScope,
}

impl Default for Appearance {
    fn default() -> Self {
        Self::from_prefs(&Preferences::default())
    }
}

impl Appearance {
    pub fn from_prefs(p: &Preferences) -> Self {
        Self {
            mode: if p.theme == cleis_core::settings::Theme::Light {
                Mode::Light
            } else {
                Mode::Dark
            },
            accent: p.accent.as_deref().and_then(parse_hex),
            scope: p.accent_scope,
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
pub fn apply(appearance: &Appearance, cx: &mut App) {
    let mode = appearance.mode;
    let t = Tokens::for_mode(mode).with_accent(appearance.accent, appearance.scope);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ochre_deepens_on_light_only() {
        let a = parse_hex(cleis_core::settings::DEFAULT_ACCENT).unwrap();
        let dark: Rgba = Tokens::for_mode(Mode::Dark).bg.into();
        let light: Rgba = Tokens::for_mode(Mode::Light).bg.into();
        assert_eq!(fit_contrast(a, dark, MIN_ACCENT_CONTRAST), a);
        let deep = fit_contrast(a, light, MIN_ACCENT_CONTRAST);
        assert!(contrast(deep, light) >= MIN_ACCENT_CONTRAST);
        assert!(contrast(deep, light) < MIN_ACCENT_CONTRAST + 0.2);
        // Close to the #a17419 shown in the comparison sent on 25/11/2026.
        let want = parse_hex("#a17419").unwrap();
        assert!((deep.r - want.r).abs() < 0.03 && (deep.g - want.g).abs() < 0.03);
        assert_eq!(ink_for(a), gpui_kit::black());
        assert_eq!(ink_for(deep), gpui_kit::black());
    }

    #[test]
    fn bronze_accent() {
        let a = parse_hex("#a57118").unwrap();
        // Former default bronze: black ink contrasts more than white (charter: ink by luminance).
        assert_eq!(ink_for(a), gpui_kit::black());
        assert_eq!(ink_for(parse_hex("#3b82f6").unwrap()), gpui_kit::black());
        assert_eq!(ink_for(parse_hex("#84580b").unwrap()), gpui_kit::white());
        assert_eq!(ink_for(parse_hex("#f59e0b").unwrap()), gpui_kit::black());
        let t = Tokens::dark().with_accent(Some(a), AccentScope::Extended);
        assert_eq!(t.primary, Hsla::from(a));
        assert_eq!(t.stage_line, Hsla::from(a));
        let m = Tokens::dark().with_accent(Some(a), AccentScope::Minimal);
        assert_eq!(m.primary, Tokens::dark().primary);
        assert_eq!(m.focus, Hsla::from(a));
        // The chosen ink passes WCAG AA (4.5:1) on bronze.
        assert!((luminance(a) + 0.05) / 0.05 > 4.5);
        assert_eq!(parse_hex("nope"), None);
    }
}
