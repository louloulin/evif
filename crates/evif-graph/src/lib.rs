// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

//! EVIF 图引擎 - 核心数据结构和算法
//!
//! 本模块提供了图系统的核心抽象，包括节点、边和图结构。

pub mod error;
pub mod node;
pub mod edge;
pub mod graph;
pub mod index;
pub mod query;
pub mod executor;

// 重新导出主要类型
pub use error::{GraphError, Result};
pub use node::{Node, NodeId, NodeType, NodeBuilder, Metadata, Attribute, ContentHandle};
pub use edge::{Edge, EdgeId, EdgeType, EdgeBuilder};
pub use graph::{Graph, GraphEngine, GraphConfig};
pub use index::{GraphIndex, MemoryIndex, IndexedGraph};
pub use query::{GraphQuery, QueryBuilder, QueryResult};
pub use executor::{QueryExecutor, PathFinder, GraphAnalyzer};
