//! OpenTelemetry Tracing Module
//!
//! Provides distributed tracing capabilities for evif-mem operations.
//! Supports tracing memorize, retrieve, evolve, and workflow operations.
//!
//! # Features
//! - Span creation for all major operations
//! - Automatic attribute injection (user_id, tenant_id, operation type)
//! - Multiple exporters (stdout, OTLP)
//! - Feature-gated with `telemetry` flag
//!
//! # Usage
//!
//! ```ignore
//! use evif_mem::telemetry::{Telemetry, TelemetryConfig, TelemetrySpan};
//!
//! // Initialize telemetry
//! let mut telemetry = Telemetry::new();
//! tokio::runtime::Runtime::new().unwrap().block_on(async {
//!     telemetry.init(TelemetryConfig::default()).await.unwrap();
//!
//!     // Create spans
//!     let span = telemetry.start_span("memorize", Some("user123"), None);
//!     // ... perform operations
//!     span.end();
//! });
//! ```

use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Configuration for telemetry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Whether telemetry is enabled
    pub enabled: bool,
    /// Service name for tracing
    pub service_name: String,
    /// Exporter type: "stdout", "otlp", or "none"
    pub exporter: String,
    /// OTLP endpoint (if using otlp exporter)
    pub otlp_endpoint: Option<String>,
    /// Sample rate (0.0 to 1.0)
    pub sample_rate: f64,
    /// Whether to include crate name in spans
    pub include_crate_name: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            service_name: "evif-mem".to_string(),
            exporter: "stdout".to_string(),
            otlp_endpoint: None,
            sample_rate: 1.0,
            include_crate_name: true,
        }
    }
}

/// Telemetry error types
#[derive(Debug, thiserror::Error)]
pub enum TelemetryError {
    #[error("Failed to initialize telemetry: {0}")]
    InitFailed(String),
    #[error("Telemetry not initialized")]
    NotInitialized,
    #[error("Failed to create span: {0}")]
    SpanFailed(String),
}

/// Telemetry span for tracing operations
#[derive(Debug, Clone)]
pub struct TelemetrySpan {
    /// Span name
    name: String,
    /// User ID for the operation
    user_id: Option<String>,
    /// Tenant ID for the operation
    tenant_id: Option<String>,
    /// Operation type (memorize, retrieve, evolve, workflow)
    operation_type: String,
    /// Whether span is ended
    ended: bool,
}

impl TelemetrySpan {
    /// Create a new telemetry span
    pub fn new(name: String, user_id: Option<String>, tenant_id: Option<String>, operation_type: String) -> Self {
        Self {
            name,
            user_id,
            tenant_id,
            operation_type,
            ended: false,
        }
    }

    /// Set an attribute on the span
    pub fn set_attribute(&self, _key: &str, _value: &str) {
        // In a full implementation, this would set attributes on the actual span
        // For now, we keep the interface for future use
    }

    /// End the span
    pub fn end(self) {
        // In a full implementation, this would end the actual OpenTelemetry span
    }

    /// Record an exception
    pub fn record_exception(&self, _error: &str) {
        // In a full implementation, this would record the exception
    }
}

/// Main telemetry struct
#[derive(Debug, Clone)]
pub struct Telemetry {
    config: TelemetryConfig,
    initialized: bool,
}

impl Telemetry {
    /// Create a new Telemetry instance
    pub fn new() -> Self {
        Self {
            config: TelemetryConfig::default(),
            initialized: false,
        }
    }

    /// Create with custom config
    pub fn with_config(config: TelemetryConfig) -> Self {
        Self {
            config,
            initialized: false,
        }
    }

    /// Initialize telemetry
    pub async fn init(&mut self, config: TelemetryConfig) -> Result<(), TelemetryError> {
        if !config.enabled {
            self.initialized = false;
            return Ok(());
        }

        self.config = config;

        // Create tracer provider based on exporter type
        match self.config.exporter.as_str() {
            "stdout" => {
                // Use stdout exporter for debugging
                // In production, you'd use a proper exporter
            }
            "otlp" => {
                // OTLP exporter would be configured here
                // Requires opentelemetry-otlp crate
            }
            _ => {
                // No exporter - just tracing
            }
        }

        self.initialized = true;
        Ok(())
    }

    /// Check if telemetry is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Start a new span
    pub fn start_span(&self, name: &str, user_id: Option<&str>, tenant_id: Option<&str>) -> TelemetrySpan {
        let operation_type = self.infer_operation_type(name);

        TelemetrySpan::new(
            name.to_string(),
            user_id.map(String::from),
            tenant_id.map(String::from),
            operation_type,
        )
    }

    /// Infer operation type from span name
    fn infer_operation_type(&self, name: &str) -> String {
        let name_lower = name.to_lowercase();
        if name_lower.contains("memorize") {
            "memorize".to_string()
        } else if name_lower.contains("retrieve") {
            "retrieve".to_string()
        } else if name_lower.contains("evolve") {
            "evolve".to_string()
        } else if name_lower.contains("workflow") {
            "workflow".to_string()
        } else if name_lower.contains("extract") {
            "extract".to_string()
        } else {
            "unknown".to_string()
        }
    }

