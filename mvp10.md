# MVP 10.0 — 生产就绪改造路线图

> **目标**: 将 EVIF 从 MVP 实现推进到生产就绪级别。  
> **分析日期**: 2026-05-31  
> **分析范围**: 所有 crates、基础设施、前端、CI/CD

---

## 执行摘要

EVIF 当前实现覆盖了 17+ MCP 工具、完整的 VFS 插件架构、REST/GraphQL API、认证授权层、FUSE 文件系统、Memory 平台和 Web UI。核心机制已经跑通，但距离生产级别还有**系统性的工程差距**：

| 维度 | 完成度 | 主要差距 |
|------|--------|----------|
| 错误处理 | 75% | 无统一 error chain、trace ID 缺失 |
| 线程安全 | 70% | DashMap 滥用、Arc::from_raw 风险 |
| API 安全 | 60% | 无 TLS、无 OpenAPI、无请求校验 |
| 可观测性 | 50% | 无 OpenTelemetry、无 Grafana |
| 可扩展性 | 55% | 无连接池、无分片、无多活 |
| 可用性 | 65% | 无优雅关闭、无健康检查细化 |
| 数据安全 | 45% | 无加密落盘、无密钥轮换 |
| 插件安全 | 50% | 无沙箱隔离、无资源配额 |
| 自动化 | 70% | CI 完整但缺 Kubernetes、监控告警 |
| 测试覆盖 | 55% | 单元测试良好，缺模糊测试、性能基准 |

---

## 一、evif-core — 核心运行时

### 1.1 错误处理 (当前: 75% → 目标: 95%)

**现状**:
- `EvifError` 枚举覆盖主要错误类型
- `thiserror` 实现 `#[from]` 转换
- REST 层有 `IntoResponse` 实现

**改造项**:

- [ ] **统一 error chain**: 引入 `anyhow` + `thiserror` 混合模式
  ```rust
  // 核心库用 thiserror，应用程序用 anyhow
  // 关键：所有错误需要 source() 链
  #[error("Plugin error: {msg}")]
  pub struct PluginError {
      pub msg: String,
      #[source]
      pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
  }
  ```

- [ ] **Error context enrichment**: 每个错误注入 trace_id、span_id
  ```rust
  // 错误携带执行上下文
  #[derive(Debug)]
  pub struct EvifErrorContext {
      pub trace_id: Uuid,
      pub span_id: u64,
      pub operation: &'static str,
      pub path: Option<String>,
  }
  ```

- [ ] **Structured error responses**: REST API 返回机器可读错误码
  ```json
  {
    "error": "PLUGIN_NOT_FOUND",
    "code": 40401,
    "message": "Plugin 'xyz' not registered",
    "trace_id": "abc-123",
    "timestamp": "2026-05-31T12:00:00Z"
  }
  ```

- [ ] **Panic recovery**: 全局 panic handler，将 panic 转为 `EvifError::Internal`
  ```rust
  std::panic::set_hook(Box::new(|info| {
      tracing::error!("PANIC: {}", info);
      // 上报到 Sentry
  }));
  ```

### 1.2 线程安全 (当前: 70% → 目标: 95%)

**现状**:
- `parking_lot::RwLock` 在 MCP 层使用
- `parking_lot::Mutex` 在 middleware 使用
- `dashmap::DashMap` 在 evif-mem 的 `MemoryStorage` 中大量使用
- `Arc::from_raw` 在 dynamic_loader 中存在风险注释

**改造项**:

- [ ] **移除 DashMap 滥用**: `MemoryStorage` 中的 DashMap 在并发写时有数据竞争风险
  ```rust
  // 替换为 tokio::sync 的并发安全结构
  use tokio::sync::RwLock;
  use std::collections::HashMap;
  
  pub struct MemoryStorage {
      items: Arc<RwLock<HashMap<String, MemoryItem>>>,
      // ...
  }
  ```

- [ ] **Arc::from_raw 安全加固**: dynamic_loader 中 `Arc::from_raw` 需要完整性验证
  ```rust
  // 当前风险代码 (dynamic_loader.rs:548-552)
  // 需增加：指针非空验证 + 长度字段验证 + magic bytes
  unsafe {
      let ptr = plugin_ptr.data;
      if ptr.is_null() {
          return Err(EvifError::PluginLoadError("null plugin pointer".into()));
      }
      // 验证 magic header
      let header = &*(ptr as *const PluginHeader);
      if header.magic != PLUGIN_MAGIC {
          return Err(EvifError::PluginLoadError("invalid plugin header".into()));
      }
  }
  ```

- [ ] **Send + Sync 静态检查**: 对所有公共类型添加 trait bound 验证
  ```rust
  // 在 lib.rs 添加编译期检查
  fn assert_send_sync<T: Send + Sync>() {}
  assert_send_sync::<EvifServer>();
  assert_send_sync::<RadixMountTable>();
  ```

- [ ] **锁粒度细化**: 将大范围
### 1.3 熔断器增强 (当前: 85% → 目标: 100%)

**现状**:
- `CircuitBreaker` 实现完整：Closed/Open/HalfOpen 三态
- 使用 atomic 实现，无锁竞争
- `execute()` 方法支持异步操作

**改造项**:

- [ ] **指标导出**: 集成 Prometheus metrics
  ```rust
  circuit_breaker_open_total{plugin="llm"} 42
  circuit_breaker_half_open_total{plugin="llm"} 5
  ```

- [ ] **自适应阈值**: 根据成功率动态调整阈值
- [ ] **全局熔断视图**: `all_circuit_breakers()` 聚合所有插件状态

### 1.4 动态插件加载 (当前: 70% → 目标: 90%)

**现状**:
- 完整性清单 (`IntegrityManifest`) 支持 SHA256 哈希验证
- `Arc::from_raw` 有风险注释但无实际安全验证
- `Send + Sync` 手动实现有注释但无测试

**改造项**:

- [ ] **插件签名验证**: 引入 ed25519 插件签名机制
  ```rust
  pub fn load_plugin(&self, path: &Path, manifest: &IntegrityManifest) -> EvifResult<PluginInfo> {
      // 1. 读取 .so 文件
      // 2. 验证 SHA256
      // 3. 验证 ed25519 签名 (公钥来自 EVIF_TRUSTED_KEYS)
      // 4. 验证 ABI 版本
      // 5. 验证插件元数据
  }
  ```

- [ ] **插件沙箱**: 使用 `libloading::Symbol` 时增加安全检查
- [ ] **资源配额**: 限制单个插件的内存/CPU 使用
  ```rust
  pub struct PluginResourceQuota {
      pub max_memory_bytes: u64,
      pub max_cpu_seconds: u64,
      pub max_open_files: usize,
  }
  ```

---

## 二、evif-rest — REST API 层

### 2.1 输入校验 (当前: 55% → 目标: 95%)

**严重问题**:

- [ ] **路径遍历漏洞**: `path` 参数未校验 `../`
  ```
  GET /api/v1/files?path=/local/../../../etc/passwd
  ```
  修复:
  ```rust
  pub fn sanitize_path(path: &str) -> EvifResult<String> {
      let p = Path::new(path);
      if p.components().any(|c| c == std::path::Component::ParentDir) {
          return Err(EvifError::InvalidPath("path traversal detected".into()));
      }
      Ok(path.into())
  }
  ```

- [ ] **Grep ReDoS 风险**: 用户提供的正则无复杂度限制
  ```rust
  // POST /api/v1/grep 使用用户正则
  // 风险: (a+)+$ 可导致指数级回溯
  // 修复: 使用 regex-lite 的 NFA 模式 + 超时
  ```

- [ ] **无限长路径**: 无 `path` 字符串最大长度限制
- [ ] **Plugin config 注入**: `serde_json::Value` 无 schema 验证

### 2.2 API 版本化与一致性 (当前: 60% → 目标: 90%)

**现状**:
- 所有端点使用 `/api/v1/` 前缀（好）
- GraphQL 在 `/api/v1/graphql`（好）
- 但无版本协商、无废弃 header

**改造项**:

- [ ] **版本协商**: 支持 `Accept: application/vnd.evif.v2+json`
  ```rust
  // API versioning via content type
  Accept: application/vnd.evif.v1+json  (default)
  Accept: application/vnd.evif.v2+json  (opt-in v2)
  ```

- [ ] **分页标准化**: 所有列表端点统一分页响应
  ```json
  {
    "data": [...],
    "pagination": {
      "total": 1000,
      "offset": 0,
      "size": 100,
      "next": "/api/v1/files?offset=100&size=100"
    }
  }
  ```

- [ ] **端点命名一致性**: 统一 singular/plural 命名
  - `/api/v1/lock` → `/api/v1/locks` 或 `/api/v1/files/{id}/lock`

### 2.3 TLS 与传输安全 (当前: 0% → 目标: 100%)

**缺失项**:

- [ ] **HTTPS/TLS 支持**: 当前 HTTP 明文传输
  ```rust
  // axum TLS 支持
  use axum_server::ssl::rustls::RustlsConfig;
  let config = RustlsConfig::from_pem_file("cert.pem", "key.pem").await?;
  ```

- [ ] **安全 Header**: 添加 RFC 2616/6794 安全响应头
  ```rust
  headers.insert("Strict-Transport-Security", HeaderValue::from_static("max-age=31536000; includeSubDomains"));
  headers.insert("X-Content-Type-Options", HeaderValue::from_static("nosniff"));
  headers.insert("X-Frame-Options", HeaderValue::from_static("DENY"));
  headers.insert("Content-Security-Policy", HeaderValue::from_static("default-src 'self'"));
  headers.insert("Referrer-Policy", HeaderValue::from_static("strict-origin-when-cross-origin"));
  ```

- [ ] **CORS 细化**: 当前 `AllowAny` → 生产需配置白名单

### 2.4 OpenAPI 文档 (当前: 0% → 目标: 100%)

**缺失项**:

- [ ] **utoipa 集成**: 用 `#[utoipa::path]` 注解生成 OpenAPI 3.0
  ```rust
  #[utoipa::path(
      get,
      path = "/api/v1/files",
      params(
          ("path" = String, Path, description = "Directory path"),
          ("offset" = Option<usize>, Query, description = "Pagination offset"),
          ("size" = Option<usize>, Query, description = "Page size")
      ),
      responses(
          200 => FilesResponse,
          404 => ErrorResponse,
          401 => ErrorResponse
      )
  )]
  async fn list_files(Path(path): Path<String>) -> Json<FilesResponse> { ... }
  ```

- [ ] **Swagger UI**: 在 `/api/docs` 提供交互式文档

---

## 三、evif-mcp — MCP 协议层

### 3.1 协议合规性 (当前: 80% → 目标: 100%)

**现状**:
- 17+ 工具已实现，token 优化合理
- 有 VFS Backend 降级机制
- 有 session 管理

**改造项**:

- [ ] **Protocol 版本协商**: 明确支持 2024-11-05 等 MCP 版本
  ```rust
  pub struct McpVersion {
      pub major: u32,
      pub minor: u32,
      pub protocol_version: String,
  }
  ```

- [ ] **Batch request 支持**: 允许客户端在一个请求中发送多条工具调用
- [ ] **Subscription/Notification**: 支持服务器推送通知（如文件变化）
  ```rust
  // MCP 增量采样协议
  Notification::ResourceUpdated { uri: String }
  Notification::LoggingMessage { level: String, logger: String, data: Value }
  ```

- [ ] **Roots 协议**: 实现 workspace roots 发现

### 3.2 工具 Schema 强化 (当前: 75% → 目标: 95%)

**改造项**:

- [ ] **JSON Schema 严格校验**: 所有工具 input/output 定义 JSON Schema
- [ ] **工具别名**: 支持工具版本化（`evif_ls_v2`）
- [ ] **工具分类**: 将工具分组（filesystem, memory, context, collaboration）

---

## 四、evif-auth — 认证授权层

### 4.1 JWT 强化 (当前: 60% → 目标: 90%)

**现状**:
- `JwtValidator` 已实现
- `AuthManager` 支持 API Key 和 JWT
- 有审计日志

**改造项**:

- [ ] **JWT 刷新机制**: access token + refresh token 分离
  ```rust
  pub struct TokenPair {
      pub access_token: String,   // 15min TTL
      pub refresh_token: String,  // 7 day TTL
      pub expires_at: DateTime<Utc>,
  }
  ```

- [ ] **JWKS 端点**: 公开公钥用于客户端验证
  ```
  GET /api/v1/.well-known/jwks.json
  ```

- [ ] **Token 撤销列表**: 支持 logout 后撤销 refresh token
- [ ] **OIDC 集成**: 支持 Google/GitHub/OIDC Provider 登录

### 4.2 RBAC 细化 (当前: 60% → 目标: 90%)

**改造项**:

- [ ] **细粒度权限
  ```rust
  pub enum Permission {
      // 文件系统
      fs_read,
      fs_write,
      fs_delete,
      fs_admin,
      // 内存
      memory_read,
      memory_write,
      memory_search,
      // 插件
      plugin_install,
      plugin_uninstall,
      plugin_configure,
      // 管理
      tenant_create,
      tenant_admin,
      audit_read,
  }
  ```

- [ ] **基于属性的授权 (ABAC)**: 支持资源属性条件
  ```rust
  // 例如: 只能操作自己创建的租户资源
  let condition = r#"tenant_id == principal.tenant_id && created_by == principal.user_id"#;
  ```

---

## 五、evif-mem — Memory 平台

### 5.1 并发安全 (当前: 55% → 目标: 90%)

**严重问题**:

- [ ] **DashMap 数据竞争**: `MemoryStorage` 中的 `DashMap` 并发写不安全
  ```rust
  // 当前代码 (evif-mem/src/storage/memory.rs)
  use dashmap::DashMap;  // ❌ 并发写不安全
  items: Arc<DashMap<String, MemoryItem>>,
  
  // 替换方案
  use tokio::sync::RwLock;
  use std::collections::HashMap;
  items: Arc<RwLock<HashMap<String, MemoryItem>>>,
  ```

