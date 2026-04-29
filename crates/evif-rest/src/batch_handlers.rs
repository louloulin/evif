// Batch Operations Handlers
//
// 提供 REST API 端点用于批量文件操作
//
// 端点：
// - POST /api/v1/batch/copy - 批量复制文件
// - POST /api/v1/batch/delete - 批量删除文件
// - GET /api/v1/batch/progress/<id> - 获取操作进度
// - GET /api/v1/batch/operations - 列出所有操作
// - DELETE /api/v1/batch/operation/<id> - 取消操作

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json, Router,
};
use evif_core::{
    BatchCopyRequest, BatchDeleteRequest, BatchExecutor, BatchProgress, RadixMountTable,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::Mutex;
use tokio::sync::broadcast;

/// 批量操作 ID 类型
type OperationId = String;

/// 批量操作状态
#[derive(Debug, Clone, Serialize)]
pub enum OperationStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// 批量操作信息
#[derive(Debug, Clone, Serialize)]
pub struct BatchOperationInfo {
    /// 操作 ID
    pub id: OperationId,

    /// 操作类型
    pub operation_type: String,

    /// 状态
    pub status: OperationStatus,

    /// 进度 (0-100)
    pub progress: f64,

    /// 当前文件
    pub current_file: Option<String>,

    /// 错误信息（如果失败）
    pub error: Option<String>,

    /// 开始时间（毫秒时间戳）
    pub start_time: u64,

    /// 结束时间（毫秒时间戳）
    pub end_time: Option<u64>,
}

/// 批量操作路由的共享状态（axum 0.7 仅支持单一 State）
#[derive(Clone)]
pub struct BatchState {
    pub manager: BatchOperationManager,
    pub mount_table: Arc<RadixMountTable>,
}

/// 批量操作管理器
#[derive(Clone)]
pub struct BatchOperationManager {
    /// 活跃操作映射
    operations: Arc<Mutex<HashMap<OperationId, BatchOperationInfo>>>,

    /// 进度更新广播
    progress_tx: broadcast::Sender<BatchOperationInfo>,
}

impl BatchOperationManager {
    pub fn new() -> Self {
        let (progress_tx, _) = broadcast::channel(100);

        Self {
            operations: Arc::new(Mutex::new(HashMap::new())),
            progress_tx,
        }
    }

    /// 创建新操作
    pub fn create_operation(&self, id: OperationId, operation_type: String) -> BatchOperationInfo {
        let info = BatchOperationInfo {
            id: id.clone(),
            operation_type,
            status: OperationStatus::Pending,
            progress: 0.0,
            current_file: None,
            error: None,
            start_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            end_time: None,
        };

        {
            let mut ops = self.operations.lock();
            ops.insert(id.clone(), info.clone());
        }

        info
    }

    /// 更新操作进度
    pub fn update_progress(&self, id: &str, progress: f64, current_file: Option<String>) {
        if let Some(mut info) = self.get_operation(id) {
            info.progress = progress;
            info.current_file = current_file;
            self.set_operation(id, info.clone());
            let _ = self.progress_tx.send(info);
        }
    }

    /// 标记操作完成
    pub fn mark_completed(&self, id: &str, _result: Option<String>) {
        if let Some(mut info) = self.get_operation(id) {
            info.status = OperationStatus::Completed;
            info.progress = 100.0;
            info.end_time = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            );
            self.set_operation(id, info.clone());
            let _ = self.progress_tx.send(info);
        }
    }

    /// 标记操作失败
    pub fn mark_failed(&self, id: &str, error: String) {
        if let Some(mut info) = self.get_operation(id) {
            info.status = OperationStatus::Failed;
            info.error = Some(error);
            info.end_time = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            );
            self.set_operation(id, info.clone());
            let _ = self.progress_tx.send(info);
        }
    }

    /// 取消操作
    pub fn cancel_operation(&self, id: &str) -> bool {
        if let Some(mut info) = self.get_operation(id) {
            info.status = OperationStatus::Cancelled;
            info.end_time = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            );
            self.set_operation(id, info.clone());
            let _ = self.progress_tx.send(info);
            true
        } else {
            false
        }
    }

    fn get_operation(&self, id: &str) -> Option<BatchOperationInfo> {
        let ops = self.operations.lock();
        ops.get(id).cloned()
    }

    fn set_operation(&self, id: &str, info: BatchOperationInfo) {
        let mut ops = self.operations.lock();
        ops.insert(id.to_string(), info);
    }

    /// 获取所有操作
    pub fn list_operations(&self) -> Vec<BatchOperationInfo> {
        let ops = self.operations.lock();
        ops.values().cloned().collect()
    }
}

