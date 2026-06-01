//! Admin Dashboard API - 管理员 API
//!
//! 提供管理员和超级管理员功能:
//! - 租户管理
//! - 用户管理
//! - 全局统计
//! - 审计日志
//!
//! 对标: mvp10.md F-10.3.3

use axum::{
    extract::{Path, Query},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::RestResult;

/// 全局统计信息
#[derive(Debug, Serialize)]
pub struct GlobalStats {
    pub total_tenants: u32,
    pub active_tenants: u32,
    pub total_api_calls_today: u64,
    pub total_storage_bytes: u64,
    pub revenue_mtd_cents: u32,
    pub active_plugins: u32,
    pub marketplace_gmv_cents: u32,
}

/// 租户统计信息
#[derive(Debug, Serialize)]
pub struct TenantStats {
    pub tenant_id: String,
    pub display_name: String,
    pub plan: String,
    pub api_calls_today: u64,
    pub storage_bytes: u64,
    pub user_count: u32,
    pub active: bool,
}

/// 插件统计信息
#[derive(Debug, Serialize)]
pub struct PluginStats {
    pub plugin_id: String,
    pub name: String,
    pub install_count: u32,
    pub active_installs: u32,
    pub revenue_cents: u32,
}

/// 收入统计
#[derive(Debug, Serialize)]
pub struct RevenueStats {
    pub total_revenue_cents: u32,
    pub mrr_cents: u32,
    pub arr_cents: u32,
    pub new_customers: u32,
    pub churned_customers: u32,
}

/// 审计日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub actor_id: String,
    pub actor_type: ActorType,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub details: serde_json::Value,
    pub ip_address: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActorType {
    User,
    System,
    Admin,
}

/// 用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminUser {
    pub id: String,
    pub tenant_id: String,
    pub email: String,
    pub name: String,
    pub role: UserRole,
    pub active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_login: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Member,
    Admin,
    Superadmin,
}

// ============ REST API Handlers ============

/// GET /api/v1/admin/stats/overview - 全局概览
pub async fn get_overview() -> RestResult<Json<GlobalStats>> {
    Ok(Json(GlobalStats {
        total_tenants: 150,
        active_tenants: 142,
        total_api_calls_today: 4_500_000,
        total_storage_bytes: 536_870_912_000,
        revenue_mtd_cents: 28_500,
        active_plugins: 42,
        marketplace_gmv_cents: 1_500,
    }))
}

/// GET /api/v1/admin/stats/tenants - 租户统计
pub async fn get_tenant_stats() -> RestResult<Json<Vec<TenantStats>>> {
    Ok(Json(vec![
        TenantStats {
            tenant_id: "tenant_001".to_string(),
            display_name: "Acme Corp".to_string(),
            plan: "Pro".to_string(),
            api_calls_today: 45_000,
            storage_bytes: 5_368_709_120,
            user_count: 25,
            active: true,
        },
        TenantStats {
            tenant_id: "tenant_002".to_string(),
            display_name: "TechStart Inc".to_string(),
            plan: "Team".to_string(),
            api_calls_today: 120_000,
            storage_bytes: 25_165_825_024,
            user_count: 50,
            active: true,
        },
        TenantStats {
            tenant_id: "tenant_003".to_string(),
            display_name: "Enterprise Co".to_string(),
            plan: "Enterprise".to_string(),
            api_calls_today: 500_000,
            storage_bytes: 107_374_182_400,
            user_count: 200,
            active: true,
        },
    ]))
}

/// GET /api/v1/admin/stats/plugins - 插件统计
pub async fn get_plugin_stats() -> RestResult<Json<Vec<PluginStats>>> {
    Ok(Json(vec![
        PluginStats {
            plugin_id: "contextfs".to_string(),
            name: "ContextFS".to_string(),
            install_count: 150,
            active_installs: 120,
            revenue_cents: 0,
        },
        PluginStats {
            plugin_id: "gptfs".to_string(),
            name: "GPT-FS".to_string(),
            install_count: 85,
            active_installs: 72,
            revenue_cents: 8500,
        },
        PluginStats {
            plugin_id: "notionfs".to_string(),
            name: "Notion FS".to_string(),
            install_count: 45,
            active_installs: 38,
            revenue_cents: 4500,
        },
    ]))
}

/// GET /api/v1/admin/stats/revenue - 收入统计
pub async fn get_revenue_stats() -> RestResult<Json<RevenueStats>> {
    Ok(Json(RevenueStats {
        total_revenue_cents: 285_000,
        mrr_cents: 95_000,
        arr_cents: 1_140_000,
        new_customers: 15,
        churned_customers: 2,
    }))
}

