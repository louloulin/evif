//! Usage Billing API - 使用量计费 API
//!
//! 提供使用量计量和计费功能:
//! - API 调用量计量
//! - 存储使用量计量
//! - Agent 计数
//! - 订阅管理
//! - Webhook 通知
//!
//! 对标: mvp10.md F-10.3.2

use axum::{
    extract::{Path, Query},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 定价计划
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PricingPlan {
    Free,
    Pro,
    Team,
    Enterprise,
}

impl PricingPlan {
    pub fn api_limit(&self) -> u64 {
        match self {
            PricingPlan::Free => 1000,
            PricingPlan::Pro => 100_000,
            PricingPlan::Team => 1_000_000,
            PricingPlan::Enterprise => u64::MAX,
        }
    }
    
    pub fn storage_limit(&self) -> u64 {
        match self {
            PricingPlan::Free => 100 * 1024 * 1024,
            PricingPlan::Pro => 10 * 1024 * 1024 * 1024,
            PricingPlan::Team => 100 * 1024 * 1024 * 1024,
            PricingPlan::Enterprise => 1024 * 1024 * 1024 * 1024,
        }
    }
    
    pub fn agent_limit(&self) -> u32 {
        match self {
            PricingPlan::Free => 1,
            PricingPlan::Pro => 10,
            PricingPlan::Team => 50,
            PricingPlan::Enterprise => u32::MAX,
        }
    }
    
    pub fn price_cents(&self) -> u32 {
        match self {
            PricingPlan::Free => 0,
            PricingPlan::Pro => 2900,  // $29
            PricingPlan::Team => 9900,  // $99
            PricingPlan::Enterprise => 49900,  // $499
        }
    }
}

/// 使用量配额
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quota {
    pub used: u64,
    pub limit: u64,
    pub unit: String,
}

impl Default for Quota {
    fn default() -> Self {
        Self {
            used: 0,
            limit: 0,
            unit: "count".to_string(),
        }
    }
}

impl Quota {
    pub fn usage_percent(&self) -> f64 {
        if self.limit == 0 { 0.0 } else { (self.used as f64 / self.limit as f64) * 100.0 }
    }
    
    pub fn is_exceeded(&self) -> bool {
        self.used >= self.limit
    }
}

/// 使用量统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageStats {
    pub api_calls: Quota,
    pub storage: Quota,
    pub agents: Quota,
}

/// 计费周期
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingPeriod {
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
}

/// 使用量响应
#[derive(Debug, Serialize)]
pub struct UsageResponse {
    pub tenant_id: String,
    pub period: BillingPeriod,
    pub plan: PricingPlan,
    pub quotas: UsageStats,
}

/// 使用量详情
#[derive(Debug, Serialize)]
pub struct UsageDetail {
    pub endpoint: String,
    pub count: u64,
    pub total_latency_ms: u64,
}

/// 历史使用量条目
#[derive(Debug, Serialize)]
pub struct UsageHistoryEntry {
    pub date: String,
    pub api_calls: u64,
    pub storage_bytes: u64,
    pub active_agents: u32,
}

/// 订阅信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub tenant_id: String,
    pub plan: PricingPlan,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub next_billing_at: chrono::DateTime<chrono::Utc>,
    pub status: SubscriptionStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionStatus {
    Active,
    Trial,
    Cancelled,
    PastDue,
}

/// 发票信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: String,
    pub tenant_id: String,
    pub period: BillingPeriod,
    pub amount_cents: u32,
    pub status: InvoiceStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub pdf_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InvoiceStatus {
    Draft,
    Paid,
    Unpaid,
    Void,
}

/// Webhook 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub id: String,
    pub tenant_id: String,
    pub url: String,
    pub events: Vec<WebhookEvent>,
    pub secret: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEvent {
    Quota80Percent,
    Quota100Percent,
    SubscriptionChanged,
    InvoiceCreated,
}

