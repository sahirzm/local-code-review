use ratatui::{layout::Rect, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, List, ListItem, ListState}, Frame};
use crate::tui::app::App;
use crate::types::FileStatus;

pub fn render_sidebar(frame: &mut Frame, app: &mut App, area: Rect, list_state: &mut ListState) {
    let filtered = app.filtered_files();
    let items: Vec<ListItem> = filtered
        .iter()
        .map(|&idx| {
            let (fc, _) = &app.files[idx];
            let status_color = status_color(&fc.status);
            let reviewed = if app.is_reviewed(&fc.path) { " ✓" } else { "" };
            let comment_count = app.comment_count(&fc.path);
            let cc_display = if comment_count > 0 {
                format!(" [{}]", comment_count)
            } else {
                String::new()
            };
            let line = Line::from(vec![
                Span::styled(status_symbol(&fc.status), Style::default().fg(status_color)),
                Span::raw(format!(" {}{}{}", fc.path, reviewed, cc_display)),
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
        .block(Block::bordered().title(title))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

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

fn status_color(status: &FileStatus) -> Color {
    match status {
        FileStatus::Added => Color::Green,
        FileStatus::Modified => Color::Yellow,
        FileStatus::Deleted => Color::Red,
        FileStatus::Renamed => Color::Cyan,
        FileStatus::Copied => Color::Cyan,
    }
}
