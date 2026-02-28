# EVIF REST API 文档

本文档描述 evif-rest 提供的 HTTP REST 接口，与 evif-client、evif-cli、evif-web 及 evif-mcp 的调用约定一致。

**基础 URL**：默认 `http://localhost:8080`（可通过 `EVIF_PORT` 等配置修改；evif-mcp 默认使用 `http://localhost:8081`）。

**版本**：API 前缀为 `/api/v1/`（兼容前端 `/api/v1/fs/*` 无版本前缀的路径仍为 `/api/v1/fs/*`）。

---

## 一、通用约定

### 1.1 请求与响应

- **Content-Type**：请求体为 JSON 时使用 `Content-Type: application/json`；响应通常为 `application/json`。
- **路径编码**：路径参数与 query 中的 `path` 为 UTF-8 字符串，需以 `/` 开头（如 `/mem/foo.txt`），且会按挂载表解析到对应插件。
- **错误响应**：所有错误响应体格式统一为：
  ```json
  {
    "error": "HTTP 状态描述",
    "message": "具体错误信息"
  }
  ```

### 1.2 HTTP 状态码与错误类型

| HTTP 状态码 | RestError 变体 | 说明 |
|-------------|----------------|------|
| 400 Bad Request | BadRequest | 参数错误、校验失败（如 mount 前插件 validate 失败） |
| 404 Not Found | NotFound | 路径未找到、插件未找到、句柄不存在等 |
| 500 Internal Server Error | Internal, Vfs, Io, Http | 服务端内部错误、插件执行失败、无效 base64 等 |

---

## 二、健康检查

### GET /health

无需认证，用于探活。

**响应示例**：
```json
{
  "status": "healthy",
  "version": "1.0.0",
  "timestamp": "2026-01-31T12:00:00Z"
}
```

### GET /api/v1/health

与 evif-client、evif-cli、evif-mcp 契约一致，返回 `status`、`version`、`uptime`（秒），用于 CLI `evif health` 与 MCP health 检查。

**响应示例**：
```json
{
  "status": "healthy",
  "version": "2.4.0",
  "uptime": 3600
}
```

---

## 三、兼容前端 API（evif-web）

以下接口供 evif-web 使用，路径为 `/api/v1/fs/*`。

### GET /api/v1/fs/list

列出目录内容。

**Query**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| path | string | 是 | 目录路径，如 `/mem` |

**响应**：`FsListResponse`
```json
{
  "nodes": [
    {
      "path": "/mem/a",
      "name": "a",
      "is_dir": false,
      "children": null
    }
  ]
}
```

### GET /api/v1/fs/read

读取文件内容（明文 UTF-8，供前端展示）。

**Query**：`path`（必填）。

**响应**：`FsReadResponse`
```json
{
  "content": "文件文本内容"
}
```

### POST /api/v1/fs/write

写入文件。

**Query**：`path`（必填）。  
**Body**：`FsWriteBody`
```json
{
  "content": "要写入的文本"
}
```

**响应**：`{ "ok": true }`

### POST /api/v1/fs/create

创建文件或目录（由 path 语义决定）。

**Body**：`{ "path": "/mem/newfile.txt" }`  
**响应**：`{ "ok": true }`

### DELETE /api/v1/fs/delete

删除文件或目录。

**Query**：`path`（必填）。  
**响应**：`{ "ok": true }`

---

## 四、文件操作（与 evif-client 契约一致）

### GET /api/v1/files

读取文件，返回明文 `content` 与 base64 `data`，供 CLI/二进制使用。

**Query**：`FileQueryParams`

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| path | string | 是 | 文件路径 |
| offset | u64 | 否 | 读取偏移，默认 0 |
| size | u64 | 否 | 读取长度，0 表示全部 |

**响应**：`FileReadResponse`
```json
{
  "content": "UTF-8 明文",
  "data": "base64 编码内容",
  "size": 123
}
```

### PUT /api/v1/files

写入文件。

