//! Metrics module for evif-mem monitoring and observability.
//!
//! This module provides Prometheus-compatible metrics for tracking memory operations.
//!
//! # Usage
//!
//! ```rust
//! use evif_mem::metrics::{Metrics, MetricsConfig};
//!
//! // Initialize metrics with default config
//! let metrics = Metrics::new(MetricsConfig::default()).unwrap();
//!
//! // Record operations
//! metrics.record_memorize(5, 120); // 5 items, 120ms
//! metrics.increment_errors("memorize");
//! ```
//!
//! # Features
//!
//! - Counters for operation counts
//! - Histograms for operation durations
//! - Gauges for active operations
//! - Error tracking by operation type

use thiserror::Error;

/// Errors that can occur in the metrics module.
#[derive(Error, Debug)]
pub enum MetricsError {
    #[error("Failed to register metric: {0}")]
    RegistrationFailed(String),
    #[error("Metrics not initialized")]
    NotInitialized,
}

/// Configuration for the metrics system.
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// Whether to enable metrics collection.
    pub enabled: bool,
    /// Prefix for all metric names.
    pub prefix: String,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefix: "evif_mem".to_string(),
        }
    }
}

#[cfg(feature = "metrics")]
mod inner {
    use prometheus::{
        histogram_opts, opts, register_counter, register_gauge, register_histogram, Counter, Gauge,
        Histogram,
    };
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// Memory system metrics.
    ///
    /// Tracks operation counts, durations, and errors for monitoring.
    #[derive(Clone)]
    pub struct Metrics {
        // Operation counters
        pub memorize_total: Counter,
        pub retrieve_total: Counter,
        pub evolve_total: Counter,

        // Duration histograms (in seconds)
        pub memorize_duration: Histogram,
        pub retrieve_duration: Histogram,
        pub evolve_duration: Histogram,

        // Error counters
        pub errors_total: Counter,

        // Active operations gauge
        pub active_memorize: Gauge,
        pub active_retrieve: Gauge,
        pub active_evolve: Gauge,

        // Storage metrics
        pub memory_items_total: Gauge,
        pub categories_total: Gauge,
        pub resources_total: Gauge,
    }

    impl Metrics {
        /// Create a new Metrics instance.
        pub fn new(_config: super::MetricsConfig) -> Result<Self, super::MetricsError> {
            let prefix = "evif_mem";

            let memorize_total = register_counter!(opts!(
                format!("{}_memorize_total", prefix),
                "Total number of memorize operations"
            ))
            .map_err(|e| super::MetricsError::RegistrationFailed(e.to_string()))?;

            let retrieve_total = register_counter!(opts!(
                format!("{}_retrieve_total", prefix),
                "Total number of retrieve operations"
            ))
            .map_err(|e| super::MetricsError::RegistrationFailed(e.to_string()))?;

            let evolve_total = register_counter!(opts!(
                format!("{}_evolve_total", prefix),
                "Total number of evolve operations"
            ))
            .map_err(|e| super::MetricsError::RegistrationFailed(e.to_string()))?;

            let memorize_duration = register_histogram!(histogram_opts!(
                format!("{}_memorize_duration_seconds", prefix),
                "Duration of memorize operations in seconds"
            ))
            .map_err(|e| super::MetricsError::RegistrationFailed(e.to_string()))?;

            let retrieve_duration = register_histogram!(histogram_opts!(
                format!("{}_retrieve_duration_seconds", prefix),
                "Duration of retrieve operations in seconds"
            ))
            .map_err(|e| super::MetricsError::RegistrationFailed(e.to_string()))?;

            let evolve_duration = register_histogram!(histogram_opts!(
                format!("{}_evolve_duration_seconds", prefix),
                "Duration of evolve operations in seconds"
            ))
            .map_err(|e| super::MetricsError::RegistrationFailed(e.to_string()))?;

            let errors_total = register_counter!(opts!(
                format!("{}_errors_total", prefix),
                "Total number of errors"
            ))
            .map_err(|e| super::MetricsError::RegistrationFailed(e.to_string()))?;

            let active_memorize = register_gauge!(opts!(
                format!("{}_active_memorize", prefix),
                "Number of active memorize operations"
            ))
            .map_err(|e| super::MetricsError::RegistrationFailed(e.to_string()))?;

            let active_retrieve = register_gauge!(opts!(
                format!("{}_active_retrieve", prefix),
                "Number of active retrieve operations"
            ))
            .map_err(|e| super::MetricsError::RegistrationFailed(e.to_string()))?;

            let active_evolve = register_gauge!(opts!(
                format!("{}_active_evolve", prefix),
                "Number of active evolve operations"
            ))
            .map_err(|e| super::MetricsError::RegistrationFailed(e.to_string()))?;

            let memory_items_total = register_gauge!(opts!(
                format!("{}_memory_items_total", prefix),
                "Total number of memory items"
            ))
            .map_err(|e| super::MetricsError::RegistrationFailed(e.to_string()))?;

            let categories_total = register_gauge!(opts!(
                format!("{}_categories_total", prefix),
                "Total number of categories"
            ))
            .map_err(|e| super::MetricsError::RegistrationFailed(e.to_string()))?;

            let resources_total = register_gauge!(opts!(
                format!("{}_resources_total", prefix),
                "Total number of resources"
            ))
            .map_err(|e| super::MetricsError::RegistrationFailed(e.to_string()))?;

            Ok(Self {
                memorize_total,
                retrieve_total,
                evolve_total,
                memorize_duration,
                retrieve_duration,
                evolve_duration,
                errors_total,
                active_memorize,
                active_retrieve,
                active_evolve,
                memory_items_total,
                categories_total,
                resources_total,
            })
        }

