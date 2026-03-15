// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Query executor for executing graph queries
//!
//! 这个模块提供了查询执行器，用于执行 GraphQuery 并返回结果

use crate::query::{GraphQuery, QueryResult};
use crate::EdgeType;
use crate::{Edge, Graph, GraphError, NodeId, Result};
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::sync::Arc;

/// 查询执行器
pub struct QueryExecutor {
    graph: Arc<Graph>,
}

impl QueryExecutor {
    /// 创建新的查询执行器
    pub fn new(graph: Arc<Graph>) -> Self {
        Self { graph }
    }

    /// 执行查询
    pub fn execute(&self, query: &GraphQuery) -> Result<QueryResult> {
        match query {
            GraphQuery::FindNode(id) => self.find_node(id),
            GraphQuery::FindNodesByType(node_type) => self.find_nodes_by_type(node_type),
            GraphQuery::FindNodesByAttr { key, value } => self.find_nodes_by_attr(key, value),
            GraphQuery::FindOutgoingEdges(id) => self.find_outgoing_edges(id),
            GraphQuery::FindIncomingEdges(id) => self.find_incoming_edges(id),
            GraphQuery::FindPath { from, to } => self.find_path(from, to),
            GraphQuery::FindNeighbors(id) => self.find_neighbors(id),
            GraphQuery::Compound(queries) => self.execute_compound(queries),
        }
    }

    /// 按ID查找节点
    fn find_node(&self, id: &NodeId) -> Result<QueryResult> {
        self.graph.get_node(id)?;
        Ok(QueryResult::Nodes(vec![*id]))
    }

    /// 按类型查找节点
    fn find_nodes_by_type(&self, node_type: &str) -> Result<QueryResult> {
        let nodes: Vec<NodeId> = self
            .graph
            .all_nodes()
            .into_iter()
            .filter(|node| node.node_type.as_str() == node_type)
            .map(|node| node.id)
            .collect();

        Ok(QueryResult::Nodes(nodes))
    }

    /// 按属性查找节点
    fn find_nodes_by_attr(&self, key: &str, value: &str) -> Result<QueryResult> {
        let nodes: Vec<NodeId> = self
            .graph
            .all_nodes()
            .into_iter()
            .filter(|node| {
                node.attributes
                    .get(key)
                    .map(|attr| match attr {
                        crate::Attribute::String(s) => s == value,
                        crate::Attribute::Integer(i) => i.to_string() == value,
                        crate::Attribute::Boolean(b) => b.to_string() == value,
                        _ => false,
                    })
                    .unwrap_or(false)
            })
            .map(|node| node.id)
            .collect();

        Ok(QueryResult::Nodes(nodes))
    }

    /// 查找出边
    fn find_outgoing_edges(&self, id: &NodeId) -> Result<QueryResult> {
        let edges = self.graph.outgoing_edges(id)?;
        let edge_ids: Vec<crate::EdgeId> = edges.into_iter().map(|e| e.id).collect();
        Ok(QueryResult::Edges(edge_ids))
    }

    /// 查找入边
    fn find_incoming_edges(&self, id: &NodeId) -> Result<QueryResult> {
        let edges = self.graph.incoming_edges(id)?;
        let edge_ids: Vec<crate::EdgeId> = edges.into_iter().map(|e| e.id).collect();
        Ok(QueryResult::Edges(edge_ids))
    }

    /// 查找路径（BFS）
    fn find_path(&self, from: &NodeId, to: &NodeId) -> Result<QueryResult> {
        // 验证节点存在
        self.graph.get_node(from)?;
        self.graph.get_node(to)?;

        let mut visited: HashSet<NodeId> = HashSet::new();
        let mut queue: VecDeque<(NodeId, Vec<NodeId>)> = VecDeque::new();

        queue.push_back((*from, vec![*from]));
        visited.insert(*from);

        while let Some((current_id, path)) = queue.pop_front() {
            if current_id == *to {
                return Ok(QueryResult::Path(path));
            }

            if let Ok(neighbors) = self.graph.neighbors(&current_id) {
                for neighbor_id in neighbors {
                    if !visited.contains(&neighbor_id) {
                        visited.insert(neighbor_id);
                        let mut new_path = path.clone();
                        new_path.push(neighbor_id);
                        queue.push_back((neighbor_id, new_path));
                    }
                }
            }
        }

        Err(GraphError::PathNotFound(*from, *to))
    }

    /// 查找邻居节点
    fn find_neighbors(&self, id: &NodeId) -> Result<QueryResult> {
        let neighbors = self.graph.neighbors(id)?;
        Ok(QueryResult::Neighbors(neighbors))
    }

    /// 执行复合查询
    fn execute_compound(&self, queries: &[GraphQuery]) -> Result<QueryResult> {
        let mut all_nodes = Vec::new();
        let mut all_edges = Vec::new();

        for query in queries {
            match self.execute(query)? {
                QueryResult::Nodes(nodes) => all_nodes.extend(nodes),
                QueryResult::Edges(edges) => all_edges.extend(edges),
                QueryResult::Path(path) => all_nodes.extend(path),
                QueryResult::Neighbors(neighbors) => all_nodes.extend(neighbors),
            }
        }

        // 去重
        all_nodes.sort();
        all_nodes.dedup();
        all_edges.sort();
        all_edges.dedup();

        Ok(QueryResult::Nodes(all_nodes))
    }
}

