use blackout_core::generator::GeneratorConfig;
use blackout_core::ipc::{
    EntryInput, EntryUpdateInput, Request, Response, VaultListPayload, VaultSnapshotPayload,
};

use blackout_core::vault::{Entry, VaultSnapshot};

use crate::state::{
    FieldConfig, FieldType, FieldValue, FormState, PasswordGeneratorState, PendingAction,
    SelectedItem, SettingsState,
};

use ratatui::widgets::TableState;

use std::env;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;

#[derive(PartialEq, Debug, Clone)]
pub enum AppState {
    InitialCheck,
    UnlockPrompt,
    VaultLocked,
    EntriesList,
    NewEntryForm,
    ViewEntry(uuid::Uuid),
    UpdateEntry,
    Settings(SettingsState),
    ChangeMasterPassword,
    SnapshotList,
    ConfirmAction {
        action: PendingAction,
        previous_state: Box<AppState>,
    },
    PasswordGenerator(PasswordGeneratorState),
}

pub struct App {
    pub state: AppState,
    pub vault_unlocked: bool,
    pub entries: Vec<Entry>,
    pub table_state: TableState,
    pub status_message: Option<String>,
    pub status_time: Option<Instant>,
    pub last_interaction: Instant,
    pub _last_tick: std::time::Instant,
    pub vault_version: u32,
    pub form_state: FormState,
    pub snapshots: Vec<VaultSnapshot>,
    pub dev_mode: bool,
    pub generator_session_config: Option<GeneratorConfig>,
}

impl App {
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        let args: Vec<String> = env::args().collect();
        let dev_mode = args.contains(&"--dev".to_string());

