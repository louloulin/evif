# EVIF mem14.md — 后续实现计划与验证测试集

> 创建时间：2026-04-02
> 基于：EVIF v1.8 完整代码分析（141 文件, 67,165 行 Rust）+ AGFS/OpenViking/AIOS 对比研究 + arXiv 论文调研
> 研究范围：AIOS/AgentBench/ToolBench/MCP-AgentBench + Context Engineering 生态

---

## 一、EVIF 核心定位（修正版）

### 1.1 一句话定位

> **EVIF = AI Agent 的虚拟上下文文件系统（对标 OpenViking）**
> 以文件为核心，增强 Claude Code / Codex / Cursor 等 AI Agent 的 Context + Memory + Multi-Agent 协同能力

### 1.2 核心公式

```
EVIF = Virtual File System + Context Engine + Skill Platform + Multi-Agent Coordination
              ↓                   ↓              ↓                  ↓
          Radix Mount        L0/L1/L2       SKILL.md          PipeFS/QueueFS
              ↓                   ↓              ↓                  ↓
      统一存储抽象         上下文分层       技能发现          Agent 通信
      (对标 AGFS)       (对标 OpenViking)
```

### 1.3 与 AGFS 和 OpenViking 的关系

**EVIF 是 AGFS + OpenViking 的增强版：**

| 维度 | AGFS | OpenViking | EVIF (增强版) |
|------|------|------------|----------------|
| **定位** | 通用虚拟文件系统 | Context Database | **AI Agent 虚拟上下文文件系统** |
| **核心语言** | Go | Python | **Rust (性能 + 安全)** |
| **L0 摘要** | ❌ 无 | ✅ `.abstract` (~100 tokens) | ⚠️ 需实现 |
| **L1 概览** | ❌ 无 | ✅ `.overview` (~2000 tokens) | ⚠️ 需实现 |
| **L2 详情** | ❌ 无 | ✅ 完整内容 | ✅ 已实现 |
| **自动摘要** | ❌ 无 | ✅ LLM 自动生成 | ⚠️ 基础版 |
| **目录递归检索** | ⚠️ 基础 | ✅ 多层级递归 | ⚠️ 需增强 |
| **检索轨迹可视化** | ❌ 无 | ✅ 有 | ❌ 需实现 |
| **自动会话管理** | ❌ 无 | ✅ 端会话总结 | ❌ 需实现 |
| **技能系统** | ❌ 无 | ✅ 有 | ✅ **SkillFS** |
| **多 Agent** | QueueFS (单向) | ❌ 无 | ✅ **PipeFS (双向)** |
| **中国云存储** | ❌ 无 | ❌ 无 | ✅ **OSS/COS/OBS** |
| **认证** | ❌ 无 | ⚠️ 基础 | ✅ **RBAC + JWT** |
| **Token 减少** | N/A | **83% vs OpenClaw** | ⚠️ 目标 80%+ |

**OpenViking 性能基准（EVIF 对标目标）：**

| 对比项 | OpenViking | EVIF 目标 |
|--------|------------|-----------|
| vs OpenClaw | +49% 提升, -83% token | 目标: +50%, -80% |
| vs LanceDB | +17% 提升, -92% token | 目标: +20%, -85% |

### 1.4 OpenViking L0/L1/L2 架构详解

**OpenViking 是字节跳动火山引擎的 Context Database：**

```
viking://resources/my_project/
├── .abstract               # L0: 摘要 (~100 tokens, 1 句话)
│                          # 用途: 快速相关性检查
├── .overview              # L1: 概览 (~2000 tokens)
│                          # 用途: 规划和决策支持
├── docs/
│   ├── .abstract         # 每个目录都有 L0/L1
│   ├── .overview
│   ├── api/
│   │   ├── .abstract
│   │   ├── .overview
│   │   ├── auth.md       # L2: 完整内容
│   │   └── endpoints.md  # 按需加载
│   └── ...
└── src/
```

**OpenViking 核心创新：**
1. **Lazy Summary Generation** — 按需生成摘要，不预先生成
2. **Progressive Detail Loading** — 先加载 L0 → L1 → 按需加载 L2
3. **Token Optimization** — 80-90% token 减少
4. **Memory Self-Iteration** — 自动会话总结和记忆提取

### 1.3 与 AGFS 的关系

**EVIF 是 AGFS 的增强版，而非竞品：**

| 维度 | AGFS | EVIF (AGFS 增强版) |
|------|------|---------------------|
| **定位** | 通用虚拟文件系统 | **AI Agent 增强的虚拟上下文文件系统** |
| **核心语言** | Go | Rust (性能 + 安全) |
| **上下文管理** | ❌ 无 | ✅ **L0/L1/L2 分层** |
| **技能系统** | ❌ 无 | ✅ **SkillFS** |
| **多 Agent** | QueueFS (单向) | **PipeFS (双向)** |
| **中国云存储** | ❌ 无 | ✅ **OSS/COS/OBS** |
| **认证** | ❌ 无 | ✅ **RBAC + JWT** |

**EVIF 不是类似 Claude Code，而是增强 Claude Code：**

