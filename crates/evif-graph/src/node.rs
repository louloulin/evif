// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 节点唯一标识符
pub type NodeId = Uuid;

/// 节点类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    /// 文件节点
    File,
    /// 目录节点
    Directory,
    /// 符号链接
    Symlink,
    /// 设备文件
    Device,
    /// 进程
    Process,
    /// 网络连接
    Network,
    // ===== 记忆平台节点类型 =====
    /// 记忆项 - 从资源中提取的结构化记忆
    MemoryItem,
    /// 记忆类别 - 自动组织的主题分类
    Category,
    /// 资源 - 原始对话、文档、图像等
    Resource,
    /// 事件 - 重要事件和经历
    Event,
    /// 用户画像 - 用户基本信息和偏好
    Profile,
    /// 技能 - 技能和能力
    Skill,
    /// 工具 - 工具使用经验
    Tool,
    /// 自定义类型
    Custom(String),
}

impl NodeType {
    pub fn is_file(&self) -> bool {
        matches!(self, NodeType::File)
    }

    pub fn is_directory(&self) -> bool {
        matches!(self, NodeType::Directory)
    }

    /// 检查是否为记忆相关节点类型
    pub fn is_memory_related(&self) -> bool {
        matches!(
            self,
            NodeType::MemoryItem
                | NodeType::Category
                | NodeType::Resource
                | NodeType::Event
                | NodeType::Profile
                | NodeType::Skill
                | NodeType::Tool
        )
    }

    pub fn as_str(&self) -> &str {
        match self {
            NodeType::File => "file",
            NodeType::Directory => "directory",
            NodeType::Symlink => "symlink",
            NodeType::Device => "device",
            NodeType::Process => "process",
            NodeType::Network => "network",
            // 记忆平台节点类型
            NodeType::MemoryItem => "memory_item",
            NodeType::Category => "category",
            NodeType::Resource => "resource",
            NodeType::Event => "event",
            NodeType::Profile => "profile",
            NodeType::Skill => "skill",
            NodeType::Tool => "tool",
            NodeType::Custom(s) => s.as_str(),
        }
    }
}

/// 属性值
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Attribute {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Binary(Vec<u8>),
    DateTime(DateTime<Utc>),
    Null,
}

impl From<String> for Attribute {
    fn from(s: String) -> Self {
        Attribute::String(s)
    }
}

impl From<i64> for Attribute {
    fn from(i: i64) -> Self {
        Attribute::Integer(i)
    }
}

impl From<f64> for Attribute {
    fn from(f: f64) -> Self {
        Attribute::Float(f)
    }
}

impl From<bool> for Attribute {
    fn from(b: bool) -> Self {
        Attribute::Boolean(b)
    }
}

/// 节点元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub accessed_at: DateTime<Utc>,
    pub owner: Option<String>,
    pub permissions: u32,
    pub version: u64,
}

impl Default for Metadata {
    fn default() -> Self {
        let now = Utc::now();
        Metadata {
            created_at: now,
            modified_at: now,
            accessed_at: now,
            owner: None,
            permissions: 0o644,
            version: 1,
        }
    }
}

/// 内容句柄
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHandle {
    pub id: Uuid,
    pub size: u64,
    pub hash: Option<String>,
}

impl ContentHandle {
    pub fn new(size: u64) -> Self {
        ContentHandle {
            id: Uuid::new_v4(),
            size,
            hash: None,
        }
    }
}

/// 节点 - 图中的基本实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub node_type: NodeType,
    pub name: String,
    pub metadata: Metadata,
    pub attributes: BTreeMap<String, Attribute>,
    pub content: Option<ContentHandle>,
}

impl Node {
    pub fn new(node_type: NodeType, name: impl Into<String>) -> Self {
        Node {
            id: Uuid::new_v4(),
            node_type,
            name: name.into(),
            metadata: Metadata::default(),
            attributes: BTreeMap::new(),
            content: None,
        }
    }

    pub fn get_attr(&self, key: &str) -> Option<&Attribute> {
        self.attributes.get(key)
    }

    pub fn set_attr(&mut self, key: impl Into<String>, value: Attribute) {
        self.attributes.insert(key.into(), value);
    }

    pub fn is_dir(&self) -> bool {
        self.node_type.is_directory()
    }

    pub fn is_file(&self) -> bool {
        self.node_type.is_file()
    }
}

/// 节点构建器
pub struct NodeBuilder {
    node: Node,
}

impl NodeBuilder {
    pub fn new(node_type: NodeType, name: impl Into<String>) -> Self {
        NodeBuilder {
            node: Node::new(node_type, name),
        }
    }

    pub fn with_id(mut self, id: NodeId) -> Self {
        self.node.id = id;
        self
    }

    pub fn with_owner(mut self, owner: impl Into<String>) -> Self {
        self.node.metadata.owner = Some(owner.into());
        self
    }

    pub fn with_permissions(mut self, permissions: u32) -> Self {
        self.node.metadata.permissions = permissions;
        self
    }

    pub fn with_attr(mut self, key: impl Into<String>, value: Attribute) -> Self {
        self.node.attributes.insert(key.into(), value);
        self
    }

    pub fn build(self) -> Node {
        self.node
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = Node::new(NodeType::File, "test.txt");
        assert_eq!(node.name, "test.txt");
        assert!(node.is_file());
    }
}
