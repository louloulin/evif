// REST API 路由 - 增强版，完全对标AGFS

use crate::{handlers, handle_handlers, wasm_handlers, ws_handlers, HandleState, batch_handlers, CompatFsHandlers, metrics_handlers, collab_handlers, memory_handlers};
use axum::Router;
use std::sync::Arc;
use std::time::Instant;
use evif_core::{RadixMountTable, GlobalHandleManager, DynamicPluginLoader, PluginRegistry};
use evif_graph::Graph;
use handlers::AppState;

/// 创建 API 路由
pub fn create_routes(mount_table: Arc<RadixMountTable>) -> Router {
    // 创建动态插件加载器
    let dynamic_loader = Arc::new(DynamicPluginLoader::new());

    let app_state = handlers::AppState {
        mount_table: mount_table.clone(),
        traffic_stats: Arc::new(metrics_handlers::TrafficStats::default()),
        start_time: Instant::now(),
        dynamic_loader,
        plugin_registry: Arc::new(PluginRegistry::new()),
        graph: Arc::new(Graph::new()),
    };

    // 创建批量操作管理器
    let batch_manager = batch_handlers::BatchOperationManager::new();

    // 创建全局Handle管理器并启动清理任务
    let handle_manager = Arc::new(GlobalHandleManager::new());
    // 启动TTL清理任务（每60秒清理一次）
    let _cleanup_handle = handle_manager
        .clone()
        .spawn_cleanup_task(std::time::Duration::from_secs(60));

    // 创建基础路由(带状态)
    let base_routes = Router::new()
        // ============== 健康检查 ==============
        .route("/health", axum::routing::get(handlers::EvifHandlers::health))
        // GET /api/v1/health：与 evif-client/CLI 契约一致（status、version、uptime）
        .route("/api/v1/health", axum::routing::get(handlers::EvifHandlers::health_v1))

        // ============== 兼容旧前端 API (/api/v1/fs/*) ==============
        .route("/api/v1/fs/list", axum::routing::get(CompatFsHandlers::list))
        .route("/api/v1/fs/read", axum::routing::get(CompatFsHandlers::read))
        .route("/api/v1/fs/write", axum::routing::post(CompatFsHandlers::write))
        .route("/api/v1/fs/create", axum::routing::post(CompatFsHandlers::create))
        .route("/api/v1/fs/delete", axum::routing::delete(CompatFsHandlers::delete))

        // ============== 文件操作 ==============
        // 读取文件
        .route("/api/v1/files", axum::routing::get(handlers::EvifHandlers::read_file))
        // 写入文件
        .route("/api/v1/files", axum::routing::put(handlers::EvifHandlers::write_file))
        // 创建空文件
        .route("/api/v1/files", axum::routing::post(handlers::EvifHandlers::create_file))
        // 删除文件
        .route("/api/v1/files", axum::routing::delete(handlers::EvifHandlers::delete_file))

        // ============== 目录操作 ==============
        // 列出目录
        .route("/api/v1/directories", axum::routing::get(handlers::EvifHandlers::list_directory))
        // 创建目录
        .route("/api/v1/directories", axum::routing::post(handlers::EvifHandlers::create_directory))
        // 删除目录
        .route("/api/v1/directories", axum::routing::delete(handlers::EvifHandlers::delete_directory))

        // ============== 元数据操作 ==============
        // 获取文件状态
        .route("/api/v1/stat", axum::routing::get(handlers::EvifHandlers::stat))
        // 计算文件哈希
        .route("/api/v1/digest", axum::routing::post(handlers::EvifHandlers::digest))
        // 更新时间戳
        .route("/api/v1/touch", axum::routing::post(handlers::EvifHandlers::touch))

        // ============== 高级操作 ==============
        // 正则搜索
        .route("/api/v1/grep", axum::routing::post(handlers::EvifHandlers::grep))
        // 重命名/移动
        .route("/api/v1/rename", axum::routing::post(handlers::EvifHandlers::rename))

        // ============== 挂载管理 ==============
        // 列出挂载点
        .route("/api/v1/mounts", axum::routing::get(handlers::EvifHandlers::list_mounts))
        // 挂载插件
        .route("/api/v1/mount", axum::routing::post(handlers::EvifHandlers::mount))
        // 卸载插件
        .route("/api/v1/unmount", axum::routing::post(handlers::EvifHandlers::unmount))

        // ============== 插件管理 ==============
        // 列出插件
        .route("/api/v1/plugins", axum::routing::get(handlers::EvifHandlers::list_plugins))
        // 获取可用插件列表
        .route("/api/v1/plugins/available", axum::routing::get(handlers::EvifHandlers::list_available_plugins))
        // Phase 8.2: 获取插件 README 与配置参数
        .route("/api/v1/plugins/:name/readme", axum::routing::get(handlers::EvifHandlers::get_plugin_readme))
        .route("/api/v1/plugins/:name/config", axum::routing::get(handlers::EvifHandlers::get_plugin_config))
        // 插件状态和重载
        .route("/api/v1/plugins/:name/status", axum::routing::get(handlers::EvifHandlers::get_plugin_status))
        .route("/api/v1/plugins/:name/reload", axum::routing::post(handlers::EvifHandlers::reload_plugin))
        // 加载外部插件
        .route("/api/v1/plugins/load", axum::routing::post(handlers::EvifHandlers::load_plugin))
        // 卸载插件
        .route("/api/v1/plugins/unload", axum::routing::post(wasm_handlers::WasmPluginHandlers::unload_plugin))
        // 列出插件详细信息
        .route("/api/v1/plugins/list", axum::routing::get(wasm_handlers::WasmPluginHandlers::list_plugins))

        // ============== WASM 插件管理 (新增) ==============
        // 加载 WASM 插件
        .route("/api/v1/plugins/wasm/load", axum::routing::post(wasm_handlers::WasmPluginHandlers::load_wasm_plugin))

        // ============== 监控和指标 (新增) ==============
        // 流量统计
        .route("/api/v1/metrics/traffic", axum::routing::get(handlers::EvifHandlers::get_traffic_stats))
        // 操作统计
        .route("/api/v1/metrics/operations", axum::routing::get(handlers::EvifHandlers::get_operation_stats))
        // 系统状态
        .route("/api/v1/metrics/status", axum::routing::get(handlers::EvifHandlers::get_system_status))
        // 重置指标
        .route("/api/v1/metrics/reset", axum::routing::post(handlers::EvifHandlers::reset_metrics))

        // ============== 图操作 (兼容原有API) ==============
        // 节点操作
        .route("/nodes/:id", axum::routing::get(handlers::EvifHandlers::get_node))
        .route("/nodes/:id", axum::routing::delete(handlers::EvifHandlers::delete_node))
        // 创建节点
        .route("/nodes/create/:node_type", axum::routing::post(handlers::EvifHandlers::create_node))
        // 查询
        .route("/query", axum::routing::post(handlers::EvifHandlers::query))
        // 子节点
        .route("/nodes/:id/children", axum::routing::get(handlers::EvifHandlers::get_children))
        // 统计
        .route("/stats", axum::routing::get(handlers::EvifHandlers::stats))
        .with_state(app_state);

    // 创建 WebSocket 状态
    let ws_state = ws_handlers::WebSocketState {
        mount_table: mount_table.clone(),
    };

    let ws_routes = Router::new()
        // ============== WebSocket 连接 ==============
        .route("/ws", axum::routing::get(ws_handlers::WebSocketHandlers::websocket_handler))
        .with_state(ws_state);

    // 创建Handle路由(需要HandleState和GlobalHandleManager)
    let handle_state = HandleState {
        mount_table: mount_table.clone(),
        handle_manager: handle_manager.clone(),
    };

    let handle_routes = Router::new()
        // ============== Handle操作 ==============
        // 打开文件句柄
        .route("/api/v1/handles/open", axum::routing::post(handle_handlers::HandleHandlers::open_handle))
        // 获取句柄信息
        .route("/api/v1/handles/:id", axum::routing::get(handle_handlers::HandleHandlers::get_handle))
        // 读取句柄数据
        .route("/api/v1/handles/:id/read", axum::routing::post(handle_handlers::HandleHandlers::read_handle))
        // 写入句柄数据
        .route("/api/v1/handles/:id/write", axum::routing::post(handle_handlers::HandleHandlers::write_handle))
        // Seek操作
        .route("/api/v1/handles/:id/seek", axum::routing::post(handle_handlers::HandleHandlers::seek_handle))
        // Sync操作
        .route("/api/v1/handles/:id/sync", axum::routing::post(handle_handlers::HandleHandlers::sync_handle))
        // 关闭句柄
        .route("/api/v1/handles/:id/close", axum::routing::post(handle_handlers::HandleHandlers::close_handle))
        // 续租句柄
        .route("/api/v1/handles/:id/renew", axum::routing::post(handle_handlers::HandleHandlers::renew_handle))
        // 列出所有句柄
        .route("/api/v1/handles", axum::routing::get(handle_handlers::HandleHandlers::list_handles))
        // 获取统计信息
        .route("/api/v1/handles/stats", axum::routing::get(handle_handlers::HandleHandlers::get_stats))
        .with_state(handle_state);

    // 创建批量操作路由
    let batch_routes = batch_handlers::create_batch_routes(batch_manager, mount_table.clone());

    // Phase 9.3: 协作 API（独立 State）
    let collab_routes = Router::new()
        .route("/api/v1/share/create", axum::routing::post(collab_handlers::CollabHandlers::create_share))
        .route("/api/v1/share/list", axum::routing::get(collab_handlers::CollabHandlers::list_shares))
        .route("/api/v1/share/revoke", axum::routing::post(collab_handlers::CollabHandlers::revoke_share))
        .route("/api/v1/permissions/set", axum::routing::post(collab_handlers::CollabHandlers::set_permissions))
        .route("/api/v1/permissions/get", axum::routing::get(collab_handlers::CollabHandlers::get_permissions))
        .route("/api/v1/comments", axum::routing::get(collab_handlers::CollabHandlers::list_comments))
        .route("/api/v1/comments", axum::routing::post(collab_handlers::CollabHandlers::add_comment))
        .route("/api/v1/comments/:id", axum::routing::put(collab_handlers::CollabHandlers::update_comment))
        .route("/api/v1/comments/:id/resolve", axum::routing::put(collab_handlers::CollabHandlers::resolve_comment))
        .route("/api/v1/comments/:id", axum::routing::delete(collab_handlers::CollabHandlers::delete_comment))
        .route("/api/v1/activities", axum::routing::get(collab_handlers::CollabHandlers::get_activities))
        .route("/api/v1/users", axum::routing::get(collab_handlers::CollabHandlers::list_users))
        .with_state(collab_handlers::CollabState::default());

    // 创建 Memory 路由
    let memory_state = memory_handlers::create_memory_state();

    let memory_routes = Router::new()
        // ============== 记忆操作 ==============
        // 创建记忆
        .route("/api/v1/memories", axum::routing::post(memory_handlers::MemoryHandlers::create_memory))
        // 列出记忆
        .route("/api/v1/memories", axum::routing::get(memory_handlers::MemoryHandlers::list_memories))
        // 搜索记忆
        .route("/api/v1/memories/search", axum::routing::post(memory_handlers::MemoryHandlers::search_memories))
        // 获取单个记忆
        .route("/api/v1/memories/:id", axum::routing::get(memory_handlers::MemoryHandlers::get_memory))
        // ============== 分类操作 ==============
        // 列出分类
        .route("/api/v1/categories", axum::routing::get(memory_handlers::MemoryHandlers::list_categories))
        // 获取单个分类
        .route("/api/v1/categories/:id", axum::routing::get(memory_handlers::MemoryHandlers::get_category))
        // 获取分类下的记忆
        .route("/api/v1/categories/:id/memories", axum::routing::get(memory_handlers::MemoryHandlers::get_category_memories))
        // ============== 图谱操作 ==============
        // 图谱查询
        .route("/api/v1/graph/query", axum::routing::post(memory_handlers::MemoryHandlers::query_graph))
        .with_state(memory_state);

    // 合并所有路由
    base_routes.merge(collab_routes).merge(handle_routes).merge(batch_routes).merge(ws_routes).merge(memory_routes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_route_creation() {
        let mount_table = Arc::new(RadixMountTable::new());
        let _app = create_routes(mount_table);
        // Router creation successful
        assert!(true);
    }
}