impl Default for BatchOperationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 请求体：批量复制
#[derive(Debug, Deserialize)]
pub struct BatchCopyRequestJson {
    pub sources: Vec<String>,
    pub destination: String,
    #[serde(default)]
    pub recursive: bool,
    #[serde(default)]
    pub overwrite: bool,
    #[serde(default)]
    pub concurrency: usize,
}

/// 请求体：批量删除
#[derive(Debug, Deserialize)]
pub struct BatchDeleteRequestJson {
    pub paths: Vec<String>,
    #[serde(default)]
    pub recursive: bool,
    #[serde(default)]
    pub concurrency: usize,
}

/// 响应体：批量复制
#[derive(Debug, Serialize)]
pub struct BatchCopyResponse {
    pub operation_id: String,
    pub message: String,
}

/// 响应体：批量删除
#[derive(Debug, Serialize)]
pub struct BatchDeleteResponse {
    pub operation_id: String,
    pub message: String,
}

/// 处理批量复制请求
pub async fn handle_batch_copy(
    State(state): State<BatchState>,
    Json(data): Json<BatchCopyRequestJson>,
) -> impl IntoResponse {
    let operation_id = uuid::Uuid::new_v4().to_string();
    let manager = state.manager.clone();
    let mount_table = state.mount_table.clone();

    manager.create_operation(operation_id.clone(), "batch_copy".to_string());

    let copy_req = BatchCopyRequest {
        sources: data.sources.clone(),
        destination: data.destination.clone(),
        recursive: data.recursive,
        overwrite: data.overwrite,
        concurrency: data.concurrency.clamp(1, 64),
    };

    // 执行实际的批量复制操作
    // 使用源路径中的第一个路径来确定目标插件
    let target_plugin = if !copy_req.sources.is_empty() {
        mount_table.lookup(&copy_req.sources[0]).await
    } else {
        mount_table.lookup(&copy_req.destination).await
    };

    match target_plugin {
        Some(plugin) => {
            let manager_cb = manager.clone();
            let op_id_cb = operation_id.clone();
            // 创建 BatchExecutor 并执行批量复制
            let executor = BatchExecutor::new(plugin)
                .with_concurrency(copy_req.concurrency)
                .with_progress(Box::new(move |progress: BatchProgress| {
                    manager_cb.update_progress(
                        &op_id_cb,
                        progress.percent,
                        progress.current_file.clone(),
                    );
                    if progress.percent >= 100.0 {
                        manager_cb.mark_completed(&op_id_cb, None);
                    }
                }));

            // 在后台执行批量复制
            let manager_clone = manager.clone();
            let op_id = operation_id.clone();
            tokio::spawn(async move {
                match executor.batch_copy(&copy_req).await {
                    Ok(_) => {
                        manager_clone.mark_completed(&op_id, None);
                    }
                    Err(e) => {
                        manager_clone.mark_failed(&op_id, e.to_string());
                    }
                }
            });

            Json(BatchCopyResponse {
                operation_id,
                message: "Batch copy operation started".to_string(),
            })
        }
        None => {
            manager.mark_failed(&operation_id, "No suitable plugin found".to_string());
            Json(BatchCopyResponse {
                operation_id,
                message: "Error: No suitable plugin found".to_string(),
            })
        }
    }
}

