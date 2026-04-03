// Phase 17.2: Encryption-at-rest support
//
// 提供文件数据加密存储功能 (AES-256-GCM)

use crate::{RestError, RestResult};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::{RngCore, rngs::OsRng};
use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
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
    persistence_path: Arc<Option<PathBuf>>,
}

struct EncryptionInner {
    enabled: bool,
    cipher: Option<Aes256Gcm>,
    key_source: String,
    key_reference: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EncryptionSnapshot {
    enabled: bool,
    key_source: String,
    key_reference: Option<String>,
}

impl Default for EncryptionState {
    fn default() -> Self {
        Self::new()
    }
}

impl EncryptionState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Self::runtime_inner())),
            persistence_path: Arc::new(None),
        }
    }

    pub fn from_env() -> Result<Self, String> {
        match std::env::var("EVIF_REST_ENCRYPTION_STATE_PATH") {
            Ok(path) if !path.trim().is_empty() => Self::persistent(path.trim()),
            _ => Ok(Self::new()),
        }
    }

    pub fn persistent(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        let inner = if path.exists() {
            let snapshot = Self::load_snapshot(&path)?;
            Self::inner_from_snapshot(snapshot)
        } else {
            let inner = Self::runtime_inner();
            let snapshot = Self::snapshot_from_inner(&inner);
            Self::save_snapshot(&path, &snapshot)?;
            inner
        };

        Ok(Self {
            inner: Arc::new(RwLock::new(inner)),
            persistence_path: Arc::new(Some(path)),
        })
    }

    fn create_cipher(key: &str) -> Result<Aes256Gcm, String> {
        // Support "env:NAME" format
        let key = if let Some(env_name) = key.strip_prefix("env:") {
            std::env::var(env_name).map_err(|_| "Environment variable not set")?
        } else {
            key.to_string()
        };

        // Derive 256-bit key from input using SHA-256
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let key_bytes: [u8; 32] = hasher.finalize().into();

        Aes256Gcm::new_from_slice(&key_bytes).map_err(|e| format!("{:?}", e))
    }

    fn runtime_inner() -> EncryptionInner {
        if std::env::var("EVIF_ENCRYPTION_KEY").is_ok() {
            match Self::create_cipher("env:EVIF_ENCRYPTION_KEY") {
                Ok(cipher) => EncryptionInner {
                    enabled: true,
                    cipher: Some(cipher),
                    key_source: "env:EVIF_ENCRYPTION_KEY".to_string(),
                    key_reference: Some("env:EVIF_ENCRYPTION_KEY".to_string()),
                },
                Err(_) => EncryptionInner {
                    enabled: false,
                    cipher: None,
                    key_source: "env:EVIF_ENCRYPTION_KEY (invalid)".to_string(),
                    key_reference: Some("env:EVIF_ENCRYPTION_KEY".to_string()),
                },
            }
        } else {
            EncryptionInner {
                enabled: false,
                cipher: None,
                key_source: String::new(),
                key_reference: None,
            }
        }
    }

    fn snapshot_from_inner(inner: &EncryptionInner) -> EncryptionSnapshot {
        EncryptionSnapshot {
            enabled: inner.enabled,
            key_source: inner.key_source.clone(),
            key_reference: inner.key_reference.clone(),
        }
    }

    fn inner_from_snapshot(snapshot: EncryptionSnapshot) -> EncryptionInner {
        if !snapshot.enabled {
            return EncryptionInner {
                enabled: false,
                cipher: None,
                key_source: snapshot.key_source,
                key_reference: snapshot.key_reference,
            };
        }

        match snapshot.key_reference.as_deref() {
            Some(key_reference) => match Self::create_cipher(key_reference) {
                Ok(cipher) => EncryptionInner {
                    enabled: true,
                    cipher: Some(cipher),
                    key_source: snapshot.key_source,
                    key_reference: Some(key_reference.to_string()),
                },
                Err(_) => EncryptionInner {
                    enabled: false,
                    cipher: None,
                    key_source: snapshot.key_source,
                    key_reference: Some(key_reference.to_string()),
                },
            },
            None => EncryptionInner {
                enabled: false,
                cipher: None,
                key_source: snapshot.key_source,
                key_reference: None,
            },
        }
    }

    fn load_snapshot(path: &Path) -> Result<EncryptionSnapshot, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read encryption state '{}': {}", path.display(), e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse encryption state '{}': {}", path.display(), e))
    }

    fn save_snapshot(path: &Path, snapshot: &EncryptionSnapshot) -> Result<(), String> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "Failed to create encryption state parent '{}': {}",
                    parent.display(),
                    e
                )
            })?;
        }
        let content = serde_json::to_string_pretty(snapshot)
            .map_err(|e| format!("Failed to serialize encryption state: {}", e))?;
        fs::write(path, content).map_err(|e| {
            format!(
                "Failed to write encryption state '{}': {}",
                path.display(),
                e
            )
        })
    }

    fn persist_inner(&self, inner: &EncryptionInner) -> Result<(), String> {
        if let Some(path) = self.persistence_path.as_ref().as_ref() {
            let snapshot = Self::snapshot_from_inner(inner);
            Self::save_snapshot(path, &snapshot)?;
        }
        Ok(())
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
        let key_source = if let Some(env_name) = key.strip_prefix("env:") {
            format!("env:{} (enabled)", env_name)
        } else {
            "provided".to_string()
        };
        let key_reference = key.strip_prefix("env:").map(|_| key.clone());

        let mut inner = self.inner.write();
        inner.enabled = true;
        inner.cipher = Some(cipher);
        inner.key_source = key_source;
        inner.key_reference = key_reference;
        self.persist_inner(&inner)?;

        Ok(EncryptionConfig {
            status: EncryptionStatus::Enabled,
            algorithm: "AES-256-GCM".to_string(),
            key_source: inner.key_source.clone(),
        })
    }

    pub async fn disable(&self) -> Result<EncryptionConfig, String> {
        let mut inner = self.inner.write();
        inner.enabled = false;
        inner.cipher = None;
        inner.key_source = String::new();
        inner.key_reference = None;
        self.persist_inner(&inner)?;

        Ok(EncryptionConfig {
            status: EncryptionStatus::Disabled,
            algorithm: "AES-256-GCM".to_string(),
            key_source: String::new(),
        })
    }

    /// Encrypt data with AES-256-GCM
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let inner = self.inner.read();
        let cipher = inner.cipher.as_ref().ok_or("Encryption not enabled")?;

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
        let config = state.disable().await.map_err(RestError::Internal)?;
        Ok(Json(config))
    }
}
