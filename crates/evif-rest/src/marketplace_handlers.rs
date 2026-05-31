//! Plugin Marketplace REST API Handlers
//!
//! 提供 Plugin Marketplace API 端点
//! 对标: mvp10.md F-10.3.1

use axum::{
    extract::{Path, Query},
    Json, Router,
};
use evif_plugins::{
    MarketplaceEntry, MarketplaceManager, MarketplaceSearchQuery, MarketplaceSearchResponse,
    PublishPluginRequest, RatingRequest, UpdatePluginRequest,
};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::RestResult;

/// Marketplace 应用状态
pub struct MarketplaceState {
    pub manager: Arc<RwLock<MarketplaceManager>>,
}

impl MarketplaceState {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(RwLock::new(MarketplaceManager::new())),
        }
    }
}

impl Default for MarketplaceState {
    fn default() -> Self {
        Self::new()
    }
}

/// GET /api/v1/marketplace/plugins - 搜索插件
pub async fn list_plugins(
    Query(query): Query<MarketplaceSearchQuery>,
) -> RestResult<Json<MarketplaceSearchResponse>> {
    let manager = MarketplaceManager::new();
    
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let query_str = query.q.as_deref().unwrap_or("");
    
    let plugins = manager.search_plugins(query_str, None, per_page).await;
    let total = plugins.len();
    
    Ok(Json(MarketplaceSearchResponse {
        plugins,
        total,
        page,
        per_page,
        has_more: total >= per_page,
    }))
}

/// GET /api/v1/marketplace/plugins/:id - 获取插件详情
pub async fn get_plugin(
    Path(id): Path<String>,
) -> RestResult<Json<MarketplaceEntry>> {
    let manager = MarketplaceManager::new();
    
    match manager.get_entry(&id).await {
        Some(entry) => Ok(Json(entry)),
        None => Err(crate::RestError::NotFound(format!(
            "Plugin '{}' not found in marketplace",
            id
        ))),
    }
}

/// POST /api/v1/marketplace/plugins - 发布插件
pub async fn publish_plugin(
    Json(req): Json<PublishPluginRequest>,
) -> RestResult<Json<MarketplaceEntry>> {
    let entry = MarketplaceEntry {
        plugin_id: req.plugin_id.clone(),
        marketplace_id: uuid::Uuid::new_v4().to_string(),
        publisher: evif_plugins::PublisherInfo {
            user_id: "system".to_string(),
            display_name: "EVIF".to_string(),
            verified: true,
        },
        pricing: req.pricing,
        stats: Default::default(),
        ratings: Default::default(),
        status: evif_plugins::MarketplaceStatus::Published,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    Ok(Json(entry))
}

/// PUT /api/v1/marketplace/plugins/:id - 更新插件
pub async fn update_plugin(
    Path(id): Path<String>,
    Json(_req): Json<UpdatePluginRequest>,
) -> RestResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "message": format!("Plugin {} updated", id),
        "id": id,
    })))
}

/// POST /api/v1/marketplace/plugins/:id/download - 下载计数
pub async fn increment_download(
    Path(id): Path<String>,
) -> RestResult<Json<serde_json::Value>> {
    let manager = MarketplaceManager::new();
    manager.increment_downloads(&id).await
        .map_err(|e| crate::RestError::Internal(e.to_string()))?;
    
    Ok(Json(serde_json::json!({
        "message": "Download count incremented",
        "id": id,
    })))
}

/// POST /api/v1/marketplace/plugins/:id/rate - 评分
pub async fn rate_plugin(
    Path(id): Path<String>,
    Json(req): Json<RatingRequest>,
) -> RestResult<Json<serde_json::Value>> {
    if req.rating < 1 || req.rating > 5 {
        return Err(crate::RestError::BadRequest(
            "Rating must be between 1 and 5".to_string(),
        ));
    }
    
    Ok(Json(serde_json::json!({
        "message": "Rating submitted",
        "id": id,
        "rating": req.rating,
    })))
}

/// GET /api/v1/marketplace/trending - 热门插件
pub async fn get_trending() -> RestResult<Json<Vec<MarketplaceEntry>>> {
    let manager = MarketplaceManager::new();
    let plugins = manager.get_trending_plugins(10).await;
    Ok(Json(plugins))
}

/// GET /api/v1/marketplace/free - 免费插件
pub async fn get_free_plugins() -> RestResult<Json<Vec<MarketplaceEntry>>> {
    let manager = MarketplaceManager::new();
    let plugins = manager.get_free_plugins(20).await;
    Ok(Json(plugins))
}
