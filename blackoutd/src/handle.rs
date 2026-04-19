use serde_json::json;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::debug;
use uuid::Uuid;

use blackout_core::ipc::{EntryInput, EntryUpdateInput, Request, Response, VaultListPayload};
use blackout_core::storage::Wallet;
use blackout_core::vault::Vault;

use crate::daemon::DaemonState;

// Helpers

// Context structs
pub struct Context {
    pub state: Arc<RwLock<DaemonState>>,
    pub storage: Arc<Wallet>,
}

impl Context {
    pub fn new(state: Arc<RwLock<DaemonState>>, storage: Arc<Wallet>) -> Self {
        Self { state, storage }
    }
}

// Auth check
fn needs_auth(req: &Request) -> bool {
    // Except Ping and Unlock
    !matches!(req, Request::Ping | Request::Unlock { .. } | Request::Lock)
}

// vault mutation
fn is_mutation(req: &Request) -> bool {
    matches!(
        req,
        Request::AddEntry { .. } | 
        Request::UpdateEntry { .. } | 
        Request::DeleteEntry { .. }
    )
}

pub async fn process_request(
    req: Request,
    state: Arc<RwLock<DaemonState>>,
    storage: Arc<Wallet>,
) -> Response {
    // Update activity timestamp on every request
    crate::daemon::Daemon::update_activity(&state);

    // Creating context
    let ctx = Context::new(state, storage);

    // Auth check
    if needs_auth(&req) {
        let st = ctx.state.read().await;
        if !st.authenticated {
            debug!("Access denied: Vault is locked.");
            return Response::Error("Vault is locked".into());
        }
    }

    if is_mutation(&req) {
        let mut st = ctx.state.write().await;
        
        if !st.authenticated {
            debug!("Vault is locked. Please unlock it first.");
            return Response::Error("Vault is locked".into());
        }        

        // Create snapshot

        let password = match st.master_password.clone() {
            Some(p) => p,
            None => return Response::Error("No password in memory".into()),
        };

        if let Some(vault) = &mut st.vault {
            match ctx.storage.create_backup_file(&vault.entries, vault.version, &password) {
                Ok(meta) => {
                    vault.history.push(meta);
                },
                Err(e) => return Response::Error(format!("Snapshot failed: {}", e)),
            }
        }
    }

    match req {
        Request::Ping => Response::Ok("Pong!".into()),
        Request::Lock => handle_lock(ctx.state).await,
        Request::Unlock { master_password } => handle_unlock(ctx, master_password).await,
        Request::AddEntry { entry_ctx } => handle_add_entry(ctx, entry_ctx).await,
        Request::ListEntries => handle_list_entries(&ctx).await,
        Request::GetEntry { service } => handle_get_entry(service, ctx.state).await,
        Request::GetEntryById { uuid } => handle_get_entry_by_id(uuid, ctx.state).await,
        Request::DeleteEntry { uuid } => handle_delete_entry(ctx, uuid).await,
        Request::UpdateEntry { entry_ctx } => handle_update_entry(ctx, entry_ctx).await,
    }
}

async fn handle_unlock(
    ctx: Context,
    password: String,
) -> Response {
    let storage = ctx.storage;
    let state = ctx.state;

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

async fn handle_add_entry(
    ctx: Context,
    entry_ctx: EntryInput,
) -> Response {
    let mut st = ctx.state.write().await;
    let password = st.master_password.clone().unwrap();

    if let Some(vault) = &mut st.vault {
        vault.add_entry(entry_ctx.service, entry_ctx.username, entry_ctx.password);
        
        if let Err(e) = ctx.storage.encrypt_and_save_vault(vault, &password) {
            return Response::Error(format!("Save failed: {}", e));
        }
        Response::Ok("Entry added".into())
    } else {
        Response::Error("Vault not loaded".into())
    }
}

pub async fn handle_list_entries(ctx: &Context) -> Response {
    let st = ctx.state.read().await;

    if let Some(vault) = &st.vault {
        
        let payload = VaultListPayload {
            entries: vault.entries.clone(),
            version: vault.version,
        };

        let data = serde_json::to_string(&payload).unwrap();
        Response::Ok(data)
    } else {
        Response::Error("Vault is locked".into())
    }
}

async fn handle_get_entry(service: String, state: Arc<RwLock<DaemonState>>) -> Response {
    let st = state.read().await;

    if let Some(vault) = &st.vault {
        let entries: Vec<String> = vault
            .get_entry(&service)
            .iter()
            .map(|entry| json!(entry).to_string())
            .collect();
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

async fn handle_delete_entry(
    ctx: Context,
    uuid: uuid::Uuid,
) -> Response {
    let mut st = ctx.state.write().await;

    let password = st.master_password.clone();
    if let Some(vault) = &mut st.vault {
        match uuid.to_string().parse::<Uuid>() {
            Ok(id) => {
                if vault.remove_entry(id) {
                    // Save after deletion
                    if let Some(p) = password.as_ref()
                        && let Err(e) = ctx.storage.encrypt_and_save_vault(vault, p)
                    {
                        debug!("Failed to save vault: {}", e);
                        return Response::Error(format!("Failed to save vault: {}", e));
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

async fn handle_update_entry(
    ctx: Context,
    entry_ctx: EntryUpdateInput,
) -> Response {
    let mut st: tokio::sync::RwLockWriteGuard<'_, DaemonState> = ctx.state.write().await;

    let password = st.master_password.clone();
    if let Some(vault) = &mut st.vault {
        vault.update_entry(entry_ctx.uuid, entry_ctx.service, entry_ctx.username, entry_ctx.password);
        if let Some(p) = password.as_ref()
            && let Err(e) = ctx.storage.encrypt_and_save_vault(vault, p)
        {
            debug!("Failed to save vault: {}", e);
            return Response::Error(format!("Failed to save vault: {}", e));
        }
        Response::Ok("Entry updated successfully".into())
    } else {
        debug!("Vault is not loaded.");
        Response::Error("Vault is not loaded.".into())
    }
}
