// EVIF Traffic Monitoring REST API Handlers
//
// 流量监控 HTTP 接口
// 对标 AGFS Traffic Monitor

use axum::{
    extract::State,
    Json,
};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use evif_core::MountTable;

/// 应用状态
#[derive(Clone)]
pub struct MetricsState {
    pub mount_table: Arc<MountTable>,
    pub traffic_stats: Arc<TrafficStats>,
    pub start_time: std::time::Instant,
}

/// 流量统计
#[derive(Debug, Default)]
pub struct TrafficStats {
    pub total_requests: AtomicU64,
    pub total_bytes_read: AtomicU64,
    pub total_bytes_written: AtomicU64,
    pub total_errors: AtomicU64,
    pub read_count: AtomicU64,
    pub write_count: AtomicU64,
    pub list_count: AtomicU64,
    pub other_count: AtomicU64,
    pub read_success_count: AtomicU64,
    pub read_error_count: AtomicU64,
    pub read_latency_micros_total: AtomicU64,
    pub write_success_count: AtomicU64,
    pub write_error_count: AtomicU64,
    pub write_latency_micros_total: AtomicU64,
    pub list_success_count: AtomicU64,
    pub list_error_count: AtomicU64,
    pub list_latency_micros_total: AtomicU64,
    pub other_success_count: AtomicU64,
    pub other_error_count: AtomicU64,
    pub other_latency_micros_total: AtomicU64,
}

/// 流量统计响应
#[derive(Debug, Serialize)]
pub struct TrafficStatsResponse {
    pub total_requests: u64,
    pub total_bytes_read: u64,
    pub total_bytes_written: u64,
    pub total_errors: u64,
    pub read_count: u64,
    pub write_count: u64,
    pub list_count: u64,
    pub other_count: u64,
    pub average_read_size: u64,
    pub average_write_size: u64,
}

/// 操作统计
#[derive(Debug, Serialize)]
pub struct OperationStats {
    pub operation: String,
    pub count: u64,
    pub bytes: u64,
    pub errors: u64,
}

/// 系统健康状态
#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub uptime_secs: u64,
    pub mount_count: usize,
    pub traffic: TrafficStatsResponse,
}

/// EVIF 监控 API 处理器
pub struct MetricsHandlers;

impl MetricsHandlers {
    /// 获取流量统计
    /// GET /api/v1/metrics/traffic
    pub async fn get_traffic_stats(
        State(state): State<MetricsState>,
    ) -> Json<TrafficStatsResponse> {
        let stats = &state.traffic_stats;

        let total_requests = stats.total_requests.load(Ordering::Relaxed);
        let total_bytes_read = stats.total_bytes_read.load(Ordering::Relaxed);
        let total_bytes_written = stats.total_bytes_written.load(Ordering::Relaxed);
        let total_errors = stats.total_errors.load(Ordering::Relaxed);
        let read_count = stats.read_count.load(Ordering::Relaxed);
        let write_count = stats.write_count.load(Ordering::Relaxed);
        let list_count = stats.list_count.load(Ordering::Relaxed);
        let other_count = stats.other_count.load(Ordering::Relaxed);

        let avg_read_size = total_bytes_read.checked_div(read_count).unwrap_or(0);
        let avg_write_size = total_bytes_written.checked_div(write_count).unwrap_or(0);

        Json(TrafficStatsResponse {
            total_requests,
            total_bytes_read,
            total_bytes_written,
            total_errors,
            read_count,
            write_count,
            list_count,
            other_count,
            average_read_size: avg_read_size,
            average_write_size: avg_write_size,
        })
    }

    /// 获取操作统计
    /// GET /api/v1/metrics/operations
    pub async fn get_operation_stats(
        State(state): State<MetricsState>,
    ) -> Json<Vec<OperationStats>> {
        let stats = &state.traffic_stats;

        Json(vec![
            OperationStats {
                operation: "read".to_string(),
                count: stats.read_count.load(Ordering::Relaxed),
                bytes: stats.total_bytes_read.load(Ordering::Relaxed),
                errors: stats.read_error_count.load(Ordering::Relaxed),
            },
            OperationStats {
                operation: "write".to_string(),
                count: stats.write_count.load(Ordering::Relaxed),
                bytes: stats.total_bytes_written.load(Ordering::Relaxed),
                errors: stats.write_error_count.load(Ordering::Relaxed),
            },
            OperationStats {
                operation: "list".to_string(),
                count: stats.list_count.load(Ordering::Relaxed),
                bytes: 0,
                errors: stats.list_error_count.load(Ordering::Relaxed),
            },
            OperationStats {
                operation: "other".to_string(),
                count: stats.other_count.load(Ordering::Relaxed),
                bytes: 0,
                errors: stats.other_error_count.load(Ordering::Relaxed),
            },
        ])
    }

