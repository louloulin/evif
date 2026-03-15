// gRPC 客户端实现

use crate::error::GrpcError;
use crate::evif;
use crate::evif::evif_service_client::EvifServiceClient;
use crate::{
    value, BatchGetNodesRequest, BatchPutNodesResponse, DataChunk, DeleteNodeRequest,
    DeleteNodeResponse, GetNodeRequest, GetNodeResponse, HealthRequest, HealthResponse,
    NodeResponse, PutNodeRequest, PutNodeResponse, QueryRequest, ReadFileRequest, StatsRequest,
    StatsResponse, Value, WriteFileResponse,
};
use evif_graph::{Attribute, Node, NodeType};
use std::collections::HashMap;
use tokio_stream::{Stream, StreamExt};
use tonic::{
    transport::{Channel, Endpoint},
    Request, Streaming,
};
use uuid::Uuid;

/// 客户端配置
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// 服务器地址 (e.g., "http://[::1]:50051")
    pub server_addr: String,
    /// 连接超时 (秒)
    pub connect_timeout_secs: u64,
    /// 最大消息大小 (字节)
    pub max_message_size: usize,
    /// 是否启用 TLS
    pub enable_tls: bool,
    /// 并发请求限制
    pub max_concurrent_requests: usize,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_addr: "http://[::1]:50051".to_string(),
            connect_timeout_secs: 10,
            max_message_size: 4 * 1024 * 1024, // 4MB
            enable_tls: false,
            max_concurrent_requests: 100,
        }
    }
}

/// EVIF gRPC 客户端
///
/// 提供异步 gRPC 客户端功能，支持所有定义的 RPC 方法
pub struct EvifClient {
    /// gRPC 客户端
    client: EvifServiceClient<Channel>,
    /// 配置
    config: ClientConfig,
    /// 服务器地址
    server_addr: String,
}

impl EvifClient {
    /// 连接到 EVIF gRPC 服务器
    pub async fn connect(config: ClientConfig) -> Result<Self, GrpcError> {
        let server_addr = config.server_addr.clone();

        let endpoint = Endpoint::from_shared(server_addr.clone())
            .map_err(|e| GrpcError::Internal(format!("Invalid endpoint: {}", e)))?
            .timeout(std::time::Duration::from_secs(config.connect_timeout_secs))
            .connect_timeout(std::time::Duration::from_secs(config.connect_timeout_secs));

        let channel = if config.enable_tls {
            // NOTE: TLS support requires certificate configuration
            return Err(GrpcError::Internal(
                "TLS requires certificate configuration. Use disable_tls=true for now.".to_string(),
            ));
        } else {
            endpoint
                .connect()
                .await
                .map_err(|e| GrpcError::Internal(format!("Failed to connect: {}", e)))?
        };

        let client = EvifServiceClient::new(channel);

        Ok(Self {
            client,
            config,
            server_addr,
        })
    }

    /// 使用默认配置连接
    pub async fn connect_default(addr: &str) -> Result<Self, GrpcError> {
        let config = ClientConfig {
            server_addr: addr.to_string(),
            ..Default::default()
        };
        Self::connect(config).await
    }

    /// 获取配置
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// 获取服务器地址
    pub fn server_addr(&self) -> &str {
        &self.server_addr
    }

    /// 获取单个节点
    pub async fn get_node(&mut self, id: &str) -> Result<Option<Node>, GrpcError> {
        let request = Request::new(evif::GetNodeRequest { id: id.to_string() });

        let response = self
            .client
            .get_node(request)
            .await
            .map_err(|e| GrpcError::Internal(format!("gRPC error: {}", e)))?;

        let response = response.into_inner();
        if let Some(proto_node) = response.node {
            Ok(Some(self.proto_node_to_graph(proto_node)?))
        } else {
            Ok(None)
        }
    }

    /// 创建或更新节点
    pub async fn put_node(&mut self, node: Node) -> Result<Uuid, GrpcError> {
        let proto_node = self.graph_node_to_proto(&node);

        let request = Request::new(evif::PutNodeRequest {
            node: Some(proto_node),
        });

        let response = self
            .client
            .put_node(request)
            .await
            .map_err(|e| GrpcError::Internal(format!("gRPC error: {}", e)))?;

        let response = response.into_inner();
        Uuid::parse_str(&response.id)
            .map_err(|e| GrpcError::Internal(format!("Invalid UUID in response: {}", e)))
    }

    /// 删除节点
    pub async fn delete_node(&mut self, id: &str) -> Result<bool, GrpcError> {
        let request = Request::new(evif::DeleteNodeRequest { id: id.to_string() });

        let response = self
            .client
            .delete_node(request)
            .await
            .map_err(|e| GrpcError::Internal(format!("gRPC error: {}", e)))?;

        let response = response.into_inner();
        Ok(response.success)
    }

