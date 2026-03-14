# EVIF 后端生产差距分析

日期：2026-03-15
任务：`task-1773524419-ccba`
目的：基于仓库代码与真实运行结果，分析 EVIF 后端距离生产级仍存在的关键差距，为后续 `xc.md` 整改方案提供排序输入。

## 1. 分析范围与证据

本次分析覆盖：

- `crates/evif-rest`
- `crates/evif-auth`
- `crates/evif-mcp`
- `crates/evif-mem`
- 根工作区交付资产与测试信号

使用的证据类型：

- 静态代码阅读：启动链、路由、中间件、memory API、MCP 调用、认证与指标模块
- 仓库资产检查：CI、容器、部署模板、版本与配置面
- 真实运行验证：启动 `cargo run -p evif-rest` 后，使用 `curl` 检查健康检查、未授权写入、CORS 预检、metrics、MCP 风格 payload

## 2. 总体结论

EVIF 后端当前仍处于“研发可运行、模块覆盖广，但生产接线不完整”的阶段，离生产级至少还差一个完整的硬化周期。最核心的问题不是“缺少某一个功能”，而是多个关键能力停留在“crate 已实现、REST 未接入”或“接口已暴露、实现仍为占位/开发态”的状态。

按后端生产成熟度粗评分：

| 维度 | 分值 | 结论 |
|------|------|------|
| 安全与访问控制 | 0.5/5 | `evif-auth` 能力存在，但 REST 实际零认证 |
| 正确性与契约一致性 | 1.5/5 | 公开接口存在占位、版本漂移、跨组件契约不一致 |
| 数据可靠性与可恢复性 | 1/5 | memory API 默认内存态，缺少持久化与启动初始化闭环 |
| 可观测性与运维 | 1/5 | 有 metrics 外壳，但未真正接入请求路径，也无标准导出 |
| 发布工程与质量门禁 | 1/5 | 无 CI/CD、无容器交付物、测试不全绿 |

**后端总体成熟度：约 1.0 到 1.2 / 5。**

## 3. P0 阻塞项

这些问题不解决，不应对外宣称“可生产部署”。

### 3.1 认证能力未接入，所有写接口默认裸奔

证据：

- `crates/evif-rest/src/server.rs` 只挂了 `LoggingMiddleware` 与 `TraceLayer`，没有接入 `AuthMiddleware`
- `crates/evif-rest/src/middleware.rs` 明确写明“当前开发模式关闭认证，生产环境再实现 JWT 或 API key”
- `crates/evif-auth/src/auth.rs` 默认策略是 `Strict`，但整条 REST 请求链没有使用该能力
- 真实验证中，未带任何 `Authorization` 头直接 `POST /api/v1/memories` 成功返回 `200`

影响：

- 任意调用方可直接读写后端状态
- `evif-auth` 的权限、审计、策略能力完全没有形成实际防线
- 一旦暴露公网，风险等级为立即阻断级

### 3.2 CORS 配置是假接线，浏览器预检直接失败

证据：

- `crates/evif-rest/src/server.rs` 的 `ServerConfig` 有 `enable_cors: bool`
- 代码库中没有 `CorsLayer` 使用点，`enable_cors` 仅定义与测试，不参与路由装配
- 真实验证中，对 `POST /api/v1/memories` 发起 `OPTIONS` 预检返回 `405 Method Not Allowed`，响应头也没有 `access-control-allow-origin`

影响：

- 前端一旦跨域部署，浏览器调用会直接失败
- 这意味着“可本地联调”不等于“可生产前后端分离部署”

### 3.3 公共 API 面存在占位实现与错误级回退

证据：

- `crates/evif-rest/src/handlers.rs` 中 `/query`、`/nodes/:id/children` 明确返回 “Graph functionality not implemented”
- 真实验证中，`POST /query` 返回 `500 Internal Server Error`
- `crates/evif-rest/src/memory_handlers.rs` 中 `/api/v1/graph/query` 的四类查询大量使用 placeholder 返回，而非真实 `TemporalGraph`
- `crates/evif-rest/src/fs_handlers.rs` 中 `md5` 摘要直接返回字符串 `md5 not implemented`

影响：

- 已公开的路由并不代表可生产使用
- 对外 API 面与真实可用能力不一致，会直接拉低集成稳定性

### 3.4 后端组件之间已有真实契约断裂

证据：

- `crates/evif-rest/src/memory_handlers.rs` 的 `CreateMemoryRequest` 要求字段 `content`
- `crates/evif-mcp/src/lib.rs` 的 `evif_memorize` 发送字段 `text`
- 真实验证中，用 MCP 风格 payload `{"text":"mcp payload","modality":"conversation"}` 调用 `POST /api/v1/memories` 返回 `422`，报错缺少 `content`

影响：

- MCP 与 REST memory 能力在生产上不可直接互通
- 任何依赖 MCP 记忆工具的上层代理都会在运行期失败

## 4. P1 关键差距

这些问题不一定马上导致接口完全不可用，但会显著阻碍稳定运行与运维接管。

### 4.1 memory API 仍是开发态实现，不具备生产数据面

证据：

- `crates/evif-rest/src/memory_handlers.rs` 的 `MemoryState::new()` 直接创建 `MemoryStorage::new()`
- `create_memory_state()` 无外部配置、无持久化后端选择
- `init_memory_pipelines()` 目前是空实现，未把 `MemorizePipeline` / `RetrievePipeline` 接入启动链
- `search_memories()` 只是对内存中的 `content` / `summary` 做简单包含匹配，并直接 `take(req.vector_k)`

影响：

