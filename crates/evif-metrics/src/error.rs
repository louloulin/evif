//! Error types for metrics system

#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    #[error("Prometheus error: {0}")]
    Prometheus(String),

    #[error("Metric not found: {0}")]
    MetricNotFound(String),

    #[error("Invalid metric name: {0}")]
    InvalidMetricName(String),

    #[error("Invalid metric label: {0}")]
    InvalidLabel(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

pub type MetricsResult<T> = Result<T, MetricsError>;
