// 目录缓存系统
//
// 缓存目录内容以提高 readdir 操作性能
// 使用 LRU 淘汰策略和 TTL 超时机制

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tracing::{debug, trace};

/// 目录条目
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// inode 编号
    pub inode: u64,

    /// 文件名
    pub name: String,

    /// 是否是目录
    pub is_dir: bool,
}

impl DirEntry {
    /// 创建新的目录条目
    pub fn new(inode: u64, name: String, is_dir: bool) -> Self {
        Self {
            inode,
            name,
            is_dir,
        }
    }
}

/// 缓存条目
#[derive(Debug, Clone)]
struct CacheEntry {
    /// 目录条目列表
    entries: Vec<DirEntry>,

    /// 创建时间
    created_at: Instant,

    /// 上次访问时间
    accessed_at: Instant,
}

impl CacheEntry {
    /// 创建新的缓存条目
    fn new(entries: Vec<DirEntry>) -> Self {
        let now = Instant::now();
        Self {
            entries,
            created_at: now,
            accessed_at: now,
        }
    }

    /// 检查是否过期
    fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() > ttl
    }

    /// 更新访问时间
    fn touch(&mut self) {
        self.accessed_at = Instant::now();
    }
}

/// LRU 节点（用于实现 LRU 淘汰）
#[derive(Debug)]
struct LruNode {
    /// 路径
    path: String,

    /// 前驱节点
    prev: Option<String>,

    /// 后继节点
    next: Option<String>,
}

/// 目录缓存
///
/// 提供：
/// - 目录内容缓存
/// - TTL 超时机制
/// - LRU 淘汰策略
pub struct DirCache {
    /// 缓存数据
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,

    /// LRU 链表头
    lru_head: Arc<RwLock<Option<String>>>,

    /// LRU 链表尾
    lru_tail: Arc<RwLock<Option<String>>>,

    /// LRU 节点映射
    lru_nodes: Arc<RwLock<HashMap<String, LruNode>>>,

    /// 最大缓存条目数
    max_entries: usize,

    /// TTL（秒）
    ttl: Duration,

    /// 当前条目数
    current_size: Arc<RwLock<usize>>,
}

