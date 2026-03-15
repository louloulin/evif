// gRPC 服务器实现

use crate::error::GrpcError;
use crate::evif;
use crate::evif::evif_service_server::EvifService;
use crate::{
    value, BatchGetNodesRequest, BatchPutNodesResponse, DataChunk, DeleteNodeRequest,
    DeleteNodeResponse, GetNodeRequest, GetNodeResponse, HealthRequest, HealthResponse,
    NodeResponse, PutNodeRequest, PutNodeResponse, QueryRequest, ReadFileRequest, StatsRequest,
    StatsResponse, Value, WriteFileResponse,
};
use evif_auth::AuthManager;
use evif_graph::{Attribute, Graph, Node, NodeType};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{Request, Response, Status, Streaming};
use uuid::Uuid;

/// 服务器配置
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// 绑定地址
    pub bind_addr: String,
    /// 端口
    pub port: u16,
    /// 最大消息大小 (字节)
    pub max_message_size: usize,
    /// 流式传输缓冲区大小
    pub stream_buffer_size: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "[::]".to_string(),
            port: 50051,
            max_message_size: 4 * 1024 * 1024, // 4MB
            stream_buffer_size: 64,
        }
    }
}

/// EVIF gRPC 服务器
///
/// 实现 EvifService trait，将 gRPC 请求转发到内部的 VFS 和 Graph 引擎
pub struct EvifServer {
    /// 图引擎
    graph: Arc<Graph>,
    /// 认证管理器
    auth: Arc<AuthManager>,
    /// 配置
    config: ServerConfig,
    /// 启动时间 (用于统计)
    start_time: SystemTime,
}

impl EvifServer {
    /// 创建新的 gRPC 服务器
    pub fn new(graph: Arc<Graph>, auth: Arc<AuthManager>) -> Self {
        Self {
            graph,
            auth,
            config: ServerConfig::default(),
            start_time: SystemTime::now(),
        }
    }

    /// 使用自定义配置创建服务器
    pub fn with_config(graph: Arc<Graph>, auth: Arc<AuthManager>, config: ServerConfig) -> Self {
        Self {
            graph,
            auth,
            config,
            start_time: SystemTime::now(),
        }
    }

    /// 获取配置
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// 获取图引擎引用
    pub fn graph(&self) -> Arc<Graph> {
        Arc::clone(&self.graph)
    }

    /// 获取认证管理器引用
    pub fn auth(&self) -> Arc<AuthManager> {
        Arc::clone(&self.auth)
    }

