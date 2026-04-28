# EVIF REST API 参考

106 个端点，14 个类别，基于 Axum 框架构建。

## 1. 概览

**基础 URL**: `http://localhost:8081/api/v1`

## 2. 认证

### 2.1 认证模式

| 模式 | Header | 描述 |
|------|--------|------|
| Disabled | 无 | 无认证 (开发模式) |
| API Key | `X-API-Key: <key>` | 静态密钥 |
| Bearer | `Authorization: Bearer <token>` | JWT 令牌 |

### 2.2 Capability 认证

```json
{
  "capabilities": {
    "paths": ["/mem/**", "/context/L0/**"],
    "operations": ["read", "write"],
    "max_size_mb": 100
  }
}
```

## 3. 通用模式

### 3.1 响应格式

**成功**：
```json
{
  "data": {},
  "meta": {
    "request_id": "uuid",
    "duration_ms": 42
  }
}
```

**错误**：
```json
{
  "error": {
    "code": "PATH_NOT_FOUND",
    "message": "Path not found: /foo",
    "details": {}
  }
}
```

### 3.2 分页

```
GET /api/v1/files?offset=0&limit=100
```

### 3.3 Content Types

- `application/json` - JSON (默认)
- `application/octet-stream` - 二进制
- `text/plain` - 文本

## 4. 文件操作

### 4.1 读取文件

```
GET /api/v1/files?path={path}
```

**参数**：
| 名称 | 位置 | 类型 | 必需 | 描述 |
|------|------|------|------|------|
| path | query | string | 是 | 文件路径 |

**响应**：
```json
{
  "content": "file content",
  "data": "base64 encoded",
  "size": 1234,
  "mime_type": "text/plain"
}
```

### 4.2 写入文件

```
PUT /api/v1/files?path={path}
```

**Body**：
```json
{
  "data": "base64 encoded content",
  "content": "raw content (alternative)",
  "overwrite": true
}
```

**响应**：
```json
{
  "bytes_written": 1234,
  "path": "/mem/test.txt"
}
```

### 4.3 创建文件 (原子)

```
POST /api/v1/files?path={path}
```

**Body**：
```json
{
  "content": "initial content",
  "mode": "0644"
}
```

### 4.4 删除文件

```
DELETE /api/v1/files?path={path}
```

**响应**：
```json
{
  "deleted": true,
  "path": "/mem/test.txt"
}
```

### 4.5 列出目录

```
GET /api/v1/directories?path={path}
```

**响应**：
```json
{
  "path": "/mem",
  "files": [
    {
      "name": "test.txt",
      "path": "/mem/test.txt",
      "size": 1234,
      "is_dir": false,
      "modified": "2026-04-27T10:00:00Z"
    }
  ],
  "total": 1
}
```

### 4.6 创建目录

```
POST /api/v1/directories
```

**Body**：
```json
{
  "path": "/mem/test",
  "mode": "0755",
  "parents": true
}
```

**响应**：
```json
{
  "created": true,
  "path": "/mem/test"
}
```

### 4.7 删除目录

```
DELETE /api/v1/directories?path={path}
```

**查询参数**：
| 名称 | 类型 | 描述 |
|------|------|------|
| recursive | boolean | 递归删除内容 |

### 4.8 Stat (元数据)

```
GET /api/v1/stat?path={path}
```

**响应**：
```json
{
  "path": "/mem/test.txt",
  "size": 1234,
  "is_dir": false,
  "mode": 33204,
  "created": "2026-04-27T10:00:00Z",
  "modified": "2026-04-27T10:00:00Z"
}
```

### 4.9 重命名/移动

```
POST /api/v1/rename
```

**Body**：
```json
{
  "old_path": "/mem/old.txt",
  "new_path": "/mem/new.txt",
  "overwrite": false
}
```

### 4.10 复制

```
POST /api/v1/copy
```

**Body**：
```json
{
  "src": "/mem/source.txt",
  "dst": "/mem/dest.txt",
  "overwrite": false,
  "recursive": false
}
```

### 4.11 Grep (搜索)

```
POST /api/v1/grep
```

**Body**：
```json
{
  "path": "/mem",
  "pattern": "hello",
  "regex": false,
  "recursive": true,
  "case_sensitive": true,
  "max_results": 100
}
```

