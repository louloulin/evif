// MemFS - 内存文件系统插件
//
// 对标 AGFS MemFS - 提供完整的内存文件系统功能

use evif_core::{EvifPlugin, FileInfo, WriteFlags, EvifResult, EvifError, PluginConfigParam};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use chrono::Utc;

/// 文件系统节点
#[derive(Debug)]
struct MemNode {
    name: String,
    is_dir: bool,
    data: Vec<u8>,
    mode: u32,
    modified: chrono::DateTime<chrono::Utc>,
    children: Option<HashMap<String, Arc<RwLock<MemNode>>>>,
}

impl MemNode {
    fn new_file(name: String, mode: u32) -> Self {
        Self {
            name,
            is_dir: false,
            data: Vec::new(),
            mode,
            modified: Utc::now(),
            children: None,
        }
    }

    fn new_dir(name: String, mode: u32) -> Self {
        Self {
            name,
            is_dir: true,
            data: Vec::new(),
            mode,
            modified: Utc::now(),
            children: Some(HashMap::new()),
        }
    }
}

#[derive(Clone)]
pub struct MemFsPlugin {
    root: Arc<RwLock<MemNode>>,
}

impl MemFsPlugin {
    pub fn new() -> Self {
        Self {
            root: Arc::new(RwLock::new(MemNode::new_dir("/".to_string(), 0o755))),
        }
    }

    /// 递归查找节点
    async fn find_node(&self, path: &str) -> EvifResult<Arc<RwLock<MemNode>>> {
        let clean_path = path.trim_start_matches('/');

        if clean_path.is_empty() || clean_path == "/" {
            return Ok(Arc::clone(&self.root));
        }

        let parts: Vec<&str> = clean_path.split('/').filter(|s| !s.is_empty()).collect();

        let mut current = Arc::clone(&self.root);

        for part in parts {
            // Clone the child reference if it exists
            let next_current = {
                let node_ref = current.read().await;

                if !node_ref.is_dir {
                    return Err(EvifError::InvalidPath("Not a directory".to_string()));
                }

                if let Some(children) = &node_ref.children {
                    if let Some(child) = children.get(part) {
                        Some(Arc::clone(child))
                    } else {
                        return Err(EvifError::NotFound(path.to_string()));
                    }
                } else {
                    return Err(EvifError::InvalidPath("Not a directory".to_string()));
                }
            };

            // Update current after the read guard is dropped
            if let Some(next) = next_current {
                current = next;
            }
        }

        Ok(current)
    }

    /// 查找父节点和基础名称
    async fn find_parent(&self, path: &str) -> EvifResult<(Arc<RwLock<MemNode>>, String)> {
        let clean_path = path.trim_start_matches('/');

        if clean_path.is_empty() || clean_path == "/" {
            return Err(EvifError::InvalidPath("Root has no parent".to_string()));
        }

        let parts: Vec<&str> = clean_path.split('/').filter(|s| !s.is_empty()).collect();

        if parts.is_empty() {
            return Err(EvifError::InvalidPath("Invalid path".to_string()));
        }

        let parent_path = if parts.len() == 1 {
            String::from("/")
        } else {
            format!("/{}", parts[..parts.len()-1].join("/"))
        };

        let basename = parts[parts.len()-1].to_string();

        let parent = self.find_node(&parent_path).await?;
        Ok((parent, basename))
    }
}

impl Default for MemFsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EvifPlugin for MemFsPlugin {
    fn name(&self) -> &str {
        "memfs"
    }

    fn get_readme(&self) -> String {
        r#"# MemFS

内存文件系统插件，数据仅存于进程内存，重启后丢失。无需配置。

## 配置

无（无需配置参数）。

## 示例

- 挂载: `/mem`，无需 config。
"#.to_string()
    }

    fn get_config_params(&self) -> Vec<PluginConfigParam> {
        vec![]
    }

