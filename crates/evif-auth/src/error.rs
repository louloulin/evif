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
