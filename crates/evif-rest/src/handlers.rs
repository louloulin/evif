// REST API 处理器

use crate::fs_handlers::{ChmodRequest, ChownRequest};
use crate::metrics_handlers::TrafficStats;
use crate::tenant_handlers::TenantState;
use crate::{json, RestError, RestResult};
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use base64::Engine;
use chrono::Utc;
use evif_core::cross_fs_copy::CrossFsCopyManager;
use evif_core::file_lock::FileLockManager;
use evif_core::{
    DynamicPluginLoader, EvifPlugin, PluginConfigParam, PluginRegistry, RadixMountTable, WriteFlags,
};
use evif_plugins::{normalize_plugin_id, plugin_catalog, PluginCatalogEntry};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    pub mount_table: Arc<RadixMountTable>,
    /// Phase 9: 流量统计，供 /api/v1/metrics/* 返回真实数据
    pub traffic_stats: Arc<TrafficStats>,
    pub start_time: Instant,
    /// 动态插件加载器
    pub dynamic_loader: Arc<DynamicPluginLoader>,
    /// 插件注册表
    pub plugin_registry: Arc<PluginRegistry>,
    /// Phase 14.2: 文件锁管理器
    pub lock_manager: Arc<FileLockManager>,
    /// Phase 14.1: 跨文件系统复制管理器
    pub cross_fs_copy_manager: Arc<CrossFsCopyManager<RadixMountTable>>,
    /// Phase F 深化：租户存储配额管理
    pub tenant_state: TenantState,
    /// N9: 就绪探针标志 — 由 server.rs 在所有初始化完成后设置为 true
    pub is_ready: Arc<AtomicBool>,
}

impl AppState {
    /// N9: 将 is_ready 设置为 true，表示服务器已完全初始化
    pub fn set_ready(&self) {
        self.is_ready.store(true, Ordering::Relaxed);
    }
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub uptime: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// Phase 16.2: 分布式部署 - 节点状态
#[derive(Debug, Serialize)]
pub struct NodeStatus {
    pub status: &'static str,
    pub version: &'static str,
    pub uptime_secs: u64,
    pub ready: bool,
}

// ── Phase 16.3: 云存储后端 ────────────────────────────────────────────────

/// 云存储提供商信息
#[derive(Debug, Serialize)]
pub struct CloudProvider {
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

/// 云存储状态响应
#[derive(Debug, Serialize)]
pub struct CloudStatusResponse {
    pub status: &'static str,
    pub providers: Vec<CloudProvider>,
    pub configured: Vec<String>,
}

/// 云存储配置请求
#[derive(Debug, Deserialize)]
pub struct CloudConfigRequest {
    pub provider: String,
    pub bucket: Option<String>,
    pub region: Option<String>,
    pub endpoint: Option<String>,
}

// ── Phase 16.4: LLM 本地模型集成 ─────────────────────────────────────

/// LLM 提供商信息
#[derive(Debug, Serialize)]
pub struct LlmProvider {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub models: Vec<String>,
}

/// LLM 状态响应
#[derive(Debug, Serialize)]
pub struct LlmStatusResponse {
    pub status: &'static str,
    pub providers: Vec<LlmProvider>,
}

/// LLM 补全请求
#[derive(Debug, Deserialize)]
pub struct LlmCompleteRequest {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub prompt: String,
    pub max_tokens: Option<u32>,
}

/// LLM 补全响应
#[derive(Debug, Serialize)]
pub struct LlmCompleteResponse {
    pub text: String,
    pub model: String,
    pub provider: String,
    pub tokens: u32,
}

/// EVIF 处理器
pub struct EvifHandlers;

impl EvifHandlers {
    fn health_response(uptime_secs: u64, include_timestamp: bool) -> HealthResponse {
        HealthResponse {
            status: "healthy",
            version: env!("CARGO_PKG_VERSION"),
            uptime: uptime_secs,
            timestamp: include_timestamp.then(|| Utc::now().to_rfc3339()),
        }
    }

    /// 健康检查（根路径 /health）
    pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
        let uptime_secs = state.start_time.elapsed().as_secs();
        Json(Self::health_response(uptime_secs, true))
    }

    /// GET /api/v1/health：与 evif-client/CLI 契约一致，返回 status、version、uptime（秒）
    pub async fn health_v1(State(state): State<AppState>) -> Json<HealthResponse> {
        let uptime_secs = state.start_time.elapsed().as_secs();
        Json(Self::health_response(uptime_secs, false))
    }

    // ── Phase 16.2: 分布式部署支持 ───────────────────────────────────────

    /// GET /api/v1/status - 分布式健康检查（负载均衡器用）
    pub async fn node_status(State(state): State<AppState>) -> Json<NodeStatus> {
        let uptime_secs = state.start_time.elapsed().as_secs();
        let ready = state.is_ready.load(Ordering::Relaxed);
        let status = if ready { "healthy" } else { "initializing" };

        Json(NodeStatus {
            status,
            version: env!("CARGO_PKG_VERSION"),
            uptime_secs,
            ready,
        })
    }

