// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fmt;

pub type RuntimeResult<T> = std::result::Result<T, RuntimeError>;

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeError {
    InitializationError(String),
    ConfigError(String),
    GraphError(String),
    StorageError(String),
    AuthError(String),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::InitializationError(msg) => write!(f, "初始化错误: {}", msg),
            RuntimeError::ConfigError(msg) => write!(f, "配置错误: {}", msg),
            RuntimeError::GraphError(msg) => write!(f, "图错误: {}", msg),
            RuntimeError::StorageError(msg) => write!(f, "存储错误: {}", msg),
            RuntimeError::AuthError(msg) => write!(f, "认证错误: {}", msg),
        }
    }
}

impl std::error::Error for RuntimeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = RuntimeError::ConfigError("test".to_string());
        assert!(err.to_string().contains("配置错误"));
    }
}