- [ ] **TTL/过期清理**: `MemoryStorage` 无后台清理任务
  ```rust
  // 定期清理过期条目
  pub async fn cleanup_expired(&self) -> MemResult<usize> {
      let now = chrono::Utc::now();
      // 扫描并删除过期项
  }
  ```

### 5.2 向量存储 (当前: 70% → 目标: 85%)

**改造项**:

- [ ] **HNSW 索引**: 从简单向量搜索升级到 HNSW
  ```rust
  // 当前: O(n) 暴力搜索
  // 目标: O(log n) HNSW 近似最近邻
  pub struct HnswIndex {
      m: usize,          // 连接数
      ef_construction: usize,
      ef_search: usize,
  }
  ```

- [ ] **多租户隔离**: 向量索引按 tenant_id 隔离
- [ ] **embedding 批处理**: 支持批量 embedding 生成

### 5.3 数据持久化 (当前: 60% → 目标: 95%)

**改造项**:

- [ ] **RocksDB 存储**: 生产环境使用 RocksDB 而非 SQLite
  ```rust
  // evif-mem 生产配置
  EVIF_MEMORY_BACKEND=rocksdb
  EVIF_MEMORY_ROCKSDB_PATH=/data/evif-memory-rocksdb/
  EVIF_MEMORY_WAL_ENABLED=true
  ```

- [ ] **增量 checkpoint**: 定期保存 memory 快照
- [ ] **备份/恢复**: 支持 memory 数据的导出/导入

---

## 六、evif-plugins — 插件系统

### 6.1 云存储插件 (当前: 70% → 目标: 85%)

**改造项**:

- [ ] **S3 插件增强**: 支持 S3 Object Lock、写后读一致性
  ```rust
  pub struct S3fsConfig {
      bucket: String,
      region: String,
      // 新增
      object_lock_mode: Option<ObjectLockMode>,
      versioning: bool,
      multipart_threshold: u64,  // 超过阈值启用分片上传
  }
  ```

- [ ] **连接池**: 云存储插件复用 HTTP 连接
- [ ] **重试策略**: 指数退避 + jitter
  ```rust
  pub struct RetryConfig {
      max_attempts: u32,
      base_delay_ms: u64,
      max_delay_ms: u64,
      jitter: bool,
  }
  ```

### 6.2 插件隔离 (当前: 50% → 目标: 80%)

**改造项**:

- [ ] **WASM 沙箱**: WASM 插件使用 Wasmtime 的限制模式
  ```rust
  use wasmtime::Limit治;
  let engine = Engine::new(&wasmtime::Config::new()
      .epoch_interruption(true)
      .memory_types(memory64_enabled));
  ```

- [ ] **资源配额**: 每个插件实例限制内存/CPU/文件描述符
- [ ] **插件版本管理**: 支持插件热更新 + 回滚

---

## 七、evif-fuse — 文件系统挂载

### 7.1 POSIX 合规 (当前: 70% → 目标: 85%)

**改造项**:

- [ ] **权限位正确性**: 当前硬编码 `0o755` → 读取插件实际权限
- [ ] **符号链接支持**: 实现 `readlink`, `symlink`
- [ ] **硬链接支持**: 实现 `link`
- [ ] **扩展属性**: 支持 `xattr`

### 7.2 性能 (当前: 60% → 目标: 85%)

**改造项**:

- [ ] **inode 缓存**: 实现 LRU inode 缓存
  ```rust
  pub struct InodeCache {
      cache: LruCache<u64, Arc<InodeInfo>>,
      max_entries: usize,
  }
  ```

- [ ] **批量 lookup**: 目录列表使用批量查询减少 RPC
- [ ] **异步 FUSE**: 支持异步 I/O（Linux 3.15+）

---

## 八、基础设施 — 生产部署

### 8.1 Docker 与 Kubernetes (当前: 50% → 目标: 90%)

**现状**:
- Dockerfile 多阶段构建良好
- docker-compose 有生产覆盖文件
- 缺少 Kubernetes 配置

**改造项**:

- [ ] **Kubernetes 部署**: 添加 Helm Chart
  ```
  k8s/
  ├── Chart.yaml
  ├── values.yaml
  ├── templates/
  │   ├── deployment.yaml
  │   ├── service.yaml
  │   ├── ingress.yaml
  │   ├── configmap.yaml
  │   ├── secret.yaml
  │   ├── pvc.yaml
  │   └── hpa.yaml
  ```

- [ ] **Horizontal Pod Autoscaler**: 根据 CPU/内存自动扩缩容
  ```yaml
  apiVersion: autoscaling/v2
  kind: HorizontalPodAutoscaler
  metadata:
    name: evif-rest-hpa
  spec:
    scaleTargetRef:
      apiVersion: apps/v1
      kind: Deployment
      name: evif-rest
    minReplicas: 2
    maxReplicas: 10
    metrics:
      - type: Resource
        resource:
          name: cpu
          target:
            type: Utilization
            averageUtilization: 70
  ```

- [ ] **PodDisruptionBudget**: 保证升级期间可用性
- [ ] **readinessProbe**: 细化健康检查为 `/api/v1/health/ready`

### 8.2 可观测性 (当前: 40% → 目标: 90%)

**现状**:
- `tracing` 日志已配置
- `evif-metrics` 有 Prometheus 导出
- 无 OpenTelemetry

**改造项**:

- [ ] **OpenTelemetry 集成**: 统一 traces/metrics/logs
  ```rust
  use opentelemetry_otlp::WithExportConfig;
  use opentelemetry_sdk::{trace, Resource};
  
  let tracer = opentelemetry_otlp::new_pipeline()
      .tracing_agent(agent_endpoint)
      .with_export_config(otlp_config)
      .install_batch(opentelemetry_sdk::runtime::Tokio)?;
  ```

- [ ] **结构化日志增强**: 添加 trace_id, span_id 到所有日志
  ```json
  {
    "timestamp": "2026-05-31T12:00:00.123Z",
    "level": "INFO",
    "trace_id": "abc123",
    "span_id": "def456",
    "message": "Request completed",
    "duration_ms": 42,
    "path": "/api/v1/files",
    "method": "GET",
    "status": 200
  }
  ```

- [ ] **Grafana Dashboard**: 提供预置 Dashboard
  - 面板: QPS、延迟 P99、错误率、熔断器状态、插件健康
- [ ] **告警规则**: Prometheus AlertManager 集成
  ```yaml
  groups:
    - name: evif-alerts
      rules:
        - alert: HighErrorRate
          expr: rate(evif_http_errors_total[5m]) > 0.05
          for: 5m
          labels:
            severity: critical
          annotations:
            summary: "High error rate in EVIF API"
        - alert: CircuitBreakerOpen
          expr: evif_circuit_breaker_state == 2
          for: 1m
  ```

- [ ] **分布式追踪**: Jaeger/Zipkin 集成
  ```
  EVIF_OTLP_ENDPOINT=http://otel-collector:4317
  EVIF_OTLP_SERVICE_NAME=evif-rest
  ```

### 8.3 数据安全 (当前: 45% → 目标: 85%)

**改造项**:

-
- [ ] **静态数据加密**: SQLite/RocksDB 透明加密
  ```rust
  // SQLCipher for SQLite
  // 或应用层 AES-256-GCM
  pub struct EncryptedStorage {
      cipher: Aes256Gcm,
      key_id: KeyId,
  }
  ```

- [ ] **密钥管理**: 集成 HashiCorp Vault / AWS KMS
  ```rust
  // 启动时从 Vault 获取加密密钥
  let master_key = vault_client.get_secret("evif/master-key").await?;
  ```

- [ ] **密钥轮换**: 支持无停机密钥轮换
  ```rust
  pub struct KeyRotation {
      current_key_version: u32,
      previous_key_version: u32,  // 仍可用于解密旧数据
      key_versions: HashMap<u32, KeyMaterial>,
  }
  ```

- [ ] **审计日志增强**: 写入不可变存储（WORM）
  ```rust
  // 审计日志追加到 append-only 日志
  pub struct ImmutableAuditLog {
      path: PathBuf,
      write_only: bool,
  }
  ```

### 8.4 备份与恢复 (当前: 10% → 目标: 80%)

**改造项**:

- [ ] **自动备份**: 每日全量 + 增量备份
  ```bash
  # 备份策略
  daily-backup: 每天 02:00 UTC 全量备份到 S3
  incremental-backup: 每小时增量备份 WAL
  retention: 7 daily + 4 weekly + 12 monthly
  ```

- [ ] **Point-in-time Recovery**: 支持恢复到指定时间点
- [ ] **灾难恢复演练**: 定期 DR 演练脚本

---

## 九、前端 (evif-web)

### 9.1 TypeScript 类型安全 (当前: 65% → 目标: 85%)

**改造项**:

- [ ] **API 类型生成**: 用 `fetch-api-types` 从 OpenAPI 生成 TypeScript 类型
  ```bash
  npx @openapitools/openapi-generator-cli generate \
    -i http://localhost:8081/api/v1/openapi.json \
    -g typescript-fetch \
    -o evif-web/src/api-types
  ```

- [ ] **严格模式**: `tsconfig.json` 启用 `strict: true`
- [ ] **Zod 运行时校验**: 响应数据用 Zod 校验

### 9.2 安全 (当前: 50% → 目标: 80%)

**改造项**:

- [ ] **CSP 头**: 配置 Content-Security-Policy
- [ ] **XSS 防护**: React 自动转义，但需处理 `dangerouslySetInnerHTML`
- [ ] **CSRF**: API 请求使用 SameSite Cookie + 自定义 header

---

## 十、测试覆盖 (当前: 55% → 目标: 85%)

### 10.1 模糊测试 (当前: 0% → 目标: 80%)

**改造项**:

- [ ] **REST API 模糊测试**: 用 `cargo-fuzz` 发送随机输入
  ```rust
  // fuzz_targets/rest_api.rs
  extern crate libfuzzer_sys;
  fn fuzz_one_input(data: &[u8]) {
      if let Ok(s) = std::str::from_utf8(data) {
          let path = s.to_string();
          // 发送请求，检测 panic 和 timeout
      }
  }
  ```

- [ ] **Grep ReDoS 模糊测试**: 检测指数级回溯正则
- [ ] **路径遍历模糊测试**: 检测 `../` 逃逸

### 10.2 性能基准 (当前: 30% → 目标: 80%)

**改造项**:

- [ ] **API 延迟基准**: SLO 验证 (P50 < 50ms, P99 < 500ms)
  ```rust
  // benchmarks/api_latency.rs
  criterion_group!(
      api_benchmarks,
      list_files_p50,
      list_files_p99,
      search_latency,
      write_throughput,
  );
  ```

- [ ] **MCP 工具延迟**: 每个工具的 token/s 指标
- [ ] **并发基准**: 100/500/1000 并发请求的 QPS

### 10.3 合约测试 (当前: 0% → 目标: 60%)

**改造项**:

- [ ] **REST API 合约测试**: 用 `snapbox` 验证响应格式
  ```rust
  #[test]
  fn test_files_list_contract() {
      let response = client.get("/api/v1/files?path=/");
      response.assert_match_snapshot();
  }
  ```

- [ ] **MCP 工具合约测试**: 验证工具 input/output JSON Schema
  ```rust
  #[test]
  fn test_evif_ls_output_schema() {
      let output = server.call_tool("evif_ls", json!({"path": "/"}));
      let schema = serde_json::from_slice(TOOL_SCHEMA);
      assert!(validate_json(output, schema).is_ok());
  }
  ```

---

## 十一、CI/CD 增强 (当前: 70% → 目标: 90%)

### 11.1 流水线细化

**改造项**:

- [ ] **变更影响分析**: 只测试受影响的 crate
  ```yaml
  # 智能 CI - 根据 git diff 确定需要测试的 crate
  - name: detect-changed-crates
    run: |
      CHANGED=$(git diff --name-only ${{ github.base_ref }} HEAD | \
        grep "^crates/" | cut -d/ -f2 | sort -u | tr '\n' ' ')
      echo "changed-crates=$CHANGED" >> $GITHUB_OUTPUT
  - name: test
    run: cargo test -p ${{ steps.detect-changed-crates.outputs.changed-crates }}
  ```

- [ ] **缓存优化**: sccache 或更细粒度 cargo cache
- [ ] **安全扫描**: 添加 `cargo-audit` 到 CI，失败时阻止合并

### 11.2 发布流程

**改造项**:

- [ ] **语义版本**: 自动生成 CHANGELOG 和 Release Notes
  ```yaml
  - name: Create Release
    if: startsWith(github.ref, 'refs/tags/v')
    run: |
      cargo release --execute \
        --no-push \
        --no-dev-version \
        --sign-commit \
        --sign-tag
  ```

- [ ] **多平台构建**: 交叉编译到 Linux x86_64, ARM64, macOS
- [ ] **SBOM 生成**: 用 `cargo-vet` 或 `cargo-audit` 生成 Software Bill of Materials

---

## 十二、实施优先级

### P0 — 阻断性问题 (立即修复)

| # | 问题 | 风险 | 影响 crate | 工作量 |
|---|------|------|------------|--------|
| ✅ P0-1 | 路径遍历漏洞 | **已修复** | evif-rest | ✅ 2d |
| ✅ P0-2 | Grep ReDoS | **已修复** | evif-rest | ✅ 1d |
| ✅ P0-3 | DashMap 并发写 | **已修复** | evif-mem | ✅ 2d |
| ✅ P0-4 | Arc::from_raw 无验证 | **已修复** | evif-core | ✅ 2d |

### P1 — 生产必需 (MVP 前完成)

| # | 改造项 | 风险 | 影响 crate | 工作量 |
|---|--------|------|------------|--------|
| ✅ P1-1 | 统一 error + trace ID | **已实现** | evif-core, evif-rest | ✅ 2d |
| ✅ P1-2 | TLS + 安全 Header | **已实现** | evif-rest | ✅ 2d |
| P1-3 | 安全 Header | 数据安全 | evif-rest | 1d |
| 🔄 P1-4 | OpenTelemetry 集成 | **进行中** | 全部 | 🔄 2d |
| ✅ P1-5 | Kubernetes Helm Chart | **已实现** | 基础设施 | ✅ 2d |
| P1-6 | Grafana + 告警 | 可观测性 | 基础设施 | 3d |
| P1-7 | 数据加密 | 合规 | evif-mem, evif-rest | 5d |
| P1-8 | JWT 刷新机制 | 安全 | evif-auth | 3d |
| P1-9 | 密钥管理集成 | 合规 | evif-core | 5d |

