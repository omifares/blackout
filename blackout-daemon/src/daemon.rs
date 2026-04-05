// blackout-daemon/src/daemon.rs
use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UnixListener;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, info};
use serde_json;

use blackout_core::storage::Wallet;
use blackout_core::vault::Vault;

use crate::handle::process_request;

/// Mut daemon struct
pub struct DaemonState {
    pub vault: Option<Vault>,
    pub authenticated: bool,
    pub running: bool,
    pub master_password: Option<String>,
    pub last_activity: Instant,
    pub auto_lock_timeout: Duration,
}

/// Main daemon struct
pub struct Daemon {
    storage: Arc<Wallet>,
    state: Arc<RwLock<DaemonState>>,
}

impl Daemon {
    pub fn new(storage: Arc<Wallet>) -> Self {
        let state = DaemonState {
            vault: None,
            authenticated: false,
            running: true,
            master_password: None,
            last_activity: Instant::now(),
            auto_lock_timeout: Duration::from_secs(30), // 30 segundos
        };

        Self {
            storage,
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

        let mut check_interval = interval(Duration::from_millis(500));

        loop {
            if !self.state.read().await.running {
                break;
            }

            tokio::select! {
                _ = check_interval.tick() => {
                    self.process_events().await?;
                }
                
                Ok((stream, _addr)) = listener.accept() => {
                    debug!("New connection recived!");
                    
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
        let mut state = self.state.write().await;
        if state.authenticated {
            if state.last_activity.elapsed() > state.auto_lock_timeout {
                debug!("Auto-locking vault due to inactivity");
                state.vault = None;
                state.authenticated = false;
                state.master_password = None;
            }
        }
        Ok(())
    }

    /// Update last activity timestamp
    pub fn update_activity(state: &Arc<RwLock<DaemonState>>) {
        if let Ok(mut s) = state.try_write() {
            s.last_activity = Instant::now();
        }
    }
    
}
