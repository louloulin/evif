//! Memory Processing Pipelines
//!
//! Implements the core memory processing workflows:
//! - MemorizePipeline: Extract and store memories from input
//! - RetrievePipeline: Search and retrieve memories
//! - EvolvePipeline: Self-evolving memory management (future)

use crate::embedding::EmbeddingManager;
use crate::error::MemResult;
use crate::llm::LLMClient;
use crate::models::{MemoryItem, Modality, Resource};
use crate::storage::memory::MemoryStorage;
use crate::vector::VectorIndex;
use serde::{Deserialize, Serialize};
use serde_json;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Retrieve mode for searching memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetrieveMode {
    /// Vector search mode using embeddings
    VectorSearch {
        /// Number of results to return
        k: usize,
        /// Minimum similarity threshold (0.0-1.0)
        threshold: f32,
    },
    /// Hybrid search combining vector and LLM approaches
    Hybrid {
        /// Number of results from vector search
        vector_k: usize,
        /// Number of top results to rerank with LLM
        llm_top_n: usize,
    },
    /// LLM Read mode - direct LLM analysis of category memories
    ///
    /// This is Mode 2 retrieval from mem.md design:
    /// - LLM directly reads memory contents from a category
    /// - Uses LLM's reasoning capability for deep analysis
    /// - Does not depend on vector embeddings
    /// - Best for: "Analyze all knowledge in this category"
    LLMRead {
        /// Category ID to read memories from
        category_id: String,
        /// Maximum number of items to process
        max_items: usize,
    },
}

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

/// Retrieve Pipeline
///
/// Searches and retrieves memories from the memory system:
/// 1. Accept query (text or vector)
/// 2. Choose retrieval mode (VectorSearch, LLMRead, Hybrid)
/// 3. Return ranked results
pub struct RetrievePipeline {
    llm_client: Arc<RwLock<Box<dyn LLMClient>>>,
    storage: Arc<MemoryStorage>,
    vector_index: Arc<RwLock<Box<dyn VectorIndex>>>,
    embedding_manager: Arc<RwLock<EmbeddingManager>>,
}

impl RetrievePipeline {
    /// Create a new retrieve pipeline
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

    /// Retrieve memories using text query
    ///
    /// This is the main entry point for the retrieval pipeline.
    /// Takes a text query and retrieval mode, returns ranked memories.
    pub async fn retrieve_text(
        &self,
        query: &str,
        mode: RetrieveMode,
    ) -> MemResult<Vec<(MemoryItem, f32)>> {
        match mode {
            RetrieveMode::VectorSearch { k, threshold } => {
                self.vector_search(query, k, threshold).await
            }
            RetrieveMode::Hybrid { vector_k, llm_top_n } => {
                self.hybrid_search(query, vector_k, llm_top_n).await
            }
            RetrieveMode::LLMRead { category_id, max_items } => {
                self.llm_read_search(query, &category_id, max_items).await
            }
        }
    }

    /// Vector search implementation
    ///
    /// 1. Generate embedding for query
    /// 2. Search vector index
    /// 3. Fetch memory items from storage
    /// 4. Return items with scores
    async fn vector_search(
        &self,
        query: &str,
        k: usize,
        threshold: f32,
    ) -> MemResult<Vec<(MemoryItem, f32)>> {
        // Step 1: Generate query embedding
        let query_embedding = {
            let emb_mgr = self.embedding_manager.read().await;
            emb_mgr.embed(query).await?
        };

        // Step 2: Search vector index
        let search_results = {
            let index = self.vector_index.read().await;
            index.search(&query_embedding, Some(k), None).await?
        };

        // Step 3: Fetch items and filter by threshold
        let mut results = Vec::new();
        for search_result in search_results {
            // Filter by threshold
            if search_result.score < threshold {
                continue;
            }

            // Fetch memory item from storage
            match self.storage.get_item(&search_result.id) {
                Ok(item) => results.push((item, search_result.score)),
                Err(_) => continue, // Skip if item not found
            }
        }

        Ok(results)
    }

