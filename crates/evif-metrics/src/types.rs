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

#[cfg(test)]
mod tests {
    use super::*;

    // --- MetricType tests ---

    #[test]
    fn test_metric_type_variants() {
        let counter = MetricType::Counter;
        let gauge = MetricType::Gauge;
        let histogram = MetricType::Histogram;
        let summary = MetricType::Summary;

        assert_eq!(counter, MetricType::Counter);
        assert_eq!(gauge, MetricType::Gauge);
        assert_eq!(histogram, MetricType::Histogram);
        assert_eq!(summary, MetricType::Summary);
    }

    #[test]
    fn test_metric_type_equality_and_inequality() {
        assert_eq!(MetricType::Counter, MetricType::Counter);
        assert_ne!(MetricType::Counter, MetricType::Gauge);
        assert_ne!(MetricType::Histogram, MetricType::Summary);
        assert_ne!(MetricType::Gauge, MetricType::Histogram);
    }

    #[test]
    fn test_metric_type_copy() {
        let a = MetricType::Counter;
        let b = a; // Copy semantics
        assert_eq!(a, b);
    }

    #[test]
    fn test_metric_type_debug() {
        assert_eq!(format!("{:?}", MetricType::Counter), "Counter");
        assert_eq!(format!("{:?}", MetricType::Gauge), "Gauge");
        assert_eq!(format!("{:?}", MetricType::Histogram), "Histogram");
        assert_eq!(format!("{:?}", MetricType::Summary), "Summary");
    }

    // --- MetricValue tests ---

    #[test]
    fn test_metric_value_counter() {
        let val = MetricValue::Counter(42.0);
        if let MetricValue::Counter(v) = val {
            assert!((v - 42.0).abs() < f64::EPSILON);
        } else {
            panic!("Expected Counter variant");
        }
    }

    #[test]
    fn test_metric_value_gauge() {
        let val = MetricValue::Gauge(-5.5);
        if let MetricValue::Gauge(v) = val {
            assert!((v - (-5.5)).abs() < f64::EPSILON);
        } else {
            panic!("Expected Gauge variant");
        }
    }

    #[test]
    fn test_metric_value_histogram() {
        let val = MetricValue::Histogram {
            sample_count: 100,
            sample_sum: 250.0,
            buckets: vec![(1.0, 10), (5.0, 50), (10.0, 100)],
        };
        if let MetricValue::Histogram {
            sample_count,
            sample_sum,
            buckets,
        } = val
        {
            assert_eq!(sample_count, 100);
            assert!((sample_sum - 250.0).abs() < f64::EPSILON);
            assert_eq!(buckets.len(), 3);
            assert_eq!(buckets[0], (1.0, 10));
        } else {
            panic!("Expected Histogram variant");
        }
    }

    #[test]
    fn test_metric_value_summary() {
        let val = MetricValue::Summary {
            sample_count: 500,
            sample_sum: 1234.5,
            quantiles: vec![(0.5, 2.0), (0.9, 5.0), (0.99, 10.0)],
        };
        if let MetricValue::Summary {
            sample_count,
            sample_sum,
            quantiles,
        } = val
        {
            assert_eq!(sample_count, 500);
            assert!((sample_sum - 1234.5).abs() < f64::EPSILON);
            assert_eq!(quantiles.len(), 3);
            assert_eq!(quantiles[0], (0.5, 2.0));
            assert_eq!(quantiles[2], (0.99, 10.0));
        } else {
            panic!("Expected Summary variant");
        }
    }

    #[test]
    fn test_metric_value_clone() {
        let val = MetricValue::Counter(7.0);
        let cloned = val.clone();
        assert!(matches!(cloned, MetricValue::Counter(v) if (v - 7.0).abs() < f64::EPSILON));
    }

    #[test]
    fn test_metric_value_debug() {
        let val = MetricValue::Counter(1.0);
        let debug_str = format!("{:?}", val);
        assert!(debug_str.contains("Counter"));

        let val = MetricValue::Gauge(0.0);
        let debug_str = format!("{:?}", val);
        assert!(debug_str.contains("Gauge"));
    }

    // --- MetricFamily / MetricData tests ---

    #[test]
    fn test_metric_family_construction() {
        let family = MetricFamily {
            name: "http_requests_total".to_string(),
            help: "Total HTTP requests".to_string(),
            metric_type: MetricType::Counter,
            metrics: vec![MetricData {
                labels: {
                    let mut l = Label::new();
                    l.insert("method".to_string(), "GET".to_string());
                    l
                },
                value: MetricValue::Counter(100.0),
            }],
        };
        assert_eq!(family.name, "http_requests_total");
        assert_eq!(family.help, "Total HTTP requests");
        assert_eq!(family.metric_type, MetricType::Counter);
        assert_eq!(family.metrics.len(), 1);
    }

    #[test]
    fn test_metric_family_clone() {
        let family = MetricFamily {
            name: "test_metric".to_string(),
            help: "A test".to_string(),
            metric_type: MetricType::Gauge,
            metrics: vec![],
        };
        let cloned = family.clone();
        assert_eq!(cloned.name, family.name);
        assert_eq!(cloned.help, family.help);
        assert_eq!(cloned.metric_type, family.metric_type);
        assert_eq!(cloned.metrics.len(), 0);
    }

    #[test]
    fn test_metric_family_debug() {
        let family = MetricFamily {
            name: "test".to_string(),
            help: "help".to_string(),
            metric_type: MetricType::Histogram,
            metrics: vec![],
        };
        let debug = format!("{:?}", family);
        assert!(debug.contains("test"));
        assert!(debug.contains("Histogram"));
    }

    #[test]
    fn test_metric_data_with_empty_labels() {
        let data = MetricData {
            labels: Label::new(),
            value: MetricValue::Gauge(0.0),
        };
        assert!(data.labels.is_empty());
    }

    #[test]
    fn test_metric_data_with_multiple_labels() {
        let mut labels = Label::new();
        labels.insert("method".to_string(), "POST".to_string());
        labels.insert("path".to_string(), "/api/v1/data".to_string());
        labels.insert("status".to_string(), "200".to_string());

        let data = MetricData {
            labels,
            value: MetricValue::Counter(42.0),
        };
        assert_eq!(data.labels.len(), 3);
        assert_eq!(data.labels.get("method").unwrap(), "POST");
        assert_eq!(data.labels.get("path").unwrap(), "/api/v1/data");
        assert_eq!(data.labels.get("status").unwrap(), "200");
    }
}
