//! Proactive Agent System
//!
//! Background monitoring and proactive memory management.
//!
//! # Components
//! - `ProactiveAgent`: Main agent struct with background monitoring
//! - `ResourceMonitor`: Monitors resources for changes
//! - `EventTrigger`: Triggers actions based on events
//! - `IntentionPredictor`: Predicts user intent (Phase 1.5.2)
//!
//! # Phase 1.5 Implementation
//! - Background task management (Tokio spawn) ✅
//! - Resource monitoring interface ✅
//! - Event trigger mechanism ✅
//! - Intent prediction module ✅ (Phase 1.5.2)

// Module structure for proactive system
pub mod intention;

pub use intention::{
    IntentionPredictor, IntentConfig, IntentResult, PredictedIntent, MemoryPattern,
};

use crate::error::MemError;
use crate::models::{MemoryItem, Resource};
use crate::pipeline::{EvolvePipeline, MemorizePipeline};
use crate::storage::MemoryStorage;
use crate::llm::LLMClient;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use chrono::{DateTime, Utc};

/// Result type for proactive operations
pub type ProactiveResult<T> = Result<T, MemError>;

/// Event types that trigger proactive actions
#[derive(Debug, Clone, PartialEq)]
pub enum ProactiveEvent {
    /// New resource added
    ResourceAdded { resource_id: String },
    /// Memory item accessed
    MemoryAccessed { item_id: String },
    /// Time-based trigger (e.g., daily evolution)
    ScheduledEvolution,
    /// Memory threshold reached (e.g., 1000 items)
    MemoryThreshold { count: usize },
    /// User intent detected
    IntentDetected { intent: String, confidence: f32 },
}

/// Configuration for proactive agent
#[derive(Debug, Clone)]
pub struct ProactiveConfig {
    /// Monitoring interval in seconds
    pub monitor_interval_secs: u64,
    /// Evolution interval in seconds (how often to run evolve_all)
    pub evolution_interval_secs: u64,
    /// Memory threshold for triggering proactive action
    pub memory_threshold: usize,
    /// Enable intent prediction
    pub enable_intent_prediction: bool,
    /// Enable cost optimization
    pub enable_cost_optimization: bool,
}

impl Default for ProactiveConfig {
    fn default() -> Self {
        Self {
            monitor_interval_secs: 60,        // Check every 60 seconds
            evolution_interval_secs: 86400,   // Evolve daily (24 hours)
            memory_threshold: 1000,           // Trigger at 1000 items
            enable_intent_prediction: true,
            enable_cost_optimization: true,
        }
    }
}

/// Statistics for proactive agent
#[derive(Debug, Clone, Default)]
pub struct ProactiveStats {
    /// Number of monitoring cycles completed
    pub monitor_cycles: u64,
    /// Number of evolutions triggered
    pub evolutions_triggered: u64,
    /// Number of memories proactively extracted
    pub proactively_extracted: u64,
    /// Number of intents predicted
    pub intents_predicted: u64,
    /// Last evolution time
    pub last_evolution: Option<DateTime<Utc>>,
    /// Last monitoring time
    pub last_monitor: Option<DateTime<Utc>>,
}

/// Resource monitor trait for custom monitoring logic
pub trait ResourceMonitor: Send + Sync {
    /// Check for new resources
    fn check_new_resources(&self) -> ProactiveResult<Vec<Resource>>;

    /// Get resource changes since last check
    fn get_resource_changes(&self, since: DateTime<Utc>) -> ProactiveResult<Vec<Resource>>;
}

/// Event trigger trait for custom event handling
pub trait EventTrigger: Send + Sync {
    /// Check if event should trigger
    fn should_trigger(&self, event: &ProactiveEvent) -> ProactiveResult<bool>;

    /// Handle event
    fn handle_event(&self, event: ProactiveEvent) -> ProactiveResult<()>;
}

/// Proactive agent for background monitoring and automatic memory management
pub struct ProactiveAgent {
    /// Configuration
    config: ProactiveConfig,
    /// Memory storage
    storage: Arc<MemoryStorage>,
    /// Memorize pipeline for proactive extraction
    memorize_pipeline: Arc<MemorizePipeline>,
    /// Evolve pipeline for automatic evolution
    evolve_pipeline: Arc<EvolvePipeline>,
    /// LLM client for intent prediction
    llm_client: Arc<RwLock<Box<dyn LLMClient>>>,
    /// Intent predictor (Phase 1.5.2)
    intent_predictor: Option<IntentionPredictor>,
    /// Statistics
    stats: Arc<RwLock<ProactiveStats>>,
    /// Shutdown signal
    shutdown: Arc<RwLock<bool>>,
}

impl ProactiveAgent {
    /// Create a new proactive agent
    pub fn new(
        config: ProactiveConfig,
        storage: Arc<MemoryStorage>,
        memorize_pipeline: Arc<MemorizePipeline>,
        evolve_pipeline: Arc<EvolvePipeline>,
        llm_client: Arc<RwLock<Box<dyn LLMClient>>>,
    ) -> Self {
        // Create intent predictor if enabled
        let intent_predictor = if config.enable_intent_prediction {
            Some(IntentionPredictor::new(
                IntentConfig::default(),
                storage.clone(),
                llm_client.clone(),
            ))
        } else {
            None
        };

        Self {
            config,
            storage,
            memorize_pipeline,
            evolve_pipeline,
            llm_client,
            intent_predictor,
            stats: Arc::new(RwLock::new(ProactiveStats::default())),
            shutdown: Arc::new(RwLock::new(false)),
        }
    }

