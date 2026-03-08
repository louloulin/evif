//! Vector search module for semantic memory retrieval
//!
//! Provides trait-based vector index with multiple backend implementations:
//! - InMemoryVectorIndex: Simple in-memory implementation for development
//! - FaissVectorIndex: Faiss-based high-performance search (optional, requires "faiss" feature)
//!
//! # Usage
//! ```rust
//! use evif_mem::vector::{VectorIndex, VectorIndexConfig, InMemoryVectorIndex};
//!
//! let config = VectorIndexConfig::default();
//! let index = InMemoryVectorIndex::new(128, config).unwrap();
//! ```
//!
//! # Feature Flags
//! - `faiss`: Enable FAISS-based vector index for production workloads

pub mod memory;

#[cfg(feature = "faiss")]
pub mod faiss;

#[cfg(feature = "qdrant")]
pub mod qdrant;

use crate::error::MemResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Search result with score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// ID of the indexed item
    pub id: String,
    /// Similarity score (higher = more similar)
    pub score: f32,
    /// Optional metadata
    pub metadata: Option<HashMap<String, String>>,
}

/// Vector index configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorIndexConfig {
    /// Number of results to return
    pub top_k: usize,
    /// Similarity threshold (0.0-1.0)
    pub threshold: f32,
    /// Whether to normalize vectors
    pub normalize: bool,
    /// Metric type
    pub metric: VectorMetric,
}

impl Default for VectorIndexConfig {
    fn default() -> Self {
        Self {
            top_k: 10,
            threshold: 0.0,
            normalize: true,
            metric: VectorMetric::Cosine,
        }
    }
}

/// Similarity metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VectorMetric {
    /// Cosine similarity
    #[default]
    Cosine,
    /// Euclidean distance (note: lower is better, scores will be inverted)
    Euclidean,
    /// Dot product
    DotProduct,
}

/// Vector index trait for semantic search
#[async_trait]
pub trait VectorIndex: Send + Sync {
    /// Add vectors to the index
    async fn add(
        &self,
        id: String,
        vector: Vec<f32>,
        metadata: Option<HashMap<String, String>>,
    ) -> MemResult<()>;

    /// Add multiple vectors at once (batch)
    async fn add_batch(
        &self,
        items: Vec<(String, Vec<f32>, Option<HashMap<String, String>>)>,
    ) -> MemResult<()>;

    /// Search for similar vectors
    async fn search(
        &self,
        query: &[f32],
        top_k: Option<usize>,
        filter: Option<Box<dyn Send + Sync + Fn(&str) -> bool + 'static>>,
    ) -> MemResult<Vec<SearchResult>>;

    /// Get vector by ID
    async fn get(&self, id: &str) -> MemResult<Option<Vec<f32>>>;

    /// Remove vector from index
    async fn remove(&self, id: &str) -> MemResult<()>;

    /// Get number of vectors in index
    async fn len(&self) -> usize;

    /// Check if index is empty
    async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// Get dimension of vectors
    fn dimension(&self) -> usize;

    /// Clear all vectors
    async fn clear(&self) -> MemResult<()>;

    /// Get index statistics
    async fn stats(&self) -> IndexStats;
}

/// Index statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub total_vectors: usize,
    pub dimension: usize,
    pub memory_bytes: usize,
}

/// Normalize vector for cosine similarity
pub fn normalize_vector(vector: &[f32]) -> Vec<f32> {
    let magnitude: f32 = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
    if magnitude == 0.0 {
        return vector.to_vec();
    }
    vector.iter().map(|v| v / magnitude).collect()
}

/// Compute cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|v| v * v).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|v| v * v).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

/// Compute euclidean distance between two vectors
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return f32::MAX;
    }

    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

/// Compute dot product between two vectors
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Re-export for convenience
pub use memory::InMemoryVectorIndex;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 1.0, 0.0];
        let d = vec![-1.0, 0.0, 0.0];

        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
        assert!(cosine_similarity(&a, &c).abs() < 0.001);
        assert!((cosine_similarity(&a, &d) - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_euclidean_distance() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![3.0, 4.0, 0.0];

        assert!((euclidean_distance(&a, &b) - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_normalize_vector() {
        let v = vec![3.0, 4.0, 0.0];
        let normalized = normalize_vector(&v);

        let magnitude: f32 = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.001);
    }
}