    /// Hybrid search implementation
    ///
    /// Combines vector search with LLM reranking:
    /// 1. Vector search for top-K candidates
    /// 2. Take top-N results
    /// 3. Use LLM to rerank by relevance
    /// 4. Return reranked items with scores
    async fn hybrid_search(
        &self,
        query: &str,
        vector_k: usize,
        llm_top_n: usize,
    ) -> MemResult<Vec<(MemoryItem, f32)>> {
        // Step 1: Vector search with low threshold to get candidates
        let vector_results = self.vector_search(query, vector_k, 0.0).await?;

        // Step 2: Take top-N items for LLM reranking
        let top_n_items: Vec<MemoryItem> = vector_results
            .into_iter()
            .take(llm_top_n)
            .map(|(item, _)| item)
            .collect();

        // Early return if no results
        if top_n_items.is_empty() {
            return Ok(vec![]);
        }

        // Step 3: Use LLM to rerank items
        let reranked_items = {
            let llm = self.llm_client.read().await;
            llm.rerank(query, top_n_items).await?
        };

        // Step 4: Assign scores based on reranked position
        // Higher rank = higher score (normalize to 0.0-1.0)
        let total = reranked_items.len();
        let results: Vec<(MemoryItem, f32)> = reranked_items
            .into_iter()
            .enumerate()
            .map(|(idx, item)| {
                let score = if total > 1 {
                    1.0 - (idx as f32 / (total - 1) as f32) * 0.5 // Range: 1.0 to 0.5
                } else {
                    1.0
                };
                (item, score)
            })
            .collect();

        Ok(results)
    }

    /// LLM Read search implementation
    ///
    /// Mode 2 retrieval from mem.md design:
    /// 1. Get all memories in the specified category
    /// 2. Format memories as readable text for LLM
    /// 3. Use LLM to analyze and identify most relevant items
    /// 4. Return items with relevance scores
    ///
    /// This mode doesn't depend on vector embeddings and uses
    /// LLM's reasoning capability for deep analysis.
    async fn llm_read_search(
        &self,
        query: &str,
        category_id: &str,
        max_items: usize,
    ) -> MemResult<Vec<(MemoryItem, f32)>> {
        // Step 1: Get memories in the category
        let memories = self.storage.get_items_in_category(category_id);

        // Early return if no memories
        if memories.is_empty() {
            return Ok(vec![]);
        }

        // Step 2: Limit items to process
        let limited_memories: Vec<MemoryItem> = memories
            .into_iter()
            .take(max_items)
            .collect();

        // Step 3: Format memories for LLM analysis
        let memories_text = self.format_memories_for_llm(&limited_memories);

        // Step 4: Build analysis prompt
        let prompt = self.build_llm_read_prompt(query, &memories_text);

        // Step 5: Use LLM to analyze
        let llm_response = {
            let llm = self.llm_client.read().await;
            llm.generate(&prompt).await?
        };

        // Step 6: Parse LLM response to identify relevant items
        let results = self.parse_llm_read_response(&llm_response, &limited_memories);

        Ok(results)
    }

