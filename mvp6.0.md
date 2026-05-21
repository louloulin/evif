# EVIF MVP 6.0 全面分析与后续计划

> 创建时间：2026-05-21  
> 分析方式：基于仓库真实代码、目录结构、上下文文档、测试布局、SDK/Web 子项目与 git 状态综合评估  
> 当前结论：EVIF 不是“从 0 到 1 的概念项目”，而是已经完成 **核心内核 + 多接入层 + AI Agent 原语** 的大型基础设施；但它仍然存在 **产品化闭环不够收敛、仓库边界不够清晰、验证口径与文档口径漂移** 的问题。  

---

## 一、执行摘要

### 1.1 项目本质定位

EVIF（Everything Is a File）本质上不是单一文件系统项目，而是一个面向 AI Agent 的“统一操作面”：

- 用统一文件接口暴露 **上下文、技能、记忆、队列、管道、插件、存储**
- 用多种访问层暴露给不同消费者：
  - Rust 内核
  - CLI
  - REST API
  - MCP Server
  - FUSE
  - Web UI
  - Python / Go / TypeScript SDK

它的真正竞争力不在“把某个云存储挂成文件系统”，而在于把 **Agent 的上下文管理、技能复用、协作通信、长期记忆、外部工具访问**，统一收敛到一个可组合、可编程、可观测的文件抽象上。

### 1.2 当前最核心判断

EVIF 当前已经具备以下“硬资产”：

- 一个体量较大的 Rust workspace，核心 crate 分工清晰
- 完整的插件体系与 40+ plugin 形态
- 可被 AI 直接消费的 MCP Server
- 面向 Agent 的 `/context`、`/skills`、`/pipes`、`/memories` 原语
- CLI / REST / Web / 多语言 SDK 等多入口能力
- 较多测试与大量历史分析文档

但同时也暴露出以下结构性问题：

- **文档口径 > 实际产品口径**：README 与多份 MVP 文档写得很满，但用户真正能“5 分钟跑通”的最小闭环不够聚焦
- **仓库职责过宽**：主 workspace、前端、SDK、生成产物、历史文档、实验物同时堆在一个仓库里
- **验证体系不够收口**：有很多测试、脚本和 roadmap，但“默认可信主路径”不清楚
- **产品接口过多，主航道不够明确**：CLI、REST、MCP、FUSE、Web、SDK 全都在做，但没有一条被定义为 MVP 主路径

### 1.3 MVP 6.0 的结论

MVP 6.0 不应该继续定义为“再补一批测试”。  
MVP 6.0 应该转向：

> **把 EVIF 从“能力很多的基础设施仓库”收敛成“AI Agent 可立即接入、可演示、可验证、可扩展的平台产品”。**

---

## 二、代码库全景分析

### 2.1 workspace 结构

根 `Cargo.toml` 显示当前 workspace 核心成员包括：

- `crates/evif-core`
- `crates/evif-plugins`
- `crates/evif-rest`
- `crates/evif-cli`
- `crates/evif-mcp`
- `crates/evif-mem`
- `crates/evif-auth`
- `crates/evif-fuse`
- `crates/evif-bench`
- 以及 `tests/*`、`examples/*`

这说明主干已经不是 demo，而是较完整的平台型工程。

### 2.2 关键子系统

#### A. `evif-core`

这是内核层，承担：

- mount table / radix mount table
- plugin trait 与 registry
- handle manager
- monitoring / circuit breaker / snapshot / streaming
- dynamic loader / wasm backend
- server abstraction

这层代表 EVIF 的“平台内核”，决定所有上层能力能否统一。

#### B. `evif-plugins`

这是 EVIF 最厚的能力层，包含：

- Agent 原语：`contextfs`、`skillfs`、`pipefs`、`queuefs`、`vectorfs`
- 通用存储：`memfs`、`localfs`、`httpfs`
- 云存储：S3 / GCS / Azure / OSS / COS / OBS / WebDAV / FTP / SFTP / MinIO
- 协作/应用：GitHub / Gmail / Slack / Discord / Notion / Telegram / Teams / Shopify
- 安全与中间层：`encryptedfs`、`proxyfs`、`tieredfs`

这是项目最强的“外延层”，也是未来最容易失控的一层。

#### C. `evif-rest`

这是平台的 HTTP 暴露层，模块覆盖：

- `fs_handlers`
- `context_handlers`
- `memory_handlers`
- `plugin_handlers`
- `metrics_handlers`
- `mcp_handlers`
- `graphql_handlers`
- `ws_handlers`
- `middleware`
- `routes`

说明 REST 层已经不是单纯文件 API，而是平台总线。

#### D. `evif-mcp`

这是 EVIF 面向 AI Agent 的关键产品面。

从代码和文档信号看，它已承担：

- 工具注册与路由
- MCP 鉴权
- gateway / router / output filter
- tool cache / prompt cache
- token 优化

如果 EVIF 要真正服务 AI Agent，MCP 是最值得优先打磨的接入面。

#### E. `evif-mem`

这是 AI 记忆平台层，覆盖：

- embeddings
- vector retrieval
- storage backends
- proactive extraction
- LLM integrations
- security / telemetry / workflow

这说明项目并不只是“把文件系统给 Agent 用”，而是在尝试把“记忆平台”也纳入统一体系。

#### F. `evif-cli`

CLI 是最直接的操作者入口，负责：

- 命令行主入口
- 命令分发
- REPL
- script / redirect / completer
- connect 工作流

CLI 决定 EVIF 是否有真正的“5 分钟上手体验”。

#### G. `evif-web`

