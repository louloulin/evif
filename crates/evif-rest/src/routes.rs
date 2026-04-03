// REST API 路由 - 增强版，完全对标AGFS

use crate::{
    batch_handlers, collab_handlers, context_handlers, encryption_handlers, graphql_handlers,
    handle_handlers, handlers, memory_handlers, metrics_handlers, sync_handlers, tenant_handlers,
    wasm_handlers, ws_handlers, AuthMiddleware, CompatFsHandlers, ContextState, EncryptionState,
    HandleState, RestAuthState, SyncState, TenantState,
};
use axum::{middleware, routing, Router};
use evif_core::{DynamicPluginLoader, GlobalHandleManager, PluginRegistry, RadixMountTable};
use std::sync::Arc;
use std::time::Instant;

/// 创建 API 路由
pub fn create_routes(mount_table: Arc<RadixMountTable>) -> Router {
    create_routes_with_memory_state(mount_table, memory_handlers::create_memory_state())
}

/// 创建带显式记忆状态的 API 路由
pub fn create_routes_with_memory_state(
    mount_table: Arc<RadixMountTable>,
    memory_state: memory_handlers::MemoryState,
) -> Router {
    let tenant_state = TenantState::from_env()
        .expect("failed to initialize tenant state from EVIF_REST_TENANT_STATE_PATH");
    let sync_state = SyncState::from_env()
        .expect("failed to initialize sync state from EVIF_REST_SYNC_STATE_PATH");
    attach_request_identity(build_routes(
        mount_table,
        memory_state,
        tenant_state,
        sync_state,
    ))
}

/// 创建带显式租户状态的 API 路由
pub fn create_routes_with_tenant_state(
    mount_table: Arc<RadixMountTable>,
    tenant_state: TenantState,
) -> Router {
    let sync_state = SyncState::from_env()
        .expect("failed to initialize sync state from EVIF_REST_SYNC_STATE_PATH");
    attach_request_identity(build_routes(
        mount_table,
        memory_handlers::create_memory_state(),
        tenant_state,
        sync_state,
    ))
}

/// 创建带显式同步状态的 API 路由
pub fn create_routes_with_sync_state(
    mount_table: Arc<RadixMountTable>,
    sync_state: SyncState,
) -> Router {
    let tenant_state = TenantState::from_env()
        .expect("failed to initialize tenant state from EVIF_REST_TENANT_STATE_PATH");
    attach_request_identity(build_routes(
        mount_table,
        memory_handlers::create_memory_state(),
        tenant_state,
        sync_state,
    ))
}

/// 创建带认证保护的 API 路由
pub fn create_routes_with_auth(
    mount_table: Arc<RadixMountTable>,
    auth_state: Arc<RestAuthState>,
) -> Router {
    create_routes_with_auth_and_memory_state(
        mount_table,
        auth_state,
        memory_handlers::create_memory_state(),
    )
}

pub(crate) fn create_routes_with_auth_and_memory_state(
    mount_table: Arc<RadixMountTable>,
    auth_state: Arc<RestAuthState>,
    memory_state: memory_handlers::MemoryState,
) -> Router {
    attach_request_identity(
        build_routes(
            mount_table,
            memory_state,
            TenantState::from_env()
                .expect("failed to initialize tenant state from EVIF_REST_TENANT_STATE_PATH"),
            SyncState::from_env()
                .expect("failed to initialize sync state from EVIF_REST_SYNC_STATE_PATH"),
        )
            .layer(middleware::from_fn_with_state(auth_state, AuthMiddleware)),
    )
}

fn attach_request_identity(router: Router) -> Router {
    router.layer(middleware::from_fn(crate::middleware::RequestIdentityMiddleware))
}

