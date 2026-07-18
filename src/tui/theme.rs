//! TUI color themes, ported from the web CSS-variable palettes
//! (`frontend/src/App.css`). Each theme mirrors the ~24 semantic + 6 syntax
//! tokens the TUI actually uses. Derived diff colors that the web computes with
//! CSS `color-mix()` are pre-computed here via [`blend`], since a terminal has
//! no `color-mix`.

use ratatui::style::Color;

/// The 6 theme ids in web order (`frontend/src/themes.ts`), used for cycling.
pub const ORDER: &[&str] = &[
    "default-dark",
    "catppuccin-mocha",
    "catppuccin-macchiato",
    "catppuccin-frappe",
    "default-light",
    "catppuccin-latte",
];

pub const DEFAULT_THEME: &str = "default-dark";

/// A fully-resolved theme: base semantic colors, the 6 syntax colors, and the
/// derived diff-row colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub bg: Color,
    pub panel: Color,
    pub panel_hover: Color,
    pub border: Color,
    pub text: Color,
    pub text_dim: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub link: Color,
    pub success: Color,
    pub danger: Color,
    pub warning: Color,
    pub on_accent: Color,
    pub selection_bg: Color,
    pub selection_text: Color,

    pub syntax_comment: Color,
    pub syntax_number: Color,
    pub syntax_string: Color,
    pub syntax_keyword: Color,
    pub syntax_function: Color,
    pub syntax_variable: Color,

    // Derived (pre-computed) diff-row backgrounds.
    pub diff_gutter_insert_bg: Color,
    pub diff_gutter_delete_bg: Color,
    pub diff_gutter_selected_bg: Color,
    pub diff_code_insert_bg: Color,
    pub diff_code_delete_bg: Color,
    pub diff_code_insert_edit_bg: Color,
    pub diff_code_delete_edit_bg: Color,
    pub diff_code_selected_bg: Color,
}

const fn rgb(hex: u32) -> Color {
    Color::Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// CSS `color-mix(in srgb, fg pct%, bg)` — a per-channel linear blend of `fg`
/// over `bg` at `pct` percent. Only meaningful for `Color::Rgb` inputs (all our
/// palette entries are); non-RGB inputs are returned unblended.
pub fn blend(fg: Color, pct: u8, bg: Color) -> Color {
    let (Color::Rgb(fr, fg_, fb), Color::Rgb(br, bg_, bb)) = (fg, bg) else {
        return fg;
    };
    let mix = |f: u8, b: u8| -> u8 {
        let p = pct as u32;
        // round(f*p/100 + b*(100-p)/100)
        (((f as u32 * p) + (b as u32 * (100 - p)) + 50) / 100) as u8
    };
    Color::Rgb(mix(fr, br), mix(fg_, bg_), mix(fb, bb))
}

/// Raw base palette (semantic + syntax) before derivation. Field order matches
/// `Theme`'s leading fields plus the 3 colors (success/danger/warning already
/// present) needed to derive diff backgrounds.
struct Base {
    bg: u32,
    panel: u32,
    panel_hover: u32,
    border: u32,
    text: u32,
    text_dim: u32,
    text_muted: u32,
    accent: u32,
    link: u32,
    success: u32,
    danger: u32,
    warning: u32,
    on_accent: u32,
    selection_bg: u32,
    selection_text: u32,
    syntax_comment: u32,
    syntax_number: u32,
    syntax_string: u32,
    syntax_keyword: u32,
    syntax_function: u32,
    syntax_variable: u32,
}

impl Base {
    /// Resolve into a `Theme`, computing derived diff backgrounds at the exact
    /// ratios the web uses (App.css `--diff-*` definitions).
    fn resolve(&self) -> Theme {
        let bg = rgb(self.bg);
        let success = rgb(self.success);
        let danger = rgb(self.danger);
        let warning = rgb(self.warning);
        Theme {
            bg,
            panel: rgb(self.panel),
            panel_hover: rgb(self.panel_hover),
            border: rgb(self.border),
            text: rgb(self.text),
            text_dim: rgb(self.text_dim),
            text_muted: rgb(self.text_muted),
            accent: rgb(self.accent),
            link: rgb(self.link),
            success,
            danger,
            warning,
            on_accent: rgb(self.on_accent),
            selection_bg: rgb(self.selection_bg),
            selection_text: rgb(self.selection_text),
            syntax_comment: rgb(self.syntax_comment),
            syntax_number: rgb(self.syntax_number),
            syntax_string: rgb(self.syntax_string),
            syntax_keyword: rgb(self.syntax_keyword),
            syntax_function: rgb(self.syntax_function),
            syntax_variable: rgb(self.syntax_variable),
            diff_gutter_insert_bg: blend(success, 22, bg),
            diff_gutter_delete_bg: blend(danger, 22, bg),
            diff_gutter_selected_bg: blend(warning, 18, bg),
            diff_code_insert_bg: blend(success, 14, bg),
            diff_code_delete_bg: blend(danger, 14, bg),
            diff_code_insert_edit_bg: blend(success, 30, bg),
            diff_code_delete_edit_bg: blend(danger, 30, bg),
            diff_code_selected_bg: blend(warning, 18, bg),
        }
    }
}

fn base_for(id: &str) -> &'static Base {
    match id {
        "default-light" => &DEFAULT_LIGHT,
        "catppuccin-mocha" => &CATPPUCCIN_MOCHA,
        "catppuccin-macchiato" => &CATPPUCCIN_MACCHIATO,
        "catppuccin-frappe" => &CATPPUCCIN_FRAPPE,
        "catppuccin-latte" => &CATPPUCCIN_LATTE,
        _ => &DEFAULT_DARK,
    }
}

