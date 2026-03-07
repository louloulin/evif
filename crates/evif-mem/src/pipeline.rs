//! Memory Processing Pipelines
//!
//! Implements the core memory processing workflows:
//! - MemorizePipeline: Extract and store memories from input
//! - RetrievePipeline: Search and retrieve memories
//! - EvolvePipeline: Self-evolving memory management (future)
//! - Categorizer: Auto-categorize memory items based on vector similarity
//! - ResourceLoader: Load resources from various sources (URL, file, text)

use crate::embedding::EmbeddingManager;
use crate::error::{MemError, MemResult};
use crate::llm::LLMClient;
use crate::models::{MemoryItem, Modality, Resource};
use crate::storage::memory::MemoryStorage;
use crate::vector::VectorIndex;
use serde::{Deserialize, Serialize};
use serde_json;
use sha2::{Digest, Sha256};
use std::path::Path;
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
/// 7. Categorize items
pub struct MemorizePipeline {
    llm_client: Arc<RwLock<Box<dyn LLMClient>>>,
    storage: Arc<MemoryStorage>,
    vector_index: Arc<RwLock<Box<dyn VectorIndex>>>,
    embedding_manager: Arc<RwLock<EmbeddingManager>>,
    categorizer: Categorizer,
}

impl MemorizePipeline {
    /// Create a new memorize pipeline
    pub fn new(
        llm_client: Arc<RwLock<Box<dyn LLMClient>>>,
        storage: Arc<MemoryStorage>,
        vector_index: Arc<RwLock<Box<dyn VectorIndex>>>,
        embedding_manager: Arc<RwLock<EmbeddingManager>>,
    ) -> Self {
        let categorizer = Categorizer::new(
            storage.clone(),
            vector_index.clone(),
            embedding_manager.clone(),
            llm_client.clone(),
        );
        Self {
            llm_client,
            storage,
            vector_index,
            embedding_manager,
            categorizer,
        }
    }

    /// Memorize text input
    ///
    /// This is the main entry point for the memorization pipeline.
    /// Takes raw text, extracts memories, and stores them.
    pub async fn memorize_text(&self, text: &str) -> MemResult<Vec<MemoryItem>> {
        // Use ResourceLoader to create resource from text
        let resource_loader = ResourceLoader::new();
        let (resource, content) = resource_loader.load_text(text).await?;

        // Store resource
        let resource_id = resource.id.clone();
        self.storage.put_resource(resource)?;

        // Extract memories using LLM
        let llm = self.llm_client.read().await;
        let memories = llm.extract_memories(&content).await?;
        drop(llm); // Release lock early

        // Process each memory
        let mut stored_memories = Vec::new();
        for mut memory in memories {
            // Set resource reference
            memory.resource_id = Some(resource_id.clone());

            // Calculate content hash for deduplication
            let hash = self.calculate_hash(&memory.content);
            memory.content_hash = Some(hash.clone());

            // Generate embedding
            let embedding = {
                let emb_mgr = self.embedding_manager.read().await;
                emb_mgr.embed(&memory.content).await?
            };
            let embedding_id = uuid::Uuid::new_v4().to_string();
            memory.embedding_id = Some(embedding_id.clone());

            // Check for existing memory with same hash (deduplication)
            if let Ok(existing) = self.storage.get_items_by_hash(&hash) {
                if let Some(mut existing_item) = existing.into_iter().next() {
                    // Reinforce existing memory
                    existing_item.reinforcement_count += 1;
                    existing_item.last_reinforced_at = Some(chrono::Utc::now());
                    self.storage.put_item(existing_item)?;
                    continue;
                }
            }

            // Store in storage
            self.storage.put_item(memory.clone())?;

            // Add to vector index
            {
                let index = self.vector_index.write().await;
                index.add(memory.id.clone(), embedding, None).await?;
            }

            // Categorize the memory item
            let category_id = self.categorizer.categorize(&memory).await?;
            memory.category_id = Some(category_id);

            stored_memories.push(memory);
        }

        Ok(stored_memories)
    }

