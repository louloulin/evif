# EVIF mem13.md — 定位、架构重设计与后续计划（v11）

> 创建时间：2026-03-31
> 更新时间：2026-04-01（v11：✅ ALL COMPLETE — 所有 Phase 8-11 功能已实现并验证通过）
> 基于：EVIF 全面代码审计 + AGFS 源码分析 + OpenClaw 深度分析 + 行业调研（50+ 源）
> 调研范围：AGFS/OpenViking/OpenClaw/Claude Code/Codex/MCP/Rust Skills 生态/arXiv 论文

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
> 为 Claude Code / Codex / Cursor / OpenClaw 提供 "Context is File" 的统一基础设施。

### 2.2 EVIF 为 AI Agent 解决的核心问题

基于对 OpenClaw（145K+ GitHub stars）、Claude Code、Codex、Cursor 等主流 AI Agent 的深度分析，EVIF 解决以下**真实痛点**：

| 痛点 | 真实案例 | EVIF 解决方案 |
|------|----------|--------------|
| **上下文窗口溢出** | OpenClaw `context_length_exceeded` 崩溃，切换模型时硬崩溃 | Context Engine L0/L1/L2 分层加载 + 智能压缩 |
| **静默压缩丢数据** | OpenClaw compaction 静默丢弃关键指令，导致删除邮件事故 | 明确的上下文层级 + 可视化压缩过程 |
| **内存管理混乱** | OpenClaw GitHub Issue #43747: "Memory management is in chaos" | 统一的 Context Manager + 生命周期管理 |
| **跨上下文干扰** | OpenClaw 统一内存导致 Project A 偏好影响 Project B | ContextFS 隔离 + per-project 上下文 |
| **多 Agent 无法共享状态** | OpenClaw 单机 SQLite 无法支持多 Agent 协作 | QueueFS + PipeFS + 多后端存储 |
| **存储后端碎片化** | 每个 Agent 自建 PostgreSQL/Milvus/LanceDB 集成 | 统一 EvifPlugin trait + 20+ 后端插件 |
| **技能发现困难** | MCP Tool Discovery 摩擦大，tool descriptions "smelly" | SkillFS: `ls /skills/` + 标准 SKILL.md |
| **安全与审计缺失** | 12% ClawHub Skills 含恶意代码，企业无法审计 | RBAC + EncryptFS + 审计日志 |
| **多模态数据无法存储** | Markdown-first 设计无法存储图像/音频 | StreamFS + 二进制文件支持 |
| **大文件处理失败** | >500 行文件导致 Aider/VS Code AI 崩溃 | 分块读取 + L0 摘要 + 流式传输 |

### 2.3 与 OpenClaw 的对比分析

OpenClaw 是 2025-2026 年最流行的开源 AI Agent 框架（145K+ GitHub stars），其架构选择和暴露的问题为 EVIF 提供了宝贵的参考。

| 维度 | OpenClaw | EVIF | EVIF 优势 |
|------|----------|------|-----------|
| **核心语言** | TypeScript/Node.js | Rust | 性能、安全性、无 GC |
| **存储模型** | Markdown 文件 + SQLite | **EvifPlugin 抽象层** | 20+ 后端可选 |
| **上下文管理** | Legacy + 可插拔 Context Engine | **内建 L0/L1/L2 ContextFS** | 原生集成，无需插件 |
| **内存后端** | SQLite/QMD/Honcho/LanceDB 分裂 | **统一 QueueBackend trait** | 一致 API |
| **多 Agent 协调** | 单机假设，跨 Agent 共享困难 | **QueueFS + PipeFS** | 原生协调原语 |
| **技能发现** | ClawHub (12% 恶意代码) | **SkillFS + 标准 SKILL.md** | 文件系统发现 + 安全执行 |
| **安全模型** | 无审计机制 | **RBAC + 审计日志** | 企业级 |
| **中国云支持** | 无 | **OSS/COS/OBS** | 阿里云/腾讯云/华为云 |
| **FUSE 挂载** | 无 | **有** | POSIX 兼容 |
| **认证** | 无 | **RBAC + JWT** | 多租户支持 |

**OpenClaw 暴露的关键问题（EVIF 直接解决）**：

1. **"Context Gap" 架构缺陷**（Tacnode 分析）
   - OpenClaw 的 SQLite + Markdown 架构在多 Agent、跨机器、亚秒级新鲜度场景下崩溃
   - EVIF 通过 QueueFS + ContextFS 提供**事务性一致的多 Agent 共享状态**

2. **静默压缩问题**
   - OpenClaw 的 compaction 在后台静默进行，用户不知道指令被丢弃
   - EVIF 通过 `/context/.meta` 暴露压缩策略，`/context/L1/decisions.md` 保留关键决策

3. **内存搜索回归**
   - OpenClaw 有多个 `memory_search` 返回空结果的 regression bug
   - EVIF 通过 VectorFS + 稳定的 FAISS/Qdrant 后端避免此问题

4. **QMD 多 Agent 隔离失败**
   - OpenClaw QMD 路径全局配置，无法 per-agent 隔离
   - EVIF 通过 Radix Mount Table 实现 per-agent/per-session 挂载隔离

### 2.4 与 AGFS 的战略差异

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
| SDK | Go+Python | Go + Python + TypeScript ✅ | EVIF 领先 |

### 2.5 核心差异化：三件 AGFS/OpenClaw 都没做到的事

1. **Context Engine（上下文引擎）** — 内建 L0/L1/L2 分层上下文，解决 OpenClaw 静默压缩和上下文溢出问题
2. **SkillFS（技能文件系统）** — Agent 通过文件系统发现和注册技能（`ls /skills/` + `cat SKILL.md`），比 ClawHub 更安全
3. **原生多 Agent 协调** — QueueFS + PipeFS 提供事务性一致的多 Agent 共享状态，OpenClaw 的单机 SQLite 无法做到

---

## 三、OpenClaw 痛点深度分析

### 3.1 OpenClaw 概述