/// Resolve a theme id to a `Theme`, mirroring the web `normalizeThemeId`
/// (legacy `dark`/`light` migration; unknown ids fall back to default-dark).
pub fn by_id(id: &str) -> Theme {
    let canonical = match id {
        "dark" => "default-dark",
        "light" => "default-light",
        other => other,
    };
    base_for(canonical).resolve()
}

/// True when `id` is a recognized theme (after legacy migration).
pub fn is_known(id: &str) -> bool {
    matches!(id, "dark" | "light") || ORDER.contains(&id)
}

// ===== Base palettes (hex from frontend/src/App.css) =====

static DEFAULT_DARK: Base = Base {
    bg: 0x1e1e1e, panel: 0x252525, panel_hover: 0x383838, border: 0x333333,
    text: 0xd4d4d4, text_dim: 0xaaaaaa, text_muted: 0x888888,
    accent: 0x569cd6, link: 0x388bfd, success: 0x3fb950, danger: 0xf85149, warning: 0xd29922,
    on_accent: 0xffffff, selection_bg: 0x264f78, selection_text: 0x8bb9fe,
    syntax_comment: 0x6a9955, syntax_number: 0xb5cea8, syntax_string: 0xce9178,
    syntax_keyword: 0x569cd6, syntax_function: 0xdcdcaa, syntax_variable: 0xd16969,
};

static DEFAULT_LIGHT: Base = Base {
    bg: 0xffffff, panel: 0xf6f8fa, panel_hover: 0xeaeef2, border: 0xd0d7de,
    text: 0x24292f, text_dim: 0x656d76, text_muted: 0x656d76,
    accent: 0x0550ae, link: 0x0969da, success: 0x1a7f37, danger: 0xcf222e, warning: 0x9a6700,
    on_accent: 0xffffff, selection_bg: 0xddf4ff, selection_text: 0x0550ae,
    syntax_comment: 0x6e7781, syntax_number: 0x0550ae, syntax_string: 0x0a3069,
    syntax_keyword: 0xcf222e, syntax_function: 0x8250df, syntax_variable: 0x953800,
};

static CATPPUCCIN_MOCHA: Base = Base {
    bg: 0x1e1e2e, panel: 0x181825, panel_hover: 0x45475a, border: 0x313244,
    text: 0xcdd6f4, text_dim: 0xa6adc8, text_muted: 0xa6adc8,
    accent: 0x89b4fa, link: 0x89b4fa, success: 0xa6e3a1, danger: 0xf38ba8, warning: 0xf9e2af,
    on_accent: 0x1e1e2e, selection_bg: 0x45475a, selection_text: 0x89b4fa,
    syntax_comment: 0x6c7086, syntax_number: 0xfab387, syntax_string: 0xa6e3a1,
    syntax_keyword: 0xcba6f7, syntax_function: 0x89b4fa, syntax_variable: 0xf38ba8,
};

