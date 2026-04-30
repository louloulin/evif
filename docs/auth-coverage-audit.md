# EVIF REST API 认证覆盖审计

> 创建时间：2026-04-30
> 审计范围：全部 108 个 REST 端点
> 认证模式：API Key + JWT

---

## 一、认证架构

### 1.1 认证模式

EVIF 支持三种认证模式（通过 `EVIF_REST_AUTH_MODE` 配置）：

| 模式 | 说明 |
|------|------|
| `api-key` | API Key 认证（默认） |
| `jwt` | JWT Token 认证 |
| `disabled` | 禁用认证（仅开发环境） |

### 1.2 认证决策流程

```
请求进入
  ↓
检查 auth_state.is_enforced()
  ↓
route_requirement(method, path) → 端点权限要求
  ↓
authorize(headers, requirement) → AuthDecision
  ↓
授权结果: Granted / MissingCredentials / InvalidCredentials / Forbidden
```

### 1.3 权限级别

| 权限级别 | 说明 | 适用场景 |
|----------|------|----------|
| **公开** | 无需认证 | 健康检查、指标端点 |
| **Write** | 需要有效凭证 | 文件读写、内存操作 |
| **Admin** | 需要管理员凭证 | 挂载管理、租户管理、加密操作 |

---

## 二、端点分类

### 2.1 公开端点（无需认证）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/health` | 健康检查 |
| GET | `/api/v1/health` | V1 健康检查 |
| GET | `/api/v1/status` | 节点状态 |
| GET | `/api/v1/ready` | 就绪探针 |
| GET/POST | `/api/v1/ping` | 存活检查 |
| GET | `/metrics` | Prometheus 指标 |
| GET | `/api/v1/metrics/traffic` | 流量统计 |
| GET | `/api/v1/metrics/operations` | 操作统计 |
| GET | `/api/v1/metrics/status` | 系统状态 |
| GET | `/api/v1/mounts` | 挂载列表 |
| GET | `/api/v1/plugins` | 插件列表 |
| GET | `/api/v1/plugins/available` | 可用插件 |
| GET | `/api/v1/plugins/:name/readme` | 插件 README |
| GET | `/api/v1/plugins/:name/config` | 插件配置 |
| GET | `/api/v1/plugins/:name/status` | 插件状态 |
| GET | `/api/v1/cloud/status` | 云存储状态 |
| GET | `/api/v1/cloud/providers` | 云提供商 |
| GET | `/api/v1/llm/status` | LLM 状态 |

