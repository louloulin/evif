// EVIF Handle REST API Handlers
//
// 对标 AGFS Handle API
// 提供有状态文件句柄操作的 HTTP 接口

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use evif_core::{EvifError, EvifResult, GlobalHandleManager, OpenFlags, RadixMountTable};

/// Handle状态（包含全局句柄管理器）
#[derive(Clone)]
pub struct HandleState {
    pub mount_table: Arc<RadixMountTable>,
    pub handle_manager: Arc<GlobalHandleManager>,
}

/// 打开文件句柄请求
#[derive(Debug, Deserialize)]
pub struct OpenHandleRequest {
    pub path: String,
    pub flags: String, // "r", "w", "rw", "rw-create", etc.
    pub mode: Option<u32>,
    pub lease: Option<u64>, // 租约时间（秒）
}

/// 读取请求
#[derive(Debug, Deserialize)]
pub struct ReadRequest {
    pub size: Option<usize>,
}

/// 写入请求
#[derive(Debug, Deserialize)]
pub struct WriteRequest {
    pub data: String, // Base64编码的数据
    pub offset: Option<u64>,
}

/// Seek请求
#[derive(Debug, Deserialize)]
pub struct SeekRequest {
    pub offset: i64,
    pub whence: String, // "set", "cur", "end"
}

/// 续租请求
#[derive(Debug, Deserialize)]
pub struct RenewRequest {
    pub lease: u64, // 租约时间（秒）
}

/// 打开句柄响应
#[derive(Debug, Serialize)]
pub struct OpenHandleResponse {
    pub handle_id: i64,
    pub path: String,
    pub flags: String,
    pub lease_expires_at: Option<i64>, // Unix timestamp
}

/// 读取响应
#[derive(Debug, Serialize)]
pub struct ReadResponse {
    pub data: String, // Base64编码
    pub bytes_read: usize,
    pub eof: bool,
}

/// 写入响应
#[derive(Debug, Serialize)]
pub struct WriteResponse {
    pub bytes_written: usize,
}

/// Seek响应
#[derive(Debug, Serialize)]
pub struct SeekResponse {
    pub new_offset: i64,
}

/// 句柄信息响应
#[derive(Debug, Serialize)]
pub struct HandleInfoResponse {
    pub handle_id: i64,
    pub path: String,
    pub flags: String,
    pub plugin_id: String,
    pub lease_expires_at: Option<i64>,
}

/// 句柄列表响应
#[derive(Debug, Serialize)]
pub struct HandleListResponse {
    pub handles: Vec<HandleInfoResponse>,
    pub count: usize,
}

/// EVIF Handle API 处理器
pub struct HandleHandlers;

impl HandleHandlers {
    // ==================== 解析辅助函数 ====================

    /// 解析打开标志字符串
    fn parse_flags(flags_str: &str) -> EvifResult<OpenFlags> {
        let flags = match flags_str.to_lowercase().as_str() {
            "r" | "read" | "readonly" => OpenFlags::READ_ONLY,
            "w" | "write" | "writeonly" => OpenFlags::WRITE_ONLY,
            "rw" | "read-write" => OpenFlags::READ_WRITE,
            "rw-create" => OpenFlags::READ_WRITE | OpenFlags::CREATE,
            "rw-create-excl" => OpenFlags::READ_WRITE | OpenFlags::CREATE | OpenFlags::EXCLUSIVE,
            "rw-truncate" => OpenFlags::READ_WRITE | OpenFlags::TRUNCATE,
            "append" => OpenFlags::WRITE_ONLY | OpenFlags::APPEND,
            "rw-append" => OpenFlags::READ_WRITE | OpenFlags::APPEND,
            _ => {
                return Err(EvifError::InvalidInput(format!(
                    "Invalid flags: {}",
                    flags_str
                )))
            }
        };
        Ok(flags)
    }

    /// 解析whence字符串
    fn parse_whence(whence: &str) -> EvifResult<u8> {
        match whence.to_lowercase().as_str() {
            "set" => Ok(0),
            "cur" => Ok(1),
            "end" => Ok(2),
            _ => Err(EvifError::InvalidInput(format!(
                "Invalid whence: {}",
                whence
            ))),
        }
    }

