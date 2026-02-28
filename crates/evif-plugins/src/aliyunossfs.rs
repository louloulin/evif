// 阿里云对象存储服务 (OSS) Plugin
//
// 基于 OpenDAL 的阿里云 OSS 插件

use crate::opendal::{OpendalPlugin, OpendalConfig, OpendalService};
use evif_core::{EvifPlugin, EvifResult, WriteFlags};

/// 阿里云 OSS 配置
#[derive(Clone, Debug)]
pub struct AliyunOssConfig {
    /// Bucket 名
    pub bucket: String,

    /// Access Key ID
    pub access_key_id: String,

    /// Access Key Secret
    pub access_key_secret: String,

    /// Endpoint (例如: oss-cn-hangzhou.aliyuncs.com)
    pub endpoint: Option<String>,

    /// 根路径
    pub root: Option<String>,
}

impl Default for AliyunOssConfig {
    fn default() -> Self {
        Self {
            bucket: String::new(),
            access_key_id: String::new(),
            access_key_secret: String::new(),
            endpoint: None,
            root: None,
        }
    }
}

/// 阿里云 OSS 插件
pub struct AliyunOssFsPlugin {
    inner: OpendalPlugin,
}

impl AliyunOssFsPlugin {
    /// 从配置创建阿里云 OSS 插件
    pub async fn from_config(config: AliyunOssConfig) -> EvifResult<Self> {
        let opendal_config = OpendalConfig {
            service: OpendalService::Oss,
            root: config.root,
            endpoint: config.endpoint,
            access_key: Some(config.access_key_id),
            secret_key: Some(config.access_key_secret),
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
impl EvifPlugin for AliyunOssFsPlugin {
    fn name(&self) -> &str {
        "aliyunoss-fs"
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
    async fn test_aliyunoss_config() {
        let config = AliyunOssConfig {
            bucket: "test-bucket".to_string(),
            access_key_id: "test-key-id".to_string(),
            access_key_secret: "test-key-secret".to_string(),
            endpoint: Some("oss-cn-hangzhou.aliyuncs.com".to_string()),
            ..Default::default()
        };

        assert_eq!(config.bucket, "test-bucket");
        assert_eq!(config.access_key_id, "test-key-id");
        assert_eq!(config.endpoint.as_deref(), Some("oss-cn-hangzhou.aliyuncs.com"));
    }
}
