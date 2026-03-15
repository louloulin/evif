// MountTable - 插件挂载表
//
// 使用简单的路径前缀匹配实现插件路由
// 对标 AGFS MountableFS
//
// 增强功能:
// - 虚拟符号链接支持（无需后端支持）
// - 递归符号链接解析
// - 循环检测

use crate::error::{EvifError, EvifResult};
use crate::plugin::EvifPlugin;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 最大符号链接递归深度（防止循环）
const MAX_SYMLINK_DEPTH: usize = 40;

/// 插件挂载表
///
/// 使用最长前缀匹配路由文件操作到对应的插件
pub struct MountTable {
    mounts: Arc<RwLock<HashMap<String, Arc<dyn EvifPlugin>>>>,
    /// 虚拟符号链接映射表: link_path -> target_path
    /// 允许符号链接跨所有文件系统工作,无需后端支持
    symlinks: Arc<RwLock<HashMap<String, String>>>,
}

impl MountTable {
    /// 创建新的挂载表
    pub fn new() -> Self {
        Self {
            mounts: Arc::new(RwLock::new(HashMap::new())),
            symlinks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建符号链接（虚拟，无需后端支持）
    ///
    /// # 参数
    /// - `target_path`: 链接目标路径
    /// - `link_path`: 符号链接路径
    ///
    /// # AGFS 对标
    /// ```go
    /// func (mfs *MountableFS) Symlink(targetPath, linkPath string) error
    /// ```
    pub async fn symlink(&self, target_path: &str, link_path: &str) -> EvifResult<()> {
        let normalized_target = Self::normalize_path(target_path);
        let normalized_link = Self::normalize_path(link_path);

        let mut symlinks = self.symlinks.write().await;
        symlinks.insert(normalized_link.clone(), normalized_target);
        Ok(())
    }

    /// 读取符号链接目标（非递归）
    ///
    /// # 参数
    /// - `link_path`: 符号链接路径
    ///
    /// # 返回
    /// 链接目标的路径
    ///
    /// # AGFS 对标
    /// ```go
    /// func (mfs *MountableFS) Readlink(path string) (string, bool)
    /// ```
    pub async fn readlink(&self, link_path: &str) -> EvifResult<String> {
        let normalized_link = Self::normalize_path(link_path);

        let symlinks = self.symlinks.read().await;
        match symlinks.get(&normalized_link) {
            Some(target) => Ok(target.clone()),
            None => Err(EvifError::NotFound(format!("Symlink: {}", link_path))),
        }
    }

    /// 解析符号链接（非递归）
    ///
    /// # 参数
    /// - `path`: 可能是符号链接的路径
    ///
    /// # 返回
    /// - (解析后的路径, 是否是符号链接)
    ///
    /// # AGFS 对标
    /// ```go
    /// func (mfs *MountableFS) resolveSymlink(path string) (string, bool)
    /// ```
    pub async fn resolve_symlink(&self, path: &str) -> (String, bool) {
        let normalized_path = Self::normalize_path(path);

        let symlinks = self.symlinks.read().await;
        match symlinks.get(&normalized_path) {
            Some(target) => (target.clone(), true),
            None => (normalized_path, false),
        }
    }

    /// 递归解析符号链接（带循环检测）
    ///
    /// # 参数
    /// - `path`: 可能是符号链接的路径
    /// - `max_depth`: 最大递归深度（防止无限循环）
    ///
    /// # 返回
    /// 完全解析后的路径
    ///
    /// # AGFS 对标
    /// ```go
    /// func (mfs *MountableFS) resolveSymlinkRecursive(path string, maxDepth int) (string, error)
    /// ```
    pub async fn resolve_symlink_recursive(
        &self,
        path: &str,
        max_depth: usize,
    ) -> EvifResult<String> {
        let mut current_path = Self::normalize_path(path);
        let mut visited = std::collections::HashSet::new();

        for _ in 0..max_depth {
            // 检查循环
            if visited.contains(&current_path) {
                return Err(EvifError::InvalidInput(format!(
                    "Symbolic link cycle detected: {}",
                    path
                )));
            }
            visited.insert(current_path.clone());

            // 尝试解析
            let (resolved, is_link) = self.resolve_symlink(&current_path).await;

            if !is_link {
                // 不是符号链接，返回当前路径
                return Ok(current_path);
            }

            // 继续解析
            current_path = resolved;
        }

        Err(EvifError::InvalidInput(format!(
            "Maximum symbolic link depth exceeded: {}",
            max_depth
        )))
    }

    /// 逐组件解析路径（处理路径中间的符号链接）
    ///
    /// # 参数
    /// - `path`: 要解析的路径
    /// - `max_depth`: 每个组件的最大递归深度
    ///
    /// # 返回
    /// 完全解析后的路径
    ///
    /// # AGFS 对标
    /// ```go
    /// func (mfs *MountableFS) resolvePathWithSymlinks(path string, maxDepth int) (string, error)
    /// ```
    pub async fn resolve_path_with_symlinks(
        &self,
        path: &str,
        max_depth: usize,
    ) -> EvifResult<String> {
        let normalized_path = Self::normalize_path(path);

        // 分割路径为组件
        let components: Vec<&str> = normalized_path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        let mut resolved_path = String::new();

        for component in components {
            // 构建当前部分路径
            let partial_path = if resolved_path.is_empty() {
                format!("/{}", component)
            } else {
                format!("{}/{}", resolved_path, component)
            };

            // 递归解析此组件
            let resolved_component = self
                .resolve_symlink_recursive(&partial_path, max_depth)
                .await?;

            resolved_path = resolved_component;
        }

        if resolved_path.is_empty() {
            Ok("/".to_string())
        } else {
            Ok(resolved_path)
        }
    }

    /// 删除符号链接
    pub async fn remove_symlink(&self, link_path: &str) -> EvifResult<()> {
        let normalized_link = Self::normalize_path(link_path);

        let mut symlinks = self.symlinks.write().await;
        if !symlinks.contains_key(&normalized_link) {
            return Err(EvifError::NotFound(format!("Symlink: {}", link_path)));
        }

        symlinks.remove(&normalized_link);
        Ok(())
    }

    /// 挂载插件
    ///
    /// # 参数
    /// - `path`: 挂载路径（如 "/local", "/kv"）
    /// - `plugin`: 插件实例
    pub async fn mount(&self, path: String, plugin: Arc<dyn EvifPlugin>) -> EvifResult<()> {
        let mut mounts = self.mounts.write().await;

        // 标准化路径
        let normalized_path = Self::normalize_path(&path);

        // 检查是否已挂载
        if mounts.contains_key(&normalized_path) {
            return Err(EvifError::AlreadyMounted(normalized_path));
        }

        mounts.insert(normalized_path, plugin);
        Ok(())
    }

    /// 卸载插件
    pub async fn unmount(&self, path: &str) -> EvifResult<()> {
        let mut mounts = self.mounts.write().await;
        let normalized_path = Self::normalize_path(path);

        if !mounts.contains_key(&normalized_path) {
            return Err(EvifError::NotFound(format!("Mount point: {}", path)));
        }

        mounts.remove(&normalized_path);
        Ok(())
    }

    /// 查找插件（最长前缀匹配）
    ///
    /// # 参数
    /// - `path`: 文件路径
    ///
    /// # 返回
    /// 匹配的插件实例（如果存在）
    pub async fn lookup(&self, path: &str) -> Option<Arc<dyn EvifPlugin>> {
        let mounts = self.mounts.read().await;
        let normalized_path = Self::normalize_path(path);

        // 找到最长的匹配前缀
        let mut best_match: Option<(&String, Arc<dyn EvifPlugin>)> = None;

        for (mount_point, plugin) in mounts.iter() {
            if normalized_path.starts_with(mount_point) {
                match &best_match {
                    None => {
                        best_match = Some((mount_point, plugin.clone()));
                    }
                    Some((current_point, _)) => {
                        if mount_point.len() > current_point.len() {
                            best_match = Some((mount_point, plugin.clone()));
                        }
                    }
                }
            }
        }

        best_match.map(|(_, plugin)| plugin)
    }

    /// 列出所有挂载点
    pub async fn list_mounts(&self) -> Vec<String> {
        let mounts = self.mounts.read().await;
        let mut list: Vec<String> = mounts.keys().cloned().collect();
        list.sort(); // 按字母排序
        list
    }

    /// 获取挂载点数量
    pub async fn mount_count(&self) -> usize {
        let mounts = self.mounts.read().await;
        mounts.len()
    }

    /// 标准化路径
    fn normalize_path(path: &str) -> String {
        let path = path.trim_start_matches('/');

        if path.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", path)
        }
    }
}

impl Default for MountTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mount_and_lookup() {
        let mount_table = MountTable::new();
        let plugin = Arc::new(MockPlugin::new("test"));