impl DirCache {
    /// 创建新的目录缓存
    ///
    /// # 参数
    /// - `ttl_seconds`: TTL 超时时间（秒）
    ///
    /// # 返回
    /// 新的 DirCache 实例
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            lru_head: Arc::new(RwLock::new(None)),
            lru_tail: Arc::new(RwLock::new(None)),
            lru_nodes: Arc::new(RwLock::new(HashMap::new())),
            max_entries: 10000,
            ttl: Duration::from_secs(ttl_seconds),
            current_size: Arc::new(RwLock::new(0)),
        }
    }

    /// 获取缓存的目录条目
    ///
    /// # 参数
    /// - `path`: 目录路径
    ///
    /// # 返回
    /// Some(条目列表) 如果缓存命中且未过期，否则 None
    pub fn get(&self, path: &str) -> Option<Vec<DirEntry>> {
        trace!("DirCache get: {}", path);

        let mut cache = self.cache.write().unwrap();

        if let Some(entry) = cache.get_mut(path) {
            // 检查是否过期
            if entry.is_expired(self.ttl) {
                debug!("Cache expired: {}", path);
                cache.remove(path);
                self.remove_lru(path);
                return None;
            }

            // 更新访问时间
            entry.touch();
            self.move_to_back_lru(path);

            trace!("Cache hit: {} ({} entries)", path, entry.entries.len());
            Some(entry.entries.clone())
        } else {
            trace!("Cache miss: {}", path);
            None
        }
    }

    /// 放入缓存
    ///
    /// # 参数
    /// - `path`: 目录路径
    /// - `entries`: 目录条目列表
    pub fn put(&self, path: String, entries: Vec<DirEntry>) {
        trace!("DirCache put: {} ({} entries)", path, entries.len());

        // 检查是否需要淘汰
        {
            let size = self.current_size.read().unwrap();
            if *size >= self.max_entries {
                self.evict_one();
            }
        }

        // 插入缓存
        {
            let mut cache = self.cache.write().unwrap();
            let entry = CacheEntry::new(entries);
            cache.insert(path.clone(), entry);
        }

        // 更新 LRU
        self.add_to_back_lru(&path);

        // 更新计数
        {
            let mut size = self.current_size.write().unwrap();
            *size += 1;
        }

        debug!("Cached: {} (now {} entries)", path, *self.current_size.read().unwrap());
    }

    /// 使缓存失效
    ///
    /// # 参数
    /// - `path`: 目录路径
    pub fn invalidate(&self, path: &str) {
        trace!("DirCache invalidate: {}", path);

        {
            let mut cache = self.cache.write().unwrap();
            cache.remove(path);
        }

        self.remove_lru(path);

        // 更新计数
        {
            let mut size = self.current_size.write().unwrap();
            if *size > 0 {
                *size -= 1;
            }
        }

        debug!("Invalidated: {}", path);
    }

    /// 清空所有缓存
    pub fn clear(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();

        let mut lru_head = self.lru_head.write().unwrap();
        *lru_head = None;

        let mut lru_tail = self.lru_tail.write().unwrap();
        *lru_tail = None;

        let mut lru_nodes = self.lru_nodes.write().unwrap();
        lru_nodes.clear();

        let mut size = self.current_size.write().unwrap();
        *size = 0;

        debug!("Cleared all cache");
    }

    /// 获取缓存统计信息
    ///
    /// # 返回
    /// (当前条目数, 最大条目数, TTL 秒数)
    pub fn stats(&self) -> (usize, usize, u64) {
        let size = *self.current_size.read().unwrap();
        (size, self.max_entries, self.ttl.as_secs())
    }

    /// 清理过期的缓存条目
    ///
    /// # 返回
    /// 清理的条目数
    pub fn cleanup_expired(&self) -> usize {
        let mut expired_paths = Vec::new();

        {
            let cache = self.cache.read().unwrap();
            for (path, entry) in cache.iter() {
                if entry.is_expired(self.ttl) {
                    expired_paths.push(path.clone());
                }
            }
        }

        for path in &expired_paths {
            self.invalidate(path);
        }

        if !expired_paths.is_empty() {
            debug!("Cleaned up {} expired entries", expired_paths.len());
        }

        expired_paths.len()
    }

    /// 淘汰一个条目（LRU）
    fn evict_one(&self) {
        let head = {
            let lru_head = self.lru_head.read().unwrap();
            lru_head.clone()
        };

        if let Some(path) = head {
            debug!("Evicting LRU entry: {}", path);
            self.invalidate(&path);
        }
    }

    /// 添加到 LRU 链表尾部
    fn add_to_back_lru(&self, path: &str) {
        let path = path.to_string();

        // 创建新节点
        let node = LruNode {
            path: path.clone(),
            prev: {
                let lru_tail = self.lru_tail.read().unwrap();
                lru_tail.clone()
            },
            next: None,
        };

        {
            let mut lru_nodes = self.lru_nodes.write().unwrap();
            lru_nodes.insert(path.clone(), node);
        }

        // 更新尾节点
        {
            let mut lru_tail = self.lru_tail.write().unwrap();
            if let Some(old_tail) = lru_tail.take() {
                // 更新旧尾节点的后继
                let mut lru_nodes = self.lru_nodes.write().unwrap();
                if let Some(old_node) = lru_nodes.get_mut(&old_tail) {
                    old_node.next = Some(path.clone());
                }
            }
            *lru_tail = Some(path.clone());
        }

        // 如果是空链表，更新头节点
        {
            let lru_head = self.lru_head.read().unwrap();
            if lru_head.is_none() {
                let mut lru_head = self.lru_head.write().unwrap();
                *lru_head = Some(path);
            }
        }
    }

    /// 从 LRU 链表中移除
    fn remove_lru(&self, path: &str) {
        let node_opt = {
            let mut lru_nodes = self.lru_nodes.write().unwrap();
            lru_nodes.remove(path)
        };

        if let Some(node) = node_opt {
            // 更新前驱节点
            if let Some(ref prev_path) = node.prev {
                let mut lru_nodes = self.lru_nodes.write().unwrap();
                if let Some(prev_node) = lru_nodes.get_mut(prev_path.as_str()) {
                    prev_node.next = node.next.clone();
                }
            } else {
                // 是头节点，更新头
                let mut lru_head = self.lru_head.write().unwrap();
                *lru_head = node.next.clone();
            }

            // 更新后继节点
            if let Some(ref next_path) = node.next {
                let mut lru_nodes = self.lru_nodes.write().unwrap();
                if let Some(next_node) = lru_nodes.get_mut(next_path.as_str()) {
                    next_node.prev = node.prev.clone();
                }
            } else {
                // 是尾节点，更新尾
                let mut lru_tail = self.lru_tail.write().unwrap();
                *lru_tail = node.prev;
            }
        }
    }

    /// 将节点移到 LRU 链表尾部
    fn move_to_back_lru(&self, path: &str) {
        self.remove_lru(path);
        self.add_to_back_lru(path);
    }
}

impl Default for DirCache {
    fn default() -> Self {
        Self::new(60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dir_cache_basic() {
        let cache = DirCache::new(10);

        // 测试缓存命中
        cache.put("/dir/".to_string(), vec![
            DirEntry::new(10, "file1.txt".to_string(), false),
            DirEntry::new(11, "file2.txt".to_string(), false),
        ]);

        let entries = cache.get("/dir/");
        assert!(entries.is_some());
        let entries = entries.unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "file1.txt");
    }

    #[test]
    fn test_dir_cache_miss() {
        let cache = DirCache::new(10);

        let entries = cache.get("/nonexistent/");
        assert!(entries.is_none());
    }

    #[test]
    fn test_dir_cache_invalidate() {
        let cache = DirCache::new(10);

        cache.put("/dir/".to_string(), vec![
            DirEntry::new(10, "file.txt".to_string(), false),
        ]);

        cache.invalidate("/dir/");

        let entries = cache.get("/dir/");
        assert!(entries.is_none());
    }

    #[test]
    fn test_dir_cache_clear() {
        let cache = DirCache::new(10);

        cache.put("/dir1/".to_string(), vec![
            DirEntry::new(10, "file1.txt".to_string(), false),
        ]);

        cache.put("/dir2/".to_string(), vec![
            DirEntry::new(11, "file2.txt".to_string(), false),
        ]);

        cache.clear();

        assert!(cache.get("/dir1/").is_none());
        assert!(cache.get("/dir2/").is_none());
    }

    #[test]
    fn test_dir_cache_stats() {
        let cache = DirCache::new(10);

        cache.put("/dir/".to_string(), vec![
            DirEntry::new(10, "file.txt".to_string(), false),
        ]);

        let (current, max, ttl) = cache.stats();
        assert_eq!(current, 1);
        assert_eq!(max, 10000);
        assert_eq!(ttl, 10);
    }
}
