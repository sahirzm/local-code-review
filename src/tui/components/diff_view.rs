use ratatui::{layout::Rect, style::{Color, Style}, text::{Line, Span, Text}, widgets::{Block, Paragraph}, Frame};
use crate::tui::app::App;
use crate::types::FileStatus;

pub fn render_diff_view(frame: &mut Frame, app: &App, area: Rect) {
    if app.files.is_empty() {
        let p = Paragraph::new("No files changed")
            .block(Block::bordered().title(" Diff "))
            .centered();
        frame.render_widget(p, area);
        return;
    }

    let filtered = app.filtered_files();
    let selected_idx = if !filtered.is_empty() {
        let sidx = app.current_file_idx.min(filtered.len() - 1);
        filtered.get(sidx).copied().unwrap_or(0)
    } else {
        0
    };

    let (fc, diff) = &app.files[selected_idx.min(app.files.len().saturating_sub(1))];
    let reviewed = if app.is_reviewed(&fc.path) { " ✓ reviewed " } else { "" };

    let title = format!(
        " {} [{:?}] +{}/-{} {}",
        fc.path, fc.status, fc.additions, fc.deletions, reviewed
    );

    let mut lines: Vec<Line> = Vec::new();

    if let Some(d) = diff {
        for hunk in &d.hunks {
            lines.push(Line::from(Span::styled(
                format!("@@ -{},{} +{},{} @@ {}",
                    hunk.old_start, hunk.old_lines,
                    hunk.new_start, hunk.new_lines,
                    hunk.content),
                Style::default().fg(Color::Cyan),
            )));

            for change in &hunk.changes {
                let (prefix, color) = match change.change_type {
                    crate::types::ChangeType::Insert => ("+", Color::Green),
                    crate::types::ChangeType::Delete => ("-", Color::Red),
                    crate::types::ChangeType::Normal => (" ", Color::Gray),
                };
                lines.push(Line::from(Span::styled(
                    format!("{}{}", prefix, change.content),
                    Style::default().fg(color),
                )));
            }
        }
    } else if fc.status == FileStatus::Deleted {
        lines.push(Line::from("File deleted"));
    } else {
        lines.push(Line::from("Binary or empty diff"));
    }

    let comments_for_file: Vec<String> = app
        .comments
        .iter()
        .filter(|c| c.file_path.as_deref() == Some(&fc.path))
        .map(|c| {
            let line_num = c.start_line.map(|l| format!("L{}", l)).unwrap_or_default();
            let cat = match c.category {
                crate::types::CommentCategory::Fix => "fix",
                crate::types::CommentCategory::Question => "question",
                crate::types::CommentCategory::Suggestion => "suggestion",
                crate::types::CommentCategory::Nit => "nit",
            };
            format!("[{}] {} {}", cat, line_num, c.text)
        })
        .collect();

    if !comments_for_file.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("── Comments ──", Style::default().fg(Color::Yellow))));
        for c in &comments_for_file {
            lines.push(Line::from(c.as_str()));
        }
    }

    let paragraph = Paragraph::new(Text::from(lines))
        .block(Block::bordered().title(title));

    frame.render_widget(paragraph, area);
}
