// WebDAV 协议 Plugin
//
// 基于 OpenDAL 的 WebDAV 插件

use crate::opendal::{OpendalPlugin, OpendalConfig, OpendalService};
use evif_core::{EvifPlugin, EvifResult, WriteFlags};

/// WebDAV 配置
#[derive(Clone, Debug)]
pub struct WebdavConfig {
    /// Endpoint (必需，例如: https://webdav.example.com)
    pub endpoint: String,

    /// 用户名 (可选，用于基本认证)
    pub username: Option<String>,

    /// 密码 (可选，用于基本认证)
    pub password: Option<String>,

    /// 根路径
    pub root: Option<String>,
}

impl Default for WebdavConfig {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            username: None,
            password: None,
            root: None,
        }
    }
}

/// WebDAV 插件
pub struct WebdavFsPlugin {
    inner: OpendalPlugin,
}

impl WebdavFsPlugin {
    /// 从配置创建 WebDAV 插件
    pub async fn from_config(config: WebdavConfig) -> EvifResult<Self> {
        let opendal_config = OpendalConfig {
            service: OpendalService::Webdav,
            root: config.root,
            endpoint: Some(config.endpoint),
            access_key: config.username,
            secret_key: config.password,
            ..Default::default()
        };

        let inner = OpendalPlugin::from_config(opendal_config).await?;

        Ok(Self { inner })
    }

    /// 获取内部 OpendalPlugin
    pub fn inner(&self) -> &OpendalPlugin {
        &self.inner
    }
}

#[async_trait::async_trait]
impl EvifPlugin for WebdavFsPlugin {
    fn name(&self) -> &str {
        "webdav-fs"
    }

    async fn create(&self, path: &str, perm: u32) -> EvifResult<()> {
        self.inner.create(path, perm).await
    }

    async fn mkdir(&self, path: &str, perm: u32) -> EvifResult<()> {
        self.inner.mkdir(path, perm).await
    }

    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>> {
        self.inner.read(path, offset, size).await
    }

    async fn write(&self, path: &str, data: Vec<u8>, offset: i64, flags: WriteFlags) -> EvifResult<u64> {
        self.inner.write(path, data, offset, flags).await
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<evif_core::FileInfo>> {
        self.inner.readdir(path).await
    }

    async fn stat(&self, path: &str) -> EvifResult<evif_core::FileInfo> {
        self.inner.stat(path).await
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        self.inner.remove(path).await
    }

    async fn rename(&self, from: &str, to: &str) -> EvifResult<()> {
        self.inner.rename(from, to).await
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        self.inner.remove_all(path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[cfg(feature = "opendal")]
    async fn test_webdav_config() {
        let config = WebdavConfig {
            endpoint: "https://webdav.example.com".to_string(),
            username: Some("testuser".to_string()),
            password: Some("testpass".to_string()),
            ..Default::default()
        };

        assert_eq!(config.endpoint, "https://webdav.example.com");
        assert_eq!(config.username.as_deref(), Some("testuser"));
        assert_eq!(config.password.as_deref(), Some("testpass"));
    }
}
