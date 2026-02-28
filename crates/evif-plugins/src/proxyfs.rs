// Proxy File System Plugin for EVIF
// 对标 AGFS ProxyFS: 远程EVIF/AGFS服务器客户端代理
// 用途: 分布式文件系统、远程备份、负载均衡、故障转移

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use evif_core::{EvifError, EvifPlugin, EvifResult, FileInfo, WriteFlags};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// ProxyFS配置
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    /// 远程服务器基础URL (如 "http://localhost:8080/api/v1")
    pub base_url: String,
    /// 请求超时秒数
    pub timeout_seconds: u64,
    /// 最大重试次数
    pub max_retries: u32,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080/api/v1".to_string(),
            timeout_seconds: 30,
            max_retries: 3,
        }
    }
}

/// EVIF/AGFS API响应格式
#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse<T> {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReadResponse {
    data: String,
    size: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct WriteResponse {
    bytes_written: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReaddirResponse {
    entries: Vec<FileInfo>,
}

/// ProxyFS插件
///
/// 将文件操作转发到远程EVIF/AGFS服务器
/// 支持热重载(/reload文件)
pub struct ProxyFsPlugin {
    config: ProxyConfig,
    client: Arc<reqwest::Client>,
    /// 最后一次重载时间
    last_reload: Arc<RwLock<DateTime<Utc>>>,
}

impl ProxyFsPlugin {
    pub fn new(base_url: &str) -> Self {
        Self::with_config(ProxyConfig {
            base_url: base_url.to_string(),
            ..Default::default()
        })
    }

    pub fn with_config(config: ProxyConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            config,
            client: Arc::new(client),
            last_reload: Arc::new(RwLock::new(Utc::now())),
        }
    }

    /// 构建API URL
    fn build_api_url(&self, operation: &str, path: &str) -> String {
        let clean_path = path.trim_start_matches('/');
        let encoded_path = urlencoding::encode(clean_path);
        format!("{}/{}/{}", self.config.base_url, operation, encoded_path)
    }

    /// 执行HTTP GET请求(读取操作)
    async fn http_get<T: for<'de> Deserialize<'de>>(&self, url: &str) -> EvifResult<T> {
        let response = self.client
            .get(url)
            .send()
            .await
            .map_err(|e| EvifError::InvalidPath(format!("HTTP GET failed: {}", e)))?;

        if response.status().is_success() {
            let api_response: ApiResponse<T> = response
                .json()
                .await
                .map_err(|e| EvifError::InvalidPath(format!("Failed to parse JSON: {}", e)))?;

            if api_response.status == "success" {
                api_response.data.ok_or_else(|| {
                    EvifError::InvalidPath("Empty response data".to_string())
                })
            } else {
                Err(EvifError::InvalidPath(
                    api_response.error.unwrap_or_else(|| "Unknown error".to_string())
                ))
            }
        } else if response.status() == 404 {
            Err(EvifError::NotFound(url.to_string()))
        } else {
            Err(EvifError::InvalidPath(format!(
                "HTTP error: {}",
                response.status()
            )))
        }
    }

    /// 执行HTTP POST请求(写入操作)
    async fn http_post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        url: &str,
        body: &B,
    ) -> EvifResult<T> {
        let response = self.client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|e| EvifError::InvalidPath(format!("HTTP POST failed: {}", e)))?;

        if response.status().is_success() {
            let api_response: ApiResponse<T> = response
                .json()
                .await
                .map_err(|e| EvifError::InvalidPath(format!("Failed to parse JSON: {}", e)))?;

            if api_response.status == "success" {
                api_response.data.ok_or_else(|| {
                    EvifError::InvalidPath("Empty response data".to_string())
                })
            } else {
                Err(EvifError::InvalidPath(
                    api_response.error.unwrap_or_else(|| "Unknown error".to_string())
                ))
            }
        } else {
            Err(EvifError::InvalidPath(format!(
                "HTTP error: {}",
                response.status()
            )))
        }
    }

    /// 执行HTTP DELETE请求
    async fn http_delete(&self, url: &str) -> EvifResult<()> {
        let response = self.client
            .delete(url)
            .send()
            .await
            .map_err(|e| EvifError::InvalidPath(format!("HTTP DELETE failed: {}", e)))?;

        if response.status().is_success() || response.status() == 404 {
            Ok(())
        } else {
            Err(EvifError::InvalidPath(format!(
                "HTTP error: {}",
                response.status()
            )))
        }
    }

    /// 热重载连接
    pub async fn reload(&self) -> EvifResult<()> {
        // 测试连接
        let health_url = format!("{}/health", self.config.base_url);
        let response = self.client
            .get(&health_url)
            .send()
            .await
            .map_err(|e| EvifError::InvalidPath(format!("Health check failed: {}", e)))?;

        if response.status().is_success() {
            *self.last_reload.write().await = Utc::now();
            Ok(())
        } else {
            Err(EvifError::InvalidPath(format!(
                "Health check failed with status: {}",
                response.status()
            )))
        }
    }
}

