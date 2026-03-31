# EVIF mem13.md — 定位、架构重设计与后续计划（v2）

> 创建时间：2026-03-31
> 更新时间：2026-03-31（v2：基于深度调研重写）
> 基于：EVIF 全面代码审计 + AGFS 源码分析 + 行业深度调研（20+ 源）
> 调研范围：AGFS/OpenViking/Claude Code/Codex CLI/MCP/Skills/arXiv 论文

---

## 一、核心理念：从 "Everything is File" 到 "Context is File"

### 1.1 哲学基础

Unix 的伟大创新是 "Everything is a file"——将设备、管道、网络、进程统一为文件接口。
Plan 9 进一步强化了这个理念。AGFS 将其引入 AI 时代："Everything is a file, for AI agents."

**EVIF 的核心哲学升级为：**

```
Everything is Context → Context is File → File is the Meta Tool
```

这意味着：
1. AI Agent 需要的一切都是「上下文」（代码、记忆、知识、技能、任务、状态）
2. 所有上下文都可以映射为文件（目录 = 组织，文件 = 内容，管道 = 通信）
3. 文件操作（ls/cat/grep/write）就是 Agent 的 Meta Tool——无需学习新 API

### 1.2 为什么文件系统是 AI Agent 的最佳接口

**学术验证**：
- arXiv:2512.05470 "Everything is Context: Agentic File System Abstraction for Context Engineering" (CSIRO/Data61, 2025) — 明确提出文件系统抽象是上下文工程的最佳接口
- arXiv:2601.22037 "Optimizing Agentic Workflows using Meta-tools" (2026) — 实证 Meta Tool 减少 11.9% LLM 调用，提升 4.2% 任务成功率

**工业验证**：
- Anthropic 的 Claude Code **核心上下文管理基于 ls/grep/write/read 等元命令**，而非 MCP 工具
- OpenViking（字节跳动）采用 `viking://` 文件系统协议作为 Context DB 的统一接口
- OpenAI Codex CLI 使用 `AGENTS.md` 文件作为 Agent 配置接口
- LangChain 和 LlamaIndex 均验证 "Files are all you need" 范式

**关键洞察**：
> Anthropic 自己说："Claude Code employs a hybrid model: CLAUDE.md files are loaded upfront,
> while **primitives like glob and grep allow it to navigate its environment just-in-time**."
> 文件系统元命令是 Claude Code 的核心，MCP 是补充。

### 1.3 MCP 的角色定位：补充而非核心

**当前 EVIF 的误区**：把 MCP 作为 AI Agent 的主要接入方式。

**修正**：
```
Layer 0（核心）：文件系统元命令 — ls/cat/grep/write/mkdir/rm
  → 任何 LLM 都天生理解，无需学习
  → Claude Code 的主要工作方式
  → 通过 FUSE 挂载、CLI、REST API 暴露

Layer 1（可选）：MCP Tools — 结构化工具调用
  → 用于需要类型安全的场景
  → 用于 Claude Desktop 等不支持直接文件操作的客户端
  → 17 个现有 MCP 工具保留，作为便捷入口

Layer 2（高阶）：Skills / Meta-Tools — 组合式工作流
  → 基于 Claude Code SKILL.md 格式
  → 声明式 YAML 发现 + Markdown 指令
  → Agent 通过 ls /skills 发现，cat /skills/*/manifest.yaml 理解
```

---

## 二、项目定位

### 2.1 一句话定位

> **EVIF = AI Agent 的 Context FileSystem（上下文文件系统）**
> 为 Claude Code / Codex / Cursor 提供 "Context is File" 的统一基础设施。

### 2.2 与 AGFS 的战略差异

AGFS 的定位是 "File System for AI Agents" — 一个通用的虚拟文件系统。
EVIF 的定位是 **"Context FileSystem for AI Agents"** — 专注于上下文管理。

