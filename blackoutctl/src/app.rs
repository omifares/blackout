use blackout_core::ipc::{Request, Response};
use blackout_core::vault::Entry;

use chrono::{DateTime, Local};

pub trait EntryView {
    fn _id(&self) -> &uuid::Uuid;
    fn service(&self) -> &str;
    fn username(&self) -> &str;
    fn secret(&self) -> &str;
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
    fn secret(&self) -> &str {
        ""
    }
    fn updated_at(&self) -> DateTime<Local> {
        self.0.updated_at
    }
}

#[derive(Debug, Clone)]
pub struct DetailEntryView(pub Entry);

impl EntryView for DetailEntryView {
    fn _id(&self) -> &uuid::Uuid {
        &self.0.id
    }
    fn service(&self) -> &str {
        &self.0.service
    }
    fn username(&self) -> &str {
        &self.0.username
    }
    fn secret(&self) -> &str {
        &self.0.secret
    }
    fn updated_at(&self) -> DateTime<Local> {
        self.0.updated_at
    }
}

#[derive(Debug, Clone)]
pub enum AppState {
    InitialCheck,
    UnlockPrompt,
    EntriesList,
    NewEntryForm,
    ViewEntry,
}

pub struct App {
    pub state: AppState,
    pub vault_unlocked: bool,
    pub entries: Vec<Entry>,
    pub detail_entry: Option<DetailEntryView>,
    pub input_buffer: String,     // For password input
    pub form_fields: [String; 3], // service, user, password
    pub current_field: usize,
    pub selected_entry: usize, // Index of selected entry in list
}

impl App {
    pub fn new() -> Self {
        Self {
            state: AppState::InitialCheck,
            vault_unlocked: false,
            entries: vec![],
            input_buffer: String::new(),
            form_fields: [String::new(), String::new(), String::new()],
            current_field: 0,
            selected_entry: 0,
            detail_entry: None,
        }
    }

    pub fn check_vault_status(&mut self) {
        match crate::send_command(Request::ListEntries) {
            Ok(response) => {
                match response {
                    Response::Ok(data) => {
                        self.parse_entries(&data);
                        self.vault_unlocked = true;
                        self.state = AppState::EntriesList;
                    }
                    Response::Error(err) => {
                        if err.contains("Vault is locked") {
                            self.vault_unlocked = false;
                            self.state = AppState::UnlockPrompt;
                        } else {
                            self.state = AppState::UnlockPrompt; // Fallback
                        }
                    }
                }
            }
            Err(_) => {
                self.state = AppState::UnlockPrompt;
            }
        }
    }

    fn parse_entries(&mut self, data: &str) {
        match serde_json::from_str::<Vec<Entry>>(data) {
            Ok(entries) => {
                self.entries = entries;
                self.selected_entry = 0;
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
            self.selected_entry = (self.selected_entry + 1) % self.entries.len();
        }
    }

    pub fn prev_entry(&mut self) {
        if !self.entries.is_empty() {
            if self.selected_entry == 0 {
                self.selected_entry = self.entries.len() - 1;
            } else {
                self.selected_entry -= 1;
            }
        }
    }

    pub fn submit_form(&mut self) {
        let service = &self.form_fields[0];
        let user = &self.form_fields[1];
        let password = &self.form_fields[2];
        if !service.is_empty() && !user.is_empty() && !password.is_empty() {
            match crate::send_command(Request::AddEntry {
                service: service.clone(),
                user: user.clone(),
                password: password.clone(),
            }) {
                Ok(Response::Ok(_)) => {
                    self.load_entries();
                    self.state = AppState::EntriesList;
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
        self.selected_entry = 0;
    }

    pub fn delete_selected_entry(&mut self) {
        if let Some(entry) = self.entries.get(self.selected_entry) {
            let id = entry.id.clone();
            match crate::send_command(Request::DeleteEntry { uuid: id }) {
                Ok(Response::Ok(_)) => {
                    self.load_entries();
                    self.state = AppState::EntriesList;
                }
                Ok(Response::Error(e)) => {
                    let debug_info = format!("Delete Error: {}\nID:{}", e, id);
                    let _ = std::fs::write("blackout_debug.txt", debug_info);
                }
                Err(_) => {}
            }
        }
    }

    pub fn view_selected_entry(&mut self) {
        let uuid = self.entries[self.selected_entry].id.clone();
        if let Ok(Response::Ok(data)) = crate::send_command(Request::GetEntryById { uuid }) {
            if let Ok(entry) = serde_json::from_str::<Entry>(&data) {
                self.detail_entry = Some(DetailEntryView(entry));
                self.state = AppState::ViewEntry;
            }
        } else {
            let debug_info = format!("View Entry Error: Failed to get entry details for ID: {}", uuid);
            let _ = std::fs::write("blackout_debug.txt", debug_info);
        }
    }
}
