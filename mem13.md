# EVIF mem13.md — 定位、架构重设计与后续计划

> 创建时间：2026-03-31
> 基于：EVIF 代码全面审计 + AGFS 对标分析 + AI Agent 文件系统行业调研

---

## 一、项目定位重定义

### 1.1 当前定位（模糊）

EVIF 目前是一个"虚拟文件系统插件平台"——功能丰富但定位不清，缺乏核心叙事。

### 1.2 建议定位

**EVIF = AI Agent 的 Meta Tool 基础设施**

核心理念（对标 AGFS "Everything is a File" + OpenViking Context DB）：

```
给 AI Agent 一个文件系统 + bash + MCP，让它自己组合工作流，
而不是给它 50 个硬编码的 API 函数。
```

**一句话定位**：
> EVIF 是面向 AI Agent 的统一文件系统抽象层（Meta Tool），为 Claude Code / Codex / Cursor 等 AI 编程助手提供可组合的存储、协调、记忆基础设施。

### 1.3 竞品对比

| 维度 | AGFS (Go) | EVIF (Rust) | 差距 |
|------|-----------|-------------|------|
| 核心语言 | Go | Rust | EVIF 性能更优 |
| 文件系统插件 | 14 种 | 32 种 | **EVIF 领先 2x** |
| WASM 插件 | Extism 单后端 | Extism + Wasmtime 双后端 | **EVIF 领先** |
| S3 支持 | 基础 | AWS SDK + OpenDAL 双通道 | **EVIF 领先** |
| 中国云支持 | 无 | 阿里云/腾讯云/华为云 OSS | **EVIF 独有** |
| MCP 工具 | 基础 6 个 | 17 个工具 | **EVIF 领先** |
| CLI 命令 | 基础 | 40+ Unix 风格命令 | **EVIF 领先** |
| REST API | 基础 | 完整 CRUD + WebSocket | **EVIF 领先** |
| FUSE 挂载 | 仅 Linux | Linux + macOS | **EVIF 领先** |
| 认证系统 | 无 | RBAC + 审计日志 | **EVIF 独有** |
| 多 Agent 协调 | QueueFS 邮箱 | QueueFS（Memory/SQLite/MySQL） | **持平** |
| Vector/语义搜索 | VectorFS (S3+TiDB) | VectorFS (SQLite) | **AGFS 领先** |
| Context DB | OpenViking (L0/L1/L2) | 无 | **AGFS 大幅领先** |
| SDK 语言 | Go + Python | Go | **AGFS 领先** |
| Shell/Web UI | agfs-shell + Web | 无 | **AGFS 领先** |
| Meta Tool 叙事 | 清晰突出 | 无 | **AGFS 领先** |
| 生产案例 | OpenViking/字节跳动 | 无 | **AGFS 领先** |

### 1.4 核心差距分析

**EVIF 的技术实现 > AGFS，但叙事和生态 < AGFS。**

EVIF 在插件数量、WASM 能力、CLI、认证等维度全面超越 AGFS，但：
1. **没有 Context DB** — 无法作为 AI Agent 的长期记忆层
2. **没有 Web Shell** — 缺少交互式调试界面
3. **SDK 不全** — 缺少 Python/TypeScript SDK
4. **叙事不清** — 没有 "Meta Tool" 定位
5. **没有实际 AI Agent 集成案例** — 未与 Claude Code / Codex 深度整合

---

## 二、架构重设计

### 2.1 当前架构

```
┌──────────────────────────────────────────┐
│  CLI  │  REST API  │  FUSE  │  MCP Tools │  ← 接入层
├──────────────────────────────────────────┤
│  Mount Table (Radix) │ Plugin System     │  ← 核心层
├──────────────────────────────────────────┤
│  MemFS │ S3FS │ SQLFS │ QueueFS │ ...   │  ← 插件层
└──────────────────────────────────────────┘
```

### 2.2 目标架构（AI Agent Meta Tool）

