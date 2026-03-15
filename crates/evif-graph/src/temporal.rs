// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

//! 时序知识图谱扩展
//! 为记忆平台提供时序关系查询能力

use crate::{EdgeType, Graph, GraphError, NodeId, Result};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

/// 时序查询结果
#[derive(Debug, Clone)]
pub struct TemporalQueryResult {
    pub nodes: Vec<NodeId>,
    pub paths: Vec<TemporalPath>,
    pub timeline: Vec<TimelineEvent>,
}

/// 时序路径
#[derive(Debug, Clone)]
pub struct TemporalPath {
    pub nodes: Vec<NodeId>,
    pub edge_types: Vec<EdgeType>,
    pub total_weight: f64,
}

/// 时间线事件
#[derive(Debug, Clone)]
pub struct TimelineEvent {
    pub node_id: NodeId,
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
}

/// 时序图扩展
pub struct TemporalGraph {
    graph: Arc<Graph>,
    /// 节点时间戳索引: node_id -> timestamp
    timestamps: HashMap<NodeId, DateTime<Utc>>,
}

impl TemporalGraph {
    pub fn new(graph: Arc<Graph>) -> Self {
        TemporalGraph {
            graph,
            timestamps: HashMap::new(),
        }
    }

    /// 设置节点时间戳
    pub fn set_timestamp(&mut self, node_id: NodeId, timestamp: DateTime<Utc>) {
        self.timestamps.insert(node_id, timestamp);
    }

    /// 获取节点时间戳
    pub fn get_timestamp(&self, node_id: &NodeId) -> Option<DateTime<Utc>> {
        self.timestamps.get(node_id).copied()
    }

