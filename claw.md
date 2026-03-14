# EVIF 与 OpenClaw 集成分析

> 第一阶段先完成 EVIF 侧代码盘点，明确这个仓库现在已经具备哪些稳定接入面、哪些能力仍然是占位实现，以及后续和 OpenClaw 结合时应优先走哪一层。

## 1. 先给结论

从当前代码看，EVIF 已经具备 3 条适合 OpenClaw 的主接入路径：

1. `REST API`：最适合把 EVIF 作为独立后端服务接入 OpenClaw。
2. `MCP Server`：最适合把 EVIF 暴露成 OpenClaw 可调用的工具集。
3. `FUSE / 文件系统挂载`：最适合把 EVIF 伪装成 OpenClaw 可直接读写的本地文件层。

如果目标是尽快完成一版稳定集成，推荐优先级如下：

1. 先接 `evif-rest`，把文件、搜索、挂载、健康检查和记忆接口跑通。
2. 再接 `evif-mcp`，把 EVIF 变成 OpenClaw 的工具能力面。
3. 需要“像本地目录一样访问”时，再补 `evif-fuse`。

当前不建议把第一版方案建立在“图查询能力”之上，因为代码里这部分仍有明显占位实现。

## 2. 代码库整体结构

根工作区 `Cargo.toml` 把 EVIF 划分成几个层次：

- 核心层：`evif-core`、`evif-vfs`、`evif-storage`、`evif-protocol`
- 访问层：`evif-rest`、`evif-cli`、`evif-client`、`evif-fuse`、`evif-mcp`
- 图与记忆层：`evif-graph`、`evif-mem`
- 运行与扩展：`evif-runtime`、`evif-plugins`、`example-dynamic-plugin`
- 前端：`evif-web`

按“和 OpenClaw 如何结合”来理解，这个仓库不是单一服务，而是一个分层平台：

- `evif-core` 定义统一插件接口、挂载表、句柄管理、动态加载等底座。
- `evif-rest` 把底座暴露成 HTTP 服务，是最自然的远程集成入口。
- `evif-mcp` 又把 REST 包装成 MCP 工具，方便 Agent 系统调用。
- `evif-fuse` 可以把 EVIF 挂到本地目录，使外部系统用“文件读写”而不是“API 调用”的方式接入。
- `evif-mem` 额外提供 AI 记忆平台，可作为 OpenClaw 的长期记忆后端。
- `evif-web` 主要是操作台和可视化界面，不是主集成面，但很适合做观测与人工干预。

## 3. 对 OpenClaw 最重要的 6 个接入面

### 3.1 REST：最稳的服务化接入层

`crates/evif-rest/src/routes.rs` 已经把主要接口组织出来，覆盖：

- 健康检查：`/health`、`/api/v1/health`
- 文件与目录：`/api/v1/files`、`/api/v1/directories`、`/api/v1/stat`
- 搜索与重命名：`/api/v1/grep`、`/api/v1/rename`
- 挂载管理：`/api/v1/mounts`、`/api/v1/mount`、`/api/v1/unmount`
- 插件管理：动态插件、WASM 插件、插件状态与重载
- 监控指标：流量、操作统计、系统状态
- 句柄操作：`/api/v1/handles/*`
- 记忆接口：`/api/v1/memories*`、`/api/v1/categories*`、`/api/v1/graph/query`

`crates/evif-rest/src/server.rs` 说明服务启动时会先创建 `RadixMountTable`，再从 `EVIF_CONFIG`、`EVIF_MOUNTS` 或默认配置加载挂载点。这意味着 OpenClaw 有两种接法：

- 把 EVIF 当成一个固定服务，通过 REST 调用
- 在 OpenClaw 启动编排里动态注入挂载配置，让不同环境挂载不同插件

结论：如果 OpenClaw 本身是服务编排型 Agent runtime，`REST` 是第一优先级接入层。

### 3.2 MCP：最适合 Agent 工具调用

