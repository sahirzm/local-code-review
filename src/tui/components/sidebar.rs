use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState},
    Frame,
};

use crate::tui::app::App;
use crate::tui::icons::{kind_for, UiGlyph};
use crate::types::FileStatus;

pub fn render_sidebar(frame: &mut Frame, app: &mut App, area: Rect, list_state: &mut ListState) {
    let theme = app.theme();
    let filtered = app.filtered_files();

    let items: Vec<ListItem> = filtered
        .iter()
        .map(|&idx| {
            let (fc, _) = &app.files[idx];
            let status_color = status_color(&fc.status, &app);
            let file_icon = app.icons.file(kind_for(&fc.path));
            let reviewed = if app.is_reviewed(&fc.path) {
                format!(" {}", app.icons.ui(UiGlyph::Reviewed))
            } else {
                String::new()
            };
            let count = app.comment_count(&fc.path);
            let count_display = if count > 0 { format!(" [{}]", count) } else { String::new() };

            let line = Line::from(vec![
                Span::styled(format!("{} ", status_symbol(&fc.status)), Style::default().fg(status_color)),
                Span::styled(format!("{} ", file_icon), Style::default().fg(theme.text_muted)),
                Span::styled(fc.path.clone(), Style::default().fg(theme.text)),
                Span::styled(reviewed, Style::default().fg(theme.success)),
                Span::styled(count_display, Style::default().fg(theme.accent)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let filter_text = if let Some(ref s) = app.filter_status {
        format!(" [filter: {:?}]", s)
    } else if !app.search_query.is_empty() {
        format!(" [search: {}]", app.search_query)
    } else {
        String::new()
    };
    let title = format!(" Files ({}/{}){} ", app.reviewed_files.len(), app.files.len(), filter_text);

    let list = List::new(items)
        .block(
            Block::bordered()
                .title(title)
                .border_style(Style::default().fg(theme.border)),
        )
        .highlight_style(
            Style::default()
                .fg(theme.selection_text)
                .bg(theme.selection_bg)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, list_state);
}

fn status_symbol(status: &FileStatus) -> &str {
    match status {
        FileStatus::Added => "A",
        FileStatus::Modified => "M",
        FileStatus::Deleted => "D",
        FileStatus::Renamed => "R",
        FileStatus::Copied => "C",
    }
}

fn status_color(status: &FileStatus, app: &App) -> ratatui::style::Color {
    let theme = app.theme();
    match status {
        FileStatus::Added => theme.success,
        FileStatus::Modified => theme.warning,
        FileStatus::Deleted => theme.danger,
        FileStatus::Renamed | FileStatus::Copied => theme.link,
    }
}
