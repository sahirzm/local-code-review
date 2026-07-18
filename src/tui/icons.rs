//! Icon glyphs for the TUI, in three vocabularies selectable via config.
//!
//! Ports the file-extension → icon mapping from the web
//! (`frontend/src/utils/file-icon.tsx`) and the UI glyph set used across the
//! React components. Each glyph has a Nerd Font (default), Unicode, and ASCII
//! rendering so the TUI degrades gracefully on terminals without a patched font.

use crate::config::IconMode;

/// Broad file categories, mirroring the lucide icon buckets used on the web.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileIconKind {
    Code,
    Json,
    Config,
    Terminal,
    Database,
    Markup,
    Text,
    Generic,
}

/// Classify a path by extension. Mirrors `EXT_TO_ICON` in file-icon.tsx exactly,
/// falling back to `Generic` for anything unlisted.
pub fn kind_for(path: &str) -> FileIconKind {
    let ext = path
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "py" | "rs" | "go" | "java" | "rb"
        | "php" | "c" | "h" | "cpp" | "hpp" | "cs" | "swift" | "kt" => FileIconKind::Code,
        "json" => FileIconKind::Json,
        "yaml" | "yml" | "toml" | "ini" | "conf" | "env" => FileIconKind::Config,
        "sh" | "bash" | "zsh" | "fish" => FileIconKind::Terminal,
        "sql" => FileIconKind::Database,
        "css" | "scss" | "less" | "html" | "xml" | "svg" => FileIconKind::Markup,
        "md" | "mdx" | "txt" | "rst" => FileIconKind::Text,
        _ => FileIconKind::Generic,
    }
}

/// A UI affordance that needs a glyph. Kept separate from file kinds so the two
/// vocabularies can evolve independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiGlyph {
    Reviewed,
    Comment,
    CategoryFix,
    CategoryQuestion,
    CategorySuggestion,
    CategoryNit,
    Cursor,
    Expanded,
    Collapsed,
    ViewSplit,
    ViewUnified,
    RangeAnchor,
}

/// Resolved glyph vocabulary for the active [`IconMode`].
#[derive(Debug, Clone, Copy)]
pub struct IconSet {
    mode: IconMode,
}

impl IconSet {
    pub fn new(mode: IconMode) -> Self {
        IconSet { mode }
    }

