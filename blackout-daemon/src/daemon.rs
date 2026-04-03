// blackout-daemon/src/daemon.rs
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UnixListener;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, info};

use blackout_core::storage::Wallet;
use blackout_core::vault::Vault;

use crate::config::Config;

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
                
                Ok((mut stream, _addr)) = listener.accept() => {
                    debug!("Nova conexão recebida no socket");
                    
                    let state_clone = Arc::clone(&self.state);
                    let storage_clone = Arc::clone(&self.storage);
                    
                    tokio::spawn(async move {
                        // TODO: Ler a stream, converter JSON para `Request`, 
                        // e manipular o estado (state_clone) de acordo.
                        // Exemplo de acesso ao estado:
                        // let mut st = state_clone.write().await;
                        // st.authenticated = true;
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
