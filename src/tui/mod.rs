use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, ListState, Paragraph},
    Frame,
};

use crate::config::Config;
use crate::git::resolve_range::RangeResult;
use crate::git::{diff_parser, GitModule};
use crate::output::file_writer::write_review_output;
use crate::output::markdown::{generate_markdown, MarkdownInput};
use crate::session;
use crate::tui::app::{App, ViewMode};
use crate::tui::components::{comment_form, diff_view, sidebar};
use crate::tui::keybindings::{handle_events, Action};
use crate::types::*;

pub mod app;
pub mod components;
pub mod icons;
pub mod keybindings;
pub mod syntax;
pub mod theme;

/// Everything the TUI needs to run — including an owned `GitModule` and the
/// resolved range, so it can re-diff at runtime when the context width changes.
pub struct TuiContext {
    pub files: Vec<FileChange>,
    pub parsed_diffs: Vec<ParsedFileDiff>,
    pub head_ref: String,
    pub base_ref: String,
    pub repo_name: String,
    pub repo_path: String,
    pub git: GitModule,
    pub range: RangeResult,
    pub include_untracked: bool,
    pub context_lines: u32,
    pub config: Config,
}

/// The commit-range string used for both the session key and metadata,
/// matching the web/server convention (`base..head`).
fn commit_range(base_ref: &str, head_ref: &str) -> String {
    format!("{}..{}", base_ref, head_ref)
}

/// Build the session key exactly as the web/server do: hash the base-ref
/// segment of the range (see session-key note in the plan).
fn session_key(base_ref: &str, head_ref: &str) -> String {
    let range = commit_range(base_ref, head_ref);
    let base = range.split("..").next().unwrap_or(&range);
    let hash = session::hash_repo_path(base);
    session::get_session_key(&hash, &range)
}

pub fn run_tui(ctx: TuiContext) -> anyhow::Result<()> {
    // Clone the initial diff into the app; `ctx` retains the git handle + range
    // so it can re-diff at runtime (context adjustment).
    let mut app = App::new(
        ctx.files.clone(),
        ctx.parsed_diffs.clone(),
        ctx.head_ref.clone(),
        &ctx.config,
    );
    app.context_lines = ctx.context_lines;
    app.reset_diff_view();

    // Restore any prior session (comments/reviewed/view-mode) for this range.
    let key = session_key(&ctx.base_ref, &ctx.head_ref);
    if let Some(saved) = session::load_session(&key) {
        app.comments = saved.comments;
        app.reviewed_files = saved.reviewed_files;
        app.set_view_mode_from_str(&saved.view_mode);
    }

    let mut list_state = ListState::default().with_selected(Some(0));

    let result: std::io::Result<()> = ratatui::try_init().and_then(|mut terminal| {
        let outcome = run_loop(&mut terminal, &mut app, &mut list_state, &ctx);
        ratatui::try_restore()?;
        outcome.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    });

    if let Err(e) = result {
        eprintln!("TUI error: {}", e);
    }

    // Persist the session (best effort) and the shared config, then export.
    persist_session(&app, &ctx, &key);
    persist_config(&app, &ctx);

    if !app.comments.is_empty() {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(export_and_quit(&app, &ctx.repo_name, &ctx.base_ref))?;
    }

    Ok(())
}

fn persist_session(app: &App, ctx: &TuiContext, key: &str) {
    let now = chrono::Utc::now().to_rfc3339();
    let range = commit_range(&ctx.base_ref, &ctx.head_ref);
    let base = range.split("..").next().unwrap_or(&range);
    let session = ReviewSession {
        version: 2,
        commit_range: range.clone(),
        repo_path: ctx.repo_name.clone(),
        repo_path_hash: session::hash_repo_path(base),
        comments: app.comments.clone(),
        reviewed_files: app.reviewed_files.clone(),
        view_mode: app.persistable_view_mode().to_string(),
        created_at: now.clone(),
        last_accessed_at: now,
    };
    if let Err(e) = session::save_session(key, &session) {
        eprintln!("Warning: could not save session: {}", e);
    }
}

fn persist_config(app: &App, ctx: &TuiContext) {
    let cfg = Config {
        theme: app.theme_id.clone(),
        icon_mode: ctx.config.icon_mode,
        diff_context_lines: app.context_lines,
    };
    if cfg != ctx.config {
        if let Err(e) = cfg.save() {
            eprintln!("Warning: could not save config: {}", e);
        }
    }
}