```
┌──────────────────────────────────────────────────────────────────┐
│                    AI Agent 生态                                 │
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────┐ │
│  │ Claude     │  │ OpenAI      │  │ Cursor     │  │ OpenClaw│ │
│  │ Code       │  │ Codex       │  │            │  │         │ │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └────┬────┘ │
│         │                │                │               │        │
│         └────────────────┴────────────────┴───────────────┘        │
│                              │                                    │
│                              ▼                                    │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                    EVIF (AGFS 增强版)                        │ │
│  │                                                              │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │ │
│  │  │ ContextFS    │  │ SkillFS      │  │ PipeFS           │  │ │
│  │  │ L0/L1/L2    │  │ SKILL.md     │  │ Multi-Agent      │  │ │
│  │  └──────────────┘  └──────────────┘  └──────────────────┘  │ │
│  │                                                              │ │
│  │  增强 AI Agent:                                                │ │
│  │  • 更长的上下文窗口 (L0/L1/L2 分层)                          │ │
│  │  • 持久化记忆 (文件即记忆)                                    │ │
│  │  • 技能发现与执行 (SKILL.md 标准)                            │ │
│  │  • 多 Agent 协同 (PipeFS 双向通信)                           │ │
│  └─────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

### 1.4 EVIF vs AIOS vs OpenViking 定位差异

| 系统 | 定位 | 类比 | 关系 |
|------|------|------|------|
| **AIOS** | LLM Agent 操作系统 | 类比 Linux Kernel | 底层平台 |
| **OpenViking** | Context Database | 类比 PostgreSQL | 数据存储 |
| **AGFS** | 通用虚拟文件系统 | 类比 FUSE | **EVIF 的基础** |
| **EVIF** | **AI Agent 增强的上下文文件系统** | 类比 AGFS + Context Engine | **AGFS 的超集** |

**EVIF 的差异化定位**：
1. **不是**通用虚拟文件系统（AGFS 的定位）
2. **不是**AI Agent 操作系统（AIOS 的定位）
3. **而是** AI Agent 的上下文增强层 —— 为 Claude Code/Codex/Cursor 提供更好的上下文、记忆和协同能力

---

## 二、研究发现总结

### 2.0 EVIF 定位验证：为什么文件系统是 AI Agent 的最佳接口

**学术验证：**

| 论文 | 关键发现 | 来源 |
|------|----------|------|
| [Agentic File System Abstraction](https://arxiv.org/abs/2512.05470) | 文件系统抽象是上下文工程的最佳接口 | CSIRO/Data61, 2025 |
| [Context Engineering: Virtual Memory for LLMs](https://www.linkedin.com/pulse/context-engineering-why-building-ai-agents-feels-like-scott-farrell-sf0cc) | LLM 上下文窗口类似虚拟内存系统——按需分页 | LinkedIn |
| [Martin Fowler: Context Engineering for Coding Agents](https://martinfowler.com/articles/exploring-gen-ai/context-engineering-coding-agents.html) | 最基本的上下文接口是文件读写和搜索 | Martin Fowler |

**工业验证：**

| 实践 | 关键发现 | 来源 |
|------|----------|------|
| [Claude Code 成功原因](https://www.reddit.com/r/LocalLLaMA/comments/1qwmxlw/i_built_a_virtual_filesystem_to_replace_mcp_for/) | Claude Code 擅长编程因为所有上下文都是文件 | Reddit |
| [Context File System (CFS)](https://www.cabeda.dev/reads) | 分层记忆架构：快速访问 + 自动过期 + 向量长期存储 | Data Impostor |
| [Beads: Memory Upgrade](https://github.com/steveyegge/beads) | 为编程 Agent 设计的持久记忆系统 | GitHub |
| [CLAUDE.md 永久记忆](https://www.mindstudio.ai/blog/what-is-claude-md-file-permanent-instruction-manual) | claude.md 文件作为永久上下文 | MindStudio |

**关键洞察：**
> "In 1969, Thompson and Ritchie decided 'everything is a file.' CFS inverts that: **everything is a context.**" — OpenClaw

### 2.1 Claude Code 增强研究

EVIF 作为 Claude Code 的增强层，支持以下功能：

| Claude Code 功能 | EVIF 增强 | 状态 |
|-----------------|-----------|------|
| CLAUDE.md | /context/L0/L1/L2 分层上下文 | ✅ 已实现 |
| Skills (.claude/skills/) | /skills/ SKILL.md 发现 | ✅ 已实现 |
| MCP Plugins | 20 个 MCP 工具 | ✅ 已实现 |
| Auto-memory | ContextFS 持久化记忆 | ✅ 已实现 |
| Subagents | PipeFS 多 Agent 协调 | ✅ 已实现 |

**Claude Code 28 个官方插件分析：**
- 大多数插件依赖上下文文件（CLAUDE.md）
- EVIF 可以统一管理这些上下文文件
- SkillFS 可以增强插件的技能发现能力

### 2.2 学术论文发现

| 论文 | 关键发现 | 对 EVIF 的启示 |
|------|----------|----------------|
| [AIOS: LLM Agent Operating System](https://arxiv.org/abs/2403.16971) | OS 级服务：调度、上下文、内存、存储、访问控制 | EVIF 可作为 AIOS 的存储/上下文层 |
| [Agentic File System Abstraction](https://arxiv.org/abs/2512.05470) | 文件系统抽象是上下文工程的最佳接口 | 验证 EVIF 核心方向正确 |
| [Optimizing Agentic Workflows using Meta-tools](https://arxiv.org/abs/2601.22037) | Meta Tool 减少 11.9% LLM 调用 | SkillFS 作为 Meta Tool 有学术支撑 |
| [Meta Context Engineering](https://arxiv.org/html/2601.21557v1) | 基础 Agent 累积处理实例到文件系统 | ContextFS L2 知识库设计正确 |
| [Solving Context Window Overflow](https://arxiv.org/html/2511.22729v1) | 任意长度工具响应的处理方法 | ContextFS L0/L1/L2 分层解决此问题 |
| [Structured Context Engineering](https://arxiv.org/pdf/2602.05447) | 文件原生代理系统的上下文工程研究 | EVIF 的文件原生设计有学术价值 |

### 2.3 基准测试发现

| 基准测试 | 描述 | EVIF 现状 |
|----------|------|----------|
| [OSWorld](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents) | OS 级 Agent 评估，包含文件系统状态检查 | ❌ 无对应测试 |
| [IDE-Bench](https://www.emergentmind.com/topics/ide-bench) | AI IDE Agent 评估：文件读写、导航任务 | ❌ 无对应测试 |
| [AgentBench](https://arxiv.org/abs/2308.03688) | 评估 LLM 作为 Agent 的多环境基准 | ❌ 无对应测试 |
| [ToolBench](https://github.com/sambanova/toolbench) | 工具操作能力评估 | ⚠️ 基础测试 |
| [MCP-AgentBench](https://arxiv.org/abs/2509.09734) | MCP 协议基准测试 | ⚠️ 基础连接测试 |
| [τ-bench](https://siera.ai/blog/benchmarking-ai-agents) | Agent 与用户/API 交互评估 | ❌ 无对应测试 |

**LlamaIndex 文件系统 Agent 基准：**
> 文件系统 Agent 在平均正确率上比传统 RAG 高 **2 分**

### 2.4 工业实践发现

| 来源 | 关键发现 | 对 EVIF 的启示 |
|------|----------|----------------|
| [Anthropic Context Engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) | 文件作为持久化记忆策略最优 | 验证 ContextFS 方向 |
| [MongoDB: Multi-Agent Memory Engineering](https://medium.com/mongodb/why-multi-agent-systems-need-memory-engineering-153a81f8d5be) | 多 Agent 失败原因是内存问题，不是通信问题 | EVIF 应加强 Memory 层 |
| [Anthropic Multi-Agent Research](https://www.anthropic.com/engineering/multi-agent-research-system) | 多 Agent 协调的挑战：协调、评估、可靠性 | EVIF 需要更好的测试和评估 |
| [Oracle: File System vs DB for Agent Memory](https://blogs.oracle.com/developers/comparing-file-systems-and-databases-for-effective-ai-agent-memory-management) | 并发写入可能静默损坏数据 | EVIF 需要更好的并发控制 |
| [Skills vs MCP](https://thenewstack.io/skills-vs-mcp-agent-architecture/) | Markdown 技能文件比 MCP 服务器减少 100 倍 token | SkillFS 设计正确 |

### 2.5 技能系统 vs MCP 架构

| 维度 | MCP 服务器 | SKILL.md 技能文件 |
|------|-----------|-------------------|
| Token 成本 | 高（完整 schema） | **低**（YAML frontmatter） |
| 发现方式 | 工具列表 API | **文件系统 ls** |
| 理解方式 | 结构化 JSON | **自然语言 Markdown** |
| EVIF 实现 | evif-mcp (20 工具) | **evif-skillfs (/skills/)** |

**结论：EVIF 同时支持 MCP 和 SKILL.md，兼顾两种架构**

---

## 三、代码分析：已实现 vs 待实现

### 3.1 代码规模统计

```
EVIF 总计: 141 文件, 67,165 行 Rust