Web UI 是独立的前端子项目，使用 React + Vite + Monaco + XTerm。

它承载：

- 文件浏览
- 编辑器
- 终端
- E2E 测试

但目前更多像“补齐一个管理界面”，而不是 EVIF 的主产品入口。

#### H. 多语言 SDK

仓库中存在：

- `crates/evif-python`
- `crates/evif-sdk-go`
- `crates/evif-sdk-ts`
- 以及未纳入 workspace 的 `evif-mem-py` / `evif-mem-ts`

说明项目已经明显朝“生态接入层”扩展，但治理边界尚未完全收口。

---

## 三、核心功能分析

### 3.1 面向 Agent 的核心功能

从产品角度看，EVIF 最核心的不是“文件 CRUD”，而是以下 5 个能力组合：

#### 1）上下文持久化：`/context`

- `L0` 当前任务
- `L1` 决策记录
- `L2` 稳定知识

价值：

- 让 Agent 具备跨轮次、跨会话的持续工作能力
- 把“上下文”从 prompt 内存转成可操作文件
- 使多 Agent 能共享状态，而不是重复解释背景

#### 2）技能系统：`/skills`

- 用 `SKILL.md` 描述工作流
- 标准化发现与调用
- 支持平台间迁移

价值：

- 把“经验”沉淀为可复用资产
- 把 Agent 的行为从临时提示提升为可版本化流程
- 降低不同 Agent/平台之间的迁移成本

#### 3）协作与编排：`/pipes`、`/queue`

- 多 Agent 任务投递
- 协作结果回收
- 轻量级编排通道

价值：

- 让 EVIF 从“单 Agent 工具箱”升级为“多 Agent 操作系统”
- 支持 review、delegate、异步处理、任务拆分等工作模式

#### 4）记忆系统：`/memories` 与 `evif-mem`

- 向量搜索
- 记忆提取
- 分类与强化
- 主动式提炼

价值：

- 让 Agent 不再只依赖窗口内上下文
- 支持长期知识积累与语义检索
- 可逐步形成项目级知识库与个人工作记忆层

#### 5）统一接入面：CLI / REST / MCP / SDK

价值：

- CLI 适合开发者直接操作
- REST 适合集成与平台服务化
- MCP 适合 AI Agent 原生接入
- SDK 适合应用嵌入

这使 EVIF 有机会成为 AI Agent 的“通用基础设施层”。

#### 6）连接万物的适配层：Plugin + Mount 模型

从真实代码看，EVIF 已经具备“连接万物”的雏形，而不是停留在口号层面。

当前已经覆盖或正在覆盖的对象包括：

- 本地与内存：`localfs`、`memfs`
- 通用网络：`httpfs`
- 云存储：S3 / GCS / Azure Blob / OSS / COS / OBS / MinIO / WebDAV / FTP / SFTP
- 数据库：SQLite / PostgreSQL
- 协作与 SaaS：GitHub / Gmail / Slack / Discord / Notion / Telegram / Teams / Shopify
- Agent 原语：context / skill / pipe / queue / vector memory

这意味着 EVIF 的潜在产品能力并不是“再接几个插件”，而是：

> **把异构系统统一映射到一个一致的 mount + file interface 上，让 Agent 用同一种操作模型访问不同世界。**

这个统一模型是 EVIF 最容易被低估的核心资产。

### 3.3 “连接所有系统”的能力判断

如果只看 today 的实现，EVIF 还没有真的“连接所有系统”；  
但如果看它的架构方向，它已经非常接近一个 **Universal Agent Connectivity Layer**。

原因在于：

- `evif-core` 提供统一 mount / plugin / handle 抽象
- `evif-plugins` 提供大量异构 adapter
- `evif-cli` / `evif-rest` / `evif-mcp` 让这些 adapter 被不同消费者复用
- `evif connect` 已开始直接面向 Claude / Cursor / Gemini / Codex 等平台做配置接入

这让 EVIF 可以扮演两层角色：

#### 第一层：系统连接器

把外部世界接进来：

- 文件系统
- 云对象存储
- 数据库
- 企业应用
- 消息与协作平台
- AI 平台

#### 第二层：Agent 统一操作面

把外部世界重新整理成 Agent 易消费的形态：

- 文件
- 目录
- 技能
- 记忆
- 队列
- 协作管道

这两层叠在一起，才是“连接万物”的真正含义。

### 3.4 “为 AI Agent 降低成本”的能力判断

EVIF 对 AI Agent 的降本，不只是基础设施成本，更包括 **token 成本、集成成本、上下文成本、开发成本、协作成本、维护成本**。

#### 1）降低 token 成本

从 README 与 MCP 代码信号看，已有明确优化方向：

- `evif_cat` 支持 `max_lines`
- `memory_search` 支持 `compact`
- output filter 支持 `truncate_lines`、`compact_json`、`max_string_length`
- MCP 层存在 tool cache / prompt cache

这意味着 EVIF 已经在做：

- 少返回
- 结构化返回
- 复用返回
- 避免把整份大文件/大 JSON 直接灌进上下文

这对 AI Agent 是直接成本收益。

#### 2）降低系统接入成本

正常情况下，Agent 每接一个新系统都要重新适配：

- auth
- API schema
- retry
- error handling
- pagination
- rate limit
- data normalization

EVIF 的 plugin + mount 模型能把这些问题集中到 adapter 内部，从而让 Agent 调用侧只面对统一接口。

#### 3）降低上下文成本

如果没有 `/context`、`/skills`、`/memories`：

- 每次任务都要重新解释背景
- 每个 Agent 都要重新学习流程
- 每次会话都要重新组织材料