/// Re-run the diff at a new context width and rebuild the file list, preserving
/// review state. Synchronous — we already hold the resolved range.
fn rebuild_files(ctx: &TuiContext, context_lines: u32) -> anyhow::Result<(Vec<FileChange>, Vec<ParsedFileDiff>)> {
    let raw = match ctx.range.mode.as_str() {
        "staged" => ctx.git.get_staged_diff(context_lines)?,
        "unstaged" => ctx.git.get_unstaged_diff(context_lines)?,
        "working" => ctx.git.get_working_diff(context_lines)?,
        "commits" => ctx.git.get_diff(&ctx.range.args[0], &ctx.range.args[1], context_lines)?,
        "all" => ctx
            .git
            .get_diff_from_to_workdir(&ctx.range.args[0], ctx.include_untracked, context_lines)?,
        other => anyhow::bail!("Unknown range mode: {}", other),
    };
    let parsed = diff_parser::parse_diff(&raw);
    let file_list: Vec<FileChange> = if ctx.range.mode == "commits" {
        ctx.git
            .get_file_list(&ctx.range.args[0], &ctx.range.args[1])
            .unwrap_or_default()
    } else {
        parsed
            .iter()
            .map(|f| FileChange {
                path: f.new_path.clone(),
                old_path: if f.old_path != f.new_path {
                    Some(f.old_path.clone())
                } else {
                    None
                },
                status: f.status.clone(),
                additions: f.additions,
                deletions: f.deletions,
            })
            .collect()
    };
    Ok((file_list, parsed))
}

fn adjust_context(app: &mut App, ctx: &TuiContext, delta: i32) {
    let next = (app.context_lines as i32 + delta).clamp(0, crate::config::MAX_CONTEXT_LINES as i32) as u32;
    if next == app.context_lines {
        return;
    }
    match rebuild_files(ctx, next) {
        Ok((files, parsed)) => {
            app.context_lines = next;
            app.rebuild_diffs(files, parsed);
        }
        Err(e) => eprintln!("Re-diff failed: {}", e),
    }
}

fn run_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    list_state: &mut ListState,
    ctx: &TuiContext,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|frame| render(frame, app, list_state))?;

        if app.view_mode == ViewMode::CommentInput {
            handle_comment_input(app)?;
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
                    app.reset_diff_view();
                }
            }
            Action::PrevFile => {
                app.current_file_idx = app.current_file_idx.saturating_sub(1);
                list_state.select(Some(app.current_file_idx));
                app.reset_diff_view();
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
            Action::CursorDown => app.move_cursor(true),
            Action::CursorUp => app.move_cursor(false),
            Action::CommentLine => app.begin_comment(CommentType::Line),
            Action::CommentRange => app.begin_comment(CommentType::Range),
            Action::CommentFile => app.begin_comment(CommentType::File),
            Action::CommentOverall => app.begin_comment(CommentType::Overall),
            Action::SetRangeAnchor => app.set_range_anchor(),
            Action::EditComment => {
                if let Some(id) = app.selected_comment_id() {
                    app.begin_edit(&id);
                }
            }
            Action::DeleteComment => {
                if let Some(id) = app.selected_comment_id() {
                    app.delete_comment(&id);
                }
            }
            Action::ToggleReviewed => {
                if let Some(path) = app.selected_file_path() {
                    app.toggle_reviewed(&path);
                }
            }
            Action::ToggleSidebar => app.sidebar_collapsed = !app.sidebar_collapsed,
            Action::CycleTheme => app.cycle_theme(true),
            Action::CycleThemeBack => app.cycle_theme(false),
            Action::IncreaseContext => adjust_context(app, ctx, 1),
            Action::DecreaseContext => adjust_context(app, ctx, -1),
            Action::ToggleViewMode => {
                app.view_mode = match app.view_mode {
                    ViewMode::Split => ViewMode::Unified,
                    _ => ViewMode::Split,
                };
            }
            Action::CloseForm => {
                app.range_anchor = None;
            }
            Action::ToggleHelp => app.show_help = !app.show_help,
            Action::None => {}
        }
    }
    Ok(())
}