组件拆分:
├── evif-core:      8,489 行 (核心: Plugin trait, Radix Mount, Handle)
├── evif-plugins:  18,765 行 (38 个插件实现)
├── evif-rest:      8,500 行 (50+ REST API 端点)
├── evif-mcp:       2,053 行 (20 MCP 工具)
├── evif-cli:       5,289 行 (40+ Unix 命令)
└── evif-auth:      1,069 行 (RBAC + JWT)
```

### 3.2 已实现功能

| 功能 | 文件 | 行数 | 测试数 |
|------|------|------|--------|
| ContextFS L0/L1/L2 | contextfs.rs | 805 | 16 |
| SkillFS (SKILL.md) | skillfs.rs | 1,135 | 13 |
| PipeFS (双向通信) | pipefs.rs | 454 | 4 |
| QueueFS (FIFO) | queuefs.rs | — | 4 |
| MCP Server | lib.rs (mcp) | 2,015 | 20 |
| Radix Mount Table | radix_mount.rs | — | 15 |
| Handle System | handle.rs | — | — |
| RBAC Auth | acl.rs | — | 8 |
| 38 个存储插件 | *.fs.rs | 18,765 | 287+ |

### 3.3 待实现功能（优先级排序）

#### P0 — 核心差距（影响 EVIF 竞争力）

| 功能 | OpenViking 实现 | EVIF 现状 | 实现方案 |
|------|-----------------|-----------|----------|
| **自动 LLM 摘要** | `.abstract` 自动生成 | ⚠️ 手动摘要 | 集成 GPTFS 或 OpenAI API |
| **目录递归检索** | 多层级递归 | ⚠️ 基础 grep | 重写 grep 支持递归 |
| **自动会话管理** | 端会话自动总结 | ❌ 无 | 添加 session lifecycle hook |
| **持久化记忆跨会话** | 自动记忆提取 | ⚠️ 基础 | ContextFS auto-memory |

#### P1 — Claude Code 增强功能

| 功能 | Claude Code 能力 | EVIF 现状 | 实现方案 |
|------|-----------------|-----------|----------|
| **更长的上下文** | CLAUDE.md 单文件 | ✅ L0/L1/L2 分层 | 增强 L2 向量搜索 |
| **技能发现** | .claude/skills/ | ✅ /skills/ SKILL.md | 增强触发词匹配 |
| **多 Agent** | Subagents | ✅ PipeFS | 增强状态流转 |
| **Auto-memory** | 基础 | ✅ /mem/ | 增强跨会话持久化 |

#### P2 — 生态功能（提升可发现性）

| 功能 | 对标基准 | 实现方案 |
|------|----------|----------|
| **OSWorld 对标** | OS 级 Agent 评估 | 创建文件系统状态测试 |
| **IDE-Bench 对标** | IDE Agent 评估 | 创建文件读写测试 |
| **AgentBench 对标** | 多环境 Agent 评估 | 创建 evif-bench crate |
| **MCP 完整测试** | MCP-AgentBench | 添加 50+ MCP 协议测试 |

---

## 四、后续实现计划

### Phase 12: Context Engine 增强（P0）

#### 12.1 自动 LLM 摘要生成

**目标**：当文件超过阈值时，自动调用 LLM 生成 `.abstract` 摘要

**实现方案**：
```rust
// crates/evif-plugins/src/contextfs.rs

// 添加自动摘要生成
async fn auto_summarize(&self, path: &str) -> EvifResult<String> {
    let content = self.read(path, 0, u64::MAX).await?;
    let summary = gptfs::summarize(&content).await?;
    self.write(&format!("{}.abstract", path), summary.as_bytes()).await?;
    Ok(summary)
}

// 触发条件：文件超过 4096 字节
const AUTO_SUMMARY_THRESHOLD: usize = 4096;
```

**验收标准**：
- [ ] 超过 4096 字节的文件自动生成摘要
- [ ] 摘要写入 `.abstract` 伴生文件
- [ ] 读取时优先返回摘要（可选）

#### 12.2 目录递归检索

**目标**：支持 `grep -r` 递归搜索

**实现方案**：
```rust
// crates/evif-core/src/grep.rs

pub async fn grep_recursive(
    &self,
    path: &str,
    pattern: &str,
    options: GrepOptions,
) -> EvifResult<Vec<GrepMatch>> {
    let mut results = Vec::new();
    let entries = self.list(path).await?;

    for entry in entries {
        if entry.is_dir {
            // 递归搜索子目录
            results.extend(self.grep_recursive(&entry.path, pattern, options).await?);
        } else {
            // 搜索文件
            if let Ok(matches) = self.grep_file(&entry.path, pattern, options).await {
                results.extend(matches);
            }
        }
    }
    Ok(results)
}
```

**验收标准**：
- [ ] `grep -r "pattern" /context/L2/` 递归搜索
- [ ] 返回匹配文件和行号
- [ ] 支持 `--include` 过滤

#### 12.3 自动会话管理

**目标**：会话结束时自动将 L1 归档到 L2

**实现方案**：
```rust
// crates/evif-plugins/src/contextfs.rs