    async fn create(&self, path: &str, perm: u32) -> EvifResult<()> {
        // 自动递归创建父目录（如果不存在）
        let clean_path = path.trim_start_matches('/');
        let parts: Vec<&str> = clean_path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() > 1 {
            for i in 1..parts.len() {
                let parent_path = format!("/{}", parts[..i].join("/"));
                if let Err(EvifError::NotFound(_)) = self.find_node(&parent_path).await {
                    self.mkdir(&parent_path, 0o755).await.ok();
                }
            }
        }

        let (parent, basename) = self.find_parent(path).await?;

        let mut parent_node = parent.write().await;

        if !parent_node.is_dir {
            return Err(EvifError::InvalidPath("Parent is not a directory".to_string()));
        }

        if let Some(children) = &mut parent_node.children {
            if children.contains_key(&basename) {
                return Err(EvifError::InvalidPath("File already exists".to_string()));
            }

            children.insert(
                basename.clone(),
                Arc::new(RwLock::new(MemNode::new_file(basename, perm)))
            );

            Ok(())
        } else {
            Err(EvifError::InvalidPath("Parent is not a directory".to_string()))
        }
    }

    async fn mkdir(&self, path: &str, perm: u32) -> EvifResult<()> {
        let (parent, basename) = self.find_parent(path).await?;

        let mut parent_node = parent.write().await;

        if !parent_node.is_dir {
            return Err(EvifError::InvalidPath("Parent is not a directory".to_string()));
        }

        if let Some(children) = &mut parent_node.children {
            if children.contains_key(&basename) {
                return Err(EvifError::InvalidPath("Directory already exists".to_string()));
            }

            children.insert(
                basename.clone(),
                Arc::new(RwLock::new(MemNode::new_dir(basename, perm)))
            );

            Ok(())
        } else {
            Err(EvifError::InvalidPath("Parent is not a directory".to_string()))
        }
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        let node = self.find_node(path).await?;
        let node_ref = node.read().await;

        if node_ref.is_dir {
            return Err(EvifError::InvalidPath("Is a directory".to_string()));
        }

        Ok(node_ref.data.clone())
    }

