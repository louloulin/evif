// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fmt;

pub type StorageResult<T> = std::result::Result<T, StorageError>;

#[derive(Debug, Clone, PartialEq)]
pub enum StorageError {
    /// 节点不存在
    NodeNotFound(uuid::Uuid),
    /// 边不存在
    EdgeNotFound(uuid::Uuid),
    /// IO 错误
    IoError(String),
    /// 序列化错误
    SerializationError(String),
    /// 反序列化错误
    DeserializationError(String),
    /// 后端错误
    BackendError(String),
    /// 事务错误
    TransactionError(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::NodeNotFound(id) => write!(f, "节点不存在: {}", id),
            StorageError::EdgeNotFound(id) => write!(f, "边不存在: {}", id),
            StorageError::IoError(msg) => write!(f, "IO 错误: {}", msg),
            StorageError::SerializationError(msg) => write!(f, "序列化错误: {}", msg),
            StorageError::DeserializationError(msg) => write!(f, "反序列化错误: {}", msg),
            StorageError::BackendError(msg) => write!(f, "后端错误: {}", msg),
            StorageError::TransactionError(msg) => write!(f, "事务错误: {}", msg),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        StorageError::SerializationError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let id = uuid::Uuid::new_v4();
        let err = StorageError::NodeNotFound(id);
        assert!(err.to_string().contains("节点不存在"));
    }
}
