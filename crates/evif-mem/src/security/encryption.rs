//! Encryption module placeholder
//!
//! Provides encryption utilities for sensitive data in memory storage.
//! This is a simplified implementation using SHA-256 for key derivation.

use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::error::{MemError, MemResult};

/// Encryption configuration
#[derive(Debug, Clone)]
pub struct EncryptionConfig {
    /// Master key for encryption (32 bytes or password string)
    pub master_key: Vec<u8>,
    /// Enable encryption flag
    pub enabled: bool,
}

impl EncryptionConfig {
    /// Create new config with master key
    pub fn new(master_key: Vec<u8>) -> MemResult<Self> {
        Ok(Self {
            master_key,
            enabled: true,
        })
    }

    /// Create config from password
    pub fn from_password(password: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        let key = hasher.finalize().to_vec();

        Self {
            master_key: key,
            enabled: true,
        }
    }

    /// Create default config
    #[cfg(feature = "security")]
    pub fn default() -> Self {
        let mut key = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        Self {
            master_key: key,
            enabled: true,
        }
    }
}

/// Simple encryption using XOR with key derivation
/// Note: This is a placeholder. In production, use proper AES-GCM.
#[derive(Debug, Clone)]
pub struct Encryption {
    config: EncryptionConfig,
}

impl Encryption {
    /// Create new encryption instance
    pub fn new(config: EncryptionConfig) -> MemResult<Self> {
        Ok(Self { config })
    }

    /// Encrypt data (XOR-based - NOT for production use)
    pub fn encrypt(&self, plaintext: &[u8]) -> MemResult<Vec<u8>> {
        if !self.config.enabled {
            return Ok(plaintext.to_vec());
        }

        // Generate random salt
        let mut salt = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut salt);

        // Derive key using SHA-256
        let mut hasher = Sha256::new();
        hasher.update(&self.config.master_key);
        hasher.update(&salt);
        let key: Vec<u8> = hasher.finalize().to_vec();

        // XOR plaintext with derived key
        let mut result = Vec::with_capacity(16 + plaintext.len());
        result.extend_from_slice(&salt);
        for (i, byte) in plaintext.iter().enumerate() {
            result.push(byte ^ key[i % key.len()]);
        }

        Ok(result)
    }

    /// Decrypt data
    pub fn decrypt(&self, data: &[u8]) -> MemResult<Vec<u8>> {
        if !self.config.enabled {
            return Ok(data.to_vec());
        }

        if data.len() < 16 {
            return Err(MemError::Security("Data too short".to_string()));
        }

        // Extract salt and ciphertext
        let salt = &data[..16];
        let ciphertext = &data[16..];

        // Derive key using SHA-256
        let mut hasher = Sha256::new();
        hasher.update(&self.config.master_key);
        hasher.update(salt);
        let key: Vec<u8> = hasher.finalize().to_vec();

        // XOR ciphertext with derived key
        let mut result = Vec::with_capacity(ciphertext.len());
        for (i, byte) in ciphertext.iter().enumerate() {
            result.push(byte ^ key[i % key.len()]);
        }

        Ok(result)
    }

    /// Encrypt string data
    pub fn encrypt_string(&self, plaintext: &str) -> MemResult<Vec<u8>> {
        self.encrypt(plaintext.as_bytes())
    }

    /// Decrypt to string
    pub fn decrypt_string(&self, data: &[u8]) -> MemResult<String> {
        let decrypted = self.decrypt(data)?;
        String::from_utf8(decrypted)
            .map_err(|e| MemError::Security(format!("Invalid UTF-8: {}", e)))
    }
}

#[cfg(feature = "security")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_from_password() {
        let config = EncryptionConfig::from_password("my_secure_password");
        assert!(config.enabled);
    }

    #[test]
    fn test_encrypt_decrypt() {
        let config = EncryptionConfig::default();
        let encryption = Encryption::new(config).unwrap();

        let plaintext = b"Hello, World! This is a secret message.";
        let encrypted = encryption.encrypt(plaintext).unwrap();
        let decrypted = encryption.decrypt(&encrypted).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_encrypt_decrypt_string() {
        let config = EncryptionConfig::default();
        let encryption = Encryption::new(config).unwrap();

        let plaintext = "Hello, 世界！Sensitive Data 🔑 🔐";
        let encrypted = encryption.encrypt_string(plaintext).unwrap();
        let decrypted = encryption.decrypt_string(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_disabled_encryption() {
        let mut config = EncryptionConfig::default();
        config.enabled = false;
        let encryption = Encryption::new(config).unwrap();

        let plaintext = b"plain text";
        let encrypted = encryption.encrypt(plaintext).unwrap();

        assert_eq!(plaintext.to_vec(), encrypted);
    }
}
