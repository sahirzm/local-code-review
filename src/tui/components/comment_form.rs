use ratatui::{layout::Rect, style::Style, text::{Line, Span, Text}, widgets::{Block, Paragraph}, Frame};
use crate::tui::app::App;
use crate::types::CommentCategory;

pub fn render_comment_form(frame: &mut Frame, app: &App, area: Rect) {
    let categories = vec![
        ("fix", CommentCategory::Fix, ratatui::style::Color::Red),
        ("question", CommentCategory::Question, ratatui::style::Color::Blue),
        ("suggestion", CommentCategory::Suggestion, ratatui::style::Color::Green),
        ("nit", CommentCategory::Nit, ratatui::style::Color::Gray),
    ];

    let mut cat_spans: Vec<Span> = Vec::new();
    for (label, cat, color) in &categories {
        let style = if app.input_category == *cat {
            Style::default().fg(ratatui::style::Color::White).bg(*color)
        } else {
            Style::default().fg(*color)
        };
        cat_spans.push(Span::styled(format!(" {} ", label), style));
        cat_spans.push(Span::raw(" "));
    }

    let mut lines = vec![
        Line::from(cat_spans),
        Line::from(""),
        Line::from(Span::styled(format!("> {}", app.input_buffer), Style::default().fg(ratatui::style::Color::White))),
    ];

    if !app.input_buffer.is_empty() {
        let remaining = 2000 - app.input_buffer.len();
        lines.push(Line::from(Span::styled(
            format!("{} chars remaining | Enter to submit | Esc to cancel | 1/2/3/4 to switch category", remaining),
            Style::default().fg(ratatui::style::Color::DarkGray),
        )));
    }

    let paragraph = Paragraph::new(Text::from(lines))
        .block(Block::bordered().title(" Add Comment ").border_style(Style::default().fg(ratatui::style::Color::Yellow)));

    frame.render_widget(paragraph, area);
}
