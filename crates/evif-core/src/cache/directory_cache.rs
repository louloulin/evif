// 目录缓存 - 用于缓存目录列表

use super::{Cache, CacheConfig, CacheKey, CacheStats};
use crate::FileInfo;
use async_trait::async_trait;
use std::sync::Arc;

/// 目录缓存
pub struct DirectoryCache {
    cache: Arc<dyn Cache<CacheKey, Vec<FileInfo>> + Send + Sync>,
}

impl DirectoryCache {
    /// 创建新的目录缓存
    pub fn new() -> Self {
        Self::with_config(CacheConfig::directory())
    }

    /// 使用配置创建目录缓存
    pub fn with_config(config: CacheConfig) -> Self {
        use super::cache::EvifCache;
        let cache = EvifCache::<CacheKey, Vec<FileInfo>>::new("directory".to_string(), config);
        Self {
            cache: Arc::new(cache),
        }
    }

    /// 获取目录列表
    pub async fn get(&self, path: &str) -> Option<Vec<FileInfo>> {
        self.cache.get(&path.to_string()).await
    }

    /// 插入目录列表
    pub async fn set(&self, path: String, files: Vec<FileInfo>) {
        self.cache.insert(path, files).await;
    }

    /// 使目录缓存失效
    pub async fn invalidate(&self, path: &str) {
        self.cache.invalidate(&path.to_string()).await;
    }

    /// 批量使目录缓存失效（路径前缀匹配）
    pub async fn invalidate_prefix(&self, prefix: &str) {
        // 实现前缀匹配失效逻辑
        // 由于缓存层不直接支持前缀查询,这里我们清空整个缓存
        // 在实际生产中,应该在缓存层维护一个路径前缀索引
        self.cache.clear().await;
    }

    /// 使父目录缓存失效（用于文件修改后）
    pub async fn invalidate_parent(&self, path: &str) {
        if let Some(parent) = std::path::Path::new(path).parent() {
            let parent_path = parent.to_string_lossy().to_string();
            if !parent_path.is_empty() {
                self.invalidate(&parent_path).await;
            }
        }
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
    async fn test_directory_cache() {
        let cache = DirectoryCache::new();

        let files = vec![
            FileInfo {
                name: "file1.txt".to_string(),
                size: 1024,
                mode: 0o644,
                modified: Utc::now(),
                is_dir: false,
            },
            FileInfo {
                name: "file2.txt".to_string(),
                size: 2048,
                mode: 0o644,
                modified: Utc::now(),
                is_dir: false,
            },
        ];

        // 设置缓存
        cache.set("/dir".to_string(), files.clone()).await;

        // 获取缓存
        let cached = cache.get("/dir").await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 2);

        // 失效
        cache.invalidate("/dir").await;
        assert!(cache.get("/dir").await.is_none());
    }

    #[tokio::test]
    async fn test_directory_cache_invalidate_parent() {
        let cache = DirectoryCache::new();

        let files = vec![FileInfo {
            name: "file1.txt".to_string(),
            size: 1024,
            mode: 0o644,
            modified: Utc::now(),
            is_dir: false,
        }];

        cache.set("/dir".to_string(), files).await;

        // 使父目录失效
        cache.invalidate_parent("/dir/file1.txt").await;

        assert!(cache.get("/dir").await.is_none());
    }
}
