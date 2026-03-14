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

`crates/evif-rest/src/server.rs` 说明服务启动时会先创建
`RadixMountTable`，再从 `EVIF_CONFIG`、`EVIF_MOUNTS`
或默认配置加载挂载点。这意味着 OpenClaw 有两种接法：

- 把 EVIF 当成一个固定服务，通过 REST 调用
- 在 OpenClaw 启动编排里动态注入挂载配置，让不同环境挂载不同插件

结论：如果 OpenClaw 本身是服务编排型 Agent runtime，`REST` 是第一优先级接入层。

### 3.2 MCP：最适合 Agent 工具调用

`crates/evif-mcp/src/main.rs` 和 `crates/evif-mcp/src/lib.rs`
表明 EVIF 已经有独立 MCP 服务器，采用 `stdio` 传输。

它暴露的能力至少包括：

- 文件工具：`evif_ls`、`evif_cat`、`evif_write`、`evif_rm`、`evif_mv`、`evif_cp`、`evif_stat`
- 挂载工具：`evif_mount`、`evif_unmount`、`evif_mounts`
- 搜索与健康检查：`evif_grep`、`evif_health`
- Handle 工具：`evif_open_handle`、`evif_close_handle`
- 记忆工具：`evif_memorize`、`evif_retrieve`

这层的价值在于：如果 OpenClaw 支持 MCP 或可以很容易桥接 MCP，那么几乎不需要重新设计“工具调用协议”，直接复用现有 EVIF 工具即可。

结论：如果 OpenClaw 的核心是“Agent 调工具”，`MCP` 是最省改造成本的接法。

### 3.3 FUSE：最适合文件原生工作流

`crates/evif-fuse/src/lib.rs` 和 `crates/evif-fuse/Cargo.toml`
显示 EVIF 已有完整 FUSE 绑定，底层使用 `fuser`，
可以把 EVIF 挂载为本地文件系统。

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

更关键的是 `crates/evif-mem/src/plugin/plugin.rs`
已经实现 `MemPlugin`，把记忆系统暴露成文件系统视图，
并使用 `Markdown + YAML Frontmatter` 表示记忆。
这很适合 OpenClaw，因为：

- Agent 可通过文件方式查看或编辑记忆
- 记忆内容天然可读，便于人类审查
- 可以同时保留结构化字段与正文语义

结论：如果 OpenClaw 需要持久记忆，EVIF 不只是能“存文件”，而是已经具备可复用的记忆后端雏形。

### 3.5 动态插件与注册表：适合做 OpenClaw 专用桥接器

`crates/evif-core/src/plugin.rs` 定义了统一 `EvifPlugin` trait，覆盖创建、读取、写入、列目录、重命名、删除、符号链接等标准文件能力。

`crates/evif-core/src/dynamic_loader.rs` 和
`crates/evif-core/src/plugin_registry.rs`
说明 EVIF 支持：

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

`crates/evif-rest/src/memory_handlers.rs` 里，
记忆图查询虽然暴露了 `/api/v1/graph/query`，
但 `causal_chain`、`timeline`、`temporal_bfs`、`temporal_path`
仍然是占位返回，注释也明确写着
“Full implementation would use evif-graph TemporalGraph”。

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

## 7. OpenClaw 官方能力边界（截至 2026-03-15）

下面这一节只基于 OpenClaw 官方文档与官方仓库 README，不基于第三方转述。

### 7.1 OpenClaw 本质上是一个本地优先的 Agent Gateway

官方文档首页把 OpenClaw 描述为一个本地优先、可扩展、可自托管的 Agent 平台，核心能力包括：

- `Gateway`：统一承接消息、工具、会话和插件调度
- `Providers`：可切换模型与推理后端
- `Chat Apps / Channels`：可接入多个消息入口
- `Plugins`：通过插件扩展 Agent、工具、应用集成
- `MCP Servers`：可消费外部 MCP 工具生态

这意味着 OpenClaw 不是单一“聊天界面”，而是已经自带运行时编排层。

