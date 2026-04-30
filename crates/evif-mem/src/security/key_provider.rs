// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Key Provider Trait - 密钥管理抽象
//!
//! 本模块定义了 KeyProvider trait，支持多种密钥管理方案：
//! - LocalKeyProvider: 本地文件存储（默认）
//! - AwsKmsProvider: AWS KMS 集成（待实现）
//! - AzureKeyVaultProvider: Azure Key Vault 集成（待实现）
//!
//! 架构设计:
//! 1. 抽象 KeyProvider trait 定义密钥操作接口
//! 2. 各 provider 实现具体密钥管理逻辑
//! 3. Encryption 使用 KeyProvider 获取密钥
//! 4. 支持密钥轮换（rotation）和版本管理

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

/// 密钥标识符
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct KeyId(pub String);

impl KeyId {
    pub fn new(id: impl Into<String>) -> Self {
        KeyId(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for KeyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Key({})", self.0)
    }
}

/// 密钥版本
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyVersion(pub u32);

impl KeyVersion {
    pub fn new(v: u32) -> Self {
        KeyVersion(v)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

impl Default for KeyVersion {
    fn default() -> Self {
        KeyVersion(1)
    }
}

impl fmt::Display for KeyVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

/// 密钥元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    pub id: KeyId,
    pub version: KeyVersion,
    pub algorithm: KeyAlgorithm,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub rotated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub is_active: bool,
}

impl KeyMetadata {
    pub fn new(id: KeyId, algorithm: KeyAlgorithm) -> Self {
        Self {
            id,
            version: KeyVersion::default(),
            algorithm,
            created_at: chrono::Utc::now(),
            rotated_at: None,
            is_active: true,
        }
    }
}

/// 密钥算法
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum KeyAlgorithm {
    /// AES-256-GCM（当前唯一支持）
    Aes256Gcm,
    /// AES-256-GCM-SIV（待支持）
    Aes256GcmSiv,
    /// ChaCha20-Poly1305（待支持）
    ChaCha20Poly1305,
}

impl Default for KeyAlgorithm {
    fn default() -> Self {
        KeyAlgorithm::Aes256Gcm
    }
}

/// 密钥提供错误
#[derive(Debug, Clone)]
pub enum KeyProviderError {
    /// 密钥不存在
    KeyNotFound(KeyId),
    /// 密钥版本不存在
    KeyVersionNotFound(KeyId, KeyVersion),
    /// 密钥获取失败
    GetKeyFailed(String),
    /// 密钥创建失败
    CreateKeyFailed(String),
    /// 密钥删除失败
    DeleteKeyFailed(String),
    /// 密钥轮换失败
    RotateKeyFailed(String),
    /// 未授权访问
    Unauthorized(String),
    /// 网络错误（KMS 提供商）
    NetworkError(String),
    /// 配置错误
    ConfigError(String),
    /// IO 错误
    IoError(String),
}

impl fmt::Display for KeyProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyProviderError::KeyNotFound(id) => write!(f, "Key not found: {}", id),
            KeyProviderError::KeyVersionNotFound(id, v) => {
                write!(f, "Key version not found: {} @ {}", id, v)
            }
            KeyProviderError::GetKeyFailed(msg) => write!(f, "Failed to get key: {}", msg),
            KeyProviderError::CreateKeyFailed(msg) => write!(f, "Failed to create key: {}", msg),
            KeyProviderError::DeleteKeyFailed(msg) => write!(f, "Failed to delete key: {}", msg),
            KeyProviderError::RotateKeyFailed(msg) => write!(f, "Failed to rotate key: {}", msg),
            KeyProviderError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            KeyProviderError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            KeyProviderError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            KeyProviderError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for KeyProviderError {}

/// 密钥管理提供 trait
///
/// 定义密钥操作的统一接口，支持：
/// - 获取密钥（get_key）
/// - 创建密钥（create_key）
/// - 删除密钥（delete_key）
/// - 密钥轮换（rotate_key）
/// - 列出密钥（list_keys）
/// - 获取密钥元数据（get_metadata）
///
/// # 设计原则
/// 1. **异步优先**：所有操作都是异步的，支持高并发场景
/// 2. **版本化**：每个密钥支持多版本，便于轮换
/// 3. **元数据驱动**：支持密钥元数据查询
/// 4. **统一错误**：统一的错误类型便于处理
///
/// # 示例
/// ```ignore
/// use evif_mem::security::key_provider::{LocalKeyProvider, KeyId};
///
/// let provider = LocalKeyProvider::new("/var/lib/evif/keys")?;
/// let key = provider.get_key(&KeyId::new("master")).await?;
/// ```
#[async_trait]
pub trait KeyProvider: Send + Sync {
    /// 获取当前版本的密钥
    async fn get_key(&self, id: &KeyId) -> Result<Vec<u8>, KeyProviderError>;

