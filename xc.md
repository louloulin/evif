# EVIF 生产级改造计划

日期：2026-03-15

适用范围：当前仓库主工作区，重点覆盖 `evif-rest`、`evif-mcp`、`evif-auth`、`evif-metrics`、`evif-mem`、`evif-web` 以及部署文档与真实交付物之间的差距。

结论：EVIF 当前更接近“研发可运行 + 功能面广”的工程状态，还不具备直接进入生产环境的条件。按 `.ralph/agent/production-baseline-inventory.md` 的基线评估，当前总体成熟度约为 `1.9 / 5`。阻断上线的核心问题不是“功能缺失”，而是“关键生产能力没有接线成闭环”。

## 1. 当前判断

### 1.1 总体结论

- 不建议以当前主干直接对外提供生产服务。
- 可以支撑本地开发、演示、功能验证和有限内测。
- 若要进入生产，必须先完成认证、持久化、可观测性、发布工程、接口契约和运行时硬化六条主线的整改。

### 1.2 当前成熟度概览

| 维度 | 当前评分 | 目标评分 | 现状判断 |
|------|----------|----------|----------|
| 代码结构与模块划分 | 3/5 | 4/5 | crate 划分清楚，但“代码存在”和“生产可用”之间有明显断层 |
| 构建与测试门禁 | 2/5 | 4/5 | 有较多测试，但缺少 CI，`evif-rest` doctest 仍失败 |
| 安全与认证 | 1/5 | 4/5 | 认证能力存在但未接入 REST 主链路 |
| 数据可靠性 | 1/5 | 4/5 | 关键记忆能力仍默认进程内存态 |
| 可观测性与运维 | 2/5 | 4/5 | 有自定义状态接口，但缺少标准 metrics/tracing/SLO 接入 |
| 发布与部署工程 | 1/5 | 4/5 | 文档写了部署方案，仓库却没有对应 CI、容器和编排资产 |
| API 正确性与契约稳定性 | 2/5 | 4/5 | 存在占位 API、版本不一致、MCP/REST 契约漂移 |
| 前端交付能力 | 3/5 | 4/5 | 可构建、可 E2E，但缺少 lint/unit test/release 流水线 |

### 1.3 关键代码证据

- `crates/evif-rest/src/server.rs`
  - 运行链路只接了 `LoggingMiddleware`，未接入 `AuthMiddleware`。
  - `ServerConfig` 只有 `bind_addr`、`port`、`enable_cors`，但 `enable_cors` 未真正进入中间件装配。
  - 默认挂载 `/mem`、`/hello`、`/local(/tmp/evif-local)`，明显偏开发环境。
  - 动态插件加载失败后直接回退到 `MemFsPlugin`，会掩盖生产配置错误。
- `crates/evif-rest/src/middleware.rs`
  - `AuthMiddleware` 明确写着“开发模式关闭认证，生产环境再实现 JWT 或 API Key”。
- `crates/evif-rest/src/memory_handlers.rs`
  - `create_memory_state()` 默认 `MemoryStorage::new()`，即进程内存存储。
  - `init_memory_pipelines()` 目前是空实现。
  - `search_memories()` 只是 `contains` 级文本匹配，不是生产级检索链路。
- `crates/evif-rest/src/handlers.rs`
  - `/health` 返回硬编码版本 `1.0.0`，`/api/v1/health` 返回 `env!("CARGO_PKG_VERSION")`，当前工作区版本是 `0.1.0`。
  - 图查询相关公开 API 仍返回 “Graph functionality not implemented”。
- `crates/evif-rest/src/routes.rs`
  - `/api/v1/metrics/reset` 作为公开 HTTP 接口暴露。
  - 记忆接口使用 `create_memory_state()` 独立挂载，未与持久化配置体系集成。
- `crates/evif-mcp/src/lib.rs`
  - `evif_memorize` 向 `/api/v1/memories` 提交 `{ text, modality }`，但 REST 侧要求 `{ content, modality, metadata }`，存在契约不一致。
- `evif-web/package.json`
  - 只有 `build`、`typecheck`、`test:e2e`，缺少 `lint`、`test:unit`、发布产物校验脚本。
- `docs/zh/chapter-9-deployment.md`
  - 文档中给出了 Docker、Compose、systemd、Nginx、TLS、回滚等完整示例。
  - 仓库根目录却缺少 `.github/workflows/*`、`Dockerfile`、`docker-compose.yml`、`k8s/`、`helm/` 等真实交付物。

