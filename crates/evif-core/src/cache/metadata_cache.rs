// 元数据缓存 - 用于缓存文件/目录的元数据

use super::{Cache, CacheConfig, CacheKey, CacheStats};
use async_trait::async_trait;
use crate::FileInfo;
use std::sync::Arc;

/// 元数据缓存
pub struct MetadataCache {
    cache: Arc<dyn Cache<CacheKey, FileInfo> + Send + Sync>,
}

impl MetadataCache {
    /// 创建新的元数据缓存
    pub fn new() -> Self {
        Self::with_config(CacheConfig::metadata())
    }

    /// 使用配置创建元数据缓存
    pub fn with_config(config: CacheConfig) -> Self {
        use super::cache::EvifCache;
        let cache = EvifCache::<CacheKey, FileInfo>::new("metadata".to_string(), config);
        Self { cache: Arc::new(cache) }
    }

    /// 获取文件元数据
    pub async fn get(&self, path: &str) -> Option<FileInfo> {
        self.cache.get(&path.to_string()).await
    }

    /// 插入文件元数据
    pub async fn set(&self, path: String, info: FileInfo) {
        self.cache.insert(path, info).await;
    }

    /// 使元数据失效
    pub async fn invalidate(&self, path: &str) {
        self.cache.invalidate(&path.to_string()).await;
    }

    /// 批量使元数据失效（路径前缀匹配）
    pub async fn invalidate_prefix(&self, prefix: &str) {
        // 实现前缀匹配失效逻辑
        // 由于缓存层不直接支持前缀查询,这里我们清空整个缓存
        // 在实际生产中,应该在缓存层维护一个路径前缀索引
        self.cache.clear().await;
    }

    /// 获取缓存统计信息
    pub async fn stats(&self) -> CacheStats {
        self.cache.stats().await
    }

    /// 获取缓存大小
    pub async fn size(&self) -> usize {
        self.cache.size().await
    }

    /// 清空所有缓存
    pub async fn clear(&self) {
        self.cache.clear().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_metadata_cache() {
        let cache = MetadataCache::new();

        let info = FileInfo {
            name: "test.txt".to_string(),
            size: 1024,
            mode: 0o644,
            modified: Utc::now(),
            is_dir: false,
        };

        // 设置缓存
        cache.set("/test.txt".to_string(), info.clone()).await;

        // 获取缓存
        let cached = cache.get("/test.txt").await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().name, "test.txt");

        // 失效
        cache.invalidate("/test.txt").await;
        assert!(cache.get("/test.txt").await.is_none());
    }

    #[tokio::test]
    async fn test_metadata_cache_stats() {
        let cache = MetadataCache::new();

        let info = FileInfo {
            name: "test.txt".to_string(),
            size: 1024,
            mode: 0o644,
            modified: Utc::now(),
            is_dir: false,
        };

        cache.set("/test.txt".to_string(), info).await;

        let stats = cache.stats().await;
        assert!(stats.entry_count >= 0);
        assert!(cache.get("/test.txt").await.is_some());
    }
}
