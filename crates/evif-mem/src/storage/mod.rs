//! Storage backends for memory platform
//!
//! Provides different storage options:
//! - In-memory storage (for development/testing)
//! - SQLite storage (persistent, with optional vector support)
//! - PostgreSQL storage (production-grade with connection pooling)

pub mod memory;

pub use memory::MemoryStorage;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "sqlite")]
pub use sqlite::SQLiteStorage;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "postgres")]
pub use postgres::PostgresStorage;
