//! EVIF Memory Platform
//!
//! A memory platform built on top of EVIF virtual filesystem.
//! Uses MD file format with YAML Frontmatter for AI-native storage.
//!
//! # Core Features
//! - Memory extraction from various modalities (conversation, document, image, video, audio)
//! - Vector-based semantic retrieval
//! - Category-based organization
//! - Memory reinforcement and deduplication
//! - Cross-reference linking via filesystem symlinks
//!
//! # Architecture
//! - `models`: Core data structures (MemoryItem, Resource, Category)
//! - `storage`: Storage backends (Memory, SQLite, RocksDB)
//! - `embedding`: Embedding management with LRU cache
//! - `vector`: Vector search index for semantic retrieval
//! - `llm`: LLM client implementations
//! - `pipeline`: Memorize and retrieve pipelines

pub mod embedding;
pub mod error;
pub mod llm;
pub mod models;
pub mod storage;
pub mod vector;

#[cfg(feature = "plugin")]
pub mod plugin;

pub use error::MemError;
pub use models::*;
