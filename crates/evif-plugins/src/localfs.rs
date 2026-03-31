// LocalFS - 本地文件系统插件
//
// 对标 AGFS LocalFS
// Phase 8: validate / get_readme / get_config_params

use evif_core::{EvifPlugin, FileInfo, WriteFlags, EvifResult, EvifError, PluginConfigParam};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::os::unix::fs::PermissionsExt;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};

pub struct LocalFsPlugin {
    base_path: PathBuf,
    read_only: bool,
}

impl LocalFsPlugin {
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
            read_only: false,
        }
    }

    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    fn resolve_path(&self, path: &str) -> EvifResult<PathBuf> {
        let clean_path = path.trim_start_matches('/');
        let full = self.base_path.join(clean_path);

        // 安全检查：防止路径遍历攻击
        // 对于已存在的路径，检查canonicalize
        if full.exists() {
            let canonical = full.canonicalize()
                .map_err(|_| EvifError::InvalidPath(path.to_string()))?;

            let base_canonical = self.base_path.canonicalize()
                .map_err(|_| EvifError::InvalidPath("base_path".to_string()))?;

            if !canonical.starts_with(&base_canonical) {
                return Err(EvifError::InvalidPath("Path traversal detected".to_string()));
            }
        } else {
            // 对于不存在的路径，检查父目录
            if let Some(parent) = full.parent() {
                if parent.exists() {
                    let parent_canonical = parent.canonicalize()
                        .map_err(|_| EvifError::InvalidPath(path.to_string()))?;

                    let base_canonical = self.base_path.canonicalize()
                        .map_err(|_| EvifError::InvalidPath("base_path".to_string()))?;

                    if !parent_canonical.starts_with(&base_canonical) {
                        return Err(EvifError::InvalidPath("Path traversal detected".to_string()));
                    }
                }
            }
        }

        Ok(full)
    }
}

#[async_trait::async_trait]
impl EvifPlugin for LocalFsPlugin {
    fn name(&self) -> &str {
        "localfs"
    }

    async fn validate(&self, config: Option<&serde_json::Value>) -> EvifResult<()> {
        if let Some(c) = config {
            if let Some(root) = c.get("root").and_then(|v| v.as_str()) {
                if root.trim().is_empty() {
                    return Err(EvifError::InvalidInput("config.root must be non-empty".to_string()));
                }
            }
        }
        Ok(())
    }

    fn get_readme(&self) -> String {
        r#"# LocalFS

本地目录挂载插件，将宿主机目录映射为只读或可读写文件系统。

## 配置

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| root | string | 否 | 本地根目录路径，默认 `/tmp/evif-local` |

## 示例

- 挂载: `/local`，config: `{ "root": "/tmp/evif-local" }`
"#.to_string()
    }

    fn get_config_params(&self) -> Vec<PluginConfigParam> {
        vec![
            PluginConfigParam {
                name: "root".to_string(),
                param_type: "string".to_string(),
                required: false,
                default: Some("/tmp/evif-local".to_string()),
                description: Some("本地根目录路径".to_string()),
            },
        ]
    }

    async fn create(&self, path: &str, _perm: u32) -> EvifResult<()> {
        if self.read_only {
            return Err(EvifError::ReadOnly);
        }

        let full_path = self.resolve_path(path)?;
        let parent = full_path.parent()
            .ok_or_else(|| EvifError::InvalidPath("No parent".to_string()))?;

        tokio::fs::create_dir_all(parent).await?;
        tokio::fs::File::create(full_path).await?;
        Ok(())
    }

    async fn mkdir(&self, path: &str, _perm: u32) -> EvifResult<()> {
        if self.read_only {
            return Err(EvifError::ReadOnly);
        }

        let full_path = self.resolve_path(path)?;
        tokio::fs::create_dir_all(full_path).await?;
        Ok(())
    }

    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>> {
        let full_path = self.resolve_path(path)?;
        let mut file = tokio::fs::File::open(full_path).await?;

        if offset > 0 {
            file.seek(SeekFrom::Start(offset)).await?;
        }

        let mut buffer = if size > 0 {
            Vec::with_capacity(size as usize)
        } else {
            Vec::new()
        };

        // 读取指定大小或全部
        if size > 0 {
            let mut take = file.take(size);
            let n = take.read_to_end(&mut buffer).await?;
            buffer.truncate(n);
        } else {
            let n = file.read_to_end(&mut buffer).await?;
            buffer.truncate(n);
        }

        Ok(buffer)
    }

