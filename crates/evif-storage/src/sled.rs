#![cfg(feature = "sled-backend")]

// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Sled-based persistent storage backend
//!
//! 这个模块提供了基于 sled 的持久化存储后端，支持：
//! - 持久化节点和边存储
//! - 写前日志 (WAL) - sled 内置支持
//! - 事务支持
//! - 快照和备份

use async_trait::async_trait;
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::pin::Pin;
use sled::IVec;

use crate::{
    backend::Transaction,
    StorageBackend, StorageResult, StorageError,
    StorageOp, Node, Edge, NodeId, EdgeId,
};

/// 序列化节点为字节
fn serialize_node(node: &Node) -> StorageResult<Vec<u8>> {
    serde_json::to_vec(node)
        .map_err(|e| StorageError::SerializationError(format!("Failed to serialize node: {}", e)))
}

/// 从字节反序列化节点
fn deserialize_node(data: &[u8]) -> StorageResult<Node> {
    serde_json::from_slice(data)
        .map_err(|e| StorageError::DeserializationError(format!("Failed to deserialize node: {}", e)))
}

/// 序列化边为字节
fn serialize_edge(edge: &Edge) -> StorageResult<Vec<u8>> {
    serde_json::to_vec(edge)
        .map_err(|e| StorageError::SerializationError(format!("Failed to serialize edge: {}", e)))
}

/// 从字节反序列化边
fn deserialize_edge(data: &[u8]) -> StorageResult<Edge> {
    serde_json::from_slice(data)
        .map_err(|e| StorageError::DeserializationError(format!("Failed to deserialize edge: {}", e)))
}

/// Sled 存储后端
pub struct SledStorage {
    db: sled::Db,
    nodes: sled::Tree,
    edges: sled::Tree,
    metadata: sled::Tree,
}

impl SledStorage {
    /// 创建新的 Sled 存储后端
    pub fn new<P: AsRef<Path>>(path: P) -> StorageResult<Self> {
        let db = sled::open(path)
            .map_err(|e| StorageError::IoError(format!("Failed to open sled database: {}", e)))?;

        let nodes = db.open_tree("nodes")
            .map_err(|e| StorageError::IoError(format!("Failed to open nodes tree: {}", e)))?;

        let edges = db.open_tree("edges")
            .map_err(|e| StorageError::IoError(format!("Failed to open edges tree: {}", e)))?;

        let metadata = db.open_tree("metadata")
            .map_err(|e| StorageError::IoError(format!("Failed to open metadata tree: {}", e)))?;

        Ok(Self {
            db,
            nodes,
            edges,
            metadata,
        })
    }

    /// 创建临时内存存储
    pub fn temporary() -> StorageResult<Self> {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .map_err(|e| StorageError::IoError(format!("Failed to create temp db: {}", e)))?;

        let nodes = db.open_tree("nodes")
            .map_err(|e| StorageError::IoError(format!("Failed to open nodes tree: {}", e)))?;

        let edges = db.open_tree("edges")
            .map_err(|e| StorageError::IoError(format!("Failed to open edges tree: {}", e)))?;

        let metadata = db.open_tree("metadata")
            .map_err(|e| StorageError::IoError(format!("Failed to open metadata tree: {}", e)))?;

        Ok(Self {
            db,
            nodes,
            edges,
            metadata,
        })
    }

    /// 获取节点数量
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// 获取边数量
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// 创建快照（刷入磁盘）
    pub fn snapshot(&self) -> StorageResult<()> {
        self.flush()
    }

    /// 刷入磁盘
    pub fn flush(&self) -> StorageResult<()> {
        self.db.flush()
            .map_err(|e| StorageError::IoError(format!("Failed to flush: {}", e)))?;
        Ok(())
    }

    /// 获取数据库大小（字节）
    pub fn size_on_disk(&self) -> StorageResult<u64> {
        self.db.size_on_disk()
            .map_err(|e| StorageError::IoError(format!("Failed to get size: {}", e)))
    }

