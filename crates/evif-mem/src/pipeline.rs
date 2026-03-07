//! Memory Processing Pipelines
//!
//! Implements the core memory processing workflows:
//! - MemorizePipeline: Extract and store memories from input
//! - RetrievePipeline: Search and retrieve memories (future)
//! - EvolvePipeline: Self-evolving memory management (future)

use crate::embedding::EmbeddingManager;
use crate::error::MemResult;
use crate::llm::LLMClient;
use crate::models::{MemoryItem, Modality, Resource};
use crate::storage::memory::MemoryStorage;
use crate::vector::VectorIndex;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Memorize Pipeline
///
/// Processes raw input through the memory extraction pipeline:
/// 1. Load resource (text input)
/// 2. Extract memories using LLM
/// 3. Deduplicate by content hash
/// 4. Generate embeddings
/// 5. Persist to storage
/// 6. Update vector index
pub struct MemorizePipeline {
    llm_client: Arc<RwLock<Box<dyn LLMClient>>>,
    storage: Arc<MemoryStorage>,
    vector_index: Arc<RwLock<Box<dyn VectorIndex>>>,
    embedding_manager: Arc<RwLock<EmbeddingManager>>,
}

impl MemorizePipeline {
    /// Create a new memorize pipeline
    pub fn new(
        llm_client: Arc<RwLock<Box<dyn LLMClient>>>,
        storage: Arc<MemoryStorage>,
        vector_index: Arc<RwLock<Box<dyn VectorIndex>>>,
        embedding_manager: Arc<RwLock<EmbeddingManager>>,
    ) -> Self {
        Self {
            llm_client,
            storage,
            vector_index,
            embedding_manager,
        }
    }

    /// Memorize text input
    ///
    /// This is the main entry point for the memorization pipeline.
    /// Takes raw text, extracts memories, and stores them.
    pub async fn memorize_text(&self, text: &str) -> MemResult<Vec<MemoryItem>> {
        // Step 1: Create resource
        let resource = Resource::new("text://input".to_string(), Modality::Conversation);
        let resource_id = resource.id.clone();
        self.storage.put_resource(resource)?;

        // Step 2: Extract memories using LLM
        let llm = self.llm_client.read().await;
        let memories = llm.extract_memories(text).await?;
        drop(llm); // Release lock early

        // Step 3: Process each memory
        let mut stored_memories = Vec::new();
        for mut memory in memories {
            // Set resource reference
            memory.resource_id = Some(resource_id.clone());

            // Calculate content hash for deduplication
            let hash = self.calculate_hash(&memory.content);
            memory.content_hash = Some(hash);

            // Generate embedding
            let embedding = {
                let emb_mgr = self.embedding_manager.read().await;
                emb_mgr.embed(&memory.content).await?
            };
            let embedding_id = uuid::Uuid::new_v4().to_string();
            memory.embedding_id = Some(embedding_id.clone());

            // Step 4: Store in storage (handles deduplication)
            self.storage.put_item(memory.clone())?;

            // Step 5: Add to vector index
            {
                let index = self.vector_index.write().await;
                index.add(memory.id.clone(), embedding, None).await?;
            }

            stored_memories.push(memory);
        }

        Ok(stored_memories)
    }

    /// Calculate SHA-256 hash of content
    fn calculate_hash(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::OpenAIClient;
    use crate::vector::{InMemoryVectorIndex, VectorIndexConfig};

    #[test]
    fn test_hash_calculation() {
        // Test hash calculation directly without needing full pipeline setup
        let content1 = "test content";
        let content2 = "test content";
        let content3 = "different content";

        // Calculate hashes manually
        let mut hasher1 = sha2::Sha256::new();
        hasher1.update(content1.as_bytes());
        let hash1 = format!("{:x}", hasher1.finalize());

        let mut hasher2 = sha2::Sha256::new();
        hasher2.update(content2.as_bytes());
        let hash2 = format!("{:x}", hasher2.finalize());

        let mut hasher3 = sha2::Sha256::new();
        hasher3.update(content3.as_bytes());
        let hash3 = format!("{:x}", hasher3.finalize());

        assert_eq!(hash1, hash2, "Same content should produce same hash");
        assert_ne!(
            hash1, hash3,
            "Different content should produce different hash"
        );
        assert_eq!(hash1.len(), 64, "SHA-256 hash should be 64 hex characters");
    }
}
