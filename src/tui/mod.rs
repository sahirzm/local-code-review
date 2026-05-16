use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, ListState, Paragraph},
    Frame,
};

use crate::output::file_writer::write_review_output;
use crate::output::markdown::{generate_markdown, MarkdownInput};
use crate::types::*;
use crate::tui::app::{App, ViewMode};
use crate::tui::components::{comment_form, diff_view, sidebar};
use crate::tui::keybindings::{handle_events, Action};

pub mod app;
pub mod components;
pub mod keybindings;

pub fn run_tui(
    files: Vec<FileChange>,
    parsed_diffs: Vec<ParsedFileDiff>,
    head_ref: String,
    repo_name: String,
    base_ref: String,
) -> anyhow::Result<()> {
    let mut app = App::new(files, parsed_diffs, head_ref);
    let mut list_state = ListState::default().with_selected(Some(0));

    let result: std::io::Result<()> = ratatui::try_init().and_then(|mut terminal| {
        let outcome = run_loop(&mut terminal, &mut app, &mut list_state);
        ratatui::try_restore()?;
        outcome.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    });

    if let Err(e) = result {
        eprintln!("TUI error: {}", e);
    }

    if !app.comments.is_empty() {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(export_and_quit(&app, &repo_name, &base_ref))?;
    }

    Ok(())
}

fn run_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    list_state: &mut ListState,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|frame| render(frame, app, list_state))?;

        if app.view_mode == ViewMode::CommentInput {
            match crossterm::event::read()? {
                crossterm::event::Event::Key(key) if key.kind == crossterm::event::KeyEventKind::Press => {
                    match key.code {
                        crossterm::event::KeyCode::Esc => {
                            app.input_buffer.clear();
                            app.view_mode = ViewMode::Split;
                        }
                        crossterm::event::KeyCode::Enter => {
                            if !app.input_buffer.trim().is_empty() {
                                let filtered = app.filtered_files();
                                let sidx = app.current_file_idx.min(filtered.len().saturating_sub(1));
                                let gidx = filtered.get(sidx).copied().unwrap_or(0);
                                let file_path = app.files.get(gidx).map(|(fc, _)| fc.path.clone());
                                let line = app.files.get(gidx)
                                    .and_then(|(_, d)| d.as_ref())
                                    .and_then(|d| d.hunks.first())
                                    .and_then(|h| h.changes.first())
                                    .and_then(|c| c.new_line_number);
                                app.add_comment(file_path, line, Some("new".into()));
                            }
                        }
                        crossterm::event::KeyCode::Char('1') => app.input_category = CommentCategory::Fix,
                        crossterm::event::KeyCode::Char('2') => app.input_category = CommentCategory::Question,
                        crossterm::event::KeyCode::Char('3') => app.input_category = CommentCategory::Suggestion,
                        crossterm::event::KeyCode::Char('4') => app.input_category = CommentCategory::Nit,
                        crossterm::event::KeyCode::Char(c) => {
                            if app.input_buffer.len() < 2000 {
                                app.input_buffer.push(c);
                            }
                        }
                        crossterm::event::KeyCode::Backspace => {
                            app.input_buffer.pop();
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            continue;
        }

        let action = handle_events(app)?;
        match action {
            Action::Quit => break,
            Action::NextFile => {
                let filtered = app.filtered_files();
                if !filtered.is_empty() {
                    app.current_file_idx = (app.current_file_idx + 1).min(filtered.len() - 1);
                    list_state.select(Some(app.current_file_idx));
                }
            }
            Action::PrevFile => {
                app.current_file_idx = app.current_file_idx.saturating_sub(1);
                list_state.select(Some(app.current_file_idx));
            }
            Action::NextComment => {
                let indices = app.sorted_comment_indices();
                if !indices.is_empty() {
                    app.current_comment_idx = (app.current_comment_idx + 1).min(indices.len() - 1);
                }
            }
            Action::PrevComment => {
                app.current_comment_idx = app.current_comment_idx.saturating_sub(1);
            }
            Action::AddComment => {
                app.input_buffer.clear();
                app.view_mode = ViewMode::CommentInput;
            }
            Action::ToggleViewMode => {
                app.view_mode = match app.view_mode {
                    ViewMode::Split => ViewMode::Unified,
                    _ => ViewMode::Split,
                };
            }
            Action::CloseForm => {}
            Action::ToggleHelp => app.show_help = !app.show_help,
            Action::None => {}
        }
    }
    Ok(())
}

fn render(frame: &mut Frame, app: &mut App, list_state: &mut ListState) {
    let area = frame.area();
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(2),
    ])
    .areas(area);

    render_header(frame, app, header);

    if app.show_help {
        render_help(frame, body);
        return;
    }

    if app.sidebar_collapsed {
        diff_view::render_diff_view(frame, app, body);
    } else {
        let [sidebar_area, diff_area] =
            Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
                .areas(body);
        sidebar::render_sidebar(frame, app, sidebar_area, list_state);
        diff_view::render_diff_view(frame, app, diff_area);
    }

    if app.view_mode == ViewMode::CommentInput {
        let popup_area = centered_rect(60, 30, body);
        comment_form::render_comment_form(frame, app, popup_area);
    }

    render_footer(frame, app, footer);
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let shortcuts = if app.view_mode == ViewMode::CommentInput {
        "COMMENT MODE | 1-fix 2-question 3-suggestion 4-nit | Enter: submit | Esc: cancel".to_string()
    } else {
        "n/p: files | j/k: comments | c: comment | d: toggle view | r: review | s: sidebar | ?: help | q: quit".to_string()
    };

    let paragraph = Paragraph::new(shortcuts)
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));
    frame.render_widget(paragraph, area);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let status = format!(
        " {} comments | {} files reviewed | fix:{} sugg:{} q:{} nit:{} ",
        app.comments.len(),
        app.reviewed_files.len(),
        app.comments.iter().filter(|c| c.category == CommentCategory::Fix).count(),
        app.comments.iter().filter(|c| c.category == CommentCategory::Suggestion).count(),
        app.comments.iter().filter(|c| c.category == CommentCategory::Question).count(),
        app.comments.iter().filter(|c| c.category == CommentCategory::Nit).count(),
    );

    let paragraph = Paragraph::new(status)
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));
    frame.render_widget(paragraph, area);
}

