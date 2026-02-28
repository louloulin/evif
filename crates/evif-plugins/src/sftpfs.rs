// SFTP (SSH File Transfer Protocol) Plugin
//
// 基于 OpenDAL 的 SFTP 插件

use crate::opendal::{OpendalPlugin, OpendalConfig, OpendalService};
use evif_core::{EvifPlugin, EvifResult, WriteFlags};

/// SFTP 配置
#[derive(Clone, Debug)]
pub struct SftpConfig {
    /// Endpoint (必需，例如: sftp://example.com:22)
    pub endpoint: String,

    /// 用户名 (必需)
    pub username: String,

    /// 密码 (可选，如果使用密钥认证则为 None)
    pub password: Option<String>,

    /// 根路径
    pub root: Option<String>,
}

impl Default for SftpConfig {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            username: String::new(),
            password: None,
            root: None,
        }
    }
}

/// SFTP 插件
pub struct SftpFsPlugin {
    inner: OpendalPlugin,
}

impl SftpFsPlugin {
    /// 从配置创建 SFTP 插件
    pub async fn from_config(config: SftpConfig) -> EvifResult<Self> {
        let opendal_config = OpendalConfig {
            service: OpendalService::Sftp,
            root: config.root,
            endpoint: Some(config.endpoint),
            access_key: Some(config.username),
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
impl EvifPlugin for SftpFsPlugin {
    fn name(&self) -> &str {
        "sftp-fs"
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
    async fn test_sftp_config() {
        let config = SftpConfig {
            endpoint: "sftp://example.com:22".to_string(),
            username: "sshuser".to_string(),
            password: Some("sshpass".to_string()),
            ..Default::default()
        };

        assert_eq!(config.endpoint, "sftp://example.com:22");
        assert_eq!(config.username, "sshuser");
        assert_eq!(config.password.as_deref(), Some("sshpass"));
    }

    #[tokio::test]
    #[cfg(feature = "opendal")]
    async fn test_sftp_key_based_config() {
        let config = SftpConfig {
            endpoint: "sftp://example.com:22".to_string(),
            username: "sshuser".to_string(),
            password: None,  // 使用密钥认证
            ..Default::default()
        };

        assert_eq!(config.endpoint, "sftp://example.com:22");
        assert_eq!(config.username, "sshuser");
        assert!(config.password.is_none());
    }
}
