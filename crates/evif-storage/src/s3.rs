#![cfg(feature = "s3-backend")]

// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Amazon S3 cloud storage backend
//!
//! 这个模块提供了基于 AWS S3 的云对象存储后端，支持：
//! - 持久化云存储
//! - 高可扩展性
//! - 跨区域复制
//! - 版本控制
//! - 生命周期管理

use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_s3::{Client, primitives::ByteStream};
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::Arc;

use crate::{
    backend::Transaction,
    StorageBackend, StorageResult, StorageError,
    StorageOp, Node, Edge, NodeId, EdgeId,
};

/// S3 存储后端配置
#[derive(Debug, Clone)]
pub struct S3StorageConfig {
    /// AWS 区域
    pub region: String,
    /// S3 存储桶名称
    pub bucket: String,
    /// 对象键前缀
    pub prefix: Option<String>,
    /// 是否使用路径式访问（S3PathStyle）
    pub force_path_style: bool,
}

impl Default for S3StorageConfig {
    fn default() -> Self {
        Self {
            region: "us-east-1".to_string(),
            bucket: "evif-storage".to_string(),
            prefix: None,
            force_path_style: false,
        }
    }
}

impl S3StorageConfig {
    pub fn new(bucket: impl Into<String>) -> Self {
        Self {
            region: "us-east-1".to_string(),
            bucket: bucket.into(),
            prefix: None,
            force_path_style: false,
        }
    }

    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = region.into();
        self
    }

    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    pub fn with_force_path_style(mut self, force_path_style: bool) -> Self {
        self.force_path_style = force_path_style;
        self
    }
}

/// S3 存储后端
pub struct S3Storage {
    client: Arc<Client>,
    config: S3StorageConfig,
}

// SAFETY: AWS S3 Client is thread-safe
unsafe impl Send for S3Storage {}
unsafe impl Sync for S3Storage {}

impl S3Storage {
    /// 创建新的 S3 存储后端
    pub async fn new(config: S3StorageConfig) -> StorageResult<Self> {
        // 加载 AWS 配置
        let config_loader = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(config.region.clone()));

        let aws_config = config_loader.load().await;

        // 创建 S3 客户端
        let client = Client::new(&aws_config);

        Ok(Self {
            client: Arc::new(client),
            config,
        })
    }

    /// 从环境变量创建 S3 存储后端
    pub async fn from_env(bucket: impl Into<String>) -> StorageResult<Self> {
        Self::new(S3StorageConfig::new(bucket)).await
    }

    /// 创建使用最小配置的 S3 存储后端（用于测试）
    #[cfg(test)]
    pub async fn test_config() -> StorageResult<Self> {
        let config = S3StorageConfig::new("evif-test")
            .with_region("us-east-1")
            .with_force_path_style(true);

        Self::new(config).await
    }

    /// 生成节点对象键
    fn node_key(&self, id: &NodeId) -> String {
        let id_hex = hex::encode(id.as_bytes());
        match &self.config.prefix {
            Some(prefix) => format!("{}/nodes/{}", prefix, id_hex),
            None => format!("nodes/{}", id_hex),
        }
    }

    /// 生成边对象键
    fn edge_key(&self, id: &EdgeId) -> String {
        let id_hex = hex::encode(id.as_bytes());
        match &self.config.prefix {
            Some(prefix) => format!("{}/edges/{}", prefix, id_hex),
            None => format!("edges/{}", id_hex),
        }
    }

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

    /// 检查对象是否存在
    pub async fn object_exists(&self, key: &str) -> StorageResult<bool> {
        match self
            .client
            .head_object()
            .bucket(&self.config.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                // 检查是否是 NotFound 错误
                if e.to_string().contains("404") || e.to_string().contains("NotFound") {
                    Ok(false)
                } else {
                    Err(StorageError::IoError(format!("Failed to check object: {}", e)))
                }
            }
        }
    }
}