    /// 获取运行时间 (秒)
    fn uptime_secs(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// 将 Graph Node 转换为 proto Node
    pub fn graph_node_to_proto(&self, node: &Node) -> evif::Node {
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

    /// 将 proto Node 转换为 Graph Node
    pub fn proto_node_to_graph(&self, proto_node: evif::Node) -> Result<Node, GrpcError> {
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
}

impl Default for EvifServer {
    fn default() -> Self {
        let graph = Arc::new(Graph::new());
        let auth = Arc::new(AuthManager::new());
        Self::new(graph, auth)
    }
}

/// 流式响应的类型别名
type BatchGetNodesStream =
    Pin<Box<dyn tokio_stream::Stream<Item = Result<NodeResponse, Status>> + Send>>;
type QueryStream = Pin<Box<dyn tokio_stream::Stream<Item = Result<NodeResponse, Status>> + Send>>;
type ReadFileStream = Pin<Box<dyn tokio_stream::Stream<Item = Result<DataChunk, Status>> + Send>>;

#[tonic::async_trait]
impl EvifService for EvifServer {
    /// 获取单个节点
    async fn get_node(
        &self,
        request: Request<GetNodeRequest>,
    ) -> Result<Response<GetNodeResponse>, Status> {
        let req = request.into_inner();

        let node_id = Uuid::parse_str(&req.id)
            .map_err(|e| Status::invalid_argument(format!("Invalid node ID: {}", e)))?;

        match self.graph.get_node(&node_id) {
            Ok(node) => {
                let proto_node = self.graph_node_to_proto(&node);
                Ok(Response::new(GetNodeResponse {
                    node: Some(proto_node),
                }))
            }
            Err(e) => Err(Status::not_found(format!("Node not found: {}", e))),
        }
    }

    /// 创建或更新节点
    async fn put_node(
        &self,
        request: Request<PutNodeRequest>,
    ) -> Result<Response<PutNodeResponse>, Status> {
        let req = request.into_inner();

        let node = req
            .node
            .ok_or_else(|| Status::invalid_argument("Node is required"))?;

        let graph_node = self
            .proto_node_to_graph(node.clone())
            .map_err(|e| Status::internal(format!("Failed to convert node: {}", e)))?;

        self.graph
            .add_node(graph_node.clone())
            .map_err(|e| Status::internal(format!("Failed to add node: {}", e)))?;

        Ok(Response::new(PutNodeResponse {
            id: graph_node.id.to_string(),
        }))
    }

    /// 删除节点
    async fn delete_node(
        &self,
        request: Request<DeleteNodeRequest>,
    ) -> Result<Response<DeleteNodeResponse>, Status> {
        let req = request.into_inner();

        let node_id = Uuid::parse_str(&req.id)
            .map_err(|e| Status::invalid_argument(format!("Invalid node ID: {}", e)))?;

        self.graph
            .remove_node(&node_id)
            .map_err(|e| Status::internal(format!("Failed to delete node: {}", e)))?;

        Ok(Response::new(DeleteNodeResponse { success: true }))
    }

    /// 批量获取节点 (流式响应)
    type BatchGetNodesStream = BatchGetNodesStream;

    async fn batch_get_nodes(
        &self,
        request: Request<BatchGetNodesRequest>,
    ) -> Result<Response<Self::BatchGetNodesStream>, Status> {
        let req = request.into_inner();

        let (tx, rx) = mpsc::channel(self.config.stream_buffer_size);
        let graph = Arc::clone(&self.graph);

        tokio::spawn(async move {
            for id in req.ids {
                let node_id = match Uuid::parse_str(&id) {
                    Ok(nid) => nid,
                    Err(e) => {
                        let _ = tx
                            .send(Err(Status::invalid_argument(format!(
                                "Invalid node ID {}: {}",
                                id, e
                            ))))
                            .await;
                        continue;
                    }
                };

                match graph.get_node(&node_id) {
                    Ok(node) => {
                        let mut metadata = HashMap::new();
                        metadata.insert("name".to_string(), node.name.clone());

                        let mut attributes = HashMap::new();
                        for (k, v) in &node.attributes {
                            if let Attribute::String(s) = v {
                                attributes.insert(
                                    k.clone(),
                                    evif::Value {
                                        value: Some(evif::value::Value::StringValue(s.clone())),
                                    },
                                );
                            }
                        }

                        let proto_node = evif::Node {
                            id: node.id.to_string(),
                            node_type: node.node_type.as_str().to_string(),
                            metadata,
                            attributes,
                            content: vec![],
                            created_at: node.metadata.created_at.timestamp(),
                            updated_at: node.metadata.modified_at.timestamp(),
                        };

                        let _ = tx
                            .send(Ok(NodeResponse {
                                node: Some(proto_node),
                            }))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Err(Status::not_found(format!("Node not found: {}", e))))
                            .await;
                    }
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    /// 批量创建节点 (流式请求)
    async fn batch_put_nodes(
        &self,
        request: Request<tonic::Streaming<PutNodeRequest>>,
    ) -> Result<Response<BatchPutNodesResponse>, Status> {
        let mut stream = request.into_inner();
        let mut ids = Vec::new();
        let graph = Arc::clone(&self.graph);

        while let Some(result) = stream.next().await {
            match result {
                Ok(req) => {
                    if let Some(node) = req.node {
                        let node_id_str = node.id.clone();
                        match Uuid::parse_str(&node_id_str) {
                            Ok(id) => {
                                // 简化版本：使用默认节点创建
                                let new_node = Node::new(NodeType::File, "batch_node");
                                let mut new_node = new_node;
                                new_node.id = id;
                                let _ = graph.add_node(new_node);
                                ids.push(node_id_str);
                            }
                            Err(e) => {
                                return Err(Status::invalid_argument(format!(
                                    "Invalid node ID: {}",
                                    e
                                )));
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(Status::internal(format!("Stream error: {}", e)));
                }
            }
        }

        let count = ids.len() as i32;
        Ok(Response::new(BatchPutNodesResponse { ids, count }))
    }

    /// 查询节点 (流式响应)
    type QueryStream = QueryStream;

    async fn query(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<Self::QueryStream>, Status> {
        let req = request.into_inner();

        let (tx, rx) = mpsc::channel(self.config.stream_buffer_size);
        let graph = Arc::clone(&self.graph);

        tokio::spawn(async move {
            // 简化版本：返回所有节点的前 N 个
            let limit = req.limit as usize;
            let nodes = graph.all_nodes();

            for (i, node) in nodes.into_iter().enumerate() {
                if i >= limit {
                    break;
                }

                let mut metadata = HashMap::new();
                metadata.insert("name".to_string(), node.name.clone());

                let mut attributes = HashMap::new();
                for (k, v) in &node.attributes {
                    if let Attribute::String(s) = v {
                        attributes.insert(
                            k.clone(),
                            evif::Value {
                                value: Some(evif::value::Value::StringValue(s.clone())),
                            },
                        );
                    }
                }

                let proto_node = evif::Node {
                    id: node.id.to_string(),
                    node_type: node.node_type.as_str().to_string(),
                    metadata,
                    attributes,
                    content: vec![],
                    created_at: node.metadata.created_at.timestamp(),
                    updated_at: node.metadata.modified_at.timestamp(),
                };

                let _ = tx
                    .send(Ok(NodeResponse {
                        node: Some(proto_node),
                    }))
                    .await;
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    /// 读取文件 (流式响应)
    type ReadFileStream = ReadFileStream;

    async fn read_file(
        &self,
        request: Request<ReadFileRequest>,
    ) -> Result<Response<Self::ReadFileStream>, Status> {
        let req = request.into_inner();

        let (tx, rx) = mpsc::channel(self.config.stream_buffer_size);

        tokio::spawn(async move {
            // 简化版本：返回空数据块
            let chunk = DataChunk {
                data: vec![],
                offset: req.offset,
                eof: true,
            };
            let _ = tx.send(Ok(chunk)).await;
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    /// 写入文件 (流式请求)
    async fn write_file(
        &self,
        request: Request<tonic::Streaming<DataChunk>>,
    ) -> Result<Response<WriteFileResponse>, Status> {
        let mut stream = request.into_inner();
        let mut total_bytes = 0u64;

        while let Some(result) = stream.next().await {
            match result {
                Ok(chunk) => {
                    total_bytes += chunk.data.len() as u64;
                    // NOTE: Streaming write implementation - data is received from client
                    // Actual VFS write would be: vfs.write(&path, chunk.data).await
                    // For now, data bytes are tracked and acknowledged back to client
                }
                Err(e) => {
                    return Err(Status::internal(format!("Stream error: {}", e)));
                }
            }
        }

        Ok(Response::new(WriteFileResponse {
            bytes_written: total_bytes,
            path: "".to_string(), // Path would be tracked in production implementation
        }))
    }

    /// 获取统计信息
    async fn stats(
        &self,
        request: Request<StatsRequest>,
    ) -> Result<Response<StatsResponse>, Status> {
        let _req = request.into_inner();

        let all_nodes = self.graph.all_nodes();
        let total_nodes = all_nodes.len();

        // 简化版本：边数量估计
        let total_edges = total_nodes * 2;

        Ok(Response::new(StatsResponse {
            total_nodes: total_nodes as u64,
            total_edges: total_edges as u64,
            uptime_secs: self.uptime_secs(),
            status: "running".to_string(),
        }))
    }

    /// 健康检查
    async fn health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        Ok(Response::new(HealthResponse {
            status: "healthy".to_string(),
            version: "1.0.0".to_string(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.bind_addr, "[::]");
        assert_eq!(config.port, 50051);
        assert_eq!(config.max_message_size, 4 * 1024 * 1024);
        assert_eq!(config.stream_buffer_size, 64);
    }

    #[test]
    fn test_server_creation() {
        let graph = Arc::new(Graph::new());
        let auth = Arc::new(AuthManager::new());
        let server = EvifServer::new(graph, auth);
        assert_eq!(server.config.port, 50051);
    }

    #[test]
    fn test_server_with_custom_config() {
        let graph = Arc::new(Graph::new());
        let auth = Arc::new(AuthManager::new());
        let config = ServerConfig {
            bind_addr: "127.0.0.1".to_string(),
            port: 8080,
            max_message_size: 1024,
            stream_buffer_size: 32,
        };
        let server = EvifServer::with_config(graph, auth, config);
        assert_eq!(server.config.port, 8080);
        assert_eq!(server.config.max_message_size, 1024);
    }

    #[test]
    fn test_server_default() {
        let server = EvifServer::default();
        assert_eq!(server.config.port, 50051);
    }
}