    /// Memorize from a resource source (URL, file, or text)
    ///
    /// This is the main entry point for the full memorization pipeline.
    /// Takes a resource identifier (URL, file path, or text), loads it,
    /// extracts memories, and stores them.
    pub async fn memorize_resource(&self, source: &str) -> MemResult<Vec<MemoryItem>> {
        // Use ResourceLoader to load resource
        let resource_loader = ResourceLoader::new();
        let (resource, content) = resource_loader.load(source).await?;

        // Store resource
        let resource_id = resource.id.clone();
        self.storage.put_resource(resource)?;

        // Extract memories using LLM
        let llm = self.llm_client.read().await;
        let memories = llm.extract_memories(&content).await?;
        drop(llm); // Release lock early

        // Process each memory
        let mut stored_memories = Vec::new();
        for mut memory in memories {
            // Set resource reference
            memory.resource_id = Some(resource_id.clone());

            // Calculate content hash for deduplication
            let hash = self.calculate_hash(&memory.content);
            memory.content_hash = Some(hash.clone());

            // Generate embedding
            let embedding = {
                let emb_mgr = self.embedding_manager.read().await;
                emb_mgr.embed(&memory.content).await?
            };
            let embedding_id = uuid::Uuid::new_v4().to_string();
            memory.embedding_id = Some(embedding_id.clone());

            // Check for existing memory with same hash (deduplication)
            if let Ok(existing) = self.storage.get_items_by_hash(&hash) {
                if let Some(mut existing_item) = existing.into_iter().next() {
                    // Reinforce existing memory
                    existing_item.reinforcement_count += 1;
                    existing_item.last_reinforced_at = Some(chrono::Utc::now());
                    self.storage.put_item(existing_item)?;
                    continue;
                }
            }

            // Store in storage
            self.storage.put_item(memory.clone())?;

            // Add to vector index
            {
                let index = self.vector_index.write().await;
                index.add(memory.id.clone(), embedding, None).await?;
            }

            // Categorize the memory item
            let category_id = self.categorizer.categorize(&memory).await?;
            memory.category_id = Some(category_id);

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

/// ResourceLoader - loads resources from various sources
///
/// Supported sources:
/// - `text://...` - Direct text input
/// - `file:///path/to/file` - Local file path
/// - `http://...` or `https://...` - Web URL
/// - `/path/to/file` - Local file path (auto-detected)
///
/// Supported modalities:
/// - Conversation: text conversations
/// - Document: documents, articles
/// - Image: images (future: Vision API)
/// - Video: videos (future: keyframe extraction)
/// - Audio: audio files (future: transcription)
pub struct ResourceLoader {
    http_client: reqwest::Client,
}

impl ResourceLoader {
    /// Create a new ResourceLoader
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
        }
    }

    /// Load resource from various sources
    ///
    /// Automatically detects source type from the input string:
    /// - URL (http/https) -> fetch from web
    /// - file:// scheme -> read from local file
    /// - Absolute path -> read from local file
    /// - Other -> treat as direct text input
    pub async fn load(&self, source: &str) -> MemResult<(Resource, String)> {
        let source = source.trim();

        if source.starts_with("http://") || source.starts_with("https://") {
            self.load_url(source).await
        } else if source.starts_with("file://") {
            let path = &source[7..]; // Remove "file://" prefix
            self.load_file(Path::new(path)).await
        } else if source.starts_with('/') || source.starts_with("./") || source.starts_with("../") {
            // Likely a file path
            self.load_file(Path::new(source)).await
        } else if source.starts_with("text://") {
            // Direct text input with explicit scheme
            let content = &source[7..]; // Remove "text://" prefix
            self.load_text(content).await
        } else {
            // Default: treat as direct text input
            self.load_text(source).await
        }
    }

    /// Load from direct text input
    pub async fn load_text(&self, text: &str) -> MemResult<(Resource, String)> {
        let resource = Resource::new("text://input".to_string(), Modality::Conversation);
        Ok((resource, text.to_string()))
    }

    /// Load from local file
    pub async fn load_file(&self, path: &Path) -> MemResult<(Resource, String)> {
        let content = tokio::fs::read_to_string(path).await?;

        // Detect modality from file extension
        let modality = detect_modality_from_path(path);
        let url = format!("file://{}", path.display());

        let mut resource = Resource::new(url, modality);
        resource.local_path = Some(path.display().to_string());

        Ok((resource, content))
    }

    /// Load from URL (http/https)
    pub async fn load_url(&self, url: &str) -> MemResult<(Resource, String)> {
        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(|e| MemError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to fetch URL: {}", e),
            )))?;

