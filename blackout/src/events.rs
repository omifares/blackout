use std::time::Instant;

use crate::app::{App, AppState};
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_event(app: &mut App, key: KeyEvent) {
    app.last_interaction = Instant::now();
    match app.state {
        AppState::InitialCheck => {
            if key.code == KeyCode::Esc {
                // Quit handled in main
            }
        }
        AppState::UnlockPrompt => {
            match key.code {
                KeyCode::Esc => {
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
        AppState::VaultLocked => {
            match key.code {
                KeyCode::Esc => {
                    // Quit
                }
                _ => {
                    app.input_buffer.clear();
                    app.state = AppState::UnlockPrompt;
                }
            }
        }
        AppState::EntriesList => {
            match key.code {
                KeyCode::Esc => {
                    // Quit
                }
                KeyCode::Char('x') => {
                    app.lock_vault();
                }
                KeyCode::Char('n') => {
                    app.state = AppState::NewEntryForm;
                }
                KeyCode::Char('e') => {
                    app.start_editing_entry();
                    app.state = AppState::UpdateEntry;
                }
                KeyCode::Up => {
                    app.prev_entry();
                }
                KeyCode::Down => {
                    app.next_entry();
                }
                KeyCode::Backspace => {
                    app.state = AppState::ConfirmEntryDelete;
                }
                KeyCode::Enter => {
                    app.view_selected_entry();
                }
                _ => {}
            }
        }
        AppState::NewEntryForm => match key.code {
            KeyCode::Tab => {
                app.current_field = (app.current_field + 1) % 3;
            }
            KeyCode::BackTab => {
                if app.current_field == 0 {
                    app.current_field = 2;
                } else {
                    app.current_field -= 1;
                }
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
        },
        AppState::ViewEntry(ref mut view) => {
            match key.code {
                KeyCode::Char('x') => {
                    app.lock_vault();
                }
                KeyCode::Backspace => {
                    app.state = AppState::ConfirmEntryDelete;
                }
                KeyCode::Enter => {
                    if let Some(entry) = &app.detail_entry {
                        app.copy_to_clipboard(entry.entry.secret.to_string());
                    }
                }
                KeyCode::Char('e') => {
                    app.start_editing_entry();
                    app.state = AppState::UpdateEntry;
                }
                KeyCode::Char('v') => {
                    view.show_password = !view.show_password;
                }
                KeyCode::Esc => {
                    app.reset_form();
                    app.state = AppState::EntriesList;
                }
                _ => {}
            }
        }

        AppState::UpdateEntry => match key.code {
            KeyCode::Tab => {
                app.current_field = (app.current_field + 1) % 3;
            }
            KeyCode::BackTab => {
                if app.current_field == 0 {
                    app.current_field = 2;
                } else {
                    app.current_field -= 1;
                }
            }
            KeyCode::Char(c) => {
                app.form_fields[app.current_field].push(c);
            }
            KeyCode::Backspace => {
                app.form_fields[app.current_field].pop();
            }
            KeyCode::Enter => {
                app.submit_entry_update();
            }
            KeyCode::Esc => {
                app.reset_form();
                app.state = AppState::EntriesList;
            }
            _ => {}
        },

        AppState::ConfirmEntryDelete => match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                app.delete_selected_entry();
                app.state = AppState::EntriesList;
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                app.state = AppState::EntriesList; 
            }
            _ => {}
        }

    }
}
