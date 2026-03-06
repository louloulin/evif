//! Embedding management with dual-layer cache (L1: memory, L2: persistent)

use async_trait::async_trait;
use crate::error::{MemError, MemResult};
use lru::LruCache;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Embedding client trait
#[async_trait]
pub trait EmbeddingClient: Send + Sync {
    async fn embed(&self, texts: &[String]) -> MemResult<Vec<Vec<f32>>>;
    fn dimension(&self) -> usize;
}

/// OpenAI embedding client
pub struct OpenAIEmbeddingClient {
    api_key: String,
    model: String,
    dimension: usize,
    client: reqwest::Client,
}

impl OpenAIEmbeddingClient {
    pub fn new(api_key: String, model: String, dimension: usize) -> Self {
        Self {
            api_key,
            model,
            dimension,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl EmbeddingClient for OpenAIEmbeddingClient {
    async fn embed(&self, texts: &[String]) -> MemResult<Vec<Vec<f32>>> {
        let response = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&serde_json::json!({
                "model": self.model,
                "input": texts
            }))
            .send()
            .await
            .map_err(|e| MemError::Embedding(e.to_string()))?;

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| MemError::Embedding(e.to_string()))?;

        let embeddings = data["data"]
            .as_array()
            .ok_or_else(|| MemError::Embedding("Invalid response format".to_string()))?
            .iter()
            .map(|item| {
                item["embedding"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                    .collect()
            })
            .collect();

        Ok(embeddings)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

/// Cache strategy for dual-layer cache
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CacheStrategy {
    /// Only use L1 in-memory cache
    #[default]
    L1Only,
    /// Use L1 first, then L2 on cache miss
    L1L2,
    /// Only use L2 persistent cache
    L2Only,
    /// Bypass all caches
    Bypass,
}

/// L2 persistent cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    embedding: Vec<f32>,
    created_at: u64,
    access_count: u32,
}

impl CacheEntry {
    fn new(embedding: Vec<f32>) -> Self {
        Self {
            embedding,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            access_count: 1,
        }
    }
}

/// L2 persistent cache (file-based)
pub struct L2Cache {
    cache_dir: PathBuf,
    index: HashMap<String, CacheEntry>,
    max_entries: usize,
}

impl L2Cache {
    /// Create new L2 cache with directory and max entries
    pub fn new(cache_dir: PathBuf, max_entries: usize) -> MemResult<Self> {
        // Create directory if not exists
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)?;
        }

        let mut cache = Self {
            cache_dir,
            index: HashMap::new(),
            max_entries,
        };

        // Load existing index
        cache.load_index()?;

        Ok(cache)
    }

    /// Load index from disk
    fn load_index(&mut self) -> MemResult<()> {
        let index_path = self.cache_dir.join("index.json");
        if index_path.exists() {
            let content = fs::read_to_string(&index_path)?;
            self.index = serde_json::from_str(&content)
                .map_err(|e| MemError::Storage(format!("Failed to load index: {}", e)))?;
        }
        Ok(())
    }

    /// Save index to disk
    fn save_index(&self) -> MemResult<()> {
        let index_path = self.cache_dir.join("index.json");
        let content = serde_json::to_string_pretty(&self.index)
            .map_err(|e| MemError::Serialization(e.to_string()))?;
        fs::write(index_path, content)?;
        Ok(())
    }

    /// Get embedding from cache
    pub fn get(&mut self, key: &str) -> Option<Vec<f32>> {
        if let Some(entry) = self.index.get_mut(key) {
            entry.access_count += 1;
            return Some(entry.embedding.clone());
        }
        None
    }

    /// Put embedding into cache
    pub fn put(&mut self, key: String, embedding: Vec<f32>) -> MemResult<()> {
        // Evict if needed
        if self.index.len() >= self.max_entries {
            self.evict_lru();
        }

        let entry = CacheEntry::new(embedding);
        self.index.insert(key.clone(), entry);

        // Save individual cache file
        let file_path = self.cache_dir.join(format!("{}.json", key));
        let content = serde_json::to_string_pretty(&self.index[&key])
            .map_err(|e| MemError::Serialization(e.to_string()))?;
        fs::write(file_path, content)?;

        // Update index periodically (every 10 writes)
        if self.index.len() % 10 == 0 {
            self.save_index()?;
        }

        Ok(())
    }

    /// Evict least recently used entries
    fn evict_lru(&mut self) {
        if self.index.is_empty() {
            return;
        }

        // Find entry with lowest access count
        let lru_key = self.index
            .iter()
            .min_by_key(|(_, v)| v.access_count)
            .map(|(k, _)| k.clone());

        if let Some(key) = lru_key {
            // Remove from index
            self.index.remove(&key);
            // Remove file
            let file_path = self.cache_dir.join(format!("{}.json", key));
            if file_path.exists() {
                let _ = fs::remove_file(file_path);
            }
        }
    }

    /// Clear all cache entries
    pub fn clear(&mut self) -> MemResult<()> {
        self.index.clear();
        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                fs::remove_file(path)?;
            }
        }
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let total_entries = self.index.len();
        let total_accesses: u32 = self.index.values().map(|e| e.access_count).sum();
        CacheStats {
            total_entries,
            total_accesses,
            cache_dir: self.cache_dir.to_string_lossy().to_string(),
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_accesses: u32,
    pub cache_dir: String,
}

/// Dual-layer cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// L1 (memory) cache size
    pub l1_size: usize,
    /// L2 (persistent) cache directory
    pub l2_dir: Option<PathBuf>,
    /// L2 cache max entries
    pub l2_max_entries: usize,
    /// Cache strategy
    pub strategy: CacheStrategy,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            l1_size: 1000,
            l2_dir: None,
            l2_max_entries: 10000,
            strategy: CacheStrategy::L1Only,
        }
    }
}