```
┌────────────────────────────────────────────────────────────────┐
│                      接入层 (Access Layer)                      │
│  ┌─────────┐ ┌──────────┐ ┌──────┐ ┌────────┐ ┌────────────┐ │
│  │Claude   │ │ Codex    │ │ REST │ │  FUSE  │ │  Web Shell │ │
│  │Code MCP │ │ CLI MCP  │ │ API  │ │ Mount  │ │  (新增)    │ │
│  └────┬────┘ └────┬─────┘ └──┬───┘ └───┬────┘ └─────┬──────┘ │
├───────┴───────────┴──────────┴─────────┴─────────────┴────────┤
│                    核心层 (Core Engine)                         │
│  ┌──────────────┐ ┌──────────────┐ ┌────────────────────────┐ │
│  │ Radix Mount  │ │ Plugin Loader│ │ Context Engine (新增)  │ │
│  │ Table        │ │ (WASM/Native)│ │ L0/L1/L2 分层加载     │ │
│  └──────────────┘ └──────────────┘ └────────────────────────┘ │
├────────────────────────────────────────────────────────────────┤
│                   存储层 (Storage Plugins)                      │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ Agent Memory Plugins (新增)                              │  │
│  │ ┌──────────┐ ┌──────────┐ ┌───────────┐ ┌────────────┐ │  │
│  │ │ContextFS │ │MemoryFS  │ │SessionFS  │ │SkillFS     │ │  │
│  │ │L0/L1/L2  │ │长期记忆  │ │会话存储   │ │技能注册    │ │  │
│  │ └──────────┘ └──────────┘ └───────────┘ └────────────┘ │  │
│  ├──────────────────────────────────────────────────────────┤  │
│  │ Coordination Plugins                                     │  │
│  │ ┌──────────┐ ┌──────────┐ ┌───────────┐ ┌────────────┐ │  │
│  │ │QueueFS   │ │StreamFS  │ │HeartbeatFS│ │PipeFS(新增)│ │  │
│  │ │消息队列  │ │流式数据  │ │心跳监控   │ │Agent管道   │ │  │
│  │ └──────────┘ └──────────┘ └───────────┘ └────────────┘ │  │
│  ├──────────────────────────────────────────────────────────┤  │
│  │ Storage Plugins                                          │  │
│  │ ┌────┐┌────┐┌────┐┌──────┐┌──────┐┌─────┐┌────────────┐│  │
│  │ │Mem ││S3  ││SQL ││Azure ││ GCS  ││ OSS ││ Encrypted  ││  │
│  │ └────┘└────┘└────┘└──────┘└──────┘└─────┘└────────────┘│  │
│  └──────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────┘
```

### 2.3 关键新增组件

#### A. Context Engine（上下文引擎）— 核心差异化

对标 OpenViking 的 L0/L1/L2 分层加载：

```
L0 (Immediate)  → 当前文件、当前工作上下文        — 毫秒级
L1 (Session)    → 本次会话的记忆、中间结果         — 秒级
L2 (Background) → 项目知识库、历史经验、最佳实践    — 分钟级
```

实现方式：
- ContextFS 插件 — 自动管理 L0/L1/L2 层级
- 基于 VectorFS 的语义检索
- YAML frontmatter 元数据（已有 evif-mem 基础）
- 自动上下文压缩和摘要

#### B. Web Shell（Web 交互界面）— 运维 + Agent 调试

对标 AGFS Shell + Web UI：
- React/Vite 前端
- 实时文件浏览和操作
- Agent 会话监控
- 队列可视化
- 插件状态仪表盘

#### C. PipeFS（Agent 管道文件系统）— 多 Agent 协调

```
# Agent A 写入任务
echo "analyze https://example.com" > /pipes/task-001/input

# Agent B 读取任务
cat /pipes/task-001/input

# Agent B 写入结果
echo "analysis result..." > /pipes/task-001/output

# 监控管道状态
cat /pipes/task-001/status    # → "completed"
```

