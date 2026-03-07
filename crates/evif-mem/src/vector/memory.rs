//! In-memory vector index implementation
//!
//! A simple but efficient in-memory vector index using brute-force search.
//! Suitable for development and small to medium datasets.

use super::{
    cosine_similarity, dot_product, euclidean_distance, normalize_vector, IndexStats, SearchResult,
    VectorIndex, VectorIndexConfig, VectorMetric,
};
use crate::error::{MemError, MemResult};
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// In-memory vector index with brute-force search
pub struct InMemoryVectorIndex {
    dimension: usize,
    vectors: RwLock<HashMap<String, Vec<f32>>>,
    metadata: RwLock<HashMap<String, HashMap<String, String>>>,
    config: VectorIndexConfig,
}

impl InMemoryVectorIndex {
    /// Create new in-memory vector index
    pub fn new(dimension: usize, config: VectorIndexConfig) -> MemResult<Self> {
        if dimension == 0 {
            return Err(MemError::InvalidConfig("dimension must be > 0".to_string()));
        }

        Ok(Self {
            dimension,
            vectors: RwLock::new(HashMap::new()),
            metadata: RwLock::new(HashMap::new()),
            config,
        })
    }

    /// Compute similarity score based on metric
    fn compute_score(&self, a: &[f32], b: &[f32]) -> f32 {
        let (vec_a, vec_b) = if self.config.normalize {
            (normalize_vector(a), normalize_vector(b))
        } else {
            (a.to_vec(), b.to_vec())
        };

        match self.config.metric {
            VectorMetric::Cosine => cosine_similarity(&vec_a, &vec_b),
            VectorMetric::Euclidean => {
                // Convert distance to similarity (higher = better)
                let dist = euclidean_distance(&vec_a, &vec_b);
                1.0 / (1.0 + dist)
            }
            VectorMetric::DotProduct => dot_product(&vec_a, &vec_b),
        }
    }
}

#[async_trait]
impl VectorIndex for InMemoryVectorIndex {
    async fn add(
        &self,
        id: String,
        vector: Vec<f32>,
        metadata: Option<HashMap<String, String>>,
    ) -> MemResult<()> {
        if vector.len() != self.dimension {
            return Err(MemError::InvalidConfig(format!(
                "vector dimension {} does not match index dimension {}",
                vector.len(),
                self.dimension
            )));
        }

        self.vectors.write().await.insert(id.clone(), vector);
        if let Some(meta) = metadata {
            self.metadata.write().await.insert(id, meta);
        }

        Ok(())
    }

    async fn add_batch(
        &self,
        items: Vec<(String, Vec<f32>, Option<HashMap<String, String>>)>,
    ) -> MemResult<()> {
        let mut vectors = self.vectors.write().await;
        let mut metadata = self.metadata.write().await;

        for (id, vector, meta) in items {
            if vector.len() != self.dimension {
                return Err(MemError::InvalidConfig(format!(
                    "vector dimension {} does not match index dimension {}",
                    vector.len(),
                    self.dimension
                )));
            }

            vectors.insert(id.clone(), vector);
            if let Some(m) = meta {
                metadata.insert(id, m);
            }
        }

        Ok(())
    }

    async fn search(
        &self,
        query: &[f32],
        top_k: Option<usize>,
        _filter: Option<Box<dyn Send + Sync + Fn(&str) -> bool + 'static>>,
    ) -> MemResult<Vec<SearchResult>> {
        if query.len() != self.dimension {
            return Err(MemError::InvalidConfig(format!(
                "query dimension {} does not match index dimension {}",
                query.len(),
                self.dimension
            )));
        }

        let k = top_k.unwrap_or(self.config.top_k);

        // Collect all vectors and metadata first to avoid borrow issues
        let all_data: Vec<(String, Vec<f32>, Option<HashMap<String, String>>)> = {
            let vectors = self.vectors.read().await;
            let metadata = self.metadata.read().await;
            vectors
                .iter()
                .map(|(id, vec)| {
                    let meta = metadata.get(id).cloned();
                    (id.clone(), vec.clone(), meta)
                })
                .collect()
        };

        // Compute scores
        let mut results: Vec<SearchResult> = all_data
            .into_iter()
            .map(|(id, vector, metadata)| {
                let score = self.compute_score(query, &vector);
                SearchResult {
                    id,
                    score,
                    metadata,
                }
            })
            .filter(|r| r.score >= self.config.threshold)
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Return top k
        results.truncate(k);
        Ok(results)
    }

    async fn get(&self, id: &str) -> MemResult<Option<Vec<f32>>> {
        Ok(self.vectors.read().await.get(id).cloned())
    }

    async fn remove(&self, id: &str) -> MemResult<()> {
        self.vectors.write().await.remove(id);
        self.metadata.write().await.remove(id);
        Ok(())
    }

    async fn len(&self) -> usize {
        self.vectors.read().await.len()
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    async fn clear(&self) -> MemResult<()> {
        self.vectors.write().await.clear();
        self.metadata.write().await.clear();
        Ok(())
    }

    async fn stats(&self) -> IndexStats {
        let vectors = self.vectors.read().await;
        let count = vectors.len();
        let memory_bytes = vectors
            .values()
            .map(|v| v.len() * std::mem::size_of::<f32>())
            .sum();

        IndexStats {
            total_vectors: count,
            dimension: self.dimension,
            memory_bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_and_search() {
        let index = InMemoryVectorIndex::new(3, VectorIndexConfig::default()).unwrap();

        // Add vectors
        index
            .add(
                "doc1".to_string(),
                vec![1.0, 0.0, 0.0],
                Some(
                    [("text".to_string(), "Document 1".to_string())]
                        .into_iter()
                        .collect(),
                ),
            )
            .await
            .unwrap();

        index
            .add("doc2".to_string(), vec![0.0, 1.0, 0.0], None)
            .await
            .unwrap();

        index
            .add("doc3".to_string(), vec![1.0, 1.0, 0.0], None)
            .await
            .unwrap();

        // Search
        let results = index.search(&[1.0, 0.0, 0.0], None, None).await.unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].id, "doc1"); // Most similar
        assert!(results[0].score > results[1].score);
    }

    #[tokio::test]
    async fn test_top_k() {
        let index = InMemoryVectorIndex::new(2, VectorIndexConfig::default()).unwrap();

        for i in 0..20 {
            index
                .add(format!("doc{}", i), vec![i as f32, 0.0], None)
                .await
                .unwrap();
        }

        let results = index.search(&[10.0, 0.0], Some(5), None).await.unwrap();

        assert_eq!(results.len(), 5);
    }

    #[tokio::test]
    async fn test_remove() {
        let index = InMemoryVectorIndex::new(2, VectorIndexConfig::default()).unwrap();

        index
            .add("doc1".to_string(), vec![1.0, 0.0], None)
            .await
            .unwrap();
        assert_eq!(index.len().await, 1);

        index.remove("doc1").await.unwrap();
        assert_eq!(index.len().await, 0);
    }

    #[tokio::test]
    async fn test_stats() {
        let index = InMemoryVectorIndex::new(128, VectorIndexConfig::default()).unwrap();

        index
            .add("doc1".to_string(), vec![0.0; 128], None)
            .await
            .unwrap();

        let stats = index.stats().await;
        assert_eq!(stats.total_vectors, 1);
        assert_eq!(stats.dimension, 128);
    }
}