/// Embedding manager with dual-layer cache
pub struct EmbeddingManager {
    client: Arc<dyn EmbeddingClient>,
    l1_cache: Arc<Mutex<LruCache<String, Vec<f32>>>>,
    l2_cache: Arc<Mutex<Option<L2Cache>>>,
    config: CacheConfig,
}

impl EmbeddingManager {
    /// Create new embedding manager with configuration
    pub fn new(client: Arc<dyn EmbeddingClient>, config: CacheConfig) -> MemResult<Self> {
        let l1_cache = LruCache::new(NonZeroUsize::new(config.l1_size).unwrap());

        let l2_cache = if let Some(ref l2_dir) = config.l2_dir {
            Some(L2Cache::new(l2_dir.clone(), config.l2_max_entries)?)
        } else {
            None
        };

        Ok(Self {
            client,
            l1_cache: Arc::new(Mutex::new(l1_cache)),
            l2_cache: Arc::new(Mutex::new(l2_cache)),
            config,
        })
    }

    /// Create with default config (L1 only)
    pub fn with_default_config(client: Arc<dyn EmbeddingClient>) -> MemResult<Self> {
        Self::new(client, CacheConfig::default())
    }

    /// Compute hash for cache key
    fn compute_hash(text: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        hex::encode(&hasher.finalize()[..16])
    }

    /// Get embedding for text (with dual-layer caching)
    pub async fn embed(&self, text: &str) -> MemResult<Vec<f32>> {
        if self.config.strategy == CacheStrategy::Bypass {
            let embeddings = self.client.embed(&[text.to_string()]).await?;
            return embeddings
                .into_iter()
                .next()
                .ok_or_else(|| MemError::Embedding("No embedding returned".to_string()));
        }

        let key = Self::compute_hash(text);

        // L1: Check in-memory cache
        if self.config.strategy == CacheStrategy::L1Only
            || self.config.strategy == CacheStrategy::L1L2
        {
            let mut l1 = self.l1_cache.lock().await;
            if let Some(cached) = l1.get(&key) {
                return Ok(cached.clone());
            }
        }

        // L2: Check persistent cache
        if self.config.strategy == CacheStrategy::L1L2 || self.config.strategy == CacheStrategy::L2Only {
            let mut l2_guard = self.l2_cache.lock().await;
            if let Some(ref mut l2) = *l2_guard {
                if let Some(cached) = l2.get(&key) {
                    // Populate L1 for future access
                    drop(l2_guard);
                    let mut l1 = self.l1_cache.lock().await;
                    l1.put(key, cached.clone());
                    return Ok(cached);
                }
            }
        }

        // Fetch from API
        let embeddings = self.client.embed(&[text.to_string()]).await?;
        let embedding = embeddings
            .into_iter()
            .next()
            .ok_or_else(|| MemError::Embedding("No embedding returned".to_string()))?;

        // Update L1
        if self.config.strategy != CacheStrategy::L2Only {
            let mut l1 = self.l1_cache.lock().await;
            l1.put(key.clone(), embedding.clone());
        }

        // Update L2
        if self.config.strategy == CacheStrategy::L1L2 {
            let mut l2_guard = self.l2_cache.lock().await;
            if let Some(ref mut l2) = *l2_guard {
                l2.put(key, embedding.clone())?;
            }
        }

        Ok(embedding)
    }

    /// Batch embed multiple texts
    pub async fn embed_batch(&self, texts: &[String]) -> MemResult<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        if self.config.strategy == CacheStrategy::Bypass {
            return self.client.embed(texts).await;
        }

