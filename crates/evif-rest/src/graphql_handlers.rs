// Phase 17.4: GraphQL API - 真实业务查询面
//
// 提供 GraphQL API 端点，查询真实系统状态

use async_graphql::{Context, EmptySubscription, InputObject, Object, Schema, SimpleObject};
use axum::{extract::State, response::IntoResponse, Json};
use base64::Engine;
use std::sync::Arc;

use crate::encryption_handlers::{EncryptionConfig, EncryptionState, KeyVersion};
use crate::metrics_handlers::TrafficStats;
use crate::sync_handlers::SyncState;
use crate::tenant_handlers::TenantState;
use evif_core::{RadixMountTable, WriteFlags};

/// GraphQL 应用上下文 - 通过 schema.data() 注入
#[derive(Clone)]
pub struct GraphqlAppContext {
    pub mount_table: Arc<RadixMountTable>,
    pub traffic_stats: Arc<TrafficStats>,
    pub tenant_state: TenantState,
    pub encryption_state: EncryptionState,
    pub sync_state: SyncState,
}

/// 挂载点信息
#[derive(SimpleObject)]
pub struct MountInfo {
    pub path: String,
    pub plugin: String,
    pub instance_name: String,
}

/// 流量统计
#[derive(SimpleObject)]
pub struct GraphqlTrafficStats {
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

impl From<crate::metrics_handlers::TrafficStatsResponse> for GraphqlTrafficStats {
    fn from(r: crate::metrics_handlers::TrafficStatsResponse) -> Self {
        Self {
            total_requests: r.total_requests,
            total_bytes_read: r.total_bytes_read,
            total_bytes_written: r.total_bytes_written,
            total_errors: r.total_errors,
            read_count: r.read_count,
            write_count: r.write_count,
            list_count: r.list_count,
            other_count: r.other_count,
            average_read_size: r.average_read_size,
            average_write_size: r.average_write_size,
        }
    }
}

/// 服务器状态
#[derive(SimpleObject)]
pub struct ServerStatus {
    pub version: String,
    pub status: String,
}

/// 加密状态
#[derive(SimpleObject)]
pub struct EncryptionStatusGql {
    pub status: String,
    pub algorithm: String,
    pub key_source: String,
    /// 密钥版本历史（最新版本在最后）
    pub versions: Vec<KeyVersionGql>,
}

/// 密钥版本信息（GraphQL）
#[derive(SimpleObject, Clone)]
pub struct KeyVersionGql {
    pub id: String,
    pub version: u32,
    pub source_hint: String,
    pub created_at: String,
    pub is_current: bool,
}

impl From<KeyVersion> for KeyVersionGql {
    fn from(kv: KeyVersion) -> Self {
        Self {
            id: kv.id,
            version: kv.version,
            source_hint: kv.source_hint,
            created_at: kv.created_at,
            is_current: kv.is_current,
        }
    }
}

impl From<EncryptionConfig> for EncryptionStatusGql {
    fn from(c: EncryptionConfig) -> Self {
        Self {
            status: serde_json::to_string(&c.status).unwrap_or_default(),
            algorithm: c.algorithm,
            key_source: c.key_source,
            versions: vec![],
        }
    }
}

/// 同步状态
#[derive(SimpleObject)]
pub struct SyncStatusGql {
    pub synced: bool,
    pub last_version: u64,
    pub pending_changes: usize,
    pub tracked_paths: Vec<String>,
}

/// Delta 变更输入
#[derive(InputObject)]
pub struct DeltaChangeInput {
    pub path: String,
    /// "created" | "modified" | "deleted"
    pub op: String,
    pub version: u64,
}

/// Delta 响应（GraphQL）
#[derive(SimpleObject)]
pub struct DeltaResponseGql {
    pub synced_version: u64,
    pub accepted: usize,
    pub conflicts: Vec<String>,
}

/// 加密操作结果
#[derive(SimpleObject)]
pub struct EncryptionOperationResult {
    pub success: bool,
    pub message: String,
    pub status: Option<EncryptionStatusGql>,
}

/// 冲突解决策略
#[derive(InputObject)]
pub struct ConflictResolution {
    /// 要解决的路径
    pub path: String,
    /// 使用的策略: "accept_local" | "accept_remote" | "last_write_wins"
    pub strategy: String,
    /// 当策略为 accept_remote 时的版本号
    pub remote_version: Option<u64>,
}

// ── GraphQL 文件操作类型 ────────────────────────────────────────────────────

/// 文件读取输入
#[derive(InputObject)]
pub struct FileReadInput {
    pub path: String,
    pub offset: Option<u64>,
    pub size: Option<u64>,
}

/// 文件写入输入
#[derive(InputObject)]
pub struct FileWriteInput {
    pub path: String,
    pub data: String,
    /// "text" (default) | "base64"
    pub encoding: Option<String>,
}

/// 文件读取结果
#[derive(SimpleObject)]
pub struct FileReadResult {
    pub content: String,
    pub data: String, // base64
    pub size: u64,
}

/// 文件写入结果
#[derive(SimpleObject)]
pub struct FileWriteResult {
    pub bytes_written: u64,
    pub path: String,
}

/// 目录列表条目
#[derive(SimpleObject)]
pub struct FileListEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: String,
}

