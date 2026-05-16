use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crate::tui::app::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    NextFile,
    PrevFile,
    NextComment,
    PrevComment,
    AddComment,
    ToggleViewMode,
    CloseForm,
    ToggleHelp,
    Quit,
    None,
}

pub fn handle_events(app: &mut App) -> std::io::Result<Action> {
    if app.is_input_active() {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Esc => return Ok(Action::CloseForm),
                KeyCode::Char(c) => {
                    app.push_input_char(c);
                }
                KeyCode::Backspace => {
                    app.pop_input_char();
                }
                _ => {}
            },
            _ => {}
        }
        return Ok(Action::None);
    }

    match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
            KeyCode::Char('q') => return Ok(Action::Quit),
            KeyCode::Char('n') => return Ok(Action::NextFile),
            KeyCode::Char('p') => return Ok(Action::PrevFile),
            KeyCode::Char('j') => return Ok(Action::NextComment),
            KeyCode::Char('k') => return Ok(Action::PrevComment),
            KeyCode::Char('c') => return Ok(Action::AddComment),
            KeyCode::Char('d') => return Ok(Action::ToggleViewMode),
            KeyCode::Esc => return Ok(Action::CloseForm),
            KeyCode::Char('?') => return Ok(Action::ToggleHelp),
            _ => {}
        },
        _ => {}
    }
    Ok(Action::None)
}
