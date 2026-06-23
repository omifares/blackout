use blackout_core::generator::{GeneratorConfig, GeneratorMode};
use ratatui::widgets::ListState;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SettingsOption {
    ChangeMasterPassword,
    SnapshotList,
    PasswordGenerator,
}

impl SettingsOption {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ChangeMasterPassword => "Change Master Password",
            Self::SnapshotList => "Snapshots",
            Self::PasswordGenerator => "Password Generator",
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
                SettingsOption::PasswordGenerator,
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum EnumChoice {
    #[default]
    GeneratorMode,
}

impl EnumChoice {
    pub fn options(&self) -> &'static [&'static str] {
        match self {
            EnumChoice::GeneratorMode => &["Random Chars", "Passphrase"],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum FieldType {
    #[default]
    Text,
    Password,
    Username,
    Service,
    Number,
    Checkbox,
    Choice(EnumChoice),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    Text(String),
    Choice(usize),
    Boolean(bool),
    Number(u16),
}

impl Default for FieldValue {
    fn default() -> Self {
        Self::Boolean(false);
        Self::Text(String::new());
        Self::Choice(0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldConfig {
    pub label: &'static str,
    pub is_password: bool,
    pub show_password: bool,
    pub field_type: FieldType,
    pub value: FieldValue,
}

impl Default for FieldConfig {
    fn default() -> Self {
        Self {
            label: "",
            is_password: false,
            show_password: false,
            field_type: FieldType::Text,
            value: FieldValue::Text(String::new()),
        }
    }
}

impl FieldConfig {
    pub fn text(label: &'static str, default_value: &str) -> Self {
        Self {
            label,
            field_type: FieldType::Text,
            value: FieldValue::Text(default_value.to_string()),
            ..Default::default()
        }
    }

    pub fn service(label: &'static str) -> Self {
        Self {
            label,
            field_type: FieldType::Service,
            value: FieldValue::Text(String::new()),
            ..Default::default()
        }
    }

    pub fn username(label: &'static str) -> Self {
        Self {
            label,
            field_type: FieldType::Username,
            value: FieldValue::Text(String::new()),
            ..Default::default()
        }
    }

    pub fn password(label: &'static str) -> Self {
        Self {
            label,
            is_password: true,
            field_type: FieldType::Password,
            value: FieldValue::Text(String::new()),
            ..Default::default()
        }
    }

    pub fn number(label: &'static str, default_val: u16) -> Self {
        Self {
            label,
            field_type: FieldType::Number,
            value: FieldValue::Number(default_val),
            ..Default::default()
        }
    }

    pub fn choice(label: &'static str, menu_type: EnumChoice) -> Self {
        Self {
            label,
            field_type: FieldType::Choice(menu_type),
            value: FieldValue::Choice(0),
            ..Default::default()
        }
    }

    pub fn checkbox(label: &'static str, value: Option<bool>) -> Self {
        Self {
            label: label,
            field_type: FieldType::Checkbox,
            value: value.map(FieldValue::Boolean).unwrap_or_default(),
            ..Default::default()
        }
    }

    pub fn from_config(config: &GeneratorConfig) -> Vec<Self> {
        vec![
            FieldConfig {
                label: "Length",
                field_type: FieldType::Number,
                value: FieldValue::Number(config.length as u16),
                ..Default::default()
            },
            FieldConfig {
                label: "Mode",
                field_type: FieldType::Choice(EnumChoice::GeneratorMode),
                value: FieldValue::Choice(match config.mode {
                    GeneratorMode::RandomChars => 0,
                    GeneratorMode::Passphrase => 1,
                }),
                ..Default::default()
            },
            FieldConfig {
                label: "Capitalize",
                field_type: FieldType::Checkbox,
                value: FieldValue::Boolean(config.capitalize),
                ..Default::default()
            },
            FieldConfig {
                label: "Word Count",
                field_type: FieldType::Number,
                value: FieldValue::Number(config.word_count as u16),
                ..Default::default()
            },
            FieldConfig {
                label: "Separator",
                field_type: FieldType::Text,
                value: FieldValue::Text(config.separator.to_string()),
                ..Default::default()
            },
            FieldConfig {
                label: "Uppercase",
                field_type: FieldType::Checkbox,
                value: FieldValue::Boolean(config.uppercase),
                ..Default::default()
            },
            FieldConfig {
                label: "Lowercase",
                field_type: FieldType::Checkbox,
                value: FieldValue::Boolean(config.lowercase),
                ..Default::default()
            },
            FieldConfig {
                label: "Numbers",
                field_type: FieldType::Checkbox,
                value: FieldValue::Boolean(config.numbers),
                ..Default::default()
            },
            FieldConfig {
                label: "Symbols",
                field_type: FieldType::Checkbox,
                value: FieldValue::Boolean(config.symbols),
                ..Default::default()
            },
        ]
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
    pub session_config: GeneratorConfig,
    pub generated_password: Option<String>,
}

impl Default for PasswordGeneratorState {
    fn default() -> Self {
        Self {
            session_config: GeneratorConfig::default(),
            generated_password: None,
        }
    }
}

impl PasswordGeneratorState {
    pub fn sync_config(&mut self, fields: &[FieldConfig]) {
        for field in fields.iter() {
            match (field.label, &field.value) {
                ("Length", FieldValue::Number(n)) => self.session_config.length = *n as usize,
                ("Capitalize", FieldValue::Boolean(b)) => self.session_config.capitalize = *b,
                ("Uppercase", FieldValue::Boolean(b)) => self.session_config.uppercase = *b,
                ("Lowercase", FieldValue::Boolean(b)) => self.session_config.lowercase = *b,
                ("Numbers", FieldValue::Boolean(b)) => self.session_config.numbers = *b,
                ("Symbols", FieldValue::Boolean(b)) => self.session_config.symbols = *b,
                ("Mode", FieldValue::Choice(i)) => {
                    self.session_config.mode = GeneratorMode::from_index(*i)
                }
                ("Word Count", FieldValue::Number(n)) => {
                    self.session_config.word_count = *n as usize
                }
                ("Separator", FieldValue::Text(s)) => {
                    if let Some(c) = s.chars().next() {
                        self.session_config.separator = c;
                    }
                }
                _ => {}
            }
        }
    }

    pub fn build_form_fields(&self) -> Vec<FieldConfig> {
        vec![
            FieldConfig::number("Length", self.session_config.length as u16),
            FieldConfig::choice("Mode", EnumChoice::GeneratorMode),
            FieldConfig::checkbox("Capitalize", Some(self.session_config.capitalize)),
            FieldConfig::checkbox("Uppercase", Some(self.session_config.uppercase)),
            FieldConfig::checkbox("Lowercase", Some(self.session_config.lowercase)),
            FieldConfig::checkbox("Numbers", Some(self.session_config.numbers)),
            FieldConfig::checkbox("Symbols", Some(self.session_config.symbols)),
            FieldConfig::number("Word Count", self.session_config.word_count as u16),
            FieldConfig::text("Separator", &self.session_config.separator.to_string()),
        ]
    }

    pub fn generate_password(&mut self) -> Result<String, String> {
        let mode = self.session_config.mode.clone();
        let pwd = blackout_core::generator::generate(mode, &self.session_config)?;

        self.generated_password = Some(pwd.clone());
        Ok(pwd)
    }
}