**Query**：`FileWriteParams`

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| path | string | 是 | 文件路径 |
| offset | u64 | 否 | 写入偏移，默认 0 |
| flags | string | 否 | 写标志，可选 |

**Body**：`FileWriteRequest`
```json
{
  "data": "明文或 base64 字符串",
  "encoding": "base64"
}
```
当 `encoding` 为 `"base64"` 时，`data` 按 base64 解码后写入；否则按 UTF-8 字节写入。

**响应**：`FileWriteResponse`
```json
{
  "bytes_written": 10,
  "path": "/mem/foo.txt"
}
```

### POST /api/v1/files

创建空文件。

**Body**：`{ "path": "/mem/empty.txt" }`  
**响应**：`{ "path": "/mem/empty.txt", "created": true }`

### DELETE /api/v1/files

删除文件。

**Query**：`path`（必填）。  
**响应**：`{ "path": "/mem/foo.txt", "deleted": true }`

---

## 五、目录操作

### GET /api/v1/directories

列出目录。

**Query**：`path`（必填）。

**响应**：`DirectoryListResponse`
```json
{
  "path": "/mem",
  "files": [
    {
      "id": null,
      "name": "a",
      "path": "/mem/a",
      "is_dir": false,
      "size": 0,
      "modified": "",
      "created": ""
    }
  ]
}
```

### POST /api/v1/directories

创建目录。

**Body**：`CreateDirectoryRequest`
```json
{
  "path": "/mem/dir",
  "parents": true,
  "mode": 0755
}
```
`parents` 为 true 时递归创建父目录；`mode` 可选。

**响应**：`{ "path": "/mem/dir", "created": true }`

### DELETE /api/v1/directories

删除目录。

**Query**：`path`（必填），`recursive`（可选，是否递归）。  
**响应**：`{ "path": "/mem/dir", "deleted": true }`

---

## 六、元数据与高级操作

### GET /api/v1/stat

获取文件/目录元数据。

**Query**：`path`（必填）。

**响应**：`FileStat`
```json
{
  "path": "/mem/foo",
  "size": 0,
  "is_dir": false,
  "modified": "",
  "created": ""
}
```

### POST /api/v1/digest

计算文件哈希。

**Body**：`FileDigestRequest`
```json
{
  "path": "/mem/foo.txt",
  "algorithm": "sha256"
}
```
`algorithm` 可选，默认 `sha256`；支持 `sha256`、`sha512`、`md5` 等。

**响应**：`FileDigestResponse`
```json
{
  "path": "/mem/foo.txt",
  "algorithm": "sha256",
  "hash": "e3b0c44..."
}
```

### POST /api/v1/touch

更新文件/目录时间戳。

**Body**：`{ "path": "/mem/foo" }`  
**响应**：`{ "path": "/mem/foo", "touched": true }`

### POST /api/v1/grep

按正则搜索文件内容。

**Body**：`GrepRequest`
```json
{
  "path": "/mem",
  "pattern": "hello",
  "recursive": true
}
```

**响应**：`GrepResponse`
```json
{
  "pattern": "hello",
  "matches": [
    {
      "path": "/mem/a.txt",
      "line": 1,
      "content": "hello world"
    }
  ]
}
```

### POST /api/v1/rename

重命名/移动。

**Body**：`RenameRequest`
```json
{
  "from": "/mem/old",
  "to": "/mem/new"
}
```
**响应**：`{ "from": "/mem/old", "to": "/mem/new", "renamed": true }`

---

## 七、挂载管理

### GET /api/v1/mounts

列出当前挂载点（与 evif-client 契约一致，返回 `mounts` 数组）。

**响应**：`ListMountsResponse`
```json
{
  "mounts": [
    {
      "plugin": "mem",
      "path": "/mem"
    }
  ]
}
```

### POST /api/v1/mount

动态挂载插件。

