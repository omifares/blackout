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
        AppState::UnlockPrompt(ref mut field) => {
            match key.code {
                KeyCode::Esc => {
                    // Quit
                }
                KeyCode::Char(c) => {
                    if let Some(field_text) =
                        app.form_state.fields.get_mut(app.form_state.current_field)
                    {
                        let mut chars: Vec<char> = field_text.chars().collect();

                        let cursor = app.form_state.cursor_index;
                        chars.insert(cursor.min(chars.len()), c);
                        *field_text = chars.into_iter().collect();

                        app.form_state.cursor_index += 1;
                    }
                }
                KeyCode::Backspace => {
                    if let Some(field_text) =
                        app.form_state.fields.get_mut(app.form_state.current_field)
                    {
                        let mut chars: Vec<char> = field_text.chars().collect();
                        let cursor = app.form_state.cursor_index;

                        if cursor > 0 && cursor <= chars.len() {
                            chars.remove(cursor - 1);
                            *field_text = chars.into_iter().collect();
                            app.form_state.cursor_index -= 1;
                        }
                    }
                }
                KeyCode::Delete => {
                    if let Some(field_text) =
                        app.form_state.fields.get_mut(app.form_state.current_field)
                    {
                        let mut chars: Vec<char> = field_text.chars().collect();
                        let cursor = app.form_state.cursor_index;

                        if cursor < chars.len() {
                            chars.remove(cursor);
                            *field_text = chars.into_iter().collect();
                        }
                    }
                }
                KeyCode::Left => {
                    if app.form_state.cursor_index > 0 {
                        app.form_state.cursor_index -= 1;
                    }
                }
                KeyCode::Right => {
                    if let Some(field_text) =
                        app.form_state.fields.get_mut(app.form_state.current_field)
                    {
                        if app.form_state.cursor_index < field_text.len() {
                            app.form_state.cursor_index += 1;
                        }
                    }
                }
                KeyCode::F(2) => {
                    field.show_password = !field.show_password;
                }
                KeyCode::Enter => {
                    app.unlock_vault(app.form_state.fields[0].clone());
                    app.form_state.clear();
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
                    app.form_state.clear();

                    app.state = AppState::UnlockPrompt(FieldConfig::password("Password"))
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
                        FieldConfig::text("Service"),
                        FieldConfig::text("Username"),
                        FieldConfig::password("Password"),
                    ];
                    app.open_form(AppState::NewEntryForm(fields), None);
                }
                KeyCode::Char('e') => {
                    let fields = vec![
                        FieldConfig::text("Edit Service"),
                        FieldConfig::text("Edit Username"),
                        FieldConfig::password("Edit Password"),
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
                            action: PendingAction::DeleteEntry(entry_id),
                            previous_state: Box::new(AppState::EntriesList),
                        };
                    }
                }
                KeyCode::Enter => {
                    let fields = vec![
                        FieldConfig::text("Service"),
                        FieldConfig::text("Username"),
                        FieldConfig::password("Password"),
                    ];
                    let uuid = app.get_selected_entry_id();
                    app.open_form(AppState::ViewEntry(fields, uuid.unwrap_or_default()), uuid);
                }
                _ => {}
            }
        }
        AppState::ViewEntry(ref mut fields, uuid) => match key.code {
            KeyCode::Backspace => {
                app.state = AppState::ConfirmAction {
                    action: PendingAction::DeleteEntry(uuid),
                    previous_state: Box::new(AppState::EntriesList),
                };
            }
            KeyCode::Enter => {
                let content = app.form_state.fields[app.form_state.current_field].clone();
                app.copy_to_clipboard(content);
            }
            KeyCode::Char('e') => {
                let fields = vec![
                    FieldConfig::text("Edit Service"),
                    FieldConfig::text("Edit Username"),
                    FieldConfig::password("Edit Password"),
                ];
                app.open_form(AppState::UpdateEntry(fields), Some(uuid));
            }
            KeyCode::Tab => {
                app.form_state.current_field = (app.form_state.current_field + 1) % fields.len();
            }
            KeyCode::BackTab => {
                if app.form_state.current_field == 0 {
                    app.form_state.current_field = fields.len() - 1;
                } else {
                    app.form_state.current_field -= 1;
                }
            }
            KeyCode::F(2) => {
                if let AppState::ViewEntry(fields, ..) = &mut app.state {
                    for field in fields.iter_mut() {
                        field.show_password = !field.show_password;
                    }
                }
            }
            KeyCode::Esc => {
                app.form_state.clear();
                app.state = AppState::EntriesList;
            }
            _ => {}
        },

        AppState::ConfirmAction { ref action, .. } => match key.code {
            KeyCode::Char('y') | KeyCode::Enter => match action {
                PendingAction::DeleteEntry(uuid) => {
                    app.delete_entry(*uuid);
                    app.state = AppState::EntriesList;
                }
                PendingAction::RestoreSnapshot(uuid, version) => {
                    app.restore_snapshot(*uuid, Some(*version));
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
                app.form_state.current_field = (app.form_state.current_field + 1) % fields.len();
                app.form_state.cursor_index = 0;
            }
            KeyCode::BackTab => {
                if app.form_state.current_field == 0 {
                    app.form_state.current_field = fields.len() - 1;
                } else {
                    app.form_state.current_field -= 1;
                }
                app.form_state.cursor_index = 0;
            }
            KeyCode::Down => {
                app.form_state.current_field = (app.form_state.current_field + 1) % fields.len();
                app.form_state.cursor_index = 0;
            }
            KeyCode::Up => {
                if app.form_state.current_field == 0 {
                    app.form_state.current_field = fields.len() - 1;
                } else {
                    app.form_state.current_field -= 1;
                }
                app.form_state.cursor_index = 0;
            }
            KeyCode::Left => {
                if app.form_state.cursor_index > 0 {
                    app.form_state.cursor_index -= 1;
                }
            }
            KeyCode::Right => {
                if let Some(field_text) =
                    app.form_state.fields.get_mut(app.form_state.current_field)
                {
                    if app.form_state.cursor_index < field_text.len() {
                        app.form_state.cursor_index += 1;
                    }
                }
            }
            KeyCode::Char(c) => {
                if let Some(field_text) =
                    app.form_state.fields.get_mut(app.form_state.current_field)
                {
                    let mut chars: Vec<char> = field_text.chars().collect();

                    let cursor = app.form_state.cursor_index;
                    chars.insert(cursor.min(chars.len()), c);
                    *field_text = chars.into_iter().collect();

                    app.form_state.cursor_index += 1;
                }
            }
            KeyCode::Backspace => {
                if let Some(field_text) =
                    app.form_state.fields.get_mut(app.form_state.current_field)
                {
                    let mut chars: Vec<char> = field_text.chars().collect();
                    let cursor = app.form_state.cursor_index;

                    if cursor > 0 && cursor <= chars.len() {
                        chars.remove(cursor - 1);
                        *field_text = chars.into_iter().collect();
                        app.form_state.cursor_index -= 1;
                    }
                }
            }
            KeyCode::Delete => {
                if let Some(field_text) =
                    app.form_state.fields.get_mut(app.form_state.current_field)
                {
                    let mut chars: Vec<char> = field_text.chars().collect();
                    let cursor = app.form_state.cursor_index;

                    if cursor < chars.len() {
                        chars.remove(cursor);
                        *field_text = chars.into_iter().collect();
                    }
                }
            }
            KeyCode::F(2) => {
                fields[app.form_state.current_field].show_password =
                    !fields[app.form_state.current_field].show_password;
            }
            KeyCode::Enter => {
                app.submit_form();
            }
            KeyCode::Esc => {
                app.form_state.clear();
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
                                FieldConfig::password("Current Password"),
                                FieldConfig::password("New Password"),
                                FieldConfig::password("Conform Password"),
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
                        action: PendingAction::RestoreSnapshot(entry_id, version.unwrap()),
                        previous_state: Box::new(AppState::EntriesList),
                    };
                }
            }
            _ => {}
        },
    }
}
