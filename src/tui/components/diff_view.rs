use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::tui::app::{App, DiffRow};
use crate::tui::icons::UiGlyph;
use crate::tui::syntax;
use crate::types::{ChangeType, FileStatus};

pub fn render_diff_view(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();

    if app.files.is_empty() {
        let p = Paragraph::new("No files changed")
            .block(Block::bordered().title(" Diff ").border_style(Style::default().fg(theme.border)))
            .style(Style::default().fg(theme.text_muted));
        frame.render_widget(p, area);
        return;
    }

    let Some(idx) = app.selected_file_index() else {
        return;
    };
    let (fc, diff) = &app.files[idx];
    let reviewed = if app.is_reviewed(&fc.path) {
        format!(" {} ", app.icons.ui(UiGlyph::Reviewed))
    } else {
        String::new()
    };
    let title = format!(
        " {} [{:?}] +{}/-{}{} ",
        fc.path, fc.status, fc.additions, fc.deletions, reviewed
    );

    let block = Block::bordered()
        .title(title)
        .border_style(Style::default().fg(theme.border));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if diff.is_none() {
        let msg = if fc.status == FileStatus::Deleted {
            "File deleted"
        } else {
            "Binary or empty diff"
        };
        frame.render_widget(
            Paragraph::new(msg).style(Style::default().fg(theme.text_muted)),
            inner,
        );
        return;
    }

    let rows = app.diff_rows();
    let highlighter = syntax::Highlighter::for_path(&fc.path, diff.as_ref().map_or(false, |d| d.is_large));
    let viewport = inner.height as usize;

    // Keep the cursor within the visible window.
    let mut offset = app.scroll_offset;
    if app.diff_cursor < offset {
        offset = app.diff_cursor;
    } else if viewport > 0 && app.diff_cursor >= offset + viewport {
        offset = app.diff_cursor + 1 - viewport;
    }

    let mut lines: Vec<Line> = Vec::new();
    for (i, row) in rows.iter().enumerate().skip(offset).take(viewport) {
        let is_cursor = i == app.diff_cursor;
        lines.push(render_row(app, row, is_cursor, &highlighter));
    }

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn render_row<'a>(app: &App, row: &'a DiffRow, is_cursor: bool, hl: &syntax::Highlighter) -> Line<'a> {
    let theme = app.theme();
    match row {
        DiffRow::HunkHeader(content) => Line::from(Span::styled(
            content.clone(),
            Style::default().fg(theme.accent).add_modifier(Modifier::DIM),
        )),
        DiffRow::Blank => Line::from(""),
        DiffRow::Change {
            change_type,
            content,
            side,
            ..
        } => {
            let (prefix, base_bg) = match change_type {
                ChangeType::Insert => ("+", Some(theme.diff_code_insert_bg)),
                ChangeType::Delete => ("-", Some(theme.diff_code_delete_bg)),
                ChangeType::Normal => (" ", None),
            };
            let bg = if is_cursor {
                Some(theme.diff_code_selected_bg)
            } else {
                base_bg
            };
            let cursor_glyph = if is_cursor {
                app.icons.ui(UiGlyph::Cursor)
            } else {
                " "
            };

            let mut spans: Vec<Span> = Vec::new();
            let mut style = Style::default().fg(theme.text_muted);
            if let Some(bg) = bg {
                style = style.bg(bg);
            }
            spans.push(Span::styled(format!("{}{} ", cursor_glyph, prefix), style));

            // Syntax-highlight the code portion, tinting spans by scope but
            // preserving the diff-row background.
            let base = Style::default().fg(theme.text);
            let base = if let Some(bg) = bg { base.bg(bg) } else { base };
            for (text, color) in hl.spans(content, &theme, *side) {
                let mut s = base.fg(color);
                if let Some(bg) = bg {
                    s = s.bg(bg);
                }
                spans.push(Span::styled(text, s));
            }
            Line::from(spans)
        }
    }
}
