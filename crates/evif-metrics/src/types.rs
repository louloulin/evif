//! Core metric types and traits

use crate::error::MetricsResult;
use async_trait::async_trait;
use std::collections::HashMap;

/// Metric types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// A metric label
pub type Label = HashMap<String, String>;

/// Base metric trait
pub trait Metric: Send + Sync {
    /// Get metric name
    fn name(&self) -> &str;

    /// Get metric description
    fn help(&self) -> &str;

    /// Get metric type
    fn metric_type(&self) -> MetricType;

    /// Get metric labels
    fn labels(&self) -> &Label;

    /// Clear the metric
    fn clear(&self);
}

/// Counter metric - monotonically increasing
pub trait Counter: Metric {
    /// Increment the counter by 1
    fn inc(&self) -> MetricsResult<()>;

    /// Increment the counter by a specific amount
    fn inc_by(&self, value: f64) -> MetricsResult<()>;

    /// Get the current value
    fn get(&self) -> MetricsResult<f64>;
}

/// Gauge metric - can go up or down
pub trait Gauge: Metric {
    /// Increment the gauge by 1
    fn inc(&self) -> MetricsResult<()>;

    /// Decrement the gauge by 1
    fn dec(&self) -> MetricsResult<()>;

    /// Set the gauge to a specific value
    fn set(&self, value: f64) -> MetricsResult<()>;

    /// Increment by a specific amount
    fn add(&self, value: f64) -> MetricsResult<()>;

    /// Decrement by a specific amount
    fn sub(&self, value: f64) -> MetricsResult<()>;

    /// Get the current value
    fn get(&self) -> MetricsResult<f64>;
}

/// Histogram metric - count observations in buckets
pub trait Histogram: Metric {
    /// Observe a value
    fn observe(&self, value: f64) -> MetricsResult<()>;

    /// Get the sample count
    fn sample_count(&self) -> MetricsResult<u64>;

    /// Get the sample sum
    fn sample_sum(&self) -> MetricsResult<f64>;
}

/// Summary metric - calculate quantiles over a sliding time window
pub trait Summary: Metric {
    /// Observe a value
    fn observe(&self, value: f64) -> MetricsResult<()>;

    /// Get the sample count
    fn sample_count(&self) -> MetricsResult<u64>;

    /// Get the sample sum
    fn sample_sum(&self) -> MetricsResult<f64>;

    /// Get a specific quantile
    fn quantile(&self, q: f64) -> MetricsResult<f64>;
}

/// Registry for managing metrics
#[async_trait]
pub trait Registry: Send + Sync {
    /// Register a new metric
    fn register(&self, metric: Box<dyn Metric>) -> MetricsResult<()>;

    /// Unregister a metric
    fn unregister(&self, name: &str) -> MetricsResult<()>;

    /// Get a metric by name
    fn get(&self, name: &str) -> MetricsResult<Box<dyn Metric>>;

    /// Gather all metrics for export
    fn gather(&self) -> MetricsResult<Vec<MetricFamily>>;

    /// Clear all metrics
    fn clear(&self) -> MetricsResult<()>;
}

/// A family of metrics with the same name and help
#[derive(Debug, Clone)]
pub struct MetricFamily {
    pub name: String,
    pub help: String,
    pub metric_type: MetricType,
    pub metrics: Vec<MetricData>,
}

/// Metric data with labels
#[derive(Debug, Clone)]
pub struct MetricData {
    pub labels: Label,
    pub value: MetricValue,
}

/// Metric value
#[derive(Debug, Clone)]
pub enum MetricValue {
    Counter(f64),
    Gauge(f64),
    Histogram {
        sample_count: u64,
        sample_sum: f64,
        buckets: Vec<(f64, u64)>,
    },
    Summary {
        sample_count: u64,
        sample_sum: f64,
        quantiles: Vec<(f64, f64)>,
    },
}