| 维度 | AGFS | EVIF | 战略差异 |
|------|------|------|----------|
| 核心语言 | Go | Rust | 性能和安全性 |
| 定位 | 通用虚拟文件系统 | **上下文文件系统** | EVIF 聚焦 Agent 上下文 |
| Context DB | OpenViking（外挂） | **内建 L0/L1/L2** | EVIF 原生集成 |
| Agent 集成 | MCP 为主 | **文件元命令为主，MCP 为辅** | EVIF 更贴近 Claude Code |
| 技能系统 | 无 | **SkillFS 声明式技能发现** | EVIF 独有 |
| 中国云 | 无 | 阿里云/腾讯云/华为云 | EVIF 独有 |
| WASM 插件 | Extism 单后端 | Extism + Wasmtime 双后端 | EVIF 领先 |
| CLI | 基础 | 40+ Unix 命令 | EVIF 领先 |
| 认证 | 无 | RBAC + 审计 | EVIF 领先 |
| 语义搜索 | S3+TiDB（重） | SQLite（轻） | 各有优势 |
| SDK | Go+Python | Go（Python/TS 待实现） | 待补齐 |

### 2.3 核心差异化：三件 AGFS 没做到的事

1. **Context Engine（上下文引擎）** — 内建 L0/L1/L2 分层上下文，不是外挂数据库
2. **SkillFS（技能文件系统）** — Agent 通过文件系统发现和注册技能（对标 Claude Code SKILL.md）
3. **原生文件元命令优先** — 不依赖 MCP，直接通过 ls/cat/grep/write 工作

---

## 三、架构设计

### 3.1 三层架构

```
┌─────────────────────────────────────────────────────────────┐
│                   Agent Access Layer                         │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  File Primitives (Primary)                          │    │
│  │  ls / cat / grep / write / mkdir / rm / find        │    │
│  │  → FUSE mount: Agent 直接操作文件路径                 │    │
│  │  → CLI: evif cat /evif/context/L0/current           │    │
│  │  → REST: GET /api/v1/files?path=/context/L0/...     │    │
│  └─────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  MCP Tools (Secondary)                              │    │
│  │  17 structured tools for Claude Desktop / MCP clients│   │
│  └─────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  Skills (Tertiary)                                  │    │
│  │  SKILL.md format, declarative discovery              │    │
│  │  ls /skills → discover, cat → understand             │    │
│  └─────────────────────────────────────────────────────┘    │
├─────────────────────────────────────────────────────────────┤
│                    Core Engine                               │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────────────┐ │
│  │ Radix Mount  │ │ Plugin Loader│ │ Context Engine       │ │
│  │ Table (O(k)) │ │ WASM/Native  │ │ L0/L1/L2 管理器      │ │
│  └──────────────┘ └──────────────┘ └──────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                    Plugin Layer                              │
│                                                             │
│  ┌─ Context Plugins ──────────────────────────────────────┐ │
│  │ ContextFS (L0/L1/L2) │ MemoryFS │ SessionFS │ SkillFS │ │
│  └─────────────────────────────────────────────────────────┘ │
│  ┌─ Coordination Plugins ─────────────────────────────────┐ │
│  │ QueueFS │ PipeFS (Agent管道) │ HeartbeatFS │ StreamFS  │ │
│  └─────────────────────────────────────────────────────────┘ │
│  ┌─ Storage Plugins ──────────────────────────────────────┐ │
│  │ MemFS │ S3FS │ SQLFS │ Azure │ GCS │ OSS │ EncryptFS  │ │
│  └─────────────────────────────────────────────────────────┘ │
│  ┌─ Compute Plugins ──────────────────────────────────────┐ │
│  │ VectorFS │ GPTFS │ ProxyFS │ WASM Runtime             │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 Context Engine 设计（核心差异化）

**对标**：OpenViking 的 L0/L1/L2 分层 + Anthropic 的上下文压缩策略

**设计原则**：
- **Progressive Disclosure**：先加载 L0 摘要（~100 tokens/项），按需深入 L1/L2
- **Compaction**：自动压缩长上下文为摘要，保留关键决策
- **Just-in-Time**：维护轻量级标识符（路径、查询、链接），运行时按需加载

```
/context/
├── L0/                          # 即时上下文（毫秒级，~200 tokens）
│   ├── current                  # 当前工作上下文
│   ├── recent_ops               # 最近操作记录
│   └── active_files/            # 当前打开的文件
│       ├── main.rs              # 文件摘要（前 20 行 + 结构概要）
│       └── config.toml
│
├── L1/                          # 会话上下文（秒级，~2000 tokens）
│   ├── session_id               # 当前会话标识
│   ├── decisions.md             # 本次会话的决策记录
│   ├── intermediate/            # 中间结果
│   │   ├── analysis_001.json
│   │   └── draft_plan.md
│   └── scratch/                 # 临时笔记（Agent 写入的推理过程）
│
├── L2/                          # 项目知识库（按需加载）
│   ├── architecture.md          # 项目架构文档
│   ├── patterns.md              # 代码模式和约定
│   ├── best_practices.md        # 最佳实践
│   ├── history/                 # 历史经验（成功/失败案例）
│   │   ├── bug_fix_001.md
│   │   └── feature_impl_002.md
│   └── embeddings/              # 语义索引（VectorFS）
│       └── *.vec
│
├── README                       # 上下文导航指南
└── .meta                        # 元数据（压缩策略、过期策略）
```

**Agent 交互方式**：
```bash
# 查看 L0 上下文（总是加载）
cat /context/L0/current