fn build_routes(
    mount_table: Arc<RadixMountTable>,
    memory_state: memory_handlers::MemoryState,
    tenant_state: TenantState,
    sync_state: SyncState,
) -> Router {
    // 创建动态插件加载器
    let dynamic_loader = Arc::new(DynamicPluginLoader::new());
    // Phase 14.2: 创建文件锁管理器
    let lock_manager = Arc::new(evif_core::file_lock::FileLockManager::new());
    // Phase 14.1: 创建跨文件系统复制管理器
    let cross_fs_copy_manager = Arc::new(evif_core::cross_fs_copy::CrossFsCopyManager::new(
        mount_table.clone(),
    ));

    let app_state = handlers::AppState {
        mount_table: mount_table.clone(),
        traffic_stats: Arc::new(metrics_handlers::TrafficStats::default()),
        start_time: Instant::now(),
        dynamic_loader,
        plugin_registry: Arc::new(PluginRegistry::new()),
        lock_manager,
        cross_fs_copy_manager,
    };
    let traffic_stats = app_state.traffic_stats.clone();

    // 创建批量操作管理器
    let batch_manager = batch_handlers::BatchOperationManager::new();

    // Phase 17.2: 创建加密状态管理器
    let encryption_state = EncryptionState::new();

    // Phase 17.4: 创建 GraphQL schema
    let graphql_schema = graphql_handlers::GraphQLState::schema();

    // 创建全局Handle管理器并启动清理任务
    let handle_manager = Arc::new(GlobalHandleManager::new());
    // 启动TTL清理任务（每60秒清理一次）
    let _cleanup_handle = handle_manager
        .clone()
        .spawn_cleanup_task(std::time::Duration::from_secs(60));

    // 创建基础路由(带状态)
    let base_routes = Router::new()
        // ============== 健康检查 ==============
        .route(
            "/health",
            axum::routing::get(handlers::EvifHandlers::health),
        )
        // GET /api/v1/health：与 evif-client/CLI 契约一致（status、version、uptime）
        .route(
            "/api/v1/health",
            axum::routing::get(handlers::EvifHandlers::health_v1),
        )
        // Phase 16.2: 分布式部署健康检查
        .route(
            "/api/v1/status",
            axum::routing::get(handlers::EvifHandlers::node_status),
        )
        // POST /api/v1/ping：快速存活检查
        .route(
            "/api/v1/ping",
            axum::routing::post(handlers::EvifHandlers::ping),
        )
        // GET /api/v1/ping：也支持 GET
        .route(
            "/api/v1/ping",
            axum::routing::get(handlers::EvifHandlers::ping),
        )
        // Phase 16.3: 云存储后端
        .route(
            "/api/v1/cloud/status",
            axum::routing::get(handlers::EvifHandlers::cloud_status),
        )
        .route(
            "/api/v1/cloud/providers",
            axum::routing::get(handlers::EvifHandlers::cloud_providers),
        )
        .route(
            "/api/v1/cloud/config",
            axum::routing::post(handlers::EvifHandlers::cloud_config),
        )
        // Phase 16.4: LLM 本地模型集成
        .route(
            "/api/v1/llm/status",
            axum::routing::get(handlers::EvifHandlers::llm_status),
        )
        .route(
            "/api/v1/llm/complete",
            axum::routing::post(handlers::EvifHandlers::llm_complete),
        )
        .route(
            "/api/v1/llm/ping",
            axum::routing::post(handlers::EvifHandlers::llm_ping),
        )
        // ============== 兼容旧前端 API (/api/v1/fs/*) ==============
        .route(
            "/api/v1/fs/list",
            axum::routing::get(CompatFsHandlers::list),
        )
        .route(
            "/api/v1/fs/read",
            axum::routing::get(CompatFsHandlers::read),
        )
        .route(
            "/api/v1/fs/write",
            axum::routing::post(CompatFsHandlers::write),
        )
        .route(
            "/api/v1/fs/create",
            axum::routing::post(CompatFsHandlers::create),
        )
        .route(
            "/api/v1/fs/delete",
            axum::routing::delete(CompatFsHandlers::delete),
        )
        // 流式读写端点（无 JSON/base64 封装，Python SDK 用 httpx.stream 调用）
        .route(
            "/api/v1/fs/stream",
            axum::routing::post(CompatFsHandlers::stream),
        )
        // ============== 文件操作 ==============
        // 读取文件
        .route(
            "/api/v1/files",
            axum::routing::get(handlers::EvifHandlers::read_file),
        )
        // 写入文件
        .route(
            "/api/v1/files",
            axum::routing::put(handlers::EvifHandlers::write_file),
        )
        // 创建空文件
        .route(
            "/api/v1/files",
            axum::routing::post(handlers::EvifHandlers::create_file),
        )
        // 删除文件
        .route(
            "/api/v1/files",
            axum::routing::delete(handlers::EvifHandlers::delete_file),
        )
        // ============== 目录操作 ==============
        // 列出目录
        .route(
            "/api/v1/directories",
            axum::routing::get(handlers::EvifHandlers::list_directory),
        )
        // 创建目录
        .route(
            "/api/v1/directories",
            axum::routing::post(handlers::EvifHandlers::create_directory),
        )
        // 删除目录
        .route(
            "/api/v1/directories",
            axum::routing::delete(handlers::EvifHandlers::delete_directory),
        )
        // ============== 元数据操作 ==============
        // 获取文件状态
        .route(
            "/api/v1/stat",
            axum::routing::get(handlers::EvifHandlers::stat),
        )
        // 计算文件哈希
        .route(
            "/api/v1/digest",
            axum::routing::post(handlers::EvifHandlers::digest),
        )
        // 更新时间戳
        .route(
            "/api/v1/touch",
            axum::routing::post(handlers::EvifHandlers::touch),
        )
        // ============== 高级操作 ==============
        // 正则搜索
        .route(
            "/api/v1/grep",
            axum::routing::post(handlers::EvifHandlers::grep),
        )
        // 重命名/移动
        .route(
            "/api/v1/rename",
            axum::routing::post(handlers::EvifHandlers::rename),
        )
        // ============== Phase 14.2: 文件锁 ==============
        // 获取文件锁
        .route(
            "/api/v1/lock",
            axum::routing::post(handlers::EvifHandlers::acquire_lock),
        )
        // 释放文件锁
        .route(
            "/api/v1/lock",
            axum::routing::delete(handlers::EvifHandlers::release_lock),
        )
        // 列出所有锁
        .route(
            "/api/v1/locks",
            axum::routing::get(handlers::EvifHandlers::list_locks),
        )
        // ============== Phase 14.1: 跨文件系统复制 ==============
        // 复制文件
        .route(
            "/api/v1/copy",
            axum::routing::post(handlers::EvifHandlers::cross_fs_copy),
        )
        // 递归复制目录
        .route(
            "/api/v1/copy/recursive",
            axum::routing::post(handlers::EvifHandlers::cross_fs_copy_recursive),
        )
        // ============== 挂载管理 ==============
        // 列出挂载点
        .route(
            "/api/v1/mounts",
            axum::routing::get(handlers::EvifHandlers::list_mounts),
        )
        // 挂载插件
        .route(
            "/api/v1/mount",
            axum::routing::post(handlers::EvifHandlers::mount),
        )
        // 卸载插件
        .route(
            "/api/v1/unmount",
            axum::routing::post(handlers::EvifHandlers::unmount),
        )
        // ============== 插件管理 ==============
        // 列出插件
        .route(
            "/api/v1/plugins",
            axum::routing::get(handlers::EvifHandlers::list_plugins),
        )
        // 获取可用插件列表
        .route(
            "/api/v1/plugins/available",
            axum::routing::get(handlers::EvifHandlers::list_available_plugins),
        )
        // Phase 8.2: 获取插件 README 与配置参数
        .route(
            "/api/v1/plugins/:name/readme",
            axum::routing::get(handlers::EvifHandlers::get_plugin_readme),
        )
        .route(
            "/api/v1/plugins/:name/config",
            axum::routing::get(handlers::EvifHandlers::get_plugin_config),
        )
        // 插件状态和重载
        .route(
            "/api/v1/plugins/:name/status",
            axum::routing::get(handlers::EvifHandlers::get_plugin_status),
        )
        .route(
            "/api/v1/plugins/:name/reload",
            axum::routing::post(handlers::EvifHandlers::reload_plugin),
        )
        // 加载外部插件
        .route(
            "/api/v1/plugins/load",
            axum::routing::post(handlers::EvifHandlers::load_plugin),
        )
        // 卸载插件
        .route(
            "/api/v1/plugins/unload",
            axum::routing::post(wasm_handlers::WasmPluginHandlers::unload_plugin),
        )
        // 列出插件详细信息
        .route(
            "/api/v1/plugins/list",
            axum::routing::get(wasm_handlers::WasmPluginHandlers::list_plugins),
        )
        // ============== WASM 插件管理 ==============
        // 加载 WASM 插件
        .route(
            "/api/v1/plugins/wasm/load",
            axum::routing::post(wasm_handlers::WasmPluginHandlers::load_wasm_plugin),
        )
        // 热重载 WASM 插件 (Phase 16.1)
        .route(
            "/api/v1/plugins/wasm/reload",
            axum::routing::post(wasm_handlers::WasmPluginHandlers::reload_wasm_plugin),
        )
        // ============== 监控和指标 (新增) ==============
        // Prometheus metrics endpoint (公开，无需认证)
        .route(
            "/metrics",
            axum::routing::get(handlers::EvifHandlers::prometheus_metrics),
        )
        // 流量统计
        .route(
            "/api/v1/metrics/traffic",
            axum::routing::get(handlers::EvifHandlers::get_traffic_stats),
        )
        // 操作统计
        .route(
            "/api/v1/metrics/operations",
            axum::routing::get(handlers::EvifHandlers::get_operation_stats),
        )
        // 系统状态
        .route(
            "/api/v1/metrics/status",
            axum::routing::get(handlers::EvifHandlers::get_system_status),
        )
        // 重置指标
        .route(
            "/api/v1/metrics/reset",
            axum::routing::post(handlers::EvifHandlers::reset_metrics),
        )
        .with_state(app_state)
        .layer(middleware::from_fn_with_state(
            traffic_stats,
            crate::middleware::TrafficMetricsMiddleware,
        ));

    // 创建 WebSocket 状态
    let ws_state = ws_handlers::WebSocketState {
        mount_table: mount_table.clone(),
        api_keys: None, // Set via EVIF_REST_WRITE_API_KEYS if auth is enabled
    };

    let ws_routes = Router::new()
        // ============== WebSocket 连接 ==============
        .route(
            "/ws",
            axum::routing::get(ws_handlers::WebSocketHandlers::websocket_handler),
        )
        .with_state(ws_state);

    // 创建Handle路由(需要HandleState和GlobalHandleManager)
    let handle_state = HandleState {
        mount_table: mount_table.clone(),
        handle_manager: handle_manager.clone(),
    };

    let handle_routes = Router::new()
        // ============== Handle操作 ==============
        // 打开文件句柄
        .route(
            "/api/v1/handles/open",
            axum::routing::post(handle_handlers::HandleHandlers::open_handle),
        )
        // 获取句柄信息
        .route(
            "/api/v1/handles/:id",
            axum::routing::get(handle_handlers::HandleHandlers::get_handle),
        )
        // 读取句柄数据
        .route(
            "/api/v1/handles/:id/read",
            axum::routing::post(handle_handlers::HandleHandlers::read_handle),
        )
        // 写入句柄数据
        .route(
            "/api/v1/handles/:id/write",
            axum::routing::post(handle_handlers::HandleHandlers::write_handle),
        )
        // Seek操作
        .route(
            "/api/v1/handles/:id/seek",
            axum::routing::post(handle_handlers::HandleHandlers::seek_handle),
        )
        // Sync操作
        .route(
            "/api/v1/handles/:id/sync",
            axum::routing::post(handle_handlers::HandleHandlers::sync_handle),
        )
        // 关闭句柄
        .route(
            "/api/v1/handles/:id/close",
            axum::routing::post(handle_handlers::HandleHandlers::close_handle),
        )
        // 续租句柄
        .route(
            "/api/v1/handles/:id/renew",
            axum::routing::post(handle_handlers::HandleHandlers::renew_handle),
        )
        // 列出所有句柄
        .route(
            "/api/v1/handles",
            axum::routing::get(handle_handlers::HandleHandlers::list_handles),
        )
        // 获取统计信息
        .route(
            "/api/v1/handles/stats",
            axum::routing::get(handle_handlers::HandleHandlers::get_stats),
        )
        .with_state(handle_state);

    // 创建批量操作路由
    let batch_routes = batch_handlers::create_batch_routes(batch_manager, mount_table.clone());

    // Phase 9.3: 协作 API（独立 State）
    let collab_routes = Router::new()
        .route(
            "/api/v1/share/create",
            axum::routing::post(collab_handlers::CollabHandlers::create_share),
        )
        .route(
            "/api/v1/share/list",
            axum::routing::get(collab_handlers::CollabHandlers::list_shares),
        )
        .route(
            "/api/v1/share/revoke",
            axum::routing::post(collab_handlers::CollabHandlers::revoke_share),
        )
        .route(
            "/api/v1/permissions/set",
            axum::routing::post(collab_handlers::CollabHandlers::set_permissions),
        )
        .route(
            "/api/v1/permissions/get",
            axum::routing::get(collab_handlers::CollabHandlers::get_permissions),
        )
        .route(
            "/api/v1/comments",
            axum::routing::get(collab_handlers::CollabHandlers::list_comments),
        )
        .route(
            "/api/v1/comments",
            axum::routing::post(collab_handlers::CollabHandlers::add_comment),
        )
        .route(
            "/api/v1/comments/:id",
            axum::routing::put(collab_handlers::CollabHandlers::update_comment),
        )
        .route(
            "/api/v1/comments/:id/resolve",
            axum::routing::put(collab_handlers::CollabHandlers::resolve_comment),
        )
        .route(
            "/api/v1/comments/:id",
            axum::routing::delete(collab_handlers::CollabHandlers::delete_comment),
        )
        .route(
            "/api/v1/activities",
            axum::routing::get(collab_handlers::CollabHandlers::get_activities),
        )
        .route(
            "/api/v1/users",
            axum::routing::get(collab_handlers::CollabHandlers::list_users),
        )
        .with_state(collab_handlers::CollabState::default());

    // Phase 17.1: 租户管理路由
    let tenant_routes = Router::new()
        // 列出所有租户
        .route(
            "/api/v1/tenants",
            axum::routing::get(tenant_handlers::TenantHandlers::list_tenants),
        )
        // 创建租户
        .route(
            "/api/v1/tenants",
            axum::routing::post(tenant_handlers::TenantHandlers::create_tenant),
        )
        // 获取当前租户信息
        .route(
            "/api/v1/tenants/me",
            axum::routing::get(tenant_handlers::TenantHandlers::get_current_tenant),
        )
        // 获取指定租户
        .route(
            "/api/v1/tenants/:id",
            axum::routing::get(tenant_handlers::TenantHandlers::get_tenant),
        )
        // 删除租户
        .route(
            "/api/v1/tenants/:id",
            axum::routing::delete(tenant_handlers::TenantHandlers::delete_tenant),
        )
        .with_state(tenant_state);

    // Phase 17.2: 加密管理路由
    let encryption_routes = Router::new()
        .route(
            "/api/v1/encryption/status",
            axum::routing::get(encryption_handlers::EncryptionHandlers::get_status),
        )
        .route(
            "/api/v1/encryption/enable",
            axum::routing::post(encryption_handlers::EncryptionHandlers::enable),
        )
        .route(
            "/api/v1/encryption/disable",
            axum::routing::post(encryption_handlers::EncryptionHandlers::disable),
        )
        .with_state(encryption_state);

    // Phase 17.3: 增量同步路由
    let sync_routes = Router::new()
        .route(
            "/api/v1/sync/status",
            axum::routing::get(sync_handlers::SyncHandlers::get_status),
        )
        .route(
            "/api/v1/sync/delta",
            axum::routing::post(sync_handlers::SyncHandlers::apply_delta),
        )
        .route(
            "/api/v1/sync/version",
            axum::routing::get(sync_handlers::SyncHandlers::get_version),
        )
        .route(
            "/api/v1/sync/:path/version",
            axum::routing::get(sync_handlers::SyncHandlers::get_path_version),
        )
        .with_state(sync_state);

    // Phase 17.4: GraphQL API 路由
    let graphql_routes = Router::new()
        .route("/api/v1/graphql", routing::post(graphql_handlers::GraphQLHandlers::handler))
        .route("/api/v1/graphql/graphiql", routing::get(graphql_handlers::GraphQLHandlers::graphiql))
        .with_state(graphql_schema);

    let memory_routes = Router::new()
        // ============== 记忆操作 ==============
        // 创建记忆
        .route(
            "/api/v1/memories",
            axum::routing::post(memory_handlers::MemoryHandlers::create_memory),
        )
        // 列出记忆
        .route(
            "/api/v1/memories",
            axum::routing::get(memory_handlers::MemoryHandlers::list_memories),
        )
        // 搜索记忆
        .route(
            "/api/v1/memories/search",
            axum::routing::post(memory_handlers::MemoryHandlers::search_memories),
        )
        // 获取单个记忆
        .route(
            "/api/v1/memories/:id",
            axum::routing::get(memory_handlers::MemoryHandlers::get_memory),
        )
        // ============== 分类操作 ==============
        // 列出分类
        .route(
            "/api/v1/categories",
            axum::routing::get(memory_handlers::MemoryHandlers::list_categories),
        )
        // 获取单个分类
        .route(
            "/api/v1/categories/:id",
            axum::routing::get(memory_handlers::MemoryHandlers::get_category),
        )
        // 获取分类下的记忆
        .route(
            "/api/v1/categories/:id/memories",
            axum::routing::get(memory_handlers::MemoryHandlers::get_category_memories),
        )
        // ============== 记忆查询 ==============
        // 时间线/关系查询
        .route(
            "/api/v1/memories/query",
            axum::routing::post(memory_handlers::MemoryHandlers::query_memories),
        )
        .with_state(memory_state);

    // 合并所有路由
    base_routes
        .merge(collab_routes)
        .merge(handle_routes)
        .merge(batch_routes)
        .merge(ws_routes)
        .merge(tenant_routes)
        .merge(encryption_routes)
        .merge(sync_routes)
        .merge(graphql_routes)
        .merge(memory_routes)
}

/// 创建带 ContextManager 的路由
pub fn create_routes_with_context(
    mount_table: Arc<RadixMountTable>,
    context_manager: evif_plugins::ContextManager,
) -> Router {
    let base = build_routes(
        mount_table,
        memory_handlers::create_memory_state(),
        TenantState::from_env()
            .expect("failed to initialize tenant state from EVIF_REST_TENANT_STATE_PATH"),
        SyncState::from_env()
            .expect("failed to initialize sync state from EVIF_REST_SYNC_STATE_PATH"),
    );
    let context_state = ContextState::new(context_manager);

    attach_request_identity(
        Router::new()
        .route(
            "/context/semantic_search",
            axum::routing::post(context_handlers::semantic_search),
        )
        .route(
            "/context/summarize",
            axum::routing::post(context_handlers::summarize),
        )
        .with_state(context_state)
        .merge(base),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_route_creation() {
        let mount_table = Arc::new(RadixMountTable::new());
        let _app = create_routes(mount_table);
    }
}