        Self {
            state: AppState::InitialCheck,
            vault_unlocked: false,
            entries: vec![],
            table_state,
            status_message: None,
            status_time: Some(Instant::now()),
            last_interaction: Instant::now(),
            _last_tick: Instant::now(),
            vault_version: 0,
            form_state: FormState::new(),
            snapshots: vec![],
            dev_mode: dev_mode,
            generator_session_config: None,
        }
    }

    pub fn set_status(&mut self, msg: String) {
        self.status_message = Some(msg);
        self.status_time = Some(Instant::now());
    }

    pub fn get_selected_item(&self) -> Option<SelectedItem<'_>> {
        match &self.state {
            AppState::EntriesList | AppState::UpdateEntry => self
                .table_state
                .selected()
                .and_then(|i| self.entries.get(i))
                .map(SelectedItem::Entry),

            AppState::SnapshotList => self
                .table_state
                .selected()
                .and_then(|visual_index| {
                    // Convert visual index to real index (fix: reverse table)
                    let len = self.snapshots.len();
                    if len == 0 || visual_index >= len {
                        return None;
                    }
                    let real_index = len - 1 - visual_index;
                    self.snapshots.get(real_index)
                })
                .map(SelectedItem::Snapshot),

            _ => None,
        }
    }

    pub fn check_vault_status(&mut self) {
        if matches!(
            self.state,
            AppState::UnlockPrompt | AppState::UpdateEntry | AppState::NewEntryForm
        ) {
            return;
        }

        match crate::send_command(Request::ListEntries) {
            Ok(response) => match response {
                Response::Ok(data) => {
                    if self.state == AppState::InitialCheck || self.state == AppState::VaultLocked {
                        self.parse_entries(&data);
                        self.state = AppState::EntriesList;
                        self.vault_unlocked = true;
                    }
                }
                Response::Error(err) if err.contains("Vault is locked") => {
                    if self.state != AppState::VaultLocked {
                        self.lock_application();
                    }
                }
                _ => {}
            },
            Err(_) => {
                if self.state != AppState::VaultLocked {
                    self.lock_application();
                }
            }
        }
    }

    pub fn lock_application(&mut self) {
        self.vault_unlocked = false;
        self.entries.clear();
        self.status_message = None;
        self.state = AppState::VaultLocked;
    }

    fn parse_entries(&mut self, data: &str) {
        match serde_json::from_str::<VaultListPayload>(data) {
            Ok(payload) => {
                self.entries = payload.entries;
                self.vault_version = payload.version;
                if self.table_state.selected().is_none() && !self.entries.is_empty() {
                    self.table_state.select(Some(0));
                }
            }
            Err(e) => {
                let debug_info = format!("Parser Error: {}\n\nReceived Data:\n{}", e, data);
                let _ = std::fs::write("blackout_debug.txt", debug_info);
            }
        }
    }

    pub fn unlock_vault(&mut self, password: &str) -> bool {
        match crate::send_command(Request::Unlock {
            master_password: password.to_owned(),
        }) {
            Ok(Response::Ok(_)) => {
                self.vault_unlocked = true;
                self.state = AppState::EntriesList;
                self.load_entries();
                true
            }
            Ok(Response::Error(e)) => {
                self.vault_unlocked = false;
                self.set_status(e.to_string());
                false
            }
            Err(_) => {
                self.set_status("Communication error".into());
                false
            }
        }
    }

    pub fn lock_vault(&mut self) {
        let _ = crate::send_command(Request::Lock);
        self.vault_unlocked = false;
        self.form_state.clear();
        self.form_state.fields = vec![FieldConfig::password("Password")];
        self.state = AppState::UnlockPrompt;
    }

    pub fn load_entries(&mut self) {
        if let Ok(Response::Ok(data)) = crate::send_command(Request::ListEntries) {
            self.parse_entries(&data);
        }
    }

    pub fn load_snapshots(&mut self) {
        if let Ok(Response::Ok(data)) = crate::send_command(Request::ListSnapshots) {
            match serde_json::from_str::<VaultSnapshotPayload>(&data) {
                Ok(payload) => {
                    self.snapshots = payload.snapshots;
                }
                Err(e) => {
                    let debug_info = format!("Parser Error: {}\n\nReceived Data:\n{}", e, data);
                    let _ = std::fs::write("blackout_debug.txt", debug_info);
                }
            }
        }
    }

    pub fn next_index(&mut self) {
        let max_index = self.cal_max_index();
        if max_index > 0 {
            let next = self
                .table_state
                .selected()
                .map_or(0, |i| (i + 1) % max_index);
            self.table_state.select(Some(next));
        }
    }

    pub fn prev_index(&mut self) {
        let max_index = self.cal_max_index();
        if max_index > 0 {
            let prev = self.table_state.selected().map_or(max_index - 1, |i| {
                if i == 0 {
                    max_index - 1 // Go to end
                } else {
                    i - 1
                }
            });
            self.table_state.select(Some(prev));
        }
    }

    pub fn cal_max_index(&self) -> usize {
        match self.state {
            AppState::EntriesList => self.entries.len(),
            AppState::SnapshotList => self.snapshots.len(),
            _ => 0,
        }
    }

    pub fn get_input_for_field(&self, index: usize) -> &str {
        match self.form_state.fields.get(index) {
            Some(field) => match &field.value {
                FieldValue::Text(text) => text.as_str(),
                FieldValue::Choice(_) => "",
                FieldValue::Boolean(_) => "",
                FieldValue::Number(_) => "",
            },
            None => "",
        }
    }

    pub fn submit_form_update(&mut self) {
        let Some(item) = self.get_selected_item() else {
            self.set_status("No entry selected for update".into());
            return;
        };

        let uuid = match item {
            SelectedItem::Entry(entry) => entry.uuid,
            _ => {
                self.set_status("Type mismatch: Selected item is not an entry".into());
                return;
            }
        };

        let mut entry_ctx = EntryUpdateInput {
            uuid,
            service: None,
            username: None,
            password: None,
        };

        for field in &self.form_state.fields {
            if let FieldValue::Text(ref text) = field.value {
                match field.field_type {
                    FieldType::Service => entry_ctx.service = Some(text.clone()),
                    FieldType::Username => entry_ctx.username = Some(text.clone()),
                    FieldType::Password => entry_ctx.password = Some(text.clone()),
                    _ => {}
                }
            }
        }

        match crate::send_command(Request::UpdateEntry { entry_ctx }) {
            Ok(Response::Ok(_)) => {
                self.load_entries();
                self.state = AppState::EntriesList;
                self.set_status("Entry successfully added!".into());
                self.form_state.clear();
            }
            Ok(Response::Error(e)) => self.log_error("Add Entry", e),
            Err(_) => self.set_status("Communication error".into()),
        }
    }

    pub fn submit_form_add(&mut self) {
        let mut entry_ctx = EntryInput {
            service: String::new(),
            username: String::new(),
            password: String::new(),
        };

        for field in &self.form_state.fields {
            if let FieldValue::Text(text) = &field.value {
                match field.field_type {
                    FieldType::Service => entry_ctx.service = text.clone(),
                    FieldType::Username => entry_ctx.username = text.clone(),
                    FieldType::Password => entry_ctx.password = text.clone(),
                    _ => {}
                }
            }
        }

        if entry_ctx.service.is_empty()
            || entry_ctx.username.is_empty()
            || entry_ctx.password.is_empty()
        {
            self.set_status("All fields are required!".into());
            return;
        }

        match crate::send_command(Request::AddEntry { entry_ctx }) {
            Ok(Response::Ok(_)) => {
                self.load_entries();
                self.state = AppState::EntriesList;
                self.set_status("Entry successfully added!".into());
                self.form_state.clear();
            }
            Ok(Response::Error(e)) => self.log_error("Add Entry", e),
            Err(_) => self.set_status("Communication error".into()),
        }
    }

    pub fn submit_form_update_master_password(&mut self) {
        let old_password = if let FieldValue::Text(text) = &self.form_state.fields[1].value {
            text.clone()
        } else {
            String::new()
        };

        let new_password = if let FieldValue::Text(text) = &self.form_state.fields[2].value {
            text.clone()
        } else {
            String::new()
        };

        if new_password.is_empty() || old_password.is_empty() {
            self.set_status("Fields cannot be empty!".into());
            return;
        }

        let confirm_password = if let FieldValue::Text(text) = &self.form_state.fields[3].value {
            text.clone()
        } else {
            String::new()
        };

        if new_password.is_empty() || old_password.is_empty() {
            self.set_status("Fields cannot be empty!".into());
            return;
        }

        if new_password != confirm_password {
            self.set_status("New passwords do not match!".into());
            return;
        }

        if new_password == old_password {
            self.set_status("New password cannot be the same!".into());
            return;
        }

        let pass_valid = self.unlock_vault(&old_password);
        if !pass_valid {
            self.set_status("Master Password invalid!".into());
            return;
        }

        match crate::send_command(Request::UpdateMasterPassword { new_password }) {
            Ok(Response::Ok(_)) => {
                self.state = AppState::EntriesList;
                self.set_status("Master password updated!".into());
                self.form_state.clear();
            }
            Ok(Response::Error(e)) => self.log_error("Update Master Pass", e),
            Err(_) => self.set_status("Communication error".into()),
        }
    }

    pub fn submit_form(&mut self) {
        match &self.state {
            AppState::NewEntryForm => self.submit_form_add(),
            AppState::UpdateEntry => self.submit_form_update(),
            AppState::ChangeMasterPassword => self.submit_form_update_master_password(),
            _ => {}
        }
    }

    fn log_error(&mut self, action: &str, err: String) {
        let debug_info = format!(
            "{} Error: {}\nData: {}",
            action,
            err,
            self.form_state
                .fields
                .iter()
                .map(|f| f.label)
                .collect::<Vec<_>>()
                .join(",")
        );
        let _ = std::fs::write("blackout_debug.txt", debug_info);
        self.set_status(format!("{} failed. Check debug log.", action));
    }

    pub fn delete_entry(&mut self, uuid: uuid::Uuid) {
        match crate::send_command(Request::DeleteEntry { uuid }) {
            Ok(Response::Ok(_)) => {
                self.load_entries();
                self.state = AppState::EntriesList;
                self.set_status("Entry successfully deleted!".into());
            }
            Ok(Response::Error(e)) => self.log_error("Delete", e),
            Err(_) => {}
        }
    }

    pub fn restore_snapshot(&mut self, uuid: uuid::Uuid, version: Option<u32>) {
        let version_default = version.unwrap_or(
            self.snapshots
                .iter()
                .filter(|s| s.uuid == uuid)
                .max_by_key(|s| s.version)
                .map(|s| s.version)
                .unwrap_or(0),
        );

        match crate::send_command(Request::RestoreSnapshot {
            version: version.unwrap_or(version_default),
            uuid,
        }) {
            Ok(Response::Ok(_)) => {
                self.load_entries();
                self.state = AppState::EntriesList;
                self.set_status(format!("Snapshot v{} restored!", version.unwrap_or(0)));
            }
            Ok(Response::Error(e)) => self.log_error("Restore Snapshot", e),
            Err(_) => {}
        }
    }

    pub fn populate_form(&mut self, uuid: uuid::Uuid) {
        if let Ok(Response::Ok(data)) = crate::send_command(Request::GetEntryById { uuid }) {
            if let Ok(entry) = serde_json::from_str::<Entry>(&data) {
                self.form_state.fields = vec![
                    FieldConfig {
                        value: FieldValue::Text(entry.service),
                        ..FieldConfig::service("Service")
                    },
                    FieldConfig {
                        value: FieldValue::Text(entry.username),
                        ..FieldConfig::username("Username")
                    },
                    FieldConfig {
                        value: FieldValue::Text(entry.secret),
                        ..FieldConfig::password("Password")
                    },
                ];

                self.form_state.current_field = 0;
                self.form_state.cursor_index = 0;
            } else {
                self.set_status("Failed to parse entry details. Check debug log.".into());
            }
        } else {
            self.set_status("Failed to load details. Check debug log.".into());
        }
    }

    pub fn open_form(&mut self, new_state: AppState, uuid: Option<uuid::Uuid>) {
        if let Some(uuid) = uuid {
            self.populate_form(uuid);
        } else {
            self.form_state.clear();
        }

        self.state = new_state;
    }

    pub fn copy_to_clipboard(&mut self, text: String) {
        let child = Command::new("wl-copy")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        match child {
            Ok(mut process) => {
                if let Some(mut stdin) = process.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                self.set_status("Copied to clipboard!".into());
            }
            Err(e) => {
                let _ = std::fs::write("blackout_debug.txt", format!("Erro wl-copy: {}", e));
                self.set_status("Failed to copy (missing wl-copy)".into());
            }
        }
    }

    pub fn get_selected_entry_id(&mut self) -> Option<uuid::Uuid> {
        let item = self.get_selected_item()?;

        match item {
            SelectedItem::Entry(entry) => Some(entry.uuid),
            _ => {
                self.set_status("Type mismatch: Selected item is not an entry".into());
                None
            }
        }
    }

    pub fn get_selected_snapshot_uuid(&mut self) -> Option<uuid::Uuid> {
        let item = self.get_selected_item()?;

        match item {
            SelectedItem::Snapshot(snap) => Some(snap.uuid),
            _ => {
                self.set_status("Type mismatch: Selected item is not a snapshot".into());
                None
            }
        }
    }

    pub fn get_selected_snapshot_version(&mut self) -> Option<u32> {
        let item = self.get_selected_item()?;

        match item {
            SelectedItem::Snapshot(snap) => Some(snap.version),
            _ => {
                self.set_status("Type mismatch: Selected item is not a snapshot".into());
                None
            }
        }
    }

    pub fn load_generator_config(&mut self) -> GeneratorConfig {
        match crate::send_command(Request::LoadGeneratorConfig) {
            Ok(Response::Ok(config)) => {
                serde_json::from_str(&config).unwrap_or_else(|_| GeneratorConfig::default())
            }
            Ok(_) => {
                self.set_status("Unexpected response from daemon".into());
                GeneratorConfig::default()
            }
            Err(e) => {
                self.set_status(format!("Failed to load generator config: {}", e));
                GeneratorConfig::default()
            }
        }
    }

    pub fn auto_fill_password(&mut self) {
        let config = self.load_generator_config();
        if let AppState::NewEntryForm | AppState::UpdateEntry = self.state {
            let password_field = self
                .form_state
                .fields
                .iter_mut()
                .find(|f| matches!(f.field_type, FieldType::Password));

            if let Some(field) = password_field {
                if let Ok(new_password) = blackout_core::generator::generate(config) {
                    field.value = FieldValue::Text(new_password);
                    self.set_status("Password auto-filled!".into());
                }
            } else {
                self.set_status("No password field found in this form".into());
            }
        }
    }
}
