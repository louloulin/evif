// EVIF缓存系统 - 使用moka实现高性能缓存

pub mod cache;
pub mod directory_cache;
pub mod metadata_cache;

pub use cache::{CacheConfig, EvifCache};
pub use directory_cache::DirectoryCache;
pub use metadata_cache::MetadataCache;

use crate::FileInfo;
use std::time::Duration;

/// 缓存键类型
pub type CacheKey = String;

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub total_size: u64,
    pub entry_count: u64,
}

/// 缓存trait - 定义统一缓存接口
#[async_trait::async_trait]
pub trait Cache<K, V>: Send + Sync {
    /// 获取缓存值
    async fn get(&self, key: &K) -> Option<V>;

    /// 插入缓存值
    async fn insert(&self, key: K, value: V);

    /// 使缓存失效
    async fn invalidate(&self, key: &K);

    /// 清空所有缓存
    async fn clear(&self);

    /// 获取缓存统计信息
    async fn stats(&self) -> CacheStats;

    /// 获取缓存大小
    async fn size(&self) -> usize;
}
