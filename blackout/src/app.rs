use blackout_core::ipc::{
    EntryInput, EntryUpdateInput, Request, Response, VaultListPayload, VaultSnapshotPayload,
};

use blackout_core::vault::{Entry, VaultSnapshot};

use crate::state::{FieldConfig, FormState, PendingAction, SelectedItem, SettingsState};

use ratatui::widgets::TableState;

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;

#[derive(PartialEq, Debug, Clone)]
pub enum AppState {
    InitialCheck,
    UnlockPrompt,
    VaultLocked,
    EntriesList,
    NewEntryForm(Vec<FieldConfig>),
    ViewEntry(Vec<FieldConfig>),
    UpdateEntry(Vec<FieldConfig>),
    Settings(SettingsState),
    ChangeMasterPassword(Vec<FieldConfig>),
    SnapshotList,
    ConfirmAction {
        action: PendingAction,
        previous_state: Box<AppState>,
    },
}

pub struct App {
    pub state: AppState,
    pub vault_unlocked: bool,
    pub entries: Vec<Entry>,
    pub input_buffer: String, // For password input
    pub table_state: TableState,
    pub status_message: Option<String>,
    pub status_time: Option<Instant>,
    pub last_interaction: Instant,
    pub _last_tick: std::time::Instant,
    pub vault_version: u32,
    pub form_state: FormState,
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
            table_state,
            status_message: None,
            status_time: Some(Instant::now()),
            last_interaction: Instant::now(),
            _last_tick: Instant::now(),
            vault_version: 0,
            form_state: FormState::new(),
            snapshots: vec![],
        }
    }

    pub fn is_cursor_visible(&self) -> bool {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            % 1000
            < 500 // interval: 500ms
    }

    pub fn set_status(&mut self, msg: String) {
        self.status_message = Some(msg);
        self.status_time = Some(Instant::now());
    }

    pub fn get_selected_item(&self) -> Option<SelectedItem<'_>> {
        match &self.state {
            AppState::EntriesList | AppState::UpdateEntry(_) => self
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

    pub fn unlock_vault(&mut self, password: String) -> bool {
        match crate::send_command(Request::Unlock {
            master_password: password,
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
        self.form_state
            .fields
            .get(index)
            .map(|s| s.as_str())
            .unwrap_or("")
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

        let service = &self.form_state.fields[0];
        let user = &self.form_state.fields[1];
        let password = &self.form_state.fields[2];

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
        let service = &self.form_state.fields[0];
        let user = &self.form_state.fields[1];
        let password = &self.form_state.fields[2];

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
        let old = self.form_state.fields[0].clone();
        let new: String = self.form_state.fields[1].clone();
        let confirm: String = self.form_state.fields[2].clone();

        let pass_valid = self.unlock_vault(old.clone());

        if !pass_valid {
            self.set_status("Master Password invalid!".into());
            return;
        }

        if new.is_empty() || old.is_empty() {
            self.set_status("Fields cannot be empty!".into());
            return;
        }

        if new != confirm {
            self.set_status("New passwords do not match!".into());
            return;
        }

        if new == old {
            self.set_status("New password cannot be the same!".into());
            return;
        }

        match crate::send_command(Request::UpdateMasterPassword { new_password: new }) {
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
            self.form_state.fields.join(",")
        );
        let _ = std::fs::write("blackout_debug.txt", debug_info);
        self.set_status(format!("{} failed. Check debug log.", action));
    }

    pub fn reset_form(&mut self) {
        for field in self.form_state.fields.iter_mut() {
            field.clear();
        }
        self.form_state.current_index = 0;
        self.table_state.select(Some(0));
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
                self.form_state.fields[0] = entry.service.to_string();
                self.form_state.fields[1] = entry.username.to_string();
                self.form_state.fields[2] = entry.secret.to_string();
            }
        } else {
            self.set_status("failed to load datails. Check debug log.".into());
        }
    }

    pub fn open_form(&mut self, new_state: AppState, uuid: Option<uuid::Uuid>) {
        if let Some(uuid) = uuid {
            self.populate_form(uuid);
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
}
