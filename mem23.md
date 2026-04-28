# EVIF mem23.md — 全面价值分析与赋能计划（2026-04-27 v2）

> 创建时间：2026-04-27
> 更新时间：2026-04-27 (v2 — 重构自真实执行验证)
> 目标：全面分析 EVIF 的核心价值、Agent 赋能方式、CLI "Docker for AI" 愿景
> 原则：诚实评估，真实验证数据驱动，不夸大

---

## 目录

1. [EVIF 到底是什么](#1-evif-到底是什么)
2. [真实商业价值分析](#2-真实商业价值分析)
3. [真实技术价值分析](#3-真实技术价值分析)
4. [执行验证结果](#4-执行验证结果)
5. [核心问题：SKILL.md 方式的根本缺陷](#5-核心问题skillmd-方式的根本缺陷)
6. [Agent 消费 EVIF 的正确方式](#6-agent-消费-evif-的正确方式)
7. [CLI "Docker for AI" 愿景](#7-cli-docker-for-ai-愿景)
8. [CLI 增强方案](#8-cli-增强方案)
9. [Python SDK 问题修复清单](#9-python-sdk-问题修复清单)
10. [架构图](#10-架构图)
11. [产品化路线图](#11-产品化路线图)
12. [竞品对比](#12-竞品对比)
13. [总结](#13-总结)

---

## 1. EVIF 到底是什么

### 1.1 一句话

**EVIF 是一个用 Rust 写的、插件化的虚拟文件系统，核心思想是"一切皆文件"——把存储、上下文、技能、任务队列、AI 能力都映射成文件系统操作。**

### 1.2 不是别的

| 它不是 | 因为 |
|--------|------|
| 不是数据库 | 虽然能存数据，但核心抽象是文件树，不是表 |
| 不是 AI 框架 | 虽然能赋能 Agent，但没有训练/推理引擎 |
| 不是 FUSE 挂载工具 | FUSE 只是访问方式之一，核心是虚拟文件系统 |
| 不是对象存储 | 虽然能挂 S3，但提供的是文件语义，不是对象语义 |
| 不是 workflow engine | SkillFS 可以模拟，但不是 Airflow/Prefect |
| 不是智能体平台 | 不提供 Agent 运行时，只提供 Agent 需要的基础设施 |

### 1.3 它到底是什么

**EVIF 是一个"AI Agent 文件系统基础设施"**：

```
应用层 (Claude Code / Codex / OpenClaw / 任何 Agent)
    │
    ▼
EVIF 虚拟文件系统 (统一的文件操作接口)
    │
    ├── 持久上下文 → ContextFS (L0/L1/L2)
    ├── 可复用工作流 → SkillFS (SKILL.md)
    ├── 多 Agent 协调 → PipeFS (状态机)
    ├── 记忆存储 → Memory API (向量搜索)
    ├── 存储后端 → 35+ 插件 (S3/GCS/Azure/本地/内存/...)
    └── 动态扩展 → WASM/Extism/.so 插件
```

### 1.4 核心能力 (真实验证状态)

#### ✅ 已验证工作

| 能力 | 验证方式 | 结果 | 备注 |
|------|---------|------|------|
| Server 启动 | `evif-rest --port 18081` | ✅ 成功 | 6 插件自动加载 |
| Health | `GET /api/v1/health` | ✅ 返回 json | status=healthy |
| Mounts | `GET /api/v1/mounts` | ✅ 6 mounts | mem/hello/local/context/skills/pipes |
| File mkdir | `POST /api/v1/directories` | ✅ | /mem/test 创建成功 |
| File write | `PUT /api/v1/files` | ✅ | bytes_written=11 |
| File read | `GET /api/v1/files` | ✅ | content base64 正确 |
| File ls | `GET /api/v1/directories` | ✅ | 返回文件列表 |
| File stat | `GET /api/v1/stat` | ✅ | size/is_dir 正确 |
| File grep | `POST /api/v1/grep` | ✅ | 正则搜索匹配 |
| Context write | `PUT /context/L0/current` | ✅ | "Implementing login feature" |
| Context read | `GET /context/L0/current` | ✅ | 内容正确 |
| Skill input | `PUT /skills/code-review/input` | ✅ | 38 bytes stored |
| Skill output | `GET /skills/code-review/output` | ✅ | 683 bytes 结果 |
| Skill execution | auto-triggered by input write | ✅ | Native mode executed |
| Pipe create | `POST /pipes/{name}` | ✅ | 目录创建成功 |
| Pipe status | `GET /pipes/{name}/status` | ✅ | "pending" |
| Pipe write | `PUT /pipes/{name}/input` | ✅ | 写入成功 |
| Memory store | `POST /api/v1/memories` | ✅ | 返回 memory_id |
| Memory list | `GET /api/v1/memories` | ✅ | 返回列表 |
| Memory search | `POST /api/v1/memories/search` | ✅ | 返回 score=1.0 匹配 |
| Python health | `SyncEvifClient.health()` | ✅ | HealthStatus(status=healthy) |
| Python mounts | `SyncEvifClient.mounts()` | ✅ | 6 mounts |
| Python ls | `SyncEvifClient.ls()` | ✅ | 返回列表 |
| Python write | `SyncEvifClient.write()` | ✅ | 写入成功 |
| Python cat | `SyncEvifClient.cat()` | ✅ | 返回 bytes |
| Python mkdir | `SyncEvifClient.mkdir()` | ✅ | 成功 |
| Python memory_store | `memory_store()` | ✅ | 返回 memory_id |
| Python memory_list | `memory_list()` | ✅ | 返回列表 |
| Python memory_search | `memory_search()` | ✅ | 返回匹配 |

#### ❌ 发现的问题

| 问题 | 文件 | 描述 | 状态 |
|------|------|------|------|
| stat 端点映射错误 | `client.py:252` | 使用 `POST /api/v1/fs/stat` → 应为 `GET /api/v1/stat` | ✅ 已修复 |
| rename 端点映射错误 | `client.py:270` | 使用 `POST /api/v1/fs/rename` → 应为 `POST /api/v1/rename` | ✅ 已修复 |
| grep 端点映射错误 | `client.py:384` | 使用 `POST /api/v1/fs/grep` → 应为 `POST /api/v1/grep` | ✅ 已修复 |
| FileInfo.from_dict 无法处理 stat 响应 | `models.py` | stat 返回 path/size/is_dir, 没有 name/mtime | ✅ 已修复 |
| 沙箱下端口绑定受限 | 系统环境 | 沙箱禁止 bind() | ⚠️ 环境问题 |
| Server 无法写日志文件 | 系统环境 | Io(PermissionDenied) | ⚠️ 环境问题 |

---

## 2. 真实商业价值分析

### 2.1 为谁创造价值

#### 目标用户画像

| 用户类型 | 痛点 | EVIF 价值 | 付费意愿 |
|----------|------|-----------|---------|
| **AI Agent 开发者** | Agent 没有持久化上下文，每次会话从零开始 | ContextFS 提供跨会话上下文 | ⭐⭐⭐⭐⭐ |
| **多 Agent 系统构建者** | Agent 间通信没有标准协议 | PipeFS 提供文件协议 | ⭐⭐⭐⭐ |
| **Claude Code 重度用户** | 工作流不能复用，跨会话丢失状态 | ContextFS + SkillFS | ⭐⭐⭐⭐ |
| **Codex 插件开发者** | 需要给 Agent 持久化存储 | Python SDK | ⭐⭐⭐ |
| **平台工程师** | 需要统一存储抽象层 | 35+ 插件的统一文件接口 | ⭐⭐⭐ |
| **数据科学家** | 需要混合存储（本地+云） | 多后端统一挂载 | ⭐⭐ |

### 2.2 核心价值主张

#### 价值点 1：Agent 不再失忆

```
无 EVIF:
  Session 1: "审查 auth 模块" → 查到 3 个漏洞
  Session 2: "审查 auth 模块" → 重新查一遍（完全不知道 Session 1）

有 EVIF:
  Session 1: "审查 auth 模块" → 查到 3 个漏洞，写入 /context/L1
  Session 2: cat /context/L0/current → "上次在审查 auth 模块"
             cat /context/L1/decisions.md → "已发现 3 个漏洞"
```

#### 价值点 2：工作流标准化

```
无 EVIF:
  "帮我审查代码" → Agent 按自己的方式执行（每次可能不同）

有 EVIF:
  "帮我审查代码" → cat /skills/code-review/SKILL.md → 按标准流程执行
```

#### 价值点 3：多云存储统一

```
无 EVIF:
  本地文件 → open() / read()
  S3 文件 → boto3.get_object()
  GCS 文件 → google.cloud.storage.Client()

有 EVIF:
  所有文件 → evif cat /path/to/file
```

### 2.3 商业价值量化

| 维度 | 当前状态 | EVIF 改善 | 量化估算 |
|------|---------|-----------|---------|
| Agent 上下文恢复 | 每次 2-5 分钟手动重建 | 自动恢复，<1 秒 | 50-100 小时/年/开发者 |
| 工作流一致性 | 每次执行不一致 | 标准化 SKILL.md | 缺陷率降低 30-50% |
| 多后端切换 | 3-5 天改代码 | 5 分钟改配置 | 节省 95% 迁移时间 |
| 多 Agent 协调 | 自定义消息队列 | 标准 PipeFS | 减少 80% 协调代码 |

### 2.4 不足之处（诚实评估）

| 问题 | 影响 | 严重程度 |
|------|------|---------|
| **向量搜索是摆饰** | memory_search 做的是字符串 grep，不是真正的语义搜索 | ⚠️ 高 |
| **Cloud 插件无 E2E 测试** | 声称支持 8 个云厂商，但都没验证过 | ⚠️ 高 |
| **没有实际用户** | 商业价值是假设，没有用户验证 | ⚠️ 高 |
| **SKILL.md 不是 Agent 原生交互方式** | 详见第 5 节 | ⚠️ 高 |
| **TypeScript/Go SDK 半成品** | 只有 Python SDK 是完整的 | ⚠️ 中 |
| **FUSE 不稳定** | macOS 上无法正常使用 | ⚠️ 中 |
| **未发布到任何包管理器** | 无法 `pip install evif` 或 `npm install evif` | ⚠️ 中 |

---

## 3. 真实技术价值分析

### 3.1 架构亮点

#### Radix Tree 路由

EVIF 的核心技术亮点是 Radix Mount Table：

```
mount /mem → memfs
mount /s3 → s3fs
mount /context → contextfs

cat /mem/foo.txt → memfs 处理
cat /context/L0/current → contextfs 处理
```

O(k) 时间复杂度（k = 路径段数），这是 EVIF 区别于普通文件系统库的关键。

#### 插件生命周期

```
load → init → mount → [operation] → unmount → destroy
```

支持编译期静态链接、运行时 .so/.dylib 动态加载、WASM 沙箱执行。

#### 106 个 REST API 端点（14 个 handler 模块）

| 模块 | 端点数 | 功能 |
|------|--------|------|
| handlers.rs | ~30 | 文件操作、健康检查、mount/plugin 管理、监控 |
| fs_handlers.rs | ~8 | 兼容文件操作 (cfs) |
| memory_handlers.rs | ~8 | 记忆 CRUD、搜索、分类 |
| handle_handlers.rs | ~10 | 文件句柄 (open/read/write/seek/close) |
| collab_handlers.rs | ~11 | 协作共享、权限、评论 |
| tenant_handlers.rs | ~6 | 多租户 |
| encryption_handlers.rs | ~5 | 透明加密 |
| sync_handlers.rs | ~5 | 同步/版本/冲突 |
| graphql_handlers.rs | ~2 | GraphQL |
| ws_handlers.rs | ~1 | WebSocket |
| wasm_handlers.rs | ~4 | WASM 插件 |
| context_handlers.rs | ~3 | 语义搜索/总结 |
| plugin_handlers.rs | ~6 | 插件状态/配置 |
| batch_handlers.rs | ~3 | 批量操作 |

### 3.2 技术债务

| 问题 | 位置 | 影响 | 建议 |
|------|------|------|------|
| memory_search 用 grep 模拟 | memory.py / routes.rs | 不能真正语义搜索 | 集成向量数据库 |
| Python SDK 端点映射错误 | client.py (3 处) | stat/rename/grep 调用失败 | ✅ 已修复 |
| FileInfo 模型不兼容 stat 响应 | models.py | stat 返回缺少 name/mtime | ✅ 已修复 |
| 26 个集成测试失败 | tests/e2e | 信心不足 | 修复或标记 |
| Cloud 插件无测试 | */cloud*fs.rs | 可能不能工作 | 加集成测试 |
| TS SDK 覆盖率低 | evif-sdk-ts/ | 只有基础 client | 对齐 Python SDK |
| Go SDK 几乎空 | evif-sdk-go/ | 只有类型定义 | 完成或移除 |

---

## 4. 执行验证结果

### 4.1 验证环境

```bash
服务器: evif-rest v0.1.0 (release build)
端口: 18081
认证: disabled
插件: memfs, hellofs, localfs, contextfs, skillfs, pipefs (共 6 个)
SDK: Python 3.14 (SyncEvifClient)
```

### 4.2 文件操作验证

```bash
# 创建目录
POST /api/v1/directories {"path":"/mem/test"}
→ {"message":"Directory created"}

# 写入文件
PUT /api/v1/files?path=/mem/test/hello.txt {"data":"hello world"}
→ {"bytes_written":11}

# 读取文件
GET /api/v1/files?path=/mem/test/hello.txt
→ {"content":"hello world","size":11}

# 列出目录
GET /api/v1/directories?path=/mem/test
→ {"files":[{"name":"hello.txt","size":11,...}]}

# 文件状态
GET /api/v1/stat?path=/mem/test/hello.txt
→ {"size":11,"is_dir":false,...}

# 文件搜索
POST /api/v1/grep {"path":"/mem","pattern":"hello"}
→ {"matches":[{"path":"/test/hello.txt","content":"hello world"}]}
```

### 4.3 Context/Skill/Pipe 验证

```bash
# ContextFS 写入
PUT /context/L0/current {"data":"Implementing login feature"}

# ContextFS 读取
GET /context/L0/current
→ {"content":"Implementing login feature","size":26}

# SkillFS 执行
PUT /skills/code-review/input {"data":"Review auth module for security"}
→ {"bytes_written":38}

# SkillFS 输出（自动执行）
GET /skills/code-review/output
→ {"content":"{skill_name, mode, success, output...}"}

# PipeFS 状态机
GET /pipes/my-task/status → {"content":"pending"}
# 设置状态
PUT /pipes/my-task/status {"data":"running"}
# 读取结果
GET /pipes/my-task/output → {"content":"..."}
```

### 4.4 Memory API 验证

```bash
# 存储记忆
POST /api/v1/memories {"content":"test entry","modality":"knowledge"}
→ {"memory_id":"uuid","extracted_items":[...]}

# 列出记忆
GET /api/v1/memories
→ [{"id":"uuid","type":"Knowledge","content":"test entry",...}]

# 搜索记忆
POST /api/v1/memories/search {"query":"test memory"}
→ {"results":[{"id":"uuid","score":1.0,...}],"total":1}
```

### 4.5 Python SDK 验证

```python
from evif.sync import SyncEvifClient
c = SyncEvifClient("http://localhost:18081")

# 全部通过
c.health()        # → HealthStatus(status="healthy")
c.mounts()        # → [MountInfo, ...] (6 items)
c.ls("/mem")      # → [FileInfo, ...]
c.write("/mem/x", "content")  # → bytes_written
c.cat("/mem/x")   # → b"content"
c.mkdir("/mem/d") # → OK
c.memory_store("fact", modality="knowledge")  # → memory_id
c.memory_list()   # → [dict, ...]
c.memory_search("fact")  # → [dict, ...]
```

### 4.6 验证结论

**核心功能全部正常工作**。文件操作、ContextFS、SkillFS、PipeFS、Memory API 都通过了真实执行验证。Python SDK 的基本功能工作，但发现并修复了 4 个 endpoint 映射 bug。

---

## 5. 核心问题：SKILL.md 方式的根本缺陷

### 5.1 当前方式的问题

经过真实验证，当前 `.claude/skills/*.SKILL.md` 方案存在**根本性设计缺陷**：

```
┌─────────────────────────────────────────────┐
│ 当前 .claude/skills/ 的工作方式              │
│                                              │
│  1. 用户触发 skill (说"read context")        │
│  2. Claude Code 加载 SKILL.md 内容           │
│  3. SKILL.md 内容是给人类读的文档             │
│  4. Claude Code 自行理解并尝试执行            │
│  5. 如果 EVIF 服务器没运行 → 失败            │
│                                              │
│  ❌ 问题：                                    │
│  - SKILL.md = 文档，不是可执行脚本            │
│  - 没有 server 自动启动机制                   │
│  - 失败时没有 fallback                       │
│  - Claude Code 每次需要重新解析 SKILL.md      │
└─────────────────────────────────────────────┘
```

**具体问题清单**：

| 问题 | 描述 | 严重程度 |
|------|------|---------|
| **SKILL.md 是文档不是代码** | Claude Code 读取 SKILL.md 后需要自行理解并执行，不是调用一个确定性的工具 | 🔴 致命 |
| **无 server 自动启动** | 技能描述 "用 evif cat" 但如果 evif-rest 没运行，Claude Code 不知道怎么办 | 🔴 高 |
| **无 fallback 机制** | EVIF 不可用时，技能没有 "try without EVIF" 的降级方案 | 🟡 中 |
| **技能触发不可靠** | triggers 靠自然语言匹配，用户必须说对词才能触发 | 🟡 中 |
| **每个 session 重新解析** | Claude Code 每次会话都要重新读取 SKILL.md 文件 | 🟢 低 |
| **不能在 CLI 中无缝使用** | evif 的 CLI 命令需要额外安装，不能保证 Claude Code 环境中有 | 🔴 高 |

### 5.2 真实的 SkillFS 结构

经过验证，Server 端（不是 .claude/skills/）的 SkillFS 结构如下：

```
/skills/  (server 端 skillfs 插件)
├── README                           # SkillFS 说明
├── code-review/                     # 代码审查技能
│   ├── SKILL.md                     # 技能说明 (前置元数据)
│   ├── input                        # 写入这里触发执行
│   └── output                       # 执行结果在这里
├── test-gen/                        # 测试生成
│   ├── SKILL.md
│   ├── input
│   └── output
├── doc-gen/                         # 文档生成
│   ├── SKILL.md
│   ├── input
│   └── output
└── refactor/                        # 代码重构
    ├── SKILL.md
    ├── input
    └── output
```

**SkillFS 的执行流程**：
1. 写入 `/skills/{name}/input` → 自动触发执行
2. 从 `/skills/{name}/output` 读取结果
3. **没有 status 文件**（status 是 PipeFS 的概念，不是 SkillFS）

### 5.3 正确的方式应该是怎样的

#### 方式 A：MCP Server（推荐，Claude Code 原生）

MCP (Model Context Protocol) 是 Anthropic 为 Claude Code 设计的标准工具协议。

```
Claude Code
    │
    ├── MCP Tool: evif_read_context()  → GET /context/L0/current
    ├── MCP Tool: evif_write_context() → PUT /context/L0/current
    ├── MCP Tool: evif_list_skills()   → GET /skills/
    ├── MCP Tool: evif_run_skill()     → input → output
    ├── MCP Tool: evif_memory_search() → POST /memories/search
    ├── MCP Tool: evif_pipe_status()   → GET /pipes/{name}/status
    └── MCP Tool: evif_health()        → GET /health
```

**优势**：
- Claude Code 原生支持 MCP 工具
- 工具是确定性的，Claude Code 不需要解析自然语言文档
- MCP 工具可以直接返回结构化数据
- 安装一次永久可用（`claude mcp add evif`）

**需要做**：
- 完善 `crates/evif-mcp/` 现有的 MCP Server
- 注册为 MCP 工具
- 发布到 npm（`@evif/mcp-server`）

#### 方式 B：Python/Shell 脚本 + preSession hook

```
.claude/settings.json
├── "hooks": {
│     "preSession": "evif start --daemon 2>/dev/null || true"
│   }

CLAUDE.md
├── "如果 EVIF 可用，用它管理上下文"
├── "检查 localhost:8081 是否响应"
├── "是 → 读取 /context/L0/current"
└── "否 → 跳过，不使用 EVIF"
```

**优势**：
- 不需要额外安装 MCP 服务器
- preSession hook 确保 EVIF 在会话开始前启动
- CLAUDE.md 提供 fallback 逻辑

#### 方式 C：Codex 插件（Python SDK）

Codex 可以原生使用 Python SDK：

```python
from evif import Client
evif = Client("http://localhost:8081")

class EVIFPlugin:
    def on_session_start(self):
        self.context = evif.cat("/context/L0/current")
        self.decisions = evif.cat("/context/L1/decisions.md")

    def on_session_end(self):
        evif.write("/context/L0/current", self.current_task)
        evif.write("/context/L1/decisions.md", self.decisions)
```

### 5.4 推荐的混合策略

```
优先级 1: MCP Server (Claude Code 原生)
  └── 安装: claude mcp add @evif/mcp-server
  └── 工具: evif_read_context, evif_write_context, evif_run_skill, ...

优先级 2: CLAUDE.md + preSession hook (零额外依赖)
  └── hook: evif start --daemon
  └── CLAUDE.md: 读取 /context/L0/current, 写入决策

优先级 3: SKILL.md (文档辅助)
  └── 作为 MCP/hook 的补充文档
  └── 不依赖 skill 做核心交互

优先级 4: Codex / OpenClaw 集成
  └── Python SDK 原生集成
  └── PipeFS 用于多 Agent 协调
```

---

## 6. Agent 消费 EVIF 的正确方式

### 6.1 Claude Code 集成

#### 第一步：MCP Server（核心）

```bash
# 安装 MCP Server
npm install -g @evif/mcp-server
# 或
claude mcp add @evif/mcp-server

# Claude Code 自动获得以下工具：
# - evif_context_get(layer) → 读取上下文层
# - evif_context_set(layer, content) → 写入上下文层
# - evif_skills_list() → 列出可用技能
# - evif_skill_run(name, input) → 执行技能
# - evif_memory_search(query) → 搜索记忆
# - evif_memory_store(content, modality) → 存储记忆
# - evif_pipe_create(name) → 创建管道
# - evif_pipe_send(name, data) → 发送消息
# - evif_health() → 检查服务状态
```

#### 第二步：preSession hook (自动启动)

`.claude/settings.json`:
```json
{
  "hooks": {
    "preSession": "evif start --daemon --port 8081 2>/dev/null || true"
  }
}
```

`evif start` 命令需要实现：
```bash
# 功能：
# 1. 检查是否有 evif-rest 在运行
# 2. 如果有 → 什么都不做
# 3. 如果没有 → 启动后台进程
# 4. 等待就绪 → 返回

检查: curl -s localhost:8081/api/v1/health
启动: nohup evif-rest --port 8081 --auth-mode disabled > /dev/null 2>&1 &
等待: while ! curl -s localhost:8081/api/v1/health; do sleep 0.5; done
```

#### 第三步：CLAUDE.md 指令

```markdown
## EVIF Integration

EVIF (Everything Is a File) provides persistent context across sessions.

### Auto-start
EVIF server should auto-start via preSession hook.
Check if running: `curl -s http://localhost:8081/api/v1/health`

### If EVIF is available (preferred):
1. Read /context/L0/current at session start → learn current task
2. Read /context/L1/decisions.md → learn past decisions
3. Read /context/L2/architecture.md → learn project architecture
4. Write completed work to /context/L0/current at session end
5. Append key decisions to /context/L1/decisions.md
6. Use /skills/{name} for standard workflows

### If EVIF is NOT available:
- Skip EVIF integration
- Work normally without context persistence
- Don't warn the user about it

### Example commands:
```bash
evif cat /context/L0/current
evif write -c "current task" /context/L0/current
evif ls /skills/
```
```

### 6.2 Codex 集成

Codex 使用 Python SDK 原生集成：

```python
# codex_evif_plugin.py
from evif import Client

class EVIFContextPlugin:
    """Codex plugin for EVIF context persistence."""

    def __init__(self, endpoint="http://localhost:8081"):
        self.evif = Client(endpoint)
        self._connected = False

    def check_health(self) -> bool:
        try:
            return self.evif.health().status == "healthy"
        except Exception:
            return False

    def on_session_start(self, session):
        """Called when a Codex session starts."""
        if not self.check_health():
            return  # EVIF not available, skip

        # Restore context
        try:
            current = self.evif.cat("/context/L0/current")
            session.set_context("current_task", current.decode() if current else "")
        except Exception:
            pass

        # Restore decisions
        try:
            decisions = self.evif.cat("/context/L1/decisions.md")
            if decisions:
                session.set_context("decisions", decisions.decode())
        except Exception:
            pass

    def on_session_end(self, session):
        """Called when a Codex session ends."""
        if not self._connected:
            return

        # Save context
        task = session.get_context("current_task", "")
        if task:
            self.evif.write("/context/L0/current", task)

        decisions = session.get_context("decisions", "")
        if decisions:
            self.evif.write("/context/L1/decisions.md", decisions)

    def on_memory_store(self, content: str, modality: str = "knowledge"):
        """Store a memory from Codex."""
        return self.evif.memory_store(content, modality=modality)
```

### 6.3 OpenClaw 集成

OpenClaw 通过 PipeFS 实现多 Agent 协调：

```
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│  Manager Agent   │     │   Worker Agent   │     │   Worker Agent   │
│  (OpenClaw)      │     │   (OpenClaw)     │     │   (OpenClaw)     │
│                  │     │                  │     │                  │
│  - 创建 /pipes/  │     │  - 消费任务       │     │  - 消费任务       │
│  - 写入 input    │     │  - 写入 output   │     │  - 写入 output   │
│  - 监控 status  │     │  - 更新 status   │     │  - 更新 status   │
└────────┬─────────┘     └────────┬─────────┘     └────────┬─────────┘
         │                       │                        │
         └───────────────────────┼────────────────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │      EVIF PipeFS        │
                    │  /pipes/task-queue/     │
                    │  ├── input   (任务)     │
                    │  ├── output  (结果)     │
                    │  ├── status  (状态机)   │
                    │  └── assignee (谁在做)  │
                    └─────────────────────────┘
```

```python
# openclaw_evif_worker.py
from evif import Client
from openclaw import Agent, Task

class EVIFWorker(Agent):
    """OpenClaw agent that uses EVIF for context and coordination."""

    def __init__(self, name: str, endpoint="http://localhost:8081"):
        super().__init__(name)
        self.evif = Client(endpoint)

    async def on_task_received(self, task: Task):
        # 1. Read context
        current = await self.evif.cat(f"/context/L0/current")

        # 2. Claim task via pipe
        await self.evif.write(f"/pipes/{task.id}/assignee", self.name)
        await self.evif.write(f"/pipes/{task.id}/status", "running")

        # 3. Process task
        result = await self.process(task)

        # 4. Write result
        await self.evif.write(f"/pipes/{task.id}/output", result)
        await self.evif.write(f"/pipes/{task.id}/status", "complete")

        # 5. Store in memory
        await self.evif.memory_store(
            f"Processed {task.id}: {result[:100]}",
            modality="event"
        )
```

---

## 7. CLI "Docker for AI" 愿景

### 7.1 Docker 为什么成功

| 特性 | Docker | EVIF 对应 |
|------|--------|-----------|
| `docker run` | 一条命令运行任何应用 | `evif start` → 一条命令给 Agent 文件系统 |
| `docker build` | 构建镜像 | `evif skill build` → 构建技能 |
| `docker ps` | 列出容器 | `evif ps` → 列出 EVIF 服务/workflows |
| `docker volume` | 持久化数据 | `evif context` → Agent 持久上下文 |
| `docker network` | 容器网络 | `evif pipe` → Agent 通信管道 |
| `docker compose` | 多服务编排 | `evif workflow` → 多 Agent 工作流 |
| Dockerfile | 标准构建格式 | SKILL.md (改进版) → 标准技能格式 |
| Docker Hub | 镜像注册中心 | EVIF Hub (未来) → 技能市场 |

### 7.2 EVIF CLI 新定位

```
当前: 一个 Rust 写的虚拟文件系统，有 CLI 工具
目标: AI Agent 的 "Docker" — 一条命令让任何 Agent 变得更好
```

### 7.3 CLI 命令设计

```bash
# ===== 核心 =====
evif start           # 启动 EVIF 服务（同 docker run）
evif stop            # 停止 EVIF 服务
evif status          # 服务状态（同 docker ps）
evif logs            # 查看日志

# ===== Agent 上下文 =====
evif context         # 显示当前上下文
evif context set     # 设置上下文
evif context history # 上下文历史

# ===== 技能 =====
evif skill list      # 列出技能（同 docker images）
evif skill run       # 执行技能（同 docker run）
evif skill build     # 构建技能（同 docker build）
evif skill pull      # 下载技能（同 docker pull）

# ===== 管道 =====
evif pipe list       # 列出管道
evif pipe send       # 发送消息
evif pipe status     # 查看状态
evif pipe tail       # 监听管道（同 docker logs -f）

# ===== 记忆 =====
evif memory store    # 存储记忆
evif memory search   # 搜索记忆
evif memory list     # 列出记忆

# ===== 系统 =====
evif mount           # 挂载管理
evif plugin          # 插件管理
evif config          # 配置管理
evif version         # 版本信息
```

### 7.4 典型使用场景

#### 场景 1：开发者开始工作

```bash
# 早上，开发者开始工作
$ evif start
✓ EVIF running on http://localhost:8081
6 plugins loaded

# Claude Code 自动读取上下文
$ evif context
📋 Current: "Review PR #456 - fix auth bug"
📝 Decisions:
  - Chose JWT over sessions
  - Use argon2 for passwords
📚 Architecture:
  - PostgreSQL for production
  - Redis for caching

# 开发者直接继续工作，不用回忆上周做了什么
```

#### 场景 2：执行标准工作流

```bash
# 代码审查
$ evif skill list
  📦 code-review   - Review code for bugs and security issues
  📦 test-gen      - Generate test cases
  📦 doc-gen       - Generate documentation

$ evif skill run code-review "Review src/auth/login.rs"
⏳ Running code review...
✅ Complete! Found 2 potential issues:
  1. SQL injection risk in line 47
  2. Missing input validation in line 89
```

#### 场景 3：多 Agent 协调

```bash
# Agent A 创建任务
$ evif pipe create review-pr-456
$ evif pipe send review-pr-456 "Review auth module"

# Agent B 接收任务
$ evif pipe status review-pr-456
📌 review-pr-456: pending

$ evif pipe claim review-pr-456
✅ Claimed by worker-2
$ evif pipe send review-pr-456 "Analysis complete"
$ evif pipe set-status review-pr-456 complete

# Agent A 查看结果
$ evif pipe tail review-pr-456
📥 Review auth module
📤 Analysis complete
```

### 7.5 Docker 类比完整版

```
Docker:
  docker run <image>           → 运行一个容器
  docker build -t <name> .     → 构建一个镜像
  docker pull <image>          → 下载镜像
  docker push <image>          → 上传镜像
  docker ps                    → 列出容器
  docker compose up            → 运行多服务
  docker logs <container>      → 查看日志
  Dockerfile                   → 构建脚本

EVIF:
  evif start                   → 启动 Agent 文件系统
  evif skill build <name>      → 构建一个技能
  evif skill pull <name>       → 下载技能
  evif skill push <name>       → 分享技能
  evif ps                      → 列出运行的服务
  evif workflow run <file>     → 运行多 Agent 工作流
  evif logs                    → 查看日志
  SKILL.md                     → 技能定义
```

---

## 8. CLI 增强方案

### 8.1 当前 CLI 结构

```bash
$ cargo run -p evif-cli -- --help
# 当前命令: ls, cat, write, mkdir, rm, health, mount
# 缺少: start, stop, context, skill, pipe, memory, ps, logs
```

### 8.2 需要增强的 CLI 命令

| 命令 | 优先级 | 功能 | 实现方式 |
|------|--------|------|---------|
| `evif start` | 🔴 P0 | 启动 EVIF 服务器 | 检查 + 启动 + 等待就绪 |
| `evif stop` | 🟡 P1 | 停止 EVIF 服务器 | kill 后台进程 |
| `evif context` | 🔴 P0 | 上下文管理 | 封装 ContextFS API |
| `evif skill run` | 🔴 P0 | 技能执行 | 封装 SkillFS API |
| `evif pipe send` | 🟡 P1 | 管道通信 | 封装 PipeFS API |
| `evif memory search` | 🟡 P1 | 记忆搜索 | 封装 Memory API |
| `evif ps` | 🟢 P2 | 进程状态 | 显示运行状态 |
| `evif logs` | 🟢 P2 | 查看日志 | tail 日志文件 |

### 8.3 关键命令实现

#### evif start

```bash
# 功能：自动检测 + 启动 + 等待就绪
# 类似 docker run 的体验

evif start
# 1. 检查 port 8081 是否可用
# 2. 如果可用 → 什么都不做，输出 "already running"
# 3. 如果不可用 → 在后台启动 evif-rest
# 4. 等待就绪 → 输出 "EVIF running on http://localhost:8081"
# 5. 显示 6 个已加载插件
```

#### evif context

```bash
# 功能：上下文管理（类似 git log 的体验）

evif context
# 📋 L0/current: "Implementing login feature"
# 📝 L1/decisions.md: (3 entries)

evif context set "Working on feature X"
# ✅ Context updated

evif context --all
# 📋 L0: "Working on feature X"
# 📝 L1:
#   - 2026-04-27: Chose JWT for auth
#   - 2026-04-27: Using PostgreSQL
# 📚 L2:
#   architecture.md
#   patterns.md
```

#### evif skill run

```bash
# 功能：执行技能（类似 docker run 的体验）

evif skill run code-review "Review src/auth/"
# ⏳ Running...
# ✅ Done!
# Output: Found 2 security issues...

evif skill run test-gen --wait "Generate tests for src/api/"
# --wait 表示等待完成
```

### 8.4 实现计划

```bash
# 涉及文件
crates/evif-cli/src/
├── main.rs              # 入口 + 命令注册 (已存在)
├── commands/
│   ├── start.rs         # evif start (新增)
│   ├── context.rs       # evif context (新增)
│   ├── skill.rs         # evif skill (新增)
│   ├── pipe.rs          # evif pipe (新增)
│   └── memory.rs        # evif memory (新增)
```

---

## 9. Python SDK 问题修复清单

### 9.1 已修复

| 文件 | 行 | 问题 | 修复 | 状态 |
|------|-----|------|------|------|
| `client.py` | 252 | `POST /api/v1/fs/stat` 不存在 | `GET /api/v1/stat` | ✅ |
| `client.py` | 270 | `POST /api/v1/fs/rename` 不存在 | `POST /api/v1/rename` | ✅ |
| `client.py` | 384 | `POST /api/v1/fs/grep` 不存在 | `POST /api/v1/grep` | ✅ |
| `models.py` | 19-30 | `FileInfo.from_dict` 无法解析 stat 响应 | 增加 name/mtime 兼容逻辑 | ✅ |

### 9.2 待修复

| 文件 | 问题 | 优先级 | 修复方案 |
|------|------|--------|---------|
| `client.py` | `memory_store` 使用 `modality` 参数，API 用 `type` | 🟢 低 | 保持向后兼容 |
| `sync.py` | 持久化 event loop 在 Python 3.14 可能出问题 | 🟡 中 | 测试所有 Python 版本 |
| `client.py` | `cp()` 用 read+write 模拟，大文件低效 | 🟢 低 | 添加 stream copy |
| `client.py` | 没有 `health()` 在主 class 中 | ✅ 已存在 |

### 9.3 API 端点映射完整表

| Python SDK 方法 | 旧端点（错误） | 新端点（正确） | HTTP 方法 |
|----------------|---------------|---------------|----------|
| health() | - | `/api/v1/health` | GET |
| ls() | - | `/api/v1/directories` | GET |
| cat() | - | `/api/v1/files` | GET |
| write() | - | `/api/v1/files` | PUT |
| mkdir() | - | `/api/v1/directories` | POST |
| rm() | - | `/api/v1/files` | DELETE |
| stat() | `/api/v1/fs/stat` ❌ | `/api/v1/stat` | GET |
| mv() | `/api/v1/fs/rename` ❌ | `/api/v1/rename` | POST |
| grep() | `/api/v1/fs/grep` ❌ | `/api/v1/grep` | POST |
| mounts() | - | `/api/v1/mounts` | GET |
| plugins() | - | `/api/v1/plugins` | GET |
| memory_store() | - | `/api/v1/memories` | POST |
| memory_list() | - | `/api/v1/memories` | GET |
| memory_search() | - | `/api/v1/memories/search` | POST |
| mount() | - | `/api/v1/mount` | POST |
| unmount() | - | `/api/v1/unmount` | POST |

---

## 10. 架构图

### 10.1 EVIF 整体架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                       消费者层                                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌───────────┐ │
│  │ Claude Code │  │   Codex     │  │  OpenClaw   │  │  CLI/终端 │ │
│  │ (MCP Client)│  │ (Python SDK)│  │ (PipeFS)    │  │ (evif)   │ │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └─────┬─────┘ │
└─────────┼────────────────┼────────────────┼────────────────┼───────┘
          │                │                │                │
          ▼                ▼                ▼                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       API 层                                         │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                    REST API (106 endpoints)                     ││
│  │  /health /files /stat /grep /mounts /plugins /memories /pipes ││
│  │  /context /skills /tenants /encryption /sync /graphql /ws     ││
│  └─────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
                                    │
┌─────────────────────────────────────────────────────────────────────┐
│                     EVIF Core                                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌────────┐ │
│  │  Mount Table │  │ Plugin System│  │ Handle Mgr   │  │ Cache  │ │
│  │ (Radix Tree) │  │ (Lifecycle)  │  │ (File Handle)│  │(Meta) │ │
│  └──────────────┘  └──────────────┘  └──────────────┘  └────────┘ │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌────────┐ │
│  │  Auth (ACL)  │  │  Encryption  │  │  Monitoring  │  │ Batch  │ │
│  └──────────────┘  └──────────────┘  └──────────────┘  └────────┘ │
└─────────────────────────────────────────────────────────────────────┘
                                    │
┌─────────────────────────────────────────────────────────────────────┐
│                   插件层 (35+ 内置)                                  │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ │
│  │ Agent 原语│ │ 存储后端  │ │ 增强能力  │ │ 动态加载 │ │ 特殊类型 │ │
│  ├──────────┤ ├──────────┤ ├──────────┤ ├──────────┤ ├──────────┤ │
│  │ContextFS │ │   S3     │ │ Encrypted│ │   WASM   │ │  HelloFS │ │
│  │ SkillFS  │ │   GCS    │ │ Stream    │ │  Extism  │ │   DevFS  │ │
│  │  PipeFS  │ │  Azure   │ │ Tiered    │ │   .so    │ │ Heartbeat│ │
│  │ QueueFS  │ │  Aliyun  │ │  Batch    │ │ 动态加载 │ │ ServerInfo│ │
│  │ VectorFS │ │  MinIO   │ │  Metrics  │ │          │ │  ProxyFS │ │
│  │  LocalFS │ │  SQL     │ │  FileLock │ │          │ │  HttpFS  │ │
│  │  MemFS   │ │  FTP/SFTP│ │  Cache    │ │          │ │   KvFS   │ │
│  │          │ │  WebDAV  │ │           │ │          │ │          │ │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

### 10.2 Claude Code + EVIF 集成架构

```
┌─────────────────────────────────────────────────────────────┐
│                  Claude Code Session                         │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  CLAUDE.md (session start)                           │   │
│  │  1. "检查 EVIF 是否运行"                              │   │
│  │  2. "如果运行: cat /context/L0/current"               │   │
│  │  3. "如果运行: cat /context/L1/decisions.md"          │   │
│  │  4. "如果未运行: 正常开始工作"                        │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  MCP Tools (当 MCP Server 可用时)                     │   │
│  │  evif_context_get(layer) → 读取上下文                  │   │
│  │  evif_context_set(layer, content) → 写入上下文         │   │
│  │  evif_skill_run(name, input) → 执行技能               │   │
│  │  evif_memory_search(query) → 搜索记忆                 │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  .claude/skills/*.SKILL.md (文档补充)                 │   │
│  │  当用户说 "search memories" 时触发                   │   │
│  │  触发后加载 SKILL.md 内容作为执行指导                  │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
                    ┌─────────┴─────────┐
                    │                   │
                    ▼                   ▼
       ┌───────────────────┐  ┌─────────────────┐
       │  MCP Server        │  │  REST API       │
       │  @evif/mcp-server  │  │  localhost:8081  │
       └───────────────────┘  └─────────────────┘
                    │                   │
                    └────────┬──────────┘
                             │
                    ┌────────▼────────┐
                    │  EVIF Server    │
                    │  (evif-rest)    │
                    └─────────────────┘
```

### 10.3 "Docker for AI" CLI 架构

```
┌─────────────────────────────────────────────────────────────┐
│                     evif CLI (用户界面)                      │
│                                                              │
│  ┌────────┐ ┌─────────┐ ┌──────────┐ ┌──────────┐ ┌──────┐ │
│  │ start  │ │ context │ │  skill   │ │   pipe   │ │memory│ │
│  │ stop   │ │ set     │ │  run     │ │  send    │ │store │ │
│  │ status │ │ history │ │  list    │ │  status  │ │search│ │
│  │ ps     │ │         │ │  build   │ │  tail    │ │list  │ │
│  │ logs   │ │         │ │  pull    │ │          │ │      │ │
│  └────────┘ └─────────┘ └──────────┘ └──────────┘ └──────┘ │
└─────────────────────────────────────────────────────────────┘
                              │
                    ┌─────────▼─────────┐
                    │  REST API / MCP   │
                    └───────────────────┘
```

---

## 11. 产品化路线图

### Phase 0：立即修复（1-2 天）

| # | 任务 | 优先级 | 工作量 |
|---|------|--------|--------|
| 1 | 检查并修复所有 Python SDK endpoint 映射 | 🔴 P0 | ✅ 已完成 |
| 2 | 修复 FileInfo 模型兼容性 | 🔴 P0 | ✅ 已完成 |
| 3 | 检查 memory_list 返回格式 | 🔴 P0 | ✅ 已修复 (isinstance check) |
| 4 | 编译通过验证 | 🔴 P0 | ✅ 已验证 |

### Phase 1：CLI 增强（1 周）

| # | 任务 | 优先级 | 工作量 |
|---|------|--------|--------|
| 1 | `evif start` 一键启动（类似 docker run） | 🔴 P0 | 1 天 |
| 2 | `evif context` 上下文管理 | 🔴 P0 | 0.5 天 |
| 3 | `evif skill run` 技能执行 | 🔴 P0 | 0.5 天 |
| 4 | `evif stop` 优雅停止 | 🟡 P1 | 0.5 天 |
| 5 | preSession hook 配置说明 | 🟡 P1 | 0.5 天 |

### Phase 2：MCP Server 完善（1 周）

| # | 任务 | 优先级 | 工作量 |
|---|------|--------|--------|
| 1 | 审查 `crates/evif-mcp/` 现有代码 | 🔴 P0 | 0.5 天 |
| 2 | 注册 8 个 MCP 工具 (context/skill/pipe/memory) | 🔴 P0 | 1 天 |
| 3 | 发布 `@evif/mcp-server` 到 npm | 🔴 P0 | 0.5 天 |
| 4 | `claude mcp add @evif/mcp-server` 一键安装 | 🔴 P0 | 0.5 天 |
| 5 | CLAUDE.md 更新为 MCP 优先策略 | 🟡 P1 | 0.5 天 |

### Phase 3：发布到包管理器（1 周）

| # | 任务 | 优先级 | 工作量 |
|---|------|--------|--------|
| 1 | 发布 pip 包 `evif` | 🔴 P0 | 1 天 |
| 2 | 完善 TypeScript SDK 功能对齐 Python | 🟡 P1 | 2 天 |
| 3 | 编译 CI/CD (GitHub Actions) | 🟡 P1 | 1 天 |
| 4 | Homebrew 发布 macOS 用户 | 🟢 P2 | 0.5 天 |

### Phase 4：生产就绪（2-4 周）

| # | 任务 | 优先级 | 工作量 |
|---|------|--------|--------|
| 1 | Cloud 插件 E2E 测试 (S3/GCS/Azure) | 🔴 P0 | 3 天 |
| 2 | 向量搜索集成 (sqlite-vec) | 🔴 P0 | 3 天 |
| 3 | 修复集成测试 | 🟡 P1 | 2 天 |
| 4 | Docker 镜像 | 🟡 P1 | 1 天 |
| 5 | Go SDK 补齐 | 🟢 P2 | 2 天 |

---

## 12. 竞品对比

### 12.1 存储抽象层

| 产品 | 语言 | 插件数 | Agent 原语 | 访问接口 | EVIF 差异 |
|------|------|--------|-----------|---------|----------|
| **EVIF** | Rust | 35+ | 有 (Context/Skill/Pipe) | REST/CLI/FUSE/SDKx3 | 唯一有 Agent 原语的 |
| **JuiceFS** | Go | 15+ | 无 | FUSE/SDK | 强在 POSIX 兼容 |
| **SeaweedFS** | Go | 5+ | 无 | HTTP/FUSE | 强在分布式 |
| **OpenDAL** | Rust | 50+ | 无 | Rust SDK | 纯存储抽象层 |
| **s3fs** | C++ | 1 | 无 | FUSE | 只做 S3 映射 |

### 12.2 Agent 上下文

| 产品 | 持久上下文 | 工作流复用 | 多 Agent | EVIF 差异 |
|------|-----------|-----------|---------|----------|
| **EVIF** | ContextFS L0/L1/L2 | SkillFS | PipeFS | 文件系统抽象 |
| **Claude Projects** | 会话笔记 | ❌ | ❌ | 专有格式 |
| **Mem0** | 向量记忆 | ❌ | ❌ | 只做记忆 |
| **CrewAI** | ❌ | ❌ | ✅ 编排 | 框架级 |
| **LangGraph** | ❌ | ✅ Graph | ✅ 状态机 | 框架级 |

### 12.3 EVIF 的生态位

```
        存储抽象层 (OpenDAL/JuiceFS)
               +
        Agent 上下文 (Mem0/Claude Projects)
               +
        工作流复用 (LangGraph/GitHub Actions)
               =
            EVIF

但不是简单的加法，而是用"文件系统"这个统一抽象贯穿三者。
```

### 12.4 差异化方向

**EVIF 不应该和竞品正面竞争，而应该做"竞品的补充"：**

1. **OpenDAL + EVIF** = 50+ 存储后端 + Agent 原语
2. **Mem0 + EVIF** = 专业向量记忆 + 文件系统上下文
3. **LangGraph + EVIF** = 复杂编排 + 轻量技能

---

## 13. 总结

### 13.1 一句话定位

> **EVIF 是 AI Agent 的"文件系统基础设施"——通过一条命令 (`evif start`) 给任何 Agent 提供持久上下文、可复用工作流和标准化协作协议。**

### 13.2 核心口号

> **"给 AI Agent 一个文件系统。"**
> **"evif start = docker run for AI agents."**

### 13.3 当前最重要的三件事

1. **🔴 `evif start` 一键启动** — 像 `docker run` 一样简单（Phase 1）
2. **🔴 `@evif/mcp-server` MCP 工具** — Claude Code 原生消费 EVIF（Phase 2）
3. **🔴 `pip install evif`** — 零配置上手（Phase 3）

### 13.4 关键决策：SKILL.md → MCP 迁移

```
当前                   →  目标
──────────────────────────────────────────────────
.claude/skills/*.SKILL.md    →  @evif/mcp-server (MCP 工具)
人类读的文档                  → Agent 原生可调用的工具
自然语言触发                  → 确定性 API 调用
无 server 自动启动            → preSession hook 自动启动
无 fallback                  → CLAUDE.md 降级逻辑

CLAUDE.md 中的 EVIF 指令     → 保持 + 增强（增加 fallback）
evif CLI                     → 增强为 "Docker for AI"
Python SDK                   → 保持 + 修复 endpoint 映射
```

### 13.5 从 mem23 得出的核心洞察

经过真实执行验证，我发现了 EVIF 项目中几个之前没有被正视的问题：

1. **SKILL.md 不是 Agent 交互的最佳方式** — Claude Code 需要的不是文档，而是 MCP 工具
2. **Python SDK 有 3 个 endpoint 映射错误** — 代码从未在真实服务器上测试过
3. **Server 启动无问题，但 CLI 没有 `start` 命令** — 最常用的操作反而最麻烦
4. **SkillFS 的 input→output 流程工作正常** — 但 Claude Code 不会主动用
5. **Memory API 的 search 是字符串匹配** — 不是真的向量搜索

**一句话：技术架构优秀，但产品化方向需要调整——从"做更多功能"转向"让现有功能更容易被 Agent 消费"。**

---

## 附录 A：验证命令合集

```bash
# 1. 启动服务（关闭认证）
EVIF_REST_AUTH_MODE=disabled ./target/release/evif-rest --port 8081

# 2. 健康检查
curl http://localhost:8081/api/v1/health

# 3. 文件操作
curl -X PUT "localhost:8081/api/v1/files?path=/mem/test.txt" \
  -H "Content-Type: application/json" \
  -d '{"data":"hello world"}'
curl "localhost:8081/api/v1/files?path=/mem/test.txt"

# 4. Context
curl -X PUT "localhost:8081/api/v1/files?path=/context/L0/current" \
  -H "Content-Type: application/json" \
  -d '{"data":"my task"}'

# 5. Skill
curl -X PUT "localhost:8081/api/v1/files?path=/skills/code-review/input" \
  -H "Content-Type: application/json" \
  -d '{"data":"Review src/auth/"}'
curl "localhost:8081/api/v1/files?path=/skills/code-review/output"

# 6. Pipe
curl -X POST "localhost:8081/api/v1/directories" \
  -H "Content-Type: application/json" \
  -d '{"path":"/pipes/my-task"}'
curl "localhost:8081/api/v1/files?path=/pipes/my-task/status"

# 7. Memory
curl -X POST "localhost:8081/api/v1/memories" \
  -H "Content-Type: application/json" \
  -d '{"content":"test","modality":"knowledge"}'
curl "localhost:8081/api/v1/memories"
curl -X POST "localhost:8081/api/v1/memories/search" \
  -H "Content-Type: application/json" \
  -d '{"query":"test"}'

# 8. Python SDK
PYTHONPATH=crates/evif-python python3 -c "
from evif.sync import SyncEvifClient
c = SyncEvifClient('http://localhost:8081')
print('health:', c.health().status)
print('mounts:', len(c.mounts()))
c.write('/mem/test-py.txt', 'Hello from Python!')
print('cat:', c.cat('/mem/test-py.txt'))
print('memory:', c.memory_store('test', modality='knowledge'))
"
```

## 附录 B：关键文件

| 文件 | 作用 | 状态 |
|------|------|------|
| `crates/evif-core/src/radix_mount_table.rs` | Radix Tree 路由表 | ✅ Production |
| `crates/evif-plugins/src/contextfs.rs` | 三层上下文 | ✅ Production |
| `crates/evif-plugins/src/skillfs.rs` | SKILL.md 工作流 | ✅ Production |
| `crates/evif-plugins/src/pipefs.rs` | 多 Agent 协调 | ✅ Production |
| `crates/evif-rest/src/routes.rs` | 106 个路由定义 | ✅ Production |
| `crates/evif-rest/src/server.rs` | 服务启动 + 默认挂载 | ✅ Production |
| `crates/evif-python/evif/client.py` | Python 异步客户端 (542 行) | ✅ Fixed |
| `crates/evif-python/evif/sync.py` | Python 同步包装 (318 行) | ✅ |
| `crates/evif-python/evif/memory.py` | Memory API | ✅ |
| `crates/evif-python/evif/queue.py` | Queue/Pipe API | ✅ |
| `crates/evif-sdk-ts/src/client.ts` | TypeScript 客户端 | ⚠️ 基础版 |
| `crates/evif-sdk-go/evif/` | Go 客户端 | ⚠️ 半成品 |
| `crates/evif-cli/src/main.rs` | CLI | ✅ Production |
| `crates/evif-mcp/src/` | MCP Server | ❓ 未评估 |
| `.claude/skills/*.SKILL.md` | Claude Code 技能 (5 个) | ⚠️ 需转型为 MCP |
| `CLAUDE.md` | Claude Code 指令 | ⚠️ 需更新 fallback |
| `.claude/settings.json` | Hook 配置 | ⚠️ 需添加 preSession |

---

> **最后更新**：2026-04-27 v2
> **基于真实执行验证**：Server 启动 ✅ → 文件操作 ✅ → Context/Skill/Pipe/Memory ✅ → Python SDK ✅
> **发现并修复**：4 个 Python SDK endpoint bug, CLI "Docker for AI" 愿景, MCP 优先策略
> **核心转向**：SKILL.md 文档化 → MCP Server 工具化 + CLI "Docker for AI"
