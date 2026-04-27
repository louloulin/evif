# EVIF mem22.md — AI Agent 集成价值白皮书

> 创建时间：2026-04-27
> 分析目标：全面分析 EVIF 为 Claude Code、OpenClaw、Codex 等 AI Agent 平台带来的核心价值
> 状态：**进行中**

---

## 1. 核心价值主张

### 1.1 EVIF 是什么

**EVIF (Everything Is a File)** 是一个 Rust 编写的虚拟文件系统平台，提供：

- **30+ 插件**：memfs、contextfs、skillfs、pipefs、localfs、S3 等
- **Radix 树路由**：O(k) 路径查找性能
- **统一的文件接口**：ls/cat/write/grep 等 POSIX 风格操作
- **多后端支持**：Memory、SQLite、PostgreSQL、RocksDB

### 1.2 为 AI Agent 解决的根本问题

| 传统 Agent 框架 | EVIF 解决方案 |
|---|---|
| 硬编码的 API 调用 | 文件系统抽象，Agent 天生理解 |
| Session 内存丢失 | ContextFS 持久化上下文 |
| 不可发现的工具 | SkillFS SKILL.md 标准 |
| 单 Agent 孤岛 | PipeFS 多 Agent 协调 |
| 专有协议 | REST/Python/CLI/FUSE 多协议接入 |

### 1.3 核心差异化

**Agent 不需要学习新概念**：
- 已有工具：`ls`, `cat`, `write`, `grep`, `mkdir`
- EVIF 处理其余一切：路由、插件、持久化、多 Agent 协调

---

## 2. 核心抽象详解

### 2.1 ContextFS — 三层上下文管理

```
/context/
├── L0/                    # 即时上下文（当前工作状态）
│   ├── current            # 当前任务描述
│   ├── recent_ops         # 最近操作记录
│   └── budget_status     # Token 预算状态
├── L1/                    # 会话上下文（决策记录）
│   ├── decisions.md       # 会话决策
│   └── scratch/          # 临时工作区
└── L2/                    # 项目知识（长期记忆）
    ├── architecture.md    # 架构文档
    ├── patterns.md        # 模式库
    └── history/           # 历史会话归档
```

**Agent 使用示例**：
```bash
cat /context/L0/current      # "我现在在做什么？"
cat /context/L1/decisions.md # "本会话做了哪些决策？"
ls /context/L2               # "项目结构是什么？"
cat /context/L2/architecture.md
```

### 2.2 SkillFS — 可发现的工作流复用

**SKILL.md 标准格式**：
```yaml
---
name: code-review
description: "Review code for bugs, security issues"
triggers:
  - "review"
  - "code review"
  - "check my code"
---
# Code Review Skill

Read the target code, identify the most important risks, and produce a concise review report.
```

**Agent 使用示例**：
```bash
ls /skills                        # 发现可用技能
cat /skills/code-review/SKILL.md  # 查看技能详情
```

### 2.3 PipeFS — 多 Agent 协调

```
/pipes/<name>/
├── input      # 任务输入
├── output     # 任务结果
├── status     # pending → running → complete
├── assignee   # 认领 Agent ID
└── timeout    # TTL 秒数
```

**Agent 使用示例**：
```bash
mkdir /pipes/review-task
echo "Review auth module" > /pipes/review-task/input
cat /pipes/review-task/status  # running
echo "Found 2 issues" > /pipes/review-task/output
```

### 2.4 Memory — 多模态记忆存储

**记忆类型**：
- Profile（用户偏好）
- Event（重要事件）
- Knowledge（习得知识）
- Behavior（行为模式）

**存储后端**：
- In-Memory（测试）
- SQLite（本地）
- PostgreSQL（生产）
- RocksDB（高性能）

---

## 3. OpenClaw 集成分析

### 3.1 OpenClaw 是什么

OpenClaw 是一个 AI Agent 编排器/调度器，核心能力：
- Gateway/Message 集成
- Agent 编排和生命周期管理
- 插件系统
- ACP (Agent Communication Protocol) 会话

### 3.2 EVIF 如何服务 OpenClaw

#### 3.2.1 Context Backend

```
OpenClaw Agent → EVIF ContextFS
                      ↓
        /context/L0/current (当前任务)
        /context/L1/decisions.md (决策记录)
        /context/L2/ (项目知识)
```

#### 3.2.2 Task Coordination via PipeFS

