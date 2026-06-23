use serde_json::json;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use blackout_core::config::DaemonConfig;
use blackout_core::ipc::{
    EntryInput, EntryUpdateInput, Request, Response, VaultListPayload, VaultSnapshotPayload,
};
use blackout_core::storage::Wallet;
use blackout_core::vault::{Entry, Vault, VaultSnapshot};

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
    // Except Ping, Unlock and Password Generation
    !matches!(
        req,
        Request::Ping | Request::Unlock { .. } | Request::Lock | Request::PasswordGen { .. }
    )
}

// vault mutation
fn is_mutation(req: &Request) -> bool {
    matches!(
        req,
        Request::AddEntry { .. }
            | Request::UpdateEntry { .. }
            | Request::DeleteEntry { .. }
            | Request::UpdateMasterPassword { .. }
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
            warn!("Access denied: Vault is locked.");
            return Response::Error("Vault is locked".into());
        }
    }

    if is_mutation(&req) {
        let mut st = ctx.state.write().await;

        if !st.authenticated {
            warn!("Vault is locked. Please unlock it first.");
            return Response::Error("Vault is locked".into());
        }

        // PRE-FLIGHT
        if let Request::RestoreSnapshot { version, uuid } = &req
            && let Some(vault) = &st.vault
        {
            let can_restore = vault
                .history
                .iter()
                .any(|s| s.uuid == *uuid && s.file_ref.is_some());

            if !can_restore {
                error!("Can't load snapshot file for uuid {}", uuid);
                return Response::Error(format!("Fail to load snapshot v{} from disk", version));
            }
        }

        // Create snapshot
        let reason = match &req {
            Request::AddEntry { entry_ctx } => format!("Add entry: {}", entry_ctx.service),
            Request::DeleteEntry { uuid } => {
                let entry_name = if let Some(vault) = &st.vault {
                    find_entry_by_id(vault, *uuid)
                        .map(|e| e.service.clone())
                        .unwrap_or_else(|| "Unknown".to_string())
                } else {
                    "Unknown".to_string()
                };
                format!("Delete entry: {}", entry_name)
            }
            Request::UpdateEntry { entry_ctx } => format!(
                "Update entry: {}",
                entry_ctx.service.as_deref().unwrap_or("Unknown")
            ),
            Request::UpdateMasterPassword { .. } => "Master password rotation".to_string(),
            Request::RestoreSnapshot { version, uuid } => {
                format!("Restore snapshot {} (version {})", uuid, version)
            }
            _ => "Unknown reason".into(),
        };

        let password = match st.master_password.clone() {
            Some(p) => p,
            None => return Response::Error("No password in memory".into()),
        };

        if let Some(vault) = &mut st.vault {
            match ctx
                .storage
                .create_backup_file(&vault.entries, vault.version, &password, &reason)
            {
                Ok(meta) => {
                    vault.history.push(meta);
                    prune_excess_snapshots(vault);
                }
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
        Request::UpdateMasterPassword { new_password } => {
            handle_update_master_password(ctx, new_password).await
        }
        Request::ListSnapshots => handle_list_snapshots(&ctx).await,
        Request::RestoreSnapshot { version, uuid } => {
            handle_restore_snapshot(ctx, uuid, version).await
        }
        Request::PasswordGen { pass_type } => handle_password_gen(&pass_type).await,
        Request::LoadGeneratorConfig => handle_load_generator_config().await,
    }
}

async fn handle_unlock(ctx: Context, password: String) -> Response {
    let storage = ctx.storage;
    let state = ctx.state;

    if !storage.exists() {
        warn!("Vault file not found, creating a new encrypted vault...");
        let new_vault = Vault::default();
        if let Err(e) = storage.encrypt_and_save_vault(&new_vault, &password) {
            error!("Failed to initialize vault: {}", e);
            return Response::Error(format!("Failed to initialize vault: {}", e));
        }
        let mut st = state.write().await;
        st.vault = Some(new_vault);
        st.authenticated = true;
        st.master_password = Some(password);

        return Response::Ok("Vault initialized and unlocked".into());
    }

    info!("Vault already exists, attempting to load...");
    match storage.load_vault(&password, None) {
        Ok(vault) => {
            let mut st = state.write().await;
            st.vault = Some(vault);
            st.authenticated = true;
            st.master_password = Some(password);
            Response::Ok("Vault unlocked successfully".into())
        }
        Err(e) => {
            error!("Failed to load vault: {}", e);
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

async fn handle_add_entry(ctx: Context, entry_ctx: EntryInput) -> Response {
    let mut st = ctx.state.write().await;
    let password = st.master_password.clone().unwrap();

    if let Some(vault) = &mut st.vault {
        vault.add_entry(entry_ctx.service, entry_ctx.username, entry_ctx.password);

        if let Err(e) = ctx.storage.encrypt_and_save_vault(vault, &password) {
            error!("Failed to save vault after adding entry: {}", e);
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
        warn!("Vault is not loaded.");
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
                    error!("No entry found with id: {}", uuid);
                    Response::Error(format!("No entry found with id: {}", uuid))
                }
            }
            Err(e) => {
                error!("Invalid UUID format: {}", e);
                Response::Error("Invalid UUID format".into())
            }
        }
    } else {
        warn!("Vault is not loaded.");
        Response::Error("Vault is not loaded.".into())
    }
}

async fn handle_delete_entry(ctx: Context, uuid: uuid::Uuid) -> Response {
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
                        error!("Failed to save vault: {}", e);
                        return Response::Error(format!("Failed to save vault: {}", e));
                    }
                    Response::Ok("Entry deleted successfully".into())
                } else {
                    error!("No entry found with id: {}", uuid);
                    Response::Error(format!("No entry found with id: {}", uuid))
                }
            }
            Err(_) => {
                error!("Invalid UUID format");
                Response::Error("Invalid UUID format".into())
            }
        }
    } else {
        warn!("Vault is not loaded.");
        Response::Error("Vault is not loaded.".into())
    }
}