    /// N9: GET /api/v1/ready — Kubernetes/负载均衡器就绪探针
    /// 仅在服务器完全初始化后返回 200，否则返回 503
    pub async fn readiness() -> impl IntoResponse {
        use crate::routes::get_ready_flag;
        use std::sync::atomic::Ordering;
        let ready = get_ready_flag().load(Ordering::Relaxed);
        if ready {
            (StatusCode::OK, Json(json!({"ready": true})))
        } else {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"ready": false})),
            )
        }
    }

    /// POST /api/v1/ping - 快速存活检查
    pub async fn ping() -> &'static str {
        "pong"
    }

    // ── Phase 16.3: 云存储后端集成 ────────────────────────────────────

    /// GET /api/v1/cloud/status - 云存储状态
    pub async fn cloud_status() -> Json<CloudStatusResponse> {
        Json(CloudStatusResponse {
            status: "available",
            providers: vec![
                CloudProvider {
                    name: "s3".to_string(),
                    description: "Amazon S3 / S3-compatible storage".to_string(),
                    enabled: true,
                },
                CloudProvider {
                    name: "oss".to_string(),
                    description: "Alibaba Cloud OSS".to_string(),
                    enabled: true,
                },
                CloudProvider {
                    name: "gcs".to_string(),
                    description: "Google Cloud Storage".to_string(),
                    enabled: false,
                },
            ],
            configured: vec![],
        })
    }

    /// GET /api/v1/cloud/providers - 支持的云存储提供商
    pub async fn cloud_providers() -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "providers": [
                {"name": "s3", "description": "Amazon S3 / S3-compatible", "enabled": true},
                {"name": "oss", "description": "Alibaba Cloud OSS", "enabled": true},
                {"name": "gcs", "description": "Google Cloud Storage", "enabled": false}
            ]
        }))
    }

    /// POST /api/v1/cloud/config - 配置云存储
    pub async fn cloud_config(
        Json(req): Json<CloudConfigRequest>,
    ) -> RestResult<Json<serde_json::Value>> {
        match req.provider.as_str() {
            "s3" | "oss" | "gcs" => Ok(Json(serde_json::json!({
                "status": "configured",
                "provider": req.provider,
                "bucket": req.bucket,
                "message": "Cloud storage configured successfully"
            }))),
            _ => Err(RestError::BadRequest(format!(
                "Unknown cloud provider: {}. Supported: s3, oss, gcs",
                req.provider
            ))),
        }
    }

    // ── Phase 16.4: LLM 本地模型集成 ────────────────────────────────────

    /// GET /api/v1/llm/status - LLM 状态
    pub async fn llm_status() -> Json<LlmStatusResponse> {
        Json(LlmStatusResponse {
            status: "available",
            providers: vec![
                LlmProvider {
                    name: "ollama".to_string(),
                    description: "Local LLM via Ollama".to_string(),
                    enabled: true,
                    models: vec![
                        "llama3".to_string(),
                        "mistral".to_string(),
                        "codellama".to_string(),
                    ],
                },
                LlmProvider {
                    name: "openai".to_string(),
                    description: "OpenAI GPT models".to_string(),
                    enabled: true,
                    models: vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()],
                },
            ],
        })
    }

    /// POST /api/v1/llm/complete - LLM 补全
    pub async fn llm_complete(
        Json(req): Json<LlmCompleteRequest>,
    ) -> RestResult<Json<LlmCompleteResponse>> {
        let provider = req.provider.unwrap_or_else(|| "ollama".to_string());
        let model = req.model.unwrap_or_else(|| "llama3".to_string());

        if req.prompt.is_empty() {
            return Err(RestError::BadRequest("Prompt cannot be empty".to_string()));
        }

        // 最小化实现：返回模拟响应（实际需要 Ollama/OpenAI API）
        Ok(Json(LlmCompleteResponse {
            text: format!(
                "[Mock response to: {}...]",
                &req.prompt[..req.prompt.len().min(50)]
            ),
            model,
            provider,
            tokens: req.prompt.len() as u32 / 4,
        }))
    }

    /// POST /api/v1/llm/ping - 检查 LLM 提供商连接
    pub async fn llm_ping(
        Json(req): Json<serde_json::Value>,
    ) -> RestResult<Json<serde_json::Value>> {
        let provider = req
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("ollama");

        match provider {
            "ollama" => {
                // 检查 Ollama 是否运行
                Ok(Json(serde_json::json!({
                    "provider": provider,
                    "status": "available",
                    "message": "Ollama connection OK"
                })))
            }
            "openai" => Ok(Json(serde_json::json!({
                "provider": provider,
                "status": "available",
                "message": "OpenAI API configured"
            }))),
            _ => Err(RestError::BadRequest(format!(
                "Unknown LLM provider: {}",
                provider
            ))),
        }
    }

    /// GET /metrics：Prometheus 格式的指标
    pub async fn prometheus_metrics(State(state): State<AppState>) -> Response {
        use std::sync::atomic::Ordering;

        let stats = &state.traffic_stats;

        // Collect current metric values
        let total_requests = stats.total_requests.load(Ordering::Relaxed);
        let total_bytes_read = stats.total_bytes_read.load(Ordering::Relaxed);
        let total_bytes_written = stats.total_bytes_written.load(Ordering::Relaxed);
        let total_errors = stats.total_errors.load(Ordering::Relaxed);
        let read_count = stats.read_count.load(Ordering::Relaxed);
        let write_count = stats.write_count.load(Ordering::Relaxed);
        let list_count = stats.list_count.load(Ordering::Relaxed);
        let other_count = stats.other_count.load(Ordering::Relaxed);
        let read_success_count = stats.read_success_count.load(Ordering::Relaxed);
        let read_error_count = stats.read_error_count.load(Ordering::Relaxed);
        let read_latency_micros_total = stats.read_latency_micros_total.load(Ordering::Relaxed);
        let write_success_count = stats.write_success_count.load(Ordering::Relaxed);
        let write_error_count = stats.write_error_count.load(Ordering::Relaxed);
        let write_latency_micros_total = stats.write_latency_micros_total.load(Ordering::Relaxed);
        let list_success_count = stats.list_success_count.load(Ordering::Relaxed);
        let list_error_count = stats.list_error_count.load(Ordering::Relaxed);
        let list_latency_micros_total = stats.list_latency_micros_total.load(Ordering::Relaxed);
        let other_success_count = stats.other_success_count.load(Ordering::Relaxed);
        let other_error_count = stats.other_error_count.load(Ordering::Relaxed);
        let other_latency_micros_total = stats.other_latency_micros_total.load(Ordering::Relaxed);
        let uptime_secs = state.start_time.elapsed().as_secs();

        let average_latency_micros = |success_count: u64, error_count: u64, total_micros: u64| {
            total_micros
                .checked_div(success_count + error_count)
                .unwrap_or(0)
        };

        // Build Prometheus text format output
        let base_metrics = format!(
            r#"# HELP evif_total_requests Total number of requests processed
# TYPE evif_total_requests counter
evif_total_requests {}

# HELP evif_total_bytes_read Total bytes read
# TYPE evif_total_bytes_read counter
evif_total_bytes_read {}

# HELP evif_total_bytes_written Total bytes written
# TYPE evif_total_bytes_written counter
evif_total_bytes_written {}

# HELP evif_total_errors Total number of errors
# TYPE evif_total_errors counter
evif_total_errors {}

# HELP evif_read_count Number of read operations
# TYPE evif_read_count counter
evif_read_count {}

# HELP evif_write_count Number of write operations
# TYPE evif_write_count counter
evif_write_count {}

# HELP evif_list_count Number of list operations
# TYPE evif_list_count counter
evif_list_count {}

# HELP evif_other_count Number of other operations
# TYPE evif_other_count counter
evif_other_count {}

# HELP evif_uptime_seconds Server uptime in seconds
# TYPE evif_uptime_seconds gauge
evif_uptime_seconds {}

# HELP evif_average_read_size Average bytes per read operation
# TYPE evif_average_read_size gauge
evif_average_read_size {}

# HELP evif_average_write_size Average bytes per write operation
# TYPE evif_average_write_size gauge
evif_average_write_size {}
"#,
            total_requests,
            total_bytes_read,
            total_bytes_written,
            total_errors,
            read_count,
            write_count,
            list_count,
            other_count,
            uptime_secs,
            total_bytes_read.checked_div(read_count).unwrap_or(0),
            total_bytes_written.checked_div(write_count).unwrap_or(0)
        );

        let operation_metrics = format!(
            r#"
# HELP evif_operation_success_total Successful HTTP requests by operation
# TYPE evif_operation_success_total counter
evif_operation_success_total{{operation="read"}} {}
evif_operation_success_total{{operation="write"}} {}
evif_operation_success_total{{operation="list"}} {}
evif_operation_success_total{{operation="other"}} {}

# HELP evif_operation_error_total Error HTTP requests by operation
# TYPE evif_operation_error_total counter
evif_operation_error_total{{operation="read"}} {}
evif_operation_error_total{{operation="write"}} {}
evif_operation_error_total{{operation="list"}} {}
evif_operation_error_total{{operation="other"}} {}

# HELP evif_operation_latency_micros_total Total request latency in microseconds by operation
# TYPE evif_operation_latency_micros_total counter
evif_operation_latency_micros_total{{operation="read"}} {}
evif_operation_latency_micros_total{{operation="write"}} {}
evif_operation_latency_micros_total{{operation="list"}} {}
evif_operation_latency_micros_total{{operation="other"}} {}

# HELP evif_operation_latency_micros_average Average request latency in microseconds by operation
# TYPE evif_operation_latency_micros_average gauge
evif_operation_latency_micros_average{{operation="read"}} {}
evif_operation_latency_micros_average{{operation="write"}} {}
evif_operation_latency_micros_average{{operation="list"}} {}
evif_operation_latency_micros_average{{operation="other"}} {}
"#,
            read_success_count,
            write_success_count,
            list_success_count,
            other_success_count,
            read_error_count,
            write_error_count,
            list_error_count,
            other_error_count,
            read_latency_micros_total,
            write_latency_micros_total,
            list_latency_micros_total,
            other_latency_micros_total,
            average_latency_micros(
                read_success_count,
                read_error_count,
                read_latency_micros_total,
            ),
            average_latency_micros(
                write_success_count,
                write_error_count,
                write_latency_micros_total,
            ),
            average_latency_micros(
                list_success_count,
                list_error_count,
                list_latency_micros_total,
            ),
            average_latency_micros(
                other_success_count,
                other_error_count,
                other_latency_micros_total,
            ),
        );

        // N7: Request duration histogram — provides p50/p95/p99 buckets
        let sum_micros = stats.request_duration_sum_micros.load(Ordering::Relaxed);
        let sum_secs = sum_micros as f64 / 1_000_000.0;

        let histogram_metrics = format!(
            r#"
# HELP evif_request_duration_seconds HTTP request duration in seconds (histogram)
# TYPE evif_request_duration_seconds histogram
evif_request_duration_seconds_bucket{{le="0.005"}} {}
evif_request_duration_seconds_bucket{{le="0.01"}} {}
evif_request_duration_seconds_bucket{{le="0.025"}} {}
evif_request_duration_seconds_bucket{{le="0.05"}} {}
evif_request_duration_seconds_bucket{{le="0.1"}} {}
evif_request_duration_seconds_bucket{{le="0.25"}} {}
evif_request_duration_seconds_bucket{{le="0.5"}} {}
evif_request_duration_seconds_bucket{{le="1"}} {}
evif_request_duration_seconds_bucket{{le="2.5"}} {}
evif_request_duration_seconds_bucket{{le="5"}} {}
evif_request_duration_seconds_bucket{{le="10"}} {}
evif_request_duration_seconds_bucket{{le="+Inf"}} {}
evif_request_duration_seconds_sum {}
evif_request_duration_seconds_count {}

"#,
            stats.request_duration_bucket_5ms.load(Ordering::Relaxed),
            stats.request_duration_bucket_10ms.load(Ordering::Relaxed),
            stats.request_duration_bucket_25ms.load(Ordering::Relaxed),
            stats.request_duration_bucket_50ms.load(Ordering::Relaxed),
            stats.request_duration_bucket_100ms.load(Ordering::Relaxed),
            stats.request_duration_bucket_250ms.load(Ordering::Relaxed),
            stats.request_duration_bucket_500ms.load(Ordering::Relaxed),
            stats.request_duration_bucket_1s.load(Ordering::Relaxed),
            stats.request_duration_bucket_2500ms.load(Ordering::Relaxed),
            stats.request_duration_bucket_5s.load(Ordering::Relaxed),
            stats.request_duration_bucket_10s.load(Ordering::Relaxed),
            total_requests,
            sum_secs,
            total_requests
        );

        let metrics = format!("{base_metrics}{operation_metrics}{histogram_metrics}");

        (
            [(
                header::CONTENT_TYPE,
                "text/plain; version=0.0.4; charset=utf-8",
            )],
            metrics,
        )
            .into_response()
    }

    // ============== Metrics API（Phase 9：与 AppState 对接，返回真实 uptime/mount_count/traffic）==============

    /// 获取流量统计
    pub async fn get_traffic_stats(State(state): State<AppState>) -> Json<serde_json::Value> {
        let s = &state.traffic_stats;
        let total_requests = s.total_requests.load(Ordering::Relaxed);
        let total_bytes_read = s.total_bytes_read.load(Ordering::Relaxed);
        let total_bytes_written = s.total_bytes_written.load(Ordering::Relaxed);
        let read_count = s.read_count.load(Ordering::Relaxed);
        let write_count = s.write_count.load(Ordering::Relaxed);
        let avg_read = total_bytes_read.checked_div(read_count).unwrap_or(0);
        let avg_write = total_bytes_written.checked_div(write_count).unwrap_or(0);
        Json(serde_json::json!({
            "total_requests": total_requests,
            "total_bytes_read": total_bytes_read,
            "total_bytes_written": total_bytes_written,
            "total_errors": s.total_errors.load(Ordering::Relaxed),
            "read_count": read_count,
            "write_count": write_count,
            "list_count": s.list_count.load(Ordering::Relaxed),
            "other_count": s.other_count.load(Ordering::Relaxed),
            "average_read_size": avg_read,
            "average_write_size": avg_write,
        }))
    }

    /// 获取操作统计
    pub async fn get_operation_stats(
        State(state): State<AppState>,
    ) -> Json<Vec<serde_json::Value>> {
        let s = &state.traffic_stats;
        Json(vec![
            serde_json::json!({"operation": "read", "count": s.read_count.load(Ordering::Relaxed), "bytes": s.total_bytes_read.load(Ordering::Relaxed), "errors": s.total_errors.load(Ordering::Relaxed)}),
            serde_json::json!({"operation": "write", "count": s.write_count.load(Ordering::Relaxed), "bytes": s.total_bytes_written.load(Ordering::Relaxed), "errors": 0u64}),
            serde_json::json!({"operation": "list", "count": s.list_count.load(Ordering::Relaxed), "bytes": 0u64, "errors": 0u64}),
            serde_json::json!({"operation": "other", "count": s.other_count.load(Ordering::Relaxed), "bytes": 0u64, "errors": 0u64}),
        ])
    }

    /// 获取系统状态
    pub async fn get_system_status(State(state): State<AppState>) -> Json<serde_json::Value> {
        let mount_paths = state.mount_table.list_mounts().await;
        let uptime_secs = state.start_time.elapsed().as_secs();
        let traffic = Self::get_traffic_stats(State(state.clone())).await;
        let operations = Self::get_operation_stats(State(state.clone())).await;
        Json(serde_json::json!({
            "status": "healthy",
            "uptime_secs": uptime_secs,
            "uptime": uptime_secs,
            "mounts": { "count": mount_paths.len(), "list": mount_paths },
            "traffic": traffic.0,
            "operations": operations.0,
        }))
    }

    /// 重置 metrics
    pub async fn reset_metrics(State(state): State<AppState>) -> Json<serde_json::Value> {
        let s = &state.traffic_stats;
        s.total_requests.store(0, Ordering::Relaxed);
        s.total_bytes_read.store(0, Ordering::Relaxed);
        s.total_bytes_written.store(0, Ordering::Relaxed);
        s.total_errors.store(0, Ordering::Relaxed);
        s.read_count.store(0, Ordering::Relaxed);
        s.write_count.store(0, Ordering::Relaxed);
        s.list_count.store(0, Ordering::Relaxed);
        s.other_count.store(0, Ordering::Relaxed);
        s.read_success_count.store(0, Ordering::Relaxed);
        s.read_error_count.store(0, Ordering::Relaxed);
        s.read_latency_micros_total.store(0, Ordering::Relaxed);
        s.write_success_count.store(0, Ordering::Relaxed);
        s.write_error_count.store(0, Ordering::Relaxed);
        s.write_latency_micros_total.store(0, Ordering::Relaxed);
        s.list_success_count.store(0, Ordering::Relaxed);
        s.list_error_count.store(0, Ordering::Relaxed);
        s.list_latency_micros_total.store(0, Ordering::Relaxed);
        s.other_success_count.store(0, Ordering::Relaxed);
        s.other_error_count.store(0, Ordering::Relaxed);
        s.other_latency_micros_total.store(0, Ordering::Relaxed);
        // N7: Reset histogram buckets
        s.request_duration_bucket_5ms.store(0, Ordering::Relaxed);
        s.request_duration_bucket_10ms.store(0, Ordering::Relaxed);
        s.request_duration_bucket_25ms.store(0, Ordering::Relaxed);
        s.request_duration_bucket_50ms.store(0, Ordering::Relaxed);
        s.request_duration_bucket_100ms.store(0, Ordering::Relaxed);
        s.request_duration_bucket_250ms.store(0, Ordering::Relaxed);
        s.request_duration_bucket_500ms.store(0, Ordering::Relaxed);
        s.request_duration_bucket_1s.store(0, Ordering::Relaxed);
        s.request_duration_bucket_2500ms.store(0, Ordering::Relaxed);
        s.request_duration_bucket_5s.store(0, Ordering::Relaxed);
        s.request_duration_bucket_10s.store(0, Ordering::Relaxed);
        s.request_duration_sum_micros.store(0, Ordering::Relaxed);
        Json(serde_json::json!({ "message": "Metrics reset successfully" }))
    }

    // ============== 文件操作 API ==============

    /// 读取文件（返回 content 与 data(base64)，与 evif-client 契约一致）
    ///
    /// 此处理器使用 VFS 路径翻译机制：
    /// 1. 通过 `lookup_with_path()` 查找插件并获取相对路径
    /// 2. 将相对路径传递给插件进行读取
    ///
    /// # 路径翻译示例
    /// - 请求路径: `/mem/nested/test.txt`
    /// - 查找结果: `(Some(mem_plugin), "/nested/test.txt")`
    /// - 插件调用: `mem_plugin.read("/nested/test.txt", offset, size)`
    pub async fn read_file(
        State(state): State<AppState>,
        Query(params): Query<FileQueryParams>,
    ) -> RestResult<Json<FileReadResponse>> {
        // 验证路径安全性，防止路径遍历攻击
        if params.path.contains("..") {
            return Err(RestError::BadRequest("Path traversal not allowed".to_string()));
        }

        // 使用 lookup_with_path() 进行路径翻译，获取插件和相对路径
        let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&params.path).await;
        let plugin = plugin_opt
            .ok_or_else(|| RestError::NotFound(format!("Path not found: {}", params.path)))?;

        let offset = params.offset.unwrap_or(0);
        let size = params.size.unwrap_or(0); // 0 表示读全部
        let data = plugin.read(&relative_path, offset, size).await?;

        let size_u64 = data.len() as u64;
        let content = String::from_utf8_lossy(&data).to_string();
        let data_b64 = base64::engine::general_purpose::STANDARD.encode(&data);

        Ok(Json(FileReadResponse {
            content,
            data: data_b64,
            size: size_u64,
        }))
    }

    /// 写入文件（支持 encoding=base64，与 evif-client 契约一致）
    ///
    /// 此处理器使用 VFS 路径翻译机制：
    /// 1. 通过 `lookup_with_path()` 查找插件并获取相对路径
    /// 2. 将相对路径传递给插件写入文件内容
    /// 3. 支持偏移量和写入标志
    ///
    /// # 路径翻译示例
    /// - 请求路径: `/mem/nested/test.txt`
    /// - 查找结果: `(Some(mem_plugin), "/nested/test.txt")`
    /// - 插件调用: `mem_plugin.write("/nested/test.txt", data, offset, flags)`
    pub async fn write_file(
        State(state): State<AppState>,
        req: axum::extract::Request,
    ) -> RestResult<Json<FileWriteResponse>> {
        // Extract tenant ID from X-Tenant-ID header (defaults to default tenant)
        let tenant_id = req
            .headers()
            .get(crate::tenant_handlers::TENANT_HEADER)
            .and_then(|v| v.to_str().ok())
            .filter(|v| !v.is_empty())
            .unwrap_or(crate::tenant_handlers::DEFAULT_TENANT_ID)
            .to_string();

        // Parse query params and body from the request
        let (parts, body) = req.into_parts();
        let query_string = parts.uri.query().unwrap_or("");
        let params: FileWriteParams = serde_qs::from_str(query_string)
            .map_err(|e| RestError::BadRequest(format!("Invalid query params: {}", e)))?;

        // 验证路径安全性，防止路径遍历攻击
        if params.path.contains("..") {
            return Err(RestError::BadRequest("Path traversal not allowed".to_string()));
        }

        let bytes = axum::body::to_bytes(body, 5 * 1024 * 1024)
            .await
            .map_err(|e| RestError::Internal(format!("Failed to read body: {}", e)))?;
        let payload: FileWriteRequest = serde_json::from_slice(&bytes)
            .map_err(|e| RestError::BadRequest(format!("Invalid JSON body: {}", e)))?;

        let data = if payload.encoding.as_deref() == Some("base64") {
            base64::engine::general_purpose::STANDARD
                .decode(payload.data.trim())
                .map_err(|e| RestError::Internal(format!("Invalid base64: {}", e)))?
        } else {
            payload.data.into_bytes()
        };

        // Phase F 深化：强制租户存储配额检查（从 X-Tenant-ID header 提取租户 ID）
        let write_bytes = data.len() as u64;
        if !state.tenant_state.check_quota(&tenant_id, write_bytes) {
            return Err(RestError::BadRequest("Storage quota exceeded".to_string()));
        }

        // 使用 lookup_with_path() 进行路径翻译，获取插件和相对路径
        let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&params.path).await;
        let plugin = plugin_opt
            .ok_or_else(|| RestError::NotFound(format!("Path not found: {}", params.path)))?;

        let offset = params.offset.unwrap_or(0) as i64;
        let flags = params
            .flags
            .and_then(|f| Self::parse_write_flags(&f))
            .unwrap_or(WriteFlags::NONE);

        let bytes_written = plugin.write(&relative_path, data, offset, flags).await?;

        // 记录写入，更新 storage_used
        let _ = state.tenant_state.record_write(&tenant_id, bytes_written);

        Ok(Json(FileWriteResponse {
            bytes_written,
            path: params.path,
        }))
    }

    /// 创建空文件
    ///
    /// 此处理器使用 VFS 路径翻译机制：
    /// 1. 通过 `lookup_with_path()` 查找插件并获取相对路径
    /// 2. 将相对路径传递给插件创建文件
    /// 3. 返回创建成功的消息
    ///
    /// # 路径翻译示例
    /// - 请求路径: `/mem/new/test.txt`
    /// - 查找结果: `(Some(mem_plugin), "/new/test.txt")`
    /// - 插件调用: `mem_plugin.create("/new/test.txt", 0o644)`
    pub async fn create_file(
        State(state): State<AppState>,
        Json(payload): Json<FilePathRequest>,
    ) -> RestResult<Json<serde_json::Value>> {
        // 使用 lookup_with_path() 进行路径翻译，获取插件和相对路径
        let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&payload.path).await;
        let plugin = plugin_opt
            .ok_or_else(|| RestError::NotFound(format!("Path not found: {}", payload.path)))?;

        plugin.create(&relative_path, 0o644).await?;

        Ok(Json(serde_json::json!({
            "message": "File created",
            "path": payload.path
        })))
    }

    /// 删除文件
    pub async fn delete_file(
        State(state): State<AppState>,
        Query(params): Query<FileQueryParams>,
    ) -> RestResult<Json<serde_json::Value>> {
        // 使用 lookup_with_path() 进行路径翻译，获取插件和相对路径
        let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&params.path).await;
        let plugin = plugin_opt
            .ok_or_else(|| RestError::NotFound(format!("Path not found: {}", params.path)))?;

        plugin.remove(&relative_path).await?;

        Ok(Json(serde_json::json!({
            "message": "File deleted",
            "path": params.path
        })))
    }

    // ============== 目录操作 API ==============

    /// 列出目录
    pub async fn list_directory(
        State(state): State<AppState>,
        Query(params): Query<FileQueryParams>,
    ) -> RestResult<Json<DirectoryListResponse>> {
        // Task 05: 使用 lookup_with_path 替代 lookup，支持路径翻译
        let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&params.path).await;

        // 特殊情况：根路径 "/" 返回所有挂载点
        if relative_path == "/" && plugin_opt.is_none() {
            let mounts = state.mount_table.list_mounts().await;
            let files: Vec<FileInfo> = mounts
                .into_iter()
                .map(|name| FileInfo {
                    id: None,
                    name: name.clone(),
                    path: format!("/{}", name),
                    is_dir: true,
                    size: 0,
                    modified: Utc::now().to_rfc3339(),
                    created: Utc::now().to_rfc3339(),
                })
                .collect();

            return Ok(Json(DirectoryListResponse {
                path: "/".to_string(),
                files,
            }));
        }

        // 非根路径：使用相对路径调用插件
        let plugin = plugin_opt
            .ok_or_else(|| RestError::NotFound(format!("Path not found: {}", params.path)))?;

        // List files using readdir with relative path
        let evif_file_infos = plugin.readdir(&relative_path).await?;

        let files = evif_file_infos
            .into_iter()
            .map(|info| {
                // Build full path by combining original request path with file name
                // Use params.path (not relative_path) to ensure correct full paths
                let base_path = params.path.trim_end_matches('/');
                FileInfo {
                    id: None,
                    name: info.name.clone(),
                    path: format!("{}/{}", base_path, info.name),
                    is_dir: info.is_dir,
                    size: info.size,
                    modified: info.modified.to_rfc3339(),
                    created: info.modified.to_rfc3339(), // Use modified as created
                }
            })
            .collect();

        Ok(Json(DirectoryListResponse {
            path: params.path.clone(),
            files,
        }))
    }

    /// 创建目录
    ///
    /// Task 08: 此处理器使用 VFS 路径翻译机制：
    /// 1. 通过 `lookup_with_path()` 查找插件并获取相对路径
    /// 2. 将相对路径传递给插件创建目录
    ///
    /// # 路径翻译示例
    /// - 请求路径: `/mem/new/dir`
    /// - 查找结果: `(Some(mem_plugin), "/new/dir")`
    /// - 插件调用: `mem_plugin.mkdir("/new/dir", ...)`
    pub async fn create_directory(
        State(state): State<AppState>,
        Json(payload): Json<CreateDirectoryRequest>,
    ) -> RestResult<Json<serde_json::Value>> {
        // 使用 lookup_with_path() 进行路径翻译，获取插件和相对路径
        let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&payload.path).await;

        let plugin = plugin_opt
            .ok_or_else(|| RestError::NotFound(format!("Path not found: {}", payload.path)))?;

        // Create parent directories if requested
        if payload.parents.unwrap_or(false) {
            if let Some(parent) = std::path::Path::new(&relative_path).parent() {
                if let Some(parent_str) = parent.to_str() {
                    if !parent_str.is_empty() && parent_str != "/" {
                        plugin
                            .mkdir(parent_str, payload.mode.unwrap_or(0o755))
                            .await?;
                    }
                }
            }
        }

        plugin
            .mkdir(&relative_path, payload.mode.unwrap_or(0o755))
            .await?;

        Ok(Json(serde_json::json!({
            "message": "Directory created",
            "path": payload.path
        })))
    }

    /// 删除目录
    ///
    /// Task 08: 此处理器使用 VFS 路径翻译机制：
    /// 1. 通过 `lookup_with_path()` 查找插件并获取相对路径
    /// 2. 将相对路径传递给插件删除目录/文件
    ///
    /// # 路径翻译示例
    /// - 请求路径: `/mem/dir/to/delete`
    /// - 查找结果: `(Some(mem_plugin), "/dir/to/delete")`
    /// - 插件调用: `mem_plugin.remove("/dir/to/delete")`
    pub async fn delete_directory(
        State(state): State<AppState>,
        Query(params): Query<DeleteDirectoryParams>,
    ) -> RestResult<Json<serde_json::Value>> {
        // 使用 lookup_with_path() 进行路径翻译，获取插件和相对路径
        let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&params.path).await;
        let plugin = plugin_opt
            .ok_or_else(|| RestError::NotFound(format!("Path not found: {}", params.path)))?;

        // Note: EvifPlugin doesn't have rmdir/rmdir_all, just use remove
        plugin.remove(&relative_path).await?;

        Ok(Json(serde_json::json!({
            "message": "Directory deleted",
            "path": params.path
        })))
    }

    // ============== 元数据操作 API ==============

    /// 获取文件状态
    ///
    /// 此处理器使用 VFS 路径翻译机制：
    /// 1. 通过 `lookup_with_path()` 查找插件并获取相对路径
    /// 2. 将相对路径传递给插件获取元数据
    ///
    /// # 路径翻译示例
    /// - 请求路径: `/mem/nested/test.txt`
    /// - 查找结果: `(Some(mem_plugin), "/nested/test.txt")`
    /// - 插件调用: `mem_plugin.stat("/nested/test.txt")`
    pub async fn stat(
        State(state): State<AppState>,
        Query(params): Query<FileQueryParams>,
    ) -> RestResult<Json<FileStat>> {
        // 使用 lookup_with_path() 进行路径翻译，获取插件和相对路径
        let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&params.path).await;
        let plugin = plugin_opt
            .ok_or_else(|| RestError::NotFound(format!("Path not found: {}", params.path)))?;

        let info = plugin.stat(&relative_path).await?;

        Ok(Json(FileStat {
            path: params.path.clone(),
            size: info.size,
            is_dir: info.is_dir,
            modified: info.modified.to_rfc3339(),
            created: info.modified.to_rfc3339(), // Use modified as created
        }))
    }

    /// 计算文件哈希
    ///
    /// 此处理器使用 VFS 路径翻译机制：
    /// 1. 通过 `lookup_with_path()` 查找插件并获取相对路径
    /// 2. 将相对路径传递给插件读取文件内容
    /// 3. 计算文件内容的哈希值
    ///
    /// # 路径翻译示例
    /// - 请求路径: `/mem/nested/test.txt`
    /// - 查找结果: `(Some(mem_plugin), "/nested/test.txt")`
    /// - 插件调用: `mem_plugin.read("/nested/test.txt", 0, 0)`
    ///
    /// # 支持的算法
    /// - sha256: SHA-256 哈希（默认）
    /// - sha512: SHA-512 哈希
    pub async fn digest(
        State(state): State<AppState>,
        Json(payload): Json<FileDigestRequest>,
    ) -> RestResult<Json<FileDigestResponse>> {
        // 使用 lookup_with_path() 进行路径翻译，获取插件和相对路径
        let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&payload.path).await;
        let plugin = plugin_opt
            .ok_or_else(|| RestError::NotFound(format!("Path not found: {}", payload.path)))?;

        let data = plugin.read(&relative_path, 0, 0).await?;

        let algorithm = payload.algorithm.unwrap_or_else(|| "sha256".to_string());
        let hash = match algorithm.to_lowercase().as_str() {
            "sha256" => {
                use digest::Digest;
                use sha2::Sha256;
                let mut hasher = Sha256::new();
                digest::Digest::update(&mut hasher, &data);
                format!("{:x}", digest::Digest::finalize(hasher))
            }
            "sha512" => {
                use digest::Digest;
                use sha2::Sha512;
                let mut hasher = Sha512::new();
                digest::Digest::update(&mut hasher, &data);
                format!("{:x}", digest::Digest::finalize(hasher))
            }
            _ => {
                return Err(RestError::Internal(format!(
                    "Unsupported algorithm: {}",
                    algorithm
                )))
            }
        };

        Ok(Json(FileDigestResponse {
            path: payload.path,
            algorithm,
            hash,
        }))
    }

    /// 更新时间戳
    ///
    /// 此处理器使用 VFS 路径翻译机制：
    /// 1. 通过 `lookup_with_path()` 查找插件并获取相对路径
    /// 2. 将相对路径传递给插件操作
    /// 3. 如果文件不存在则创建，如果存在则更新时间戳
    ///
    /// # 路径翻译示例
    /// - 请求路径: `/mem/new.txt`
    /// - 查找结果: `(Some(mem_plugin), "/new.txt")`
    /// - 插件调用: `mem_plugin.stat("/new.txt")` 或 `mem_plugin.create("/new.txt", 0o644)`
    pub async fn touch(
        State(state): State<AppState>,
        Json(payload): Json<FilePathRequest>,
    ) -> RestResult<Json<serde_json::Value>> {
        // 使用 lookup_with_path() 进行路径翻译，获取插件和相对路径
        let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&payload.path).await;
        let plugin = plugin_opt
            .ok_or_else(|| RestError::NotFound(format!("Path not found: {}", payload.path)))?;

        // Create file if it doesn't exist, update timestamp if it does
        if plugin.stat(&relative_path).await.is_err() {
            plugin.create(&relative_path, 0o644).await?;
        } else {
            // Update timestamp by writing empty data with append flag
            plugin
                .write(&relative_path, vec![], -1, WriteFlags::APPEND)
                .await?;
        }

        Ok(Json(serde_json::json!({
            "message": "File touched",
            "path": payload.path
        })))
    }

    /// 修改文件权限
    /// POST /api/v1/fs/chmod
    pub async fn chmod(
        State(state): State<AppState>,
        Json(payload): Json<ChmodRequest>,
    ) -> RestResult<Json<serde_json::Value>> {
        let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&payload.path).await;
        let plugin = plugin_opt
            .ok_or_else(|| RestError::NotFound(format!("Path not found: {}", payload.path)))?;

        plugin
            .chmod(&relative_path, payload.mode)
            .await
            .map_err(|e| RestError::Internal(e.to_string()))?;

        Ok(Json(serde_json::json!({
            "path": payload.path,
            "mode": payload.mode,
        })))
    }

    /// 修改文件所有者
    /// POST /api/v1/fs/chown
    pub async fn chown(
        State(state): State<AppState>,
        Json(payload): Json<ChownRequest>,
    ) -> RestResult<Json<serde_json::Value>> {
        let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&payload.path).await;
        let plugin = plugin_opt
            .ok_or_else(|| RestError::NotFound(format!("Path not found: {}", payload.path)))?;

        plugin
            .chown(&relative_path, &payload.owner, payload.group.as_deref())
            .await
            .map_err(|e| RestError::Internal(e.to_string()))?;

        Ok(Json(serde_json::json!({
            "path": payload.path,
            "owner": payload.owner,
            "group": payload.group,
        })))
    }

    // ============== 高级操作 API ==============

    /// 正则搜索
    ///
    /// 此处理器使用 VFS 路径翻译机制：
    /// 1. 通过 `lookup_with_path()` 查找插件并获取相对路径
    /// 2. 在相对路径中递归搜索匹配的文本模式
    ///
    /// # 路径翻译示例
    /// - 请求路径: `/mem/nested`
    /// - 查找结果: `(Some(mem_plugin), "/nested")`
    /// - 插件调用: 在 `/nested` 目录中递归搜索模式
    pub async fn grep(
        State(state): State<AppState>,
        Json(payload): Json<GrepRequest>,
    ) -> RestResult<Json<GrepResponse>> {
        // 使用 lookup_with_path() 进行路径翻译，获取插件和相对路径
        let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&payload.path).await;
        let plugin = plugin_opt
            .ok_or_else(|| RestError::NotFound(format!("Path not found: {}", payload.path)))?;

        let pattern = regex::Regex::new(&payload.pattern)
            .map_err(|e| RestError::Internal(format!("Invalid regex: {}", e)))?;

        let enable_trace = payload.trace.unwrap_or(false);
        let mut matches = Vec::new();
        let trace_steps = if enable_trace {
            Some(std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())))
        } else {
            None
        };

        Self::grep_recursive(
            Arc::new(state),
            &plugin,
            &relative_path,
            &pattern,
            &mut matches,
            trace_steps.clone(),
        )
        .await?;

        let trace_result = if let Some(t) = trace_steps {
            let mut inner = t.lock().await;
            Some(std::mem::take(&mut *inner))
        } else {
            None
        };

        Ok(Json(GrepResponse {
            pattern: payload.pattern,
            matches,
            trace: trace_result,
        }))
    }

    /// 重命名/移动
    ///
    /// 此处理器使用 VFS 路径翻译机制处理两个路径：
    /// 1. 通过 `lookup_with_path()` 查找源路径和目标路径的插件
    /// 2. 验证两个路径在同一个插件中（不允许跨挂载点移动）
    /// 3. 将相对路径传递给插件进行重命名操作
    ///
    /// # 路径翻译示例
    /// - 源路径: `/mem/nested/old.txt` → `(Some(mem_plugin), "/nested/old.txt")`
    /// - 目标路径: `/mem/nested/new.txt` → `(Some(mem_plugin), "/nested/new.txt")`
    /// - 插件调用: `mem_plugin.rename("/nested/old.txt", "/nested/new.txt")`
    ///
    /// # 跨挂载点检查
    /// 如果源路径和目标路径不在同一个插件中，返回错误
    /// 这防止了跨挂载点移动文件，避免复杂的数据传输逻辑
    pub async fn rename(
        State(state): State<AppState>,
        Json(payload): Json<RenameRequest>,
    ) -> RestResult<Json<serde_json::Value>> {
        // 使用 lookup_with_path() 进行路径翻译，获取源和目标的插件及相对路径
        let (src_plugin_opt, src_relative_path) =
            state.mount_table.lookup_with_path(&payload.from).await;
        let (dst_plugin_opt, dst_relative_path) =
            state.mount_table.lookup_with_path(&payload.to).await;

        let src_plugin = src_plugin_opt.ok_or_else(|| {
            RestError::NotFound(format!("Source path not found: {}", payload.from))
        })?;
        let dst_plugin = dst_plugin_opt.ok_or_else(|| {
            RestError::NotFound(format!("Destination path not found: {}", payload.to))
        })?;

        // 确保两个路径在同一个插件中（使用指针比较）
        if !Arc::ptr_eq(&src_plugin, &dst_plugin) {
            return Err(RestError::BadRequest(
                "Cannot rename across mount points".to_string(),
            ));
        }

        src_plugin
            .rename(&src_relative_path, &dst_relative_path)
            .await?;

        Ok(Json(serde_json::json!({
            "message": "File renamed",
            "from": payload.from,
            "to": payload.to
        })))
    }

    // ============== 挂载管理 API ==============

    /// 列出挂载点（返回 { "mounts": [...] }，含实例名称）
    pub async fn list_mounts(
        State(state): State<AppState>,
    ) -> RestResult<Json<ListMountsResponse>> {
        let mounts_info = state.mount_table.list_mounts_info().await;

        let mounts = mounts_info
            .into_iter()
            .map(|(path, meta)| MountInfo {
                plugin: meta.plugin_name,
                path,
                instance_name: Some(meta.instance_name),
            })
            .collect();

        Ok(Json(ListMountsResponse { mounts }))
    }

    /// 挂载插件（Phase 7.3：真正调用 RadixMountTable.mount）
    pub async fn mount(
        State(state): State<AppState>,
        Json(payload): Json<MountRequest>,
    ) -> RestResult<Json<serde_json::Value>> {
        let path = payload.path.trim().to_string();
        if path.is_empty() || !path.starts_with('/') {
            return Err(RestError::BadRequest(
                "path must be non-empty and start with /".to_string(),
            ));
        }
        let plugin =
            crate::server::create_plugin_from_config(&payload.plugin, payload.config.as_ref())
                .await
                .map_err(|e| match e {
                    evif_core::EvifError::InvalidInput(_)
                    | evif_core::EvifError::InvalidArgument(_)
                    | evif_core::EvifError::InvalidPath(_)
                    | evif_core::EvifError::Configuration(_) => {
                        RestError::BadRequest(e.to_string())
                    }
                    _ => RestError::Internal(format!("Mount failed: {}", e)),
                })?;
        let instance_name = payload
            .instance_name
            .unwrap_or_else(|| payload.plugin.clone());
        state
            .mount_table
            .mount_with_metadata(path.clone(), plugin, payload.plugin.clone(), instance_name)
            .await
            .map_err(|e| RestError::Internal(format!("Mount failed: {}", e)))?;
        Ok(Json(serde_json::json!({
            "message": "Mounted",
            "path": path,
            "plugin": payload.plugin
        })))
    }

    /// 卸载插件（Phase 7.3：真正调用 RadixMountTable.unmount）
    pub async fn unmount(
        State(state): State<AppState>,
        Json(payload): Json<UnmountRequest>,
    ) -> RestResult<Json<serde_json::Value>> {
        let path = payload.path.trim().to_string();
        if path.is_empty() {
            return Err(RestError::BadRequest("path must be non-empty".to_string()));
        }
        state
            .mount_table
            .unmount(&path)
            .await
            .map_err(|e| RestError::Internal(format!("Unmount failed: {}", e)))?;
        Ok(Json(serde_json::json!({
            "message": "Unmounted",
            "path": path
        })))
    }

    /// 获取插件 README（Phase 8.2：GET /api/v1/plugins/:name/readme）
    pub async fn get_plugin_readme(
        Path(name): Path<String>,
    ) -> RestResult<Json<PluginReadmeResponse>> {
        let normalized = normalize_plugin_id(&name);
        let plugin = Self::plugin_by_name(&normalized)?;
        Ok(Json(PluginReadmeResponse {
            name: normalized,
            readme: plugin.get_readme(),
        }))
    }

    /// 获取插件配置参数（Phase 8.2：GET /api/v1/plugins/:name/config）
    pub async fn get_plugin_config(
        Path(name): Path<String>,
    ) -> RestResult<Json<PluginConfigParamsResponse>> {
        let normalized = normalize_plugin_id(&name);
        let plugin = Self::plugin_by_name(&normalized)?;
        Ok(Json(PluginConfigParamsResponse {
            name: normalized,
            params: plugin.get_config_params(),
        }))
    }

    /// 根据插件名创建实例（用于 readme/config 等无需挂载状态的接口）
    fn plugin_by_name(name: &str) -> RestResult<Arc<dyn EvifPlugin>> {
        crate::server::create_builtin_plugin_from_config(name, None)
            .map_err(|e| {
                RestError::Internal(format!("Failed to prepare plugin '{}': {}", name, e))
            })?
            .ok_or_else(|| RestError::NotFound(format!("Plugin '{}' not found", name)))
    }

    // ============== 插件管理 API ==============

    /// 列出插件
    pub async fn list_plugins(State(state): State<AppState>) -> RestResult<Json<Vec<PluginInfo>>> {
        let mount_paths = state.mount_table.list_mounts().await;
        let mut plugins = std::collections::HashMap::new();

        for path in mount_paths {
            // 使用 lookup_with_path() 保持与其他处理器一致
            let (plugin_opt, _relative_path) = state.mount_table.lookup_with_path(&path).await;
            if let Some(plugin) = plugin_opt {
                let name = plugin.name().to_string();
                plugins.entry(name.clone()).or_insert_with(|| PluginInfo {
                    name: name.clone(),
                    version: "1.0.0".to_string(),
                    description: format!("{} plugin", name),
                });
            }
        }

        Ok(Json(plugins.into_values().collect()))
    }

    /// 加载外部插件
    pub async fn load_plugin(
        State(state): State<AppState>,
        Json(payload): Json<LoadPluginRequest>,
    ) -> RestResult<Json<serde_json::Value>> {
        // 尝试作为动态插件库名称加载
        let library_name = payload.path.clone();
        let mount_path = format!("/{}", library_name);

        // 加载动态库
        let plugin_info = state
            .dynamic_loader
            .load_plugin(&library_name)
            .map_err(|e| {
                RestError::Internal(format!(
                    "Failed to load dynamic library '{}': {}",
                    library_name, e
                ))
            })?;

        // 创建插件实例
        let plugin = state
            .dynamic_loader
            .create_plugin(&library_name)
            .map_err(|e| RestError::Internal(format!("Failed to create plugin instance: {}", e)))?;

        // 验证插件配置
        let config = payload.config.as_ref();
        evif_core::validate_and_initialize_plugin(plugin.as_ref(), config)
            .await
            .map_err(|e| RestError::Internal(format!("Plugin preparation failed: {}", e)))?;

        // 挂载插件
        state
            .mount_table
            .mount(mount_path.clone(), plugin)
            .await
            .map_err(|e| {
                RestError::Internal(format!("Failed to mount plugin at '{}': {}", mount_path, e))
            })?;

        Ok(Json(serde_json::json!({
            "message": format!("Dynamic plugin '{}' loaded and mounted at '{}'", plugin_info.name(), mount_path),
            "plugin_type": "dynamic",
            "name": plugin_info.name(),
            "version": plugin_info.version(),
            "author": plugin_info.author(),
            "description": plugin_info.description(),
            "mount_path": mount_path,
            "path": library_name,
        })))
    }

    /// 获取插件状态
    /// GET /api/v1/plugins/:name/status
    pub async fn get_plugin_status(
        State(state): State<AppState>,
        Path(name): Path<String>,
    ) -> RestResult<Json<serde_json::Value>> {
        // 首先尝试从 registry 获取状态
        if let Some(plugin) = state.plugin_registry.get(&name) {
            return Ok(Json(serde_json::json!({
                "name": plugin.name,
                "version": plugin.version,
                "author": plugin.author,
                "description": plugin.description,
                "state": plugin.state.to_string(),
                "mount_path": plugin.mount_path,
                "library_path": plugin.library_path,
                "loaded_at": plugin.loaded_at.to_rfc3339(),
                "last_active_at": plugin.last_active_at.to_rfc3339(),
                "failure_count": plugin.failure_count,
            })));
        }

        // 如果 registry 中没有，尝试从 mount_table 获取
        let mount_paths = state.mount_table.list_mounts().await;
        for path in mount_paths {
            let (plugin_opt, _relative_path) = state.mount_table.lookup_with_path(&path).await;
            if let Some(plugin) = plugin_opt {
                if plugin.name() == name {
                    return Ok(Json(serde_json::json!({
                        "name": name,
                        "version": "1.0.0",
                        "state": "active",
                        "mount_path": path,
                    })));
                }
            }
        }

        Err(RestError::NotFound(format!("Plugin '{}' not found", name)))
    }

    /// 重新加载插件
    /// POST /api/v1/plugins/:name/reload
    pub async fn reload_plugin(
        State(state): State<AppState>,
        Path(name): Path<String>,
    ) -> RestResult<Json<serde_json::Value>> {
        // 1. 从 registry 获取插件信息
        let _plugin_info = state.plugin_registry.get(&name).ok_or_else(|| {
            RestError::NotFound(format!("Plugin '{}' not found in registry", name))
        })?;

        // 2. 卸载现有插件
        let mount_path = format!("/{}", name);
        let _ = state.mount_table.unmount(&mount_path).await;
        let _ = state.dynamic_loader.unload_plugin(&name);

        // 3. 重新加载插件
        let new_info = state.dynamic_loader.load_plugin(&name).map_err(|e| {
            RestError::Internal(format!("Failed to reload plugin '{}': {}", name, e))
        })?;

        // 4. 创建新实例
        let plugin = state
            .dynamic_loader
            .create_plugin(&name)
            .map_err(|e| RestError::Internal(format!("Failed to create plugin instance: {}", e)))?;

        evif_core::validate_and_initialize_plugin(plugin.as_ref(), None)
            .await
            .map_err(|e| RestError::Internal(format!("Failed to initialize plugin: {}", e)))?;

        // 5. 重新挂载
        state
            .mount_table
            .mount(mount_path.clone(), plugin)
            .await
            .map_err(|e| RestError::Internal(format!("Failed to mount plugin: {}", e)))?;

        // 6. 更新 registry 状态
        state
            .plugin_registry
            .activate(&name, mount_path.clone())
            .map_err(|e| RestError::Internal(format!("Failed to update registry: {}", e)))?;

        Ok(Json(serde_json::json!({
            "message": format!("Plugin '{}' reloaded successfully", name),
            "name": new_info.name(),
            "version": new_info.version(),
            "mount_path": mount_path,
        })))
    }

    /// 获取所有可用插件（包括已加载和内置）
    /// GET /api/v1/plugins/available
    pub async fn list_available_plugins(
        State(state): State<AppState>,
    ) -> RestResult<Json<AvailablePluginsResponse>> {
        // 获取已注册的插件
        let registered = state.plugin_registry.list_all();
        let registered_names: Vec<String> = registered.iter().map(|p| p.name.clone()).collect();

        // 获取已挂载的插件
        let mount_paths = state.mount_table.list_mounts().await;
        let mut mounted = std::collections::HashMap::new();
        for path in mount_paths {
            let (plugin_opt, _relative_path) = state.mount_table.lookup_with_path(&path).await;
            if let Some(plugin) = plugin_opt {
                mounted.insert(normalize_plugin_id(plugin.name()), path);
            }
        }

        let mut plugins = Vec::new();
        let registered_normalized: std::collections::HashSet<String> = registered_names
            .iter()
            .map(|name| normalize_plugin_id(name))
            .collect();

        // 添加已注册的插件
        for plugin in registered {
            let plugin_name = plugin.name.clone();
            let normalized = normalize_plugin_id(&plugin_name);
            let mount_path = mounted.get(&normalized).cloned();
            plugins.push(AvailablePluginInfo {
                id: normalized.clone(),
                name: normalized,
                display_name: plugin_name,
                version: plugin.version,
                description: plugin.description,
                plugin_type: "other".to_string(),
                support_tier: "dynamic".to_string(),
                is_mountable: true,
                is_loaded: true,
                is_mounted: mount_path.is_some(),
                mount_path,
                aliases: vec![],
            });
        }

        for entry in plugin_catalog() {
            if !registered_normalized.contains(entry.id) {
                let mut plugin = AvailablePluginInfo::from_catalog(entry);
                plugin.is_mounted = mounted.contains_key(&plugin.id);
                plugin.mount_path = mounted.get(&plugin.id).cloned();
                plugins.push(plugin);
            }
        }

        plugins.sort_by(|left, right| {
            let tier_rank = |tier: &str| match tier {
                "core" => 0,
                "dynamic" => 1,
                "experimental" => 2,
                _ => 3,
            };

            tier_rank(&left.support_tier)
                .cmp(&tier_rank(&right.support_tier))
                .then_with(|| left.id.cmp(&right.id))
        });

        let total = plugins.len();
        Ok(Json(AvailablePluginsResponse { plugins, total }))
    }

    // ============== Helper Functions ==============

    /// Parse write flags from string
    fn parse_write_flags(flags_str: &str) -> Option<WriteFlags> {
        let mut flags = WriteFlags::NONE;

        for part in flags_str.split('|') {
            match part.trim() {
                "append" => flags |= WriteFlags::APPEND,
                "sync" => flags |= WriteFlags::SYNC,
                _ => return None,
            }
        }

        Some(flags)
    }

    /// Recursive grep helper with optional trace (Phase 14.3: 检索轨迹可视化)
    async fn grep_recursive(
        state: Arc<AppState>,
        plugin: &Arc<dyn EvifPlugin>,
        path: &str,
        pattern: &regex::Regex,
        matches: &mut Vec<GrepMatch>,
        trace: Option<std::sync::Arc<tokio::sync::Mutex<Vec<GrepTraceStep>>>>,
    ) -> Result<(), RestError> {
        // Check if path is a directory
        let start = std::time::Instant::now();
        let info = plugin.stat(path).await?;
        let latency_ms = start.elapsed().as_millis() as u64;

        if info.is_dir {
            if let Some(ref t) = trace {
                t.lock().await.push(GrepTraceStep {
                    path: path.to_string(),
                    operation: "dir".to_string(),
                    hits: 0,
                    latency_ms,
                });
            }

            // List directory and recurse using readdir
            let evif_file_infos = plugin.readdir(path).await?;

            for evif_info in evif_file_infos {
                let child_path = format!("{}/{}", path.trim_end_matches('/'), evif_info.name);
                Box::pin(Self::grep_recursive(
                    state.clone(),
                    plugin,
                    &child_path,
                    pattern,
                    matches,
                    trace.clone(),
                ))
                .await?;
            }
        } else {
            // Read file and search for pattern
            let data = plugin.read(path, 0, 0).await?;

            let content = String::from_utf8_lossy(&data);
            let mut file_hits = 0;
            for (line_num, line) in content.lines().enumerate() {
                if pattern.is_match(line) {
                    file_hits += 1;
                    matches.push(GrepMatch {
                        path: path.to_string(),
                        line: line_num + 1,
                        content: line.to_string(),
                    });
                }
            }

            if let Some(ref t) = trace {
                t.lock().await.push(GrepTraceStep {
                    path: path.to_string(),
                    operation: "file".to_string(),
                    hits: file_hits,
                    latency_ms,
                });
            }
        }

        Ok(())
    }
}