`crates/evif-mcp/src/main.rs` 和 `crates/evif-mcp/src/lib.rs` 表明 EVIF 已经有独立 MCP 服务器，采用 `stdio` 传输。

它暴露的能力至少包括：

- 文件工具：`evif_ls`、`evif_cat`、`evif_write`、`evif_rm`、`evif_mv`、`evif_cp`、`evif_stat`
- 挂载工具：`evif_mount`、`evif_unmount`、`evif_mounts`
- 搜索与健康检查：`evif_grep`、`evif_health`
- Handle 工具：`evif_open_handle`、`evif_close_handle`
- 记忆工具：`evif_memorize`、`evif_retrieve`

这层的价值在于：如果 OpenClaw 支持 MCP 或可以很容易桥接 MCP，那么几乎不需要重新设计“工具调用协议”，直接复用现有 EVIF 工具即可。

结论：如果 OpenClaw 的核心是“Agent 调工具”，`MCP` 是最省改造成本的接法。

### 3.3 FUSE：最适合文件原生工作流

`crates/evif-fuse/src/lib.rs` 和 `crates/evif-fuse/Cargo.toml` 显示 EVIF 已有完整 FUSE 绑定，底层使用 `fuser`，可以把 EVIF 挂载为本地文件系统。

这对 OpenClaw 的价值是：

- 如果 OpenClaw 某些能力默认假设“本地目录 + 文件读写”，可以直接把 EVIF 伪装成这类目录。
- 这条路径对遗留脚本、shell 工具、编辑器生态特别友好。
- 当 OpenClaw 更习惯通过路径而不是通过 API 访问资源时，FUSE 比二次封装 SDK 更自然。

结论：如果 OpenClaw 的工作流天然依赖文件系统语义，`FUSE` 是高价值补充入口，但不应先于 REST。

### 3.4 记忆平台：可以直接充当长期记忆后端

`crates/evif-mem/src/lib.rs` 和 `crates/evif-mem/Cargo.toml` 说明 `evif-mem` 已经是一个独立记忆平台，核心模块包括：

- `storage`：内存、SQLite、Postgres 等存储扩展
- `vector`：向量检索
- `pipeline`：记忆化与检索流水线
- `proactive`：主动监控与意图预测
- `workflow`：工作流系统
- `plugin`：将记忆平台暴露成 EVIF 插件

更关键的是 `crates/evif-mem/src/plugin/plugin.rs` 已经实现 `MemPlugin`，把记忆系统暴露成文件系统视图，并使用 `Markdown + YAML Frontmatter` 表示记忆。这很适合 OpenClaw，因为：

- Agent 可通过文件方式查看或编辑记忆
- 记忆内容天然可读，便于人类审查
- 可以同时保留结构化字段与正文语义

结论：如果 OpenClaw 需要持久记忆，EVIF 不只是能“存文件”，而是已经具备可复用的记忆后端雏形。

### 3.5 动态插件与注册表：适合做 OpenClaw 专用桥接器

`crates/evif-core/src/plugin.rs` 定义了统一 `EvifPlugin` trait，覆盖创建、读取、写入、列目录、重命名、删除、符号链接等标准文件能力。

`crates/evif-core/src/dynamic_loader.rs` 和 `crates/evif-core/src/plugin_registry.rs` 说明 EVIF 支持：

- 从 `.so` / `.dylib` / `.dll` 动态加载插件
- 追踪插件状态、失败次数、挂载路径和生命周期
- 在 REST 层查询插件状态、热重载、挂载与卸载

这意味着后续完全可以写一个“OpenClaw 适配插件”：

- 一端说 OpenClaw 的协议、任务、消息或数据源
- 另一端实现 `EvifPlugin`
- 最终被挂载到 EVIF 的某个路径下

结论：如果后续要做深度耦合，不必硬改 EVIF 核心，更合理的方式是增加 OpenClaw 专用插件。

### 3.6 Web UI：适合观测，不适合做主集成

`evif-web/README.md` 显示前端定位是 VS Code 风格的管理界面，依赖 REST 和 WebSocket 提供：

