// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fmt;

pub type Result<T> = std::result::Result<T, GraphError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    /// 节点不存在
    NodeNotFound(uuid::Uuid),
    /// 边不存在
    EdgeNotFound(uuid::Uuid),
    /// 节点已存在
    NodeAlreadyExists(uuid::Uuid),
    /// 边已存在
    EdgeAlreadyExists(uuid::Uuid),
    /// 无效操作
    InvalidOperation(String),
    /// 循环依赖
    CycleDetected,
    /// 索引错误
    IndexError(String),
    /// 序列化错误
    SerializationError(String),
    /// IO 错误
    IoError(String),
    /// 路径不存在
    PathNotFound(uuid::Uuid, uuid::Uuid),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::NodeNotFound(id) => write!(f, "节点不存在: {}", id),
            GraphError::EdgeNotFound(id) => write!(f, "边不存在: {}", id),
            GraphError::NodeAlreadyExists(id) => write!(f, "节点已存在: {}", id),
            GraphError::EdgeAlreadyExists(id) => write!(f, "边已存在: {}", id),
            GraphError::InvalidOperation(msg) => write!(f, "无效操作: {}", msg),
            GraphError::CycleDetected => write!(f, "检测到循环依赖"),
            GraphError::IndexError(msg) => write!(f, "索引错误: {}", msg),
            GraphError::SerializationError(msg) => write!(f, "序列化错误: {}", msg),
            GraphError::IoError(msg) => write!(f, "IO 错误: {}", msg),
            GraphError::PathNotFound(from, to) => write!(f, "路径不存在: {} -> {}", from, to),
        }
    }
}

impl std::error::Error for GraphError {}

impl From<serde_json::Error> for GraphError {
    fn from(err: serde_json::Error) -> Self {
        GraphError::SerializationError(err.to_string())
    }
}
