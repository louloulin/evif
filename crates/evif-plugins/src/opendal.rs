// OpenDAL Plugin - 基于 Apache OpenDAL 的统一存储插件
//
// 支持 50+ 存储后端，配置驱动，统一接口
// 参考: https://opendal.apache.org/

use evif_core::{EvifPlugin, FileInfo, WriteFlags, EvifResult, EvifError};
use async_trait::async_trait;

#[cfg(feature = "opendal")]
use opendal::Operator;

/// OpenDAL 支持的服务类型 (简化版本，先支持核心服务)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpendalService {
    /// 内存存储 (默认支持)
    Memory,

    /// 本地文件系统 (默认支持)
    Fs,

    /// AWS S3
    S3,

    /// Azure Blob Storage
    Azblob,

    /// Google Cloud Storage
    Gcs,

    /// 阿里云对象存储 (OSS)
    Oss,

    /// 腾讯云对象存储 (COS)
    Cos,

    /// 华为云对象存储 (OBS)
    Obs,

    // WebDAV, FTP, SFTP 暂时禁用，等待 OpenDAL 0.50.x TLS 冲突修复
    // Webdav,
    // Ftp,
    // Sftp,
}

/// OpenDAL 配置
#[derive(Clone, Debug)]
pub struct OpendalConfig {
    /// 服务类型
    pub service: OpendalService,

    /// 根路径
    pub root: Option<String>,

    /// Endpoint (可选，用于兼容性存储)
    pub endpoint: Option<String>,

    /// 访问密钥
    pub access_key: Option<String>,

    /// 密钥
    pub secret_key: Option<String>,

    /// Bucket/容器名称
    pub bucket: Option<String>,

    /// Region
    pub region: Option<String>,

    /// 启用缓存
    pub enable_cache: bool,

    /// 缓存最大条目数
    pub cache_max_entries: usize,

    /// 缓存 TTL (秒)
    pub cache_ttl_secs: u64,
}

impl Default for OpendalConfig {
    fn default() -> Self {
        Self {
            service: OpendalService::Memory,
            root: None,
            endpoint: None,
            access_key: None,
            secret_key: None,
            bucket: None,
            region: None,
            enable_cache: true,
            cache_max_entries: 1000,
            cache_ttl_secs: 3600,
        }
    }
}

/// OpenDAL 插件
pub struct OpendalPlugin {
    operator: Operator,
    config: OpendalConfig,
}

impl OpendalPlugin {
    /// 从配置创建 OpenDAL 插件
    #[cfg(feature = "opendal")]
    pub async fn from_config(config: OpendalConfig) -> EvifResult<Self> {
        let operator = Self::build_operator(&config).await?;

        Ok(Self {
            operator,
            config,
        })
    }

