//! LlamaIndex Integration Module
//!
//! This module provides LlamaIndex-compatible memory interfaces for evif-mem.
//! It allows evif-mem to be used as a memory backend in LlamaIndex applications.
//!
//! # Features
//! - ChatStore: Chat history management
//! - VectorStore: RAG-based retrieval
//! - Memory: Document-based memory
//!
//! # Usage
//! ```ignore
//! use evif_mem::llamaindex::{EvifChatStore, EvifVectorStore};
//! use std::sync::Arc();
//!
//! let storage = Arc::new(evif_mem::storage::MemoryStorage::new());
//! let chat_store = EvifChatStore::new(storage);
//! ```

use crate::embedding::EmbeddingManager;
use crate::error::MemError;
use crate::models::MemoryItem;
use crate::storage::MemoryStorage;
use crate::vector::VectorIndex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for LlamaIndex integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaIndexConfig {
    /// Session ID for chat history
    pub session_id: Option<String>,
    /// Whether to store messages in memory
    pub store_messages: bool,
    /// Maximum messages to keep
    pub max_messages: usize,
    /// Return as JSON string
    pub return_json: bool,
}

impl Default for LlamaIndexConfig {
    fn default() -> Self {
        Self {
            session_id: None,
            store_messages: true,
            max_messages: 100,
            return_json: false,
        }
    }
}

/// Chat message for LlamaIndex compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageLLM {
    /// Message role: "user", "assistant", "system"
    pub role: String,
    /// Message content
    pub content: String,
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

impl ChatMessageLLM {
    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
            metadata: None,
        }
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
            metadata: None,
        }
    }

    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
            metadata: None,
        }
    }
}

/// EvifChatStore - LlamaIndex-compatible chat store
///
/// Provides chat history management compatible with LlamaIndex's ChatStore interface.
#[derive(Clone)]
pub struct EvifChatStore {
    storage: Arc<MemoryStorage>,
    config: LlamaIndexConfig,
    session_id: String,
}

impl EvifChatStore {
    /// Create a new EvifChatStore
    pub fn new(storage: Arc<MemoryStorage>, config: LlamaIndexConfig) -> Self {
        let session_id = config
            .session_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        Self {
            storage,
            config,
            session_id,
        }
    }

    /// Create with default configuration
    pub fn with_defaults(storage: Arc<MemoryStorage>) -> Self {
        Self::new(storage, LlamaIndexConfig::default())
    }

    /// Add a chat message
    pub async fn add_message(&self, message: ChatMessageLLM) -> Result<String, MemError> {
        let content = format!("{}: {}", message.role, message.content);

        let item = MemoryItem::new(
            crate::models::MemoryType::Knowledge,
            content.clone(),
            content,
        );

        self.storage.put_item(item.clone())?;
        Ok(item.id.clone())
    }

    /// Add user message
    pub async fn add_user_message(&self, content: &str) -> Result<String, MemError> {
        self.add_message(ChatMessageLLM::user(content)).await
    }

    /// Add assistant message
    pub async fn add_assistant_message(&self, content: &str) -> Result<String, MemError> {
        self.add_message(ChatMessageLLM::assistant(content)).await
    }

    /// Get chat history
    pub async fn get_messages(&self) -> Result<Vec<ChatMessageLLM>, MemError> {
        let items = self.storage.get_items_by_type("knowledge");

        let messages: Vec<ChatMessageLLM> = items
            .into_iter()
            .rev()
            .take(self.config.max_messages)
            .map(|item| {
                let content = item.content.clone();
                let parts: Vec<&str> = content.splitn(2, ": ").collect();
                let (role, msg_content) = if parts.len() == 2 {
                    (parts[0].to_string(), parts[1].to_string())
                } else {
                    ("user".to_string(), content)
                };

                ChatMessageLLM {
                    role,
                    content: msg_content,
                    metadata: None,
                }
            })
            .collect();

        Ok(messages)
    }

    /// Get messages as JSON string
    pub async fn get_messages_json(&self) -> Result<String, MemError> {
        let messages = self.get_messages().await?;
        Ok(serde_json::to_string(&messages)?)
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Delete message by ID
    pub async fn delete_message(&self, _message_id: &str) -> Result<(), MemError> {
        // Note: Storage doesn't support delete, this is a placeholder
        Ok(())
    }

    /// Clear all messages
    pub async fn clear(&self) -> Result<(), MemError> {
        // Note: Storage doesn't support delete, this is a placeholder
        Ok(())
    }
}

/// EvifVectorStore - LlamaIndex-compatible vector store
///
/// Provides RAG capabilities compatible with LlamaIndex's VectorStore interface.
pub struct EvifVectorStore {
    storage: Arc<MemoryStorage>,
    vector_index: Arc<RwLock<Box<dyn VectorIndex>>>,
    embedding_manager: Arc<RwLock<EmbeddingManager>>,
    k: usize,
}

impl EvifVectorStore {
    /// Create a new EvifVectorStore
    pub fn new(
        storage: Arc<MemoryStorage>,
        vector_index: Arc<RwLock<Box<dyn VectorIndex>>>,
        embedding_manager: Arc<RwLock<EmbeddingManager>>,
        k: usize,
    ) -> Self {
        Self {
            storage,
            vector_index,
            embedding_manager,
            k,
        }
    }

