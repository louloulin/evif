// EVIF 监控和指标系统

use std::sync::Arc;
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::Utc;

/// 监控指标类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// 监控指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub metric_type: MetricType,
    pub value: f64,
    pub labels: HashMap<String, String>,
    pub timestamp: i64,
}

/// 系统统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    pub uptime_secs: u64,
    pub total_requests: u64,
    pub active_connections: u64,
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f64,
    pub disk_io_mb: u64,
    pub network_io_mb: u64,
}

/// 插件统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStats {
    pub plugin_name: String,
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub avg_latency_ms: f64,
    pub p99_latency_ms: f64,
}

/// 性能指标收集器
pub struct MetricsCollector {
    metrics: Arc<RwLock<Vec<Metric>>>,
    start_time: Instant,
    counters: Arc<RwLock<HashMap<String, u64>>>,
    gauges: Arc<RwLock<HashMap<String, f64>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(Vec::new())),
            start_time: Instant::now(),
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 记录计数器指标
    pub async fn increment_counter(&self, name: &str, value: u64) {
        let mut counters = self.counters.write().await;
        *counters.entry(name.to_string()).or_insert(0) += value;
    }

    /// 设置仪表盘指标
    pub async fn set_gauge(&self, name: &str, value: f64) {
        let mut gauges = self.gauges.write().await;
        gauges.insert(name.to_string(), value);
    }

    /// 记录直方图指标
    pub async fn record_histogram(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        let metric = Metric {
            name: name.to_string(),
            metric_type: MetricType::Histogram,
            value,
            labels,
            timestamp: Utc::now().timestamp(),
        };

        let mut metrics = self.metrics.write().await;
        metrics.push(metric);
    }

    /// 获取所有计数器
    pub async fn get_counters(&self) -> HashMap<String, u64> {
        self.counters.read().await.clone()
    }

    /// 获取所有仪表盘
    pub async fn get_gauges(&self) -> HashMap<String, f64> {
        self.gauges.read().await.clone()
    }

    /// 获取系统统计信息
    pub async fn get_system_stats(&self) -> SystemStats {
        let uptime = self.start_time.elapsed().as_secs();

        SystemStats {
            uptime_secs: uptime as u64,
            total_requests: self.counters.read().await.get("total_requests").copied().unwrap_or(0),
            active_connections: *self.gauges.read().await.get("active_connections").unwrap_or(&0.0) as u64,
            memory_usage_mb: Self::get_memory_usage(),
            cpu_usage_percent: Self::get_cpu_usage(),
            disk_io_mb: *self.gauges.read().await.get("disk_io_mb").unwrap_or(&0.0) as u64,
            network_io_mb: *self.gauges.read().await.get("network_io_mb").unwrap_or(&0.0) as u64,
        }
    }

    /// 获取内存使用量 (平台相关)
    fn get_memory_usage() -> u64 {
        // 简化实现，实际应该使用sysinfo或其他平台相关库
        #[cfg(target_os = "linux")]
        {
            // Linux实现可以读取 /proc/self/status
            0
        }
        #[cfg(not(target_os = "linux"))]
        {
            0
        }
    }

    /// 获取CPU使用率 (平台相关)
    fn get_cpu_usage() -> f64 {
        // 简化实现，实际应该使用平台相关API
        0.0
    }

    /// 导出Prometheus格式的指标
    pub async fn export_prometheus(&self) -> String {
        let counters = self.counters.read().await;
        let gauges = self.gauges.read().await;
        let metrics = self.metrics.read().await;

        let mut output = String::new();

        // 导出计数器
        for (name, value) in counters.iter() {
            output.push_str(&format!("{}{} {}\n", name, "_total", value));
        }

        // 导出仪表盘
        for (name, value) in gauges.iter() {
            output.push_str(&format!("{} {}\n", name, value));
        }

        // 导出直方图
        for metric in metrics.iter() {
            if matches!(metric.metric_type, MetricType::Histogram) {
                output.push_str(&format!("{}_bucket{{{}}} {}\n",
                    metric.name,
                    metric.labels.iter()
                        .map(|(k, v)| format!("{}=\"{}\"", k, v))
                        .collect::<Vec<_>>()
                        .join(","),
                    metric.value
                ));
            }
        }

        output
    }

    /// 重置指标
    pub async fn reset(&self) {
        self.metrics.write().await.clear();
        self.counters.write().await.clear();
        self.gauges.write().await.clear();
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// 健康检查状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime: u64,
    pub plugins: HashMap<String, PluginHealth>,
}

/// 插件健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHealth {
    pub healthy: bool,
    pub last_check: i64,
    pub error_count: u64,
    pub message: String,
}

/// 性能监控
pub struct PerformanceMonitor {
    metrics_collector: Arc<MetricsCollector>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            metrics_collector: Arc::new(MetricsCollector::new()),
        }
    }

    /// 记录请求延迟
    pub async fn record_latency(&self, operation: &str, latency_ms: f64) {
        let mut labels = HashMap::new();
        labels.insert("operation".to_string(), operation.to_string());

        self.metrics_collector.record_histogram(
            &format!("{}_latency", operation),
            latency_ms,
            labels,
        ).await;
    }

    /// 记录请求
    pub async fn record_request(&self, operation: &str) {
        self.metrics_collector.increment_counter("total_requests", 1).await;
        self.metrics_collector.increment_counter(&format!("{}_requests", operation), 1).await;
    }

    /// 获取指标收集器
    pub fn collector(&self) -> Arc<MetricsCollector> {
        self.metrics_collector.clone()
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collector() {
        let collector = MetricsCollector::new();

        // 测试计数器
        collector.increment_counter("test_counter", 10).await;
        let counters = collector.get_counters().await;
        assert_eq!(counters.get("test_counter"), Some(&10));

        // 测试仪表盘
        collector.set_gauge("test_gauge", 42.0).await;
        let gauges = collector.get_gauges().await;
        assert_eq!(gauges.get("test_gauge"), Some(&42.0));
    }

    #[tokio::test]
    async fn test_system_stats() {
        let collector = MetricsCollector::new();
        let stats = collector.get_system_stats().await;
        assert!(stats.uptime_secs >= 0);
    }

    #[tokio::test]
    async fn test_performance_monitor() {
        let monitor = PerformanceMonitor::new();

        monitor.record_request("test_op").await;
        monitor.record_latency("test_op", 123.45).await;

        let counters = monitor.collector().get_counters().await;
        assert_eq!(counters.get("test_op_requests"), Some(&1));
    }
}