    // ==================== Handle操作 ====================

    /// 打开文件句柄
    /// POST /api/v1/handles/open
    pub async fn open_handle(
        State(state): State<HandleState>,
        Json(req): Json<OpenHandleRequest>,
    ) -> Result<Json<OpenHandleResponse>, RestError> {
        // 解析标志
        let flags = Self::parse_flags(&req.flags)?;

        // 使用lookup查找插件
        let plugin = state
            .mount_table
            .lookup(&req.path)
            .await
            .ok_or_else(|| RestError::NotFound(format!("Path not found: {}", req.path)))?;

        // 尝试转换为HandleFS
        let handle_fs = plugin
            .as_handle_fs()
            .ok_or_else(|| RestError::Internal("Plugin does not support HandleFS".to_string()))?;

        // 打开句柄
        let file_handle = handle_fs
            .open_handle(&req.path, flags, req.mode.unwrap_or(0o644))
            .await
            .map_err(|e| RestError::Internal(e.to_string()))?;

        // 分配全局ID
        let handle_id = state.handle_manager.allocate_id();

        // 计算租约过期时间
        let lease_duration = req.lease.map(Duration::from_secs);
        let lease_expires_at = lease_duration.map(|d| {
            chrono::Utc::now()
                .checked_add_signed(chrono::Duration::from_std(d).unwrap())
                .unwrap()
                .timestamp()
        });

        // 注册到全局管理器
        state
            .handle_manager
            .register_handle(
                handle_id,
                req.path.clone(),
                req.path.clone(), // full_path same as path for now
                Some(file_handle),
            )
            .await
            .map_err(|e| RestError::Internal(e.to_string()))?;

        // 不需要关闭本地句柄，它已经被转移到管理器
        // let _ = file_handle.close().await;

        Ok(Json(OpenHandleResponse {
            handle_id,
            path: req.path,
            flags: req.flags,
            lease_expires_at,
        }))
    }

    /// 获取句柄信息
    /// GET /api/v1/handles/{id}
    pub async fn get_handle(
        State(state): State<HandleState>,
        Path(id): Path<i64>,
    ) -> Result<Json<HandleInfoResponse>, RestError> {
        // 获取句柄信息 (id, mount_path, full_path, expires_at)
        let (_hid, mount_path, full_path, expires_at) =
            state
                .handle_manager
                .get_handle(id)
                .await
                .map_err(|e| RestError::NotFound(e.to_string()))?;

        Ok(Json(HandleInfoResponse {
            handle_id: id,
            path: full_path,
            flags: "N/A".to_string(), // HandleInfo doesn't store flags
            plugin_id: mount_path,    // Using mount_path as plugin_id
            lease_expires_at: Some(
                expires_at
                    .duration_since(std::time::Instant::now())
                    .as_secs() as i64,
            ),
        }))
    }

    /// 读取数据
    /// POST /api/v1/handles/{id}/read
    pub async fn read_handle(
        State(state): State<HandleState>,
        Path(id): Path<i64>,
        Json(req): Json<ReadRequest>,
    ) -> Result<Json<ReadResponse>, RestError> {
        // 获取句柄信息 (hid, mount_path, full_path, expires_at)
        let (_hid, mount_path, full_path, _expires_at) = state
            .handle_manager
            .get_handle(id)
            .await
            .map_err(|e| RestError::NotFound(e.to_string()))?;

        // 使用mount_table的lookup查找插件
        let plugin =
            state.mount_table.lookup(&mount_path).await.ok_or_else(|| {
                RestError::NotFound(format!("Plugin not found at: {}", mount_path))
            })?;

        // 读取文件数据
        let size = req.size.unwrap_or(4096) as u64;
        let data = plugin
            .read(&full_path, 0, size)
            .await
            .map_err(|e| RestError::Internal(format!("Failed to read: {}", e)))?;

        // Base64编码
        use base64::Engine;
        let encoded_data = base64::engine::general_purpose::STANDARD.encode(&data);
        let bytes_read = data.len();

        Ok(Json(ReadResponse {
            data: encoded_data,
            bytes_read,
            eof: bytes_read == 0,
        }))
    }