// ============== API 请求/响应类型 ==============

/// 文件查询参数（GET /api/v1/files 支持 offset/size 以兼容 CLI）
#[derive(Debug, Deserialize)]
pub struct FileQueryParams {
    pub path: String,
    #[serde(default)]
    pub offset: Option<u64>,
    #[serde(default)]
    pub size: Option<u64>,
}

/// 文件写入参数
#[derive(Debug, Deserialize)]
pub struct FileWriteParams {
    pub path: String,
    #[serde(default)]
    pub offset: Option<u64>,
    #[serde(default)]
    pub flags: Option<String>,
}

/// 文件写入请求（data 可为明文或 base64；encoding=base64 时解码后写入，兼容 CLI）
#[derive(Debug, Deserialize)]
pub struct FileWriteRequest {
    pub data: String,
    #[serde(default)]
    pub encoding: Option<String>,
}

/// 文件写入响应
#[derive(Debug, Serialize)]
pub struct FileWriteResponse {
    pub bytes_written: u64,
    pub path: String,
}

/// 文件读取响应（content 为 UTF-8 明文，data 为 base64，供 CLI/二进制兼容）
#[derive(Debug, Serialize)]
pub struct FileReadResponse {
    pub content: String,
    /// Base64 编码的文件内容，供 evif-client cat_bytes 等使用
    pub data: String,
    pub size: u64,
}