#[async_trait]
impl StorageBackend for S3Storage {
    async fn get_node(&self, id: &NodeId) -> StorageResult<Option<Node>> {
        let key = self.node_key(id);

        match self
            .client
            .get_object()
            .bucket(&self.config.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(output) => {
                let bytes = output.body
                    .collect()
                    .await
                    .map_err(|e| StorageError::IoError(format!("Failed to read body: {}", e)))?
                    .to_vec();
                let node = Self::deserialize_node(&bytes)?;
                Ok(Some(node))
            }
            Err(e) => {
                // 检查是否是 NotFound 错误
                if e.to_string().contains("404") || e.to_string().contains("NoSuchKey") {
                    Ok(None)
                } else {
                    Err(StorageError::IoError(format!("Failed to get node: {}", e)))
                }
            }
        }
    }

    async fn put_node(&self, node: &Node) -> StorageResult<()> {
        let key = self.node_key(&node.id);
        let data = Self::serialize_node(node)?;

        self.client
            .put_object()
            .bucket(&self.config.bucket)
            .key(&key)
            .body(ByteStream::new(data.into()))
            .send()
            .await
            .map_err(|e| StorageError::IoError(format!("Failed to put node: {}", e)))?;

        Ok(())
    }

    async fn delete_node(&self, id: &NodeId) -> StorageResult<()> {
        let key = self.node_key(id);

        self.client
            .delete_object()
            .bucket(&self.config.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| StorageError::IoError(format!("Failed to delete node: {}", e)))?;

        Ok(())
    }

    async fn get_edge(&self, id: &EdgeId) -> StorageResult<Option<Edge>> {
        let key = self.edge_key(id);

        match self
            .client
            .get_object()
            .bucket(&self.config.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(output) => {
                let bytes = output.body
                    .collect()
                    .await
                    .map_err(|e| StorageError::IoError(format!("Failed to read body: {}", e)))?
                    .to_vec();
                let edge = Self::deserialize_edge(&bytes)?;
                Ok(Some(edge))
            }
            Err(e) => {
                if e.to_string().contains("404") || e.to_string().contains("NoSuchKey") {
                    Ok(None)
                } else {
                    Err(StorageError::IoError(format!("Failed to get edge: {}", e)))
                }
            }
        }
    }

    async fn put_edge(&self, edge: &Edge) -> StorageResult<()> {
        let key = self.edge_key(&edge.id);
        let data = Self::serialize_edge(edge)?;

        self.client
            .put_object()
            .bucket(&self.config.bucket)
            .key(&key)
            .body(ByteStream::new(data.into()))
            .send()
            .await
            .map_err(|e| StorageError::IoError(format!("Failed to put edge: {}", e)))?;

        Ok(())
    }

    async fn delete_edge(&self, id: &EdgeId) -> StorageResult<()> {
        let key = self.edge_key(id);

        self.client
            .delete_object()
            .bucket(&self.config.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| StorageError::IoError(format!("Failed to delete edge: {}", e)))?;

        Ok(())
    }

