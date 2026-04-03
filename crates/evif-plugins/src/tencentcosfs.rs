// 腾讯云对象存储服务 (COS) Plugin
//
// 基于 OpenDAL 的腾讯云 COS 插件

use crate::opendal::{OpendalPlugin, OpendalConfig, OpendalService};
use evif_core::{EvifPlugin, EvifResult, WriteFlags};

/// 腾讯云 COS 配置
#[derive(Clone, Debug, Default)]
pub struct TencentCosConfig {
    /// Bucket 名
    pub bucket: String,

    /// Secret ID
    pub secret_id: String,

    /// Secret Key
    pub secret_key: String,

    /// Endpoint (例如: cos.ap-guangzhou.myqcloud.com)
    pub endpoint: Option<String>,

    /// Region (例如: ap-guangzhou)
    pub region: Option<String>,

    /// 根路径
    pub root: Option<String>,
}

/// 腾讯云 COS 插件
pub struct TencentCosFsPlugin {
    inner: OpendalPlugin,
}

impl TencentCosFsPlugin {
    /// 从配置创建腾讯云 COS 插件
    pub async fn from_config(config: TencentCosConfig) -> EvifResult<Self> {
        let opendal_config = OpendalConfig {
            service: OpendalService::Cos,
            root: config.root,
            endpoint: config.endpoint,
            access_key: Some(config.secret_id),
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
impl EvifPlugin for TencentCosFsPlugin {
    fn name(&self) -> &str {
        "tencentcos-fs"
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
    async fn test_tencentcos_config() {
        let config = TencentCosConfig {
            bucket: "test-bucket".to_string(),
            secret_id: "test-secret-id".to_string(),
            secret_key: "test-secret-key".to_string(),
            region: Some("ap-guangzhou".to_string()),
            ..Default::default()
        };

        assert_eq!(config.bucket, "test-bucket");
        assert_eq!(config.secret_id, "test-secret-id");
        assert_eq!(config.region.as_deref(), Some("ap-guangzhou"));
    }
}