    /// Add a document to the vector store
    pub async fn add_document(
        &self,
        text: &str,
        _metadata: Option<serde_json::Value>,
    ) -> Result<String, MemError> {
        // Create embedding
        let embedding_guard = self.embedding_manager.read().await;
        let embedding = embedding_guard.embed(text).await?;
        drop(embedding_guard);

        // Create memory item
        let item = MemoryItem::new(
            crate::models::MemoryType::Knowledge,
            text.to_string(),
            text.to_string(),
        );

        // Add to storage
        self.storage.put_item(item.clone())?;

        // Add to vector index
        let index_guard = self.vector_index.write().await;
        index_guard.add(item.id.clone(), embedding, None).await?;

        Ok(item.id.clone())
    }

    /// Query the vector store
    pub async fn query(&self, query: &str) -> Result<Vec<QueryResult>, MemError> {
        // Get query embedding
        let embedding_guard = self.embedding_manager.read().await;
        let embedding = embedding_guard.embed(query).await?;
        drop(embedding_guard);

        // Search vector index
        let index_guard = self.vector_index.read().await;
        let results = index_guard.search(&embedding, Some(self.k), None).await?;
        drop(index_guard);

        // Get items from storage
        let mut query_results = Vec::new();
        for result in results {
            if let Ok(item) = self.storage.get_item(&result.id) {
                query_results.push(QueryResult {
                    id: item.id.clone(),
                    text: item.content.clone(),
                    score: result.score,
                    metadata: None,
                });
            }
        }

        Ok(query_results)
    }

    /// Delete a document by ID
    pub async fn delete(&self, _id: &str) -> Result<(), MemError> {
        // Note: Storage doesn't support delete
        Ok(())
    }
}

/// Query result from vector store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Document ID
    pub id: String,
    /// Document text
    pub text: String,
    /// Similarity score
    pub score: f32,
    /// Metadata
    pub metadata: Option<serde_json::Value>,
}

/// EvifDocument - For storing documents
#[derive(Clone)]
pub struct EvifDocument {
    text: String,
    metadata: Option<serde_json::Value>,
}

impl EvifDocument {
    /// Create a new document
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            metadata: None,
        }
    }

    /// Create with metadata
    pub fn with_metadata(text: impl Into<String>, metadata: serde_json::Value) -> Self {
        Self {
            text: text.into(),
            metadata: Some(metadata),
        }
    }

    /// Get text content
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get metadata
    pub fn metadata(&self) -> Option<&serde_json::Value> {
        self.metadata.as_ref()
    }
}

/// EvifKVStore - Key-value store for LlamaIndex
///
/// Simple key-value storage for caching and temporary storage.
#[derive(Clone)]
pub struct EvifKVStore {
    storage: Arc<MemoryStorage>,
}

impl EvifKVStore {
    /// Create a new KV store
    pub fn new(storage: Arc<MemoryStorage>) -> Self {
        Self { storage }
    }

    /// Set a value
    pub async fn set(&self, key: &str, value: &str) -> Result<(), MemError> {
        let item = MemoryItem::new(
            crate::models::MemoryType::Knowledge,
            key.to_string(),
            value.to_string(),
        );
        self.storage.put_item(item)?;
        Ok(())
    }

    /// Get a value
    pub async fn get(&self, key: &str) -> Result<Option<String>, MemError> {
        let items = self.storage.get_items_by_type("knowledge");
        for item in items {
            if item.summary == key {
                return Ok(Some(item.content));
            }
        }
        Ok(None)
    }

    /// Delete a key
    pub async fn delete(&self, _key: &str) -> Result<(), MemError> {
        // Note: Storage doesn't support delete
        Ok(())
    }

    /// Check if key exists
    pub async fn exists(&self, key: &str) -> Result<bool, MemError> {
        Ok(self.get(key).await?.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create test chat store
    fn create_test_chat_store() -> EvifChatStore {
        let storage = Arc::new(MemoryStorage::new());
        EvifChatStore::with_defaults(storage)
    }

    #[tokio::test]
    async fn test_chat_store_creation() {
        let store = create_test_chat_store();
        assert!(!store.session_id().is_empty());
    }

    #[tokio::test]
    async fn test_add_and_get_messages() -> Result<(), MemError> {
        let store = create_test_chat_store();

        store.add_user_message("Hello").await?;
        store.add_assistant_message("Hi there!").await?;

        let messages = store.get_messages().await?;
        assert!(!messages.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_chat_message_llm() {
        let msg = ChatMessageLLM::user("Hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");

        let msg = ChatMessageLLM::assistant("Hi there");
        assert_eq!(msg.role, "assistant");

        let msg = ChatMessageLLM::system("You are helpful");
        assert_eq!(msg.role, "system");
    }

    #[tokio::test]
    async fn test_llamaindex_config_defaults() {
        let config = LlamaIndexConfig::default();
        assert!(config.store_messages);
        assert_eq!(config.max_messages, 100);
    }

    #[tokio::test]
    async fn test_evif_document() {
        let doc = EvifDocument::new("Test content");
        assert_eq!(doc.text(), "Test content");
        assert!(doc.metadata().is_none());

        let doc =
            EvifDocument::with_metadata("Test content", serde_json::json!({"source": "test"}));
        assert_eq!(doc.text(), "Test content");
        assert!(doc.metadata().is_some());
    }

    #[tokio::test]
    async fn test_kv_store() -> Result<(), MemError> {
        let storage = Arc::new(MemoryStorage::new());
        let kv = EvifKVStore::new(storage);

        kv.set("key1", "value1").await?;
        let value = kv.get("key1").await?;
        assert_eq!(value, Some("value1".to_string()));

        let exists = kv.exists("key1").await?;
        assert!(exists);

        Ok(())
    }
}