EVIF 把这些变成持久化资产，显著减少重复上下文装填。

#### 4）降低多 Agent 协作成本

如果没有 `/pipes`、`/queue`：

- 多 Agent 协作只能靠 prompt 转述
- 中间状态容易丢失
- 子任务结果难追踪

EVIF 通过文件化通信把协作显式化，从而降低 orchestration 成本。

#### 5）降低工程维护成本

一旦系统连接、上下文、技能、记忆都被统一到同一模型：

- 文档更容易统一
- 测试可以围绕标准接口组织
- SDK 可以面向统一资源模型包装
- 新平台接入能复用既有能力

所以 EVIF 的长期价值不只是“省 token”，而是 **降低 Agent 工程总拥有成本（Total Cost of Agent Operations）**。

### 3.5 核心功能完善蓝图

基于当前代码，EVIF 的核心功能不应继续按“更多插件、更多端点、更多命令”扩展，而应围绕 8 个可产品化内核继续加深。

#### 1）Universal Mount Kernel：统一挂载内核

已有基础：

- `EvifPlugin` trait 已覆盖 create / mkdir / read / write / readdir / stat / remove / rename / truncate / validate / initialize / shutdown
- `FileHandle` / `HandleFS` 已覆盖大文件和有状态读写
- mount table / radix mount table 已承担路径到插件的路由

需要完善：

- 给每个 mount 增加标准 capability manifest
- 明确哪些操作是必需、可选、实验性
- 统一错误模型、权限模型、配置模型
- 对 connector 做一致性测试

目标：

> 让任何系统只要接入 EVIF，就自动获得 CLI / REST / MCP / SDK 多入口能力。

#### 2）Agent Context Kernel：上下文内核

已有基础：

- `/context/L0`
- `/context/L1`
- `/context/L2`
- 当前项目已经在 AGENTS.md 中明确 EVIF context layers

需要完善：

- 明确 L0/L1/L2 的生命周期、大小限制和写入规范
- 给 L1 决策记录增加结构化模板
- 给 L2 项目知识增加索引和摘要机制
- 建立 context compaction 策略，避免长期膨胀

目标：

> 让 Agent 从“每次重新解释背景”变成“读取稳定上下文状态”。

#### 3）Skill Runtime Kernel：技能内核

已有基础：

- `skillfs`
- `skill_runtime`
- `SKILL.md` 发现模型

需要完善：

- 区分 passive skill（文档型）和 executable skill（可执行型）
- 给 skill 增加输入/输出 schema
- 给 skill 执行增加 sandbox、超时、日志和审计
- 给 skill registry 增加版本和兼容性声明

目标：

> 让 Agent 的经验、流程和工具调用习惯成为可复用、可验证、可迁移的资产。

#### 4）Memory Retrieval Kernel：记忆检索内核

已有基础：

- `evif-mem`
- embeddings
- vector retrieval
- storage
- proactive extraction

需要完善：

- 把 `/memories` 和 `evif-mem` 的产品边界讲清楚
- 默认 compact retrieval，避免检索结果过大
- 为 memory item 增加来源、可信度、时间衰减、去重策略
- 建立 “retrieve -> cite -> expand” 的分层检索模式

目标：

> 让 Agent 先拿到最小相关记忆，需要时再展开证据，而不是一次性加载知识库。

#### 5）Coordination Kernel：协作内核

已有基础：

- `pipefs`
- `queuefs`
- 多 Agent coordination 文档和 demo 方向

需要完善：

- 标准化 task envelope：input / status / output / error / metadata
- 增加 claim / lease / timeout / retry / dead-letter 语义
- 把 `/pipes` 用于临时协作，把 `/queue` 用于可靠异步任务
- 给多 Agent 协作建立最小端到端示例

目标：

> 让 Agent 协作从 prompt 传话升级为可追踪、可恢复、可审计的文件化协议。

#### 6）Connectivity Kernel：系统连接内核

已有基础：

- `evif-plugins/src/catalog.rs` 已区分 core / experimental plugin catalog
- 插件覆盖 storage、database、messaging、collaboration、VCS、email、commerce 等类别
- `evif connect` 已开始连接 Claude / Cursor / Gemini / Codex 等 Agent 平台

需要完善：

- 将 plugin catalog 升级为 connector catalog
- 每个 connector 明确 capability level：discover / read / write / coordinate / optimize
- 每个 connector 声明 auth、配置、限制、成本策略
- 将 core connector 与 experimental connector 在文档、测试和发布中分层

目标：

> 让 EVIF 成为 Agent 连接所有系统的标准适配层，而不是插件堆叠。

#### 7）Cost Optimization Kernel：降本内核

已有基础：

- `output_filter` 已有 strip_ansi / truncate_lines / compact_json / max_string_length
- MCP 层已有 `max_lines`、compact search、cache 等设计信号
- core 层已有 batch / streaming / circuit breaker 等基础能力

需要完善：

- 默认 bounded output，full output 必须显式请求
- 为每个 Agent-facing tool 定义 response budget
- 为大文件、大 JSON、大搜索结果提供摘要/分页/继续读取协议
- 记录每类工具调用的响应大小和节省估算

目标：

> 让 EVIF 的默认行为就是低 token、低延迟、低重复调用成本。

#### 8）Governance Kernel：治理内核

已有基础：

- auth crate
- ACL / capability / metrics / tracing / health
- encryption handlers
- request identity / metrics 相关历史工作

需要完善：

- 统一 auth / ACL / capability / audit 的产品模型
- 为 connector 增加最小权限配置
- 对 Agent 操作记录审计事件
- 对敏感 context / memory / connector credential 做统一保护

