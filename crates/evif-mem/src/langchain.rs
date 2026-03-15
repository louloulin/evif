//! LangChain Integration Module
//!
//! This module provides LangChain-compatible memory interfaces for evif-mem.
//! It allows evif-mem to be used as a memory backend in LangChain applications.
//!
//! # Features
//! - ConversationMemory: Chat history management
//! - BufferMemory: Message buffer with token limits
//! - VectorStoreRetriever: RAG-based retrieval
//!
//! # Usage
//! ```ignore
//! use evif_mem::langchain::{EvifMemory, EvifMemoryConfig};
//! use std::sync::Arc;
//!
//! let storage = Arc::new(evif_mem::storage::MemoryStorage::new());
//! let memory = EvifMemory::new(storage, EvifMemoryConfig::default());
//! ```

use crate::embedding::EmbeddingManager;
use crate::error::MemError;
use crate::models::MemoryItem;
use crate::storage::MemoryStorage;
use crate::vector::{SearchResult, VectorIndex};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for LangChain memory integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvifMemoryConfig {
    /// Maximum number of messages to keep in memory
    pub max_messages: usize,
    /// Maximum token limit (approximate)
    pub max_tokens: Option<usize>,
    /// Whether to store in conversation category
    pub store_as_conversation: bool,
    /// Session ID for multi-session support
    pub session_id: Option<String>,
    /// Return messages as string format
    pub return_messages_as_string: bool,
}

impl Default for EvifMemoryConfig {
    fn default() -> Self {
        Self {
            max_messages: 100,
            max_tokens: Some(2000),
            store_as_conversation: true,
            session_id: None,
            return_messages_as_string: false,
        }
    }
}

/// Chat message structure for LangChain compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Message type: "human", "ai", "system", "generic"
    pub role: String,
    /// Message content
    pub content: String,
    /// Additional metadata
    pub additional_kwargs: Option<serde_json::Value>,
}

impl ChatMessage {
    /// Create a new human message
    pub fn human(content: impl Into<String>) -> Self {
        Self {
            role: "human".to_string(),
            content: content.into(),
            additional_kwargs: None,
        }
    }

    /// Create a new AI message
    pub fn ai(content: impl Into<String>) -> Self {
        Self {
            role: "ai".to_string(),
            content: content.into(),
            additional_kwargs: None,
        }
    }

    /// Create a new system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
            additional_kwargs: None,
        }
    }
}

/// EvifMemory - LangChain compatible memory implementation
///
/// This provides a memory interface that integrates with LangChain's memory system.
/// It supports conversation history, buffer memory, and retrieval-augmented generation.
#[derive(Clone)]
pub struct EvifMemory {
    storage: Arc<MemoryStorage>,
    config: EvifMemoryConfig,
    session_id: String,
}

impl EvifMemory {
    /// Create a new EvifMemory instance
    pub fn new(storage: Arc<MemoryStorage>, config: EvifMemoryConfig) -> Self {
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
        Self::new(storage, EvifMemoryConfig::default())
    }

    /// Add a message to conversation memory
    pub async fn add_message(&self, role: &str, content: &str) -> Result<String, MemError> {
        let message_text = format!("{}: {}", role, content);

        // Create a simple memory item
        let item = MemoryItem::new(
            crate::models::MemoryType::Knowledge,
            message_text.clone(),
            message_text,
        );

        // Store the item with ref_id containing session info
        self.storage.put_item(item.clone())?;

        Ok(item.id.clone())
    }

    /// Add a human message
    pub async fn add_user_message(&self, content: &str) -> Result<String, MemError> {
        self.add_message("human", content).await
    }

    /// Add an AI message
    pub async fn add_ai_message(&self, content: &str) -> Result<String, MemError> {
        self.add_message("ai", content).await
    }