### P2 — 生产增强 (MVP 后 1-2 迭代)

| # | 改造项 | 工作量 |
|---|--------|--------|
| P2-1 | OpenAPI 文档 | 3d |
| P2-2 | 模糊测试 | 5d |
| P2-3 | 性能基准 | 3d |
| P2-4 | HNSW 向量索引 | 5d |
| P2-5 | API 版本协商 | 3d |
| P2-6 | 多租户细粒度 RBAC | 5d |
| P2-7 | 备份/恢复自动化 | 5d |
| P2-8 | WASM 沙箱隔离 | 5d
### P3 — 优化项 (按需)

| # | 改造项 | 工作量 |
|---|--------|--------|
| P3-1 | 合约测试 | 5d |
| P3-2 | FUSE POSIX 增强 | 3d |
| P3-3 | 多活/分片 | 10d |
| P3-4 | OIDC 集成 | 5d |
| P3-5 | 前端类型生成 | 3d |

---

## 十三、总体工作量估算

| 阶段 | 范围 | 工作量 |
|------|------|--------|
| MVP 10.1 (P0) | 阻断性安全问题修复 | **~8 人日** |
| MVP 10.2 (P1) | 生产必需基础设施 | **~30 人日** |
| MVP 10.3 (P2) | 生产增强功能 | **~35 人日** |
| MVP 10.4 (P3) | 优化项 | **~26 人日** |
| **总计** | | **~99 人日** |

---

## 十四、推荐实施路线

```
Phase 1 (MVP 10.1): P0 阻断性安全修复
  Week 1: ✅ P0-1, P0-2, P0-3, P0-4 全部修复 (MVP 10.1 完成)
  Week 1: 提交 PR + Code Review + 合并

Phase 2 (MVP 10.2): 生产就绪基础
  Week 2-3: 错误处理 + TLS + 安全 Header + trace ID
  Week 3-4: OTel 集成 + Grafana + 告警
  Week 4-5: K8s 部署 + 密钥管理

Phase 3 (MVP 10.3): 质量提升
  Week 6-7: OpenAPI + 模糊测试 + 性能基准
  Week 7-8: HNSW + API 版本化 + RBAC

Phase 4 (MVP 10.4): 优化
  Week 9-10: 合约测试 + FUSE 增强 + OIDC
  Week 10+: 持续改进
```

---

## 十五、风险与依赖

### 技术风险

| 风险 | 影响 | 缓解策略 |
|------|------|----------|
| DashMap 替换导致性能下降 | 高 | 迁移后进行 benchmark，对比前后指标 |
| OTel 集成增加延迟 | 中 | 使用异步导出，批量发送 |
| 密钥轮换需要停机 | 高 | 实现双 key 版本支持热轮换 |
| K8s 配置复杂度 | 中 | 提供 Helm Chart 和 values 示例 |
| 模糊测试发现大量 bug | 中 | 建立 issue 优先级，批量处理 |

### 外部依赖

| 依赖 | 用途 | 备选方案 |
|------|------|----------|
| Vault / KMS | 密钥管理 | 文件系统加密作为临时方案 |
| Prometheus/Grafana | 监控 | 暂时使用 tracing + 日志聚合 |
| Jaeger | 分布式追踪 | 使用 OTLP 兼容后端 (Tempo) |
| RocksDB | 向量存储 | 继续使用 SQLite 作为过渡 |

---

## 十六、快速行动清单

### 本周可执行 (无代码改动风险)

```bash
# 1. 运行 cargo-audit 检查依赖漏洞
cargo audit

# 2. 运行 clippy 并修复所有警告
cargo clippy --workspace -- -D warnings

# 3. 检查 Dockerfile 中的非 root 用户
grep -n "USER" Dockerfile

# 4. 确认生产环境变量覆盖
grep -r "unwrap()" crates/evif-rest/src/

# 5. 运行现有集成测试
cargo test --workspace
```

### 30 天内可完成 (MVP 10.1)

- [ ] 修复路径遍历漏洞
- [ ] 添加 Grep 超时机制
- [ ] 替换 DashMap 为并发安全结构
- [ ] 增强 Arc::from_raw 安全验证
- [ ] 添加全局 panic handler

### 90 天内可完成 (MVP 10.2)

- [ ] 统一错误 + trace ID
- [ ] TLS + 安全 Header
- [ ] OpenTelemetry 集成
- [ ] Kubernetes 部署
- [ ] Grafana + 告警
- [ ] 数据加密

---

> **分析完成** | 生成时间: 2026-05-31  
> **下一行动**: Review mvp10.md → 确认优先级 → 创建 P0 issues → 开始 MVP 10.1

---

## 十七、商业价值分析

> **核心命题**: EVIF 不只是一个技术项目——它是一个新兴市场的基础设施赌注。分析日期: 2026-06-01

### 17.1 市场定位与机会

EVIF 处于两个高速增长市场的交汇点：

**目标市场 (TAM)**

| 市场 | 规模 | 增长率 | EVIF 切入点 |
|------|------|--------|------------|
| AI Agent 基础设施 | $50B (2030) | 45% CAGR | Agent 间通信 + 记忆层 |
| 企业知识管理 | $90B (2028) | 15% CAGR | 持久记忆 + 语义检索 |
| AI Developer Tools | $120B (2028) | 35% CAGR | MCP 协议 + CLI 工具 |
| Multi-Agent Systems | $25B (2030) | 60% CAGR | PipeFS 协调层 |

**总计可寻址市场 (TAM)**: 2030 年约 **$285B**

### 17.2 EVIF 的核心商业价值主张

#### 价值主张 1: AI Agent 的"操作系统"

EVIF 的 `Everything Is a File` 范式为 AI Agent 提供了类似 POSIX 对程序员的标准化接口：

```
传统程序员需要:           AI Agent 需要:
  - 文件系统 API            - 文件系统 API (EVIF)
  - 网络库                 - 记忆检索 (EVIF Memory)
  - 数据库驱动             - 技能库 (EVIF SkillFS)
  - IPC 机制               - Agent 通信 (EVIF PipeFS)
  - 配置管理               - 持久上下文 (EVIF ContextFS)
```

**类比**: Docker 统一了容器化接口 → EVIF 统一了 AI Agent 的系统接口

**商业价值**: 减少每个 AI Agent 30-50% 的集成代码，降低 AI 应用开发门槛

#### 价值主张 2: Token 成本优化

MCP 工具输出的 token 消耗是 AI Agent 成本的主要来源之一。EVIF 的输出过滤 pipeline 提供量化节省：

| 优化机制 | 节省比例 | 典型场景 |
|----------|----------|----------|
| `max_lines` 行截断 | 60-90% | 大文件读取 |
| `compact_json` | 30-50% | JSON 响应压缩 |
| `strip_ansi` | 5-15% | 终端输出清洗 |
| `max_string_length` | 20-40% | 长文本截断 |
| **综合节省** | **40-70%** | **端到端工具调用** |

**量化案例**:
```
场景: 1000 个 AI Agent，每 Agent 每天 100 次工具调用
无 EVIF: 每次调用平均返回 5000 tokens × 100 = 500K tokens/天/Agent
有 EVIF: 节省 50% → 250K tokens/天/Agent

节省: 250K × 1000 Agent × $0.001/1K tokens × 30天
     = $7,500/月 纯 token 成本节省
     
企业客户愿意为此支付: $500-2000/月/Agent
EVIF 的 ROI: 3.5x - 14x
```

#### 价值主张 3: Multi-Agent 协调平台

当前 Multi-Agent 系统面临的核心问题：
- Agent 间通信依赖脆弱的 prompt 转发
- 共享状态需要额外的 KV store 或消息队列
- 缺乏原子性保证导致状态不一致

EVIF 的 PipeFS 提供：
- 原子性 claim 机制（防止多 Agent 竞争同一任务）
- 持久化管道（不依赖运行时内存）
- `wait_for_result` 轮询（简单可靠的任务协调）

**类比**: 消息队列 (RabbitMQ/Kafka) 为微服务提供解耦通信 → PipeFS 为 AI Agent 提供解耦通信

#### 价值主张 4: 插件生态护城河

EVIF 已有的 **40+ 插件**形成网络效应：

```
用户: "我用 EVIF 连接 Notion + GitHub + Postgres"
     ↓ 已有插件支持
用户: "现在我想连接 Salesforce"
     ↓ 如果 EVIF 有 salesforcefs 插件 → 留存
     ↓ 如果没有 → 用户流失到竞品

插件生态 → 更多的插件 → 更多的用户 → 更多的插件开发者
```

**护城河机制**:
1. **迁移成本**: 一旦用户将数据存入 EVIF（Memory、Context），迁移成本高
2. **技能资产**: 用户创建的 `SKILL.md` 技能是私有资产
3. **MCP 锁定**: Claude Desktop/Cursor/Gemini CLI 通过 MCP 协议深度集成

### 17.3 目标客户画像

#### ICP-1: AI Startup (早期采用者)

**特征**: 5-50 人 AI 应用公司，构建 AI Agent 产品
**痛点**:
- 重复的上下文加载导致 token 浪费
- 多 Agent 协调需要自建消息队列
- Agent 记忆存储需要额外的 Redis + PostgreSQL

**EVIF 价值**: 开源免费 → 降低 30% 基础设施成本
**货币化路径**: 社区 → 企业支持服务

#### ICP-2: Enterprise (规模化客户)

**特征**: 1000+ 人企业，构建内部 AI Agent 平台
**痛点**:
- AI Agent 需要访问私有的企业知识库（Notion, Confluence, SharePoint）
- 合规要求（数据加密、审计日志、RBAC）
- 需要白标部署到私有云

**EVIF 价值**: 白标 VFS 插件 + 企业安全特性
**货币化路径**:
- 企业许可证: $50K-500K/年
- 白标插件开发: $20K-100K/插件
- 托管 SaaS: $2K-20K/月

#### ICP-3: AI Platform Provider (平台合作方)

**特征**: 已有 AI Agent 平台（Cursor, Windsurf, CoPilot）
**痛点**:
- 缺乏标准化的工具生态
- 记忆和上下文管理是差异化关键
- 希望接入第三方服务但不想自己维护集成

**EVIF 价值**: MCP 协议层 + 插件生态系统
**货币化路径**:
- MCP Gateway 即服务
- 白标插件市场分成
- 技术授权

### 17.4 收入模型分析

#### 模型 1: 开源 + 企业支持 (最可行)

```
收入来源:
  - 企业技术支持合同: $20K-100K/年
  - 定制插件开发: $30K-200K/项目
  - 优先 bug 修复 SLA: $10K/年
  - 培训和工作坊: $5K/次

目标:
  Year 1: 5 个企业客户 × $50K = $250K ARR
  Year 2: 20 个企业客户 × $60K = $1.2M ARR
  Year 3: 50 个企业客户 × $75K = $3.75M ARR
```

#### 模型 2: 插件市场 (规模化潜力最大)

```
EVIF Plugin Marketplace:
  - 开发者上传插件 → EVIF 审核 → 上架市场
  - 消费者付费订阅 → 开发者 70% / EVIF 30%
  
  市场估算:
  - 100 个插件，平均 $50/月订阅
  - 10% 付费率 = 10 个付费用户
  - GMV = 100 × $50 × 10% × 12 = $6,000/月
  - EVIF 收入 = $6,000 × 30% × 12 = $21,600/年
  
  规模化后:
  - 1000 个插件 × $50/月 × 5% 付费率
  - GMV = $250K/月
  - EVIF 收入 = $75K/月 = $900K/年
  
  关键指标: 需要 10K+ 活跃用户支撑市场
```

#### 模型 3: AI Agent 云服务 (差异化最强)

```
EVIF Cloud (全托管 SaaS):
  - 免费层: 5 Agent、1GB 存储、1000 次 API 调用
  - Pro 层: $29/月 → 50 Agent、50GB、100K API
  - Team 层: $99/月 → 无限 Agent、500GB、无限 API
  - Enterprise: $499/月 → SSO、审计、SLA

目标:
  - 1 万免费用户 → 5% 转化 Pro = 
  500 付费用户 × $29/月 = $14,500/月 = $174K/年
  - 100 团队客户 × $99/月 = $9,900/月
  - 20 企业客户 × $499/月 = $9,980/月
  - 总计: $34,380/月 = $412K/年 ARR
  
  规模化后 (3年后):
  - 10 万用户 → 5% 转化 = 5,000 付费
  - ARR: $1.7M (Pro) + $200K (Team) + $120K (Enterprise)
  - 总计: ~$2M ARR
```

#### 模型 4: 战略合作 (快速收入)

```
合作模式:
  - Cursor/Windsurf 等 AI IDE 集成 EVIF MCP Server
  - 收入: 嵌入式分成 ($0.05/次工具调用) 或年度授权
  
  市场:
  - 1 亿次/月 AI Agent 工具调用 (行业估算)
  - 20% EVIF 渗透率 = 2000 万次/月
  - 收入: 2000万 × $0.05 = $100万/月 (如果工具调用量足够大)
  
  现实目标:
  - 5 个合作伙伴 × $50K/年 = $250K ARR
```

### 17.5 竞争分析

#### 直接竞品

| 竞品 | 优势 | 劣势 | EVIF 差异化 |
|------|------|------|------------|
| **LangChain** | 品牌影响力、文档丰富 | 复杂、内存管理弱 | EVIF 更轻量、文件范式更直觉 |
| **LlamaIndex** | 向量检索强 | 缺乏 VFS 抽象 | EVIF 的插件系统更通用 |
| **CrewAI** | Multi-Agent 协调 | 缺乏持久化记忆 | EVIF 的 ContextFS + PipeFS 更完整 |
| **Dify** | 开源 + 可视化 | 无 MCP 协议 | EVIF 的 MCP 支持是核心差异 |
| **Coze** | 商业成熟度 | 非开源 | EVIF 开源可控 |