#### D. SkillFS（技能注册文件系统）— Meta Tool 体现

```
/skills/
├── code-review/          # 代码审查技能
│   ├── manifest.yaml     # 技能描述
│   ├── input/            # 输入规范
│   └── output/           # 输出规范
├── test-gen/             # 测试生成技能
├── doc-gen/              # 文档生成技能
└── deploy/               # 部署技能
```

Agent 通过 `ls /skills` 发现可用技能，通过 `cat /skills/code-review/manifest.yaml` 了解如何调用。

---

## 三、问题清单（从代码审计中发现）

### 3.1 架构级问题

| # | 问题 | 影响 | 优先级 |
|---|------|------|--------|
| A1 | 无 Context Engine — Agent 无长期记忆 | AI Agent 无法跨会话学习 | P0 |
| A2 | 无 Web Shell — 缺少交互式调试 | 开发体验差 | P1 |
| A3 | MCP 工具未与 Claude Code 深度集成 | Meta Tool 叙事无法落地 | P0 |
| A4 | evif-mem 未被任何插件实际使用 | 记忆系统形同虚设 | P0 |
| A5 | 无多 Agent 协调原语（PipeFS） | 多 Agent 场景受限 | P1 |
| A6 | 无 SDK 生成器（Python/TS） | 生态覆盖不足 | P1 |

### 3.2 代码级问题

| # | 问题 | 文件 | 优先级 |
|---|------|------|--------|
| C1 | WebDAV/FTP/SFTP 因 OpenDAL TLS 冲突禁用 | evif-plugins/Cargo.toml | P2 |
| C2 | TypeScript SDK 空壳 | evif-sdk-ts/ | P1 |
| C3 | Python SDK 空壳 | evif-sdk-python/ | P1 |
| C4 | evif-metrics 仅基础框架 | evif-metrics/src/ | P2 |
| C5 | 部分插件缺少独立单元测试 | 多个插件 | P2 |
| C6 | 示例 WASM 插件 extism-pdk API 不稳定 | examples/wasm-plugin/ | P2 |
| C7 | Go SDK 缺少错误重试和断路器 | evif-sdk-go/ | P2 |
| C8 | REST API 缺少 OpenAPI/Swagger 文档 | evif-rest/ | P2 |

### 3.3 定位与叙事问题

| # | 问题 | 影响 | 优先级 |
|---|------|------|--------|
| N1 | 无 "Meta Tool" 定位叙事 | 与 AGFS 竞争时缺乏故事 | P0 |
| N2 | 无 README.md 重写 | 访客无法快速理解价值 | P0 |
| N3 | 无架构图和快速上手指南 | 新用户门槛高 | P1 |
| N4 | 无 AI Agent 集成示例 | 潜在用户看不到用途 | P0 |

---

## 四、后续实施计划

### Phase 8: AI Agent 集成层（核心差异化，预估 40h）

**目标**：让 EVIF 成为 Claude Code / Codex 的 Meta Tool

#### 8.1 Context Engine — 上下文引擎（P0，15h）

- [ ] `ContextFS` 插件 — L0/L1/L2 分层上下文管理
  - [ ] L0: 当前工作上下文（文件内容 + 光标位置 + 最近操作）
  - [ ] L1: 会话记忆（中间结果 + 推理链 + 决策记录）
  - [ ] L2: 项目知识库（架构文档 + 最佳实践 + 历史经验）
- [ ] 自动上下文压缩（长上下文 → 摘要 + 关键信息）
- [ ] 上下文持久化（跨会话恢复）
- [ ] 与 evif-mem 集成（利用现有 Memory Platform）
- [ ] MCP 工具暴露：`context_load`、`context_save`、`context_search`

#### 8.2 Claude Code 深度集成（P0，10h）

