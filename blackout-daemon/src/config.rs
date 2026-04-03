use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{PathBuf};
use tracing::info;

/// Configuração do daemon
/// Contém parâmetros de configuração para o funcionamento do daemon, como caminhos, portas e timeouts.
///  - `storage_path`: Wallet storage path (default: ~/.blackout)
///  - `port`: API port (default: 9400)
///  - `inactivity_timeout_secs`: Inactivity timeout in seconds before auto-locking (default: 60)
///  - `json_logging`: Enable JSON logging (default: false)
///  - `max_auth_attempts`: Maximum authentication attempts before lockout (default: 3)
///  - `event_check_interval_ms`: Interval in milliseconds to check for events (default: 500)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub storage_path: PathBuf,
    pub port: u16,
    pub inactivity_timeout_secs: u64,
    pub json_logging: bool,
    pub max_auth_attempts: u32,
    pub event_check_interval_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap();
        Self {
            storage_path: home.join(".blackout"),
            port: 9400,
            inactivity_timeout_secs: 60,
            json_logging: false,
            max_auth_attempts: 5,
            event_check_interval_ms: 500,
        }
    }
}

impl Config {
    /// Load configuration from file or use defaults
    pub fn load(config_path: Option<PathBuf>) -> Result<Self> {
        let path = config_path.unwrap_or_else(|| {
            let home = dirs::home_dir().unwrap();
            home.join(".config/blackout/daemon.toml")
        });

        if path.exists() {
            info!("Loading config from: {}", path.display());
            let content = fs::read_to_string(&path)
                .context("Failed to read config file")?;
            toml::from_str(&content).context("Failed to parse config file")
        } else {
            info!("Config file not found, using default configuration");
            Ok(Self::default())
        }
    }

    // Save configuration
    pub fn save(&self, config_path: Option<PathBuf>) -> Result<()> {
        let path = config_path.unwrap_or_else(|| {
            let home = dirs::home_dir().unwrap();
            home.join(".config/blackout/daemon.toml")
        });

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let content = toml::to_string_pretty(&self)
            .context("Failed to serialize configuration")?;
        fs::write(&path, content)
            .context("Failed to write config file")?;

        info!("Config saved to: {}", path.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.port, 9400);
        assert_eq!(config.inactivity_timeout_secs, 60);
    }
}
