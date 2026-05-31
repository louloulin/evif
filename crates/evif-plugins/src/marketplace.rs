//! Plugin Marketplace - 插件市场核心模块
//!
//! 提供 Plugin Marketplace API 核心功能:
//! - 插件注册和发现
//! - 插件评分和评论
//! - 付费插件集成 (通过 Stripe)
//!
//! 对标: mvp10.md F-10.3.1

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 插件市场条目扩展信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceEntry {
    /// 插件 ID (来自 catalog)
    pub plugin_id: String,
    /// 市场唯一标识符
    pub marketplace_id: String,
    /// 发布者信息
    pub publisher: PublisherInfo,
    /// 定价信息
    pub pricing: PluginPricing,
    /// 下载统计
    pub stats: DownloadStats,
    /// 评分统计
    pub ratings: RatingStats,
    /// 市场状态
    pub status: MarketplaceStatus,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 更新时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 发布者信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublisherInfo {
    pub user_id: String,
    pub display_name: String,
    pub verified: bool,
}

/// 插件定价
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PluginPricing {
    /// 免费插件
    Free,
    /// 订阅插件
    Subscription { monthly_price_cents: u32 },
    /// 单次购买插件
    Purchase { price_cents: u32 },
    /// 免费增值插件
    Freenium {
        free_tier: bool,
        pro_price_cents: Option<u32>,
    },
}

/// 下载统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DownloadStats {
    pub total: u64,
    pub monthly: u32,
    pub weekly: u32,
}

/// 评分统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RatingStats {
    pub average: f32,
    pub count: u32,
    pub distribution: HashMap<u8, u32>, // 1-5 星分布
}

/// 市场状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketplaceStatus {
    Draft,
    PendingReview,
    Published,
    Suspended,
    Removed,
}

/// 插件市场管理器
pub struct MarketplaceManager {
    /// 已发布的插件
    entries: Arc<RwLock<HashMap<String, MarketplaceEntry>>>,
    /// Stripe 产品映射
    stripe_products: Arc<RwLock<HashMap<String, String>>>,
}

impl MarketplaceManager {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            stripe_products: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 发布插件到市场
    pub async fn publish_plugin(&self, entry: MarketplaceEntry) -> Result<(), MarketplaceError> {
        let mut entries = self.entries.write().await;
        entries.insert(entry.marketplace_id.clone(), entry);
        Ok(())
    }

    /// 获取插件市场条目
    pub async fn get_entry(&self, marketplace_id: &str) -> Option<MarketplaceEntry> {
        let entries = self.entries.read().await;
        entries.get(marketplace_id).cloned()
    }

    /// 搜索插件
    pub async fn search_plugins(
        &self,
        query: &str,
        tier: Option<&str>,
        limit: usize,
    ) -> Vec<MarketplaceEntry> {
        let entries = self.entries.read().await;
        let query_lower = query.to_lowercase();
        
        entries
            .values()
            .filter(|e| {
                // 过滤已发布的
                e.status == MarketplaceStatus::Published &&
                // 过滤搜索匹配
                (e.plugin_id.to_lowercase().contains(&query_lower) ||
                 e.publisher.display_name.to_lowercase().contains(&query_lower))
            })
            .take(limit)
            .cloned()
            .collect()
    }

    /// 获取热门插件
    pub async fn get_trending_plugins(&self, limit: usize) -> Vec<MarketplaceEntry> {
        let entries = self.entries.read().await;
        
        let mut sorted: Vec<_> = entries
            .values()
            .filter(|e| e.status == MarketplaceStatus::Published)
            .collect();
        
        sorted.sort_by(|a, b| b.stats.monthly.cmp(&a.stats.monthly));
        sorted.into_iter().take(limit).cloned().collect()
    }

    /// 获取免费插件
    pub async fn get_free_plugins(&self, limit: usize) -> Vec<MarketplaceEntry> {
        let entries = self.entries.read().await;
        
        entries
            .values()
            .filter(|e| {
                e.status == MarketplaceStatus::Published &&
                matches!(e.pricing, PluginPricing::Free)
            })
            .take(limit)
            .cloned()
            .collect()
    }

    /// 更新下载统计
    pub async fn increment_downloads(&self, marketplace_id: &str) -> Result<(), MarketplaceError> {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(marketplace_id) {
            entry.stats.total += 1;
            entry.updated_at = chrono::Utc::now();
            Ok(())
        } else {
            Err(MarketplaceError::NotFound(marketplace_id.to_string()))
        }
    }

    /// 关联 Stripe 产品
    pub async fn link_stripe_product(
        &self,
        marketplace_id: &str,
        stripe_product_id: &str,
    ) -> Result<(), MarketplaceError> {
        let mut products = self.stripe_products.write().await;
        products.insert(marketplace_id.to_string(), stripe_product_id.to_string());
        Ok(())
    }

    /// 获取 Stripe 产品 ID
    pub async fn get_stripe_product(&self, marketplace_id: &str) -> Option<String> {
        let products = self.stripe_products.read().await;
        products.get(marketplace_id).cloned()
    }
}

impl Default for MarketplaceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 市场错误类型
#[derive(Debug, thiserror::Error)]
pub enum MarketplaceError {
    #[error("Plugin not found: {0}")]
    NotFound(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Payment error: {0}")]
    Payment(String),
}

// ============ REST API 模型 ============

/// 插件市场搜索请求
#[derive(Debug, Deserialize)]
pub struct MarketplaceSearchQuery {
    pub q: Option<String>,
    pub tier: Option<String>,
    pub category: Option<String>,
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    #[serde(default)]
    pub sort_by: SortBy,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    #[default]
    Popular,
    Recent,
    Rating,
    Price,
}

/// 插件市场搜索响应
#[derive(Debug, Serialize)]
pub struct MarketplaceSearchResponse {
    pub plugins: Vec<MarketplaceEntry>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
    pub has_more: bool,
}

/// 插件发布请求
#[derive(Debug, Deserialize)]
pub struct PublishPluginRequest {
    pub plugin_id: String,
    pub description: Option<String>,
    pub pricing: PluginPricing,
    pub support_tier: Option<String>,
}

/// 插件更新请求
#[derive(Debug, Deserialize)]
pub struct UpdatePluginRequest {
    pub description: Option<String>,
    pub pricing: Option<PluginPricing>,
    pub status: Option<MarketplaceStatus>,
}

/// 插件评分请求
#[derive(Debug, Deserialize)]
pub struct RatingRequest {
    pub rating: u8,
    pub review: Option<String>,
}
