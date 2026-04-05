use std::sync::Arc;
use serde_json::json;

use tokio::sync::RwLock;
use uuid::Uuid;
use tracing::debug;

use blackout_core::ipc::{Request, Response};
use blackout_core::storage::{Wallet};
use blackout_core::vault::Vault;

use crate::daemon::DaemonState;

pub async fn process_request(req: Request, state: Arc<RwLock<DaemonState>>, storage: Arc<Wallet>) -> Response {
    // Update activity timestamp on every request
    crate::daemon::Daemon::update_activity(&state);
    
    match req {
        Request::Ping => Response::Ok("Pong!".into()),
        Request::Lock => handle_lock(state).await,
        Request::Unlock { master_password } => handle_unlock(master_password, state, storage).await,
        Request::AddEntry { service, user, password } => handle_add_entry(service, user, password, state, storage).await,
        Request::ListEntries => handle_list_entries(state).await,
        Request::GetEntry { service } => handle_get_entry(service, state).await,
        Request::GetEntryById { uuid } => handle_get_entry_by_id(uuid, state).await,
        Request::DeleteEntry { uuid } => handle_delete_entry(uuid, state, storage).await,
    }
}

async fn handle_unlock(password: String, state: Arc<RwLock<DaemonState>>, storage: Arc<Wallet>) -> Response {
    if !storage.exists() {
        debug!("Vault file not found, creating a new encrypted vault...");
        let new_vault = Vault::default();
        if let Err(e) = storage.encrypt_and_save_vault(&new_vault, &password) {
            debug!("Failed to initialize vault: {}", e);
            return Response::Error(format!("Failed to initialize vault: {}", e));
        }
        let mut st = state.write().await;
        st.vault = Some(new_vault);
        st.authenticated = true;
        st.master_password = Some(password);
        return Response::Ok("Vault initialized and unlocked".into());
    }

    debug!("Vault already exists, attempting to load...");
    match storage.load_vault(&password) {
        Ok(vault) => {
            let mut st = state.write().await;
            st.vault = Some(vault);
            st.authenticated = true;
            st.master_password = Some(password);
            Response::Ok("Vault unlocked successfully".into())
        }
        Err(e) => {
            debug!("Failed to load vault: {}", e);
            Response::Error(format!("Failed to load vault: {}", e))
        }
    }
}

async fn handle_lock(state: Arc<RwLock<DaemonState>>) -> Response {
    let mut st = state.write().await;
    st.authenticated = false;
    st.vault = None;
    st.master_password = None;
    
    Response::Ok("Vault locked and memory cleared".into())
}

async fn handle_add_entry(service: String, user: String, pass: String, state: Arc<RwLock<DaemonState>>, storage: Arc<Wallet>) -> Response {
    let mut st = state.write().await;
    if !st.authenticated {
        debug!("Vault is locked. Please unlock it first.");
        return Response::Error("Vault is locked. Please unlock it first.".into());
    }

    let password = st.master_password.clone();
    if let Some(vault) = &mut st.vault {
        vault.add_entry(service, user, pass);
        // Save after adding
        if let Some(p) = password.as_ref() {
            if let Err(e) = storage.encrypt_and_save_vault(vault, p) {
                debug!("Failed to save vault: {}", e);
                return Response::Error(format!("Failed to save vault: {}", e));
            }
        }
        Response::Ok("Entry added successfully".into())
    } else {
        debug!("Vault is not loaded.");
        Response::Error("Vault is not loaded.".into())
    }
}

async fn handle_list_entries(state: Arc<RwLock<DaemonState>>) -> Response {
    let st = state.read().await;
    if !st.authenticated {
        debug!("Vault is locked. Please unlock it first.");
        return Response::Error("Vault is locked. Please unlock it first.".into());
    }

    if let Some(vault) = &st.vault {
        let entries: Vec<serde_json::Value> = vault.list_entries().iter().map(|entry| json!({
            "id": entry.id.to_string(),
            "service": entry.service,
            "username": entry.username,
            "updated_at": entry.updated_at.to_string(),
        })).collect();
        Response::Ok(serde_json::to_string(&entries).unwrap())
    } else {
        debug!("Vault is not loaded.");
        Response::Error("Vault is not loaded.".into())
    }
}

async fn handle_get_entry(service: String, state: Arc<RwLock<DaemonState>>) -> Response {
    let st = state.read().await;
    if !st.authenticated {
        debug!("Vault is locked. Please unlock it first.");
        return Response::Error("Vault is locked. Please unlock it first.".into());
    }

    if let Some(vault) = &st.vault {
        let entries: Vec<String> = vault.get_entry(&service).iter().map(|entry| json!(entry).to_string()).collect();
        if !entries.is_empty() {
            Response::Ok(format!("{:?}", entries))
        } else {
            debug!("No entry found for service: {}", service);
            Response::Error(format!("No entry found for service: {}", service))
        }
    } else {
        debug!("Vault is not loaded.");
        Response::Error("Vault is not loaded.".into())
    }
}

async fn handle_get_entry_by_id(uuid: Uuid, state: Arc<RwLock<DaemonState>>) -> Response {
    let st = state.read().await;
    if !st.authenticated {
        debug!("Vault is locked. Please unlock it first.");
        return Response::Error("Vault is locked. Please unlock it first.".into());
    }

    if let Some(vault) = &st.vault {
        match uuid.to_string().parse::<Uuid>() {
            Ok(id) => {
                if let Some(entry) = vault.get_entry_by_id(id) {
                    Response::Ok(serde_json::to_string(&entry).unwrap())
                } else {
                    debug!("No entry found with id: {}", uuid);
                    Response::Error(format!("No entry found with id: {}", uuid))
                }
            }
            Err(e) => {
                debug!("Invalid UUID format: {}", e);
                Response::Error("Invalid UUID format".into())
            }
        }
    } else {
        debug!("Vault is not loaded.");
        Response::Error("Vault is not loaded.".into())
    }
}

async fn handle_delete_entry(uuid: uuid::Uuid, state: Arc<RwLock<DaemonState>>, storage: Arc<Wallet>) -> Response {
    let mut st = state.write().await;
    if !st.authenticated {
        debug!("Vault is locked. Please unlock it first.");
        return Response::Error("Vault is locked. Please unlock it first.".into());
    }

    let password = st.master_password.clone();
    if let Some(vault) = &mut st.vault {
        match uuid.to_string().parse::<Uuid>() {
            Ok(id) => {
                if vault.remove_entry(id) {
                    // Save after deletion
                    if let Some(p) = password.as_ref() {
                        if let Err(e) = storage.encrypt_and_save_vault(vault, p) {
                            debug!("Failed to save vault: {}", e);
                            return Response::Error(format!("Failed to save vault: {}", e));
                        }
                    }
                    Response::Ok("Entry deleted successfully".into())
                } else {
                    debug!("No entry found with id: {}", uuid);
                    Response::Error(format!("No entry found with id: {}", uuid))
                }
            }
            Err(_) => {
                debug!("Invalid UUID format");
                Response::Error("Invalid UUID format".into())
            }
        }
    } else {
        debug!("Vault is not loaded.");
        Response::Error("Vault is not loaded.".into())
    }
}