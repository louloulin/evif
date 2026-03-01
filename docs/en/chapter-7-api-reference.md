# Chapter 7: API Reference (第七章:API 参考)

## Table of Contents (目录)

1. [API Overview (API 概述)](#api-overview-api-概述)
2. [REST API (REST API)](#rest-api-rest-api)
3. [gRPC API (gRPC API)](#grpc-api-grpc-api)
4. [Python Client SDK (Python 客户端 SDK)](#python-client-sdk-python-客户端-sdk)
5. [WebSocket API (WebSocket API)](#websocket-api-websocket-api)
6. [Error Handling (错误处理)](#error-handling-错误处理)
7. [Authentication and Security (认证与安全)](#authentication-and-security-认证与安全)
8. [Performance Optimization (性能优化)](#performance-optimization-性能优化)
9. [API Versioning (API 版本管理)](#api-versioning-api-版本管理)
10. [Examples and Best Practices (示例与最佳实践)](#examples-and-best-practices-示例与最佳实践)

---

## API Overview (API 概述)

### EVIF API Ecosystem (EVIF API 生态系统)

EVIF provides multiple access interfaces for different scenarios:

| API Type | Use Cases | Protocol | Features |
|---------|-----------|----------|----------|
| **REST API** | Web apps, quick integration | HTTP/JSON | Simple, widely supported |
| **gRPC API** | High-performance, microservices | HTTP/2 + Protobuf | Streaming, strongly-typed |
| **Python SDK** | Data science, automation | Python | Async, high-level abstractions |
| **WebSocket API** | Real-time notifications, events | WebSocket | Bidirectional, low latency |

### API Architecture (API 架构)

```
┌─────────────────────────────────────────────────────────┐
│                   Client Applications                   │
│  (Web Apps, CLI, Python Scripts, Microservices)         │
└──────────────┬──────────────┬──────────────┬───────────┘
               │              │              │
               ▼              ▼              ▼
        ┌──────────┐   ┌──────────┐   ┌──────────┐
        │   REST   │   │   gRPC   │   │WebSocket │
        │   API    │   │   API    │   │   API    │
        └────┬─────┘   └────┬─────┘   └────┬─────┘
             │              │              │
             └──────────────┼──────────────┘
                           ▼
                   ┌────────────────┐
                   │  API Gateway   │
                   │  (Axum/Tokio)  │
                   └────────┬───────┘
                            ▼
                   ┌────────────────┐
                   │   EVIF Core    │
                   │  (VFS + Mount) │
                   └────────┬───────┘
                            ▼
                   ┌────────────────┐
                   │   EVIF Plugin  │
                   │   (Storage)    │
                   └────────────────┘
```

### Basic Configuration (基础配置)

**Default Ports (默认端口):**
- REST API: `8080`
- gRPC API: `50051`
- WebSocket: `8080/ws`

**Base URLs (基础 URL):**
```
REST API:    http://localhost:8080/api/v1
gRPC API:    http://localhost:50051
WebSocket:   ws://localhost:8080/ws
```

---

## REST API (REST API)

### Overview (概述)

REST API is based on HTTP/JSON, providing a simple and easy-to-use filesystem and plugin management interface.

**Features (特性):**
- ✅ **Standard HTTP methods**: GET, POST, PUT, DELETE
- ✅ **JSON request/response**: Easy to parse and debug
- ✅ **Error handling**: Standard HTTP status codes + detailed error messages
- ✅ **Compatibility**: Compatible with AGFS API for easy migration

### Authentication (认证方式)

```bash
# API Key authentication (recommended)
curl -H "Authorization: Bearer YOUR_API_KEY" \
     http://localhost:8080/api/v1/health

# No authentication mode (development)
curl http://localhost:8080/api/v1/health
```

### Core API Endpoints (核心 API 端点)

#### 1. Health Check (健康检查)

**Check Service Status (检查服务状态)**

```http
GET /health
GET /api/v1/health
```

**Response Example (响应示例):**
```json
{
  "status": "ok",
  "version": "0.1.0",
  "uptime": 3600.5,
  "plugins_count": 5
}
```

**Client Example (客户端示例):**
```bash
# Check service health status
curl http://localhost:8080/api/v1/health

# Response: {"status":"ok","version":"0.1.0","uptime":3600.5,"plugins_count":5}
```

---

#### 2. File Operations (文件操作)

##### Read File (读取文件)

```http
GET /api/v1/files?path=/mount/example/test.txt
```

**Python Client Example (Python 客户端示例):**
```python
import asyncio
from evif import EvifClient

async def read_file():
    async with EvifClient("http://localhost:8080") as client:
        content = await client.read_file("/mount/example/test.txt")
        print(content)

asyncio.run(read_file())
```

**Rust Client Example (Rust 客户端示例):**
```rust
use evif_client::EvifClient;
use evif_client::ClientConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ClientConfig::default();
    let client = EvifClient::new(config).await?;

    let content = client.read_file("/mount/example/test.txt").await?;
    println!("{}", content);

    Ok(())
}
```

##### Write File (写入文件)

```http
PUT /api/v1/files
Content-Type: application/json

{
  "path": "/mount/example/test.txt",
  "content": "Hello, EVIF!",
  "offset": 0
}
```

**Response Example (响应示例):**
```json
{
  "success": true,
  "bytes_written": 13
}
```

##### Create File (创建文件)

```http
POST /api/v1/files
Content-Type: application/json

{
  "path": "/mount/example/newfile.txt",
  "mode": 420
}
```

##### Delete File (删除文件)

```http
DELETE /api/v1/files?path=/mount/example/test.txt
```

---

#### 3. Directory Operations (目录操作)

##### List Directory (列出目录)

```http
GET /api/v1/directories?path=/mount/example
```

**Response Example (响应示例):**
```json
{
  "entries": [
    {
      "name": "test.txt",
      "path": "/mount/example/test.txt",
      "size": 1024,
      "mode": 33188,
      "mtime": 1640995200.0,
      "is_dir": false,
      "is_file": true
    },
    {
      "name": "subdir",
      "path": "/mount/example/subdir",
      "size": 0,
      "mode": 16877,
      "mtime": 1640995200.0,
      "is_dir": true,
      "is_file": false
    }
  ]
}
```

**Python Example (Python 示例):**
```python
entries = await client.list_directory("/mount/example")
for entry in entries:
    print(f"{entry.name}: {'dir' if entry.is_dir else 'file'}")
```

##### Create Directory (创建目录)

```http
POST /api/v1/directories
Content-Type: application/json

{
  "path": "/mount/example/newdir",
  "mode": 493
}
```

##### Delete Directory (删除目录)

```http
DELETE /api/v1/directories?path=/mount/example/newdir
```

---

#### 4. Metadata Operations (元数据操作)

##### Get File Status (获取文件状态)

```http
GET /api/v1/stat?path=/mount/example/test.txt
```

**Response Example (响应示例):**
```json
{
  "path": "/mount/example/test.txt",
  "size": 1024,
  "mode": 33188,
  "mtime": 1640995200.0,
  "atime": 1640995200.0,
  "ctime": 1640995200.0
}
```

##### Compute File Hash (计算文件哈希)

```http
POST /api/v1/digest
Content-Type: application/json

{
  "path": "/mount/example/test.txt",
  "algorithm": "sha256"
}
```

**Response Example (响应示例):**
```json
{
  "hash": "a1b2c3d4e5f6...",
  "algorithm": "sha256"
}
```

##### Update Timestamps (Touch)

```http
POST /api/v1/touch
Content-Type: application/json

{
  "path": "/mount/example/test.txt",
  "mtime": 1640995200.0
}
```

---

#### 5. Advanced Operations (高级操作)

##### Grep (正则搜索)

```http
POST /api/v1/grep
Content-Type: application/json

{
  "path": "/mount/example",
  "pattern": "TODO",
  "recursive": true
}
```

**Response Example (响应示例):**
```json
{
  "matches": [
    {
      "path": "/mount/example/file1.txt",
      "line": 10,
      "content": "// TODO: implement feature"
    },
    {
      "path": "/mount/example/file2.txt",
      "line": 25,
      "content": "TODO: fix bug"
    }
  ]
}
```

##### Rename/Move (重命名/移动)

```http
POST /api/v1/rename
Content-Type: application/json

{
  "from": "/mount/example/old.txt",
  "to": "/mount/example/new.txt"
}
```

---

#### 6. Mount Management (挂载管理)

##### List Mounts (列出挂载点)

```http
GET /api/v1/mounts
```

**Response Example (响应示例):**
```json
{
  "mounts": [
    {
      "path": "/mount/memory",
      "plugin": "memory",
      "options": {
        "max_size": "1GB"
      }
    },
    {
      "path": "/mount/s3",
      "plugin": "s3",
      "options": {
        "bucket": "my-bucket",
        "region": "us-east-1"
      }
    }
  ]
}
```

##### Mount Plugin (挂载插件)

```http
POST /api/v1/mount
Content-Type: application/json

{
  "path": "/mount/s3",
  "plugin": "s3",
  "options": {
    "bucket": "my-bucket",
    "region": "us-east-1",
    "access_key": "AKIA...",
    "secret_key": "..."
  }
}
```

##### Unmount Plugin (卸载插件)

```http
POST /api/v1/unmount
Content-Type: application/json

{
  "path": "/mount/s3"
}
```

---

#### 7. Plugin Management (插件管理)

##### List Plugins (列出插件)

```http
GET /api/v1/plugins
```

**Response Example (响应示例):**
```json
{
  "plugins": [
    {
      "name": "memory",
      "version": "0.1.0",
      "description": "In-memory storage plugin",
      "enabled": true
    },
    {
      "name": "s3",
      "version": "0.1.0",
      "description": "AWS S3 storage plugin",
      "enabled": true
    }
  ]
}
```

##### Get Plugin README (获取插件 README)

```http
GET /api/v1/plugins/{name}/readme
```

##### Get Plugin Config Parameters (获取插件配置参数)

```http
GET /api/v1/plugins/{name}/config
```

**Response Example (响应示例):**
```json
{
  "name": "s3",
  "parameters": [
    {
      "name": "bucket",
      "type": "string",
      "required": true,
      "description": "S3 bucket name"
    },
    {
      "name": "region",
      "type": "string",
      "required": true,
      "description": "AWS region"
    }
  ]
}
```

##### Load External Plugin (加载外部插件)

```http
POST /api/v1/plugins/load
Content-Type: application/json

{
  "path": "/path/to/plugin.so",
  "config": {}
}
```

##### Load WASM Plugin (加载 WASM 插件)

```http
POST /api/v1/plugins/wasm/load
Content-Type: application/json

{
  "wasm_path": "/path/to/plugin.wasm",
  "config": {}
}
```

---

#### 8. Handle Operations (句柄操作)

##### Open File Handle (打开文件句柄)

```http
POST /api/v1/handles/open
Content-Type: application/json

{
  "path": "/mount/example/test.txt",
  "flags": "O_RDWR",
  "ttl": 3600
}
```

**Response Example (响应示例):**
```json
{
  "handle_id": 12345,
  "path": "/mount/example/test.txt",
  "flags": 2,
  "offset": 0,
  "expires_at": 1641000000.0
}
```

##### Get Handle Info (获取句柄信息)

```http
GET /api/v1/handles/{id}
```

##### Read from Handle (读取句柄数据)

```http
POST /api/v1/handles/{id}/read
Content-Type: application/json

{
  "size": 1024
}
```

##### Write to Handle (写入句柄数据)

```http
POST /api/v1/handles/{id}/write
Content-Type: application/json

{
  "data": "SGVsbG8sIEVWSUYh",
  "offset": 0
}
```

**Note**: The `data` field uses Base64 encoding.

##### Seek Operation (Seek 操作)

```http
POST /api/v1/handles/{id}/seek
Content-Type: application/json

{
  "offset": 100,
  "whence": "SEEK_SET"
}
```

##### Close Handle (关闭句柄)

```http
POST /api/v1/handles/{id}/close
```

##### List All Handles (列出所有句柄)

```http
GET /api/v1/handles
```

##### Get Handle Stats (获取句柄统计)

```http
GET /api/v1/handles/stats
```

**Response Example (响应示例):**
```json
{
  "total_handles": 10,
  "active_handles": 8,
  "expired_handles": 2
}
```

---

#### 9. Batch Operations (批量操作)

##### Batch Copy (批量复制)

```http
POST /api/v1/batch/copy
Content-Type: application/json

{
  "operations": [
    {
      "from": "/mount/src/file1.txt",
      "to": "/mount/dst/file1.txt"
    },
    {
      "from": "/mount/src/file2.txt",
      "to": "/mount/dst/file2.txt"
    }
  ]
}
```

**Response Example (响应示例):**
```json
{
  "operation_id": "batch-123",
  "status": "completed",
  "total": 2,
  "succeeded": 2,
  "failed": 0,
  "results": [
    {
      "operation": "copy",
      "from": "/mount/src/file1.txt",
      "to": "/mount/dst/file1.txt",
      "status": "success"
    },
    {
      "operation": "copy",
      "from": "/mount/src/file2.txt",
      "to": "/mount/dst/file2.txt",
      "status": "success"
    }
  ]
}
```

##### Batch Delete (批量删除)

```http
POST /api/v1/batch/delete
Content-Type: application/json

{
  "paths": [
    "/mount/example/file1.txt",
    "/mount/example/file2.txt",
    "/mount/example/file3.txt"
  ]
}
```

---

#### 10. Monitoring & Metrics (监控与指标)

##### Traffic Stats (流量统计)

```http
GET /api/v1/metrics/traffic
```

**Response Example (响应示例):**
```json
{
  "total_requests": 10000,
  "total_bytes_sent": 104857600,
  "total_bytes_received": 52428800,
  "requests_per_second": 100.5,
  "bytes_per_second": 1048576.0
}
```

##### Operation Stats (操作统计)

```http
GET /api/v1/metrics/operations
```

**Response Example (响应示例):**
```json
{
  "operations": {
    "read": 5000,
    "write": 3000,
    "create": 1000,
    "delete": 500,
    "list": 500
  },
  "average_latency_ms": {
    "read": 10.5,
    "write": 25.3,
    "create": 15.2,
    "delete": 8.7,
    "list": 20.1
  }
}
```

##### System Status (系统状态)

```http
GET /api/v1/metrics/status
```

**Response Example (响应示例):**
```json
{
  "status": "healthy",
  "uptime": 86400.0,
  "memory_usage_mb": 512.0,
  "cpu_usage_percent": 25.5,
  "active_handles": 10,
  "active_mounts": 5
}
```

##### Reset Metrics (重置指标)

```http
POST /api/v1/metrics/reset
```

---

#### 11. Collaboration Features (协作功能)

##### Create Share Link (创建分享链接)

```http
POST /api/v1/share/create
Content-Type: application/json

{
  "path": "/mount/example/document.txt",
  "permission": "read",
  "expires_in": 86400
}
```

**Response Example (响应示例):**
```json
{
  "share_id": "share-abc123",
  "share_url": "https://evif.example.com/share/share-abc123",
  "expires_at": 1641086400.0
}
```

##### List Shares (列出分享)

```http
GET /api/v1/share/list?path=/mount/example
```

##### Revoke Share (撤销分享)

```http
POST /api/v1/share/revoke
Content-Type: application/json

{
  "share_id": "share-abc123"
}
```

##### Set Permissions (设置权限)

```http
POST /api/v1/permissions/set
Content-Type: application/json

{
  "path": "/mount/example",
  "user": "user1",
  "permissions": ["read", "write"]
}
```

##### Get Permissions (获取权限)

```http
GET /api/v1/permissions/get?path=/mount/example&user=user1
```

##### List Comments (列出评论)

```http
GET /api/v1/comments?path=/mount/example/document.txt
```

##### Add Comment (添加评论)

```http
POST /api/v1/comments
Content-Type: application/json

{
  "path": "/mount/example/document.txt",
  "content": "Great work!",
  "user": "user1"
}
```

##### Get Activity Log (获取活动日志)

```http
GET /api/v1/activities?path=/mount/example&limit=50
```

**Response Example (响应示例):**
```json
{
  "activities": [
    {
      "timestamp": 1640995200.0,
      "user": "user1",
      "action": "modified",
      "path": "/mount/example/document.txt"
    },
    {
      "timestamp": 1640995100.0,
      "user": "user2",
      "action": "created",
      "path": "/mount/example/newfile.txt"
    }
  ]
}
```

---

### REST API Error Codes (REST API 错误码)

| HTTP Status Code | Meaning | Example Scenario |
|-----------------|---------|-----------------|
| 200 OK | Success | Request completed successfully |
| 400 Bad Request | Invalid request | Invalid JSON parameters |
| 401 Unauthorized | Not authenticated | Missing or invalid API key |
| 403 Forbidden | Access denied | Insufficient permissions |
| 404 Not Found | Resource not found | File or directory does not exist |
| 409 Conflict | Conflict | File already exists, directory not empty |
| 500 Internal Server Error | Internal error | Server internal error |
| 502 Bad Gateway | Gateway error | Upstream service error |
| 503 Service Unavailable | Unavailable | Service overloaded or maintenance |
| 504 Gateway Timeout | Timeout | Upstream service timeout |

**Error Response Format (错误响应格式):**
```json
{
  "error": "Not Found",
  "message": "File not found: /mount/example/test.txt"
}
```

---

## gRPC API (gRPC API)

### Overview (概述)

gRPC API is based on HTTP/2 and Protocol Buffers, providing high-performance streaming RPC interfaces.

**Features (特性):**
- ✅ **Strong typing**: Protocol Buffers provide type safety
- ✅ **Streaming**: Supports bidirectional streaming, server streaming, client streaming
- ✅ **High performance**: HTTP/2 multiplexing, binary encoding
- ✅ **Code generation**: Automatically generates multi-language clients

### Proto Definition (Proto 定义)

The complete proto definition is located at `crates/evif-grpc/proto/evif.proto`:

```protobuf
syntax = "proto3";

package evif;

service EvifService {
  // Node operations
  rpc GetNode(GetNodeRequest) returns (GetNodeResponse);
  rpc PutNode(PutNodeRequest) returns (PutNodeResponse);
  rpc DeleteNode(DeleteNodeRequest) returns (DeleteNodeResponse);

  // Batch operations
  rpc BatchGetNodes(BatchGetNodesRequest) returns (stream NodeResponse);
  rpc BatchPutNodes(stream PutNodeRequest) returns (BatchPutNodesResponse);

  // Query operations
  rpc Query(QueryRequest) returns (stream NodeResponse);

  // File operations
  rpc ReadFile(ReadFileRequest) returns (stream DataChunk);
  rpc WriteFile(stream DataChunk) returns (WriteFileResponse);

  // Statistics and management
  rpc Stats(StatsRequest) returns (StatsResponse);
  rpc Health(HealthRequest) returns (HealthResponse);
}

message Node {
  string id = 1;
  string node_type = 2;
  map<string, string> metadata = 3;
  map<string, Value> attributes = 4;
  bytes content = 5;
  int64 created_at = 6;
  int64 updated_at = 7;
}

message Value {
  oneof value {
    string string_value = 1;
    int64 int_value = 2;
    double double_value = 3;
    bool bool_value = 4;
    bytes bytes_value = 5;
    string string_array = 6;
  }
}
```

### Rust Client Example (Rust 客户端示例)

```rust
use evif_grpc::{EvifClient, ClientConfig};
use tonic::transport::Channel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to gRPC server
    let channel = Channel::from_static("http://localhost:50051").connect().await?;

    let mut client = EvifClient::new(channel);

    // Call GetNode RPC
    let request = GetNodeRequest {
        id: "node-123".to_string(),
    };

    let response = client.get_node(request).await?;
    println!("Node: {:?}", response.into_inner().node);

    // Streaming file read
    let read_request = ReadFileRequest {
        path: "/mount/example/test.txt".to_string(),
        offset: 0,
        size: 1024,
    };

    let mut stream = client.read_file(read_request).await?.into_inner();
    while let Some(chunk) = stream.message().await? {
        println!("Received chunk: {} bytes", chunk.data.len());
    }

    Ok(())
}
```

### Python Client Example (Python 客户端示例)

```python
import grpc
from evif_pb2 import GetNodeRequest, ReadFileRequest
from evif_pb2_grpc import EvifServiceStub

# Connect to gRPC server
channel = grpc.insecure_channel('localhost:50051')
client = EvifServiceStub(channel)

# Call GetNode RPC
request = GetNodeRequest(id="node-123")
response = client.GetNode(request)
print(f"Node: {response.node}")

# Streaming file read
read_request = ReadFileRequest(
    path="/mount/example/test.txt",
    offset=0,
    size=1024
)

for chunk in client.ReadFile(read_request):
    print(f"Received chunk: {len(chunk.data)} bytes")
```

### Go Client Example (Go 客户端示例)

```go
package main

import (
    "context"
    "fmt"
    "log"
    "io"

    pb "path/to/proto/evif"
    "google.golang.org/grpc"
)

func main() {
    // Connect to gRPC server
    conn, err := grpc.Dial("localhost:50051", grpc.WithInsecure())
    if err != nil {
        log.Fatalf("Failed to connect: %v", err)
    }
    defer conn.Close()

    client := pb.NewEvifServiceClient(conn)

    // Call GetNode RPC
    ctx := context.Background()
    req := &pb.GetNodeRequest{
        Id: "node-123",
    }

    resp, err := client.GetNode(ctx, req)
    if err != nil {
        log.Fatalf("GetNode failed: %v", err)
    }

    fmt.Printf("Node: %v\n", resp.Node)

    // Streaming file read
    readReq := &pb.ReadFileRequest{
        Path:   "/mount/example/test.txt",
        Offset: 0,
        Size:   1024,
    }

    stream, err := client.ReadFile(ctx, readReq)
    if err != nil {
        log.Fatalf("ReadFile failed: %v", err)
    }

    for {
        chunk, err := stream.Recv()
        if err == io.EOF {
            break
        }
        if err != nil {
            log.Fatalf("Stream error: %v", err)
        }
        fmt.Printf("Received chunk: %d bytes\n", len(chunk.Data))
    }
}
```

### gRPC Operations Detail (gRPC 操作详解)

#### 1. Node Operations (节点操作)

##### GetNode - Get Node (获取节点)

```protobuf
rpc GetNode(GetNodeRequest) returns (GetNodeResponse);
```

**Request (请求):**
```protobuf
message GetNodeRequest {
  string id = 1;
}
```

**Response (响应):**
```protobuf
message GetNodeResponse {
  Node node = 1;
}
```

##### PutNode - Create/Update Node (创建/更新节点)

```protobuf
rpc PutNode(PutNodeRequest) returns (PutNodeResponse);
```

**Request (请求):**
```protobuf
message PutNodeRequest {
  Node node = 1;
}
```

**Response (响应):**
```protobuf
message PutNodeResponse {
  string id = 1;
}
```

##### DeleteNode - Delete Node (删除节点)

```protobuf
rpc DeleteNode(DeleteNodeRequest) returns (DeleteNodeResponse);
```

---

#### 2. Batch Operations (批量操作)

##### BatchGetNodes - Batch Get Nodes (批量获取节点, Server Streaming)

```protobuf
rpc BatchGetNodes(BatchGetNodesRequest) returns (stream NodeResponse);
```

**Request (请求):**
```protobuf
message BatchGetNodesRequest {
  repeated string ids = 1;
}
```

**Response (响应):**
```protobuf
message NodeResponse {
  Node node = 1;
}
```

**Streaming response, one message per node. (流式响应,每个节点一个消息。)**

##### BatchPutNodes - Batch Create/Update Nodes (批量创建/更新节点, Client Streaming)

```protobuf
rpc BatchPutNodes(stream PutNodeRequest) returns (BatchPutNodesResponse);
```

**Response (响应):**
```protobuf
message BatchPutNodesResponse {
  repeated string ids = 1;
  int32 count = 2;
}
```

**Client streams nodes, server returns all created IDs. (客户端流式发送节点,服务端返回所有创建的 ID。)**

---

#### 3. Query Operations (查询操作)

##### Query - Query Nodes (查询节点, Server Streaming)

```protobuf
rpc Query(QueryRequest) returns (stream NodeResponse);
```

**Request (请求):**
```protobuf
message QueryRequest {
  string query = 1;
  uint32 limit = 2;
}
```

**Example (示例):**
```rust
let request = QueryRequest {
    query: "node_type='file' AND metadata.mount='s3'".to_string(),
    limit: 100,
};

let mut stream = client.query(request).await?.into_inner();
while let Some(response) = stream.message().await? {
    println!("Found node: {:?}", response.node);
}
```

---

#### 4. File Operations (文件操作)

##### ReadFile - Read File (读取文件, Server Streaming)

```protobuf
rpc ReadFile(ReadFileRequest) returns (stream DataChunk);
```

**Request (请求):**
```protobuf
message ReadFileRequest {
  string path = 1;
  uint64 offset = 2;
  uint64 size = 3;
}
```

**Response (响应):**
```protobuf
message DataChunk {
  bytes data = 1;
  uint64 offset = 2;
  bool eof = 3;
}
```

**Streaming response, large files transferred in chunks. (流式响应,大文件分块传输。)**

##### WriteFile - Write File (写入文件, Client Streaming)

```protobuf
rpc WriteFile(stream DataChunk) returns (WriteFileResponse);
```

**Response (响应):**
```protobuf
message WriteFileResponse {
  uint64 bytes_written = 1;
  string path = 2;
}
```

**Client streams data chunks, suitable for large file uploads. (客户端流式发送数据块,适合大文件上传。)**

**Example (示例):**
```rust
let file_data = std::fs::read("local_file.txt")?;
let chunk_size = 64 * 1024; // 64KB chunks

let mut stream = client.write_file().await?;

for (i, chunk) in file_data.chunks(chunk_size).enumerate() {
    let data_chunk = DataChunk {
        data: Bytes::from(chunk.to_vec()),
        offset: (i * chunk_size) as u64,
        eof: false,
    };

    stream.send(data_chunk).await?;
}

// Send EOF marker
let eof_chunk = DataChunk {
    data: Bytes::new(),
    offset: file_data.len() as u64,
    eof: true,
};

stream.send(eof_chunk).await?;

let response = stream.close().await?;
println!("Written {} bytes to {}", response.bytes_written, response.path);
```

---

#### 5. Statistics and Management (统计和管理)

##### Stats - Get Statistics (获取统计信息)

```protobuf
rpc Stats(StatsRequest) returns (StatsResponse);
```

**Request (请求):**
```protobuf
message StatsRequest {
  bool detailed = 1;
}
```

**Response (响应):**
```protobuf
message StatsResponse {
  uint64 total_nodes = 1;
  uint64 total_edges = 2;
  uint64 uptime_secs = 3;
  string status = 4;
}
```

##### Health - Health Check (健康检查)

```protobuf
rpc Health(HealthRequest) returns (HealthResponse);
```

**Request (请求):**
```protobuf
message HealthRequest {}
```

**Response (响应):**
```protobuf
message HealthResponse {
  string status = 1;
  string version = 2;
}
```

---

### gRPC Error Handling (gRPC 错误处理)

gRPC uses standard gRPC status codes:

| Status Code | Name | Description |
|-----------|------|-------------|
| 0 | OK | Success |
| 1 | CANCELLED | Operation cancelled |
| 3 | INVALID_ARGUMENT | Invalid argument |
| 5 | NOT_FOUND | Resource not found |
| 6 | ALREADY_EXISTS | Resource already exists |
| 7 | PERMISSION_DENIED | Permission denied |
| 8 | RESOURCE_EXHAUSTED | Resource exhausted |
| 9 | FAILED_PRECONDITION | Failed precondition |
| 10 | ABORTED | Operation aborted |
| 11 | OUT_OF_RANGE | Out of range |
| 12 | UNIMPLEMENTED | Unimplemented |
| 13 | INTERNAL | Internal error |
| 14 | UNAVAILABLE | Service unavailable |
| 15 | DATA_LOSS | Data loss |

**Error Mapping (错误映射):**
```rust
impl From<GrpcError> for Status {
    fn from(err: GrpcError) -> Self {
        match err {
            GrpcError::NotFound(msg) => Status::not_found(msg),
            GrpcError::Vfs(err) => Status::internal(err),
            GrpcError::Io(err) => Status::internal(err.to_string()),
            GrpcError::Protocol(msg) => Status::invalid_argument(msg),
            GrpcError::Internal(msg) => Status::internal(msg),
            GrpcError::AddrParse(err) => Status::internal(err.to_string()),
        }
    }
}
```

---

## Python Client SDK (Python 客户端 SDK)

### Overview (概述)

Python SDK provides high-level async interfaces to simplify EVIF usage.

**Features (特性):**
- ✅ **Async I/O**: Based on asyncio and httpx
- ✅ **Type hints**: Complete type annotations
- ✅ **Error handling**: Structured exceptions and retry mechanism
- ✅ **Context managers**: Automatic connection management
- ✅ **High-level abstractions**: File handles, mount management, etc.

### Installation (安装)

```bash
# Install from PyPI (when published)
pip install evif

# Install from local development version
cd crates/evif-python
pip install -e .
```

### Quick Start (快速开始)

```python
import asyncio
from evif import EvifClient

async def main():
    # Create client
    async with EvifClient("http://localhost:8080") as client:
        # Health check
        health = await client.health()
        print(f"Status: {health.status}, Version: {health.version}")

        # Read file
        content = await client.read_file("/mount/memory/test.txt")
        print(f"Content: {content}")

        # Write file
        await client.write_file("/mount/memory/hello.txt", "Hello, EVIF!")

        # List directory
        entries = await client.list_directory("/mount/memory")
        for entry in entries:
            print(f"  {entry.name}: {'dir' if entry.is_dir else 'file'}")

asyncio.run(main())
```

### Core API (核心 API)

#### 1. Client Configuration (客户端配置)

```python
from evif import EvifClient

# Basic configuration
client = EvifClient(
    base_url="http://localhost:8080",
    timeout=30.0,
    max_retries=3,
    api_key="your-api-key"  # Optional
)

# Use context manager
async with client:
    # Operations...
    pass
```

#### 2. File Operations (文件操作)

##### Read File (读取文件)

```python
# Read entire file
content = await client.read_file("/mount/memory/test.txt")
print(content)

# Read large file using file handle
async with client.open_file("/mount/memory/large.txt", "r") as f:
    while True:
        chunk = await f.read(4096)
        if not chunk:
            break
        print(chunk, end="")
```

##### Write File (写入文件)

```python
# Write string
await client.write_file("/mount/memory/test.txt", "Hello, EVIF!")

# Write large file using file handle
async with client.open_file("/mount/memory/large.txt", "w") as f:
    for i in range(1000):
        await f.write(f"Line {i}\n")
```

##### Create File (创建文件)

```python
await client.create_file("/mount/memory/newfile.txt", mode=0o644)
```

##### Delete File (删除文件)

```python
await client.delete_file("/mount/memory/test.txt")
```

---

#### 3. Directory Operations (目录操作)

##### List Directory (列出目录)

```python
entries = await client.list_directory("/mount/memory")

for entry in entries:
    if entry.is_dir:
        print(f"DIR  {entry.name}/")
    else:
        print(f"FILE {entry.name} ({entry.size} bytes)")
```

##### Create Directory (创建目录)

```python
await client.create_directory("/mount/memory/subdir", mode=0o755)
```

##### Delete Directory (删除目录)

```python
await client.delete_directory("/mount/memory/subdir", recursive=False)
```

---

#### 4. Metadata Operations (元数据操作)

##### Get File Info (获取文件信息)

```python
info = await client.stat("/mount/memory/test.txt")
print(f"Size: {info.size} bytes")
print(f"Mode: {oct(info.mode)}")
print(f"Modified: {info.mtime}")
```

##### Compute File Hash (计算文件哈希)

```python
hash_value = await client.digest("/mount/memory/test.txt", algorithm="sha256")
print(f"SHA256: {hash_value}")
```

##### Update Timestamps (Touch)

```python
import time
await client.touch("/mount/memory/test.txt", mtime=time.time())
```

---

#### 5. Advanced Operations (高级操作)

##### Grep (正则搜索)

```python
matches = await client.grep(
    path="/mount/memory",
    pattern=r"TODO|FIXME",
    recursive=True
)

for match in matches:
    print(f"{match.path}:{match.line}: {match.content}")
```

##### Rename/Move (重命名/移动)

```python
await client.rename(
    from_path="/mount/memory/old.txt",
    to_path="/mount/memory/new.txt"
)
```

---

#### 6. Mount Management (挂载管理)

##### List Mounts (列出挂载点)

```python
mounts = await client.list_mounts()

for mount in mounts:
    print(f"{mount.path}: {mount.plugin}")
    for key, value in mount.options.items():
        print(f"  {key}: {value}")
```

##### Mount Plugin (挂载插件)

```python
await client.mount(
    path="/mount/s3",
    plugin="s3",
    options={
        "bucket": "my-bucket",
        "region": "us-east-1",
        "access_key": "AKIA...",
        "secret_key": "..."
    }
)
```

##### Unmount Plugin (卸载插件)

```python
await client.unmount("/mount/s3")
```

---

#### 7. Plugin Management (插件管理)

##### List Plugins (列出插件)

```python
plugins = await client.list_plugins()

for plugin in plugins:
    status = "enabled" if plugin.enabled else "disabled"
    print(f"{plugin.name} v{plugin.version} [{status}]")
    print(f"  {plugin.description}")
```

##### Load External Plugin (加载外部插件)

```python
await client.load_plugin(
    path="/path/to/plugin.so",
    config={}
)
```

---

#### 8. File Handles (文件句柄)

Python SDK provides high-level file handle API:

```python
from evif import FileHandle

# Open file handle
handle: FileHandle = await client.open_handle(
    path="/mount/memory/test.txt",
    flags="O_RDWR"
)

try:
    # Seek operation
    await handle.seek(100, whence="SEEK_SET")

    # Read data
    data = await handle.read(1024)

    # Write data
    await handle.write(b"Hello, World!")

    # Sync to disk
    await handle.sync()

    # Get handle info
    info = await client.get_handle_info(handle.id)
    print(f"Offset: {info.offset}")

finally:
    # Close handle
    await client.close_handle(handle.id)
```

**Context Manager Approach (上下文管理器方式):**

```python
async with await client.open_handle("/mount/memory/test.txt", "O_RDWR") as f:
    await f.seek(0)
    data = await f.read(1024)
    await f.write(b"New content")
    # Automatically closes handle
```

---

### Error Handling (错误处理)

Python SDK provides structured exception hierarchy:

```python
from evif.exceptions import (
    EvifError,
    ClientError,
    AuthenticationError,
    FileNotFoundError,
    PermissionError,
    TimeoutError,
    ValidationError,
)

try:
    content = await client.read_file("/mount/memory/test.txt")
except AuthenticationError:
    print("API key is invalid or expired")
except FileNotFoundError:
    print("File not found")
except PermissionError:
    print("Insufficient permissions")
except TimeoutError:
    print("Request timeout, please retry later")
except ValidationError as e:
    print(f"Validation error: {e}")
except EvifError as e:
    print(f"EVIF error: {e}")
```

### Retry Mechanism (重试机制)

SDK uses `tenacity` library to automatically retry failed requests:

```python
client = EvifClient(
    base_url="http://localhost:8080",
    max_retries=3,  # Maximum retry attempts
    timeout=30.0    # Request timeout
)

# Internal retry strategy:
# - Exponential backoff: 1s, 2s, 4s, ...
# - Only retry idempotent operations (GET, HEAD)
# - Don't retry client errors (4xx)
```

### Type Definitions (类型定义)

All models are defined using Pydantic:

```python
from evif.models import FileInfo, MountInfo, HealthStatus, HandleInfo

# File information
info = FileInfo(
    name="test.txt",
    path="/mount/memory/test.txt",
    size=1024,
    mode=33188,
    mtime=1640995200.0,
    is_dir=False,
    is_file=True
)

# Mount information
mount = MountInfo(
    path="/mount/s3",
    plugin="s3",
    options={"bucket": "my-bucket", "region": "us-east-1"}
)

# Health status
health = HealthStatus(
    status="ok",
    version="0.1.0",
    uptime=3600.5,
    plugins_count=5
)

# Handle information
handle_info = HandleInfo(
    id=12345,
    path="/mount/memory/test.txt",
    flags=2,
    offset=0,
    expires_at=1641000000.0
)
```

---

## WebSocket API (WebSocket API)

### Overview (概述)

WebSocket API provides real-time bidirectional communication for event notifications and real-time updates.

**Features (特性):**
- ✅ **Real-time notifications**: File changes, mount status changes, etc.
- ✅ **Bidirectional communication**: Clients can send commands
- ✅ **Low latency**: Persistent connection, avoids HTTP overhead
- ✅ **Auto-reconnect**: Automatically reconnects on disconnect

### Connection (连接)

```javascript
// JavaScript client example
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onopen = () => {
  console.log('WebSocket connected');
};

ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  console.log('Received:', message);

  switch (message.type) {
    case 'file_changed':
      console.log(`File ${message.path} changed`);
      break;
    case 'mount_added':
      console.log(`Mount added: ${message.path}`);
      break;
    case 'mount_removed':
      console.log(`Mount removed: ${message.path}`);
      break;
  }
};

ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

ws.onclose = () => {
  console.log('WebSocket disconnected');
};
```

**Python Client Example (Python 客户端示例):**

```python
import asyncio
import websockets
import json

async def websocket_client():
    uri = "ws://localhost:8080/ws"

    async with websockets.connect(uri) as websocket:
        print("WebSocket connected")

        while True:
            message = await websocket.recv()
            data = json.loads(message)

            print(f"Received: {data}")

            if data['type'] == 'file_changed':
                print(f"File {data['path']} changed")
            elif data['type'] == 'mount_added':
                print(f"Mount added: {data['path']}")

asyncio.run(websocket_client())
```

### Message Format (消息格式)

**Generic Message Structure (通用消息结构):**

```json
{
  "type": "message_type",
  "timestamp": 1640995200.0,
  "data": { }
}
```

### Event Types (事件类型)

#### 1. File Change Events (文件变更事件)

```json
{
  "type": "file_changed",
  "timestamp": 1640995200.0,
  "data": {
    "path": "/mount/memory/test.txt",
    "operation": "modified",
    "size": 1024
  }
}
```

**Operation Types (操作类型):**
- `created`: File created
- `modified`: File modified
- `deleted`: File deleted
- `renamed`: File renamed

#### 2. Mount Events (挂载事件)

```json
{
  "type": "mount_added",
  "timestamp": 1640995200.0,
  "data": {
    "path": "/mount/s3",
    "plugin": "s3",
    "options": {
      "bucket": "my-bucket"
    }
  }
}
```

```json
{
  "type": "mount_removed",
  "timestamp": 1640995200.0,
  "data": {
    "path": "/mount/s3"
  }
}
```

#### 3. Error Events (错误事件)

```json
{
  "type": "error",
  "timestamp": 1640995200.0,
  "data": {
    "code": "VFS_ERROR",
    "message": "Failed to read file",
    "path": "/mount/memory/test.txt"
  }
}
```

#### 4. Heartbeat Events (心跳事件)

```json
{
  "type": "heartbeat",
  "timestamp": 1640995200.0,
  "data": {
    "uptime": 3600.5,
    "active_handles": 10,
    "active_mounts": 5
  }
}
```

### Client Commands (客户端命令)

Clients can send commands through WebSocket:

```javascript
// Subscribe to specific path events
ws.send(JSON.stringify({
  type: 'subscribe',
  data: {
    path: '/mount/memory'
  }
}));

// Unsubscribe
ws.send(JSON.stringify({
  type: 'unsubscribe',
  data: {
    path: '/mount/memory'
  }
}));

// Ping (keep connection alive)
ws.send(JSON.stringify({
  type: 'ping'
}));
```

---

## Error Handling (错误处理)

### REST API Errors (REST API 错误)

**Error Response Format (错误响应格式):**

```json
{
  "error": "Not Found",
  "message": "File not found: /mount/example/test.txt"
}
```

### gRPC Errors (gRPC 错误)

gRPC uses standard gRPC status codes, see [gRPC Error Handling](#grpc-error-handling-grpc-错误处理) section.

### Python SDK Errors (Python SDK 错误)

**Exception Hierarchy (异常层次):**

```
EvifError
├── ClientError
│   ├── AuthenticationError
│   ├── FileNotFoundError
│   ├── PermissionError
│   ├── TimeoutError
│   └── ValidationError
└── ...
```

**Error Handling Example (错误处理示例):**

```python
from evif.exceptions import *

try:
    content = await client.read_file("/mount/memory/test.txt")
except AuthenticationError:
    # Invalid API key
    pass
except FileNotFoundError:
    # File not found
    pass
except PermissionError:
    # Insufficient permissions
    pass
except TimeoutError:
    # Request timeout
    pass
except ValidationError as e:
    # Parameter validation failed
    print(f"Validation error: {e}")
except EvifError as e:
    # Other EVIF errors
    print(f"EVIF error: {e}")
```

---

## Authentication and Security (认证与安全)

### API Key Authentication (API Key 认证)

**Set API Key (设置 API Key):**

```bash
# Set environment variable
export EVIF_API_KEY="your-api-key-here"

# Or set in request header
curl -H "Authorization: Bearer your-api-key-here" \
     http://localhost:8080/api/v1/health
```

**Python SDK:**

```python
client = EvifClient(
    base_url="http://localhost:8080",
    api_key="your-api-key-here"
)
```

### Generate API Key (生成 API Key)

```bash
# Generate API key using CLI
evif-cli api-key generate

# Or use REST API
curl -X POST http://localhost:8080/api/v1/api-keys \
     -H "Content-Type: application/json" \
     -d '{"description": "My API Key"}'
```

### Permission Management (权限管理)

**Set File Permissions (设置文件权限):**

```http
POST /api/v1/permissions/set
Content-Type: application/json

{
  "path": "/mount/example",
  "user": "user1",
  "permissions": ["read", "write"]
}
```

**Get File Permissions (获取文件权限):**

```http
GET /api/v1/permissions/get?path=/mount/example&user=user1
```

### Security Best Practices (安全最佳实践)

1. **Use HTTPS**: Always use HTTPS in production
2. **Regular API Key Rotation**: Rotate every 90 days
3. **Principle of Least Privilege**: Grant only necessary permissions
4. **Audit Logging**: Log all API access
5. **IP Whitelisting**: Only allow trusted IPs

---

## Performance Optimization (性能优化)

### Batch Operations (批量操作)

Use batch APIs to reduce network round trips:

```python
# ❌ Multiple individual requests
for file in files:
    await client.delete_file(file)

# ✅ Single batch request
await client.batch_delete(files)
```

### Concurrent Requests (并发请求)

Use concurrency to improve throughput:

```python
import asyncio

async def fetch_multiple_files(paths):
    tasks = [client.read_file(path) for path in paths]
    results = await asyncio.gather(*tasks)
    return results

paths = [
    "/mount/memory/file1.txt",
    "/mount/memory/file2.txt",
    "/mount/memory/file3.txt"
]

results = await fetch_multiple_files(paths)
```

### Client Cache (客户端缓存)

Enable client cache to reduce requests:

```python
client = EvifClient(
    base_url="http://localhost:8080",
    enable_cache=True,
    cache_ttl=300  # 5 minutes
)
```

### Connection Pool (连接池)

Configure connection pool size:

```python
client = EvifClient(
    base_url="http://localhost:8080",
    max_connections=100,
    max_keepalive_connections=20
)
```

### gRPC Streaming (gRPC 流式传输)

Use streaming for large files:

```rust
// Server streaming: read large file
let mut stream = client.read_file(request).await?.into_inner();
while let Some(chunk) = stream.message().await? {
    // Process each data chunk
}

// Client streaming: write large file
let mut stream = client.write_file().await?;
for chunk in chunks {
    stream.send(chunk).await?;
}
let response = stream.close().await?;
```

---

## API Versioning (API 版本管理)

### Versioning Strategy (版本策略)

EVIF uses URL path versioning:

```
/api/v1/...  # Current stable version
/api/v2/...  # Future version (backward incompatible)
```

### Version Compatibility (版本兼容性)

**Major Version (主版本):**
- Backward incompatible API changes
- Major version number change (v1 → v2)

**Minor Version (次版本):**
- Backward compatible new features
- Minor version number change (v1.0 → v1.1)

**Patch Version (补丁版本):**
- Backward compatible bug fixes
- Patch number change (v1.0.0 → v1.0.1)

### Deprecation Policy (弃用策略)

1. **Mark as Deprecated**: Add `Deprecated` header to API endpoints
2. **Update Documentation**: Mark deprecation date in documentation
3. **Grace Period**: At least 6 months before removal
4. **Migration Guide**: Provide guide to migrate to new API

**Deprecation Example (弃用示例):**

```http
GET /api/v1/old-endpoint
Deprecated: true
Sunset: 2025-12-31
Link: </api/v2/new-endpoint>; rel="successor-version"
```

---

## Examples and Best Practices (示例与最佳实践)

### Example 1: File Upload (示例 1: 文件上传)

**REST API:**

```bash
# Upload small file (< 1MB)
curl -X PUT http://localhost:8080/api/v1/files \
     -H "Content-Type: application/json" \
     -d '{
       "path": "/mount/s3/upload.txt",
       "content": "SGVsbG8sIFMzIQ==",
       "offset": 0
     }'
```

**Python SDK:**

```python
# Upload small file
await client.write_file("/mount/s3/upload.txt", "Hello, S3!")

# Upload large file (chunked)
async def upload_large_file(local_path, remote_path, chunk_size=1024*1024):
    async with await client.open_handle(remote_path, "O_WRONLY | O_CREAT") as f:
        with open(local_path, "rb") as local_file:
            while True:
                chunk = local_file.read(chunk_size)
                if not chunk:
                    break
                await f.write(chunk)

await upload_large_file("large_file.bin", "/mount/s3/large.bin")
```

**gRPC:**

```python
# Stream upload large file
async def upload_file_grpc(local_path, remote_path):
    with open(local_path, "rb") as f:
        async with client.write_file() as stream:
            offset = 0
            while True:
                chunk = f.read(64*1024)
                if not chunk:
                    break

                await stream.send(DataChunk(
                    data=chunk,
                    offset=offset,
                    eof=False
                ))

                offset += len(chunk)

            # Send EOF
            await stream.send(DataChunk(
                data=b"",
                offset=offset,
                eof=True
            ))

            response = await stream.close()
            print(f"Uploaded {response.bytes_written} bytes")
```

---

### Example 2: Batch Operations (示例 2: 批量操作)

**Batch Copy Files (批量复制文件):**

```python
async def batch_copy(src_files, dst_dir):
    operations = [
        {
            "from": src_file,
            "to": f"{dst_dir}/{os.path.basename(src_file)}"
        }
        for src_file in src_files
    ]

    result = await client.batch_copy(operations)

    print(f"Total: {result.total}")
    print(f"Succeeded: {result.succeeded}")
    print(f"Failed: {result.failed}")

    for item in result.results:
        if item["status"] == "success":
            print(f"✓ {item['from']} → {item['to']}")
        else:
            print(f"✗ {item['from']}: {item.get('error', 'Unknown error')}")

await batch_copy(
    ["/mount/src/file1.txt", "/mount/src/file2.txt"],
    "/mount/dst"
)
```

---

### Example 3: Real-time Monitoring (示例 3: 实时监控)

**Monitor File Changes Using WebSocket (使用 WebSocket 监控文件变化):**

```python
import asyncio
import websockets
import json

async def monitor_files(path):
    uri = "ws://localhost:8080/ws"

    async with websockets.connect(uri) as ws:
        # Subscribe to path
        await ws.send(json.dumps({
            "type": "subscribe",
            "data": {"path": path}
        }))

        # Listen for events
        async for message in ws:
            data = json.loads(message)

            if data["type"] == "file_changed":
                op = data["data"]["operation"]
                file_path = data["data"]["path"]
                print(f"[{op}] {file_path}")

asyncio.run(monitor_files("/mount/memory"))
```

---

### Best Practices (最佳实践)

#### 1. Error Handling (错误处理)

```python
# ✅ Good: Catch specific exceptions
try:
    content = await client.read_file(path)
except FileNotFoundError:
    logger.error(f"File not found: {path}")
except EvifError as e:
    logger.error(f"EVIF error: {e}")

# ❌ Bad: Catch all exceptions
try:
    content = await client.read_file(path)
except Exception:
    pass
```

#### 2. Resource Management (资源管理)

```python
# ✅ Good: Use context managers
async with await client.open_handle(path, "O_RDWR") as f:
    data = await f.read(1024)
    # Automatically closes handle

# ❌ Bad: Manual resource management
handle = await client.open_handle(path, "O_RDWR")
data = await handle.read(1024)
# Forgot to close handle
```

#### 3. Timeout Control (超时控制)

```python
# ✅ Good: Set timeout
try:
    content = await asyncio.wait_for(
        client.read_file(path),
        timeout=30.0
    )
except asyncio.TimeoutError:
    logger.error(f"Timeout reading {path}")

# ❌ Bad: No timeout
content = await client.read_file(path)  # May block forever
```

#### 4. Logging (日志记录)

```python
# ✅ Good: Log key operations
logger.info(f"Reading file: {path}")
content = await client.read_file(path)
logger.info(f"Read {len(content)} bytes from {path}")

# ❌ Bad: No logging
content = await client.read_file(path)
```

---

## Summary (总结)

EVIF provides a complete API ecosystem:

- **REST API**: Simple and easy-to-use HTTP/JSON interface
- **gRPC API**: High-performance binary RPC interface
- **Python SDK**: High-level async Python client
- **WebSocket API**: Real-time bidirectional communication

Choose the right API interface:
- **Web Applications**: Use REST API
- **Microservices**: Use gRPC API
- **Data Science**: Use Python SDK
- **Real-time Monitoring**: Use WebSocket API

**Related Chapters (相关章节):**
- [Chapter 3: Architecture (架构设计)](chapter-3-architecture.md)
- [Chapter 4: Virtual Filesystem (虚拟文件系统)](chapter-4-virtual-filesystem.md)
- [Chapter 5: Plugin Development (插件开发)](chapter-5-plugin-development.md)
- [Chapter 6: FUSE Integration (FUSE 集成)](chapter-6-fuse.md)
- [Chapter 8: Authentication and Security (认证与安全)](chapter-8-authentication-security.md)
