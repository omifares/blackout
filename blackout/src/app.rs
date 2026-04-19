use blackout_core::ipc::{EntryInput, Request, Response, VaultListPayload};
use blackout_core::vault::Entry;

use chrono::{DateTime, Local};

use ratatui::widgets::{TableState, ListState};

use std::process::{Command, Stdio};
use std::io::Write;
use std::time::Instant;

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
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SettingsOption {
    ChangeMasterPassword,
    A,
    B,
}

impl SettingsOption {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ChangeMasterPassword => "Change Master Password",
            Self::A => "Not implemented",
            Self::B => "Not implemented",
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
                SettingsOption::A,
                SettingsOption::B,
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
}

pub struct App {
    pub state: AppState,
    pub vault_unlocked: bool,
    pub entries: Vec<Entry>,
    pub detail_entry: Option<DetailEntryView>,
    pub input_buffer: String,     // For password input
    pub table_state: TableState,
    pub status_message: Option<String>,
    pub last_interaction: Instant,
    pub last_tick: std::time::Instant,
    pub vault_version: u32,
    pub form_fields: Vec<String>,
    pub current_field: usize,
    pub obscure_inputs: bool,
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
            last_interaction: Instant::now(),
            last_tick: Instant::now(),
            vault_version: 0,
            form_fields: vec![String::new(), String::new(), String::new()], // Agora é Vec (Resolve E0308)
            obscure_inputs: true,
        }
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
            Ok(Response::Error(_)) => {
                self.vault_unlocked = false;
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

    pub fn next_entry(&mut self) {
        if !self.entries.is_empty() {
            self.table_state.select(Some(
                (self.table_state.selected().unwrap_or(0) + 1) % self.entries.len(),
            ));
        }
    }

    pub fn prev_entry(&mut self) {
        if !self.entries.is_empty() {
            let current_index = self.table_state.selected().unwrap_or(0);
            if current_index == 0 {
                self.table_state.select(Some(self.entries.len() - 1));
            } else {
                self.table_state.select(Some(current_index - 1));
            }
        }
    }

    pub fn get_input_for_field(&self, index: usize) -> &str {
        self.form_fields.get(index).map(|s| s.as_str()).unwrap_or("")
    }

    pub fn submit_form(&mut self) {
        match &self.state {
            AppState::NewEntryForm(_) => {
                let service = &self.form_fields[0];
                let user = &self.form_fields[1];
                let password = &self.form_fields[2];

                if service.is_empty() || user.is_empty() || password.is_empty() {
                    self.status_message = Some("All fields are required!".to_string());
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
                        self.status_message = Some("Entry successfully added!".to_string());
                        self.reset_form();
                    }
                    Ok(Response::Error(e)) => self.log_error("Add Entry", e),
                    Err(_) => self.status_message = Some("Communication error".into()),
                }
            }

            AppState::ChangeMasterPassword(_) => {
                let old = &self.form_fields[0];
                let new = &self.form_fields[1];
                let confirm = &self.form_fields[2];

                if new.is_empty() || old.is_empty() {
                    self.status_message = Some("Fields cannot be empty!".into());
                    return;
                }

                if new != confirm {
                    self.status_message = Some("New passwords do not match!".into());
                    return;
                }

                match crate::send_command(Request::UpdateMasterPassword {
                    new_password: new.clone()
                }) {
                    Ok(Response::Ok(_)) => {
                        self.state = AppState::EntriesList;
                        self.status_message = Some("Master password updated!".into());
                        self.reset_form();
                    }
                    Ok(Response::Error(e)) => self.log_error("Update Master Pass", e),
                    Err(_) => self.status_message = Some("Communication error".into()),
                }
            }

            _ => {}
        }
    }

    fn log_error(&mut self, action: &str, err: String) {
        let debug_info = format!(
            "{} Error: {}\nData: {}",
            action, err, self.form_fields.join(",")
        );
        let _ = std::fs::write("blackout_debug.txt", debug_info);
        self.status_message = Some(format!("❌ {} failed. Check debug log.", action));
    }

    pub fn reset_form(&mut self) {
        for field in self.form_fields.iter_mut() {
            field.clear();
        }
        self.current_field = 0;
        self.table_state.select(Some(0));
        self.status_message = None;
    }

    pub fn delete_selected_entry(&mut self) {
        if let Some(entry) = self.get_selected_entry() {
            let uuid = entry.id;
            match crate::send_command(Request::DeleteEntry { uuid: uuid }) {
                Ok(Response::Ok(_)) => {
                    self.load_entries();
                    self.state = AppState::EntriesList;
                    self.status_message = Some("Entry successfully deleted!".to_string());
                }
                Ok(Response::Error(e)) => {
                    let debug_info = format!("Delete Error: {}\nID:{}", e, uuid);
                    let _ = std::fs::write("blackout_debug.txt", debug_info);
                }
                Err(_) => {}
            }
        }
    }

    pub fn view_selected_entry(&mut self) {
        if let Some(entry) = self.get_selected_entry() {
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
    }

    pub fn populate_form(&mut self) {
        if let Some(entry) = self.get_selected_entry() {
            let uuid = entry.id;

            if let Ok(Response::Ok(data)) = crate::send_command(Request::GetEntryById { uuid }) {
                if let Ok(entry) = serde_json::from_str::<Entry>(&data) {
                    self.form_fields[0] = entry.service.to_string();
                    self.form_fields[1] = entry.username.to_string();
                    self.form_fields[2] = entry.secret.to_string();
                }
            } else {
                self.status_message = Some("❌ Falha ao carregar detalhes".into());
            }
        }
    }

    pub fn open_form(&mut self, new_state: AppState, populate: bool) {
        self.reset_form();

        if populate {
            self.populate_form();
        }

        self.current_field = 0;
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
                self.status_message = Some("Password copied to clipboard!".to_string());
            }
            Err(e) => {
                let _ = std::fs::write("blackout_debug.txt", format!("Erro wl-copy: {}", e));
                self.status_message = Some("Failed to copy (missing wl-copy)".to_string());
            }
        }
    }
}
