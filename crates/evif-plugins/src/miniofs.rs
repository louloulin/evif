// MinIO 对象存储 Plugin
//
// 基于 OpenDAL 的 MinIO 插件 (使用 S3 兼容协议)

use crate::opendal::{OpendalPlugin, OpendalConfig, OpendalService};
use evif_core::{EvifPlugin, EvifResult, WriteFlags};

/// MinIO 配置
#[derive(Clone, Debug)]
pub struct MinioConfig {
    /// Bucket 名
    pub bucket: String,

    /// Access Key
    pub access_key: String,

    /// Secret Key
    pub secret_key: String,

    /// Endpoint (必需，例如: http://localhost:9000)
    pub endpoint: String,

    /// Region (可选，默认 us-east-1)
    pub region: Option<String>,

    /// 根路径
    pub root: Option<String>,
}

impl Default for MinioConfig {
    fn default() -> Self {
        Self {
            bucket: String::new(),
            access_key: String::new(),
            secret_key: String::new(),
            endpoint: String::new(),
            region: Some("us-east-1".to_string()),
            root: None,
        }
    }
}

/// MinIO 插件
pub struct MinioFsPlugin {
    inner: OpendalPlugin,
}

impl MinioFsPlugin {
    /// 从配置创建 MinIO 插件
    pub async fn from_config(config: MinioConfig) -> EvifResult<Self> {
        let opendal_config = OpendalConfig {
            service: OpendalService::S3,
            root: config.root,
            endpoint: Some(config.endpoint),
            access_key: Some(config.access_key),
            secret_key: Some(config.secret_key),
            bucket: Some(config.bucket),
            region: config.region,
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
impl EvifPlugin for MinioFsPlugin {
    fn name(&self) -> &str {
        "minio-fs"
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
    async fn test_minio_config() {
        let config = MinioConfig {
            bucket: "test-bucket".to_string(),
            access_key: "minioadmin".to_string(),
            secret_key: "minioadmin".to_string(),
            endpoint: "http://localhost:9000".to_string(),
            ..Default::default()
        };

        assert_eq!(config.bucket, "test-bucket");
        assert_eq!(config.endpoint, "http://localhost:9000");
        assert_eq!(config.region.as_deref(), Some("us-east-1"));
    }
}
