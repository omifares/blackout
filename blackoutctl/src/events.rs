use crossterm::event::{KeyCode, KeyEvent};
use crate::app::{App, AppState};

pub fn handle_event(app: &mut App, key: KeyEvent) {
    match app.state {
        AppState::InitialCheck => {
            // No input needed, but perhaps allow quit
            if key.code == KeyCode::Char('q') {
                // Quit handled in main
            }
        }
        AppState::UnlockPrompt => {
            match key.code {
                KeyCode::Char('q') => {
                    // Quit
                }
                KeyCode::Char(c) => {
                    app.input_buffer.push(c);
                }
                KeyCode::Backspace => {
                    app.input_buffer.pop();
                }
                KeyCode::Enter => {
                    app.unlock_vault(app.input_buffer.clone());
                    app.input_buffer.clear();
                }
                _ => {}
            }
        }
        AppState::EntriesList => {
            match key.code {
                KeyCode::Char('q') => {
                    // Quit
                }
                KeyCode::Char('x') => {
                    app.lock_vault();
                }
                KeyCode::Char('n') => {
                    app.state = AppState::NewEntryForm;
                }
                KeyCode::Up => {
                    app.prev_entry();
                }
                KeyCode::Down => {
                    app.next_entry();
                }
                KeyCode::Backspace => {
                    app.delete_selected_entry();
                }
                KeyCode::Enter => {
                    app.view_selected_entry();
                }
                _ => {}
            }
        }
        AppState::NewEntryForm => {
            match key.code {
                KeyCode::Tab => {
                    app.current_field = (app.current_field + 1) % 3;
                }
                KeyCode::Char(c) => {
                    app.form_fields[app.current_field].push(c);
                }
                KeyCode::Backspace => {
                    app.form_fields[app.current_field].pop();
                }
                KeyCode::Enter => {
                    app.submit_form();
                }
                KeyCode::Esc => {
                    app.reset_form();
                    app.state = AppState::EntriesList;
                }
                _ => {}
            }
        }
        AppState::ViewEntry(_) => {
            match key.code {
                KeyCode::Char('q') => {
                    // Quit
                }
                KeyCode::Char('x') => {
                    app.lock_vault();
                }
                KeyCode::Backspace => {
                    app.delete_selected_entry();
                    app.state = AppState::EntriesList;
                }
                KeyCode::Esc => {
                    app.state = AppState::EntriesList;
                }
                _ => {}
            }
        }
    }
}