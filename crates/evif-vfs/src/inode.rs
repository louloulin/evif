// INode - 索引节点管理

use crate::error::VfsResult;
use evif_graph::NodeId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 索引节点 (INode)
///
/// INode 代表文件系统中的文件或目录的元数据
#[derive(Debug, Clone)]
pub struct INode {
    /// 节点 ID
    pub id: NodeId,

    /// INode 编号
    pub ino: u64,

    /// 文件大小
    pub size: u64,

    /// 块数量
    pub blocks: u64,

    /// 权限模式
    pub mode: u32,

    /// 硬链接数
    pub nlink: u64,

    /// 用户 ID
    pub uid: u32,

    /// 组 ID
    pub gid: u32,

    /// 设备 ID
    pub rdev: u64,

    /// 访问时间
    pub atime: u64,

    /// 修改时间
    pub mtime: u64,

    /// 创建时间
    pub ctime: u64,
}

impl Default for INode {
    fn default() -> Self {
        INode {
            id: NodeId::new_v4(),
            ino: 0,
            size: 0,
            blocks: 0,
            mode: 0o644,
            nlink: 1,
            uid: 0,
            gid: 0,
            rdev: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
        }
    }
}

impl INode {
    /// 创建新的 INode
    pub fn new(id: NodeId, ino: u64) -> Self {
        INode {
            id,
            ino,
            ..Default::default()
        }
    }

    /// 是否为目录
    pub fn is_directory(&self) -> bool {
        (self.mode & 0o170000) == 0o040000
    }

    /// 是否为普通文件
    pub fn is_regular_file(&self) -> bool {
        (self.mode & 0o170000) == 0o100000
    }

    /// 是否为符号链接
    pub fn is_symlink(&self) -> bool {
        (self.mode & 0o170000) == 0o120000
    }
}

/// INode 缓存
///
/// 缓存 INode 以提高性能
#[derive(Debug)]
pub struct INodeCache {
    /// INode 映射
    inodes: Arc<RwLock<HashMap<NodeId, INode>>>,

    /// INode 编号映射
    ino_map: Arc<RwLock<HashMap<u64, NodeId>>>,

    /// 下一个可用的 INode 编号
    next_ino: Arc<std::sync::atomic::AtomicU64>,

    /// 最大缓存大小
    max_size: usize,
}

impl INodeCache {
    /// 创建新的 INode 缓存
    pub fn new(max_size: usize) -> Self {
        INodeCache {
            inodes: Arc::new(RwLock::new(HashMap::new())),
            ino_map: Arc::new(RwLock::new(HashMap::new())),
            next_ino: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            max_size,
        }
    }

    /// 获取 INode
    pub async fn get(&self, id: &NodeId) -> Option<INode> {
        self.inodes.read().await.get(id).cloned()
    }

    /// 通过 INode 编号获取
    pub async fn get_by_ino(&self, ino: u64) -> Option<INode> {
        let id = self.ino_map.read().await.get(&ino).copied()?;
        self.get(&id).await
    }

    /// 插入 INode
    pub async fn insert(&self, inode: INode) -> VfsResult<()> {
        // 检查缓存大小
        if self.inodes.read().await.len() >= self.max_size {
            // 简单的驱逐策略：移除第一个
            self.evict_one().await;
        }

        let id = inode.id;
        let ino = inode.ino;

        self.inodes.write().await.insert(id, inode.clone());
        self.ino_map.write().await.insert(ino, id);

        Ok(())
    }

    /// 移除 INode
    pub async fn remove(&self, id: &NodeId) -> Option<INode> {
        let inode = self.inodes.write().await.remove(id)?;

        // 同时移除编号映射
        self.ino_map.write().await.remove(&inode.ino);

        Some(inode)
    }

    /// 分配新的 INode 编号
    pub fn alloc_ino(&self) -> u64 {
        self.next_ino.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// 缓存大小
    pub async fn len(&self) -> usize {
        self.inodes.read().await.len()
    }

    /// 是否为空
    pub async fn is_empty(&self) -> bool {
        self.inodes.read().await.is_empty()
    }

    /// 清空缓存
    pub async fn clear(&self) {
        self.inodes.write().await.clear();
        self.ino_map.write().await.clear();
    }

    /// 驱逐一个 INode
    async fn evict_one(&self) {
        let inodes = self.inodes.read().await;
        if let Some((id, ino)) = inodes.iter().next().map(|(id, inode)| (*id, inode.ino)) {
            drop(inodes);
            self.inodes.write().await.remove(&id);
            self.ino_map.write().await.remove(&ino);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_inode_default() {
        let inode = INode::default();
        assert_eq!(inode.nlink, 1);
        assert_eq!(inode.mode, 0o644);
    }

    #[test]
    fn test_inode_checks() {
        let mut inode = INode::default();

        inode.mode = 0o100000; // 普通文件
        assert!(inode.is_regular_file());
        assert!(!inode.is_directory());

        inode.mode = 0o040000; // 目录
        assert!(inode.is_directory());
        assert!(!inode.is_regular_file());

        inode.mode = 0o120000; // 符号链接
        assert!(inode.is_symlink());
    }

    #[tokio::test]
    async fn test_inode_cache() {
        let cache = INodeCache::new(10);

        let id = NodeId::new_v4();
        let inode = INode::new(id, 1);

        cache.insert(inode.clone()).await.unwrap();

        let retrieved = cache.get(&id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().ino, 1);
    }

    #[tokio::test]
    async fn test_inode_cache_by_ino() {
        let cache = INodeCache::new(10);

        let id = NodeId::new_v4();
        let inode = INode::new(id, 42);

        cache.insert(inode).await.unwrap();

        let retrieved = cache.get_by_ino(42).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, id);
    }

    #[tokio::test]
    async fn test_inode_alloc_ino() {
        let cache = INodeCache::new(10);

        let ino1 = cache.alloc_ino();
        let ino2 = cache.alloc_ino();

        assert_eq!(ino1, 1);
        assert_eq!(ino2, 2);
    }

    #[tokio::test]
    async fn test_inode_cache_remove() {
        let cache = INodeCache::new(10);

        let id = NodeId::new_v4();
        let inode = INode::new(id, 1);

        cache.insert(inode.clone()).await.unwrap();
        let removed = cache.remove(&id).await;

        assert!(removed.is_some());
        assert_eq!(removed.unwrap().ino, 1);
        assert!(cache.get(&id).await.is_none());
    }

    #[tokio::test]
    async fn test_inode_cache_clear() {
        let cache = INodeCache::new(10);

        let id1 = NodeId::new_v4();
        let id2 = NodeId::new_v4();

        cache.insert(INode::new(id1, 1)).await.unwrap();
        cache.insert(INode::new(id2, 2)).await.unwrap();

        assert_eq!(cache.len().await, 2);

        cache.clear().await;

        assert_eq!(cache.len().await, 0);
        assert!(cache.is_empty().await);
    }
}
