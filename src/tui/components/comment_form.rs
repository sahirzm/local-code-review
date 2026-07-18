use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Clear, Paragraph},
    Frame,
};

use crate::tui::app::App;
use crate::types::{CommentCategory, CommentType};

pub fn render_comment_form(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();

    let categories = [
        ("fix", CommentCategory::Fix, theme.danger),
        ("question", CommentCategory::Question, theme.link),
        ("suggestion", CommentCategory::Suggestion, theme.success),
        ("nit", CommentCategory::Nit, theme.text_muted),
    ];

    let mut cat_spans: Vec<Span> = Vec::new();
    for (label, cat, color) in &categories {
        let style = if app.input_category == *cat {
            Style::default().fg(theme.on_accent).bg(*color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(*color)
        };
        cat_spans.push(Span::styled(format!(" {} ", label), style));
        cat_spans.push(Span::raw(" "));
    }

    let title = match app.pending_comment.as_ref() {
        Some(p) if p.editing_id.is_some() => " Edit Comment ".to_string(),
        Some(p) => match p.kind {
            CommentType::Line => format!(" Comment · line {} ", p.start_line.unwrap_or(0)),
            CommentType::Range => format!(
                " Comment · lines {}-{} ",
                p.start_line.unwrap_or(0),
                p.end_line.unwrap_or(0)
            ),
            CommentType::File => " File Comment ".to_string(),
            CommentType::Overall => " Overall Comment ".to_string(),
        },
        None => " Comment ".to_string(),
    };

    let remaining = 2000usize.saturating_sub(app.input_buffer.chars().count());
    let lines = vec![
        Line::from(cat_spans),
        Line::from(""),
        Line::from(Span::styled(
            format!("> {}", app.input_buffer),
            Style::default().fg(theme.text),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!(
                "{} left · Enter submit · Esc cancel · 1-4 category",
                remaining
            ),
            Style::default().fg(theme.text_muted),
        )),
    ];

    frame.render_widget(Clear, area);
    let p = Paragraph::new(Text::from(lines)).block(
        Block::bordered()
            .title(title)
            .border_style(Style::default().fg(theme.accent))
            .style(Style::default().bg(theme.panel)),
    );
    frame.render_widget(p, area);
}
