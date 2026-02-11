use crate::vault::{Vault, derive_key_argon2id};
use std::{fs};
use std::path::{PathBuf};
use std::error::Error;
use std::result::Result;
use rand::RngCore;
use serde_cbor;
use chacha20poly1305::XChaCha20Poly1305;
use chacha20poly1305::aead::{Aead, KeyInit, OsRng};
use chacha20poly1305::XNonce;
use std::io::{Write};

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
        let path = dirs::home_dir().unwrap().join(".blackout");
        if !path.exists() {
            fs::create_dir_all(&path).expect("Failed to create directory");
        }
        Self { path }
    }

    pub fn load_vault(&self, password: &str) -> Result<Vault, Box<dyn Error>> {
        let file_path = self.path.join("vault.blackout");
        let bytes = fs::read(file_path)?;

        // Deserialize the encrypted vault
        let encrypted: EncryptedVault = serde_cbor::from_slice(&bytes)?;
        let (derived_key, _) = derive_key_argon2id(password, Some(&encrypted.salt), 16, 32)?;

        // Decrypt the vault
        let aead = XChaCha20Poly1305::new(
            chacha20poly1305::Key::from_slice(&derived_key[..32])
        );
        let nonce = XNonce::from_slice(&encrypted.nonce);
        
        let plaintext = aead.decrypt(nonce, encrypted.ciphertext.as_ref())
            .map_err(|_| "Decryption failed")?;

        let vault: Vault = serde_cbor::from_slice(&plaintext)?;

        Ok(vault)
    }

    pub fn exists(&self) -> bool {
        self.path.join("vault.blackout").exists()
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
        password: &str
    ) -> Result<(), Box<dyn Error>> {
        let file_path: PathBuf = self.path.join("vault.blackout");
        let temp_path: PathBuf = self.path.join("vault.tmp");

        let serialized = serde_cbor::to_vec(vault).expect("Failed to serialize vault");

        // Derive key from password using Argon2id
        let (derived_key, salt) = derive_key_argon2id(password, None, 16, 32)?;

        let aead = XChaCha20Poly1305::new(
            chacha20poly1305::Key::from_slice(&derived_key[..32])
        );
        
        // Generate a random nonce
        let mut nonce_bytes = [0u8; 24];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = XNonce::from_slice(&nonce_bytes);

        // Encrypt the vault
        let ciphertext = aead.encrypt(nonce, serialized.as_ref())
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

}
