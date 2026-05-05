//! Prometheus metrics implementation

#[cfg(feature = "prometheus")]
use {
    super::error::{MetricsError, MetricsResult},
    prometheus::{
        self, CounterVec as PrometheusCounterVec, GaugeVec as PrometheusGaugeVec, HistogramOpts,
        HistogramVec as PrometheusHistogramVec, Opts, Registry as PrometheusRegistry,
    },
    std::collections::HashMap,
    std::sync::Arc,
    tokio::sync::RwLock,
};

/// Prometheus-based registry
#[cfg(feature = "prometheus")]
pub struct PrometheusMetricsRegistry {
    registry: Arc<PrometheusRegistry>,
    counters: Arc<RwLock<HashMap<String, PrometheusCounterVec>>>,
    gauges: Arc<RwLock<HashMap<String, PrometheusGaugeVec>>>,
    histograms: Arc<RwLock<HashMap<String, PrometheusHistogramVec>>>,
}

#[cfg(feature = "prometheus")]
impl Clone for PrometheusMetricsRegistry {
    fn clone(&self) -> Self {
        Self {
            registry: Arc::clone(&self.registry),
            counters: Arc::clone(&self.counters),
            gauges: Arc::clone(&self.gauges),
            histograms: Arc::clone(&self.histograms),
        }
    }
}

