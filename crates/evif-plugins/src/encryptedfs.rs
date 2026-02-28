// EncryptedFS - Transparent Encryption Plugin
//
// Provides transparent encryption/decryption for stored files using:
// - AES-256-GCM for authenticated encryption
// - Argon2id for key derivation
// - Per-file unique nonces
// - Metadata versioning for future algorithm upgrades

use evif_core::{
    EvifPlugin, FileInfo, EvifResult, EvifError, WriteFlags, PluginConfigParam,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm,
};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, SaltString},
    Argon2, Params,
};
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose};

const NONCE_SIZE: usize = 12; // 96 bits for GCM
const TAG_SIZE: usize = 16; // 128-bit authentication tag
const KEY_SIZE: usize = 32; // 256 bits for AES-256

/// Encryption algorithm version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionVersion {
    V1 = 1, // AES-256-GCM + Argon2id
}

/// Encrypted file metadata stored at the beginning of each file
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedFileHeader {
    version: u8,
    nonce: String, // Base64 encoded nonce
    tag: String,   // Base64 encoded auth tag
}

impl EncryptedFileHeader {
    fn new(nonce: &[u8], tag: &[u8]) -> Self {
        Self {
            version: EncryptionVersion::V1 as u8,
            nonce: general_purpose::STANDARD.encode(nonce),
            tag: general_purpose::STANDARD.encode(tag),
        }
    }

    fn to_bytes(&self) -> EvifResult<Vec<u8>> {
        let json = serde_json::to_vec(self)
            .map_err(|e| EvifError::InvalidInput(format!("Failed to serialize header: {}", e)))?;
        // Format: {length:4 bytes}{json_data}{newline}
        let len = json.len() as u32;
        let mut result = Vec::with_capacity(4 + json.len() + 1);
        result.extend_from_slice(&len.to_be_bytes());
        result.extend_from_slice(&json);
        result.push(b'\n');
        Ok(result)
    }

    fn from_bytes(data: &[u8]) -> EvifResult<Self> {
        if data.len() < 5 {
            return Err(EvifError::InvalidInput("Header too short".to_string()));
        }

        let len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        if data.len() < 4 + len + 1 {
            return Err(EvifError::InvalidInput("Incomplete header".to_string()));
        }

        let json = &data[4..4 + len];
        serde_json::from_slice(json)
            .map_err(|e| EvifError::InvalidInput(format!("Failed to parse header: {}", e)))
    }
}

/// Configuration for EncryptedFS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedConfig {
    /// Master password for encryption (should be stored securely)
    pub master_password: String,

    /// Argon2id memory cost in KB
    pub argon2_memory_kb: u32,

    /// Argon2id iterations
    pub argon2_iterations: u32,

    /// Argon2id parallelism
    pub argon2_parallelism: u32,
}

impl Default for EncryptedConfig {
    fn default() -> Self {
        Self {
            master_password: String::new(),
            argon2_memory_kb: 65536, // 64 MB
            argon2_iterations: 3,
            argon2_parallelism: 4,
        }
    }
}

