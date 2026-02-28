// EVIF Cache System Tests

use super::*;
use tokio::time::{sleep, Duration};
use crate::{MountTable, RadixMountTable};

#[cfg(test)]
mod cache_tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let config = CacheConfig {
            max_capacity: 100,
            ttl_secs: 60,
            tti_secs: None,
        };

        let cache = EvifCache::<String, String>::new("test_cache".to_string(), config);

        // Test set and get
        cache.set("key1".to_string(), "value1".to_string()).await;
        let value = cache.get(&"key1".to_string()).await;
        assert_eq!(value, Some("value1".to_string()));

        // Test non-existent key
        let missing = cache.get(&"key2".to_string()).await;
        assert_eq!(missing, None);
    }

    #[tokio::test]
    async fn test_cache_ttl() {
        let config = CacheConfig {
            max_capacity: 100,
            ttl_secs: 1, // 1 second TTL
            tti_secs: None,
        };

        let cache = EvifCache::<String, String>::new("ttl_cache".to_string(), config);

        cache.set("key1".to_string(), "value1".to_string()).await;

        // Value should exist immediately
        let value = cache.get(&"key1".to_string()).await;
        assert_eq!(value, Some("value1".to_string()));

        // Wait for TTL to expire
        sleep(Duration::from_secs(2)).await;

        // Value should be gone after TTL
        let expired = cache.get(&"key1".to_string()).await;
        assert_eq!(expired, None);
    }

    #[tokio::test]
    async fn test_cache_remove() {
        let config = CacheConfig {
            max_capacity: 100,
            ttl_secs: 60,
            tti_secs: None,
        };

        let cache = EvifCache::<String, String>::new("remove_cache".to_string(), config);

        cache.set("key1".to_string(), "value1".to_string()).await;
        cache.set("key2".to_string(), "value2".to_string()).await;

        // Remove one key
        cache.remove(&"key1".to_string()).await;

        assert_eq!(cache.get(&"key1".to_string()).await, None);
        assert_eq!(cache.get(&"key2".to_string()).await, Some("value2".to_string()));
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let config = CacheConfig {
            max_capacity: 100,
            ttl_secs: 60,
            tti_secs: None,
        };

        let cache = EvifCache::<String, String>::new("clear_cache".to_string(), config);

        cache.set("key1".to_string(), "value1".to_string()).await;
        cache.set("key2".to_string(), "value2".to_string()).await;

        // Clear all
        cache.clear().await;

        assert_eq!(cache.get(&"key1".to_string()).await, None);
        assert_eq!(cache.get(&"key2".to_string()).await, None);

        let stats = cache.stats().await;
        assert_eq!(stats.entry_count, 0);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let config = CacheConfig {
            max_capacity: 100,
            ttl_secs: 60,
            tti_secs: None,
        };

        let cache = EvifCache::<String, String>::new("stats_cache".to_string(), config);

        cache.set("key1".to_string(), "value1".to_string()).await;
        cache.set("key2".to_string(), "value2".to_string()).await;

        let stats = cache.stats().await;
        assert_eq!(stats.entry_count, 2);
        assert_eq!(stats.name, "stats_cache");
    }
}

#[cfg(test)]
mod metadata_cache_tests {
    use super::*;
    use crate::FileInfo;

    #[tokio::test]
    async fn test_metadata_cache() {
        let cache = MetadataCache::new(60);

        let file_info = FileInfo {
            name: "test.txt".to_string(),
            path: "/test/test.txt".to_string(),
            size: 1024,
            modified: 0,
            is_dir: false,
            file_type: "file".to_string(),
        };

        // Cache and retrieve
        cache.set("/test/test.txt".to_string(), file_info.clone()).await;
        let retrieved = cache.get("/test/test.txt").await;

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test.txt");
    }
}

#[cfg(test)]
mod directory_cache_tests {
    use super::*;
    use crate::FileInfo;

    #[tokio::test]
    async fn test_directory_cache() {
        let cache = DirectoryCache::new(30);

        let files = vec![
            FileInfo {
                name: "file1.txt".to_string(),
                path: "/dir/file1.txt".to_string(),
                size: 100,
                modified: 0,
                is_dir: false,
                file_type: "file".to_string(),
            },
            FileInfo {
                name: "file2.txt".to_string(),
                path: "/dir/file2.txt".to_string(),
                size: 200,
                modified: 0,
                is_dir: false,
                file_type: "file".to_string(),
            },
        ];

        // Cache and retrieve
        cache.set("/dir".to_string(), files.clone()).await;
        let retrieved = cache.get("/dir").await;

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().len(), 2);
    }
}