    /// 构建 OpenDAL Operator (仅支持基础服务)
    #[cfg(feature = "opendal")]
    async fn build_operator(config: &OpendalConfig) -> EvifResult<Operator> {
        match config.service {
            OpendalService::Memory => {
                let builder = opendal::services::Memory::default();
                Ok(Operator::new(builder).map_err(|e| {
                    EvifError::Other(format!("Failed to create Memory operator: {}", e))
                })?.finish())
            }

            OpendalService::Fs => {
                let mut builder = opendal::services::Fs::default();
                // 设置 root 路径
                if let Some(root) = &config.root {
                    builder = builder.root(root);
                }
                Ok(Operator::new(builder).map_err(|e| {
                    EvifError::Other(format!("Failed to create Fs operator: {}", e))
                })?.finish())
            }

            OpendalService::S3 => {
                use opendal::services::S3;

                // 获取 bucket (必需)
                let bucket = config.bucket.as_ref().ok_or_else(|| {
                    EvifError::Other("S3 service requires 'bucket' in config".to_string())
                })?;

                // 构建 S3 builder
                let builder = S3::default()
                    .bucket(bucket);

                // 添加可选配置
                let builder = if let Some(region) = &config.region {
                    builder.region(region)
                } else {
                    builder
                };

                let builder = if let Some(endpoint) = &config.endpoint {
                    builder.endpoint(endpoint)
                } else {
                    builder
                };

                // OpenDAL 0.50 的凭证设置方式
                // 需要通过环境变量或 Config 设置
                // 这里我们使用环境变量方式，让用户设置:
                // AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_REGION
                Ok(Operator::new(builder).map_err(|e| {
                    EvifError::Other(format!("Failed to create S3 operator: {}", e))
                })?.finish())
            }

            OpendalService::Azblob => {
                use opendal::services::Azblob;

                // 获取 container (必需)
                let container = config.bucket.as_ref().ok_or_else(|| {
                    EvifError::Other("Azblob service requires 'container' (bucket) in config".to_string())
                })?;

                // 构建 Azblob builder
                let builder = Azblob::default()
                    .container(container);

                // 添加 endpoint (可选)
                let builder = if let Some(endpoint) = &config.endpoint {
                    builder.endpoint(endpoint)
                } else {
                    builder
                };

                // 添加账户名和密钥
                let builder = if let (Some(account_name), Some(account_key)) = (&config.access_key, &config.secret_key) {
                    builder.account_name(account_name).account_key(account_key)
                } else {
                    builder
                };

                Ok(Operator::new(builder).map_err(|e| {
                    EvifError::Other(format!("Failed to create Azblob operator: {}", e))
                })?.finish())
            }

            OpendalService::Gcs => {
                use opendal::services::Gcs;

                // 获取 bucket (必需)
                let bucket = config.bucket.as_ref().ok_or_else(|| {
                    EvifError::Other("Gcs service requires 'bucket' in config".to_string())
                })?;

                // 构建 Gcs builder
                let builder = Gcs::default()
                    .bucket(bucket);

                // 添加 endpoint (可选)
                let builder = if let Some(endpoint) = &config.endpoint {
                    builder.endpoint(endpoint)
                } else {
                    builder
                };

                // 添加凭证（通过环境变量或配置）
                // GCS 支持服务账号凭证，这里简化处理
                Ok(Operator::new(builder).map_err(|e| {
                    EvifError::Other(format!("Failed to create Gcs operator: {}", e))
                })?.finish())
            }

            OpendalService::Oss => {
                use opendal::services::Oss;

                // 获取 bucket (必需)
                let bucket = config.bucket.as_ref().ok_or_else(|| {
                    EvifError::Other("Oss service requires 'bucket' in config".to_string())
                })?;

                // 构建 Oss builder
                let builder = Oss::default()
                    .bucket(bucket);

                // 添加 endpoint (可选)
                let builder = if let Some(endpoint) = &config.endpoint {
                    builder.endpoint(endpoint)
                } else {
                    builder
                };

                // 添加访问密钥
                let builder = if let (Some(access_key), Some(secret_key)) = (&config.access_key, &config.secret_key) {
                    builder.access_key_id(access_key).access_key_secret(secret_key)
                } else {
                    builder
                };

                Ok(Operator::new(builder).map_err(|e| {
                    EvifError::Other(format!("Failed to create Oss operator: {}", e))
                })?.finish())
            }

            OpendalService::Cos => {
                use opendal::services::Cos;

                // 获取 bucket (必需)
                let bucket = config.bucket.as_ref().ok_or_else(|| {
                    EvifError::Other("Cos service requires 'bucket' in config".to_string())
                })?;

                // 构建 Cos builder
                let builder = Cos::default()
                    .bucket(bucket);

                // 添加 endpoint (可选)
                let builder = if let Some(endpoint) = &config.endpoint {
                    builder.endpoint(endpoint)
                } else {
                    builder
                };

                // 添加访问密钥
                let builder = if let (Some(access_key), Some(secret_key)) = (&config.access_key, &config.secret_key) {
                    builder.secret_id(access_key).secret_key(secret_key)
                } else {
                    builder
                };

                Ok(Operator::new(builder).map_err(|e| {
                    EvifError::Other(format!("Failed to create Cos operator: {}", e))
                })?.finish())
            }

            OpendalService::Obs => {
                use opendal::services::Obs;

                // 获取 bucket (必需)
                let bucket = config.bucket.as_ref().ok_or_else(|| {
                    EvifError::Other("Obs service requires 'bucket' in config".to_string())
                })?;

                // 构建 Obs builder
                let builder = Obs::default()
                    .bucket(bucket);

                // 添加 endpoint (可选)
                let builder = if let Some(endpoint) = &config.endpoint {
                    builder.endpoint(endpoint)
                } else {
                    builder
                };

                // 添加访问密钥
                let builder = if let (Some(access_key), Some(secret_key)) = (&config.access_key, &config.secret_key) {
                    builder.access_key_id(access_key).secret_access_key(secret_key)
                } else {
                    builder
                };

                Ok(Operator::new(builder).map_err(|e| {
                    EvifError::Other(format!("Failed to create Obs operator: {}", e))
                })?.finish())
            }

//             OpendalService::Webdav => {
//                 use opendal::services::Webdav;
// 
//                 // 获取 endpoint (必需)
//                 let endpoint = config.endpoint.as_ref().ok_or_else(|| {
//                     EvifError::Other("Webdav service requires 'endpoint' in config".to_string())
//                 })?;
// 
//                 // 构建 Webdav builder
//                 let builder = Webdav::default()
//                     .endpoint(endpoint);
// 
//                 // 添加访问密钥（如果需要认证）
//                 let builder = if let (Some(access_key), Some(secret_key)) = (&config.access_key, &config.secret_key) {
//                     builder.username(access_key).password(secret_key)
//                 } else {
//                     builder
//                 };
// 
//                 // 添加根路径
//                 let builder = if let Some(root) = &config.root {
//                     builder.root(root)
//                 } else {
//                     builder
//                 };
// 
//                 Ok(Operator::new(builder).map_err(|e| {
//                     EvifError::Other(format!("Failed to create Webdav operator: {}", e))
//                 })?.finish())
//             }
//
//             OpendalService::Ftp => {
//                 use opendal::services::Ftp;
// 
//                 // 获取 endpoint (必需)
//                 let endpoint = config.endpoint.as_ref().ok_or_else(|| {
//                     EvifError::Other("Ftp service requires 'endpoint' in config".to_string())
//                 })?;
// 
//                 // 构建 Ftp builder
//                 let builder = Ftp::default()
//                     .endpoint(endpoint);
// 
//                 // 添加访问密钥（如果需要认证）
//                 let builder = if let (Some(access_key), Some(secret_key)) = (&config.access_key, &config.secret_key) {
//                     builder.username(access_key).password(secret_key)
//                 } else {
//                     builder
//                 };
// 
//                 // 添加根路径
//                 let builder = if let Some(root) = &config.root {
//                     builder.root(root)
//                 } else {
//                     builder
//                 };
// 
//                 Ok(Operator::new(builder).map_err(|e| {
//                     EvifError::Other(format!("Failed to create Ftp operator: {}", e))
//                 })?.finish())
//             }
//
//             OpendalService::Sftp => {
//                 use opendal::services::Sftp;
//
//                 // 获取 endpoint (必需)
//                 let endpoint = config.endpoint.as_ref().ok_or_else(|| {
//                     EvifError::Other("Sftp service requires 'endpoint' in config".to_string())
//                 })?;
//
//                 // 构建 Sftp builder
//                 let builder = Sftp::default()
//                     .endpoint(endpoint);
//
//                 // 添加访问密钥（如果需要认证）
//                 let builder = if let (Some(access_key), Some(secret_key)) = (&config.access_key, &config.secret_key) {
//                     builder.username(access_key).password(secret_key)
//                 } else {
//                     builder
//                 };
//
//                 // 添加根路径
//                 let builder = if let Some(root) = &config.root {
//                     builder.root(root)
//                 } else {
//                     builder
//                 };
//
//                 Ok(Operator::new(builder).map_err(|e| {
//                     EvifError::Other(format!("Failed to create Sftp operator: {}", e))
//                 })?.finish())
//             }
        }
    }