/// 处理批量删除请求
pub async fn handle_batch_delete(
    State(state): State<BatchState>,
    Json(data): Json<BatchDeleteRequestJson>,
) -> impl IntoResponse {
    let operation_id = uuid::Uuid::new_v4().to_string();
    let manager = state.manager.clone();
    let mount_table = state.mount_table.clone();

    manager.create_operation(operation_id.clone(), "batch_delete".to_string());

    let delete_req = BatchDeleteRequest {
        paths: data.paths.clone(),
        recursive: data.recursive,
        concurrency: data.concurrency.clamp(1, 64),
    };

    // 执行实际的批量删除操作
    // 使用第一个路径来确定目标插件
    let target_plugin = if !delete_req.paths.is_empty() {
        mount_table.lookup(&delete_req.paths[0]).await
    } else {
        None
    };

    match target_plugin {
        Some(plugin) => {
            let manager_cb = manager.clone();
            let op_id_cb = operation_id.clone();
            // 创建 BatchExecutor 并执行批量删除
            let executor = BatchExecutor::new(plugin)
                .with_concurrency(delete_req.concurrency)
                .with_progress(Box::new(move |progress: BatchProgress| {
                    manager_cb.update_progress(
                        &op_id_cb,
                        progress.percent,
                        progress.current_file.clone(),
                    );
                    if progress.percent >= 100.0 {
                        manager_cb.mark_completed(&op_id_cb, None);
                    }
                }));

            // 在后台执行批量删除
            let manager_clone = manager.clone();
            let op_id = operation_id.clone();
            tokio::spawn(async move {
                match executor.batch_delete(&delete_req).await {
                    Ok(_) => {
                        manager_clone.mark_completed(&op_id, None);
                    }
                    Err(e) => {
                        manager_clone.mark_failed(&op_id, e.to_string());
                    }
                }
            });

            Json(BatchDeleteResponse {
                operation_id,
                message: "Batch delete operation started".to_string(),
            })
        }
        None => {
            manager.mark_failed(&operation_id, "No suitable plugin found".to_string());
            Json(BatchDeleteResponse {
                operation_id,
                message: "Error: No suitable plugin found".to_string(),
            })
        }
    }
}

/// 路径参数：操作 ID（axum 0.7 路径参数需具名）
#[derive(serde::Deserialize)]
pub struct OperationIdPath {
    pub id: String,
}

/// 获取操作进度
pub async fn get_batch_progress(
    Path(params): Path<OperationIdPath>,
    State(state): State<BatchState>,
) -> impl IntoResponse {
    let manager = &state.manager;
    let operation_id = &params.id;
    match manager.get_operation(operation_id) {
        Some(info) => Json(info).into_response(),
        None => Json(serde_json::json!({
            "error": "Operation not found",
            "operation_id": operation_id
        }))
        .into_response(),
    }
}

/// 列出所有批量操作
pub async fn list_batch_operations(State(state): State<BatchState>) -> impl IntoResponse {
    let manager = &state.manager;
    let operations = manager.list_operations();
    Json(serde_json::json!({
        "operations": operations,
        "count": operations.len()
    }))
    .into_response()
}

/// 取消批量操作
pub async fn cancel_batch_operation(
    Path(params): Path<OperationIdPath>,
    State(state): State<BatchState>,
) -> impl IntoResponse {
    let operation_id = &params.id;
    if state.manager.cancel_operation(operation_id) {
        Json(serde_json::json!({
            "message": "Operation cancelled",
            "operation_id": operation_id
        }))
        .into_response()
    } else {
        Json(serde_json::json!({
            "error": "Operation not found",
            "operation_id": operation_id
        }))
        .into_response()
    }
}

/// 创建批量操作路由
pub fn create_batch_routes(
    manager: BatchOperationManager,
    mount_table: Arc<RadixMountTable>,
) -> Router {
    let state = BatchState {
        manager,
        mount_table,
    };
    Router::new()
        // ============== 批量操作 ==============
        .route("/api/v1/batch/copy", axum::routing::post(handle_batch_copy))
        .route(
            "/api/v1/batch/delete",
            axum::routing::post(handle_batch_delete),
        )
        .route(
            "/api/v1/batch/progress/:id",
            axum::routing::get(get_batch_progress),
        )
        .route(
            "/api/v1/batch/operations",
            axum::routing::get(list_batch_operations),
        )
        .route(
            "/api/v1/batch/operation/:id",
            axum::routing::delete(cancel_batch_operation),
        )
        .with_state(state)
}
