use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    Ping,
    Lock,
    Unlock {
        master_password: String,
    },
    AddEntry {
        service: String,
        user: String,
        password: String,
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