- 服务重启即丢失 memory 数据
- 记忆检索不具备生产级召回、重排、持久化和索引能力
- API 虽然叫 `vector` / `hybrid`，但后端并未真正执行对应链路

### 4.2 API 契约在单个后端内部都不稳定

证据：

- `crates/evif-rest/src/handlers.rs` 的 `/health` 返回版本 `"1.0.0"`
- 同文件的 `/api/v1/health` 返回 `env!("CARGO_PKG_VERSION")`，当前工作区版本是 `0.1.0`
- 真实验证中两个端点分别返回 `1.0.0` 与 `0.1.0`
- 真实验证中，`create_memory` 响应里的 `type` 为 `knowledge`，但 `search_memories` 返回同类对象时 `type` 为 `Knowledge`

影响：

- 监控、SDK、前端和自动化脚本会遇到同一概念的多种输出格式
- 生产问题排查会被“同接口族不一致”放大

### 4.3 metrics 接口存在，但没有真正连上业务路径

证据：

- `crates/evif-rest/src/metrics_handlers.rs` 定义了 `record_read` / `record_write` / `record_list` / `record_error`
- 全仓库搜索结果显示这些 `record_*` 方法只在定义处出现，没有在业务 handler 中调用
- 真实验证中，已经请求 `/health`、`/api/v1/health`、`/api/v1/memories`、`/query` 后，再访问 `/api/v1/metrics/traffic`，所有计数仍然是 `0`
- `/api/v1/metrics/reset` 还是公开可写接口

影响：

- 当前 metrics 不能反映真实流量
- 运维看到的指标面板会是“看起来有接口，实际上没有信号”
- 无鉴权 reset 接口会破坏审计与故障回放

### 4.4 日志与追踪能力未完成服务级初始化

证据：

- `crates/evif-rest/src/main.rs` 仅创建 `EvifServer::new(ServerConfig::default())` 并 `run()`
- `crates/evif-rest` 没有 `tracing_subscriber` 初始化
- 对比可见，`crates/evif-cli/src/main.rs` 和 `crates/evif-mcp/src/main.rs` 都显式初始化了 tracing

影响：

- `info!` / `warn!` 输出依赖外部 subscriber，默认运行下可观测性弱
- 缺少统一日志格式、过滤级别、trace 上下文与请求链路关联

### 4.5 服务缺少优雅关闭、超时与边界保护

证据：

- `crates/evif-rest/src/server.rs` 使用 `axum::serve(listener, ...)` 直接 await，没有 `with_graceful_shutdown`
- `evif-rest` 代码中未见 `tokio::signal`、请求超时中间件、并发限制、中间件级 request id
- 当前只接入 `LoggingMiddleware` 和 `TraceLayer`

影响：

- 发布、滚动升级、异常退出时难以保证请求排空与状态一致性
- 缺少最基本的 API 保护边界，容易在生产高并发或恶意流量下失稳

### 4.6 默认挂载与插件失败回退偏开发友好，不适合生产

证据：

- `crates/evif-rest/src/server.rs` 默认挂载 `/mem`、`/hello`、`/local`
- `local` 默认根目录为 `/tmp/evif-local`
- 动态插件加载失败时会直接回退到 `MemFsPlugin`

影响：

- 配置错误可能被静默替换成内存文件系统，导致真实故障被掩盖
- 默认 `/tmp` 路径与 demo mount 不适合作为生产基线

## 5. P2 工程化缺口

这些问题主要影响交付效率、回归控制和团队协作质量。

### 5.1 发布工程资产缺失

证据：

- 仓库根目录未发现 `.github/workflows/`
- 未发现根级 `Dockerfile`、`docker-compose.*`、`k8s/`、`helm/`
- 当前唯一可见的 compose 文件是 `crates/evif-mem/dashboards/docker-compose.yml`

影响：

- 文档描述了生产部署，但仓库本身不能直接产出标准交付物
- 无法建立可重复、可审计的发布链

### 5.2 测试信号仍不够硬

证据：

- `cargo test -p evif-rest --quiet` 当前不是全绿，doctest 在 `crates/evif-rest/src/compat_fs.rs` 失败
- `cargo run -p evif-rest` 编译过程中，`evif-rest` 仍产生 41 个 warning，`evif-mem` 仍产生 33 个 warning

影响：

- 当前质量门禁更像“局部可用”，不是“发布前可自动兜底”
- warning 债务会掩盖真正的行为变更与风险信号

### 5.3 能力实现和接入状态被频繁混淆

证据：

- `evif-auth`、`evif-mem` metrics、`evif-mem` telemetry、`TemporalGraph` 都已有 crate 级代码
- 但 `evif-rest` 中大量仍是空接线、placeholder 或开发态回退

影响：

- 团队容易高估系统成熟度
- 文档、代码、可发布资产三者不一致，会直接拖慢后续整改优先级判断

## 6. 对 `xc.md` 的排序输入

后续生产改造计划建议按下面顺序展开：

1. 先补 P0：认证接线、CORS、MCP/REST 契约统一、去掉公开占位接口或补齐真实实现
2. 再补运行面：memory 持久化、pipeline 接线、graceful shutdown、真实 metrics/tracing、配置分层
3. 最后补交付面：CI/CD、容器化、部署模板、发布门禁、warning/doctest 清零

这里的关键原则不是“继续加功能”，而是把现有后端从“模块集合”收敛成“可验证、可部署、可回滚”的单一服务面。

## 7. 一句话结论

EVIF 后端当前最大的生产缺口，不是能力不足，而是**安全、契约、状态、观测、交付五条主链都还没有真正闭环**。
