# EVIF 生产基线盘点

日期：2026-03-15
任务：`task-1773524419-d3c4`
目的：建立 EVIF 当前代码库的生产级基线，不做整改方案，只沉淀事实、成熟度信号和后续分析框架。

## 1. 盘点方法

- 静态盘点：`Cargo.toml`、`evif-web/package.json`、`tests/`、`docs/`、`crates/evif-rest`、`crates/evif-auth`、`crates/evif-mem`、`crates/evif-mcp`
- 结构搜索：CI、Docker/K8s、TLS、认证、指标、测试、TODO/未实现路径
- 轻量验证：
  - `cargo test -p evif-rest --quiet`
  - `cargo test -p evif-rest --lib --tests --quiet`
  - `cargo test -p evif-auth --quiet`
  - `cd evif-web && bun run typecheck`

## 2. 工作区表面清单

### 2.1 代码与交付面

- Rust 工作区成员覆盖 18 个核心 crate，外加 `tests/e2e`、`tests/common`、`tests/cli` 等测试包。
- 实际交付面同时包含：
  - 后端：`evif-rest`、`evif-cli`、`evif-fuse`、`evif-mcp`、`evif-grpc`
  - 核心能力：`evif-core`、`evif-vfs`、`evif-graph`、`evif-storage`、`evif-auth`、`evif-metrics`
  - 记忆子系统：`evif-mem`、`evif-mem-ts`、`evif-mem-py`
  - 前端：`evif-web`
- 文档面较大：`docs/` 下 31 个文件，含中英双语章节。
- 测试面较广：Rust 测试标记分布在 149 个文件，前端 `evif-web/e2e` 下有 23 个 Playwright 规格文件。

### 2.2 运行入口

- REST 入口：`crates/evif-rest/src/main.rs`
- CLI 入口：`crates/evif-cli/src/main.rs`
- MCP 入口：`crates/evif-mcp/src/main.rs`
- Web 前端：`evif-web/package.json`
- FUSE 挂载二进制：`crates/evif-fuse/src/bin/evif-fuse-mount.rs`

## 3. 生产成熟度评分尺

| 分值 | 含义 |
|------|------|
| 0 | 仅概念或文档存在，运行面缺失 |
| 1 | 有代码骨架，但缺少关键接线或交付物 |
| 2 | 本地可运行，生产前置条件明显不足 |
| 3 | 主要能力可用，但仍缺少一到两个生产关键面 |
| 4 | 生产要件大体齐备，仅剩收尾和硬化 |
| 5 | 具备稳定发布、运维和回滚能力 |

## 4. 当前基线评分

| 维度 | 分值 | 现状摘要 |
|------|------|----------|
| 代码结构与模块边界 | 3 | crate 划分清晰，交付面完整，但存在“能力存在、接线不完整”的情况 |
| 构建与测试信号 | 2 | 测试资产丰富，但 `evif-rest` 当前 doctest 失败，缺少 CI 自动门禁 |
| 部署与发布工程 | 1 | 文档写了 Docker/K8s/Systemd，但仓库根目录没有真实 CI/Docker/K8s 交付物 |
| 安全与认证 | 1 | `evif-auth` crate 存在，但 REST 未真正接入认证中间件 |
| 可观测性与运维 | 2 | 有自定义 metrics/status 接口，但无 REST 侧 tracing 初始化、无标准 `/metrics` 暴露 |
| 数据持久化与状态可靠性 | 1 | 记忆接口默认走内存存储，多个关键面仍以进程内状态为主 |
| API 一致性与运行正确性 | 2 | 主要 REST 面可跑，但存在硬编码版本、未实现查询接口、文档与代码漂移 |
| 前端可交付性 | 3 | 可构建、可 typecheck、有 E2E；但缺少 lint/unit test/release packaging |
| 文档与操作手册可信度 | 2 | 文档覆盖广，但与仓库实际部署资产存在明显偏差 |

**当前总体基线：1.9 / 5**

结论：EVIF 更接近“研发可运行、功能面广”的工程样态，还没有形成生产级系统所需的发布、认证、可观测性、持久化和交付闭环。

## 5. 核心事实盘点

### 5.1 仓库与发布工程

- 工作区版本仍为 `0.1.0`，整体仍处早期版本期。
- 仓库根目录未发现 `.github/workflows/*`，没有可见的 CI/CD 流水线定义。
- 根目录未发现 `Dockerfile`、`docker-compose.yml`、`k8s/`、`helm/` 等真实部署资产。
- 仓库中唯一可见的 compose 文件位于 `crates/evif-mem/dashboards/docker-compose.yml`，更像监控样例而不是整站部署方案。

### 5.2 REST 运行面