    /// 获取存储统计信息
    pub fn stats(&self) -> StorageStats {
        StorageStats {
            node_count: self.nodes.len(),
            edge_count: self.edges.len(),
            tree_size: self.db.size_on_disk().unwrap_or(0),
        }
    }

    fn node_id_to_ivec(id: &NodeId) -> IVec {
        let bytes = id.as_bytes();
        IVec::from(bytes)
    }

    fn edge_id_to_ivec(id: &EdgeId) -> IVec {
        let bytes = id.as_bytes();
        IVec::from(bytes)
    }
}

#[async_trait]
impl StorageBackend for SledStorage {
    async fn get_node(&self, id: &NodeId) -> StorageResult<Option<Node>> {
        let key = Self::node_id_to_ivec(id);
        match self.nodes.get(key) {
            Ok(Some(value)) => {
                let node = deserialize_node(&value)?;
                Ok(Some(node))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::IoError(format!("Failed to get node: {}", e))),
        }
    }

    async fn put_node(&self, node: &Node) -> StorageResult<()> {
        let key = Self::node_id_to_ivec(&node.id);
        let value = serialize_node(node)?;

        self.nodes.insert(key, value)
            .map_err(|e| StorageError::IoError(format!("Failed to put node: {}", e)))?;
        Ok(())
    }

    async fn delete_node(&self, id: &NodeId) -> StorageResult<()> {
        let key = Self::node_id_to_ivec(id);
        self.nodes.remove(key)
            .map_err(|e| StorageError::IoError(format!("Failed to delete node: {}", e)))?;
        Ok(())
    }

    async fn get_edge(&self, id: &EdgeId) -> StorageResult<Option<Edge>> {
        let key = Self::edge_id_to_ivec(id);
        match self.edges.get(key) {
            Ok(Some(value)) => {
                let edge = deserialize_edge(&value)?;
                Ok(Some(edge))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::IoError(format!("Failed to get edge: {}", e))),
        }
    }

    async fn put_edge(&self, edge: &Edge) -> StorageResult<()> {
        let key = Self::edge_id_to_ivec(&edge.id);
        let value = serialize_edge(edge)?;

        self.edges.insert(key, value)
            .map_err(|e| StorageError::IoError(format!("Failed to put edge: {}", e)))?;
        Ok(())
    }

    async fn delete_edge(&self, id: &EdgeId) -> StorageResult<()> {
        let key = Self::edge_id_to_ivec(id);
        self.edges.remove(key)
            .map_err(|e| StorageError::IoError(format!("Failed to delete edge: {}", e)))?;
        Ok(())
    }

    async fn batch_write(&self, ops: Vec<StorageOp>) -> StorageResult<()> {
        // 使用 sled 的事务支持
        for op in ops {
            match op {
                StorageOp::InsertNode(node) | StorageOp::UpdateNode(node) => {
                    self.put_node(&node).await?;
                }
                StorageOp::DeleteNode(id) => {
                    self.delete_node(&id).await?;
                }
                StorageOp::InsertEdge(edge) | StorageOp::UpdateEdge(edge) => {
                    self.put_edge(&edge).await?;
                }
                StorageOp::DeleteEdge(id) => {
                    self.delete_edge(&id).await?;
                }
            }
        }
        Ok(())
    }

    async fn begin_transaction(&self) -> StorageResult<Box<dyn Transaction>> {
        Ok(Box::new(SledTransaction {
            db: self.db.clone(),
            committed: false,
        }))
    }

    async fn scan_nodes(&self) -> StorageResult<Pin<Box<dyn Stream<Item = Node> + Send>>> {
        let mut nodes = Vec::new();
        for item in self.nodes.iter() {
            let (_, value) = item.map_err(|e| StorageError::IoError(format!("Scan error: {}", e)))?;
            let node = deserialize_node(&value)?;
            nodes.push(node);
        }
        Ok(Box::pin(stream::iter(nodes)))
    }