/// Raw key handling for the comment popup (text entry + category hotkeys).
fn handle_comment_input(app: &mut App) -> anyhow::Result<()> {
    use crossterm::event::{self, Event, KeyCode, KeyEventKind};
    if let Event::Key(key) = event::read()? {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }
        match key.code {
            KeyCode::Esc => app.cancel_input(),
            KeyCode::Enter => app.commit_pending_comment(),
            KeyCode::Char('1') => app.input_category = CommentCategory::Fix,
            KeyCode::Char('2') => app.input_category = CommentCategory::Question,
            KeyCode::Char('3') => app.input_category = CommentCategory::Suggestion,
            KeyCode::Char('4') => app.input_category = CommentCategory::Nit,
            KeyCode::Char(c) => app.push_input_char(c),
            KeyCode::Backspace => app.pop_input_char(),
            _ => {}
        }
    }
    Ok(())
}

fn render(frame: &mut Frame, app: &mut App, list_state: &mut ListState) {
    let area = frame.area();
    let theme = app.theme();
    frame.render_widget(
        Block::default().style(Style::default().bg(theme.bg).fg(theme.text)),
        area,
    );

    let [header, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(2),
    ])
    .areas(area);

    render_header(frame, app, header);

    if app.show_help {
        render_help(frame, app, body);
    } else if app.sidebar_collapsed {
        diff_view::render_diff_view(frame, app, body);
    } else {
        let [sidebar_area, diff_area] =
            Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)]).areas(body);
        sidebar::render_sidebar(frame, app, sidebar_area, list_state);
        diff_view::render_diff_view(frame, app, diff_area);
    }

    if app.view_mode == ViewMode::CommentInput {
        let popup_area = centered_rect(60, 40, body);
        comment_form::render_comment_form(frame, app, popup_area);
    }

    render_footer(frame, app, footer);
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let text = if app.view_mode == ViewMode::CommentInput {
        "COMMENT  1-4 category · Enter submit · Esc cancel".to_string()
    } else {
        "n/p file · ↑/↓ line · j/k comment · c line · v range · F file · O overall · e edit · x del · r reviewed · s sidebar · d view · t theme · +/- ctx · ? help · q quit".to_string()
    };
    let p = Paragraph::new(text).style(Style::default().fg(theme.on_accent).bg(theme.accent));
    frame.render_widget(p, area);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let counts = |cat: CommentCategory| app.comments.iter().filter(|c| c.category == cat).count();
    let status = format!(
        " {} comments · {} reviewed · fix:{} sugg:{} q:{} nit:{} · ctx:{} · {} ",
        app.comments.len(),
        app.reviewed_files.len(),
        counts(CommentCategory::Fix),
        counts(CommentCategory::Suggestion),
        counts(CommentCategory::Question),
        counts(CommentCategory::Nit),
        app.context_lines,
        app.theme_id,
    );
    let p = Paragraph::new(status).style(Style::default().fg(theme.text_dim).bg(theme.panel));
    frame.render_widget(p, area);
}

fn render_help(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let rows = [
        ("n / p", "Next / previous file"),
        ("↑ / ↓", "Move diff cursor"),
        ("j / k", "Next / previous comment"),
        ("c", "Comment on the cursor line"),
        ("v", "Set range anchor, then c to comment the range"),
        ("F", "File-level comment"),
        ("O", "Overall comment"),
        ("e", "Edit the selected comment"),
        ("x / Del", "Delete the selected comment"),
        ("r", "Toggle file reviewed"),
        ("s", "Toggle sidebar"),
        ("d", "Toggle split / unified view"),
        ("t / T", "Cycle theme forward / back"),
        ("+ / -", "Increase / decrease diff context"),
        ("1-4", "In comment mode: pick category"),
        ("?", "Toggle this help"),
        ("q", "Quit (saves session + exports markdown)"),
    ];
    let mut lines = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    for (k, d) in rows {
        lines.push(Line::from(vec![
            Span::styled(format!(" {:<9}", k), Style::default().fg(theme.warning)),
            Span::styled(d.to_string(), Style::default().fg(theme.text)),
        ]));
    }
    let p = Paragraph::new(Text::from(lines)).block(
        Block::bordered()
            .title(" Help ")
            .border_style(Style::default().fg(theme.border)),
    );
    frame.render_widget(p, centered_rect(60, 70, area));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_key_matches_web_convention() {
        // Web/server hash the base-ref segment of the range.
        let key = session_key("main", "feature");
        let expect_hash = session::hash_repo_path("main");
        assert_eq!(key, format!("local-review:{}:main..feature", expect_hash));
    }
}
