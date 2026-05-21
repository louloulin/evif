// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fmt;

pub type AuthResult<T> = std::result::Result<T, AuthError>;

#[derive(Debug, Clone, PartialEq)]
pub enum AuthError {
    Unauthorized(String),
    Forbidden(String),
    InvalidToken(String),
    Expired,
    Internal(String),
    IoError(String),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::Unauthorized(msg) => write!(f, "未授权: {}", msg),
            AuthError::Forbidden(msg) => write!(f, "禁止访问: {}", msg),
            AuthError::InvalidToken(msg) => write!(f, "无效令牌: {}", msg),
            AuthError::Expired => write!(f, "令牌已过期"),
            AuthError::Internal(msg) => write!(f, "内部错误: {}", msg),
            AuthError::IoError(msg) => write!(f, "IO错误: {}", msg),
        }
    }
}

impl std::error::Error for AuthError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_error_unauthorized() {
        let err = AuthError::Unauthorized("missing token".to_string());
        assert!(err.to_string().contains("未授权"));
        assert!(err.to_string().contains("missing token"));
    }

    #[test]
    fn test_auth_error_forbidden() {
        let err = AuthError::Forbidden("insufficient permissions".to_string());
        assert!(err.to_string().contains("禁止访问"));
    }

    #[test]
    fn test_auth_error_invalid_token() {
        let err = AuthError::InvalidToken("malformed jwt".to_string());
        assert!(err.to_string().contains("无效令牌"));
    }

    #[test]
    fn test_auth_error_expired() {
        let err = AuthError::Expired;
        assert!(err.to_string().contains("令牌已过期"));
    }

    #[test]
    fn test_auth_error_internal() {
        let err = AuthError::Internal("config missing".to_string());
        assert!(err.to_string().contains("内部错误"));
    }

    #[test]
    fn test_auth_error_io_error() {
        let err = AuthError::IoError("file not found".to_string());
        assert!(err.to_string().contains("IO错误"));
    }

    #[test]
    fn test_auth_error_clone() {
        let err = AuthError::Unauthorized("test".to_string());
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_auth_error_debug() {
        let err = AuthError::Unauthorized("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("Unauthorized"));
    }

    #[test]
    fn test_auth_result_ok() {
        let result: AuthResult<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_auth_result_err() {
        let result: AuthResult<i32> = Err(AuthError::Expired);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::Expired));
    }
}