/// 路径查找器 - 支持多种路径算法
pub struct PathFinder {
    graph: Arc<Graph>,
}

impl PathFinder {
    pub fn new(graph: Arc<Graph>) -> Self {
        Self { graph }
    }

    /// 查找最短路径（BFS - 无权图）
    pub fn shortest_path(&self, from: NodeId, to: NodeId) -> Result<Vec<NodeId>> {
        let mut visited: HashSet<NodeId> = HashSet::new();
        let mut queue: VecDeque<(NodeId, Vec<NodeId>)> = VecDeque::new();

        queue.push_back((from, vec![from]));
        visited.insert(from);

        while let Some((current_id, path)) = queue.pop_front() {
            if current_id == to {
                return Ok(path);
            }

            if let Ok(neighbors) = self.graph.neighbors(&current_id) {
                for neighbor_id in neighbors {
                    if !visited.contains(&neighbor_id) {
                        visited.insert(neighbor_id);
                        let mut new_path = path.clone();
                        new_path.push(neighbor_id);
                        queue.push_back((neighbor_id, new_path));
                    }
                }
            }
        }

        Err(GraphError::PathNotFound(from, to))
    }

    /// 查找所有路径
    pub fn all_paths(&self, from: NodeId, to: NodeId) -> Result<Vec<Vec<NodeId>>> {
        let mut paths = Vec::new();
        let mut current_path = Vec::new();
        let mut visited = HashSet::new();

        self.all_paths_util(from, to, &mut visited, &mut current_path, &mut paths)?;
        Ok(paths)
    }

    fn all_paths_util(
        &self,
        current: NodeId,
        target: NodeId,
        visited: &mut HashSet<NodeId>,
        path: &mut Vec<NodeId>,
        all_paths: &mut Vec<Vec<NodeId>>,
    ) -> Result<()> {
        visited.insert(current);
        path.push(current);

        if current == target {
            all_paths.push(path.clone());
        } else {
            if let Ok(neighbors) = self.graph.neighbors(&current) {
                for neighbor in neighbors {
                    if !visited.contains(&neighbor) {
                        self.all_paths_util(neighbor, target, visited, path, all_paths)?;
                    }
                }
            }
        }

        path.pop();
        visited.remove(&current);

        Ok(())
    }

    /// 查找可达节点
    pub fn reachable_from(&self, start: NodeId) -> Result<HashSet<NodeId>> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back(start);
        visited.insert(start);