    /// 获取指定版本的密钥
    async fn get_key_version(
        &self,
        id: &KeyId,
        version: KeyVersion,
    ) -> Result<Vec<u8>, KeyProviderError>;

    /// 获取密钥元数据
    async fn get_metadata(&self, id: &KeyId) -> Result<KeyMetadata, KeyProviderError>;

    /// 创建新密钥
    async fn create_key(
        &self,
        id: &KeyId,
        key: Vec<u8>,
        algorithm: KeyAlgorithm,
    ) -> Result<KeyMetadata, KeyProviderError>;

    /// 删除密钥
    async fn delete_key(&self, id: &KeyId) -> Result<(), KeyProviderError>;

    /// 轮换密钥（创建新版本）
    async fn rotate_key(
        &self,
        id: &KeyId,
        new_key: Vec<u8>,
    ) -> Result<KeyMetadata, KeyProviderError>;

    /// 列出所有密钥 ID
    async fn list_keys(&self) -> Result<Vec<KeyId>, KeyProviderError>;

    /// 检查密钥是否存在
    async fn exists(&self, id: &KeyId) -> Result<bool, KeyProviderError>;
}

// ============================================================================
// Local Key Provider
// ============================================================================

/// 本地文件存储密钥提供程序
///
/// 将密钥存储在本地文件系统中。
///
/// # 文件结构
/// ```
/// key_dir/
/// ├── {key_id}/
/// │   ├── metadata.json    # 密钥元数据
/// │   ├── v1              # 版本 1 的密钥
/// │   └── v2              # 版本 2 的密钥
/// ```
///
/// # 安全性
/// - 密钥文件权限应设置为 0600
/// - 建议使用文件系统加密（如 dm-crypt）
/// - 生产环境建议使用 HSM/KMS
#[derive(Debug, Clone)]
pub struct LocalKeyProvider {
    base_path: PathBuf,
}

impl LocalKeyProvider {
    /// 创建新的本地密钥提供程序
    pub fn new<P: Into<PathBuf>>(base_path: P) -> Result<Self, KeyProviderError> {
        let base_path = base_path.into();

        // 创建基础目录
        if !base_path.exists() {
            std::fs::create_dir_all(&base_path).map_err(|e| {
                KeyProviderError::IoError(format!("Failed to create key directory: {}", e))
            })?;
        }

        Ok(Self { base_path })
    }

    /// 从环境变量创建
    pub fn from_env() -> Result<Self, KeyProviderError> {
        let path = std::env::var("EVIF_KEY_PROVIDER_PATH")
            .unwrap_or_else(|_| "/var/lib/evif/keys".to_string());
        Self::new(path)
    }

    /// 获取密钥目录路径
    fn key_dir(&self, id: &KeyId) -> PathBuf {
        self.base_path.join(&id.0)
    }

    /// 获取密钥文件路径
    fn key_file(&self, id: &KeyId, version: KeyVersion) -> PathBuf {
        self.key_dir(id).join(format!("v{}", version.as_u32()))
    }

    /// 获取元数据文件路径
    fn metadata_file(&self, id: &KeyId) -> PathBuf {
        self.key_dir(id).join("metadata.json")
    }

    fn read_key_file(&self, path: &PathBuf) -> Result<Vec<u8>, KeyProviderError> {
        std::fs::read(path).map_err(|e| KeyProviderError::IoError(format!("Failed to read key: {}", e)))
    }

    fn write_key_file(&self, path: &PathBuf, key: &[u8]) -> Result<(), KeyProviderError> {
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                KeyProviderError::IoError(format!("Failed to create key directory: {}", e))
            })?;
        }

        std::fs::write(path, key).map_err(|e| {
            KeyProviderError::IoError(format!("Failed to write key: {}", e))
        })?;

