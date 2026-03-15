// EVIF 客户端实现

use crate::{ClientCache, ClientError, ClientResult, Transport};
use base64::Engine;
use evif_core::FileInfo;
use evif_graph::{Node, NodeId};
use evif_protocol::{Message, Request, Response};
use reqwest::Client as HttpClient;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

/// 客户端配置
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// 服务器地址
    pub server_addr: String,

    /// 连接超时（秒）
    pub connect_timeout: u64,

    /// 请求超时（秒）
    pub request_timeout: u64,

    /// 缓存大小
    pub cache_size: usize,

    /// 启用缓存
    pub enable_cache: bool,

    /// HTTP基础URL (用于REST API)
    pub base_url: String,

    /// 超时时间
    pub timeout: std::time::Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_addr: "localhost:8081".to_string(),
            connect_timeout: 10,
            request_timeout: 30,
            cache_size: 1000,
            enable_cache: true,
            base_url: "http://localhost:8081".to_string(),
            timeout: std::time::Duration::from_secs(30),
        }
    }
}

/// EVIF 客户端
pub struct EvifClient {
    config: ClientConfig,
    transport: Arc<dyn Transport>,
    cache: Option<ClientCache>,
    http_client: HttpClient,
}

impl EvifClient {
    /// 创建新客户端(异步)
    pub async fn new(config: ClientConfig) -> ClientResult<Self> {
        let transport = crate::transport::create_transport(&config.server_addr).await?;
        let cache = if config.enable_cache {
            Some(ClientCache::new(config.cache_size))
        } else {
            None
        };

        Ok(Self {
            config,
            transport,
            cache,
            http_client: HttpClient::new(),
        })
    }

    /// 创建新客户端(同步,用于CLI)
    pub fn new_sync(config: ClientConfig) -> Self {
        Self {
            config: config.clone(),
            transport: std::sync::Arc::new(crate::transport::DummyTransport),
            cache: None,
            http_client: HttpClient::new(),
        }
    }

    /// 获取节点
    pub async fn get_node(&self, id: NodeId) -> ClientResult<Option<Node>> {
        // 检查缓存
        if let Some(cache) = &self.cache {
            if let Some(node) = cache.get(&id).await {
                return Ok(Some(node));
            }
        }

        // 创建请求
        let request = Request::get_node(id);
        let response = self.send_request(request).await?;

        match response.kind {
            evif_protocol::ResponseKind::Node { node } => {
                if let Some(cache) = &self.cache {
                    if let Some(ref node) = node {
                        cache.put(id, node.clone()).await;
                    }
                }
                Ok(node)
            }
            _ => Err(ClientError::Protocol(
                "Unexpected response type".to_string(),
            )),
        }
    }

    /// 创建节点
    pub async fn create_node(
        &self,
        node_type: evif_graph::NodeType,
        name: String,
        parent_id: Option<NodeId>,
    ) -> ClientResult<NodeId> {
        let request = Request::create_node(node_type, name, parent_id);
        let response = self.send_request(request).await?;

        match response.kind {
            evif_protocol::ResponseKind::Created { id } => Ok(id),
            _ => Err(ClientError::Protocol(
                "Unexpected response type".to_string(),
            )),
        }
    }

    /// 删除节点
    pub async fn delete_node(&self, id: NodeId) -> ClientResult<()> {
        let request = Request::delete_node(id);
        let _response = self.send_request(request).await?;

        if let Some(cache) = &self.cache {
            cache.remove(&id).await;
        }

        Ok(())
    }

    /// 查询图
    pub async fn query(&self, query: String) -> ClientResult<Vec<NodeId>> {
        let request = Request::query_graph(query);
        let response = self.send_request(request).await?;

        match response.kind {
            evif_protocol::ResponseKind::QueryResult { ids, .. } => Ok(ids),
            _ => Err(ClientError::Protocol(
                "Unexpected response type".to_string(),
            )),
        }
    }

    /// 获取子节点
    pub async fn get_children(&self, id: NodeId) -> ClientResult<Vec<NodeId>> {
        let request = Request::get_children(id);
        let response = self.send_request(request).await?;

        match response.kind {
            evif_protocol::ResponseKind::Children { ids } => Ok(ids),
            _ => Err(ClientError::Protocol(
                "Unexpected response type".to_string(),
            )),
        }
    }

    /// 读取文件
    pub async fn read_file(&self, path: &Path) -> ClientResult<Vec<u8>> {
        let request = Request::new(evif_protocol::RequestKind::FileOperation {
            path: path.to_path_buf(),
            operation: evif_protocol::FileOperation::Read {
                handle: 0,
                offset: 0,
                size: 4096,
            },
        });

        let response = self.send_request(request).await?;

        match response.kind {
            evif_protocol::ResponseKind::FileResult { data, .. } => {
                data.ok_or_else(|| ClientError::Protocol("No data in response".to_string()))
            }
            _ => Err(ClientError::Protocol(
                "Unexpected response type".to_string(),
            )),
        }
    }