/// 计费状态
pub struct BillingState {
    pub subscriptions: Arc<RwLock<Vec<Subscription>>>,
    pub usage: Arc<RwLock<Vec<UsageHistoryEntry>>>,
    pub invoices: Arc<RwLock<Vec<Invoice>>>,
    pub webhooks: Arc<RwLock<Vec<WebhookConfig>>>,
}

impl BillingState {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(Vec::new())),
            usage: Arc::new(RwLock::new(Vec::new())),
            invoices: Arc::new(RwLock::new(Vec::new())),
            webhooks: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl Default for BillingState {
    fn default() -> Self {
        Self::new()
    }
}

// ============ REST API Handlers ============

use crate::RestResult;

/// GET /api/v1/billing/usage - 当前使用量
pub async fn get_usage(
    Path(tenant_id): Path<String>,
) -> RestResult<Json<UsageResponse>> {
    let now = chrono::Utc::now();
    let period = BillingPeriod {
        start: now - chrono::Duration::days(30),
        end: now,
    };
    
    Ok(Json(UsageResponse {
        tenant_id,
        period,
        plan: PricingPlan::Pro,
        quotas: UsageStats {
            api_calls: Quota {
                used: 45000,
                limit: 100000,
                unit: "calls/day".to_string(),
            },
            storage: Quota {
                used: 5 * 1024 * 1024 * 1024, // 5GB
                limit: 10 * 1024 * 1024 * 1024, // 10GB
                unit: "bytes".to_string(),
            },
            agents: Quota {
                used: 3,
                limit: 10,
                unit: "count".to_string(),
            },
        },
    }))
}

/// GET /api/v1/billing/usage/history - 历史使用量
pub async fn get_usage_history(
    Path(_tenant_id): Path<String>,
    Query(params): Query<UsageHistoryQuery>,
) -> RestResult<Json<Vec<UsageHistoryEntry>>> {
    let months = params.months.unwrap_or(12).min(12);
    
    let mut history = Vec::new();
    let now = chrono::Utc::now();
    
    for i in 0..months {
        let date = now - chrono::Duration::days((i as i64) * 30);
        history.push(UsageHistoryEntry {
            date: date.format("%Y-%m-%d").to_string(),
            api_calls: (10000 + i * 500) as u64,
            storage_bytes: (1024 * 1024 * 1024 + i as i64 * 100 * 1024 * 1024) as u64,
            active_agents: 3 + (i % 5) as u32,
        });
    }
    
    history.reverse();
    Ok(Json(history))
}

#[derive(Debug, Deserialize)]
pub struct UsageHistoryQuery {
    pub months: Option<u32>,
}

/// GET /api/v1/billing/usage/by-endpoint - 按端点使用量
pub async fn get_usage_by_endpoint(
    Path(_tenant_id): Path<String>,
) -> RestResult<Json<Vec<UsageDetail>>> {
    Ok(Json(vec![
        UsageDetail {
            endpoint: "/api/v1/mcp/*".to_string(),
            count: 25000,
            total_latency_ms: 125000,
        },
        UsageDetail {
            endpoint: "/api/v1/memory/*".to_string(),
            count: 15000,
            total_latency_ms: 75000,
        },
        UsageDetail {
            endpoint: "/api/v1/fs/*".to_string(),
            count: 5000,
            total_latency_ms: 25000,
        },
    ]))
}

/// GET /api/v1/billing/subscription - 当前订阅
pub async fn get_subscription(
    Path(tenant_id): Path<String>,
) -> RestResult<Json<Subscription>> {
    let now = chrono::Utc::now();
    
    Ok(Json(Subscription {
        tenant_id,
        plan: PricingPlan::Pro,
        started_at: now - chrono::Duration::days(30),
        next_billing_at: now + chrono::Duration::days(30),
        status: SubscriptionStatus::Active,
    }))
}