    async fn batch_write(&self, ops: Vec<StorageOp>) -> StorageResult<()> {
        // S3 不支持原生批量写入，使用并发写入
        let mut tasks = Vec::new();

        for op in ops {
            let client = Arc::clone(&self.client);
            let bucket = self.config.bucket.clone();
            let config = self.config.clone();

            let task = async move {
                match op {
                    StorageOp::InsertNode(node) | StorageOp::UpdateNode(node) => {
                        let id_hex = hex::encode(node.id.as_bytes());
                        let key = match &config.prefix {
                            Some(prefix) => format!("{}/nodes/{}", prefix, id_hex),
                            None => format!("nodes/{}", id_hex),
                        };
                        let data = Self::serialize_node(&node)?;

                        client
                            .put_object()
                            .bucket(&bucket)
                            .key(&key)
                            .body(ByteStream::new(data.into()))
                            .send()
                            .await
                            .map_err(|e| StorageError::IoError(format!("Batch put node failed: {}", e)))?;
                    }
                    StorageOp::DeleteNode(id) => {
                        let id_hex = hex::encode(id.as_bytes());
                        let key = match &config.prefix {
                            Some(prefix) => format!("{}/nodes/{}", prefix, id_hex),
                            None => format!("nodes/{}", id_hex),
                        };

                        client
                            .delete_object()
                            .bucket(&bucket)
                            .key(&key)
                            .send()
                            .await
                            .map_err(|e| StorageError::IoError(format!("Batch delete node failed: {}", e)))?;
                    }
                    StorageOp::InsertEdge(edge) | StorageOp::UpdateEdge(edge) => {
                        let id_hex = hex::encode(edge.id.as_bytes());
                        let key = match &config.prefix {
                            Some(prefix) => format!("{}/edges/{}", prefix, id_hex),
                            None => format!("edges/{}", id_hex),
                        };
                        let data = Self::serialize_edge(&edge)?;

                        client
                            .put_object()
                            .bucket(&bucket)
                            .key(&key)
                            .body(ByteStream::new(data.into()))
                            .send()
                            .await
                            .map_err(|e| StorageError::IoError(format!("Batch put edge failed: {}", e)))?;
                    }
                    StorageOp::DeleteEdge(id) => {
                        let id_hex = hex::encode(id.as_bytes());
                        let key = match &config.prefix {
                            Some(prefix) => format!("{}/edges/{}", prefix, id_hex),
                            None => format!("edges/{}", id_hex),
                        };

                        client
                            .delete_object()
                            .bucket(&bucket)
                            .key(&key)
                            .send()
                            .await
                            .map_err(|e| StorageError::IoError(format!("Batch delete edge failed: {}", e)))?;
                    }
                }
                Ok::<(), StorageError>(())
            };

            tasks.push(task);
        }

        // 并发执行所有操作
        let results = futures::future::join_all(tasks).await;
        for result in results {
            result?;
        }

        Ok(())
    }

    async fn begin_transaction(&self) -> StorageResult<Box<dyn Transaction>> {
        Ok(Box::new(S3Transaction {
            committed: false,
        }))
    }

    async fn scan_nodes(&self) -> StorageResult<Pin<Box<dyn Stream<Item = Node> + Send>>> {
        let prefix = match &self.config.prefix {
            Some(p) => format!("{}/nodes/", p),
            None => "nodes/".to_string(),
        };

        let mut nodes = Vec::new();

        // 列出所有节点对象
        let mut continuation_token = None;
        loop {
            let mut list_request = self
                .client
                .list_objects_v2()
                .bucket(&self.config.bucket)
                .prefix(&prefix);

            if let Some(token) = &continuation_token {
                list_request = list_request.continuation_token(token);
            }

            match list_request.send().await {
                Ok(output) => {
                    if let Some(objects) = output.contents {
                        for object in objects {
                            if let Some(key) = object.key {
                                match self
                                    .client
                                    .get_object()
                                    .bucket(&self.config.bucket)
                                    .key(&key)
                                    .send()
                                    .await
                                {
                                    Ok(obj_output) => {
                                        let bytes = obj_output.body
                                            .collect()
                                            .await
                                            .map_err(|e| {
                                                StorageError::IoError(format!(
                                                    "Failed to read body: {}",
                                                    e
                                                ))
                                            })?
                                            .to_vec();
                                        let node = Self::deserialize_node(&bytes)?;
                                        nodes.push(node);
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to get object {}: {}", key, e);
                                    }
                                }
                            }
                        }
                    }

                    continuation_token = output.next_continuation_token;
                    if continuation_token.is_none() {
                        break;
                    }
                }
                Err(e) => {
                    return Err(StorageError::IoError(format!("Failed to list objects: {}", e)));
                }
            }
        }

        Ok(Box::pin(stream::iter(nodes)))
    }

