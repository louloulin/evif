// Phase 17.2: Encryption-at-rest support
//
// 提供文件数据加密存储功能 (AES-256-GCM)

use crate::{RestError, RestResult};
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Encryption status
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EncryptionStatus {
    Disabled,
    Enabled,
    KeyMissing,
}

/// Encryption configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub status: EncryptionStatus,
    pub algorithm: String,
    pub key_source: String,
}

/// Request to enable encryption
#[derive(Debug, Deserialize)]
pub struct EnableEncryptionRequest {
    /// Base64-encoded 256-bit key, or "env:KEY_NAME" to load from environment
    pub key: String,
}

/// Encryption state manager
#[derive(Clone)]
pub struct EncryptionState {
    inner: Arc<RwLock<EncryptionInner>>,
}

struct EncryptionInner {
    enabled: bool,
    cipher: Option<Aes256Gcm>,
    key_source: String,
}

impl Default for EncryptionState {
    fn default() -> Self {
        Self::new()
    }
}

impl EncryptionState {
    pub fn new() -> Self {
        // Check if EVIF_ENCRYPTION_KEY environment variable is set
        let key = std::env::var("EVIF_ENCRYPTION_KEY").ok();
        let inner = if let Some(key) = key {
            if let Ok(cipher) = Self::create_cipher(&key) {
                EncryptionInner {
                    enabled: true,
                    cipher: Some(cipher),
                    key_source: "env:EVIF_ENCRYPTION_KEY".to_string(),
                }
            } else {
                EncryptionInner {
                    enabled: false,
                    cipher: None,
                    key_source: "env:EVIF_ENCRYPTION_KEY (invalid)".to_string(),
                }
            }
        } else {
            EncryptionInner {
                enabled: false,
                cipher: None,
                key_source: String::new(),
            }
        };

        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    fn create_cipher(key: &str) -> Result<Aes256Gcm, String> {
        // Support "env:NAME" format
        let key = if key.starts_with("env:") {
            std::env::var(&key[4..]).map_err(|_| "Environment variable not set")?
        } else {
            key.to_string()
        };

        // Derive 256-bit key from input using SHA-256
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let key_bytes: [u8; 32] = hasher.finalize().into();

        Aes256Gcm::new_from_slice(&key_bytes).map_err(|e| format!("{:?}", e))
    }

    pub fn get_config(&self) -> EncryptionConfig {
        let inner = self.inner.read();
        EncryptionConfig {
            status: if inner.enabled {
                EncryptionStatus::Enabled
            } else if inner.key_source.is_empty() {
                EncryptionStatus::Disabled
            } else {
                EncryptionStatus::KeyMissing
            },
            algorithm: "AES-256-GCM".to_string(),
            key_source: inner.key_source.clone(),
        }
    }

    pub async fn enable(&self, key: String) -> Result<EncryptionConfig, String> {
        let cipher = Self::create_cipher(&key)?;
        let key_source = if key.starts_with("env:") {
            format!("env:{} (enabled)", &key[4..])
        } else {
            "provided".to_string()
        };

        let mut inner = self.inner.write();
        inner.enabled = true;
        inner.cipher = Some(cipher);
        inner.key_source = key_source;

        Ok(EncryptionConfig {
            status: EncryptionStatus::Enabled,
            algorithm: "AES-256-GCM".to_string(),
            key_source: inner.key_source.clone(),
        })
    }

    pub async fn disable(&self) -> EncryptionConfig {
        let mut inner = self.inner.write();
        inner.enabled = false;
        inner.cipher = None;
        inner.key_source = String::new();

        EncryptionConfig {
            status: EncryptionStatus::Disabled,
            algorithm: "AES-256-GCM".to_string(),
            key_source: String::new(),
        }
    }

    /// Encrypt data with AES-256-GCM
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let inner = self.inner.read();
        let cipher = inner.cipher.as_ref().ok_or("Encryption not enabled")?;

        let nonce_bytes: [u8; 12] = rand::RngCore::fill_bytes(&mut OsRng, &mut [0u8; 12]).into();
        // Generate random nonce
        let mut nonce_arr = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_arr);
        let nonce = Nonce::from_slice(&nonce_arr);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| format!("Encryption failed: {:?}", e))?;

        // Prepend nonce to ciphertext
        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_arr);
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    /// Decrypt data with AES-256-GCM
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() < 12 {
            return Err("Data too short to contain nonce".to_string());
        }

        let inner = self.inner.read();
        let cipher = inner.cipher.as_ref().ok_or("Encryption not enabled")?;

        let nonce = Nonce::from_slice(&data[..12]);
        let ciphertext = &data[12..];

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed: {:?}", e))
    }

    pub fn is_enabled(&self) -> bool {
        self.inner.read().enabled
    }
}

/// Encryption handlers
pub struct EncryptionHandlers;

impl EncryptionHandlers {
    /// GET /api/v1/encryption/status - Get encryption status
    pub async fn get_status(
        State(state): State<EncryptionState>,
    ) -> RestResult<impl IntoResponse> {
        Ok(Json(state.get_config()))
    }

    /// POST /api/v1/encryption/enable - Enable encryption
    pub async fn enable(
        State(state): State<EncryptionState>,
        Json(req): Json<EnableEncryptionRequest>,
    ) -> RestResult<impl IntoResponse> {
        if req.key.is_empty() {
            return Err(RestError::BadRequest("Encryption key cannot be empty".into()));
        }

        match state.enable(req.key).await {
            Ok(config) => Ok(Json(config)),
            Err(e) => Err(RestError::BadRequest(e)),
        }
    }

    /// POST /api/v1/encryption/disable - Disable encryption
    pub async fn disable(
        State(state): State<EncryptionState>,
    ) -> RestResult<impl IntoResponse> {
        let config = state.disable().await;
        Ok(Json(config))
    }
}