static CATPPUCCIN_MACCHIATO: Base = Base {
    bg: 0x24273a, panel: 0x1e2030, panel_hover: 0x494d64, border: 0x363a4f,
    text: 0xcad3f5, text_dim: 0xa5adcb, text_muted: 0xa5adcb,
    accent: 0x8aadf4, link: 0x8aadf4, success: 0xa6da95, danger: 0xed8796, warning: 0xeed49f,
    on_accent: 0x24273a, selection_bg: 0x494d64, selection_text: 0x8aadf4,
    syntax_comment: 0x6e738d, syntax_number: 0xf5a97f, syntax_string: 0xa6da95,
    syntax_keyword: 0xc6a0f6, syntax_function: 0x8aadf4, syntax_variable: 0xed8796,
};

static CATPPUCCIN_FRAPPE: Base = Base {
    bg: 0x303446, panel: 0x292c3c, panel_hover: 0x51576d, border: 0x414559,
    text: 0xc6d0f5, text_dim: 0xa5adce, text_muted: 0xa5adce,
    accent: 0x8caaee, link: 0x8caaee, success: 0xa6d189, danger: 0xe78284, warning: 0xe5c890,
    on_accent: 0x303446, selection_bg: 0x51576d, selection_text: 0x8caaee,
    syntax_comment: 0x737994, syntax_number: 0xef9f76, syntax_string: 0xa6d189,
    syntax_keyword: 0xca9ee6, syntax_function: 0x8caaee, syntax_variable: 0xe78284,
};

static CATPPUCCIN_LATTE: Base = Base {
    bg: 0xeff1f5, panel: 0xe6e9ef, panel_hover: 0xccd0da, border: 0xccd0da,
    text: 0x4c4f69, text_dim: 0x6c6f85, text_muted: 0x6c6f85,
    accent: 0x1e66f5, link: 0x1e66f5, success: 0x40a02b, danger: 0xd20f39, warning: 0xdf8e1d,
    on_accent: 0xeff1f5, selection_bg: 0xccd0da, selection_text: 0x1e66f5,
    syntax_comment: 0x8c8fa1, syntax_number: 0xfe640b, syntax_string: 0x40a02b,
    syntax_keyword: 0x8839ef, syntax_function: 0x1e66f5, syntax_variable: 0xd20f39,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_matches_css_color_mix() {
        // success #3fb950 at 14% over bg #1e1e1e:
        //   R: round(0x3f*.14 + 0x1e*.86) = round(63*.14 + 30*.86) = round(8.82+25.8)=35 = 0x23
        //   G: round(0xb9*.14 + 0x1e*.86) = round(185*.14 + 30*.86)= round(25.9+25.8)=52 = 0x34
        //   B: round(0x50*.14 + 0x1e*.86) = round(80*.14 + 30*.86) = round(11.2+25.8)=37 = 0x25
        let got = blend(rgb(0x3fb950), 14, rgb(0x1e1e1e));
        assert_eq!(got, Color::Rgb(0x23, 0x34, 0x25));
    }

    #[test]
    fn blend_endpoints() {
        let a = rgb(0x3fb950);
        let b = rgb(0x1e1e1e);
        assert_eq!(blend(a, 100, b), a);
        assert_eq!(blend(a, 0, b), b);
    }

    #[test]
    fn by_id_fallback_for_unknown() {
        assert_eq!(by_id("nonsense"), by_id("default-dark"));
    }

    #[test]
    fn by_id_migrates_legacy_ids() {
        assert_eq!(by_id("dark"), by_id("default-dark"));
        assert_eq!(by_id("light"), by_id("default-light"));
    }

    #[test]
    fn all_six_themes_have_distinct_backgrounds() {
        let bgs: Vec<Color> = ORDER.iter().map(|id| by_id(id).bg).collect();
        for i in 0..bgs.len() {
            for j in (i + 1)..bgs.len() {
                assert_ne!(bgs[i], bgs[j], "themes {} and {} share a bg", ORDER[i], ORDER[j]);
            }
        }
    }

    #[test]
    fn order_has_six_known_themes() {
        assert_eq!(ORDER.len(), 6);
        assert!(ORDER.iter().all(|id| is_known(id)));
    }
}