        let mut results = vec![None; texts.len()];
        let mut uncached = Vec::new();

        // L1 check
        if self.config.strategy != CacheStrategy::L2Only {
            let mut l1 = self.l1_cache.lock().await;
            for (i, text) in texts.iter().enumerate() {
                let key = Self::compute_hash(text);
                if let Some(cached) = l1.get(&key) {
                    results[i] = Some(cached.clone());
                } else {
                    uncached.push((i, text.clone(), key));
                }
            }
        }

        // L2 check for remaining - collect keys to fetch from L2 first
        let mut l2_hits: Vec<(usize, Vec<f32>)> = Vec::new();
        if !uncached.is_empty() && (self.config.strategy == CacheStrategy::L1L2 || self.config.strategy == CacheStrategy::L2Only) {
            let mut l2_guard = self.l2_cache.lock().await;
            if let Some(ref mut l2) = *l2_guard {
                let mut still_uncached = Vec::new();
                for (i, text, key) in uncached {
                    if let Some(cached) = l2.get(&key) {
                        l2_hits.push((i, cached.clone()));
                    } else {
                        still_uncached.push((i, text, key));
                    }
                }
                uncached = still_uncached;
            }
        }

        // Populate L1 from L2 hits
        if !l2_hits.is_empty() && self.config.strategy == CacheStrategy::L1L2 {
            let mut l1 = self.l1_cache.lock().await;
            for (i, embedding) in l2_hits.iter() {
                let key = Self::compute_hash(&texts[*i]);
                l1.put(key, embedding.clone());
            }
        }

        // Add L2 hits to results
        for (i, embedding) in l2_hits {
            results[i] = Some(embedding);
        }

        // Fetch remaining from API
        if !uncached.is_empty() {
            let texts_to_fetch: Vec<String> = uncached.iter().map(|(_, t, _)| t.clone()).collect();
            let embeddings = self.client.embed(&texts_to_fetch).await?;

            // Update caches
            // First, get L1 lock if needed
            if self.config.strategy != CacheStrategy::L2Only {
                let mut l1 = self.l1_cache.lock().await;

                // If L1L2, also get L2 lock
                if self.config.strategy == CacheStrategy::L1L2 {
                    let mut l2_guard = self.l2_cache.lock().await;
                    for ((i, _text, key), embedding) in uncached.iter().zip(embeddings.iter()) {
                        results[*i] = Some(embedding.clone());
                        l1.put(key.clone(), embedding.clone());
                        if let Some(ref mut l2) = *l2_guard {
                            let _ = l2.put(key.clone(), embedding.clone());
                        }
                    }
                } else {
                    // L1 only
                    for ((i, _text, key), embedding) in uncached.iter().zip(embeddings.iter()) {
                        results[*i] = Some(embedding.clone());
                        l1.put(key.clone(), embedding.clone());
                    }
                }
            } else if self.config.strategy == CacheStrategy::L2Only {
                // L2 only
                let mut l2_guard = self.l2_cache.lock().await;
                for ((i, _text, key), embedding) in uncached.iter().zip(embeddings.iter()) {
                    results[*i] = Some(embedding.clone());
                    if let Some(ref mut l2) = *l2_guard {
                        let _ = l2.put(key.clone(), embedding.clone());
                    }
                }
            }
        }

        results
            .into_iter()
            .map(|r| r.ok_or_else(|| MemError::Embedding("Missing embedding".to_string())))
            .collect()
    }

    /// Clear all caches
    pub async fn clear_cache(&self) -> MemResult<()> {
        // Clear L1
        {
            let mut l1 = self.l1_cache.lock().await;
            *l1 = LruCache::new(NonZeroUsize::new(self.config.l1_size).unwrap());
        }

        // Clear L2
        if self.config.strategy != CacheStrategy::L1Only {
            let mut l2_guard = self.l2_cache.lock().await;
            if let Some(ref mut l2) = *l2_guard {
                l2.clear()?;
            }
        }

        Ok(())
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        let l1_size = {
            let l1 = self.l1_cache.lock().await;
            l1.len()
        };

        let l2_stats = {
            let l2_guard = self.l2_cache.lock().await;
            if let Some(ref l2) = *l2_guard {
                l2.stats()
            } else {
                CacheStats {
                    total_entries: 0,
                    total_accesses: 0,
                    cache_dir: "none".to_string(),
                }
            }
        };

        CacheStats {
            total_entries: l1_size + l2_stats.total_entries,
            total_accesses: l2_stats.total_accesses,
            cache_dir: format!("L1: {} entries, L2: {}", l1_size, l2_stats.cache_dir),
        }
    }

    /// Get cache configuration
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }
}
