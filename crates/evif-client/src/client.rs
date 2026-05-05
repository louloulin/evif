// EVIF 客户端实现

use crate::{ClientError, ClientResult};
use base64::Engine;
use evif_core::FileInfo;
use reqwest::Client as HttpClient;
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

/// 客户端配置
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// 请求超时（秒）
    pub request_timeout: u64,

    /// HTTP基础URL (用于REST API)
    pub base_url: String,

    /// 超时时间
    pub timeout: std::time::Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            request_timeout: 30,
            base_url: "http://localhost:8081".to_string(),
            timeout: std::time::Duration::from_secs(30),
        }
    }
}

/// EVIF 客户端
pub struct EvifClient {
    config: ClientConfig,
    http_client: HttpClient,
}

#[derive(Debug, Deserialize)]
struct RestFileInfo {
    name: String,
    #[allow(dead_code)]
    path: String,
    is_dir: bool,
    size: u64,
    modified: String,
    #[allow(dead_code)]
    created: String,
}

#[derive(Debug, Deserialize)]
struct RestFileStat {
    path: String,
    size: u64,
    is_dir: bool,
    modified: String,
    #[allow(dead_code)]
    created: String,
}

fn parse_modified(ts: &str) -> ClientResult<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| ClientError::Protocol(format!("Invalid timestamp '{}': {}", ts, e)))
}

fn file_name_from_path(path: &str) -> String {
    path.trim_end_matches('/')
        .split('/')
        .next_back()
        .filter(|s| !s.is_empty())
        .unwrap_or("/")
        .to_string()
}

impl EvifClient {
    /// 创建新客户端(异步)
    pub async fn new(config: ClientConfig) -> ClientResult<Self> {
        Ok(Self {
            config,
            http_client: HttpClient::builder().no_proxy().build().unwrap(),
        })
    }

    /// 创建新客户端(同步,用于CLI)
    pub fn new_sync(config: ClientConfig) -> Self {
        Self {
            config: config.clone(),
            http_client: HttpClient::builder().no_proxy().build().unwrap(),
        }
    }

    /// 读取文件
    pub async fn read_file(&self, path: &Path) -> ClientResult<Vec<u8>> {
        self.cat_bytes(path.to_string_lossy().as_ref()).await
    }

    // ==================== HTTP REST API 方法 ====================

    /// 列出文件
    pub async fn ls(&self, path: &str) -> ClientResult<Vec<FileInfo>> {
        let url = format!("{}/api/v1/directories?path={}", self.config.base_url, path);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;

        let status = response.status();
        let json: Value = response
            .json()
            .await
            .map_err(|e| ClientError::Protocol(e.to_string()))?;

        // 如果返回错误，返回错误信息
        if !status.is_success() {
            if let Some(msg) = json.get("message").and_then(|v| v.as_str()) {
                return Err(ClientError::Protocol(msg.to_string()));
            }
            return Err(ClientError::Protocol(format!("HTTP {}", status.as_u16())));
        }

        let files = json["files"].as_array().ok_or_else(|| {
            ClientError::Protocol("Invalid response: missing 'files' field".to_string())
        })?;

        files
            .iter()
            .map(|v| {
                let info: RestFileInfo = serde_json::from_value(v.clone())
                    .map_err(|e| ClientError::Protocol(e.to_string()))?;
                Ok(FileInfo {
                    name: info.name,
                    size: info.size,
                    mode: if info.is_dir { 0o755 } else { 0o644 },
                    modified: parse_modified(&info.modified)?,
                    is_dir: info.is_dir,
                })
            })
            .collect()
    }

    /// 读取文件
    pub async fn cat(&self, path: &str) -> ClientResult<String> {
        let bytes = self.cat_bytes(path).await?;
        String::from_utf8(bytes).map_err(|e| ClientError::Protocol(format!("Invalid UTF-8: {}", e)))
    }

