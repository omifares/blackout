use crate::vault::{Entry, Vault, VaultSnapshot, derive_key_argon2id};
use chacha20poly1305::XChaCha20Poly1305;
use chacha20poly1305::XNonce;
use chacha20poly1305::aead::{Aead, KeyInit, OsRng};
use chrono::Local;
use rand::RngCore;
use serde_cbor;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::result::Result;

const VAULT_FILENAME: &str = "vault.blackout";

#[derive(serde::Serialize, serde::Deserialize)]
pub struct EncryptedVault {
    pub salt: Vec<u8>,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

pub struct Wallet {
    pub path: PathBuf,
}

impl Wallet {
    pub fn init() -> Self {
        // $XDG_DATA_LOCAL_DIR/share/blackout
        let path = dirs::data_local_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap().join(".local/share"))
            .join("blackout");
        if !path.exists() {
            fs::create_dir_all(&path).expect("Failed to create directory");
        }
        Self { path }
    }

    pub fn load_vault(
        &self,
        password: &str,
        custom_path: Option<&PathBuf>,
    ) -> Result<Vault, Box<dyn Error + Send + Sync>> {
        let default_vault_path: PathBuf = self.path.join(VAULT_FILENAME);
        let file_path = custom_path.cloned().unwrap_or(default_vault_path);
        let bytes = fs::read(file_path)?;

        // Deserialize the encrypted vault
        let encrypted: EncryptedVault = serde_cbor::from_slice(&bytes)?;
        let (derived_key, _) = derive_key_argon2id(password, Some(&encrypted.salt), 16, 32)?;

        // Decrypt the vault
        let aead = XChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(&derived_key[..32]));
        let nonce = XNonce::from_slice(&encrypted.nonce);

        let plaintext = aead
            .decrypt(nonce, encrypted.ciphertext.as_ref())
            .map_err(|_| "Decryption failed")?;

        let vault: Vault = serde_cbor::from_slice(&plaintext)?;

        Ok(vault)
    }

    pub fn load_snapshot_entries(
        &self,
        password: &str,
        backup_path: &PathBuf,
    ) -> Result<Vec<Entry>, Box<dyn Error + Send + Sync>> {
        let bytes = fs::read(backup_path)?;

        // Deserialize the encrypted vault
        let encrypted: EncryptedVault = serde_cbor::from_slice(&bytes)?;
        let (derived_key, _) = derive_key_argon2id(password, Some(&encrypted.salt), 16, 32)?;

        // Decrypt the vault
        let aead = XChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(&derived_key[..32]));
        let nonce = XNonce::from_slice(&encrypted.nonce);

        let plaintext = aead
            .decrypt(nonce, encrypted.ciphertext.as_ref())
            .map_err(|_| "Decryption failed")?;

        let entries: Vec<Entry> = serde_cbor::from_slice(&plaintext)?;

        Ok(entries)
    }

    pub fn exists(&self) -> bool {
        self.path.join(VAULT_FILENAME).exists()
    }

    pub fn delete_vault(&self) -> std::io::Result<()> {
        let file_path = self.path.join("vault.blackout");
        if file_path.exists() {
            fs::remove_file(file_path)?;
        }
        Ok(())
    }

    pub fn encrypt_and_save_vault(
        &self,
        vault: &Vault,
        password: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let file_path: PathBuf = self.path.join(VAULT_FILENAME);
        let temp_path: PathBuf = self.path.join("vault.tmp");

        let serialized = serde_cbor::to_vec(vault).expect("Failed to serialize vault");

        // Derive key from password using Argon2id
        let (derived_key, salt) = derive_key_argon2id(password, None, 16, 32)?;

        let aead = XChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(&derived_key[..32]));

        // Generate a random nonce
        let mut nonce_bytes = [0u8; 24];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = XNonce::from_slice(&nonce_bytes);

        // Encrypt the vault
        let ciphertext = aead
            .encrypt(nonce, serialized.as_ref())
            .map_err(|_| "Encryption failed")?;

        // Create encrypted vault structure
        let encrypted = EncryptedVault {
            salt: salt.clone(),
            nonce: nonce_bytes.to_vec(),
            ciphertext,
        };

        // Serialize and write
        let encrypted_bytes = serde_cbor::to_vec(&encrypted)?;
        let mut writer = fs::File::create(&temp_path)?;
        writer.write_all(&encrypted_bytes)?;
        writer.flush()?;
        fs::rename(&temp_path, &file_path)?;

        Ok(())
    }

    pub fn calculate_sha256(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        hex::encode(result)
    }

    pub fn create_backup_file(
        &self,
        entries: &Vec<Entry>,
        version: u32,
        password: &str,
        reason: &str,
    ) -> Result<VaultSnapshot, Box<dyn Error + Send + Sync>> {
        let snapshots_dir = self.path.join(".snapshots");
        if !snapshots_dir.exists() {
            fs::create_dir_all(&snapshots_dir)?;
        }

        let file_name = format!("v{}.blackout.bak", version);
        let file_path = snapshots_dir.join(&file_name);
        let temp_file_name = file_name + ".tmp";
        let temp_path = snapshots_dir.join(&temp_file_name);

        // Serialize
        let serialized = serde_cbor::to_vec(entries)?;

        // Encrypt
        let (derived_key, salt) = derive_key_argon2id(password, None, 16, 32)?;
        let aead = XChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(&derived_key));

        let mut nonce_bytes = [0u8; 24];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = XNonce::from_slice(&nonce_bytes);

        let ciphertext = aead
            .encrypt(nonce, serialized.as_ref())
            .map_err(|_| "Backup encryption failed")?;

        let encrypted_backup = EncryptedVault {
            salt,
            nonce: nonce_bytes.to_vec(),
            ciphertext,
        };

        // Checksum
        let final_bytes = serde_cbor::to_vec(&encrypted_backup)?;
        let checksum = Self::calculate_sha256(&final_bytes);

        // Atomic write
        let mut writer = fs::File::create(&temp_path)?;
        writer.write_all(&final_bytes)?;
        writer.flush()?;
        fs::rename(&temp_path, &file_path)?;

        Ok(VaultSnapshot {
            uuid: uuid::Uuid::now_v7(),
            version,
            created_at: Local::now(),
            checksum,
            reason: reason.into(),
            file_ref: Some(PathBuf::new().join(file_path.to_string_lossy().into_owned())),
        })
    }

    pub fn update_vault_password(
        &self,
        current_passsword: String,
        new_password: String,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let vault = self.load_vault(&current_passsword, None)?;
        self.encrypt_and_save_vault(&vault, &new_password)?;
        Ok(())
    }
}
