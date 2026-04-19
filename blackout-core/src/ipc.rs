use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::path::PathBuf;

use crate::vault::Entry;

// Context
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EntryInput {
    pub service: String,
    pub username: String,
    pub password: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EntryUpdateInput {
    pub uuid: Uuid,
    pub service: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VaultListPayload {
    pub entries: Vec<Entry>,
    pub version: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    Ping,
    Lock,
    Unlock {
        master_password: String,
    },
    AddEntry {
        entry_ctx: EntryInput,
    },
    ListEntries,
    GetEntry {
        service: String,
    },
    GetEntryById {
        uuid: uuid::Uuid,
    },
    DeleteEntry {
        uuid: uuid::Uuid,
    },
    UpdateEntry {
        entry_ctx: EntryUpdateInput,
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Ok(String),
    Error(String),
}

pub fn get_socket_path() -> PathBuf {
    let uid = unsafe { libc::geteuid() };
    let base_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/run/user/{}", uid));
    
    PathBuf::from(base_dir).join("blackout.sock")
}