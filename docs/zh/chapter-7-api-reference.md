# 第七章：API 参考 (Chapter 7: API Reference)

## 目录 (Table of Contents)

1. [API 概述 (API Overview)](#api-概述-api-overview)
2. [REST API (REST API)](#rest-api-rest-api)
3. [gRPC API (gRPC API)](#grpc-api-grpc-api)
4. [Python 客户端 SDK (Python Client SDK)](#python-客户端-sdk-python-client-sdk)
5. [WebSocket API (WebSocket API)](#websocket-api-websocket-api)
6. [错误处理 (Error Handling)](#错误处理-error-handling)
7. [认证与安全 (Authentication and Security)](#认证与安全-authentication-and-security)
8. [性能优化 (Performance Optimization)](#性能优化-performance-optimization)
9. [API 版本管理 (API Versioning)](#api-版本管理-api-versioning)
10. [示例与最佳实践 (Examples and Best Practices)](#示例与最佳实践-examples-and-best-practices)

---

## API 概述 (API Overview)

### EVIF API 生态系统 (EVIF API Ecosystem)

EVIF 提供多种访问接口，满足不同场景需求：

| API 类型 | 适用场景 | 协议 | 特性 |
|---------|---------|------|------|
| **REST API** | Web 应用、快速集成 | HTTP/JSON | 简单易用、广泛支持 |
| **gRPC API** | 高性能、微服务 | HTTP/2 + Protobuf | 流式传输、强类型 |
| **Python SDK** | 数据科学、自动化 | Python | 异步、高级抽象 |
| **WebSocket API** | 实时通知、事件 | WebSocket | 双向通信、低延迟 |

### API 架构 (API Architecture)

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

### 基础配置 (Basic Configuration)

**默认端口 (Default Ports):**
- REST API: `8080`
- gRPC API: `50051`
- WebSocket: `8080/ws`

**基础 URL (Base URLs):**
```
REST API:    http://localhost:8080/api/v1
gRPC API:    http://localhost:50051
WebSocket:   ws://localhost:8080/ws
```

---

## REST API (REST API)

### 概述 (Overview)

REST API 基于 HTTP/JSON，提供简单易用的文件系统和插件管理接口。

**特性 (Features):**
- ✅ **标准 HTTP 方法**: GET、POST、PUT、DELETE
- ✅ **JSON 请求/响应**: 易于解析和调试
- ✅ **错误处理**: 标准 HTTP 状态码 + 详细错误信息
- ✅ **兼容性**: 兼容 AGFS API，便于迁移

### 认证方式 (Authentication)

```bash
# API Key 认证 (推荐)
curl -H "Authorization: Bearer YOUR_API_KEY" \
     http://localhost:8080/api/v1/health

# 无认证模式 (开发环境)
curl http://localhost:8080/api/v1/health
```

### 核心 API 端点 (Core API Endpoints)

#### 1. 健康检查 (Health Check)

**检查服务状态 (Check Service Status)**

```http
GET /health
GET /api/v1/health
```

**响应示例 (Response Example):**
```json
{
  "status": "ok",
  "version": "0.1.0",
  "uptime": 3600.5,
  "plugins_count": 5
}
```

**客户端示例 (Client Example):**
```bash
# 检查服务健康状态
curl http://localhost:8080/api/v1/health

# 响应: {"status":"ok","version":"0.1.0","uptime":3600.5,"plugins_count":5}
```

---

#### 2. 文件操作 (File Operations)

##### 读取文件 (Read File)

```http
GET /api/v1/files?path=/mount/example/test.txt
```

**Python 客户端示例 (Python Client Example):**
```python
import asyncio
from evif import EvifClient

async def read_file():
    async with EvifClient("http://localhost:8080") as client:
        content = await client.read_file("/mount/example/test.txt")
        print(content)

asyncio.run(read_file())
```

**Rust 客户端示例 (Rust Client Example):**
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

##### 写入文件 (Write File)

```http
PUT /api/v1/files
Content-Type: application/json

{
  "path": "/mount/example/test.txt",
  "content": "Hello, EVIF!",
  "offset": 0
}
```

**响应示例 (Response):**
```json
{
  "success": true,
  "bytes_written": 13
}
```

##### 创建文件 (Create File)

```http
POST /api/v1/files
Content-Type: application/json

{
  "path": "/mount/example/newfile.txt",
  "mode": 0o644
}
```

##### 删除文件 (Delete File)

```http
DELETE /api/v1/files?path=/mount/example/test.txt
```

---

#### 3. 目录操作 (Directory Operations)

##### 列出目录 (List Directory)

```http
GET /api/v1/directories?path=/mount/example
```

**响应示例 (Response):**
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

**Python 示例 (Python Example):**
```python
entries = await client.list_directory("/mount/example")
for entry in entries:
    print(f"{entry.name}: {'dir' if entry.is_dir else 'file'}")
```

##### 创建目录 (Create Directory)

```http
POST /api/v1/directories
Content-Type: application/json

{
  "path": "/mount/example/newdir",
  "mode": 0o755
}
```

##### 删除目录 (Delete Directory)

```http
DELETE /api/v1/directories?path=/mount/example/newdir
```

---

#### 4. 元数据操作 (Metadata Operations)

##### 获取文件状态 (Get File Status)

```http
GET /api/v1/stat?path=/mount/example/test.txt
```

**响应示例 (Response):**
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

##### 计算文件哈希 (Compute File Hash)

```http
POST /api/v1/digest
Content-Type: application/json

{
  "path": "/mount/example/test.txt",
  "algorithm": "sha256"
}
```

**响应示例 (Response):**
```json
{
  "hash": "a1b2c3d4e5f6...",
  "algorithm": "sha256"
}
```

##### 更新时间戳 (Touch)

```http
POST /api/v1/touch
Content-Type: application/json

{
  "path": "/mount/example/test.txt",
  "mtime": 1640995200.0
}
```

---

#### 5. 高级操作 (Advanced Operations)

##### 正则搜索 (Grep)

```http
POST /api/v1/grep
Content-Type: application/json

{
  "path": "/mount/example",
  "pattern": "TODO",
  "recursive": true
}
```

**响应示例 (Response):**
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

##### 重命名/移动 (Rename/Move)

```http
POST /api/v1/rename
Content-Type: application/json

{
  "from": "/mount/example/old.txt",
  "to": "/mount/example/new.txt"
}
```

---

#### 6. 挂载管理 (Mount Management)

##### 列出挂载点 (List Mounts)

```http
GET /api/v1/mounts
```

**响应示例 (Response):**
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

##### 挂载插件 (Mount Plugin)

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

##### 卸载插件 (Unmount Plugin)

```http
POST /api/v1/unmount
Content-Type: application/json

{
  "path": "/mount/s3"
}
```

---

#### 7. 插件管理 (Plugin Management)

##### 列出插件 (List Plugins)

```http
GET /api/v1/plugins
```

**响应示例 (Response):**
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

##### 获取插件 README (Get Plugin README)

```http
GET /api/v1/plugins/{name}/readme
```

##### 获取插件配置参数 (Get Plugin Config)

```http
GET /api/v1/plugins/{name}/config
```

**响应示例 (Response):**
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

##### 加载外部插件 (Load External Plugin)

```http
POST /api/v1/plugins/load
Content-Type: application/json

{
  "path": "/path/to/plugin.so",
  "config": {}
}
```

##### 加载 WASM 插件 (Load WASM Plugin)

```http
POST /api/v1/plugins/wasm/load
Content-Type: application/json

{
  "wasm_path": "/path/to/plugin.wasm",
  "config": {}
}
```

---

#### 8. 句柄操作 (Handle Operations)

##### 打开文件句柄 (Open File Handle)

```http
POST /api/v1/handles/open
Content-Type: application/json

{
  "path": "/mount/example/test.txt",
  "flags": "O_RDWR",
  "ttl": 3600
}
```

**响应示例 (Response):**
```json
{
  "handle_id": 12345,
  "path": "/mount/example/test.txt",
  "flags": 2,
  "offset": 0,
  "expires_at": 1641000000.0
}
```

##### 获取句柄信息 (Get Handle Info)

```http
GET /api/v1/handles/{id}
```

##### 读取句柄数据 (Read from Handle)

```http
POST /api/v1/handles/{id}/read
Content-Type: application/json

{
  "size": 1024
}
```

##### 写入句柄数据 (Write to Handle)

```http
POST /api/v1/handles/{id}/write
Content-Type: application/json

{
  "data": "SGVsbG8sIEVWSUYh",
  "offset": 0
}
```

**注意**: `data` 字段使用 Base64 编码。

##### Seek 操作 (Seek Operation)

```http
POST /api/v1/handles/{id}/seek
Content-Type: application/json

{
  "offset": 100,
  "whence": "SEEK_SET"
}
```

##### 关闭句柄 (Close Handle)

```http
POST /api/v1/handles/{id}/close
```

##### 列出所有句柄 (List All Handles)

```http
GET /api/v1/handles
```

##### 获取句柄统计 (Get Handle Stats)

```http
GET /api/v1/handles/stats
```

**响应示例 (Response):**
```json
{
  "total_handles": 10,
  "active_handles": 8,
  "expired_handles": 2
}
```

---

#### 9. 批量操作 (Batch Operations)

##### 批量复制 (Batch Copy)

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

**响应示例 (Response):**
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

##### 批量删除 (Batch Delete)

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

#### 10. 监控与指标 (Monitoring & Metrics)

##### 流量统计 (Traffic Stats)

```http
GET /api/v1/metrics/traffic
```

**响应示例 (Response):**
```json
{
  "total_requests": 10000,
  "total_bytes_sent": 104857600,
  "total_bytes_received": 52428800,
  "requests_per_second": 100.5,
  "bytes_per_second": 1048576.0
}
```

##### 操作统计 (Operation Stats)

```http
GET /api/v1/metrics/operations
```

**响应示例 (Response):**
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

##### 系统状态 (System Status)

```http
GET /api/v1/metrics/status
```

**响应示例 (Response):**
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

##### 重置指标 (Reset Metrics)

```http
POST /api/v1/metrics/reset
```

---

#### 11. 协作功能 (Collaboration Features)

##### 创建分享链接 (Create Share Link)

```http
POST /api/v1/share/create
Content-Type: application/json

{
  "path": "/mount/example/document.txt",
  "permission": "read",
  "expires_in": 86400
}
```

**响应示例 (Response):**
```json
{
  "share_id": "share-abc123",
  "share_url": "https://evif.example.com/share/share-abc123",
  "expires_at": 1641086400.0
}
```

##### 列出分享 (List Shares)

```http
GET /api/v1/share/list?path=/mount/example
```

##### 撤销分享 (Revoke Share)

```http
POST /api/v1/share/revoke
Content-Type: application/json

{
  "share_id": "share-abc123"
}
```

##### 设置权限 (Set Permissions)

```http
POST /api/v1/permissions/set
Content-Type: application/json

{
  "path": "/mount/example",
  "user": "user1",
  "permissions": ["read", "write"]
}
```

##### 获取权限 (Get Permissions)

```http
GET /api/v1/permissions/get?path=/mount/example&user=user1
```

##### 列出评论 (List Comments)

```http
GET /api/v1/comments?path=/mount/example/document.txt
```

##### 添加评论 (Add Comment)

```http
POST /api/v1/comments
Content-Type: application/json

{
  "path": "/mount/example/document.txt",
  "content": "Great work!",
  "user": "user1"
}
```

##### 获取活动日志 (Get Activity Log)

```http
GET /api/v1/activities?path=/mount/example&limit=50
```

**响应示例 (Response):**
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

### REST API 错误码 (REST API Error Codes)

| HTTP 状态码 | 含义 | 示例场景 |
|-----------|------|---------|
| 200 OK | 成功 | 请求成功完成 |
| 400 Bad Request | 错误请求 | 无效的 JSON 参数 |
| 401 Unauthorized | 未授权 | 缺少或无效的 API Key |
| 403 Forbidden | 禁止访问 | 权限不足 |
| 404 Not Found | 未找到 | 文件或目录不存在 |
| 409 Conflict | 冲突 | 文件已存在、目录非空 |
| 500 Internal Server Error | 内部错误 | 服务器内部错误 |
| 502 Bad Gateway | 网关错误 | 上游服务错误 |
| 503 Service Unavailable | 不可用 | 服务过载或维护中 |
| 504 Gateway Timeout | 超时 | 上游服务超时 |

**错误响应格式 (Error Response Format):**
```json
{
  "error": "Not Found",
  "message": "File not found: /mount/example/test.txt"
}
```

---

## gRPC API (gRPC API)

### 概述 (Overview)

gRPC API 基于 HTTP/2 和 Protocol Buffers，提供高性能的流式 RPC 接口。

**特性 (Features):**
- ✅ **强类型**: Protocol Buffers 提供类型安全
- ✅ **流式传输**: 支持双向流、服务端流、客户端流
- ✅ **高性能**: HTTP/2 多路复用、二进制编码
- ✅ **代码生成**: 自动生成多语言客户端

### Proto 定义 (Proto Definition)

完整的 proto 定义位于 `crates/evif-grpc/proto/evif.proto`:

```protobuf
syntax = "proto3";

package evif;

service EvifService {
  // 节点操作
  rpc GetNode(GetNodeRequest) returns (GetNodeResponse);
  rpc PutNode(PutNodeRequest) returns (PutNodeResponse);
  rpc DeleteNode(DeleteNodeRequest) returns (DeleteNodeResponse);

  // 批量操作
  rpc BatchGetNodes(BatchGetNodesRequest) returns (stream NodeResponse);
  rpc BatchPutNodes(stream PutNodeRequest) returns (BatchPutNodesResponse);

  // 查询操作
  rpc Query(QueryRequest) returns (stream NodeResponse);

  // 文件操作
  rpc ReadFile(ReadFileRequest) returns (stream DataChunk);
  rpc WriteFile(stream DataChunk) returns (WriteFileResponse);

  // 统计和管理
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

### Rust 客户端示例 (Rust Client Example)

```rust
use evif_grpc::{EvifClient, ClientConfig};
use tonic::transport::Channel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 连接到 gRPC 服务器
    let channel = Channel::from_static("http://localhost:50051").connect().await?;

    let mut client = EvifClient::new(channel);

    // 调用 GetNode RPC
    let request = GetNodeRequest {
        id: "node-123".to_string(),
    };

    let response = client.get_node(request).await?;
    println!("Node: {:?}", response.into_inner().node);

    // 流式读取文件
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

### Python 客户端示例 (Python Client Example)

```python
import grpc
from evif_pb2 import GetNodeRequest, ReadFileRequest
from evif_pb2_grpc import EvifServiceStub

# 连接到 gRPC 服务器
channel = grpc.insecure_channel('localhost:50051')
client = EvifServiceStub(channel)

# 调用 GetNode RPC
request = GetNodeRequest(id="node-123")
response = client.GetNode(request)
print(f"Node: {response.node}")

# 流式读取文件
read_request = ReadFileRequest(
    path="/mount/example/test.txt",
    offset=0,
    size=1024
)

for chunk in client.ReadFile(read_request):
    print(f"Received chunk: {len(chunk.data)} bytes")
```

### Go 客户端示例 (Go Client Example)

```go
package main

import (
    "context"
    "fmt"
    "log"

    pb "path/to/proto/evif"
    "google.golang.org/grpc"
)

func main() {
    // 连接到 gRPC 服务器
    conn, err := grpc.Dial("localhost:50051", grpc.WithInsecure())
    if err != nil {
        log.Fatalf("Failed to connect: %v", err)
    }
    defer conn.Close()

    client := pb.NewEvifServiceClient(conn)

    // 调用 GetNode RPC
    ctx := context.Background()
    req := &pb.GetNodeRequest{
        Id: "node-123",
    }

    resp, err := client.GetNode(ctx, req)
    if err != nil {
        log.Fatalf("GetNode failed: %v", err)
    }

    fmt.Printf("Node: %v\n", resp.Node)

    // 流式读取文件
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

### gRPC 操作详解 (gRPC Operations Detail)

#### 1. 节点操作 (Node Operations)

##### GetNode - 获取节点

```protobuf
rpc GetNode(GetNodeRequest) returns (GetNodeResponse);
```

**请求 (Request):**
```protobuf
message GetNodeRequest {
  string id = 1;
}
```

**响应 (Response):**
```protobuf
message GetNodeResponse {
  Node node = 1;
}
```

##### PutNode - 创建/更新节点

```protobuf
rpc PutNode(PutNodeRequest) returns (PutNodeResponse);
```

**请求 (Request):**
```protobuf
message PutNodeRequest {
  Node node = 1;
}
```

**响应 (Response):**
```protobuf
message PutNodeResponse {
  string id = 1;
}
```

##### DeleteNode - 删除节点

```protobuf
rpc DeleteNode(DeleteNodeRequest) returns (DeleteNodeResponse);
```

---

#### 2. 批量操作 (Batch Operations)

##### BatchGetNodes - 批量获取节点 (服务端流)

```protobuf
rpc BatchGetNodes(BatchGetNodesRequest) returns (stream NodeResponse);
```

**请求 (Request):**
```protobuf
message BatchGetNodesRequest {
  repeated string ids = 1;
}
```

**响应 (Response):**
```protobuf
message NodeResponse {
  Node node = 1;
}
```

**流式响应，每个节点一个消息。**

##### BatchPutNodes - 批量创建/更新节点 (客户端流)

```protobuf
rpc BatchPutNodes(stream PutNodeRequest) returns (BatchPutNodesResponse);
```

**响应 (Response):**
```protobuf
message BatchPutNodesResponse {
  repeated string ids = 1;
  int32 count = 2;
}
```

**客户端流式发送节点，服务端返回所有创建的 ID。**

---

#### 3. 查询操作 (Query Operations)

##### Query - 查询节点 (服务端流)

```protobuf
rpc Query(QueryRequest) returns (stream NodeResponse);
```

**请求 (Request):**
```protobuf
message QueryRequest {
  string query = 1;
  uint32 limit = 2;
}
```

**示例 (Example):**
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

#### 4. 文件操作 (File Operations)

##### ReadFile - 读取文件 (服务端流)

```protobuf
rpc ReadFile(ReadFileRequest) returns (stream DataChunk);
```

**请求 (Request):**
```protobuf
message ReadFileRequest {
  string path = 1;
  uint64 offset = 2;
  uint64 size = 3;
}
```

**响应 (Response):**
```protobuf
message DataChunk {
  bytes data = 1;
  uint64 offset = 2;
  bool eof = 3;
}
```

**流式响应，大文件分块传输。**

##### WriteFile - 写入文件 (客户端流)

```protobuf
rpc WriteFile(stream DataChunk) returns (WriteFileResponse);
```

**响应 (Response):**
```protobuf
message WriteFileResponse {
  uint64 bytes_written = 1;
  string path = 2;
}
```

**客户端流式发送数据块，适合大文件上传。**

**示例 (Example):**
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

// 发送 EOF 标记
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

#### 5. 统计和管理 (Statistics and Management)

##### Stats - 获取统计信息

```protobuf
rpc Stats(StatsRequest) returns (StatsResponse);
```

**请求 (Request):**
```protobuf
message StatsRequest {
  bool detailed = 1;
}
```

**响应 (Response):**
```protobuf
message StatsResponse {
  uint64 total_nodes = 1;
  uint64 total_edges = 2;
  uint64 uptime_secs = 3;
  string status = 4;
}
```

##### Health - 健康检查

```protobuf
rpc Health(HealthRequest) returns (HealthResponse);
```

**请求 (Request):**
```protobuf
message HealthRequest {}
```

**响应 (Response):**
```protobuf
message HealthResponse {
  string status = 1;
  string version = 2;
}
```

---

### gRPC 错误处理 (gRPC Error Handling)

gRPC 使用标准 gRPC 状态码：

| 状态码 | 名称 | 描述 |
|-------|------|------|
| 0 | OK | 成功 |
| 1 | CANCELLED | 操作被取消 |
| 3 | INVALID_ARGUMENT | 无效参数 |
| 5 | NOT_FOUND | 资源未找到 |
| 6 | ALREADY_EXISTS | 资源已存在 |
| 7 | PERMISSION_DENIED | 权限不足 |
| 8 | RESOURCE_EXHAUSTED | 资源耗尽 |
| 9 | FAILED_PRECONDITION | 前置条件失败 |
| 10 | ABORTED | 操作中止 |
| 11 | OUT_OF_RANGE | 超出范围 |
| 12 | UNIMPLEMENTED | 未实现 |
| 13 | INTERNAL | 内部错误 |
| 14 | UNAVAILABLE | 服务不可用 |
| 15 | DATA_LOSS | 数据丢失 |

**错误映射 (Error Mapping):**
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

## Python 客户端 SDK (Python Client SDK)

### 概述 (Overview)

Python SDK 提供高级异步接口，简化 EVIF 的使用。

**特性 (Features):**
- ✅ **异步 I/O**: 基于 asyncio 和 httpx
- ✅ **类型提示**: 完整的类型注解
- ✅ **错误处理**: 结构化异常和重试机制
- ✅ **上下文管理**: 自动连接管理
- ✅ **高级抽象**: 文件句柄、挂载管理等

### 安装 (Installation)

```bash
# 从 PyPI 安装 (发布后)
pip install evif

# 从本地开发版本安装
cd crates/evif-python
pip install -e .
```

### 快速开始 (Quick Start)

```python
import asyncio
from evif import EvifClient

async def main():
    # 创建客户端
    async with EvifClient("http://localhost:8080") as client:
        # 健康检查
        health = await client.health()
        print(f"Status: {health.status}, Version: {health.version}")

        # 读取文件
        content = await client.read_file("/mount/memory/test.txt")
        print(f"Content: {content}")

        # 写入文件
        await client.write_file("/mount/memory/hello.txt", "Hello, EVIF!")

        # 列出目录
        entries = await client.list_directory("/mount/memory")
        for entry in entries:
            print(f"  {entry.name}: {'dir' if entry.is_dir else 'file'}")

asyncio.run(main())
```

### 核心 API (Core API)

#### 1. 客户端配置 (Client Configuration)

```python
from evif import EvifClient

# 基础配置
client = EvifClient(
    base_url="http://localhost:8080",
    timeout=30.0,
    max_retries=3,
    api_key="your-api-key"  # 可选
)

# 使用上下文管理器
async with client:
    # 操作...
    pass
```

#### 2. 文件操作 (File Operations)

##### 读取文件 (Read File)

```python
# 读取整个文件
content = await client.read_file("/mount/memory/test.txt")
print(content)

# 使用文件句柄读取大文件
async with client.open_file("/mount/memory/large.txt", "r") as f:
    while True:
        chunk = await f.read(4096)
        if not chunk:
            break
        print(chunk, end="")
```

##### 写入文件 (Write File)

```python
# 写入字符串
await client.write_file("/mount/memory/test.txt", "Hello, EVIF!")

# 使用文件句柄写入大文件
async with client.open_file("/mount/memory/large.txt", "w") as f:
    for i in range(1000):
        await f.write(f"Line {i}\n")
```

##### 创建文件 (Create File)

```python
await client.create_file("/mount/memory/newfile.txt", mode=0o644)
```

##### 删除文件 (Delete File)

```python
await client.delete_file("/mount/memory/test.txt")
```

---

#### 3. 目录操作 (Directory Operations)

##### 列出目录 (List Directory)

```python
entries = await client.list_directory("/mount/memory")

for entry in entries:
    if entry.is_dir:
        print(f"DIR  {entry.name}/")
    else:
        print(f"FILE {entry.name} ({entry.size} bytes)")
```

##### 创建目录 (Create Directory)

```python
await client.create_directory("/mount/memory/subdir", mode=0o755)
```

##### 删除目录 (Delete Directory)

```python
await client.delete_directory("/mount/memory/subdir", recursive=False)
```

---

#### 4. 元数据操作 (Metadata Operations)

##### 获取文件信息 (Get File Info)

```python
info = await client.stat("/mount/memory/test.txt")
print(f"Size: {info.size} bytes")
print(f"Mode: {oct(info.mode)}")
print(f"Modified: {info.mtime}")
```

##### 计算文件哈希 (Compute File Hash)

```python
hash_value = await client.digest("/mount/memory/test.txt", algorithm="sha256")
print(f"SHA256: {hash_value}")
```

##### 更新时间戳 (Touch)

```python
import time
await client.touch("/mount/memory/test.txt", mtime=time.time())
```

---

#### 5. 高级操作 (Advanced Operations)

##### 正则搜索 (Grep)

```python
matches = await client.grep(
    path="/mount/memory",
    pattern=r"TODO|FIXME",
    recursive=True
)

for match in matches:
    print(f"{match.path}:{match.line}: {match.content}")
```

##### 重命名/移动 (Rename/Move)

```python
await client.rename(
    from_path="/mount/memory/old.txt",
    to_path="/mount/memory/new.txt"
)
```

---

#### 6. 挂载管理 (Mount Management)

##### 列出挂载点 (List Mounts)

```python
mounts = await client.list_mounts()

for mount in mounts:
    print(f"{mount.path}: {mount.plugin}")
    for key, value in mount.options.items():
        print(f"  {key}: {value}")
```

##### 挂载插件 (Mount Plugin)

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

##### 卸载插件 (Unmount Plugin)

```python
await client.unmount("/mount/s3")
```

---

#### 7. 插件管理 (Plugin Management)

##### 列出插件 (List Plugins)

```python
plugins = await client.list_plugins()

for plugin in plugins:
    status = "enabled" if plugin.enabled else "disabled"
    print(f"{plugin.name} v{plugin.version} [{status}]")
    print(f"  {plugin.description}")
```

##### 加载外部插件 (Load External Plugin)

```python
await client.load_plugin(
    path="/path/to/plugin.so",
    config={}
)
```

---

#### 8. 文件句柄 (File Handles)

Python SDK 提供高级文件句柄 API：

```python
from evif import FileHandle

# 打开文件句柄
handle: FileHandle = await client.open_handle(
    path="/mount/memory/test.txt",
    flags="O_RDWR"
)

try:
    # Seek 操作
    await handle.seek(100, whence="SEEK_SET")

    # 读取数据
    data = await handle.read(1024)

    # 写入数据
    await handle.write(b"Hello, World!")

    # Sync 到磁盘
    await handle.sync()

    # 获取句柄信息
    info = await client.get_handle_info(handle.id)
    print(f"Offset: {info.offset}")

finally:
    # 关闭句柄
    await client.close_handle(handle.id)
```

**上下文管理器方式 (Context Manager):**

```python
async with await client.open_handle("/mount/memory/test.txt", "O_RDWR") as f:
    await f.seek(0)
    data = await f.read(1024)
    await f.write(b"New content")
    # 自动关闭句柄
```

---

### 错误处理 (Error Handling)

Python SDK 提供结构化异常层次：

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
    print("API Key 无效或过期")
except FileNotFoundError:
    print("文件不存在")
except PermissionError:
    print("权限不足")
except TimeoutError:
    print("请求超时，请稍后重试")
except ValidationError as e:
    print(f"验证错误: {e}")
except EvifError as e:
    print(f"EVIF 错误: {e}")
```

### 重试机制 (Retry Mechanism)

SDK 使用 `tenacity` 库自动重试失败请求：

```python
client = EvifClient(
    base_url="http://localhost:8080",
    max_retries=3,  # 最大重试次数
    timeout=30.0    # 请求超时
)

# 内部重试策略：
# - 指数退避: 1s, 2s, 4s, ...
# - 只重试幂等操作 (GET、HEAD)
# - 不重试客户端错误 (4xx)
```

### 类型定义 (Type Definitions)

所有模型使用 Pydantic 定义：

```python
from evif.models import FileInfo, MountInfo, HealthStatus, HandleInfo

# 文件信息
info = FileInfo(
    name="test.txt",
    path="/mount/memory/test.txt",
    size=1024,
    mode=33188,
    mtime=1640995200.0,
    is_dir=False,
    is_file=True
)

# 挂载信息
mount = MountInfo(
    path="/mount/s3",
    plugin="s3",
    options={"bucket": "my-bucket", "region": "us-east-1"}
)

# 健康状态
health = HealthStatus(
    status="ok",
    version="0.1.0",
    uptime=3600.5,
    plugins_count=5
)

# 句柄信息
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

### 概述 (Overview)

WebSocket API 提供实时双向通信，用于事件通知和实时更新。

**特性 (Features):**
- ✅ **实时通知**: 文件变更、挂载状态变化等
- ✅ **双向通信**: 客户端可发送命令
- ✅ **低延迟**: 持久连接，避免 HTTP 开销
- ✅ **自动重连**: 连接断开时自动重连

### 连接 (Connection)

```javascript
// JavaScript 客户端示例
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

**Python 客户端示例:**

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

### 消息格式 (Message Format)

**通用消息结构 (Generic Message Structure):**

```json
{
  "type": "message_type",
  "timestamp": 1640995200.0,
  "data": { }
}
```

### 事件类型 (Event Types)

#### 1. 文件变更事件 (File Change Events)

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

**操作类型 (Operation Types):**
- `created`: 文件创建
- `modified`: 文件修改
- `deleted`: 文件删除
- `renamed`: 文件重命名

#### 2. 挂载事件 (Mount Events)

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

#### 3. 错误事件 (Error Events)

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

#### 4. 心跳事件 (Heartbeat Events)

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

### 客户端命令 (Client Commands)

客户端可以通过 WebSocket 发送命令：

```javascript
// 订阅特定路径的事件
ws.send(JSON.stringify({
  type: 'subscribe',
  data: {
    path: '/mount/memory'
  }
}));

// 取消订阅
ws.send(JSON.stringify({
  type: 'unsubscribe',
  data: {
    path: '/mount/memory'
  }
}));

// Ping (保持连接)
ws.send(JSON.stringify({
  type: 'ping'
}));
```

---

## 错误处理 (Error Handling)

### REST API 错误 (REST API Errors)

**错误响应格式 (Error Response Format):**

```json
{
  "error": "Not Found",
  "message": "File not found: /mount/example/test.txt"
}
```

### gRPC 错误 (gRPC Errors)

gRPC 使用标准 gRPC 状态码，详见 [gRPC 错误处理](#grpc-错误处理-grpc-error-handling) 章节。

### Python SDK 错误 (Python SDK Errors)

**异常层次 (Exception Hierarchy):**

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

**错误处理示例 (Error Handling Example):**

```python
from evif.exceptions import *

try:
    content = await client.read_file("/mount/memory/test.txt")
except AuthenticationError:
    # API Key 无效
    pass
except FileNotFoundError:
    # 文件不存在
    pass
except PermissionError:
    # 权限不足
    pass
except TimeoutError:
    # 请求超时
    pass
except ValidationError as e:
    # 参数验证失败
    print(f"Validation error: {e}")
except EvifError as e:
    # 其他 EVIF 错误
    print(f"EVIF error: {e}")
```

---

## 认证与安全 (Authentication and Security)

### API Key 认证 (API Key Authentication)

**设置 API Key:**

```bash
# 设置环境变量
export EVIF_API_KEY="your-api-key-here"

# 或在请求头中设置
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

### 生成 API Key (Generate API Key)

```bash
# 使用 CLI 生成 API Key
evif-cli api-key generate

# 或使用 REST API
curl -X POST http://localhost:8080/api/v1/api-keys \
     -H "Content-Type: application/json" \
     -d '{"description": "My API Key"}'
```

### 权限管理 (Permission Management)

**设置文件权限 (Set File Permissions):**

```http
POST /api/v1/permissions/set
Content-Type: application/json

{
  "path": "/mount/example",
  "user": "user1",
  "permissions": ["read", "write"]
}
```

**获取文件权限 (Get File Permissions):**

```http
GET /api/v1/permissions/get?path=/mount/example&user=user1
```

### 安全最佳实践 (Security Best Practices)

1. **使用 HTTPS**: 生产环境务必使用 HTTPS
2. **定期轮换 API Key**: 每 90 天更换一次
3. **最小权限原则**: 只授予必要的权限
4. **审计日志**: 记录所有 API 访问
5. **限制 IP 白名单**: 只允许可信 IP 访问

---

## 性能优化 (Performance Optimization)

### 批量操作 (Batch Operations)

使用批量 API 减少网络往返：

```python
# ❌ 多次单独请求
for file in files:
    await client.delete_file(file)

# ✅ 单次批量请求
await client.batch_delete(files)
```

### 并发请求 (Concurrent Requests)

使用并发提高吞吐量：

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

### 客户端缓存 (Client Cache)

启用客户端缓存减少请求：

```python
client = EvifClient(
    base_url="http://localhost:8080",
    enable_cache=True,
    cache_ttl=300  # 5 分钟
)
```

### 连接池 (Connection Pool)

配置连接池大小：

```python
client = EvifClient(
    base_url="http://localhost:8080",
    max_connections=100,
    max_keepalive_connections=20
)
```

### gRPC 流式传输 (gRPC Streaming)

使用流式传输处理大文件：

```rust
// 服务端流: 读取大文件
let mut stream = client.read_file(request).await?.into_inner();
while let Some(chunk) = stream.message().await? {
    // 处理每个数据块
}

// 客户端流: 写入大文件
let mut stream = client.write_file().await?;
for chunk in chunks {
    stream.send(chunk).await?;
}
let response = stream.close().await?;
```

---

## API 版本管理 (API Versioning)

### 版本策略 (Versioning Strategy)

EVIF 使用 URL 路径版本控制：

```
/api/v1/...  # 当前稳定版本
/api/v2/...  # 未来版本 (向后不兼容)
```

### 版本兼容性 (Version Compatibility)

**主版本 (Major Version):**
- 不兼容的 API 变更
- 主版本号变更 (v1 → v2)

**次版本 (Minor Version):**
- 向后兼容的新功能
- 次版本号变更 (v1.0 → v1.1)

**补丁版本 (Patch Version):**
- 向后兼容的问题修复
- 补丁号变更 (v1.0.0 → v1.0.1)

### 弃用策略 (Deprecation Policy)

1. **标记为弃用**: API 端点添加 `Deprecated` 头
2. **文档更新**: 在文档中标注弃用日期
3. **宽限期**: 至少 6 个月后才移除
4. **迁移指南**: 提供迁移到新 API 的指南

**弃用示例 (Deprecation Example):**

```http
GET /api/v1/old-endpoint
Deprecated: true
Sunset: 2025-12-31
Link: </api/v2/new-endpoint>; rel="successor-version"
```

---

## 示例与最佳实践 (Examples and Best Practices)

### 示例 1: 文件上传 (Example 1: File Upload)

**REST API:**

```bash
# 上传小文件 (< 1MB)
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
# 上传小文件
await client.write_file("/mount/s3/upload.txt", "Hello, S3!")

# 上传大文件 (分块)
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
# 流式上传大文件
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

            # 发送 EOF
            await stream.send(DataChunk(
                data=b"",
                offset=offset,
                eof=True
            ))

            response = await stream.close()
            print(f"Uploaded {response.bytes_written} bytes")
```

---

### 示例 2: 批量操作 (Example 2: Batch Operations)

**批量复制文件:**

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

### 示例 3: 实时监控 (Example 3: Real-time Monitoring)

**使用 WebSocket 监控文件变化:**

```python
import asyncio
import websockets
import json

async def monitor_files(path):
    uri = "ws://localhost:8080/ws"

    async with websockets.connect(uri) as ws:
        # 订阅路径
        await ws.send(json.dumps({
            "type": "subscribe",
            "data": {"path": path}
        }))

        # 监听事件
        async for message in ws:
            data = json.loads(message)

            if data["type"] == "file_changed":
                op = data["data"]["operation"]
                file_path = data["data"]["path"]
                print(f"[{op}] {file_path}")

asyncio.run(monitor_files("/mount/memory"))
```

---

### 最佳实践 (Best Practices)

#### 1. 错误处理 (Error Handling)

```python
# ✅ 好的做法: 捕获特定异常
try:
    content = await client.read_file(path)
except FileNotFoundError:
    logger.error(f"File not found: {path}")
except EvifError as e:
    logger.error(f"EVIF error: {e}")

# ❌ 不好的做法: 捕获所有异常
try:
    content = await client.read_file(path)
except Exception:
    pass
```

#### 2. 资源管理 (Resource Management)

```python
# ✅ 好的做法: 使用上下文管理器
async with await client.open_handle(path, "O_RDWR") as f:
    data = await f.read(1024)
    # 自动关闭句柄

# ❌ 不好的做法: 手动管理资源
handle = await client.open_handle(path, "O_RDWR")
data = await handle.read(1024)
# 忘记关闭句柄
```

#### 3. 超时控制 (Timeout Control)

```python
# ✅ 好的做法: 设置超时
try:
    content = await asyncio.wait_for(
        client.read_file(path),
        timeout=30.0
    )
except asyncio.TimeoutError:
    logger.error(f"Timeout reading {path}")

# ❌ 不好的做法: 无超时限制
content = await client.read_file(path)  # 可能永久阻塞
```

#### 4. 日志记录 (Logging)

```python
# ✅ 好的做法: 记录关键操作
logger.info(f"Reading file: {path}")
content = await client.read_file(path)
logger.info(f"Read {len(content)} bytes from {path}")

# ❌ 不好的做法: 无日志
content = await client.read_file(path)
```

---

## 总结 (Summary)

EVIF 提供完整的 API 生态系统：

- **REST API**: 简单易用的 HTTP/JSON 接口
- **gRPC API**: 高性能的二进制 RPC 接口
- **Python SDK**: 高级异步 Python 客户端
- **WebSocket API**: 实时双向通信

选择合适的 API 接口：
- **Web 应用**: 使用 REST API
- **微服务**: 使用 gRPC API
- **数据科学**: 使用 Python SDK
- **实时监控**: 使用 WebSocket API

**相关章节 (Related Chapters):**
- [第三章：架构设计 (Architecture)](chapter-3-architecture.md)
- [第四章：虚拟文件系统 (Virtual Filesystem)](chapter-4-virtual-filesystem.md)
- [第五章：插件开发 (Plugin Development)](chapter-5-plugin-development.md)
- [第六章：FUSE 集成 (FUSE Integration)](chapter-6-fuse.md)
- [第八章：认证与安全 (Authentication and Security)](chapter-8-authentication-security.md)