- 文件浏览
- 编辑器
- 终端
- 监控面板
- 搜索与上传
- 记忆视图

这层对 OpenClaw 的意义主要是：

- 给人工运维一个观察与调试界面
- 用于验证 OpenClaw 写入 EVIF 之后的状态
- 用于监控流量、操作、记忆内容

结论：`evif-web` 更像操作台，不是 OpenClaw 的首选集成接口。

## 4. 已确认的集成风险与代码空洞

### 4.1 图查询能力不能作为第一阶段依赖

`crates/evif-rest/src/handlers.rs` 里，通用图查询接口直接返回：

- `Graph functionality not implemented.`

`crates/evif-rest/src/memory_handlers.rs` 里，记忆图查询虽然暴露了 `/api/v1/graph/query`，但 `causal_chain`、`timeline`、`temporal_bfs`、`temporal_path` 仍然是占位返回，注释也明确写着 “Full implementation would use evif-graph TemporalGraph”。

结论：OpenClaw 第一阶段不能把“时间图谱推理”当作稳定依赖，只能先把它视作后续增强项。

### 4.2 记忆 REST 现在主要依赖基础存储，不是完整流水线

`crates/evif-rest/src/memory_handlers.rs` 的 `init_memory_pipelines()` 注释明确写着：

- pipeline 初始化是可选的
- 当前 handlers 主要使用基础存储即可工作

这代表当前 REST 记忆接口已经能用，但并不是把 `evif-mem` 里完整的 LLM / 向量 / 主动式流水线都自动接起来了。

结论：如果 OpenClaw 只需要“可用的持久记忆接口”，现在已经够做第一版；如果要更高级的自动提炼和演化，还需要后续接线。

### 4.3 MCP 记忆接口与 REST 记忆接口存在契约不一致

这是当前最值得优先修复的一处问题。

在 `crates/evif-rest/src/memory_handlers.rs` 中，创建记忆请求结构是：

- `CreateMemoryRequest { content, modality, metadata }`

但在 `crates/evif-mcp/src/lib.rs` 中，`evif_memorize` 调 REST 时发送的是：

- `{ "text": ..., "modality": ... }`

也就是说：

- MCP 工具 schema 要求字段名 `text`
- REST 接口实际要求字段名 `content`

如果 OpenClaw 通过 MCP 写入记忆，这里很可能直接失败。

结论：后续正式做 OpenClaw 集成前，必须先修这处 MCP/REST 契约偏差。

## 5. 面向 OpenClaw 的 EVIF 侧建议

在还没有深入研究 OpenClaw 代码之前，仅从 EVIF 现状出发，最合理的接入顺序是：

1. 以 `evif-rest` 作为主后端
2. 以 `evif-mcp` 作为 Agent 工具面
3. 以 `evif-mem` 作为长期记忆层
4. 以 `evif-fuse` 作为文件语义增强层
5. 以动态插件系统作为后续深度集成手段

换句话说，OpenClaw 不需要一开始就“嵌进 EVIF 内核”，而是可以按下面的演进路径走：

1. 服务调用：OpenClaw 调 EVIF REST
2. 工具调用：OpenClaw 调 EVIF MCP
3. 文件映射：OpenClaw 读写 EVIF FUSE 挂载目录
4. 深度桥接：开发 OpenClaw 专用 EVIF 插件

## 6. 本阶段结论

第一阶段盘点的核心判断是：

- EVIF 已具备与 OpenClaw 结合的基础设施，且以 `REST + MCP + FUSE` 三层最为清晰。
- `evif-mem` 让 EVIF 不只是文件层，也具备长期记忆层潜力。
- 真正阻碍第一版集成的，不是“缺入口”，而是“某些高级能力尚未完全接线”。
- 因此下一阶段应转向 OpenClaw 侧研究：确认它的运行模型、工具协议、消息入口、记忆机制，再把这些需求映射到 EVIF 的现有能力上。