// 会话生命周期钩子
pub async fn on_session_end(&self, session_id: &str) -> EvifResult<()> {
    // 1. 将 L1 关键决策归档到 L2
    let decisions = self.read("/L1/session_id/decisions.md").await?;
    let archive_path = format!("/L2/history/session_{}.md", session_id);
    self.write(&archive_path, decisions.as_bytes()).await?;

    // 2. 生成会话摘要
    let summary = self.generate_session_summary(session_id).await?;
    self.write(&format!("/L2/session_summary/{}.md", session_id), summary.as_bytes()).await?;

    // 3. 清理 L1 临时文件
    self.delete(&format!("/L1/scratch/{}", session_id)).await?;
    Ok(())
}
```

**验收标准**：
- [ ] 会话结束时自动触发归档
- [ ] L1 决策写入 L2 history
- [ ] 生成会话摘要

---

### Phase 13: 验证测试集（P1/P2）

#### 13.1 OSWorld 对标测试

**对标**：OSWorld 评估 Agent 在完整操作系统中的表现，文件系统状态检查是核心

**EVIF 测试设计**：
```rust
// crates/evif-bench/src/osworld.rs

/// OSWorld 对标：文件系统状态验证
#[tokio::test]
async fn test_file_system_state_after_task() {
    // 1. 创建测试文件树
    let server = TestServer::new().await;
    server.mkdir("/test/project").await;
    server.write("/test/project/main.rs", "fn main() {}").await;

    // 2. 模拟 Agent 执行任务
    // ... (通过 MCP 或 CLI 调用)

    // 3. 验证文件系统状态
    let entries = server.list("/test/project").await;
    assert!(entries.contains("main.rs"));

    // 4. 验证修改时间戳
    let stat = server.stat("/test/project/main.rs").await;
    assert!(stat.modified > start_time);
}

/// OSWorld 对标：并发文件操作
#[tokio::test]
async fn test_concurrent_file_operations() {
    let server = TestServer::new().await;
    let mut handles = Vec::new();

    // 100 个并发 Agent 同时操作
    for i in 0..100 {
        handles.push(tokio::spawn({
            let server = server.clone();
            async move {
                server.write(&format!("/test/file_{}", i), b"data").await
            }
        }));
    }

    let results = futures::future::join_all(handles).await;
    let success_count = results.iter().filter(|r| r.is_ok()).count();
    assert!(success_count >= 95); // 95% 成功率
}
```

#### 13.2 IDE-Bench 对标测试

**对标**：IDE-Bench 评估 AI IDE Agent 的文件读写和导航任务

**EVIF 测试设计**：
```rust
// crates/evif-bench/src/idebench.rs

/// IDE-Bench 对标：文件读取任务
#[tokio::test]
async fn test_ide_read_file() {
    let server = TestServer::new().await;

    // 写入测试文件
    server.write("/test/src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }").await;

    // 模拟 Agent 读取文件
    let content = server.read("/test/src/lib.rs").await.unwrap();
    assert!(content.contains("fn add"));
}

/// IDE-Bench 对标：目录导航任务
#[tokio::test]
async fn test_ide_navigation() {
    let server = TestServer::new().await;

    // 创建目录结构
    server.mkdir("/test/src").await;
    server.mkdir("/test/tests").await;
    server.write("/test/Cargo.toml", "[package]").await;

    // 模拟 Agent 导航
    let entries = server.list("/test").await;
    assert!(entries.contains("src"));
    assert!(entries.contains("tests"));
    assert!(entries.contains("Cargo.toml"));
}

/// IDE-Bench 对标：文件搜索任务
#[tokio::test]
async fn test_ide_search() {
    let server = TestServer::new().await;

    // 创建多个文件
    server.write("/test/a.rs", "fn test_a() {}").await;
    server.write("/test/b.rs", "fn test_b() {}").await;
    server.write("/test/c.rs", "fn test_c() {}").await;

    // 模拟 Agent 搜索 "fn test"
    let results = server.grep("/test", "fn test").await;
    assert_eq!(results.len(), 3);
}
```

#### 13.3 AgentBench 对标测试

**对标**：AgentBench 多环境评估框架

**EVIF 测试设计**：
```rust
// crates/evif-bench/src/agentbench.rs

/// AgentBench 对标：工具调用评估
#[tokio::test]
async fn test_tool_use_success_rate() {
    let server = TestServer::new().await;
    let mut success = 0;
    let total = 100;

    for i in 0..total {
        let result = server.mkdir(&format!("/test/dir_{}", i)).await;
        if result.is_ok() {
            success += 1;
        }
    }

    let rate = success as f64 / total as f64;
    assert!(rate >= 0.95); // 95% 成功率
}

/// AgentBench 对标：多步骤任务
#[tokio::test]
async fn test_multi_step_task() {
    let server = TestServer::new().await;

    // 步骤 1: 创建目录
    server.mkdir("/test/project").await.unwrap();

    // 步骤 2: 创建文件
    server.write("/test/project/main.rs", "fn main() {}").await.unwrap();

    // 步骤 3: 读取验证
    let content = server.read("/test/project/main.rs").await.unwrap();
    assert!(content.contains("fn main"));
}
```

#### 13.4 MCP 协议合规测试

**对标**：MCP-AgentBench 评估 MCP 协议实现

```rust
// crates/evif-mcp/tests/protocol_compliance.rs

/// MCP 初始化
#[tokio::test]
async fn test_mcp_initialize() {
    let response = send_mcp_request("initialize", json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {"name": "test", "version": "1.0"}
    })).await;

    assert!(response.contains("protocolVersion"));
    assert!(response.contains("capabilities"));
}

/// MCP 工具列表（验证 20 个工具）
#[tokio::test]
async fn test_mcp_tools_list() {
    let response = send_mcp_request("tools/list", json!({})).await;
    let tools = parse_tools(&response);

    assert_eq!(tools.len(), 20);
    assert!(tools.contains(&"evif_ls".to_string()));
    assert!(tools.contains(&"evif_cat".to_string()));
    assert!(tools.contains(&"evif_write".to_string()));
    assert!(tools.contains(&"evif_skill_execute".to_string()));
}

