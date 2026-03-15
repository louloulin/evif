// Inode 管理器
//
// 管理 FUSE 文件系统的 inode 映射
// 提供：
// - 路径到 inode 的双向映射
// - inode 分配和回收
// - 线程安全的并发访问

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::sync::Mutex;
use tracing::{debug, trace};

/// Inode 类型别名
pub type Inode = u64;

/// 预定义的特殊 inode
pub const ROOT_INODE: Inode = 1;
pub const PARENT_INODE: Inode = 2;

/// Inode 信息
#[derive(Debug, Clone)]
pub struct InodeInfo {
    /// inode 编号
    pub inode: Inode,

    /// 路径
    pub path: String,

    /// 是否是目录
    pub is_dir: bool,

    /// 引用计数
    pub ref_count: u32,
}

impl InodeInfo {
    /// 创建新的 inode 信息
    pub fn new(inode: Inode, path: String, is_dir: bool) -> Self {
        Self {
            inode,
            path,
            is_dir,
            ref_count: 1,
        }
    }
}

/// Inode 管理器
///
/// 维护路径和 inode 之间的双向映射
pub struct InodeManager {
    /// 下一个可用的 inode 编号
    next_inode: Arc<Mutex<Inode>>,

    /// 路径到 inode 的映射
    path_to_inode: Arc<RwLock<HashMap<String, Inode>>>,

    /// inode 到信息的映射
    inode_to_info: Arc<RwLock<HashMap<Inode, InodeInfo>>>,
}

impl InodeManager {
    /// 创建新的 inode 管理器
    ///
    /// # 参数
    /// - `_cache_size`: 缓存大小（预留参数）
    ///
    /// # 返回
    /// 新的 InodeManager 实例
    pub fn new(_cache_size: usize) -> Self {
        let mut path_to_inode = HashMap::new();
        let mut inode_to_info = HashMap::new();

        // 初始化根目录
        path_to_inode.insert("/".to_string(), ROOT_INODE);
        inode_to_info.insert(
            ROOT_INODE,
            InodeInfo::new(ROOT_INODE, "/".to_string(), true),
        );

        Self {
            next_inode: Arc::new(Mutex::new(3)), // 从 3 开始分配
            path_to_inode: Arc::new(RwLock::new(path_to_inode)),
            inode_to_info: Arc::new(RwLock::new(inode_to_info)),
        }
    }

    /// 获取或创建 inode
    ///
    /// 如果路径已存在，返回对应的 inode
    /// 否则分配新的 inode 并创建映射
    ///
    /// # 参数
    /// - `path`: 文件路径
    ///
    /// # 返回
    /// inode 编号
    pub fn get_or_create(&self, path: &str) -> Inode {
        trace!("get_or_create: {}", path);

        // 检查是否已存在
        {
            let path_map = self.path_to_inode.read().unwrap();
            if let Some(&inode) = path_map.get(path) {
                trace!("Path exists: {} -> {}", path, inode);
                return inode;
            }
        }

        // 分配新的 inode
        let inode = self.allocate_inode();
        let is_dir = path.ends_with('/');

        // 更新映射
        {
            let mut path_map = self.path_to_inode.write().unwrap();
            path_map.insert(path.to_string(), inode);
        }

        {
            let mut info_map = self.inode_to_info.write().unwrap();
            info_map.insert(inode, InodeInfo::new(inode, path.to_string(), is_dir));
        }

        debug!("Created new inode: {} -> {}", inode, path);
        inode
    }

    /// 根据 inode 获取路径
    ///
    /// # 参数
    /// - `inode`: inode 编号
    ///
    /// # 返回
    /// Some(路径) 如果 inode 存在，否则 None
    pub fn get_path(&self, inode: Inode) -> Option<String> {
        let info_map = self.inode_to_info.read().unwrap();
        info_map.get(&inode).map(|info| info.path.clone())
    }

    /// 根据路径获取 inode
    ///
    /// # 参数
    /// - `path`: 文件路径
    ///
    /// # 返回
    /// Some(inode) 如果路径存在，否则 None
    pub fn get_inode(&self, path: &str) -> Option<Inode> {
        let path_map = self.path_to_inode.read().unwrap();
        path_map.get(path).copied()
    }