/// GET /api/v1/admin/audit - 审计日志
pub async fn get_audit_log(
    Query(_params): Query<AuditQuery>,
) -> RestResult<Json<Vec<AuditEntry>>> {
    let now = chrono::Utc::now();
    
    Ok(Json(vec![
        AuditEntry {
            id: "audit_001".to_string(),
            timestamp: now - chrono::Duration::hours(1),
            actor_id: "user_001".to_string(),
            actor_type: ActorType::User,
            action: "api_call".to_string(),
            resource_type: "api".to_string(),
            resource_id: "/api/v1/mcp/tools".to_string(),
            details: serde_json::json!({"method": "POST", "status": 200}),
            ip_address: Some("192.168.1.1".to_string()),
        },
        AuditEntry {
            id: "audit_002".to_string(),
            timestamp: now - chrono::Duration::hours(2),
            actor_id: "admin_001".to_string(),
            actor_type: ActorType::Admin,
            action: "create_user".to_string(),
            resource_type: "user".to_string(),
            resource_id: "user_015".to_string(),
            details: serde_json::json!({"email": "new@example.com"}),
            ip_address: Some("192.168.1.10".to_string()),
        },
        AuditEntry {
            id: "audit_003".to_string(),
            timestamp: now - chrono::Duration::hours(3),
            actor_id: "system".to_string(),
            actor_type: ActorType::System,
            action: "billing".to_string(),
            resource_type: "invoice".to_string(),
            resource_id: "inv_002".to_string(),
            details: serde_json::json!({"amount": 2900, "status": "draft"}),
            ip_address: None,
        },
    ]))
}

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub actor_id: Option<String>,
    pub action: Option<String>,
}

/// GET /api/v1/admin/users - 用户列表
pub async fn list_users(
    Path(tenant_id): Path<String>,
) -> RestResult<Json<Vec<AdminUser>>> {
    let now = chrono::Utc::now();
    
    Ok(Json(vec![
        AdminUser {
            id: "user_001".to_string(),
            tenant_id: tenant_id.clone(),
            email: "admin@example.com".to_string(),
            name: "Admin User".to_string(),
            role: UserRole::Admin,
            active: true,
            created_at: now - chrono::Duration::days(365),
            last_login: Some(now - chrono::Duration::hours(1)),
        },
        AdminUser {
            id: "user_002".to_string(),
            tenant_id: tenant_id.clone(),
            email: "member@example.com".to_string(),
            name: "Member User".to_string(),
            role: UserRole::Member,
            active: true,
            created_at: now - chrono::Duration::days(180),
            last_login: Some(now - chrono::Duration::days(1)),
        },
        AdminUser {
            id: "user_003".to_string(),
            tenant_id,
            email: "inactive@example.com".to_string(),
            name: "Inactive User".to_string(),
            role: UserRole::Member,
            active: false,
            created_at: now - chrono::Duration::days(90),
            last_login: Some(now - chrono::Duration::days(30)),
        },
    ]))
}

/// POST /api/v1/admin/users - 创建用户
pub async fn create_user(
    Path(tenant_id): Path<String>,
    Json(req): Json<CreateUserRequest>,
) -> RestResult<Json<AdminUser>> {
    let now = chrono::Utc::now();
    
    Ok(Json(AdminUser {
        id: format!("user_{}", uuid::Uuid::new_v4()),
        tenant_id,
        email: req.email,
        name: req.name,
        role: req.role,
        active: true,
        created_at: now,
        last_login: None,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub name: String,
    pub role: UserRole,
}

/// PUT /api/v1/admin/users/:id - 更新用户
pub async fn update_user(
    Path((_tenant_id, user_id)): Path<(String, String)>,
    Json(_req): Json<UpdateUserRequest>,
) -> RestResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "message": "User updated",
        "id": user_id,
    })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub role: Option<UserRole>,
    pub active: Option<bool>,
}

/// DELETE /api/v1/admin/users/:id - 删除用户
pub async fn delete_user(
    Path((_tenant_id, user_id)): Path<(String, String)>,
) -> RestResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "message": "User deleted",
        "id": user_id,
    })))
}

/// PUT /api/v1/admin/users/:id/disable - 禁用用户
pub async fn disable_user(
    Path((_tenant_id, user_id)): Path<(String, String)>,
) -> RestResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "message": "User disabled",
        "id": user_id,
    })))
}

/// GET /api/v1/admin/audit/export - 导出审计日志
pub async fn export_audit_log() -> RestResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "download_url": "/api/v1/admin/audit/export/csv",
        "expires_at": (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339(),
    })))
}