        while let Some(current) = queue.pop_front() {
            if let Ok(neighbors) = self.graph.neighbors(&current) {
                for neighbor in neighbors {
                    if !visited.contains(&neighbor) {
                        visited.insert(neighbor);
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        Ok(visited)
    }
}

/// 图分析器 - 提供图分析功能
pub struct GraphAnalyzer {
    graph: Arc<Graph>,
}

impl GraphAnalyzer {
    pub fn new(graph: Arc<Graph>) -> Self {
        Self { graph }
    }

    /// 计算节点的度中心性
    pub fn degree_centrality(&self, node_id: &NodeId) -> Result<f64> {
        let total_nodes = self.graph.node_count();
        if total_nodes == 0 {
            return Ok(0.0);
        }

        let outgoing = self.graph.outgoing_edges(node_id)?.len();
        let incoming = self.graph.incoming_edges(node_id)?.len();
        let degree = outgoing + incoming;

        Ok(degree as f64 / (total_nodes - 1) as f64)
    }

    /// 计算所有节点的度中心性
    pub fn all_degree_centrality(&self) -> HashMap<NodeId, f64> {
        let mut result = HashMap::new();
        let total_nodes = self.graph.node_count();

        if total_nodes == 0 {
            return result;
        }

        for node in self.graph.all_nodes() {
            let id = node.id;
            if let Ok(centrality) = self.degree_centrality(&id) {
                result.insert(id, centrality);
            }
        }

        result
    }

    /// 计算图的密度
    pub fn density(&self) -> f64 {
        let node_count = self.graph.node_count();
        let edge_count = self.graph.edge_count();

        if node_count == 0 {
            return 0.0;
        }

        let max_edges = node_count * (node_count - 1);
        if max_edges == 0 {
            return 0.0;
        }

        edge_count as f64 / max_edges as f64
    }

    /// 检测连通分量
    pub fn connected_components(&self) -> Vec<HashSet<NodeId>> {
        let mut visited = HashSet::new();
        let mut components = Vec::new();

        for node in self.graph.all_nodes() {
            let node_id = node.id;
            if !visited.contains(&node_id) {
                let component = self.bfs_component(node_id, &mut visited);
                components.push(component);
            }
        }

        components
    }

    fn bfs_component(&self, start: NodeId, visited: &mut HashSet<NodeId>) -> HashSet<NodeId> {
        let mut component = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back(start);
        visited.insert(start);
        component.insert(start);

        while let Some(current) = queue.pop_front() {
            if let Ok(neighbors) = self.graph.neighbors(&current) {
                for neighbor in neighbors {
                    if !visited.contains(&neighbor) {
                        visited.insert(neighbor);
                        component.insert(neighbor);
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        component
    }

    /// 查找孤立节点
    pub fn isolated_nodes(&self) -> Vec<NodeId> {
        self.graph
            .all_nodes()
            .into_iter()
            .filter(|node| {
                self.graph
                    .outgoing_edges(&node.id)
                    .unwrap_or_default()
                    .is_empty()
                    && self
                        .graph
                        .incoming_edges(&node.id)
                        .unwrap_or_default()
                        .is_empty()
            })
            .map(|node| node.id)
            .collect()
    }

    /// 计算聚类系数
    pub fn clustering_coefficient(&self, node_id: &NodeId) -> Result<f64> {
        let neighbors = self.graph.neighbors(node_id)?;

        if neighbors.len() < 2 {
            return Ok(0.0);
        }

        let neighbor_set: HashSet<NodeId> = neighbors.into_iter().collect();
        let k = neighbor_set.len() as f64;
        let mut links = 0.0;

        for &n1 in &neighbor_set {
            for &n2 in &neighbor_set {
                if n1 != n2 {
                    if let Ok(edges) = self.graph.outgoing_edges(&n1) {
                        if edges.iter().any(|e| e.target == n2) {
                            links += 1.0;
                        }
                    }
                }
            }
        }

        let possible_links = k * (k - 1.0);
        if possible_links == 0.0 {
            return Ok(0.0);
        }

        Ok(links / possible_links)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Edge, Node, NodeType};
    use uuid::Uuid;

    fn setup_test_graph() -> Arc<Graph> {
        let graph = Arc::new(Graph::new());

        let root = Node::new(NodeType::Directory, "root");
        let child1 = Node::new(NodeType::File, "child1");
        let child2 = Node::new(NodeType::File, "child2");
        let grandchild = Node::new(NodeType::File, "grandchild");

        let root_id = graph.add_node(root).unwrap();
        let child1_id = graph.add_node(child1).unwrap();
        let child2_id = graph.add_node(child2).unwrap();
        let grandchild_id = graph.add_node(grandchild).unwrap();

        graph
            .add_edge(Edge::new(root_id, child1_id, EdgeType::Parent))
            .unwrap();
        graph
            .add_edge(Edge::new(root_id, child2_id, EdgeType::Parent))
            .unwrap();
        graph
            .add_edge(Edge::new(child1_id, grandchild_id, EdgeType::Parent))
            .unwrap();

        graph
    }

    #[test]
    fn test_query_executor_find_by_type() {
        let graph = setup_test_graph();
        let executor = QueryExecutor::new(graph);

        let query = GraphQuery::FindNodesByType("file".to_string());
        let result = executor.execute(&query).unwrap();

        match result {
            QueryResult::Nodes(nodes) => {
                assert_eq!(nodes.len(), 3); // child1, child2, grandchild
            }
            _ => panic!("Expected Nodes result"),
        }
    }

    #[test]
    fn test_path_finder_shortest_path() {
        let graph = setup_test_graph();
        let finder = PathFinder::new(graph.clone());

        let nodes = graph.all_nodes();
        let root_id = nodes
            .iter()
            .find(|n| n.name == "root")
            .map(|n| n.id)
            .unwrap();
        let grandchild_id = nodes
            .iter()
            .find(|n| n.name == "grandchild")
            .map(|n| n.id)
            .unwrap();

        let path = finder.shortest_path(root_id, grandchild_id).unwrap();
        assert_eq!(path.len(), 3); // root -> child1 -> grandchild
    }

    #[test]
    fn test_graph_analyzer_density() {
        let graph = setup_test_graph();
        let analyzer = GraphAnalyzer::new(graph);

        let density = analyzer.density();
        assert!(density > 0.0 && density <= 1.0);
    }

    #[test]
    fn test_graph_analyzer_connected_components() {
        let graph = setup_test_graph();
        let analyzer = GraphAnalyzer::new(graph);

        let components = analyzer.connected_components();
        assert!(components.len() >= 1);
    }

    #[test]
    fn test_query_executor_find_path() {
        let graph = setup_test_graph();
        let executor = QueryExecutor::new(graph.clone());

        let nodes = graph.all_nodes();
        let root_id = nodes
            .iter()
            .find(|n| n.name == "root")
            .map(|n| n.id)
            .unwrap();
        let grandchild_id = nodes
            .iter()
            .find(|n| n.name == "grandchild")
            .map(|n| n.id)
            .unwrap();

        let query = GraphQuery::FindPath {
            from: root_id,
            to: grandchild_id,
        };

        let result = executor.execute(&query).unwrap();
        match result {
            QueryResult::Path(path) => {
                assert!(path.len() >= 2);
            }
            _ => panic!("Expected Path result"),
        }
    }
}