    /// 批量获取节点 (流式)
    pub async fn batch_get_nodes(&mut self, ids: Vec<String>) -> Result<Vec<Node>, GrpcError> {
        let request = Request::new(evif::BatchGetNodesRequest { ids });

        let mut stream = self
            .client
            .batch_get_nodes(request)
            .await
            .map_err(|e| GrpcError::Internal(format!("gRPC error: {}", e)))?
            .into_inner();

        let mut nodes = Vec::new();
        while let Some(result) = stream.next().await {
            match result {
                Ok(node_response) => {
                    if let Some(proto_node) = node_response.node {
                        nodes.push(self.proto_node_to_graph(proto_node)?);
                    }
                }
                Err(e) => {
                    return Err(GrpcError::Internal(format!("Stream error: {}", e)));
                }
            }
        }
        Ok(nodes)
    }

    /// 批量创建节点 (流式)
    pub async fn batch_put_nodes(
        &mut self,
        nodes: Vec<Node>,
    ) -> Result<(Vec<String>, i32), GrpcError> {
        // 创建输出流
        let (tx, rx) = tokio::sync::mpsc::channel(64);

        // 在后台任务中发送所有节点
        let proto_nodes: Vec<evif::Node> =
            nodes.iter().map(|n| self.graph_node_to_proto(n)).collect();

        tokio::spawn(async move {
            for node in proto_nodes {
                let req = evif::PutNodeRequest { node: Some(node) };
                if tx.send(req).await.is_err() {
                    break;
                }
            }
        });

        // 创建流式请求
        let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        let request = Request::new(stream);

        let response = self
            .client
            .batch_put_nodes(request)
            .await
            .map_err(|e| GrpcError::Internal(format!("gRPC error: {}", e)))?;

        let response = response.into_inner();
        Ok((response.ids, response.count))
    }

    /// 查询节点 (流式)
    pub async fn query(&mut self, query: &str, limit: u32) -> Result<Vec<Node>, GrpcError> {
        let request = Request::new(evif::QueryRequest {
            query: query.to_string(),
            limit,
        });

        let mut stream = self
            .client
            .query(request)
            .await
            .map_err(|e| GrpcError::Internal(format!("gRPC error: {}", e)))?
            .into_inner();

        let mut nodes = Vec::new();
        while let Some(result) = stream.next().await {
            match result {
                Ok(node_response) => {
                    if let Some(proto_node) = node_response.node {
                        nodes.push(self.proto_node_to_graph(proto_node)?);
                    }
                }
                Err(e) => {
                    return Err(GrpcError::Internal(format!("Stream error: {}", e)));
                }
            }
        }
        Ok(nodes)
    }

    /// 读取文件 (流式)
    pub async fn read_file(
        &mut self,
        path: &str,
        offset: u64,
        size: u64,
    ) -> Result<Vec<u8>, GrpcError> {
        let request = Request::new(evif::ReadFileRequest {
            path: path.to_string(),
            offset,
            size,
        });

        let mut stream = self
            .client
            .read_file(request)
            .await
            .map_err(|e| GrpcError::Internal(format!("gRPC error: {}", e)))?
            .into_inner();

        let mut data = Vec::new();
        while let Some(result) = stream.next().await {
            match result {
                Ok(chunk) => {
                    data.extend_from_slice(&chunk.data);
                    if chunk.eof {
                        break;
                    }
                }
                Err(e) => {
                    return Err(GrpcError::Internal(format!("Stream error: {}", e)));
                }
            }
        }
        Ok(data)
    }

    /// 写入文件 (流式)
    pub async fn write_file(&mut self, path: &str, data: Vec<u8>) -> Result<u64, GrpcError> {
        // 创建输出流
        let (tx, rx) = tokio::sync::mpsc::channel(64);

        // 分块发送数据
        let chunk_size = 64 * 1024; // 64KB chunks
        let mut total_bytes = 0u64;

        tokio::spawn(async move {
            for chunk in data.chunks(chunk_size) {
                let data_chunk = evif::DataChunk {
                    data: chunk.to_vec(),
                    offset: total_bytes,
                    eof: false,
                };
                total_bytes += chunk.len() as u64;
                if tx.send(data_chunk).await.is_err() {
                    break;
                }
            }
            // 发送 EOF 标记
            let eof_chunk = evif::DataChunk {
                data: vec![],
                offset: total_bytes,
                eof: true,
            };
            let _ = tx.send(eof_chunk).await;
        });

        // 创建流式请求
        let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        let request = Request::new(stream);

        let response = self
            .client
            .write_file(request)
            .await
            .map_err(|e| GrpcError::Internal(format!("gRPC error: {}", e)))?;

        let response = response.into_inner();
        Ok(response.bytes_written)
    }