        // 设置文件权限为 0600
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(path, perms).map_err(|e| {
                KeyProviderError::IoError(format!("Failed to set key permissions: {}", e))
            })?;
        }

        Ok(())
    }

    fn read_metadata(&self, id: &KeyId) -> Result<KeyMetadata, KeyProviderError> {
        let path = self.metadata_file(id);
        let content = std::fs::read_to_string(&path).map_err(|e| {
            KeyProviderError::IoError(format!("Failed to read metadata: {}", e))
        })?;
        serde_json::from_str(&content).map_err(|e| {
            KeyProviderError::ConfigError(format!("Failed to parse metadata: {}", e))
        })
    }

    fn write_metadata(&self, id: &KeyId, metadata: &KeyMetadata) -> Result<(), KeyProviderError> {
        let path = self.metadata_file(id);
        let content = serde_json::to_string_pretty(metadata).map_err(|e| {
            KeyProviderError::ConfigError(format!("Failed to serialize metadata: {}", e))
        })?;
        std::fs::write(&path, content).map_err(|e| {
            KeyProviderError::IoError(format!("Failed to write metadata: {}", e))
        })?;
        Ok(())
    }
}

#[async_trait]
impl KeyProvider for LocalKeyProvider {
    async fn get_key(&self, id: &KeyId) -> Result<Vec<u8>, KeyProviderError> {
        let metadata = self.get_metadata(id).await?;
        if !metadata.is_active {
            return Err(KeyProviderError::KeyNotFound(id.clone()));
        }
        self.get_key_version(id, metadata.version).await
    }

    async fn get_key_version(
        &self,
        id: &KeyId,
        version: KeyVersion,
    ) -> Result<Vec<u8>, KeyProviderError> {
        let path = self.key_file(id, version);
        if !path.exists() {
            return Err(KeyProviderError::KeyVersionNotFound(id.clone(), version));
        }
        self.read_key_file(&path)
    }

    async fn get_metadata(&self, id: &KeyId) -> Result<KeyMetadata, KeyProviderError> {
        let path = self.metadata_file(id);
        if !path.exists() {
            return Err(KeyProviderError::KeyNotFound(id.clone()));
        }
        self.read_metadata(id)
    }

    async fn create_key(
        &self,
        id: &KeyId,
        key: Vec<u8>,
        algorithm: KeyAlgorithm,
    ) -> Result<KeyMetadata, KeyProviderError> {
        let key_dir = self.key_dir(id);
        if key_dir.exists() {
            return Err(KeyProviderError::CreateKeyFailed(format!(
                "Key already exists: {}",
                id
            )));
        }

        // 创建目录
        std::fs::create_dir_all(&key_dir).map_err(|e| {
            KeyProviderError::IoError(format!("Failed to create key directory: {}", e))
        })?;

        // 写入密钥文件
        let key_path = self.key_file(id, KeyVersion::default());
        self.write_key_file(&key_path, &key)?;

        // 创建元数据
        let metadata = KeyMetadata::new(id.clone(), algorithm);
        self.write_metadata(id, &metadata)?;

        Ok(metadata)
    }

    async fn delete_key(&self, id: &KeyId) -> Result<(), KeyProviderError> {
        let key_dir = self.key_dir(id);
        if !key_dir.exists() {
            return Err(KeyProviderError::KeyNotFound(id.clone()));
        }

        std::fs::remove_dir_all(&key_dir).map_err(|e| {
            KeyProviderError::DeleteKeyFailed(format!("Failed to delete key directory: {}", e))
        })?;

        Ok(())
    }

    async fn rotate_key(
        &self,
        id: &KeyId,
        new_key: Vec<u8>,
    ) -> Result<KeyMetadata, KeyProviderError> {
        let mut metadata = self.get_metadata(id).await?;
        let new_version = KeyVersion(metadata.version.as_u32() + 1);

        // 写入新版本密钥
        let key_path = self.key_file(id, new_version);
        self.write_key_file(&key_path, &new_key)?;

        // 更新元数据
        metadata.version = new_version;
        metadata.rotated_at = Some(chrono::Utc::now());
        self.write_metadata(id, &metadata)?;

        Ok(metadata)
    }

    async fn list_keys(&self) -> Result<Vec<KeyId>, KeyProviderError> {
        if !self.base_path.exists() {
            return Ok(Vec::new());
        }

        let mut keys = Vec::new();
        for entry in std::fs::read_dir(&self.base_path).map_err(|e| {
            KeyProviderError::IoError(format!("Failed to read key directory: {}", e))
        })? {
            let entry = entry.map_err(|e| {
                KeyProviderError::IoError(format!("Failed to read directory entry: {}", e))
            })?;
            let name = entry.file_name();
            if entry.file_type().map_err(|e| {
                KeyProviderError::IoError(format!("Failed to get file type: {}", e))
            })?.is_dir()
            {
                keys.push(KeyId::new(name.to_string_lossy()));
            }
        }

        Ok(keys)
    }