/// MCP 资源列表
#[tokio::test]
async fn test_mcp_resources_list() {
    let response = send_mcp_request("resources/list", json!({})).await;
    assert!(response.contains("resources"));
}

/// MCP Ping
#[tokio::test]
async fn test_mcp_ping() {
    let response = send_mcp_request("ping", json!({})).await;
    assert!(response.contains("\"result\":{}"));
}
```

#### 13.5 Claude Code E2E 集成测试

**目标**：端到端验证 Claude Code 与 EVIF 的集成

```rust
// crates/evif-integration/tests/claude_code.rs

/// Claude Code MCP 连接测试
#[tokio::test]
async fn test_claude_code_mcp_connection() {
    // 1. 启动 EVIF REST 服务器
    let server = EvifServer::new().await;

    // 2. 启动 MCP 服务器
    let mcp = McpServer::new(&server).await;

    // 3. 验证 Claude Code 可以连接
    let output = Command::new("claude")
        .args(["mcp", "list"])
        .output()
        .await?;

    assert!(String::from_utf8_lossy(&output.stdout).contains("evif"));
}

/// Claude Code 上下文工作流测试
#[tokio::test]
async fn test_claude_code_context_workflow() {
    // 1. Agent 读取 L0 当前上下文
    // 2. Agent 读取 L1 决策记录
    // 3. Agent 发现技能
    // 4. Agent 执行技能
    // 5. Agent 写入 L0 更新状态
    // 6. Agent 写入 L1 记录决策
}
```

#### 13.6 性能基准测试

```rust
// crates/evif-bench/src/performance.rs

/// 吞吐量测试
#[tokio::test]
async fn bench_throughput() {
    let server = TestServer::new().await;
    let start = Instant::now();
    let mut count = 0;

    while start.elapsed().as_secs() < 10 {
        server.write(&format!("/test/file_{}", count), b"data").await;
        count += 1;
    }

    let throughput = count as f64 / 10.0;
    assert!(throughput >= 100.0); // > 100 req/s
}

/// P99 延迟测试
#[tokio::test]
async fn bench_latency_p99() {
    let server = TestServer::new().await;
    let mut latencies = Vec::new();

    for _ in 0..1000 {
        let start = Instant::now();
        server.read("/context/L0/current").await;
        latencies.push(start.elapsed().as_millis() as u64);
    }

    latencies.sort();
    let p99 = latencies[990];
    assert!(p99 <= 50); // P99 < 50ms
}

/// 并发写入稳定性测试
#[tokio::test]
async fn bench_concurrent_writes_stability() {
    let server = TestServer::new().await;
    let mut errors = 0;

    // 100 并发连接，每个执行 100 次写入
    for _ in 0..100 {
        let server = server.clone();
        let result = tokio::spawn(async move {
            for i in 0..100 {
                if server.write(&format!("/test/f_{}", i), b"x").await.is_err() {
                    return 1;
                }
            }
            0
        }).await;

        if result.unwrap_or(1) == 1 {
            errors += 1;
        }
    }

    assert!(errors == 0); // 零错误
}

/// PipeFS 基准测试
mod pipe_bench {
    #[tokio::test]
    async fn bench_bidirectional_latency() {
        // 测试 PipeFS 双向通信延迟
    }

    #[tokio::test]
    async fn bench_concurrent_pipes() {
        // 测试 100 个并发管道
    }
}

---

### Phase 13.7: OpenViking L0CO 基准复制测试

**对标**：OpenViking L0 Context Optimization — 83% token 减少，+49% 性能提升

#### 13.7.1 L0CO 测试设计原理

```
OpenViking L0CO 核心公式：
Token Reduction = 1 - (L0_tokens + L1_tokens) / Original_tokens
目标: ≥ 80%

分层加载策略：
1. L0 (.abstract, ~100 tokens) → 快速相关性过滤
2. L1 (.overview, ~2000 tokens) → 规划与决策
3. L2 (完整内容) → 按需加载，仅加载匹配部分
```

#### 13.7.2 Token 减少率测试

```rust
// crates/evif-bench/src/l0co.rs

/// L0CO Token Reduction Test
/// 目标: ≥ 80% token 减少 vs OpenClaw 完整上下文
#[tokio::test]
async fn test_l0co_token_reduction() {
    let server = TestServer::new().await;

    // 创建测试项目结构 (模拟 50 个文件的 Rust 项目)
    let project_files = create_rust_project(&server, 50).await;
    let original_tokens = count_tokens(&project_files);

    // EVIF L0CO 流程: L0 → L1 → L2 (按需)
    let l0_summary = server.read("/context/L0/project_abstract").await.unwrap();
    let l0_tokens = count_tokens_str(&l0_summary);

    let l1_overview = server.read("/context/L1/project_overview").await.unwrap();
    let l1_tokens = count_tokens_str(&l1_overview);

    let l2_partial = server.read("/context/L2/src/lib.rs").await.unwrap();
    let l2_tokens = count_tokens_str(&l2_partial);

    let evif_tokens = l0_tokens + l1_tokens + l2_tokens;
    let reduction = 1.0 - (evif_tokens as f64 / original_tokens as f64);

    // OpenViking 基准: 83% token 减少
    // EVIF 目标: ≥ 80% token 减少
    assert!(
        reduction >= 0.80,
        "Token reduction {}% < 80% target (original: {}, evif: {})",
        (reduction * 100.0) as i32,
        original_tokens,
        evif_tokens
    );
    println!("[L0CO] Token reduction: {:.1}% (target: 80%)", reduction * 100.0);
}

/// L0 vs OpenClaw 性能对比测试
#[tokio::test]
async fn test_l0_vs_openclaw_performance() {
    let server = TestServer::new().await;

    // OpenClaw 方式: 加载完整上下文
    let openclaw_start = Instant::now();
    let _openclaw_full = server.read("/context/L2/full_project").await.unwrap();
    let openclaw_time = openclaw_start.elapsed().as_millis();

    // EVIF L0CO 方式: L0 摘要过滤
    let evif_start = Instant::now();
    let _l0_summary = server.read("/context/L0/project_abstract").await.unwrap();
    let l1_overview = server.read("/context/L1/project_overview").await.unwrap();
    let evif_time = evif_start.elapsed().as_millis();

    let speedup = openclaw_time as f64 / evif_time as f64;

    // OpenViking 基准: +49% 性能提升
    // EVIF 目标: ≥ +40% 性能提升
    assert!(
        speedup >= 1.40,
        "Speedup {}x < 1.40x target (openclaw: {}ms, evif: {}ms)",
        speedup,
        openclaw_time,
        evif_time
    );
    println!("[L0CO] Speedup: {:.2f}x (target: 1.40x)", speedup);
}
```

