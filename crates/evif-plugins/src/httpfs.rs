// HTTP File System Plugin for EVIF
// 对标 AGFS HTTPFS: 通过HTTP暴露文件系统内容
// 用途: 集成测试、REST API桥接、Web界面

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use evif_core::{EvifError, EvifPlugin, EvifResult, FileInfo, WriteFlags};
use std::sync::Arc;
use tokio::sync::RwLock;

/// HTTP文件系统插件
///
/// 提供基于HTTP的文件访问接口,支持:
/// - 文件读写通过HTTP动词映射
/// - 元数据查询
/// - 目录列表
///
/// 路径格式:
/// - `/httpfs/{hostname}/{port}/{path}` - 远程文件访问
pub struct HttpFsPlugin {
    base_url: String,
    client: Arc<reqwest::Client>,
    timeout_seconds: u64,
}

impl HttpFsPlugin {
    /// 创建新的HTTP文件系统插件
    ///
    /// # 参数
    /// - `base_url`: HTTP服务基础URL (如 "http://localhost:8080")
    /// - `timeout`: 请求超时秒数
    pub fn new(base_url: &str, timeout_seconds: u64) -> Self {
        let base_url = base_url.trim_end_matches('/');

        Self {
            base_url: base_url.to_string(),
            client: Arc::new(reqwest::Client::new()),
            timeout_seconds,
        }
    }

    /// 构建完整URL
    fn build_url(&self, path: &str) -> String {
        let clean_path = path.trim_start_matches('/');
        if clean_path.is_empty() {
            format!("{}/", self.base_url)
        } else {
            format!("{}/{}", self.base_url, clean_path)
        }
    }

    /// 执行HTTP GET请求
    async fn http_get(&self, path: &str) -> EvifResult<reqwest::Response> {
        let url = self.build_url(path);
        let timeout = std::time::Duration::from_secs(self.timeout_seconds);

        let response = self.client
            .get(&url)
            .timeout(timeout)
            .send()
            .await
            .map_err(|e| EvifError::InvalidPath(format!("HTTP GET failed: {}", e)))?;

        if response.status().is_success() {
            Ok(response)
        } else if response.status() == 404 {
            Err(EvifError::NotFound(path.to_string()))
        } else {
            Err(EvifError::InvalidPath(format!(
                "HTTP error: {}",
                response.status()
            )))
        }
    }

    /// 执行HTTP PUT请求
    async fn http_put(&self, path: &str, data: Vec<u8>) -> EvifResult<u64> {
        let url = self.build_url(path);
        let timeout = std::time::Duration::from_secs(self.timeout_seconds);

        let response = self.client
            .put(&url)
            .header("Content-Type", "application/octet-stream")
            .timeout(timeout)
            .body(data)
            .send()
            .await
            .map_err(|e| EvifError::InvalidPath(format!("HTTP PUT failed: {}", e)))?;

        if response.status().is_success() {
            Ok(response.content_length().unwrap_or(0))
        } else {
            Err(EvifError::InvalidPath(format!(
                "HTTP PUT failed with status: {}",
                response.status()
            )))
        }
    }

    /// 执行HTTP DELETE请求
    async fn http_delete(&self, path: &str) -> EvifResult<()> {
        let url = self.build_url(path);
        let timeout = std::time::Duration::from_secs(self.timeout_seconds);

        let response = self.client
            .delete(&url)
            .timeout(timeout)
            .send()
            .await
            .map_err(|e| EvifError::InvalidPath(format!("HTTP DELETE failed: {}", e)))?;

        if response.status().is_success() || response.status() == 404 {
            Ok(())
        } else {
            Err(EvifError::InvalidPath(format!(
                "HTTP DELETE failed with status: {}",
                response.status()
            )))
        }
    }