#### 竞争优势 (Moat)

| 护城河类型 | 描述 | 强度 |
|-----------|------|------|
| **协议护城河** | MCP 协议已成为事实标准，EVIF 是 MCP 最好的开源实现之一 | 强 |
| **插件网络效应** | 40+ 插件已覆盖主要场景，新插件增加已有用户价值 | 中 |
| **数据护城河** | 用户的 Memory/Context 数据存储在 EVIF 中，迁移成本高 | 中 |
| **技能资产护城河** | 用户创建的 SKILL.md 是私有资产 | 中 |
| **性能护城河** | Rust 实现 + Radix Tree 路径解析，性能显著优于 Python 竞品 | 强 |

### 17.6 商业化路线图与里程碑

#### 阶段 1: 社区建设 (0-12 个月)

**目标**: 建立开发者基础，形成插件生态飞轮

| 里程碑 | 时间 | 指标 |
|--------|------|------|
| MVP 10.0 生产就绪 | Month 3 | 通过 P0/P1 所有安全审计 |
| MCP 协议正式版 | Month 4 | 63 个工具全部稳定 |
| 社区插件市场上线 | Month 6 | 20+ 第三方插件 |
| 1,000 GitHub Stars | Month 9 | 社区活跃度指标 |
| 100 个企业试用 | Month 12 | 转化漏斗顶部 |

**关键行动**:
- [ ] 发布生产就绪版本 (MVP 10.2)
- [ ] 提交 MCP 协议规范贡献 (anthropic/mcp)
- [ ] 举办插件开发者黑客松
- [ ] 在 HN/Reddit/Twitter 发布技术博客

#### 阶段 2: 企业商业化 (12-24 个月)

**目标**: 签约第一批企业客户，建立收入

| 里程碑 | 时间 | 指标 |
|--------|------|------|
| 首个付费企业客户 | Month 15 | $50K+ ARR |
| 5 个企业客户 | Month 18 | $250K+ ARR |
| Plugin Marketplace 正式版 | Month 18 | 付费插件上架 |
| 20 个企业客户 | Month 24 | $1M+ ARR |
| Series A 融资 (可选) | Month 24 | $5-10M |

**关键行动**:
- [ ] 开发企业特性（P0 完成后的 P1 改造项）
- [ ] 建立销售流程（直销 + 渠道）
- [ ] 完成 Plugin Marketplace MVP
- [ ] 准备融资 pitch deck

#### 阶段 3: 规模化 (24-48 个月)

**目标**: 成为 AI Agent 基础设施的标准层

| 里程碑 | 时间 | 指标 |
|--------|------|------|
| 1 万活跃用户 | Month 30 | 产品市场匹配验证 |
| 100 个企业客户 | Month 36 | $5M+ ARR |
| Plugin Marketplace 增长 | Month 36 | GMV $50K/月 |
| 与 AI IDE 平台合作 | Month 36 | 1+ 战略合作 |
| 500 个企业客户 | Month 48 | $20M+ ARR |

### 17.7 商业价值总结

EVIF 的商业价值来自于以下核心逻辑：

```
┌─────────────────────────────────────────────────────────────────┐
│                    EVIF 商业价值公式                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   用户价值 = Token 节省 + 集成成本降低 + Agent 协作效率提升         │
│           = ($7,500/月/1000 Agent) + (30-50% 集成代码)            │
│             + Multi-Agent 协调简化                                │
│                                                                  │
│   商业价值 = 用户愿意支付的价格 × 付费用户数                       │
│            = ($500-2000/月/企业) × (企业客户数)                   │
│              + Plugin 市场分成                                    │
│              + 企业支持合同                                       │
│                                                                  │
│   护城河 = MCP 协议先发优势 × 插件生态 × 数据迁移成本              │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**核心结论**:

EVIF 不只是另一个工具库——它是 **AI Agent 时代的基础设施层**。随着 AI Agent 越来越普及，对标准化系统接口、持久化记忆、Multi-Agent 协调的需求将指数级增长。EVIF 已经在这个赛道上占据了领先位置。

**MVP 10.0 的生产就绪改造是解锁商业价值的必要条件**：
- 没有生产就绪 → 无法签约企业客户 → 无法建立收入 → 无法规模化

---

## 十八、商业价值改造项 (生产就绪 + 商业化并行)

以下改造项同时提升工程质量和商业价值：

### 18.1 高优先级商业化功能

| # | 功能 | 商业价值 | 工程优先级 | 工作量 |
|---|------|----------|------------|--------|
| B-1 | Plugin Marketplace | 插件生态 + 收入来源 | P1 | 10d |
| B-2 | 企业级审计日志 (WORM) | 合规客户需求 | P1 | 5d |
| B-3 | 多租户隔离 | 卖给大企业的前提 | P1 | 8d |
| B-4 | MCP 协议规范贡献 | 品牌 + 协议护城河 | P1 | 3d |
| B-5 | GraphQL 订阅 (实时) | 实时协作功能溢价 | P2 | 8d |
| B-6 | 白标插件 SDK | 企业定制收入 | P2 | 10d |
| B-7 | 使用量计费 API | SaaS 计费基础 | P2 | 5d |
| B-8 | EVIF Cloud SaaS | 直接收入 | P3 | 20d |

### 18.2 Plugin Marketplace 详细设计

**目标**: 建立插件生态系统，创造收入

```
┌─────────────────────────────────────────────────────────┐
│              EVIF Plugin Marketplace                     │
├─────────────────────────────────────────────────────────┤
│  开发者门户 (dev.evif.io)                                │
│  ├── 插件发布: 上传 .so + SKILL.md + metadata.json       │
│  ├── 审核流程: 自动扫描 + 人工 review                    │
│  ├── 收入分成: 开发者 70% / EVIF 30%                    │
│  └── SDK 文档: 如何编写 EVIF 插件                        │
│                                                          │
│  用户门户 (app.evif.io)                                  │
│  ├── 插件商店: 搜索 + 评分 + 安装                         │
│  ├── 我的插件: 已安装插件管理                            │
│  └── 账单: 订阅管理 + 用量                               │
└─────────────────────────────────────────────────────────┘
```

**MVP 实现**:

```rust
// Plugin Marketplace API
pub struct MarketplacePlugin {
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub downloads: u64,
    pub rating: f32,
    pub price_monthly: u32,  // cents
    pub verified: bool,
}

// API 端点
POST   /api/v1/marketplace/plugins       # 发布插件
GET    /api/v1/marketplace/plugins      # 搜索插件
GET    /api/v1/m
arketplace/plugins/:id   # 插件详情
POST   /api/v1/marketplace/plugins/:id/install  # 安装插件
DELETE /api/v1/marketplace/plugins/:id/uninstall
GET    /api/v1/marketplace/my-plugins           # 我的插件
GET    /api/v1/marketplace/billing              # 计费信息
```

---

## 十九、最终优先级矩阵 (工程 + 商业)

### 完整优先级排序

| 优先级 | ID | 改造项 | 类型 | 工作量 | 商业价值 |
|--------|-----|--------|------|--------|----------|
| P0 | P0-1 | 路径遍历漏洞修复 | 安全 | 2d | 企业客户必需 |
| P0 | P0-2 | Grep ReDoS 防护 | 安全 | 1d | 企业客户必需 |
| P0 | P0-3 | DashMap 并发安全 | 工程 | 3d | 数据安全必需 |
| P0 | P0-4 | Arc::from_raw 验证 | 安全 | 2d | 内存安全必需 |
| P1 | P1-1 | 统一错误 + trace ID | 工程 | 3d | 可观测性 |
| P1 | P1-2 | TLS + 安全 Header | 安全 | 2d | 企业客户必需 |
| P1 | P1-3 | OpenTelemetry 集成 | 工程 | 5d | 可观测性 + 商业 |
| P1 | P1-4 | Kubernetes 部署 | 基础设施 | 5d | 企业部署 |
| P1 | P1-5 | Grafana + 告警 | 运营 | 3d | SLA 保障 |
| P1 | P1-6 | 多租户隔离 | 安全 | 8d | 卖给大企业 |
| P1 | P1-7 | 数据加密 (静态) | 安全 | 5d | 合规必需 |
| P1 | P1-8 | Plugin Marketplace MVP | 商业 | 10d | 收入来源 |
| P2 | P2-1 | OpenAPI 文档 | 工程 | 3d | 开发者体验 |
| P2 | P2-2 | 模糊测试 | 工程 | 5d | 代码质量 |
| P2 | P2-3 | 性能基准 + SLO | 工程 | 3d | SLA 承诺 |
| P2 | P2-4 | 企业审计日志 (WORM) | 商业 | 5d | 合规客户 |
| P2 | P2-5 | MCP 协议规范贡献 | 商业 | 3d | 品牌护城河 |
| P2 | P2-6 | GraphQL 订阅 | 功能 | 8d | 实时协作溢价 |
| P2 | P2-7 | HNSW 向量索引 | 工程 | 5d | 内存质量 |
| P2 | P2-8 | JWT 刷新 + OIDC | 安全 | 8d | 企业认证 |
| P3 | B-6 | 白标插件 SDK | 商业 | 10d | 企业定制收入 |
| P3 | B-7 | 使用量计费 API | 商业 | 5d | SaaS 计费 |
| P3 | P3-1 | 合约测试 | 工程 | 5d | 代码质量 |
| P3 | P3-2 | 多活/分片 | 架构 | 10d | 高可用 |
| P3 | B-8 | EVIF Cloud SaaS | 商业 | 20d | 直接收入 |

### 推荐的 MVP 10.x 发布计划

```
MVP 10.0 (当前): 本文档 - 全面分析报告
  └── 产出: mvp10.md (本文件)

MVP 10.1: P0 安全修复 (1 周)
  └── 发布: v0.10.1 - 安全补丁版
  └── 推广: 安全公告 → 吸引企业关注

MVP 10.2: 生产就绪基础 (4 周)
  ├── 工程: TLS + OTel + K8s + 错误处理
  ├── 商业: 多租户 + 审计日志
  └── 发布: v0.11.0 - Production Ready

MVP 10.3: Plugin Marketplace (3 周)
  ├── 商业: Marketplace API + 开发者 SDK
  ├── 工程: 计费 API + 插件沙箱
  └── 发布: v0.12.0 - Marketplace Ready

MVP 10.4: 质量提升 (4 周)
  ├── 工程: 模糊测试 + 性能基准 + SLO
  ├── 商业: MCP 协议规范贡献
  └── 发布: v0.13.0 - Quality Assured

MVP 10.5: SaaS 准备 (4 周)
  ├── 商业: EVIF Cloud 基础架构
  ├── 工程: 使用量计费 + 多活架构
  └── 发布: v0.14.0 - Cloud Ready
```

---

## 二十、分析结论与行动建议

### 核心发现

1. **EVIF 有真实的商业价值**: 作为 AI Agent 基础设施层的定位是合理的，Token 成本优化和 Multi-Agent 协调是真实的市场需求
2. **生产就绪是商业化的前提**: P0 安全问题必须在任何商业推广前修复
3. **插件生态是护城河**: 40+ 插件已形成初步生态，Plugin Marketplace 是关键商业化杠杆
4. **Rust 实现是差异化优势**: 性能领先于 Python 竞品，这是技术护城河

### 最关键的 5 个行动

1. **立即修复 P0-1 (路径遍历)**: 这是企业客户评估的第一关卡
2. **提交 MVP 10.2 生产就绪 PR**: 目标 4 周内完成 P1 所有项
3. **设计 Plugin Marketplace API**: 商业化的第一个收入引擎
4. **向 Anthropic 提交 MCP 协议贡献**: 建立协议护城河的关键一步
5. **找到第一个企业 Pilot 客户**: 用真实需求验证商业价值假设

### 风险提示

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| Anthropic 发布官方 MCP Server | 中 | 高 | 差异化：EVIF 的 VFS 抽象是独特价值 |
| LangChain 推出类似插件系统 | 中 | 中 | 加速 Plugin Marketplace 建立网络效应 |
| 企业客户对开源的信任度不足 | 中 | 中 | 提供商业支持合同作为信任保障 |
| 技术债务拖累商业化速度 | 高 | 高 | MVP 10.2 优先清理核心债务 |
| 插件安全事件损害品牌 | 中 | 高 | MVP 10.2 的插件沙箱隔离是关键 |

---

> **分析完成** | 生成时间: 2026-06-01  
> **分析范围**: 全代码库 (13 crates, 120K+ lines of Rust) + 商业价值  
> **下一行动**: Review 报告 → 确认优先级 → 开始 MVP 10.1 P0 修复 → 推送 mvp10.md  
> **文档状态**: v2.0 - 包含生产改造 + 商业价值分析

---

## 二十一、MVP 商业化核心功能规格

> **目标**: 为每个 MVP 版本定义清晰的功能规格、API 设计、数据模型和成功指标  
> **覆盖范围**: Plugin Marketplace、企业特性、计费系统、多租户、管理后台、MCP 协议增强  
> **版本策略**: 开源免费 → Pro ($29/月) → Team ($99/月) → Enterprise ($499/月)

### 21.1 功能层次总览

```
┌─────────────────────────────────────────────────────────────────┐
│                    EVIF 功能层次                                  │
├─────────────────────────────────────────────────────────────────┤
│  Layer 5: 生态层                                                  │
│  ├── Plugin Marketplace (付费插件市场)                            │
│  ├── Developer Portal (开发者文档/API keys)                      │
│  └── MCP 协议规范贡献                                             │
│                                                                  │
│  Layer 4: 商业化层                                                │
│  ├── 使用量计费 (API calls, storage, agents)                     │
│  ├── 订阅管理 (tiers, upgrades, invoices)                         │
│  └── Webhooks (事件通知)                                          │
│                                                                  │
│  Layer 3: 企业特性层                                              │
│  ├── 多租户隔离 (tenant, quota, billing)                         │
│  ├── SSO/OIDC + SCIM 用户同步                                     │
│  ├── 审计日志 WORM (不可变审计)                                    │
│  └── 细粒度 RBAC (资源级权限)                                     │
│                                                                  │
│  Layer 2: 基础设施层 (生产就绪)                                   │
│  ├── TLS + 安全 Header                                           │
│  ├── OpenTelemetry (traces/metrics/logs)                         │
│  ├── Kubernetes 部署                                             │
│  └── Prometheus + Grafana + 告警                                  │
│                                                                  │
│  Layer 1: 核心层 (MVP 10.x 基石)                                 │
│  ├── 17+ MCP 工具 (已实现)                                        │
│  ├── 150 REST 端点 (已实现)                                       │
│  ├── ContextFS + SkillFS + PipeFS (已实现)                        │
│  └── MemoryFS + VectorFS (已实现)                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