**Body**：`MountRequest`
```json
{
  "plugin": "mem",
  "path": "/mem2",
  "config": {}
}
```
- `plugin`：支持 `mem`/`memfs`、`hello`/`hellofs`、`local`/`localfs`。local 时 `config` 可含 `root` 等。
- `path`：必须以 `/` 开头且非空。  
挂载前会调用插件的 `validate(config)`，失败返回 400。

**响应**：`{ "path": "/mem2", "plugin": "mem", "mounted": true }`

### POST /api/v1/unmount

动态卸载挂载点。

**Body**：`UnmountRequest`
```json
{
  "path": "/mem2"
}
```
**响应**：`{ "path": "/mem2", "unmounted": true }`

---

## 八、插件管理

### GET /api/v1/plugins

列出内置插件信息（名称、版本、描述）。

**响应**：`Vec<PluginInfo>`
```json
[
  {
    "name": "mem",
    "version": "0.1.0",
    "description": "In-memory filesystem"
  }
]
```

### GET /api/v1/plugins/:name/readme

获取插件 README 文本。

**路径参数**：`name`（如 `mem`、`hello`、`local`）。

**响应**：`PluginReadmeResponse`
```json
{
  "name": "mem",
  "readme": "# MemFs\n\n..."
}
```

### GET /api/v1/plugins/:name/config

获取插件配置参数元数据（名称、类型、必填等）。

**响应**：`PluginConfigParamsResponse`
```json
{
  "name": "local",
  "params": [
    {
      "name": "root",
      "param_type": "string",
      "required": true,
      "description": "Local root path"
    }
  ]
}
```

### POST /api/v1/plugins/load

加载外部插件（当前实现返回“未支持”）。

**Body**：`{ "path": "/path/to/plugin", "config": {} }`

### POST /api/v1/plugins/unload

卸载插件（WASM 等）。

### GET /api/v1/plugins/list

列出已加载插件详情（含 WASM）。

### POST /api/v1/plugins/wasm/load

加载 WASM 插件。

---

## 九、监控与指标

### GET /api/v1/metrics/traffic

流量统计。

**响应**：含 `total_requests`、`total_bytes_read`、`total_bytes_written`、`total_errors`、`read_count`、`write_count`、`list_count`、`other_count`、`average_read_size`、`average_write_size` 等。

### GET /api/v1/metrics/operations

按操作类型统计（read/write/list/other 的 count、bytes、errors）。

### GET /api/v1/metrics/status

系统状态（供 evif-web 监控页使用）。

**响应**：含 `status`、`uptime_secs`、`uptime`、`mounts.count`、`mounts.list`、`traffic`、`operations`。

### POST /api/v1/metrics/reset

重置所有指标计数。

**响应**：`{ "message": "Metrics reset successfully" }`

---

## 十、句柄 API（HandleFS）

需要插件实现 HandleFS；否则返回 500。

### POST /api/v1/handles/open

打开文件句柄。

**Body**：`OpenHandleRequest`
```json
{
  "path": "/mem/foo",
  "flags": "r",
  "mode": 0644,
  "lease": 60
}
```
`flags`：`r`/`w`/`rw`/`rw-create` 等。

**响应**：`OpenHandleResponse`
```json
{
  "handle_id": 1,
  "path": "/mem/foo",
  "flags": "r",
  "lease_expires_at": 1234567890
}
```

### GET /api/v1/handles/:id

获取句柄信息。

**响应**：`HandleInfoResponse`（handle_id, path, flags, plugin_id, lease_expires_at）。

### POST /api/v1/handles/:id/read

从句柄读取。**Body**：`{ "size": 4096 }`。**响应**：`{ "data": "base64", "bytes_read": 100, "eof": false }`

### POST /api/v1/handles/:id/write

写入句柄。**Body**：`{ "data": "base64", "offset": 0 }`。**响应**：`{ "bytes_written": 100 }`

### POST /api/v1/handles/:id/seek

Seek。**Body**：`{ "offset": 0, "whence": "set" }`（whence: set/cur/end）。**响应**：`{ "new_offset": 0 }`