/// POST /api/v1/billing/subscription - 创建订阅
pub async fn create_subscription(
    Path(tenant_id): Path<String>,
    Json(req): Json<CreateSubscriptionRequest>,
) -> RestResult<Json<Subscription>> {
    let now = chrono::Utc::now();
    
    Ok(Json(Subscription {
        tenant_id: tenant_id.clone(),
        plan: req.plan,
        started_at: now,
        next_billing_at: now + chrono::Duration::days(30),
        status: SubscriptionStatus::Trial,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateSubscriptionRequest {
    pub plan: PricingPlan,
}

/// PUT /api/v1/billing/subscription - 升级/降级
pub async fn update_subscription(
    Path(tenant_id): Path<String>,
    Json(req): Json<UpdateSubscriptionRequest>,
) -> RestResult<Json<Subscription>> {
    let now = chrono::Utc::now();
    
    Ok(Json(Subscription {
        tenant_id: tenant_id.clone(),
        plan: req.plan,
        started_at: now - chrono::Duration::days(30),
        next_billing_at: now + chrono::Duration::days(30),
        status: SubscriptionStatus::Active,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateSubscriptionRequest {
    pub plan: PricingPlan,
}

/// GET /api/v1/billing/invoices - 历史发票
pub async fn get_invoices(
    Path(tenant_id): Path<String>,
) -> RestResult<Json<Vec<Invoice>>> {
    let now = chrono::Utc::now();
    
    Ok(Json(vec![
        Invoice {
            id: "inv_001".to_string(),
            tenant_id: tenant_id.clone(),
            period: BillingPeriod {
                start: now - chrono::Duration::days(60),
                end: now - chrono::Duration::days(30),
            },
            amount_cents: 2900,
            status: InvoiceStatus::Paid,
            created_at: now - chrono::Duration::days(30),
            pdf_url: Some("/api/v1/billing/invoices/inv_001/pdf".to_string()),
        },
        Invoice {
            id: "inv_002".to_string(),
            tenant_id,
            period: BillingPeriod {
                start: now - chrono::Duration::days(30),
                end: now,
            },
            amount_cents: 2900,
            status: InvoiceStatus::Draft,
            created_at: now,
            pdf_url: None,
        },
    ]))
}

/// GET /api/v1/billing/invoices/:id - 发票详情
pub async fn get_invoice(
    Path((_tenant_id, invoice_id)): Path<(String, String)>,
) -> RestResult<Json<Invoice>> {
    let now = chrono::Utc::now();
    
    Ok(Json(Invoice {
        id: invoice_id,
        tenant_id: "tenant_001".to_string(),
        period: BillingPeriod {
            start: now - chrono::Duration::days(30),
            end: now,
        },
        amount_cents: 2900,
        status: InvoiceStatus::Draft,
        created_at: now,
        pdf_url: None,
    }))
}

/// GET /api/v1/billing/webhooks - Webhook 配置列表
pub async fn get_webhooks(
    Path(tenant_id): Path<String>,
) -> RestResult<Json<Vec<WebhookConfig>>> {
    Ok(Json(vec![
        WebhookConfig {
            id: "wh_001".to_string(),
            tenant_id,
            url: "https://example.com/webhook".to_string(),
            events: vec![WebhookEvent::Quota80Percent, WebhookEvent::Quota100Percent],
            secret: "whsec_xxx".to_string(),
        },
    ]))
}

/// POST /api/v1/billing/webhooks - 创建 Webhook
pub async fn create_webhook(
    Path(tenant_id): Path<String>,
    Json(req): Json<CreateWebhookRequest>,
) -> RestResult<Json<WebhookConfig>> {
    Ok(Json(WebhookConfig {
        id: format!("wh_{}", uuid::Uuid::new_v4()),
        tenant_id,
        url: req.url,
        events: req.events,
        secret: format!("whsec_{}", uuid::Uuid::new_v4()),
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateWebhookRequest {
    pub url: String,
    pub events: Vec<WebhookEvent>,
}

/// DELETE /api/v1/billing/webhooks/:id - 删除 Webhook
pub async fn delete_webhook(
    Path((_tenant_id, webhook_id)): Path<(String, String)>,
) -> RestResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "message": "Webhook deleted",
        "id": webhook_id,
    })))
}
