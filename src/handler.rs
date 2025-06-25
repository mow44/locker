use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use wrap_context::arg_context;

use crate::{app::App, types::CursorDirection};

#[rustfmt::skip]
/// Handles the key events and updates the state of [`App`].
pub fn handle_key_events(key_event: KeyEvent, app: &mut App) -> anyhow::Result<()> {
    match key_event.code {
        // Exit application on `ESC` or `q`
        KeyCode::Esc | KeyCode::Char('q') => {
            app.quit();
        }
        // Exit application on `Ctrl-C`
        KeyCode::Char('c') | KeyCode::Char('C') if key_event.modifiers == KeyModifiers::CONTROL => {
            app.quit();
        }
        KeyCode::Char('c') | KeyCode::Backspace => {
            arg_context!(app.clear_selected())?;
        }
        KeyCode::Down | KeyCode::Char('J') if key_event.modifiers == KeyModifiers::SHIFT => {
            arg_context!(app.dec_left_table_column_width())?;
        }
        KeyCode::Up | KeyCode::Char('K') if key_event.modifiers == KeyModifiers::SHIFT => {
            arg_context!(app.inc_left_table_column_width())?;
        }
        KeyCode::Down | KeyCode::Char('j') if key_event.modifiers == KeyModifiers::CONTROL => {
            arg_context!(app.dec_rght_table_column_width())?;
        }
        KeyCode::Up | KeyCode::Char('k') if key_event.modifiers == KeyModifiers::CONTROL => {
            arg_context!(app.inc_rght_table_column_width())?;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            arg_context!(app.cursor_move(CursorDirection::Down))?
        }
        KeyCode::Up | KeyCode::Char('k') => {
            arg_context!(app.cursor_move(CursorDirection::Up))?
        },
        KeyCode::Right | KeyCode::Char('l') => {
            arg_context!(app.cursor_move(CursorDirection::Right))?
        }
        KeyCode::Left | KeyCode::Char('h') => {
            arg_context!(app.cursor_move(CursorDirection::Left))?
        },
        KeyCode::Enter | KeyCode::Char(' ') => {
            arg_context!(app.cursor_select())?
        },
        _ => {}
    }

    anyhow::Ok(())
}
