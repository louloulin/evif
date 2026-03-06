// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use crate::{NodeId, Result, GraphError};

/// 边唯一标识符
pub type EdgeId = Uuid;

/// 边类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeType {
    /// 父子关系（目录包含）
    Parent,
    /// 引用关系
    Reference,
    /// 依赖关系
    Dependency,
    /// 符号链接
    Symlink,
    // ===== 时序关系 =====
    /// 之前（时序）- 源节点发生在目标节点之前
    Before,
    /// 之后（时序）- 源节点发生在目标节点之后
    After,
    /// 同时（时序）- 源节点与目标节点同时发生
    Simultaneous,
    /// 导致（因果）- 源节点导致目标节点
    Causes,
    // ===== 记忆关系 =====
    /// 属于 - 记忆项属于某个类别
    BelongsTo,
    /// 派生自 - 记忆项派生自某个资源
    DerivedFrom,
    /// 引用 - 记忆项交叉引用
    References,
    /// 相似 - 记忆项相似
    SimilarTo,
    /// 自定义类型
    Custom(String),
}

impl EdgeType {
    pub fn as_str(&self) -> &str {
        match self {
            EdgeType::Parent => "parent",
            EdgeType::Reference => "reference",
            EdgeType::Dependency => "dependency",
            EdgeType::Symlink => "symlink",
            // 时序关系
            EdgeType::Before => "before",
            EdgeType::After => "after",
            EdgeType::Simultaneous => "simultaneous",
            EdgeType::Causes => "causes",
            // 记忆关系
            EdgeType::BelongsTo => "belongs_to",
            EdgeType::DerivedFrom => "derived_from",
            EdgeType::References => "references",
            EdgeType::SimilarTo => "similar_to",
            EdgeType::Custom(s) => s.as_str(),
        }
    }

    /// 检查是否为时序边类型
    pub fn is_temporal(&self) -> bool {
        matches!(
            self,
            EdgeType::Before | EdgeType::After | EdgeType::Simultaneous | EdgeType::Causes
        )
    }

    /// 检查是否为记忆相关边类型
    pub fn is_memory_related(&self) -> bool {
        matches!(
            self,
            EdgeType::BelongsTo
                | EdgeType::DerivedFrom
                | EdgeType::References
                | EdgeType::SimilarTo
        )
    }
}

/// 边 - 表示节点之间的关系
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// 边 ID
    pub id: EdgeId,
    /// 源节点
    pub source: NodeId,
    /// 目标节点
    pub target: NodeId,
    /// 边类型
    pub edge_type: EdgeType,
    /// 权重（可选，用于算法）
    pub weight: Option<f64>,
    /// 属性
    pub properties: HashMap<String, serde_json::Value>,
}

impl Edge {
    /// 创建新边
    pub fn new(source: NodeId, target: NodeId, edge_type: EdgeType) -> Self {
        Edge {
            id: Uuid::new_v4(),
            source,
            target,
            edge_type,
            weight: None,
            properties: HashMap::new(),
        }
    }

    /// 设置权重
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = Some(weight);
        self
    }

    /// 添加属性
    pub fn with_property(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.properties.insert(key.into(), value);
        self
    }

    /// 检查是否形成自环
    pub fn is_self_loop(&self) -> bool {
        self.source == self.target
    }
}

/// 边构建器
pub struct EdgeBuilder {
    edge: Edge,
}

impl EdgeBuilder {
    pub fn new(source: NodeId, target: NodeId, edge_type: EdgeType) -> Self {
        EdgeBuilder {
            edge: Edge::new(source, target, edge_type),
        }
    }

    pub fn with_id(mut self, id: EdgeId) -> Self {
        self.edge.id = id;
        self
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.edge.weight = Some(weight);
        self
    }

    pub fn with_property(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.edge.properties.insert(key.into(), value);
        self
    }

    pub fn build(self) -> Edge {
        self.edge
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_creation() {
        let source = Uuid::new_v4();
        let target = Uuid::new_v4();
        let edge = Edge::new(source, target, EdgeType::Parent);

        assert_eq!(edge.source, source);
        assert_eq!(edge.target, target);
    }

    #[test]
    fn test_self_loop_detection() {
        let id = Uuid::new_v4();
        let edge = Edge::new(id, id, EdgeType::Reference);
        assert!(edge.is_self_loop());
    }
}