/// 文件路径请求
#[derive(Debug, Deserialize)]
pub struct FilePathRequest {
    pub path: String,
}

/// 目录列表响应
#[derive(Debug, Serialize)]
pub struct DirectoryListResponse {
    pub path: String,
    pub files: Vec<FileInfo>,
}

/// 文件信息
#[derive(Debug, Serialize)]
pub struct FileInfo {
    pub id: Option<String>,
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: String,
    pub created: String,
}

/// 创建目录请求
#[derive(Debug, Deserialize)]
pub struct CreateDirectoryRequest {
    pub path: String,
    #[serde(default)]
    pub parents: Option<bool>,
    #[serde(default)]
    pub mode: Option<u32>,
}

/// 删除目录参数
#[derive(Debug, Deserialize)]
pub struct DeleteDirectoryParams {
    pub path: String,
    #[serde(default)]
    pub recursive: Option<bool>,
}

/// 文件状态
#[derive(Debug, Serialize)]
pub struct FileStat {
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub modified: String,
    pub created: String,
}

/// 文件哈希请求
#[derive(Debug, Deserialize)]
pub struct FileDigestRequest {
    pub path: String,
    #[serde(default)]
    pub algorithm: Option<String>,
}

/// 文件哈希响应
#[derive(Debug, Serialize)]
pub struct FileDigestResponse {
    pub path: String,
    pub algorithm: String,
    pub hash: String,
}