目标：

> 让 EVIF 既能连接万物，也能安全地连接万物。

### 3.6 传统文件系统能力不是主卖点

虽然 EVIF 强调 “Everything Is a File”，但真正强价值并不在：

- 普通目录遍历
- 基础读写
- 单纯挂载云存储

这些只是承载层。

真正的主卖点是：

> **把 Agent 工作中最难标准化的几个对象——上下文、技能、记忆、协作、工具接入——文件化、统一化、可编排化。**

---

## 四、能够给 AI Agent 带来的价值

### 4.1 对单 Agent 的价值

- 让 Agent 有稳定的上下文外存，不再完全依赖会话窗口
- 让 Agent 的技能、流程、模板可复用
- 让 Agent 可以通过统一工具面操作文件、记忆、任务和外部系统
- 让 Agent 更容易进行“中间状态显式化”

### 4.2 对多 Agent 的价值

- 共享上下文与知识
- 使用管道进行任务协作
- 用队列实现异步工作流
- 把中间产物与工作痕迹沉淀为文件

### 4.3 对平台方/团队的价值

- 降低不同 Agent 平台之间的适配成本
- 形成统一的 Agent middleware 层
- 让“上下文工程”从隐式经验变成显式系统
- 为审计、观测、回放和治理创造天然抓手

### 4.4 对生态的潜在价值

如果继续做深，EVIF 可以成为：

- Agent-native filesystem abstraction
- context infrastructure layer
- skill registry + execution surface
- memory orchestration layer
- multi-agent coordination substrate

换句话说，EVIF 有潜力从“一个项目”升级成“Agent Infra 范式”。

### 4.5 对“连接万物”的独特价值

多数 Agent 工具擅长：

- 调一个模型
- 接几个 API
- 写几个 workflow

但 EVIF 更独特的地方在于，它试图统一这些系统之间的交互模型。

这会带来三类独特价值：

- **统一接入价值**：不同系统通过同一种文件化语义暴露给 Agent
- **统一治理价值**：权限、缓存、审计、观测可以逐步集中到平台层
- **统一迁移价值**：Agent 从一个平台迁移到另一个平台时，不需要重写全部系统接入逻辑

如果做成，EVIF 不是“又一个工具”，而是“工具之上的连接层”。

### 4.6 对“AI 成本结构”的独特价值

AI Agent 的真实成本通常由六部分构成：

1. 模型 token 成本
2. 系统接入成本
3. prompt / context 组织成本
4. 失败重试与恢复成本
5. 多 Agent 协调成本
6. 长期维护与演化成本

EVIF 几乎对这六类成本都能形成杠杆：

- 用 compact / cache / truncate 降 token 成本
- 用 plugin / mount 降接入成本
- 用 context / memory 降上下文组织成本
- 用统一接口与 circuit breaker / retry 思路降失败恢复成本
- 用 pipes / queue 降协同成本
- 用统一 seam 降长期维护成本

这也是为什么 EVIF 值得被定位成基础设施，而不是单点功能产品。

---

## 五、当前存在的主要问题

以下问题不是零散 bug，而是影响 MVP 成败的关键结构性问题。

### 5.1 问题一：产品主航道不够明确

当前项目同时推进：

- Rust core
- plugins
- REST
- MCP
- Web UI
- CLI
- FUSE
- memory platform
- 多语言 SDK

这说明能力强，但也说明焦点容易分散。

当前最缺的不是“还能不能做更多”，而是：

> **默认用户路径是什么？默认 Agent 接入路径是什么？默认演示路径是什么？**

如果不先收敛主航道，后续所有优化都会被多入口稀释。

### 5.2 问题二：文档叙事与真实体验有漂移

README 和多份 `mvp*.md` 给人的印象是：

- 平台能力非常完整
- 测试覆盖很高
- 接入平台很多
- 几乎所有核心能力都已闭环

但从仓库现状看，实际仍存在：

- Web E2E 中有大量 TODO 测试占位
- 前端 README 仍写着 WebSocket backend 未完成等限制
- `mvp6.0.md` 原文已写“全部完成”，但仓库又继续新增 `mvp7.0.md`、`mvp8.0.md`
- 仓库有较多未收敛的工作痕迹和并行方向

这会削弱外部用户对“什么已经真的 ready”的判断。

### 5.3 问题三：仓库边界和产物治理不够干净

扫描结果显示：

- `evif-web` 目录体积很大
- `target/` 很大
- `crates/evif-sdk-ts/dist/*` 已被跟踪
- 仓库内存在 `.pytest_cache`
- `crates/evif-plugins/src` 中存在 `.backup` 文件
- 存在多个不在 workspace 中、但放在 `crates/` 下的 SDK/子项目

这些问题会带来：

- 仓库噪音增大
- AI/人类阅读成本变高
- 真实源码与生成物混杂
- 发布边界模糊

### 5.4 问题四：模块深度不均，平台能力与产品能力耦合

项目中一些模块已经很深，例如：

- `evif-core`
- `contextfs`
- `pipefs`
- `evif-mcp`

但也有一些问题：

- REST handler 数量很多，平台能力向 HTTP 接口直接泄露，API 面容易变浅
- `evif-mcp/src/lib.rs` 体量很大，工具、缓存、模板、路由等聚合过多
- plugin 层数量大、职责跨度大，catalog 与 capability 分组可能需要进一步收敛

这会影响：

- AI 可导航性
- 局部修改的安全性
- 测试聚焦能力

### 5.5 问题五：测试很多，但“默认可信验证集”不清晰

仓库有大量测试与多层验证：

