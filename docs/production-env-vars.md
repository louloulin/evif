# EVIF 生产环境变量最小清单

> 创建时间：2026-04-03
> 适用版本：EVIF 0.1.0+
> 用途：为生产模式部署提供最小必需环境变量清单，区分"必须设置"和"建议设置"

---

## 一、必须设置（生产模式启动校验）

以下环境变量在 `EVIF_REST_PRODUCTION_MODE=true` 时由 `EvifServer::run` 强制校验，缺失则拒绝启动。

| 变量 | 说明 | 示例值 |
|------|------|--------|
| `EVIF_REST_PRODUCTION_MODE` | 启用生产模式校验 | `true` |
| `EVIF_REST_TENANT_STATE_PATH` | 租户状态持久化路径 | `/data/tenant-state.json` |
| `EVIF_REST_SYNC_STATE_PATH` | 同步状态持久化路径 | `/data/sync-state.json` |
| `EVIF_REST_ENCRYPTION_STATE_PATH` | 加密状态持久化路径 | `/data/encryption-state.json` |
| `EVIF_REST_MEMORY_BACKEND` | 记忆后端类型，生产必须为持久化后端：`sqlite` 或 `postgres` | `sqlite` |
| `EVIF_REST_MEMORY_SQLITE_PATH` | SQLite 数据库文件路径（当后端为 `sqlite` 时必填） | `/data/evif-memory.db` |
| `EVIF_REST_MEMORY_POSTGRES_URL` | PostgreSQL 连接串（当后端为 `postgres` 时必填） | `postgres://postgres:password@db:5432/evif` |
| `EVIF_REST_MEMORY_POSTGRES_MAX_CONNECTIONS` | PostgreSQL 连接池最大连接数（当后端为 `postgres` 时可选） | `10` |
| `EVIF_REST_MEMORY_POSTGRES_MIN_CONNECTIONS` | PostgreSQL 连接池最小连接数（当后端为 `postgres` 时可选） | `0` |
| `EVIF_REST_MAX_BODY_BYTES` | 最大请求体字节数，防止 oversized JSON/body 触发内存压力 | `1048576` |

> **注意**：`EVIF_REST_PRODUCTION_MODE=true` 时，记忆后端强制要求持久化存储，不允许内存态；当前支持 `sqlite` 和 `postgres`。

---

## 二、建议设置（安全与可观测性）

| 变量 | 说明 | 示例值 |
|------|------|--------|
| `EVIF_REST_AUTH_MODE` | 启用认证 | `enabled` |
| `EVIF_REST_WRITE_API_KEYS` | 写权限 API Key（多个用逗号分隔） | `key1,key2` |
| `EVIF_REST_ADMIN_API_KEYS` | 管理员 API Key（多个用逗号分隔） | `admin-key` |
| `EVIF_REST_WRITE_API_KEYS_SHA256` | 写权限 API Key 的 SHA-256 哈希列表，可替代明文 key | `3b7f...` |
| `EVIF_REST_ADMIN_API_KEYS_SHA256` | 管理员 API Key 的 SHA-256 哈希列表，可替代明文 key | `8a91...` |
| `EVIF_REST_API_KEY_MAX_CONCURRENT_REQUESTS` | 单个 API Key 的最大并发请求数，超出返回 `429` | `8` |
| `EVIF_REST_IP_MAX_CONCURRENT_REQUESTS` | 单个客户端 IP 的最大并发请求数，超出返回 `429` | `16` |
| `EVIF_REST_AUTH_AUDIT_LOG` | 审计日志路径 | `/var/log/evif/audit.log` |
| `RUST_LOG` | 日志级别 | `warn`（生产推荐）/ `info`（调试） |
| `EVIF_CORS_ENABLED` | 启用 CORS | `true` |
| `EVIF_CORS_ORIGINS` | 允许的源（逗号分隔） | `https://evif.example.com` |

---

## 三、可选设置（高级配置）

