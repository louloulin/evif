//! Intent Prediction Module
//!
//! Predicts user intent based on memory history and current context.
//!
//! # Phase 1.5.2 Implementation
//! - IntentionPredictor: Core prediction engine
//! - Pattern recognition: Find recurring patterns in memory
//! - LLM inference: Use LLM to predict intent from context

use crate::error::MemError;
use crate::llm::LLMClient;
use crate::models::MemoryItem;
use crate::storage::MemoryStorage;
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Result type for intent prediction operations
pub type IntentResult<T> = Result<T, MemError>;

/// Predicted user intent
#[derive(Debug, Clone)]
pub struct PredictedIntent {
    /// Intent type (e.g., "search", "create", "update", "delete")
    pub intent_type: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Related memory items
    pub related_items: Vec<String>,
    /// Suggested action
    pub suggested_action: Option<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Pattern found in memory history
#[derive(Debug, Clone)]
pub struct MemoryPattern {
    /// Pattern type (e.g., "frequent_topic", "time_based", "sequence")
    pub pattern_type: String,
    /// Pattern description
    pub description: String,
    /// Related memory items
    pub item_ids: Vec<String>,
    /// Pattern frequency
    pub frequency: usize,
    /// Last occurrence
    pub last_occurrence: DateTime<Utc>,
}

/// Configuration for intent prediction
#[derive(Debug, Clone)]
pub struct IntentConfig {
    /// Lookback period in hours for pattern analysis
    pub lookback_hours: i64,
    /// Minimum pattern frequency to consider
    pub min_pattern_frequency: usize,
    /// Confidence threshold for predictions
    pub confidence_threshold: f32,
    /// Maximum related items to return
    pub max_related_items: usize,
}

impl Default for IntentConfig {
    fn default() -> Self {
        Self {
            lookback_hours: 24,        // Analyze last 24 hours
            min_pattern_frequency: 2,  // At least 2 occurrences
            confidence_threshold: 0.7, // 70% confidence
            max_related_items: 5,      // Top 5 related items
        }
    }
}

/// Intent predictor for anticipating user needs
pub struct IntentionPredictor {
    /// Configuration
    config: IntentConfig,
    /// Memory storage
    storage: Arc<MemoryStorage>,
    /// LLM client for inference
    llm_client: Arc<RwLock<Box<dyn LLMClient>>>,
}

impl IntentionPredictor {
    /// Create a new intent predictor
    pub fn new(
        config: IntentConfig,
        storage: Arc<MemoryStorage>,
        llm_client: Arc<RwLock<Box<dyn LLMClient>>>,
    ) -> Self {
        Self {
            config,
            storage,
            llm_client,
        }
    }

    /// Predict intent from current context
    pub async fn predict(&self, context: &str) -> IntentResult<Option<PredictedIntent>> {
        // 1. Get recent memories for context
        let recent_memories = self.get_recent_memories().await?;

        // 2. Find patterns in history
        let patterns = self.find_patterns(&recent_memories).await?;

        // 3. Use LLM to infer intent
        let intent = self
            .infer_intent_with_llm(context, &patterns, &recent_memories)
            .await?;

        // 4. Filter by confidence threshold
        if intent.confidence >= self.config.confidence_threshold {
            Ok(Some(intent))
        } else {
            Ok(None)
        }
    }

    /// Get recent memories for analysis
    async fn get_recent_memories(&self) -> IntentResult<Vec<MemoryItem>> {
        let since = Utc::now() - Duration::hours(self.config.lookback_hours);

        // Get all items and filter by time
        let all_items = self.storage.get_all_items();
        let recent: Vec<MemoryItem> = all_items
            .into_iter()
            .filter(|item| item.created_at >= since)
            .collect();

        Ok(recent)
    }

    /// Find patterns in memory history
    pub async fn find_patterns(&self, memories: &[MemoryItem]) -> IntentResult<Vec<MemoryPattern>> {
        let mut patterns = Vec::new();

        if memories.is_empty() {
            return Ok(patterns);
        }

        // 1. Topic frequency patterns
        let topic_patterns = self.analyze_topic_patterns(memories).await?;
        patterns.extend(topic_patterns);

        // 2. Time-based patterns (e.g., daily/weekly routines)
        let time_patterns = self.analyze_time_patterns(memories).await?;
        patterns.extend(time_patterns);

        // 3. Sequence patterns (e.g., action A → action B)
        let sequence_patterns = self.analyze_sequence_patterns(memories).await?;
        patterns.extend(sequence_patterns);

        // Filter by minimum frequency
        patterns.retain(|p| p.frequency >= self.config.min_pattern_frequency);

        Ok(patterns)
    }

