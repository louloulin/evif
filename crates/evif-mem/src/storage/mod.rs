//! Storage backends for memory platform
//!
//! Provides different storage options:
//! - In-memory storage (for development/testing)
//! - SQLite storage (with optional vector support)
//! - RocksDB storage (for high performance)

pub mod memory;

pub use memory::MemoryStorage;