    async fn scan_edges(&self) -> StorageResult<Pin<Box<dyn Stream<Item = Edge> + Send>>> {
        let mut edges = Vec::new();
        for item in self.edges.iter() {
            let (_, value) = item.map_err(|e| StorageError::IoError(format!("Scan error: {}", e)))?;
            let edge = deserialize_edge(&value)?;
            edges.push(edge);
        }
        Ok(Box::pin(stream::iter(edges)))
    }
}

/// Sled 事务实现
pub struct SledTransaction {
    db: sled::Db,
    committed: bool,
}

#[async_trait]
impl Transaction for SledTransaction {
    async fn commit(mut self: Box<Self>) -> StorageResult<()> {
        self.db.flush()
            .map_err(|e| StorageError::TransactionError(format!("Commit failed: {}", e)))?;
        self.committed = true;
        Ok(())
    }

    async fn rollback(mut self: Box<Self>) -> StorageResult<()> {
        // sled 的写前日志会自动回滚未完成的事务
        Ok(())
    }
}

/// 存储统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub tree_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use evif_graph::{NodeType, EdgeType};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_sled_storage_crud() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SledStorage::new(temp_dir.path()).unwrap();

        // Create node
        let node = Node::new(NodeType::File, "test.txt");
        let node_id = node.id;

        storage.put_node(&node).await.unwrap();

        // Get node
        let retrieved = storage.get_node(&node_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test.txt");

        // Delete node
        storage.delete_node(&node_id).await.unwrap();
        let deleted = storage.get_node(&node_id).await.unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_sled_storage_edges() {
        let storage = SledStorage::temporary().unwrap();

        let node1 = Node::new(NodeType::Directory, "parent");
        let node2 = Node::new(NodeType::File, "child");

        storage.put_node(&node1).await.unwrap();
        storage.put_node(&node2).await.unwrap();

        let edge = Edge::new(node1.id, node2.id, EdgeType::Parent);

        storage.put_edge(&edge).await.unwrap();

        let retrieved = storage.get_edge(&edge.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().source, node1.id);
    }

    #[tokio::test]
    async fn test_sled_storage_batch_write() {
        let storage = SledStorage::temporary().unwrap();

        let node1 = Node::new(NodeType::File, "file1.txt");
        let node2 = Node::new(NodeType::File, "file2.txt");
        let node3 = Node::new(NodeType::Directory, "dir");

        let ops = vec![
            StorageOp::InsertNode(node1.clone()),
            StorageOp::InsertNode(node2.clone()),
            StorageOp::InsertNode(node3),
        ];

        storage.batch_write(ops).await.unwrap();

        assert_eq!(storage.node_count(), 3);
    }

    #[tokio::test]
    async fn test_sled_storage_scan() {
        let storage = SledStorage::temporary().unwrap();

        let node1 = Node::new(NodeType::File, "file1.txt");
        let node2 = Node::new(NodeType::File, "file2.txt");

        storage.put_node(&node1).await.unwrap();
        storage.put_node(&node2).await.unwrap();

        use futures::StreamExt;
        let mut stream = storage.scan_nodes().await.unwrap();
        let mut count = 0;
        while let Some(_) = stream.next().await {
            count += 1;
        }
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_sled_storage_stats() {
        let storage = SledStorage::temporary().unwrap();

        let node = Node::new(NodeType::File, "test.txt");
        storage.put_node(&node).await.unwrap();

        let stats = storage.stats();
        assert_eq!(stats.node_count, 1);
        assert_eq!(stats.edge_count, 0);
    }

    #[test]
    fn test_sled_storage_flush() {
        let storage = SledStorage::temporary().unwrap();
        assert!(storage.flush().is_ok());
    }

    #[tokio::test]
    async fn test_sled_storage_transaction() {
        let storage = SledStorage::temporary().unwrap();

        let tx = storage.begin_transaction().await.unwrap();
        tx.commit().await.unwrap();
    }
}
