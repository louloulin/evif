//! EVIF Core Error Types
//!
//! 统一错误处理,支持所有插件和服务组件。
//!
//! # P1-1: 增强错误处理
//! - trace_id 支持用于请求追踪 (待添加到 Internal)
//! - 结构化错误码 (error_code)
//! - 错误码映射表
//!
//! # 错误处理
//!
//! 使用 `EvifResult<T>` 作为标准返回类型:
//! ```rust,ignore
//! use evif_core::{EvifResult, EvifError};
//!
//! fn read_file(path: &str) -> EvifResult<Vec<u8>> {
//!     if path.is_empty() {
//!         Err(EvifError::InvalidPath("empty path".to_string()))
//!     } else {
//!         Ok(vec![])
//!     }
//! }
//! ```
//!
//! # 错误来源
//!
//! - **IO**: 文件系统读写错误
//! - **Path**: 路径相关错误 (NotFound, AlreadyExists)
//! - **Plugin**: 插件相关错误 (NotSupported, NotMounted)
//! - **Handle**: 句柄相关错误 (HandleNotFound, HandleClosed)
//! - **Network**: 网络相关错误 (Http, Network, Timeout)

use std::io;

/// P1-1: 错误码常量 (用于结构化响应)
pub mod error_codes {
    /// 标准错误码前缀
    pub const EVIF_PREFIX: &str = "EVIF";
    
    // 4xx 客户端错误
    pub const NOT_FOUND: &str = "EVIF_0404";
    pub const BAD_REQUEST: &str = "EVIF_0400";
    pub const UNAUTHORIZED: &str = "EVIF_0401";
    pub const FORBIDDEN: &str = "EVIF_0403";
    pub const CONFLICT: &str = "EVIF_0409";
    
    // 5xx 服务器错误
    pub const INTERNAL_ERROR: &str = "EVIF_0500";
    pub const NOT_IMPLEMENTED: &str = "EVIF_0501";
    pub const SERVICE_UNAVAILABLE: &str = "EVIF_0503";
    pub const TIMEOUT: &str = "EVIF_0504";
}

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

    /// Phase 14.2: 文件锁冲突
    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Operation not supported by plugin '{plugin_name}': {operation}")]
    NotSupported {
        plugin_name: String,
        operation: String,
    },

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

// P1-1: 错误码映射和工具方法
impl EvifError {
    /// 获取结构化错误码 (用于 API 响应)
    pub fn error_code(&self) -> &'static str {
        use error_codes::*;
        match self {
            EvifError::NotFound(_) => NOT_FOUND,
            EvifError::AlreadyExists(_) => CONFLICT,
            EvifError::InvalidPath(_) => BAD_REQUEST,
            EvifError::InvalidInput(_) => BAD_REQUEST,
            EvifError::PermissionDenied(_) => FORBIDDEN,
            EvifError::Authentication(_) => UNAUTHORIZED,
            EvifError::Timeout(_) => TIMEOUT,
            EvifError::Internal(_) => INTERNAL_ERROR,
            _ => INTERNAL_ERROR,
        }
    }

    /// 转换为可追踪的结构化格式
    pub fn to_structured(&self) -> StructuredError {
        StructuredError {
            error: self.error_code().to_string(),
            message: self.to_string(),
            source: std::error::Error::source(self).map(|e| e.to_string()),
        }
    }
}

/// P1-1: 结构化错误响应 (用于 API)
#[derive(Debug, serde::Serialize)]
pub struct StructuredError {
    /// 错误码 (如 "EVIF_0404")
    pub error: String,
    /// 人类可读错误消息
    pub message: String,
    /// 错误来源 (如果有)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}