    async fn write(&self, path: &str, data: Vec<u8>, _offset: i64, _flags: WriteFlags)
        -> EvifResult<u64>
    {
        // 自动创建父目录和文件（如果不存在）
        if let Err(EvifError::NotFound(_)) = self.find_node(path).await {
            self.create(path, 0o644).await.ok();
        }
        let node = self.find_node(path).await?;
        let mut node_ref = node.write().await;

        if node_ref.is_dir {
            return Err(EvifError::InvalidPath("Is a directory".to_string()));
        }

        node_ref.data = data;
        node_ref.modified = Utc::now();
        Ok(node_ref.data.len() as u64)
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let node = self.find_node(path).await?;
        let node_ref = node.read().await;

        if !node_ref.is_dir {
            return Err(EvifError::InvalidPath("Not a directory".to_string()));
        }

        let mut entries = Vec::new();

        if let Some(children) = &node_ref.children {
            for (name, child) in children.iter() {
                let child_ref = child.read().await;

                entries.push(FileInfo {
                    name: name.clone(),
                    size: child_ref.data.len() as u64,
                    mode: child_ref.mode,
                    modified: child_ref.modified,
                    is_dir: child_ref.is_dir,
                });
            }
        }

        Ok(entries)
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let node = self.find_node(path).await?;
        let node_ref = node.read().await;

        Ok(FileInfo {
            name: node_ref.name.clone(),
            size: node_ref.data.len() as u64,
            mode: node_ref.mode,
            modified: node_ref.modified,
            is_dir: node_ref.is_dir,
        })
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        let (parent, basename) = self.find_parent(path).await?;

        let mut parent_node = parent.write().await;

        if !parent_node.is_dir {
            return Err(EvifError::InvalidPath("Parent is not a directory".to_string()));
        }

        if let Some(children) = &mut parent_node.children {
            if let Some(child) = children.remove(&basename) {
                // 检查是否为非空目录
                let is_dir_with_children = {
                    let child_ref = child.read().await;
                    if child_ref.is_dir {
                        if let Some(child_children) = &child_ref.children {
                            !child_children.is_empty()
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                };

                if is_dir_with_children {
                    // 恢复节点
                    children.insert(basename, child);
                    return Err(EvifError::InvalidPath("Directory not empty".to_string()));
                }

                Ok(())
            } else {
                Err(EvifError::NotFound(path.to_string()))
            }
        } else {
            Err(EvifError::InvalidPath("Parent is not a directory".to_string()))
        }
    }

    async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()> {
        // 获取旧节点
        let (old_parent, old_basename) = self.find_parent(old_path).await?;
        let (new_parent, new_basename) = self.find_parent(new_path).await?;

        // 从旧父节点移除
        let old_node = {
            let mut old_parent_node = old_parent.write().await;

            if !old_parent_node.is_dir {
                return Err(EvifError::InvalidPath("Parent is not a directory".to_string()));
            }

            if let Some(children) = &mut old_parent_node.children {
                children.remove(&old_basename)
                    .ok_or_else(|| EvifError::NotFound(old_path.to_string()))?
            } else {
                return Err(EvifError::InvalidPath("Parent is not a directory".to_string()));
            }
        };

        // 添加到新父节点
        let mut new_parent_node = new_parent.write().await;

        if !new_parent_node.is_dir {
            return Err(EvifError::InvalidPath("Parent is not a directory".to_string()));
        }

        if let Some(children) = &mut new_parent_node.children {
            if children.contains_key(&new_basename) {
                // 恢复旧节点
                let mut old_parent_node = old_parent.write().await;
                if let Some(old_children) = &mut old_parent_node.children {
                    old_children.insert(old_basename, old_node);
                }
                return Err(EvifError::InvalidPath("Target already exists".to_string()));
            }

            children.insert(new_basename, old_node);
            Ok(())
        } else {
            Err(EvifError::InvalidPath("Parent is not a directory".to_string()))
        }
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        let clean_path = path.trim_start_matches('/');

        if clean_path.is_empty() || clean_path == "/" {
            return Err(EvifError::InvalidPath("Cannot remove root".to_string()));
        }

        // 找到要删除的节点
        let node = self.find_node(path).await?;

        // 递归删除所有子节点
        let node_ref = node.read().await;
        if let Some(children) = &node_ref.children {
            let child_names: Vec<String> = children.keys().cloned().collect();
            drop(node_ref);

            for child_name in child_names {
                let child_path = format!("{}/{}", path.trim_end_matches('/'), child_name);
                self.remove_all(&child_path).await?;
            }
        }

        // 删除自身 - 从父节点的 children 中移除
        let (parent, basename) = self.find_parent(path).await?;
        let mut parent_ref = parent.write().await;
        if let Some(parent_children) = &mut parent_ref.children {
            parent_children.remove(&basename);
        }

        Ok(())
    }

    async fn chmod(&self, path: &str, mode: u32) -> EvifResult<()> {
        let node = self.find_node(path).await?;
        let mut node_ref = node.write().await;
        node_ref.mode = mode;
        node_ref.modified = Utc::now();
        Ok(())
    }

    async fn truncate(&self, path: &str, size: u64) -> EvifResult<()> {
        let node = self.find_node(path).await?;
        let mut node_ref = node.write().await;

        if node_ref.is_dir {
            return Err(EvifError::InvalidPath("Is a directory".to_string()));
        }

        node_ref.data.truncate(size as usize);
        node_ref.modified = Utc::now();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memfs_basic() {
        let fs = MemFsPlugin::new();

        // 创建目录
        fs.mkdir("/data", 0o755).await.unwrap();

        // 创建文件
        fs.create("/data/file1.txt", 0o644).await.unwrap();
        fs.write("/data/file1.txt", b"Hello, MemFS!".to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        // 读取文件
        let data = fs.read("/data/file1.txt", 0, 100).await.unwrap();
        assert_eq!(data, b"Hello, MemFS!");

        // 列出目录
        let entries = fs.readdir("/data").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "file1.txt");

        // 文件信息
        let info = fs.stat("/data/file1.txt").await.unwrap();
        assert_eq!(info.size, 13);
        assert!(!info.is_dir);
    }

    #[tokio::test]
    async fn test_memfs_hierarchy() {
        let fs = MemFsPlugin::new();

        // 创建多层目录
        fs.mkdir("/a", 0o755).await.unwrap();
        fs.mkdir("/a/b", 0o755).await.unwrap();
        fs.mkdir("/a/b/c", 0o755).await.unwrap();

        // 创建文件
        fs.create("/a/b/c/file.txt", 0o644).await.unwrap();
        fs.write("/a/b/c/file.txt", b"deep file".to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        // 验证文件可访问
        let data = fs.read("/a/b/c/file.txt", 0, 100).await.unwrap();
        assert_eq!(data, b"deep file");

        // 重命名
        fs.rename("/a/b/c/file.txt", "/a/b/c/renamed.txt").await.unwrap();
        let result = fs.read("/a/b/c/file.txt", 0, 100).await;
        assert!(result.is_err());

        let data = fs.read("/a/b/c/renamed.txt", 0, 100).await.unwrap();
        assert_eq!(data, b"deep file");
    }

    #[tokio::test]
    async fn test_memfs_delete() {
        let fs = MemFsPlugin::new();

        fs.mkdir("/test", 0o755).await.unwrap();
        fs.create("/test/file.txt", 0o644).await.unwrap();
        fs.write("/test/file.txt", b"test data".to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        // 删除文件
        fs.remove("/test/file.txt").await.unwrap();

        // 验证已删除
        let entries = fs.readdir("/test").await.unwrap();
        assert_eq!(entries.len(), 0);

        // 删除空目录
        fs.remove("/test").await.unwrap();
    }

    #[tokio::test]
    async fn test_memfs_remove_all() {
        let fs = MemFsPlugin::new();

        // 创建多层目录
        fs.mkdir("/dir1", 0o755).await.unwrap();
        fs.mkdir("/dir1/dir2", 0o755).await.unwrap();
        fs.create("/dir1/file1.txt", 0o644).await.unwrap();
        fs.create("/dir1/dir2/file2.txt", 0o644).await.unwrap();

        // 测试 RemoveAll
        fs.remove_all("/dir1").await.unwrap();

        // 验证所有内容都被删除
        assert!(fs.stat("/dir1").await.is_err());
        assert!(fs.stat("/dir1/dir2").await.is_err());
        assert!(fs.stat("/dir1/file1.txt").await.is_err());
        assert!(fs.stat("/dir1/dir2/file2.txt").await.is_err());
    }

    #[tokio::test]
    async fn test_memfs_remove_all_deeply_nested() {
        let fs = MemFsPlugin::new();

        // 创建深层嵌套结构
        fs.mkdir("/a", 0o755).await.unwrap();
        fs.mkdir("/a/b", 0o755).await.unwrap();
        fs.mkdir("/a/b/c", 0o755).await.unwrap();
        fs.mkdir("/a/b/c/d", 0o755).await.unwrap();
        fs.create("/a/b/c/d/file.txt", 0o644).await.unwrap();
        fs.write("/a/b/c/d/file.txt", b"deep".to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        // 从中间删除
        fs.remove_all("/a/b").await.unwrap();

        // 验证
        assert!(fs.stat("/a").await.is_ok());  // /a 仍然存在
        assert!(fs.stat("/a/b").await.is_err());
        assert!(fs.stat("/a/b/c").await.is_err());
    }
}