## 2. 距离生产级别的主要问题

## 2.1 P0：当前上线阻断项

### P0-1 认证与授权没有真正接线

问题：
- `evif-auth` 能力存在，但 `evif-rest` 主服务没有把认证中间件串进请求链。
- 当前写接口、挂载接口、插件接口、协作接口默认都处于“无认证保护”的状态。

风险：
- 任意客户端都可能直接写文件、改挂载、重置指标、操作记忆数据。
- 无法满足最基础的 API 安全要求，也无法落地审计和权限边界。

改造要求：
- 统一引入认证入口，至少覆盖所有写操作和管理操作。
- 支持 API Key 或 JWT 二选一作为第一阶段方案。
- 把 `evif-auth` 的策略、权限和审计日志真正挂到 REST 主路径。

验收标准：
- 所有写接口默认 401/403，只有显式认证后才能访问。
- 审计日志可追踪到用户、动作、资源路径、结果。

### P0-2 生产部署资产缺失，文档与仓库脱节

问题：
- 仓库没有真实 CI/CD、容器镜像、部署编排、环境模板。
- 当前部署章节主要是文档级样板，不是仓库级交付物。

风险：
- 无法稳定重复发布，也没有可回滚、可验证、可推广的标准交付路径。

改造要求：
- 先确定唯一主发布路径，建议优先做 `Docker + GitHub Actions + 单机/Compose`，再扩展 K8s。
- 把部署文档中的样板变成仓库内真实资产。

验收标准：
- 新提交自动执行构建、测试、打包。
- 仓库内存在可直接运行的容器化部署方案和环境模板。

### P0-3 记忆能力仍是内存态，重启即丢

问题：
- 记忆接口默认使用 `MemoryStorage::new()`。
- 记忆流水线初始化是空实现，搜索能力停留在简单字符串匹配。

风险：
- 服务重启后数据丢失。
- 即使 API 可用，也不能视为生产可用的数据服务。

改造要求：
- 将 REST 记忆接口接入正式持久化后端。
- 明确 `MemoryStorage` 在生产环境的真实后端选择。
- 建立启动初始化、迁移、备份和恢复流程。

验收标准：
- 服务重启后数据仍可读写。
- 至少具备持久化回归测试、备份恢复演练和异常退出恢复验证。

### P0-4 公开 API 仍有占位实现

问题：
- 图查询相关接口仍直接返回未实现错误。
- 健康检查出口版本信息不一致。
- MCP 与 REST 记忆接口参数格式不一致。

风险：
- 对外接口表面存在，但行为不完整或不稳定，极易引发客户端误判。

改造要求：
- 生产前必须做到“公开即可信”。
- 未完成的接口要么补齐实现，要么从默认路由中下线。
- 修正跨入口契约，统一 OpenAPI/README/客户端实现。

验收标准：
- 对外公开的每个接口都有真实 E2E 用例。
- 所有客户端入口返回结构一致，健康检查版本唯一。

### P0-5 可观测性未达生产要求

问题：
- `evif-rest` 启动路径未统一初始化 `tracing_subscriber`。
- 指标系统主要依赖进程内计数器，且支持 HTTP 重置。
- `evif-metrics` 中已有 Prometheus 实现，但 REST 未接出标准 `/metrics`。

风险：
- 线上故障无法追踪。
- 监控数据不稳定，且可被外部重置。

改造要求：
- 接入结构化日志、请求链 trace/span、Prometheus `/metrics` 暴露。
- 将 `reset` 仅保留为管理接口或彻底移除。
- 增加错误率、耗时、吞吐、挂载健康度、记忆接口成功率等核心指标。

验收标准：
- 单请求可以跨日志和 trace 追踪。
- Prometheus 能抓取到统一指标。
- 高风险管理接口受控或移除。

## 2.2 P1：进入稳定生产前必须补齐的硬化项

### P1-1 运行时中间件链路不足

缺口：
- 未见标准 Request ID、Timeout、Concurrency Limit、Compression、Graceful Shutdown 等接线。
- `enable_cors` 只是配置字段，实际不生效。

要求：
- 在 `evif-rest` 启动层引入统一 HTTP 中间件栈。
- 显式区分开发和生产 CORS 策略。
- 增加优雅停机和启动失败早停。

