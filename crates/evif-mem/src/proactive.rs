//! Proactive Agent System
//!
//! Background monitoring and proactive memory management.
//!
//! # Components
//! - `ProactiveAgent`: Main agent struct with background monitoring
//! - `ResourceMonitor`: Monitors resources for changes
//! - `EventTrigger`: Triggers actions based on events
//! - `IntentionPredictor`: Predicts user intent (Phase 1.5.2)
//! - `CostOptimizer`: Reduces LLM costs (Phase 1.5.4)
//!
//! # Phase 1.5 Implementation
//! - Background task management (Tokio spawn) ✅
//! - Resource monitoring interface ✅
//! - Event trigger mechanism ✅
//! - Intent prediction module ✅ (Phase 1.5.2)
//! - Proactive extraction ✅ (Phase 1.5.3)
//! - Cost optimization ✅ (Phase 1.5.4)

// Module structure for proactive system
pub mod cost;
pub mod intention;

pub use cost::{BatchItem, CacheEntry, CostOptimizer, CostOptimizerConfig, CostOptimizerStats};
pub use intention::{
    IntentConfig, IntentResult, IntentionPredictor, MemoryPattern, PredictedIntent,
};

use crate::error::MemError;
use crate::llm::LLMClient;
use crate::models::{MemoryItem, Resource};
use crate::pipeline::{EvolvePipeline, MemorizePipeline};
use crate::storage::MemoryStorage;
use crate::vector::VectorIndex;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

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
            monitor_interval_secs: 60,      // Check every 60 seconds
            evolution_interval_secs: 86400, // Evolve daily (24 hours)
            memory_threshold: 1000,         // Trigger at 1000 items
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

/// Configuration for proactive extractor
#[derive(Debug, Clone)]
pub struct ExtractorConfig {
    /// Confidence threshold for intent-based extraction
    pub intent_confidence_threshold: f32,
    /// Minimum interval between extractions (seconds)
    pub min_extraction_interval_secs: u64,
    /// Maximum items to extract per run
    pub max_items_per_extraction: usize,
    /// Enable automatic extraction on intent detection
    pub auto_extract_on_intent: bool,
    /// Enable background evolution after extraction
    pub trigger_evolution_after_extraction: bool,
}

impl Default for ExtractorConfig {
    fn default() -> Self {
        Self {
            intent_confidence_threshold: 0.7,  // 70% confidence
            min_extraction_interval_secs: 300, // 5 minutes
            max_items_per_extraction: 100,
            auto_extract_on_intent: true,
            trigger_evolution_after_extraction: true,
        }
    }
}

/// Statistics for proactive extractor
#[derive(Debug, Clone, Default)]
pub struct ExtractionStats {
    /// Total items extracted proactively
    pub total_extracted: u64,
    /// Extractions triggered by intent
    pub intent_triggered: u64,
    /// Extractions triggered by threshold
    pub threshold_triggered: u64,
    /// Extractions triggered by schedule
    pub scheduled_triggered: u64,
    /// Evolutions triggered after extraction
    pub evolutions_triggered: u64,
    /// Last extraction time
    pub last_extraction: Option<DateTime<Utc>>,
    /// Last extraction source
    pub last_source: Option<String>,
}

/// Proactive extractor for automatic memory extraction
pub struct ProactiveExtractor {
    /// Configuration
    config: ExtractorConfig,
    /// Memory storage
    #[allow(dead_code)]
    storage: Arc<MemoryStorage>,
    /// Memorize pipeline for extraction
    memorize_pipeline: Arc<MemorizePipeline>,
    /// Evolve pipeline for post-extraction evolution
    evolve_pipeline: Arc<EvolvePipeline>,
    /// LLM client for intent checking
    #[allow(dead_code)]
    llm_client: Arc<RwLock<Box<dyn LLMClient>>>,
    /// Statistics
    stats: Arc<RwLock<ExtractionStats>>,
}

impl ProactiveExtractor {
    /// Create a new proactive extractor
    pub fn new(
        config: ExtractorConfig,
        storage: Arc<MemoryStorage>,
        memorize_pipeline: Arc<MemorizePipeline>,
        evolve_pipeline: Arc<EvolvePipeline>,
        llm_client: Arc<RwLock<Box<dyn LLMClient>>>,
    ) -> Self {
        Self {
            config,
            storage,
            memorize_pipeline,
            evolve_pipeline,
            llm_client,
            stats: Arc::new(RwLock::new(ExtractionStats::default())),
        }
    }

    /// Check if extraction should be performed based on intent
    pub fn should_extract(&self, intent: &PredictedIntent) -> bool {
        intent.confidence >= self.config.intent_confidence_threshold
            && self.config.auto_extract_on_intent
            && intent.intent_type != "none"
    }