/// 目录列表结果
#[derive(SimpleObject)]
pub struct FileListResult {
    pub path: String,
    pub entries: Vec<FileListEntry>,
}

/// 文件删除输入
#[derive(InputObject)]
pub struct FileDeleteInput {
    pub path: String,
}

/// 文件删除结果
#[derive(SimpleObject)]
pub struct FileDeleteResult {
    pub path: String,
    pub deleted: bool,
}

/// 文件创建输入
#[derive(InputObject)]
pub struct FileCreateInput {
    pub path: String,
    /// 权限（八进制字符串，如 "0644"），默认为 "0644"
    pub perm: Option<String>,
}

/// 文件创建结果
#[derive(SimpleObject)]
pub struct FileCreateResult {
    pub path: String,
    pub created: bool,
}

/// 目录删除输入
#[derive(InputObject)]
pub struct DirectoryDeleteInput {
    pub path: String,
}

/// 目录删除结果
#[derive(SimpleObject)]
pub struct DirectoryDeleteResult {
    pub path: String,
    pub deleted: bool,
}

/// Query root
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// 获取服务器状态
    async fn status(&self) -> ServerStatus {
        ServerStatus {
            version: env!("CARGO_PKG_VERSION").to_string(),
            status: "healthy".to_string(),
        }
    }

    /// 获取健康检查
    async fn health(&self, ctx: &Context<'_>) -> bool {
        // Verify the system is healthy by checking mount table exists
        ctx.data::<Arc<RadixMountTable>>().is_ok()
    }

    /// 获取挂载点列表
    async fn mounts(&self, ctx: &Context<'_>) -> Vec<MountInfo> {
        let mount_table = match ctx.data::<Arc<RadixMountTable>>() {
            Ok(t) => t,
            Err(_) => return vec![],
        };
        mount_table
            .list_mounts_info()
            .await
            .into_iter()
            .map(|(path, metadata)| MountInfo {
                path,
                plugin: metadata.plugin_name,
                instance_name: metadata.instance_name,
            })
            .collect()
    }

    /// 获取流量统计
    async fn traffic(&self, ctx: &Context<'_>) -> GraphqlTrafficStats {
        let traffic_stats = match ctx.data::<Arc<TrafficStats>>() {
            Ok(t) => t,
            Err(_) => {
                return GraphqlTrafficStats {
                    total_requests: 0,
                    total_bytes_read: 0,
                    total_bytes_written: 0,
                    total_errors: 0,
                    read_count: 0,
                    write_count: 0,
                    list_count: 0,
                    other_count: 0,
                    average_read_size: 0,
                    average_write_size: 0,
                };
            }
        };
        let r = std::sync::atomic::Ordering::Relaxed;
        let total_requests = traffic_stats.total_requests.load(r);
        let total_bytes_read = traffic_stats.total_bytes_read.load(r);
        let total_bytes_written = traffic_stats.total_bytes_written.load(r);
        let total_errors = traffic_stats.total_errors.load(r);
        let read_count = traffic_stats.read_count.load(r);
        let write_count = traffic_stats.write_count.load(r);
        let list_count = traffic_stats.list_count.load(r);
        let other_count = traffic_stats.other_count.load(r);
        let avg_read = total_bytes_read.saturating_div(if read_count > 0 { read_count } else { 1 });
        let avg_write =
            total_bytes_written.saturating_div(if write_count > 0 { write_count } else { 1 });

        GraphqlTrafficStats {
            total_requests,
            total_bytes_read,
            total_bytes_written,
            total_errors,
            read_count,
            write_count,
            list_count,
            other_count,
            average_read_size: avg_read,
            average_write_size: avg_write,
        }
    }

    /// 获取租户列表
    async fn tenants(&self, ctx: &Context<'_>) -> Vec<crate::tenant_handlers::TenantInfo> {
        let tenant_state = match ctx.data::<TenantState>() {
            Ok(t) => t,
            Err(_) => return vec![],
        };
        tenant_state.list_tenants().await
    }

    /// 获取加密状态
    async fn encryption(&self, ctx: &Context<'_>) -> Option<EncryptionStatusGql> {
        let encryption_state = ctx.data::<EncryptionState>().ok()?;
        let config = encryption_state.get_config();
        let versions = encryption_state.get_key_versions();
        Some(EncryptionStatusGql {
            status: serde_json::to_string(&config.status).unwrap_or_default(),
            algorithm: config.algorithm,
            key_source: config.key_source,
            versions: versions.into_iter().map(KeyVersionGql::from).collect(),
        })
    }

    /// 获取同步状态
    async fn sync_status(&self, ctx: &Context<'_>) -> SyncStatusGql {
        let sync_state = match ctx.data::<SyncState>() {
            Ok(s) => s,
            Err(_) => {
                return SyncStatusGql {
                    synced: false,
                    last_version: 0,
                    pending_changes: 0,
                    tracked_paths: vec![],
                };
            }
        };
        let status = sync_state.get_status();
        SyncStatusGql {
            synced: status.synced,
            last_version: status.last_version,
            pending_changes: status.pending_changes,
            tracked_paths: status.tracked_paths,
        }
    }
}

