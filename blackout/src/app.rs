use blackout_core::ipc::{
    EntryInput, EntryUpdateInput, Request, Response, VaultListPayload, VaultSnapshotPayload,
};
use blackout_core::vault::{Entry, VaultSnapshot};

use chrono::{DateTime, Local};

use ratatui::widgets::{ListState, TableState};

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;

pub struct SnapshotView {
    pub version: u32,
    pub created_at: DateTime<Local>,
    pub checksum: String,
    pub reason: String,
}

pub trait EntryView {
    fn _id(&self) -> &uuid::Uuid;
    fn service(&self) -> &str;
    fn username(&self) -> &str;
    fn updated_at(&self) -> DateTime<Local>;
}

#[derive(Debug, Clone)]
pub struct ListEntryView(pub Entry);

impl EntryView for ListEntryView {
    fn _id(&self) -> &uuid::Uuid {
        &self.0.id
    }
    fn service(&self) -> &str {
        &self.0.service
    }
    fn username(&self) -> &str {
        &self.0.username
    }
    fn updated_at(&self) -> DateTime<Local> {
        self.0.updated_at
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DetailEntryView {
    pub entry: Entry,
    pub show_password: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldConfig {
    pub label: String,
    pub is_password: bool,
    pub show_password: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SettingsOption {
    ChangeMasterPassword,
    SnapshotList,
}

impl SettingsOption {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ChangeMasterPassword => "Change Master Password",
            Self::SnapshotList => "Snapshots",
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct SettingsState {
    pub list_state: ListState,
    pub options: Vec<SettingsOption>,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            list_state: ListState::default(),
            options: vec![
                SettingsOption::ChangeMasterPassword,
                SettingsOption::SnapshotList,
            ],
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum AppState {
    InitialCheck,
    UnlockPrompt,
    VaultLocked,
    EntriesList,
    NewEntryForm(Vec<FieldConfig>),
    ViewEntry(DetailEntryView),
    UpdateEntry(Vec<FieldConfig>),
    ConfirmEntryDelete,
    Settings(SettingsState),
    ChangeMasterPassword(Vec<FieldConfig>),
    SnapshotList,
}

pub struct App {
    pub state: AppState,
    pub vault_unlocked: bool,
    pub entries: Vec<Entry>,
    pub detail_entry: Option<DetailEntryView>,
    pub input_buffer: String, // For password input
    pub table_state: TableState,
    pub status_message: Option<String>,
    pub status_time: Option<Instant>,
    pub last_interaction: Instant,
    pub _last_tick: std::time::Instant,
    pub vault_version: u32,
    pub form_fields: Vec<String>,
    pub current_field: usize,
    pub obscure_inputs: bool,
    pub snapshots: Vec<VaultSnapshot>,
}

impl App {
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        Self {
            state: AppState::InitialCheck,
            vault_unlocked: false,
            entries: vec![],
            input_buffer: String::new(),
            current_field: 0,
            detail_entry: None,
            table_state,
            status_message: None,
            status_time: Some(Instant::now()),
            last_interaction: Instant::now(),
            _last_tick: Instant::now(),
            vault_version: 0,
            form_fields: vec![String::new(), String::new(), String::new()],
            obscure_inputs: true,
            snapshots: vec![],
        }
    }

    pub fn set_status(&mut self, msg: String) {
        self.status_message = Some(msg);
        self.status_time = Some(Instant::now());
    }

    pub fn get_selected_entry(&self) -> Option<&Entry> {
        self.table_state
            .selected()
            .and_then(|i| self.entries.get(i))
    }

    pub fn check_vault_status(&mut self) {
        if matches!(
            self.state,
            AppState::UnlockPrompt | AppState::UpdateEntry(_) | AppState::NewEntryForm(_)
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
        self.detail_entry = None;
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

    pub fn unlock_vault(&mut self, password: String) {
        match crate::send_command(Request::Unlock {
            master_password: password,
        }) {
            Ok(Response::Ok(_)) => {
                self.vault_unlocked = true;
                self.state = AppState::EntriesList;
                self.load_entries();
            }
            Ok(Response::Error(e)) => {
                self.vault_unlocked = false;
                self.set_status(e.to_string());
                self.state = AppState::UnlockPrompt;
            }
            Err(_) => {}
        }
    }

    pub fn lock_vault(&mut self) {
        let _ = crate::send_command(Request::Lock);
        self.vault_unlocked = false;
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
            self.table_state.select(Some(
                (self.table_state.selected().unwrap_or(0) + 1) % max_index,
            ));
        }
    }

    pub fn prev_index(&mut self) {
        let max_index = self.cal_max_index();
        if max_index > 0 {
            let current_index = self.table_state.selected().unwrap_or(0);
            if current_index == 0 {
                self.table_state.select(Some(max_index - 1));
            } else {
                self.table_state.select(Some(current_index - 1));
            }
        }
    }

    pub fn cal_max_index(&mut self) -> usize {
        let mut max_index = 0;
        match &self.state {
            AppState::EntriesList => max_index = self.entries.len(),
            AppState::SnapshotList => max_index = self.snapshots.len(),
            _ => {}
        }

        max_index
    }

    pub fn get_input_for_field(&self, index: usize) -> &str {
        self.form_fields
            .get(index)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    pub fn submit_form_update(&mut self) {
        let Some(entry) = self.get_selected_entry() else {
            self.set_status("No entry selected for update".into());
            return;
        };

        let uuid = entry.id;

        let service = &self.form_fields[0];
        let user = &self.form_fields[1];
        let password = &self.form_fields[2];

        let entry_ctx = EntryUpdateInput {
            uuid,
            service: Some(service.clone()),
            username: Some(user.clone()),
            password: Some(password.clone()),
        };

        match crate::send_command(Request::UpdateEntry { entry_ctx }) {
            Ok(Response::Ok(_)) => {
                self.load_entries();
                self.state = AppState::EntriesList;
                self.set_status("Entry successfully added!".into());
                self.reset_form();
            }
            Ok(Response::Error(e)) => self.log_error("Add Entry", e),
            Err(_) => self.set_status("Communication error".into()),
        }
    }

    pub fn submit_form_add(&mut self) {
        let service = &self.form_fields[0];
        let user = &self.form_fields[1];
        let password = &self.form_fields[2];

        if service.is_empty() || user.is_empty() || password.is_empty() {
            self.set_status("All fields are required!".into());
            return;
        }

        let entry_ctx = EntryInput {
            service: service.clone(),
            username: user.clone(),
            password: password.clone(),
        };

        match crate::send_command(Request::AddEntry { entry_ctx }) {
            Ok(Response::Ok(_)) => {
                self.load_entries();
                self.state = AppState::EntriesList;
                self.set_status("Entry successfully added!".into());
                self.reset_form();
            }
            Ok(Response::Error(e)) => self.log_error("Add Entry", e),
            Err(_) => self.set_status("Communication error".into()),
        }
    }

    pub fn submit_form_update_master_password(&mut self) {
        let old = &self.form_fields[0];
        let new = &self.form_fields[1];
        let confirm = &self.form_fields[2];

        if new.is_empty() || old.is_empty() {
            self.set_status("Fields cannot be empty!".into());
            return;
        }

        if new != confirm {
            self.set_status("New passwords do not match!".into());
            return;
        }

        match crate::send_command(Request::UpdateMasterPassword {
            new_password: new.clone(),
        }) {
            Ok(Response::Ok(_)) => {
                self.state = AppState::EntriesList;
                self.set_status("Master password updated!".into());
                self.reset_form();
            }
            Ok(Response::Error(e)) => self.log_error("Update Master Pass", e),
            Err(_) => self.set_status("Communication error".into()),
        }
    }

    pub fn submit_form(&mut self) {
        match &self.state {
            AppState::NewEntryForm(_) => self.submit_form_add(),
            AppState::UpdateEntry(_) => self.submit_form_update(),
            AppState::ChangeMasterPassword(_) => self.submit_form_update_master_password(),
            _ => {}
        }
    }

    fn log_error(&mut self, action: &str, err: String) {
        let debug_info = format!(
            "{} Error: {}\nData: {}",
            action,
            err,
            self.form_fields.join(",")
        );
        let _ = std::fs::write("blackout_debug.txt", debug_info);
        self.set_status(format!("{} failed. Check debug log.", action));
    }

    pub fn reset_form(&mut self) {
        for field in self.form_fields.iter_mut() {
            field.clear();
        }
        self.current_field = 0;
        self.table_state.select(Some(0));
    }

    pub fn delete_selected_entry(&mut self) {
        let Some(entry) = self.get_selected_entry() else {
            self.set_status("No entry selected for update".into());
            return;
        };

        let uuid = entry.id;
        match crate::send_command(Request::DeleteEntry { uuid }) {
            Ok(Response::Ok(_)) => {
                self.load_entries();
                self.state = AppState::EntriesList;
                self.set_status("Entry successfully deleted!".into());
            }
            Ok(Response::Error(e)) => {
                let debug_info = format!("Delete Error: {}\nID:{}", e, uuid);
                let _ = std::fs::write("blackout_debug.txt", debug_info);
            }
            Err(_) => {}
        }
    }

    pub fn view_selected_entry(&mut self) {
        let Some(entry) = self.get_selected_entry() else {
            self.set_status("No entry selected for update".into());
            return;
        };
        let uuid = entry.id;
        if let Ok(Response::Ok(data)) = crate::send_command(Request::GetEntryById { uuid }) {
            if let Ok(entry) = serde_json::from_str::<Entry>(&data) {
                let entry_clone = entry.clone();
                self.detail_entry = Some(DetailEntryView {
                    entry: entry_clone,
                    show_password: false,
                });
                self.state = AppState::ViewEntry(DetailEntryView {
                    entry,
                    show_password: false,
                });
            }
        } else {
            let debug_info = format!(
                "View Entry Error: Failed to get entry details for ID: {}",
                uuid
            );
            let _ = std::fs::write("blackout_debug.txt", debug_info);
        }
    }

    pub fn populate_form(&mut self) {
        let Some(entry) = self.get_selected_entry() else {
            self.set_status("No entry selected for update".into());
            return;
        };
        let uuid = entry.id;
        if let Ok(Response::Ok(data)) = crate::send_command(Request::GetEntryById { uuid }) {
            if let Ok(entry) = serde_json::from_str::<Entry>(&data) {
                self.form_fields[0] = entry.service.to_string();
                self.form_fields[1] = entry.username.to_string();
                self.form_fields[2] = entry.secret.to_string();
            }
        } else {
            self.set_status("failed to load datails. Check debug log.".into());
        }
    }

    pub fn open_form(&mut self, new_state: AppState, populate: bool) {
        if populate {
            self.populate_form();
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
                self.set_status("Password copied to clipboard!".into());
            }
            Err(e) => {
                let _ = std::fs::write("blackout_debug.txt", format!("Erro wl-copy: {}", e));
                self.set_status("Failed to copy (missing wl-copy)".into());
            }
        }
    }
}