    /// 获取 inode 信息
    ///
    /// # 参数
    /// - `inode`: inode 编号
    ///
    /// # 返回
    /// Some(InodeInfo) 如果 inode 存在，否则 None
    pub fn get_info(&self, inode: Inode) -> Option<InodeInfo> {
        let info_map = self.inode_to_info.read().unwrap();
        info_map.get(&inode).cloned()
    }

    /// 增加 inode 引用计数
    ///
    /// # 参数
    /// - `inode`: inode 编号
    pub fn incref(&self, inode: Inode) {
        let mut info_map = self.inode_to_info.write().unwrap();
        if let Some(info) = info_map.get_mut(&inode) {
            info.ref_count += 1;
            trace!("Incref inode {} -> ref_count: {}", inode, info.ref_count);
        }
    }

    /// 减少 inode 引用计数
    ///
    /// # 参数
    /// - `inode`: inode 编号
    ///
    /// # 返回
    /// true 如果引用计数归零，false 否则
    pub fn decref(&self, inode: Inode) -> bool {
        let mut info_map = self.inode_to_info.write().unwrap();
        if let Some(info) = info_map.get_mut(&inode) {
            if info.ref_count > 0 {
                info.ref_count -= 1;
            }
            trace!("Decref inode {} -> ref_count: {}", inode, info.ref_count);
            info.ref_count == 0
        } else {
            false
        }
    }

    /// 回收 inode
    ///
    /// # 参数
    /// - `inode`: inode 编号
    pub fn recycle(&self, inode: Inode) {
        let info = {
            let info_map = self.inode_to_info.read().unwrap();
            info_map.get(&inode).cloned()
        };

        if let Some(info) = info {
            // 从路径映射中删除
            {
                let mut path_map = self.path_to_inode.write().unwrap();
                path_map.remove(&info.path);
            }

            // 从信息映射中删除
            {
                let mut info_map = self.inode_to_info.write().unwrap();
                info_map.remove(&inode);
            }

            debug!("Recycled inode {} (path: {})", inode, info.path);
        }
    }

    /// 分配新的 inode
    ///
    /// # 返回
    /// 新分配的 inode 编号
    fn allocate_inode(&self) -> Inode {
        let mut next = self.next_inode.blocking_lock();
        let inode = *next;
        *next += 1;
        inode
    }

    /// 统计信息
    ///
    /// # 返回
    /// (总 inode 数, 总路径数)
    pub fn stats(&self) -> (usize, usize) {
        let info_map = self.inode_to_info.read().unwrap();
        let path_map = self.path_to_inode.read().unwrap();
        (info_map.len(), path_map.len())
    }
}

impl Default for InodeManager {
    fn default() -> Self {
        Self::new(10000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inode_manager_basic() {
        let manager = InodeManager::new(100);

        // 测试根目录
        assert_eq!(manager.get_inode("/"), Some(ROOT_INODE));
        assert_eq!(manager.get_path(ROOT_INODE), Some("/".to_string()));

        // 测试创建新 inode
        let inode1 = manager.get_or_create("/test.txt");
        let inode2 = manager.get_or_create("/dir/");

        assert!(inode1 > ROOT_INODE);
        assert!(inode2 > ROOT_INODE);
        assert_ne!(inode1, inode2);

        // 测试路径到 inode 映射
        assert_eq!(manager.get_inode("/test.txt"), Some(inode1));
        assert_eq!(manager.get_path(inode1), Some("/test.txt".to_string()));
    }

    #[test]
    fn test_inode_manager_duplicate() {
        let manager = InodeManager::new(100);

        // 创建相同的路径应该返回相同的 inode
        let inode1 = manager.get_or_create("/file.txt");
        let inode2 = manager.get_or_create("/file.txt");

        assert_eq!(inode1, inode2);
    }

    #[test]
    fn test_inode_refcount() {
        let manager = InodeManager::new(100);

        let inode = manager.get_or_create("/ref.txt");

        manager.incref(inode);
        manager.incref(inode);

        // 减少引用计数
        assert!(!manager.decref(inode));
        assert!(!manager.decref(inode));
        assert!(manager.decref(inode)); // 引用计数归零
    }

    #[test]
    fn test_inode_recycle() {
        let manager = InodeManager::new(100);

        let inode = manager.get_or_create("/to_delete.txt");
        assert_eq!(manager.get_path(inode), Some("/to_delete.txt".to_string()));

        manager.recycle(inode);
        assert_eq!(manager.get_path(inode), None);
        assert_eq!(manager.get_inode("/to_delete.txt"), None);
    }
}
