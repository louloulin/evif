// gRPC 集成测试
//
// 测试完整的 gRPC 服务器和客户端交互

use evif_grpc::{EvifServer, ServerConfig, ClientConfig, evif::evif_service_server::EvifService};
use evif_graph::{Graph, Node, NodeType, Attribute};
use evif_auth::AuthManager;
use std::sync::Arc;
use tokio::time::{timeout, Duration};
use tonic::Request;
use uuid::Uuid;
use tokio_stream::StreamExt;

#[tokio::test]
async fn test_server_creation() {
    let graph = Arc::new(Graph::new());
    let auth = Arc::new(AuthManager::new());
    let server = EvifServer::new(graph, auth);

    assert_eq!(server.config().port, 50051);
    assert_eq!(server.config().bind_addr, "[::]");
    assert_eq!(server.config().max_message_size, 4 * 1024 * 1024);
}

#[tokio::test]
async fn test_server_with_custom_config() {
    let graph = Arc::new(Graph::new());
    let auth = Arc::new(AuthManager::new());
    let config = ServerConfig {
        bind_addr: "127.0.0.1".to_string(),
        port: 8080,
        max_message_size: 1024 * 1024,
        stream_buffer_size: 32,
    };
    let server = EvifServer::with_config(graph, auth, config);

    assert_eq!(server.config().port, 8080);
    assert_eq!(server.config().bind_addr, "127.0.0.1");
    assert_eq!(server.config().max_message_size, 1024 * 1024);
}

