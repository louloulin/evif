//! Embedding management with LRU cache

use async_trait::async_trait;
use crate::error::{MemError, MemResult};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::Mutex;
use sha2::{Sha256, Digest};

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
                "input": texts,
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

/// Embedding manager with LRU cache
pub struct EmbeddingManager {
    client: Arc<dyn EmbeddingClient>,
    cache: Arc<Mutex<LruCache<String, Vec<f32>>>>,
}

impl EmbeddingManager {
    pub fn new(client: Arc<dyn EmbeddingClient>, cache_size: usize) -> Self {
        let cache = LruCache::new(NonZeroUsize::new(cache_size).unwrap());
        Self {
            client,
            cache: Arc::new(Mutex::new(cache)),
        }
    }

    /// Compute hash for cache key
    fn compute_hash(text: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        hex::encode(&hasher.finalize()[..16])
    }

    /// Get embedding for text (with caching)
    pub async fn embed(&self, text: &str) -> MemResult<Vec<f32>> {
        let key = Self::compute_hash(text);

        // Check cache
        {
            let mut cache = self.cache.lock().await;
            if let Some(cached) = cache.get(&key) {
                return Ok(cached.clone());
            }
        }

        // Call API
        let embeddings = self.client.embed(&[text.to_string()]).await?;
        let embedding = embeddings
            .into_iter()
            .next()
            .ok_or_else(|| MemError::Embedding("No embedding returned".to_string()))?;

        // Update cache
        {
            let mut cache = self.cache.lock().await;
            cache.put(key, embedding.clone());
        }

        Ok(embedding)
    }

    /// Batch embed multiple texts
    pub async fn embed_batch(&self, texts: &[String]) -> MemResult<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        // Check cache for each text
        let mut results = vec![None; texts.len()];
        let mut uncached = Vec::new();

        {
            let mut cache = self.cache.lock().await;
            for (i, text) in texts.iter().enumerate() {
                let key = Self::compute_hash(text);
                if let Some(cached) = cache.get(&key) {
                    results[i] = Some(cached.clone());
                } else {
                    uncached.push((i, text.clone(), key));
                }
            }
        }

        // Fetch uncached embeddings
        if !uncached.is_empty() {
            let texts_to_fetch: Vec<String> = uncached.iter().map(|(_, t, _)| t.clone()).collect();
            let embeddings = self.client.embed(&texts_to_fetch).await?;

            {
                let mut cache = self.cache.lock().await;
                for ((i, _, key), embedding) in uncached.iter().zip(embeddings.iter()) {
                    results[*i] = Some(embedding.clone());
                    cache.put(key.clone(), embedding.clone());
                }
            }
        }

        results
            .into_iter()
            .map(|r| r.ok_or_else(|| MemError::Embedding("Missing embedding".to_string())))
            .collect()
    }
}