### 2.2 Write 权限端点

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/files` | 读取文件 |
| PUT | `/api/v1/files` | 写入文件 |
| POST | `/api/v1/files` | 创建文件 |
| DELETE | `/api/v1/files` | 删除文件 |
| GET | `/api/v1/directories` | 列出目录 |
| POST | `/api/v1/directories` | 创建目录 |
| DELETE | `/api/v1/directories` | 删除目录 |
| POST | `/api/v1/fs/list` | 兼容列表 |
| POST | `/api/v1/fs/read` | 兼容读取 |
| POST | `/api/v1/fs/write` | 兼容写入 |
| POST | `/api/v1/fs/create` | 兼容创建 |
| DELETE | `/api/v1/fs/delete` | 兼容删除 |
| POST | `/api/v1/fs/stream` | 流式读写 |
| POST | `/api/v1/fs/chmod` | 修改权限 |
| POST | `/api/v1/fs/chown` | 修改所有者 |
| POST | `/api/v1/digest` | 计算哈希 |
| POST | `/api/v1/grep` | 正则搜索 |
| POST | `/api/v1/touch` | 更新时间戳 |
| POST | `/api/v1/rename` | 重命名 |
| POST | `/api/v1/copy` | 跨文件系统复制 |
| POST | `/api/v1/copy/recursive` | 递归复制 |
| POST | `/api/v1/lock` | 获取文件锁 |
| DELETE | `/api/v1/lock` | 释放文件锁 |
| GET | `/api/v1/locks` | 列出锁 |
| GET | `/api/v1/handles` | 列出句柄 |
| POST | `/api/v1/handles/open` | 打开句柄 |
| GET | `/api/v1/handles/:id` | 获取句柄 |
| POST | `/api/v1/handles/:id/read` | 读句柄 |
| POST | `/api/v1/handles/:id/write` | 写句柄 |
| POST | `/api/v1/handles/:id/seek` | Seek 操作 |
| POST | `/api/v1/handles/:id/sync` | Sync 操作 |
| POST | `/api/v1/handles/:id/close` | 关闭句柄 |
| POST | `/api/v1/handles/:id/renew` | 续租句柄 |
| GET | `/api/v1/handles/stats` | 句柄统计 |
| POST | `/api/v1/batch/copy` | 批量复制 |
| POST | `/api/v1/batch/delete` | 批量删除 |
| DELETE | `/api/v1/batch/operation/:id` | 取消操作 |
| GET | `/api/v1/batch/progress/:id` | 进度查询 |
| GET | `/api/v1/batch/operations` | 操作列表 |
| GET | `/api/v1/share/list` | 分享列表 |
| POST | `/api/v1/share/create` | 创建分享 |
| POST | `/api/v1/share/revoke` | 撤销分享 |
| GET | `/api/v1/permissions/get` | 获取权限 |
| POST | `/api/v1/permissions/set` | 设置权限 |
| GET | `/api/v1/comments` | 列出评论 |
| POST | `/api/v1/comments` | 添加评论 |
| PUT | `/api/v1/comments/:id` | 更新评论 |
| PUT | `/api/v1/comments/:id/resolve` | 解决评论 |
| DELETE | `/api/v1/comments/:id` | 删除评论 |
| GET | `/api/v1/activities` | 获取活动 |
| GET | `/api/v1/users` | 列出用户 |
| GET | `/api/v1/memories` | 列出记忆 |
| POST | `/api/v1/memories` | 创建记忆 |
| GET | `/api/v1/memories/:id` | 获取记忆 |
| POST | `/api/v1/memories/search` | 搜索记忆 |
| POST | `/api/v1/memories/query` | 查询记忆 |
| GET | `/api/v1/categories` | 列出分类 |
| GET | `/api/v1/categories/:id` | 获取分类 |
| GET | `/api/v1/categories/:id/memories` | 分类记忆 |
| POST | `/api/v1/llm/complete` | LLM 补全 |
| POST | `/api/v1/llm/ping` | LLM ping |
| GET | `/api/v1/sync/status` | 同步状态 |
| GET | `/api/v1/sync/version` | 同步版本 |
| GET | `/api/v1/sync/:path/version` | 路径版本 |
| GET | `/api/v1/sync/conflicts` | 冲突列表 |
| POST | `/api/v1/sync/delta` | 应用增量 |
| POST | `/api/v1/sync/resolve` | 解决冲突 |

### 2.3 Admin 权限端点

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/v1/mount` | 挂载插件 |
| POST | `/api/v1/unmount` | 卸载插件 |
| GET | `/api/v1/plugins/list` | 插件列表 |
| POST | `/api/v1/plugins/load` | 加载插件 |
| POST | `/api/v1/plugins/unload` | 卸载插件 |
| POST | `/api/v1/plugins/wasm/load` | 加载 WASM |
| POST | `/api/v1/plugins/wasm/reload` | 重载 WASM |
| POST | `/api/v1/plugins/:name/reload` | 重载插件 |
| GET | `/api/v1/encryption/status` | 加密状态 |
| GET | `/api/v1/encryption/versions` | 密钥版本 |
| POST | `/api/v1/encryption/enable` | 启用加密 |
| POST | `/api/v1/encryption/disable` | 禁用加密 |
| POST | `/api/v1/encryption/rotate` | 轮换密钥 |
| GET | `/api/v1/tenants` | 租户列表 |
| POST | `/api/v1/tenants` | 创建租户 |
| GET | `/api/v1/tenants/me` | 当前租户 |
| GET | `/api/v1/tenants/:id` | 获取租户 |
| DELETE | `/api/v1/tenants/:id` | 删除租户 |
| PATCH | `/api/v1/tenants/:id/quota` | 更新配额 |
| POST | `/api/v1/metrics/reset` | 重置指标 |
| POST | `/api/v1/cloud/config` | 云存储配置 |
| POST | `/api/v1/graphql` | GraphQL 查询 |
| GET | `/api/v1/graphql/graphiql` | GraphiQL UI |