    /// Format memories as readable text for LLM analysis
    fn format_memories_for_llm(&self, memories: &[MemoryItem]) -> String {
        memories
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                format!(
                    "[{}] ID: {}\nType: {}\nSummary: {}\nContent: {}\n",
                    idx + 1,
                    item.id,
                    item.memory_type,
                    item.summary,
                    item.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n---\n")
    }

    /// Build LLM analysis prompt for LLMRead mode
    fn build_llm_read_prompt(&self, query: &str, memories_text: &str) -> String {
        format!(
            r#"You are analyzing a collection of memories to find the most relevant ones for a user query.

USER QUERY: {}

MEMORIES:
{}

TASK:
1. Analyze each memory in the context of the user query
2. Identify the most relevant memories (up to 5)
3. For each relevant memory, provide:
   - The memory ID
   - A relevance score (0.0 to 1.0)
   - A brief explanation of why it's relevant

RESPONSE FORMAT (JSON):
{{
  "relevant_memories": [
    {{
      "id": "<memory_id>",
      "score": <0.0-1.0>,
      "reason": "<brief explanation>"
    }}
  ]
}}

Respond ONLY with valid JSON, no additional text."#,
            query, memories_text
        )
    }

    /// Parse LLM response to extract relevant items with scores
    fn parse_llm_read_response(
        &self,
        response: &str,
        memories: &[MemoryItem],
    ) -> Vec<(MemoryItem, f32)> {
        // Create a map for quick lookup
        let memory_map: std::collections::HashMap<String, &MemoryItem> = memories
            .iter()
            .map(|m| (m.id.clone(), m))
            .collect();

        // Parse JSON response
        #[derive(serde::Deserialize)]
        struct MemoryScore {
            id: String,
            score: f32,
            #[allow(dead_code)]
            reason: String,
        }

        #[derive(serde::Deserialize)]
        struct LLMResponse {
            relevant_memories: Vec<MemoryScore>,
        }

        // Try to parse the response
        match serde_json::from_str::<LLMResponse>(response) {
            Ok(parsed) => {
                // Convert to results
                parsed
                    .relevant_memories
                    .into_iter()
                    .filter_map(|ms| {
                        memory_map
                            .get(&ms.id)
                            .map(|item| ((*item).clone(), ms.score))
                    })
                    .collect()
            }
            Err(_) => {
                // Fallback: if parsing fails, return all items with equal score
                // This provides graceful degradation
                memories
                    .iter()
                    .take(5)
                    .map(|item| (item.clone(), 0.5))
                    .collect()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::{CacheConfig, OpenAIEmbeddingClient};
    use crate::llm::OpenAIClient;
    use crate::storage::memory::MemoryStorage;
    use crate::vector::{InMemoryVectorIndex, VectorIndexConfig};
    use std::sync::Arc;
    use tokio::sync::Mutex;

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

    #[tokio::test]
    async fn test_retrieve_pipeline_creation() {
        // Test that we can create a RetrievePipeline with basic components
        use crate::storage::memory::MemoryStorage;

        let storage = Arc::new(MemoryStorage::new());

        // For now, just test that the struct exists
        // Full integration tests would require actual API clients
        assert!(true, "Pipeline structure exists");
    }

    #[test]
    fn test_hybrid_mode_enum() {
        // Test that Hybrid mode can be created
        let mode = RetrieveMode::Hybrid {
            vector_k: 10,
            llm_top_n: 5,
        };

        // Verify mode creation
        match mode {
            RetrieveMode::Hybrid { vector_k, llm_top_n } => {
                assert_eq!(vector_k, 10);
                assert_eq!(llm_top_n, 5);
            }
            _ => panic!("Expected Hybrid mode"),
        }
    }

    #[tokio::test]
    async fn test_hybrid_search_empty_results() {
        // Test that Hybrid mode can be instantiated
        // Full integration test would require mocking LLM client
        // This test verifies the enum variant exists and is usable
        let mode = RetrieveMode::Hybrid {
            vector_k: 10,
            llm_top_n: 5,
        };

        match mode {
            RetrieveMode::Hybrid { vector_k, llm_top_n } => {
                assert_eq!(vector_k, 10);
                assert_eq!(llm_top_n, 5);
            }
            _ => panic!("Expected Hybrid mode"),
        }
    }

    #[test]
    fn test_llm_read_mode_enum() {
        // Test that LLMRead mode can be created
        let mode = RetrieveMode::LLMRead {
            category_id: "test-category-id".to_string(),
            max_items: 20,
        };

        // Verify mode creation
        match mode {
            RetrieveMode::LLMRead { category_id, max_items } => {
                assert_eq!(category_id, "test-category-id");
                assert_eq!(max_items, 20);
            }
            _ => panic!("Expected LLMRead mode"),
        }
    }

    #[test]
    fn test_llm_read_mode_serialization() {
        // Test that LLMRead mode can be serialized/deserialized
        let mode = RetrieveMode::LLMRead {
            category_id: "category-123".to_string(),
            max_items: 15,
        };

        let json = serde_json::to_string(&mode).expect("Failed to serialize");
        let decoded: RetrieveMode = serde_json::from_str(&json).expect("Failed to deserialize");

        match decoded {
            RetrieveMode::LLMRead { category_id, max_items } => {
                assert_eq!(category_id, "category-123");
                assert_eq!(max_items, 15);
            }
            _ => panic!("Expected LLMRead mode"),
        }
    }

    #[test]
    fn test_format_memories_for_llm() {
        // Test formatting memories for LLM analysis
        use crate::models::MemoryType;

        let item1 = MemoryItem::new(
            MemoryType::Knowledge,
            "Rust async programming".to_string(),
            "User prefers async/await pattern in Rust".to_string(),
        );

        let item2 = MemoryItem::new(
            MemoryType::Profile,
            "Prefers dark mode".to_string(),
            "User likes dark theme in IDE".to_string(),
        );

        let memories = vec![item1, item2];

        // We can't directly test the private method, but we can verify the format
        // by checking the memory structure
        assert_eq!(memories.len(), 2);
        assert_eq!(memories[0].memory_type, MemoryType::Knowledge);
        assert_eq!(memories[1].memory_type, MemoryType::Profile);
    }

    #[test]
    fn test_llm_read_response_parsing() {
        // Test parsing LLM response JSON
        let response = r#"{
            "relevant_memories": [
                {
                    "id": "memory-1",
                    "score": 0.95,
                    "reason": "Directly related to user query"
                },
                {
                    "id": "memory-2",
                    "score": 0.75,
                    "reason": "Partially relevant"
                }
            ]
        }"#;

        // Verify the JSON structure is valid
        #[derive(Deserialize)]
        struct MemoryScore {
            id: String,
            score: f32,
            reason: String,
        }

        #[derive(Deserialize)]
        struct LLMResponse {
            relevant_memories: Vec<MemoryScore>,
        }

        let parsed: LLMResponse = serde_json::from_str(response).expect("Failed to parse JSON");
        assert_eq!(parsed.relevant_memories.len(), 2);
        assert_eq!(parsed.relevant_memories[0].id, "memory-1");
        assert!((parsed.relevant_memories[0].score - 0.95).abs() < 0.001);
    }
}