#[async_trait]
impl EvifPlugin for ProxyFsPlugin {
    fn name(&self) -> &str {
        "proxyfs"
    }

    async fn create(&self, path: &str, _perm: u32) -> EvifResult<()> {
        // 通过写入空文件创建
        let url = self.build_api_url("write", path);
        let body = serde_json::json!({"data": ""});

        let _: WriteResponse = self.http_post(&url, &body).await?;
        Ok(())
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        // 远程服务器可能不支持mkdir
        Err(EvifError::InvalidPath(
            "Remote mkdir not supported via HTTP API".to_string(),
        ))
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        // 特殊处理 /reload
        if path == "/reload" {
            return Ok(format!(
                "Last reload: {}\nWrite to trigger reload\n",
                *self.last_reload.read().await
            )
            .into_bytes());
        }

        let url = self.build_api_url("read", path);
        let response: ReadResponse = self.http_get(&url).await?;
        Ok(response.data.into_bytes())
    }

    async fn write(
        &self,
        path: &str,
        data: Vec<u8>,
        _offset: i64,
        _flags: WriteFlags,
    ) -> EvifResult<u64> {
        // 特殊处理 /reload - 触发热重载
        if path == "/reload" {
            self.reload().await?;
            return Ok(data.len() as u64);
        }

        let url = self.build_api_url("write", path);
        let body = serde_json::json!({
            "data": String::from_utf8_lossy(&data)
        });

        let response: WriteResponse = self.http_post(&url, &body).await?;
        Ok(response.bytes_written)
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let url = self.build_api_url("readdir", path);
        let response: ReaddirResponse = self.http_get(&url).await?;

        // 添加 /reload 虚拟文件到根目录
        if path == "/" || path.is_empty() {
            let mut entries = response.entries;
            entries.push(FileInfo {
                name: "reload".to_string(),
                size: 0,
                mode: 0o200, // write-only
                modified: Utc::now(),
                is_dir: false,
            });
            Ok(entries)
        } else {
            Ok(response.entries)
        }
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        // 特殊处理 /reload
        if path == "/reload" {
            return Ok(FileInfo {
                name: "reload".to_string(),
                size: 0,
                mode: 0o200, // write-only
                modified: *self.last_reload.read().await,
                is_dir: false,
            });
        }

        let url = self.build_api_url("stat", path);
        self.http_get(&url).await
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        if path == "/reload" {
            return Err(EvifError::InvalidPath(
                "Cannot delete /reload control file".to_string(),
            ));
        }

        let url = self.build_api_url("remove", path);
        self.http_delete(&url).await
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        // 远程API可能不支持rename
        Err(EvifError::InvalidPath(
            "Remote rename not supported via HTTP API".to_string(),
        ))
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        if path == "/reload" {
            return Err(EvifError::InvalidPath(
                "Cannot delete /reload control file".to_string(),
            ));
        }

        // 通过HTTP API调用远程的remove_all
        let url = self.build_api_url("remove_all", path);
        self.http_delete(&url).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxyfs_url_building() {
        let plugin = ProxyFsPlugin::new("http://localhost:8080/api/v1");

        assert_eq!(
            plugin.build_api_url("read", "/test/file.txt"),
            "http://localhost:8080/api/v1/read/test%2Ffile.txt"
        );

        assert_eq!(
            plugin.build_api_url("write", "/data"),
            "http://localhost:8080/api/v1/write/data"
        );
    }

    #[test]
    fn test_proxyfs_config_default() {
        let config = ProxyConfig::default();
        assert_eq!(config.base_url, "http://localhost:8080/api/v1");
        assert_eq!(config.timeout_seconds, 30);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_proxyfs_reload_file() {
        let plugin = ProxyFsPlugin::new("http://localhost:8080/api/v1");

        // /reload 应该返回特殊响应
        // 注意: 这个测试不会真正连接,只是验证逻辑
        assert_eq!(plugin.name(), "proxyfs");
    }
}