    /// Glyph for a file category.
    pub fn file(&self, kind: FileIconKind) -> &'static str {
        match self.mode {
            IconMode::NerdFont => match kind {
                // Nerd Font (Material/Dev icons) codepoints.
                FileIconKind::Code => "\u{e796}",     // nf-dev-code_badge
                FileIconKind::Json => "\u{e60b}",     // nf-seti-json
                FileIconKind::Config => "\u{e615}",   // nf-seti-config
                FileIconKind::Terminal => "\u{f489}", // nf-oct-terminal
                FileIconKind::Database => "\u{f1c0}", // nf-fa-database
                FileIconKind::Markup => "\u{e736}",   // nf-dev-html5
                FileIconKind::Text => "\u{f0219}",    // nf-md-file_document
                FileIconKind::Generic => "\u{f0214}", // nf-md-file
            },
            IconMode::Unicode => match kind {
                FileIconKind::Code => "◆",
                FileIconKind::Json => "❴",
                FileIconKind::Config => "⚙",
                FileIconKind::Terminal => "❯",
                FileIconKind::Database => "▤",
                FileIconKind::Markup => "◈",
                FileIconKind::Text => "¶",
                FileIconKind::Generic => "▭",
            },
            IconMode::Ascii => match kind {
                FileIconKind::Code => "[]",
                FileIconKind::Json => "{}",
                FileIconKind::Config => "#",
                FileIconKind::Terminal => "$",
                FileIconKind::Database => "=",
                FileIconKind::Markup => "<>",
                FileIconKind::Text => "\"",
                FileIconKind::Generic => "*",
            },
        }
    }

    /// Glyph for a UI affordance.
    pub fn ui(&self, glyph: UiGlyph) -> &'static str {
        match self.mode {
            IconMode::NerdFont => match glyph {
                UiGlyph::Reviewed => "\u{f00c}",           // nf-fa-check
                UiGlyph::Comment => "\u{f075}",            // nf-fa-comment
                UiGlyph::CategoryFix => "\u{f188}",        // nf-fa-bug
                UiGlyph::CategoryQuestion => "\u{f059}",   // nf-fa-question_circle
                UiGlyph::CategorySuggestion => "\u{f0eb}", // nf-fa-lightbulb_o
                UiGlyph::CategoryNit => "\u{f0e5}",        // nf-fa-comment_o
                UiGlyph::Cursor => "\u{f054}",             // nf-fa-chevron_right
                UiGlyph::Expanded => "\u{f078}",           // nf-fa-chevron_down
                UiGlyph::Collapsed => "\u{f054}",          // nf-fa-chevron_right
                UiGlyph::ViewSplit => "\u{f0db}",          // nf-fa-columns
                UiGlyph::ViewUnified => "\u{f039}",        // nf-fa-align_justify
                UiGlyph::RangeAnchor => "\u{f068}",        // nf-fa-minus (range bar)
            },
            IconMode::Unicode => match glyph {
                UiGlyph::Reviewed => "✓",
                UiGlyph::Comment => "💬",
                UiGlyph::CategoryFix => "●",
                UiGlyph::CategoryQuestion => "◆",
                UiGlyph::CategorySuggestion => "◇",
                UiGlyph::CategoryNit => "○",
                UiGlyph::Cursor => "▶",
                UiGlyph::Expanded => "▼",
                UiGlyph::Collapsed => "▶",
                UiGlyph::ViewSplit => "⇔",
                UiGlyph::ViewUnified => "≡",
                UiGlyph::RangeAnchor => "▚",
            },
            IconMode::Ascii => match glyph {
                UiGlyph::Reviewed => "v",
                UiGlyph::Comment => "#",
                UiGlyph::CategoryFix => "!",
                UiGlyph::CategoryQuestion => "?",
                UiGlyph::CategorySuggestion => "+",
                UiGlyph::CategoryNit => "-",
                UiGlyph::Cursor => ">",
                UiGlyph::Expanded => "v",
                UiGlyph::Collapsed => ">",
                UiGlyph::ViewSplit => "||",
                UiGlyph::ViewUnified => "=",
                UiGlyph::RangeAnchor => "|",
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_known_extensions() {
        assert_eq!(kind_for("src/main.rs"), FileIconKind::Code);
        assert_eq!(kind_for("a/b/component.tsx"), FileIconKind::Code);
        assert_eq!(kind_for("package.json"), FileIconKind::Json);
        assert_eq!(kind_for("config.yaml"), FileIconKind::Config);
        assert_eq!(kind_for("deploy.sh"), FileIconKind::Terminal);
        assert_eq!(kind_for("schema.sql"), FileIconKind::Database);
        assert_eq!(kind_for("styles.css"), FileIconKind::Markup);
        assert_eq!(kind_for("README.md"), FileIconKind::Text);
    }

    #[test]
    fn unknown_extension_is_generic() {
        assert_eq!(kind_for("binary.bin"), FileIconKind::Generic);
        assert_eq!(kind_for("noext"), FileIconKind::Generic);
        assert_eq!(kind_for("Makefile"), FileIconKind::Generic);
    }

    #[test]
    fn extension_matching_is_case_insensitive() {
        assert_eq!(kind_for("SRC/Main.RS"), FileIconKind::Code);
        assert_eq!(kind_for("DATA.JSON"), FileIconKind::Json);
    }

    #[test]
    fn dotted_paths_use_last_segment() {
        // A dot in a directory name must not confuse extension detection.
        assert_eq!(kind_for("my.dir/file.py"), FileIconKind::Code);
    }

    #[test]
    fn every_mode_yields_nonempty_glyphs() {
        for mode in [IconMode::NerdFont, IconMode::Unicode, IconMode::Ascii] {
            let set = IconSet::new(mode);
            for kind in [
                FileIconKind::Code,
                FileIconKind::Json,
                FileIconKind::Config,
                FileIconKind::Terminal,
                FileIconKind::Database,
                FileIconKind::Markup,
                FileIconKind::Text,
                FileIconKind::Generic,
            ] {
                assert!(!set.file(kind).is_empty(), "empty file glyph for {:?}/{:?}", mode, kind);
            }
            for g in [
                UiGlyph::Reviewed,
                UiGlyph::Comment,
                UiGlyph::CategoryFix,
                UiGlyph::CategoryQuestion,
                UiGlyph::CategorySuggestion,
                UiGlyph::CategoryNit,
                UiGlyph::Cursor,
                UiGlyph::Expanded,
                UiGlyph::Collapsed,
                UiGlyph::ViewSplit,
                UiGlyph::ViewUnified,
                UiGlyph::RangeAnchor,
            ] {
                assert!(!set.ui(g).is_empty(), "empty ui glyph for {:?}/{:?}", mode, g);
            }
        }
    }
}