```
OpenClaw Orchestrator
        ↓
   Pipe: /pipes/agent-{id}/input
        ↓
   Worker Agent 1, 2, 3...
        ↓
   Pipe: /pipes/agent-{id}/output
        ↓
OpenClaw Aggregator
```

#### 3.2.3 Multi-Agent Sessions

```
/pipes/
├── session-{session_id}/
│   ├── input      # 主 session 消息
│   ├── output     # Session 结果
│   └── status     # Session 状态
└── broadcast/    # 广播通道
    └── subscribers/{name}/output
```

### 3.3 OpenClaw 集成架构

```
┌─────────────────────────────────────────────────────────────┐
│                      OpenClaw                               │
├─────────────────────────────────────────────────────────────┤
│  Gateway/Message    │  Agent Orchestrator  │  Plugins     │
└──────────────┬─────────────────┬──────────────┬────────────┘
               │                 │              │
               ▼                 ▼              ▼
        ┌─────────────────────────────────────────────┐
        │              EVIF Platform                   │
        ├─────────────────────────────────────────────┤
        │  ContextFS  │  PipeFS  │  SkillFS  │  Memory  │
        │  (L0/L1/L2) │ (Tasks)  │  (Skills)  │ (Memories) │
        └──────────────┴──────────┴───────────┴──────────┘
```

### 3.4 OpenClaw-EVIF 集成代码

```python
#!/usr/bin/env python3
"""
OpenClaw-EVIF 集成示例
展示多 Agent 任务分发和结果聚合
"""

import asyncio
import json
import uuid
from evif import EvifClient

EVIF_URL = "http://localhost:8081"

class OpenClawEVIFBridge:
    def __init__(self):
        self.client = EvifClient(EVIF_URL)
    
    async def setup_context_structure(self):
        """初始化 OpenClaw 兼容的上下文结构"""
        dirs = ["/context/L0", "/context/L1", "/context/L2", "/pipes"]
        for d in dirs:
            try:
                await self.client.mkdir(d)
            except Exception:
                pass
    
    async def spawn_analysis_agent(self, agent_id: str, task: str) -> str:
        """派发 Agent 分析任务"""
        pipe_name = f"agent-{agent_id}"
        
        # 创建任务 Pipe
        await self.client.mkdir(f"/pipes/{pipe_name}")
        
        # 写入任务输入
        await self.client.write(f"/pipes/{pipe_name}/input", json.dumps({
            "agent_id": agent_id,
            "task": task,
            "created_at": "now"
        }))
        
        # 更新 L0 上下文
        await self.client.write(
            "/context/L0/current",
            f"Agent {agent_id} analyzing: {task}"
        )
        
        return pipe_name
    
    async def agent_read_task(self, agent_id: str) -> dict:
        """Agent 读取分配的任务"""
        pipe_name = f"agent-{agent_id}"
        input_data = await self.client.cat(f"/pipes/{pipe_name}/input")
        return json.loads(input_data)
    
    async def agent_write_result(self, agent_id: str, result: str):
        """Agent 写入任务结果"""
        pipe_name = f"agent-{agent_id}"
        await self.client.write(f"/pipes/{pipe_name}/output", result)
        await self.client.write(f"/pipes/{pipe_name}/status", "complete")
    
    async def get_aggregated_results(self) -> list:
        """收集所有 Agent 结果"""
        pipes = await self.client.ls("/pipes")
        results = []
        for pipe in pipes:
            if pipe.name.startswith("agent-"):
                try:
                    output = await self.client.cat(f"/pipes/{pipe.name}/output")
                    status = await self.client.cat(f"/pipes/{pipe.name}/status")
                    results.append({
                        "agent": pipe.name,
                        "status": status.decode() if isinstance(status, bytes) else status,
                        "output": output.decode() if isinstance(output, bytes) else output
                    })
                except Exception:
                    pass
        return results

async def main():
    bridge = OpenClawEVIFBridge()
    await bridge.client.connect()
    
    # 设置上下文结构
    await bridge.setup_context_structure()
    
    # 派发 3 个分析 Agent
    tasks = [
        ("auth-analyzer", "Review authentication module for security issues"),
        ("perf-analyzer", "Profile API endpoints for performance bottlenecks"),
        ("coverage-analyzer", "Identify untested code paths"),
    ]
    
    for agent_id, task in tasks:
        pipe = await bridge.spawn_analysis_agent(agent_id, task)
        print(f"Spawned {agent_id} -> pipe: {pipe}")
    
    # 模拟 Agent 工作并收集结果
    await asyncio.sleep(1)
    
    results = await bridge.get_aggregated_results()
    print(f"\nAggregated Results ({len(results)} agents):")
    for r in results:
        print(f"  {r['agent']}: {r['status']}")
    
    await bridge.client.close()

if __name__ == "__main__":
    asyncio.run(main())
```

