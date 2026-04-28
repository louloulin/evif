# EVIF mem19.md — 核心功能实现计划（2026-04-26）

> 创建时间：2026-04-26
> 分析基础：mem17（生产差距）、mem18（AGFS 对标差距）、本轮全量测试诊断
> 计划目标：聚焦"让 EVIF 被外部用户 5 分钟内用起来"的最短路径

---

## 1. 背景与判断

### 1.1 当前状态

从本轮全面分析得出的关键事实：

| 维度 | 状态 | 说明 |
|---|---|---|
| clippy 全量 | ✅ 通过 | `cargo clippy --workspace --all-targets -- -D warnings` 退出 0 |
| 核心包测试 | ✅ 通过 | evif-core(76)、evif-auth(15)、evif-fuse(13)、evif-client(2) 均通过 |
| 集成测试 | ⚠️ 部分崩坏 | ~200 个测试因 `system-configuration` macOS panic 失败，非代码 bug |
| REST/MCP/FUSE 接入面 | ✅ 就绪 | create_routes() 可正常启动 |
| Python SDK | ❌ 缺失 | 外部用户无法从 Python 接入 EVIF |
| Agent demo 资产 | ❌ 缺失 | 无法快速演示 EVIF 的 agent 协同价值 |
| 磁盘空间 | ✅ 已清理 | target/debug 已清理，恢复 68GB 可用 |

### 1.2 为什么选这条路

mem18 给出了两条路线：

- **路线 A**：对标 AGFS，补 Python SDK、agent demos、shell 产品化
- **路线 B**：对标生产平台，补 N4 选主、JWT/OAuth、供应链安全

本计划选 **路线 A**，理由是：

1. **EVIF 的核心抽象已经非常完整**，再补 FS 名称边际收益低
2. **Python SDK 是当前最直接的外部触达能力**，AGFS 有 Python/Go SDK，EVIF 没有，这直接限制了 Python agent 生态接入
3. **agent demos 是展示 EVIF 价值的最低成本方式**，一个能跑起来的 demo 比文档更有说服力
4. **N4/JWT/OAuth 等是"做对"的问题，SDK/demo 是"有人用"的问题**，先解决有人用

---

## 2. Phase A：Python SDK（最高优先级）

### 2.1 目标

在 `sdk/python/` 目录下实现最小可用 Python SDK，外部 Python 用户能通过 `pip install evif-sdk` 或直接引用源码的方式接入 EVIF REST API。

### 2.2 覆盖的 API

| 方法 | 对应 REST 端点 | 说明 |
|---|---|---|
| `client.health()` | GET /api/v1/health | 健康检查 |
| `client.ls(path)` | GET /api/v1/directories | 列出目录 |
| `client.cat(path)` | GET /api/v1/files | 读取文件 |
| `client.write(path, content)` | PUT /api/v1/files | 写入文件 |
| `client.mkdir(path)` | POST /api/v1/directories | 创建目录 |
| `client.stat(path)` | GET /api/v1/stat | 文件元数据 |
| `client.mount(plugin, path)` | POST /api/v1/mount | 挂载插件 |
| `client.plugins()` | GET /api/v1/plugins | 列出插件 |
| `client.memory_store(content, modality)` | POST /api/v1/memories | 存储记忆 |
| `client.memory_search(query)` | GET /api/v1/memories/search | 搜索记忆 |

### 2.3 实现要求

- 纯 Python，无 Rust 绑定，通过 `httpx` 或 `requests` 调用 REST API
- API key 通过构造函数传入：`Client("http://localhost:8081", api_key="write-key")`
- 错误处理：HTTP 4xx/5xx 转成 `EvifError` 异常
- 类型注解完整
- 包含基本单元测试（mock HTTP 响应）

### 2.4 验收标准

```python
from evif import Client

client = Client("http://localhost:8081", api_key="write-key")
print(client.health())
# {'status': 'healthy', 'version': '...', 'uptime': ...}

client.write("/test/demo.txt", "hello from python sdk")
print(client.cat("/test/demo.txt"))
# "hello from python sdk"
```

