use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use argon2::Argon2;
use rand::RngCore;
use rand::rngs::OsRng;
use zeroize::Zeroize;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Entry {
    pub uuid: Uuid,
    pub service: String,
    pub username: String,

    #[serde(default)]
    pub secret: String,

    #[serde(default)]
    pub updated_at: DateTime<Local>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct VaultSnapshot {
    pub uuid: Uuid,
    pub version: u32,
    pub created_at: DateTime<Local>,
    pub checksum: String,
    pub file_ref: Option<std::path::PathBuf>,

    #[serde(default)]
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Vault {
    pub version: u32,
    pub entries: Vec<Entry>,

    #[serde(default)]
    pub history: Vec<VaultSnapshot>,
}

impl Vault {
    pub fn add_entry(&mut self, service: String, user: String, pass: String) {
        let new_entry = Entry {
            uuid: Uuid::now_v7(),
            service,
            username: user,
            secret: pass,
            updated_at: Local::now(),
        };
        self.entries.push(new_entry);
        self.version += 1;
    }

    pub fn list_entries(&self) -> Vec<Entry> {
        self.entries.to_vec()
    }

    // Get entry by service name (returns all matches)
    pub fn get_entry(&self, service: &str) -> Vec<Entry> {
        self.entries
            .iter()
            .filter(|e| e.service == service)
            .cloned()
            .collect()
    }

    pub fn get_entry_by_id(&self, id: Uuid) -> Option<Entry> {
        self.entries.iter().find(|e| e.uuid == id).map(|e| Entry {
            uuid: e.uuid,
            service: e.service.clone(),
            username: e.username.clone(),
            secret: e.secret.clone(),
            updated_at: e.updated_at,
        })
    }

    pub fn remove_entry(&mut self, id: Uuid) -> bool {
        if let Some(pos) = self.entries.iter().position(|e| e.uuid == id) {
            self.entries.remove(pos);
            self.version += 1;
            true
        } else {
            false
        }
    }

    pub fn update_entry(
        &mut self,
        id: Uuid,
        service: Option<String>,
        user: Option<String>,
        pass: Option<String>,
    ) -> bool {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.uuid == id) {
            if let Some(s) = service {
                entry.service = s;
            }
            if let Some(u) = user {
                entry.username = u;
            }
            if let Some(p) = pass {
                entry.secret = p;
            }
            entry.updated_at = Local::now();
            self.version += 1;
            true
        } else {
            false
        }
    }

    pub fn get_secret(&self, id: Uuid) -> Option<String> {
        self.entries
            .iter()
            .find(|e| e.uuid == id)
            .map(|e| e.secret.clone())
    }

    pub fn get_snapshots(&self) -> Vec<VaultSnapshot> {
        self.history.to_vec()
    }

    pub fn get_snapshot_by_version(&self, version: u32) -> Option<VaultSnapshot> {
        self.history.iter().find(|h| h.version == version).cloned()
    }

    pub fn restore_entries(&mut self, entries: Vec<Entry>) -> bool {
        self.version += 1;
        self.entries = entries;
        true
    }
}

/// Derive a key from a password using Argon2id.
///
/// - `password`: password/passphrase input
/// - `salt_opt`: optional salt; if `None` a random 16-byte salt is generated
/// - `salt_len`: length of salt to generate when `salt_opt` is `None` (default 16 is reasonable)
/// - `key_len`: desired output key length in bytes
///
/// Returns `(derived_key, salt_used)` on success. The derived key is allocated in a `Vec<u8>`.
pub fn derive_key_argon2id(
    password: &str,
    salt_opt: Option<&[u8]>,
    salt_len: usize,
    key_len: usize,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    // prepare salt
    let salt: Vec<u8> = match salt_opt {
        Some(s) => s.to_vec(),
        None => {
            let mut s = vec![0u8; salt_len];
            OsRng.fill_bytes(&mut s);
            s
        }
    };

    // Create Argon2id instance
    let argon2 = Argon2::default();
    let mut out = vec![0u8; key_len];

    argon2
        .hash_password_into(password.as_bytes(), &salt, &mut out)
        .map_err(|e| format!("argon2 error: {}", e))?;

    // Zeroize
    let mut _pw = password.as_bytes().to_vec();
    _pw.zeroize();

    Ok((out, salt))
}