#[tokio::test]
async fn test_get_node_nonexistent() {
    let graph = Arc::new(Graph::new());
    let auth = Arc::new(AuthManager::new());
    let server = EvifServer::new(graph, auth);

    let random_id = Uuid::new_v4().to_string();
    let request = Request::new(evif_grpc::GetNodeRequest {
        id: random_id.clone(),
    });

    let result = server.get_node(request).await;

    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn test_get_node_invalid_id() {
    let graph = Arc::new(Graph::new());
    let auth = Arc::new(AuthManager::new());
    let server = EvifServer::new(graph, auth);

    let request = Request::new(evif_grpc::GetNodeRequest {
        id: "invalid-uuid".to_string(),
    });

    let result = server.get_node(request).await;

    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_put_and_get_node() {
    let graph = Arc::new(Graph::new());
    let auth = Arc::new(AuthManager::new());
    let server = EvifServer::new(Arc::clone(&graph), auth);

    let node_id = Uuid::new_v4();
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("name".to_string(), "test_file.txt".to_string());

    let mut attributes = std::collections::HashMap::new();
    attributes.insert("size".to_string(), evif_grpc::Value {
        value: Some(evif_grpc::value::Value::IntValue(1024)),
    });

    let proto_node = evif_grpc::Node {
        id: node_id.to_string(),
        node_type: "file".to_string(),
        metadata: metadata.clone(),
        attributes: attributes.clone(),
        content: vec![],
        created_at: 0,
        updated_at: 0,
    };

    let put_request = Request::new(evif_grpc::PutNodeRequest {
        node: Some(proto_node.clone()),
    });

    let put_result = server.put_node(put_request).await;
    assert!(put_result.is_ok());
    let response = put_result.unwrap().into_inner();
    assert_eq!(response.id, node_id.to_string());

    let get_request = Request::new(evif_grpc::GetNodeRequest {
        id: node_id.to_string(),
    });

    let get_result = server.get_node(get_request).await;
    assert!(get_result.is_ok());
    let response = get_result.unwrap().into_inner();
    assert!(response.node.is_some());
    let retrieved_node = response.node.unwrap();
    assert_eq!(retrieved_node.id, node_id.to_string());
    assert_eq!(retrieved_node.node_type, "file");
    assert_eq!(retrieved_node.metadata.get("name"), Some(&"test_file.txt".to_string()));
}

#[tokio::test]
async fn test_delete_node() {
    let graph = Arc::new(Graph::new());
    let auth = Arc::new(AuthManager::new());
    let server = EvifServer::new(Arc::clone(&graph), auth);

    let node_id = Uuid::new_v4();
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("name".to_string(), "to_delete.txt".to_string());

    let proto_node = evif_grpc::Node {
        id: node_id.to_string(),
        node_type: "file".to_string(),
        metadata,
        attributes: std::collections::HashMap::new(),
        content: vec![],
        created_at: 0,
        updated_at: 0,
    };

    let put_request = Request::new(evif_grpc::PutNodeRequest {
        node: Some(proto_node),
    });
    let _ = server.put_node(put_request).await.unwrap();

    let delete_request = Request::new(evif_grpc::DeleteNodeRequest {
        id: node_id.to_string(),
    });
    let delete_result = server.delete_node(delete_request).await;
    assert!(delete_result.is_ok());
    let response = delete_result.unwrap().into_inner();
    assert!(response.success);

    let get_request = Request::new(evif_grpc::GetNodeRequest {
        id: node_id.to_string(),
    });
    let get_result = server.get_node(get_request).await;
    assert!(get_result.is_err());
}

#[tokio::test]
async fn test_batch_get_nodes() {
    let graph = Arc::new(Graph::new());
    let auth = Arc::new(AuthManager::new());
    let server = EvifServer::new(Arc::clone(&graph), auth);

    let node_ids = vec![
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    ];

    for id in &node_ids {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("name".to_string(), format!("node_{}.txt", id));

        let proto_node = evif_grpc::Node {
            id: id.to_string(),
            node_type: "file".to_string(),
            metadata,
            attributes: std::collections::HashMap::new(),
            content: vec![],
            created_at: 0,
            updated_at: 0,
        };

        let put_request = Request::new(evif_grpc::PutNodeRequest {
            node: Some(proto_node),
        });
        let _ = server.put_node(put_request).await.unwrap();
    }

    let ids_str: Vec<String> = node_ids.iter().map(|id| id.to_string()).collect();
    let batch_request = Request::new(evif_grpc::BatchGetNodesRequest {
        ids: ids_str.clone(),
    });

    let batch_result = server.batch_get_nodes(batch_request).await;
    assert!(batch_result.is_ok());

    let mut stream = batch_result.unwrap().into_inner();
    let mut count = 0;
    let timeout_duration = Duration::from_secs(5);

    while let Ok(Some(result)) = timeout(timeout_duration, stream.next()).await {
        assert!(result.is_ok());
        let node_response = result.unwrap();
        assert!(node_response.node.is_some());
        count += 1;
    }

    assert_eq!(count, 3);
}

#[tokio::test]
async fn test_query() {
    let graph = Arc::new(Graph::new());
    let auth = Arc::new(AuthManager::new());
    let server = EvifServer::new(Arc::clone(&graph), auth);

    let query_request = Request::new(evif_grpc::QueryRequest {
        query: "all".to_string(),
        limit: 10,
    });

    let query_result = server.query(query_request).await;
    assert!(query_result.is_ok());

    let mut stream = query_result.unwrap().into_inner();
    let timeout_duration = Duration::from_secs(5);

    if let Ok(Some(result)) = timeout(timeout_duration, stream.next()).await {
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_stats() {
    let graph = Arc::new(Graph::new());
    let auth = Arc::new(AuthManager::new());
    let server = EvifServer::new(graph, auth);

    let stats_request = Request::new(evif_grpc::StatsRequest {
        detailed: false,
    });

    let stats_result = server.stats(stats_request).await;
    assert!(stats_result.is_ok());

    let response = stats_result.unwrap().into_inner();
    assert_eq!(response.status, "running");
    assert!(response.uptime_secs > 0);
}

#[tokio::test]
async fn test_health() {
    let graph = Arc::new(Graph::new());
    let auth = Arc::new(AuthManager::new());
    let server = EvifServer::new(graph, auth);

    let health_request = Request::new(evif_grpc::HealthRequest {});

    let health_result = server.health(health_request).await;
    assert!(health_result.is_ok());

    let response = health_result.unwrap().into_inner();
    assert_eq!(response.status, "healthy");
    assert_eq!(response.version, "1.0.0");
}

#[tokio::test]
async fn test_read_file() {
    let graph = Arc::new(Graph::new());
    let auth = Arc::new(AuthManager::new());
    let server = EvifServer::new(graph, auth);

    let read_request = Request::new(evif_grpc::ReadFileRequest {
        path: "/test/path.txt".to_string(),
        offset: 0,
        size: 1024,
    });

    let read_result = server.read_file(read_request).await;
    assert!(read_result.is_ok());

    let mut stream = read_result.unwrap().into_inner();
    let timeout_duration = Duration::from_secs(5);

    if let Ok(Some(result)) = timeout(timeout_duration, stream.next()).await {
        assert!(result.is_ok());
        let chunk = result.unwrap();
        assert!(chunk.eof);
    }
}

#[tokio::test]
async fn test_node_conversion_graph_to_proto() {
    let graph = Arc::new(Graph::new());
    let auth = Arc::new(AuthManager::new());
    let server = EvifServer::new(graph, auth);

    let mut node = Node::new(NodeType::File, "test.txt");
    node.attributes.insert("size".to_string(), Attribute::Integer(2048));
    node.attributes.insert("mime_type".to_string(), Attribute::String("text/plain".to_string()));

    let proto_node = server.graph_node_to_proto(&node);

    assert_eq!(proto_node.node_type, "file");
    assert_eq!(proto_node.metadata.get("name"), Some(&"test.txt".to_string()));
    assert!(proto_node.attributes.contains_key("size"));
    assert!(proto_node.attributes.contains_key("mime_type"));
}

#[tokio::test]
async fn test_node_conversion_proto_to_graph() {
    let graph = Arc::new(Graph::new());
    let auth = Arc::new(AuthManager::new());
    let server = EvifServer::new(graph, auth);

    let node_id = Uuid::new_v4();
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("name".to_string(), "converted.txt".to_string());

    let mut attributes = std::collections::HashMap::new();
    attributes.insert("attr1".to_string(), evif_grpc::Value {
        value: Some(evif_grpc::value::Value::StringValue("value1".to_string())),
    });

    let proto_node = evif_grpc::Node {
        id: node_id.to_string(),
        node_type: "file".to_string(),
        metadata,
        attributes,
        content: vec![],
        created_at: 0,
        updated_at: 0,
    };

    let result = server.proto_node_to_graph(proto_node);
    assert!(result.is_ok());

    let graph_node = result.unwrap();
    assert_eq!(graph_node.id, node_id);
    assert_eq!(graph_node.name, "converted.txt");
    assert_eq!(graph_node.node_type, NodeType::File);
}

#[tokio::test]
async fn test_proto_to_graph_invalid_uuid() {
    let graph = Arc::new(Graph::new());
    let auth = Arc::new(AuthManager::new());
    let server = EvifServer::new(graph, auth);

    let metadata = std::collections::HashMap::new();
    let attributes = std::collections::HashMap::new();

    let proto_node = evif_grpc::Node {
        id: "invalid-uuid".to_string(),
        node_type: "file".to_string(),
        metadata,
        attributes,
        content: vec![],
        created_at: 0,
        updated_at: 0,
    };

    let result = server.proto_node_to_graph(proto_node);
    assert!(result.is_err());
}

#[test]
fn test_client_config_default() {
    let config = ClientConfig::default();
    assert_eq!(config.server_addr, "http://[::1]:50051");
    assert_eq!(config.connect_timeout_secs, 10);
    assert_eq!(config.max_message_size, 4 * 1024 * 1024);
    assert!(!config.enable_tls);
    assert_eq!(config.max_concurrent_requests, 100);
}

#[test]
fn test_server_config_default() {
    let config = ServerConfig::default();
    assert_eq!(config.bind_addr, "[::]");
    assert_eq!(config.port, 50051);
    assert_eq!(config.max_message_size, 4 * 1024 * 1024);
    assert_eq!(config.stream_buffer_size, 64);
}

#[tokio::test]
async fn test_graph_node_to_proto_all_attribute_types() {
    let graph = Arc::new(Graph::new());
    let auth = Arc::new(AuthManager::new());
    let server = EvifServer::new(graph, auth);

    let mut node = Node::new(NodeType::File, "test.txt");
    node.attributes.insert("string_attr".to_string(), Attribute::String("test".to_string()));
    node.attributes.insert("int_attr".to_string(), Attribute::Integer(42));
    node.attributes.insert("float_attr".to_string(), Attribute::Float(3.14));
    node.attributes.insert("bool_attr".to_string(), Attribute::Boolean(true));
    node.attributes.insert("bytes_attr".to_string(), Attribute::Binary(vec![1, 2, 3, 4]));
    node.attributes.insert("datetime_attr".to_string(), Attribute::DateTime(chrono::Utc::now()));
    node.attributes.insert("null_attr".to_string(), Attribute::Null);

    let proto_node = server.graph_node_to_proto(&node);

    assert_eq!(proto_node.attributes.len(), 7);

    // Verify string attribute
    if let Some(evif_grpc::Value { value: Some(evif_grpc::value::Value::StringValue(s)) }) =
        proto_node.attributes.get("string_attr") {
        assert_eq!(s, "test");
    } else {
        panic!("String attribute conversion failed");
    }

    // Verify int attribute
    if let Some(evif_grpc::Value { value: Some(evif_grpc::value::Value::IntValue(i)) }) =
        proto_node.attributes.get("int_attr") {
        assert_eq!(*i, 42);
    } else {
        panic!("Int attribute conversion failed");
    }

    // Verify float attribute
    if let Some(evif_grpc::Value { value: Some(evif_grpc::value::Value::DoubleValue(f)) }) =
        proto_node.attributes.get("float_attr") {
        assert!((f - 3.14) < 0.001);
    } else {
        panic!("Float attribute conversion failed");
    }

    // Verify bool attribute
    if let Some(evif_grpc::Value { value: Some(evif_grpc::value::Value::BoolValue(b)) }) =
        proto_node.attributes.get("bool_attr") {
        assert!(b);
    } else {
        panic!("Bool attribute conversion failed");
    }

    // Verify bytes attribute
    if let Some(evif_grpc::Value { value: Some(evif_grpc::value::Value::BytesValue(b)) }) =
        proto_node.attributes.get("bytes_attr") {
        assert_eq!(b.clone(), vec![1, 2, 3, 4]);
    } else {
        panic!("Bytes attribute conversion failed");
    }
}

#[tokio::test]
async fn test_batch_get_nodes_with_invalid_ids() {
    let graph = Arc::new(Graph::new());
    let auth = Arc::new(AuthManager::new());
    let server = EvifServer::new(Arc::clone(&graph), auth);

    let ids = vec![
        Uuid::new_v4().to_string(),
        "invalid-uuid".to_string(),
        Uuid::new_v4().to_string(),
    ];

    let batch_request = Request::new(evif_grpc::BatchGetNodesRequest { ids });

    let batch_result = server.batch_get_nodes(batch_request).await;
    assert!(batch_result.is_ok());

    let mut stream = batch_result.unwrap().into_inner();
    let timeout_duration = Duration::from_secs(5);
    let mut error_count = 0;

    while let Ok(Some(result)) = timeout(timeout_duration, stream.next()).await {
        if result.is_err() {
            error_count += 1;
        }
    }

    assert!(error_count > 0, "Expected at least one error for invalid UUID");
}
