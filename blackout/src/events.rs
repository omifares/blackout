use std::time::Instant;

use crate::app::{App, AppState};
use crate::state::settings::{FieldConfig, PendingAction, SettingsOption, SettingsState};

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
                    app.open_form(AppState::NewEntryForm(fields), None);
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
                    let uuid = app.get_selected_entry_id();
                    app.open_form(AppState::UpdateEntry(fields), uuid);
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
                    let entry_id = app.get_selected_entry_id();
                    if let Some(entry_id) = entry_id {
                        app.state = AppState::ConfirmAction {
                            action: PendingAction::DeleteEntry(entry_id.clone()),
                            previous_state: Box::new(AppState::EntriesList),
                        };
                    }
                }
                KeyCode::Enter => {
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
                    let uuid = app.get_selected_entry_id();
                    app.open_form(AppState::ViewEntry(fields), uuid);
                }
                _ => {}
            }
        }
        AppState::ViewEntry(ref mut fields) => match key.code {
            KeyCode::Backspace => {
                let entry_id = app.get_selected_entry_id();
                if let Some(entry_id) = entry_id {
                    app.state = AppState::ConfirmAction {
                        action: PendingAction::DeleteEntry(entry_id.clone()),
                        previous_state: Box::new(AppState::EntriesList),
                    };
                }
            }
            KeyCode::Enter => {
                let content = app.form_state.fields[app.form_state.current_index].clone();
                app.copy_to_clipboard(content);
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
                let uuid = app.get_selected_entry_id();
                app.open_form(AppState::UpdateEntry(fields), uuid);
            }
            KeyCode::Tab => {
                app.form_state.current_index = (app.form_state.current_index + 1) % fields.len();
            }
            KeyCode::BackTab => {
                if app.form_state.current_index == 0 {
                    app.form_state.current_index = fields.len() - 1;
                } else {
                    app.form_state.current_index -= 1;
                }
            }
            KeyCode::F(2) => {
                if let AppState::ViewEntry(fields) = &mut app.state {
                    for field in fields.iter_mut() {
                        field.show_password = !field.show_password;
                    }
                }
            }
            KeyCode::Esc => {
                app.reset_form();
                app.state = AppState::EntriesList;
            }
            _ => {}
        },

        AppState::ConfirmAction { ref action, .. } => match key.code {
            KeyCode::Char('y') | KeyCode::Enter => match action {
                PendingAction::DeleteEntry(uuid) => {
                    app.delete_entry(uuid.clone());
                    app.state = AppState::EntriesList;
                }
                PendingAction::RestoreSnapshot(uuid, version) => {
                    app.restore_snapshot(uuid.clone(), Some(*version));
                    app.state = AppState::Settings(SettingsState::default());
                }
            },
            KeyCode::Char('n') | KeyCode::Esc => {
                app.state = AppState::EntriesList;
            }
            _ => {}
        },

        AppState::NewEntryForm(ref mut fields)
        | AppState::UpdateEntry(ref mut fields)
        | AppState::ChangeMasterPassword(ref mut fields) => match key.code {
            KeyCode::Tab => {
                app.form_state.current_index = (app.form_state.current_index + 1) % fields.len();
            }
            KeyCode::BackTab => {
                if app.form_state.current_index == 0 {
                    app.form_state.current_index = fields.len() - 1;
                } else {
                    app.form_state.current_index -= 1;
                }
            }
            KeyCode::Char(c) => {
                if let Some(field_text) =
                    app.form_state.fields.get_mut(app.form_state.current_index)
                {
                    field_text.push(c);
                }
            }
            KeyCode::Backspace => {
                if let Some(field_text) =
                    app.form_state.fields.get_mut(app.form_state.current_index)
                {
                    field_text.pop();
                }
            }
            KeyCode::F(2) => {
                fields[app.form_state.current_index].show_password =
                    !fields[app.form_state.current_index].show_password;
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
                            app.open_form(AppState::ChangeMasterPassword(fields), None);
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
                let uuid = app.get_selected_snapshot_uuid();
                let version = app.get_selected_snapshot_version();
                if let Some(entry_id) = uuid {
                    app.state = AppState::ConfirmAction {
                        action: PendingAction::RestoreSnapshot(entry_id.clone(), version.unwrap()),
                        previous_state: Box::new(AppState::EntriesList),
                    };
                }
            }
            _ => {}
        },
    }
}
