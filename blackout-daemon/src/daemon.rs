// blackout-daemon/src/daemon.rs
use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UnixListener;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, info};
use serde_json;

use blackout_core::storage::Wallet;
use blackout_core::vault::Vault;

use crate::config::Config;
use crate::handle::process_request;

/// Mut daemon struct
pub struct DaemonState {
    pub vault: Option<Vault>,
    pub authenticated: bool,
    pub running: bool,
}

/// Main daemon struct
pub struct Daemon {
    storage: Arc<Wallet>,
    config: Config,
    state: Arc<RwLock<DaemonState>>,
}

impl Daemon {
    pub fn new(storage: Arc<Wallet>, config: Config) -> Self {
        let state = DaemonState {
            vault: None,
            authenticated: false,
            running: true,
        };

        Self {
            storage,
            config,
            state: Arc::new(RwLock::new(state)),
        }
    }

    /// Starter
    pub async fn run(&self, socket_path: &str) -> Result<()> {
        info!("Daemon started! Waiting for authentication...");

        let _ = std::fs::remove_file(socket_path); // Clean old socket
        let listener = UnixListener::bind(socket_path)?;
        
        let mut perms = std::fs::metadata(socket_path)?.permissions();
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o600); // rw-----
        std::fs::set_permissions(socket_path, perms)?;

        let mut check_interval = interval(Duration::from_millis(self.config.event_check_interval_ms));

        loop {
            if !self.state.read().await.running {
                break;
            }

            tokio::select! {
                _ = check_interval.tick() => {
                    self.process_events().await?;
                }
                
                Ok((stream, _addr)) = listener.accept() => {
                    debug!("Nova conexão recebida no socket");
                    
                    let state_clone = Arc::clone(&self.state);
                    let storage_clone = Arc::clone(&self.storage);
                    
                    tokio::spawn(async move {
                        use tokio::io::BufReader;
                        
                        let (reader, mut writer) = stream.into_split();
                        let mut buf_reader = BufReader::new(reader);
                        let mut line = String::new();

                        if let Ok(bytes_read) = buf_reader.read_line(&mut line).await {
                            if bytes_read > 0 {
                                use blackout_core::ipc::{Request, Response};

                                let response = match serde_json::from_str::<Request>(&line) {
                                    Ok(req) => process_request(req, state_clone, storage_clone).await,
                                    Err(e) => {
                                        debug!("Failed to parse request: {}", e);
                                        Response::Error(format!("Invalid request: {}", e))
                                    }
                                };

                                let res_json = serde_json::to_string(&response).unwrap() + "\n";
                                let _ = writer.write_all(res_json.as_bytes()).await;
                            }
                        };
                    });
                }
            }
        }
        
        Ok(())
    }

    /// Pending events processor
    async fn process_events(&self) -> Result<()> {
        // Exemplo: Ler o estado sem bloqueá-lo por muito tempo
        let state = self.state.read().await;
        if state.authenticated {
            // debug!("Checando eventos para vault autenticado...");
        }
        Ok(())
    }

    // Graceful shutdown
    pub async fn shutdown(&self, password_for_save: &str) -> Result<()> {
        info!("Starting graceful shutdown...");

        let mut state = self.state.write().await;

        if let Some(vault) = &state.vault {
            debug!("Saving vault state before shutdown...");
            self.storage.encrypt_and_save_vault(vault, password_for_save)
                .map_err(|e| anyhow::anyhow!("Failed to save vault during shutdown: {}", e))?;
            debug!("Vault state saved successfully");
        }

        state.running = false;
        info!("Daemon shutdown completed successfully");
        Ok(())
    }

    /// Load vault with password authentication
    pub async fn load_vault(&self, password: &str) -> Result<()> {
        info!("Loading vault...");
        
        let vault = self.storage
            .load_vault(password)
            .map_err(|e| anyhow::anyhow!("Failed to load vault: {}", e))?;
            
        let mut state = self.state.write().await;
        state.vault = Some(vault);
        state.authenticated = true;
        
        info!("Vault loaded successfully");
        Ok(())
    }
}
