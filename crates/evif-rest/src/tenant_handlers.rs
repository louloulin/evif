// Phase 17.1: Multi-tenant support
//
// 提供租户隔离和多租户管理功能

use crate::{RestError, RestResult};
use axum::{extract::State, http::HeaderValue, response::IntoResponse, Json};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

/// Default tenant ID for requests without tenant header
pub const DEFAULT_TENANT_ID: &str = "default";

/// Header name for tenant identification
pub const TENANT_HEADER: &str = "x-tenant-id";

/// Tenant information
#[derive(Clone, Debug, Serialize, Deserialize, async_graphql::SimpleObject)]
pub struct TenantInfo {
    pub id: String,
    pub name: String,
    /// Storage quota in bytes (0 = unlimited)
    pub storage_quota: u64,
    /// Current storage usage in bytes
    pub storage_used: u64,
    pub status: TenantStatus,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Copy, async_graphql::Enum)]
#[serde(rename_all = "lowercase")]
pub enum TenantStatus {
    Active,
    Suspended,
    Deleted,
}

/// Tenant state manager
#[derive(Clone)]
pub struct TenantState {
    tenants: Arc<RwLock<HashMap<String, TenantInfo>>>,
    persistence_path: Arc<Option<PathBuf>>,
}

impl Default for TenantState {
    fn default() -> Self {
        Self::new()
    }
}

impl TenantState {
    pub fn new() -> Self {
        let tenants = Self::default_tenants();
        Self {
            tenants: Arc::new(RwLock::new(tenants)),
            persistence_path: Arc::new(None),
        }
    }

    pub fn from_env() -> Result<Self, String> {
        match std::env::var("EVIF_REST_TENANT_STATE_PATH") {
            Ok(path) if !path.trim().is_empty() => Self::persistent(path.trim()),
            _ => Ok(Self::new()),
        }
    }

    pub fn persistent(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        let tenants = if path.exists() {
            Self::load_snapshot(&path)?
        } else {
            let tenants = Self::default_tenants();
            Self::save_snapshot(&path, &tenants)?;
            tenants
        };

        Ok(Self {
            tenants: Arc::new(RwLock::new(tenants)),
            persistence_path: Arc::new(Some(path)),
        })
    }

    fn default_tenants() -> HashMap<String, TenantInfo> {
        let mut map = HashMap::new();
        map.insert(
            DEFAULT_TENANT_ID.to_string(),
            TenantInfo {
                id: DEFAULT_TENANT_ID.to_string(),
                name: "Default Tenant".to_string(),
                storage_quota: 0,
                storage_used: 0,
                status: TenantStatus::Active,
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        );
        map
    }

    fn load_snapshot(path: &Path) -> Result<HashMap<String, TenantInfo>, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read tenant state '{}': {}", path.display(), e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse tenant state '{}': {}", path.display(), e))
    }

    fn save_snapshot(path: &Path, tenants: &HashMap<String, TenantInfo>) -> Result<(), String> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            if let Err(e) = fs::create_dir_all(parent) {
                // Ignore "already exists" errors — directory is fine as-is
                if e.kind() != std::io::ErrorKind::AlreadyExists {
                    return Err(format!(
                        "Failed to create tenant state parent '{}': {}",
                        parent.display(),
                        e
                    ));
                }
            }
        }
        let content = serde_json::to_string_pretty(tenants)
            .map_err(|e| format!("Failed to serialize tenant state: {}", e))?;
        fs::write(path, content)
            .map_err(|e| format!("Failed to write tenant state '{}': {}", path.display(), e))
    }

    fn persist(&self, tenants: &HashMap<String, TenantInfo>) -> Result<(), String> {
        if let Some(path) = self.persistence_path.as_ref().as_ref() {
            Self::save_snapshot(path, tenants)?;
        }
        Ok(())
    }

    pub async fn create_tenant(&self, req: CreateTenantRequest) -> Result<TenantInfo, String> {
        let tenant = TenantInfo {
            id: Uuid::new_v4().to_string(),
            name: req.name,
            storage_quota: req.storage_quota.unwrap_or(0),
            storage_used: 0,
            status: TenantStatus::Active,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        let mut tenants = self.tenants.write();
        tenants.insert(tenant.id.clone(), tenant.clone());
        self.persist(&tenants)?;
        Ok(tenant)
    }

    pub async fn list_tenants(&self) -> Vec<TenantInfo> {
        let tenants = self.tenants.read();
        tenants.values().cloned().collect()
    }

    pub async fn get_tenant(&self, id: &str) -> Option<TenantInfo> {
        let tenants = self.tenants.read();
        tenants.get(id).cloned()
    }

    pub async fn delete_tenant(&self, id: &str) -> Result<bool, String> {
        if id == DEFAULT_TENANT_ID {
            return Ok(false); // Cannot delete default tenant
        }
        let mut tenants = self.tenants.write();
        let deleted = tenants.remove(id).is_some();
        if deleted {
            self.persist(&tenants)?;
        }
        Ok(deleted)
    }

    pub async fn update_storage_used(&self, id: &str, used: u64) -> Result<(), String> {
        let mut tenants = self.tenants.write();
        if let Some(tenant) = tenants.get_mut(id) {
            tenant.storage_used = used;
        }
        self.persist(&tenants)
    }

    /// 检查租户是否有足够配额写入指定字节数
    /// quota=0 表示无限制（不限制）
    pub fn check_quota(&self, id: &str, additional_bytes: u64) -> bool {
        let tenants = self.tenants.read();
        if let Some(tenant) = tenants.get(id) {
            if tenant.storage_quota == 0 {
                return true; // unlimited
            }
            return tenant.storage_used.saturating_add(additional_bytes) <= tenant.storage_quota;
        }
        false // 租户不存在，拒绝写入
    }

    /// 记录一次写入操作，更新 storage_used
    /// 返回是否成功
    pub fn record_write(&self, id: &str, bytes: u64) -> Result<(), String> {
        let mut tenants = self.tenants.write();
        if let Some(tenant) = tenants.get_mut(id) {
            tenant.storage_used = tenant.storage_used.saturating_add(bytes);
        }
        self.persist(&tenants)
    }

    /// Update storage quota for a tenant
    pub async fn update_storage_quota(&self, id: &str, quota: u64) -> Result<TenantInfo, String> {
        let mut tenants = self.tenants.write();
        let tenant = tenants
            .get_mut(id)
            .ok_or_else(|| format!("Tenant '{}' not found", id))?;
        tenant.storage_quota = quota;
        let info = tenant.clone();
        self.persist(&tenants)?;
        Ok(info)
    }

    /// 同步设置租户存储配额（供测试和初始化使用）
    pub fn update_storage_quota_sync(&self, id: &str, quota: u64) {
        let mut tenants = self.tenants.write();
        if let Some(tenant) = tenants.get_mut(id) {
            tenant.storage_quota = quota;
        }
        // Don't persist here — caller should call persist() explicitly if needed
    }

    /// 直接插入租户（ID 由调用方指定，用于测试）
    pub fn insert_tenant(&self, id: &str, tenant: TenantInfo) {
        let mut tenants = self.tenants.write();
        tenants.insert(id.to_string(), tenant);
    }

    /// Get the effective tenant ID, defaulting to DEFAULT_TENANT_ID
    pub fn effective_tenant_id(tenant_header: Option<&HeaderValue>) -> String {
        tenant_header
            .and_then(|v| v.to_str().ok())
            .filter(|v| !v.is_empty())
            .unwrap_or(DEFAULT_TENANT_ID)
            .to_string()
    }
}

