//! Cost Optimization Module (Phase 1.5.4)
//!
//! Implements LLM cost reduction strategies:
//! - LRU cache for query responses
//! - Batch processing for multiple queries
//! - Similar query detection to avoid redundant LLM calls

use crate::error::MemError;
use crate::vector::VectorIndex;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Result type for cost optimization operations
pub type CostResult<T> = Result<T, MemError>;

/// Configuration for cost optimizer
#[derive(Debug, Clone)]
pub struct CostOptimizerConfig {
    /// Maximum cache size (number of entries)
    pub cache_max_size: usize,
    /// Cache TTL in seconds
    pub cache_ttl_secs: i64,
    /// Similarity threshold for query matching (0.0-1.0)
    pub similarity_threshold: f32,
    /// Enable batch processing
    pub enable_batching: bool,
    /// Batch window in milliseconds
    pub batch_window_ms: u64,
    /// Maximum batch size
    pub max_batch_size: usize,
    /// Enable similar query detection
    pub enable_similar_detection: bool,
}

impl Default for CostOptimizerConfig {
    fn default() -> Self {
        Self {
            cache_max_size: 1000,
            cache_ttl_secs: 3600,       // 1 hour
            similarity_threshold: 0.95, // 95% similarity
            enable_batching: true,
            batch_window_ms: 100, // 100ms batch window
            max_batch_size: 10,
            enable_similar_detection: true,
        }
    }
}

/// Cache entry for LLM response
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Cached response
    pub response: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Query hash for lookup
    pub query_hash: String,
    /// Access count
    pub access_count: u64,
    /// Last access timestamp
    pub last_accessed: DateTime<Utc>,
}

impl CacheEntry {
    fn new(response: String, query_hash: String) -> Self {
        let now = Utc::now();
        Self {
            response,
            created_at: now,
            query_hash,
            access_count: 1,
            last_accessed: now,
        }
    }

    fn is_expired(&self, ttl_secs: i64) -> bool {
        let now = Utc::now();
        let age = now.signed_duration_since(self.created_at);
        age.num_seconds() > ttl_secs
    }

    fn touch(&mut self) {
        self.access_count += 1;
        self.last_accessed = Utc::now();
    }
}

/// Batch item for processing
#[derive(Debug, Clone)]
pub struct BatchItem {
    /// Unique ID for this batch item
    pub id: Uuid,
    /// Query to process
    pub query: String,
    /// Query embedding for similarity check
    pub embedding: Option<Vec<f32>>,
    /// Timestamp when added to batch
    pub added_at: DateTime<Utc>,
}

impl BatchItem {
    fn new(query: String, embedding: Option<Vec<f32>>) -> Self {
        Self {
            id: Uuid::new_v4(),
            query,
            embedding,
            added_at: Utc::now(),
        }
    }
}

/// Statistics for cost optimizer
#[derive(Debug, Clone, Default)]
pub struct CostOptimizerStats {
    /// Total LLM calls made
    pub total_llm_calls: u64,
    /// LLM calls saved by cache
    pub cache_hits: u64,
    /// LLM calls saved by batch processing
    pub batch_savings: u64,
    /// LLM calls saved by similar query detection
    pub similar_query_savings: u64,
    /// Total cache entries
    pub cache_entries: usize,
    /// Current batch queue size
    pub batch_queue_size: usize,
    /// Estimated cost saved (in USD)
    pub estimated_cost_saved: f32,
}

/// Cost optimizer for reducing LLM API costs
pub struct CostOptimizer {
    /// Configuration
    config: CostOptimizerConfig,
    /// LRU cache for responses
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// Cache access order for LRU eviction
    cache_order: Arc<RwLock<Vec<String>>>,
    /// Batch queue for pending queries
    batch_queue: Arc<RwLock<Vec<BatchItem>>>,
    /// Statistics
    stats: Arc<RwLock<CostOptimizerStats>>,
    /// Vector index for similarity check
    vector_index: Arc<RwLock<Box<dyn VectorIndex>>>,
}