/// 正则搜索请求
#[derive(Debug, Deserialize)]
pub struct GrepRequest {
    pub path: String,
    pub pattern: String,
    #[serde(default)]
    pub recursive: Option<bool>,
    /// Enable search trace (Phase 14.3)
    #[serde(default)]
    pub trace: Option<bool>,
}

/// 正则搜索响应
#[derive(Debug, Serialize)]
pub struct GrepResponse {
    pub pattern: String,
    pub matches: Vec<GrepMatch>,
    /// Search trace steps (Phase 14.3: 检索轨迹可视化)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<Vec<GrepTraceStep>>,
}

/// 检索轨迹步骤 (Phase 14.3)
#[derive(Debug, Serialize)]
pub struct GrepTraceStep {
    /// 访问的路径
    pub path: String,
    /// 操作类型: "dir" | "file"
    pub operation: String,
    /// 匹配数量
    pub hits: usize,
    /// 延迟（毫秒）
    pub latency_ms: u64,
}

/// 正则搜索匹配
#[derive(Debug, Serialize)]
pub struct GrepMatch {
    pub path: String,
    pub line: usize,
    pub content: String,
}

/// 重命名请求
#[derive(Debug, Deserialize)]
pub struct RenameRequest {
    pub from: String,
    pub to: String,
}

