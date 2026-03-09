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
//! - `proactive`: Background monitoring and proactive extraction (Phase 1.5)

pub mod embedding;
pub mod error;
pub mod langchain;
pub mod llamaindex;
pub mod llm;
pub mod metrics;
pub mod models;
pub mod pipeline;
pub mod proactive;
pub mod storage;
pub mod vector;
pub mod workflow;

#[cfg(feature = "plugin")]
pub mod plugin;

pub use error::MemError;
pub use models::*;
pub use pipeline::*;
pub use proactive::{
    ExtractionStats, ExtractorConfig, ProactiveAgent, ProactiveConfig, ProactiveEvent,
    ProactiveExtractor, ProactiveStats, ProactiveResult, ResourceMonitor, EventTrigger,
    IntentionPredictor, IntentConfig, IntentResult, PredictedIntent, MemoryPattern,
    CostOptimizer, CostOptimizerConfig, CostOptimizerStats, CacheEntry, BatchItem,
};
pub use langchain::{
    EvifMemory, EvifMemoryConfig, ChatMessage, BufferMemory, ConversationTokenBuffer,
    VectorStoreRetriever,
};
pub use llamaindex::{
    LlamaIndexConfig, ChatMessageLLM, EvifChatStore, EvifVectorStore, EvifDocument,
    EvifKVStore, QueryResult,
};
pub use metrics::{Metrics, MetricsConfig, MetricsRegistry, MetricsError};