    /// 读取文件字节
    pub async fn cat_bytes(&self, path: &str) -> ClientResult<Vec<u8>> {
        let url = format!("{}/api/v1/files?path={}", self.config.base_url, path);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        let status = response.status();

        let json: Value = response
            .json()
            .await
            .map_err(|e| ClientError::Protocol(e.to_string()))?;

        if !status.is_success() {
            if let Some(msg) = json.get("message").and_then(|v| v.as_str()) {
                return Err(ClientError::Protocol(msg.to_string()));
            }
            return Err(ClientError::Protocol(format!("HTTP {}", status.as_u16())));
        }

        let data = json["data"]
            .as_str()
            .ok_or_else(|| ClientError::Protocol("Invalid response".to_string()))?;

        base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|e| ClientError::Protocol(e.to_string()))
    }

    /// 写入文件（与 evif-rest 契约一致：JSON body data + encoding=base64）
    pub async fn write(&self, path: &str, content: &str, append: bool) -> ClientResult<()> {
        let payload = if append {
            match self.cat(path).await {
                Ok(existing) => format!("{}{}", existing, content),
                Err(_) => content.to_string(),
            }
        } else {
            content.to_string()
        };

        let bytes = payload.as_bytes().to_vec();
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);

        let create_url = format!("{}/api/v1/files", self.config.base_url);
        let create_body = serde_json::json!({ "path": path });
        let _ = self
            .http_client
            .post(&create_url)
            .json(&create_body)
            .send()
            .await;

        let url = format!(
            "{}/api/v1/files?path={}&offset=0",
            self.config.base_url, path
        );
        let body = serde_json::json!({ "data": encoded, "encoding": "base64" });

        let response = self
            .http_client
            .put(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ClientError::Protocol(format!(
                "HTTP {}",
                response.status().as_u16()
            )));
        }

        Ok(())
    }

    /// 创建目录
    pub async fn mkdir(&self, path: &str, parents: bool) -> ClientResult<()> {
        let url = format!("{}/api/v1/directories", self.config.base_url);
        let body = serde_json::json!({ "path": path, "parents": parents });
        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        if !response.status().is_success() {
            return Err(ClientError::Protocol(format!(
                "HTTP {}",
                response.status().as_u16()
            )));
        }
        Ok(())
    }

    /// 删除文件
    pub async fn remove(&self, path: &str) -> ClientResult<()> {
        let url = format!("{}/api/v1/files?path={}", self.config.base_url, path);
        self.http_client
            .delete(&url)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        Ok(())
    }

    /// 递归删除
    pub async fn remove_all(&self, path: &str) -> ClientResult<()> {
        let url = format!("{}/api/v1/directories?path={}", self.config.base_url, path);
        self.http_client
            .delete(&url)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        Ok(())
    }

    /// 重命名文件
    pub async fn rename(&self, old_path: &str, new_path: &str) -> ClientResult<()> {
        let url = format!("{}/api/v1/rename", self.config.base_url);
        let body = serde_json::json!({"from": old_path, "to": new_path});

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        if !response.status().is_success() {
            return Err(ClientError::Protocol(format!(
                "HTTP {}",
                response.status().as_u16()
            )));
        }
        Ok(())
    }

    /// 获取文件信息
    pub async fn stat(&self, path: &str) -> ClientResult<FileInfo> {
        let url = format!("{}/api/v1/stat?path={}", self.config.base_url, path);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        let status = response.status();

        let json: Value = response
            .json()
            .await
            .map_err(|e| ClientError::Protocol(e.to_string()))?;
        if !status.is_success() {
            if let Some(msg) = json.get("message").and_then(|v| v.as_str()) {
                return Err(ClientError::Protocol(msg.to_string()));
            }
            return Err(ClientError::Protocol(format!("HTTP {}", status.as_u16())));
        }

        let stat: RestFileStat =
            serde_json::from_value(json).map_err(|e| ClientError::Protocol(e.to_string()))?;
        Ok(FileInfo {
            name: file_name_from_path(&stat.path),
            size: stat.size,
            mode: if stat.is_dir { 0o755 } else { 0o644 },
            modified: parse_modified(&stat.modified)?,
            is_dir: stat.is_dir,
        })
    }

    /// 健康检查
    pub async fn health(&self) -> ClientResult<HealthInfo> {
        let url = format!("{}/api/v1/health", self.config.base_url);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;

        let json: Value = response
            .json()
            .await
            .map_err(|e| ClientError::Protocol(e.to_string()))?;

        Ok(HealthInfo {
            status: json["status"].as_str().unwrap_or("unknown").to_string(),
            version: json["version"].as_str().unwrap_or("unknown").to_string(),
            uptime: json["uptime"].as_u64().unwrap_or(0),
        })
    }

    /// 挂载插件（与 evif-rest POST /api/v1/mount 契约一致）
    pub async fn mount(&self, plugin: &str, path: &str, config: Option<&str>) -> ClientResult<()> {
        let url = format!("{}/api/v1/mount", self.config.base_url);
        let mut body = serde_json::json!({"plugin": plugin, "path": path});
        if let Some(cfg) = config {
            body["config"] = serde_json::json!(cfg);
        }

        self.http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        Ok(())
    }

    /// 卸载插件（与 evif-rest POST /api/v1/unmount 契约一致）
    pub async fn unmount(&self, path: &str) -> ClientResult<()> {
        let url = format!("{}/api/v1/unmount", self.config.base_url);
        let body = serde_json::json!({"path": path});
        self.http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        Ok(())
    }

    /// 列出挂载点
    pub async fn mounts(&self) -> ClientResult<Vec<MountInfo>> {
        let url = format!("{}/api/v1/mounts", self.config.base_url);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;

        let status = response.status();
        let json: Value = response
            .json()
            .await
            .map_err(|e| ClientError::Protocol(e.to_string()))?;

        // 如果返回错误，返回错误信息
        if !status.is_success() {
            if let Some(msg) = json.get("message").and_then(|v| v.as_str()) {
                return Err(ClientError::Protocol(msg.to_string()));
            }
            return Err(ClientError::Protocol(format!("HTTP {}", status.as_u16())));
        }

        // 尝试两种格式：{"mounts": [...]} 或直接的数组 [...]
        let mounts = if let Some(mounts_array) = json.get("mounts").and_then(|v| v.as_array()) {
            mounts_array
        } else if let Some(array) = json.as_array() {
            array
        } else {
            return Err(ClientError::Protocol(
                "Invalid response: expected array or object with 'mounts' field".to_string(),
            ));
        };

        mounts
            .iter()
            .map(|v| {
                serde_json::from_value(v.clone()).map_err(|e| ClientError::Protocol(e.to_string()))
            })
            .collect()
    }

    /// 计算文件摘要（Phase 10.1：POST /api/v1/digest）
    pub async fn digest(
        &self,
        path: &str,
        algorithm: Option<&str>,
    ) -> ClientResult<(String, String)> {
        let url = format!("{}/api/v1/digest", self.config.base_url);
        let mut body = serde_json::json!({ "path": path });
        if let Some(algo) = algorithm {
            body["algorithm"] = serde_json::json!(algo);
        }
        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        let json: Value = response
            .json()
            .await
            .map_err(|e| ClientError::Protocol(e.to_string()))?;
        let algo = json["algorithm"].as_str().unwrap_or("sha256").to_string();
        let hash = json["hash"]
            .as_str()
            .ok_or_else(|| ClientError::Protocol("Missing hash".to_string()))?
            .to_string();
        Ok((algo, hash))
    }

    /// 修改文件权限（POST /api/v1/fs/chmod）
    pub async fn chmod(&self, path: &str, mode: u32) -> ClientResult<()> {
        let url = format!("{}/api/v1/fs/chmod", self.config.base_url);
        let body = serde_json::json!({ "path": path, "mode": mode });
        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;

        if !response.status().is_success() {
            let json: Value = response
                .json()
                .await
                .map_err(|e| ClientError::Protocol(e.to_string()))?;
            let msg = json.get("message").and_then(|v| v.as_str()).unwrap_or("chmod failed");
            return Err(ClientError::Protocol(msg.to_string()));
        }
        Ok(())
    }

    /// 修改文件所有者（POST /api/v1/fs/chown）
    pub async fn chown(
        &self,
        path: &str,
        owner: &str,
        group: Option<&str>,
    ) -> ClientResult<()> {
        let url = format!("{}/api/v1/fs/chown", self.config.base_url);
        let mut body = serde_json::json!({ "path": path, "owner": owner });
        if let Some(g) = group {
            body["group"] = serde_json::json!(g);
        }
        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;

        if !response.status().is_success() {
            let json: Value = response
                .json()
                .await
                .map_err(|e| ClientError::Protocol(e.to_string()))?;
            let msg = json.get("message").and_then(|v| v.as_str()).unwrap_or("chown failed");
            return Err(ClientError::Protocol(msg.to_string()));
        }
        Ok(())
    }

    /// 正则搜索（Phase 10.1：POST /api/v1/grep）
    pub async fn grep(
        &self,
        path: &str,
        pattern: &str,
        recursive: Option<bool>,
    ) -> ClientResult<Vec<GrepMatch>> {
        let url = format!("{}/api/v1/grep", self.config.base_url);
        let mut body = serde_json::json!({ "path": path, "pattern": pattern });
        if let Some(r) = recursive {
            body["recursive"] = serde_json::json!(r);
        }
        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        let json: Value = response
            .json()
            .await
            .map_err(|e| ClientError::Protocol(e.to_string()))?;
        let matches = json["matches"]
            .as_array()
            .ok_or_else(|| ClientError::Protocol("Invalid grep response".to_string()))?;
        matches
            .iter()
            .map(|v| {
                serde_json::from_value(v.clone()).map_err(|e| ClientError::Protocol(e.to_string()))
            })
            .collect()
    }
}