    /// Start background monitoring task
    pub async fn start(&self) -> ProactiveResult<()> {
        let monitor_interval = Duration::from_secs(self.config.monitor_interval_secs);
        let evolution_interval = Duration::from_secs(self.config.evolution_interval_secs);

        // Clone necessary components for the background task
        let storage = self.storage.clone();
        let evolve_pipeline = self.evolve_pipeline.clone();
        let stats = self.stats.clone();
        let shutdown = self.shutdown.clone();
        let memory_threshold = self.config.memory_threshold;

        // Spawn background monitoring task
        tokio::spawn(async move {
            let mut monitor_timer = interval(monitor_interval);
            let mut evolution_timer = interval(evolution_interval);

            loop {
                tokio::select! {
                    _ = monitor_timer.tick() => {
                        // Check shutdown signal
                        if *shutdown.read().await {
                            break;
                        }

                        // Monitoring cycle
                        if let Ok(count) = storage.item_count() {
                            if count >= memory_threshold {
                                // Trigger proactive action
                                let event = ProactiveEvent::MemoryThreshold { count };
                                // Handle event (to be implemented with EventTrigger)
                                tracing::info!("Memory threshold reached: {} items", count);
                            }
                        }

                        // Update stats
                        let mut stats = stats.write().await;
                        stats.monitor_cycles += 1;
                        stats.last_monitor = Some(Utc::now());
                    }

                    _ = evolution_timer.tick() => {
                        // Check shutdown signal
                        if *shutdown.read().await {
                            break;
                        }

                        // Run evolution
                        match evolve_pipeline.evolve_all().await {
                            Ok(evolve_stats) => {
                                let mut stats = stats.write().await;
                                stats.evolutions_triggered += 1;
                                stats.last_evolution = Some(Utc::now());
                                tracing::info!(
                                    "Evolution completed: total_items={}, avg_weight={:.2}, low_weight={}",
                                    evolve_stats.total_items,
                                    evolve_stats.average_weight,
                                    evolve_stats.low_weight_items
                                );
                            }
                            Err(e) => {
                                tracing::error!("Evolution failed: {}", e);
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop background monitoring
    pub async fn stop(&self) -> ProactiveResult<()> {
        let mut shutdown = self.shutdown.write().await;
        *shutdown = true;
        Ok(())
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> ProactiveStats {
        self.stats.read().await.clone()
    }

    /// Predict user intent (Phase 1.5.2)
    pub async fn predict_intent(&self, context: &str) -> ProactiveResult<Option<String>> {
        if !self.config.enable_intent_prediction {
            return Ok(None);
        }

        // Use intent predictor if available
        if let Some(ref predictor) = self.intent_predictor {
            match predictor.predict(context).await? {
                Some(intent) => {
                    // Update stats
                    let mut stats = self.stats.write().await;
                    stats.intents_predicted += 1;

                    // Emit event
                    let event = ProactiveEvent::IntentDetected {
                        intent: intent.intent_type.clone(),
                        confidence: intent.confidence,
                    };
                    tracing::info!(
                        "Intent predicted: {} (confidence: {:.2})",
                        intent.intent_type,
                        intent.confidence
                    );

                    Ok(Some(intent.intent_type))
                }
                None => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    /// Proactively extract memories from resource (Phase 1.5.3 - to be implemented)
    pub async fn proactive_extract(&self, _resource: Resource) -> ProactiveResult<Vec<MemoryItem>> {
        // TODO: Implement proactive extraction
        // This will be implemented in Phase 1.5.3
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proactive_config_default() {
        let config = ProactiveConfig::default();
        assert_eq!(config.monitor_interval_secs, 60);
        assert_eq!(config.evolution_interval_secs, 86400);
        assert_eq!(config.memory_threshold, 1000);
        assert!(config.enable_intent_prediction);
        assert!(config.enable_cost_optimization);
    }

    #[test]
    fn test_proactive_event_variants() {
        let event1 = ProactiveEvent::ResourceAdded {
            resource_id: "res-123".to_string(),
        };
        let event2 = ProactiveEvent::ScheduledEvolution;
        let event3 = ProactiveEvent::MemoryThreshold { count: 1500 };
        let event4 = ProactiveEvent::IntentDetected {
            intent: "search".to_string(),
            confidence: 0.85,
        };

        assert!(matches!(event1, ProactiveEvent::ResourceAdded { .. }));
        assert!(matches!(event2, ProactiveEvent::ScheduledEvolution));
        assert!(matches!(event3, ProactiveEvent::MemoryThreshold { .. }));
        assert!(matches!(event4, ProactiveEvent::IntentDetected { .. }));
    }

    #[test]
    fn test_proactive_stats_default() {
        let stats = ProactiveStats::default();
        assert_eq!(stats.monitor_cycles, 0);
        assert_eq!(stats.evolutions_triggered, 0);
        assert_eq!(stats.proactively_extracted, 0);
        assert_eq!(stats.intents_predicted, 0);
        assert!(stats.last_evolution.is_none());
        assert!(stats.last_monitor.is_none());
    }

    #[test]
    fn test_proactive_config_custom() {
        let config = ProactiveConfig {
            monitor_interval_secs: 30,
            evolution_interval_secs: 3600,
            memory_threshold: 500,
            enable_intent_prediction: false,
            enable_cost_optimization: true,
        };
        assert_eq!(config.monitor_interval_secs, 30);
        assert_eq!(config.evolution_interval_secs, 3600);
        assert_eq!(config.memory_threshold, 500);
        assert!(!config.enable_intent_prediction);
        assert!(config.enable_cost_optimization);
    }
}

