use std::time::Instant;

use crate::app::{App, AppState, FieldConfig, SettingsOption, SettingsState};
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
                    let fields = vec![
                        FieldConfig {
                            label: "Service ".into(),
                            is_password: false,
                            show_password: false,
                        },
                        FieldConfig {
                            label: "Username ".into(),
                            is_password: false,
                            show_password: false,
                        },
                        FieldConfig {
                            label: "Password ".into(),
                            is_password: true,
                            show_password: false,
                        },
                    ];
                    app.open_form(AppState::NewEntryForm(fields), false);
                }
                KeyCode::Char('e') => {
                    let fields = vec![
                        FieldConfig {
                            label: "Edit Service ".into(),
                            is_password: false,
                            show_password: false,
                        },
                        FieldConfig {
                            label: "Edit Username ".into(),
                            is_password: false,
                            show_password: false,
                        },
                        FieldConfig {
                            label: "Edit Password ".into(),
                            is_password: true,
                            show_password: false,
                        },
                    ];
                    app.open_form(AppState::UpdateEntry(fields), true);
                }
                KeyCode::Char('?') => {
                    app.state = AppState::Settings(SettingsState::default());
                }
                KeyCode::Up => {
                    app.prev_index();
                }
                KeyCode::Down => {
                    app.next_index();
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
        AppState::ViewEntry(ref mut view) => match key.code {
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
                let fields = vec![
                    FieldConfig {
                        label: "Edit Service ".into(),
                        is_password: false,
                        show_password: false,
                    },
                    FieldConfig {
                        label: "Edit Username ".into(),
                        is_password: false,
                        show_password: false,
                    },
                    FieldConfig {
                        label: "Edit Password ".into(),
                        is_password: true,
                        show_password: false,
                    },
                ];
                app.open_form(AppState::UpdateEntry(fields), true);
            }
            KeyCode::F(2) => {
                view.show_password = !view.show_password;
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
        },

        AppState::NewEntryForm(ref mut fields)
        | AppState::UpdateEntry(ref mut fields)
        | AppState::ChangeMasterPassword(ref mut fields) => match key.code {
            KeyCode::Tab => {
                app.current_field = (app.current_field + 1) % fields.len();
            }
            KeyCode::BackTab => {
                if app.current_field == 0 {
                    app.current_field = fields.len() - 1;
                } else {
                    app.current_field -= 1;
                }
            }
            KeyCode::Char(c) => {
                if let Some(field_text) = app.form_fields.get_mut(app.current_field) {
                    field_text.push(c);
                }
            }
            KeyCode::Backspace => {
                if let Some(field_text) = app.form_fields.get_mut(app.current_field) {
                    field_text.pop();
                }
            }
            KeyCode::F(2) => {
                if let Some(field) = fields.get_mut(app.current_field)
                    && field.is_password
                {
                    field.show_password = !field.show_password;
                }
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

        AppState::Settings(ref mut settings) => match key.code {
            KeyCode::Esc => {
                app.state = AppState::EntriesList;
            }
            KeyCode::Up => {
                settings.list_state.select_previous();
            }
            KeyCode::Down => {
                settings.list_state.select_next();
            }
            KeyCode::Enter => {
                if let Some(index) = settings.list_state.selected() {
                    match settings.options[index] {
                        SettingsOption::ChangeMasterPassword => {
                            let fields = vec![
                                FieldConfig {
                                    label: "Current Password ".into(),
                                    is_password: true,
                                    show_password: false,
                                },
                                FieldConfig {
                                    label: "New Password ".into(),
                                    is_password: true,
                                    show_password: false,
                                },
                                FieldConfig {
                                    label: "Confirm Password ".into(),
                                    is_password: true,
                                    show_password: false,
                                },
                            ];
                            app.open_form(AppState::ChangeMasterPassword(fields), false);
                        }
                        SettingsOption::SnapshotList => {
                            app.load_snapshots();
                            app.state = AppState::SnapshotList
                        }
                    }
                }
            }
            _ => {}
        },

        AppState::SnapshotList => match key.code {
            KeyCode::Esc => {
                app.state = AppState::Settings(SettingsState::default());
            }
            KeyCode::Up => {
                app.prev_index();
            }
            KeyCode::Down => {
                app.next_index();
            }
            KeyCode::Enter => {
                app.restore_selected_snapshot();
            }
            _ => {}
        },
    }
}