---

## 4. Claude Code 集成分析

### 4.1 Claude Code 是什么

Anthropic 的 CLI Agent 工具，核心能力：
- 终端交互
- 代码编辑
- Git 操作
- 多轮对话
- Session 管理

### 4.2 EVIF 如何服务 Claude Code

#### 4.2.1 Skill Backend

Claude Code 可以通过 EVIF 获取标准化的 Skills：
```
/skills/
├── code-review/
│   └── SKILL.md
├── test-gen/
│   └── SKILL.md
├── refactor/
│   └── SKILL.md
└── doc-gen/
    └── SKILL.md
```

#### 4.2.2 Session Persistence

Claude Code Session 可以通过 EVIF 持久化：
```
Session 1: "Reviewed auth module"
Session 2: "Continued from previous session"
```

#### 4.2.3 Cross-Session Memory

Claude Code 可以通过 EVIF Memory 跨会话检索：
```python
# Session 1
memory_store("Reviewed auth module, found SQL injection")

# Session 2
memory_search("auth module")  # 检索到之前的记忆
```

### 4.3 Claude Code 集成架构

```
┌─────────────────────────────────────────────────────────────┐
│                    Claude Code                              │
├─────────────────────────────────────────────────────────────┤
│  Terminal  │  Editor  │  Git  │  Session Manager          │
└──────────────┬─────────────┬───────┬───────────────────────┘
               │             │       │
               ▼             ▼       ▼
        ┌─────────────────────────────────────────────┐
        │              EVIF Platform                   │
        ├─────────────────────────────────────────────┤
        │  SkillFS    │  ContextFS  │  Memory        │
        │  (/skills)   │  (/context) │  (memories)    │
        └──────────────┴────────────┴────────────────┘
```

### 4.4 Claude Code-EVIF 集成代码