    async fn exists(&self, id: &KeyId) -> Result<bool, KeyProviderError> {
        Ok(self.metadata_file(id).exists())
    }
}

// ============================================================================
// Key Provider Registry
// ============================================================================

/// 密钥提供程序注册表
///
/// 统一管理多种密钥提供程序。
#[derive(Clone)]
pub struct KeyProviderRegistry {
    providers: std::collections::HashMap<String, Arc<dyn KeyProvider>>,
    default_provider: Option<String>,
}

impl KeyProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: std::collections::HashMap::new(),
            default_provider: None,
        }
    }

    /// 注册密钥提供程序
    pub fn register(&mut self, name: impl Into<String>, provider: Arc<dyn KeyProvider>) {
        let name = name.into();
        if self.default_provider.is_none() {
            self.default_provider = Some(name.clone());
        }
        self.providers.insert(name, provider);
    }

    /// 获取默认提供程序
    pub fn get_default(&self) -> Option<Arc<dyn KeyProvider>> {
        self.default_provider
            .as_ref()
            .and_then(|name| self.providers.get(name).cloned())
    }

    /// 按名称获取提供程序
    pub fn get(&self, name: &str) -> Option<Arc<dyn KeyProvider>> {
        self.providers.get(name).cloned()
    }

    /// 获取所有已注册的提供程序名称
    pub fn names(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}

impl fmt::Debug for KeyProviderRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyProviderRegistry")
            .field("providers", &self.providers.keys().collect::<Vec<_>>())
            .field("default_provider", &self.default_provider)
            .finish()
    }
}

impl Default for KeyProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_local_key_provider_crud() {
        let temp_dir = TempDir::new().unwrap();
        let provider = LocalKeyProvider::new(temp_dir.path()).unwrap();
        let key_id = KeyId::new("test-key");
        let key_data = vec![0x01, 0x02, 0x03, 0x04];

        // Create
        let metadata = provider
            .create_key(&key_id, key_data.clone(), KeyAlgorithm::Aes256Gcm)
            .await
            .unwrap();
        assert_eq!(metadata.id, key_id);
        assert_eq!(metadata.version, KeyVersion::default());

        // Get
        let retrieved = provider.get_key(&key_id).await.unwrap();
        assert_eq!(retrieved, key_data);

        // Get metadata
        let meta = provider.get_metadata(&key_id).await.unwrap();
        assert!(meta.is_active);

        // List
        let keys = provider.list_keys().await.unwrap();
        assert!(keys.contains(&key_id));

        // Exists
        assert!(provider.exists(&key_id).await.unwrap());

        // Delete
        provider.delete_key(&key_id).await.unwrap();
        assert!(!provider.exists(&key_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_key_rotation() {
        let temp_dir = TempDir::new().unwrap();
        let provider = LocalKeyProvider::new(temp_dir.path()).unwrap();
        let key_id = KeyId::new("rotate-test");
        let key_v1 = vec![0x11; 32];
        let key_v2 = vec![0x22; 32];

        // Create v1
        provider
            .create_key(&key_id, key_v1.clone(), KeyAlgorithm::Aes256Gcm)
            .await
            .unwrap();

        // Rotate to v2
        let meta = provider.rotate_key(&key_id, key_v2.clone()).await.unwrap();
        assert_eq!(meta.version, KeyVersion(2));

        // Get should return v2
        let current = provider.get_key(&key_id).await.unwrap();
        assert_eq!(current, key_v2);

        // Get v1 should still work
        let v1 = provider
            .get_key_version(&key_id, KeyVersion(1))
            .await
            .unwrap();
        assert_eq!(v1, key_v1);
    }

    #[tokio::test]
    async fn test_key_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let provider = LocalKeyProvider::new(temp_dir.path()).unwrap();
        let key_id = KeyId::new("nonexistent");

        let result = provider.get_key(&key_id).await;
        assert!(matches!(result, Err(KeyProviderError::KeyNotFound(_))));
    }

    #[tokio::test]
    async fn test_key_provider_registry() {
        let temp_dir = TempDir::new().unwrap();
        let provider: Arc<dyn KeyProvider> =
            Arc::new(LocalKeyProvider::new(temp_dir.path()).unwrap());

        let mut registry = KeyProviderRegistry::new();
        registry.register("local", provider.clone());

        // Get default
        let default = registry.get_default();
        assert!(default.is_some());

        // Get by name
        let by_name = registry.get("local");
        assert!(by_name.is_some());

        // Names
        let names = registry.names();
        assert_eq!(names, vec!["local".to_string()]);
    }
}