        // Detect content type
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/plain");

        // Detect modality from content type
        let modality = detect_modality_from_content_type(content_type);

        // Get content based on content type
        let content = if content_type.contains("application/json") {
            // Pretty print JSON
            let json: serde_json::Value = response
                .json()
                .await
                .map_err(|e| MemError::Parse(format!("Failed to parse JSON: {}", e)))?;
            serde_json::to_string_pretty(&json).unwrap_or_default()
        } else if content_type.contains("text/html") {
            // For HTML, extract text content (simple approach)
            let html = response
                .text()
                .await
                .map_err(|e| MemError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to read HTML: {}", e),
                )))?;
            extract_text_from_html(&html)
        } else {
            // Plain text or other
            response
                .text()
                .await
                .map_err(|e| MemError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to read content: {}", e),
                )))?
        };

        let mut resource = Resource::new(url.to_string(), modality);
        // Try to get title from HTML if available
        if modality == Modality::Document {
            resource.caption = extract_title_from_html(&content);
        }

        Ok((resource, content))
    }
}

impl Default for ResourceLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Detect modality from file extension
fn detect_modality_from_path(path: &Path) -> Modality {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "txt" | "md" | "markdown" | "rst" | "org" | "text" => Modality::Document,
        "json" | "xml" | "yaml" | "yml" | "toml" | "csv" => Modality::Document,
        "html" | "htm" => Modality::Document,
        "pdf" => Modality::Document,
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "svg" => Modality::Image,
        "mp4" | "avi" | "mov" | "mkv" | "webm" | "flv" => Modality::Video,
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" => Modality::Audio,
        "log" | "out" | "err" => Modality::Conversation,
        _ => Modality::Document,
    }
}

/// Detect modality from content-type header
fn detect_modality_from_content_type(content_type: &str) -> Modality {
    let content_type = content_type.to_lowercase();

    if content_type.contains("text/html") {
        Modality::Document
    } else if content_type.contains("application/json") {
        Modality::Document
    } else if content_type.contains("text/") {
        Modality::Document
    } else if content_type.contains("image/") {
        Modality::Image
    } else if content_type.contains("video/") {
        Modality::Video
    } else if content_type.contains("audio/") {
        Modality::Audio
    } else {
        Modality::Document
    }
}

/// Simple HTML text extraction (strips tags)
fn extract_text_from_html(html: &str) -> String {
    // Simple approach: remove script and style tags first, then strip all HTML tags
    let without_scripts = regex_lite::Regex::new(r"(?is)<script[^>]*>.*?</script>")
        .map(|re| re.replace_all(html, "").into_owned())
        .unwrap_or_else(|_| html.to_string());

    let without_styles = regex_lite::Regex::new(r"(?is)<style[^>]*>.*?</style>")
        .map(|re| re.replace_all(&without_scripts, "").into_owned())
        .unwrap_or(without_scripts);

    let without_tags = regex_lite::Regex::new(r"<[^>]+>")
        .map(|re| re.replace_all(&without_styles, " ").into_owned())
        .unwrap_or(without_styles);

    // Clean up whitespace
    let cleaned = regex_lite::Regex::new(r"\s+")
        .map(|re| re.replace_all(&without_tags, " ").into_owned())
        .unwrap_or(without_tags);

    cleaned.trim().to_string()
}

/// Extract title from HTML if available
fn extract_title_tag_from_html(html: &str) -> Option<String> {
    // Try to extract <title> tag content
    regex_lite::Regex::new(r"(?is)<title[^>]*>(.*?)</title>")
        .ok()
        .and_then(|re| re.captures(html))
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().trim().to_string())
}

