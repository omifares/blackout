use std::time::Instant;

use crate::app::{App, AppState};
use crate::state::PasswordGeneratorState;
use crate::state::settings::{
    FieldConfig, FieldType, FieldValue, PendingAction, SettingsOption, SettingsState,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

const MAX_PASSWORD_LENGTH: usize = 999;

pub fn handle_event(app: &mut App, key: KeyEvent) {
    app.last_interaction = Instant::now();
    match app.state {
        AppState::InitialCheck => {
            if key.code == KeyCode::Esc {
                // Quit handled in main
            }
        }
        AppState::UnlockPrompt => match key.code {
            KeyCode::Esc => app.state = AppState::VaultLocked,
            KeyCode::Backspace => {
                if let Some(field_config) =
                    app.form_state.fields.get_mut(app.form_state.current_field)
                {
                    match field_config.value {
                        FieldValue::Text(ref mut field_text) => {
                            let cursor = app.form_state.cursor_index;
                            if cursor > 0 && cursor <= field_text.len() {
                                field_text.remove(cursor - 1);
                                app.form_state.cursor_index -= 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
            KeyCode::Char(c) => {
                if let Some(field_config) =
                    app.form_state.fields.get_mut(app.form_state.current_field)
                {
                    if matches!(field_config.field_type, FieldType::Number) && !c.is_numeric() {
                        return;
                    }

                    match &mut field_config.value {
                        FieldValue::Text(field_text) => {
                            let cursor = app.form_state.cursor_index;
                            let mut chars: Vec<char> = field_text.chars().collect();
                            let insert_pos = cursor.min(chars.len());

                            chars.insert(insert_pos, c);
                            *field_text = chars.into_iter().collect();

                            app.form_state.cursor_index += 1;
                        }
                        _ => {}
                    }
                }
            }
            KeyCode::Delete => {
                if let Some(field_config) =
                    app.form_state.fields.get_mut(app.form_state.current_field)
                {
                    match field_config.value {
                        FieldValue::Text(ref mut field_text) => {
                            let mut chars: Vec<char> = field_text.chars().collect();
                            let cursor = app.form_state.cursor_index;

                            if cursor < chars.len() {
                                chars.remove(cursor);
                                *field_text = chars.into_iter().collect();
                            }
                        }
                        _ => {}
                    }
                }
            }
            KeyCode::Left => {
                if app.form_state.cursor_index > 0 {
                    app.form_state.cursor_index -= 1;
                }
            }
            KeyCode::Right => {
                if let Some(field_config) =
                    app.form_state.fields.get_mut(app.form_state.current_field)
                {
                    match field_config.value {
                        FieldValue::Text(ref field_text) => {
                            if app.form_state.cursor_index < field_text.len() {
                                app.form_state.cursor_index += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
            KeyCode::F(2) => {
                app.form_state.obscure_inputs = !app.form_state.obscure_inputs;
            }
            KeyCode::Enter => {
                let current_field = app.form_state.current_field;
                let password = match app.form_state.fields[current_field].value {
                    FieldValue::Text(ref field_text) => field_text.clone(),
                    _ => return,
                };
                app.unlock_vault(&password);
            }

            _ => {}
        },
        AppState::VaultLocked => {
            match key.code {
                KeyCode::Esc => {
                    // Quit
                }
                _ => {
                    app.form_state.clear();
                    app.form_state.fields = vec![FieldConfig::password("Master Password")];
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
                        FieldConfig::service("Service"),
                        FieldConfig::username("Username"),
                        FieldConfig::password("Password"),
                    ];
                    app.form_state.clear();
                    app.form_state.fields = fields;
                    app.open_form(AppState::NewEntryForm, None);
                }
                KeyCode::Char('e') => {
                    let fields = vec![
                        FieldConfig::service("Edit Service"),
                        FieldConfig::username("Edit Username"),
                        FieldConfig::password("Edit Password"),
                    ];
                    app.form_state.clear();
                    app.form_state.fields = fields;
                    let uuid = app.get_selected_entry_id();
                    app.open_form(AppState::UpdateEntry, uuid);
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
                        FieldConfig::text("Service", ""),
                        FieldConfig::text("Username", ""),
                        FieldConfig::password("Password"),
                    ];
                    app.form_state.clear();
                    app.form_state.fields = fields;
                    let uuid = app.get_selected_entry_id();
                    app.open_form(AppState::ViewEntry(uuid.unwrap_or_default()), uuid);
                }
                _ => {}
            }
        }
        AppState::ViewEntry(ref uuid) => match key.code {
            KeyCode::Backspace => {
                app.state = AppState::ConfirmAction {
                    action: PendingAction::DeleteEntry(*uuid),
                    previous_state: Box::new(AppState::EntriesList),
                };
            }
            KeyCode::Enter => {
                let content = match app.form_state.fields[app.form_state.current_field].value {
                    FieldValue::Text(ref field_text) => field_text.clone(),
                    _ => return,
                };
                app.copy_to_clipboard(content);
            }
            KeyCode::Char('e') => {
                let fields = vec![
                    FieldConfig::service("Edit Service"),
                    FieldConfig::username("Edit Username"),
                    FieldConfig::password("Edit Password"),
                ];
                app.form_state.clear();
                app.form_state.fields = fields;
                let uuid = app.get_selected_entry_id();
                app.open_form(AppState::UpdateEntry, uuid);
            }
            KeyCode::Tab => {
                app.form_state.current_field =
                    (app.form_state.current_field + 1) % app.form_state.fields.len();
            }
            KeyCode::BackTab => {
                if app.form_state.current_field == 0 {
                    app.form_state.current_field = app.form_state.fields.len() - 1;
                } else {
                    app.form_state.current_field -= 1;
                }
            }
            KeyCode::F(2) => {
                if let AppState::ViewEntry(..) = &mut app.state {
                    for field in app.form_state.fields.iter_mut() {
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

        AppState::NewEntryForm | AppState::UpdateEntry | AppState::ChangeMasterPassword => {
            match key.modifiers {
                KeyModifiers::CONTROL => match key.code {
                    KeyCode::Char('a') => {
                        app.auto_fill_password();
                        return;
                    }
                    _ => {}
                },
                _ => {}
            }
            match key.code {
                KeyCode::Tab => {
                    app.form_state.current_field =
                        (app.form_state.current_field + 1) % app.form_state.fields.len();
                    app.form_state.cursor_index = 0;
                }
                KeyCode::BackTab => {
                    if app.form_state.current_field == 0 {
                        app.form_state.current_field = app.form_state.fields.len() - 1;
                    } else {
                        app.form_state.current_field -= 1;
                    }
                    app.form_state.cursor_index = 0;
                }
                KeyCode::Down => {
                    app.form_state.current_field =
                        (app.form_state.current_field + 1) % app.form_state.fields.len();
                    app.form_state.cursor_index = 0;
                }
                KeyCode::Up => {
                    if app.form_state.current_field == 0 {
                        app.form_state.current_field = app.form_state.fields.len() - 1;
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
                    if let Some(field_config) =
                        app.form_state.fields.get_mut(app.form_state.current_field)
                    {
                        match field_config.value {
                            FieldValue::Text(ref field_text) => {
                                if app.form_state.cursor_index < field_text.len() {
                                    app.form_state.cursor_index += 1;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                KeyCode::Char(c) => {
                    if let Some(field_config) =
                        app.form_state.fields.get_mut(app.form_state.current_field)
                    {
                        match field_config.value {
                            FieldValue::Text(ref mut field_text) => {
                                let mut chars: Vec<char> = field_text.chars().collect();

                                let cursor = app.form_state.cursor_index;
                                chars.insert(cursor.min(chars.len()), c);
                                *field_text = chars.into_iter().collect();

                                app.form_state.cursor_index += 1;
                            }
                            _ => {}
                        }
                    }
                }
                KeyCode::Backspace => {
                    if let Some(field_config) =
                        app.form_state.fields.get_mut(app.form_state.current_field)
                    {
                        match field_config.value {
                            FieldValue::Text(ref mut field_text) => {
                                let mut chars: Vec<char> = field_text.chars().collect();
                                let cursor = app.form_state.cursor_index;

                                if cursor > 0 && cursor <= chars.len() {
                                    chars.remove(cursor - 1);
                                    *field_text = chars.into_iter().collect();
                                    app.form_state.cursor_index -= 1;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                KeyCode::Delete => {
                    if let Some(field_config) =
                        app.form_state.fields.get_mut(app.form_state.current_field)
                    {
                        match field_config.value {
                            FieldValue::Text(ref mut field_text) => {
                                let mut chars: Vec<char> = field_text.chars().collect();
                                let cursor = app.form_state.cursor_index;

                                if cursor < chars.len() {
                                    chars.remove(cursor);
                                    *field_text = chars.into_iter().collect();
                                }
                            }
                            _ => {}
                        }
                    }
                }
                KeyCode::F(2) => {
                    for field in &mut app.form_state.fields {
                        field.show_password = !field.show_password;
                    }
                }
                KeyCode::Enter => {
                    app.submit_form();
                }
                KeyCode::Esc => {
                    app.form_state.clear();
                    app.state = AppState::EntriesList;
                }
                _ => {}
            }
        }

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
                            app.form_state.fields = fields;
                            app.open_form(AppState::ChangeMasterPassword, None);
                        }
                        SettingsOption::SnapshotList => {
                            app.load_snapshots();
                            app.state = AppState::SnapshotList
                        }
                        SettingsOption::PasswordGenerator => {
                            let config =
                                app.generator_session_config.clone().unwrap_or_else(|| {
                                    let loaded = app.load_generator_config();
                                    app.generator_session_config = Some(loaded.clone());
                                    loaded
                                });

                            let pass_generator = PasswordGeneratorState {
                                session_config: config,
                                generated_password: None,
                            };

                            app.form_state.clear();
                            app.form_state.fields = pass_generator.build_form_fields();
                            app.state = AppState::PasswordGenerator(pass_generator);
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

        AppState::PasswordGenerator(_) => match key.code {
            KeyCode::Esc => {
                if let AppState::PasswordGenerator(_state) = &mut app.state {
                    app.state = AppState::VaultLocked;
                }
            }
            KeyCode::Enter => {
                if let AppState::PasswordGenerator(state) = &mut app.state {
                    if let Some(pwd) = state.generated_password.clone() {
                        app.copy_to_clipboard(pwd.to_string());
                    }
                }
            }
            KeyCode::Char('r') => {
                if let AppState::PasswordGenerator(ref mut state) = app.state {
                    state.sync_config(&app.form_state.fields);
                    app.generator_session_config = Some(state.session_config.clone());
                    let _ = state.generate_password();
                }
            }
            KeyCode::Tab => {
                if let AppState::PasswordGenerator(_) = &mut app.state {
                    app.form_state.current_field =
                        (app.form_state.current_field + 1) % app.form_state.fields.len();
                    app.form_state.cursor_index = 0;
                }
            }
            KeyCode::BackTab => {
                if let AppState::PasswordGenerator(_) = &mut app.state {
                    if app.form_state.current_field == 0 {
                        app.form_state.current_field = app.form_state.fields.len() - 1;
                    } else {
                        app.form_state.current_field -= 1;
                    }
                    app.form_state.cursor_index = 0;
                }
            }
            KeyCode::Right | KeyCode::Char(' ') => {
                if let AppState::PasswordGenerator(_) = &mut app.state {
                    if let Some(field_config) =
                        app.form_state.fields.get_mut(app.form_state.current_field)
                    {
                        match field_config.value {
                            FieldValue::Boolean(b) => {
                                field_config.value = FieldValue::Boolean(!b);
                            }
                            FieldValue::Choice(ref mut idx) => {
                                if let FieldType::Choice(enum_choice) = &field_config.field_type {
                                    let max_options = enum_choice.options().len();

                                    *idx = (*idx + 1) % max_options;
                                }
                            }
                            FieldValue::Text(ref text) => {
                                if app.form_state.cursor_index < text.len() {
                                    app.form_state.cursor_index += 1;
                                }
                            }
                            FieldValue::Number(ref mut num) => {
                                if *num < MAX_PASSWORD_LENGTH as u16 {
                                    *num += 1;
                                }
                            }
                        }
                    }
                }
            }
            KeyCode::Left => {
                if let AppState::PasswordGenerator(_) = &mut app.state {
                    if let Some(field_config) =
                        app.form_state.fields.get_mut(app.form_state.current_field)
                    {
                        match field_config.value {
                            FieldValue::Text(ref text) => {
                                if text.len() > 0 && app.form_state.cursor_index > 0 {
                                    app.form_state.cursor_index -= 1;
                                }
                            }
                            FieldValue::Choice(ref mut idx) => {
                                if let FieldType::Choice(enum_choice) = &field_config.field_type {
                                    let max_options = enum_choice.options().len();

                                    *idx = (*idx + max_options - 1) % max_options;
                                }
                            }
                            FieldValue::Boolean(ref b) => {
                                field_config.value = FieldValue::Boolean(!b);
                            }
                            FieldValue::Number(ref mut num) => {
                                if *num > 0 {
                                    *num -= 1;
                                }
                            }
                        }
                    }
                }
            }
            KeyCode::Down => {
                app.form_state.current_field =
                    (app.form_state.current_field + 1) % app.form_state.fields.len();
                app.form_state.cursor_index = 0;
            }
            KeyCode::Up => {
                if app.form_state.current_field == 0 {
                    app.form_state.current_field = app.form_state.fields.len() - 1;
                } else {
                    app.form_state.current_field -= 1;
                }
                app.form_state.cursor_index = 0;
            }
            KeyCode::Char(c) => {
                if let AppState::PasswordGenerator(_) = &mut app.state {
                    if let Some(field_config) =
                        app.form_state.fields.get_mut(app.form_state.current_field)
                    {
                        match field_config.value {
                            FieldValue::Text(ref mut text) => {
                                text.insert(app.form_state.cursor_index, c);
                                app.form_state.cursor_index += 1;
                            }
                            FieldValue::Number(ref mut num) => {
                                if let Some(digit) = c.to_digit(10) {
                                    if let Some(new_val) = num
                                        .checked_mul(10)
                                        .and_then(|n| n.checked_add(digit as u16))
                                    {
                                        if new_val <= MAX_PASSWORD_LENGTH as u16 {
                                            *num = new_val;
                                        } else {
                                            app.set_status("Limite máximo atingido!".into());
                                        }
                                    } else {
                                        app.set_status("Número muito grande!".into());
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(field_config) =
                    app.form_state.fields.get_mut(app.form_state.current_field)
                {
                    match field_config.value {
                        FieldValue::Number(ref mut num) => {
                            let mut num_str = num.to_string();
                            if num_str.len() > 1 && app.form_state.cursor_index > 0 {
                                num_str.remove(app.form_state.cursor_index - 1);
                                *num = num_str.parse().unwrap();
                            } else {
                                *num = 0;
                            }
                        }
                        FieldValue::Text(ref mut text) => {
                            if app.form_state.cursor_index > 0 {
                                text.remove(app.form_state.cursor_index - 1);
                                app.form_state.cursor_index -= 1;
                            }
                        }

                        _ => {}
                    }
                }
            }
            KeyCode::Delete => {
                if let Some(field_config) =
                    app.form_state.fields.get_mut(app.form_state.current_field)
                {
                    match field_config.value {
                        FieldValue::Text(ref mut text) => {
                            if app.form_state.cursor_index < text.len() {
                                text.remove(app.form_state.cursor_index);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        },
    }
}