- [ ] EVIF MCP Server 注册为 Claude Code MCP Server
- [ ] 新增 MCP 工具：
  - [ ] `evif_project_init` — 初始化 EVIF 项目（自动挂载推荐插件）
  - [ ] `evif_context` — 加载/保存项目上下文
  - [ ] `evif_agent_pipe` — 创建 Agent 管道
  - [ ] `evif_skill_list` — 列出可用技能
  - [ ] `evif_memory_search` — 语义搜索项目记忆
- [ ] Claude Code CLAUDE.md 集成指南
- [ ] Codex CLI AGENTS.md 集成指南

#### 8.3 PipeFS — Agent 管道文件系统（P1，8h）

- [ ] `PipeFS` 插件 — Agent 间通信管道
  - [ ] 创建管道：`mkdir /pipes/task-001`
  - [ ] 输入/输出：读写 `/pipes/task-001/input`、`/pipes/task-001/output`
  - [ ] 状态监控：`cat /pipes/task-001/status`
  - [ ] 超时和自动清理
  - [ ] 广播模式：一个写入 → 多个读取
- [ ] 基于 QueueFS 扩展

#### 8.4 SkillFS — 技能注册文件系统（P1，7h）

- [ ] `SkillFS` 插件 — Agent 技能发现和调用
- [ ] YAML manifest 规范
- [ ] 内置技能模板（code-review、test-gen、doc-gen）
- [ ] Agent 自注册技能
- [ ] MCP 工具：`skill_register`、`skill_discover`、`skill_execute`

### Phase 9: 开发者体验（预估 25h）

#### 9.1 Python SDK（P1，8h）

- [ ] HTTP 客户端（requests/httpx）
- [ ] 异步客户端（asyncio + httpx）
- [ ] 文件操作、挂载管理、句柄操作
- [ ] Context API（上下文加载/保存）
- [ ] 流式读写支持
- [ ] 完整测试套件

#### 9.2 TypeScript SDK（P1，8h）

- [ ] Node.js 客户端（fetch API）
- [ ] 类型定义（TypeScript）
- [ ] 文件操作、挂载管理、句柄操作
- [ ] Context API
- [ ] 流式读写支持
- [ ] 完整测试套件

#### 9.3 Web Shell（P1，9h）

- [ ] React + Vite 前端框架
- [ ] 文件浏览器（树形 + 列表视图）
- [ ] 在线文件编辑器
- [ ] 插件状态仪表盘
- [ ] 队列可视化
- [ ] 实时日志流
- [ ] REST API 交互式文档

### Phase 10: 生产增强（预估 15h）

#### 10.1 OpenAPI 文档（P2，3h）

- [ ] 自动生成 OpenAPI 3.0 spec
- [ ] Swagger UI 集成
- [ ] API 示例和说明

#### 10.2 Metrics 增强（P2，4h）

- [ ] Prometheus metrics 完整实现
- [ ] Grafana 仪表盘模板
- [ ] 插件级性能指标
- [ ] 请求延迟分布

#### 10.3 Go SDK 增强（P2，3h）

- [ ] 错误重试（指数退避）
- [ ] 断路器模式
- [ ] 连接池管理
- [ ] Context API 集成

#### 10.4 CI/CD 完善（P2，5h）

- [ ] 多平台 Release 自动化（GoReleaser 风格）
- [ ] Docker 镜像构建和推送
- [ ] 自动 changelog 生成
- [ ] 性能基准测试 CI

---

## 五、核心设计原则

### 5.1 Meta Tool 设计哲学

```
原则 1：文件系统是 AI Agent 的通用接口
  → 任何 LLM 都知道 cat/ls/grep/echo，无需学习新 API

原则 2：组合优于硬编码
  → 给 Agent 原子操作（read/write/list/search），让它自己组合

原则 3：分层上下文是 Agent 智能的基础
  → L0（即时）→ L1（会话）→ L2（知识）分层加载

原则 4：Agent 间通过文件系统协调
  → QueueFS/PipeFS/StreamFS 替代 API 调用

原则 5：一切皆可观测
  → 文件系统天然可审计（ls/cat/grep 即可调试）
```

