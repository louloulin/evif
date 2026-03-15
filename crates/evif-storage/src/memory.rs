// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

use async_trait::async_trait;
use dashmap::DashMap;
use futures::stream::{self, Stream};
use std::pin::Pin;

use crate::{MemoryTransaction, StorageBackend, StorageOp, StorageResult};
use evif_graph::{Edge, EdgeId, Node, NodeId};

/// 内存存储后端
pub struct MemoryStorage {
    nodes: DashMap<NodeId, Node>,
    edges: DashMap<EdgeId, Edge>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        MemoryStorage {
            nodes: DashMap::new(),
            edges: DashMap::new(),
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn clear(&self) {
        self.nodes.clear();
        self.edges.clear();
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageBackend for MemoryStorage {
    async fn get_node(&self, id: &NodeId) -> StorageResult<Option<Node>> {
        Ok(self.nodes.get(id).map(|entry| entry.clone()))
    }

    async fn put_node(&self, node: &Node) -> StorageResult<()> {
        self.nodes.insert(node.id, node.clone());
        Ok(())
    }

    async fn delete_node(&self, id: &NodeId) -> StorageResult<()> {
        self.nodes.remove(id);
        Ok(())
    }

    async fn get_edge(&self, id: &EdgeId) -> StorageResult<Option<Edge>> {
        Ok(self.edges.get(id).map(|entry| entry.clone()))
    }

    async fn put_edge(&self, edge: &Edge) -> StorageResult<()> {
        self.edges.insert(edge.id, edge.clone());
        Ok(())
    }

    async fn delete_edge(&self, id: &EdgeId) -> StorageResult<()> {
        self.edges.remove(id);
        Ok(())
    }

    async fn batch_write(&self, ops: Vec<StorageOp>) -> StorageResult<()> {
        for op in ops {
            match op {
                StorageOp::InsertNode(node) | StorageOp::UpdateNode(node) => {
                    self.nodes.insert(node.id, node);
                }
                StorageOp::DeleteNode(id) => {
                    self.nodes.remove(&id);
                }
                StorageOp::InsertEdge(edge) | StorageOp::UpdateEdge(edge) => {
                    self.edges.insert(edge.id, edge);
                }
                StorageOp::DeleteEdge(id) => {
                    self.edges.remove(&id);
                }
            }
        }
        Ok(())
    }

    async fn begin_transaction(&self) -> StorageResult<Box<dyn crate::backend::Transaction>> {
        Ok(Box::new(MemoryTransaction::new()))
    }

    async fn scan_nodes(&self) -> StorageResult<Pin<Box<dyn Stream<Item = Node> + Send>>> {
        let nodes: Vec<Node> = self.nodes.iter().map(|entry| entry.clone()).collect();
        Ok(Box::pin(stream::iter(nodes)))
    }

    async fn scan_edges(&self) -> StorageResult<Pin<Box<dyn Stream<Item = Edge> + Send>>> {
        let edges: Vec<Edge> = self.edges.iter().map(|entry| entry.clone()).collect();
        Ok(Box::pin(stream::iter(edges)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use evif_graph::{EdgeType, NodeType};

    #[tokio::test]
    async fn test_memory_storage_basic() {
        let storage = MemoryStorage::new();
        let node = Node::new(NodeType::File, "test.txt");
        let id = node.id;

        storage.put_node(&node).await.unwrap();
        assert_eq!(storage.node_count(), 1);

        let retrieved = storage.get_node(&id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test.txt");
    }

    #[tokio::test]
    async fn test_batch_write() {
        let storage = MemoryStorage::new();
        let node1 = Node::new(NodeType::File, "file1.txt");
        let node2 = Node::new(NodeType::File, "file2.txt");

        let ops = vec![StorageOp::InsertNode(node1), StorageOp::InsertNode(node2)];

        storage.batch_write(ops).await.unwrap();
        assert_eq!(storage.node_count(), 2);
    }
}