#[cfg(feature = "prometheus")]
impl PrometheusMetricsRegistry {
    /// Create a new Prometheus registry
    pub fn new() -> MetricsResult<Self> {
        Ok(Self {
            registry: Arc::new(PrometheusRegistry::new()),
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Register a counter
    pub async fn register_counter(
        &self,
        name: &str,
        help: &str,
        label_names: &[&str],
    ) -> MetricsResult<()> {
        let opts = Opts::new(name, help);
        let counter = PrometheusCounterVec::new(opts, label_names)
            .map_err(|e| MetricsError::Prometheus(e.to_string()))?;

        self.registry
            .register(Box::new(counter.clone()))
            .map_err(|e| MetricsError::Prometheus(e.to_string()))?;

        let mut counters = self.counters.write().await;
        counters.insert(name.to_string(), counter);
        Ok(())
    }

    /// Register a gauge
    pub async fn register_gauge(
        &self,
        name: &str,
        help: &str,
        label_names: &[&str],
    ) -> MetricsResult<()> {
        let opts = Opts::new(name, help);
        let gauge = PrometheusGaugeVec::new(opts, label_names)
            .map_err(|e| MetricsError::Prometheus(e.to_string()))?;

        self.registry
            .register(Box::new(gauge.clone()))
            .map_err(|e| MetricsError::Prometheus(e.to_string()))?;

        let mut gauges = self.gauges.write().await;
        gauges.insert(name.to_string(), gauge);
        Ok(())
    }

    /// Register a histogram
    pub async fn register_histogram(
        &self,
        name: &str,
        help: &str,
        label_names: &[&str],
        buckets: Option<Vec<f64>>,
    ) -> MetricsResult<()> {
        let opts = HistogramOpts::new(name, help).buckets(buckets.unwrap_or_else(|| {
            vec![
                0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]
        }));
        let histogram = PrometheusHistogramVec::new(opts, label_names)
            .map_err(|e| MetricsError::Prometheus(e.to_string()))?;

        self.registry
            .register(Box::new(histogram.clone()))
            .map_err(|e| MetricsError::Prometheus(e.to_string()))?;

        let mut histograms = self.histograms.write().await;
        histograms.insert(name.to_string(), histogram);
        Ok(())
    }

    /// Increment a counter
    pub async fn counter_inc(&self, name: &str, labels: &[&str]) -> MetricsResult<()> {
        let counters = self.counters.read().await;
        let counter = counters
            .get(name)
            .ok_or_else(|| MetricsError::MetricNotFound(name.to_string()))?;
        counter.with_label_values(labels).inc();
        Ok(())
    }

    /// Increment a counter by value
    pub async fn counter_inc_by(
        &self,
        name: &str,
        labels: &[&str],
        value: f64,
    ) -> MetricsResult<()> {
        let counters = self.counters.read().await;
        let counter = counters
            .get(name)
            .ok_or_else(|| MetricsError::MetricNotFound(name.to_string()))?;
        counter.with_label_values(labels).inc_by(value);
        Ok(())
    }

    /// Set a gauge
    pub async fn gauge_set(&self, name: &str, labels: &[&str], value: f64) -> MetricsResult<()> {
        let gauges = self.gauges.read().await;
        let gauge = gauges
            .get(name)
            .ok_or_else(|| MetricsError::MetricNotFound(name.to_string()))?;
        gauge.with_label_values(labels).set(value);
        Ok(())
    }

    /// Increment a gauge
    pub async fn gauge_inc(&self, name: &str, labels: &[&str]) -> MetricsResult<()> {
        let gauges = self.gauges.read().await;
        let gauge = gauges
            .get(name)
            .ok_or_else(|| MetricsError::MetricNotFound(name.to_string()))?;
        gauge.with_label_values(labels).inc();
        Ok(())
    }

    /// Decrement a gauge
    pub async fn gauge_dec(&self, name: &str, labels: &[&str]) -> MetricsResult<()> {
        let gauges = self.gauges.read().await;
        let gauge = gauges
            .get(name)
            .ok_or_else(|| MetricsError::MetricNotFound(name.to_string()))?;
        gauge.with_label_values(labels).dec();
        Ok(())
    }

    /// Observe a histogram value
    pub async fn histogram_observe(
        &self,
        name: &str,
        labels: &[&str],
        value: f64,
    ) -> MetricsResult<()> {
        let histograms = self.histograms.read().await;
        let histogram = histograms
            .get(name)
            .ok_or_else(|| MetricsError::MetricNotFound(name.to_string()))?;
        histogram.with_label_values(labels).observe(value);
        Ok(())
    }

    /// Export metrics in Prometheus text format
    pub fn export(&self) -> MetricsResult<String> {
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder
            .encode_to_string(&metric_families)
            .map_err(|e| MetricsError::Prometheus(e.to_string()))
    }

    /// Get the underlying Prometheus registry
    pub fn registry(&self) -> &PrometheusRegistry {
        &self.registry
    }
}

#[cfg(feature = "prometheus")]
impl Default for PrometheusMetricsRegistry {
    fn default() -> Self {
        Self::new().expect("Failed to create Prometheus registry")
    }
}

/// No-op metrics implementation (when prometheus feature is disabled)
#[cfg(not(feature = "prometheus"))]
pub struct PrometheusMetricsRegistry;

#[cfg(not(feature = "prometheus"))]
impl PrometheusMetricsRegistry {
    pub fn new() -> MetricsResult<Self> {
        Ok(Self)
    }

    pub async fn register_counter(
        &self,
        _name: &str,
        _help: &str,
        _label_names: &[&str],
    ) -> MetricsResult<()> {
        Ok(())
    }

    pub async fn register_gauge(
        &self,
        _name: &str,
        _help: &str,
        _label_names: &[&str],
    ) -> MetricsResult<()> {
        Ok(())
    }

    pub async fn register_histogram(
        &self,
        _name: &str,
        _help: &str,
        _label_names: &[&str],
        _buckets: Option<Vec<f64>>,
    ) -> MetricsResult<()> {
        Ok(())
    }

    pub async fn counter_inc(&self, _name: &str, _labels: &[&str]) -> MetricsResult<()> {
        Ok(())
    }

    pub async fn counter_inc_by(
        &self,
        _name: &str,
        _labels: &[&str],
        _value: f64,
    ) -> MetricsResult<()> {
        Ok(())
    }

    pub async fn gauge_set(&self, _name: &str, _labels: &[&str], _value: f64) -> MetricsResult<()> {
        Ok(())
    }

    pub async fn gauge_inc(&self, _name: &str, _labels: &[&str]) -> MetricsResult<()> {
        Ok(())
    }

    pub async fn gauge_dec(&self, _name: &str, _labels: &[&str]) -> MetricsResult<()> {
        Ok(())
    }

    pub async fn histogram_observe(
        &self,
        _name: &str,
        _labels: &[&str],
        _value: f64,
    ) -> MetricsResult<()> {
        Ok(())
    }

    pub fn export(&self) -> MetricsResult<String> {
        Ok(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_creation() {
        let registry = PrometheusMetricsRegistry::new();
        assert!(registry.is_ok());
    }

    #[tokio::test]
    async fn test_register_counter() {
        let registry = PrometheusMetricsRegistry::new().unwrap();
        let result = registry
            .register_counter("test_counter", "A test counter", &["method"])
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_register_gauge() {
        let registry = PrometheusMetricsRegistry::new().unwrap();
        let result = registry
            .register_gauge("test_gauge", "A test gauge", &["path"])
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_register_histogram() {
        let registry = PrometheusMetricsRegistry::new().unwrap();
        let result = registry
            .register_histogram("test_histogram", "A test histogram", &["endpoint"], None)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_register_histogram_custom_buckets() {
        let registry = PrometheusMetricsRegistry::new().unwrap();
        let result = registry
            .register_histogram(
                "test_histogram_buckets",
                "Histogram with custom buckets",
                &["status"],
                Some(vec![0.1, 0.5, 1.0, 5.0]),
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_counter_inc_nonexistent() {
        let registry = PrometheusMetricsRegistry::new().unwrap();
        let result = registry.counter_inc("nonexistent", &[]).await;
        assert!(result.is_err());
        if let Err(e) = result {
            let msg = format!("{}", e);
            assert!(msg.contains("nonexistent") || msg.contains("Metric not found"));
        }
    }

    #[tokio::test]
    async fn test_gauge_set_nonexistent() {
        let registry = PrometheusMetricsRegistry::new().unwrap();
        let result = registry.gauge_set("nonexistent", &[], 1.0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_counter_increment_and_export() {
        let registry = PrometheusMetricsRegistry::new().unwrap();
        registry
            .register_counter("export_test_counter", "Counter for export test", &["method"])
            .await
            .unwrap();

        registry
            .counter_inc("export_test_counter", &["GET"])
            .await
            .unwrap();
        registry
            .counter_inc("export_test_counter", &["GET"])
            .await
            .unwrap();
        registry
            .counter_inc_by("export_test_counter", &["POST"], 5.0)
            .await
            .unwrap();

        let output = registry.export().unwrap();
        assert!(output.contains("export_test_counter"));
    }

    #[tokio::test]
    async fn test_gauge_operations_and_export() {
        let registry = PrometheusMetricsRegistry::new().unwrap();
        registry
            .register_gauge("export_test_gauge", "Gauge for export test", &["label"])
            .await
            .unwrap();

        registry
            .gauge_set("export_test_gauge", &["a"], 100.0)
            .await
            .unwrap();
        registry
            .gauge_inc("export_test_gauge", &["a"])
            .await
            .unwrap();
        registry
            .gauge_dec("export_test_gauge", &["a"])
            .await
            .unwrap();

        let output = registry.export().unwrap();
        assert!(output.contains("export_test_gauge"));
    }

    #[tokio::test]
    async fn test_histogram_observe_and_export() {
        let registry = PrometheusMetricsRegistry::new().unwrap();
        registry
            .register_histogram(
                "export_test_histogram",
                "Histogram for export test",
                &["endpoint"],
                None,
            )
            .await
            .unwrap();

        for val in [0.01, 0.1, 0.5, 1.0, 2.5] {
            registry
                .histogram_observe("export_test_histogram", &["/api/test"], val)
                .await
                .unwrap();
        }

        let output = registry.export().unwrap();
        assert!(output.contains("export_test_histogram"));
    }

    #[tokio::test]
    async fn test_export_empty_registry() {
        let registry = PrometheusMetricsRegistry::new().unwrap();
        let output = registry.export().unwrap();
        // Empty registry should produce an empty or minimal output
        assert!(output.len() <= 1 || output.trim().is_empty() || !output.contains("HELP"));
    }

    #[tokio::test]
    async fn test_histogram_observe_nonexistent() {
        let registry = PrometheusMetricsRegistry::new().unwrap();
        let result = registry.histogram_observe("missing", &[], 1.0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_gauge_inc_dec_nonexistent() {
        let registry = PrometheusMetricsRegistry::new().unwrap();
        assert!(registry.gauge_inc("nope", &[]).await.is_err());
        assert!(registry.gauge_dec("nope", &[]).await.is_err());
    }
}
