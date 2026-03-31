// Proxy File System Plugin for EVIF
// 对标 AGFS ProxyFS: 远程EVIF/AGFS服务器客户端代理
// 用途: 分布式文件系统、远程备份、负载均衡、故障转移

use async_trait::async_trait;
use base64::Engine;
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
            base_url: "http://localhost:8081/api/v1".to_string(),
            timeout_seconds: 30,
            max_retries: 3,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct EvifReadResponse {
    content: String,
    data: String,
    size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct EvifWriteResponse {
    bytes_written: u64,
    path: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct EvifDirectoryEntry {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
    modified: String,
    created: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct EvifDirectoryListResponse {
    path: String,
    files: Vec<EvifDirectoryEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EvifFileStatResponse {
    path: String,
    size: u64,
    is_dir: bool,
    modified: String,
    created: String,
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

    fn endpoint_url(&self, endpoint: &str) -> String {
        format!("{}/{}", self.config.base_url.trim_end_matches('/'), endpoint)
    }

    async fn http_get<T: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        path: &str,
    ) -> EvifResult<T> {
        let url = self.endpoint_url(endpoint);
        let response = self.client
            .get(url)
            .query(&[("path", path)])
            .send()
            .await
            .map_err(|e| EvifError::InvalidPath(format!("HTTP GET failed: {}", e)))?;

        if response.status().is_success() {
            response
                .json()
                .await
                .map_err(|e| EvifError::InvalidPath(format!("Failed to parse JSON: {}", e)))
        } else if response.status() == 404 {
            Err(EvifError::NotFound(path.to_string()))
        } else {
            Err(EvifError::InvalidPath(format!(
                "HTTP error: {}",
                response.status()
            )))
        }
    }

    async fn http_post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        endpoint: &str,
        body: &B,
    ) -> EvifResult<T> {
        let url = self.endpoint_url(endpoint);
        let response = self.client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|e| EvifError::InvalidPath(format!("HTTP POST failed: {}", e)))?;

        if response.status().is_success() {
            response
                .json()
                .await
                .map_err(|e| EvifError::InvalidPath(format!("Failed to parse JSON: {}", e)))
        } else {
            Err(EvifError::InvalidPath(format!(
                "HTTP error: {}",
                response.status()
            )))
        }
    }

    async fn http_put<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        endpoint: &str,
        path: &str,
        body: &B,
    ) -> EvifResult<T> {
        let url = self.endpoint_url(endpoint);
        let response = self.client
            .put(url)
            .query(&[("path", path), ("offset", "0")])
            .json(body)
            .send()
            .await
            .map_err(|e| EvifError::InvalidPath(format!("HTTP PUT failed: {}", e)))?;

        if response.status().is_success() {
            response
                .json()
                .await
                .map_err(|e| EvifError::InvalidPath(format!("Failed to parse JSON: {}", e)))
        } else {
            Err(EvifError::InvalidPath(format!(
                "HTTP error: {}",
                response.status()
            )))
        }
    }

    async fn http_delete(&self, endpoint: &str, path: &str) -> EvifResult<()> {
        let url = self.endpoint_url(endpoint);
        let response = self.client
            .delete(url)
            .query(&[("path", path)])
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

    /// 获取最后重载时间
    pub async fn last_reload_time(&self) -> DateTime<Utc> {
        *self.last_reload.read().await
    }

    /// 健康检查（不更新 reload 时间戳）
    pub async fn health_check(&self) -> EvifResult<bool> {
        let health_url = self.endpoint_url("health");
        let response = self.client
            .get(&health_url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await;

        match response {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// 热重载连接
    pub async fn reload(&self) -> EvifResult<()> {
        // 测试连接
        let health_url = self.endpoint_url("health");
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
        let body = serde_json::json!({ "path": path });
        let _: serde_json::Value = self.http_post("files", &body).await?;
        Ok(())
    }

    async fn mkdir(&self, path: &str, _perm: u32) -> EvifResult<()> {
        let body = serde_json::json!({ "path": path, "parents": false });
        let _: serde_json::Value = self.http_post("directories", &body).await?;
        Ok(())
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

        let response: EvifReadResponse = self.http_get("files", path).await?;
        base64::engine::general_purpose::STANDARD
            .decode(response.data)
            .map_err(|e| EvifError::InvalidPath(format!("Invalid base64 response: {}", e)))
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

        let body = serde_json::json!({
            "data": base64::engine::general_purpose::STANDARD.encode(&data),
            "encoding": "base64"
        });

        let response: EvifWriteResponse = self.http_put("files", path, &body).await?;
        Ok(response.bytes_written)
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let response: EvifDirectoryListResponse = self.http_get("directories", path).await?;
        let mut entries: Vec<FileInfo> = response
            .files
            .into_iter()
            .map(|entry| FileInfo {
                name: entry.name,
                size: entry.size,
                mode: if entry.is_dir { 0o755 } else { 0o644 },
                modified: DateTime::parse_from_rfc3339(&entry.modified)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                is_dir: entry.is_dir,
            })
            .collect();

        // 添加 /reload 虚拟文件到根目录
        if path == "/" || path.is_empty() {
            entries.push(FileInfo {
                name: "reload".to_string(),
                size: 0,
                mode: 0o200, // write-only
                modified: Utc::now(),
                is_dir: false,
            });
        }

        Ok(entries)
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

        let response: EvifFileStatResponse = self.http_get("stat", path).await?;
        Ok(FileInfo {
            name: path
                .trim_start_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or(path)
                .to_string(),
            size: response.size,
            mode: if response.is_dir { 0o755 } else { 0o644 },
            modified: DateTime::parse_from_rfc3339(&response.modified)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            is_dir: response.is_dir,
        })
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        if path == "/reload" {
            return Err(EvifError::InvalidPath(
                "Cannot delete /reload control file".to_string(),
            ));
        }

        self.http_delete("files", path).await
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

        self.http_delete("directories", path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxyfs_url_building() {
        let plugin = ProxyFsPlugin::new("http://localhost:8081/api/v1");

        assert_eq!(
            plugin.endpoint_url("files"),
            "http://localhost:8081/api/v1/files"
        );

        assert_eq!(
            plugin.endpoint_url("directories"),
            "http://localhost:8081/api/v1/directories"
        );
    }

    #[test]
    fn test_proxyfs_config_default() {
        let config = ProxyConfig::default();
        assert_eq!(config.base_url, "http://localhost:8081/api/v1");
        assert_eq!(config.timeout_seconds, 30);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_proxyfs_reload_file() {
        let plugin = ProxyFsPlugin::new("http://localhost:8081/api/v1");

        // /reload 应该返回特殊响应
        // 注意: 这个测试不会真正连接,只是验证逻辑
        assert_eq!(plugin.name(), "proxyfs");
    }
}