/// Request to create a new tenant
#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
    #[serde(default)]
    pub storage_quota: Option<u64>,
}

/// Request to update tenant storage quota
#[derive(Debug, Deserialize)]
pub struct UpdateQuotaRequest {
    pub storage_quota: u64,
}

/// Tenant handlers
pub struct TenantHandlers;

impl TenantHandlers {
    /// GET /api/v1/tenants - List all tenants (admin only)
    pub async fn list_tenants(
        State(state): State<TenantState>,
    ) -> RestResult<Json<Vec<TenantInfo>>> {
        let tenants = state.list_tenants().await;
        Ok(Json(tenants))
    }

    /// POST /api/v1/tenants - Create a new tenant
    pub async fn create_tenant(
        State(state): State<TenantState>,
        Json(req): Json<CreateTenantRequest>,
    ) -> RestResult<impl IntoResponse> {
        if req.name.is_empty() {
            return Err(RestError::BadRequest("Tenant name cannot be empty".into()));
        }
        let tenant = state
            .create_tenant(req)
            .await
            .map_err(RestError::Internal)?;
        Ok(Json(tenant))
    }

    /// GET /api/v1/tenants/:id - Get tenant details
    pub async fn get_tenant(
        State(state): State<TenantState>,
        tenant_id: axum::extract::Path<String>,
    ) -> RestResult<impl IntoResponse> {
        let tenant = state.get_tenant(&tenant_id).await;
        match tenant {
            Some(t) => Ok(Json(t)),
            None => Err(RestError::NotFound(format!(
                "Tenant {:?} not found",
                tenant_id
            ))),
        }
    }

    /// DELETE /api/v1/tenants/:id - Delete a tenant
    pub async fn delete_tenant(
        State(state): State<TenantState>,
        tenant_id: axum::extract::Path<String>,
    ) -> RestResult<impl IntoResponse> {
        let deleted = state
            .delete_tenant(&tenant_id)
            .await
            .map_err(RestError::Internal)?;
        if deleted {
            Ok(Json(serde_json::json!({ "deleted": true })))
        } else {
            Err(RestError::BadRequest("Cannot delete default tenant".into()))
        }
    }

    /// PATCH /api/v1/tenants/:id/quota - Update storage quota
    pub async fn update_quota(
        State(state): State<TenantState>,
        tenant_id: axum::extract::Path<String>,
        Json(req): Json<UpdateQuotaRequest>,
    ) -> RestResult<impl IntoResponse> {
        let updated = state
            .update_storage_quota(&tenant_id, req.storage_quota)
            .await
            .map_err(RestError::NotFound)?;
        Ok(Json(updated))
    }

    /// GET /api/v1/tenants/me - Get current tenant info
    /// Returns the default tenant or the tenant identified by X-Tenant-ID header
    pub async fn get_current_tenant(
        State(state): State<TenantState>,
        req: axum::extract::Request,
    ) -> RestResult<impl IntoResponse> {
        // Try to get tenant from X-Tenant-ID header, fallback to default
        let tenant_id = req
            .headers()
            .get(TENANT_HEADER)
            .and_then(|v| v.to_str().ok())
            .filter(|v| !v.is_empty())
            .unwrap_or(DEFAULT_TENANT_ID)
            .to_string();

        let tenant = state.get_tenant(&tenant_id).await;
        match tenant {
            Some(t) => Ok(Json(t)),
            None => {
                // Return minimal tenant info if not found
                Ok(Json(TenantInfo {
                    id: tenant_id,
                    name: "Unknown".to_string(),
                    storage_quota: 0,
                    storage_used: 0,
                    status: TenantStatus::Active,
                    created_at: String::new(),
                }))
            }
        }
    }
}
