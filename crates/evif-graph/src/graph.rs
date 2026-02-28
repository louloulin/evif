// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{Node, Edge, NodeId, EdgeId, Result, GraphError};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use std::collections::{HashSet, VecDeque};
use uuid::Uuid;

/// 图配置
#[derive(Debug, Clone)]
pub struct GraphConfig {
    pub max_nodes: usize,
    pub max_edges: usize,
    pub enable_index: bool,
}

impl Default for GraphConfig {
    fn default() -> Self {
        GraphConfig {
            max_nodes: 1_000_000,
            max_edges: 10_000_000,
            enable_index: true,
        }
    }
}

/// 主图结构
pub struct Graph {
    nodes: DashMap<NodeId, Node>,
    edges: DashMap<EdgeId, Edge>,
    adjacency_out: DashMap<NodeId, Vec<EdgeId>>,
    adjacency_in: DashMap<NodeId, Vec<EdgeId>>,
    config: GraphConfig,
}

impl Graph {
    pub fn new() -> Self {
        Self::with_config(GraphConfig::default())
    }

    pub fn with_config(config: GraphConfig) -> Self {
        Graph {
            nodes: DashMap::new(),
            edges: DashMap::new(),
            adjacency_out: DashMap::new(),
            adjacency_in: DashMap::new(),
            config,
        }
    }

    pub fn add_node(&self, node: Node) -> Result<NodeId> {
        if self.nodes.len() >= self.config.max_nodes {
            return Err(GraphError::InvalidOperation("已达到最大节点数限制".to_string()));
        }

        let id = node.id;
        if self.nodes.contains_key(&id) {
            return Err(GraphError::NodeAlreadyExists(id));
        }

        self.nodes.insert(id, node);
        self.adjacency_out.insert(id, Vec::new());
        self.adjacency_in.insert(id, Vec::new());

        Ok(id)
    }

    pub fn get_node(&self, id: &NodeId) -> Result<Node> {
        self.nodes
            .get(id)
            .map(|entry| entry.clone())
            .ok_or_else(|| GraphError::NodeNotFound(*id))
    }

    pub fn remove_node(&self, id: &NodeId) -> Result<()> {
        if !self.nodes.contains_key(id) {
            return Err(GraphError::NodeNotFound(*id));
        }

        let outgoing_edges: Vec<EdgeId> = self
            .adjacency_out
            .get(id)
            .map(|edges| edges.clone())
            .unwrap_or_default();

        let incoming_edges: Vec<EdgeId> = self
            .adjacency_in
            .get(id)
            .map(|edges| edges.clone())
            .unwrap_or_default();

        for edge_id in outgoing_edges.into_iter().chain(incoming_edges) {
            let _ = self.remove_edge(&edge_id);
        }

        self.nodes.remove(id);
        self.adjacency_out.remove(id);
        self.adjacency_in.remove(id);

        Ok(())
    }

    pub fn add_edge(&self, edge: Edge) -> Result<EdgeId> {
        if self.edges.len() >= self.config.max_edges {
            return Err(GraphError::InvalidOperation("已达到最大边数限制".to_string()));
        }

        let id = edge.id;
        let source = edge.source;
        let target = edge.target;

        if !self.nodes.contains_key(&source) {
            return Err(GraphError::NodeNotFound(source));
        }
        if !self.nodes.contains_key(&target) {
            return Err(GraphError::NodeNotFound(target));
        }

        if self.edges.contains_key(&id) {
            return Err(GraphError::EdgeAlreadyExists(id));
        }

        self.edges.insert(id, edge);

        self.adjacency_out
            .entry(source)
            .or_insert_with(Vec::new)
            .push(id);

        self.adjacency_in
            .entry(target)
            .or_insert_with(Vec::new)
            .push(id);

        Ok(id)
    }

    pub fn get_edge(&self, id: &EdgeId) -> Result<Edge> {
        self.edges
            .get(id)
            .map(|entry| entry.clone())
            .ok_or_else(|| GraphError::EdgeNotFound(*id))
    }

    pub fn remove_edge(&self, id: &EdgeId) -> Result<()> {
        let edge = self.get_edge(id)?;
        let source = edge.source;
        let target = edge.target;

        if let Some(mut outgoing) = self.adjacency_out.get_mut(&source) {
            outgoing.retain(|e| e != id);
        }

        if let Some(mut incoming) = self.adjacency_in.get_mut(&target) {
            incoming.retain(|e| e != id);
        }

        self.edges.remove(id);
        Ok(())
    }

