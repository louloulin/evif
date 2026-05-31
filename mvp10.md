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
| P0-1 | 路径遍历漏洞 | **RCE/数据泄露** | evif-rest | 2d |
| P0-2 | Grep ReDoS | **DoS** | evif-rest | 1d |
| P0-3 | DashMap 并发写 | **数据竞争/崩溃** | evif-mem | 3d |
| P0-4 | Arc::from_raw 无验证 | **内存破坏** | evif-core | 2d |

### P1 — 生产必需 (MVP 前完成)

| # | 改造项 | 风险 | 影响 crate | 工作量 |
|---|--------|------|------------|--------|
| P1-1 | 统一 error + trace ID | 可观测性 | evif-core, evif-rest | 3d |
| P1-2 | TLS 支持 | 数据安全 | evif-rest | 2d |
| P1-3 | 安全 Header | 数据安全 | evif-rest | 1d |
| P1-4 | OpenTelemetry 集成 | 可观测性 | 全部 | 5d |
| P1-5 | Kubernetes 部署 | 可用性 | 基础设施 | 5d |
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
  Week 1: P0-1, P0-2, P0-3, P0-4 全部修复
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