### P1-2 插件加载与默认挂载策略偏开发模式

缺口：
- 生产环境下动态插件加载失败不应静默回退到 `MemFsPlugin`。
- 默认挂载 `/hello` 与 `/tmp/evif-local` 不应出现在生产基线。

要求：
- 引入严格的 `production` 模式。
- 生产模式下配置错误直接失败启动。
- 默认挂载改成显式配置驱动。

### P1-3 测试资产多，但没有真正形成门禁

缺口：
- `evif-rest` doctest 失败说明“测试通过”说法还不够严谨。
- 缺少 CI 自动化，无法保证新提交持续绿灯。

要求：
- 把 `cargo test --workspace --all-targets`、前端 `typecheck/build/e2e` 变成提交门禁。
- 后续再加 `clippy -D warnings`、格式检查、安全扫描。

### P1-4 前端交付链路不完整

缺口：
- 只有构建、类型检查和 E2E；缺少 lint、组件级测试、bundle 约束。
- 没有环境分层和发布流程定义。

要求：
- 增加 `lint`、关键组件单元测试、生产环境变量模板、构建产物校验。
- 打通与后端联调的真实发布 smoke test。

### P1-5 文档可信度需要重建

缺口：
- 当前部署文档比真实交付物更“完整”，容易给出错误预期。

要求：
- 所有文档都标注状态：已实现 / 示例 / 规划中。
- 以仓库真实资产反向驱动文档，而不是文档先行。

## 2.3 P2：生产化之后的扩展项

- 多环境发布策略：dev/staging/prod 的配置隔离、数据隔离、访问隔离。
- 灾备与高可用：定期备份、恢复时间目标、跨节点部署、故障切换。
- 安全治理：依赖漏洞扫描、密钥轮换、最小权限、审计保留策略。
- 性能治理：压测基线、容量模型、热点路径优化、队列/缓存策略。
- 组织化交付：版本号策略、Release Note、升级指南、回滚演练。

## 3. 改造路线图

## 3.1 Phase 0：冻结生产基线

目标：先把“什么是真实问题、什么是文档样板、什么已经可运行”分层清楚，避免边修边漂移。

工作项：
- 将本文件作为生产改造主计划。
- 维护一份真实资产清单：CI、部署、配置、密钥、监控、备份。
- 明确首个生产目标形态：单实例、容器化、带外部持久化后端。
- 确定生产模式开关：关闭开发默认挂载、禁止弱配置回退。

交付物：
- `xc.md`
- 仓库级生产模式定义文档
- 环境矩阵和配置矩阵

## 3.2 Phase 1：关闭 P0 上线阻断项

目标：让 EVIF 至少具备“可以安全地作为服务暴露出去”的最低条件。

工作项：
- [x] 接入认证中间件，保护所有写和管理操作。
- 统一健康检查与版本输出。
- [x] 修复 MCP/REST 记忆契约。
- 处理图 API：补实现或下线路由。
- 去除或保护 `/api/v1/metrics/reset`。
- 修复 `evif-rest` doctest。

交付物：
- 受保护的 REST API
- 契约一致的 MCP/REST 记忆入口
- 无占位公开 API 的服务版本

阶段验收：
- `cargo test -p evif-rest --quiet` 全绿，含 doctest。
- 未认证访问写接口返回 401/403。
- MCP 调用 `evif_memorize` 可以成功落库并读回。

## 3.3 Phase 2：补齐数据可靠性与运行时硬化

目标：让服务重启、故障、配置错误时仍然可控。

工作项：
- 为记忆子系统接入持久化后端，并增加初始化与迁移流程。
- 调整默认挂载和插件策略，生产模式下严格失败。
- 引入 Request ID、Timeout、Concurrency Limit、Graceful Shutdown。
- 明确 CORS、配置加载优先级、生产环境变量规范。

交付物：
- 持久化的记忆服务
- 生产模式 HTTP 中间件栈
- 环境配置模板

阶段验收：
- 重启后数据不丢失。
- 配置错误时服务拒绝启动而不是静默降级。
- 有重启恢复和异常路径测试。

## 3.4 Phase 3：可观测性、发布工程与运维闭环

目标：让系统可部署、可监控、可追责、可回滚。

