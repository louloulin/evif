// HelloFS - 演示插件
//
// 最小化演示插件,展示 EVIF 插件开发基础

use evif_core::{EvifPlugin, FileInfo, WriteFlags, EvifResult, EvifError};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

/// HelloFS 插件
///
/// 提供简单的演示文件系统
pub struct HelloFsPlugin {
    message: Arc<RwLock<String>>,
}

impl HelloFsPlugin {
    /// 创建新的 HelloFS 插件
    pub fn new() -> Self {
        Self {
            message: Arc::new(RwLock::new("Hello, EVIF!".to_string())),
        }
    }

    /// 设置自定义消息
    pub async fn set_message(&self, msg: String) {
        let mut message = self.message.write().await;
        *message = msg;
    }

    /// 获取当前消息
    pub async fn get_message(&self) -> String {
        let message = self.message.read().await;
        message.clone()
    }
}

impl Default for HelloFsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EvifPlugin for HelloFsPlugin {
    fn name(&self) -> &str {
        "hellofs"
    }

    fn get_readme(&self) -> String {
        r#"# HelloFS

演示插件，提供只读 `/hello` 与可写 `/message` 虚拟文件。无需配置。

## 配置

无。

## 示例

- 挂载: `/hello`，无需 config。
"#.to_string()
    }

    async fn create(&self, path: &str, _perm: u32) -> EvifResult<()> {
        match path {
            "/hello" | "/message" => Ok(()),
            _ => Err(EvifError::NotFound(path.to_string())),
        }
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        // 不支持目录
        Err(EvifError::InvalidPath("Directories not supported".to_string()))
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        match path {
            "/hello" => Ok(b"Hello, EVIF!\n".to_vec()),
            "/message" => {
                let message = self.message.read().await;
                Ok(format!("{}\n", message.as_str()).into_bytes())
            }
            _ => Err(EvifError::NotFound(path.to_string())),
        }
    }

    async fn write(&self, path: &str, data: Vec<u8>, _offset: i64, _flags: WriteFlags)
        -> EvifResult<u64>
    {
        if path == "/message" {
            let msg = String::from_utf8(data)
                .map_err(|_| EvifError::InvalidPath("Invalid UTF-8".to_string()))?;
            let mut message = self.message.write().await;
            *message = msg;
            Ok(message.len() as u64)
        } else {
            Err(EvifError::InvalidPath("Cannot write to /hello".to_string()))
        }
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        if path == "/" || path.is_empty() {
            Ok(vec![
                FileInfo {
                    name: "hello".to_string(),
                    size: 13,
                    mode: 0o644,
                    modified: chrono::Utc::now(),
                    is_dir: false,
                },
                FileInfo {
                    name: "message".to_string(),
                    size: 0,
                    mode: 0o644,
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
            "/hello" => Ok(FileInfo {
                name: "hello".to_string(),
                size: 13,
                mode: 0o644,
                modified: chrono::Utc::now(),
                is_dir: false,
            }),
            "/message" => {
                let message = self.message.read().await;
                Ok(FileInfo {
                    name: "message".to_string(),
                    size: message.len() as u64,
                    mode: 0o644,
                    modified: chrono::Utc::now(),
                    is_dir: false,
                })
            }
            _ => Err(EvifError::NotFound(path.to_string())),
        }
    }

    async fn remove(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::InvalidPath("Cannot remove virtual files".to_string()))
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        Err(EvifError::InvalidPath("Rename not supported".to_string()))
    }

    async fn remove_all(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::InvalidPath("RemoveAll not supported".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hellofs_basic() {
        let plugin = HelloFsPlugin::new();

        // 测试读取 /hello
        let data = plugin.read("/hello", 0, 100).await.unwrap();
        assert_eq!(data, b"Hello, EVIF!\n");

        // 测试读取 /message
        let data = plugin.read("/message", 0, 100).await.unwrap();
        assert_eq!(data, b"Hello, EVIF!\n");

        // 测试 stat
        let info = plugin.stat("/hello").await.unwrap();
        assert_eq!(info.name, "hello");
        assert_eq!(info.size, 13);
        assert!(!info.is_dir);
    }

    #[tokio::test]
    async fn test_hellofs_write_message() {
        let plugin = HelloFsPlugin::new();

        // 写入自定义消息
        plugin.write("/message", b"Custom message".to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        // 读取验证
        let data = plugin.read("/message", 0, 100).await.unwrap();
        assert_eq!(data, b"Custom message\n");

        // 验证 stat 大小更新
        let info = plugin.stat("/message").await.unwrap();
        assert_eq!(info.size, 14); // "Custom message\n".len()
    }

    #[tokio::test]
    async fn test_hellofs_readdir() {
        let plugin = HelloFsPlugin::new();

        let entries = plugin.readdir("/").await.unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|e| e.name == "hello"));
        assert!(entries.iter().any(|e| e.name == "message"));
    }

    #[tokio::test]
    async fn test_hellofs_set_get_message() {
        let plugin = HelloFsPlugin::new();

        // 设置消息
        plugin.set_message("New greeting".to_string()).await;

        // 获取消息
        let msg = plugin.get_message().await;
        assert_eq!(msg, "New greeting");

        // 通过文件系统读取验证
        let data = plugin.read("/message", 0, 100).await.unwrap();
        assert_eq!(data, b"New greeting\n");
    }

    #[tokio::test]
    async fn test_hellofs_invalid_operations() {
        let plugin = HelloFsPlugin::new();

        // 测试不支持的路径
        let result = plugin.read("/nonexistent", 0, 100).await;
        assert!(result.is_err());

        // 测试不支持的写操作
        let result = plugin.write("/hello", b"data".to_vec(), 0, WriteFlags::CREATE).await;
        assert!(result.is_err());

        // 测试不支持的删除
        let result = plugin.remove("/hello").await;
        assert!(result.is_err());

        // 测试不支持的目录
        let result = plugin.mkdir("/dir", 0o755).await;
        assert!(result.is_err());
    }
}