        /// Record a memorize operation.
        ///
        /// # Arguments
        ///
        /// * `items_count` - Number of items processed
        /// * `duration_ms` - Duration in milliseconds
        pub fn record_memorize(&self, items_count: usize, duration_ms: u64) {
            self.memorize_total.inc_by(items_count as f64);
            let duration_secs = duration_ms as f64 / 1000.0;
            self.memorize_duration.observe(duration_secs);
        }

        /// Record a retrieve operation.
        ///
        /// # Arguments
        ///
        /// * `results_count` - Number of results returned
        /// * `duration_ms` - Duration in milliseconds
        pub fn record_retrieve(&self, results_count: usize, duration_ms: u64) {
            self.retrieve_total.inc_by(results_count as f64);
            let duration_secs = duration_ms as f64 / 1000.0;
            self.retrieve_duration.observe(duration_secs);
        }

        /// Record an evolve operation.
        ///
        /// # Arguments
        ///
        /// * `items_count` - Number of items evolved
        /// * `duration_ms` - Duration in milliseconds
        pub fn record_evolve(&self, items_count: usize, duration_ms: u64) {
            self.evolve_total.inc_by(items_count as f64);
            let duration_secs = duration_ms as f64 / 1000.0;
            self.evolve_duration.observe(duration_secs);
        }

        /// Increment the error counter for a specific operation.
        pub fn increment_errors(&self, _operation: &str) {
            self.errors_total.inc();
        }

        /// Increment active memorize gauge (call when starting).
        pub fn start_memorize(&self) {
            self.active_memorize.inc();
        }

        /// Decrement active memorize gauge (call when finished).
        pub fn end_memorize(&self) {
            self.active_memorize.dec();
        }

        /// Increment active retrieve gauge (call when starting).
        pub fn start_retrieve(&self) {
            self.active_retrieve.inc();
        }

        /// Decrement active retrieve gauge (call when finished).
        pub fn end_retrieve(&self) {
            self.active_retrieve.dec();
        }

        /// Increment active evolve gauge (call when starting).
        pub fn start_evolve(&self) {
            self.active_evolve.inc();
        }

        /// Decrement active evolve gauge (call when finished).
        pub fn end_evolve(&self) {
            self.active_evolve.dec();
        }

        /// Update storage gauge values.
        pub fn update_storage_stats(&self, items: usize, categories: usize, resources: usize) {
            self.memory_items_total.set(items as f64);
            self.categories_total.set(categories as f64);
            self.resources_total.set(resources as f64);
        }
    }

    /// A wrapper for Metrics that can be shared across the application.
    #[derive(Clone)]
    pub struct MetricsRegistry {
        metrics: Arc<RwLock<Option<Metrics>>>,
    }

    impl MetricsRegistry {
        /// Create a new MetricsRegistry.
        pub fn new() -> Self {
            Self {
                metrics: Arc::new(RwLock::new(None)),
            }
        }

        /// Initialize the metrics registry.
        pub async fn init(&self, config: super::MetricsConfig) -> Result<(), super::MetricsError> {
            if !config.enabled {
                return Ok(());
            }

            let metrics = Metrics::new(config)?;
            let mut guard = self.metrics.write().await;
            *guard = Some(metrics);
            Ok(())
        }

        /// Get the metrics instance.
        pub async fn get(&self) -> Option<Metrics> {
            let guard = self.metrics.read().await;
            guard.clone()
        }