### POST /api/v1/handles/:id/sync

同步到存储。

### POST /api/v1/handles/:id/close

关闭句柄。

### POST /api/v1/handles/:id/renew

续租。**Body**：`{ "lease": 60 }`

### GET /api/v1/handles

列出所有句柄。**响应**：`{ "handles": [...], "count": 0 }`

### GET /api/v1/handles/stats

获取句柄统计信息。

**响应**：`HandleStatsResponse`
```json
{
  "total": 10,
  "active": 8,
  "idle": 2
}
```

---

## 十一、批量操作

### POST /api/v1/batch/copy

批量复制。

**Body**：
```json
{
  "sources": ["/mem/a", "/mem/b"],
  "destination": "/mem/dest",
  "recursive": false,
  "overwrite": false,
  "concurrency": 4
}
```
**响应**：`{ "operation_id": "uuid", "message": "Batch copy operation started" }`

### POST /api/v1/batch/delete

批量删除。

**Body**：
```json
{
  "paths": ["/mem/a", "/mem/b"],
  "recursive": false,
  "concurrency": 4
}
```
**响应**：`{ "operation_id": "uuid", "message": "Batch delete operation started" }`

### GET /api/v1/batch/progress/:id

查询批量操作进度。**响应**：`BatchOperationInfo`（id, operation_type, status, progress, current_file, error, start_time, end_time）。

### GET /api/v1/batch/operations

列出所有批量操作。**响应**：`{ "operations": [...], "count": 0 }`

### DELETE /api/v1/batch/operation/:id

取消批量操作。**响应**：`{ "message": "Operation cancelled", "operation_id": "..." }`

---

## 十二、WebSocket

### GET /ws

建立 WebSocket 连接，供 evif-web 终端等使用。协议与消息格式见服务端与前端约定。

---

## 十三、图 API（占位，未实现）

以下端点当前均返回 **500**，并提示使用文件系统 API 替代：

- GET /nodes/:id  
- DELETE /nodes/:id  
- POST /nodes/create/:node_type  
- POST /query  
- GET /nodes/:id/children  
- GET /stats  

若需图能力，需基于 evif-graph 实现或明确废弃，参见 Phase 11.3。

---

## 十四、与 evif-client / evif-cli / evif-mcp 的对应关系

| 能力 | evif-client 方法 | REST 端点 | 说明 |
|------|------------------|-----------|------|
| 读文件 | read_file / 使用 data base64 | GET /api/v1/files?path= | 响应含 content + data(base64) |
| 写文件 | write（JSON body，encoding=base64） | PUT /api/v1/files?path= + FileWriteRequest | 与 7.1 契约一致 |
| 列目录 | list_directory | GET /api/v1/directories?path= | 返回 files 数组 |
| 创建目录 | create_directory | POST /api/v1/directories | Body path (+ parents/mode) |
| 删除文件/目录 | delete_file / delete_directory | DELETE /api/v1/files、DELETE /api/v1/directories | Query path |
| 元数据 | stat | GET /api/v1/stat?path= | |
| 哈希 | digest | POST /api/v1/digest | Body path + algorithm |
| 搜索 | grep | POST /api/v1/grep | Body path, pattern, recursive |
| 重命名 | rename | POST /api/v1/rename | Body from, to |
| 挂载列表 | list_mounts | GET /api/v1/mounts | 返回 { "mounts": [...] } |
| 挂载/卸载 | mount / unmount | POST /api/v1/mount、POST /api/v1/unmount | JSON body |

evif-mcp 工具（evif_ls、evif_cat、evif_write、evif_mkdir、evif_rm、evif_stat、evif_mv、evif_cp、evif_mounts、evif_grep、evif_mount、evif_unmount、evif_health）按上述 REST 路径与 body 格式调用，默认 base URL 为 `http://localhost:8081`。

---

**文档版本**：与 EVIF 2.4 计划 Phase 12.1 对应；如有接口变更，以代码与集成测试为准。
