// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde::{Deserialize, Serialize};

/// 运行时配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub max_nodes: usize,
    pub max_connections: usize,
    pub storage_backend: StorageConfig,
    pub auth_policy: AuthPolicy,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        RuntimeConfig {
            max_nodes: 1_000_000,
            max_connections: 10_000,
            storage_backend: StorageConfig::Memory,
            auth_policy: AuthPolicy::Strict,
        }
    }
}

/// 存储配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageConfig {
    Memory,
    Sled { path: String },
    RocksDB { path: String },
    S3 { bucket: String, region: String },
}

/// 认证策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthPolicy {
    Open,
    Strict,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RuntimeConfig::default();
        assert_eq!(config.max_nodes, 1_000_000);
    }
}