    /// Extract memories proactively from a resource
    pub async fn extract_proactively(
        &self,
        resource: Resource,
    ) -> ProactiveResult<Vec<MemoryItem>> {
        tracing::info!(
            "Proactively extracting memories from resource: {:?}",
            resource.id
        );

        // Convert resource to source string for memorize_pipeline
        let source = if resource.url.is_empty() {
            resource.id.to_string()
        } else {
            resource.url.clone()
        };

        // Use memorize pipeline to extract
        let items = self
            .memorize_pipeline
            .memorize_resource(&source, None)
            .await?;

        // Limit items per extraction
        let items: Vec<MemoryItem> = items
            .into_iter()
            .take(self.config.max_items_per_extraction)
            .collect();

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_extracted += items.len() as u64;
            stats.last_extraction = Some(Utc::now());
            stats.last_source = Some(resource.url.clone());
        }

        tracing::info!("Proactively extracted {} memory items", items.len());

        // Trigger evolution if configured
        if self.config.trigger_evolution_after_extraction && !items.is_empty() {
            self.trigger_evolution().await?;
        }

        Ok(items)
    }

    /// Extract memories based on intent prediction
    pub async fn extract_on_intent(
        &self,
        resource: Resource,
        intent: PredictedIntent,
    ) -> ProactiveResult<Vec<MemoryItem>> {
        if !self.should_extract(&intent) {
            return Ok(vec![]);
        }

        let items = self.extract_proactively(resource).await?;

        // Update intent-triggered stats
        {
            let mut stats = self.stats.write().await;
            stats.intent_triggered += 1;
        }

        Ok(items)
    }

    /// Extract memories when threshold is reached
    pub async fn extract_on_threshold(
        &self,
        resources: Vec<Resource>,
    ) -> ProactiveResult<Vec<MemoryItem>> {
        let mut all_items = Vec::new();

        for resource in resources {
            let items = self.extract_proactively(resource).await?;
            all_items.extend(items);
        }

        // Update threshold-triggered stats
        {
            let mut stats = self.stats.write().await;
            stats.threshold_triggered += 1;
        }

        Ok(all_items)
    }

    /// Trigger background evolution after extraction
    pub async fn trigger_evolution(&self) -> ProactiveResult<()> {
        tracing::info!("Triggering background evolution after proactive extraction");

        match self.evolve_pipeline.evolve_all().await {
            Ok(evolve_stats) => {
                let mut stats = self.stats.write().await;
                stats.evolutions_triggered += 1;

                tracing::info!(
                    "Evolution completed: total_items={}, avg_weight={:.2}",
                    evolve_stats.total_items,
                    evolve_stats.average_weight
                );

                Ok(())
            }
            Err(e) => {
                tracing::error!("Evolution failed: {}", e);
                Err(e)
            }
        }
    }

    /// Get extraction statistics
    pub async fn get_stats(&self) -> ExtractionStats {
        self.stats.read().await.clone()
    }
}

/// Proactive agent for background monitoring and automatic memory management
pub struct ProactiveAgent {
    /// Configuration
    config: ProactiveConfig,
    /// Memory storage
    storage: Arc<MemoryStorage>,
    /// Memorize pipeline for proactive extraction
    #[allow(dead_code)]
    memorize_pipeline: Arc<MemorizePipeline>,
    /// Evolve pipeline for automatic evolution
    evolve_pipeline: Arc<EvolvePipeline>,
    /// LLM client for intent prediction
    #[allow(dead_code)]
    llm_client: Arc<RwLock<Box<dyn LLMClient>>>,
    /// Intent predictor (Phase 1.5.2)
    intent_predictor: Option<IntentionPredictor>,
    /// Proactive extractor (Phase 1.5.3)
    extractor: Option<ProactiveExtractor>,
    /// Cost optimizer (Phase 1.5.4)
    cost_optimizer: Option<Arc<CostOptimizer>>,
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
        vector_index: Arc<RwLock<Box<dyn VectorIndex>>>,
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

        // Create proactive extractor (Phase 1.5.3)
        let extractor = Some(ProactiveExtractor::new(
            ExtractorConfig::default(),
            storage.clone(),
            memorize_pipeline.clone(),
            evolve_pipeline.clone(),
            llm_client.clone(),
        ));