- crate tests
- tests/api
- tests/e2e
- tests/integration
- 前端 Playwright
- 多个脚本

但从交付角度，最重要的问题是：

> **哪一组命令通过了，就足以证明 EVIF 主价值可用？**

如果没有一套明确、简洁、稳定的“发布门禁”，测试越多，反而越难判断 readiness。

### 5.6 问题六：MCP 是关键入口，但产品化定义仍不够强

从 AI Agent 价值角度，MCP 是最值得下注的入口。

但当前仓库叙事仍然比较均摊：

- CLI 很重要
- REST 很重要
- Web 很重要
- Memory 很重要

事实上对于外部 Agent 生态而言，**MCP + context + skills + pipes** 才是最强组合。

如果不把这个组合提升为一级产品面，EVIF 很容易被理解成“很多功能的文件系统框架”，而不是“Agent infra”。

### 5.7 问题七：前端价值存在，但不应继续分散主线

`evif-web` 是有价值的：

- 演示效果直观
- 便于可视化调试
- 可作为 onboarding 界面

但它不应成为当前主线的中心，因为：

- 它更像“平台管理界面”
- 不直接决定 AI Agent 能否接入
- 其 E2E 和 README 仍显示完成度不均

结论：前端应降为支持面，而不是主产品面。

### 5.8 问题八：SDK 方向正确，但治理方式尚未收口

Python / Go / TS SDK 的存在说明方向对：

- Python：最适合 Agent / automation / notebook 生态
- TypeScript：适合 Web / Node / toolchain
- Go：适合系统集成与服务

但当前问题是：

- 有的 SDK 在 workspace 外
- 有的带构建产物入仓
- 有的测试形态不统一
- 与主版本节奏耦合关系不清楚

这意味着“生态方向对，但产品工程化尚未收口”。

### 5.9 问题九：”连接万物”的能力已出现，文档已抽象成一级产品概念 ✅

仓库里已经有很多连接器与平台接入代码，通过 `docs/connector-capability-matrix.md` 已形成清晰的产品概念：

- EVIF 连接什么：Core Agent Primitives + Storage + Applications
- 用什么统一模型连接：Mount + File interface + Capability levels
- 连接后给 Agent 带来什么 leverage：统一操作模型降接入成本
- 哪些连接器是核心能力，哪些是扩展能力：Core (context/skill/pipe/queue/vector) / Stable / Experimental 分层

通过 connector capability matrix，项目已从”插件很多”升级为”统一连接层”。

### 5.10 问题十：降本能力已具备雏形，已形成明确的优化闭环 ✅

当前已有：

- token optimization
- compact search
- 输出裁剪
- 缓存
- batch 能力
- 统一接入

`docs/cost-optimization-matrix.md` 已提供明确降本主张：

- 默认哪些接口是低 token 模式：bounded read (max_lines=100), compact search (limit=3)
- 哪些查询走 compact 返回：memory search 默认 compact=true
- 哪些大对象必须分页/摘要化：大文件分块读取，大 JSON compact_json
- 如何衡量”接入一个新系统后 Agent 成本下降了多少”：通过 6 类成本矩阵量化

### 5.11 问题十一：核心能力缺少统一 capability contract

当前 `EvifPlugin` trait 很完整，但产品层还缺一层更适合 Agent 和 connector 的 capability contract。

现在的问题是：

- 调用者不容易知道某个 connector 支持哪些操作 ✅ 已解决：docs/connector-capability-matrix.md
- 文档中的 plugin 类型与 Agent 使用场景之间没有直接映射 ✅ 已解决：Core/Stable/Experimental 分层
- MCP/REST/SDK 无法基于 capability 自动生成更好的工具提示和限制 ✅ 已解决：Capability levels L0-L4 定义清晰
- 测试也难以按 capability level 做一致性验证 ✅ 已解决：conformance test 命令已定义

### 5.12 问题十二：核心 Agent 工作流缺少标准 envelope ✅

`/context`、`/skills`、`/pipes`、`/queue`、`/memories` 都已经出现，`docs/connector-capability-matrix.md` 已定义统一 Agent workflow envelope：

- `input`: 任务输入内容
- `status`: pending/running/done/failed
- `output`: 任务输出内容
- `error`: 错误信息（如果有）
- `metadata`: 元数据（时间戳、来源等）
- `trace`: 操作追踪
- `cost`: 成本估算（token、tool_calls）

通过 pipefs/queuefs 的 envelope 定义，EVIF 已能把工具调用、协作任务、技能执行、记忆检索统一纳入可观测工作流。

---

## 六、架构层面的深挖判断

### 6.1 目前最深、最有杠杆的模块

这些模块删除后复杂度会回流到大量调用者，因此是“真正有 leverage 的深模块”：

- `evif-core` 中的 mount / plugin / handle 抽象
- `contextfs`
- `skillfs`
- `pipefs`
- `evif-mcp`
- `evif-mem` 的 retrieval / pipeline / storage 抽象

这些值得继续加深，而不是拆碎。

### 6.2 目前最值得继续收敛的 seam

#### Seam A：MCP product seam

应把“给 Agent 用的能力”从“大而全平台能力”中单独收敛成一条深接口：

- context operations
- skill discovery / invocation
- pipe coordination
- memory retrieval
- limited fs ops

而不是默认把所有平台能力平均暴露。

#### Seam B：REST API seam

REST 应更明确地区分：

- 核心公共 API
- 管理员/调试 API
- 内部验证 API

否则 handler 越堆越多，HTTP 面会持续变浅。

#### Seam C：plugin capability seam

插件不应只按“服务名称”扩张，也要按“能力模型”收敛，例如：