#### 13.7.3 L0 摘要生成测试

```rust
/// L0 .abstract 自动生成测试
/// 目标: 每次会话生成项目级摘要 (~100 tokens)
#[tokio::test]
async fn test_l0_abstract_generation() {
    let server = TestServer::new().await;

    // 创建包含 20 个 Rust 文件的项目
    create_rust_project(&server, 20).await;

    // 触发 L0 摘要生成
    let abstract_content = server.read("/context/L0/project_abstract").await.unwrap();

    // 验证摘要格式
    let tokens = count_tokens_str(&abstract_content);
    assert!(
        tokens <= 150,
        "L0 abstract should be ~100 tokens, got {}",
        tokens
    );

    // 验证摘要包含关键信息
    assert!(abstract_content.to_lowercase().contains("rust"));
    assert!(abstract_content.to_lowercase().contains("project"));

    println!("[L0] Abstract generated: {} tokens", tokens);
}

/// L1 .overview 生成测试
/// 目标: 每次会话生成项目概览 (~2000 tokens)
#[tokio::test]
async fn test_l1_overview_generation() {
    let server = TestServer::new().await;

    create_rust_project(&server, 20).await;

    let overview_content = server.read("/context/L1/project_overview").await.unwrap();
    let tokens = count_tokens_str(&overview_content);

    // L1 概览应在 1500-2500 tokens 之间
    assert!(
        tokens >= 1500 && tokens <= 2500,
        "L1 overview should be ~2000 tokens, got {}",
        tokens
    );

    // 验证包含目录结构信息
    assert!(overview_content.contains("src/"));
    assert!(overview_content.contains("Cargo.toml"));

    println!("[L1] Overview generated: {} tokens", tokens);
}
```

#### 13.7.4 分层加载测试

```rust
/// L0 → L1 → L2 分层加载测试
#[tokio::test]
async fn test_progressive_loading() {
    let server = TestServer::new().await;
    create_rust_project(&server, 50).await;

    // Step 1: L0 快速过滤 (目标: < 10ms)
    let l0_start = Instant::now();
    let _abstract = server.read("/context/L0/project_abstract").await.unwrap();
    let l0_time = l0_start.elapsed().as_millis();
    assert!(l0_time < 10, "L0 should load in < 10ms, got {}ms", l0_time);

    // Step 2: L1 决策支持 (目标: < 50ms)
    let l1_start = Instant::now();
    let _overview = server.read("/context/L1/project_overview").await.unwrap();
    let l1_time = l1_start.elapsed().as_millis();
    assert!(l1_time < 50, "L1 should load in < 50ms, got {}ms", l1_time);

    // Step 3: L2 按需加载 (目标: < 100ms)
    let l2_start = Instant::now();
    let _detail = server.read("/context/L2/src/lib.rs").await.unwrap();
    let l2_time = l2_start.elapsed().as_millis();
    assert!(l2_time < 100, "L2 should load in < 100ms, got {}ms", l2_time);

    println!("[L0CO] Progressive loading: L0={}ms, L1={}ms, L2={}ms",
        l0_time, l1_time, l2_time);
}

/// L2 按需加载测试 (仅加载匹配文件)
#[tokio::test]
async fn test_l2_lazy_loading() {
    let server = TestServer::new().await;
    create_rust_project(&server, 50).await;

    // 只加载一个文件
    let start = Instant::now();
    let content = server.read("/context/L2/src/handlers/auth.rs").await.unwrap();
    let load_time = start.elapsed().as_millis();

    // 验证只加载了请求的文件
    let tokens = count_tokens_str(&content);
    assert!(tokens < 5000, "Single file should be < 5000 tokens, got {}", tokens);

    // 验证加载时间 < 50ms
    assert!(load_time < 50, "L2 lazy load should be < 50ms, got {}ms", load_time);
    println!("[L0CO] Lazy load: {} tokens in {}ms", tokens, load_time);
}
```

#### 13.7.5 记忆自迭代测试 (OpenViking Memory Self-Iteration)

```rust
/// 端会话自动总结测试
/// 目标: 会话结束时自动提取关键信息到 L2
#[tokio::test]
async fn test_memory_self_iteration() {
    let server = TestServer::new().await;

    // 创建测试会话
    let session_id = "test-session-001";
    server.write("/context/L1/session_id", session_id.as_bytes()).await.unwrap();

    // 模拟会话期间的操作
    server.write("/context/L1/decisions.md",
        "# Decisions\n\n1. Use Rust for performance\n2. Use EVIF for context management\n".as_bytes()
    ).await.unwrap();

    // 模拟会话结束
    server.trigger_session_end(session_id).await.unwrap();

    // 验证: L1 → L2 归档
    let archive_path = format!("/context/L2/history/session_{}.md", session_id);
    let archived = server.read(&archive_path).await.unwrap();
    assert!(archived.contains("Use Rust for performance"));

    // 验证: 会话摘要生成
    let summary_path = format!("/context/L1/session_summary/{}.md", session_id);
    let summary_exists = server.exists(&summary_path).await.unwrap();
    assert!(summary_exists, "Session summary should be generated");

    println!("[L0CO] Memory self-iteration: session archived to L2");
}
```

#### 13.7.6 L0CO vs OpenViking 基准对比表

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     L0CO Benchmark Results                              │
├──────────────────┬─────────────┬─────────────┬─────────────────────────┤
│ 指标             │ OpenViking  │ EVIF (当前) │ EVIF (Phase 12 目标)    │
├──────────────────┼─────────────┼─────────────┼─────────────────────────┤
│ Token 减少       │ 83%         │ ~30%        │ 80%+                    │
│ L0 加载延迟      │ < 5ms       │ < 10ms ✅   │ < 5ms                   │
│ L1 加载延迟      │ < 20ms      │ < 50ms ✅   │ < 20ms                  │
│ L2 按需加载      │ < 50ms      │ < 100ms ✅  │ < 50ms                  │
│ 性能提升         │ +49%        │ ~20%        │ +40%+                   │
│ 自动摘要         │ ✅ LLM 生成  │ ⚠️ 手动     │ ✅ LLM 自动             │
│ 会话自迭代       │ ✅          │ ❌          │ ✅                      │
│ 多层级递归检索    │ ✅          │ ⚠️ 基础     │ ✅                      │
│ 检索轨迹可视化    │ ✅          │ ❌          │ ✅                      │
└──────────────────┴─────────────┴─────────────┴─────────────────────────┘