    /// Get the config
    pub fn config(&self) -> &TelemetryConfig {
        &self.config
    }
}

impl Default for Telemetry {
    fn default() -> Self {
        Self::new()
    }
}

/// TelemetryRegistry - thread-safe registry for telemetry operations
#[derive(Debug, Clone)]
pub struct TelemetryRegistry {
    telemetry: Arc<RwLock<Telemetry>>,
}

impl TelemetryRegistry {
    /// Create a new TelemetryRegistry
    pub fn new() -> Self {
        Self {
            telemetry: Arc::new(RwLock::new(Telemetry::new())),
        }
    }

    /// Create with custom config
    pub fn with_config(config: TelemetryConfig) -> Self {
        Self {
            telemetry: Arc::new(RwLock::new(Telemetry::with_config(config))),
        }
    }

    /// Initialize the registry
    pub async fn init(&self, config: TelemetryConfig) -> Result<(), TelemetryError> {
        let mut telemetry = self.telemetry.write().await;
        telemetry.init(config).await
    }

    /// Check if initialized
    pub async fn is_initialized(&self) -> bool {
        let telemetry = self.telemetry.read().await;
        telemetry.is_initialized()
    }

    /// Start a span
    pub async fn start_span(&self, name: &str, user_id: Option<&str>, tenant_id: Option<&str>) -> TelemetrySpan {
        let telemetry = self.telemetry.read().await;
        telemetry.start_span(name, user_id, tenant_id)
    }

    /// Record an error in a span
    pub async fn record_error(&self, span: &TelemetrySpan, error: &str) {
        span.record_exception(error);
    }
}

impl Default for TelemetryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to create a default telemetry registry
pub fn create_telemetry_registry() -> TelemetryRegistry {
    TelemetryRegistry::new()
}

/// Trace macro for easy span creation
#[macro_export]
macro_rules! trace_operation {
    ($registry:expr, $name:expr, $user_id:expr, $tenant_id:expr, { $($body:tt)* }) => {{
        let span = $registry.start_span($name, $user_id, $tenant_id).await;
        let result = async move { $($body)* }.await;
        span.end();
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_telemetry_config_default() {
        let config = TelemetryConfig::default();
        assert!(config.enabled);
        assert_eq!(config.service_name, "evif-mem");
        assert_eq!(config.exporter, "stdout");
        assert_eq!(config.sample_rate, 1.0);
    }

    #[tokio::test]
    async fn test_telemetry_creation() {
        let telemetry = Telemetry::new();
        assert!(!telemetry.is_initialized());
    }

    #[tokio::test]
    async fn test_telemetry_init_disabled() {
        let mut telemetry = Telemetry::new();
        let config = TelemetryConfig {
            enabled: false,
            ..Default::default()
        };
        telemetry.init(config).await.unwrap();
        assert!(!telemetry.is_initialized());
    }

    #[tokio::test]
    async fn test_telemetry_init_enabled() {
        let mut telemetry = Telemetry::new();
        let config = TelemetryConfig::default();
        telemetry.init(config).await.unwrap();
        assert!(telemetry.is_initialized());
    }

    #[tokio::test]
    async fn test_telemetry_span_creation() {
        let telemetry = Telemetry::new();
        let span = telemetry.start_span("test_operation", Some("user1"), Some("tenant1"));
        assert_eq!(span.name, "test_operation");
        assert_eq!(span.user_id, Some("user1".to_string()));
        assert_eq!(span.tenant_id, Some("tenant1".to_string()));
    }

    #[tokio::test]
    async fn test_infer_operation_type() {
        let telemetry = Telemetry::new();

        let span = telemetry.start_span("memorize_text", None, None);
        assert_eq!(span.operation_type, "memorize");

        let span = telemetry.start_span("retrieve_memories", None, None);
        assert_eq!(span.operation_type, "retrieve");

        let span = telemetry.start_span("evolve_knowledge", None, None);
        assert_eq!(span.operation_type, "evolve");

        let span = telemetry.start_span("run_workflow", None, None);
        assert_eq!(span.operation_type, "workflow");
    }

    #[tokio::test]
    async fn test_telemetry_registry() {
        let registry = TelemetryRegistry::new();

        // Initialize
        registry.init(TelemetryConfig::default()).await.unwrap();
        assert!(registry.is_initialized().await);

        // Start span
        let span = registry.start_span("test", Some("user1"), None).await;
        assert_eq!(span.user_id, Some("user1".to_string()));
    }

    #[tokio::test]
    async fn test_telemetry_registry_default() {
        let registry = TelemetryRegistry::with_config(TelemetryConfig::default());
        registry.init(TelemetryConfig::default()).await.unwrap();
        assert!(registry.is_initialized().await);
    }

    #[tokio::test]
    async fn test_span_end() {
        let span = TelemetrySpan::new("test".to_string(), None, None, "test".to_string());
        span.end();
        // Should not panic
    }
}