    /// 获取配置
    pub fn config(&self) -> &OpendalConfig {
        &self.config
    }
}

#[async_trait]
#[cfg(feature = "opendal")]
impl EvifPlugin for OpendalPlugin {
    fn name(&self) -> &str {
        "opendal"
    }

    async fn create(&self, path: &str, _perm: u32) -> EvifResult<()> {
        self.write(path, Vec::new(), -1, WriteFlags::empty()).await?;
        Ok(())
    }

    async fn mkdir(&self, path: &str, _perm: u32) -> EvifResult<()> {
        let full_path = if let Some(root) = &self.config.root {
            format!("{}/{}", root.trim_end_matches('/'), path.trim_start_matches('/'))
        } else {
            path.to_string()
        };

        // OpenDAL 要求目录路径以 / 结尾
        let dir_path = if !full_path.ends_with('/') {
            format!("{}/", full_path)
        } else {
            full_path
        };

        self.operator.create_dir(&dir_path).await.map_err(|e| {
            EvifError::Other(format!("OpenDAL mkdir error: {}", e))
        })?;

        Ok(())
    }

    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>> {
        let full_path = if let Some(root) = &self.config.root {
            format!("{}/{}", root.trim_end_matches('/'), path.trim_start_matches('/'))
        } else {
            path.to_string()
        };

        let data = self.operator.read(&full_path).await.map_err(|e| {
            EvifError::Other(format!("OpenDAL read error: {}", e))
        })?;

        // 将 Buffer 转换为 Vec 以便索引
        let vec = data.to_vec();

        // 处理 offset 和 size
        let start = offset as usize;
        let end = if size == 0 {
            vec.len()
        } else {
            std::cmp::min((offset + size) as usize, vec.len())
        };

        if start >= vec.len() {
            return Ok(Vec::new());
        }

        Ok(vec[start..end].to_vec())
    }