验收标准:
- Token 减少 ≥ 80%  (当前差距: -53%)
- L0 加载 < 5ms     (当前: < 10ms, 差距: -5ms)
- L1 加载 < 20ms    (当前: < 50ms, 差距: -30ms)
- 性能提升 ≥ +40%   (当前差距: -20%)
```

#### 13.7.7 L0CO 测试用例清单

| ID | 测试名称 | 目标指标 | 验收标准 |
|----|----------|----------|----------|
| LC-01 | `l0co_token_reduction` | ≥ 80% 减少 | 原始 50000 tokens → EVIF ≤ 10000 |
| LC-02 | `l0_vs_openclaw` | ≥ +40% 提升 | 响应时间减少 40%+ |
| LC-03 | `l0_abstract_generation` | ~100 tokens | 自动摘要生成成功 |
| LC-04 | `l1_overview_generation` | ~2000 tokens | 项目概览生成成功 |
| LC-05 | `progressive_loading` | L0 < 10ms | 分层加载性能达标 |
| LC-06 | `l2_lazy_loading` | < 50ms | 按需加载 < 50ms |
| LC-07 | `memory_self_iteration` | 会话归档 | L1 → L2 自动归档 |

---
```

---

### Phase 14: 生态增强（P2）

#### 14.1 跨文件系统复制

```rust
// crates/evif-core/src/commands/cp.rs

pub async fn cross_fs_copy(
    src: &str,
    dst: &str,
) -> EvifResult<u64> {
    // 解析源和目标的文件系统
    let src_fs = resolve_filesystem(src)?;
    let dst_fs = resolve_filesystem(dst)?;

    // 读取源文件
    let data = src_fs.read(src).await?;

    // 写入目标文件系统
    let bytes = dst_fs.write(dst, &data).await?;

    Ok(bytes)
}
```

#### 14.2 并发控制增强

```rust
// crates/evif-core/src/file_lock.rs

pub struct FileLock {
    path: String,
    handle: RwLock<()>,
}

impl FileLock {
    pub async fn lock(&self) -> EvifGuard {
        EvifGuard(self.handle.write().await)
    }
}

// 使用示例
let lock = FileLock::new("/context/L0/current").await;
let _guard = lock.lock().await;
// 临界区操作
```

#### 14.3 检索轨迹可视化

```rust
// crates/evif-plugins/src/contextfs.rs

pub struct SearchTrace {
    pub steps: Vec<TraceStep>,
    pub total_hits: usize,
}

pub struct TraceStep {
    pub path: String,
    pub operation: String,
    pub hits: usize,
    pub latency_ms: u64,
}

// 在检索时记录轨迹
pub async fn search_with_trace(
    &self,
    query: &str,
) -> EvifResult<(Vec<SearchResult>, SearchTrace)> {
    let mut trace = SearchTrace { steps: Vec::new(), total_hits: 0 };

    // L0 检索
    let start = Instant::now();
    let l0_hits = self.grep("/context/L0", query).await?;
    trace.steps.push(TraceStep {
        path: "/context/L0".to_string(),
        operation: "grep".to_string(),
        hits: l0_hits.len(),
        latency_ms: start.elapsed().as_millis() as u64,
    });

    // L1 检索...

    Ok((results, trace))
}
```

---

## 五、验证测试集设计

### 5.1 测试分层

```
EVIF 测试集
├── Unit Tests (单元测试)
│   ├── ContextFS Tests (16)
│   ├── SkillFS Tests (13)
│   ├── PipeFS Tests (4)
│   └── Core Tests (15)
│
├── Integration Tests (集成测试)
│   ├── REST API Contract Tests (8)
│   ├── MCP Protocol Tests (20)
│   ├── Claude Code E2E Tests (10)
│   └── Multi-Agent Tests (5)
│
├── Benchmark Tests (基准测试)
│   ├── Throughput Tests (10)
│   ├── Latency Tests (10)
│   ├── Concurrency Tests (10)
│   ├── Memory Tests (5)
│   └── **L0CO Tests (7)** ← 对标 OpenViking
│
└── Compliance Tests (合规测试)
    ├── MCP Compliance Tests (50)
    ├── SKILL.md Schema Tests (10)
    └── Security Tests (15)
```

### 5.2 核心测试用例

#### ContextFS 测试

| ID | 测试名称 | 验证内容 | 预期 |
|----|----------|----------|------|
| CF-01 | `l0_write_read` | L0 即时读写 | < 10ms |
| CF-02 | `l1_persistence` | L1 重启后持久化 | 数据不丢失 |
| CF-03 | `l2_semantic_search` | L2 向量搜索 | < 100ms |
| CF-04 | `auto_summary` | 自动摘要生成 | .abstract 生成 |
| CF-05 | `recursive_grep` | 递归检索 | 返回所有匹配 |
| CF-06 | `session_archive` | 会话归档 | L1→L2 成功 |

#### SkillFS 测试

| ID | 测试名称 | 验证内容 | 预期 |
|----|----------|----------|------|
| SF-01 | `skill_discovery` | 技能发现 | 4 个技能列出 |
| SF-02 | `skill_parse` | SKILL.md 解析 | YAML 提取正确 |
| SF-03 | `skill_trigger` | 触发词匹配 | "review" 触发 |
| SF-04 | `skill_execute` | 技能执行 | 输出正确 |

#### PipeFS 测试

| ID | 测试名称 | 验证内容 | 预期 |
|----|----------|----------|------|
| PF-01 | `pipe_create` | 创建管道 | 目录创建成功 |
| PF-02 | `pipe_bidirectional` | 双向通信 | input→output |
| PF-03 | `pipe_broadcast` | 广播模式 | 多订阅者 |
| PF-04 | `pipe_timeout` | 超时清理 | 过期管道删除 |

#### MCP 测试

