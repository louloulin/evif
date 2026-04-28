# EVIF REST API Reference

## 1. Overview

The REST API provides 106 endpoints across 14 handler modules. Built with Axum framework.

**Base URL**: `http://localhost:8081/api/v1`

## 2. Authentication

### 2.1 Modes

| Mode | Header | Description |
|------|--------|-------------|
| Disabled | None | No auth (dev mode) |
| API Key | `X-API-Key: <key>` | Static key |
| Bearer | `Authorization: Bearer <token>` | JWT token |

### 2.2 Capability-Based Auth

```json
{
  "capabilities": {
    "paths": ["/mem/**", "/context/L0/**"],
    "operations": ["read", "write"],
    "max_size_mb": 100
  }
}
```

## 3. Common Patterns

### 3.1 Response Format

**Success**:
```json
{
  "data": {},
  "meta": {
    "request_id": "uuid",
    "duration_ms": 42
  }
}
```

**Error**:
```json
{
  "error": {
    "code": "PATH_NOT_FOUND",
    "message": "Path not found: /foo",
    "details": {}
  }
}
```

### 3.2 Pagination

```
GET /api/v1/files?offset=0&limit=100
```

### 3.3 Content Types

- `application/json` - JSON (default)
- `application/octet-stream` - Binary
- `text/plain` - Text

## 4. File Operations

### 4.1 Read File

```
GET /api/v1/files?path={path}
```

**Parameters**:
| Name | In | Type | Required | Description |
|------|-----|------|----------|-------------|
| path | query | string | Yes | File path |

**Response**:
```json
{
  "content": "file content",
  "data": "base64 encoded",
  "size": 1234,
  "mime_type": "text/plain"
}
```

### 4.2 Write File

```
PUT /api/v1/files?path={path}
```

**Body**:
```json
{
  "data": "base64 encoded content",
  "content": "raw content (alternative)",
  "overwrite": true
}
```

**Response**:
```json
{
  "bytes_written": 1234,
  "path": "/mem/test.txt"
}
```

### 4.3 Create File (Atomic)

```
POST /api/v1/files?path={path}
```

**Body**:
```json
{
  "content": "initial content",
  "mode": "0644"
}
```

### 4.4 Delete File

```
DELETE /api/v1/files?path={path}
```

**Response**:
```json
{
  "deleted": true,
  "path": "/mem/test.txt"
}
```

### 4.5 List Directory

```
GET /api/v1/directories?path={path}
```

**Response**:
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

### 4.6 Create Directory

```
POST /api/v1/directories
```

**Body**:
```json
{
  "path": "/mem/test",
  "mode": "0755",
  "parents": true
}
```

**Response**:
```json
{
  "created": true,
  "path": "/mem/test"
}
```

### 4.7 Delete Directory

```
DELETE /api/v1/directories?path={path}
```

**Query Parameters**:
| Name | Type | Description |
|------|------|-------------|
| recursive | boolean | Delete contents recursively |

### 4.8 Stat (Metadata)

```
GET /api/v1/stat?path={path}
```

**Response**:
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

### 4.9 Rename/Move

```
POST /api/v1/rename
```

**Body**:
```json
{
  "old_path": "/mem/old.txt",
  "new_path": "/mem/new.txt",
  "overwrite": false
}
```

### 4.10 Copy

```
POST /api/v1/copy
```

**Body**:
```json
{
  "src": "/mem/source.txt",
  "dst": "/mem/dest.txt",
  "overwrite": false,
  "recursive": false
}
```

### 4.11 Grep (Search)

```
POST /api/v1/grep
```

**Body**:
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

**Response**:
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

### 4.12 Digest (Hash)

```
POST /api/v1/digest
```

**Body**:
```json
{
  "path": "/mem/test.txt",
  "algorithm": "sha256"
}
```

**Response**:
```json
{
  "path": "/mem/test.txt",
  "algorithm": "sha256",
  "hash": "abc123..."
}
```

### 4.13 Touch (Update Timestamp)

