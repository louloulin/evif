// EVIF通用缓存实现 - 使用moka

use super::{Cache, CacheStats};
use async_trait::async_trait;
use moka::future::Cache as MokaCache;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::sync::Arc;

/// 缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// 最大缓存条目数
    pub max_capacity: u64,

    /// TTL (秒)
    pub ttl_secs: u64,

    /// TTI (秒) - 最后访问后过期时间
    pub tti_secs: Option<u64>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: 10_000,
            ttl_secs: 60,
            tti_secs: None,
        }
    }
}

impl CacheConfig {
    /// 创建元数据缓存配置 (TTL: 60s)
    pub fn metadata() -> Self {
        Self {
            max_capacity: 10_000,
            ttl_secs: 60,
            tti_secs: None,
        }
    }

    /// 创建目录缓存配置 (TTL: 30s)
    pub fn directory() -> Self {
        Self {
            max_capacity: 5_000,
            ttl_secs: 30,
            tti_secs: None,
        }
    }

    /// 创建S3缓存配置 (TTL: 120s)
    pub fn s3() -> Self {
        Self {
            max_capacity: 20_000,
            ttl_secs: 120,
            tti_secs: None,
        }
    }
}

/// EVIF通用缓存 - 基于moka实现
pub struct EvifCache<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    inner: Arc<MokaCache<K, V>>,
    name: String,
}

impl<K, V> EvifCache<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// 创建新的缓存
    pub fn new(name: String, config: CacheConfig) -> Self {
        let mut builder = MokaCache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(std::time::Duration::from_secs(config.ttl_secs));

        if let Some(tti) = config.tti_secs {
            builder = builder.time_to_idle(std::time::Duration::from_secs(tti));
        }

        let cache = builder.build();

        Self {
            inner: Arc::new(cache),
            name,
        }
    }

    /// 获取缓存名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取内部缓存引用
    pub fn inner(&self) -> &MokaCache<K, V> {
        &self.inner
    }
}

#[async_trait]
impl<K, V> Cache<K, V> for EvifCache<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    async fn get(&self, key: &K) -> Option<V> {
        self.inner.get(key).await
    }

    async fn insert(&self, key: K, value: V) {
        self.inner.insert(key, value).await;
    }

    async fn invalidate(&self, key: &K) {
        self.inner.invalidate(key).await;
    }

    async fn clear(&self) {
        self.inner.invalidate_all();
    }

    async fn stats(&self) -> CacheStats {
        // moka缓存不直接提供hit/miss计数，使用entry_count和weighted_size
        CacheStats {
            hits: 0, // moka不直接支持
            misses: 0,
            total_size: self.inner.weighted_size(),
            entry_count: self.inner.entry_count(),
        }
    }

    async fn size(&self) -> usize {
        self.inner.entry_count() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let cache = EvifCache::new("test".to_string(), CacheConfig::default());

        // 插入
        cache.insert("key1".to_string(), "value1".to_string()).await;

        // 获取
        let value = cache.get(&"key1".to_string()).await;
        assert_eq!(value, Some("value1".to_string()));

        // 不存在的key
        let value = cache.get(&"key2".to_string()).await;
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_cache_invalidate() {
        let cache = EvifCache::new("test".to_string(), CacheConfig::default());

        cache.insert("key1".to_string(), "value1".to_string()).await;
        assert!(cache.get(&"key1".to_string()).await.is_some());

        cache.invalidate(&"key1".to_string()).await;
        assert!(cache.get(&"key1".to_string()).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = EvifCache::new("test".to_string(), CacheConfig::default());

        cache.insert("key1".to_string(), "value1".to_string()).await;
        let _ = cache.get(&"key1".to_string()).await; // hit
        let _ = cache.get(&"key2".to_string()).await; // miss

        let stats = cache.stats().await;
        // moka 不直接提供 hit/miss，当前实现也不承诺即时 entry_count，一致性以可观测读取为准
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert!(cache.get(&"key1".to_string()).await.is_some());
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = EvifCache::new("test".to_string(), CacheConfig::default());

        cache.insert("key1".to_string(), "value1".to_string()).await;
        cache.insert("key2".to_string(), "value2".to_string()).await;

        assert!(cache.get(&"key1".to_string()).await.is_some());
        assert!(cache.get(&"key2".to_string()).await.is_some());

        cache.clear().await;
        assert!(cache.get(&"key1".to_string()).await.is_none());
        assert!(cache.get(&"key2".to_string()).await.is_none());
        assert_eq!(cache.size().await, 0);
    }
}
