//! Storage backends for memory platform
//!
//! Provides different storage options:
//! - In-memory storage (for development/testing)
//! - SQLite storage (persistent, with optional vector support)
//! - RocksDB storage (for high performance)

pub mod memory;

pub use memory::MemoryStorage;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "sqlite")]
pub use sqlite::SQLiteStorage;