    /// 时间感知的 BFS 遍历
    /// 按时间顺序访问节点
    pub fn temporal_bfs(&self, start: NodeId, max_depth: usize) -> Result<Vec<NodeId>> {
        if !self.graph.contains_node(&start) {
            return Err(GraphError::NodeNotFound(start));
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        queue.push_back((start, 0));
        visited.insert(start);

        while let Some((node_id, depth)) = queue.pop_front() {
            result.push(node_id);

            if depth >= max_depth {
                continue;
            }

            // 获取所有出边并按时间排序
            let mut successors = self.get_temporal_successors(&node_id)?;
            successors.sort_by(|a, b| {
                let ts_a = self.get_timestamp(a).unwrap_or(Utc::now());
                let ts_b = self.get_timestamp(b).unwrap_or(Utc::now());
                ts_a.cmp(&ts_b)
            });

            for successor_id in successors {
                if !visited.contains(&successor_id) {
                    visited.insert(successor_id);
                    queue.push_back((successor_id, depth + 1));
                }
            }
        }

        Ok(result)
    }

    /// 获取时序后继节点
    fn get_temporal_successors(&self, node_id: &NodeId) -> Result<Vec<NodeId>> {
        let edges = self.graph.outgoing_edges(node_id)?;
        let mut successors = Vec::new();

        for edge in edges {
            if edge.edge_type.is_temporal() {
                successors.push(edge.target);
            }
        }

        Ok(successors)
    }

    /// 查找两个节点之间的时序路径
    pub fn find_temporal_path(&self, start: NodeId, end: NodeId) -> Result<Option<TemporalPath>> {
        if !self.graph.contains_node(&start) {
            return Err(GraphError::NodeNotFound(start));
        }
        if !self.graph.contains_node(&end) {
            return Err(GraphError::NodeNotFound(end));
        }

        // BFS 查找最短时序路径
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut parent: HashMap<NodeId, (NodeId, EdgeType)> = HashMap::new();

        queue.push_back(start);
        visited.insert(start);

        while let Some(current) = queue.pop_front() {
            if current == end {
                // 重建路径
                let mut path = TemporalPath {
                    nodes: Vec::new(),
                    edge_types: Vec::new(),
                    total_weight: 0.0,
                };

                let mut node = end;
                while node != start {
                    path.nodes.push(node);
                    if let Some((prev, edge_type)) = parent.get(&node) {
                        path.edge_types.push(edge_type.clone());
                        node = *prev;
                    } else {
                        break;
                    }
                }
                path.nodes.push(start);
                path.nodes.reverse();
                path.edge_types.reverse();

                return Ok(Some(path));
            }

            // 探索时序后继
            let successors = self.get_temporal_successors(&current)?;
            for successor in successors {
                if !visited.contains(&successor) {
                    visited.insert(successor);
                    let edges = self.graph.outgoing_edges(&current)?;
                    let edge = edges.into_iter().find(|e| e.target == successor).unwrap();

                    parent.insert(successor, (current, edge.edge_type.clone()));
                    queue.push_back(successor);
                }
            }
        }

        Ok(None)
    }

    /// 获取事件时间线
    pub fn get_event_timeline(
        &self,
        start: NodeId,
        event_type: &str,
    ) -> Result<Vec<TimelineEvent>> {
        let mut timeline = Vec::new();

        // 查找所有 Event 类型的节点
        for node in self.graph.get_all_nodes() {
            if node.node_type.as_str() == event_type {
                if let Some(timestamp) = self.get_timestamp(&node.id) {
                    timeline.push(TimelineEvent {
                        node_id: node.id,
                        timestamp,
                        event_type: event_type.to_string(),
                    });
                }
            }
        }

        // 按时间排序
        timeline.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(timeline)
    }

    /// 查找因果链
    pub fn find_causal_chain(&self, start: NodeId) -> Result<Vec<NodeId>> {
        if !self.graph.contains_node(&start) {
            return Err(GraphError::NodeNotFound(start));
        }

        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();

        queue.push_back(start);
        visited.insert(start);

        while let Some(current) = queue.pop_front() {
            result.push(current);

            // 查找导致当前节点的所有节点
            let incoming = self.graph.incoming_edges(&current)?;
            for edge in incoming {
                if edge.edge_type == EdgeType::Causes {
                    if !visited.contains(&edge.source) {
                        visited.insert(edge.source);
                        queue.push_back(edge.source);
                    }
                }
            }
        }

        Ok(result)
    }

    /// 挖掘周期性模式
    pub fn find_periodic_patterns(
        &self,
        node_id: NodeId,
        window_days: i64,
    ) -> Result<Vec<(DateTime<Utc>, DateTime<Utc>)>> {
        let timestamp = self
            .get_timestamp(&node_id)
            .ok_or_else(|| GraphError::InvalidOperation("节点没有时间戳".to_string()))?;

        let window = chrono::Duration::days(window_days);
        let mut patterns = Vec::new();

        // 查找在时间窗口内的相似事件
        for (other_id, other_ts) in &self.timestamps {
            if *other_id == node_id {
                continue;
            }

            let diff = (*other_ts - timestamp).num_days().abs();
            if diff <= window_days {
                // 检查是否有相似边
                let edges = self.graph.outgoing_edges(&node_id)?;
                for edge in edges {
                    if edge.edge_type == EdgeType::SimilarTo && edge.target == *other_id {
                        patterns.push((timestamp, *other_ts));
                    }
                }
            }
        }

        Ok(patterns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Edge, EdgeType, Node, NodeType};
    fn test_temporal_edge_types() {
        assert!(EdgeType::Before.is_temporal());
        assert!(EdgeType::After.is_temporal());
        assert!(EdgeType::Simultaneous.is_temporal());
        assert!(EdgeType::Causes.is_temporal());

        assert!(!EdgeType::Parent.is_temporal());
    }

    #[test]
    fn test_memory_node_types() {
        assert!(NodeType::MemoryItem.is_memory_related());
        assert!(NodeType::Category.is_memory_related());
        assert!(NodeType::Resource.is_memory_related());
        assert!(NodeType::Event.is_memory_related());

        assert!(!NodeType::File.is_memory_related());
    }

    #[test]
    fn test_temporal_graph() {
        let graph = Arc::new(Graph::new());
        let mut temporal = TemporalGraph::new(graph.clone());

        // 创建测试节点
        let node1 = Node::new(NodeType::Event, "event1");
        let node2 = Node::new(NodeType::Event, "event2");
        let node3 = Node::new(NodeType::Event, "event3");

        let id1 = graph.add_node(node1).unwrap();
        let id2 = graph.add_node(node2).unwrap();
        let id3 = graph.add_node(node3).unwrap();

        // 设置时间戳
        let ts1 = DateTime::parse_from_rfc3339("2026-01-01T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let ts2 = DateTime::parse_from_rfc3339("2026-01-02T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let ts3 = DateTime::parse_from_rfc3339("2026-01-03T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        temporal.set_timestamp(id1, ts1);
        temporal.set_timestamp(id2, ts2);
        temporal.set_timestamp(id3, ts3);

        // 创建时序边
        let edge1 = Edge::new(id1, id2, EdgeType::Before);
        let edge2 = Edge::new(id2, id3, EdgeType::Before);

        graph.add_edge(edge1).unwrap();
        graph.add_edge(edge2).unwrap();

        // 测试时序 BFS
        let result = temporal.temporal_bfs(id1, 10).unwrap();
        assert_eq!(result.len(), 3);

        // 测试时间线
        let timeline = temporal.get_event_timeline(id1, "event").unwrap();
        assert_eq!(timeline.len(), 3);
    }
}