**响应**：
```json
{
  "pattern": "hello",
  "matches": [
    {
      "path": "/mem/test.txt",
      "line": 5,
      "content": "hello world"
    }
  ]
}
```

### 4.12 Digest (哈希)

```
POST /api/v1/digest
```

**Body**：
```json
{
  "path": "/mem/test.txt",
  "algorithm": "sha256"
}
```

**响应**：
```json
{
  "path": "/mem/test.txt",
  "algorithm": "sha256",
  "hash": "abc123..."
}
```

## 5. Handle 操作

### 5.1 打开 Handle

```
POST /api/v1/handles/open
```

**Body**：
```json
{
  "path": "/mem/test.txt",
  "flags": 1,
  "mode": 0644,
  "lease_seconds": 60
}
```

**Flags**：
| 值 | 名称 | 描述 |
|----|------|------|
| 0 | READ_ONLY | 只读 |
| 1 | READ_WRITE | 读写 |
| 2 | WRITE_ONLY | 只写 |
| 4 | APPEND | 追加模式 |
| 8 | CREATE | 不存在则创建 |
| 16 | TRUNCATE | 打开时截断 |

### 5.2 读取 Handle

```
POST /api/v1/handles/read
```

**Body**：
```json
{
  "handle": 12345,
  "offset": 0,
  "size": 4096
}
```

### 5.3 写入 Handle

```
POST /api/v1/handles/write
```

**Body**：
```json
{
  "handle": 12345,
  "offset": 0,
  "data": "base64..."
}
```

### 5.4 关闭 Handle

```
POST /api/v1/handles/close
```

**Body**：
```json
{
  "handle": 12345
}
```

### 5.5 列出 Handles

```
GET /api/v1/handles
```

**响应**：
```json
{
  "handles": [
    {
      "id": 12345,
      "path": "/mem/test.txt",
      "flags": 1,
      "offset": 100,
      "lease_expires": "2026-04-27T11:00:00Z"
    }
  ]
}
```

## 6. 挂载管理

### 6.1 列出挂载

```
GET /api/v1/mounts
```

**响应**：
```json
{
  "mounts": [
    {
      "path": "/mem",
      "plugin": "memfs",
      "instance_name": "mem",
      "options": {}
    }
  ]
}
```

### 6.2 挂载插件

```
POST /api/v1/mounts
```

**Body**：
```json
{
  "path": "/s3",
  "plugin": "s3fs",
  "config": {
    "bucket": "my-bucket",
    "region": "us-east-1"
  }
}
```

### 6.3 卸载

```
POST /api/v1/unmounts
```

**Body**：
```json
{
  "path": "/s3"
}
```

## 7. 内存操作

### 7.1 存储记忆

```
POST /api/v1/memories
```

**Body**：
```json
{
  "content": "Important fact about the project",
  "modality": "knowledge",
  "metadata": {
    "source": "user",
    "tags": ["important"]
  }
}
```

**响应**：
```json
{
  "memory_id": "uuid",
  "extracted_items": [
    {
      "id": "uuid",
      "type": "knowledge",
      "summary": "Important fact..."
    }
  ]
}
```

### 7.2 列出记忆

```
GET /api/v1/memories?modality={modality}&limit={limit}&offset={offset}
```

**响应**：
```json
{
  "memories": [
    {
      "id": "uuid",
      "content": "Important fact...",
      "type": "knowledge",
      "created": "2026-04-27T10:00:00Z"
    }
  ],
  "total": 42
}
```

### 7.3 搜索记忆

```
POST /api/v1/memories/search
```

**Body**：
```json
{
  "query": "authentication setup",
  "limit": 10,
  "modality": null
}
```

**响应**：
```json
{
  "results": [
    {
      "id": "uuid",
      "content": "The auth module uses JWT...",
      "score": 0.95,
      "type": "knowledge"
    }
  ],
  "total": 1
}
```

### 7.4 删除记忆

```
DELETE /api/v1/memories/{id}
```

## 8. 上下文操作

### 8.1 获取上下文

```
GET /api/v1/context/{layer}
```

**Layers**: `L0`, `L1`, `L2`

### 8.2 设置上下文