    /// Analyze topic frequency patterns
    async fn analyze_topic_patterns(
        &self,
        memories: &[MemoryItem],
    ) -> IntentResult<Vec<MemoryPattern>> {
        use std::collections::HashMap;

        let mut topic_counts: HashMap<String, Vec<String>> = HashMap::new();

        // Group by memory type (as topics)
        for memory in memories {
            let topic = format!("{:?}", memory.memory_type);
            topic_counts
                .entry(topic)
                .or_default()
                .push(memory.id.clone());
        }

        // Convert to patterns
        let patterns: Vec<MemoryPattern> = topic_counts
            .into_iter()
            .filter(|(_, items)| items.len() >= self.config.min_pattern_frequency)
            .map(|(topic, items)| {
                let last_occurrence = memories
                    .iter()
                    .filter(|m| items.contains(&m.id))
                    .map(|m| m.created_at)
                    .max()
                    .unwrap_or_else(Utc::now);

                MemoryPattern {
                    pattern_type: "frequent_topic".to_string(),
                    description: format!("Frequent topic: {}", topic),
                    item_ids: items,
                    frequency: 0, // Will be set below
                    last_occurrence,
                }
            })
            .map(|mut p| {
                p.frequency = p.item_ids.len();
                p
            })
            .collect();

        Ok(patterns)
    }

    /// Analyze time-based patterns
    async fn analyze_time_patterns(
        &self,
        memories: &[MemoryItem],
    ) -> IntentResult<Vec<MemoryPattern>> {
        use chrono::Timelike;
        use std::collections::HashMap;

        let mut hour_counts: HashMap<u32, Vec<String>> = HashMap::new();

        // Group by hour of day
        for memory in memories {
            let hour = memory.created_at.hour();
            hour_counts
                .entry(hour)
                .or_default()
                .push(memory.id.clone());
        }

        // Find peak hours
        let patterns: Vec<MemoryPattern> = hour_counts
            .into_iter()
            .filter(|(_, items)| items.len() >= self.config.min_pattern_frequency)
            .map(|(hour, items)| MemoryPattern {
                pattern_type: "time_based".to_string(),
                description: format!("Active at {}:00", hour),
                item_ids: items.clone(),
                frequency: items.len(),
                last_occurrence: Utc::now(),
            })
            .collect();

        Ok(patterns)
    }

    /// Analyze sequence patterns (action A → action B)
    async fn analyze_sequence_patterns(
        &self,
        memories: &[MemoryItem],
    ) -> IntentResult<Vec<MemoryPattern>> {
        // Sort by creation time
        let mut sorted_memories = memories.to_vec();
        sorted_memories.sort_by_key(|m| m.created_at);

        // Look for sequential patterns in memory types
        use std::collections::HashMap;
        let mut sequence_counts: HashMap<(String, String), Vec<String>> = HashMap::new();

        for window in sorted_memories.windows(2) {
            let prev_type = format!("{:?}", window[0].memory_type);
            let next_type = format!("{:?}", window[1].memory_type);
            let key = (prev_type, next_type);

            sequence_counts
                .entry(key)
                .or_default()
                .push(window[1].id.clone());
        }

        let patterns: Vec<MemoryPattern> = sequence_counts
            .into_iter()
            .filter(|(_, items)| items.len() >= self.config.min_pattern_frequency)
            .map(|((prev, next), items)| MemoryPattern {
                pattern_type: "sequence".to_string(),
                description: format!("Sequence: {} → {}", prev, next),
                item_ids: items.clone(),
                frequency: items.len(),
                last_occurrence: Utc::now(),
            })
            .collect();

        Ok(patterns)
    }