| ID | 测试名称 | 验证内容 | 预期 |
|----|----------|----------|------|
| MC-01 | `mcp_initialize` | 初始化协议 | 返回 capabilities |
| MC-02 | `mcp_tools_list` | 工具列表 | 20 个工具 |
| MC-03 | `mcp_tool_call` | 工具调用 | 正确响应 |
| MC-04 | `mcp_ping` | Ping/Pong | result: {} |

#### 性能测试

| ID | 测试名称 | 验证内容 | 预期 |
|----|----------|----------|------|
| PE-01 | `concurrent_writes` | 100 并发写 | 95%+ 成功率 |
| PE-02 | `throughput` | 吞吐量 | > 1000 req/s |
| PE-03 | `latency` | P99 延迟 | < 50ms |
| PE-04 | `memory_usage` | 内存占用 | < 500MB |

#### L0CO 基准测试（对标 OpenViking）

| ID | 测试名称 | 验证内容 | 预期 |
|----|----------|----------|------|
| LC-01 | `l0co_token_reduction` | Token 减少率 | ≥ 80% |
| LC-02 | `l0_vs_openclaw` | 性能提升 | ≥ +40% |
| LC-03 | `l0_abstract_generation` | L0 摘要生成 | ~100 tokens |
| LC-04 | `l1_overview_generation` | L1 概览生成 | ~2000 tokens |
| LC-05 | `progressive_loading` | 分层加载 | L0 < 10ms |
| LC-06 | `l2_lazy_loading` | 按需加载 | < 50ms |
| LC-07 | `memory_self_iteration` | 会话自迭代 | L1→L2 归档 |

---

## 六、里程碑

### Phase 12: Context Engine 增强 (6h)

- [ ] 自动 LLM 摘要生成（GPTFS 集成）
- [ ] 目录递归检索（grep -r）
- [ ] 自动会话管理（session lifecycle hook）
- [ ] 持久化记忆跨会话增强

### Phase 13: 验证测试集 (12h)

- [ ] OSWorld 对标测试 (10 tests)
  - 文件系统状态验证
  - 并发文件操作
  - 任务完成验证
- [ ] IDE-Bench 对标测试 (20 tests)
  - 文件读取任务
  - 目录导航任务
  - 文件搜索任务
- [ ] AgentBench 对标测试 (20 tests)
  - 工具调用成功率
  - 多步骤任务
- [ ] MCP 协议合规测试 (50 tests)
- [ ] Claude Code E2E 测试 (10 tests)
- [ ] **OpenViking L0CO 基准复制测试 (7 tests)**
  - Token 减少率测试 (目标: ≥ 80%)
  - 性能提升测试 (目标: ≥ +40%)
  - L0/L1/L2 分层加载测试
  - 记忆自迭代测试

### Phase 14: 生态增强 (8h)

- [ ] 跨文件系统复制（`cp local:/s3:/`）
- [ ] 并发控制增强（文件锁）
- [ ] 检索轨迹可视化（debug trace）
- [ ] 性能基准测试套件（evif-bench crate）

### Phase 15: Claude Code 集成 (4h)

- [ ] Claude Code MCP 完整集成
- [ ] CLAUDE.md 自动生成
- [ ] Auto-memory 增强
- [ ] Subagent 协调示例

---

## 七、参考资料

### 学术论文

- [AIOS: LLM Agent Operating System](https://arxiv.org/abs/2403.16971)
- [Agentic File System Abstraction for Context Engineering](https://arxiv.org/abs/2512.05470)
- [Optimizing Agentic Workflows using Meta-tools](https://arxiv.org/abs/2601.22037)
- [Meta Context Engineering](https://arxiv.org/html/2601.21557v1)
- [Solving Context Window Overflow in AI Agents](https://arxiv.org/html/2511.22729v1)
- [Structured Context Engineering for File-Native Agentic Systems](https://arxiv.org/pdf/2602.05447)

### 基准测试

- [OSWorld: OS-Level Agent Evaluation](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents)
- [IDE-Bench: AI IDE Agent Evaluation](https://www.emergentmind.com/topics/ide-bench)
- [AgentBench: Evaluating LLMs as Agents](https://arxiv.org/abs/2308.03688)
- [ToolBench: LLM Tool Manipulation](https://github.com/sambanova/toolbench)
- [MCP-AgentBench](https://arxiv.org/abs/2509.09734)
- [τ-bench: Agent Policy Evaluation](https://siera.ai/blog/benchmarking-ai-agents)

### 工业实践

- [Effective Context Engineering for AI Agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
- [Claude Code Context Management](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
- [Why Multi-Agent Systems Need Memory Engineering](https://medium.com/mongodb/why-multi-agent-systems-need-memory-engineering-153a81f8d5be)
- [Multi-Agent Research System](https://www.anthropic.com/engineering/multi-agent-research-system)
- [File System vs DB for Agent Memory](https://blogs.oracle.com/developers/comparing-file-systems-and-databases-for-effective-ai-agent-memory-management)
- [Skills vs MCP Architecture](https://thenewstack.io/skills-vs-mcp-agent-architecture/)
- [Context Engineering for Coding Agents](https://martinfowler.com/articles/exploring-gen-ai/context-engineering-coding-agents.html)
- [AI Context Flow: Universal Memory](https://plurality.network/blogs/ai-long-term-memory-with-ai-context-flow/)
- [Mem0: AI Memory Layer](https://mem0.ai/)
- [Beads: Memory Upgrade for Coding Agents](https://github.com/steveyegge/beads)

### Claude Code 生态

- [Claude Code 28 Official Plugins](https://www.reddit.com/r/ClaudeAI/comments/1r4tk3u/there_are_28_official_claude_code_plugins_most/)
- [Claude Code Toolkit](https://github.com/applied-artificial-intelligence/claude-code-toolkit)
- [CLAUDE.md: Permanent Instruction Manual](https://www.mindstudio.ai/blog/what-is-claude-md-file-permanent-instruction-manual)
- [AGENTS.md: Context File for 2026](https://www.augmentcode.com/guides/how-to-build-agents-md)

---

*v14 更新时间：2026-04-02*
*EVIF 版本：1.8.0*
*后续计划：Phase 12-15 实现 + 验证测试集 + OpenViking L0CO 基准复制*
*核心定位：AI Agent 的虚拟上下文文件系统，增强 Claude Code/Codex/Cursor 等 AI Agent*
*对标 OpenViking：83% token 减少 → EVIF 目标 80%+；+49% 性能 → EVIF 目标 +40%*
