// EVIF REST API - HTTP/JSON 接口

mod batch_handlers;
mod collab_handlers;
mod compat_fs;
mod context_handlers;
mod encryption_handlers;
mod fs_handlers;
mod graphql_handlers;
mod handle_handlers;
mod handlers;
mod memory_handlers;
mod metrics_handlers;
mod middleware;
mod plugin_handlers;
mod routes;
mod server;
mod sync_handlers;
mod tenant_handlers;
mod wasm_handlers;
mod ws_handlers;

pub use batch_handlers::{
    create_batch_routes, BatchCopyRequestJson, BatchDeleteRequestJson, BatchOperationInfo,
    BatchOperationManager, OperationStatus,
};
pub use compat_fs::CompatFsHandlers;
pub use context_handlers::{
    ContextState, SemanticResultJson, SemanticSearchRequest, SemanticSearchResponse,
    SummarizeRequest, SummarizeResponse,
};
pub use encryption_handlers::{
    EnableEncryptionRequest, EncryptionConfig, EncryptionHandlers, EncryptionState,
    EncryptionStatus, KeyVersion, RotateKeyRequest,
};
pub use fs_handlers::{FsHandlers, FsState};
pub use graphql_handlers::{
    ConflictResolution, DeltaChangeInput, DeltaResponseGql, DirectoryDeleteInput,
    DirectoryDeleteResult, EncryptionOperationResult, EncryptionStatusGql, EvifSchema,
    FileCreateInput, FileCreateResult, FileDeleteInput, FileDeleteResult, FileListEntry,
    FileListResult, FileReadInput, FileReadResult, FileWriteInput, FileWriteResult,
    GraphQLHandlers, GraphQLState, GraphqlAppContext, GraphqlTrafficStats, KeyVersionGql,
    MountInfo, MutationRoot, QueryRoot, ServerStatus, SyncStatusGql,
};
pub use handle_handlers::{HandleHandlers, HandleState};
pub use handlers::{AppState, EvifHandlers};
pub use memory_handlers::{
    create_memory_state, create_memory_state_from_config, create_memory_state_from_env,
    init_memory_pipelines, is_production_mode, validate_memory_for_production, MemoryBackendConfig,
    MemoryBackendKind, MemoryHandlers, MemoryState,
};
pub use metrics_handlers::{MetricsHandlers, MetricsState, TrafficStats};
pub use middleware::{AuthMiddleware, LoggingMiddleware, RestAuthState, TenantMiddleware};
pub use plugin_handlers::{PluginHandlers, PluginState};
pub use routes::{
    create_routes, create_routes_with_auth, create_routes_with_context,
    create_routes_with_encryption_state, create_routes_with_memory_state,
    create_routes_with_sync_state, create_routes_with_tenant_state,
};
pub use server::{EvifServer, ServerConfig};
pub use sync_handlers::{
    ConflictHistoryResponse, ConflictRecord, DeltaChange, DeltaRequest, DeltaResponse,
    ResolveConflictRequest, SingleConflictResolution, SyncHandlers, SyncState, SyncStatus,
    WatchEvent,
};
pub use tenant_handlers::{
    CreateTenantRequest, TenantHandlers, TenantInfo, TenantState, TenantStatus, UpdateQuotaRequest,
    DEFAULT_TENANT_ID, TENANT_HEADER,
};
pub use ws_handlers::{WSMessage, WebSocketHandlers, WebSocketState};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// REST API 错误类型
#[derive(Debug, thiserror::Error)]
pub enum RestError {
    #[error("HTTP error: {0}")]
    Http(#[from] axum::http::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    /// Phase 14.2: 资源冲突（锁已存在）
    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

// From EvifError to RestError conversion
impl From<evif_core::EvifError> for RestError {
    fn from(err: evif_core::EvifError) -> Self {
        match err {
            evif_core::EvifError::NotFound(_) => RestError::NotFound(err.to_string()),
            evif_core::EvifError::InvalidPath(_) => RestError::BadRequest(err.to_string()),
            evif_core::EvifError::InvalidArgument(_) => RestError::BadRequest(err.to_string()),
            evif_core::EvifError::InvalidInput(_) => RestError::BadRequest(err.to_string()),
            evif_core::EvifError::PermissionDenied(_) => RestError::BadRequest(err.to_string()),
            evif_core::EvifError::AlreadyExists(_) => RestError::BadRequest(err.to_string()),
            evif_core::EvifError::NotMounted(_) => RestError::NotFound(err.to_string()),
            evif_core::EvifError::Conflict(_) => RestError::Conflict(err.to_string()),
            evif_core::EvifError::Io(io_err) => match io_err.kind() {
                std::io::ErrorKind::NotFound => RestError::NotFound(io_err.to_string()),
                std::io::ErrorKind::PermissionDenied => RestError::BadRequest(io_err.to_string()),
                std::io::ErrorKind::AlreadyExists => RestError::BadRequest(io_err.to_string()),
                std::io::ErrorKind::InvalidInput => RestError::BadRequest(io_err.to_string()),
                _ => RestError::Internal(io_err.to_string()),
            },
            _ => RestError::Internal(err.to_string()),
        }
    }
}

impl IntoResponse for RestError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            RestError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            RestError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            RestError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            RestError::Io(err) => match err.kind() {
                std::io::ErrorKind::NotFound => (StatusCode::NOT_FOUND, err.to_string()),
                std::io::ErrorKind::PermissionDenied => (StatusCode::FORBIDDEN, err.to_string()),
                std::io::ErrorKind::AlreadyExists => (StatusCode::CONFLICT, err.to_string()),
                std::io::ErrorKind::InvalidInput => (StatusCode::BAD_REQUEST, err.to_string()),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            },
            RestError::Http(err) => (StatusCode::BAD_REQUEST, err.to_string()),
            RestError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(json!({
            "error": status.to_string(),
            "message": message,
        }));

        (status, body).into_response()
    }
}

pub type RestResult<T> = Result<T, RestError>;