    /// 获取统计信息
    pub async fn stats(&mut self, detailed: bool) -> Result<(u64, u64, u64, String), GrpcError> {
        let request = Request::new(evif::StatsRequest { detailed });

        let response = self
            .client
            .stats(request)
            .await
            .map_err(|e| GrpcError::Internal(format!("gRPC error: {}", e)))?;

        let response = response.into_inner();
        Ok((
            response.total_nodes,
            response.total_edges,
            response.uptime_secs,
            response.status,
        ))
    }

    /// 健康检查
    pub async fn health(&mut self) -> Result<(String, String), GrpcError> {
        let request = Request::new(evif::HealthRequest {});

        let response = self
            .client
            .health(request)
            .await
            .map_err(|e| GrpcError::Internal(format!("gRPC error: {}", e)))?;

        let response = response.into_inner();
        Ok((response.status, response.version))
    }

    /// 将 proto Node 转换为 Graph Node
    fn proto_node_to_graph(&self, proto_node: evif::Node) -> Result<Node, GrpcError> {
        let id = Uuid::parse_str(&proto_node.id)
            .map_err(|_| GrpcError::Internal(format!("Invalid UUID: {}", proto_node.id)))?;

        let name = proto_node
            .metadata
            .get("name")
            .cloned()
            .unwrap_or_else(|| "unnamed".to_string());

        let node_type = match proto_node.node_type.as_str() {
            "file" => NodeType::File,
            "directory" => NodeType::Directory,
            "symlink" => NodeType::Symlink,
            "device" => NodeType::Device,
            "process" => NodeType::Process,
            "network" => NodeType::Network,
            _ => NodeType::Custom(proto_node.node_type.clone()),
        };

        let mut node = Node::new(node_type, name);
        node.id = id;

        for (k, v) in proto_node.attributes {
            if let Some(evif::value::Value::StringValue(s)) = v.value {
                node.attributes.insert(k, Attribute::String(s));
            }
        }

        Ok(node)
    }

    /// 将 Graph Node 转换为 proto Node
    fn graph_node_to_proto(&self, node: &Node) -> evif::Node {
        let mut metadata = HashMap::new();
        metadata.insert("name".to_string(), node.name.clone());
        metadata.insert(
            "permissions".to_string(),
            node.metadata.permissions.to_string(),
        );

        let mut attributes = HashMap::new();
        for (k, v) in &node.attributes {
            match v {
                Attribute::String(s) => {
                    attributes.insert(
                        k.clone(),
                        evif::Value {
                            value: Some(evif::value::Value::StringValue(s.clone())),
                        },
                    );
                }
                Attribute::Integer(i) => {
                    attributes.insert(
                        k.clone(),
                        evif::Value {
                            value: Some(evif::value::Value::IntValue(*i)),
                        },
                    );
                }
                Attribute::Float(f) => {
                    attributes.insert(
                        k.clone(),
                        evif::Value {
                            value: Some(evif::value::Value::DoubleValue(*f)),
                        },
                    );
                }
                Attribute::Boolean(b) => {
                    attributes.insert(
                        k.clone(),
                        evif::Value {
                            value: Some(evif::value::Value::BoolValue(*b)),
                        },
                    );
                }
                Attribute::Binary(data) => {
                    attributes.insert(
                        k.clone(),
                        evif::Value {
                            value: Some(evif::value::Value::BytesValue(data.clone())),
                        },
                    );
                }
                Attribute::DateTime(dt) => {
                    attributes.insert(
                        k.clone(),
                        evif::Value {
                            value: Some(evif::value::Value::StringValue(dt.to_rfc3339())),
                        },
                    );
                }
                Attribute::Null => {
                    attributes.insert(
                        k.clone(),
                        evif::Value {
                            value: Some(evif::value::Value::StringValue("null".to_string())),
                        },
                    );
                }
            }
        }

        evif::Node {
            id: node.id.to_string(),
            node_type: node.node_type.as_str().to_string(),
            metadata,
            attributes,
            content: vec![],
            created_at: node.metadata.created_at.timestamp(),
            updated_at: node.metadata.modified_at.timestamp(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.server_addr, "http://[::1]:50051");
        assert_eq!(config.connect_timeout_secs, 10);
        assert_eq!(config.max_message_size, 4 * 1024 * 1024);
        assert_eq!(config.enable_tls, false);
        assert_eq!(config.max_concurrent_requests, 100);
    }

    #[test]
    fn test_client_config_custom() {
        let config = ClientConfig {
            server_addr: "http://localhost:8080".to_string(),
            connect_timeout_secs: 30,
            max_message_size: 1024,
            enable_tls: true,
            max_concurrent_requests: 50,
        };
        assert_eq!(config.server_addr, "http://localhost:8080");
        assert_eq!(config.connect_timeout_secs, 30);
        assert_eq!(config.enable_tls, true);
    }
}