/// Mutation root
pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Echo 测试
    async fn echo(&self, message: String) -> String {
        message
    }

    /// 通过 GraphQL 读取文件内容
    async fn file_read(&self, ctx: &Context<'_>, input: FileReadInput) -> Option<FileReadResult> {
        let mount_table = ctx.data::<Arc<RadixMountTable>>().ok()?;
        let (plugin_opt, relative_path) = mount_table.lookup_with_path(&input.path).await;
        let plugin = plugin_opt?;

        let offset = input.offset.unwrap_or(0);
        let size = input.size.unwrap_or(0);
        let data = plugin.read(&relative_path, offset, size).await.ok()?;

        let size_u64 = data.len() as u64;
        let content = String::from_utf8_lossy(&data).to_string();
        let data_b64 = base64::engine::general_purpose::STANDARD.encode(&data);

        Some(FileReadResult {
            content,
            data: data_b64,
            size: size_u64,
        })
    }

    /// 通过 GraphQL 写入文件内容（不存在时自动创建）
    async fn file_write(
        &self,
        ctx: &Context<'_>,
        input: FileWriteInput,
    ) -> Option<FileWriteResult> {
        let mount_table = ctx.data::<Arc<RadixMountTable>>().ok()?;
        let (plugin_opt, relative_path) = mount_table.lookup_with_path(&input.path).await;
        let plugin = plugin_opt?;

        let data = if input.encoding.as_deref() == Some("base64") {
            base64::engine::general_purpose::STANDARD
                .decode(input.data.trim())
                .ok()?
        } else {
            input.data.into_bytes()
        };

        // Auto-create if file doesn't exist (match real filesystem behavior)
        if plugin.stat(&relative_path).await.is_err() {
            plugin.create(&relative_path, 0o644).await.ok()?;
        }

        let bytes_written = plugin
            .write(&relative_path, data, 0, WriteFlags::NONE)
            .await
            .ok()?;

        Some(FileWriteResult {
            bytes_written,
            path: input.path,
        })
    }

    /// 通过 GraphQL 列出目录内容
    async fn file_list(&self, ctx: &Context<'_>, path: String) -> Option<FileListResult> {
        let mount_table = ctx.data::<Arc<RadixMountTable>>().ok()?;
        let (plugin_opt, relative_path) = mount_table.lookup_with_path(&path).await;

        // Root path "/" lists all mounts
        if relative_path == "/" && plugin_opt.is_none() {
            let mounts = mount_table.list_mounts().await;
            let entries: Vec<FileListEntry> = mounts
                .into_iter()
                .map(|name| {
                    let display_name = name.trim_start_matches('/').to_string();
                    FileListEntry {
                        name: display_name,
                        path: name,
                        is_dir: true,
                        size: 0,
                        modified: chrono::Utc::now().to_rfc3339(),
                    }
                })
                .collect();
            return Some(FileListResult {
                path: "/".to_string(),
                entries,
            });
        }

        let plugin = plugin_opt?;
        let evif_entries = plugin.readdir(&relative_path).await.ok()?;

        let base_path = path.trim_end_matches('/');
        let entries: Vec<FileListEntry> = evif_entries
            .into_iter()
            .map(|info| FileListEntry {
                name: info.name.clone(),
                path: format!("{}/{}", base_path, info.name),
                is_dir: info.is_dir,
                size: info.size,
                modified: info.modified.to_rfc3339(),
            })
            .collect();

        Some(FileListResult { path, entries })
    }

    /// 解决同步冲突
    async fn resolve_sync_conflicts(
        &self,
        ctx: &Context<'_>,
        resolutions: Vec<ConflictResolution>,
    ) -> SyncStatusGql {
        let sync_state = match ctx.data::<SyncState>() {
            Ok(s) => s,
            Err(_) => {
                return SyncStatusGql {
                    synced: false,
                    last_version: 0,
                    pending_changes: 0,
                    tracked_paths: vec![],
                };
            }
        };
        for resolution in &resolutions {
            sync_state.resolve_conflicts(
                &resolution.path,
                &resolution.strategy,
                resolution.remote_version,
            );
        }
        let status = sync_state.get_status();
        SyncStatusGql {
            synced: true,
            last_version: status.last_version,
            pending_changes: 0,
            tracked_paths: status.tracked_paths,
        }
    }

    /// 通过 GraphQL 删除文件
    async fn file_delete(
        &self,
        ctx: &Context<'_>,
        input: FileDeleteInput,
    ) -> Option<FileDeleteResult> {
        let mount_table = ctx.data::<Arc<RadixMountTable>>().ok()?;
        let (plugin_opt, relative_path) = mount_table.lookup_with_path(&input.path).await;
        let plugin = plugin_opt?;

        plugin.remove(&relative_path).await.ok()?;

        Some(FileDeleteResult {
            path: input.path,
            deleted: true,
        })
    }

    /// 通过 GraphQL 创建空文件
    async fn file_create(
        &self,
        ctx: &Context<'_>,
        input: FileCreateInput,
    ) -> Option<FileCreateResult> {
        let mount_table = ctx.data::<Arc<RadixMountTable>>().ok()?;
        let (plugin_opt, relative_path) = mount_table.lookup_with_path(&input.path).await;
        let plugin = plugin_opt?;

        // Parse permission: default 0o644
        let perm = input
            .perm
            .as_ref()
            .and_then(|s| u32::from_str_radix(s.trim_start_matches("0"), 8).ok())
            .unwrap_or(0o644);

        plugin.create(&relative_path, perm).await.ok()?;

        Some(FileCreateResult {
            path: input.path,
            created: true,
        })
    }

    /// 通过 GraphQL 删除目录（调用插件 remove）
    async fn directory_delete(
        &self,
        ctx: &Context<'_>,
        input: DirectoryDeleteInput,
    ) -> Option<DirectoryDeleteResult> {
        let mount_table = ctx.data::<Arc<RadixMountTable>>().ok()?;
        let (plugin_opt, relative_path) = mount_table.lookup_with_path(&input.path).await;
        let plugin = plugin_opt?;

        plugin.remove(&relative_path).await.ok()?;

        Some(DirectoryDeleteResult {
            path: input.path,
            deleted: true,
        })
    }

    /// 启用加密
    async fn enable_encryption(
        &self,
        ctx: &Context<'_>,
        key: String,
    ) -> Option<EncryptionOperationResult> {
        let encryption_state = ctx.data::<EncryptionState>().ok()?;

        match encryption_state.enable(key).await {
            Ok(config) => {
                let versions = encryption_state.get_key_versions();
                Some(EncryptionOperationResult {
                    success: true,
                    message: "Encryption enabled".to_string(),
                    status: Some(EncryptionStatusGql {
                        status: serde_json::to_string(&config.status).unwrap_or_default(),
                        algorithm: config.algorithm,
                        key_source: config.key_source,
                        versions: versions.into_iter().map(KeyVersionGql::from).collect(),
                    }),
                })
            }
            Err(e) => Some(EncryptionOperationResult {
                success: false,
                message: e,
                status: None,
            }),
        }
    }

    /// 禁用加密
    async fn disable_encryption(&self, ctx: &Context<'_>) -> Option<EncryptionOperationResult> {
        let encryption_state = ctx.data::<EncryptionState>().ok()?;

        match encryption_state.disable().await {
            Ok(config) => {
                let versions = encryption_state.get_key_versions();
                Some(EncryptionOperationResult {
                    success: true,
                    message: "Encryption disabled".to_string(),
                    status: Some(EncryptionStatusGql {
                        status: serde_json::to_string(&config.status).unwrap_or_default(),
                        algorithm: config.algorithm,
                        key_source: config.key_source,
                        versions: versions.into_iter().map(KeyVersionGql::from).collect(),
                    }),
                })
            }
            Err(e) => Some(EncryptionOperationResult {
                success: false,
                message: e,
                status: None,
            }),
        }
    }

    /// 轮换加密密钥
    async fn rotate_encryption_key(
        &self,
        ctx: &Context<'_>,
        new_key: String,
    ) -> Option<EncryptionOperationResult> {
        let encryption_state = ctx.data::<EncryptionState>().ok()?;

        match encryption_state.rotate_key(new_key).await {
            Ok(config) => {
                let versions = encryption_state.get_key_versions();
                Some(EncryptionOperationResult {
                    success: true,
                    message: "Encryption key rotated".to_string(),
                    status: Some(EncryptionStatusGql {
                        status: serde_json::to_string(&config.status).unwrap_or_default(),
                        algorithm: config.algorithm,
                        key_source: config.key_source,
                        versions: versions.into_iter().map(KeyVersionGql::from).collect(),
                    }),
                })
            }
            Err(e) => Some(EncryptionOperationResult {
                success: false,
                message: e,
                status: None,
            }),
        }
    }

    /// 应用增量变更
    async fn apply_delta(
        &self,
        ctx: &Context<'_>,
        base_version: u64,
        changes: Vec<DeltaChangeInput>,
    ) -> Option<DeltaResponseGql> {
        let sync_state = ctx.data::<SyncState>().ok()?;

        let delta_changes = changes
            .into_iter()
            .map(|c| crate::sync_handlers::DeltaChange {
                path: c.path,
                op: c.op,
                version: c.version,
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
            .collect();

        let response = sync_state
            .apply_delta(crate::sync_handlers::DeltaRequest {
                base_version,
                changes: delta_changes,
            })
            .await;

        Some(DeltaResponseGql {
            synced_version: response.synced_version,
            accepted: response.accepted,
            conflicts: response.conflicts,
        })
    }
}

/// GraphQL schema
pub type EvifSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

/// GraphQL 应用上下文持有者（供路由注册用）
#[derive(Clone)]
pub struct GraphQLState;

impl GraphQLState {
    pub fn new() -> Self {
        Self
    }

    /// 使用给定上下文构建 schema
    pub fn schema_with_context(context: GraphqlAppContext) -> EvifSchema {
        Schema::build(QueryRoot, MutationRoot, EmptySubscription)
            .data(context.mount_table.clone())
            .data(context.traffic_stats.clone())
            .data(context.tenant_state.clone())
            .data(context.encryption_state.clone())
            .data(context.sync_state.clone())
            .finish()
    }

    /// 旧接口：创建无状态 schema（仅保留用于向后兼容）
    pub fn schema() -> EvifSchema {
        Schema::build(QueryRoot, MutationRoot, EmptySubscription).finish()
    }
}

impl Default for GraphQLState {
    fn default() -> Self {
        Self::new()
    }
}

/// GraphQL handlers
pub struct GraphQLHandlers;

impl GraphQLHandlers {
    /// GraphQL endpoint - POST /api/v1/graphql
    pub async fn handler(
        State(schema): State<EvifSchema>,
        req: Json<async_graphql::Request>,
    ) -> impl IntoResponse {
        Json(schema.execute(req.0).await)
    }

    /// GraphQL IDE (GraphiQL) - GET /api/v1/graphql/graphiql
    pub async fn graphiql() -> impl IntoResponse {
        axum::response::Html(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>EVIF GraphQL</title>
    <link rel="stylesheet" href="https://unpkg.com/graphiql/graphiql.min.css" />
</head>
<body style="margin: 0;">
    <div id="graphiql" style="height: 100vh;"></div>
    <script crossorigin src="https://unpkg.com/react/umd/react.production.min.js"></script>
    <script crossorigin src="https://unpkg.com/react-dom/umd/react-dom.production.min.js"></script>
    <script crossorigin src="https://unpkg.com/graphiql/graphiql.min.js"></script>
    <script>
        const fetcher = GraphiQL.createFetcher({ url: '/api/v1/graphql' });
        ReactDOM.render(React.createElement(GraphiQL, { fetcher }), document.getElementById('graphiql'));
    </script>
</body>
</html>"#,
        )
    }
}