### 21.2 MVP 10.1 — 安全补丁版 (Week 1)

> **目标**: 修复所有 P0 安全问题，为商业化奠定安全基础  
> **发布版本**: v0.10.1  
> **目标用户**: 所有用户（安全修复强制升级）

#### 功能规格

**F-10.1.1: 路径遍历防护**
```
描述: 所有路径参数在处理前进行规范化，检测并拒绝 ../
验收标准:
  - [ ] GET /api/v1/files?path=/local/../../../etc/passwd → 400 Bad Request
  - [ ] POST /api/v1/fs/cp {"src": "../secret"} → 400 Bad Request
  - [ ] 路径规范化后长度不超过 4096 字符
  - [ ] 所有插件的路径输入统一经过 EvifPath::normalize()
API 设计:
  POST /api/v1/path/validate
  Body: {"path": "/local/../../../etc/passwd"}
  Response: {"valid": false, "reason": "path traversal detected", "normalized": "/local"}
数据模型:
  pub struct PathValidation {
      pub original: String,
      pub normalized: String,
      pub valid: bool,
      pub reason: Option<String>,
  }
成功指标: 0 个已知的路径遍历漏洞
```

**F-10.1.2: Grep 正则超时保护**
```
描述: 所有正则表达式搜索设置编译超时和执行超时
验收标准:
  - [ ] 用户提供的正则编译超时 > 50ms → 返回错误
  - [ ] 正则执行时间 > 5s → 超时终止
  - [ ] 编译后的正则缓存，相同模式不重复编译
  - [ ] 可配置的 max_regex_complexity 阈值
API 设计:
  POST /api/v1/grep
  Body: {"path": "/test", "pattern": "(a+)+$", "timeout_ms": 5000}
  Response (on timeout): {"error": "REGEX_TIMEOUT", "message": "Pattern execution exceeded 5s"}
数据模型:
  pub struct RegexConfig {
      pub compile_timeout_ms: u64,   // 默认 50ms
      pub execution_timeout_ms: u64, // 默认 5000ms
      pub max_pattern_length: usize,  // 默认 200 chars
      pub cache_size: usize,          // 默认 1000 patterns
  }
成功指标: 100% 的 Grep 调用在 10s 内返回
```

**F-10.1.3: MemoryStorage 并发安全**
```
描述: 将 DashMap 替换为 tokio::sync 的并发安全结构
验收标准:
  - [ ] MemoryStorage 使用 RwLock<HashMap> 替代 DashMap
  - [ ] 100 并发写入测试无数据竞争 (运行 1000 次无失败)
  - [ ] 读写性能下降 < 20% (对比 DashMap baseline)
  - [ ] 所有 HashMap 写入操作使用事务边界
数据模型变更:
  // Before
  items: Arc<DashMap<String, MemoryItem>>
  // After
  items: Arc<tokio::sync::RwLock<HashMap<String, MemoryItem>>>
成功指标: cargo test -p evif-mem -- 100 并发测试 100% 通过
```

**F-10.1.4: 动态插件 Arc::from_raw 安全验证**
```
描述: 增强插件指针验证，添加 magic header + 长度检查
验收标准:
  - [ ] 插件指针加载时验证 magic bytes (0x45564946 = "EVIF")
  - [ ] 指针非空验证
  - [ ] 结构体大小验证 (不超过声明大小)
  - [ ] 加载后立即执行完整性校验
  - [ ] 任何验证失败返回清晰的错误码 (PLUGIN_LOAD_UNSAFE_PTR)
数据模型:
  const PLUGIN_MAGIC: u32 = 0x45564946;
  const PLUGIN_VERSION: u32 = 1;
  
  pub struct PluginHeader {
      pub magic: u32,           // 必须为 PLUGIN_MAGIC
      pub version: u32,         // 必须为 PLUGIN_VERSION
      pub struct_size: u32,     // 结构体大小
      pub abi_version: u32,     // ABI 版本
  }
成功指标: 恶意构造的插件 .so 文件无法导致内存破坏
```

---

### 21.3 MVP 10.2 — 生产就绪版 (Weeks 2-5)

> **目标**: 完整生产就绪，支持企业客户评估  
> **发布版本**: v0.11.0  
> **目标用户**: 企业 AI 平台团队、AI Startup

#### 功能规格

**F-10.2.1: 统一错误处理 + trace ID**
```
描述: 全系统统一错误格式，每个请求携带 trace_id
验收标准:
  - [ ] 所有 API 响应包含 trace_id header (X-Trace-ID)
  - [ ] 所有日志包含 trace_id 和 span_id
  - [ ] 错误响应格式: {error, code, message, trace_id, timestamp}
  - [ ] 全局 panic handler 将 panic 转为带 trace_id 的错误响应
  - [ ] 错误码体系: 4xxxx = 客户端错误, 5xxxx = 服务端错误
API 设计:
  // 错误响应格式
  {
    "error": "PLUGIN_NOT_FOUND",
    "code": 40401,
    "message": "Plugin 'xyz' not registered",
    "trace_id": "abc123-def456-ghi789",
    "timestamp": "2026-06-01T12:00:00Z",
    "details": {}  // 可选的额外上下文
  }
成功指标: 100% 的 API 响应包含 trace_id
```

**F-10.2.2: TLS + 安全 Header**
```
描述: HTTPS 支持 + RFC 安全响应头
验收标准:
  - [ ] 支持 TLS 1.2 + 1.3 (通过 axum_server 或 nginx 反向代理)
  - [ ]
  响应头包含:
    - Strict-Transport-Security: max-age=31536000; includeSubDomains
    - X-Content-Type-Options: nosniff
    - X-Frame-Options: DENY
    - X-XSS-Protection: 1; mode=block
    - Content-Security-Policy: default-src 'self'
    - Referrer-Policy: strict-origin-when-cross-origin
    - Permissions-Policy: camera=(), microphone=(), geolocation=()
  - [ ] 支持 TLS 终止 (EVIF_REST_TLS_CERT, EVIF_REST_TLS_KEY)
API 设计:
  # 启动参数
  evif-rest --tls-cert /path/to/cert.pem --tls-key /path/to/key.pem
  # 或环境变量
  EVIF_REST_TLS_CERT=/path/to/cert.pem
  EVIF_REST_TLS_KEY=/path/to/key.pem
成功指标: Qualys SSL Labs 评分 A 或以上
```

**F-10.2.3: OpenTelemetry 集成**
```
描述: 统一 traces/metrics/logs 到 OTLP 兼容后端
验收标准:
  - [ ] 每个 HTTP 请求生成 trace，自动传播 trace context
  - [ ] 所有插件操作生成 span
  - [ ] 自定义 metrics: evif_api_requests_total, evif_tool_calls_total,
         evif_token_usage_estimate, evif_error_total
  - [ ] 结构化日志 (JSON) 输出到 stdout，由 OTEL Collector 收集
  - [ ] 支持 OTLP HTTP 导出 (无 gRPC 依赖)
环境变量:
  EVIF_OTEL_ENDPOINT=http://otel-collector:4317
  EVIF_OTEL_SERVICE_NAME=evif-rest
  EVIF_OTEL_EXPORTER=otlp-http
成功指标: 100% 的 API 调用可追踪到具体 span
```

**F-10.2.4: Kubernetes 部署**
```
描述: Helm Chart + HPA + PodDisruptionBudget + 细化健康检查
验收标准:
  - [ ] Helm Chart 安装: helm install evif oci://evif/evif --version 0.11.0
  - [ ] HorizontalPodAutoscaler: CPU > 70% 或内存 > 80% 自动扩容 (2-10 pod)
  - [ ] PodDisruptionBudget: 升级时至少保留 1 个可用 pod
  - [ ] 细化健康检查:
         /health/live → liveness (进程存活)
         /health/ready → readiness (所有插件初始化完成)
         /health/startup → startup (启动探测, 30s 超时)
  - [ ] 资源限制: request CPU 250m / memory 256Mi, limit CPU 2 / memory 2Gi
  - [ ] 持久化存储: PVC for /data and /var/log/evif
API 设计:
  # Helm values 示例 (values.yaml)
  replicaCount: 2
  
  image:
    repository: evif/evif-rest
    tag: "0.11.0"
  
  service:
    type: ClusterIP
    port: 8081
  
  ingress:
    enabled: true
    className: "nginx"
    hosts:
      - host: evif.example.com
        paths: [{path: /, pathType: Prefix}]
    tls:
      - secretName: evif-tls
        hosts: [evif.example.com]
  
  autoscaling:
    enabled: true
    minReplicas: 2
    maxReplicas: 10
    targetCPUUtilizationPercentage: 70
    targetMemoryUtilizationPercentage: 80
  
  env:
    - name: EVIF_REST_TLS_CERT
      valueFrom:
        secretKeyRef:
          name: evif-tls-secret
          key: cert
    - name: EVIF_REST_TLS_KEY
      valueFrom:
        secretKeyRef:
          name: evif-tls-secret
          key: key
成功指标: helm install 在 5 分钟内完成，HPA 在 3 分钟内响应负载变化
```

**F-10.2.5: Grafana Dashboard + 告警**
```
描述: 预置 Dashboard + Prometheus AlertManager 告警规则
验收标准:
  - [ ] Dashboard JSON 模板提供，包含以下面板:
         API QPS (req/s)
         延迟 P50 / P95 / P99
         错误率 (5xx 比例)
         熔断器状态 (每个插件)
         插件健康状态
         Memory 使用量
         活跃连接数
         Token 使用估算
  - [ ] 告警规则:
         HighErrorRate: 5xx 比例 > 5%，持续 5 分钟
         CircuitBreakerOpen: 熔断器 Open 状态 > 1 分钟
         HighLatency: P99 > 1s，持续 5 分钟
         OutOfMemory: memory usage > 90%
  - [ ] 告警通知到 Slack + PagerDuty (通过 AlertManager)
Grafana Dashboard JSON: 见 docs/grafana/evif-dashboard.json
成功指标: 告警在 30 秒内触发，通知在 1 分钟内送达
```

**F-10.2.6: 多租户隔离**
```
描述: 完整的多租户架构，支持租户级别的资源配额和计费
验收标准:
  - [ ] 每个请求携带 X-Tenant-ID header (必填)
  - [ ] 租户间数据完全隔离 (插件状态、Memory、Context、Queue)
  - [ ] 租户级资源配额:
         max_api_calls_per_day: u64
         max_storage_bytes: u64
         max_concurrent_agents: usize
         max_plugins: usize
  - [ ] 配额超限返回 429 Too Many Requests (带 Retry-After)
  - [ ] 租户元数据: name, plan (free/pro/team/enterprise), created_at
  - [ ] 租户管理员可查看本租户使用量 (GET /api/v1/tenants/:id/usage)
  - [ ] 超级管理员可管理所有租户
API 设计:
  # 租户管理
  POST   /api/v1/admin/tenants          # 创建租户 (superadmin)
  GET    /api/v1/admin/tenants          # 列出所有租户 (superadmin)
  GET    /api/v1/admin/tenants/:id      # 租户详情 (superadmin)
  PUT    /api/v1/admin/tenants/:id      # 更新租户 (superadmin)
  DELETE /api/v1/admin/tenants/:id     # 删除租户 (superadmin)
  
  # 租户使用量
  GET    /api/v1/tenants/:id/usage           # 当前租户使用量
  GET    /api/v1/tenants/:id/usage/daily     # 每日使用历史
  GET    /api/v1/tenants/:id/usage/summary   # 使用量汇总 (API calls, storage, agents)
  
  # 配额管理
  GET    /api/v1/tenants/:id/quotas          # 租户配额
  PUT    /api/v1/tenants/:id/quotas          # 更新配额 (superadmin)
  
  # 配额超限响应
  HTTP/1.1 429 Too Many Requests
  X-RateLimit-Limit: 10000
  X-RateLimit-Remaining: 0
  X-RateLimit-Reset: 1719830400
  Retry-After: 86400
  
  {
    "error": "QUOTA_EXCEEDED",
    "code": 42901,
    "message": "Daily API calls quota exceeded (10000/10000)",
    "quota_type": "api_calls_per_day",
    "current": 10000,
    "limit": 10000,
    "reset_at": "2026-06-02T00:00:00Z",
    "upgrade_url": "/billing/upgrade"
  }
数据模型:
  pub struct Tenant {
      pub id: Uuid,
      pub name: String,
      pub plan: TenantPlan,
      pub quotas: TenantQuotas,
      pub created_at: DateTime<Utc>,
      pub updated_at: DateTime<Utc>,
      pub status: TenantStatus,
  }
  
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub enum TenantPlan {
      Free,
      Pro,       // $29/月
      Team,      // $99/月
      Enterprise, // $499/月
  }
  
  pub struct TenantQuotas {
      pub max_api_calls_per_day: u64,
      pub max_storage_bytes: u64,
      pub max_concurrent_agents: usize,
      pub max_plugins: usize,
      pub max_mcp_tools: usize,
      pub rate_limit_rpm: u32,  // requests per minute
  }
  
  pub enum TenantStatus {
      Active,
      Suspended,
      Trial,
      Cancelled,
  }
成功指标: 租户 A 的请求无法访问租户 B 的数据 (100% 隔离)
```

---

###
### 21.4 MVP 10.3 — Plugin Marketplace + 计费系统版 (Weeks 6-8)

> **目标**: Plugin Marketplace MVP + 使用量计费 API，为商业化建立收入引擎  
> **发布版本**: v0.12.0  
> **目标用户**: Plugin 开发者 (收入来源)、企业客户 (采购插件)、EVIF 团队 (平台运营)