    /// Use LLM to infer intent from context and patterns
    async fn infer_intent_with_llm(
        &self,
        context: &str,
        patterns: &[MemoryPattern],
        recent_memories: &[MemoryItem],
    ) -> IntentResult<PredictedIntent> {
        // Build prompt for LLM
        let pattern_descriptions: Vec<String> = patterns
            .iter()
            .map(|p| format!("- {} (frequency: {})", p.description, p.frequency))
            .collect();

        let memory_summaries: Vec<String> = recent_memories
            .iter()
            .take(10) // Limit to avoid token limits
            .map(|m| format!("- {}", m.summary))
            .collect();

        let prompt = format!(
            r#"Based on the following context and memory patterns, predict the user's next intent.

Context: {}

Recent Memory Patterns:
{}

Recent Memories:
{}

What is the user likely to do next? Respond in the following JSON format:
{{
  "intent_type": "search|create|update|delete|query|other",
  "confidence": 0.0-1.0,
  "suggested_action": "description of suggested action"
}}

Only respond with the JSON object, no additional text."#,
            context,
            pattern_descriptions.join("\n"),
            memory_summaries.join("\n")
        );

        // Call LLM
        let llm = self.llm_client.read().await;
        let response = llm.generate(&prompt).await?;

        // Parse response
        let intent = self.parse_llm_response(&response, recent_memories)?;

        Ok(intent)
    }

    /// Parse LLM response into PredictedIntent
    fn parse_llm_response(
        &self,
        response: &str,
        recent_memories: &[MemoryItem],
    ) -> IntentResult<PredictedIntent> {
        // Simple JSON parsing (in production, use serde_json)
        // For now, extract key fields with basic string parsing

        let intent_type = if response.contains("search") {
            "search"
        } else if response.contains("create") {
            "create"
        } else if response.contains("update") {
            "update"
        } else if response.contains("delete") {
            "delete"
        } else if response.contains("query") {
            "query"
        } else {
            "other"
        }
        .to_string();

        // Extract confidence (simple heuristic)
        let confidence =
            if response.contains("high") || response.contains("0.8") || response.contains("0.9") {
                0.85
            } else if response.contains("medium")
                || response.contains("0.6")
                || response.contains("0.7")
            {
                0.7
            } else {
                0.5
            };

        // Get related items (top recent)
        let related_items: Vec<String> = recent_memories
            .iter()
            .take(self.config.max_related_items)
            .map(|m| m.id.clone())
            .collect();

        // Extract suggested action (simple heuristic)
        let suggested_action = if response.contains("suggested_action") {
            Some(format!("Suggested action based on intent: {}", intent_type))
        } else {
            None
        };

        Ok(PredictedIntent {
            intent_type,
            confidence,
            related_items,
            suggested_action,
            timestamp: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MemoryItem, MemoryType};
    use uuid::Uuid;

    #[test]
    fn test_intent_config_default() {
        let config = IntentConfig::default();
        assert_eq!(config.lookback_hours, 24);
        assert_eq!(config.min_pattern_frequency, 2);
        assert_eq!(config.confidence_threshold, 0.7);
        assert_eq!(config.max_related_items, 5);
    }

    #[test]
    fn test_predicted_intent_creation() {
        let intent = PredictedIntent {
            intent_type: "search".to_string(),
            confidence: 0.85,
            related_items: vec!["item-1".to_string()],
            suggested_action: Some("Search for related items".to_string()),
            timestamp: Utc::now(),
        };

        assert_eq!(intent.intent_type, "search");
        assert_eq!(intent.confidence, 0.85);
        assert_eq!(intent.related_items.len(), 1);
    }

    #[test]
    fn test_memory_pattern_creation() {
        let pattern = MemoryPattern {
            pattern_type: "frequent_topic".to_string(),
            description: "Frequent topic: programming".to_string(),
            item_ids: vec!["item-1".to_string(), "item-2".to_string()],
            frequency: 2,
            last_occurrence: Utc::now(),
        };

        assert_eq!(pattern.pattern_type, "frequent_topic");
        assert_eq!(pattern.frequency, 2);
    }

    #[test]
    fn test_intent_config_custom() {
        let config = IntentConfig {
            lookback_hours: 48,
            min_pattern_frequency: 3,
            confidence_threshold: 0.8,
            max_related_items: 10,
        };

        assert_eq!(config.lookback_hours, 48);
        assert_eq!(config.min_pattern_frequency, 3);
        assert_eq!(config.confidence_threshold, 0.8);
        assert_eq!(config.max_related_items, 10);
    }
}
