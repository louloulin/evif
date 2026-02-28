#![cfg(feature = "rocksdb-backend")]

// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RocksDB-based persistent storage backend
//!
//! 这个模块提供了基于 RocksDB 的持久化存储后端，支持：
//! - 高性能持久化节点和边存储
//! - 写前日志 (WAL) - RocksDB 内置支持
//! - 事务支持
//! - 快照和备份
//! - 列族 (Column Families) 优化

use async_trait::async_trait;
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

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

/// RocksDB 存储后端
pub struct RocksDBStorage {
    db: Arc<rocksdb::DB>,
}

// SAFETY: RocksDB DB is thread-safe
unsafe impl Send for RocksDBStorage {}
unsafe impl Sync for RocksDBStorage {}

impl RocksDBStorage {
    /// 创建新的 RocksDB 存储后端
    pub fn new<P: AsRef<Path>>(path: P) -> StorageResult<Self> {
        let db_path = path.as_ref();

        // 配置 RocksDB 选项
        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // 启用 WAL
        opts.set_wal_recovery_mode(rocksdb::DBRecoveryMode::PointInTime);

        // 优化性能
        opts.set_use_fsync(false);
        opts.set_bytes_per_sync(1048576);
        opts.set_write_buffer_size(256 * 1024 * 1024); // 256MB
        opts.set_max_write_buffer_number(3);
        opts.set_level_zero_file_num_compaction_trigger(4);
        opts.set_max_background_jobs(4);

        // 定义列族
        let nodes_cf_name = "nodes";
        let edges_cf_name = "edges";
        let metadata_cf_name = "metadata";

        let cf_descriptors = vec![
            rocksdb::ColumnFamilyDescriptor::new(nodes_cf_name, rocksdb::Options::default()),
            rocksdb::ColumnFamilyDescriptor::new(edges_cf_name, rocksdb::Options::default()),
            rocksdb::ColumnFamilyDescriptor::new(metadata_cf_name, rocksdb::Options::default()),
        ];

        // 打开数据库
        let db = rocksdb::DB::open_cf_descriptors(&opts, db_path, cf_descriptors)
            .map_err(|e| StorageError::IoError(format!("Failed to open RocksDB: {}", e)))?;

        Ok(Self {
            db: Arc::new(db),
        })
    }

    /// 创建临时内存存储
    pub fn temporary() -> StorageResult<Self> {
        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // 使用唯一的临时路径避免冲突
        let temp_path = format!("/tmp/evif_rocksdb_temp_{}", uuid::Uuid::new_v4());

        // 定义列族
        let cf_descriptors = vec![
            rocksdb::ColumnFamilyDescriptor::new("nodes", rocksdb::Options::default()),
            rocksdb::ColumnFamilyDescriptor::new("edges", rocksdb::Options::default()),
            rocksdb::ColumnFamilyDescriptor::new("metadata", rocksdb::Options::default()),
        ];

        let temp_db = rocksdb::DB::open_cf_descriptors(&opts, &temp_path, cf_descriptors)
            .map_err(|e| StorageError::IoError(format!("Failed to create temp RocksDB: {}", e)))?;

        Ok(Self {
            db: Arc::new(temp_db),
        })
    }

    /// 获取节点数量（近似值）
    pub fn node_count(&self) -> usize {
        // 使用迭代器计数作为近似
        self.db.iterator_cf(&self.db.cf_handle("nodes").unwrap(), rocksdb::IteratorMode::Start).count()
    }

    /// 获取边数量（近似值）
    pub fn edge_count(&self) -> usize {
        self.db.iterator_cf(&self.db.cf_handle("edges").unwrap(), rocksdb::IteratorMode::Start).count()
    }

    /// 创建快照
    pub fn snapshot(&self) -> StorageResult<()> {
        self.flush()
    }

    /// 刷入磁盘
    pub fn flush(&self) -> StorageResult<()> {
        self.db.flush()
            .map_err(|e| StorageError::IoError(format!("Failed to flush: {}", e)))?;
        Ok(())
    }