    async fn write(&self, path: &str, data: Vec<u8>, offset: i64, flags: WriteFlags)
        -> EvifResult<u64>
    {
        if self.read_only {
            return Err(EvifError::ReadOnly);
        }

        let full_path = self.resolve_path(path)?;

        // 确保父目录存在
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut file = if flags.contains(WriteFlags::CREATE) {
            if flags.contains(WriteFlags::EXCLUSIVE) {
                tokio::fs::File::create_new(full_path).await?
            } else {
                tokio::fs::File::create(full_path).await?
            }
        } else {
            tokio::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(full_path)
                .await?
        };

        if flags.contains(WriteFlags::TRUNCATE) {
            file.set_len(0).await?;
        }

        let write_offset = if flags.contains(WriteFlags::APPEND) {
            file.seek(SeekFrom::End(0)).await?
        } else if offset >= 0 {
            file.seek(SeekFrom::Start(offset as u64)).await?
        } else {
            0
        };

        file.seek(SeekFrom::Start(write_offset)).await?;
        let written = file.write(&data).await?;
        file.sync_all().await?;  // 确保数据写入磁盘

        Ok(written as u64)
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let full_path = self.resolve_path(path)?;
        let mut entries = vec![];

        let mut dir = tokio::fs::read_dir(full_path).await?;

        while let Some(entry) = dir.next_entry().await? {
            let metadata = entry.metadata().await?;
            let modified: std::time::SystemTime = metadata.modified()?;
            let chrono_modified: chrono::DateTime<chrono::Utc> = modified.into();

            entries.push(FileInfo {
                name: entry.file_name().to_string_lossy().to_string(),
                size: metadata.len(),
                mode: metadata.permissions().mode(),
                modified: chrono_modified,
                is_dir: metadata.is_dir(),
            });
        }

        Ok(entries)
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let full_path = self.resolve_path(path)?;
        let metadata = tokio::fs::metadata(full_path).await?;
        let modified: std::time::SystemTime = metadata.modified()?;
        let chrono_modified: chrono::DateTime<chrono::Utc> = modified.into();

        Ok(FileInfo {
            name: PathBuf::from(path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            size: metadata.len(),
            mode: metadata.permissions().mode(),
            modified: chrono_modified,
            is_dir: metadata.is_dir(),
        })
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        if self.read_only {
            return Err(EvifError::ReadOnly);
        }

        let full_path = self.resolve_path(path)?;
        let metadata = tokio::fs::metadata(&full_path).await?;

        if metadata.is_dir() {
            tokio::fs::remove_dir(full_path).await?;
        } else {
            tokio::fs::remove_file(full_path).await?;
        }

        Ok(())
    }

    async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()> {
        if self.read_only {
            return Err(EvifError::ReadOnly);
        }

        let old_full = self.resolve_path(old_path)?;
        let new_full = self.resolve_path(new_path)?;

        // 确保目标目录存在
        if let Some(parent) = new_full.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::rename(old_full, new_full).await?;
        Ok(())
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        if self.read_only {
            return Err(EvifError::ReadOnly);
        }

        let full_path = self.resolve_path(path)?;

        // 检查是文件还是目录
        let metadata = tokio::fs::metadata(&full_path).await;

        match metadata {
            Ok(meta) => {
                if meta.is_dir() {
                    // 目录: 使用 remove_dir_all 进行递归删除
                    tokio::fs::remove_dir_all(&full_path).await
                        .map_err(|e| match e.kind() {
                            std::io::ErrorKind::NotFound => EvifError::NotFound(path.to_string()),
                            _ => EvifError::Io(e),
                        })?;
                } else {
                    // 文件: 使用 remove_file
                    tokio::fs::remove_file(&full_path).await
                        .map_err(|e| match e.kind() {
                            std::io::ErrorKind::NotFound => EvifError::NotFound(path.to_string()),
                            _ => EvifError::Io(e),
                        })?;
                }
            }
            Err(e) => {
                return Err(match e.kind() {
                    std::io::ErrorKind::NotFound => EvifError::NotFound(path.to_string()),
                    _ => EvifError::Io(e),
                });
            }
        }

        Ok(())
    }

    async fn chmod(&self, path: &str, mode: u32) -> EvifResult<()> {
        if self.read_only {
            return Err(EvifError::ReadOnly);
        }

        let full_path = self.resolve_path(path)?;
        let mut perms = tokio::fs::metadata(&full_path).await?.permissions();
        perms.set_mode(mode);
        tokio::fs::set_permissions(full_path, perms).await?;
        Ok(())
    }

    async fn truncate(&self, path: &str, size: u64) -> EvifResult<()> {
        if self.read_only {
            return Err(EvifError::ReadOnly);
        }

        let full_path = self.resolve_path(path)?;
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .open(full_path)
            .await?;
        file.set_len(size).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_localfs_basic() {
        let temp_dir = tempdir().unwrap();
        let plugin = LocalFsPlugin::new(temp_dir.path());

        // 测试创建和写入
        plugin.create("test.txt", 0o644).await.unwrap();
        plugin.write("test.txt", b"Hello, EVIF!".to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        // 测试读取
        let data = plugin.read("test.txt", 0, 100).await.unwrap();
        assert_eq!(data, b"Hello, EVIF!");

        // 测试 stat
        let info = plugin.stat("test.txt").await.unwrap();
        assert_eq!(info.size, 12);
        assert!(!info.is_dir);

        // 测试目录
        plugin.mkdir("subdir", 0o755).await.unwrap();
        let entries = plugin.readdir("/").await.unwrap();
        assert!(entries.iter().any(|e| e.name == "test.txt"));
        assert!(entries.iter().any(|e| e.name == "subdir"));

        // 测试重命名
        plugin.rename("test.txt", "renamed.txt").await.unwrap();
        let data = plugin.read("renamed.txt", 0, 100).await.unwrap();
        assert_eq!(data, b"Hello, EVIF!");

        // 测试删除
        plugin.remove("renamed.txt").await.unwrap();
        let result = plugin.stat("renamed.txt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_localfs_remove_all() {
        let temp_dir = tempdir().unwrap();
        let plugin = LocalFsPlugin::new(temp_dir.path());

        // 创建嵌套目录结构
        plugin.mkdir("dir1", 0o755).await.unwrap();
        plugin.mkdir("dir1/subdir1", 0o755).await.unwrap();
        plugin.mkdir("dir1/subdir2", 0o755).await.unwrap();
        plugin.create("dir1/file1.txt", 0o644).await.unwrap();
        plugin.write("dir1/file1.txt", b"data1".to_vec(), 0, WriteFlags::CREATE).await.unwrap();
        plugin.create("dir1/subdir1/file2.txt", 0o644).await.unwrap();
        plugin.write("dir1/subdir1/file2.txt", b"data2".to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        // 测试递归删除
        plugin.remove_all("dir1").await.unwrap();

        // 验证目录和所有子项都被删除
        let result = plugin.stat("dir1").await;
        assert!(result.is_err());

        let result = plugin.stat("dir1/file1.txt").await;
        assert!(result.is_err());

        let result = plugin.stat("dir1/subdir1/file2.txt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_localfs_remove_all_single_file() {
        let temp_dir = tempdir().unwrap();
        let plugin = LocalFsPlugin::new(temp_dir.path());

        // 创建单个文件
        plugin.create("single.txt", 0o644).await.unwrap();
        plugin.write("single.txt", b"data".to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        // RemoveAll 应该能删除单个文件
        plugin.remove_all("single.txt").await.unwrap();

        let result = plugin.stat("single.txt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_localfs_path_traversal_rejected() {
        let temp_dir = tempdir().unwrap();
        let plugin = LocalFsPlugin::new(temp_dir.path());

        // Try path traversal with ..
        let result = plugin.stat("../../../etc/passwd").await;
        assert!(result.is_err(), "Path traversal with ../ should be rejected");

        let result = plugin.read("../../../etc/passwd", 0, 100).await;
        assert!(result.is_err(), "Path traversal read should be rejected");

        let result = plugin.write("../../../tmp/evil", b"data".to_vec(), 0, WriteFlags::CREATE).await;
        assert!(result.is_err(), "Path traversal write should be rejected");

        // Absolute path outside base should also be rejected
        let result = plugin.stat("/etc/passwd").await;
        assert!(result.is_err(), "Absolute path outside base should be rejected");
    }

    #[tokio::test]
    async fn test_localfs_symlink_traversal_rejected() {
        let temp_dir = tempdir().unwrap();
        let plugin = LocalFsPlugin::new(temp_dir.path());

        // Create a file inside base dir
        plugin.create("safe.txt", 0o644).await.unwrap();
        plugin.write("safe.txt", b"safe data".to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        // Create a symlink pointing outside the base dir
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let link_path = temp_dir.path().join("escape_link");
            symlink("/etc", &link_path).unwrap();

            // Reading through symlink should fail
            let result = plugin.stat("escape_link/passwd").await;
            assert!(result.is_err(), "Symlink traversal should be rejected");
        }
    }
}