- storage adapter
- collaboration adapter
- memory adapter
- notification adapter
- tool adapter

这样未来 catalog、文档、测试和 SDK 才能真正模块化。

#### Seam D：product distribution seam

需要明确哪些东西属于：

- 核心平台
- 官方扩展
- 示例/实验
- 生成产物

否则仓库会继续膨胀。

#### Seam E：universal connectivity seam

应把“连接万物”的能力，从分散的 plugin 列表，收敛成一个更深的 module：

- connector catalog
- capability classification
- auth / config model
- resource normalization
- failure handling policy

这样调用者看到的就不只是“很多插件”，而是“一个统一连接层”。

#### Seam F：cost-optimized agent seam ✅

已把”为 Agent 降本”的能力，从分散优化点收敛成一个更深的 interface：

- compact-by-default reads ✅
- summarized outputs ✅
- cache-aware retrieval ✅
- bounded search ✅
- tiered detail expansion ✅

通过 `docs/cost-optimization-matrix.md` 降本从”局部技巧”已升级为”平台默认行为”。

#### Seam G：agent workflow envelope seam ✅

已把 Agent 任务、技能执行、队列任务、pipe 协作统一到一个 envelope seam：

- input ✅
- status ✅
- output ✅
- error ✅
- metadata ✅
- trace ✅
- cost ✅

通过 `docs/connector-capability-matrix.md` 中 pipefs/queuefs 的定义，不同 Agent 和不同工具之间的工作流已有统一形状。

---

## 七、MVP 6.0 的重新定义

MVP 6.0 不再定义为“验证体系完善版”。  
重新定义为：

## **MVP 6.0 = Agent Productization**

目标：

> 把 EVIF 收敛成一个对 AI Agent 有明确价值主张、明确主航道、明确默认验证路径、明确分发边界的可接入平台。

### 7.1 成功标准

当以下条件满足时，MVP 6.0 才算真正完成：

1. 新用户能在 5~10 分钟内跑通一个 Agent 工作流
2. MCP 成为最清晰的主入口之一
3. `/context + /skills + /pipes + /memories` 被包装成清晰产品叙事
4. 仓库中的 generated artifacts / 历史残留 / 试验性目录得到治理
5. 有一套默认可信的验证命令证明“主价值可用”
6. 后续 roadmap 不再按“功能堆叠”推进，而按“产品闭环”推进
7. EVIF 能清楚证明自己是一个连接万物的 Agent 连接层，而不只是插件集合
8. EVIF 能清楚证明自己能降低 Agent 的 token / 接入 / 协作总成本

---

## 八、后续计划（已实现部分标记 ✅）

下面的计划不是泛泛 roadmap，而是按优先级收敛主线。

### Phase A：定义主航道（最高优先级）✅

目标：先决定 EVIF 到底先服务谁，以及默认从哪条路径进入。

#### A1. 明确产品主入口 ✅

已明确优先级：
1. **MCP** - AI Agent 核心入口
2. **CLI** - 人类开发者快速上手
3. **REST** - 平台服务化基础
4. Web UI - 演示和管理
5. FUSE - 高级场景
6. 其他 SDK

#### A2. 明确产品主叙事 ✅

已升级为：
> EVIF 是 AI Agent 的上下文、技能、协作、记忆与系统连接基础设施。

补充句：
> 它通过统一文件接口连接异构系统，并通过 compact、cache、structured retrieval 为 Agent 降低操作成本。

#### A3. 明确主流程 demo ✅

已固化官方 demo 脚本：`scripts/demo-agent-workflow.sh`

流程：
1. 启动 EVIF
2. 读取 `/context`
3. 写入 `L0/L1`
4. 发现 `/skills`
5. 创建 `/pipes`
6. 调用 MCP 工具完成一个实际任务

### Phase B：治理仓库边界与可导航性 ✅

目标：降低阅读成本、减少噪音、为 AI / 人类协作优化仓库。

#### B1. 清理生成物与缓存 ✅

已完善 `.gitignore`：
- `crates/evif-sdk-ts/dist/*` ✅
- 各类 `.pytest_cache` ✅
- `.backup` 残留文件 ✅
- 不应入仓的 build outputs ✅
- `console-*.log` ✅
- `.playwright-mcp/` ✅

#### B2. 重新定义目录分层

建议划分为：

- `crates/`：Rust 主平台
- `sdks/`：多语言 SDK
- `apps/`：Web UI 等应用
- `examples/`：演示项目
- `docs/`：面向用户与架构文档
- `archive/`：历史分析资料（如果保留）

这样比把所有内容都塞在 `crates/` 或根目录更清晰。

#### B3. 收敛 root 文档

当前根目录中 `mem*.md`、`mvp*.md` 很多，建议：

- 保留最新版主 roadmap
- 历史文档迁移到归档目录
- 对外只暴露少量权威文档

### Phase C：把 MCP 打造成一级产品面 ✅

目标：从”有 MCP 支持”升级到”以 MCP 为核心产品之一”。

#### C1. 定义 MCP 核心工具集 ✅

第一层已明确突出：
- context ✅
- skill ✅
- pipe ✅
- memory ✅
- essential fs ✅
- search / health ✅

其余工具视为扩展能力。

#### C2. 重构 MCP 文档与示例 ✅

已提供：
- README 中的真实工具列表 ✅
- 完整 Agent workflow 示例 (`scripts/demo-agent-workflow.sh`) ✅
- CLAUDE.md / AGENTS.md 模板 ✅

#### C3. 收敛 MCP 大文件复杂度

`crates/evif-mcp/src/lib.rs` 已经很大，后续应考虑按职责拆分为：

