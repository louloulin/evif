// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{RuntimeConfig, RuntimeError, RuntimeResult};
use evif_auth::{AuthManager as EvifAuthManager, AuthPolicy as EvifAuthPolicy};
use evif_graph::{GraphConfig, GraphEngine};
use evif_storage::MemoryStorage;
use std::sync::Arc;

/// EVIF 运行时
pub struct EvifRuntime {
    config: RuntimeConfig,
    graph_engine: Arc<GraphEngine>,
    storage_manager: Arc<MemoryStorage>,
    auth_manager: Arc<EvifAuthManager>,
}

impl EvifRuntime {
    pub fn new() -> RuntimeResult<Self> {
        Self::with_config(RuntimeConfig::default())
    }

    pub fn with_config(config: RuntimeConfig) -> RuntimeResult<Self> {
        let graph_config = GraphConfig {
            max_nodes: config.max_nodes,
            max_edges: config.max_nodes * 10,
            enable_index: true,
        };
        let graph_engine = Arc::new(GraphEngine::with_config(graph_config));
        let storage_manager = Arc::new(MemoryStorage::new());

        let auth_policy = match config.auth_policy {
            crate::AuthPolicy::Open => EvifAuthPolicy::Open,
            crate::AuthPolicy::Strict => EvifAuthPolicy::Strict,
        };
        let auth_manager = Arc::new(EvifAuthManager::with_policy(auth_policy));

        Ok(EvifRuntime {
            config,
            graph_engine,
            storage_manager,
            auth_manager,
        })
    }

    pub fn graph_engine(&self) -> Arc<GraphEngine> {
        Arc::clone(&self.graph_engine)
    }

    pub fn storage_manager(&self) -> Arc<MemoryStorage> {
        Arc::clone(&self.storage_manager)
    }

    pub fn auth_manager(&self) -> Arc<EvifAuthManager> {
        Arc::clone(&self.auth_manager)
    }

    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }
}

impl Default for EvifRuntime {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let runtime = EvifRuntime::new();
        assert!(runtime.is_ok());

        let runtime = runtime.unwrap();
        assert_eq!(runtime.config().max_nodes, 1_000_000);
    }

    #[test]
    fn test_runtime_components() {
        let runtime = EvifRuntime::new().unwrap();

        let graph = runtime.graph_engine();
        let storage = runtime.storage_manager();
        let auth = runtime.auth_manager();

        assert_eq!(graph.graph().node_count(), 0);
        assert_eq!(storage.node_count(), 0);
    }
}
