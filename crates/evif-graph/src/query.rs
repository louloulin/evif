// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::NodeId;
use serde::{Deserialize, Serialize};

/// 图查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GraphQuery {
    /// 按 ID 查找节点
    FindNode(NodeId),
    /// 按类型查找节点
    FindNodesByType(String),
    /// 按属性查找节点
    FindNodesByAttr { key: String, value: String },
    /// 查找从源节点出发的所有边
    FindOutgoingEdges(NodeId),
    /// 查找指向目标节点的所有边
    FindIncomingEdges(NodeId),
    /// 查找路径
    FindPath { from: NodeId, to: NodeId },
    /// 查找邻居节点
    FindNeighbors(NodeId),
    /// 复合查询
    Compound(Vec<GraphQuery>),
}

/// 查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryResult {
    Nodes(Vec<NodeId>),
    Edges(Vec<crate::EdgeId>),
    Path(Vec<NodeId>),
    Neighbors(Vec<NodeId>),
}

/// 查询构建器
pub struct QueryBuilder {
    queries: Vec<GraphQuery>,
}

impl QueryBuilder {
    pub fn new() -> Self {
        QueryBuilder {
            queries: Vec::new(),
        }
    }

    pub fn find_node(mut self, id: NodeId) -> Self {
        self.queries.push(GraphQuery::FindNode(id));
        self
    }

    pub fn find_by_type(mut self, node_type: impl Into<String>) -> Self {
        self.queries
            .push(GraphQuery::FindNodesByType(node_type.into()));
        self
    }

    pub fn find_neighbors(mut self, id: NodeId) -> Self {
        self.queries.push(GraphQuery::FindNeighbors(id));
        self
    }

    pub fn find_path(mut self, from: NodeId, to: NodeId) -> Self {
        self.queries.push(GraphQuery::FindPath { from, to });
        self
    }

    pub fn build(self) -> GraphQuery {
        if self.queries.len() == 1 {
            self.queries.into_iter().next().unwrap()
        } else {
            GraphQuery::Compound(self.queries)
        }
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_query_builder() {
        let id = Uuid::new_v4();
        let query = QueryBuilder::new()
            .find_node(id)
            .find_by_type("file")
            .build();

        match query {
            GraphQuery::Compound(queries) => {
                assert_eq!(queries.len(), 2);
            }
            _ => panic!("Expected compound query"),
        }
    }
}
