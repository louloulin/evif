// REST API 处理器

use base64::Engine;
use crate::{RestResult, RestError};
use axum::{Json, extract::{Path, State, Query}};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use evif_core::{RadixMountTable, EvifPlugin, WriteFlags, OpenFlags, PluginConfigParam, DynamicPluginLoader, PluginRegistry, PluginState as RegistryPluginState, RegisteredPlugin};
use evif_core::FileInfo as EvifFileInfo;
use evif_graph::{Graph, Node, NodeId, Metadata, NodeType, NodeBuilder};
use crate::metrics_handlers::TrafficStats;
use std::time::Instant;
use std::sync::atomic::Ordering;
use chrono::Utc;

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
    /// 图引擎实例
    pub graph: Arc<Graph>,
}

/// 节点响应
#[derive(Debug, Serialize)]
pub struct NodeResponse {
    pub id: String,
    pub node_type: String,
    pub name: String,
    pub metadata: Metadata,
}

/// 查询响应
#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub ids: Vec<String>,
    pub count: usize,
}

/// 统计响应
#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub uptime_secs: u64,
    pub total_nodes: usize,
    pub total_edges: usize,
    pub status: String,
}

/// 错误响应
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

/// EVIF 处理器
pub struct EvifHandlers;

impl EvifHandlers {
    /// 健康检查（根路径 /health，无状态）
    pub async fn health() -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "status": "healthy",
            "version": "1.0.0",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }

    /// GET /api/v1/health：与 evif-client/CLI 契约一致，返回 status、version、uptime（秒）
    pub async fn health_v1(State(state): State<AppState>) -> Json<serde_json::Value> {
        let uptime_secs = state.start_time.elapsed().as_secs();
        Json(serde_json::json!({
            "status": "healthy",
            "version": env!("CARGO_PKG_VERSION"),
            "uptime": uptime_secs
        }))
    }

    /// 获取节点
    pub async fn get_node(
        State(state): State<AppState>,
        Path(id): Path<String>,
    ) -> RestResult<Json<NodeResponse>> {
        let node_id = match uuid::Uuid::parse_str(&id) {
            Ok(id) => id,
            Err(e) => return Err(RestError::BadRequest(format!("Invalid UUID format: {}", e))),
        };
        match state.graph.get_node(&node_id) {
            Ok(node) => Ok(Json(NodeResponse {
                id: node.id.to_string(),
                node_type: node.node_type.as_str().to_string(),
                name: node.name.clone(),
                metadata: node.metadata.clone(),
            })),
            Err(e) => Err(RestError::NotFound(format!("Node not found: {}", e))),
        }
    }

    /// 创建节点
    pub async fn create_node(
        State(state): State<AppState>,
        Path(node_type): Path<String>,
        Json(payload): Json<CreateNodeRequest>,
    ) -> RestResult<Json<NodeResponse>> {
        let node_type = match node_type.to_lowercase().as_str() {
            "file" => NodeType::File,
            "directory" | "dir" => NodeType::Directory,
            "symlink" => NodeType::Symlink,
            "device" => NodeType::Device,
            "process" => NodeType::Process,
            "network" => NodeType::Network,
            other => NodeType::Custom(other.to_string()),
        };

        let node = NodeBuilder::new(node_type, payload.name)
            .build();

        match state.graph.add_node(node) {
            Ok(node_id) => {
                let node = state.graph.get_node(&node_id).unwrap();
                Ok(Json(NodeResponse {
                    id: node.id.to_string(),
                    node_type: node.node_type.as_str().to_string(),
                    name: node.name.clone(),
                    metadata: node.metadata.clone(),
                }))
            }
            Err(e) => Err(RestError::Internal(format!("Failed to create node: {}", e))),
        }
    }

    /// 删除节点
    pub async fn delete_node(
        State(state): State<AppState>,
        Path(id): Path<String>,
    ) -> RestResult<Json<serde_json::Value>> {
        let node_id = match uuid::Uuid::parse_str(&id) {
            Ok(id) => id,
            Err(e) => return Err(RestError::BadRequest(format!("Invalid UUID format: {}", e))),
        };
        match state.graph.remove_node(&node_id) {
            Ok(_) => Ok(Json(serde_json::json!({
                "success": true,
                "message": format!("Node {} deleted", id)
            }))),
            Err(e) => Err(RestError::NotFound(format!("Failed to delete node: {}", e))),
        }
    }

    /// 查询图
    /// NOTE: Graph functionality intentionally not implemented (confirmed not required for EVIF 1.8)
    pub async fn query(
        Json(payload): Json<QueryRequest>,
    ) -> RestResult<Json<QueryResponse>> {
        Err(RestError::Internal(
            "Graph functionality not implemented. Filesystem queries available via /api/v1/files/list.".to_string()
        ))
    }

    /// 获取子节点
    /// NOTE: Graph functionality intentionally not implemented (confirmed not required for EVIF 1.8)
    pub async fn get_children(
        Path(id): Path<String>,
    ) -> RestResult<Json<Vec<String>>> {
        Err(RestError::Internal(
            "Graph functionality not implemented. Use /api/v1/directories/list for filesystem operations.".to_string()
        ))
    }

    /// 获取统计信息
    pub async fn stats(State(state): State<AppState>) -> Json<StatsResponse> {
        let uptime_secs = state.start_time.elapsed().as_secs();
        Json(StatsResponse {
            uptime_secs,
            total_nodes: state.graph.node_count(),
            total_edges: state.graph.edge_count(),
            status: "running".to_string(),
        })
    }

    // ============== Metrics API（Phase 9：与 AppState 对接，返回真实 uptime/mount_count/traffic）==============

    /// 获取流量统计
    pub async fn get_traffic_stats(
        State(state): State<AppState>,
    ) -> Json<serde_json::Value> {
        let s = &state.traffic_stats;
        let total_requests = s.total_requests.load(Ordering::Relaxed);
        let total_bytes_read = s.total_bytes_read.load(Ordering::Relaxed);
        let total_bytes_written = s.total_bytes_written.load(Ordering::Relaxed);
        let read_count = s.read_count.load(Ordering::Relaxed);
        let write_count = s.write_count.load(Ordering::Relaxed);
        let avg_read = if read_count > 0 { total_bytes_read / read_count } else { 0 };
        let avg_write = if write_count > 0 { total_bytes_written / write_count } else { 0 };
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
    pub async fn get_system_status(
        State(state): State<AppState>,
    ) -> Json<serde_json::Value> {
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
    pub async fn reset_metrics(
        State(state): State<AppState>,
    ) -> Json<serde_json::Value> {
        let s = &state.traffic_stats;
        s.total_requests.store(0, Ordering::Relaxed);
        s.total_bytes_read.store(0, Ordering::Relaxed);
        s.total_bytes_written.store(0, Ordering::Relaxed);
        s.total_errors.store(0, Ordering::Relaxed);
        s.read_count.store(0, Ordering::Relaxed);
        s.write_count.store(0, Ordering::Relaxed);
        s.list_count.store(0, Ordering::Relaxed);
        s.other_count.store(0, Ordering::Relaxed);
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
        // 使用 lookup_with_path() 进行路径翻译，获取插件和相对路径
        let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&params.path).await;
        let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(format!("Path not found: {}", params.path)))?;

        let offset = params.offset.unwrap_or(0);
        let size = params.size.unwrap_or(0); // 0 表示读全部
        let data = plugin
            .read(&relative_path, offset, size)
            .await?;

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
        Query(params): Query<FileWriteParams>,
        Json(payload): Json<FileWriteRequest>,
    ) -> RestResult<Json<FileWriteResponse>> {
        // 使用 lookup_with_path() 进行路径翻译，获取插件和相对路径
        let (plugin_opt, relative_path) = state.mount_table.lookup_with_path(&params.path).await;
        let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(format!("Path not found: {}", params.path)))?;

        let offset = params.offset.unwrap_or(0) as i64;
        let flags = params.flags
            .and_then(|f| Self::parse_write_flags(&f))
            .unwrap_or(WriteFlags::NONE);

        let data = if payload.encoding.as_deref() == Some("base64") {
            base64::engine::general_purpose::STANDARD
                .decode(payload.data.trim())
                .map_err(|e| RestError::Internal(format!("Invalid base64: {}", e)))?
        } else {
            payload.data.into_bytes()
        };

        let bytes_written = plugin
            .write(&relative_path, data, offset, flags)
            .await?;

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
        let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(format!("Path not found: {}", payload.path)))?;

        plugin
            .create(&relative_path, 0o644)
            .await
            ?;

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
        let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(format!("Path not found: {}", params.path)))?;

        plugin
            .remove(&relative_path)
            .await
            ?;

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
        let (plugin_opt, relative_path) = state.mount_table
            .lookup_with_path(&params.path)
            .await;

        // 特殊情况：根路径 "/" 返回所有挂载点
        if relative_path == "/" && plugin_opt.is_none() {
            let mounts = state.mount_table.list_mounts().await;
            let files: Vec<FileInfo> = mounts.into_iter().map(|name| FileInfo {
                id: None,
                name: name.clone(),
                path: format!("/{}", name),
                is_dir: true,
                size: 0,
                modified: Utc::now().to_rfc3339(),
                created: Utc::now().to_rfc3339(),
            }).collect();

            return Ok(Json(DirectoryListResponse {
                path: "/".to_string(),
                files,
            }));
        }

        // 非根路径：使用相对路径调用插件
        let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(format!("Path not found: {}", params.path)))?;

        // List files using readdir with relative path
        let evif_file_infos = plugin
            .readdir(&relative_path)
            .await
            ?;

        let files = evif_file_infos.into_iter().map(|info| {
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
        }).collect();

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
        let (plugin_opt, relative_path) = state.mount_table
            .lookup_with_path(&payload.path)
            .await;

        let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(format!("Path not found: {}", payload.path)))?;

        // Create parent directories if requested
        if payload.parents.unwrap_or(false) {
            if let Some(parent) = std::path::Path::new(&relative_path).parent() {
                let parent_path = parent.to_str().unwrap();
                if !parent_path.is_empty() && parent_path != "/" {
                    plugin
                        .mkdir(parent_path, payload.mode.unwrap_or(0o755))
                        .await
                        ?;
                }
            }
        }

        plugin
            .mkdir(&relative_path, payload.mode.unwrap_or(0o755))
            .await
            ?;

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
        let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(format!("Path not found: {}", params.path)))?;

        // Note: EvifPlugin doesn't have rmdir/rmdir_all, just use remove
        plugin
            .remove(&relative_path)
            .await
            ?;

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
        let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(format!("Path not found: {}", params.path)))?;

        let info = plugin
            .stat(&relative_path)
            .await
            ?;

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
        let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(format!("Path not found: {}", payload.path)))?;

        let data = plugin
            .read(&relative_path, 0, 0)
            .await
            ?;

        let algorithm = payload.algorithm.unwrap_or_else(|| "sha256".to_string());
        let hash = match algorithm.to_lowercase().as_str() {
            "sha256" => {
                use sha2::Sha256;
                use digest::Digest;
                let mut hasher = Sha256::new();
                digest::Digest::update(&mut hasher, &data);
                format!("{:x}", digest::Digest::finalize(hasher))
            }
            "sha512" => {
                use sha2::Sha512;
                use digest::Digest;
                let mut hasher = Sha512::new();
                digest::Digest::update(&mut hasher, &data);
                format!("{:x}", digest::Digest::finalize(hasher))
            }
            _ => return Err(RestError::Internal(format!("Unsupported algorithm: {}", algorithm))),
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
        let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(format!("Path not found: {}", payload.path)))?;

        // Create file if it doesn't exist, update timestamp if it does
        if let Err(_) = plugin.stat(&relative_path).await {
            plugin
                .create(&relative_path, 0o644)
                .await
                ?;
        } else {
            // Update timestamp by writing empty data with append flag
            plugin
                .write(&relative_path, vec![], -1, WriteFlags::APPEND)
                .await
                ?;
        }

        Ok(Json(serde_json::json!({
            "message": "File touched",
            "path": payload.path
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
        let plugin = plugin_opt.ok_or_else(|| RestError::NotFound(format!("Path not found: {}", payload.path)))?;

        let pattern = regex::Regex::new(&payload.pattern)
            .map_err(|e| RestError::Internal(format!("Invalid regex: {}", e)))?;

        let mut matches = Vec::new();
        Self::grep_recursive(Arc::new(state), &plugin, &relative_path, &pattern, &mut matches).await?;

        Ok(Json(GrepResponse {
            pattern: payload.pattern,
            matches,
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
        let (src_plugin_opt, src_relative_path) = state.mount_table.lookup_with_path(&payload.from).await;
        let (dst_plugin_opt, dst_relative_path) = state.mount_table.lookup_with_path(&payload.to).await;

        let src_plugin = src_plugin_opt.ok_or_else(|| RestError::NotFound(format!("Source path not found: {}", payload.from)))?;
        let dst_plugin = dst_plugin_opt.ok_or_else(|| RestError::NotFound(format!("Destination path not found: {}", payload.to)))?;

        // 确保两个路径在同一个插件中（使用指针比较）
        if !Arc::ptr_eq(&src_plugin, &dst_plugin) {
            return Err(RestError::BadRequest("Cannot rename across mount points".to_string()));
        }

        src_plugin
            .rename(&src_relative_path, &dst_relative_path)
            .await
            ?;

        Ok(Json(serde_json::json!({
            "message": "File renamed",
            "from": payload.from,
            "to": payload.to
        })))
    }

    // ============== 挂载管理 API ==============

    /// 列出挂载点（返回 { "mounts": [...] }，与 evif-client 契约一致）
    pub async fn list_mounts(
        State(state): State<AppState>,
    ) -> RestResult<Json<ListMountsResponse>> {
        let mount_paths = state.mount_table.list_mounts().await;

        let mut mounts = Vec::new();
        for path in mount_paths {
            // 使用 lookup_with_path() 保持与其他处理器一致
            let (plugin_opt, _relative_path) = state.mount_table.lookup_with_path(&path).await;
            if let Some(plugin) = plugin_opt {
                mounts.push(MountInfo {
                    plugin: plugin.name().to_string(),
                    path,
                });
            }
        }

        Ok(Json(ListMountsResponse { mounts }))
    }

    /// 挂载插件（Phase 7.3：真正调用 RadixMountTable.mount）
    pub async fn mount(
        State(state): State<AppState>,
        Json(payload): Json<MountRequest>,
    ) -> RestResult<Json<serde_json::Value>> {
        let path = payload.path.trim().to_string();
        if path.is_empty() || !path.starts_with('/') {
            return Err(RestError::BadRequest("path must be non-empty and start with /".to_string()));
        }
        let plugin_name = payload.plugin.to_lowercase();
        let plugin: Arc<dyn EvifPlugin> = match plugin_name.as_str() {
            "mem" | "memfs" => Arc::new(evif_plugins::MemFsPlugin::new()),
            "hello" | "hellofs" => Arc::new(evif_plugins::HelloFsPlugin::new()),
            "local" | "localfs" => {
                let root = payload.config
                    .as_ref()
                    .and_then(|c| c.get("root"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("/tmp/evif-local")
                    .to_string();
                Arc::new(evif_plugins::LocalFsPlugin::new(&root))
            }
            _ => return Err(RestError::BadRequest(format!(
                "Unknown plugin '{}'. Supported: mem, hello, local",
                payload.plugin
            ))),
        };
        plugin.validate(payload.config.as_ref()).await
            .map_err(|e| RestError::BadRequest(format!("Plugin config validation failed: {}", e)))?;
        state.mount_table.mount(path.clone(), plugin).await
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
        state.mount_table.unmount(&path).await
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
        let plugin = Self::plugin_by_name(&name)?;
        Ok(Json(PluginReadmeResponse {
            name: plugin.name().to_string(),
            readme: plugin.get_readme(),
        }))
    }

    /// 获取插件配置参数（Phase 8.2：GET /api/v1/plugins/:name/config）
    pub async fn get_plugin_config(
        Path(name): Path<String>,
    ) -> RestResult<Json<PluginConfigParamsResponse>> {
        let plugin = Self::plugin_by_name(&name)?;
        Ok(Json(PluginConfigParamsResponse {
            name: plugin.name().to_string(),
            params: plugin.get_config_params(),
        }))
    }

    /// 根据插件名创建实例（用于 readme/config 等无需挂载状态的接口）
    fn plugin_by_name(name: &str) -> RestResult<Arc<dyn EvifPlugin>> {
        let name_lower = name.to_lowercase();
        let plugin: Arc<dyn EvifPlugin> = match name_lower.as_str() {
            "mem" | "memfs" => Arc::new(evif_plugins::MemFsPlugin::new()),
            "hello" | "hellofs" => Arc::new(evif_plugins::HelloFsPlugin::new()),
            "local" | "localfs" => Arc::new(evif_plugins::LocalFsPlugin::new("/tmp/evif-local")),
            _ => return Err(RestError::NotFound(format!("Plugin '{}' not found", name))),
        };
        Ok(plugin)
    }

    // ============== 插件管理 API ==============

    /// 列出插件
    pub async fn list_plugins(
        State(state): State<AppState>,
    ) -> RestResult<Json<Vec<PluginInfo>>> {
        let mount_paths = state.mount_table.list_mounts().await;
        let mut plugins = std::collections::HashMap::new();

        for path in mount_paths {
            // 使用 lookup_with_path() 保持与其他处理器一致
            let (plugin_opt, _relative_path) = state.mount_table.lookup_with_path(&path).await;
            if let Some(plugin) = plugin_opt {
                let name = plugin.name().to_string();
                plugins.entry(name.clone()).or_insert_with(|| {
                    PluginInfo {
                        name: name.clone(),
                        version: "1.0.0".to_string(),
                        description: format!("{} plugin", name),
                    }
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
        let plugin_info = state.dynamic_loader.load_plugin(&library_name)
            .map_err(|e| RestError::Internal(format!("Failed to load dynamic library '{}': {}", library_name, e)))?;

        // 创建插件实例
        let plugin = state.dynamic_loader.create_plugin(&library_name)
            .map_err(|e| RestError::Internal(format!("Failed to create plugin instance: {}", e)))?;

        // 验证插件配置
        let config = payload.config.as_ref();
        plugin.validate(config).await
            .map_err(|e| RestError::Internal(format!("Plugin validation failed: {}", e)))?;

        // 挂载插件
        state.mount_table.mount(mount_path.clone(), plugin).await
            .map_err(|e| RestError::Internal(format!("Failed to mount plugin at '{}': {}", mount_path, e)))?;

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
        let plugin_info = state.plugin_registry.get(&name)
            .ok_or_else(|| RestError::NotFound(format!("Plugin '{}' not found in registry", name)))?;

        let library_path = plugin_info.library_path.clone();

        // 2. 卸载现有插件
        let mount_path = format!("/{}", name);
        let _ = state.mount_table.unmount(&mount_path).await;
        let _ = state.dynamic_loader.unload_plugin(&name);

        // 3. 重新加载插件
        let new_info = state.dynamic_loader.load_plugin(&name)
            .map_err(|e| RestError::Internal(format!("Failed to reload plugin '{}': {}", name, e)))?;

        // 4. 创建新实例
        let plugin = state.dynamic_loader.create_plugin(&name)
            .map_err(|e| RestError::Internal(format!("Failed to create plugin instance: {}", e)))?;

        // 5. 重新挂载
        state.mount_table.mount(mount_path.clone(), plugin).await
            .map_err(|e| RestError::Internal(format!("Failed to mount plugin: {}", e)))?;

        // 6. 更新 registry 状态
        state.plugin_registry.activate(&name, mount_path.clone())
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
    ) -> RestResult<Json<serde_json::Value>> {
        // 获取已注册的插件
        let registered = state.plugin_registry.list_all();
        let registered_names: Vec<String> = registered.iter().map(|p| p.name.clone()).collect();

        // 获取已挂载的插件
        let mount_paths = state.mount_table.list_mounts().await;
        let mut mounted = std::collections::HashMap::new();
        for path in mount_paths {
            let (plugin_opt, _relative_path) = state.mount_table.lookup_with_path(&path).await;
            if let Some(plugin) = plugin_opt {
                mounted.insert(plugin.name().to_string(), path);
            }
        }

        let mut plugins = Vec::new();

        // 添加已注册的插件
        for plugin in registered {
            plugins.push(serde_json::json!({
                "name": plugin.name,
                "version": plugin.version,
                "state": plugin.state.to_string(),
                "is_loaded": true,
                "is_mounted": mounted.contains_key(&plugin.name),
            }));
        }

        // 添加内置插件（未注册的）
        let built_in = vec![
            "localfs", "s3fs", "memoryfs", "httpfs", "queuefs",
            "sqlfs", "gitfs", "encryptfs", "tieredfs", "webdavfs",
            "tarfs", "zipfs", "httpcachefs", "redisfs", "mongofs"
        ];

        for name in built_in {
            if !registered_names.contains(&name.to_string()) {
                let is_mounted = mounted.contains_key(name);
                plugins.push(serde_json::json!({
                    "name": name,
                    "version": "1.0.0",
                    "state": if is_mounted { "active" } else { "available" },
                    "is_loaded": false,
                    "is_mounted": is_mounted,
                }));
            }
        }

        Ok(Json(serde_json::json!({
            "plugins": plugins,
            "total": plugins.len(),
        })))
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

    /// Recursive grep helper
    async fn grep_recursive(
        state: Arc<AppState>,
        plugin: &Arc<dyn EvifPlugin>,
        path: &str,
        pattern: &regex::Regex,
        matches: &mut Vec<GrepMatch>,
    ) -> Result<(), RestError> {
        // Check if path is a directory
        let info = plugin
            .stat(path)
            .await
            ?;

        if info.is_dir {
            // List directory and recurse using readdir
            let evif_file_infos = plugin
                .readdir(path)
                .await
                ?;

            for evif_info in evif_file_infos {
                let child_path = format!("{}/{}", path.trim_end_matches('/'), evif_info.name);
                Box::pin(Self::grep_recursive(state.clone(), plugin, &child_path, pattern, matches)).await?;
            }
        } else {
            // Read file and search for pattern
            let data = plugin
                .read(path, 0, 0)
                .await
                ?;

            let content = String::from_utf8_lossy(&data);
            for (line_num, line) in content.lines().enumerate() {
                if pattern.is_match(line) {
                    matches.push(GrepMatch {
                        path: path.to_string(),
                        line: line_num + 1,
                        content: line.to_string(),
                    });
                }
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
}

/// 正则搜索响应
#[derive(Debug, Serialize)]
pub struct GrepResponse {
    pub pattern: String,
    pub matches: Vec<GrepMatch>,
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

/// 创建节点请求
#[derive(Debug, Deserialize)]
pub struct CreateNodeRequest {
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// 查询请求
#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub query: String,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn test_error_mapping_vfs_file_not_found() {
        let vfs_err = evif_vfs::VfsError::FileNotFound("test.txt".to_string());
        let rest_err = RestError::Vfs(vfs_err);
        let response = rest_err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_error_mapping_vfs_invalid_path() {
        let vfs_err = evif_vfs::VfsError::InvalidPath("invalid".to_string());
        let rest_err = RestError::Vfs(vfs_err);
        let response = rest_err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_error_mapping_vfs_permission_denied() {
        let vfs_err = evif_vfs::VfsError::PermissionDenied("access denied".to_string());
        let rest_err = RestError::Vfs(vfs_err);
        let response = rest_err.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_error_mapping_vfs_timeout() {
        let vfs_err = evif_vfs::VfsError::Timeout;
        let rest_err = RestError::Vfs(vfs_err);
        let response = rest_err.into_response();
        assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);
    }

    #[test]
    fn test_create_node_request() {
        let req = CreateNodeRequest {
            name: "test".to_string(),
            parent_id: None,
            metadata: None,
        };
        assert_eq!(req.name, "test");
    }

    #[test]
    fn test_query_request() {
        let req = QueryRequest {
            query: "test".to_string(),
            limit: Some(10),
        };
        assert_eq!(req.query, "test");
        assert_eq!(req.limit, Some(10));
    }

    #[test]
    fn test_node_response() {
        let resp = NodeResponse {
            id: "123".to_string(),
            node_type: "file".to_string(),
            name: "test".to_string(),
            metadata: Metadata::default(),
        };
        assert_eq!(resp.id, "123");
    }
}
