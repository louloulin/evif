// EVIF DevFS Plugin - 设备文件系统
//
// 提供 /dev/null 等设备文件支持
// 对标 AGFS DevFS

use evif_core::{EvifError, EvifResult, EvifPlugin, FileInfo, WriteFlags};
use async_trait::async_trait;

/// DevFS 配置
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct DevConfig {
    /// 是否只读
    pub read_only: bool,
}


/// DevFS 插件
///
/// 提供类 Unix 设备文件支持
pub struct DevFsPlugin {
    config: DevConfig,
}

impl DevFsPlugin {
    pub fn new() -> Self {
        Self {
            config: DevConfig::default(),
        }
    }

    pub fn with_config(config: DevConfig) -> Self {
        Self { config }
    }
}

impl Default for DevFsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EvifPlugin for DevFsPlugin {
    fn name(&self) -> &str {
        "devfs"
    }

    async fn create(&self, path: &str, _perm: u32) -> EvifResult<()> {
        match path {
            "/null" | "/zero" | "/full" => Ok(()),
            _ => Err(EvifError::InvalidPath(format!(
                "Unknown device: {}",
                path
            ))),
        }
    }

    async fn mkdir(&self, path: &str, _perm: u32) -> EvifResult<()> {
        // DevFS 不支持创建目录
        Err(EvifError::InvalidPath(format!(
            "Cannot mkdir in devfs: {}",
            path
        )))
    }

    async fn read(&self, path: &str, _offset: u64, size: u64) -> EvifResult<Vec<u8>> {
        match path {
            "/null" => Ok(Vec::new()), // /dev/null 总是返回空数据
            "/zero" => {
                // /dev/zero 返回无限个零字节
                let size = if size == 0 { 1024 } else { size as usize };
                Ok(vec![0u8; size])
            }
            "/full" => {
                // /dev/full 总是返回 ENOSPC (设备已满)
                Err(EvifError::Io(std::io::Error::other(
                    "No space left on device",
                )))
            }
            _ => Err(EvifError::NotFound(path.to_string())),
        }
    }

    async fn write(&self, path: &str, data: Vec<u8>, _offset: i64, _flags: WriteFlags) -> EvifResult<u64> {
        if self.config.read_only {
            return Err(EvifError::ReadOnly);
        }

        match path {
            "/null" => Ok(data.len() as u64), // /dev/null 吸收所有数据
            "/zero" => Ok(data.len() as u64), // /dev/zero 吸收所有数据
            "/full" => {
                // /dev/full 总是返回 ENOSPC
                Err(EvifError::Io(std::io::Error::other(
                    "No space left on device",
                )))
            }
            _ => Err(EvifError::NotFound(path.to_string())),
        }
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        if path == "/" || path.is_empty() {
            Ok(vec![
                FileInfo {
                    name: "null".to_string(),
                    size: 0,
                    mode: 0o666,
                    modified: chrono::Utc::now(),
                    is_dir: false,
                },
                FileInfo {
                    name: "zero".to_string(),
                    size: 0,
                    mode: 0o666,
                    modified: chrono::Utc::now(),
                    is_dir: false,
                },
                FileInfo {
                    name: "full".to_string(),
                    size: 0,
                    mode: 0o666,
                    modified: chrono::Utc::now(),
                    is_dir: false,
                },
            ])
        } else {
            Err(EvifError::NotFound(path.to_string()))
        }
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        match path {
            "/null" | "/zero" | "/full" => {
                let name = path.trim_start_matches('/');
                Ok(FileInfo {
                    name: name.to_string(),
                    size: 0,
                    mode: 0o666,
                    modified: chrono::Utc::now(),
                    is_dir: false,
                })
            }
            "/" | "" => Ok(FileInfo {
                name: "".to_string(),
                size: 0,
                mode: 0o755,
                modified: chrono::Utc::now(),
                is_dir: true,
            }),
            _ => Err(EvifError::NotFound(path.to_string())),
        }
    }

    async fn remove(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::InvalidPath(
            "Cannot remove device files".to_string(),
        ))
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        Err(EvifError::InvalidPath(
            "Cannot rename device files".to_string(),
        ))
    }

    async fn remove_all(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::InvalidPath(
            "Cannot remove device files".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_devfs_null() {
        let dev = DevFsPlugin::new();

        // 测试 /dev/null
        let data = dev.read("/null", 0, 100).await.unwrap();
        assert_eq!(data.len(), 0);

        // 写入 /dev/null 应该成功
        let len = dev.write("/null", b"test".to_vec(), 0, WriteFlags::CREATE).await.unwrap();
        assert_eq!(len, 4);
    }

    #[tokio::test]
    async fn test_devfs_zero() {
        let dev = DevFsPlugin::new();

        // 测试 /dev/zero
        let data = dev.read("/zero", 0, 100).await.unwrap();
        assert_eq!(data.len(), 100);
        assert!(data.iter().all(|&b| b == 0));
    }

    #[tokio::test]
    async fn test_devfs_full() {
        let dev = DevFsPlugin::new();

        // 测试 /dev/full - 读取返回 ENOSPC
        let result = dev.read("/full", 0, 100).await;
        assert!(result.is_err());

        // 写入 /dev/full 也返回 ENOSPC
        let result = dev.write("/full", b"test".to_vec(), 0, WriteFlags::CREATE).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_devfs_stat() {
        let dev = DevFsPlugin::new();

        let info = dev.stat("/null").await.unwrap();
        assert_eq!(info.name, "null");
        assert!(!info.is_dir);

        let info = dev.stat("/zero").await.unwrap();
        assert_eq!(info.name, "zero");
        assert!(!info.is_dir);
    }

    #[tokio::test]
    async fn test_devfs_readdir() {
        let dev = DevFsPlugin::new();

        let entries = dev.readdir("/").await.unwrap();
        assert_eq!(entries.len(), 3);

        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"null"));
        assert!(names.contains(&"zero"));
        assert!(names.contains(&"full"));
    }
}