```python
#!/usr/bin/env python3
"""
Claude Code - EVIF 集成示例
展示 Skill Backend 和 Session 持久化
"""

import asyncio
import json
from evif import EvifClient

EVIF_URL = "http://localhost:8081"

class ClaudeCodeEVIFIntegration:
    def __init__(self):
        self.client = EvifClient(EVIF_URL)
    
    async def setup_claude_project(self, project_root: str):
        """初始化 Claude Code 项目 EVIF 上下文"""
        for ctx_dir in ["/context/L0", "/context/L1", "/skills"]:
            try:
                await self.client.mkdir(ctx_dir)
            except Exception:
                pass
        
        # 写入 CLAUDE.md 到上下文
        claude_md = """# Project Context

## Overview
This project uses EVIF for context management.

## Context Layers
- `/context/L0` - Current task state
- `/context/L1` - Decision rationale
- `/context/L2` - Historical archives

## Skills
Skills are stored in `/skills` directory.
"""
        await self.client.write("/context/L0/CLAUDE.md", claude_md)
        await self.client.write("/context/L0/current", "Initializing Claude Code integration")
        return True
    
    async def save_session(self, session_id: str, content: str):
        """保存 Claude Code Session 上下文"""
        await self.client.write(
            f"/context/L0/current",
            f"Session {session_id}: {content}"
        )
        
        # 同时存入 Memory 供跨会话检索
        await self.client.memory_store(
            content=content,
            modality="conversation",
            metadata={
                "session_id": session_id,
                "type": "claude_session"
            }
        )
        return True
    
    async def discover_and_execute_skill(self, query: str) -> str:
        """查找并执行匹配查询的 Skill"""
        # 发现 Skill
        skill_name = await self.client.skill_match(query)
        
        if not skill_name:
            return f"No skill found for: {query}"
        
        # 获取 Skill 详情
        skill_content = await self.client.skill_read(skill_name)
        
        # 执行 Skill
        result = await self.client.skill_execute(
            skill_name,
            f"Execute {query} for current project"
        )
        
        return f"Executed {skill_name}: {result}"
    
    async def register_skill(self, name: str, description: str, triggers: list, body: str):
        """注册新 Skill 供 Claude Code 使用"""
        skill_md = f"""---
name: {name}
description: "{description}"
triggers:
"""
        for trigger in triggers:
            skill_md += f"  - \"{trigger}\"\n"
        skill_md += f"""---
{body}
"""
        await self.client.skill_register(name, skill_md)
        return True
    
    async def get_project_context(self) -> dict:
        """获取完整项目上下文供 Claude Code 使用"""
        context = {}
        
        try:
            l0_current = await self.client.cat("/context/L0/current")
            context["current_task"] = l0_current.decode() if isinstance(l0_current, bytes) else l0_current
        except Exception:
            context["current_task"] = None
        
        try:
            decisions = await self.client.cat("/context/L1/decisions.md")
            context["decisions"] = decisions.decode() if isinstance(decisions, bytes) else decisions
        except Exception:
            context["decisions"] = None
        
        context["skills"] = await self.client.skill_discover()
        
        return context
    
    async def cross_session_memory(self, query: str) -> list:
        """跨 Claude Code Session 搜索记忆"""
        results = await self.client.memory_search(query, limit=10)
        return results

async def main():
    integration = ClaudeCodeEVIFIntegration()
    await integration.client.connect()
    
    # 设置项目
    await integration.setup_claude_project("/projects/myapp")
    print("Project initialized with EVIF context")
    
    # 保存 Session
    await integration.save_session(
        "session-001",
        "Reviewed auth module, identified SQL injection vulnerability"
    )
    print("Session saved")
    
    # 注册自定义 Skill
    await integration.register_skill(
        name="security-check",
        description="Performs security analysis on code",
        triggers=["security check", "vulnerability", "security audit"],
        body="""# Security Check Skill

Runs security analysis on provided code.
"""
    )
    print("Security skill registered")
    
    # 获取项目上下文
    ctx = await integration.get_project_context()
    print(f"\nProject Context:")
    print(f"  Current Task: {ctx.get('current_task', 'None')[:50]}...")
    print(f"  Skills: {ctx.get('skills', [])}")
    
    # 跨 Session 记忆
    memories = await integration.cross_session_memory("auth")
    print(f"\nCross-session memories: {len(memories)} found")
    
    await integration.client.close()

if __name__ == "__main__":
    asyncio.run(main())
```

---

## 5. 生产 MVP 差距分析

### 5.1 认证 (JWT/OAuth)

**当前状态**：
- API Key 认证 (`Authorization: Bearer <key>`)
- SHA256 哈希支持
- Capability-based 访问控制
- 审计日志

**缺失**：
- 无原生 JWT Token 支持
- 无 OAuth2/OIDC 集成
- 无 Token 过期/刷新
- 无细粒度 RBAC

**最小可行修复**：
```rust
// JWT 验证器
pub struct JwtValidator {
    secret: String,
    issuer: String,
}

// 配置
EVIF_AUTH_JWT_SECRET=your-secret-key
EVIF_AUTH_MODE=jwt  // or "apikey" for backwards compat
```

### 5.2 多租户隔离

**当前状态**：
- `TenantState` 存在
- 存储配额管理
- `x-tenant-id` header

**缺失**：
- 无路径级租户隔离
- 无租户特定插件配置
- 无跨租户访问控制

**最小可行修复**：
```rust
// 租户隔离挂载表
pub struct TenantScopedMountTable {
    base_table: Arc<RadixMountTable>,
    tenant_overrides: RwLock<HashMap<String, Arc<RadixMountTable>>>,
}

EVIF_REST_TENANT_ISOLATION=true
EVIF_REST_TENANT_DEFAULT_MOUNTS=/mem,/context,/skills
```

### 5.3 分布式一致性

**当前状态**：
- 文件锁 (`FileLockManager`)
- Handle 管理 (TTL cleanup)
- 跨文件系统复制

**缺失**：
- 无分布式共识 (Raft/Paxos)
- 无最终一致性保证
- 无领导者选举

**最小可行修复**：
```rust
// Lease 管理
pub struct LeaseManager {
    etcd_client: etcd_client::Client,
    ttl_secs: u64,
}

EVIF_DISTRIBUTED_MODE=etcd
EVIF_ETCD_ENDPOINTS=http://localhost:2379
```

### 5.4 安全性