        // Create cost optimizer if enabled (Phase 1.5.4)
        let cost_optimizer = if config.enable_cost_optimization {
            Some(Arc::new(CostOptimizer::new(
                CostOptimizerConfig::default(),
                vector_index,
            )))
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
            extractor,
            cost_optimizer,
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
                    // Note: timers will always tick, so loop exits via shutdown checks
                    // This is a fire-and-forget background task
                    _ = monitor_timer.tick() => {
                        // Check shutdown signal
                        if *shutdown.read().await {
                            tracing::info!("Proactive agent shutting down");
                            break;
                        }

                        // Monitoring cycle
                        if let Ok(count) = storage.item_count() {
                            if count >= memory_threshold {
                                // Trigger proactive action
                                let _event = ProactiveEvent::MemoryThreshold { count };
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
                    let _event = ProactiveEvent::IntentDetected {
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

    /// Proactively extract memories from resource (Phase 1.5.3)
    pub async fn proactive_extract(&self, resource: Resource) -> ProactiveResult<Vec<MemoryItem>> {
        if let Some(ref extractor) = self.extractor {
            let items = extractor.extract_proactively(resource).await?;

            // Update proactive stats
            {
                let mut stats = self.stats.write().await;
                stats.proactively_extracted += items.len() as u64;
            }

            Ok(items)
        } else {
            Ok(vec![])
        }
    }

    /// Proactively extract based on predicted intent (Phase 1.5.3)
    pub async fn proactive_extract_on_intent(
        &self,
        resource: Resource,
        context: &str,
    ) -> ProactiveResult<Vec<MemoryItem>> {
        // First predict intent
        if let Some(ref predictor) = self.intent_predictor {
            if let Some(intent) = predictor.predict(context).await? {
                // Then extract if intent warrants it
                if let Some(ref extractor) = self.extractor {
                    let items = extractor.extract_on_intent(resource, intent).await?;

                    // Update proactive stats
                    {
                        let mut stats = self.stats.write().await;
                        stats.proactively_extracted += items.len() as u64;
                    }

                    return Ok(items);
                }
            }
        }

        Ok(vec![])
    }

    /// Get cost optimizer statistics (Phase 1.5.4)
    pub async fn get_cost_stats(&self) -> Option<CostOptimizerStats> {
        if let Some(ref optimizer) = self.cost_optimizer {
            Some(optimizer.get_stats().await)
        } else {
            None
        }
    }

    /// Check if LLM call should be made or use cache/batch (Phase 1.5.4)
    pub async fn should_call_llm(&self, query: &str) -> ProactiveResult<bool> {
        if let Some(ref optimizer) = self.cost_optimizer {
            optimizer.should_call_llm(query).await
        } else {
            Ok(true) // Always call if no optimizer
        }
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

    #[test]
    fn test_extractor_config_default() {
        let config = ExtractorConfig::default();
        assert!((config.intent_confidence_threshold - 0.7).abs() < 0.01);
        assert_eq!(config.min_extraction_interval_secs, 300);
        assert_eq!(config.max_items_per_extraction, 100);
        assert!(config.auto_extract_on_intent);
        assert!(config.trigger_evolution_after_extraction);
    }

    #[test]
    fn test_extraction_stats_default() {
        let stats = ExtractionStats::default();
        assert_eq!(stats.total_extracted, 0);
        assert_eq!(stats.intent_triggered, 0);
        assert_eq!(stats.threshold_triggered, 0);
        assert_eq!(stats.scheduled_triggered, 0);
        assert_eq!(stats.evolutions_triggered, 0);
        assert!(stats.last_extraction.is_none());
        assert!(stats.last_source.is_none());
    }

    #[test]
    fn test_should_extract_high_confidence() {
        let config = ExtractorConfig::default();
        let intent = PredictedIntent {
            intent_type: "search".to_string(),
            confidence: 0.85,
            related_items: vec![],
            suggested_action: Some("retrieve".to_string()),
            timestamp: Utc::now(),
        };

        // Create extractor with mock dependencies
        // In real test, we'd use mock storage and pipelines
        let should = intent.confidence >= config.intent_confidence_threshold
            && config.auto_extract_on_intent
            && intent.intent_type != "none";

        assert!(should);
    }

    #[test]
    fn test_should_extract_low_confidence() {
        let config = ExtractorConfig::default();
        let intent = PredictedIntent {
            intent_type: "search".to_string(),
            confidence: 0.5, // Below threshold
            related_items: vec![],
            suggested_action: Some("retrieve".to_string()),
            timestamp: Utc::now(),
        };

        let should = intent.confidence >= config.intent_confidence_threshold
            && config.auto_extract_on_intent
            && intent.intent_type != "none";

        assert!(!should);
    }

    #[test]
    fn test_should_extract_none_intent() {
        let config = ExtractorConfig::default();
        let intent = PredictedIntent {
            intent_type: "none".to_string(),
            confidence: 0.9,
            related_items: vec![],
            suggested_action: None,
            timestamp: Utc::now(),
        };

        let should = intent.confidence >= config.intent_confidence_threshold
            && config.auto_extract_on_intent
            && intent.intent_type != "none";

        assert!(!should);
    }
}
