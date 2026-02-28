// Google Cloud Storage Plugin
//
// 基于 OpenDAL 的 Google Cloud Storage 插件

use crate::opendal::{OpendalPlugin, OpendalConfig, OpendalService};
use evif_core::{EvifPlugin, EvifResult, WriteFlags};

/// Google Cloud Storage 配置
#[derive(Clone, Debug)]
pub struct GcsConfig {
    /// Bucket 名
    pub bucket: String,

    /// Endpoint (可选，用于兼容服务)
    pub endpoint: Option<String>),

    /// 根路径
    pub root: Option<String>,
}

impl Default for GcsConfig {
    fn default() -> Self {
        Self {
            bucket: String::new(),
            endpoint: None,
            root: None,
        }
    }
}

/// Google Cloud Storage 插件
pub struct GcsFsPlugin {
    inner: OpendalPlugin,
}

impl GcsFsPlugin {
    /// 从配置创建 GCS 插件
    pub async fn from_config(config: GcsConfig) -> EvifResult<Self> {
        let opendal_config = OpendalConfig {
            service: OpendalService::Gcs,
            mount_point: "/gcs".to_string(),
            root: config.root,
            endpoint: config.endpoint,
            bucket: Some(config.bucket),
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
impl EvifPlugin for GcsFsPlugin {
    fn name(&self) -> &str {
        "gcs-fs"
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
    async fn test_gcs_config() {
        let config = GcsConfig {
            bucket: "test-bucket".to_string(),
            ..Default::default()
        };

        assert_eq!(config.bucket, "test-bucket");
    }
}
