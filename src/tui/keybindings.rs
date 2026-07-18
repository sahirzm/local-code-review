use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crate::tui::app::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    NextFile,
    PrevFile,
    NextComment,
    PrevComment,
    CursorDown,
    CursorUp,
    CommentLine,
    CommentRange,
    CommentFile,
    CommentOverall,
    SetRangeAnchor,
    EditComment,
    DeleteComment,
    ToggleReviewed,
    ToggleSidebar,
    CycleTheme,
    CycleThemeBack,
    IncreaseContext,
    DecreaseContext,
    ToggleViewMode,
    CloseForm,
    ToggleHelp,
    Quit,
    None,
}

pub fn handle_events(app: &mut App) -> std::io::Result<Action> {
    // The comment popup owns raw input elsewhere; this path is normal mode only.
    if app.is_input_active() {
        return Ok(Action::None);
    }

    match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => Ok(match key.code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('n') => Action::NextFile,
            KeyCode::Char('p') => Action::PrevFile,
            KeyCode::Char('j') => Action::NextComment,
            KeyCode::Char('k') => Action::PrevComment,
            KeyCode::Down => Action::CursorDown,
            KeyCode::Up => Action::CursorUp,
            KeyCode::Char('c') => Action::CommentLine,
            KeyCode::Char('v') => Action::SetRangeAnchor,
            KeyCode::Char('R') => Action::CommentRange,
            KeyCode::Char('F') => Action::CommentFile,
            KeyCode::Char('O') => Action::CommentOverall,
            KeyCode::Char('e') => Action::EditComment,
            KeyCode::Char('x') | KeyCode::Delete => Action::DeleteComment,
            KeyCode::Char('r') => Action::ToggleReviewed,
            KeyCode::Char('s') => Action::ToggleSidebar,
            KeyCode::Char('t') => Action::CycleTheme,
            KeyCode::Char('T') => Action::CycleThemeBack,
            KeyCode::Char('+') | KeyCode::Char('=') => Action::IncreaseContext,
            KeyCode::Char('-') => Action::DecreaseContext,
            KeyCode::Char('d') => Action::ToggleViewMode,
            KeyCode::Esc => Action::CloseForm,
            KeyCode::Char('?') => Action::ToggleHelp,
            _ => Action::None,
        }),
        _ => Ok(Action::None),
    }
}