#### 功能规格

**F-10.3.1: Plugin Marketplace API**
```
描述: 插件发布、发现、安装、评价的完整 API
验收标准:
  - [ ] 开发者可发布插件 (上传 .so + SKILL.md + metadata.json)
  - [ ] 插件审核流程: DRAFT → PENDING_REVIEW → APPROVED/REJECTED
  - [ ] 用户可搜索插件 (按名称、标签、类别)
  - [ ] 用户可安装/卸载插件 (安装到自己的挂载表)
  - [ ] 插件可设置付费 ($0-$999/月，精度 $0.01)
  - [ ] 付费插件支持免费试用 (7 天)
  - [ ] 开发者可查看插件收入报表
  - [ ] EVIF 管理员可审核/下架插件

插件元数据 metadata.json:
  {
    "id": "notionfs",
    "name": "Notion FS",
    "version": "1.0.0",
    "author": "evif-team",
    "description": "Mount your Notion workspace as a filesystem",
    "category": "productivity",
    "tags": ["notion", "docs", "productivity"],
    "homepage": "https://github.com/evif/notionfs",
    "license": "MIT",
    "pricing": {
      "monthly_cents": 499,  // $4.99/月, 0 = 免费
      "trial_days": 7
    },
    "requirements": {
      "min_evif_version": "0.10.0",
      "env_vars": ["NOTION_API_KEY"],
      "permissions": ["filesystem:read", "network:outbound"]
    },
    "manifest": {
      "so_file": "notionfs.so",
      "skill_file": "SKILL.md",
      "size_bytes": 2097152
    }
  }

API 设计:
  # 开发者端
  POST   /api/v1/marketplace/plugins              # 发布插件 (开发者)
  PUT    /api/v1/marketplace/plugins/:id         # 更新插件 (开发者)
  DELETE /api/v1/marketplace/plugins/:id         # 删除插件 (开发者)
  GET    /api/v1/marketplace/plugins/:id/stats  # 插件统计 (开发者)
  
  # 市场端
  GET    /api/v1/marketplace/plugins            # 搜索插件 (分页)
  GET    /api/v1/marketplace/plugins/:id       # 插件详情
  GET    /api/v1/marketplace/plugins/:id/schema # 插件 Schema
  POST   /api/v1/marketplace/plugins/:id/install # 安装插件
  DELETE /api/v1/marketplace/plugins/:id/install # 卸载插件
  POST   /api/v1/marketplace/plugins/:id/reviews # 评价插件
  GET    /api/v1/marketplace/plugins/:id/reviews # 查看评价
  
  # 管理端
  GET    /api/v1/marketplace/admin/plugins       # 所有插件 (含审核状态)
  PUT    /api/v1/marketplace/admin/plugins/:id/review # 审核插件
  DELETE /api/v1/marketplace/admin/plugins/:id # 下架插件
  
  # 开发者收入
  GET    /api/v1/marketplace/developer/earnings       # 收入汇总
  GET    /api/v1/marketplace/developer/earnings/daily # 每日收入明细
  GET    /api/v1/marketplace/developer/payouts        # 支付历史

  # 搜索参数
  GET /api/v1/marketplace/plugins?category=productivity&tags=notion&sort=popular&page=1&size=20&free_only=false&price_max=500

数据模型:
  pub struct MarketplacePlugin {
      pub id: Uuid,
      pub developer_id: Uuid,
      pub status: PluginStatus,
      pub download_count: u64,
      pub rating_avg: f32,
      pub review_count: u32,
      pub monthly_revenue_cents: u32,
      pub created_at: DateTime<Utc>,
      pub metadata: PluginMetadata,
  }
  
  pub enum PluginStatus {
      Draft,
      PendingReview,
      Approved,
      Rejected,
      Suspended,
      Deprecated,
  }
  
  pub struct PluginReview {
      pub id: Uuid,
      pub user_id: Uuid,
      pub tenant_id: Uuid,
      pub rating: u8,           // 1-5
      pub title: String,
      pub body: String,
      pub verified_purchase: bool,
      pub created_at: DateTime<Utc>,
  }
成功指标: Plugin Marketplace 上线 30 天内 10+ 插件、100+ 安装量
```

**F-10.3.2: 使用量计费 API**
```
描述: 精确计量每个租户的 API 调用量、存储量、Agent 数量
验收标准:
  - [ ] 每个 API 请求计量 (tenant_id, endpoint, timestamp, latency_ms)
  - [ ] 存储使用量计量 (tenant_id, path, size_bytes, timestamp)
  - [ ] 活跃 Agent 计数 (per tenant, per minute)
  - [ ] 按计划 tier 计算配额和超限计费
  - [ ] 使用量数据保留 12 个月
  - [ ] 支持 Webhook 通知配额使用达到 80%/100%
  - [ ] 计费 API 支持导出 CSV/PDF 发票

配额与定价:
  | Plan       | API calls/day | Storage    | Agents | MCP tools | Price       |
  |------------|---------------|------------|--------|-----------|-------------|
  | Free       | 1,000         | 100MB      | 1      | 10        | $0          |
  | Pro        | 100,000       | 10GB       | 10     | 100       | $29/月      |
  | Team       | 1,000,000     | 100GB      | 50     | 500       | $99/月      |
  | Enterprise | unlimited     | 1TB        | unlimited | unlimited | $499/月   |
  
  超限计费 (API calls):
  - Free: 超限后请求被拒绝
  - Pro/Team: 超限部分 $0.001/1000 calls
  - Enterprise: 无超限

API 设计:
  # 使用量查询
  GET    /api/v1/billing/usage                    # 当前计费周期使用量
  GET    /api/v1/billing/usage/history             # 历史使用量 (最多 12 个月)
  GET    /api/v1/billing/usage/by-endpoint         # 按端点分布
  GET    /api/v1/billing/usage/by-day             # 每日使用量
  GET    /api/v1/billing/subscription             # 当前订阅
  GET    /api/v1/billing/invoices                 # 历史发票
  GET    /api/v1/billing/invoices/:id             # 发票详情 (PDF)
  
  # 订阅管理
  POST   /api/v1/billing/subscription             # 创建订阅
  PUT    /api/v1/billing/subscription             # 升级/降级
  DELETE /api/v1/billing/subscription             # 取消订阅
  
  # Webhook 配置
  GET    /api/v1/billing/webhooks                 # Webhook 配置
  POST   /api/v1/billing/webhooks                # 创建 Webhook
  DELETE /api/v1/billing/webhooks/:id           # 删除 Webhook

  # 使用量响应格式
  {
    "period": {
      "start": "2026-06-01T00:00:00Z",
      "end": "2026-06-30T23:59:59Z"
    },
    "plan": "Pro",
    "quotas": {
      "api_calls": {"used": 45000, "limit": 100000, "unit": "calls/day"},
      "storage": {"used": 5368709120, "limit": 10737418240, "unit": "bytes"},
      "agents": {"used": 3, "limit": 10, "unit": "count"}
    },
    "usage": {
      "api_calls": {
        "current": 45000,
        "limit": 100
**F-10.3.3: Admin Dashboard API**
```
描述: 超级管理员和租户管理员的管理 API
验收标准:
  - [ ] 超级管理员可管理所有租户 (CRUD)
  - [ ] 超级管理员可查看全局使用量统计
  - [ ] 超级管理员可审核 Plugin Marketplace 插件
  - [ ] 超级管理员可查看全局错误日志和性能指标
  - [ ] 租户管理员可管理本租户用户
  - [ ] 租户管理员可查看本租户使用量和配额
  - [ ] 管理员操作全部记录审计日志

API 设计:
  # 全局统计 (superadmin)
  GET    /api/v1/admin/stats/overview            # 全局概览
  GET    /api/v1/admin/stats/tenants             # 租户统计
  GET    /api/v1/admin/stats/plugins             # 插件统计
  GET    /api/v1/admin/stats/revenue            # 收入统计
  GET    /api/v1/admin/stats/usage/by-day      # 全局每日使用量
  
  # 用户管理 (租户管理员)
  GET    /api/v1/admin/users                   # 租户用户列表
  POST   /api/v1/admin/users                   # 创建用户
  GET    /api/v1/admin/users/:id              # 用户详情
  PUT    /api/v1/admin/users/:id              # 更新用户
  DELETE /api/v1/admin/users/:id              # 删除用户
  PUT    /api/v1/admin/users/:id/disable      # 禁用用户
  
  # 审计日志 (租户管理员)
  GET    /api/v1/admin/audit                   # 本租户审计日志
  GET    /api/v1/admin/audit/export            # 导出审计日志 (CSV)
  
  # 全局审计日志 (superadmin)
  GET    /api/v1/admin/audit/global            # 全局审计日志
  GET    /api/v1/admin/audit/global/export     # 导出全局审计日志

  # 响应格式
  {
    "stats": {
      "total_tenants": 150,
      "active_tenants": 142,
      "total_api_calls_today": 4500000,
      "total_storage_bytes": 536870912000,
      "revenue_mtd_cents": 285000,
      "active_plugins": 42,
      "marketplace_gmv_cents": 15000
    }
  }
成功指标: Admin Dashboard 覆盖 100% 的管理操作
```

---

### 21.5 MVP 10.4 — 企业特性版 (Weeks 9-12)

> **目标**: 企业级安全特性，支持大企业采购  
> **发布版本**: v0.13.0  
> **目标用户**: Enterprise 客户 (1000+ 人企业)

#### 功能规格

**F-10.4.1: SSO/OIDC + SCIM 用户同步**
```
描述: 企业身份提供商集成 + 用户自动Provisioning
验收标准:
  - [ ] OIDC 登录: 支持 Okta, Azure AD, Google Workspace
  - [ ] SAML 登录: 支持企业 SSO
  - [ ] SCIM 2.0: 自动同步用户目录 (创建/更新/删除)
  - [ ] 角色映射: IdP groups → EVIF roles
  - [ ] JIT (Just-In-Time) provisioning: 首次登录时自动创建用户
  - [ ] 会话管理: 单点登出 (SLO), 会话超时配置
  - [ ] 强制 MFA: 企业可要求所有用户启用 MFA

OIDC 配置 (evif.toml):
  [auth.oidc]
  enabled = true
  issuer = "https://your-okta.com"
  client_id = "${OKTA_CLIENT_ID}"
  client_secret = "${OKTA_CLIENT_SECRET}"
  scopes = ["openid", "profile", "email", "groups"]
  role_mapping = [
    {group = "evif-admins", role = "superadmin"},
    {group = "evif-users", role = "member"}
  ]
  
  [auth.sso]
  enabled = false  # SAML configuration when enabled

SCIM 端点:
  POST   /scim/v2/Users         # 创建用户
  GET    /scim/v2/Users         # 列出用户
  GET    /scim/v2/Users/:id    # 用户详情
  PUT    /scim/v2/Users/:id    # 更新用户
  DELETE /scim/v2/Users/:id    # 删除用户
  POST   /scim/v2/Groups        # 创建组
  GET    /scim/v2/Groups       # 列出组
  PATCH  /scim/v2/Users/:id    # 部分更新
成功指标: Okta/Azure AD 集成在 2 小时内完成配置
```

**F-10.4.2: 不可变审计日志 (WORM)**
```
描述: 审计日志写入不可变存储，支持合规要求
验收标准:
  - [ ] 审计日志追加写入 append-only 存储 (无法修改/删除)
  - [ ] 日志加密存储 (AES-256-GCM)
  - [ ] 每日归档到冷存储 (S3 Glacier 或等价物)
  - [ ] 保留期限: 默认 7 年 (可配置)
  - [ ] 支持完整性校验 (Merkle tree 或 hash chain)
  - [ ] 合规导出: 支持导出为 PDF/CSV 用于审计
  - [ ] 审计事件类型覆盖: 认证、授权、文件操作、插件操作、计费、管理操作

审计事件类型:
  - AuthenticationSuccess/Failed
  - SessionCreated/Terminated/Expired
  - UserCreated/Updated/Deleted/Disabled
  - PermissionGranted/Revoked
  - FileRead/Write/Delete/Mkdir/Rename
  - PluginInstalled/Uninstalled/Updated
  - TenantCreated/Updated/Suspended
  - BillingPlanChanged/InvoiceGenerated
  - ApiKeyCreated/Revoked
  - SsoLogin/Logout/SCIMSync

API 设计:
  GET /api/v1/admin/audit/export
    ?from=2026-01-01&to=2026-06-01
    &types=AuthenticationSuccess,FileRead
    &format=csv|pdf
    &encrypted=true
  
  响应: CSV/PDF 文件下载 (加密压缩包)
  
  审计日志记录格式:
  {
    "id": "audit-uuid",
    "event_type": "FileRead",
    "timestamp": "2026-06-01T12:00:00.123Z",
    "actor": {
      "type": "User",
      "id": "user-uuid",
      "email": "alice@example.com",
      "ip_address": "203.0.113.10",
      "user_agent": "EVIF-CLI/0.10.0"
    },
    "resource": {
      "type": "File",
      "path": "/context/L2/architecture.md",
      "tenant_id": "tenant-uuid"
    },
    "action": "read",
    "outcome": "success",
    "metadata": {
      "size_bytes": 4096,
      "duration_ms": 12
    },
    "integrity": {
      "hash": "sha256:abc123...",
      "prev_hash": "sha256:prev..."
    }
  }
成功指标: 审计日志导出响应时间 < 10s (for 1 年数据)
```

**F-10.4.3: Webhooks 事件系统**
```
描述: Webhook 通知，支持插件和外部系统集成
验收标准:
  - [ ] Webhook 配置: URL, secret, 事件类型过滤, 激活状态
  - [ ] 事件类型: 所有审计事件类型 + 自定义事件
  - [ ] Payload 签名: HMAC-SHA256 (X-EVIF-Signature header)
  - [ ] 重试机制: 指数退避 (1s → 2s → 4s → ... → 10m), 最多 10 次
  - [ ] 事件队列: 持久化到磁盘，防止 Webhook 漏发
  - [ ] 投递日志: 每个 Webhook 投递的历史记录
  - [ ] 签名验证 SDK: 提供多语言 SDK 验证 Webhook 签名

