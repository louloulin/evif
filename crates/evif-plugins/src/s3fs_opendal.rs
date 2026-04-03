#![allow(unexpected_cfgs)]

// S3FS - 基于 OpenDAL 的 S3 云存储插件
//
// 使用 Apache OpenDAL 提供 AWS S3 和 S3 兼容存储支持

#[cfg(all(feature = "opendal", feature = "services-s3"))]
use crate::opendal::{OpendalConfig, OpendalService};

/// S3 配置 (基于 OpenDAL)
#[derive(Clone, Debug, Default)]
pub struct S3Config {
    /// Bucket 名称
    pub bucket: String,

    /// Region (可选)
    pub region: Option<String>,

    /// Endpoint (可选，用于 MinIO 等兼容存储)
    pub endpoint: Option<String>,

    /// Access Key ID (可选)
    pub access_key_id: Option<String>,

    /// Secret Access Key (可选)
    pub secret_access_key: Option<String>,

    /// 根路径/前缀 (可选)
    pub root: Option<String>,
}

/// S3FS 插件 (基于 OpenDAL)
#[cfg(all(feature = "opendal", feature = "services-s3"))]
pub struct S3FsPlugin {
    #[allow(dead_code)]
    inner: crate::opendal::OpendalPlugin,
}

#[cfg(not(all(feature = "opendal", feature = "services-s3")))]
pub struct S3FsPlugin;

#[cfg(all(feature = "opendal", feature = "services-s3"))]
impl S3FsPlugin {
    /// 从配置创建 S3FS 插件
    #[cfg(all(feature = "opendal", feature = "services-s3"))]
    pub async fn from_config(config: S3Config) -> EvifResult<Self> {
        let opendal_config = OpendalConfig {
            service: OpendalService::S3,
            bucket: Some(config.bucket),
            region: config.region,
            endpoint: config.endpoint,
            access_key: config.access_key_id,
            secret_key: config.secret_access_key,
            root: config.root,
            ..Default::default()
        };

        let inner = OpendalPlugin::from_config(opendal_config).await?;

        Ok(Self { inner })
    }

    /// 创建默认的 AWS S3 插件
    #[cfg(all(feature = "opendal", feature = "services-s3"))]
    pub async fn new_aws(bucket: &str, region: &str) -> EvifResult<Self> {
        let config = S3Config {
            bucket: bucket.to_string(),
            region: Some(region.to_string()),
            ..Default::default()
        };
        Self::from_config(config).await
    }

    /// 创建 MinIO 插件
    #[cfg(all(feature = "opendal", feature = "services-s3"))]
    pub async fn new_minio(
        bucket: &str,
        endpoint: &str,
        access_key_id: &str,
        secret_access_key: &str,
    ) -> EvifResult<Self> {
        let config = S3Config {
            bucket: bucket.to_string(),
            endpoint: Some(endpoint.to_string()),
            access_key_id: Some(access_key_id.to_string()),
            secret_access_key: Some(secret_access_key.to_string()),
            ..Default::default()
        };
        Self::from_config(config).await
    }
}

#[async_trait]
#[cfg(all(feature = "opendal", feature = "services-s3"))]
impl EvifPlugin for S3FsPlugin {
    fn name(&self) -> &str {
        "s3fs-opendal"
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

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        self.inner.readdir(path).await
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        self.inner.stat(path).await
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        self.inner.remove(path).await
    }

    async fn rename(&self, src: &str, dst: &str) -> EvifResult<()> {
        self.inner.rename(src, dst).await
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        self.inner.remove_all(path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = S3Config::default();
        assert!(config.bucket.is_empty());
        assert!(config.region.is_none());
        assert!(config.endpoint.is_none());
    }

    #[test]
    fn test_config_aws() {
        let config = S3Config {
            bucket: "my-bucket".to_string(),
            region: Some("us-west-2".to_string()),
            ..Default::default()
        };
        assert_eq!(config.bucket, "my-bucket");
        assert_eq!(config.region, Some("us-west-2".to_string()));
    }

    #[test]
    fn test_config_minio() {
        let config = S3Config {
            bucket: "test-bucket".to_string(),
            endpoint: Some("http://localhost:9000".to_string()),
            access_key_id: Some("minioadmin".to_string()),
            secret_access_key: Some("minioadmin".to_string()),
            ..Default::default()
        };
        assert_eq!(config.bucket, "test-bucket");
        assert_eq!(config.endpoint, Some("http://localhost:9000".to_string()));
    }
}
