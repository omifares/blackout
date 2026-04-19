use blackout_core::ipc::{EntryInput, EntryUpdateInput, Request, Response, VaultListPayload};
use blackout_core::vault::Entry;

use chrono::{DateTime, Local};

use ratatui::widgets::TableState;

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

#[derive(PartialEq, Debug, Clone)]
pub enum AppState {
    InitialCheck,
    UnlockPrompt,
    VaultLocked,
    EntriesList,
    NewEntryForm,
    ViewEntry(DetailEntryView),
    UpdateEntry,
    ConfirmEntryDelete,
}

pub struct App {
    pub state: AppState,
    pub vault_unlocked: bool,
    pub entries: Vec<Entry>,
    pub detail_entry: Option<DetailEntryView>,
    pub input_buffer: String,     // For password input
    pub form_fields: [String; 3], // service, user, password
    pub current_field: usize,
    pub table_state: TableState,
    pub status_message: Option<String>,
    pub last_interaction: Instant,
    pub last_tick: std::time::Instant,
    pub vault_version: u32,
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
            form_fields: [String::new(), String::new(), String::new()],
            current_field: 0,
            detail_entry: None,
            table_state,
            status_message: None,
            last_interaction: Instant::now(),
            last_tick: Instant::now(),
            vault_version: 0,
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

    pub fn submit_form(&mut self) {
        let service = &self.form_fields[0];
        let user = &self.form_fields[1];
        let password = &self.form_fields[2];

        let entry_ctx = EntryInput {
            service: service.clone(),
            username: user.clone(),
            password: password.clone(),
        };

        if !service.is_empty() && !user.is_empty() && !password.is_empty() {
            match crate::send_command(Request::AddEntry { entry_ctx }) {
                Ok(Response::Ok(_)) => {
                    self.load_entries();
                    self.state = AppState::EntriesList;
                    self.status_message = Some("Entry successfully added!".to_string());
                    self.reset_form();
                }
                Ok(Response::Error(e)) => {
                    let debug_info = format!(
                        "Add Entry Error: {}\nData:{}",
                        e,
                        &self.form_fields.join(",")
                    );
                    let _ = std::fs::write("blackout_debug.txt", debug_info);
                }
                Err(_) => {}
            }
        }
    }

    pub fn reset_form(&mut self) {
        self.form_fields = [String::new(), String::new(), String::new()];
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

    pub fn start_editing_entry(&mut self) {
        if let Some(entry) = self.get_selected_entry() {
            let uuid = entry.id;

            if let Ok(Response::Ok(data)) = crate::send_command(Request::GetEntryById { uuid }) {
                if let Ok(entry) = serde_json::from_str::<Entry>(&data) {
                    let detail_entry = DetailEntryView {
                        entry,
                        show_password: false,
                    };

                    self.form_fields[0] = detail_entry.entry.service.to_string();
                    self.form_fields[1] = detail_entry.entry.username.to_string();
                    self.form_fields[2] = detail_entry.entry.secret.to_string();

                    self.state = AppState::UpdateEntry;
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

    pub fn submit_entry_update(&mut self) {
        if let Some(entry) = self.get_selected_entry() {
            let uuid = entry.id;

            let service = self.form_fields[0].clone();
            let username = self.form_fields[1].clone();
            let password = self.form_fields[2].clone();

            let entry_ctx = EntryUpdateInput {
                uuid: uuid,
                service: Some(service.clone()),
                username: Some(username.clone()),
                password: Some(password.clone()),
            };

            match crate::send_command(Request::UpdateEntry { entry_ctx }) {
                Ok(Response::Ok(_)) => {
                    self.load_entries();
                    self.state = AppState::EntriesList;
                    self.status_message = Some("Entry successfully edited!".to_string());
                    self.reset_form();
                }
                Ok(Response::Error(e)) => {
                    let debug_info = format!(
                        "Update Entry Error: {}\nData:{}",
                        e,
                        self.form_fields.join(",")
                    );
                    let _ = std::fs::write("blackout_debug.txt", debug_info);
                }
                Err(_) => {}
            }
        }
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
