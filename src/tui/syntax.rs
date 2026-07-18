//! Lightweight syntax highlighting for the diff view.
//!
//! Uses `syntect` to tokenize a line, then recolors each token into one of the
//! active theme's six syntax buckets — so highlighting always tracks the
//! current TUI theme rather than a syntect color scheme. Falls back to plain
//! text for unknown extensions, binary, or large diffs (perf).

use std::sync::OnceLock;

use ratatui::style::Color;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme as SynTheme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

use crate::tui::app::Side;
use crate::tui::theme::Theme;

fn syntax_set() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// A neutral syntect theme used only to obtain scope classifications; its
/// colors are discarded in favor of the active TUI theme.
fn probe_theme() -> &'static SynTheme {
    static THEME: OnceLock<SynTheme> = OnceLock::new();
    THEME.get_or_init(|| {
        let ts = ThemeSet::load_defaults();
        ts.themes
            .get("base16-ocean.dark")
            .or_else(|| ts.themes.values().next())
            .cloned()
            .expect("syntect ships default themes")
    })
}

/// Which theme syntax bucket a syntect color maps to. We can't read scopes back
/// out of `HighlightLines` cheaply, so we classify by the probe theme's output
/// color into coarse buckets. This is approximate but stable and theme-driven.
#[derive(Clone, Copy)]
enum Bucket {
    Comment,
    Keyword,
    String,
    Number,
    Function,
    Variable,
    Plain,
}

impl Bucket {
    fn color(self, theme: &Theme) -> Color {
        match self {
            Bucket::Comment => theme.syntax_comment,
            Bucket::Keyword => theme.syntax_keyword,
            Bucket::String => theme.syntax_string,
            Bucket::Number => theme.syntax_number,
            Bucket::Function => theme.syntax_function,
            Bucket::Variable => theme.syntax_variable,
            Bucket::Plain => theme.text,
        }
    }
}

/// Classify a base16-ocean foreground color into a theme bucket. The base16
/// palette maps scopes to a fixed set of colors, which we bucket here.
fn bucket_for(c: syntect::highlighting::Color) -> Bucket {
    // base16-ocean.dark accent colors (approximate hues).
    match (c.r, c.g, c.b) {
        (0x65, 0x73, 0x7e) => Bucket::Comment,   // comments (gray)
        (0xb4, 0x8e, 0xad) => Bucket::Keyword,   // keywords (purple)
        (0xa3, 0xbe, 0x8c) => Bucket::String,    // strings (green)
        (0xd0, 0x87, 0x70) => Bucket::Number,    // numbers/constants (orange)
        (0x8f, 0xa1, 0xb3) => Bucket::Function,  // functions (blue)
        (0xbf, 0x61, 0x6a) => Bucket::Variable,  // variables (red)
        _ => Bucket::Plain,
    }
}

/// Per-file highlighter. Cheap to construct; holds a syntax reference or none.
pub struct Highlighter {
    syntax: Option<&'static SyntaxReference>,
}

impl Highlighter {
    /// Resolve a syntax by file extension. `skip` (large/binary diffs) forces
    /// the plain fallback.
    pub fn for_path(path: &str, skip: bool) -> Self {
        if skip {
            return Highlighter { syntax: None };
        }
        let ext = path.rsplit('.').next().unwrap_or("");
        let syntax = syntax_set().find_syntax_by_extension(ext);
        Highlighter { syntax }
    }

    /// Split `content` into `(text, color)` spans for the given theme. Without a
    /// resolved syntax, returns the whole line as one theme-text span.
    pub fn spans(&self, content: &str, theme: &Theme, _side: Side) -> Vec<(String, Color)> {
        let Some(syntax) = self.syntax else {
            return vec![(content.to_string(), theme.text)];
        };
        let mut h = HighlightLines::new(syntax, probe_theme());
        match h.highlight_line(content, syntax_set()) {
            Ok(ranges) => ranges
                .into_iter()
                .map(|(style, text)| (text.to_string(), bucket_for(style.foreground).color(theme)))
                .collect(),
            Err(_) => vec![(content.to_string(), theme.text)],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::theme;

    #[test]
    fn plain_fallback_for_unknown_extension() {
        let hl = Highlighter::for_path("data.unknownext", false);
        let t = theme::by_id("default-dark");
        let spans = hl.spans("hello world", &t, Side::New);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].0, "hello world");
        assert_eq!(spans[0].1, t.text);
    }

    #[test]
    fn skip_forces_plain() {
        let hl = Highlighter::for_path("main.rs", true);
        assert!(hl.syntax.is_none());
    }

    #[test]
    fn rust_is_recognized_and_tokenizes() {
        let hl = Highlighter::for_path("main.rs", false);
        assert!(hl.syntax.is_some());
        let t = theme::by_id("default-dark");
        let spans = hl.spans("let x = 1;", &t, Side::New);
        // At least one span, and joined text preserves the source.
        let joined: String = spans.iter().map(|(s, _)| s.as_str()).collect();
        assert_eq!(joined.trim_end(), "let x = 1;");
    }
}
