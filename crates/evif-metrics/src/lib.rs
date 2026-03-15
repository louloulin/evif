//! EVIF Metrics System
//!
//! Provides metrics collection and export functionality for monitoring EVIF systems.
//! Supports Prometheus metrics export with multiple metric types (Counter, Gauge, Histogram).

pub mod error;
pub mod prometheus;
pub mod types;

pub use error::{MetricsError, MetricsResult};
pub use types::*;

#[cfg(feature = "prometheus")]
pub use prometheus::PrometheusMetricsRegistry;
