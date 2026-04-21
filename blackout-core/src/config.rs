use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub auto_lock_timeout: u64,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            auto_lock_timeout: 30,
        }
    }
}

impl DaemonConfig {
    pub fn file_path() -> PathBuf {
        let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("blackout");
        path.push("config.toml");
        path
    }

    pub fn load_config() -> Self {
        let path = Self::file_path();
        if let Ok(contents) = fs::read_to_string(&path) {
            toml::from_str(&contents).unwrap_or_default()
        } else {
            let default_cfg = Self::default();
            default_cfg.save();
            default_cfg
        }
    }

    pub fn save(&self) {
        let path = Self::file_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(toml_str) = toml::to_string_pretty(self) {
            let _ = fs::write(path, toml_str);
        }
    }
}