/// EncryptedFS plugin
pub struct EncryptedFsPlugin {
    backend: Arc<dyn EvifPlugin>,
    cipher: Aes256Gcm,
    config: EncryptedConfig,
    file_cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl EncryptedFsPlugin {
    /// Create a new EncryptedFS plugin
    ///
    /// # Arguments
    /// * `backend` - The underlying storage backend
    /// * `config` - Encryption configuration including master password
    pub fn new(backend: Arc<dyn EvifPlugin>, config: EncryptedConfig) -> EvifResult<Self> {
        let cipher_key = Self::derive_key(&config)?;

        // Convert key array to bytes for KeyInit
        let cipher = Aes256Gcm::new_from_slice(&cipher_key)
            .map_err(|e| EvifError::InvalidInput(format!("Failed to create cipher: {}", e)))?;

        Ok(Self {
            backend,
            cipher,
            config,
            file_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Derive encryption key from password using Argon2id
    fn derive_key(config: &EncryptedConfig) -> EvifResult<[u8; KEY_SIZE]> {
        // Use simpler params that match argon2 crate expectations
        let params = Params::new(
            config.argon2_memory_kb.try_into()
                .map_err(|e| EvifError::InvalidInput(format!("Invalid memory size: {}", e)))?,
            config.argon2_iterations,
            config.argon2_parallelism,
            Some(KEY_SIZE),
        ).map_err(|e| EvifError::InvalidInput(format!("Invalid Argon2 params: {}", e)))?;

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

        let password_hash = argon2.hash_password(
            config.master_password.as_bytes(),
            &salt,
        ).map_err(|e| EvifError::InvalidInput(format!("Key derivation failed: {}", e)))?;

        let hash_string = password_hash.to_string();
        let hash = PasswordHash::new(&hash_string)
            .map_err(|e| EvifError::InvalidInput(format!("Invalid hash format: {}", e)))?;

        // Extract the hash bytes as the key
        let mut key = [0u8; KEY_SIZE];
        if let Some(hash_bytes) = hash.hash {
            let bytes = hash_bytes.as_bytes();
            key.copy_from_slice(&bytes[..KEY_SIZE.min(bytes.len())]);
        }

        Ok(key)
    }

    /// Encrypt data with authenticated encryption
    fn encrypt(&self, plaintext: &[u8]) -> EvifResult<Vec<u8>> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let ciphertext = self.cipher.encrypt(&nonce, plaintext)
            .map_err(|e| EvifError::InvalidInput(format!("Encryption failed: {}", e)))?;

        // In GCM, the ciphertext returned by the encrypt method already includes the tag
        // Split to extract nonce and tag for storage
        let nonce_bytes = nonce.as_slice();

        // For GCM, the last 16 bytes are the auth tag
        if ciphertext.len() < TAG_SIZE {
            return Err(EvifError::InvalidInput("Ciphertext too short".to_string()));
        }

        let tag_pos = ciphertext.len() - TAG_SIZE;
        let encrypted_data = &ciphertext[..tag_pos];
        let tag = &ciphertext[tag_pos..];

        // Create header with nonce and tag
        let header = EncryptedFileHeader::new(nonce_bytes, tag);
        let header_bytes = header.to_bytes()?;

        // Combine header + encrypted data
        let mut result = header_bytes;
        result.extend_from_slice(encrypted_data);

        Ok(result)
    }

    /// Decrypt data with authentication
    fn decrypt(&self, data: &[u8]) -> EvifResult<Vec<u8>> {
        // Parse header
        let header = EncryptedFileHeader::from_bytes(data)?;

        // Decode nonce and tag
        let nonce_bytes = general_purpose::STANDARD.decode(&header.nonce)
            .map_err(|e| EvifError::InvalidInput(format!("Invalid nonce encoding: {}", e)))?;

        let tag = general_purpose::STANDARD.decode(&header.tag)
            .map_err(|e| EvifError::InvalidInput(format!("Invalid tag encoding: {}", e)))?;

        // Calculate header size
        let header_size = {
            let len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
            4 + len + 1
        };

        // Extract encrypted data
        let encrypted_data = &data[header_size..];

        // Combine ciphertext + tag for decryption
        let mut ciphertext_with_tag = Vec::with_capacity(encrypted_data.len() + tag.len());
        ciphertext_with_tag.extend_from_slice(encrypted_data);
        ciphertext_with_tag.extend_from_slice(&tag);

        // Decrypt - convert nonce bytes to GenericArray format
        // Use aes_gcm's internal Nonce type
        let nonce = aes_gcm::aead::Nonce::<Aes256Gcm>::from_slice(&nonce_bytes);

        let plaintext = self.cipher.decrypt(nonce, ciphertext_with_tag.as_ref())
            .map_err(|e| EvifError::InvalidInput(format!("Decryption failed: {}. This may indicate data corruption or wrong password.", e)))?;

        Ok(plaintext)
    }
}

#[async_trait::async_trait]
impl EvifPlugin for EncryptedFsPlugin {
    fn name(&self) -> &str {
        "EncryptedFS"
    }

    async fn create(&self, path: &str, perm: u32) -> EvifResult<()> {
        self.backend.create(path, perm).await
    }

    async fn mkdir(&self, path: &str, perm: u32) -> EvifResult<()> {
        self.backend.mkdir(path, perm).await
    }

    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>> {
        let encrypted = self.backend.read(path, offset, size).await?;

        if encrypted.is_empty() {
            return Ok(encrypted);
        }

        // Decrypt the data
        let decrypted = self.decrypt(&encrypted)?;

        Ok(decrypted)
    }

    async fn write(&self, path: &str, data: Vec<u8>, _offset: i64, _flags: WriteFlags) -> EvifResult<u64> {
        // Encrypt the data
        let encrypted = self.encrypt(&data)?;

        // Write encrypted data
        let written = self.backend.write(path, encrypted, 0, WriteFlags::NONE).await?;

        // Cache the plaintext for subsequent reads (optimization)
        let mut cache = self.file_cache.write().await;
        cache.insert(path.to_string(), data);

        Ok(written)
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        self.backend.readdir(path).await
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        self.backend.stat(path).await
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        let mut cache = self.file_cache.write().await;
        cache.remove(path);
        self.backend.remove(path).await
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        let mut cache = self.file_cache.write().await;
        cache.remove(path);
        self.backend.remove_all(path).await
    }

    async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()> {
        self.backend.rename(old_path, new_path).await?;

        // Update cache
        let mut cache = self.file_cache.write().await;
        if let Some(data) = cache.remove(old_path) {
            cache.insert(new_path.to_string(), data);
        }

        Ok(())
    }

    async fn chmod(&self, path: &str, mode: u32) -> EvifResult<()> {
        self.backend.chmod(path, mode).await
    }

    async fn truncate(&self, path: &str, size: u64) -> EvifResult<()> {
        self.backend.truncate(path, size).await
    }

    async fn validate(&self, _config: Option<&serde_json::Value>) -> EvifResult<()> {
        Ok(())
    }

    fn get_readme(&self) -> String {
        r#"
# EncryptedFS Plugin

Transparent encryption layer that automatically encrypts all data before storage.

## Features

- **AES-256-GCM**: Authenticated encryption with 256-bit keys
- **Argon2id**: Memory-hard key derivation for password protection
- **Per-file Nonces**: Unique nonce for each encrypted file
- **Authentication**: Built-in data integrity verification
- **Transparent**: Automatic encryption/decryption

## Security

- AES-256-GCM provides both confidentiality and integrity
- Argon2id protects against brute-force and dictionary attacks
- Each file uses a unique nonce to prevent nonce reuse
- Authentication tag detects any tampering

## Configuration

```json
{
  "master_password": "your-secure-password-here",
  "argon2_memory_kb": 65536,
  "argon2_iterations": 3,
  "argon2_parallelism": 4
}
```

**Important**: Store your master password securely. Losing it means losing access to all encrypted data.

## Usage

Files are automatically encrypted on write and decrypted on read:

```bash
# Write (automatically encrypted)
echo "secret data" > /encrypted/file.txt

# Read (automatically decrypted)
cat /encrypted/file.txt
```

## File Format

Each encrypted file has:
1. Header (JSON): version, nonce, tag
2. Encrypted data

The header allows for future algorithm upgrades.
"#.to_string()
    }

    fn get_config_params(&self) -> Vec<PluginConfigParam> {
        vec![
            PluginConfigParam {
                name: "master_password".to_string(),
                param_type: "string".to_string(),
                description: Some("Master password for encryption (required)".to_string()),
                required: true,
                default: None,
            },
            PluginConfigParam {
                name: "argon2_memory_kb".to_string(),
                param_type: "number".to_string(),
                description: Some("Argon2 memory cost in KB".to_string()),
                required: false,
                default: Some("65536".to_string()),
            },
            PluginConfigParam {
                name: "argon2_iterations".to_string(),
                param_type: "number".to_string(),
                description: Some("Argon2 iterations".to_string()),
                required: false,
                default: Some("3".to_string()),
            },
            PluginConfigParam {
                name: "argon2_parallelism".to_string(),
                param_type: "number".to_string(),
                description: Some("Argon2 parallelism".to_string()),
                required: false,
                default: Some("4".to_string()),
            },
        ]
    }
}