### 5.2 与 Claude Code 集成模式

```bash
# 模式 1: 作为 MCP Server
# claude_desktop_config.json
{
  "mcpServers": {
    "evif": {
      "command": "evif-mcp",
      "args": ["--config", "evif.toml"]
    }
  }
}

# 模式 2: 作为文件系统后端
evif-fuse mount /evif --config evif.toml
# Agent 直接操作 /evif/ 路径

# 模式 3: 作为 REST API
curl http://localhost:8080/api/v1/files?path=/context/L2/architecture.md
```

### 5.3 与 Codex CLI 集成模式

```bash
# Codex 通过 REST API 使用 EVIF
# AGENTS.md 配置
export EVIF_URL=http://localhost:8080/api/v1

# Agent 自动发现上下文
curl $EVIF_URL/context/L0/current
curl $EVIF_URL/context/L2/knowledge-base
```

---

## 六、优先级排序与里程碑

### M1: "AI Agent Meta Tool" 基础版（Phase 8.1 + 8.2）

**里程碑**：EVIF 可以作为 Claude Code 的上下文管理后端

- Context Engine (L0/L1/L2)
- Claude Code MCP 集成
- 重写 README.md（Meta Tool 叙事）

### M2: "Multi-Agent 平台"（Phase 8.3 + 8.4）

**里程碑**：多个 AI Agent 可以通过 EVIF 协调工作

- PipeFS 多 Agent 管道
- SkillFS 技能注册
- 多 Agent 示例

### M3: "开发者生态"（Phase 9）

**里程碑**：开发者可以用任意语言接入 EVIF

- Python SDK
- TypeScript SDK
- Web Shell

### M4: "生产就绪"（Phase 10）

**里程碑**：EVIF 可以在生产环境部署

- OpenAPI 文档
- Metrics + Grafana
- CI/CD 完善

---

## 七、与 mem12.md 的关系

mem12.md 记录了 Phase 4-7 的实施计划，**已全部完成**：
- Phase 4: WASM 插件系统（Extism + Wasmtime）✅
- Phase 5: S3 分片上传 ✅
- Phase 6: 存储后端增强（VectorFS SQLite, SQLFS, QueueFS MySQL）✅
- Phase 7: 生产增强（ProxyFS 健康检查, CI 每日构建）✅

mem13.md 从 mem12.md 基础上延伸，将 EVIF 从"虚拟文件系统插件平台"升级为 **"AI Agent Meta Tool 基础设施"**。

---

## 八、参考资料

### 行业调研
- [AGFS: File Systems for AI Agents](https://langcopilot.com/posts/2025-12-04-file-systems-for-ai-agents-next)
- [OpenViking: Context DB for AI Agents](https://github.com/volcengine/OpenViking)
- [Everything is Context (arXiv:2512.05470)](https://arxiv.org/abs/2512.05470)
- [Meta-Tool: Unleash Open-World Function Calling (ACL 2025)](https://aclanthology.org/2025.acl-long.1481.pdf)
- [AIOS-LSFS: LLM-based Semantic File System (ICLR 2025)](https://github.com/agiresearch/AIOS-LSFS)
- [LangChain: How Agents Can Use Filesystems](https://blog.langchain.com/how-agents-can-use-filesystems-for-context-engineering/)
- [LlamaIndex: Files Are All You Need](https://www.llamaindex.ai/blog/files-are-all-you-need)

### 技术参考
- [MCP Specification](https://modelcontextprotocol.io/specification/2025-06-18)
- [Claude Code MCP Integration](https://www.anthropic.com/engineering/advanced-tool-use)
- [Codex CLI](https://github.com/openai/codex)
- [AGFS GitHub](https://github.com/c4pt0r/agfs)
