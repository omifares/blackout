use crate::{state::FormState};
use ratatui::widgets::ListState;
use blackout_core::generator::{
    GeneratorConfig, GeneratorMode, generate_passphrase, generate_password,
};


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

impl Default for FieldConfig {
    fn default() -> Self {
        Self {
            label: String::new(),
            is_password: false,
            show_password: false,
        }
    }
}

impl FieldConfig {
    pub fn text(label: &str) -> Self {
        Self {
            label: label.into(),
            ..Default::default()
        }
    }

    pub fn password(label: &str) -> Self {
        Self {
            label: label.into(),
            is_password: true,
            ..Default::default()
        }
    }
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

#[derive(Clone, Debug, PartialEq)]
pub struct PasswordGeneratorState {
    pub config: GeneratorConfig,
    pub generated_password: Option<String>,
    pub form_state: FormState,
}

impl PasswordGeneratorState {
    pub fn generate_password(&mut self) -> Result<String, String> {
        let result = match self.config.mode {
            GeneratorMode::RandomChars => generate_password(&self.config),
            GeneratorMode::Passphrase => generate_passphrase(
                self.config.word_count,
                &self.config.separator.to_string(),
                self.config.capitalize,
            ),
        };
        match result {
            Ok(pwd) => {
                self.generated_password = Some(pwd);
                Ok("Password generated!".into())
            }
            Err(e) => {
                self.generated_password = None;
                Err(format!("Generation failed: {}", e))
            }
        }
    }
}