```
POST /api/v1/touch
```

**Body**:
```json
{
  "path": "/mem/test.txt",
  "modified": null  // Use current time
}
```

## 5. Handle Operations

### 5.1 Open Handle

```
POST /api/v1/handles/open
```

**Body**:
```json
{
  "path": "/mem/test.txt",
  "flags": 1,
  "mode": 0644,
  "lease_seconds": 60
}
```

**Flags**:
| Value | Name | Description |
|-------|------|-------------|
| 0 | READ_ONLY | Read only |
| 1 | READ_WRITE | Read and write |
| 2 | WRITE_ONLY | Write only |
| 4 | APPEND | Append mode |
| 8 | CREATE | Create if not exists |
| 16 | TRUNCATE | Truncate on open |

### 5.2 Read Handle

```
POST /api/v1/handles/read
```

**Body**:
```json
{
  "handle": 12345,
  "offset": 0,
  "size": 4096
}
```

### 5.3 Write Handle

```
POST /api/v1/handles/write
```

**Body**:
```json
{
  "handle": 12345,
  "offset": 0,
  "data": "base64..."
}
```

### 5.4 Seek Handle

```
POST /api/v1/handles/seek
```

**Body**:
```json
{
  "handle": 12345,
  "offset": 100,
  "whence": 0
}
```

**Whence**:
| Value | Name | Description |
|-------|------|-------------|
| 0 | SET | From start |
| 1 | CUR | From current |
| 2 | END | From end |

### 5.5 Close Handle

```
POST /api/v1/handles/close
```

**Body**:
```json
{
  "handle": 12345
}
```

### 5.6 Renew Handle Lease

```
POST /api/v1/handles/renew
```

**Body**:
```json
{
  "handle": 12345,
  "seconds": 60
}
```

### 5.7 List Handles

```
GET /api/v1/handles
```

**Response**:
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

## 6. Mount Management

### 6.1 List Mounts

```
GET /api/v1/mounts
```

**Response**:
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

### 6.2 Mount Plugin

```
POST /api/v1/mounts
```

**Body**:
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

### 6.3 Unmount

```
POST /api/v1/unmounts
```

**Body**:
```json
{
  "path": "/s3"
}
```

## 7. Plugin Management

### 7.1 List Plugins

```
GET /api/v1/plugins
```

**Response**:
```json
{
  "plugins": [
    {
      "name": "memfs",
      "version": "0.1.0",
      "description": "In-memory filesystem",
      "capabilities": ["create", "read", "write", "delete"]
    }
  ]
}
```

### 7.2 Plugin Status

```
GET /api/v1/plugins/{name}/status
```

### 7.3 Reload Plugin

```
POST /api/v1/plugins/{name}/reload
```

### 7.4 Plugin Config

```
GET /api/v1/plugins/{name}/config
PUT /api/v1/plugins/{name}/config
```

## 8. Memory Operations

### 8.1 Store Memory

```
POST /api/v1/memories
```

**Body**:
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

**Response**:
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

### 8.2 List Memories

```
GET /api/v1/memories?modality={modality}&limit={limit}&offset={offset}
```

**Response**:
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

### 8.3 Search Memories

```
POST /api/v1/memories/search
```

**Body**:
```json
{
  "query": "authentication setup",
  "limit": 10,
  "modality": null
}
```

**Response**:
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

### 8.4 Delete Memory

```
DELETE /api/v1/memories/{id}
```

## 9. Context Operations

### 9.1 Get Context

```
GET /api/v1/context/{layer}
```

**Layers**: `L0`, `L1`, `L2`

### 9.2 Set Context

```
PUT /api/v1/context/{layer}
```

**Body**:
```json
{
  "content": "Current task description",
  "append": false
}
```

### 9.3 Search Context

```
POST /api/v1/context/search
```

**Body**:
```json
{
  "query": "authentication decisions",
  "layers": ["L1", "L2"]
}
```

## 10. Skill Operations

### 10.1 List Skills