```
PUT /api/v1/context/{layer}
```

**Body**：
```json
{
  "content": "Current task description",
  "append": false
}
```

### 8.3 搜索上下文

```
POST /api/v1/context/search
```

**Body**：
```json
{
  "query": "authentication decisions",
  "layers": ["L1", "L2"]
}
```

## 9. 技能操作

### 9.1 列出技能

```
GET /api/v1/skills
```

### 9.2 获取技能

```
GET /api/v1/skills/{name}
```

### 9.3 运行技能

```
POST /api/v1/skills/{name}/run
```

**Body**：
```json
{
  "input": "Review auth module for security",
  "async": false
}
```

## 10. 管道操作

### 10.1 创建管道

```
POST /api/v1/pipes
```

**Body**：
```json
{
  "name": "review-task",
  "timeout_seconds": 3600
}
```

### 10.2 管道状态

```
GET /api/v1/pipes/{name}/status
```

### 10.3 发送数据

```
PUT /api/v1/pipes/{name}/input
```

### 10.4 接收数据

```
GET /api/v1/pipes/{name}/output
```

### 10.5 认领管道

```
POST /api/v1/pipes/{name}/claim
```

### 10.6 完成管道

```
POST /api/v1/pipes/{name}/complete
```

## 11. 系统操作

### 11.1 健康检查

```
GET /api/v1/health
```

**响应**：
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime": 3600
}
```

### 11.2 就绪检查

```
GET /api/v1/readiness
```

### 11.3 系统状态

```
GET /api/v1/system/status
```

### 11.4 指标

```
GET /api/v1/metrics
```

**响应**: Prometheus text format

## 12. 租户操作

### 12.1 列出租户

```
GET /api/v1/tenants
```

### 12.2 创建租户

```
POST /api/v1/tenants
```

### 12.3 租户配额

```
GET /api/v1/tenants/{id}/quota
PUT /api/v1/tenants/{id}/quota
```

## 13. 加密操作

### 13.1 启用加密

```
POST /api/v1/encryption/enable
```

### 13.2 禁用加密

```
POST /api/v1/encryption/disable
```

### 13.3 轮转密钥

```
POST /api/v1/encryption/rotate
```

### 13.4 列出版本

```
GET /api/v1/encryption/versions
```

## 14. 协作

### 14.1 创建分享

```
POST /api/v1/shares
```

### 14.2 列出分享

```
GET /api/v1/shares
```

### 14.3 设置权限

```
PUT /api/v1/shares/{id}/permissions
```

### 14.4 添加评论

```
POST /api/v1/comments
```

### 14.5 列出评论

```
GET /api/v1/comments?path={path}
```

## 15. GraphQL

```
POST /api/v1/graphql
```

**Playground**:
```
GET /api/v1/graphql/playground
```

## 16. WebSocket

```
GET /api/v1/ws
```

**事件**：
- `file.created`
- `file.modified`
- `file.deleted`
- `pipe.status_changed`
- `metrics.updated`

## 17. 错误码

| 代码 | HTTP 状态 | 描述 |
|------|-----------|------|
| PATH_NOT_FOUND | 404 | 路径不存在 |
| PATH_EXISTS | 409 | 路径已存在 |
| IS_DIRECTORY | 400 | 无法对目录执行操作 |
| NOT_DIRECTORY | 400 | 路径不是目录 |
| PERMISSION_DENIED | 403 | 操作不允许 |
| CIRCUIT_BREAKER_OPEN | 503 | 服务暂时不可用 |
| INVALID_PATH | 400 | 无效的路径格式 |
| HANDLE_NOT_FOUND | 404 | Handle 不存在 |
| HANDLE_EXPIRED | 410 | Handle 租约过期 |
| QUOTA_EXCEEDED | 507 | 存储配额超出 |

## 18. 速率限制

| 端点 | 限制 |
|------|------|
| `/api/v1/files` (读) | 1000/分钟 |
| `/api/v1/files` (写) | 100/分钟 |
| `/api/v1/memories` | 100/分钟 |
| 其他 | 1000/分钟 |

## 19. 相关文档

- [快速开始](../GETTING_STARTED.md)
- [SDK 集成](04-sdk-integration.md)
- [CLI 参考](../cli-mode.md)