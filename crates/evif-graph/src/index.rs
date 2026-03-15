// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{GraphError, Node, NodeId, Result};
use ahash::AHashSet;
use std::collections::HashMap;
use std::sync::RwLock;

/// 图索引 trait
pub trait GraphIndex: Send + Sync {
    /// 插入节点到索引
    fn insert(&mut self, node: &Node) -> Result<()>;

    /// 从索引删除节点
    fn remove(&mut self, id: &NodeId) -> Result<()>;

    /// 查询节点
    fn query(&self, query: &str) -> Result<Vec<NodeId>>;

    /// 优化索引
    fn optimize(&mut self) -> Result<()>;
}

/// 内存索引实现
pub struct MemoryIndex {
    /// 按类型索引
    by_type: RwLock<HashMap<String, AHashSet<NodeId>>>,
    /// 按名称索引
    by_name: RwLock<HashMap<String, AHashSet<NodeId>>>,
}

impl MemoryIndex {
    pub fn new() -> Self {
        MemoryIndex {
            by_type: RwLock::new(HashMap::new()),
            by_name: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphIndex for MemoryIndex {
    fn insert(&mut self, node: &Node) -> Result<()> {
        let node_type = node.node_type.as_str().to_string();
        let name = node.name.clone();
        let id = node.id;

        // 索引类型
        {
            let mut by_type = self
                .by_type
                .write()
                .map_err(|e| GraphError::IndexError(format!("获取写锁失败: {}", e)))?;
            by_type.entry(node_type).or_default().insert(id);
        }

        // 索引名称
        {
            let mut by_name = self
                .by_name
                .write()
                .map_err(|e| GraphError::IndexError(format!("获取写锁失败: {}", e)))?;
            by_name.entry(name).or_default().insert(id);
        }

        Ok(())
    }

    fn remove(&mut self, id: &NodeId) -> Result<()> {
        // 从所有索引中删除
        {
            let mut by_type = self
                .by_type
                .write()
                .map_err(|e| GraphError::IndexError(format!("获取写锁失败: {}", e)))?;
            for set in by_type.values_mut() {
                set.remove(id);
            }
        }

        {
            let mut by_name = self
                .by_name
                .write()
                .map_err(|e| GraphError::IndexError(format!("获取写锁失败: {}", e)))?;
            for set in by_name.values_mut() {
                set.remove(id);
            }
        }

        Ok(())
    }

    fn query(&self, query: &str) -> Result<Vec<NodeId>> {
        // 简单实现：查询名称索引
        let by_name = self
            .by_name
            .read()
            .map_err(|e| GraphError::IndexError(format!("获取读锁失败: {}", e)))?;

        if let Some(set) = by_name.get(query) {
            Ok(set.iter().copied().collect())
        } else {
            Ok(Vec::new())
        }
    }

    fn optimize(&mut self) -> Result<()> {
        // 清理空的索引项
        {
            let mut by_type = self
                .by_type
                .write()
                .map_err(|e| GraphError::IndexError(format!("获取写锁失败: {}", e)))?;
            by_type.retain(|_, set| !set.is_empty());
        }

        {
            let mut by_name = self
                .by_name
                .write()
                .map_err(|e| GraphError::IndexError(format!("获取写锁失败: {}", e)))?;
            by_name.retain(|_, set| !set.is_empty());
        }

        Ok(())
    }
}

/// 带索引的图包装器
pub struct IndexedGraph<G> {
    inner: G,
    index: MemoryIndex,
}

impl<G> IndexedGraph<G> {
    pub fn new(graph: G) -> Self {
        IndexedGraph {
            inner: graph,
            index: MemoryIndex::new(),
        }
    }

    pub fn index(&mut self, node: &Node) -> Result<()> {
        self.index.insert(node)
    }

    pub fn unindex(&mut self, id: &NodeId) -> Result<()> {
        self.index.remove(id)
    }

    pub fn query_index(&self, query: &str) -> Result<Vec<NodeId>> {
        self.index.query(query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Node;

    #[test]
    fn test_memory_index() {
        let mut index = MemoryIndex::new();
        let node = Node::new(crate::NodeType::File, "test.txt");
        let id = node.id;

        index.insert(&node).unwrap();

        let results = index.query("test.txt").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], id);
    }

    #[test]
    fn test_index_remove() {
        let mut index = MemoryIndex::new();
        let node = Node::new(crate::NodeType::File, "test.txt");
        let id = node.id;

        index.insert(&node).unwrap();
        index.remove(&id).unwrap();

        let results = index.query("test.txt").unwrap();
        assert_eq!(results.len(), 0);
    }
}