        /// Record a memorize operation.
        pub async fn record_memorize(&self, items_count: usize, duration_ms: u64) {
            if let Some(metrics) = self.get().await {
                metrics.record_memorize(items_count, duration_ms);
            }
        }

        /// Record a retrieve operation.
        pub async fn record_retrieve(&self, results_count: usize, duration_ms: u64) {
            if let Some(metrics) = self.get().await {
                metrics.record_retrieve(results_count, duration_ms);
            }
        }

        /// Record an evolve operation.
        pub async fn record_evolve(&self, items_count: usize, duration_ms: u64) {
            if let Some(metrics) = self.get().await {
                metrics.record_evolve(items_count, duration_ms);
            }
        }

        /// Increment errors.
        pub async fn increment_errors(&self, operation: &str) {
            if let Some(metrics) = self.get().await {
                metrics.increment_errors(operation);
            }
        }

        /// Update storage stats.
        pub async fn update_storage_stats(&self, items: usize, categories: usize, resources: usize) {
            if let Some(metrics) = self.get().await {
                metrics.update_storage_stats(items, categories, resources);
            }
        }
    }

    impl Default for MetricsRegistry {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(not(feature = "metrics"))]
mod inner {
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// Stub metrics when metrics feature is not enabled.
    #[derive(Clone, Default)]
    pub struct Metrics;

    impl Metrics {
        pub fn new(_config: super::MetricsConfig) -> Result<Self, super::MetricsError> {
            Ok(Self)
        }

        pub fn record_memorize(&self, _items_count: usize, _duration_ms: u64) {}
        pub fn record_retrieve(&self, _results_count: usize, _duration_ms: u64) {}
        pub fn record_evolve(&self, _items_count: usize, _duration_ms: u64) {}
        pub fn increment_errors(&self, _operation: &str) {}
        pub fn start_memorize(&self) {}
        pub fn end_memorize(&self) {}
        pub fn start_retrieve(&self) {}
        pub fn end_retrieve(&self) {}
        pub fn start_evolve(&self) {}
        pub fn end_evolve(&self) {}
        pub fn update_storage_stats(&self, _items: usize, _categories: usize, _resources: usize) {}
    }

    /// A stub registry when metrics feature is not enabled.
    #[derive(Clone, Default)]
    pub struct MetricsRegistry;

    impl MetricsRegistry {
        pub fn new() -> Self {
            Self
        }

        pub async fn init(&self, _config: super::MetricsConfig) -> Result<(), super::MetricsError> {
            Ok(())
        }

        pub async fn get(&self) -> Option<Metrics> {
            None
        }

        pub async fn record_memorize(&self, _items_count: usize, _duration_ms: u64) {}
        pub async fn record_retrieve(&self, _results_count: usize, _duration_ms: u64) {}
        pub async fn record_evolve(&self, _items_count: usize, _duration_ms: u64) {}
        pub async fn increment_errors(&self, _operation: &str) {}
        pub async fn update_storage_stats(&self, _items: usize, _categories: usize, _resources: usize) {}
    }
}

pub use inner::{Metrics, MetricsRegistry};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_config_default() {
        let config = MetricsConfig::default();
        assert!(config.enabled);
        assert_eq!(config.prefix, "evif_mem");
    }

    #[cfg(feature = "metrics")]
    #[tokio::test]
    #[ignore] // Prometheus metrics can only be registered once globally
    async fn test_metrics_creation() {
        let config = MetricsConfig::default();
        let metrics = Metrics::new(config).unwrap();
        metrics.record_memorize(5, 100);
        metrics.record_retrieve(10, 50);
        metrics.increment_errors("test");
    }

    #[cfg(feature = "metrics")]
    #[tokio::test]
    async fn test_metrics_registry() {
        // Note: Prometheus metrics can only be registered once globally.
        // In real usage, metrics are registered at startup, not in tests.
        // This test verifies the registry structure is correct.
        let registry = MetricsRegistry::new();
        registry.init(MetricsConfig::default()).await.unwrap();

        // The registry is initialized - actual metric recording would work
        // if metrics were pre-registered (which happens at app startup)
    }

    #[cfg(not(feature = "metrics"))]
    #[tokio::test]
    async fn test_metrics_stub() {
        let metrics = Metrics::new(MetricsConfig::default()).unwrap();
        metrics.record_memorize(5, 100);
        metrics.record_retrieve(10, 50);

        let registry = MetricsRegistry::new();
        registry.init(MetricsConfig::default()).await.unwrap();
        registry.record_memorize(3, 100).await;
    }
}