    async fn write(&self, path: &str, data: Vec<u8>, offset: i64, _flags: WriteFlags) -> EvifResult<u64> {
        let full_path = if let Some(root) = &self.config.root {
            format!("{}/{}", root.trim_end_matches('/'), path.trim_start_matches('/'))
        } else {
            path.to_string()
        };

        if offset >= 0 {
            // 如果指定了 offset，需要先读取现有数据，然后写入指定位置
            let mut existing_data = self.operator.read(&full_path).await.unwrap_or_default();
            let offset_usize = offset as usize;

            // 将 Buffer 转换为 Vec 以便修改
            let mut vec_data = existing_data.to_vec();

            // 扩展数据如果需要
            if offset_usize + data.len() > vec_data.len() {
                vec_data.resize(offset_usize + data.len(), 0);
            }

            // 写入指定位置
            vec_data[offset_usize..offset_usize + data.len()].copy_from_slice(&data);
            self.operator.write(&full_path, vec_data).await.map_err(|e| {
                EvifError::Other(format!("OpenDAL write error: {}", e))
            })?;
        } else {
            // 直接写入
            self.operator.write(&full_path, data.clone()).await.map_err(|e| {
                EvifError::Other(format!("OpenDAL write error: {}", e))
            })?;
        }

        Ok(data.len() as u64)
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let full_path = if let Some(root) = &self.config.root {
            format!("{}/{}", root.trim_end_matches('/'), path.trim_start_matches('/'))
        } else {
            path.to_string()
        };

        let lister = self.operator.lister(&full_path).await.map_err(|e| {
            EvifError::Other(format!("OpenDAL readdir error: {}", e))
        })?;

        let mut files = Vec::new();
        let mut entries = lister;

        // OpenDAL 0.50 的 Lister 实现了 Stream
        use futures::StreamExt;
        while let Some(entry) = entries.next().await {
            let entry = entry.map_err(|e| {
                EvifError::Other(format!("OpenDAL entry error: {}", e))
            })?;

            let name = entry.name().to_string();
            let metadata = entry.metadata();
            let is_dir = metadata.is_dir();

            // 使用 stat 获取完整的文件信息（包括大小）
            let entry_path = format!("{}/{}", full_path.trim_end_matches('/'), name);
            let file_info = if let Ok(info) = self.stat(&entry_path).await {
                info
            } else {
                // 如果 stat 失败，返回基本信息
                FileInfo {
                    name,
                    size: 0,
                    mode: 0,
                    modified: chrono::Utc::now(),
                    is_dir,
                }
            };

            files.push(file_info);
        }

        Ok(files)
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let full_path = if let Some(root) = &self.config.root {
            format!("{}/{}", root.trim_end_matches('/'), path.trim_start_matches('/'))
        } else {
            path.to_string()
        };

        let metadata = self.operator.stat(&full_path).await.map_err(|e| {
            EvifError::Other(format!("OpenDAL stat error: {}", e))
        })?;

        let name = full_path.split('/').last().unwrap_or_default().to_string();
        let is_dir = metadata.is_dir();

        // 尝试获取文件大小
        // 对于 Memory 服务，我们通过读取数据来获取大小
        let size = if !is_dir {
            match self.operator.read(&full_path).await {
                Ok(data) => data.len() as u64,
                Err(_) => 0,
            }
        } else {
            0
        };

        Ok(FileInfo {
            name,
            size,
            mode: 0,
            modified: chrono::Utc::now(),
            is_dir,
        })
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        let full_path = if let Some(root) = &self.config.root {
            format!("{}/{}", root.trim_end_matches('/'), path.trim_start_matches('/'))
        } else {
            path.to_string()
        };

        self.operator.delete(&full_path).await.map_err(|e| {
            EvifError::Other(format!("OpenDAL remove error: {}", e))
        })?;

        Ok(())
    }

