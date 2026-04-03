//! Integration tests for metrics system

use evif_metrics::PrometheusMetricsRegistry;

#[tokio::test]
async fn test_registry_creation() {
    let registry = PrometheusMetricsRegistry::new();
    assert!(registry.is_ok());
}

#[tokio::test]
async fn test_counter_registration() {
    let registry = PrometheusMetricsRegistry::new().unwrap();

    let result = registry
        .register_counter("test_counter", "A test counter", &["label1", "label2"])
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_counter_increment() {
    let registry = PrometheusMetricsRegistry::new().unwrap();

    registry
        .register_counter("test_counter_inc", "A test counter", &["method", "status"])
        .await
        .unwrap();

    let result = registry
        .counter_inc("test_counter_inc", &["GET", "200"])
        .await;
    assert!(result.is_ok());

    let result = registry
        .counter_inc_by("test_counter_inc", &["POST", "201"], 5.0)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_gauge_registration() {
    let registry = PrometheusMetricsRegistry::new().unwrap();

    let result = registry
        .register_gauge("test_gauge", "A test gauge", &["label1"])
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_gauge_operations() {
    let registry = PrometheusMetricsRegistry::new().unwrap();

    registry
        .register_gauge("test_gauge_ops", "A test gauge", &["service"])
        .await
        .unwrap();

    // Test set
    let result = registry.gauge_set("test_gauge_ops", &["api"], 42.0).await;
    assert!(result.is_ok());

    // Test inc
    let result = registry.gauge_inc("test_gauge_ops", &["api"]).await;
    assert!(result.is_ok());

    // Test dec
    let result = registry.gauge_dec("test_gauge_ops", &["api"]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_histogram_registration() {
    let registry = PrometheusMetricsRegistry::new().unwrap();

    let result = registry
        .register_histogram("test_histogram", "A test histogram", &["operation"], None)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_histogram_observe() {
    let registry = PrometheusMetricsRegistry::new().unwrap();

    registry
        .register_histogram(
            "test_histogram_obs",
            "A test histogram",
            &["operation"],
            None,
        )
        .await
        .unwrap();

    let result = registry
        .histogram_observe("test_histogram_obs", &["read"], 0.123)
        .await;
    assert!(result.is_ok());

    let result = registry
        .histogram_observe("test_histogram_obs", &["write"], 0.456)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_export_metrics() {
    let registry = PrometheusMetricsRegistry::new().unwrap();

    // Register some metrics
    registry
        .register_counter("export_test_counter", "Test counter", &[])
        .await
        .unwrap();

    registry
        .register_gauge("export_test_gauge", "Test gauge", &[])
        .await
        .unwrap();

    // Increment counter
    registry
        .counter_inc("export_test_counter", &[])
        .await
        .unwrap();

    // Set gauge
    registry
        .gauge_set("export_test_gauge", &[], 100.0)
        .await
        .unwrap();

    // Export
    let result = registry.export();
    assert!(result.is_ok());

    let exported = result.unwrap();
    assert!(exported.contains("export_test_counter"));
    assert!(exported.contains("export_test_gauge"));
}

#[tokio::test]
async fn test_custom_histogram_buckets() {
    let registry = PrometheusMetricsRegistry::new().unwrap();

    let custom_buckets = vec![0.1, 0.5, 1.0, 5.0, 10.0];
    let result = registry
        .register_histogram(
            "custom_buckets",
            "Histogram with custom buckets",
            &["operation"],
            Some(custom_buckets),
        )
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_multiple_labels() {
    let registry = PrometheusMetricsRegistry::new().unwrap();

    registry
        .register_counter(
            "multi_label_counter",
            "Counter with multiple labels",
            &["method", "status", "endpoint"],
        )
        .await
        .unwrap();

    // Test different label combinations
    registry
        .counter_inc("multi_label_counter", &["GET", "200", "/api/nodes"])
        .await
        .unwrap();

    registry
        .counter_inc("multi_label_counter", &["POST", "201", "/api/nodes"])
        .await
        .unwrap();

    registry
        .counter_inc("multi_label_counter", &["GET", "404", "/api/edges"])
        .await
        .unwrap();

    assert!(registry.export().unwrap().contains("multi_label_counter"));
}

#[tokio::test]
async fn test_metric_not_found() {
    let registry = PrometheusMetricsRegistry::new().unwrap();

    // Try to increment a non-existent metric
    let result = registry.counter_inc("non_existent", &[]).await;
    assert!(result.is_err());

    // Try to set a non-existent gauge
    let result = registry.gauge_set("non_existent", &[], 10.0).await;
    assert!(result.is_err());

    // Try to observe a non-existent histogram
    let result = registry.histogram_observe("non_existent", &[], 1.0).await;
    assert!(result.is_err());
}
