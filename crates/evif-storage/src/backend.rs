// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

use async_trait::async_trait;
use futures::stream::Stream;
use std::pin::Pin;

use crate::{Edge, EdgeId, Node, NodeId, StorageError, StorageResult};

/// 存储操作
#[derive(Debug, Clone)]
pub enum StorageOp {
    InsertNode(Node),
    UpdateNode(Node),
    DeleteNode(NodeId),
    InsertEdge(Edge),
    UpdateEdge(Edge),
    DeleteEdge(EdgeId),
}

/// 存储后端 trait
#[async_trait]
pub trait StorageBackend: Send + Sync {
    // 节点操作
    async fn get_node(&self, id: &NodeId) -> StorageResult<Option<Node>>;
    async fn put_node(&self, node: &Node) -> StorageResult<()>;
    async fn delete_node(&self, id: &NodeId) -> StorageResult<()>;

    // 边操作
    async fn get_edge(&self, id: &EdgeId) -> StorageResult<Option<Edge>>;
    async fn put_edge(&self, edge: &Edge) -> StorageResult<()>;
    async fn delete_edge(&self, id: &EdgeId) -> StorageResult<()>;

    // 批量操作
    async fn batch_write(&self, ops: Vec<StorageOp>) -> StorageResult<()> {
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

    // 事务（可选实现）
    async fn begin_transaction(&self) -> StorageResult<Box<dyn Transaction>> {
        Err(StorageError::TransactionError(
            "此后端不支持事务".to_string(),
        ))
    }

    // 扫描操作
    async fn scan_nodes(&self) -> StorageResult<Pin<Box<dyn Stream<Item = Node> + Send>>> {
        use futures::stream;
        Ok(Box::pin(stream::empty()))
    }

    async fn scan_edges(&self) -> StorageResult<Pin<Box<dyn Stream<Item = Edge> + Send>>> {
        use futures::stream;
        Ok(Box::pin(stream::empty()))
    }
}

/// 事务 trait
#[async_trait]
pub trait Transaction: Send + Sync {
    async fn commit(self: Box<Self>) -> StorageResult<()>;
    async fn rollback(self: Box<Self>) -> StorageResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_op_creation() {
        use evif_graph::{EdgeType, NodeType};
        let node = Node::new(NodeType::File, "test.txt");
        let op = StorageOp::InsertNode(node);
        // 测试操作创建
        match op {
            StorageOp::InsertNode(_) => {}
            _ => panic!("Expected InsertNode"),
        }
    }
}