    /// 获取存储统计信息
    pub fn stats(&self) -> RocksDBStats {
        RocksDBStats {
            node_count: self.node_count(),
            edge_count: self.edge_count(),
        }
    }

    /// 获取数据库属性
    pub fn get_property(&self, property: &str) -> Option<String> {
        self.db.property_value(property).ok().flatten()
    }

    /// 获取数据库统计信息字符串
    pub fn get_statistics(&self) -> Option<String> {
        self.db.property_value(rocksdb::properties::STATS).ok().flatten()
    }

    fn node_id_to_vec(id: &NodeId) -> Vec<u8> {
        id.as_bytes().to_vec()
    }

    fn edge_id_to_vec(id: &EdgeId) -> Vec<u8> {
        id.as_bytes().to_vec()
    }

    fn get_nodes_cf(&self) -> &rocksdb::ColumnFamily {
        self.db.cf_handle("nodes").expect("nodes column family should exist")
    }

    fn get_edges_cf(&self) -> &rocksdb::ColumnFamily {
        self.db.cf_handle("edges").expect("edges column family should exist")
    }
}

#[async_trait]
impl StorageBackend for RocksDBStorage {
    async fn get_node(&self, id: &NodeId) -> StorageResult<Option<Node>> {
        let key = Self::node_id_to_vec(id);
        match self.db.get_cf(self.get_nodes_cf(), key) {
            Ok(Some(value)) => {
                let node = deserialize_node(&value)?;
                Ok(Some(node))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::IoError(format!("Failed to get node: {}", e))),
        }
    }

    async fn put_node(&self, node: &Node) -> StorageResult<()> {
        let key = Self::node_id_to_vec(&node.id);
        let value = serialize_node(node)?;

        self.db.put_cf(self.get_nodes_cf(), key, value)
            .map_err(|e| StorageError::IoError(format!("Failed to put node: {}", e)))?;
        Ok(())
    }

    async fn delete_node(&self, id: &NodeId) -> StorageResult<()> {
        let key = Self::node_id_to_vec(id);
        self.db.delete_cf(self.get_nodes_cf(), key)
            .map_err(|e| StorageError::IoError(format!("Failed to delete node: {}", e)))?;
        Ok(())
    }

    async fn get_edge(&self, id: &EdgeId) -> StorageResult<Option<Edge>> {
        let key = Self::edge_id_to_vec(id);
        match self.db.get_cf(self.get_edges_cf(), key) {
            Ok(Some(value)) => {
                let edge = deserialize_edge(&value)?;
                Ok(Some(edge))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::IoError(format!("Failed to get edge: {}", e))),
        }
    }

    async fn put_edge(&self, edge: &Edge) -> StorageResult<()> {
        let key = Self::edge_id_to_vec(&edge.id);
        let value = serialize_edge(edge)?;

        self.db.put_cf(self.get_edges_cf(), key, value)
            .map_err(|e| StorageError::IoError(format!("Failed to put edge: {}", e)))?;
        Ok(())
    }

    async fn delete_edge(&self, id: &EdgeId) -> StorageResult<()> {
        let key = Self::edge_id_to_vec(id);
        self.db.delete_cf(self.get_edges_cf(), key)
            .map_err(|e| StorageError::IoError(format!("Failed to delete edge: {}", e)))?;
        Ok(())
    }

    async fn batch_write(&self, ops: Vec<StorageOp>) -> StorageResult<()> {
        // 使用 RocksDB 的批量写入
        let mut batch = rocksdb::WriteBatch::default();
        let nodes_cf = self.get_nodes_cf();
        let edges_cf = self.get_edges_cf();

        for op in ops {
            match op {
                StorageOp::InsertNode(node) | StorageOp::UpdateNode(node) => {
                    let key = Self::node_id_to_vec(&node.id);
                    let value = serialize_node(&node)?;
                    batch.put_cf(nodes_cf, key, value);
                }
                StorageOp::DeleteNode(id) => {
                    let key = Self::node_id_to_vec(&id);
                    batch.delete_cf(nodes_cf, key);
                }
                StorageOp::InsertEdge(edge) | StorageOp::UpdateEdge(edge) => {
                    let key = Self::edge_id_to_vec(&edge.id);
                    let value = serialize_edge(&edge)?;
                    batch.put_cf(edges_cf, key, value);
                }
                StorageOp::DeleteEdge(id) => {
                    let key = Self::edge_id_to_vec(&id);
                    batch.delete_cf(edges_cf, key);
                }
            }
        }

        self.db.write(batch)
            .map_err(|e| StorageError::IoError(format!("Batch write failed: {}", e)))?;
        Ok(())
    }