- tool catalog
- tool handlers
- caching
- prompt/template
- auth
- output filtering

目的是提升 locality 和 AI navigability。

#### C4. 把”低成本调用”做成 MCP 默认体验 ✅

已将以下能力做成默认策略：
- 紧凑返回优先 ✅ (bounded read max_lines=100)
- 分页/摘要优先 ✅ (compact search limit=3)
- 大对象延迟展开 ✅
- 重复查询缓存 ✅
- 大结果自动降采样 ✅

通过 `docs/cost-optimization-matrix.md` EVIF 已真正体现”为 Agent 降本”。

### Phase D：把”AI Agent 原语”产品化 ✅

目标：让 `/context`、`/skills`、`/pipes`、`/memories` 不只是底层插件，而是明确产品能力。

#### D1. 为四个原语写统一模型文档 ✅

分别说明：

- 它们解决什么问题
- 面向谁
- 典型工作流是什么
- CLI / REST / MCP 各自怎么接入

#### D2. 统一 naming 与 examples

官方样例已统一：

- 单 Agent 工作流 ✅ (CLAUDE.md/AGENTS.md 模板)
- 多 Agent 协作 ✅ (pipefs 文档)
- 长期记忆检索 ✅ (evif-memory skill)
- 技能复用 ✅ (evif-workflows skill, skillfs)

#### D3. 为 AI 原语建立单独验证集 ✅

已建立主价值验证脚本 `scripts/verify-main-value.sh`：

- context 持续性 ✅
- skill 发现/执行 ✅
- pipe 协作成功 ✅
- memory 检索命中 ✅

#### D4. 把”连接器能力”纳入官方产品模型 ✅

已建立统一分类 `docs/connector-capability-matrix.md`：

- storage connectors ✅
- application connectors ✅
- collaboration connectors ✅
- knowledge connectors ✅
- runtime connectors ✅

并给出统一的配置、能力、限制和验证方式。

### Phase E：建立”连接万物 + 降本”的产品闭环 ✅

目标：把这两条价值从隐含特性变成官方可验证能力。

#### E1. 建立连接能力矩阵 ✅

`docs/connector-capability-matrix.md` 已为每类 connector 记录：

- 连接对象 ✅
- 认证方式 ✅
- 支持读/写/搜索/流式/事件的哪些能力 ✅
- 是否适合 Agent 主路径 ✅

#### E2. 建立降本能力矩阵 ✅

`docs/cost-optimization-matrix.md` 已为每类核心操作记录：

- 默认返回大小 ✅
- 是否支持 compact ✅
- 是否支持 cache ✅
- 是否支持 batch ✅
- 是否支持摘要化/分页 ✅

#### E3. 建立成本收益示例 ✅

已在 `docs/cost-optimization-matrix.md` 给出 3 类真实例子：

- 读取大文件时如何减少 token (bounded read 节省 60-90%)
- 跨系统检索时如何减少重复接入代码 (mount 模型)
- 多 Agent 协作时如何减少 prompt 中转成本 (pipes/queue)

#### E4. 建立 capability contract ✅

`docs/connector-capability-matrix.md` 已为每个 connector 和 Agent 原语定义统一 contract：

```yaml
id: contextfs
category: agent-primitive
support_tier: core
capabilities:
  discover: true
  read: true
  write: true
  coordinate: false
  optimize: true
cost_policy:
  bounded_read: true
  compact_default: true
  full_output_requires_opt_in: true
security:
  requires_auth: configurable
  audit_events: true
```

这个 contract 后续可驱动：

- MCP tool 描述 ✅
- REST capability endpoint ✅
- SDK 类型生成 ✅
- connector conformance tests ✅
- 文档矩阵自动生成 ✅

#### E5. 建立 Agent workflow envelope ✅

`docs/connector-capability-matrix.md` 已为 `/skills`、`/pipes`、`/queue`、MCP tool call 建立统一 envelope：

```json
{
  “input”: {},
  “status”: “pending|running|done|failed”,
  “output”: {},
  “error”: null,
  “metadata”: {},
  “trace”: [],
  “cost”: {
    “estimated_tokens”: 0,
    “tool_calls”: 0
  }
}
```

通过 pipefs/queuefs envelope 定义，”Agent 工作流”已从隐式 prompt 变成显式、可追踪、可恢复的文件化协议。

### Phase F：收敛默认可信验证路径 ✅

目标：定义”通过这些检查，就说明 EVIF 主价值可用”。

已形成三层发布门禁：

#### F1. 快速门禁 ✅

- `cargo test -p evif-core --quiet` ✅
- `cargo test -p evif-mcp --quiet` ✅
- `cargo test -p evif-rest --quiet` ✅

#### F2. 主价值门禁 ✅

已围绕以下跑最小端到端验证：

- context ✅
- skill ✅
- pipe ✅
- MCP ✅

验证脚本：`scripts/verify-main-value.sh`

#### E3. 扩展门禁

- 前端 E2E
- 可选 plugin tests
- memory 扩展场景

重点是把主价值门禁和扩展门禁分开，不再把所有验证混成一个“大杂烩完成度”。

### Phase G：SDK 与分发策略收口

目标：让 SDK 真正成为接入层，而不是仓库噪音来源。

#### F1. 明确 SDK 优先级

建议优先级：

1. Python SDK
2. TypeScript SDK
3. Go SDK

理由：Python 最贴近 Agent 生态，TS 次之。

#### F2. SDK 目录独立化

把不属于 Rust workspace 的 SDK 迁移出 `crates/` 语义区，避免误导。

#### F3. 统一版本与发布策略