impl CostOptimizer {
    /// Create a new cost optimizer
    pub fn new(
        config: CostOptimizerConfig,
        vector_index: Arc<RwLock<Box<dyn VectorIndex>>>,
    ) -> Self {
        Self {
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_order: Arc::new(RwLock::new(Vec::new())),
            batch_queue: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(CostOptimizerStats::default())),
            vector_index,
        }
    }

    /// Compute hash for a query
    fn compute_query_hash(query: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(query.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Check if we should call LLM or use cache/batch
    pub async fn should_call_llm(&self, query: &str) -> CostResult<bool> {
        // 1. Check cache
        if self.check_cache(query).await? {
            return Ok(false);
        }

        // 2. Check if can batch
        if self.config.enable_batching && self.can_batch(query).await {
            return Ok(false);
        }

        // 3. Check similar queries
        if self.config.enable_similar_detection && self.find_similar_query(query).await?.is_some() {
            return Ok(false);
        }

        Ok(true)
    }

    /// Check cache for query
    async fn check_cache(&self, query: &str) -> CostResult<bool> {
        let hash = Self::compute_query_hash(query);
        let mut cache = self.cache.write().await;
        let mut order = self.cache_order.write().await;

        if let Some(entry) = cache.get_mut(&hash) {
            // Check if expired
            if entry.is_expired(self.config.cache_ttl_secs) {
                cache.remove(&hash);
                order.retain(|h| h != &hash);
                return Ok(false);
            }

            // Cache hit - update access
            entry.touch();

            // Update LRU order
            order.retain(|h| h != &hash);
            order.push(hash);

            // Update stats
            let mut stats = self.stats.write().await;
            stats.cache_hits += 1;
            stats.estimated_cost_saved += 0.002; // Estimate $0.002 per cached call

            return Ok(true);
        }

        Ok(false)
    }

    /// Get cached response
    pub async fn get_cached_response(&self, query: &str) -> CostResult<Option<String>> {
        let hash = Self::compute_query_hash(query);
        let cache = self.cache.read().await;

        if let Some(entry) = cache.get(&hash) {
            if !entry.is_expired(self.config.cache_ttl_secs) {
                return Ok(Some(entry.response.clone()));
            }
        }

        Ok(None)
    }

    /// Cache a response
    pub async fn cache_response(&self, query: &str, response: String) -> CostResult<()> {
        let hash = Self::compute_query_hash(query);
        let mut cache = self.cache.write().await;
        let mut order = self.cache_order.write().await;

        // Evict if at capacity
        while cache.len() >= self.config.cache_max_size {
            if let Some(old_hash) = order.first().cloned() {
                cache.remove(&old_hash);
                order.remove(0);
            }
        }

        // Add new entry
        let entry = CacheEntry::new(response, hash.clone());
        cache.insert(hash.clone(), entry);
        order.push(hash);

        // Update stats
        let mut stats = self.stats.write().await;
        stats.cache_entries = cache.len();

        Ok(())
    }

    /// Check if query can be batched
    async fn can_batch(&self, _query: &str) -> bool {
        let queue = self.batch_queue.read().await;
        queue.len() < self.config.max_batch_size
    }

    /// Add query to batch queue
    pub async fn add_to_batch(
        &self,
        query: String,
        embedding: Option<Vec<f32>>,
    ) -> CostResult<Uuid> {
        let item = BatchItem::new(query, embedding);
        let id = item.id;

        let mut queue = self.batch_queue.write().await;
        queue.push(item);

        // Update stats
        let mut stats = self.stats.write().await;
        stats.batch_queue_size = queue.len();

        Ok(id)
    }

    /// Process batch and return results
    pub async fn process_batch(&self) -> CostResult<Vec<(Uuid, String)>> {
        let mut queue = self.batch_queue.write().await;
        let items: Vec<BatchItem> = queue.drain(..).collect();

        if items.is_empty() {
            return Ok(vec![]);
        }

        // Update stats
        let mut stats = self.stats.write().await;
        stats.batch_queue_size = 0;
        let saved_calls = items.len().saturating_sub(1);
        stats.batch_savings += saved_calls as u64;
        stats.estimated_cost_saved += saved_calls as f32 * 0.002;

        // Return placeholder results (actual LLM call would happen here)
        // In production, this would call LLM with batched queries
        let results: Vec<(Uuid, String)> = items
            .iter()
            .map(|item| (item.id, format!("Batch processed: {}", item.query)))
            .collect();

        Ok(results)
    }

    /// Find similar query in cache
    async fn find_similar_query(&self, _query: &str) -> CostResult<Option<String>> {
        // Get embedding for query
        let _vector_index = self.vector_index.read().await;

        // For now, return None - actual implementation would:
        // 1. Get embedding for query
        // 2. Search vector index for similar queries
        // 3. Return cached response if similarity > threshold

        Ok(None)
    }

    /// Get statistics
    pub async fn get_stats(&self) -> CostOptimizerStats {
        self.stats.read().await.clone()
    }

    /// Clear cache
    pub async fn clear_cache(&self) -> CostResult<()> {
        let mut cache = self.cache.write().await;
        let mut order = self.cache_order.write().await;

        cache.clear();
        order.clear();

        let mut stats = self.stats.write().await;
        stats.cache_entries = 0;

        Ok(())
    }

    /// Update LLM call count
    pub async fn record_llm_call(&self) {
        let mut stats = self.stats.write().await;
        stats.total_llm_calls += 1;
    }

    /// Calculate estimated cost
    pub fn estimate_cost(tokens: usize, model: &str) -> f32 {
        // Rough cost estimates per 1K tokens
        let cost_per_1k = match model {
            "gpt-4o" => 0.005,
            "gpt-4o-mini" => 0.00015,
            "claude-3-5-sonnet" => 0.003,
            _ => 0.002,
        };

        (tokens as f32 / 1000.0) * cost_per_1k
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_optimizer_config_default() {
        let config = CostOptimizerConfig::default();
        assert_eq!(config.cache_max_size, 1000);
        assert_eq!(config.cache_ttl_secs, 3600);
        assert!((config.similarity_threshold - 0.95).abs() < 0.01);
        assert!(config.enable_batching);
        assert!(config.enable_similar_detection);
    }

    #[test]
    fn test_cache_entry_creation() {
        let entry = CacheEntry::new("response".to_string(), "hash123".to_string());
        assert_eq!(entry.response, "response");
        assert_eq!(entry.query_hash, "hash123");
        assert_eq!(entry.access_count, 1);
        assert!(!entry.is_expired(3600));
    }

    #[test]
    fn test_cache_entry_expiration() {
        use chrono::Duration;

        let mut entry = CacheEntry::new("response".to_string(), "hash123".to_string());
        // Manually set old timestamp
        entry.created_at = Utc::now() - Duration::seconds(7200);
        assert!(entry.is_expired(3600));
    }

    #[test]
    fn test_cache_entry_touch() {
        let mut entry = CacheEntry::new("response".to_string(), "hash123".to_string());
        let initial_count = entry.access_count;
        entry.touch();
        assert_eq!(entry.access_count, initial_count + 1);
    }

    #[test]
    fn test_batch_item_creation() {
        let item = BatchItem::new("query".to_string(), Some(vec![1.0, 2.0, 3.0]));
        assert_eq!(item.query, "query");
        assert!(item.embedding.is_some());
        assert!(!item.id.is_nil());
    }

    #[test]
    fn test_cost_optimizer_stats_default() {
        let stats = CostOptimizerStats::default();
        assert_eq!(stats.total_llm_calls, 0);
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.batch_savings, 0);
        assert_eq!(stats.similar_query_savings, 0);
        assert_eq!(stats.cache_entries, 0);
        assert_eq!(stats.batch_queue_size, 0);
    }

    #[test]
    fn test_estimate_cost() {
        let cost = CostOptimizer::estimate_cost(1000, "gpt-4o");
        assert!((cost - 0.005).abs() < 0.0001);

        let cost_mini = CostOptimizer::estimate_cost(1000, "gpt-4o-mini");
        assert!((cost_mini - 0.00015).abs() < 0.00001);
    }

    #[test]
    fn test_query_hash() {
        let hash1 = CostOptimizer::compute_query_hash("test query");
        let hash2 = CostOptimizer::compute_query_hash("test query");
        let hash3 = CostOptimizer::compute_query_hash("different query");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