---

## 三、已修复的认证漏洞

### 3.1 本次更新修复的端点

| 原状态 | 现状态 | 路径 |
|--------|--------|------|
| 公开 | Write | `/api/v1/fs/stream` |
| 公开 | Write | `/api/v1/fs/chmod` |
| 公开 | Write | `/api/v1/fs/chown` |
| 公开 | Write | `/api/v1/digest` |
| 公开 | Write | `/api/v1/grep` |
| 公开 | Write | `/api/v1/copy` |
| 公开 | Write | `/api/v1/copy/recursive` |
| 公开 | Write | `/api/v1/lock` |
| 公开 | Write | `/api/v1/llm/complete` |
| 公开 | Write | `/api/v1/memories/search` |
| 公开 | Write | `/api/v1/memories/query` |
| 公开 | Write | `/api/v1/sync/delta` |
| 公开 | Write | `/api/v1/sync/resolve` |
| 公开 | Admin | `/api/v1/plugins/wasm/reload` |
| 公开 | Admin | `/api/v1/cloud/config` |
| 公开 | Admin | `/api/v1/graphql` |

### 3.2 安全改进说明

1. **文件操作保护**：`chmod`、`chown`、`copy` 等权限操作现需要 Write 权限
2. **LLM 成本控制**：`/api/v1/llm/complete` 现需要 Write 权限，防止滥用
3. **GraphQL 保护**：`/api/v1/graphql` 现需要 Admin 权限，保护敏感数据查询
4. **WASM 重载保护**：`/api/v1/plugins/wasm/reload` 现需要 Admin 权限

---

## 四、仍需注意的公开端点

以下端点设计为公开，这是有意为之：

| 路径 | 说明 | 理由 |
|------|------|------|
| `/health` | 健康检查 | 负载均衡器需要 |
| `/metrics` | Prometheus 指标 | 监控抓取需要 |
| `/api/v1/cloud/status` | 云存储状态 | 状态查询无需认证 |
| `/api/v1/llm/status` | LLM 状态 | 状态查询无需认证 |
| `/api/v1/plugins/:name/readme` | 插件文档 | 只读信息 |

---

## 五、API Key 格式

### 5.1 请求头格式

```bash
# 方式 1: x-api-key header
curl -H "x-api-key: sk-evif-xxx" http://localhost:8081/api/v1/files

# 方式 2: x-evif-api-key header
curl -H "x-evif-api-key: sk-evif-xxx" http://localhost:8081/api/v1/files

# 方式 3: Bearer token
curl -H "Authorization: Bearer sk-evif-xxx" http://localhost:8081/api/v1/files
```

### 5.2 环境变量配置

```bash
# 启用 API Key 认证
EVIF_REST_AUTH_MODE=api-key
EVIF_API_KEY=sk-evif-your-secret-key-here

# 禁用认证（仅开发）
EVIF_REST_AUTH_MODE=disabled
```

---

## 六、WebSocket 认证

WebSocket 连接 (`/ws`) 目前无认证。建议在生产环境中：

1. 使用 WSS（WebSocket Secure）
2. 在连接时传递 API Key 作为查询参数或首帧消息
3. 或使用 JWT 进行 WebSocket 认证

---

## 七、测试验证

```bash
# 测试公开端点（无需认证）
curl http://localhost:8081/health
curl http://localhost:8081/metrics

# 测试受保护端点（需要 Write 权限）
curl -H "x-api-key: sk-evif-test" http://localhost:8081/api/v1/files

# 测试管理员端点（需要 Admin 权限）
curl -H "x-api-key: sk-evif-admin" http://localhost:8081/api/v1/tenants
```

---

**文档版本**：EVIF 0.1.0+
**最后更新**：2026-04-30