async fn handle_update_entry(ctx: Context, entry_ctx: EntryUpdateInput) -> Response {
    let mut st: tokio::sync::RwLockWriteGuard<'_, DaemonState> = ctx.state.write().await;

    let password = st.master_password.clone();
    if let Some(vault) = &mut st.vault {
        vault.update_entry(
            entry_ctx.uuid,
            entry_ctx.service,
            entry_ctx.username,
            entry_ctx.password,
        );
        if let Some(p) = password.as_ref()
            && let Err(e) = ctx.storage.encrypt_and_save_vault(vault, p)
        {
            error!("Failed to save vault: {}", e);
            return Response::Error(format!("Failed to save vault: {}", e));
        }
        Response::Ok("Entry updated successfully".into())
    } else {
        warn!("Vault is not loaded.");
        Response::Error("Vault is not loaded.".into())
    }
}

async fn handle_update_master_password(ctx: Context, new_password: String) -> Response {
    let mut st = ctx.state.write().await;

    let Some(password) = st.master_password.clone() else {
        return Response::Error("Vault is locked or password not in memory".into());
    };

    let Some(_vault) = &mut st.vault else {
        warn!("Vault is not loaded.");
        return Response::Error("Vault is not loaded.".into());
    };

    match ctx.storage.update_vault_password(password, new_password) {
        Ok(_) => Response::Ok("Master password updated successfully".into()),
        Err(e) => Response::Error(format!("Failed to update master password: {}", e)),
    }
}

pub async fn handle_list_snapshots(ctx: &Context) -> Response {
    let st = ctx.state.read().await;

    if let Some(vault) = &st.vault {
        let payload = VaultSnapshotPayload {
            snapshots: vault.get_snapshots().clone(),
        };

        let data = serde_json::to_string(&payload).unwrap();
        Response::Ok(data)
    } else {
        Response::Error("Vault is locked".into())
    }
}

async fn handle_restore_snapshot(ctx: Context, uuid: Uuid, target_version: u32) -> Response {
    let mut st = ctx.state.write().await;
    let password = st.master_password.clone().unwrap();

    if let Some(vault) = &mut st.vault {
        let snapshot_path = match vault
            .history
            .iter()
            .find(|s| s.uuid == uuid)
            .and_then(|s| s.file_ref.as_ref())
        {
            Some(path) => path.clone(),
            None => {
                return Response::Error(
                    "The file for this snapshot has expired or does not exist.".into(),
                );
            }
        };

        let entries_to_restore = match ctx.storage.load_snapshot_entries(&password, &snapshot_path)
        {
            Ok(entries) => entries,
            Err(e) => return Response::Error(format!("Fail to load snapshot from disk: {}", e)),
        };

        let reason = format!("Rollback to v{}", target_version);
        match ctx
            .storage
            .create_backup_file(&vault.entries, vault.version, &password, &reason)
        {
            Ok(meta) => vault.history.push(meta),
            Err(e) => return Response::Error(format!("Safety snapshot failed: {}", e)),
        }

        vault.restore_entries(entries_to_restore);

        if let Err(e) = ctx.storage.encrypt_and_save_vault(vault, &password) {
            error!("Fail to save vault after rollback: {}", e);
            return Response::Error(format!("Fail to save vault after rollback: {}", e));
        }

        prune_excess_snapshots(vault);

        Response::Ok(format!(
            "Snapshot restored successfully to 'v{}'",
            target_version
        ))
    } else {
        Response::Error("Vault is not loaded.".into())
    }
}

async fn handle_password_gen(pass_type: &str) -> Response {
    let config = DaemonConfig::load_config();

    let pass_result = match pass_type {
        "passphrase" => blackout_core::generator::generate(config.password_generation.clone()),
        "password" => blackout_core::generator::generate(config.password_generation.clone()),
        _ => return Response::Error(format!("Invalid password generation type: {}", pass_type)),
    };

    match pass_result {
        Ok(generated_password) => Response::Ok(generated_password),
        Err(e) => Response::Error(format!("Failed to generate password: {}", e)),
    }
}

async fn handle_load_generator_config() -> Response {
    let config = DaemonConfig::load_config();
    Response::Ok(serde_json::to_string(&config.password_generation).unwrap_or_default())
}

// No lock
fn find_entry_by_id(vault: &Vault, uuid: Uuid) -> Option<Entry> {
    vault.get_entry_by_id(uuid)
}

fn prune_excess_snapshots(vault: &mut Vault) {
    let config = DaemonConfig::load_config();
    if config.max_snapshots == 0 {
        return;
    }

    let snapshots: Vec<&mut VaultSnapshot> = vault
        .history
        .iter_mut()
        .filter(|s| s.file_ref.is_some())
        .collect();

    let over_snaps = snapshots.len().saturating_sub(config.max_snapshots);

    if over_snaps > 0 {
        for snap in snapshots.into_iter().take(over_snaps) {
            if let Some(path) = &snap.file_ref
                && let Err(e) = std::fs::remove_file(path)
            {
                warn!(
                    "Fail to remove snapshot file '{}': {}",
                    path.display().to_string(),
                    e
                );
            }
            snap.file_ref = None;
        }
    }
}