### 7.2 OpenClaw 默认长期记忆是 Markdown 文件，而不是专用数据库 API

官方 Memory 文档说明，OpenClaw 的记忆目录默认为 `./memory`，并预置一组 Markdown 文件：

- `preferences.md`
- `short-term.md`
- `tasks.md`
- `rules.md`

文档还明确要求这些文件用 Markdown 组织长期偏好、短期上下文、待办事项和行为规则。  
这点非常关键，因为它说明 OpenClaw 对“长期记忆”的第一原语其实是文件，而不是某个专有 RPC。

### 7.3 OpenClaw 工具体系分层明确，且支持 richer interaction

官方 Tools 文档把工具分成三类：

- `Built-in Tools`
- `MCP Tools`
- `Agent Tools`

同时文档还支持：

- 普通同步返回
- 异步任务型工具（长任务轮询）
- 需要用户确认的工具
- 带表单输入的工具

这代表如果 EVIF 只以“普通命令式 MCP 工具”接入，只能覆盖 OpenClaw 工具体系的一部分，不能天然覆盖确认流、表单流和异步任务语义。

### 7.4 OpenClaw 仍然把“外部 coding agent runtime”视为插件能力

官方 ACP Agents 文档说明，OpenClaw 可以通过 ACP Agents 插件接入外部 coding agent runtime，并暴露：

- `sessions/spawn`
- `message/send`
- `session/cancel`
- `session/list`

从这些动作可以推断，OpenClaw 对外部 coding runtime 的抽象是：

- 可以被创建的长生命周期会话
- 可以持续投喂消息
- 可以取消、枚举和查询状态

这也解释了为什么官方 README 会把 `Pi`
作为当前原生 coding agent 路径，
而把 Claude / Codex 等旧路径标记为移除或迁移：  
对 OpenClaw 来说，“把另一个系统接进来当 agent”目前更自然的方式，是 `ACP 插件`，而不是直接硬编码更多内建 runtime。

### 7.5 对 EVIF 最重要的一点：OpenClaw 天然拥有“消息入口”

从官方定位看，OpenClaw 的外层边界首先是消息渠道和 Gateway，其次才是工具和文件。  
因此如果和 EVIF 深度结合，最稳的职责分界应该是：

- OpenClaw 保留消息入口、会话状态、Agent 编排
- EVIF 提供文件、搜索、记忆、挂载、观测等底层能力

如果反过来让 EVIF 去承担 OpenClaw 的消息入口职责，
会额外引入大量适配工作，而仓库里目前并没有现成的
Telegram / Discord / iMessage 之类渠道桥。

### 7.6 OpenClaw 官方来源

