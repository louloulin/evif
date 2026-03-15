// DEntry - 目录项管理

use crate::error::VfsResult;
use evif_graph::NodeId;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 目录项 (DEntry)
///
/// DEntry 表示目录中的一个条目，缓存路径到节点的映射
#[derive(Debug, Clone)]
pub struct DEntry {
    /// 名称
    pub name: String,

    /// 节点 ID
    pub inode: NodeId,

    /// 父目录项
    pub parent: Option<NodeId>,

    /// 是否为目录
    pub is_directory: bool,

    /// 引用计数
    ref_count: Arc<std::sync::atomic::AtomicUsize>,
}

impl DEntry {
    /// 创建新的目录项
    pub fn new(name: impl Into<String>, inode: NodeId, is_directory: bool) -> Self {
        DEntry {
            name: name.into(),
            inode,
            parent: None,
            is_directory,
            ref_count: Arc::new(std::sync::atomic::AtomicUsize::new(1)),
        }
    }

    /// 设置父目录
    pub fn with_parent(mut self, parent: NodeId) -> Self {
        self.parent = Some(parent);
        self
    }

    /// 获取名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取节点 ID
    pub fn inode(&self) -> NodeId {
        self.inode
    }

    /// 增加引用计数
    pub fn inc_ref(&self) {
        self.ref_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// 减少引用计数
    pub fn dec_ref(&self) -> usize {
        self.ref_count
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed)
            .saturating_sub(1)
    }

