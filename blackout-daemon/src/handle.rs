use std::sync::Arc;
use serde_json::json;
use tokio::sync::RwLock;

use blackout_core::ipc::{Request, Response};
use blackout_core::storage::{Wallet};
use blackout_core::vault::Vault;
use tracing::debug;

use crate::daemon::DaemonState;

pub async fn process_request(req: Request, state: Arc<RwLock<DaemonState>>, storage: Arc<Wallet>) -> Response {
    match req {
        Request::Ping => Response::Ok("Pong! Daemon vivo.".into()),
        Request::Lock => handle_lock(state).await,
        Request::Unlock { master_password } => handle_unlock(master_password, state, storage).await,
        Request::AddEntry { service, user, password } => handle_add_entry(service, user, password, state).await,
        Request::ListEntries => handle_list_entries(state).await,
        Request::GetEntry { service } => handle_get_entry(service, state).await,
         _ => Response::Error("Comando desconhecido".into()),
    }
}

async fn handle_unlock(password: String, state: Arc<RwLock<DaemonState>>, storage: Arc<Wallet>) -> Response {
    if !storage.exists() {
        debug!("Vault file not found, creating a new encrypted vault...");
        let new_vault = Vault::default();
        if let Err(e) = storage.encrypt_and_save_vault(&new_vault, &password) {
            return Response::Error(format!("Failed to initialize vault: {}", e));
        }
        let mut st = state.write().await;
        st.vault = Some(new_vault);
        st.authenticated = true;
        return Response::Ok("Vault initialized and unlocked".into());
    }

    debug!("Vault already exists, attempting to load...");
    match storage.load_vault(&password) {
        Ok(vault) => {
            let mut st = state.write().await;
            st.vault = Some(vault);
            st.authenticated = true;
            Response::Ok("Vault unlocked successfully".into())
        }
        Err(e) => Response::Error(format!("Failed to load vault: {}", e)),
    }
}

async fn handle_lock(state: Arc<RwLock<DaemonState>>) -> Response {
    let mut st = state.write().await;
    st.authenticated = false;
    st.vault = None;    
    
    Response::Ok("Vault locked and memory cleared".into())
}

async fn handle_add_entry(service: String, user: String, pass: String, state: Arc<RwLock<DaemonState>>) -> Response {
    let mut st = state.write().await;
    if !st.authenticated {
        return Response::Error("Vault is locked. Please unlock it first.".into());
    }

    if let Some(vault) = &mut st.vault {
        vault.add_entry(service, user, pass);
        Response::Ok("Entry added successfully".into())
    } else {
        Response::Error("Vault is not loaded.".into())
    }
}

async fn handle_list_entries(state: Arc<RwLock<DaemonState>>) -> Response {
    let st = state.read().await;
    if !st.authenticated {
        return Response::Error("Vault is locked. Please unlock it first.".into());
    }

    if let Some(vault) = &st.vault {
        let entries: Vec<String> = vault.list_entries().iter().map(|entry| json!({
            "id": entry.id,
            "service": entry.service,
            "username": entry.username
        }).to_string()).collect();
        Response::Ok(format!("Entries: {:?}", entries))
    } else {
        Response::Error("Vault is not loaded.".into())
    }
}

async fn handle_get_entry(service: String, state: Arc<RwLock<DaemonState>>) -> Response {
    let st = state.read().await;
    if !st.authenticated {
        return Response::Error("Vault is locked. Please unlock it first.".into());
    }

    if let Some(vault) = &st.vault {
        let entries: Vec<String> = vault.get_entry(&service).iter().map(|entry| json!(entry).to_string()).collect();
        if !entries.is_empty() {
            Response::Ok(format!("{:?}", entries))
        } else {
            Response::Error(format!("No entry found for service: {}", service))
        }
    } else {
        Response::Error("Vault is not loaded.".into())
    }

}