// Azure Blob Storage Plugin
//
// 基于 OpenDAL 的 Azure Blob Storage 插件

use crate::opendal::{OpendalPlugin, OpendalConfig, OpendalService};
use evif_core::{EvifPlugin, EvifResult, WriteFlags};

/// Azure Blob Storage 配置
#[derive(Clone, Debug)]
pub struct AzureBlobConfig {
    /// 账户名
    pub account_name: String,

    /// 账户密钥
    pub account_key: String,

    /// 容器名
    pub container: String,

    /// Endpoint (可选，用于 Azure Stack 等)
    pub endpoint: Option<String>,

    /// 根路径
    pub root: Option<String>,
}

impl Default for AzureBlobConfig {
    fn default() -> Self {
        Self {
            account_name: String::new(),
            account_key: String::new(),
            container: String::new(),
            endpoint: None,
            root: None,
        }
    }
}

/// Azure Blob Storage 插件
pub struct AzureBlobFsPlugin {
    inner: OpendalPlugin,
}

impl AzureBlobFsPlugin {
    /// 从配置创建 Azure Blob Storage 插件
    pub async fn from_config(config: AzureBlobConfig) -> EvifResult<Self> {
        let opendal_config = OpendalConfig {
            service: OpendalService::Azblob,
            mount_point: "/azureblob".to_string(),
            root: config.root,
            endpoint: config.endpoint,
            access_key: Some(config.account_name),
            secret_key: Some(config.account_key),
            bucket: Some(config.container),
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
impl EvifPlugin for AzureBlobFsPlugin {
    fn name(&self) -> &str {
        "azureblob-fs"
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
    async fn test_azureblob_config() {
        let config = AzureBlobConfig {
            account_name: "testaccount".to_string(),
            account_key: "testkey".to_string(),
            container: "testcontainer".to_string(),
            ..Default::default()
        };

        assert_eq!(config.account_name, "testaccount");
        assert_eq!(config.container, "testcontainer");
    }
}