# 查看会话决策
cat /context/L1/decisions.md

# 搜索项目知识
grep "authentication" /context/L2/

# 语义搜索（通过 VectorFS）
echo "how does auth work?" > /context/L2/embeddings/search
cat /context/L2/embeddings/results

# 保存当前上下文
echo "decision: use JWT for auth" >> /context/L1/decisions.md
```

### 3.3 SkillFS 设计（对标 Claude Code SKILL.md）

**核心思想**：技能 = 一个目录 + manifest.yaml + 指令文件

```
/skills/
├── README                       # 技能导航
├── code-review/                 # 代码审查技能
│   ├── manifest.yaml            # 声明式描述（对标 SKILL.md frontmatter）
│   │   ---
│   │   name: code-review
│   │   description: "Review code for bugs, security issues, and best practices"
│   │   triggers: ["review", "code review", "check my code"]
│   │   input_schema:
│   │     path: { type: string, required: true }
│   │     focus: { type: string, enum: [security, performance, style, all] }
│   │   ---
│   ├── instructions.md          # 执行指令（对标 SKILL.md body）
│   ├── examples/                # 示例输入/输出
│   └── templates/               # 模板文件
│
├── test-gen/                    # 测试生成技能
├── doc-gen/                     # 文档生成技能
├── deploy/                      # 部署技能
└── refactor/                    # 重构技能
```

**Agent 发现和使用技能**：
```bash
# 发现所有技能
ls /skills/
# → code-review  test-gen  doc-gen  deploy  refactor

# 理解某个技能
cat /skills/code-review/manifest.yaml
cat /skills/code-review/instructions.md

# 执行技能（写入输入，读取输出）
echo '{"path":"/src/auth.rs","focus":"security"}' > /skills/code-review/input
cat /skills/code-review/output
```

**与 Claude Code Skills 的对应关系**：
```
SKILL.md frontmatter  → /skills/*/manifest.yaml
SKILL.md body         → /skills/*/instructions.md
Skill invocation      → write to /skills/*/input, read from /skills/*/output
Skill discovery       → ls /skills/, cat manifest.yaml
```

### 3.4 PipeFS 设计（多 Agent 协调）

```
/pipes/
├── README
├── task-001/
│   ├── input           # Agent A 写入任务描述
│   ├── output          # Agent B 写入处理结果
│   ├── status          # pending → running → completed/failed
│   ├── assignee        # 当前处理者
│   └── timeout         # 超时时间
├── task-002/
└── broadcast/
    ├── input           # 一个写入，多个读取
    └── subscribers/    # 订阅者列表
