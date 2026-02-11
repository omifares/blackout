use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use argon2::{Argon2};
use rand::RngCore;
use rand::rngs::OsRng;
use zeroize::Zeroize;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Entry {
    pub id: Uuid,
    pub service: String,
    pub username: String,
    pub secret: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Vault {
    pub version: u32,
    pub entries: Vec<Entry>,
}

impl Vault {
    pub fn add_entry(&mut self, service: String, user: String, pass: String) {
        let new_entry = Entry {
            id: Uuid::now_v7(),
            service,
            username: user,
            secret: pass,
            updated_at: Utc::now(),
        };
        self.entries.push(new_entry);
        self.version += 1;
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
