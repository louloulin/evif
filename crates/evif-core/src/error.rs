// EVIF Core Error Types
//
// 统一错误处理,支持所有插件和服务组件

use std::io;

pub type EvifResult<T> = Result<T, EvifError>;

#[derive(Debug, thiserror::Error)]
pub enum EvifError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Path not found: {0}")]
    NotFound(String),

    #[error("Path already exists: {0}")]
    AlreadyExists(String),

    #[error("Already mounted at: {0}")]
    AlreadyMounted(String),

    #[error("Not mounted: {0}")]
    NotMounted(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Read-only filesystem")]
    ReadOnly,

    #[error("Operation not supported by plugin '{plugin_name}': {operation}")]
    NotSupported { plugin_name: String, operation: String },

    #[error("Operation not supported")]
    NotSupportedGeneric,

    #[error("Empty queue: {0}")]
    EmptyQueue(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Timeout: operation timed out after {0}s")]
    Timeout(u64),

    #[error("Handle not found: {0}")]
    HandleNotFound(i64),

    #[error("Handle expired: {0}")]
    HandleExpired(i64),

    #[error("Handle closed: {0}")]
    HandleClosed(i64),

    #[error("Invalid handle flags: {0}")]
    InvalidHandleFlags(String),

    #[error("Lease expired for handle: {0}")]
    LeaseExpired(i64),

    #[error("Queue full: {0}")]
    QueueFull(String),

    #[error("Index error: {0}")]
    Index(String),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Plugin error: {0}")]
    Plugin(String),

    #[error("Plugin load error: {0}")]
    PluginLoadError(String),

    #[error("Mount error: {0}")]
    Mount(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Other error: {0}")]
    Other(String),
}

// 从 evif-storage 错误转换（简化处理）
impl From<evif_storage::StorageError> for EvifError {
    fn from(err: evif_storage::StorageError) -> Self {
        EvifError::Storage(err.to_string())
    }
}

// 从 String 转换
impl From<String> for EvifError {
    fn from(s: String) -> Self {
        EvifError::Storage(s)
    }
}

// 从 serde_json 错误转换
impl From<serde_json::Error> for EvifError {
    fn from(err: serde_json::Error) -> Self {
        EvifError::Serialization(err.to_string())
    }
}

// 从&str 转换
impl From<&str> for EvifError {
    fn from(s: &str) -> Self {
        EvifError::Other(s.to_string())
    }
}