| 变量 | 说明 | 示例值 |
|------|------|--------|
| `EVIF_REST_BIND_ADDR` | 绑定地址 | `0.0.0.0`（默认） |
| `EVIF_REST_PORT` | 监听端口 | `8081`（默认） |
| `EVIF_CONFIG` | 挂载配置文件路径（JSON/YAML/TOML） | `/etc/evif/evif.json` |
| `EVIF_MOUNTS` | 内联挂载配置（JSON 数组） | `[{"path":"/mem","plugin":"mem"}]` |
| `EVIF_SHUTDOWN_TIMEOUT` | 优雅关闭超时秒数 | `30`（默认） |

---

## 四、Docker Compose 生产部署示例

```yaml
# docker-compose.prod.yml 环境变量片段
services:
  evif-rest:
    environment:
      # ─── 必须 ────────────────────────────────────────────
      - EVIF_REST_PRODUCTION_MODE=true
      - EVIF_REST_TENANT_STATE_PATH=/data/tenant-state.json
      - EVIF_REST_SYNC_STATE_PATH=/data/sync-state.json
      - EVIF_REST_ENCRYPTION_STATE_PATH=/data/encryption-state.json
      - EVIF_REST_MEMORY_BACKEND=sqlite
      - EVIF_REST_MEMORY_SQLITE_PATH=/data/evif-memory.db
      # 如使用 PostgreSQL，可改为：
      # - EVIF_REST_MEMORY_BACKEND=postgres
      # - EVIF_REST_MEMORY_POSTGRES_URL=postgres://postgres:password@db:5432/evif
      # - EVIF_REST_MEMORY_POSTGRES_MAX_CONNECTIONS=10
      # - EVIF_REST_MEMORY_POSTGRES_MIN_CONNECTIONS=1
      - EVIF_REST_MAX_BODY_BYTES=1048576

      # ─── 建议 ────────────────────────────────────────────
      - EVIF_REST_AUTH_MODE=enabled
      # 明文 key 与哈希 key 二选一即可：
      # - EVIF_REST_WRITE_API_KEYS=write-key
      # - EVIF_REST_ADMIN_API_KEYS=admin-key
      # 或：
      # - EVIF_REST_WRITE_API_KEYS_SHA256=<sha256(write-key)>
      # - EVIF_REST_ADMIN_API_KEYS_SHA256=<sha256(admin-key)>
      - EVIF_REST_API_KEY_MAX_CONCURRENT_REQUESTS=8
      - EVIF_REST_IP_MAX_CONCURRENT_REQUESTS=16
      - EVIF_REST_AUTH_AUDIT_LOG=/var/log/evif/audit.log
      - RUST_LOG=warn
```

---

## 五、环境变量优先级

1. **最高**：容器 `environment` 直接指定
2. **中**：`.env` 文件（`docker compose --env-file` 加载）
3. **最低**：镜像内置默认值

生产部署推荐使用 Docker Secrets 或 Vault 等密钥管理服务，通过 `--env-file` 加载 API Key 类敏感变量。

---

## 六、验证清单

- [ ] `EVIF_REST_PRODUCTION_MODE=true` 已设置
- [ ] 三个状态持久化路径已设置且容器内目录可写
- [ ] `EVIF_REST_MEMORY_BACKEND` 已设置为 `sqlite` 或 `postgres`
- [ ] 若为 `sqlite`，`EVIF_REST_MEMORY_SQLITE_PATH` 已设置且路径位于持久化卷内
- [ ] 若为 `postgres`，`EVIF_REST_MEMORY_POSTGRES_URL` 已设置且数据库可连接
- [ ] 若为 `postgres` 且显式设置连接池参数，`EVIF_REST_MEMORY_POSTGRES_MIN_CONNECTIONS <= EVIF_REST_MEMORY_POSTGRES_MAX_CONNECTIONS`
- [ ] `EVIF_REST_MAX_BODY_BYTES` 已按部署入口的最大合法请求大小设置
- [ ] 如启用认证，已设置明文 key 或对应的 `*_SHA256` 哈希 key 环境变量
- [ ] 如启用认证且需限制单 key 并发，已设置 `EVIF_REST_API_KEY_MAX_CONCURRENT_REQUESTS`
- [ ] 如需按来源 IP 进一步限制并发，已设置 `EVIF_REST_IP_MAX_CONCURRENT_REQUESTS`
- [ ] `RUST_LOG` 设置为 `warn` 或更低日志级别
- [ ] `curl http://localhost:8081/api/v1/health` 返回 `healthy`