---

## 3. Phase B：Agent Workflow Demo（次高优先级）

### 3.1 目标

在 `demos/agent_workflow/` 目录下提供一组端到端可运行的 demo，展示 EVIF 的 agent 协同能力。

### 3.2 Demo 清单

#### Demo 1：`task_queue_worker.py`（queuefs + memory）

展示模式：
- 主 agent 向 queuefs 提交任务（JSON 格式：任务类型、参数）
- Worker agent 从 queuefs 消费任务、执行、写入结果到 memfs
- 展示 queue-based 的多 agent 并行消费模型

关键文件：
- `demos/agent_workflow/task_queue_worker.py`
- `demos/agent_workflow/start_workers.sh`

#### Demo 2：`pipe_triggered_agent.py`（pipefs + skillfs + contextfs）

展示模式：
- pipefs 接收外部信号（写入 trigger 文件）
- agent 检测到 trigger，执行 skillfs 中的技能
- contextfs 读取 L0/L1 上下文，指导执行
- 结果写入 memfs

关键文件：
- `demos/agent_workflow/pipe_triggered_agent.py`
- `demos/agent_workflow/skills/`（至少一个示例 skill）

#### Demo 3：`start_all.sh`（一键启动）

```bash
#!/bin/bash
# 1. 启动 evif-rest
cargo run -p evif-rest &
sleep 2

# 2. 运行 Python SDK smoke test
python -c "from evif import Client; c = Client('http://localhost:8081'); print(c.health())"

# 3. 运行 task_queue_worker demo
python demos/agent_workflow/task_queue_worker.py

# 4. 运行 pipe_triggered_agent demo
python demos/agent_workflow/pipe_triggered_agent.py
```

### 3.3 验收标准

```bash
./demos/agent_workflow/start_all.sh
# 全程无报错，输出展示 queue-based + pipe-based 两种 agent 协同模式
```

---

## 4. Phase C：REPL History 路径修复（快速修复）

### 4.1 问题

`crates/evif-cli/src/repl.rs:23` 在测试环境或无 home 目录时 panic：

```rust
let history = Box::new(
    FileBackedHistory::with_file(1000, history_path)
        .expect("Failed to create history file"),  // ← panic here
);
```

导致 4 个 `evif-cli` bin 测试失败。

### 4.2 修复方案

在 history 文件创建失败时降级为无 history 模式，而不是 panic：

```rust
let history: Box<dyn Editor> = match FileBackedHistory::with_file(1000, history_path) {
    Ok(h) => Box::new(h),
    Err(e) => {
        eprintln!("Warning: history file unavailable ({e}), running without history");
        Box::new(Reedline::create())
    }
};
```

### 4.3 验收标准

```bash
cargo test -p evif-cli --bin evif repl::tests::test_repl_creation -- --nocapture
# 通过，无 panic
```

---

## 5. Phase D：测试环境诊断（不承诺修复）

### 5.1 问题

`system-configuration` v0.5.1 在 macOS 上触发 `"Attempted to create a NULL object"` panic，导致 ~200 个测试失败。

根因链：
```
create_routes() / reqwest::Client::new()
  → axum → hyper-util → system-configuration v0.5.1
    → macOS System Configuration framework
      → panic: "Attempted to create a NULL object"
```

### 5.2 影响范围

| 包 | 失败数 | 是否影响核心功能 |
|---|---|---|
| api-tests (26) | 26 | ❌ 仅集成测试 |
| cli-tests (37) | 37 | ❌ 仅 CLI 集成测试 |
| evif-mem (25) | 25 | ❌ 仅 LLM client 测试 |
| evif-bench (23) | 23 | ❌ 仅 benchmark 测试 |
| evif-mcp (22) | 22 | ❌ 仅 MCP 集成测试 |
| evif-rest lib (5) | 5 | ❌ 仅 middleware/postgres 单元测试 |
| evif-plugins (4) | 4 | ❌ 仅 proxyfs/httpfs 集成测试 |
| **通过的核心包** | — | ✅ **evif-core(76), evif-auth(15), evif-fuse(13)** |

