// EVIF Server - 核心服务器
//
// 提供 EvifPlugin trait 和 EvifServer 实现
// 对标 AGFS 服务器架构

use crate::error::{EvifError, EvifResult};
use crate::mount_table::MountTable;
use crate::plugin::{EvifPlugin, FileInfo, WriteFlags};
use std::sync::Arc;

/// EVIF 服务器
///
/// 核心组件：插件挂载系统
/// 负责路由文件操作到对应的插件
pub struct EvifServer {
    mount_table: Arc<MountTable>,
}

impl EvifServer {
    /// 创建新的 EVIF 服务器
    pub fn new() -> Self {
        Self {
            mount_table: Arc::new(MountTable::new()),
        }
    }

    /// 注册插件
    ///
    /// # 参数
    /// - `path`: 挂载路径（如 "/local", "/kv"）
    /// - `plugin`: 插件实例
    pub async fn register_plugin(
        &self,
        path: String,
        plugin: Arc<dyn EvifPlugin>,
    ) -> EvifResult<()> {
        self.mount_table.mount(path, plugin).await
    }

    /// 卸载插件
    pub async fn unregister_plugin(&self, path: &str) -> EvifResult<()> {
        self.mount_table.unmount(path).await
    }

    /// 列出所有挂载点
    pub async fn list_mounts(&self) -> Vec<String> {
        self.mount_table.list_mounts().await
    }

    /// 路由文件操作到对应的插件
    async fn route(&self, path: &str) -> EvifResult<Arc<dyn EvifPlugin>> {
        self.mount_table
            .lookup(path)
            .await
            .ok_or_else(|| EvifError::NotFound(path.to_string()))
    }

    /// 创建文件
    pub async fn create(&self, path: &str, perm: u32) -> EvifResult<()> {
        let plugin = self.route(path).await?;
        plugin.create(path, perm).await
    }

    /// 创建目录
    pub async fn mkdir(&self, path: &str, perm: u32) -> EvifResult<()> {
        let plugin = self.route(path).await?;
        plugin.mkdir(path, perm).await
    }

    /// 读取文件
    pub async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>> {
        let plugin = self.route(path).await?;
        plugin.read(path, offset, size).await
    }

    /// 写入文件
    pub async fn write(
        &self,
        path: &str,
        data: Vec<u8>,
        offset: i64,
        flags: WriteFlags,
    ) -> EvifResult<u64> {
        let plugin = self.route(path).await?;
        plugin.write(path, data, offset, flags).await
    }

    /// 读取目录
    pub async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let plugin = self.route(path).await?;
        plugin.readdir(path).await
    }

    /// 获取文件信息
    pub async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let plugin = self.route(path).await?;
        plugin.stat(path).await
    }

    /// 删除文件
    pub async fn remove(&self, path: &str) -> EvifResult<()> {
        let plugin = self.route(path).await?;
        plugin.remove(path).await
    }

    /// 重命名文件
    pub async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()> {
        let old_plugin = self.route(old_path).await?;
        let new_plugin = self.route(new_path).await?;

        // 如果源和目标在不同插件，暂不支持跨插件移动
        if old_plugin.name() != new_plugin.name() {
            return Err(EvifError::NotSupportedGeneric);
        }

        old_plugin.rename(old_path, new_path).await
    }

    /// 递归删除文件或目录
    pub async fn remove_all(&self, path: &str) -> EvifResult<()> {
        let plugin = self.route(path).await?;
        plugin.remove_all(path).await
    }

    /// 获取挂载表引用（供外部使用）
    pub fn mount_table(&self) -> &Arc<MountTable> {
        &self.mount_table
    }
}

impl Default for EvifServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mount_table::MockPlugin;

    #[tokio::test]
    async fn test_server_plugin_registration() {
        let server = EvifServer::new();
        let plugin = Arc::new(MockPlugin::new("test"));

        server
            .register_plugin("/test".to_string(), plugin.clone())
            .await
            .unwrap();

        let mounts = server.list_mounts().await;
        assert_eq!(mounts.len(), 1);
        assert_eq!(mounts[0], "/test");
    }

    #[tokio::test]
    async fn test_server_route() {
        let server = EvifServer::new();
        let plugin = Arc::new(MockPlugin::new("test"));

        server
            .register_plugin("/test".to_string(), plugin)
            .await
            .unwrap();

        // 路由应该找到插件
        let result = server.create("/test/file.txt", 0o644).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_server_cross_plugin_rename() {
        let server = EvifServer::new();
        let plugin1 = Arc::new(MockPlugin::new("test1"));
        let plugin2 = Arc::new(MockPlugin::new("test2"));

        server
            .register_plugin("/test1".to_string(), plugin1)
            .await
            .unwrap();
        server
            .register_plugin("/test2".to_string(), plugin2)
            .await
            .unwrap();

        // 跨插件重命名应该失败
        let result = server.rename("/test1/file.txt", "/test2/file.txt").await;
        assert!(result.is_err());
    }
}
