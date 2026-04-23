use ratatui::widgets::ListState;

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

#[derive(Debug, Clone, PartialEq)]
pub struct FieldConfig {
    pub label: String,
    pub is_password: bool,
    pub show_password: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PendingAction {
    DeleteEntry(uuid::Uuid),
    RestoreSnapshot(uuid::Uuid, u32),
}

impl PendingAction {
    pub fn get_prompt_text(&self) -> String {
        match self {
            PendingAction::DeleteEntry(_id) => {
                "Are you sure you want to delete this entry?".to_string()
            }
            PendingAction::RestoreSnapshot(_uuid, version) => format!(
                "Warning: Restore snapshot v{}. This action will overwrite the current state. Continue?",
                version
            ),
        }
    }
}