/// 健康信息
#[derive(Debug, Clone)]
pub struct HealthInfo {
    pub status: String,
    pub version: String,
    pub uptime: u64,
}

/// 挂载信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MountInfo {
    pub plugin: String,
    pub path: String,
}

/// Grep 匹配结果（Phase 10.1，与 evif-rest GrepMatch 一致）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrepMatch {
    pub path: String,
    pub line: usize,
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ClientConfig Tests ====================

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.base_url, "http://localhost:8081");
        assert_eq!(config.request_timeout, 30);
    }

    #[test]
    fn test_client_config_custom() {
        let config = ClientConfig {
            request_timeout: 60,
            base_url: "http://localhost:8080".to_string(),
            timeout: std::time::Duration::from_secs(30),
        };

        assert_eq!(config.request_timeout, 60);
        assert_eq!(config.base_url, "http://localhost:8080");
    }

    #[test]
    fn test_client_config_default_timeout() {
        let config = ClientConfig::default();
        assert_eq!(config.timeout, std::time::Duration::from_secs(30));
    }

    #[test]
    fn test_client_config_custom_timeout() {
        let config = ClientConfig {
            request_timeout: 120,
            base_url: "http://example.com:9090".to_string(),
            timeout: std::time::Duration::from_secs(120),
        };
        assert_eq!(config.timeout, std::time::Duration::from_secs(120));
    }

    #[test]
    fn test_client_config_zero_timeout() {
        let config = ClientConfig {
            request_timeout: 0,
            base_url: "http://localhost:8081".to_string(),
            timeout: std::time::Duration::from_secs(0),
        };
        assert_eq!(config.request_timeout, 0);
        assert!(config.timeout.is_zero());
    }

    #[test]
    fn test_client_config_clone() {
        let config = ClientConfig::default();
        let cloned = config.clone();
        assert_eq!(config.base_url, cloned.base_url);
        assert_eq!(config.request_timeout, cloned.request_timeout);
        assert_eq!(config.timeout, cloned.timeout);
    }

    #[test]
    fn test_client_config_debug() {
        let config = ClientConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("http://localhost:8081"));
        assert!(debug_str.contains("30"));
    }

    #[test]
    fn test_client_config_https_url() {
        let config = ClientConfig {
            request_timeout: 30,
            base_url: "https://secure.example.com:443".to_string(),
            timeout: std::time::Duration::from_secs(30),
        };
        assert!(config.base_url.starts_with("https://"));
    }

    // ==================== EvifClient Construction Tests ====================

    #[test]
    fn test_client_new_sync() {
        let config = ClientConfig::default();
        let _client = EvifClient::new_sync(config);
    }

    #[tokio::test]
    async fn test_client_new_async() {
        let config = ClientConfig::default();
        let result = EvifClient::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_client_new_async_custom_config() {
        let config = ClientConfig {
            request_timeout: 10,
            base_url: "http://custom-host:9999".to_string(),
            timeout: std::time::Duration::from_secs(10),
        };
        let result = EvifClient::new(config).await;
        assert!(result.is_ok());
    }

    // ==================== Helper Function Tests ====================

    #[test]
    fn test_parse_modified_valid_rfc3339() {
        let ts = "2024-01-15T10:30:00Z";
        let result = parse_modified(ts);
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2024-01-15");
    }

    #[test]
    fn test_parse_modified_valid_with_timezone() {
        let ts = "2024-06-15T14:30:00+08:00";
        let result = parse_modified(ts);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_modified_invalid() {
        let ts = "not-a-date";
        let result = parse_modified(ts);
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            ClientError::Protocol(msg) => {
                assert!(msg.contains("Invalid timestamp"));
                assert!(msg.contains("not-a-date"));
            }
            _ => panic!("Expected Protocol error"),
        }
    }

    #[test]
    fn test_parse_modified_empty_string() {
        let result = parse_modified("");
        assert!(result.is_err());
    }

    #[test]
    fn test_file_name_from_path_regular() {
        assert_eq!(file_name_from_path("/foo/bar/baz.txt"), "baz.txt");
    }

    #[test]
    fn test_file_name_from_path_single_component() {
        assert_eq!(file_name_from_path("file.txt"), "file.txt");
    }

    #[test]
    fn test_file_name_from_path_root() {
        assert_eq!(file_name_from_path("/"), "/");
    }

    #[test]
    fn test_file_name_from_path_trailing_slash() {
        assert_eq!(file_name_from_path("/foo/bar/"), "bar");
    }

    #[test]
    fn test_file_name_from_path_deeply_nested() {
        assert_eq!(
            file_name_from_path("/a/b/c/d/e/f/g/deep_file.rs"),
            "deep_file.rs"
        );
    }

    #[test]
    fn test_file_name_from_path_empty_string() {
        assert_eq!(file_name_from_path(""), "/");
    }

    // ==================== Data Structure Tests ====================

    #[test]
    fn test_health_info_construction() {
        let info = HealthInfo {
            status: "ok".to_string(),
            version: "1.0.0".to_string(),
            uptime: 3600,
        };
        assert_eq!(info.status, "ok");
        assert_eq!(info.version, "1.0.0");
        assert_eq!(info.uptime, 3600);
    }

    #[test]
    fn test_health_info_debug() {
        let info = HealthInfo {
            status: "ok".to_string(),
            version: "0.1.0".to_string(),
            uptime: 0,
        };
        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("ok"));
        assert!(debug_str.contains("0.1.0"));
    }

    #[test]
    fn test_health_info_clone() {
        let info = HealthInfo {
            status: "healthy".to_string(),
            version: "2.0".to_string(),
            uptime: 999,
        };
        let cloned = info.clone();
        assert_eq!(info.status, cloned.status);
        assert_eq!(info.version, cloned.version);
        assert_eq!(info.uptime, cloned.uptime);
    }

    #[test]
    fn test_mount_info_serde_roundtrip() {
        let info = MountInfo {
            plugin: "memory".to_string(),
            path: "/tmp/test".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: MountInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.plugin, "memory");
        assert_eq!(deserialized.path, "/tmp/test");
    }

    #[test]
    fn test_mount_info_debug() {
        let info = MountInfo {
            plugin: "test_plugin".to_string(),
            path: "/mnt/data".to_string(),
        };
        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("test_plugin"));
        assert!(debug_str.contains("/mnt/data"));
    }

    #[test]
    fn test_mount_info_clone() {
        let info = MountInfo {
            plugin: "plugin".to_string(),
            path: "/path".to_string(),
        };
        let cloned = info.clone();
        assert_eq!(info.plugin, cloned.plugin);
        assert_eq!(info.path, cloned.path);
    }

    #[test]
    fn test_grep_match_serde_roundtrip() {
        let m = GrepMatch {
            path: "/foo/bar.txt".to_string(),
            line: 42,
            content: "hello world".to_string(),
        };
        let json = serde_json::to_string(&m).unwrap();
        let deserialized: GrepMatch = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, "/foo/bar.txt");
        assert_eq!(deserialized.line, 42);
        assert_eq!(deserialized.content, "hello world");
    }

    #[test]
    fn test_grep_match_debug() {
        let m = GrepMatch {
            path: "/test".to_string(),
            line: 1,
            content: "match".to_string(),
        };
        let debug_str = format!("{:?}", m);
        assert!(debug_str.contains("/test"));
        assert!(debug_str.contains("match"));
    }

    #[test]
    fn test_grep_match_clone() {
        let m = GrepMatch {
            path: "/a/b".to_string(),
            line: 10,
            content: "found it".to_string(),
        };
        let cloned = m.clone();
        assert_eq!(m.path, cloned.path);
        assert_eq!(m.line, cloned.line);
        assert_eq!(m.content, cloned.content);
    }

    #[test]
    fn test_mount_info_deserialize_from_json() {
        let json = r#"{"plugin":"disk","path":"/data"}"#;
        let info: MountInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.plugin, "disk");
        assert_eq!(info.path, "/data");
    }

    #[test]
    fn test_grep_match_deserialize_from_json() {
        let json = r#"{"path":"/log.txt","line":5,"content":"error found"}"#;
        let m: GrepMatch = serde_json::from_str(json).unwrap();
        assert_eq!(m.path, "/log.txt");
        assert_eq!(m.line, 5);
        assert_eq!(m.content, "error found");
    }

    #[test]
    fn test_grep_match_array_deserialize() {
        let json = r#"[
            {"path":"/a.txt","line":1,"content":"foo"},
            {"path":"/b.txt","line":2,"content":"bar"}
        ]"#;
        let matches: Vec<GrepMatch> = serde_json::from_str(json).unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].path, "/a.txt");
        assert_eq!(matches[1].content, "bar");
    }
}