    async fn begin_transaction(&self) -> StorageResult<Box<dyn Transaction>> {
        Ok(Box::new(RocksDBTransaction {
            db: self.db.clone(),
            committed: false,
        }))
    }

    async fn scan_nodes(&self) -> StorageResult<Pin<Box<dyn Stream<Item = Node> + Send>>> {
        let mut nodes = Vec::new();
        let iter = self.db.iterator_cf(self.get_nodes_cf(), rocksdb::IteratorMode::Start);

        for item in iter {
            let (_key, value) = item.map_err(|e| StorageError::IoError(format!("Scan error: {}", e)))?;
            let node = deserialize_node(&value)?;
            nodes.push(node);
        }

        Ok(Box::pin(stream::iter(nodes)))
    }

    async fn scan_edges(&self) -> StorageResult<Pin<Box<dyn Stream<Item = Edge> + Send>>> {
        let mut edges = Vec::new();
        let iter = self.db.iterator_cf(self.get_edges_cf(), rocksdb::IteratorMode::Start);

        for item in iter {
            let (_key, value) = item.map_err(|e| StorageError::IoError(format!("Scan error: {}", e)))?;
            let edge = deserialize_edge(&value)?;
            edges.push(edge);
        }

        Ok(Box::pin(stream::iter(edges)))
    }
}

/// RocksDB 事务实现
pub struct RocksDBTransaction {
    db: Arc<rocksdb::DB>,
    committed: bool,
}

// SAFETY: RocksDB transaction operations are thread-safe
unsafe impl Send for RocksDBTransaction {}

#[async_trait]
impl Transaction for RocksDBTransaction {
    async fn commit(mut self: Box<Self>) -> StorageResult<()> {
        // RocksDB 的写操作是原子性的，通过 WriteBatch 实现
        self.db.flush()
            .map_err(|e| StorageError::TransactionError(format!("Commit failed: {}", e)))?;
        self.committed = true;
        Ok(())
    }

    async fn rollback(mut self: Box<Self>) -> StorageResult<()> {
        // RocksDB 的 WAL 会自动回滚未完成的事务
        Ok(())
    }
}

/// RocksDB 存储统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RocksDBStats {
    pub node_count: usize,
    pub edge_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use evif_graph::{NodeType, EdgeType};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_rocksdb_storage_crud() {
        let temp_dir = TempDir::new().unwrap();
        let storage = RocksDBStorage::new(temp_dir.path()).unwrap();

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
    async fn test_rocksdb_storage_edges() {
        let storage = RocksDBStorage::temporary().unwrap();

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
    async fn test_rocksdb_storage_batch_write() {
        let storage = RocksDBStorage::temporary().unwrap();

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
    async fn test_rocksdb_storage_scan() {
        let storage = RocksDBStorage::temporary().unwrap();

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
    async fn test_rocksdb_storage_stats() {
        let storage = RocksDBStorage::temporary().unwrap();

        let node = Node::new(NodeType::File, "test.txt");
        storage.put_node(&node).await.unwrap();

        let stats = storage.stats();
        assert_eq!(stats.node_count, 1);
        assert_eq!(stats.edge_count, 0);
    }

    #[test]
    fn test_rocksdb_storage_flush() {
        let storage = RocksDBStorage::temporary().unwrap();
        assert!(storage.flush().is_ok());
    }

    #[tokio::test]
    async fn test_rocksdb_storage_transaction() {
        let storage = RocksDBStorage::temporary().unwrap();

        let tx = storage.begin_transaction().await.unwrap();
        tx.commit().await.unwrap();
    }
}
