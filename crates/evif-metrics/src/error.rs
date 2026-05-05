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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prometheus_error_display() {
        let err = MetricsError::Prometheus("connection refused".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Prometheus error"));
        assert!(msg.contains("connection refused"));
    }

    #[test]
    fn test_metric_not_found_display() {
        let err = MetricsError::MetricNotFound("http_requests".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Metric not found"));
        assert!(msg.contains("http_requests"));
    }

    #[test]
    fn test_invalid_metric_name_display() {
        let err = MetricsError::InvalidMetricName("bad name!".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid metric name"));
        assert!(msg.contains("bad name!"));
    }

    #[test]
    fn test_invalid_label_display() {
        let err = MetricsError::InvalidLabel("label/invalid".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid metric label"));
        assert!(msg.contains("label/invalid"));
    }

    #[test]
    fn test_io_error_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err: MetricsError = io_err.into();
        let msg = format!("{}", err);
        assert!(msg.contains("IO error"));
        assert!(msg.contains("access denied"));
    }

    #[test]
    fn test_serialization_error_display() {
        let err = MetricsError::Serialization("json parse failed".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Serialization error"));
        assert!(msg.contains("json parse failed"));
    }

    #[test]
    fn test_error_debug() {
        let err = MetricsError::Prometheus("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("Prometheus"));
    }

    #[test]
    fn test_metrics_result_ok() {
        let result: MetricsResult<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_metrics_result_err() {
        let result: MetricsResult<i32> = Err(MetricsError::MetricNotFound("x".to_string()));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, MetricsError::MetricNotFound(name) if name == "x"));
    }

    #[test]
    fn test_all_variants_are_distinct() {
        let errors = vec![
            MetricsError::Prometheus("a".to_string()),
            MetricsError::MetricNotFound("b".to_string()),
            MetricsError::InvalidMetricName("c".to_string()),
            MetricsError::InvalidLabel("d".to_string()),
            MetricsError::Io(std::io::Error::new(std::io::ErrorKind::Other, "e")),
            MetricsError::Serialization("f".to_string()),
        ];
        // Verify each variant produces a unique display string prefix
        let prefixes: Vec<String> = errors
            .iter()
            .map(|e| {
                let msg = format!("{}", e);
                // Extract the part before the colon
                msg.split(':')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string()
            })
            .collect();

        // All prefixes should be unique
        for i in 0..prefixes.len() {
            for j in (i + 1)..prefixes.len() {
                assert_ne!(prefixes[i], prefixes[j], "Duplicate prefix: {}", prefixes[i]);
            }
        }
    }
}