    async fn scan_edges(&self) -> StorageResult<Pin<Box<dyn Stream<Item = Edge> + Send>>> {
        let prefix = match &self.config.prefix {
            Some(p) => format!("{}/edges/", p),
            None => "edges/".to_string(),
        };

        let mut edges = Vec::new();

        // 列出所有边对象
        let mut continuation_token = None;
        loop {
            let mut list_request = self
                .client
                .list_objects_v2()
                .bucket(&self.config.bucket)
                .prefix(&prefix);

            if let Some(token) = &continuation_token {
                list_request = list_request.continuation_token(token);
            }

            match list_request.send().await {
                Ok(output) => {
                    if let Some(objects) = output.contents {
                        for object in objects {
                            if let Some(key) = object.key {
                                match self
                                    .client
                                    .get_object()
                                    .bucket(&self.config.bucket)
                                    .key(&key)
                                    .send()
                                    .await
                                {
                                    Ok(obj_output) => {
                                        let bytes = obj_output.body
                                            .collect()
                                            .await
                                            .map_err(|e| {
                                                StorageError::IoError(format!(
                                                    "Failed to read body: {}",
                                                    e
                                                ))
                                            })?
                                            .to_vec();
                                        let edge = Self::deserialize_edge(&bytes)?;
                                        edges.push(edge);
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to get object {}: {}", key, e);
                                    }
                                }
                            }
                        }
                    }

                    continuation_token = output.next_continuation_token;
                    if continuation_token.is_none() {
                        break;
                    }
                }
                Err(e) => {
                    return Err(StorageError::IoError(format!("Failed to list objects: {}", e)));
                }
            }
        }

        Ok(Box::pin(stream::iter(edges)))
    }
}

/// S3 事务实现
pub struct S3Transaction {
    committed: bool,
}

#[async_trait]
impl Transaction for S3Transaction {
    async fn commit(mut self: Box<Self>) -> StorageResult<()> {
        // S3 的写操作是原子性的（每个对象操作）
        self.committed = true;
        Ok(())
    }

    async fn rollback(mut self: Box<Self>) -> StorageResult<()> {
        // S3 不支持回滚，需要应用层实现补偿逻辑
        tracing::warn!("S3 transactions do not support rollback");
        Ok(())
    }
}

/// S3 存储统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Stats {
    pub node_count: usize,
    pub edge_count: usize,
    pub bucket_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use evif_graph::{NodeType, EdgeType};

    // 注意：这些测试需要真实的 AWS 凭证和 S3 存储桶
    // 或者使用 LocalStack 等本地模拟服务

    #[tokio::test]
    #[ignore] // 需要真实 AWS 环境或 LocalStack
    async fn test_s3_storage_crud() {
        let storage = S3Storage::test_config().await.unwrap();

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
    #[ignore]
    async fn test_s3_storage_edges() {
        let storage = S3Storage::test_config().await.unwrap();

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
    #[ignore]
    async fn test_s3_storage_batch_write() {
        let storage = S3Storage::test_config().await.unwrap();

        let node1 = Node::new(NodeType::File, "file1.txt");
        let node2 = Node::new(NodeType::File, "file2.txt");
        let node3 = Node::new(NodeType::Directory, "dir");

        let ops = vec![
            StorageOp::InsertNode(node1),
            StorageOp::InsertNode(node2),
            StorageOp::InsertNode(node3),
        ];

        storage.batch_write(ops).await.unwrap();

        // 验证节点已创建
        let nodes = storage.scan_nodes().await.unwrap();
        let node_count = futures::stream::iter(nodes).count().await;
        assert!(node_count >= 3);
    }

    #[tokio::test]
    #[ignore]
    async fn test_s3_storage_transaction() {
        let storage = S3Storage::test_config().await.unwrap();

        let tx = storage.begin_transaction().await.unwrap();
        tx.commit().await.unwrap();
    }
}