    /// 写入数据
    /// POST /api/v1/handles/{id}/write
    pub async fn write_handle(
        State(state): State<HandleState>,
        Path(id): Path<i64>,
        Json(req): Json<WriteRequest>,
    ) -> Result<Json<WriteResponse>, RestError> {
        // 解码Base64数据
        use base64::Engine;
        let data = base64::engine::general_purpose::STANDARD
            .decode(&req.data)
            .map_err(|e| RestError::Internal(format!("Invalid base64: {}", e)))?;

        // 获取句柄信息 (hid, mount_path, full_path, expires_at)
        let (_hid, mount_path, _full_path, _expires_at) = state
            .handle_manager
            .get_handle(id)
            .await
            .map_err(|e| RestError::NotFound(e.to_string()))?;

        // 使用mount_path查找插件
        let plugin =
            state.mount_table.lookup(&mount_path).await.ok_or_else(|| {
                RestError::NotFound(format!("Plugin not found at: {}", mount_path))
            })?;

        // 转换为HandleFS
        let handle_fs = plugin
            .as_handle_fs()
            .ok_or_else(|| RestError::Internal("Plugin does not support HandleFS".to_string()))?;

        // 获取句柄
        let mut file_handle = handle_fs
            .get_handle(id)
            .await
            .map_err(|e| RestError::Internal(e.to_string()))?;

        // 写入数据
        let bytes_written = if let Some(offset) = req.offset {
            file_handle
                .write_at(&data, offset)
                .await
                .map_err(|e| RestError::Internal(e.to_string()))?
        } else {
            file_handle
                .write(&data)
                .await
                .map_err(|e| RestError::Internal(e.to_string()))?
        };

        Ok(Json(WriteResponse { bytes_written }))
    }

    /// Seek操作
    /// POST /api/v1/handles/{id}/seek
    pub async fn seek_handle(
        State(state): State<HandleState>,
        Path(id): Path<i64>,
        Json(req): Json<SeekRequest>,
    ) -> Result<Json<SeekResponse>, RestError> {
        let whence = Self::parse_whence(&req.whence)?;

        // 获取句柄信息 (hid, mount_path, full_path, expires_at)
        let (_hid, mount_path, _full_path, _expires_at) = state
            .handle_manager
            .get_handle(id)
            .await
            .map_err(|e| RestError::NotFound(e.to_string()))?;

        // 使用mount_path查找插件
        let plugin =
            state.mount_table.lookup(&mount_path).await.ok_or_else(|| {
                RestError::NotFound(format!("Plugin not found at: {}", mount_path))
            })?;

        // 转换为HandleFS
        let handle_fs = plugin
            .as_handle_fs()
            .ok_or_else(|| RestError::Internal("Plugin does not support HandleFS".to_string()))?;

        // 获取句柄
        let mut file_handle = handle_fs
            .get_handle(id)
            .await
            .map_err(|e| RestError::Internal(e.to_string()))?;

        // 执行seek
        let new_offset = file_handle
            .seek(req.offset, whence)
            .await
            .map_err(|e| RestError::Internal(e.to_string()))?;

        Ok(Json(SeekResponse { new_offset }))
    }

    /// 同步文件
    /// POST /api/v1/handles/{id}/sync
    pub async fn sync_handle(
        State(state): State<HandleState>,
        Path(id): Path<i64>,
    ) -> Result<(), RestError> {
        // 获取句柄信息 (hid, mount_path, full_path, expires_at)
        let (_hid, mount_path, _full_path, _expires_at) = state
            .handle_manager
            .get_handle(id)
            .await
            .map_err(|e| RestError::NotFound(e.to_string()))?;

        // 使用mount_path查找插件
        let plugin =
            state.mount_table.lookup(&mount_path).await.ok_or_else(|| {
                RestError::NotFound(format!("Plugin not found at: {}", mount_path))
            })?;

        // 转换为HandleFS
        let handle_fs = plugin
            .as_handle_fs()
            .ok_or_else(|| RestError::Internal("Plugin does not support HandleFS".to_string()))?;

        // 获取句柄
        let file_handle = handle_fs
            .get_handle(id)
            .await
            .map_err(|e| RestError::Internal(e.to_string()))?;

        // 同步
        file_handle
            .sync()
            .await
            .map_err(|e| RestError::Internal(e.to_string()))?;

        Ok(())
    }