    /// 发送请求
    async fn send_request(&self, request: Request) -> ClientResult<Response> {
        let message = Message::Request(request);

        // 发送消息
        let response_msg = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.request_timeout),
            self.transport.send(message),
        )
        .await
        .map_err(|_| ClientError::Timeout)?
        .map_err(|e| ClientError::Transport(e))?;

        // 解析响应
        match response_msg {
            Message::Response(response) => Ok(response),
            Message::Error { code, message } => Err(ClientError::Protocol(format!(
                "Error {}: {}",
                code, message
            ))),
            _ => Err(ClientError::Protocol("Unexpected message type".to_string())),
        }
    }

    // ==================== HTTP REST API 方法 ====================

    /// 列出文件
    pub async fn ls(&self, path: &str) -> ClientResult<Vec<FileInfo>> {
        let url = format!("{}/api/v1/directories?path={}", self.config.base_url, path);
        let response = self.http_client.get(&url).send().await.map_err(|e| {
            ClientError::Transport(crate::TransportError::ConnectionFailed(e.to_string()))
        })?;

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
                serde_json::from_value(v.clone()).map_err(|e| ClientError::Protocol(e.to_string()))
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
        let response = self.http_client.get(&url).send().await.map_err(|e| {
            ClientError::Transport(crate::TransportError::ConnectionFailed(e.to_string()))
        })?;

        let json: Value = response
            .json()
            .await
            .map_err(|e| ClientError::Protocol(e.to_string()))?;

        let data = json["data"]
            .as_str()
            .ok_or_else(|| ClientError::Protocol("Invalid response".to_string()))?;

        base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|e| ClientError::Protocol(e.to_string()))
    }

    /// 写入文件（与 evif-rest 契约一致：JSON body data + encoding=base64）
    pub async fn write(&self, path: &str, content: &str, append: bool) -> ClientResult<()> {
        let offset = if append { -1 } else { 0 };
        let bytes = content.as_bytes().to_vec();
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);

        let url = format!(
            "{}/api/v1/files?path={}&offset={}",
            self.config.base_url, path, offset
        );
        let body = serde_json::json!({ "data": encoded, "encoding": "base64" });

        self.http_client
            .put(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                ClientError::Transport(crate::TransportError::ConnectionFailed(e.to_string()))
            })?;

        Ok(())
    }

    /// 创建目录
    pub async fn mkdir(&self, path: &str) -> ClientResult<()> {
        let url = format!("{}/api/v1/directories?path={}", self.config.base_url, path);
        self.http_client.post(&url).send().await.map_err(|e| {
            ClientError::Transport(crate::TransportError::ConnectionFailed(e.to_string()))
        })?;
        Ok(())
    }

    /// 删除文件
    pub async fn remove(&self, path: &str) -> ClientResult<()> {
        let url = format!("{}/api/v1/files?path={}", self.config.base_url, path);
        self.http_client.delete(&url).send().await.map_err(|e| {
            ClientError::Transport(crate::TransportError::ConnectionFailed(e.to_string()))
        })?;
        Ok(())
    }

    /// 递归删除
    pub async fn remove_all(&self, path: &str) -> ClientResult<()> {
        let url = format!("{}/api/v1/directories?path={}", self.config.base_url, path);
        self.http_client.delete(&url).send().await.map_err(|e| {
            ClientError::Transport(crate::TransportError::ConnectionFailed(e.to_string()))
        })?;
        Ok(())
    }

    /// 重命名文件
    pub async fn rename(&self, old_path: &str, new_path: &str) -> ClientResult<()> {
        let url = format!("{}/api/v1/rename", self.config.base_url);
        let body = serde_json::json!({"old_path": old_path, "new_path": new_path});

        self.http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                ClientError::Transport(crate::TransportError::ConnectionFailed(e.to_string()))
            })?;
        Ok(())
    }

    /// 获取文件信息
    pub async fn stat(&self, path: &str) -> ClientResult<FileInfo> {
        let url = format!("{}/api/v1/stat?path={}", self.config.base_url, path);
        let response = self.http_client.get(&url).send().await.map_err(|e| {
            ClientError::Transport(crate::TransportError::ConnectionFailed(e.to_string()))
        })?;

        let json: Value = response
            .json()
            .await
            .map_err(|e| ClientError::Protocol(e.to_string()))?;

        serde_json::from_value(json).map_err(|e| ClientError::Protocol(e.to_string()))
    }

    /// 健康检查
    pub async fn health(&self) -> ClientResult<HealthInfo> {
        let url = format!("{}/api/v1/health", self.config.base_url);
        let response = self.http_client.get(&url).send().await.map_err(|e| {
            ClientError::Transport(crate::TransportError::ConnectionFailed(e.to_string()))
        })?;

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
            .map_err(|e| {
                ClientError::Transport(crate::TransportError::ConnectionFailed(e.to_string()))
            })?;
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
            .map_err(|e| {
                ClientError::Transport(crate::TransportError::ConnectionFailed(e.to_string()))
            })?;
        Ok(())
    }

    /// 列出挂载点
    pub async fn mounts(&self) -> ClientResult<Vec<MountInfo>> {
        let url = format!("{}/api/v1/mounts", self.config.base_url);
        let response = self.http_client.get(&url).send().await.map_err(|e| {
            ClientError::Transport(crate::TransportError::ConnectionFailed(e.to_string()))
        })?;

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
            .map_err(|e| {
                ClientError::Transport(crate::TransportError::ConnectionFailed(e.to_string()))
            })?;
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
            .map_err(|e| {
                ClientError::Transport(crate::TransportError::ConnectionFailed(e.to_string()))
            })?;
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

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.server_addr, "localhost:8081");
        assert_eq!(config.base_url, "http://localhost:8081");
        assert_eq!(config.connect_timeout, 10);
        assert!(config.enable_cache);
    }

    #[test]
    fn test_client_config_custom() {
        let config = ClientConfig {
            server_addr: "example.com:8080".to_string(),
            connect_timeout: 30,
            request_timeout: 60,
            cache_size: 2000,
            enable_cache: false,
            base_url: "http://localhost:8080".to_string(),
            timeout: std::time::Duration::from_secs(30),
        };

        assert_eq!(config.server_addr, "example.com:8080");
        assert_eq!(config.connect_timeout, 30);
        assert!(!config.enable_cache);
    }
}