    async fn rename(&self, from: &str, to: &str) -> EvifResult<()> {
        let from_path = if let Some(root) = &self.config.root {
            format!("{}/{}", root.trim_end_matches('/'), from.trim_start_matches('/'))
        } else {
            from.to_string()
        };

        let to_path = if let Some(root) = &self.config.root {
            format!("{}/{}", root.trim_end_matches('/'), to.trim_start_matches('/'))
        } else {
            to.to_string()
        };

        self.operator.rename(&from_path, &to_path).await.map_err(|e| {
            EvifError::Other(format!("OpenDAL rename error: {}", e))
        })?;

        Ok(())
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        let full_path = if let Some(root) = &self.config.root {
            format!("{}/{}", root.trim_end_matches('/'), path.trim_start_matches('/'))
        } else {
            path.to_string()
        };

        // 先尝试删除所有子项
        let entries = self.readdir(path).await.unwrap_or_default();
        for entry in entries {
            let child_path = format!("{}/{}", path.trim_end_matches('/'), entry.name);
            self.remove_all(&child_path).await?;
        }

        // 然后删除当前路径
        self.operator.delete(&full_path).await.map_err(|e| {
            EvifError::Other(format!("OpenDAL remove_all error: {}", e))
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[cfg(feature = "opendal")]
    async fn test_memory_backend() {
        let config = OpendalConfig {
            service: OpendalService::Memory,
            ..Default::default()
        };

        let plugin = OpendalPlugin::from_config(config).await.unwrap();

        // 创建文件测试
        plugin.create("/test.txt", 0o644).await.unwrap();

        // 写入测试
        plugin.write("/test.txt", b"Hello, OpenDAL!".to_vec(), -1, WriteFlags::empty()).await.unwrap();

        // 读取测试
        let data = plugin.read("/test.txt", 0, 0).await.unwrap();
        assert_eq!(data, b"Hello, OpenDAL!");

        // Stat 测试
        let info = plugin.stat("/test.txt").await.unwrap();
        assert_eq!(info.size, 15);
        assert!(!info.is_dir);

        // Readdir 测试
        let files = plugin.readdir("/").await.unwrap();
        assert_eq!(files.len(), 1);

        // 删除测试
        plugin.remove("/test.txt").await.unwrap();
        assert!(plugin.readdir("/").await.unwrap().is_empty());
    }
}
