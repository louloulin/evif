//! Production-grade encryption module using AES-256-GCM
//!
//! Provides secure encryption for sensitive data in memory storage.
//! Uses PBKDF2 for key derivation and AES-256-GCM for authenticated encryption.

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use pbkdf2::pbkdf2_hmac_array;
use rand::RngCore;
use sha2::Sha256;

use crate::error::{MemError, MemResult};

/// Salt size for key derivation
const SALT_SIZE: usize = 16;
/// Nonce size for AES-GCM
const NONCE_SIZE: usize = 12;
/// Number of PBKDF2 iterations
const PBKDF2_ITERATIONS: u32 = 100_000;

/// Encryption configuration
#[derive(Debug, Clone)]
pub struct EncryptionConfig {
    /// Master key for encryption (32 bytes)
    pub master_key: Vec<u8>,
    /// Salt for key derivation (optional, for deterministic testing)
    pub salt: Option<[u8; SALT_SIZE]>,
    /// Enable encryption flag
    pub enabled: bool,
}

impl EncryptionConfig {
    /// Create new config with master key
    pub fn new(master_key: Vec<u8>) -> MemResult<Self> {
        Ok(Self {
            master_key,
            salt: None,
            enabled: true,
        })
    }

    /// Create config from password using PBKDF2
    pub fn from_password(password: &str, salt: Option<[u8; SALT_SIZE]>) -> Self {
        let salt = salt.unwrap_or_else(|| {
            let mut s = [0u8; SALT_SIZE];
            OsRng.fill_bytes(&mut s);
            s
        });

        // Derive 32-byte key using PBKDF2
        let key = pbkdf2_hmac_array::<Sha256, 32>(
            password.as_bytes(),
            &salt,
            PBKDF2_ITERATIONS,
        );

        Self {
            master_key: key.to_vec(),
            salt: Some(salt),
            enabled: true,
        }
    }

    /// Create default config with random key
    #[cfg(feature = "security")]
    pub fn default() -> Self {
        let mut key = vec![0u8; 32];
        OsRng.fill_bytes(&mut key);
        Self {
            master_key: key,
            salt: None,
            enabled: true,
        }
    }

    /// Get the salt (if any)
    pub fn salt(&self) -> Option<&[u8; SALT_SIZE]> {
        self.salt.as_ref()
    }
}

/// Production-grade encryption using AES-256-GCM
/// Provides authenticated encryption with PBKDF2 key derivation
#[derive(Debug, Clone)]
pub struct Encryption {
    config: EncryptionConfig,
}

impl Encryption {
    /// Create new encryption instance
    pub fn new(config: EncryptionConfig) -> MemResult<Self> {
        if config.master_key.len() != 32 {
            return Err(MemError::Security(
                "Master key must be 32 bytes".to_string(),
            ));
        }
        Ok(Self { config })
    }

    /// Encrypt data using AES-256-GCM
    pub fn encrypt(&self, plaintext: &[u8]) -> MemResult<Vec<u8>> {
        if !self.config.enabled {
            return Ok(plaintext.to_vec());
        }

        // Create cipher with derived key
        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&self.config.master_key);
        let cipher = Aes256Gcm::new(key);

        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt with AES-GCM (provides authenticated encryption)
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| MemError::Security(format!("Encryption failed: {}", e)))?;

        // Format: salt (16 bytes) + nonce (12 bytes) + ciphertext
        let mut result = Vec::with_capacity(SALT_SIZE + NONCE_SIZE + ciphertext.len());

        // Use existing salt or generate new one
        if let Some(salt) = &self.config.salt {
            result.extend_from_slice(salt);
        } else {
            let mut salt = [0u8; SALT_SIZE];
            OsRng.fill_bytes(&mut salt);
            result.extend_from_slice(&salt);
        }

        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// Decrypt data using AES-256-GCM
    pub fn decrypt(&self, data: &[u8]) -> MemResult<Vec<u8>> {
        if !self.config.enabled {
            return Ok(data.to_vec());
        }

        let min_len = SALT_SIZE + NONCE_SIZE + 16; // salt + nonce + minimum ciphertext
        if data.len() < min_len {
            return Err(MemError::Security("Data too short".to_string()));
        }

        // Extract salt, nonce, and ciphertext
        let _salt = &data[..SALT_SIZE];
        let nonce = Nonce::from_slice(&data[SALT_SIZE..SALT_SIZE + NONCE_SIZE]);
        let ciphertext = &data[SALT_SIZE + NONCE_SIZE..];

        // Create cipher with key
        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&self.config.master_key);
        let cipher = Aes256Gcm::new(key);

        // Decrypt (AEAD will verify integrity)
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| MemError::Security("Decryption failed: invalid key or corrupted data".to_string()))?;

        Ok(plaintext)
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
        let salt = [1u8; SALT_SIZE];
        let config = EncryptionConfig::from_password("my_secure_password", Some(salt));
        assert!(config.enabled);
        assert_eq!(config.master_key.len(), 32);
    }

    #[test]
    fn test_encrypt_decrypt() {
        let config = EncryptionConfig::default();
        let encryption = Encryption::new(config).unwrap();

        let plaintext = b"Hello, World! This is a secret message.";
        let encrypted = encryption.encrypt(plaintext).unwrap();
        let decrypted = encryption.decrypt(&encrypted).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
        // Verify ciphertext is different from plaintext
        assert_ne!(plaintext.to_vec(), encrypted);
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

    #[test]
    fn test_wrong_key_fails() {
        let mut config1 = EncryptionConfig::default();
        let mut config2 = EncryptionConfig::default();

        // Make keys different
        config1.master_key[0] ^= 0xFF;
        config2.master_key[0] = 0x00;

        let encryption1 = Encryption::new(config1).unwrap();
        let encryption2 = Encryption::new(config2).unwrap();

        let plaintext = b"Secret message";
        let encrypted = encryption1.encrypt(plaintext).unwrap();

        // Decryption with wrong key should fail
        let result = encryption2.decrypt(&encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_deterministic_with_same_salt() {
        let salt = [42u8; SALT_SIZE];
        let config1 = EncryptionConfig::from_password("password123", Some(salt));
        let config2 = EncryptionConfig::from_password("password123", Some(salt));

        let encryption1 = Encryption::new(config1).unwrap();
        let encryption2 = Encryption::new(config2).unwrap();

        let plaintext = b"Test data";
        let encrypted1 = encryption1.encrypt(plaintext).unwrap();
        let encrypted2 = encryption2.encrypt(plaintext).unwrap();

        // Different nonce each time, so ciphertext differs
        assert_ne!(encrypted1, encrypted2);

        // But both can decrypt correctly
        assert_eq!(plaintext.to_vec(), encryption1.decrypt(&encrypted1).unwrap());
        assert_eq!(plaintext.to_vec(), encryption2.decrypt(&encrypted2).unwrap());
    }
}