API 设计:
  POST   /api/v1/webhooks               # 创建 Webhook
  GET    /api/v1/webhooks               # 列出 Webhook
  GET    /api/v1/webhooks/:id           # Webhook 详情
  PUT    /api/v1/webhooks/:id           # 更新 Webhook
  DELETE /api/v1/webhooks/:id           # 删除 Webhook
  GET    /api/v1/webhooks/:id/deliveries # 投递历史
  POST   /api/v1/webhooks/:id/test      # 发送测试事件
  
  # Webhook payload 格式
  {
    "id": "evt_uuid",
    "type
**F-10.4.4: 细粒度 RBAC**
```
描述: 资源级权限控制，支持 ABAC 条件
验收标准:
  - [ ] 权限粒度: tenant > namespace > path > plugin
  - [ ] 内置角色: superadmin, tenant_admin, developer, member, guest
  - [ ] 自定义角色: 企业可定义自己的角色组合
  - [ ] ABAC 条件: 基于资源属性的动态权限判断
  - [ ] 权限继承: path 权限继承自 namespace 权限
  - [ ] 即时权限撤销: 权限变更立即生效

权限模型:
  enum Permission {
      // 文件系统
      fs_read,
      fs_write,
      fs_delete,
      fs_admin,
      // 内存
      memory_read,
      memory_write,
      memory_search,
      memory_admin,
      // 上下文
      context_read_l0,
      context_write_l0,
      context_read_l1,
      context_write_l1,
      context_read_l2,
      context_write_l2,
      // 插件
      plugin_install,
      plugin_uninstall,
      plugin_configure,
      plugin_admin,
      // Marketplace
      marketplace_publish,
      marketplace_manage,
      // 管理
      tenant_manage,
      user_manage,
      billing_manage,
      audit_read,
  }
  
  // ABAC 条件示例
  condition: "tenant_id == resource.tenant_id && (role == 'admin' || owner == principal.user_id)"
成功指标: 权限检查延迟 < 1ms，权限验证覆盖率 100%
```

---

### 21.6 MVP 10.5 — MCP 协议增强 + Developer Portal 版 (Weeks 13-16)

> **目标**: MCP 协议完整实现 + 开发者生态基础设施  
> **发布版本**: v0.14.0  
> **目标用户**: Plugin 开发者、企业 AI 平台

#### 功能规格

**F-10.5.1: MCP 协议完整实现**
```
描述: 完整 MCP 协议支持，包括 batching、subscriptions、roots
验收标准:
  - [ ] Protocol 版本协商: 支持 MCP 2024-11-05
  - [ ] Batch 工具调用: 单请求多个工具调用，结果并行处理
  - [ ] Roots 协议: workspace roots 发现和更新通知
  - [ ] Subscriptions: 服务器推送文件变化、工具变化通知
  - [ ] Progress 通知: 长时间操作的进度反馈
  - [ ] Cancelled 通知: 客户端取消请求的优雅处理
  - [ ] 完整的 JSON Schema for all 工具 input/output

MCP Protocol 版本协商:
  // 客户端发送
  {
    "jsonrpc": "2.0",
    "method": "initialize",
    "params": {
      "protocolVersion": "2024-11-05",
      "capabilities": {
        "roots": {"listChanged": true},
        "tools": {},
        "sampling": {}
      },
      "clientInfo": {"name": "evif-cli", "version": "0.14.0"}
    }
  }
  
  // 服务器响应
  {
    "jsonrpc": "2.0",
    "result": {
      "protocolVersion": "2024-11-05",
      "capabilities": {
        "tools": {"changeNotifications": true},
        "resources": {"subscribe": true, "listChanged": true}
      },
      "serverInfo": {"name": "evif-mcp", "version": "0.14.0"}
    }
  }

Batch 工具调用:
  // 单请求多个工具
  {
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "requests": [
        {"name": "evif_ls", "arguments": {"path": "/skills"}},
        {"name": "evif_cat", "arguments": {"path": "/context/L0/current"}}
      ],
      "parallel": true  // 并行执行
    }
  }

  // Batch 响应
  {
    "jsonrpc": "2.0",
    "result": {
      "results": [
        {"index": 0, "content": [{"type": "text", "text": "skill1\nskill2"}]},
        {"index": 1, "content": [{"type": "text", "text": "Implementing feature X"}]}
      ]
    }
  }
成功指标: Batch 调用吞吐量提升 3x (对比串行调用)
```

**F-10.5.2: Developer Portal API**
```
描述: 开发者自助服务门户 API
验收标准:
  - [ ] API Key 管理: 创建/撤销/轮换 API keys
  - [ ] 速率限制: 按 plan 应用不同的 RPM/RPD 限制
  - [ ] OAuth 应用: 创建 OAuth 应用供第三方集成
  - [ ] 使用量仪表盘: 可视化 API 调用、错误率、延迟
  - [ ] 文档访问: OpenAPI spec, SDK 下载
  - [ ] Webhook 管理: 配置事件通知
  - [ ] 开发者社区: 论坛/API status page

API 设计:
  # API Keys
  GET    /api/v1/developer/keys             # 列出 API Keys
  POST   /api/v1/developer/keys            # 创建 API Key
  PUT    /api/v1/developer/keys/:id/rotate # 轮换 API Key
  DELETE /api/v1/developer/keys/:id         # 撤销 API Key
  
  # OAuth 应用 (for 第三方集成)
  GET    /api/v1/developer/oauth-apps      # 列出 OAuth 应用
  POST   /api/v1/developer/oauth-apps    # 创建 OAuth 应用
  PUT    /api/v1/developer/oauth-apps/:id
  DELETE /api/v1/developer/oauth-apps/:id
  GET    /api/v1/developer/oauth-apps/:id/tokens # 应用访问令牌
  
  # 使用量仪表盘
  GET    /api/v1/developer/dashboard/usage      # 使用量总览
  GET    /api/v1/developer/dashboard/errors     # 错误分析
  GET    /api/v1/developer/dashboard/latency    # 延迟分析
  GET    /api/v1/developer/dashboard/top-endpoints # Top 端点
  
  # 文档
  GET    /api/v1/developer/docs/openapi.json   # OpenAPI Spec
  GET    /api/v1/developer/docs/sdk/python     # Python SDK 下载
  GET    /api/v1/developer/docs/sdk/typescript # TypeScript SDK 下载
  GET    /api/v1/developer/status              # API Status Page

  # API Key 响应格式
  {
    "id": "key_uuid",
    "name": "Production Key",
    "key_prefix": "evif_sk_prod_abc1...",
    "created_at": "2026-06-01T00:00:00Z",
    "last_used_at": "2026-06-01T12:00:00Z",
    "rate_limit_rpm": 100,
    "permissions": ["fs_read", "memory_search", "context_read"]
  }
成功指标: 开发者自助注册到第一个 API 调用 < 5 分钟
```

**F-10.5.3: Token 使用量精确计量**
```
描述: 基于实际 LLM token 消耗的精确计费
验收标准:
  - [ ] 每个 MCP 工具调用估算 token 输入/输出
  - [ ] 按工具聚合 token 使用量
  - [ ] 按 Agent 聚合 token 使用量
  - [ ] 每日 token 使用报表
  - [ ] Token 使用趋势图
  - [ ] 与实际 LLM API 成本对标 (支持 OpenAI/Anthropic/Azure 定价)
  - [ ] Agent 成本分析: 每个 Agent 的 token 消耗和成本

Token 估算模型:
  // 工具输出 token 估算
  let input_tokens = estimate_tokens(input_json);
  let output_tokens = estimate_tokens(output_text);
  let cached_tokens = estimate_cached_tokens(output_text); // 输出过滤节省
  let actual_tokens = output_tokens - cached_tokens;
  
  // 成本计算
  let input_cost = input_tokens * model_pricing.input_per_token;
  let output_cost = actual_tokens * model_pricing.output_per_token;
  let total_cost = input_cost + output_cost;

API 设计:
  GET /api/v1/billing/usage/tokens
    ?from=2026-06-01&to=2026-06-30&group_by=agent|tool|day
  
  响应:
  {
    "period": {...},
    "tokens": {
      "input": 15000000,
      "output": 45000000,
      "cached_saved": 12000000,
      "actual_output": 33000000
    },
    "cost_usd": 87.50,
    "breakdown": [
      {"agent_id": "agent-1", "input": 5000000, "output": 15000000, "cost": 29.17},
     
**F-10.5.4: 多区域部署支持**
```
描述: EVIF Cloud SaaS 的多区域架构
验收标准:
  - [ ] 支持区域: us-east-1, eu-west-1, ap-southeast-1
  - [ ] 数据 residency: 用户数据存储在选定区域
  - [ ] 全球负载均衡: 就近路由到最低延迟区域
  - [ ] 区域级故障隔离: 单区域故障不影响其他区域
  - [ ] 跨区域数据同步: 异步复制保证最终一致性
  - [ ] 区域健康检查: 每个区域的 /health/ready 端点
  - [ ] 区域选择 API: 允许企业指定数据存储区域

多区域配置:
  # DNS 路由 (Cloudflare/AWS Route53)
  evif.io → latency-based routing
    us-east-1.evif.io (美区用户)
    eu-west-1.evif.io (欧洲用户)
    ap-southeast-1.evif.io (亚太用户)
  
  # 租户区域配置
  {
    "region": "eu-west-1",  // GDPR 合规
    "data_residency": "EU"
  }
成功指标: 跨区域复制延迟 < 5s, 故障切换 < 30s
```

---

### 21.7 完整 API 端点清单 (MVP 10.x)

```
API 命名空间                   端点数量    MVP 版本
─────────────────────────────────────────────────
/api/v1/files                  8          10.x (已有)
/api/v1/directories            5          10.x (已有)
/api/v1/context                6          10.x (已有)
/api/v1/skills                 4          10.x (已有)
/api/v1/memories               5          10.x (已有)
/api/v1/pipes                  4          10.x (已有)
/api/v1/plugins                6          10.x (已有)
/api/v1/health                 3          10.x (已有)
/api/v1/admin/tenants          5          10.2
/api/v1/admin/users            6          10.2
/api/v1/billing/usage          5          10.3
/api/v1/billing/subscription    3          10.3
/api/v1/billing/webhooks        4          10.3
/api/v1/marketplace/plugins     12         10.3
/api/v1/marketplace/developer  4          10.3
/api/v1/webhooks               7          10.4
/api/v1/developer/keys         5          10.5
/api/v1/developer/dashboard    5          10.5
/scim/v2/Users                 6          10.4
/scim/v2/Groups                4          10.4
─────────────────────────────────────────────────
总计                           ~107       + 57 新增
```

---

## 二十二、定价模型与商业模式

### 22.1 定价层级详细设计

```
┌─────────────────────────────────────────────────────────────────┐
│                    EVIF 定价层级                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  FREE                          PRO ($29/月)                     │
│  ────                          ──────────────                    │
│  ✓ 1,000 API calls/day          ✓ 100,000 API calls/day         │
│  ✓ 1 Agent                     ✓ 10 Agents                      │
│  ✓ 100MB Storage               ✓ 10GB Storage                   │
│  ✓ 10 MCP tools                ✓ 100 MCP tools                 │
│  ✓ 基础 ContextFS              ✓ 完整 ContextFS + SkillFS      │
│  ✓ 社区支持                    ✓ 邮件支持 (48h 响应)            │
│                                  ✓ 使用量仪表盘                  │
│                                  ✓ API Keys                     │
│                                                                  │
│  TEAM ($99/月)                  ENTERPRISE ($499/月)           │
│  ───────────────                ─────────────────────────       │
│  ✓ 1,000,000 API calls/day     ✓ 无限 API calls                │
│  ✓ 50 Agents                   ✓ 无限 Agents                   │
│  ✓ 100GB Storage               ✓ 1TB Storage                    │
│  ✓ 500 MCP tools               ✓ 无限 MCP tools                 │
│  ✓ Plugin Marketplace 访问     ✓ Plugin Marketplace + 发布      │
│  ✓ Webhooks (5 endpoints)      ✓ Webhooks (50 endpoints)        │
│  ✓ 邮件支持 (24h 响应)          ✓ SSO/OIDC + SCIM               │
│  ✓ 使用量仪表盘                ✓ 不可变审计日志                  │
│  ✓ 团队协作功能                ✓ 细粒度 RBAC                    │
│                                  ✓ SLA 99.9%                    │
│                                  ✓ 专属客户经理                   │
│                                  ✓ 电话支持                      │
│                                                                  │
│  CUSTOM (年度合同)                                               │
│  ─────────────────                                               │
│  ✓ Enterprise 全部功能                                          │
│  ✓ 私有部署 (On-premise)                                        │
│  ✓ 白标定制                                                     │
│  ✓ 插件定制开发                                                 │
│  ✓ SLA 99.99%                                                   │
│  ✓ 合规认证 (SOC2, ISO27001, GDPR)                             │
│  ✓ 专属解决方案工程师                                           │
└─────────────────────────────────────────────────────────────────┘
```

### 22.2 Plugin Marketplace 分成模型

```
收入来源                分成比例    开发者收益    EVIF 收益
───────────────────────────────────────────────────────
免费插件安装             0%         0            0
付费插件月度订阅         70%/30%    $3.50/月     $1.50/月
付费插件年度订阅         75%/25%    $42/年       $14/年
Featured 插件推广        60%/40%    $6.00/月     $4.00/月
插件内购 (premium功能)   75%/25%    $7.50/月     $2.50/月

最低保证金: 每个付费插件每月至少 $5 (即使安装量不足)
结算周期: 每月结算，下月 15 日支付
最低支付门槛: $50 (低于门槛结转下月)
```

### 22.3 收入预测

| 月份 | ARR 目标 | 关键里程碑 |
|------|----------|------------|
| Month 6 | $50K | MVP 10.2 发布 → 首批 3 个 Pro 客户 |
| Month 9 | $200K | MVP 10.3 发布 → 10 个 Pro + 5 个 Team |
| Month 12 | $500K | Marketplace 上线 → 50 个付费插件 |
| Month 18 | $1M | 200 个企业客户 + Marketplace 增长 |
| Month 24 | $3M | Series A 准备 + 500 个企业客户 |
| Month 36 | $10M | Series B + 国际化 |

---

## 二十三、MVP 10.x 实现路线图 (完整版)

### 23.1 Sprint 分解

```
Sprint 1 (Week 1): MVP 10.1 — 安全修复
  Day 1-2:   F-10.1.1 路径遍历防护 (P0-1)
  Day 3:     F-10.1.2 Grep 超时保护 (P0-2)
  Day 4-5:   F-10.1.3 MemoryStorage 并发安全 (P0-3)
  Day 5:     F-10.1.4 Arc::from_raw 验证 (P0-4)
  Day 5:     测试 + Code Review + 合并
  Day 5:     发布 v0.10.1

Sprint 2 (Week 2-3): MVP 10.2 Part A
  Day 6-7:   F-10.2.1 统一错误 + trace ID
  Day 8:     F-10.2.2 TLS + 安全 Header
  Day 9-10:  F-10.2.3 OpenTelemetry 集成
  Day 10-11: F-10.2.4 Kubernetes Helm Chart
  Day 12:    F-10.2.5 Grafana + 告警规则
  Day 12-13: F-10.2.6 多租户隔离 (核心)
  Day 14:    测试 + Code Review

Sprint 3 (Week 4-5): MVP 10.2 Part B
  Day 15-17: F-10.2.6 多租户隔离 (API + 配额)
  Day 18-19: F-10.2.7 静态数据加密
  Day 19-20: F-10.2.8 细粒度 RBAC
  Day 20-21: 集成测试 + 性能测试
  Day 22:    Code Review + 合并
  Day 23-24: 文档 + 发布 v0.11.0

Sprint 4 (Week 6-7): MVP 10.3 Part A
  Day 25-27: F-10.3.1 Plugin
  Marketplace API
  Day 28-29: F-10.3.2 使用量计费 API
  Day 30-31: F-10.3.3 Admin Dashboard API
  Day 32-33: 前端 Plugin Marketplace 界面
  Day 34:    测试 + 合并

Sprint 5 (Week 8): MVP 10.3 Part B
  Day 35-36: F-10.3.3 Admin Dashboard (完善)
  Day 37-38: 前端 Admin Dashboard 界面
  Day 39-40: 订阅管理 + 发票生成
  Day 41:    集成测试
  Day 42:    发布 v0.12.0

Sprint 6 (Week 9-10): MVP 10.4 Part A
  Day 43-45: F-10.4.1 SSO/OIDC + SCIM
  Day 46-47: F-10.4.2 不可变审计日志
  Day 48-49: F-10.4.3 Webhooks 事件系统
  Day 50:    测试 + 合并

Sprint 7 (Week 11-12): MVP 10.4 Part B
  Day 51-52: F-10.4.4 细粒度 RBAC (完善)
  Day 53-54: 前端企业特性界面
  Day 55-56: 集成测试 + 安全审计
  Day 57:    发布 v0.13.0

Sprint 8 (Week 13-14): MVP 10.5 Part A
  Day 58-60: F-10.5.1 MCP 协议完整实现
  Day 61-62: F-10.5.2 Developer Portal API
  Day 63:    测试 + 合并

Sprint 9 (Week 15-16): MVP 10.5 Part B
  Day 64-65: F-10.5.3 Token 使用量计量
  Day 66-67: F-10.5.4 多区域部署
  Day 68-70: 前端开发者门户
  Day 71:    全量测试 + 性能基准
  Day 72:    发布 v0.14.0
```

### 23.2 团队规模建议

| 角色 | Sprint 1-3 | Sprint 4-6 | Sprint 7-9 |
|------|-----------|-----------|-----------|
| 后端工程师 | 3 | 4 | 3 |
| 前端工程师 | 1 | 2 | 2 |
| 安全工程师 | 1 (顾问) | 1 | 0 |
| DevOps | 1 | 1 | 1 |
| 产品经理 | 0.5 | 0.5 | 0.5 |
| 设计师 | 0 | 0.5 | 0.5 |
| QA | 1 | 1 | 1 |

**总人月**: ~30 人月 (MVP 10.x 全部完成)

### 23.3 技术债务清理并行任务

与 MVP 开发并行进行的技术债务清理：

```
持续进行:
  □ 每周一: 运行 cargo audit + 修复高危漏洞
  □ Sprint 间隙: cargo clippy --fix 自动修复
  □ Sprint 2+: 用 #[expect(lint)] 标记已知 clippy 警告，逐步清理
  □ Sprint 3+: 迁移 DashMap → tokio::sync (F-10.1.3)
  □ Sprint 4+: 统一所有日志格式为 JSON + trace_id
  □ Sprint 5+: 完善单元测试覆盖率 (目标 80%)
  □ Sprint 6+: 添加模糊测试 (fuzzing)
  □ Sprint 7+: 添加性能基准测试并设置 SLO
  □ Sprint 8+: 完整 E2E 测试覆盖
```

---

## 二十四、Go-To-Market 策略

### 24.1 获客渠道

| 渠道 | 目标 | 策略 | 预期转化 |
|------|------|------|----------|
| GitHub Stars + README | 开发者 | 高质量 README + Demo 视频 | 5% 转化到试用 |
| MCP 协议博客 | AI 开发者 | 技术 SEO + MCP 协议教程 | 10% 转化到试用 |
| Product Hunt 发布 | 早期用户 | 精心准备 launch | 500+ signups |
| Hacker News | 技术社区 | Show HN + 技术深度文章 | 1% 转化到付费 |
| LangChain 社区 | AI 开发者 | 插件集成 + 合作内容 | 15% 转化到付费 |
| 企业销售 (Outbound) | Enterprise | LinkedIn targeting | 3% 转化到付费 |

### 24.2 上市准备清单

**MVP 10.2 发布前 (Week 5)**:
- [ ] Landing page 完整 (evif.io)
- [ ] 定价页面明确 (Pro $29, Team $99, Enterprise $499)
- [ ] 免费版注册流程打通
- [ ] 文档网站 (docs.evif.io) 完整
- [ ] 安全白皮书 v1
- [ ] 隐私政策 + Terms of Service

**MVP 10.3 发布前 (Week 8)**:
- [ ] Plugin Marketplace 上线
- [ ] 开发者文档完整 (docs.evif.io/developers)
- [ ] Plugin SDK + 教程
- [ ] Stripe 支付集成 (订阅 + 插件购买)
- [ ] 发票系统对接

**MVP 10.4 发布前 (Week 12)**:
- [ ] Enterprise 销售材料 (PPT + 案例研究)
- [ ] 安全合规文档 (SOC2 Type I prep)
- [ ] SSO 配置指南 (Okta, Azure AD)
- [ ] 客户成功手册

### 24.3 关键指标 (KPIs)

| KPI | Month 6 目标 | Month 12 目标 | Month 24 目标 |
|-----|-------------|---------------|---------------|
| 活跃用户 | 500 | 5,000 | 50,000 |
| 付费客户 | 10 | 100 | 500 |
| ARR | $50K | $500K | $3M |
| Plugin Marketplace 插件 | 20 | 100 | 500 |
| Plugin 安装量 | 100 | 1,000 | 10,000 |
| NPS | 40 | 50 | 60 |
| API 调用量/天 | 100K | 5M | 50M |
| SLA 可用性 | 99.5% | 99.9% | 99.99% |

---

## 二十五、风险矩阵 (更新版)

### 25.1 新增商业化风险

| 风险 | 概率 | 影响 | 缓解策略 |
|------|------|------|----------|
| Stripe/支付集成复杂度超预期 | 高 | 中 | 使用 Stripe Elements + 订阅管理，减少自研 |
| Plugin Marketplace 冷启动 | 高 | 高 | 种子插件 (自研 20 个) + 开发者激励计划 |
| 企业销售周期过长 (6-12 月) | 中 | 中 | 同时发展 SMB/Pro 客户维持现金流 |
| 大厂 (Anthropic/Microsoft) 推出竞品 | 中 | 高 | 加速 Plugin Marketplace 网络效应 + 差异化 VFS 抽象 |
| 数据泄露影响品牌 | 中 | 极高 | 安全审计 + 渗透测试 + 应急响应预案 |
| 计费系统 bug 导致收入损失 | 中 | 高 | 使用成熟计费系统 (Stripe) + 双人对账 |
| 合规认证 (SOC2/ISO27001) 延期 | 中 | 中 | 提前 6 个月启动认证流程，MVP 10.4 后专注合规 |

### 25.2 技术债务触发条件

| 债务类型 | 触发条件 | 响应策略 |
|----------|----------|----------|
| DashMap 迁移 | P0-3 后仍有 DashMap 使用 | Sprint 3 强制清理 |
| 性能退化 | P99 > 1s 或 QPS 下降 > 20% | 暂停新功能，性能优化冲刺 |
| 测试覆盖率下降 | < 70% | 暂停功能开发，补测试 |
| 安全漏洞 | cargo audit 高危 | 24 小时内修复 |

---

## 二十六、成功标准

### 26.1 MVP 10.2 成功标准

- [ ] 所有 P0 安全问题已修复
- [ ] Qualys SSL Labs 评分 A+
- [ ] 100% API 请求可追踪 (trace_id)
- [ ] Helm Chart 安装 < 5 分钟
- [ ] 告警在 30 秒内触发
- [ ] 100 并发无数据竞争
- [ ] 至少 1 个企业 Pilot 客户签约

### 26.2 MVP 10.3 成功标准

- [ ] Plugin Marketplace 上线 (10+ 插件)
- [ ] Stripe 支付集成完成
- [ ] 计费 API 误差 < 0.1%
- [ ] Admin Dashboard 可管理所有租户
- [ ] 10 个付费插件，$500/月 Marketplace GMV
- [ ] 100 个付费客户 ($2,500/月 ARR)

### 26.
### 26.3 MVP 10.4 成功标准

- [ ] SSO 集成通过 Okta 认证测试
- [ ] SCIM 同步测试 100% 通过
- [ ] 审计日志不可变性验证通过
- [ ] Webhooks 投递成功率 > 99.5%
- [ ] RBAC 权限检查延迟 < 1ms
- [ ] 3 个 Enterprise 客户 Pilot

### 26.4 MVP 10.5 成功标准

- [ ] MCP Batch 调用吞吐量 3x 提升
- [ ] Developer Portal 自助注册 < 5 分钟
- [ ] Token 计量误差 < 1% (对标 OpenAI 实际消耗)
- [ ] 多区域故障切换 < 30 秒
- [ ] 全量 API 端点覆盖 OpenAPI spec
- [ ] 5 个战略合作伙伴 (AI IDE 集成)

---

## 二十七、总结与行动路线

### 27.1 核心发现

1. **EVIF 是 AI Agent 基础设施的真实需求**: Token 优化、Multi-Agent 协调、持久记忆是每个 AI Agent 的刚需
2. **MVP 10.x 是商业化的唯一路径**: 没有生产就绪 → 没有企业客户 → 没有收入
3. **Plugin Marketplace 是护城河**: 40+ 插件 + Marketplace = 网络效应
4. **Rust 实现是性能护城河**: 领先 Python 竞品 5-10x 性能
5. **MCP 协议先发优势**: EVIF 是 MCP 最好的开源实现之一

### 27.2 最关键的 10 个行动

```
P0 (本周):
  ✅ □ 1. 修复路径遍历漏洞 (P0-1) → ✅ 完成
  ✅ □ 2. 添加 Grep 超时保护 (P0-2) → ✅ 完成

Week 2-3:
  ✅ □ 3. 替换 DashMap (P0-3) → ✅ 完成
  □ 4. 统一错误 + trace ID (P1-1) → 3 天
  □ 5. TLS + 安全 Header (P1-2) → 2 天

Week 4-5:
  □ 6. 多租户隔离核心 (P1-6) → 5 天
  □ 7. OpenTelemetry 集成 (P1-4) → 5 天
  □ 8. Kubernetes 部署 (P1-5) → 5 天

Week 6-8:
  □ 9. Plugin Marketplace MVP (B-2) → 10 天
  □ 10. 使用量计费 API (B-3) → 8 天
```

### 27.3 快速价值捕获

在完成 MVP 10.x 的同时，以下功能可以**快速创造商业价值**，应优先实现：

```
快速价值 #1: Token 使用量报告
  → 企业愿意为节省的 LLM 成本付费
  → 立即开始收集 token 估算数据

快速价值 #2: Plugin Marketplace (简化版)
  → 只需 Marketplace API + Stripe
  → 开发者即可开始发布付费插件

快速价值 #3: Pro/Team 订阅
  → 仅需配额系统 + Stripe 订阅
  → 可在 2 周内实现第一个付费客户
```

---

> **文档版本**: v3.2  
> **分析完成**: 2026-06-01  
> **MVP 10.1 完成**: 2026-06-01
> - ✅ P0-1: 路径遍历防护 → `crates/evif-core/src/mount_table.rs`
> - ✅ P0-2: Grep 超时保护 → `crates/evif-rest/src/fs_handlers.rs`  
> - ✅ P0-3: DashMap 竞态修复 → `crates/evif-mem/src/storage/memory.rs`
> - ✅ P0-4: Arc::from_raw 验证 → `crates/evif-core/src/dynamic_loader.rs`
> **MVP 10.2 完成**:
> - ✅ P1-1: 统一错误码 → `evif-core/src/error.rs`
> - ✅ P1-2: TLS + 安全 Header → `evif-rest/src/routes.rs`
> - 🔄 P1-4: OpenTelemetry 集成 (文档已添加)
> - ✅ P1-5: Kubernetes Helm Chart → `deploy/kubernetes/helm/`
>
> **MVP 10.3 进行中**:
> - ✅ B-2: Plugin Marketplace API → `evif-plugins/src/marketplace.rs`, `evif-rest/src/marketplace_handlers.rs`
>   - MarketplaceManager 核心逻辑
>   - REST API 端点 (8个)
>   - 搜索/热门/免费插件查询
>
> **下一步行动**: 1) Stripe 支付集成 2) 多租户隔离核心 3) Grafana 集成