    pub fn outgoing_edges(&self, id: &NodeId) -> Result<Vec<Edge>> {
        if !self.nodes.contains_key(id) {
            return Err(GraphError::NodeNotFound(*id));
        }

        let edge_ids = self
            .adjacency_out
            .get(id)
            .map(|edges| edges.clone())
            .unwrap_or_default();

        let mut edges = Vec::new();
        for edge_id in edge_ids {
            if let Some(edge) = self.edges.get(&edge_id) {
                edges.push(edge.clone());
            }
        }

        Ok(edges)
    }

    pub fn incoming_edges(&self, id: &NodeId) -> Result<Vec<Edge>> {
        if !self.nodes.contains_key(id) {
            return Err(GraphError::NodeNotFound(*id));
        }

        let edge_ids = self
            .adjacency_in
            .get(id)
            .map(|edges| edges.clone())
            .unwrap_or_default();

        let mut edges = Vec::new();
        for edge_id in edge_ids {
            if let Some(edge) = self.edges.get(&edge_id) {
                edges.push(edge.clone());
            }
        }

        Ok(edges)
    }

    pub fn neighbors(&self, id: &NodeId) -> Result<Vec<NodeId>> {
        let edges = self.outgoing_edges(id)?;
        Ok(edges.into_iter().map(|e| e.target).collect())
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn all_nodes(&self) -> Vec<Node> {
        self.nodes.iter().map(|entry| entry.clone()).collect()
    }

    pub fn all_edges(&self) -> Vec<Edge> {
        self.edges.iter().map(|entry| entry.clone()).collect()
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

/// 图引擎 - 提供高级图操作
pub struct GraphEngine {
    graph: Arc<Graph>,
}

impl GraphEngine {
    pub fn new() -> Self {
        GraphEngine {
            graph: Arc::new(Graph::new()),
        }
    }

    pub fn with_config(config: GraphConfig) -> Self {
        GraphEngine {
            graph: Arc::new(Graph::with_config(config)),
        }
    }

    pub fn graph(&self) -> Arc<Graph> {
        Arc::clone(&self.graph)
    }

    /// 检测环
    pub fn detect_cycle(&self) -> bool {
        let visited = Arc::new(RwLock::new(HashSet::new()));
        let rec_stack = Arc::new(RwLock::new(HashSet::new()));

        for node_entry in self.graph.nodes.iter() {
            let node_id = *node_entry.key();
            {
                let visited_read = visited.read();
                if !visited_read.contains(&node_id) {
                    drop(visited_read);
                    if self.has_cycle_util(node_id, Arc::clone(&visited), Arc::clone(&rec_stack)) {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn has_cycle_util(
        &self,
        node_id: NodeId,
        visited: Arc<RwLock<HashSet<NodeId>>>,
        rec_stack: Arc<RwLock<HashSet<NodeId>>>,
    ) -> bool {
        {
            let mut visited_write = visited.write();
            let mut rec_write = rec_stack.write();
            visited_write.insert(node_id);
            rec_write.insert(node_id);
        }

        if let Ok(neighbors) = self.graph.neighbors(&node_id) {
            for neighbor_id in neighbors {
                {
                    let rec_read = rec_stack.read();
                    if rec_read.contains(&neighbor_id) {
                        return true;
                    }
                }

                {
                    let visited_read = visited.read();
                    if !visited_read.contains(&neighbor_id) {
                        drop(visited_read);
                        if self.has_cycle_util(neighbor_id, Arc::clone(&visited), Arc::clone(&rec_stack)) {
                            return true;
                        }
                    }
                }
            }
        }

        {
            let mut rec_write = rec_stack.write();
            rec_write.remove(&node_id);
        }

        false
    }

    /// BFS 遍历
    pub fn bfs(&self, start: NodeId) -> Result<Vec<NodeId>> {
        if !self.graph.nodes.contains_key(&start) {
            return Err(GraphError::NodeNotFound(start));
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        queue.push_back(start);
        visited.insert(start);

        while let Some(node_id) = queue.pop_front() {
            result.push(node_id);

            if let Ok(neighbors) = self.graph.neighbors(&node_id) {
                for neighbor_id in neighbors {
                    if !visited.contains(&neighbor_id) {
                        visited.insert(neighbor_id);
                        queue.push_back(neighbor_id);
                    }
                }
            }
        }

        Ok(result)
    }

    /// DFS 遍历
    pub fn dfs(&self, start: NodeId) -> Result<Vec<NodeId>> {
        if !self.graph.nodes.contains_key(&start) {
            return Err(GraphError::NodeNotFound(start));
        }

        let mut visited = HashSet::new();
        let mut result = Vec::new();
        self.dfs_util(start, &mut visited, &mut result)?;

        Ok(result)
    }

    fn dfs_util(
        &self,
        node_id: NodeId,
        visited: &mut HashSet<NodeId>,
        result: &mut Vec<NodeId>,
    ) -> Result<()> {
        visited.insert(node_id);
        result.push(node_id);

        if let Ok(neighbors) = self.graph.neighbors(&node_id) {
            for neighbor_id in neighbors {
                if !visited.contains(&neighbor_id) {
                    self.dfs_util(neighbor_id, visited, result)?;
                }
            }
        }

        Ok(())
    }
}

impl Default for GraphEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NodeType, Node, EdgeType};

    #[test]
    fn test_graph_creation() {
        let graph = Graph::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let graph = Graph::new();
        let node = Node::new(NodeType::File, "test.txt");
        let id = graph.add_node(node).unwrap();

        let retrieved = graph.get_node(&id).unwrap();
        assert_eq!(retrieved.name, "test.txt");
    }

    #[test]
    fn test_add_edge() {
        let graph = Graph::new();
        let node1 = Node::new(NodeType::Directory, "parent");
        let node2 = Node::new(NodeType::File, "child");

        let id1 = graph.add_node(node1).unwrap();
        let id2 = graph.add_node(node2).unwrap();

        let edge = Edge::new(id1, id2, EdgeType::Parent);
        let edge_id = graph.add_edge(edge).unwrap();

        let retrieved_edge = graph.get_edge(&edge_id).unwrap();
        assert_eq!(retrieved_edge.source, id1);
        assert_eq!(retrieved_edge.target, id2);
    }

    #[test]
    fn test_neighbors() {
        let graph = Graph::new();
        let parent = Node::new(NodeType::Directory, "parent");
        let child1 = Node::new(NodeType::File, "child1");
        let child2 = Node::new(NodeType::File, "child2");

        let parent_id = graph.add_node(parent).unwrap();
        let child1_id = graph.add_node(child1).unwrap();
        let child2_id = graph.add_node(child2).unwrap();

        let edge1 = Edge::new(parent_id, child1_id, EdgeType::Parent);
        let edge2 = Edge::new(parent_id, child2_id, EdgeType::Parent);

        graph.add_edge(edge1).unwrap();
        graph.add_edge(edge2).unwrap();

        let neighbors = graph.neighbors(&parent_id).unwrap();
        assert_eq!(neighbors.len(), 2);
    }

    #[test]
    fn test_bfs_traversal() {
        let engine = GraphEngine::new();
        let graph = engine.graph();

        let root = Node::new(NodeType::Directory, "root");
        let child1 = Node::new(NodeType::File, "child1");
        let child2 = Node::new(NodeType::File, "child2");
        let grandchild = Node::new(NodeType::File, "grandchild");

        let root_id = graph.add_node(root).unwrap();
        let child1_id = graph.add_node(child1).unwrap();
        let child2_id = graph.add_node(child2).unwrap();
        let grandchild_id = graph.add_node(grandchild).unwrap();

        graph.add_edge(Edge::new(root_id, child1_id, EdgeType::Parent)).unwrap();
        graph.add_edge(Edge::new(root_id, child2_id, EdgeType::Parent)).unwrap();
        graph.add_edge(Edge::new(child1_id, grandchild_id, EdgeType::Parent)).unwrap();

        let result = engine.bfs(root_id).unwrap();
        assert_eq!(result.len(), 4);
        assert_eq!(result[0], root_id);
    }

    #[test]
    fn test_remove_node() {
        let graph = Graph::new();
        let node = Node::new(NodeType::File, "test.txt");
        let id = graph.add_node(node).unwrap();

        assert!(graph.remove_node(&id).is_ok());
        assert!(graph.get_node(&id).is_err());
    }
}