[OpenClaw](https://github.com/openclaw/openclaw)（原名 ClawdBot/MoltBot）是 2025-2026 年最流行的开源 AI Agent 框架，拥有 **145,000+ GitHub stars**。它使用 TypeScript/Node.js 构建，采用微内核架构。

**核心设计哲学**：Markdown 文件是唯一的真相来源。模型只"记住"保存到磁盘的内容，没有隐藏的向量数据库。

### 3.2 架构组件

```
┌─────────────────────────────────────────────────────────────────┐
│                        OpenClaw 架构                             │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    Gateway (Hub)                         │    │
│  │    WebSocket 控制平面，消息路由，会话管理，工具绑定       │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                   │
│  ┌───────────┬───────────┬───┴───┬───────────┬───────────┐     │
│  │ WhatsApp  │ Telegram  │ Slack │  Discord  │  iMessage │     │
│  │ (Baileys) │ (grammY)  │       │           │  (BlueBubb)│     │
│  └───────────┴───────────┴───────┴───────────┴───────────┘     │
│                              │                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              Agent Runtime (Pi Core)                     │    │
│  │   Models | Tools | Prompt Pipeline | Context Engine      │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                   │
│  ┌───────────┬───────────┬───┴───┬───────────┬───────────┐     │
│  │  Memory   │  Skills   │ Tools │  Context  │  Session  │     │
│  │  System   │  (ClawHub)│       │  Engine   │  Storage  │     │
│  └───────────┴───────────┴───────┴───────────┴───────────┘     │
│                                                                  │
│  Workspace: ~/.openclaw/workspace/                               │
│  ├── AGENTS.md    # 操作指令 + "记忆"                            │
│  ├── SOUL.md      # 人设、边界、语气                             │
│  ├── TOOLS.md     # 工具使用说明                                 │
│  ├── MEMORY.md    # 长期记忆（持久事实、偏好、决策）             │
│  └── memory/      # 每日笔记                                     │
│      └── YYYY-MM-DD.md                                          │
└─────────────────────────────────────────────────────────────────┘
```

### 3.3 真实痛点清单（来自 GitHub Issues + 社区）

| # | 痛点 | 严重性 | 来源 | EVIF 解决方案 |
|---|------|--------|------|---------------|
| 1 | **内存管理混乱** | Critical | [GitHub #43747](https://github.com/openclaw/openclaw/issues/43747) | Context Manager 统一生命周期 |
| 2 | **静默压缩丢数据** | Critical | [Medium 分析](https://medium.com/@dingzhanjun/analyzing-the-incident-of-openclaw-deleting-emails-a-technical-deep-dive-56e50028637b) | 可视化压缩 + L1/decisions.md |
| 3 | **压缩时机错误** | High | [GitHub #24031](https://github.com/openclaw/openclaw/issues/24031) | Token 预算管理 + 智能触发 |
| 4 | **上下文不匹配崩溃** | High | [GitHub #44303](https://github.com/openclaw/openclaw/issues/44303) | 模型上下文探测 + 优雅降级 |
| 5 | **跨上下文干扰** | High | [Deep Dive](https://grapeot.me/openclaw-en.html) | ContextFS per-project 隔离 |
| 6 | **内存搜索返回空** | High | [GitHub #46671](https://github.com/openclaw/openclaw/issues/46671) | VectorFS + 稳定向量后端 |
| 7 | **QMD 多 Agent 隔离失败** | High | [GitHub #19005](https://github.com/openclaw/openclaw/issues/19005) | Radix Mount Table 隔离 |
| 8 | **SQLite 并发问题** | Medium | [GitHub #16844](https://github.com/openclaw/openclaw/issues/16844) | QueueBackend trait + MySQL 选项 |
| 9 | **技能市场安全隐患** | Critical | [Nebius Security](https://nebius.com/blog/posts/openclaw-security) | SkillFS + WASM 沙箱执行 |
| 10 | **多模态数据存储** | Medium | 架构限制 | StreamFS + 二进制支持 |

### 3.4 "Context Gap" 问题详解

[Tacnode 的分析](https://tacnode.io/post/openclaw-and-the-context-gap)指出：OpenClaw 证明了 Agent 在个人使用场景下可行，但其架构（SQLite + Markdown 文件）在需要以下特性时崩溃：

1. **多 Agent 共享状态** — SQLite 无法提供事务性一致的跨 Agent 状态
2. **跨机器部署** — 本地文件系统假设无法扩展到云端
3. **亚秒级新鲜度** — 无订阅机制，Agent 轮询导致延迟
4. **访问控制** — 无 RBAC，无法区分 Agent 权限

**EVIF 的解决方案**：

```
OpenClaw 问题                    EVIF 解决方案
─────────────────────────────────────────────────────
SQLite 单写者              →     QueueBackend trait (MySQL/SQLite/Memory)
本地文件系统               →     EvifPlugin 抽象 (S3/SQLFS/OSS/...)
无订阅机制                 →     PipeFS + WebSocket 推送
无访问控制                  →     RBAC + per-agent 权限
Markdown-only              →     StreamFS + 二进制文件
```

### 3.5 OpenClaw 社区自建解决方案

由于 OpenClaw 架构限制，社区被迫自建：

| 项目 | 功能 | EVIF 对标 |
|------|------|-----------|
| **memsearch** (Milvus) | 提取 OpenClaw 内存架构为独立库 | VectorFS |
| **Aura** | 支持 60+ 格式的内存 + RAG 引擎 | ContextFS + VectorFS |
| **QMD** | 本地优先的 BM25 + 向量 + 重排序 | QueueBackend |
| **Honcho** | AI 原生跨会话内存 | evif-mem |
| **PostgreSQL + RAG** | 企业级扩展方案 | QueueFS MySQL + VectorFS |

**EVIF 的优势**：将这些分散的解决方案统一为一个文件系统接口，无需学习多个 API。

---

## 四、架构设计

### 4.1 EVIF 全局架构图

```
┌─────────────────────────────────────────────────────────────────────┐
│                        AI Agent 生态层                               │
│                                                                     │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────────┐   │
│  │Claude    │  │OpenAI    │  │OpenClaw  │  │Cursor / Continue │   │
│  │Code      │  │Codex     │  │          │  │/ Aider / SWE-Agent│   │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────────┬─────────┘   │
│       │              │              │                  │             │
│       └──────────────┴──────────────┴──────────────────┘             │
│                              │                                       │
│                   ┌──────────┴──────────┐                            │
│                   │   接入协议层         │                            │
│                   │ FUSE │ CLI │ REST   │                            │
│                   │ MCP  │ WebSocket    │                            │
│                   └──────────┬──────────┘                            │
└──────────────────────────────┼──────────────────────────────────────┘
                               │
┌──────────────────────────────┼──────────────────────────────────────┐
│                   EVIF Core Engine                                   │
│                              │                                       │
│  ┌───────────────────────────┴───────────────────────────────────┐  │
│  │                    Agent Access Layer                          │  │
│  │                                                               │  │
│  │  ┌────────────────────────────────────────────────────────┐   │  │
│  │  │  Layer 0: File Primitives (Primary)                    │   │  │
│  │  │  ls / cat / grep / write / mkdir / rm / find / stat    │   │  │
│  │  │  → 任何 LLM 天生理解，无需学习                          │   │  │
│  │  └────────────────────────────────────────────────────────┘   │  │
│  │  ┌──────────────────────────┐  ┌───────────────────────────┐  │  │
│  │  │ Layer 1: MCP Tools       │  │ Layer 2: Skills           │  │  │
│  │  │ 17 结构化工具             │  │ SKILL.md 声明式发现        │  │  │
│  │  │ Claude Desktop 兼容      │  │ ls /skills → discover     │  │  │
│  │  └──────────────────────────┘  └───────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌───────────────┐  ┌───────────────┐  ┌────────────────────────┐  │
│  │ Radix Mount   │  │ Plugin Loader │  │ Context Engine         │  │
│  │ Table         │  │ WASM + Native │  │ L0/L1/L2 管理器        │  │
│  │ O(k) 路由     │  │ 动态加载/卸载 │  │ Token 预算 + 压缩      │  │
│  └───────────────┘  └───────────────┘  └────────────────────────┘  │
│                                                                     │
│  ┌───────────────┐  ┌───────────────┐  ┌────────────────────────┐  │
│  │ Handle System │  │ Auth & RBAC   │  │ Metrics & Audit        │  │
│  │ 状态文件句柄   │  │ JWT + 权限    │  │ Prometheus + 日志      │  │
│  │ Lease + TTL   │  │ 多租户隔离    │  │ 不可变审计追踪          │  │
│  └───────────────┘  └───────────────┘  └────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                               │
┌──────────────────────────────┼──────────────────────────────────────┐
│                   Plugin Layer (20+ 插件)                            │
│                              │                                       │
│  ┌─ Context Plugins ─────────┴──────────────────────────────────┐  │
│  │                                                               │  │
│  │  ContextFS          MemoryFS         SessionFS       SkillFS │  │
│  │  ┌─────────┐       ┌─────────┐     ┌──────────┐    ┌───────┐│  │
│  │  │L0 即时  │       │长期记忆 │     │会话状态  │    │技能   ││  │
│  │  │L1 会话  │       │MD 格式  │     │跨会话    │    │SKILL  ││  │
│  │  │L2 知识  │       │语义搜索 │     │恢复      │    │.md    ││  │
│  │  └─────────┘       └─────────┘     └──────────┘    └───────┘│  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌─ Coordination Plugins ───────────────────────────────────────┐  │
│  │  QueueFS (FIFO)  │  PipeFS (Agent 管道)  │  HeartbeatFS     │  │
│  │  SQLite/MySQL    │  双向通信 + 状态     │  心跳 + 租约     │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌─ Storage Plugins ────────────────────────────────────────────┐  │
│  │                                                               │  │
│  │  本地:  MemFS │ LocalFS │ SQLFS │ SQLFS2                    │  │
│  │  云:    S3FS │ Azure Blob │ GCS │ OSS │ COS │ OBS │ MinIO  │  │
│  │  安全:  EncryptedFS │ TieredFS                            │  │
│  │  工具:  KVFS │ CatalogFS │ HandleFS │ ServerInfoFS         │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌─ Compute Plugins ────────────────────────────────────────────┐  │
│  │  VectorFS (FAISS/Qdrant) │ GPTFS │ ProxyFS │ StreamFS      │  │
│  │  StreamRotateFS          │ DevFS │ HelloFS │ WASM Runtime  │  │
│  └───────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                               │
┌──────────────────────────────┼──────────────────────────────────────┐
│                   SDK 层                                             │
│                              │                                       │
│  ┌──────────────┐  ┌────────┴───────┐  ┌────────────────────────┐ │
│  │ Go SDK ✅    │  │ Python SDK ✅  │  │ TypeScript SDK ✅      │ │
│  │ 完整 API     │  │ httpx+asyncio │  │ fetch API              │ │
│  │ 流式读写     │  │ 37 tests ✅   │  │ 69 tests ✅            │ │
│  └──────────────┘  └────────────────┘  └────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

### 4.2 数据流架构图

```
Agent 请求 → REST API / CLI / FUSE
                │
                ▼
         Radix Mount Table (O(k) 路由)
                │
    ┌───────────┼───────────┐
    ▼           ▼           ▼
/context/   /skills/    /storage/
    │           │           │
    ▼           ▼           ▼
ContextFS   SkillFS     S3FS/SQLFS/
  L0/L1/L2  SKILL.md    MemFS/...
    │           │           │
    ▼           ▼           ▼
SQLite     agent-skills   云存储
VectorFS   skill-runtime  本地文件
```

### 4.3 Context Engine 架构图

```
┌──────────────────────────────────────────────────────────────────┐
│                    Context Engine 架构                             │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  L0: 即时上下文 (~200 tokens, 毫秒级)                    │   │
│  │  ┌────────────┐  ┌────────────┐  ┌───────────────────┐  │   │
│  │  │ /current   │  │ /recent_ops│  │ /active_files/    │  │   │
│  │  │ 当前工作    │  │ 最近操作   │  │ 文件摘要列表     │  │   │
│  │  │ 上下文      │  │ 记录       │  │ (前20行+结构)    │  │   │
│  │  └────────────┘  └────────────┘  └───────────────────┘  │   │
│  │  加载策略: 每个 session 启动时自动加载                    │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                    │
│                     按需深入 (grep / cat)                         │
│                              │                                    │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  L1: 会话上下文 (~2000 tokens, 秒级)                     │   │
│  │  ┌────────────┐  ┌────────────┐  ┌───────────────────┐  │   │
│  │  │ /session_id│  │/decisions  │  │ /scratch/         │  │   │
│  │  │ 会话标识    │  │.md 决策记录│  │ 临时推理笔记     │  │   │
│  │  └────────────┘  └────────────┘  └───────────────────┘  │   │
│  │  加载策略: Agent 主动 cat 时加载                         │   │
│  │  压缩策略: session 结束时 L1 → L2 归档                   │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                    │
│                     按需深入 (grep / 搜索)                        │
│                              │                                    │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  L2: 项目知识库 (按需加载, 可无限扩展)                    │   │
│  │  ┌────────────┐  ┌────────────┐  ┌───────────────────┐  │   │
│  │  │/architect  │  │ /patterns  │  │ /embeddings/      │  │   │
│  │  │ure.md      │  │.md 代码模式│  │ 语义索引         │  │   │
│  │  │ 项目架构    │  │ 和约定     │  │ (VectorFS)       │  │   │
│  │  └────────────┘  └────────────┘  └───────────────────┘  │   │
│  │  加载策略: grep/语义搜索时按需加载                        │   │
│  │  持久化: SQLite + 可选向量数据库                         │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Context Manager Service                                 │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐ │   │
│  │  │Token 预算│  │自动摘要  │  │生命周期  │  │压缩策略│ │   │
│  │  │管理      │  │生成      │  │管理      │  │L2→L1→L0│ │   │
│  │  └──────────┘  └──────────┘  └──────────┘  └─────────┘ │   │
│  └──────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────┘
```

### 4.4 多 Agent 协调架构图

```
┌──────────────────────────────────────────────────────────────────┐
│                    多 Agent 协调架构                                │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────┐     QueueFS          ┌──────────┐                 │
│  │ Agent A  │──── enqueue ────────▶│ Agent B  │                 │
│  │ (生产者) │                      │ (消费者) │                 │
│  └──────────┘                      └──────────┘                 │
│       │                                │                         │
│       │  PipeFS                        │                         │
│       │  /pipes/task-001/             │                         │
│       │  ├── input (A 写)             │                         │
│       │  ├── output (B 写)            │                         │
│       │  ├── status (pending→done)    │                         │
│       │  └── assignee (当前处理者)    │                         │
│       │                                │                         │
│       └──────────── ───────────────────┘                        │
│                                                                  │
│  ┌──────────┐     HeartbeatFS       ┌──────────┐                │
│  │ Agent C  │◀── heartbeat ────────│ Agent D  │                │
│  │ (监控)   │── liveness check ──▶│ (工作者) │                │
│  └──────────┘                      └──────────┘                │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  共享上下文层 (ContextFS)                                  │   │
│  │  /context/L0/team_status    → 团队当前状态               │   │
│  │  /context/L1/task_assignments → 任务分配                 │   │
│  │  /context/L2/shared_knowledge → 共享知识库               │   │
│  └──────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────┘
```

### 4.5 SkillFS 交互架构图

```
┌──────────────────────────────────────────────────────────────────┐
│                    SkillFS 交互架构                                │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  AI Agent                                                        │
│  │                                                               │
│  │  1. 发现技能: ls /skills/                                     │
│  │     → code-review  test-gen  doc-gen  refactor                │
│  │                                                               │
│  │  2. 理解技能: cat /skills/code-review/SKILL.md               │
│  │     → YAML frontmatter (name, description, triggers)         │
│  │     → Markdown body (执行指令)                               │
│  │                                                               │
│  │  3. 触发技能:                                                 │
│  │     a) Claude Code 方式: 自然语言 "review this" → triggers 匹配│
│  │     b) 文件方式: echo JSON > /skills/code-review/input       │
│  │                                                               │
│  │  4. 获取结果: cat /skills/code-review/output                 │
│  │                                                               │
│  └──────────────────┬───────────────────────────────────────────┘
│                     │                                            │
│  ┌──────────────────┴───────────────────────────────────────────┐│
│  │              Skill Execution Engine                           ││
│  │  ┌────────────┐  ┌────────────┐  ┌────────────────────────┐ ││
│  │  │ Native     │  │ WASM       │  │ Docker                 │ ││
│  │  │ (开发)     │  │ (推荐)     │  │ (最安全)               │ ││
│  │  │ skill-rt   │  │ skill-rt   │  │ skill-rt               │ ││
│  │  └────────────┘  └────────────┘  └────────────────────────┘ ││
│  └──────────────────────────────────────────────────────────────┘│
│                                                                  │
│  ┌──────────────────────────────────────────────────────────────┐│
│  │              互操作层                                         ││
│  │  EVIF /skills/*/SKILL.md ←→ .claude/skills/  (Claude Code)  ││
│  │  EVIF /skills/*/SKILL.md ←→ codex skills/    (OpenAI Codex) ││
│  │  EVIF /skills/*/SKILL.md → rmcp MCP tools   (MCP 暴露)     ││
│  └──────────────────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────────────┘
```

### 4.6 EVIF 代码库架构（实际实现）

```
EVIF Crate 结构
├── evif-core/          # 核心库：Plugin trait、Mount Table、Handle System
├── evif-plugins/       # 20+ 文件系统插件实现
├── evif-rest/          # HTTP REST API (50+ endpoints) + WebSocket
├── evif-fuse/          # FUSE 内核挂载 (Linux/BSD)
├── evif-cli/           # 命令行工具 (40+ Unix 命令)
├── evif-client/        # Rust 客户端库
├── evif-mem/           # 记忆平台：MD 格式 + 语义搜索
├── evif-auth/          # 认证授权：JWT + RBAC
├── evif-metrics/       # 指标收集：Prometheus
├── evif-mcp/           # MCP 协议集成
├── evif-macros/        # 过程宏
├── evif-sdk-go/        # Go SDK ✅ (完整 API + 流式读写)
├── evif-sdk-python/    # Python SDK ✅ (httpx+asyncio, 37 tests)
└── evif-sdk-ts/        # TypeScript SDK ✅ (fetch API, 69 tests)
```

### 4.7 Context Engine 设计（核心差异化）

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

### 4.8 SkillFS 设计（完全兼容 Claude Code + Codex 协议）

**核心原则：不发明自定义格式，直接使用行业标准**

#### Claude Code SKILL.md 协议

Claude Code 的 Skills 使用 `SKILL.md` 文件，格式如下：

```markdown
---
name: code-review
description: "Review code for bugs, security issues, and best practices"
triggers:
  - "review"
  - "code review"
  - "check my code"
---

# Code Review Skill

You are a code reviewer. Analyze the given file for:
1. Security vulnerabilities
2. Performance issues
3. Code style violations

Focus areas: {focus}
File path: {path}

## Steps
1. Read the file
2. Identify issues
3. Write a structured report
```

- **Frontmatter** (YAML between `---`): `name`, `description`, `triggers`（触发关键词）
- **Body** (Markdown): 执行指令，支持变量占位符 `{variable}`
- **存放位置**: `.claude/skills/` 目录下

#### Codex Skills 协议（`codex-rs` 已用 Rust 实现）

Codex 使用 `SKILL.md` + `agents/openai.yaml` 双格式：
- `SKILL.md` — 与 Claude Code 完全相同的格式（开放 Agent Skills 标准）
- `agents/openai.yaml` — OpenAI 特定的元数据（UI 展示、调用策略、工具依赖）
- `AGENTS.md` — 项目级 Agent 指令

**关键发现**：Codex CLI 已用 Rust 重写（`codex-rs`），其中的 skill 解析逻辑可直接参考。
Rust crate `agent-skills` (Govcraft/agent-skills) 已实现了 SKILL.md 的解析和验证。

#### EVIF SkillFS 实现：直接集成行业标准

**不使用自定义 manifest.yaml，直接解析 SKILL.md 格式：**

```
/skills/
├── README                              # 技能导航（自动生成）
├── code-review/
│   ├── SKILL.md                        # 标准 SKILL.md（Claude Code/Codex 兼容）
│   │   ---
│   │   name: code-review
│   │   description: "Review code for bugs, security, and best practices"
│   │   triggers: ["review", "code review", "check my code"]
│   │   ---
│   │   # Code Review
│   │   Analyze the given file...
│   ├── examples/                       # 示例（可选）
│   └── templates/                      # 模板（可选）
│
├── test-gen/
│   └── SKILL.md
├── doc-gen/
│   └── SKILL.md
└── refactor/
    └── SKILL.md
```

**Rust 实现栈**：
```
agent-skills crate   → 解析和验证 SKILL.md（frontmatter + body）
gray_matter crate    → 提取 YAML frontmatter，serde 反序列化
rmcp crate (v1.3.0)  → 将技能暴露为 MCP tools（Claude Code 可发现）
skill-runtime crate  → WASM/Docker/Native 安全执行引擎（生产环境）
```

**Agent 交互方式（文件元命令）**：
```bash
# 发现技能
ls /skills/
# → code-review  test-gen  doc-gen  refactor

# 理解技能（读取标准 SKILL.md）
cat /skills/code-review/SKILL.md

# 触发技能（Claude Code 方式：自然语言匹配 triggers）
# Claude Code 自动匹配: "review this code" → code-review skill

# 触发技能（文件系统方式）
echo '{"path":"/src/auth.rs","focus":"security"}' > /skills/code-review/input
cat /skills/code-review/output
```

**与 Claude Code/Codex 的互操作**：
```
EVIF /skills/*/SKILL.md  ←→  .claude/skills/*/SKILL.md   (完全相同格式)
EVIF /skills/*/SKILL.md  ←→  codex skills/*/SKILL.md     (完全相同格式)
Skill discovery           ←→  ls /skills/ (EVIF) = scan .claude/skills/ (Claude Code)
```

**执行方式（三层安全模型）**：
```
Native  → 开发环境：直接执行（最快）
WASM    → 沙箱执行：通过 skill-runtime crate（安全）
Docker  → 隔离执行：通过 skill-runtime crate（最安全）
```

### 4.9 PipeFS 设计（多 Agent 协调）

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

## 五、问题清单

### 5.1 战略级问题（P0）

| # | 问题 | 根因 | 解决方案 |
|---|------|------|----------|
| S1 | MCP 被当作主要接入方式 | 误解了 Claude Code 的工作模式 | 文件元命令优先，MCP 降级为辅助 |
| S2 | 无 Context Engine | evif-mem 未被实际使用 | 新建 ContextFS 插件，集成 evif-mem |
| S3 | 无 SkillFS | 未考虑技能发现场景 | 新建 SkillFS，对标 SKILL.md |
| S4 | 缺少 Claude Code 原生集成示例 | 无 FUSE + CLAUDE.md 指南 | 编写集成指南和示例 |
| S5 | 叙事不清 | 定位模糊 | 重写 README：Context FileSystem |

### 5.2 架构级问题（P1）

| # | 问题 | 解决方案 |
|---|------|----------|
| A1 | 无 L0/L1/L2 分层 | ContextFS 实现 OpenViking 式分层加载 |
| A2 | 无 Agent 间通信原语 | PipeFS 基于 QueueFS 扩展 |
| A3 | SDK 不全（Python/TS） | 实现 Python + TypeScript SDK |
| A4 | 无 Web Shell | React + Vite 管理界面 |
| A5 | OpenAPI 文档缺失 | 自动生成 OpenAPI 3.0 spec |

### 5.3 代码级问题（P2）

| # | 问题 | 文件 |
|---|------|------|
| C1 | WebDAV/FTP/SFTP 因 OpenDAL TLS 冲突禁用 | evif-plugins/Cargo.toml |
| C2 | TypeScript SDK 空壳 | evif-sdk-ts/ |
| C3 | Python SDK 空壳 | evif-sdk-python/ |
| C4 | evif-metrics 仅基础框架 | evif-metrics/src/ |
| C5 | Go SDK 缺少错误重试和断路器 | evif-sdk-go/ |
| C6 | REST API 缺少 OpenAPI 文档 | evif-rest/ |

---

## 六、实施计划

### Phase 8: Context Engine（核心差异化，P0，预估 20h）

**目标**：EVIF 内建 L0/L1/L2 上下文管理，成为 AI Agent 的长期记忆层

#### 8.1 ContextFS 插件（12h）

- [x] `ContextFS` 插件实现
  - [x] L0 即时层：`/context/L0/current`、`/context/L0/active_files/`
  - [x] L1 会话层：`/context/L1/decisions.md`、`/context/L1/scratch/`
  - [x] L2 知识层：`/context/L2/architecture.md`、`/context/L2/patterns.md`
  - [x] 自动压缩：L2 文件超过阈值自动生成 `.summary` 伴生文件，按需生成摘要（10 tests ✅）
  - [x] 持久化：跨会话恢复，基于 SQLite（✅ `new_with_persistence(db_path)`，L0/L1 文件自动持久化，重启后恢复，3 tests ✅）
  - [x] 语义检索：集成 VectorFS 搜索 L2 知识库（✅ `semantic_search()` + 文本 fallback，70 tests ✅）
  - 进度说明：已完成最小可用 `ContextFS` 插件、内建种子文件、插件目录注册、REST 挂载接入，**自动压缩（`.summary` 伴生文件 + 按需生成）和操作追踪（`/L0/recent_ops`）已实现并通过 16 项测试验证**
- [x] Context Manager 服务（✅ `context_manager.rs`，9 tests ✅）
  - [x] 上下文生命周期管理（创建、更新、过期、归档）
  - [x] Token 预算管理（Anthropic 的 "smallest possible set" 原则）
  - [x] 上下文搜索（递归 grep L0/L1/L2）
  - [x] 自动摘要生成（调用 LLM 或本地摘要）（✅ `generate_summary()` + OpenAI fallback）
- [x] 测试：ContextFS 完整单元测试（16 tests ✅: 分层结构、持久化、.meta v2、自动压缩、按需摘要、recent_ops 追踪、L2 排除、滑动窗口上限、配置参数、SQLite 持久化 ×3、token 估算、session 生命周期、budget 状态追踪）

#### 8.2 Claude Code 集成指南（4h）

- [x] `CLAUDE.md` 模板 — EVIF 项目快速上手
- [x] FUSE 挂载 + Claude Code 工作流示例
- [x] `/context` 目录使用最佳实践文档
- [x] 多 Agent 协调示例（QueueFS + PipeFS）

#### 8.3 Codex CLI 集成指南（2h）

- [x] `AGENTS.md` 模板 — EVIF + Codex 配置
- [x] `agents/openai.yaml` 技能定义
- [x] REST API + Codex 工作流示例

#### 8.4 README.md 重写（2h）

- [x] "Context FileSystem for AI Agents" 定位叙事
- [x] 架构图（三层：Agent Access → Core → Plugins）
- [x] 快速上手：3 分钟搭建 EVIF + Claude Code 环境
- [x] 30 秒演示：ls /context → cat /context/L0/current → write decision

### Phase 9: SkillFS + PipeFS（Agent 协作，P1，预估 15h）

#### 9.1 SkillFS 技能文件系统（8h）

- [x] `SkillFS` 插件实现（使用标准 SKILL.md 格式，不发明自定义格式）
  - [x] 集成 `agent-skills` crate — 解析和验证 SKILL.md（✅ 内联实现 `SkillMetadata`/`SkillValidationError`/`validate_skill_md()`，23 tests ✅；因 Rust edition 不兼容（agent-skills 2024 vs EVIF 2021）采用 inline 方式）
  - [x] 集成 `gray_matter` crate — YAML frontmatter 提取（✅ 已替换手动解析）
  - [x] 技能发现：`ls /skills/` + `cat /skills/*/SKILL.md`
  - [x] 技能触发：自然语言匹配 `triggers` 字段（Claude Code 方式）
  - [x] 技能调用：`write /skills/*/input` → `read /skills/*/output`
  - [x] 技能注册：`mkdir /skills/new-skill/` + `write SKILL.md`
  - 进度说明：已完成最小可用 `SkillFS` 插件，兼容标准 `SKILL.md` frontmatter/body、内置 4 个技能模板，**`gray_matter` 集成完成（替换手动 YAML 解析），通过 13 项测试验证（含复杂 YAML 和无效 frontmatter 测试）**；**`agent-skills` 验证逻辑内联实现，23 tests ✅**
- [x] MCP 暴露：`evif-mcp` crate — 将 Skills 暴露为 MCP tools（✅ 20 个工具：17 文件操作 + 3 SkillFS 工具，21 tests ✅，`run_stdio()` 支持 Claude Desktop，`claude-desktop-config.json`）
  - [x] 每个 SKILL.md 自动注册为 MCP tool（✅ `evif_skill_list`/`evif_skill_info`/`evif_skill_execute`）
  - [x] Claude Code 通过 MCP 协议发现和调用技能（✅ stdio transport + Claude Desktop config）
- [x] 安全执行：`skill_runtime.rs` — Native/WASM/Docker 三模式框架（✅ Native 执行完整实现 + WASM sandbox 实现，21 tests ✅）
  - [x] Native 执行（开发环境）（✅ `execute_skill()` + `SkillExecutionContext`，含 timeout/env/verbose 配置）
  - [x] WASM sandbox（生产推荐）（✅ `execute_wasm_impl()` 使用 wasmtime v26 + WASI Preview 1，含 fuel 限制、内存隔离，`build_skill_wasm_module()` 生成最小 WASM 模块）
  - [x] Docker isolation（最高安全）（✅ `execute_docker()` 使用 Docker CLI，支持镜像自定义、资源限制、超时控制，2 Docker tests ✅）
- [x] 内置技能模板（标准 SKILL.md 格式）（4 tests ✅）
  - [x] `code-review` — 代码审查（安全、性能、风格）
  - [x] `test-gen` — 测试生成
  - [x] `doc-gen` — 文档生成
  - [x] `refactor` — 代码重构建议
- [x] 与 Claude Code/Codex 互操作（✅ 6 tests）
  - [x] `/skills/*/SKILL.md` = `.claude/skills/*/SKILL.md`（相同格式）
  - [x] 自动生成 `agents/openai.yaml`（Codex 兼容）
  - [x] 符号链接：EVIF `/skills/` → `.claude/skills/`
- [x] 测试：SkillFS 完整单元测试（13 tests ✅: 内置技能发现、自定义技能注册/执行、复杂 YAML 解析、无效 frontmatter 拒绝、Claude/Codex 互操作、openai.yaml 生成）

#### 9.2 PipeFS Agent 管道（7h）

- [x] `PipeFS` 插件实现（4 tests ✅）
  - [x] 创建管道：`mkdir /pipes/task-001`
  - [x] 双向通信：`input`/`output` 文件
  - [x] 状态监控：`status` 文件（pending → running → completed/failed）
  - [x] 超时和自动清理
  - [x] 广播模式：`/pipes/broadcast/` 一写多读
  - 进度说明：已完成最小可用 `PipeFS` 插件，支持管道状态流转、subscriber 广播和超时清理，并通过插件行为测试与 REST 挂载测试验证
- [x] 基于 QueueFS 扩展（复用 Backend trait）（✅ `new_with_backend(Arc<dyn QueueBackend>)`，input/output 持久化到后端队列，元数据保留在内存）
- [x] 测试：PipeFS 完整单元测试（4 tests ✅: 状态流转、广播、超时清理、QueueBackend 跨实例持久化）

### Phase 10: 开发者生态（P1，预估 25h）

#### 10.1 Python SDK（8h）

- [x] HTTP 客户端（httpx + asyncio）（✅ 已有 `client.py`）
- [x] 文件操作：read/write/list/stat/mkdir/rm/mv/cp（✅ 已有）
- [x] 挂载管理：mount/unmount/list（✅ 已有）
- [x] Context API：context_read/context_write/context_search 等 12 个方法（✅ `context.py` mixin）
- [x] Skill API：skill_discover/skill_execute 等 6 个方法（✅ `skill.py` mixin）
- [x] 流式读写支持（✅ `stream_read()`/`stream_write()` 使用 httpx `stream()` + `/api/v1/fs/stream` 原生字节流，12 streaming tests ✅）
- [x] 完整测试套件（pytest + httpx mock）（✅ 37 tests 全部通过）

#### 10.2 TypeScript SDK（8h）

- [x] Node.js 客户端（fetch API）（✅ `evif-sdk-ts/src/client.ts`）
- [x] TypeScript 类型定义（✅ `evif-sdk-ts/src/types.ts`）
- [x] 文件操作、挂载管理、Context API、Skill API（✅ 30+ 方法）
- [x] 流式读写支持（✅ `streamRead()`/`streamWrite()`，17 streaming tests ✅）
- [x] 完整测试套件（vitest）（✅ 52 tests 全部通过）

#### 10.3 Web Shell（9h）

- [x] React + Vite + TypeScript（✅ 已有）
- [x] 文件浏览器（树形 + 列表视图）（✅ 已有）
- [x] Monaco Editor 在线编辑（✅ 已有）
- [x] Context Explorer（L0/L1/L2 可视化）（✅ `ContextExplorer.tsx`）
- [x] Queue/Pipe 可视化（消息流、Agent 状态）（✅ `QueuePipePanel.tsx`，TypeScript 验证通过）
- [x] Skill Gallery（技能发现和管理）（✅ `SkillGallery.tsx`）
- [x] 实时日志流（WebSocket）（✅ `useWebSocket.ts` + `LogViewer.tsx`，TypeScript 验证通过）

### Phase 11: 生产增强（P2，预估 15h）

- [x] OpenAPI 3.0 自动生成（✅ `openapi.yaml`，2189 行，覆盖所有端点）
- [x] Prometheus metrics 完整实现 + Grafana 模板（✅ `prometheus_metrics()` 端点 + 10-panel Grafana dashboard `docs/grafana/evif-dashboard.json`，含 datasource/dashboard provisioning + Prometheus scrape config）
- [x] Go SDK 增强：重试/断路器（✅ `retry.go`，RetryConfig + CircuitBreaker，10 tests ✅）
- [x] CI/CD：多平台 Release + Docker + 性能基准（✅ `.github/workflows/ci.yml` 含 check/test/build-release/docker/frontend jobs，`Dockerfile` multi-arch build + push）

---

## 七、里程碑

| 里程碑 | Phase | 交付物 | 预估 |
|--------|-------|--------|------|
| M1: Context FileSystem 基础版 | 8 | ContextFS + Claude Code 集成 + README 重写 | 20h |
| M2: Agent 协作平台 | 9 | SkillFS + PipeFS + 多 Agent 示例 | 15h |
| M3: 开发者生态 | 10 | Python SDK + TypeScript SDK + Web Shell | 25h |
| M4: 生产就绪 | 11 | OpenAPI + Metrics + CI/CD | 15h |

---

## 八、关键设计决策

### 9.1 为什么弱化 MCP？

1. **Claude Code 的实际工作方式**：使用 glob/grep/head/tail/read/write，不是 MCP 工具
2. **Anthropic 自己的验证**："primitives like glob and grep allow it to navigate its environment just-in-time"
3. **MCP 的扩展性问题**：100+ 工具时 context burn 和成本暴增（Tool-RAG 正在解决）
4. **通用性**：ls/cat/grep 任何 LLM 都懂，MCP 需要专门的客户端支持

### 9.2 为什么 Skills 必须完全兼容 Claude Code/Codex 协议？

1. **开放标准已存在**：Claude Code 的 `SKILL.md` 和 Codex 的 `SKILL.md` 使用完全相同的格式
2. **Rust 生态已就绪**：
   - `agent-skills` crate — 专门解析 SKILL.md 的 Rust 库
   - `gray_matter` crate — YAML frontmatter 提取（serde 兼容）
   - `rmcp` crate (v1.3.0, 4.7M downloads) — 官方 Rust MCP SDK，将 Skills 暴露为 MCP tools
   - `skill-runtime` crate — WASM/Docker/Native 安全执行引擎
   - `codex-rs` — OpenAI 官方 Rust 参考实现
3. **零成本互操作**：EVIF `/skills/*/SKILL.md` = `.claude/skills/*/SKILL.md` = Codex skills
4. **不应自造格式**：自定义 `manifest.yaml` 会导致生态割裂，用户需要学习两套格式
5. **实证数据**：Meta-Tool 论文证明组合式工具减少 11.9% LLM 调用

### 9.3 为什么 Context Engine 是核心？

1. **OpenViking 已证明**：L0/L1/L2 分层加载是解决上下文窗口限制的有效方法
2. **Anthropic 推荐**：Compaction + Structured Note-taking + Sub-agent 都依赖上下文管理
3. **AGFS 没做到**：OpenViking 是外挂的，EVIF 可以原生集成
4. **差异化**：从 "虚拟文件系统" 升级为 "上下文文件系统"

---

## 九、Rust Skills 生态（直接集成）

### 10.1 已有的 Rust Skills 生态

| Crate | 功能 | 版本 | 下载量 | 用途 |
|-------|------|------|--------|------|
| `agent-skills` | 解析/验证 SKILL.md | published | — | **直接使用**：解析 EVIF Skills |
| `agent-skills-cli` | SKILL.md CLI 验证工具 | published | — | 开发时验证 Skills 格式 |
| `gray_matter` | YAML/JSON/TOML frontmatter 提取 | v0.3.2 | — | 解析 SKILL.md frontmatter |
| `rmcp` | 官方 Rust MCP SDK | v1.3.0 | 4.7M+ | 将 Skills 暴露为 MCP tools |
| `skill-runtime` | WASM/Docker/Native 技能执行 | v0.3.0 | — | 安全执行技能 |
| `skillsrs-runtime` | 技能编排+工作流+追踪 | published | — | 复杂技能工作流 |
| `rig-core` | LLM Agent 框架+工具调用 | published | — | LLM 集成层 |
| `codex-rs` | OpenAI Codex Rust 参考 | production | — | 参考实现 |

### 10.2 集成架构

```
EVIF SkillFS
├── Skill Parsing Layer
│   ├── agent-skills crate     → 解析 SKILL.md（frontmatter + body）
│   ├── gray_matter crate      → 提取 YAML frontmatter
│   └── serde + serde_yaml     → 反序列化为 Rust struct
│
├── Skill Discovery Layer
│   ├── ls /skills/            → 文件系统级发现
│   ├── cat SKILL.md           → 读取技能描述
│   └── triggers 匹配          → 自然语言触发（Claude Code 方式）
│
├── Skill Execution Layer
│   ├── skill-runtime crate    → WASM/Docker/Native 三种后端
│   ├── rmcp crate             → 暴露为 MCP tools（Claude Code 可发现）
│   └── Native fallback        → 直接 Rust 函数调用（开发环境）
│
└── Interop Layer
    ├── .claude/skills/ 链接    → 符号链接到 /skills/（Claude Code 兼容）
    ├── agents/openai.yaml      → 自动生成 Codex 元数据
    └── SKILL.md 转换器         → 双向兼容（如果格式有差异）
```

### 10.3 实施策略：直接使用现有 crate

**Phase 9 SkillFS 实施变更**：

原计划（v2）：自定义 manifest.yaml + instructions.md
新计划（v3）：直接使用 `agent-skills` + `gray_matter` crate

```toml
# Cargo.toml 新增依赖
[dependencies]
agent-skills = "0.1"       # SKILL.md 解析和验证
gray_matter = "0.3"        # YAML frontmatter 提取
rmcp = "1.3"               # MCP SDK（将 Skills 暴露为 tools）
skill-runtime = "0.3"      # 技能执行引擎（WASM/Docker/Native）
```

这避免了重复造轮子，直接复用已验证的 Rust 生态。

---

## 十、参考资料

### 学术论文
- [Everything is Context: Agentic File System Abstraction (arXiv:2512.05470)](https://arxiv.org/abs/2512.05470)
- [Optimizing Agentic Workflows using Meta-tools (arXiv:2601.22037)](https://arxiv.org/abs/2601.22037)
- [AIOS-LSFS: LLM-based Semantic File System (ICLR 2025)](https://github.com/agiresearch/AIOS-LSFS)

### AI Agent 集成
- [Effective Context Engineering for AI Agents — Anthropic](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
- [Claude Code Skills Documentation](https://code.claude.com/docs/en/skills)
- [Codex CLI AGENTS.md](https://developers.openai.com/codex/guides/agents-md/)
- [Codex Skills — OpenAI Developers](https://developers.openai.com/codex/skills/)
- [Claude Skills Deep Dive — Han Lee](https://leehanchung.github.io/blogs/2025/10/26/claude-skills-deep-dive/)

### Rust Skills 生态
- [agent-skills crate (Govcraft)](https://github.com/Govcraft/agent-skills) — SKILL.md 解析验证
- [skill-runtime crate (Kubiya)](https://github.com/kubiyabot/skill) — WASM/Docker/Native 执行
- [rmcp crate — Official Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk) (v1.3.0, 4.7M+ downloads)
- [gray_matter crate](https://crates.io/crates/gray_matter) — YAML frontmatter 提取
- [codex-rs — OpenAI Codex Rust 实现](https://github.com/openai/codex/blob/main/codex-rs/README.md)
- [agent-skills CLI](https://lib.rs/crates/agent-skills-cli)

### OpenClaw / AI Agent 痛点
- [OpenClaw GitHub Repository](https://github.com/openclaw/openclaw) — 145K+ stars 开源 AI Agent 框架
- [OpenClaw Context Docs](https://docs.openclaw.ai/concepts/context) — 上下文管理官方文档
- [OpenClaw Memory Docs](https://docs.openclaw.ai/concepts/memory) — 内存系统官方文档
- [OpenClaw Proves Agents Work — But Exposes the Context Gap (Tacnode)](https://tacnode.io/post/openclaw-and-the-context-gap) — Context Gap 核心分析
- [OpenClaw Deep Dive: Why It Went Viral](https://grapeot.me/openclaw-en.html) — 统一内存问题分析
- [We Extracted OpenClaw's Memory System (Milvus)](https://milvus.io/blog/we-extracted-openclaws-memory-system-and-opensourced-it-memsearch.md) — memsearch 独立库
- [OpenClaw Memory Architecture (coolmanns)](https://github.com/coolmanns/openclaw-memory-architecture) — 多层内存系统
- [GitHub Issue #43747: Memory management is in chaos](https://github.com/openclaw/openclaw/issues/43747) — 内存管理混乱问题
- [GitHub Issue #24031: contextTokens not used as compaction trigger](https://github.com/openclaw/openclaw/issues/24031) — 压缩时机错误
- [GitHub Issue #44303: Switching to smaller-context model causes crash](https://github.com/openclaw/openclaw/issues/44303) — 上下文不匹配崩溃
- [OpenClaw Security: Architecture and Hardening Guide (Nebius)](https://nebius.com/blog/posts/openclaw-security) — 安全分析
- [Context Is AI Coding's Real Bottleneck in 2026 (The New Stack)](https://thenewstack.io/context-is-ai-codings-real-bottleneck-in-2026/) — 上下文瓶颈分析
- [OpenHands Context Condensation](https://openhands.dev/blog/openhands-context-condensensation-for-more-efficient-ai-agents) — 上下文压缩实践
- [AI Agent Security in 2026 (Beam AI)](https://beam.ai/ar/agentic-insights/ai-agent-security-in-2026-the-risks-most-enterprises-still-ignore) — 企业安全风险

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

## 十一、与 mem12.md 的关系

mem12.md Phase 4-7 **已全部完成**（0 个未勾选项）：
- Phase 4: WASM 双后端（Extism + Wasmtime）✅
- Phase 5: S3 分片上传 ✅
- Phase 6: QueueFS MySQL + VectorFS SQLite + SQLFS ✅
- Phase 7: CI 每日构建 + ProxyFS 健康检查 ✅

mem13.md v3 核心变更（vs v2）：
1. **Skills 不再使用自定义 manifest.yaml**，直接使用标准 SKILL.md 格式
2. **集成现有 Rust crate**：`agent-skills` + `gray_matter` + `rmcp` + `skill-runtime`
3. **零成本互操作**：`/skills/*/SKILL.md` = `.claude/skills/*/SKILL.md` = Codex skills

---

> **v4 核心转变（vs v3）**：
> 1. **新增真实定位分析**：基于 OpenClaw（145K+ stars）深度分析，明确 EVIF 为 AI Agent 解决的 10 大痛点
> 2. **新增 OpenClaw 痛点深度分析**：GitHub Issues + 社区反馈 + 架构缺陷
> 3. **新增 6 个架构图**：全局架构、数据流、Context Engine、多 Agent 协调、SkillFS 交互、代码库结构
> 4. **新增解决方案映射**：每个痛点对应 EVIF 具体解决方案
> 5. **对比 OpenClaw**：TypeScript vs Rust、Markdown-only vs 多后端、单机 vs 多 Agent