        mount_table
            .mount("/test".to_string(), plugin.clone())
            .await
            .unwrap();

        let found = mount_table.lookup("/test/file.txt").await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "test");
    }

    #[tokio::test]
    async fn test_mount_duplicate() {
        let mount_table = MountTable::new();
        let plugin1 = Arc::new(MockPlugin::new("test1"));
        let plugin2 = Arc::new(MockPlugin::new("test2"));

        mount_table
            .mount("/test".to_string(), plugin1)
            .await
            .unwrap();
        let result = mount_table.mount("/test".to_string(), plugin2).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unmount() {
        let mount_table = MountTable::new();
        let plugin = Arc::new(MockPlugin::new("test"));

        mount_table
            .mount("/test".to_string(), plugin)
            .await
            .unwrap();
        mount_table.unmount("/test").await.unwrap();

        let found = mount_table.lookup("/test/file.txt").await;
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_longest_prefix_match() {
        let mount_table = MountTable::new();
        let plugin1 = Arc::new(MockPlugin::new("root"));
        let plugin2 = Arc::new(MockPlugin::new("sub"));

        mount_table.mount("/".to_string(), plugin1).await.unwrap();
        mount_table
            .mount("/sub".to_string(), plugin2)
            .await
            .unwrap();

        // 应该匹配到更具体的 /sub 而不是 /
        let found = mount_table.lookup("/sub/file.txt").await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "sub");

        // 应该匹配到根 /
        let found = mount_table.lookup("/other/file.txt").await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "root");
    }

    #[tokio::test]
    async fn test_path_normalization() {
        assert_eq!(MountTable::normalize_path("test"), "/test");
        assert_eq!(MountTable::normalize_path("/test"), "/test");
        assert_eq!(MountTable::normalize_path("test/"), "/test/");
        assert_eq!(MountTable::normalize_path(""), "/");
    }
}

