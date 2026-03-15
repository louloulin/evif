// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

//! EVIF 图引擎 - 核心数据结构和算法
//!
//! 本模块提供了图系统的核心抽象，包括节点、边和图结构。

pub mod edge;
pub mod error;
pub mod executor;
pub mod graph;
pub mod index;
pub mod node;
pub mod query;
pub mod temporal;

// 重新导出主要类型
pub use edge::{Edge, EdgeBuilder, EdgeId, EdgeType};
pub use error::{GraphError, Result};
pub use executor::{GraphAnalyzer, PathFinder, QueryExecutor};
pub use graph::{Graph, GraphConfig, GraphEngine};
pub use index::{GraphIndex, IndexedGraph, MemoryIndex};
pub use node::{Attribute, ContentHandle, Metadata, Node, NodeBuilder, NodeId, NodeType};
pub use query::{GraphQuery, QueryBuilder, QueryResult};
pub use temporal::{TemporalGraph, TemporalPath, TemporalQueryResult, TimelineEvent};