- `evif-rest` 启动路径仅创建 `EvifServer::new(ServerConfig::default())` 并 `run()`。
- `ServerConfig` 目前只有 `bind_addr`、`port`、`enable_cors` 三个字段；其中 `enable_cors` 在路由装配中未实际使用。
- REST 实际只挂载了 `LoggingMiddleware`；`AuthMiddleware` 没有接入请求链。
- `AuthMiddleware` 本身明确写着“当前为开发模式关闭认证，生产环境再实现 JWT 或 API Key”。
- 根健康检查 `/health` 返回硬编码版本 `1.0.0`，而 `/api/v1/health` 返回 `env!("CARGO_PKG_VERSION")`，当前工作区版本是 `0.1.0`，两个出口不一致。
- `handlers.rs` 中 `/query` 与 `/nodes/:id/children` 仍直接返回 “Graph functionality not implemented”，说明部分公开 API 面仍是占位。

### 5.3 安全与认证

- `evif-auth` 已有 `AuthManager`、`AuthPolicy`、审计日志等基础能力，但未进入 REST 主路径。
- REST 侧未见 API Key、JWT、Bearer、RBAC 的真实校验逻辑。
- `evif-rest` 代码中未见 TLS/HTTPS 监听配置；部署文档中的 TLS、证书、systemd 加固属于文档层，而不是仓库内可执行资产。

### 5.4 可观测性与日志

- `evif-rest` 暴露了 `/api/v1/metrics/traffic`、`/operations`、`/status`、`/reset`。
- 这些指标本质上是进程内 `AtomicU64` 计数器，重启丢失，且允许通过 HTTP reset。
- `evif-metrics` crate 有 Prometheus registry 实现，但 REST 侧没有实际 `/metrics` text endpoint 接线。
- `evif-rest` 主程序未初始化 `tracing_subscriber`；相比之下 CLI 和 MCP 二进制都做了 tracing 初始化。

### 5.5 数据与状态可靠性

- `memory_handlers.rs` 中 `create_memory_state()` 默认直接构造 `MemoryStorage::new()`，即进程内存存储。
- `init_memory_pipelines()` 只是可选空实现，当前并没有把完整记忆流水线接到 REST 启动链路。
- `create_memory` 只是把 `content` 直接保存为 `MemoryItem`；`search_memories` 只是做简单文本包含匹配，不是生产级检索链路。
- `server.rs` 中动态插件加载失败会回退到 `MemFsPlugin`，这对可用性友好，但会掩盖真实配置/加载错误并放大误挂载风险。
- 默认挂载仍包含 `/mem`、`/hello` 和基于 `/tmp/evif-local` 的 `local`，更偏开发体验而不是生产基线。

### 5.6 前端交付面

- `evif-web` 有 `dev`、`build`、`typecheck`、`test:e2e` 脚本，说明可构建性较好。
- 前端未见 `lint`、`test:unit`、`storybook` 或类似发布质量门禁。
- 当前更像“IDE 风格应用原型 + 丰富 Playwright 验证”，不是已经封装好生产发布流程的前端产品。

### 5.7 文档可信度

- `docs/zh/chapter-9-deployment.md` 和英文部署章提供了相当完整的生产部署叙述。
- 但仓库实际缺少与文档相对应的部署资产，说明当前文档部分是“设计/示例级”，还不是“仓库即交付物”。
- 这意味着后续分析必须区分“代码已实现”“文档已描述”“仓库可直接发布”三个层级，不能把它们等同。

## 6. 当前验证结果

### 6.1 通过项

- `cargo test -p evif-rest --lib --tests --quiet`
  - 结果：27 个 `evif-rest` 测试通过，依赖链相关测试也通过
- `cargo test -p evif-auth --quiet`
  - 结果：15 个测试通过
- `cd evif-web && bun run typecheck`
  - 结果：通过

### 6.2 失败项

- `cargo test -p evif-rest --quiet`
  - 结果：单元测试通过，但 doctest 失败
  - 失败位置：`crates/evif-rest/src/compat_fs.rs`
  - 失败原因：文档注释中的中文树形示例被 rustdoc 当作 Rust 代码编译
  - 含义：当前“后端测试通过”的表述不能直接等同于 crate 级测试全绿

## 7. 对后续任务的输入

后续 `Backend production gap analysis` 应重点展开：

- REST 接线缺口：认证、CORS、TLS、tracing、标准 metrics 暴露
- 运行正确性缺口：未实现 API、版本出口不一致、doctest 失败
- 数据可靠性缺口：memory API 的内存态实现、插件加载失败回退策略、默认开发挂载
- 交付工程缺口：CI/CD、容器化、部署模板、环境分层配置

后续 `Write xc.md production-readiness plan` 应以本文件为事实基线，不要直接沿用现有文档中的“生产部署”表述。