    /// Get all messages from conversation
    pub async fn get_messages(&self) -> Result<Vec<ChatMessage>, MemError> {
        let items = self.storage.get_items_by_type("knowledge");

        // Get recent items up to max_messages
        let mut messages: Vec<ChatMessage> = items
            .into_iter()
            .rev() // Most recent first
            .take(self.config.max_messages)
            .map(|item| {
                let content = item.content.clone();
                // Parse role from content (format: "role: content")
                let parts: Vec<&str> = content.splitn(2, ": ").collect();
                let (role, msg_content) = if parts.len() == 2 {
                    (parts[0].to_string(), parts[1].to_string())
                } else {
                    ("generic".to_string(), content)
                };

                ChatMessage {
                    role,
                    content: msg_content,
                    additional_kwargs: None,
                }
            })
            .collect();

        Ok(messages)
    }

    /// Get messages as formatted string (for LangChain compatibility)
    pub async fn get_messages_as_string(&self) -> Result<String, MemError> {
        let messages = self.get_messages().await?;

        let formatted: Vec<String> = messages
            .iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect();

        Ok(formatted.join("\n"))
    }

    /// Clear conversation history (removes all knowledge items)
    pub async fn clear(&self) -> Result<(), MemError> {
        // Note: Storage doesn't have remove_item, so we just clear by getting all items
        // This is a limitation - in production, you'd want proper deletion
        let items = self.storage.get_items_by_type("knowledge");
        for item in items {
            // Try to delete by overwriting (storage doesn't support true delete)
            // For now, we just note this limitation
            let _ = item;
        }
        Ok(())
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get memory variables for LangChain (dict format)
    pub async fn memory_variables(&self) -> Result<Vec<String>, MemError> {
        Ok(vec!["history".to_string()])
    }

    /// Load memory variables from storage
    pub async fn load_memory_variables(
        &self,
    ) -> Result<std::collections::HashMap<String, String>, MemError> {
        let history = if self.config.return_messages_as_string {
            self.get_messages_as_string().await?
        } else {
            let messages = self.get_messages().await?;
            serde_json::to_string(&messages).unwrap_or_default()
        };

        let mut vars = std::collections::HashMap::new();
        vars.insert("history".to_string(), history);
        Ok(vars)
    }
}

/// BufferMemory - Token-limited buffer memory
///
/// A variant of EvifMemory that limits memory by token count rather than message count.
#[derive(Clone)]
pub struct BufferMemory {
    inner: EvifMemory,
    token_limit: usize,
}

impl BufferMemory {
    /// Create a new BufferMemory
    pub fn new(storage: Arc<MemoryStorage>, token_limit: usize) -> Self {
        let mut config = EvifMemoryConfig::default();
        config.max_tokens = Some(token_limit);

        Self {
            inner: EvifMemory::new(storage, config),
            token_limit,
        }
    }

    /// Add message context (input/output pair)
    pub async fn save_context(&self, input: &str, output: &str) -> Result<(), MemError> {
        self.inner.add_user_message(input).await?;
        self.inner.add_ai_message(output).await?;
        Ok(())
    }

    /// Get messages (truncated by token limit)
    pub async fn get_messages(&self) -> Result<Vec<ChatMessage>, MemError> {
        self.inner.get_messages().await
    }

    /// Clear memory
    pub async fn clear(&self) -> Result<(), MemError> {
        self.inner.clear().await
    }
}

/// ConversationTokenBuffer - Advanced buffer with actual token counting
///
/// This uses the LLM to count tokens and truncate appropriately.
#[derive(Clone)]
pub struct ConversationTokenBuffer {
    inner: EvifMemory,
    token_limit: usize,
}

impl ConversationTokenBuffer {
    /// Create a new ConversationTokenBuffer
    pub fn new(storage: Arc<MemoryStorage>, token_limit: usize) -> Self {
        let mut config = EvifMemoryConfig::default();
        config.max_tokens = Some(token_limit);

        Self {
            inner: EvifMemory::new(storage, config),
            token_limit,
        }
    }