工作项：
- 将 `evif-metrics` Prometheus registry 真正接入 `evif-rest`。
- 统一 `tracing` 初始化和日志格式。
- 增加 CI 流水线：Rust、前端、E2E、制品打包。
- 补全容器镜像、Compose 或首版编排资产。
- 增加 staging smoke test 和升级/回滚脚本。

交付物：
- `.github/workflows/*`
- `Dockerfile` / `docker-compose.yml`
- `/metrics` 暴露
- 运维 Runbook

阶段验收：
- 新提交自动跑完整质量门禁。
- 可从仓库直接构建镜像并启动服务。
- 监控系统可抓取关键指标并触发基础告警。

## 3.5 Phase 4：前端生产交付与试运行

目标：让 Web 界面和后端一起形成可对外试运行的产品形态。

工作项：
- 为 `evif-web` 增加 lint、关键组件测试和发布前 smoke test。
- 明确前端环境配置、API 地址、错误上报和降级行为。
- 做一次真实 staging 联调，覆盖搜索、监控、记忆、文件读写、插件管理。

交付物：
- 前端发布质量门禁
- staging 联调记录
- 试运行报告

阶段验收：
- `bun run typecheck`、`bun run build`、`bun run test:e2e` 稳定通过。
- 前后端关键路径可在 staging 环境连续验证。

## 4. 建议的优先级拆分

### 4.1 第一批必须立即做

1. [x] 认证中间件接线
2. [x] MCP/REST 记忆契约修复
3. `evif-rest` doctest 修复
4. 统一健康检查和版本输出
5. 去掉公开占位 API 或补真实实现

状态更新（2026-03-15）：
- `evif-rest` 现已把生产认证接到主服务请求链：默认严格模式保护写路径与管理路径，开发环境若需显式关闭可设置 `EVIF_REST_AUTH_MODE=disabled`。
- 第一阶段凭据方案使用 API key：`EVIF_REST_WRITE_API_KEYS` 可访问写接口，`EVIF_REST_ADMIN_API_KEYS` 可访问管理接口，支持 `x-api-key` 与 `Authorization: Bearer ...`。
- REST 认证中间件已调用 `evif-auth` 的 `AuthPolicy`、`Permission` 与 `AuditLogManager`，审计事件会记录动作、资源路径和结果。
- `evif-mcp` 的 `evif_memorize` 现已对齐 REST `content/modality/metadata` 契约，并保留 `text` 兼容别名。
- `evif-mcp` 的 `evif_retrieve` 现已发送 REST 侧实际消费的 `vector_k/llm_top_n` 字段。
- `evif-rest` 为旧客户端补充了 `text`、`k`、`top_n` 反序列化兼容层，并新增 focused tests 覆盖。

### 4.2 第二批必须跟进

1. 记忆持久化
2. 生产模式配置和默认挂载清理
3. Prometheus `/metrics` 与 tracing 接线
4. CI/CD 与容器化发布

### 4.3 第三批用于拉齐稳定交付

1. 前端 lint/unit test
2. staging 环境和 smoke test
3. 备份恢复、升级回滚、压测基线

## 5. 建议的验收门槛

达到“可进入生产试运行”的最低标准，建议至少满足以下门槛：

### 5.1 后端门槛

- `cargo test --workspace --all-targets` 通过
- `cargo clippy --workspace --all-targets -- -D warnings` 通过
- 关键 REST E2E 测试通过：健康检查、文件操作、记忆写入/查询、插件管理
- 未认证访问被阻断
- 服务重启后关键数据仍可读取

### 5.2 前端门槛

- `bun run typecheck` 通过
- `bun run build` 通过
- `bun run test:e2e` 通过
- 新增 `bun run lint` 后保持绿灯

### 5.3 运维门槛

- 仓库内有可复现部署方案
- 指标、日志、追踪至少完成其中两项并接入同一环境
- 有 staging 环境和一次完整回滚演练记录

## 6. 最终判断

EVIF 不是“离生产只差一点文档”，而是“离生产还差最后一公里的关键工程化闭环”。好消息是核心代码面和模块面已经具备改造基础，问题集中在几个清晰的边界：

- 安全链路没有真正启用
- 运行时状态还不可靠
- 监控和发布工程没有落地
- 对外接口仍有占位和契约漂移
- 文档领先于真实交付物

建议按本计划先完成 P0 和 P1，再进入小范围 staging 试运行；在这些项目完成之前，不应把当前仓库描述为“生产级系统”。