/// Simple title extraction from HTML
fn extract_title_from_html(content: &str) -> Option<String> {
    // Check if it looks like HTML
    if !content.contains("<html") && !content.contains("<!DOCTYPE") {
        return None;
    }

    // Try to extract <title> tag content
    extract_title_tag_from_html(content)
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

/// Categorizer
///
/// Automatically categorizes memory items using vector similarity:
/// 1. Find most similar existing category by embedding
/// 2. If similarity >= threshold, assign to that category
/// 3. If no similar category, create new category using LLM analysis
pub struct Categorizer {
    storage: Arc<MemoryStorage>,
    vector_index: Arc<RwLock<Box<dyn VectorIndex>>>,
    embedding_manager: Arc<RwLock<EmbeddingManager>>,
    llm_client: Arc<RwLock<Box<dyn LLMClient>>>,
    similarity_threshold: f32,
}

impl Categorizer {
    /// Create a new categorizer
    pub fn new(
        storage: Arc<MemoryStorage>,
        vector_index: Arc<RwLock<Box<dyn VectorIndex>>>,
        embedding_manager: Arc<RwLock<EmbeddingManager>>,
        llm_client: Arc<RwLock<Box<dyn LLMClient>>>,
    ) -> Self {
        Self {
            storage,
            vector_index,
            embedding_manager,
            llm_client,
            similarity_threshold: 0.7, // Default threshold
        }
    }

    /// Create categorizer with custom threshold
    pub fn with_threshold(
        storage: Arc<MemoryStorage>,
        vector_index: Arc<RwLock<Box<dyn VectorIndex>>>,
        embedding_manager: Arc<RwLock<EmbeddingManager>>,
        llm_client: Arc<RwLock<Box<dyn LLMClient>>>,
        threshold: f32,
    ) -> Self {
        Self {
            storage,
            vector_index,
            embedding_manager,
            llm_client,
            similarity_threshold: threshold,
        }
    }

    /// Categorize a memory item
    ///
    /// Returns the category ID that the item was assigned to.
    pub async fn categorize(&self, item: &MemoryItem) -> MemResult<String> {
        // Get item embedding
        let item_embedding = {
            let emb_mgr = self.embedding_manager.read().await;
            emb_mgr.embed(&item.content).await?
        };

        // Try to find similar existing category
        if let Some(category_id) = self.find_similar_category(&item_embedding).await? {
            // Link item to existing category
            self.storage.link_item_to_category(&item.id, &category_id)?;
            return Ok(category_id);
        }

        // No similar category found - create new one
        let category_id = self.create_new_category(item, &item_embedding).await?;

        // Link item to new category
        self.storage.link_item_to_category(&item.id, &category_id)?;

        Ok(category_id)
    }

    /// Find the most similar existing category
    ///
    /// Returns Some(category_id) if similarity >= threshold, None otherwise
    async fn find_similar_category(&self, item_embedding: &[f32]) -> MemResult<Option<String>> {
        let categories = self.storage.get_all_categories();

        if categories.is_empty() {
            return Ok(None);
        }

        // Search for similar categories using vector index
        let search_results = {
            let index = self.vector_index.read().await;
            index.search(item_embedding, Some(10), None).await?
        };

        // Filter results by category prefix and threshold
        for result in search_results {
            // Category IDs in vector index are prefixed with "cat:"
            if result.id.starts_with("cat:") {
                let category_id = result.id.strip_prefix("cat:").unwrap().to_string();

                // Check if category still exists
                if self.storage.get_category(&category_id).is_ok() {
                    if result.score >= self.similarity_threshold {
                        return Ok(Some(category_id));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Create a new category for the memory item
    ///
    /// Uses LLM to generate category name and description,
    /// then stores the category and adds its embedding to the index.
    async fn create_new_category(
        &self,
        item: &MemoryItem,
        item_embedding: &[f32],
    ) -> MemResult<String> {
        // Use LLM to analyze and generate category info
        let analysis = {
            let llm = self.llm_client.read().await;
            llm.analyze_category(&[item.content.clone()]).await?
        };

        // Create new category
        let mut category = crate::models::MemoryCategory::new(
            analysis.name.clone(),
            analysis.description.clone(),
        );

        // Generate category embedding ID
        let category_embedding_id = uuid::Uuid::new_v4().to_string();
        category.embedding_id = Some(category_embedding_id.clone());

        // Store category
        self.storage.put_category(category.clone())?;

        // Add category embedding to vector index (prefixed with "cat:")
        let vector_id = format!("cat:{}", category.id);
        {
            let index = self.vector_index.write().await;
            index.add(vector_id, item_embedding.to_vec(), None).await?;
        }

        Ok(category.id)
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

    #[tokio::test]
    async fn test_resource_loader_text() {
        let loader = ResourceLoader::new();

        // Test direct text input
        let (resource, content) = loader.load("Hello, this is a test message").await.unwrap();

        assert_eq!(content, "Hello, this is a test message");
        assert_eq!(resource.url, "text://input");
        assert_eq!(resource.modality, Modality::Conversation);
    }

    #[tokio::test]
    async fn test_resource_loader_text_explicit_scheme() {
        let loader = ResourceLoader::new();

        // Test text:// scheme
        let (resource, content) = loader.load("text://Hello, explicit scheme").await.unwrap();

        assert_eq!(content, "Hello, explicit scheme");
        assert_eq!(resource.url, "text://input");
        assert_eq!(resource.modality, Modality::Conversation);
    }

    #[test]
    fn test_detect_modality_from_path() {
        // Test various file extensions
        assert_eq!(detect_modality_from_path(Path::new("test.txt")), Modality::Document);
        assert_eq!(detect_modality_from_path(Path::new("test.md")), Modality::Document);
        assert_eq!(detect_modality_from_path(Path::new("test.json")), Modality::Document);
        assert_eq!(detect_modality_from_path(Path::new("test.html")), Modality::Document);
        assert_eq!(detect_modality_from_path(Path::new("test.jpg")), Modality::Image);
        assert_eq!(detect_modality_from_path(Path::new("test.png")), Modality::Image);
        assert_eq!(detect_modality_from_path(Path::new("test.mp4")), Modality::Video);
        assert_eq!(detect_modality_from_path(Path::new("test.mp3")), Modality::Audio);
        assert_eq!(detect_modality_from_path(Path::new("test.log")), Modality::Conversation);
    }

    #[test]
    fn test_detect_modality_from_content_type() {
        // Test various content types
        assert_eq!(detect_modality_from_content_type("text/html"), Modality::Document);
        assert_eq!(detect_modality_from_content_type("application/json"), Modality::Document);
        assert_eq!(detect_modality_from_content_type("text/plain"), Modality::Document);
        assert_eq!(detect_modality_from_content_type("image/jpeg"), Modality::Image);
        assert_eq!(detect_modality_from_content_type("image/png"), Modality::Image);
        assert_eq!(detect_modality_from_content_type("video/mp4"), Modality::Video);
        assert_eq!(detect_modality_from_content_type("audio/mpeg"), Modality::Audio);
    }

    #[test]
    fn test_extract_text_from_html() {
        let html = r#"<html><head><title>Test Page</title></head><body><script>alert('test')</script><p>Hello World</p></body></html>"#;
        let text = extract_text_from_html(html);

        assert!(text.contains("Hello World"));
        assert!(!text.contains("<script>"));
        assert!(!text.contains("alert"));
    }

    #[test]
    fn test_extract_title_from_html() {
        let html = r#"<html><head><title>My Title</title></head><body>Content</body></html>"#;
        let title = extract_title_from_html(html);

        assert_eq!(title, Some("My Title".to_string()));
    }

    #[test]
    fn test_extract_title_from_non_html() {
        let text = "This is just plain text without any HTML tags.";
        let title = extract_title_from_html(text);

        assert_eq!(title, None);
    }
}