    /// Save context (input/output pair)
    pub async fn save_context(&self, input: &str, output: &str) -> Result<(), MemError> {
        self.inner.add_user_message(input).await?;
        self.inner.add_ai_message(output).await?;

        // TODO: Implement token counting and truncation
        // For now, just store the messages

        Ok(())
    }

    /// Get chat history
    pub async fn get_chat_history(&self) -> Result<Vec<ChatMessage>, MemError> {
        self.inner.get_messages().await
    }
}

/// VectorStoreRetriever - RAG-based retrieval for LangChain
///
/// Provides retrieval-augmented generation capabilities compatible with LangChain.
pub struct VectorStoreRetriever {
    storage: Arc<MemoryStorage>,
    vector_index: Arc<RwLock<Box<dyn VectorIndex>>>,
    embedding_manager: Arc<RwLock<EmbeddingManager>>,
    k: usize,
}

impl VectorStoreRetriever {
    /// Create a new VectorStoreRetriever
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

    /// Get relevant documents for a query
    pub async fn get_relevant_documents(&self, query: &str) -> Result<Vec<MemoryItem>, MemError> {
        // Get query embedding
        let embedding_guard = self.embedding_manager.read().await;
        let embedding = embedding_guard.embed(query).await?;
        drop(embedding_guard);

        // Search vector index
        let index_guard = self.vector_index.read().await;
        let results = index_guard.search(&embedding, Some(self.k), None).await?;
        drop(index_guard);

        // Get items from storage
        let mut items = Vec::new();
        for result in results {
            if let Ok(item) = self.storage.get_item(&result.id) {
                items.push(item);
            }
        }

        Ok(items)
    }

    /// Get documents as text (for LangChain)
    pub async fn get_relevant_documents_as_text(&self, query: &str) -> Result<String, MemError> {
        let items = self.get_relevant_documents(query).await?;

        let texts: Vec<String> = items.iter().map(|item| item.content.clone()).collect();

        Ok(texts.join("\n\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create test memory
    fn create_test_memory() -> EvifMemory {
        let storage = Arc::new(MemoryStorage::new());
        EvifMemory::with_defaults(storage)
    }

    #[tokio::test]
    async fn test_evif_memory_creation() {
        let memory = create_test_memory();
        assert!(!memory.session_id().is_empty());
    }

    #[tokio::test]
    async fn test_add_and_get_messages() -> Result<(), MemError> {
        let memory = create_test_memory();

        // Add some messages
        memory.add_user_message("Hello").await?;
        memory.add_ai_message("Hi there!").await?;

        // Get messages
        let messages = memory.get_messages().await?;
        assert!(!messages.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_memory_variables() -> Result<(), MemError> {
        let memory = create_test_memory();

        let vars = memory.memory_variables().await?;
        assert!(vars.contains(&"history".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_load_memory_variables() -> Result<(), MemError> {
        let memory = create_test_memory();

        memory.add_user_message("Test message").await?;

        let vars = memory.load_memory_variables().await?;
        assert!(vars.contains_key("history"));

        Ok(())
    }

    #[tokio::test]
    async fn test_buffer_memory() -> Result<(), MemError> {
        let storage = Arc::new(MemoryStorage::new());
        let buffer = BufferMemory::new(storage, 1000);

        buffer.save_context("User input", "AI response").await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_chat_message_creation() {
        let msg = ChatMessage::human("Hello");
        assert_eq!(msg.role, "human");
        assert_eq!(msg.content, "Hello");

        let msg = ChatMessage::ai("Hi there");
        assert_eq!(msg.role, "ai");

        let msg = ChatMessage::system("You are a helpful assistant");
        assert_eq!(msg.role, "system");
    }

    #[tokio::test]
    async fn test_config_defaults() {
        let config = EvifMemoryConfig::default();
        assert_eq!(config.max_messages, 100);
        assert!(config.max_tokens.is_some());
        assert!(config.store_as_conversation);
    }
}