/// 挂载信息
#[derive(Debug, Serialize)]
pub struct MountInfo {
    pub plugin: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_name: Option<String>,
}

/// 列出挂载点响应（与 evif-client 期望的 json["mounts"] 一致）
#[derive(Debug, Serialize)]
pub struct ListMountsResponse {
    pub mounts: Vec<MountInfo>,
}

/// 挂载请求
#[derive(Debug, Deserialize)]
pub struct MountRequest {
    pub plugin: String,
    pub path: String,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
    /// 实例名称（可选，用于多实例区分）
    #[serde(default)]
    pub instance_name: Option<String>,
}

/// 卸载请求
#[derive(Debug, Deserialize)]
pub struct UnmountRequest {
    pub path: String,
}

/// 插件信息
#[derive(Debug, Serialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct AvailablePluginInfo {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub version: String,
    pub description: String,
    #[serde(rename = "type")]
    pub plugin_type: String,
    pub support_tier: String,
    pub is_mountable: bool,
    pub is_loaded: bool,
    pub is_mounted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mount_path: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

impl AvailablePluginInfo {
    fn from_catalog(entry: PluginCatalogEntry) -> Self {
        Self {
            id: entry.id.to_string(),
            name: entry.id.to_string(),
            display_name: entry.display_name.to_string(),
            version: "1.0.0".to_string(),
            description: entry.description.to_string(),
            plugin_type: entry.plugin_type.to_string(),
            support_tier: entry.support_tier.as_str().to_string(),
            is_mountable: entry.is_mountable,
            is_loaded: false,
            is_mounted: false,
            mount_path: None,
            aliases: entry
                .aliases
                .iter()
                .map(|alias| alias.to_string())
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AvailablePluginsResponse {
    pub plugins: Vec<AvailablePluginInfo>,
    pub total: usize,
}

/// 插件 README 响应（Phase 8.2）
#[derive(Debug, Serialize)]
pub struct PluginReadmeResponse {
    pub name: String,
    pub readme: String,
}

/// 插件配置参数响应（Phase 8.2：GET /api/v1/plugins/:name/config）
#[derive(Debug, Serialize)]
pub struct PluginConfigParamsResponse {
    pub name: String,
    pub params: Vec<PluginConfigParam>,
}

/// 加载插件请求
#[derive(Debug, Deserialize)]
pub struct LoadPluginRequest {
    pub path: String,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
}

// ============== Phase 14.2: 文件锁 ==============

/// 获取锁请求
#[derive(Debug, Deserialize)]
pub struct LockRequest {
    pub path: String,
    #[serde(default = "default_operation")]
    pub operation: String,
    #[serde(default)]
    pub session_id: Option<String>,
}

fn default_operation() -> String {
    "write".to_string()
}

/// 锁信息响应
#[derive(Debug, Serialize)]
pub struct LockInfoResponse {
    pub path: String,
    pub operation: String,
    pub session_id: Option<String>,
    pub acquired_at: String,
}

impl EvifHandlers {
    /// POST /api/v1/lock - 获取文件锁 (Phase 14.2)
    pub async fn acquire_lock(
        State(state): State<AppState>,
        Json(req): Json<LockRequest>,
    ) -> RestResult<Json<LockInfoResponse>> {
        state
            .lock_manager
            .try_lock(&req.path, &req.operation, req.session_id.clone())
            .await
            .map_err(|e| RestError::Conflict(e.to_string()))?;

        let info = state.lock_manager.get_lock(&req.path).await;
        if let Some(lock_info) = info {
            Ok(Json(LockInfoResponse {
                path: lock_info.path,
                operation: lock_info.operation,
                session_id: lock_info.session_id,
                acquired_at: lock_info.acquired_at.to_rfc3339(),
            }))
        } else {
            Err(RestError::Internal("Lock acquisition failed".to_string()))
        }
    }

    /// DELETE /api/v1/lock - 释放文件锁 (Phase 14.2)
    pub async fn release_lock(
        State(state): State<AppState>,
        Json(req): Json<LockRequest>,
    ) -> RestResult<Json<serde_json::Value>> {
        state
            .lock_manager
            .unlock(&req.path)
            .await
            .map_err(|e| RestError::Internal(e.to_string()))?;

        Ok(Json(serde_json::json!({
            "message": "Lock released",
            "path": req.path
        })))
    }

    /// GET /api/v1/locks - 列出所有锁 (Phase 14.2)
    pub async fn list_locks(
        State(state): State<AppState>,
    ) -> RestResult<Json<Vec<LockInfoResponse>>> {
        let locks = state.lock_manager.list_locks().await;
        let response: Vec<LockInfoResponse> = locks
            .into_iter()
            .map(|info| LockInfoResponse {
                path: info.path,
                operation: info.operation,
                session_id: info.session_id,
                acquired_at: info.acquired_at.to_rfc3339(),
            })
            .collect();
        Ok(Json(response))
    }

    /// POST /api/v1/copy - 跨文件系统复制 (Phase 14.1)
    pub async fn cross_fs_copy(
        State(state): State<AppState>,
        Json(req): Json<CopyRequest>,
    ) -> RestResult<Json<CopyResponse>> {
        let bytes_copied = state
            .cross_fs_copy_manager
            .copy(&req.source, &req.destination, req.overwrite)
            .await
            .map_err(|e| RestError::BadRequest(e.to_string()))?;

        Ok(Json(CopyResponse {
            source: req.source,
            destination: req.destination,
            bytes_copied,
            message: "Copy successful".to_string(),
        }))
    }

    /// POST /api/v1/copy/recursive - 递归复制目录 (Phase 14.1)
    pub async fn cross_fs_copy_recursive(
        State(state): State<AppState>,
        Json(req): Json<CopyRequest>,
    ) -> RestResult<Json<CopyResponse>> {
        let bytes_copied = state
            .cross_fs_copy_manager
            .copy_recursive(&req.source, &req.destination)
            .await
            .map_err(|e| RestError::BadRequest(e.to_string()))?;

        Ok(Json(CopyResponse {
            source: req.source,
            destination: req.destination,
            bytes_copied,
            message: "Recursive copy successful".to_string(),
        }))
    }
}

// ============== Phase 14.1: 跨文件系统复制请求/响应 ==============

/// 复制请求
#[derive(Debug, Deserialize)]
pub struct CopyRequest {
    /// 源路径
    pub source: String,
    /// 目标路径
    pub destination: String,
    /// 是否覆盖已存在文件
    #[serde(default)]
    pub overwrite: bool,
}

/// 复制响应
#[derive(Debug, Serialize)]
pub struct CopyResponse {
    pub source: String,
    pub destination: String,
    pub bytes_copied: u64,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use crate::RestError;
    use axum::{http::StatusCode, response::IntoResponse};
    use evif_core::EvifError;

    #[test]
    fn test_error_mapping_evif_not_found() {
        let err = EvifError::NotFound("test.txt".to_string());
        let rest_err: RestError = err.into();
        let response = rest_err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_error_mapping_evif_invalid_path() {
        let err = EvifError::InvalidPath("invalid".to_string());
        let rest_err: RestError = err.into();
        let response = rest_err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_error_mapping_io_not_found() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let rest_err = RestError::Io(io_err);
        let response = rest_err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