**当前状态**：
- Capability-based 访问控制
- API Key 哈希
- 审计日志

**缺失**：
- 无静态加密
- 无 TLS 强制
- 无路径遍历防护
- 无速率限制

**最小可行修复**：
```rust
// 路径净化
pub fn sanitize_path(path: &str) -> Result<String, EvifError> {
    let normalized = path.replace("\0", "");
    if normalized.contains("..") {
        return Err(EvifError::InvalidPath(path.into()));
    }
    Ok(normalized)
}

EVIF_TLS_CERT=/path/to/cert.pem
EVIF_RATE_LIMIT_PER_TENANT=1000
```

### 5.5 可观测性

**当前状态**：
- Prometheus metrics
- 审计日志
- Health/readiness probes

**缺失**：
- 无分布式追踪 (OpenTelemetry)
- 无结构化日志 + correlation IDs
- 无告警规则
- 无 Dashboard

**最小可行修复**：
```rust
// OpenTelemetry 追踪
pub fn init_tracing(service_name: &str) -> Result<()> {
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        .install_batch(opentelemetry::runtime::Tokio)?;
    Ok(())
}

EVIF_OTEL_ENDPOINT=http://localhost:4317
EVIF_LOG_FORMAT=json
```

### 5.6 差距总结

| 类别 | 必须有 | 应该有 | 可以有 |
|---|---|---|---|
| **认证** | JWT Token | OAuth2 | mTLS |
| **多租户** | 路径隔离 | 配额强制 | 跨租户搜索 |
| **一致性** | Lease 管理 | 分布式锁 | Raft 共识 |
| **安全** | TLS + 速率限制 | 静态加密 | 零信任 |
| **可观测性** | OpenTelemetry | 结构化日志 | Dashboard |
| **稳定性** | FUSE 稳定 | 内存泄漏修复 | HA |

---

## 6. 验证示例

### 6.1 Agent 读取上下文

```bash
# EVIF CLI
evif cat /context/L0/current
evif ls /context

# REST API
curl "http://localhost:8081/api/v1/files?path=/context/L0/current"
curl "http://localhost:8081/api/v1/directories?path=/context"

# Python SDK
from evif import Client
c = Client("http://localhost:8081")
print(c.cat("/context/L0/current"))
```

### 6.2 Agent 发现和使用 Skills

```bash
# 发现技能
curl "http://localhost:8081/api/v1/directories?path=/skills"

# 查看技能详情
curl "http://localhost:8081/api/v1/files?path=/skills/code-review/SKILL.md"

# Python SDK
c.skill_discover()
c.skill_match("review code")
c.skill_read("code-review")
```

### 6.3 多 Agent 任务协调

```bash
# 创建任务 Pipe
curl -X POST "http://localhost:8081/api/v1/directories" -d '{"path": "/pipes/task-001"}'

# Producer 发送任务
curl -X PUT "http://localhost:8081/api/v1/files?path=/pipes/task-001/input" \
  -d '{"data": "Review auth module"}'

# 检查状态
curl "http://localhost:8081/api/v1/files?path=/pipes/task-001/status"

# Consumer 写入结果
curl -X PUT "http://localhost:8081/api/v1/files?path=/pipes/task-001/output" \
  -d '{"data": "Found SQL injection"}'
```

### 6.4 记忆存储和检索

```python
from evif import Client
c = Client("http://localhost:8081")

# 存储记忆
c.memory_store(
    content="Reviewed auth module, found SQL injection",
    modality="code",
    metadata={"severity": "high"}
)

# 搜索记忆
results = c.memory_search("SQL injection")
for r in results:
    print(r["content"])
```

---

## 7. 集成架构总览

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Claude Code                                  │
│  /skills discovery  │  Session persistence  │  Cross-session memory │
└─────────────────────┬───────────────────────────────────────────────┘
                      │ MCP Tools / REST API
                      ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         OpenClaw                                     │
│  Gateway/Message  │  Agent orchestration  │  Plugin system         │
└─────────────────────┬───────────────────────────────────────────────┘
                      │ REST API / MCP
                      ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         EVIF Platform                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │  ContextFS  │  │   SkillFS   │  │   PipeFS    │              │
│  │  L0/L1/L2   │  │  SKILL.md   │  │  Task pipes │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │  MemPlugin  │  │  REST API   │  │   MCP Server │              │
│  │  Memory     │  │  HTTP/JSON   │  │   stdio     │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 8. 关键集成点总结