```
GET /api/v1/skills
```

### 10.2 Get Skill

```
GET /api/v1/skills/{name}
```

### 10.3 Run Skill

```
POST /api/v1/skills/{name}/run
```

**Body**:
```json
{
  "input": "Review auth module for security",
  "async": false
}
```

## 11. Pipe Operations

### 11.1 Create Pipe

```
POST /api/v1/pipes
```

**Body**:
```json
{
  "name": "review-task",
  "timeout_seconds": 3600
}
```

### 11.2 Pipe Status

```
GET /api/v1/pipes/{name}/status
```

### 11.3 Send to Pipe

```
PUT /api/v1/pipes/{name}/input
```

### 11.4 Receive from Pipe

```
GET /api/v1/pipes/{name}/output
```

### 11.5 Claim Pipe

```
POST /api/v1/pipes/{name}/claim
```

### 11.6 Complete Pipe

```
POST /api/v1/pipes/{name}/complete
```

## 12. System Operations

### 12.1 Health Check

```
GET /api/v1/health
```

**Response**:
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime": 3600
}
```

### 12.2 Readiness

```
GET /api/v1/readiness
```

### 12.3 System Status

```
GET /api/v1/system/status
```

### 12.4 Metrics

```
GET /api/v1/metrics
```

**Response**: Prometheus text format

### 12.5 Traffic Stats

```
GET /api/v1/metrics/traffic
```

### 12.6 Reset Metrics

```
POST /api/v1/metrics/reset
```

## 13. Tenant Operations

### 13.1 List Tenants

```
GET /api/v1/tenants
```

### 13.2 Create Tenant

```
POST /api/v1/tenants
```

### 13.3 Tenant Quota

```
GET /api/v1/tenants/{id}/quota
PUT /api/v1/tenants/{id}/quota
```

## 14. Encryption Operations

### 14.1 Enable Encryption

```
POST /api/v1/encryption/enable
```

### 14.2 Disable Encryption

```
POST /api/v1/encryption/disable
```

### 14.3 Rotate Key

```
POST /api/v1/encryption/rotate
```

### 14.4 List Versions

```
GET /api/v1/encryption/versions
```

## 15. Collaboration

### 15.1 Create Share

```
POST /api/v1/shares
```

### 15.2 List Shares

```
GET /api/v1/shares
```

### 15.3 Set Permissions

```
PUT /api/v1/shares/{id}/permissions
```

### 15.4 Add Comment

```
POST /api/v1/comments
```

### 15.5 List Comments

```
GET /api/v1/comments?path={path}
```

## 16. GraphQL

```
POST /api/v1/graphql
```

**Playground**:
```
GET /api/v1/graphql/playground
```

## 17. WebSocket

```
GET /api/v1/ws
```

**Events**:
- `file.created`
- `file.modified`
- `file.deleted`
- `pipe.status_changed`
- `metrics.updated`

## 18. Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| PATH_NOT_FOUND | 404 | Path does not exist |
| PATH_EXISTS | 409 | Path already exists |
| IS_DIRECTORY | 400 | Cannot perform operation on directory |
| NOT_DIRECTORY | 400 | Path is not a directory |
| PERMISSION_DENIED | 403 | Operation not permitted |
| CIRCUIT_BREAKER_OPEN | 503 | Service temporarily unavailable |
| INVALID_PATH | 400 | Invalid path format |
| HANDLE_NOT_FOUND | 404 | Handle does not exist |
| HANDLE_EXPIRED | 410 | Handle lease expired |
| QUOTA_EXCEEDED | 507 | Storage quota exceeded |

## 19. Rate Limits

| Endpoint | Limit |
|----------|-------|
| `/api/v1/files` (read) | 1000/min |
| `/api/v1/files` (write) | 100/min |
| `/api/v1/memories` | 100/min |
| Other | 1000/min |

## 20. Related Documents

- [Getting Started](GETTING_STARTED.md)
- [SDK Reference]()
- [CLI Reference](cli-mode.md)