    /// 获取引用计数
    pub fn ref_count(&self) -> usize {
        self.ref_count.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// DEntry 缓存
///
/// 缓存目录项以提高路径解析性能
#[derive(Debug)]
pub struct DEntryCache {
    /// 路径到目录项的映射
    entries: Arc<RwLock<HashMap<PathBuf, DEntry>>>,

    /// 节点到路径的反向映射
    reverse_map: Arc<RwLock<HashMap<NodeId, PathBuf>>>,

    /// 最大缓存大小
    max_size: usize,
}

impl DEntryCache {
    /// 创建新的目录项缓存
    pub fn new(max_size: usize) -> Self {
        DEntryCache {
            entries: Arc::new(RwLock::new(HashMap::new())),
            reverse_map: Arc::new(RwLock::new(HashMap::new())),
            max_size,
        }
    }

    /// 获取目录项
    pub async fn get(&self, path: &PathBuf) -> Option<DEntry> {
        self.entries.read().await.get(path).cloned()
    }

    /// 通过节点 ID 获取路径
    pub async fn get_path(&self, inode: &NodeId) -> Option<PathBuf> {
        self.reverse_map.read().await.get(inode).cloned()
    }

    /// 插入目录项
    pub async fn insert(&self, path: PathBuf, entry: DEntry) -> VfsResult<()> {
        // 检查缓存大小
        if self.entries.read().await.len() >= self.max_size {
            self.evict_one().await;
        }

        let inode = entry.inode;

        self.entries.write().await.insert(path.clone(), entry);
        self.reverse_map.write().await.insert(inode, path);

        Ok(())
    }

    /// 移除目录项
    pub async fn remove(&self, path: &PathBuf) -> Option<DEntry> {
        let entry = self.entries.write().await.remove(path)?;

        // 同时移除反向映射
        self.reverse_map.write().await.remove(&entry.inode);

        Some(entry)
    }

    /// 更新目录项
    pub async fn update<F>(&self, path: &PathBuf, f: F) -> VfsResult<()>
    where
        F: FnOnce(&mut DEntry),
    {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(path) {
            f(entry);
        }
        Ok(())
    }

    /// 缓存大小
    pub async fn len(&self) -> usize {
        self.entries.read().await.len()
    }

    /// 是否为空
    pub async fn is_empty(&self) -> bool {
        self.entries.read().await.is_empty()
    }

    /// 清空缓存
    pub async fn clear(&self) {
        self.entries.write().await.clear();
        self.reverse_map.write().await.clear();
    }

    /// 驱逐一个目录项
    async fn evict_one(&self) {
        let entries = self.entries.read().await;
        if let Some((path, entry)) = entries.iter().next() {
            let path = path.clone();
            let inode = entry.inode;
            drop(entries);
            self.entries.write().await.remove(&path);
            self.reverse_map.write().await.remove(&inode);
        }
    }

    /// 获取子目录项
    pub async fn children(&self, parent_path: &PathBuf) -> Vec<DEntry> {
        self.entries
            .read()
            .await
            .iter()
            .filter(|(path, _)| {
                path.parent()
                    .map(|p| p == parent_path.as_path())
                    .unwrap_or(false)
            })
            .map(|(_, entry)| entry.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dentry_creation() {
        let inode = NodeId::new_v4();
        let dentry = DEntry::new("test.txt", inode, false);

        assert_eq!(dentry.name(), "test.txt");
        assert_eq!(dentry.inode(), inode);
        assert!(!dentry.is_directory);
        assert_eq!(dentry.ref_count(), 1);
    }

    #[tokio::test]
    async fn test_dentry_parent() {
        let inode = NodeId::new_v4();
        let parent = NodeId::new_v4();

        let dentry = DEntry::new("test.txt", inode, false).with_parent(parent);

        assert_eq!(dentry.parent, Some(parent));
    }

    #[tokio::test]
    async fn test_dentry_ref_count() {
        let inode = NodeId::new_v4();
        let dentry = DEntry::new("test.txt", inode, false);

        dentry.inc_ref();
        assert_eq!(dentry.ref_count(), 2);

        dentry.dec_ref();
        assert_eq!(dentry.ref_count(), 1);
    }

    #[tokio::test]
    async fn test_dentry_cache() {
        let cache = DEntryCache::new(10);

        let path = PathBuf::from("/test.txt");
        let inode = NodeId::new_v4();
        let dentry = DEntry::new("test.txt", inode, false);

        cache.insert(path.clone(), dentry.clone()).await.unwrap();

        let retrieved = cache.get(&path).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "test.txt");
    }

    #[tokio::test]
    async fn test_dentry_cache_reverse() {
        let cache = DEntryCache::new(10);

        let path = PathBuf::from("/test.txt");
        let inode = NodeId::new_v4();
        let dentry = DEntry::new("test.txt", inode, false);

        cache.insert(path.clone(), dentry).await.unwrap();

        let retrieved_path = cache.get_path(&inode).await;
        assert_eq!(retrieved_path, Some(path));
    }

    #[tokio::test]
    async fn test_dentry_cache_remove() {
        let cache = DEntryCache::new(10);

        let path = PathBuf::from("/test.txt");
        let inode = NodeId::new_v4();
        let dentry = DEntry::new("test.txt", inode, false);

        cache.insert(path.clone(), dentry).await.unwrap();
        let removed = cache.remove(&path).await;

        assert!(removed.is_some());
        assert!(cache.get(&path).await.is_none());
        assert!(cache.get_path(&inode).await.is_none());
    }

    #[tokio::test]
    async fn test_dentry_cache_children() {
        let cache = DEntryCache::new(10);

        let parent = NodeId::new_v4();
        let child1 = NodeId::new_v4();
        let child2 = NodeId::new_v4();

        let dir_path = PathBuf::from("/dir");
        let file1_path = PathBuf::from("/dir/file1.txt");
        let file2_path = PathBuf::from("/dir/file2.txt");

        cache
            .insert(dir_path.clone(), DEntry::new("dir", parent, true))
            .await
            .unwrap();
        cache
            .insert(
                file1_path.clone(),
                DEntry::new("file1.txt", child1, false).with_parent(parent),
            )
            .await
            .unwrap();
        cache
            .insert(
                file2_path.clone(),
                DEntry::new("file2.txt", child2, false).with_parent(parent),
            )
            .await
            .unwrap();

        let children = cache.children(&dir_path).await;
        assert_eq!(children.len(), 2);
    }
}