    /// 获取系统健康状态
    /// GET /api/v1/health
    pub async fn get_health(State(state): State<MetricsState>) -> Json<HealthStatus> {
        let mounts = state.mount_table.list_mounts().await;
        let mount_count = mounts.len();

        let stats = Self::get_traffic_stats(State(state.clone())).await;

        // 跟踪实际启动时间
        let uptime_secs = state.start_time.elapsed().as_secs();

        Json(HealthStatus {
            status: "healthy".to_string(),
            uptime_secs,
            mount_count,
            traffic: stats.0,
        })
    }

    /// 重置统计
    /// POST /api/v1/metrics/reset
    pub async fn reset_metrics(State(state): State<MetricsState>) -> Json<serde_json::Value> {
        let stats = &state.traffic_stats;

        stats.total_requests.store(0, Ordering::Relaxed);
        stats.total_bytes_read.store(0, Ordering::Relaxed);
        stats.total_bytes_written.store(0, Ordering::Relaxed);
        stats.total_errors.store(0, Ordering::Relaxed);
        stats.read_count.store(0, Ordering::Relaxed);
        stats.write_count.store(0, Ordering::Relaxed);
        stats.list_count.store(0, Ordering::Relaxed);
        stats.other_count.store(0, Ordering::Relaxed);
        stats.read_success_count.store(0, Ordering::Relaxed);
        stats.read_error_count.store(0, Ordering::Relaxed);
        stats.read_latency_micros_total.store(0, Ordering::Relaxed);
        stats.write_success_count.store(0, Ordering::Relaxed);
        stats.write_error_count.store(0, Ordering::Relaxed);
        stats.write_latency_micros_total.store(0, Ordering::Relaxed);
        stats.list_success_count.store(0, Ordering::Relaxed);
        stats.list_error_count.store(0, Ordering::Relaxed);
        stats.list_latency_micros_total.store(0, Ordering::Relaxed);
        stats.other_success_count.store(0, Ordering::Relaxed);
        stats.other_error_count.store(0, Ordering::Relaxed);
        stats.other_latency_micros_total.store(0, Ordering::Relaxed);

        Json(serde_json::json!({
            "message": "Metrics reset successfully"
        }))
    }

    /// 获取详细的系统状态
    /// GET /api/v1/metrics/status
    pub async fn get_system_status(State(state): State<MetricsState>) -> Json<serde_json::Value> {
        let mounts = state.mount_table.list_mounts().await;
        let health = Self::get_health(State(state.clone())).await;
        let traffic = Self::get_traffic_stats(State(state.clone())).await;
        let operations = Self::get_operation_stats(State(state.clone())).await;

        Json(serde_json::json!({
            "status": health.status,
            "uptime_secs": health.uptime_secs,
            "mounts": {
                "count": health.mount_count,
                "list": mounts // Just list the mount paths
            },
            "traffic": traffic.0,
            "operations": operations.0,
        }))
    }
}

impl TrafficStats {
    /// 记录读取操作
    pub fn record_read(&self, bytes: u64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.read_count.fetch_add(1, Ordering::Relaxed);
        self.total_bytes_read.fetch_add(bytes, Ordering::Relaxed);
    }

    /// 记录写入操作
    pub fn record_write(&self, bytes: u64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.write_count.fetch_add(1, Ordering::Relaxed);
        self.total_bytes_written.fetch_add(bytes, Ordering::Relaxed);
    }

    /// 记录列表操作
    pub fn record_list(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.list_count.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录其他操作
    pub fn record_other(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.other_count.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录错误
    pub fn record_error(&self) {
        self.total_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录读取结果与延迟
    pub fn record_read_outcome(&self, success: bool, latency_micros: u64) {
        Self::record_operation_outcome(
            success,
            latency_micros,
            &self.read_success_count,
            &self.read_error_count,
            &self.read_latency_micros_total,
        );
    }

    /// 记录写入结果与延迟
    pub fn record_write_outcome(&self, success: bool, latency_micros: u64) {
        Self::record_operation_outcome(
            success,
            latency_micros,
            &self.write_success_count,
            &self.write_error_count,
            &self.write_latency_micros_total,
        );
    }

    /// 记录列表结果与延迟
    pub fn record_list_outcome(&self, success: bool, latency_micros: u64) {
        Self::record_operation_outcome(
            success,
            latency_micros,
            &self.list_success_count,
            &self.list_error_count,
            &self.list_latency_micros_total,
        );
    }

    /// 记录其他操作结果与延迟
    pub fn record_other_outcome(&self, success: bool, latency_micros: u64) {
        Self::record_operation_outcome(
            success,
            latency_micros,
            &self.other_success_count,
            &self.other_error_count,
            &self.other_latency_micros_total,
        );
    }

    fn record_operation_outcome(
        success: bool,
        latency_micros: u64,
        success_count: &AtomicU64,
        error_count: &AtomicU64,
        latency_total: &AtomicU64,
    ) {
        if success {
            success_count.fetch_add(1, Ordering::Relaxed);
        } else {
            error_count.fetch_add(1, Ordering::Relaxed);
        }
        latency_total.fetch_add(latency_micros, Ordering::Relaxed);
    }
}