| 能力 | OpenClaw 用途 | Claude Code 用途 | EVIF 端点 |
|---|---|---|---|
| Context Backend | Markdown memory | Session 持久化 | `/context/L*/` |
| Skill Discovery | Tool plugins | Skill backend | `/skills/` |
| Task Coordination | ACP agent sessions | N/A | `/pipes/` |
| Long-term Memory | `./memory/*.md` | 跨 Session | `POST /api/v1/memories` |
| Search | MCP grep | Memory search | `POST /api/v1/memories/search` |
| Health | Gateway status | Server check | `GET /api/v1/health` |

---

## 9. 下一步行动

### 立即可行（本周）

1. **清理供应链漏洞**
   ```bash
   cargo audit
   # 逐个分析 22 个 CVE
   ```

2. **完善端到端测试**
   ```bash
   ./demos/agent_workflow/start_demo.sh
   # 覆盖更多 Agent 场景
   ```

3. **补充 API 文档**
   ```bash
   # 生成 OpenAPI spec
   cargo doc --no-deps
   ```

### 短期目标（1-2 周）

1. **JWT 认证支持**
   ```bash
   EVIF_AUTH_MODE=jwt
   EVIF_AUTH_JWT_SECRET=...
   ```

2. **FUSE 稳定性**
   ```bash
   # 修复 macOS FUSE 兼容问题
   ```

3. **监控埋点**
   ```bash
   EVIF_OTEL_ENDPOINT=http://localhost:4317
   ```

### 中期目标（1 个月）

1. **技能执行沙箱**
   ```rust
   // WASM 或 Docker 隔离执行
   pub trait SkillExecutor {
       async fn execute(&self, skill: &Skill) -> Result<Output>;
   }
   ```

2. **向量检索优化**
   ```rust
   // Phase roadmap: 向量索引
   pub trait VectorIndex {
       async fn search(&self, query: &[f32], k: usize) -> Vec<SearchResult>;
   }
   ```

3. **多租户隔离**
   ```bash
   EVIF_REST_TENANT_ISOLATION=true
   ```

---

## 10. 结论

### 10.1 EVIF 的核心价值

| 传统方案 | EVIF 带来的改变 |
|---|---|
| 每个 Agent 框架独立实现上下文管理 | **统一**的 ContextFS |
| 工具硬编码在框架里 | **可发现**的 SkillFS |
| Agent 间通过消息队列通信 | **文件系统风格**的 PipeFS |
| Session 数据存在内存里 | **持久化**的 Memory |

### 10.2 一句话总结

**EVIF = 文件系统的 Agent 原语化 + Context/Skill/Pipe 的标准化 + 多后端的插件化**

Agent 不再需要学习专有框架，只需要用 `ls/cat/write/grep`，EVIF 处理其余一切。

### 10.3 验证命令

```bash
# 启动 EVIF
EVIF_REST_AUTH_MODE=disabled ./target/debug/evif-rest --port 8081 &

# 验证 Context
curl "http://localhost:8081/api/v1/directories?path=/context"

# 验证 Skills
curl "http://localhost:8081/api/v1/directories?path=/skills"

# 验证 Pipes
curl -X POST "http://localhost:8081/api/v1/directories" -d '{"path": "/pipes/test"}'
curl "http://localhost:8081/api/v1/directories?path=/pipes"

# Python SDK 验证
PYTHONPATH=crates/evif-python python3 -c "
from evif import Client
c = Client('http://localhost:8081')
print('Health:', c.health())
print('Skills:', c.skill_discover())
print('Context:', c.ls('/context'))
"
```

---

## 参考文件

| 组件 | 路径 |
|---|---|
| ContextFS | `crates/evif-plugins/src/contextfs.rs` |
| SkillFS | `crates/evif-plugins/src/skillfs.rs` |
| PipeFS | `crates/evif-plugins/src/pipefs.rs` |
| REST API | `crates/evif-rest/src/routes.rs` |
| Memory API | `crates/evif-rest/src/memory_handlers.rs` |
| Auth | `crates/evif-auth/src/auth.rs` |
| MCP Server | `crates/evif-mcp/src/lib.rs` |
| Python SDK | `crates/evif-python/evif/client.py` |
| Prometheus | `crates/evif-metrics/src/prometheus.rs` |

---

*最后更新：2026-04-27*