    /// 执行HTTP HEAD请求
    async fn http_head(&self, path: &str) -> EvifResult<FileInfo> {
        let url = self.build_url(path);
        let timeout = std::time::Duration::from_secs(self.timeout_seconds);

        let response = self.client
            .head(&url)
            .timeout(timeout)
            .send()
            .await
            .map_err(|e| EvifError::InvalidPath(format!("HTTP HEAD failed: {}", e)))?;

        if response.status().is_success() {
            let name = path.rsplit('/')
                .find(|s| !s.is_empty())
                .unwrap_or("unknown")
                .to_string();

            let size = response.content_length().unwrap_or(0);

            let last_modified = response.headers()
                .get("last-modified")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| DateTime::parse_from_rfc2822(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            Ok(FileInfo {
                name,
                size,
                mode: 0o644,
                modified: last_modified,
                is_dir: false,
            })
        } else if response.status() == 404 {
            Err(EvifError::NotFound(path.to_string()))
        } else {
            Err(EvifError::InvalidPath(format!(
                "HTTP HEAD failed with status: {}",
                response.status()
            )))
        }
    }
}

#[async_trait]
impl EvifPlugin for HttpFsPlugin {
    fn name(&self) -> &str {
        "httpfs"
    }

    async fn create(&self, path: &str, _perm: u32) -> EvifResult<()> {
        // HTTP PUT with empty data creates file
        self.http_put(path, vec![]).await?;
        Ok(())
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        // HTTP协议不支持直接创建目录
        // 实际实现中可能需要调用特定的MKCOL方法(WebDAV)
        Err(EvifError::InvalidPath(
            "HTTP does not support mkdir directly".to_string(),
        ))
    }

    async fn read(&self, path: &str, _offset: u64, size: u64) -> EvifResult<Vec<u8>> {
        let response = self.http_get(path).await?;

        // 读取响应数据
        let bytes = response.bytes().await
            .map_err(|e| EvifError::InvalidPath(format!("Failed to read response: {}", e)))?;

        // 根据size限制返回
        if size > 0 && bytes.len() > size as usize {
            Ok(bytes[..size as usize].to_vec())
        } else {
            Ok(bytes.to_vec())
        }
    }

    async fn write(
        &self,
        path: &str,
        data: Vec<u8>,
        _offset: i64,
        _flags: WriteFlags,
    ) -> EvifResult<u64> {
        self.http_put(path, data).await
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        // 尝试读取路径(通常HTTP服务器会返回HTML目录列表)
        match self.http_get(path).await {
            Ok(response) => {
                let html = response.text().await
                    .map_err(|e| EvifError::InvalidPath(format!("Failed to read HTML: {}", e)))?;

                // 简单解析HTML目录列表
                // 实际实现中应该使用HTML解析器
                let mut entries = Vec::new();

                // 查找所有<a>标签
                for line in html.lines() {
                    if let Some(start) = line.find("<a href=\"") {
                        let href_start = start + 9;
                        if let Some(end) = line[ href_start..].find('"') {
                            let href = &line[href_start..href_start + end];
                            let name = href.trim_end_matches('/');

                            if !name.is_empty() && !name.starts_with('?') && !name.starts_with('/') {
                                entries.push(FileInfo {
                                    name: name.to_string(),
                                    size: 0,
                                    mode: if href.ends_with('/') { 0o755 } else { 0o644 },
                                    modified: Utc::now(),
                                    is_dir: href.ends_with('/'),
                                });
                            }
                        }
                    }
                }

                Ok(entries)
            }
            Err(EvifError::NotFound(_)) => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        self.http_head(path).await
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        self.http_delete(path).await
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        // HTTP协议不支持直接重命名
        Err(EvifError::InvalidPath(
            "HTTP does not support rename directly".to_string(),
        ))
    }

    async fn remove_all(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::InvalidPath(
            "HTTP does not support remove_all".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_httpfs_basic() {
        // 注意: 此测试需要运行中的HTTP服务器
        // 在CI/CD环境中应该被mock或跳过

        let plugin = HttpFsPlugin::new("https://httpbin.org", 30);

        // 测试读取
        match plugin.read("/get", 0, 1024).await {
            Ok(data) => {
                assert!(!data.is_empty());
                println!("HTTP read success: {} bytes", data.len());
            }
            Err(e) => {
                println!("HTTP read failed (expected in offline mode): {:?}", e);
            }
        }
    }

    #[test]
    fn test_httpfs_url_building() {
        let plugin = HttpFsPlugin::new("http://localhost:8080", 10);

        assert_eq!(plugin.build_url(""), "http://localhost:8080/");
        assert_eq!(plugin.build_url("/"), "http://localhost:8080/");
        assert_eq!(plugin.build_url("/test"), "http://localhost:8080/test");
        assert_eq!(plugin.build_url("path/to/file"), "http://localhost:8080/path/to/file");
    }
}