```

---

## 四、问题清单

### 4.1 战略级问题（P0）

| # | 问题 | 根因 | 解决方案 |
|---|------|------|----------|
| S1 | MCP 被当作主要接入方式 | 误解了 Claude Code 的工作模式 | 文件元命令优先，MCP 降级为辅助 |
| S2 | 无 Context Engine | evif-mem 未被实际使用 | 新建 ContextFS 插件，集成 evif-mem |
| S3 | 无 SkillFS | 未考虑技能发现场景 | 新建 SkillFS，对标 SKILL.md |
| S4 | 缺少 Claude Code 原生集成示例 | 无 FUSE + CLAUDE.md 指南 | 编写集成指南和示例 |
| S5 | 叙事不清 | 定位模糊 | 重写 README：Context FileSystem |

### 4.2 架构级问题（P1）

| # | 问题 | 解决方案 |
|---|------|----------|
| A1 | 无 L0/L1/L2 分层 | ContextFS 实现 OpenViking 式分层加载 |
| A2 | 无 Agent 间通信原语 | PipeFS 基于 QueueFS 扩展 |
| A3 | SDK 不全（Python/TS） | 实现 Python + TypeScript SDK |
| A4 | 无 Web Shell | React + Vite 管理界面 |
| A5 | OpenAPI 文档缺失 | 自动生成 OpenAPI 3.0 spec |

### 4.3 代码级问题（P2）

| # | 问题 | 文件 |
|---|------|------|
| C1 | WebDAV/FTP/SFTP 因 OpenDAL TLS 冲突禁用 | evif-plugins/Cargo.toml |
| C2 | TypeScript SDK 空壳 | evif-sdk-ts/ |
| C3 | Python SDK 空壳 | evif-sdk-python/ |
| C4 | evif-metrics 仅基础框架 | evif-metrics/src/ |
| C5 | Go SDK 缺少错误重试和断路器 | evif-sdk-go/ |
| C6 | REST API 缺少 OpenAPI 文档 | evif-rest/ |

---

## 五、实施计划

### Phase 8: Context Engine（核心差异化，P0，预估 20h）

**目标**：EVIF 内建 L0/L1/L2 上下文管理，成为 AI Agent 的长期记忆层

#### 8.1 ContextFS 插件（12h）

- [ ] `ContextFS` 插件实现
  - [ ] L0 即时层：`/context/L0/current`、`/context/L0/active_files/`
  - [ ] L1 会话层：`/context/L1/decisions.md`、`/context/L1/scratch/`
  - [ ] L2 知识层：`/context/L2/architecture.md`、`/context/L2/patterns.md`
  - [ ] 自动压缩：长文件 → 摘要 + 关键信息（L2 → L1 → L0）
  - [ ] 持久化：跨会话恢复，基于 SQLite + evif-mem
  - [ ] 语义检索：集成 VectorFS 搜索 L2 知识库
- [ ] Context Manager 服务
  - [ ] 上下文生命周期管理（创建、更新、过期、归档）
  - [ ] Token 预算管理（Anthropic 的 "smallest possible set" 原则）
  - [ ] 自动摘要生成（调用 LLM 或本地摘要）
- [ ] 测试：ContextFS 完整单元测试

#### 8.2 Claude Code 集成指南（4h）

- [ ] `CLAUDE.md` 模板 — EVIF 项目快速上手
- [ ] FUSE 挂载 + Claude Code 工作流示例
- [ ] `/context` 目录使用最佳实践文档
- [ ] 多 Agent 协调示例（QueueFS + PipeFS）

#### 8.3 Codex CLI 集成指南（2h）

- [ ] `AGENTS.md` 模板 — EVIF + Codex 配置
- [ ] `agents/openai.yaml` 技能定义
- [ ] REST API + Codex 工作流示例

#### 8.4 README.md 重写（2h）

- [ ] "Context FileSystem for AI Agents" 定位叙事
- [ ] 架构图（三层：Agent Access → Core → Plugins）
- [ ] 快速上手：3 分钟搭建 EVIF + Claude Code 环境
- [ ] 30 秒演示：ls /context → cat /context/L0/current → write decision

### Phase 9: SkillFS + PipeFS（Agent 协作，P1，预估 15h）

#### 9.1 SkillFS 技能文件系统（8h）

- [ ] `SkillFS` 插件实现
  - [ ] YAML manifest 规范（对标 SKILL.md frontmatter）
  - [ ] Markdown 指令文件（对标 SKILL.md body）
  - [ ] 技能发现：`ls /skills/` + `cat /skills/*/manifest.yaml`
  - [ ] 技能调用：`write /skills/*/input` → `read /skills/*/output`
  - [ ] 技能注册：Agent 或开发者通过 mkdir + write 添加技能
- [ ] 内置技能模板
  - [ ] `code-review` — 代码审查（安全、性能、风格）
  - [ ] `test-gen` — 测试生成
  - [ ] `doc-gen` — 文档生成
  - [ ] `refactor` — 代码重构建议
- [ ] 与 Claude Code Skills 互操作：manifest.yaml ↔ SKILL.md 转换
- [ ] 测试：SkillFS 完整单元测试

#### 9.2 PipeFS Agent 管道（7h）

- [ ] `PipeFS` 插件实现
  - [ ] 创建管道：`mkdir /pipes/task-001`
  - [ ] 双向通信：`input`/`output` 文件
  - [ ] 状态监控：`status` 文件（pending → running → completed/failed）
  - [ ] 超时和自动清理
  - [ ] 广播模式：`/pipes/broadcast/` 一写多读
- [ ] 基于 QueueFS 扩展（复用 Backend trait）
- [ ] 测试：PipeFS 完整单元测试

### Phase 10: 开发者生态（P1，预估 25h）

#### 10.1 Python SDK（8h）

- [ ] HTTP 客户端（httpx + asyncio）
- [ ] 文件操作：read/write/list/stat/mkdir/rm/mv/cp
- [ ] 挂载管理：mount/unmount/list
- [ ] Context API：context_load/context_save/context_search
- [ ] Skill API：skill_discover/skill_execute
- [ ] 流式读写支持
- [ ] 完整测试套件（pytest + httpx mock）

#### 10.2 TypeScript SDK（8h）

- [ ] Node.js 客户端（fetch API）
- [ ] TypeScript 类型定义
- [ ] 文件操作、挂载管理、Context API、Skill API
- [ ] 流式读写支持
- [ ] 完整测试套件（vitest）

#### 10.3 Web Shell（9h）

- [ ] React + Vite + TypeScript
- [ ] 文件浏览器（树形 + 列表视图）
- [ ] Monaco Editor 在线编辑
- [ ] Context Explorer（L0/L1/L2 可视化）
- [ ] Queue/Pipe 可视化（消息流、Agent 状态）
- [ ] Skill Gallery（技能发现和管理）
- [ ] 实时日志流（WebSocket）

### Phase 11: 生产增强（P2，预估 15h）

- [ ] OpenAPI 3.0 自动生成 + Swagger UI（3h）
- [ ] Prometheus metrics 完整实现 + Grafana 模板（4h）
- [ ] Go SDK 增强：重试/断路器/连接池（3h）
- [ ] CI/CD：多平台 Release + Docker + 性能基准（5h）

---

## 六、里程碑

| 里程碑 | Phase | 交付物 | 预估 |
|--------|-------|--------|------|
| M1: Context FileSystem 基础版 | 8 | ContextFS + Claude Code 集成 + README 重写 | 20h |
| M2: Agent 协作平台 | 9 | SkillFS + PipeFS + 多 Agent 示例 | 15h |
| M3: 开发者生态 | 10 | Python SDK + TypeScript SDK + Web Shell | 25h |
| M4: 生产就绪 | 11 | OpenAPI + Metrics + CI/CD | 15h |

---

## 七、关键设计决策

### 7.1 为什么弱化 MCP？

1. **Claude Code 的实际工作方式**：使用 glob/grep/head/tail/read/write，不是 MCP 工具
2. **Anthropic 自己的验证**："primitives like glob and grep allow it to navigate its environment just-in-time"
3. **MCP 的扩展性问题**：100+ 工具时 context burn 和成本暴增（Tool-RAG 正在解决）
4. **通用性**：ls/cat/grep 任何 LLM 都懂，MCP 需要专门的客户端支持

### 7.2 为什么采用 Skills 方式？

1. **Claude Code 已验证**：SKILL.md 格式是声明式、人类可读的技能描述
2. **Codex 也采用**：agents/openai.yaml 格式类似
3. **实证数据**：Meta-Tool 论文证明组合式工具减少 11.9% LLM 调用
4. **文件系统原生**：技能通过 ls/cat 发现，不需要额外的注册协议
5. **可移植**：manifest.yaml 可以转换为 SKILL.md 或 agents/openai.yaml

### 7.3 为什么 Context Engine 是核心？

1. **OpenViking 已证明**：L0/L1/L2 分层加载是解决上下文窗口限制的有效方法
2. **Anthropic 推荐**：Compaction + Structured Note-taking + Sub-agent 都依赖上下文管理
3. **AGFS 没做到**：OpenViking 是外挂的，EVIF 可以原生集成
4. **差异化**：从 "虚拟文件系统" 升级为 "上下文文件系统"

---

## 八、参考资料

### 学术论文
- [Everything is Context: Agentic File System Abstraction (arXiv:2512.05470)](https://arxiv.org/abs/2512.05470)
- [Optimizing Agentic Workflows using Meta-tools (arXiv:2601.22037)](https://arxiv.org/abs/2601.22037)
- [AIOS-LSFS: LLM-based Semantic File System (ICLR 2025)](https://github.com/agiresearch/AIOS-LSFS)

### AI Agent 集成
- [Effective Context Engineering for AI Agents — Anthropic](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
- [Claude Code Skills Documentation](https://code.claude.com/docs/en/skills)
- [Codex CLI AGENTS.md](https://developers.openai.com/codex/guides/agents-md/)
- [Claude Skills Deep Dive — Han Lee](https://leehanchung.github.io/blogs/2025/10/26/claude-skills-deep-dive/)

### AGFS / OpenViking
- [AGFS GitHub](https://github.com/c4pt0r/agfs)
- [AGFS: File Systems for AI Agents — LangCopilot](https://langcopilot.com/posts/2025-12-04-file-systems-for-ai-agents-next)
- [OpenViking GitHub](https://github.com/volcengine/OpenViking)
- [OpenViking Docs](https://volcengine-openviking.mintlify.app/)

### MCP / 工具生态
- [MCP Specification](https://modelcontextprotocol.io/specification/2025-11-25)
- [Scaling MCP to 100+ Tools](https://apxml.com/posts/scaling-mcp-with-tool-rag)
- [LangChain: How Agents Use Filesystems](https://blog.langchain.com/how-agents-can-use-filesystems-for-context-engineering/)
- [LlamaIndex: Files Are All You Need](https://www.llamaindex.ai/blog/files-are-all-you-need)

---

## 九、与 mem12.md 的关系

mem12.md Phase 4-7 **已全部完成**（0 个未勾选项）：
- Phase 4: WASM 双后端（Extism + Wasmtime）✅
- Phase 5: S3 分片上传 ✅
- Phase 6: QueueFS MySQL + VectorFS SQLite + SQLFS ✅
- Phase 7: CI 每日构建 + ProxyFS 健康检查 ✅

mem13.md v2 是一次战略级重定位：从"虚拟文件系统插件平台" → **"AI Agent 上下文文件系统"**。

---

> **核心转变**：
> 1. MCP 降级为辅助接口，文件元命令（ls/cat/grep/write）升级为核心接口
> 2. 新增 Context Engine（L0/L1/L2），对标 OpenViking 但内建而非外挂
> 3. 新增 SkillFS，对标 Claude Code SKILL.md 但用文件系统实现
> 4. 定位从"AGFS 的 Rust 版"升级为"AI Agent 的 Context FileSystem"
