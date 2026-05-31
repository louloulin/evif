//! EVIF Metrics System
//!
//! Provides metrics collection and export functionality for monitoring EVIF systems.
//! Supports Prometheus metrics export with multiple metric types (Counter, Gauge, Histogram).
//!
//! # P1-4: OpenTelemetry Integration
//!
//! 集成计划:
//! - 添加 `otel` feature flag 到 evif-metrics
//! - 在 evif-rest 中集成 OpenTelemetry tracing
//! - 导出 trace_id 到 Prometheus metrics
//!
//! 配置环境变量:
//! - `OTEL_EXPORTER_OTLP_ENDPOINT`: OTLP 收集器地址
//! - `OTEL_SERVICE_NAME`: 服务名称

pub mod error;
pub mod prometheus;
pub mod traffic;
pub mod types;

pub use error::{MetricsError, MetricsResult};
pub use traffic::{TrafficMonitor, TrafficStats};
pub use types::*;

#[cfg(feature = "prometheus")]
pub use prometheus::PrometheusMetricsRegistry;

/// P1-4: Trace-aware metrics helper
pub trait TraceMetrics {
    /// 记录带 trace_id 的指标
    fn record_with_trace(&self, trace_id: &str, value: f64);
}