fn render_help(frame: &mut Frame, area: Rect) {
    let shortcuts = vec![
        " n           Next file",
        " p           Previous file",
        " j           Next comment",
        " k           Previous comment",
        " c           Add comment",
        " d           Toggle split/unified view",
        " r           Toggle file reviewed",
        " s           Toggle sidebar",
        " Esc         Close / cancel",
        " ?           Toggle help",
        " q           Quit and export",
        "",
        " In comment mode:",
        "  1-4         Select category",
        "  Enter       Submit comment",
        "  Esc         Cancel",
    ];

    let mut lines = vec![Line::from(Span::styled("Keyboard Shortcuts", Style::default().fg(Color::Yellow)))];
    lines.push(Line::from(""));
    for s in shortcuts {
        lines.push(Line::from(s));
    }

    let paragraph = Paragraph::new(Text::from(lines))
        .block(Block::bordered().title(" Help "))
        .centered();
    frame.render_widget(paragraph, centered_rect(50, 60, area));
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}

async fn export_and_quit(app: &App, repo_name: &str, base_ref: &str) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();

    let files_for_meta: Vec<FileChange> = app.files.iter().map(|(fc, _)| fc.clone()).collect();

    let metadata = ReviewMetadata {
        repo_name: repo_name.to_string(),
        commit_range: format!("{}..HEAD", base_ref),
        base_ref: base_ref.to_string(),
        head_ref: "HEAD".to_string(),
        files: files_for_meta,
        timestamp: now,
        csrf_token: String::new(),
    };

    let diff_files: Vec<ParsedFileDiff> = app.files.iter().filter_map(|(_, d)| d.clone()).collect();

    let diff_data = DiffResponse { files: diff_files };

    let input = MarkdownInput {
        comments: app.comments.clone(),
        diff_data,
        metadata,
    };

    let markdown = generate_markdown(&input);
    print!("{}", markdown);

    let path = crate::output::file_writer::get_default_output_path();
    let abs = write_review_output(&markdown, &path).await?;
    eprintln!("Exported to: {}", abs);

    Ok(())
}