    /// 关闭句柄
    /// POST /api/v1/handles/{id}/close
    pub async fn close_handle(
        State(state): State<HandleState>,
        Path(id): Path<i64>,
    ) -> Result<(), RestError> {
        // 获取句柄信息 (hid, mount_path, full_path, expires_at)
        let (_hid, mount_path, _full_path, _expires_at) = state
            .handle_manager
            .get_handle(id)
            .await
            .map_err(|e| RestError::NotFound(e.to_string()))?;

        // 使用mount_path查找插件
        let plugin =
            state.mount_table.lookup(&mount_path).await.ok_or_else(|| {
                RestError::NotFound(format!("Plugin not found at: {}", mount_path))
            })?;

        // 转换为HandleFS
        let handle_fs = plugin
            .as_handle_fs()
            .ok_or_else(|| RestError::Internal("Plugin does not support HandleFS".to_string()))?;

        // 关闭句柄
        handle_fs
            .close_handle(id)
            .await
            .map_err(|e| RestError::Internal(e.to_string()))?;

        // 从全局管理器移除
        state
            .handle_manager
            .close_handle(id)
            .await
            .map_err(|e| RestError::Internal(e.to_string()))?;

        Ok(())
    }

    /// 续租
    /// POST /api/v1/handles/{id}/renew
    pub async fn renew_handle(
        State(state): State<HandleState>,
        Path(id): Path<i64>,
        Json(req): Json<RenewRequest>,
    ) -> Result<(), RestError> {
        let lease = Duration::from_secs(req.lease);

        state
            .handle_manager
            .renew_handle(id, Some(lease))
            .await
            .map_err(|e| RestError::Internal(e.to_string()))?;

        Ok(())
    }

    /// 列出所有句柄
    /// GET /api/v1/handles
    pub async fn list_handles(
        State(state): State<HandleState>,
    ) -> Result<Json<HandleListResponse>, RestError> {
        let handle_ids = state.handle_manager.list_handles().await;

        let mut handle_responses = Vec::new();
        for handle_id in handle_ids {
            // 获取每个handle的详细信息
            if let Ok((_hid, mount_path, full_path, _expires_at)) =
                state.handle_manager.get_handle(handle_id).await
            {
                handle_responses.push(HandleInfoResponse {
                    handle_id,
                    path: full_path.clone(),
                    flags: "READ_WRITE".to_string(),
                    plugin_id: mount_path.clone(),
                    lease_expires_at: Some(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64
                            + 3600, // 默认1小时过期
                    ),
                });
            }
        }

        let count = handle_responses.len();
        Ok(Json(HandleListResponse {
            handles: handle_responses,
            count,
        }))
    }

    /// 获取句柄统计信息
    /// GET /api/v1/handles/stats
    pub async fn get_stats(
        State(state): State<HandleState>,
    ) -> Result<Json<HandleStatsResponse>, RestError> {
        let count = state.handle_manager.handle_count().await;

        Ok(Json(HandleStatsResponse {
            total: count,
            active: count, // 当前实现所有handle都是active
            idle: 0,       // 当前实现没有idle状态
        }))
    }
}

/// 句柄统计响应
#[derive(Debug, Serialize)]
pub struct HandleStatsResponse {
    pub total: usize,
    pub active: usize,
    pub idle: usize,
}

/// REST API 错误类型
#[derive(Debug, thiserror::Error)]
pub enum RestError {
    #[error("Not found: {0}")]
    NotFound(String),

    /// Phase 14.2: 资源冲突
    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

// 添加EvifError转换
impl From<EvifError> for RestError {
    fn from(err: EvifError) -> Self {
        RestError::Internal(err.to_string())
    }
}

impl axum::response::IntoResponse for RestError {
    fn into_response(self) -> axum::response::Response {
        use axum::{http::StatusCode, Json};
        use serde_json::json;

        let (status, message) = match self {
            RestError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            RestError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            RestError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(json!({
            "error": status.to_string(),
            "message": message,
        }));

        (status, body).into_response()
    }
}
