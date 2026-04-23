// blackout-daemon/src/daemon.rs
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, info, warn};

use blackout_core::config::DaemonConfig;
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
    socket_path: PathBuf,
    uid: u32,
}

impl Daemon {
    pub fn new(storage: Arc<Wallet>) -> Self {
        let config = DaemonConfig::load_config();
        let state = DaemonState {
            vault: None,
            authenticated: false,
            running: true,
            master_password: None,
            last_activity: Instant::now(),
            auto_lock_timeout: Duration::from_secs(config.auto_lock_timeout),
        };

        let uid = unsafe { libc::geteuid() };
        let socket_path = blackout_core::ipc::get_socket_path();

        Self {
            storage,
            state: Arc::new(RwLock::new(state)),
            socket_path,
            uid,
        }
    }

    /// Starter
    pub async fn run(&self) -> Result<()> {
        info!("Daemon started! Waiting for authentication...");

        let _ = std::fs::remove_file(&self.socket_path);

        let listener = {
            let old_umask = unsafe { libc::umask(0o077) };
            let bind_result = UnixListener::bind(&self.socket_path);
            unsafe { libc::umask(old_umask) };
            bind_result?
        };

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
                    match stream.peer_cred() {
                        Ok(cred) => {
                            if cred.uid() != self.uid {
                                warn!("UID {} try to connect. Dropping - Not owner {}", cred.uid(), &self.uid);
                                continue;
                            }
                            debug!("Accepting UID: {}", cred.uid());
                        }
                        Err(e) => {
                            warn!("Failed to read creds: {}. Dropping.", e);
                            continue;
                        }
                    }

                    let state_clone = Arc::clone(&self.state);
                    let storage_clone = Arc::clone(&self.storage);

                    tokio::spawn(async move {
                        use tokio::io::BufReader;

                        let (reader, mut writer) = stream.into_split();
                        let mut buf_reader = BufReader::new(reader);
                        let mut line = String::new();

                        if let Ok(bytes_read) = buf_reader.read_line(&mut line).await
                            && bytes_read > 0 {
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
                            };
                    });
                }
            }
        }

        let _ = std::fs::remove_file(&self.socket_path);

        Ok(())
    }

    /// Pending events processor
    async fn process_events(&self) -> Result<()> {
        let mut state = self.state.write().await;
        if state.authenticated && state.last_activity.elapsed() > state.auto_lock_timeout {
            debug!("Auto-locking vault due to inactivity");
            state.vault = None;
            state.authenticated = false;
            state.master_password = None;
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