- [OpenClaw Docs 首页](https://docs.openclaw.net)
- [OpenClaw Memory 文档](https://docs.openclaw.net/core-concepts/memory)
- [OpenClaw Tools 文档](https://docs.openclaw.net/plugins/tools)
- [OpenClaw ACP Agents 文档](https://docs.openclaw.net/plugins/agents/acp-agents)
- [OpenClaw 官方 README](https://github.com/openclaw/openclaw)

## 8. OpenClaw 需求到 EVIF 能力的映射

### 8.1 映射矩阵

<!-- markdownlint-disable MD013 -->
| OpenClaw 需求 | OpenClaw 官方形态 | EVIF 可复用能力 | 推荐接法 | 当前缺口 |
| --- | --- | --- | --- | --- |
| 长期记忆文件 | `./memory/*.md` | `evif-mem` + `MemPlugin` + `evif-fuse` | 把 OpenClaw memory 目录映射到 EVIF 记忆挂载，或做双向同步 | 需要对齐默认文件名、Frontmatter 字段、目录布局 |
| 文件式上下文 | Markdown / worktree / 本地文件 | `evif-rest` 文件接口、`evif-fuse`、本地挂载表 | 优先 `FUSE + REST` 组合，不建议只靠 MCP | 需补 memory 目录约定和权限边界 |
| 工具调用 | Built-in / MCP / Agent Tools | `evif-mcp` 现有 17 个工具，`evif-rest` 可封装成 OpenClaw 插件工具 | 短期直接消费 EVIF MCP；中期改成 OpenClaw 原生 tool plugin 调 EVIF REST | EVIF MCP 暂未表达确认流、表单流、异步任务语义 |
| 搜索与检索 | 工具调用或工作区文件扫描 | `/api/v1/grep`、`/api/v1/memories/search` | 搜索走 REST，比 FUSE 目录扫描更可靠 | 图查询仍是占位，不能承接高级时序推理 |
| 会话式 coding runtime | ACP session lifecycle | `evif-rest`、`/ws`、`evif-mem` workflow | 二阶段再考虑用 ACP 插件包装 EVIF 执行器 | EVIF 目前没有 ACP 协议层，也没有 `sessions/spawn` 这类对象模型 |
| 多消息渠道入口 | Gateway + chat apps | 无直接等价层 | 保持 OpenClaw 负责渠道接入，EVIF 不碰 ingress | EVIF 目前缺少消息平台适配器 |
| 运行观测 | Gateway / task / tool 状态 | EVIF metrics、monitor、web UI | 用 EVIF 做后端观测补充面板 | 尚未和 OpenClaw 会话状态做统一关联 |
<!-- markdownlint-enable MD013 -->

### 8.2 其中最适合一阶段复用的是“Markdown 记忆层”

OpenClaw 的默认记忆就是 Markdown 文件，这一点与 EVIF 的记忆设计天然接近：

- `evif-mem` 已经支持把记忆表示为 `Markdown + YAML Frontmatter`
- `MemPlugin` 已经能把记忆系统暴露为文件系统视图
- `evif-fuse` 已经能把 EVIF 挂载为本地目录

因此一阶段最顺的路线不是“先把 OpenClaw 接到 EVIF 的图引擎”，而是：

1. 先把 `./memory` 这层文件语义打通
2. 再把搜索、检索、分类等能力通过 REST 补上
3. 最后再考虑更复杂的知识图谱增强

### 8.3 其中最不适合一阶段复用的是“ACP 级 runtime 接管”

虽然 OpenClaw 官方确实提供 ACP Agents 扩展点，但这条路对 EVIF 来说太深：

- EVIF 现在的主对象是文件、挂载、插件、记忆、搜索
- OpenClaw ACP 需要的是可创建、可取消、可持续发送消息的 session runtime
- EVIF 现有 WebSocket 主要是终端命令流，不是完整 agent session 协议

所以如果一开始就要让 EVIF
“伪装成 OpenClaw 的 coding agent runtime”，
工作量会显著高于
“让 EVIF 先做 OpenClaw 的底层状态与记忆后端”。

### 8.4 MCP 可以接，但不应成为第一阶段唯一通路

从表面上看，OpenClaw 支持 MCP，EVIF 也有 MCP Server，二者似乎可以直接对接。  
但结合 EVIF 现状，MCP-only 路线至少有 3 个问题：

1. `evif_memorize` 当前和 REST 的字段契约不一致，真实调用有失败风险。
2. OpenClaw 工具体系不只有普通 MCP 调用，还有确认、表单和异步任务语义。
3. 只走 MCP 不利于复用 OpenClaw 对 `./memory` 目录的原生文件语义。

所以更合理的判断是：

- `MCP` 适合作为补充工具面
- `REST + FUSE/文件映射` 更适合作为第一阶段主通路

## 9. 推荐的集成方案

### 9.1 一阶段推荐方案：OpenClaw 负责入口，EVIF 负责状态层

这一阶段建议采用下面的职责边界：

- `OpenClaw`
  - 负责 Gateway、消息渠道、Provider 选择、Agent 编排
  - 负责把用户输入转成工具调用和记忆读写需求
- `EVIF`
  - 负责长期记忆存储
  - 负责文件系统视图
  - 负责搜索、挂载、观测和运维面板

对应的技术接法建议是：

1. `./memory` 目录优先映射到 EVIF 记忆层
2. 搜索、挂载、健康检查、分类查询走 `evif-rest`
3. 常规文件工具和补充操作可走 `evif-mcp`
4. 人工观测和调试使用 `evif-web`

### 9.2 一阶段建议的最小可落地架构

可以把第一版拆成两个桥接器：

1. `OpenClaw -> EVIF Memory Bridge`
   - 把 OpenClaw 的 `./memory/*.md` 读写映射到 EVIF 记忆目录或 `MemPlugin`
   - 保证 `preferences / short-term / tasks / rules` 等默认文件仍可被 OpenClaw 原生读取

2. `OpenClaw -> EVIF Tool Bridge`
   - 把搜索、grep、文件操作、挂载管理接到 EVIF REST
   - MCP 只承接那些已经稳定的通用工具调用

这条路径的优势是：

- 不破坏 OpenClaw 原生工作方式
- 不要求 EVIF 先实现 ACP runtime
- 先用 EVIF 最成熟的文件/记忆/REST 能力建立价值

### 9.3 二阶段增强方案：让 EVIF 成为 OpenClaw 的外部 Agent Runtime

如果后续希望让 EVIF 更深度进入 OpenClaw 执行面，推荐方向不是继续堆 REST endpoint，而是新增一个 ACP 兼容层。

更具体地说，可以考虑：

1. 新增一个 `openclaw-evif-acp` 适配器
2. 对外实现 `sessions/spawn`、`message/send`、`session/cancel`
3. 对内把请求转成 EVIF workflow、记忆检索、文件操作和工具执行

这条路的前提是：

- 先定义 EVIF 的 session 对象模型
- 先补齐消息流与状态机
- 先明确单会话工作区和多会话隔离边界

所以它明显应该放在二阶段，而不是一阶段。

### 9.4 当前最合理的优先级

综合 OpenClaw 官方模型和 EVIF 现状，当前推荐顺序是：

1. `Memory / Markdown` 对接
2. `REST` 工具与检索对接
3. `MCP` 补充工具对接
4. `FUSE` 本地目录增强
5. `ACP` 深度 runtime 桥接

## 10. 本轮结论

这一轮研究后的核心结论是：

- OpenClaw 的第一原语不是“Agent 工具”而是“Gateway + 渠道 + Markdown 记忆 + 插件”。
- EVIF 最适合承接的是 OpenClaw 的状态层，而不是消息入口层。
- 第一阶段最优方案是 `OpenClaw 管入口，EVIF 管记忆/文件/搜索`。
- 真正想让 EVIF 变成 OpenClaw 的外部执行 runtime，应走 `ACP 插件`，但这应放在第二阶段。
- 下面进一步把结论收敛为实施蓝图：拆分桥接组件、定义目录/协议映射、列出风险与实施步骤。

## 11. 实施蓝图

### 11.1 推荐的分层职责

如果目标是尽快落地一版可运行集成，建议把整体拆成三层：

1. `OpenClaw Gateway Layer`
   - 负责聊天渠道、Provider 选择、Agent 编排、会话生命周期
   - 继续保留 OpenClaw 自己的消息入口，不迁移到 EVIF
2. `OpenClaw-EVIF Bridge Layer`
   - 负责 `./memory/*.md` 和 EVIF typed memory 之间的兼容映射
   - 负责把 OpenClaw 的工具调用转成 EVIF REST / MCP 调用
   - 负责补 session、workspace、user 关联信息
3. `EVIF State Layer`
   - 负责长期记忆、文件访问、搜索、挂载、监控与运维界面
   - 作为 OpenClaw 之外的可观察、可检索、可持久化底座

用一句话概括，就是：

- OpenClaw 负责“对话入口和调度”
- Bridge 负责“协议与视图兼容”
- EVIF 负责“状态与存储”

### 11.2 推荐新增的 3 个桥接组件

#### A. `openclaw-evif-memory-bridge`

这是第一阶段最关键的组件，职责是把 OpenClaw 的固定记忆文件视图和 EVIF 的 typed memory 视图对齐。

它至少要做 4 件事：

1. 监听或接管 `./memory/preferences.md`、`short-term.md`、`tasks.md`、`rules.md`
2. 把这些文件解析成可写入 EVIF 的结构化记忆单元
3. 在需要时把 EVIF 里的记忆重新投影回 OpenClaw 兼容文件
4. 处理冲突、去重、更新时间和失败重试

为什么这层是必须的：

- OpenClaw 默认是固定文件集合
- EVIF `MemPlugin` 当前暴露的是按类型分目录的多文件结构，而不是固定四个文件
- `MemPlugin` 真实目录更接近 `/profile/*.md`、`/event/*.md`、`/knowledge/*.md`、`/behavior/*.md`、`/skill/*.md`、`/tool/*.md`

也就是说，如果没有 compatibility bridge，二者虽然都“用 Markdown”，但文件视图并不兼容。

#### B. `openclaw-evif-tool-bridge`

这一层建议优先实现为 OpenClaw 原生插件，内部调用 EVIF REST。

原因很直接：

- `evif-rest` 的接口面最清晰，覆盖文件、搜索、挂载、监控、记忆
- OpenClaw 的工具语义比普通 MCP 更丰富
- 如果只消费 EVIF MCP，会受到当前 MCP 工具表达能力和契约问题的限制

这一层一阶段建议先支持：

1. 文件读写与目录查询
2. `grep` / memory search
3. 健康检查
4. mount / unmount / mount list

MCP 仍然有价值，但更适合做补充工具面，而不是唯一主通路。

#### C. `openclaw-evif-observer`

这层不是必须先做，但强烈建议一阶段就留埋点。

职责包括：

1. 把 OpenClaw 的 `session_id`、`workspace_id`、`channel_id` 关联到 EVIF 操作
2. 把 EVIF 的搜索、写入、挂载、失败事件汇总成可观测日志
3. 让 `evif-web` 或其它监控面板能按 OpenClaw 会话维度排查问题

如果没有这层，集成即使“能跑”，排障成本也会很高。

### 11.3 一阶段与二阶段的 source of truth

从当前代码状态看，不建议一上来就让 EVIF 成为唯一真源。

更稳妥的做法是：

1. 一阶段：
   - OpenClaw 的 `./memory/*.md` 仍是主写入界面
   - EVIF 负责派生索引、搜索、持久化增强和文件视图
   - Bridge 负责把文件变更同步进 EVIF
2. 二阶段：
   - 等 EVIF 的 metadata 持久化、兼容投影和会话关联跑通后
   - 再把 EVIF 提升为主要状态真源
   - OpenClaw 的固定记忆文件退化为 compatibility projection

这样做的原因不是理念，而是实现现状：

- `MemPlugin` 当前没有 OpenClaw 固定文件投影视图
- `evif-rest` 的创建记忆接口虽然接收 `metadata`，但当前 handler 实际只用 `content`
- 这意味着如果现在直接把 EVIF 当成唯一真源，很多 OpenClaw 侧上下文信息会丢失或无法往返

## 12. 目录与协议映射

### 12.1 记忆文件映射建议

下面是更适合一阶段落地的推荐映射，不是强制一一对应。

<!-- markdownlint-disable MD013 -->
| OpenClaw 文件 | 语义 | EVIF 推荐落点 | 说明 |
| --- | --- | --- | --- |
| `preferences.md` | 用户偏好、长期习惯、个人设定 | `profile/*.md` 为主，必要时补 `behavior/*.md` | 最接近 EVIF 的 `Profile` / `Behavior` 记忆 |
| `short-term.md` | 当前会话短期上下文 | 一阶段保留为 OpenClaw 本地兼容文件；如需检索再派生为 `event/*.md` 或 `knowledge/*.md` | 不建议默认全部沉淀成长期记忆 |
| `tasks.md` | 待办、计划、执行中事项 | 一阶段作为兼容文件保留；如要索引，建议写入 `knowledge/*.md` 并打 `task` 标签 | EVIF 当前没有独立 `Task` memory type |
| `rules.md` | 行为约束、操作规则、系统规约 | `behavior/*.md` 为主，必要时落到 `knowledge/*.md` | 更接近操作规则而不是用户事实 |
<!-- markdownlint-enable MD013 -->

这里最重要的不是文件名，而是要明确两点：

1. OpenClaw 的四个默认文件并不能直接映射到 EVIF 现有六种 `MemoryType`
2. 因此桥接层必须负责“拆分、归类、聚合”，而不是简单搬文件

### 12.2 Markdown frontmatter 映射建议

EVIF 当前 Markdown 记忆格式已经有明确 frontmatter 字段，至少包括：

- `id`
- `type`
- `created`
- `updated`
- `happened_at`
- `tags`
- `embedding_id`
- `category_id`
- `content_hash`
- `reinforcement_count`
- `ref_id`
- `references`

而 OpenClaw 的默认记忆文件更偏向纯 Markdown 语义组织。  
因此 bridge 在写入 EVIF 时建议增加一层内部映射：

<!-- markdownlint-disable MD013 -->
| OpenClaw 侧信息 | 一阶段建议写入方式 | 原因 |
| --- | --- | --- |
| 文件路径 (`preferences.md` 等) | 转成 bridge 规则选出的 `type` + `tags` | EVIF 不以固定文件名建模 |
| 标题/段落/列表项 | 拆成独立 `MemoryItem` 或逻辑块 | 便于搜索、去重和分类 |
| 最近修改时间 | 写入 `updated` | 可保留投影更新时间 |
| 会话或频道上下文 | 先放 bridge sidecar index 或兼容文件注释；不要依赖当前 REST metadata round-trip | 当前 REST create_memory 未实际持久化 metadata |
| 引用关系 | 写入 `references` 或 `ref_id` | 便于后续做关联跳转 |
<!-- markdownlint-enable MD013 -->

### 12.3 OpenClaw 调 EVIF 的协议优先级

推荐顺序如下：

1. `REST`
   - 用于搜索、记忆查询、文件操作、挂载和健康检查
2. `FUSE`
   - 用于保留 OpenClaw 的本地文件工作流语义
3. `MCP`
   - 用于补充通用工具调用，而不是承担全部状态流转

不要反过来把 `MCP` 放到最前面，原因有两个：

1. 当前 `evif_memorize` 仍然向 `/api/v1/memories` 发送
   `{ text, modality }`，而 REST 真实契约要求
   `{ content, modality, metadata }`
2. OpenClaw 的工具交互不只是普通 request-response，后续大概率还需要 richer interaction

## 13. 分阶段实施建议

### 13.1 Phase 1A：先做 memory compatibility bridge

这一阶段只解决一件事：

- 让 OpenClaw 原生 `./memory/*.md` 能继续工作
- 同时把内容稳定同步到 EVIF

最小实现建议：

1. 以 `./memory` 为输入面
2. 监听文件变更
3. 按标题或逻辑块拆分内容
4. 按映射规则写入 EVIF typed memory
5. 记录同步状态、失败重试和冲突标记

这一步跑通后，OpenClaw 才真正“用上了 EVIF”。

### 13.2 Phase 1B：补 OpenClaw 原生 REST tool bridge

第二步再补工具，不要一开始就把问题做成“协议兼容工程”。

建议先接下面这些接口：

- `/api/v1/grep`
- `/api/v1/memories/search`
- `/api/v1/memories`
- `/api/v1/categories`
- `/api/v1/mounts`
- `/health`

这一阶段目标是让 OpenClaw 能：

1. 搜索 EVIF 中的文件和记忆
2. 查询 EVIF 健康状态
3. 管理挂载点
4. 把工具结果回填到自己的会话中

### 13.3 Phase 1C：加观测与失败回退

如果只做同步和工具，而没有回退机制，生产可用性会很差。

这一阶段至少要补：

1. EVIF 不可用时的本地降级
2. Bridge 队列积压告警
3. 基于 session 的操作日志
4. 文件冲突与重复写入检测

建议原则是：

- OpenClaw 的本地对话和记忆文件不能因为 EVIF 短时故障而完全失效
- EVIF 是增强层，不应成为第一阶段单点故障

### 13.4 Phase 2：再评估 ACP runtime bridge

只有在下面条件都成立后，才值得进入 ACP 方向：

1. Memory bridge 已长期稳定
2. REST tool bridge 已覆盖主要工具
3. 会话关联和观测闭环已经建立
4. 团队确实需要“把 EVIF 当成外部 agent runtime”

否则 ACP 很容易把问题从“系统集成”升级成“运行时重构”。

## 14. 验收标准

### 14.1 Happy path

至少应验证以下路径：

1. 修改 `preferences.md` 后，Bridge 能在 EVIF 中生成可搜索的 profile 类记忆
2. 修改 `rules.md` 后，OpenClaw 下一轮工具决策仍能读到最新规则
3. 通过 OpenClaw 工具调用 EVIF 搜索接口，结果能回到会话
4. `evif-web` 或 REST 查询能观察到 Bridge 写入后的状态

### 14.2 Failure path

至少应覆盖以下异常路径：

1. EVIF REST 暂时不可用时，OpenClaw 本地 `memory/*.md` 仍可继续写入
2. 同一文件短时间被多次改写时，Bridge 不发生静默覆盖
3. Bridge 重试失败时，能留下待处理标记或 sidecar 状态
4. MCP 记忆调用在契约修复前不得被当作唯一写入路径

### 14.3 Data path

至少应确认以下一致性：

1. OpenClaw 文件改动能进入 EVIF
2. EVIF 搜索结果能定位回原始 OpenClaw 文件或逻辑块
3. 删除或归档操作不会留下孤儿索引
4. 时间戳与引用信息不会在同步中被无声丢弃

## 15. 当前最需要注意的风险

### 15.1 REST 记忆接口还不是完整 memory pipeline

`init_memory_pipelines()` 现在仍是可选初始化；当前 handler 主要依赖基础 `MemoryStorage`。  
这意味着一阶段集成可以用，但不要预设已经拥有完整的自动提炼、自动分类、自动演化行为。

### 15.2 CreateMemoryRequest 的 metadata 目前没有真正落库

虽然 REST 接口 schema 中已有 `metadata` 字段，
但当前 `create_memory` handler 实际只基于 `content`
构造 `MemoryItem`。  
所以如果 Bridge 想保存 `session_id`、`channel_id`、
`workspace_id` 等上下文，不能假设现在已经能通过
REST metadata 完成往返。

### 15.3 OpenClaw 固定文件模型与 EVIF typed memory 模型并不等价

这不是字段名问题，而是信息组织方式不同：

- OpenClaw 偏“少量固定文件 + Markdown 组织”
- EVIF 偏“多条结构化记忆 + typed files + frontmatter”

因此必须接受 bridge 是产品级组件，而不是临时脚本。

### 15.4 图谱与 ACP 都不该提前成为一阶段阻塞项

从当前仓库状态看：

- 图查询 REST 仍有占位实现
- ACP runtime bridge 需要新的对象模型与协议层

所以两者都应是二阶段增强，而不是第一阶段门槛。

## 16. 最终建议

如果只给一个落地建议，就是：

1. 不要试图让 OpenClaw 和 EVIF 一步到位“深度融合”
2. 先用 `memory compatibility bridge + REST tool bridge` 建立稳定价值
3. 把 EVIF 放在状态层和记忆层，把 OpenClaw 保留在入口层和编排层
4. 等 metadata、会话关联、兼容投影成熟后，再考虑提升 EVIF 的真源地位
5. ACP 只在确认确有 runtime 托管需求时再进入

按这个顺序推进，风险最小、复用最多，也最符合当前代码现实。