### 5.3 行动计划

1. **定位根因**：检查是否是 `hyper-util` 或特定 macOS 版本问题
2. **找 workaround**：
   - 方案 A：在 CI 中加环境变量绕过（如有）
   - 方案 B：降级 `hyper-util` 版本
   - 方案 C：在测试中 mock network layer 避免触发 system-configuration
3. **文档记录**：即使修不好，也要把根因和影响范围记录清楚，避免其他人重复踩坑

### 5.4 不承诺的原因

这是第三方 crate（`system-configuration`/`hyper-util`）的 macOS 兼容性问题，不是 EVIF 代码 bug。核心包均正常，不阻断开发。

---

## 6. 不做的事（保持专注）

以下项不在本计划范围内，留给后续：

| 项 | 理由 |
|---|---|
| JWT/OAuth | 大工程，需要设计 token 生命周期，当前不影响 EVIF 被用起来 |
| N4 选主/协调 | 需要架构设计，不是一周能完成 |
| 供应链漏洞清理 | 22 个漏洞需要逐个分析，单独成任务 |
| Docker healthcheck | 用户要求忽略 Docker |
| 安全响应头 | 不影响核心功能，是运维增强 |
| 分布式追踪 | 不影响核心功能，是可观测性增强 |
| GraphQL 扩展 | 当前 REST 已够用，GraphQL 是后续优化 |

---

## 7. 时间线

```
Week 1
  ├── Phase A: Python SDK
  │     ├── sdk/python/evif/__init__.py
  │     ├── health, ls, cat, write, mkdir, stat
  │     └── Python smoke test
  │
  └── Phase C: REPL History Fix
        └── 30 min，快速收尾

Week 2
  ├── Phase B: Agent Workflow Demo
  │     ├── task_queue_worker.py
  │     ├── pipe_triggered_agent.py
  │     ├── skills/ 示例
  │     └── start_all.sh
  │
  └── Phase D: 测试环境诊断
        ├── 定位根因
        └── 输出 workaround 或文档说明
```

---

## 8. 验收命令

| Phase | 验收命令 | 预期结果 |
|---|---|---|
| Phase A | `python -c "from evif import Client; c = Client('http://localhost:8081', api_key='write-key'); print(c.health())"` | `{'status': 'healthy', ...}` |
| Phase B | `cd demos/agent_workflow && ./start_all.sh` | 全程无报错，输出协同结果 |
| Phase C | `cargo test -p evif-cli --bin evif repl::tests -- --nocapture` | 3 passed, 0 failed |
| Phase D | `cargo test --workspace --all-targets --quiet 2>&1 | grep FAILED` | 受影响测试数量有记录 |

---

## 9. 与 mem17/mem18 的关系

| mem | 关注点 | mem19 是否覆盖 |
|---|---|---|
| mem17 | 生产平台可靠性（N4/JWT/供应链） | ❌ 不覆盖 |
| mem18 | 对标 AGFS（SDK/demos/shell） | ✅ 覆盖 SDK + demos |
| mem19 | 核心功能实现（让 EVIF 被用起来） | ✅ 全部覆盖 |

mem19 是 mem18 路线 A 的具体执行计划，不是另起炉灶。

---

## 10. 最终判断

EVIF 今天的工程状态已经非常扎实（clippy 全绿、核心包全测通），但缺少的是**外部用户接触 EVIF 的最短路径**。Python SDK + agent demos 是这条最短路径上最值得优先填的两个坑。

Phase C（REPL history）和 Phase D（测试诊断）是顺手的优化，不值得单独成计划，所以并入本 mem19。

一旦 Python SDK 和 agent demo 就绪，EVIF 就从"内部平台"变成了"外部可用产品"——这是今天最有价值的增量。
