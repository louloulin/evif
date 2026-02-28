// FTP 文件传输协议 Plugin
//
// 基于 OpenDAL 的 FTP 插件

use crate::opendal::{OpendalPlugin, OpendalConfig, OpendalService};
use evif_core::{EvifPlugin, EvifResult, WriteFlags};

/// FTP 配置
#[derive(Clone, Debug)]
pub struct FtpConfig {
    /// Endpoint (必需，例如: ftp://ftp.example.com:21)
    pub endpoint: String,

    /// 用户名 (可选，匿名访问则为 None)
    pub username: Option<String>,

    /// 密码 (可选，匿名访问则为 None)
    pub password: Option<String>,

    /// 根路径
    pub root: Option<String>,
}

impl Default for FtpConfig {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            username: None,
            password: None,
            root: None,
        }
    }
}

/// FTP 插件
pub struct FtpFsPlugin {
    inner: OpendalPlugin,
}

impl FtpFsPlugin {
    /// 从配置创建 FTP 插件
    pub async fn from_config(config: FtpConfig) -> EvifResult<Self> {
        let opendal_config = OpendalConfig {
            service: OpendalService::Ftp,
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
impl EvifPlugin for FtpFsPlugin {
    fn name(&self) -> &str {
        "ftp-fs"
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
    async fn test_ftp_config() {
        let config = FtpConfig {
            endpoint: "ftp://ftp.example.com:21".to_string(),
            username: Some("ftpuser".to_string()),
            password: Some("ftppass".to_string()),
            ..Default::default()
        };

        assert_eq!(config.endpoint, "ftp://ftp.example.com:21");
        assert_eq!(config.username.as_deref(), Some("ftpuser"));
        assert_eq!(config.password.as_deref(), Some("ftppass"));
    }

    #[tokio::test]
    #[cfg(feature = "opendal")]
    async fn test_ftp_anonymous_config() {
        let config = FtpConfig {
            endpoint: "ftp://ftp.example.com:21".to_string(),
            ..Default::default()
        };

        assert_eq!(config.endpoint, "ftp://ftp.example.com:21");
        assert!(config.username.is_none());
        assert!(config.password.is_none());
    }
}
