// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

//! EVIF 存储层 - 可插拔的存储后端

pub mod backend;
pub mod error;
pub mod memory;
pub mod transaction;

#[cfg(feature = "sled-backend")]
pub mod sled;

#[cfg(feature = "rocksdb-backend")]
pub mod rocksdb;

#[cfg(feature = "s3-backend")]
pub mod s3;

pub use backend::{StorageBackend, StorageOp};
pub use error::{StorageError, StorageResult};
pub use memory::MemoryStorage;
pub use transaction::MemoryTransaction;

#[cfg(feature = "sled-backend")]
pub use sled::{SledStorage, SledTransaction, StorageStats};

#[cfg(feature = "rocksdb-backend")]
pub use rocksdb::{RocksDBStats, RocksDBStorage, RocksDBTransaction};

#[cfg(feature = "s3-backend")]
pub use s3::{S3Stats, S3Storage, S3StorageConfig, S3Transaction};

// 重新导出 evif-graph 类型以方便使用
pub use evif_graph::{Edge, EdgeId, Node, NodeId};