每个 SDK 需要明确：

- 是否官方支持
- 版本如何跟随主仓
- 是否允许生成物入仓
- 验证标准是什么

### Phase H：前端降噪但保留价值

目标：把 Web UI 从“潜在主线”降为“优秀辅助面”。

#### G1. 定位调整

前端主要承担：

- demo
- 可视化管理
- 调试辅助

而不是产品主入口。

#### G2. 补齐最关键缺口

只修最影响演示和主流程的问题，不在前端上继续铺大摊子。

#### G3. 与主路径对齐

前端应围绕官方 demo，而不是独立发展另一套复杂产品目标。

---

## 九、建议的里程碑拆分

### Milestone 1：主航道收敛

交付物：

- 更新后的 README
- 一条官方 demo
- 明确主入口排序
- 统一产品定位语句

### Milestone 2：仓库治理

交付物：

- 目录收敛
- 生成物清理
- 历史文档归档
- 代码导航成本下降

### Milestone 3：MCP 一级产品化

交付物：

- MCP 最小官方接入文档
- MCP 核心工具集定义
- 真实 Agent workflow 示例
- 必要的模块拆分计划

### Milestone 4：连接层与降本能力产品化

交付物：

- connector capability matrix
- 成本优化能力矩阵
- 2~3 个低成本 Agent workflow 样例
- 默认 compact / cache / bounded retrieval 策略
- connector capability contract 草案
- Agent workflow envelope 草案

### Milestone 5：Agent 原语验证闭环

交付物：

- context / skill / pipe / memory 的主流程测试
- 一套默认可信门禁
- 发布口径一致

### Milestone 6：SDK/前端分发收口

交付物：

- SDK 分层清晰
- Web UI 定位清晰
- 仓库边界清楚

---

## 十、最终建议

### 10.1 现在最不该做的事

- 不要继续无上限扩展插件数量
- 不要继续把所有入口都当同等优先级
- 不要继续堆新的 roadmap 文档而不收敛旧文档
- 不要把“测试数量很多”当作“产品已闭环”

### 10.2 现在最该做的事

- 先定义 EVIF 的主产品面：**MCP + Context + Skills + Pipes + Memory**
- 先把“连接万物”和“降低成本”明确成官方价值主张
- 先定义默认上手路径与官方 demo
- 先治理仓库边界和产物噪音
- 先建立主价值验证集

### 10.3 一句话结论

EVIF 已经拥有成为 **AI Agent 基础设施平台** 的核心骨架；  
MVP 6.0 的关键，不是再“增加功能”，而是把现有能力 **收敛成清晰、可信、可接入、可演示、能连接万物并能为 Agent 降本的产品闭环**。

---

## 十一、建议的下一步执行清单

- [x] 重写根 `README.md`，把产品主叙事收敛到 Agent Infra
- [x] 输出一份 connector capability matrix，明确 EVIF 连接层能力
- [x] 输出一份 cost-optimization matrix，明确 EVIF 的 Agent 降本策略
- [x] 定义官方最小 demo：`context + skills + pipes + MCP` ✅ (scripts/demo-agent-workflow.sh)
- [x] 盘点并清理 generated artifacts / `.backup` / 缓存残留 ✅ (.gitignore 完善)
- [x] 给 `evif-mcp` 输出单独的产品化设计文档 ✅ (docs/mcp-product-design.md)
- [x] 起草 connector capability contract，并映射到现有 plugin catalog ✅ (docs/connector-capability-matrix.md)
- [x] 起草 Agent workflow envelope，并映射到 `/skills`、`/pipes`、`/queue` ✅ (connector-capability-matrix.md 中包含 pipefs/queuefs envelope)
- [x] 明确 8 个核心 Kernel 的 owner、接口和验证标准 ✅ (docs/kernel-definitions.md)
- [x] 建立主价值验证脚本，只覆盖默认主路径 ✅ (scripts/verify-main-value.sh)
- [ ] 重新分层 SDK / Web / examples / archive 目录
- [ ] 收敛现有 `mvp*.md` / `mem*.md` 文档体系

---

## 十二、核心功能完善优先级

综合当前代码成熟度、AI Agent 价值和产品收敛难度，建议按以下顺序完善核心功能：

### P0：先做产品主闭环

目标：让用户和 Agent 明确知道 EVIF 先解决什么问题。

- MCP core tool set：context / skill / pipe / memory / bounded fs
- 官方 demo：`context + skill + pipe + memory_search`
- low-cost defaults：bounded read、compact search、output filter
- README / docs / mvp6.0 口径统一

### P1：再做连接层 contract ✅

目标：让”连接万物”从插件清单变成稳定产品能力。

- connector capability contract ✅ (docs/connector-capability-matrix.md)
- connector support tiers：core / stable / experimental ✅
- connector conformance tests ✅ (已定义测试命令)
- connector capability endpoint / MCP resource ✅

### P2：再做 Agent workflow envelope ✅

目标：让 Agent 的技能执行、pipe 协作、queue 任务统一形状。

- 标准 envelope schema ✅
- skill execution result 写入 envelope ✅
- pipe task 使用 envelope ✅
- queue task 使用 envelope ✅
- cost / trace 字段进入 envelope ✅

### P3：再做治理与安全

目标：让 EVIF 可以安全连接更多系统。

- connector credential model
- per-mount permissions
- audit trail
- sensitive output masking
- memory/context encryption policy

### P4：最后扩展生态

目标：在主闭环稳定后再扩张。

- Python SDK 优先
- TypeScript SDK 次之
- Go SDK 保持系统集成定位
- Web UI 作为 demo / admin / debugging surface