// 简单的 Mock Plugin 用于测试
pub struct MockPlugin {
    name: String,
}

impl MockPlugin {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl EvifPlugin for MockPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    async fn create(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        Ok(())
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        Ok(())
    }

    async fn read(&self, _path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        Ok(Vec::new())
    }

    async fn write(
        &self,
        _path: &str,
        _data: Vec<u8>,
        _offset: i64,
        _flags: crate::plugin::WriteFlags,
    ) -> EvifResult<u64> {
        Ok(0)
    }

    async fn readdir(&self, _path: &str) -> EvifResult<Vec<crate::plugin::FileInfo>> {
        Ok(Vec::new())
    }

    async fn stat(&self, _path: &str) -> EvifResult<crate::plugin::FileInfo> {
        Ok(crate::plugin::FileInfo {
            name: "".to_string(),
            size: 0,
            mode: 0o644,
            modified: chrono::Utc::now(),
            is_dir: false,
        })
    }

    async fn remove(&self, _path: &str) -> EvifResult<()> {
        Ok(())
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        Ok(())
    }

    async fn remove_all(&self, _path: &str) -> EvifResult<()> {
        Ok(())
    }

    async fn symlink(&self, _target_path: &str, _link_path: &str) -> EvifResult<()> {
        Ok(())
    }

    async fn readlink(&self, _link_path: &str) -> EvifResult<String> {
        Ok("".to_string())
    }
}
