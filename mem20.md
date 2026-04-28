# EVIF mem20.md — MVP 1.0 实现计划（2026-04-26）

> 创建时间：2026-04-26
> 分析基础：mem19 完成 + 代码分析 + 商业价值分析
> 计划目标：制定最小可用 MVP，让 EVIF 在 2 周内具备"外部用户 5 分钟上手"能力

---

## 1. MVP 定义

### 1.1 什么是 MVP

**最小可行产品** = 能让外部 Python 开发者/AI Agent 在 5 分钟内体验到 EVIF 核心价值的版本。

### 1.2 MVP 必须包含

| 功能 | 说明 | 商业价值 |
|---|---|---|
| **Python SDK** | 能 `pip install` 后一行代码接入 | 降低 70% 接入成本 |
| **Agent Demo** | 可运行的 queue/pipe 示例 | 展示多 Agent 协调能力 |
| **基础文档** | Quick Start + API Reference | 减少上手摩擦 |
| **REST API 就绪** | 健康检查 + 基础 CRUD | 所有接入面的基础 |

### 1.3 MVP 不包含

以下功能虽有价值，但在 MVP 阶段暂不实现：

| 功能 | 原因 |
|---|---|
| JWT/OAuth | 大工程，不是 MVP 阻塞项 |
| N4 分布式集群 | 架构级改动，不是一周能完成 |
| 供应链安全清理 | 可以后续处理 |
| GraphQL production readiness | REST 已够用 |
| 云存储插件（S3/Azure/GCS） | 先让核心跑起来 |

---

## 2. MVP 功能清单

### 2.1 必须实现（Must Have）

#### F1：Python SDK Bug 修复

**当前问题**：
1. Sync wrapper 方法签名与 async 方法不匹配
2. 使用 compat API 端点而非官方端点

**修复**：
- `crates/evif-python/evif/sync.py` — 修正 skill 方法签名
- `crates/evif-python/evif/client.py` — 迁移到官方 REST 端点

**验收**：
```bash
PYTHONPATH=crates/evif-python python3 -c "
from evif import Client
c = Client('http://localhost:8081')
# 不报错
import inspect
print(inspect.signature(type(c).skill_discover))
"
```

#### F2：Python SDK README

**内容**：
- 安装方式（pip install 或 PYTHONPATH）
- Quick Start 示例（健康检查 + 基础操作）
- API Reference（ls/cat/write/mkdir/mount/memory/queue/pipe）
- Demo 说明

**验收**：`ls crates/evif-python/README.md` → 文件存在且 > 100 行

#### F3：Agent Demo 可运行

**Demo 清单**：
1. `task_queue_worker.py` — Queue-based 多 Agent
2. `pipe_triggered_agent.py` — Pipe-based Agent 协调
3. `start_demo.sh` — 一键启动脚本

**验收**：
```bash
cargo run -p evif-rest -- --port 8081 &
sleep 3
./demos/agent_workflow/start_demo.sh
# 全程无报错
```

#### F4：REST API 健康检查

**确保**：
- `GET /api/v1/health` 返回状态信息
- `GET /api/v1/mounts` 返回挂载点列表
- `GET /api/v1/plugins` 返回插件列表

**验收**：
```bash
curl http://localhost:8081/api/v1/health
# 返回 {"status": "healthy", ...}
```

### 2.2 可选实现（Should Have）

#### F5：EVIF 主 README 优化

更新 README.md，添加：
- 快速开始（60 秒内跑起来）
- Python SDK 使用示例
- Agent Demo 说明

#### F6：测试清理

标记失败的集成测试为 e2e（需要运行服务器），不影响核心包测试通过。

### 2.3 暂不实现（Won't Have）

| 功能 | 原因 |
|---|---|
| JWT 认证 | MVP 用 API Key 足够 |
| 分布式集群 | 单机 MVP 先跑起来 |
| 云存储插件 | 先验证核心逻辑 |
| 供应链清理 | 后续迭代处理 |

---

## 3. MVP 技术方案

### 3.1 Python SDK 架构

```
evif/
├── client.py      # EvifClient (async) + REST 端点映射
├── sync.py        # SyncEvifClient + Client() 工厂函数
├── memory.py      # MemoryApi mixin
├── queue.py       # QueueApi + PipeApi mixin
├── context.py     # ContextApi mixin
├── skill.py       # SkillApi mixin
└── models.py      # Pydantic 模型
```

**设计原则**：
- 纯 async，httpx 作为 HTTP 客户端
- 统一错误处理（EvifError）
- retry 逻辑（tenacity）
- 通过 sync.py 提供同步接口

### 3.2 REST API 端点

```python
# 官方端点（迁移目标）
GET  /api/v1/files?path=...
PUT  /api/v1/files?path=...
POST /api/v1/files (create)
DELETE /api/v1/files?path=...
GET  /api/v1/directories?path=...
POST /api/v1/directories
DELETE /api/v1/directories?path=...
POST /api/v1/stat
GET  /api/v1/mounts
POST /api/v1/mount
POST /api/v1/unmount
GET  /api/v1/health
GET  /api/v1/plugins
```

### 3.3 Demo 架构

```
demos/agent_workflow/
├── start_demo.sh      # 一键启动：evif-rest + Python SDK smoke test + demo
├── task_queue_worker.py  # 展示 Queue-based 多 Agent
└── pipe_triggered_agent.py  # 展示 Pipe-based Agent 协调
```

---

## 4. 实施计划

### Week 1：核心功能

#### Day 1-2：Python SDK Bug 修复

| 任务 | 文件 | 验收 |
|---|---|---|
| 修正 skill 方法签名 | sync.py | `inspect.signature` 输出正确 |
| 迁移到官方端点 | client.py | API 调用走 `/api/v1/files` 等 |
| 更新 health 端点 | client.py | `/api/v1/health` |

#### Day 3-4：Python SDK README

| 任务 | 内容 |
|---|---|
| 安装指南 | pip install / PYTHONPATH |
| Quick Start | 5 行代码示例 |
| API Reference | 所有方法说明 |
| Demo 说明 | 如何运行 |

#### Day 5：测试

| 任务 | 验收 |
|---|---|
| 核心包测试通过 | `cargo test -p evif-core -p evif-auth` |
| SDK 导入正常 | `python3 -c "from evif import Client"` |

### Week 2：Demo + 验证

#### Day 6-7：Agent Demo

| 任务 | 文件 |
|---|---|
| 确保 queue demo 可跑 | task_queue_worker.py |
| 确保 pipe demo 可跑 | pipe_triggered_agent.py |
| 创建一键启动脚本 | start_demo.sh |

#### Day 8-9：E2E 验证

| 任务 | 验收 |
|---|---|
| evif-rest 启动 | `curl /api/v1/health` 正常 |
| Python SDK 调用 | `from evif import Client; c.health()` 正常 |
| Agent demo 运行 | `./start_demo.sh` 无报错 |
| Clippy 检查 | `cargo clippy --workspace -- -D warnings` 退出 0 |

#### Day 10：文档完善

| 任务 | 产出 |
|---|---|
| README 更新 | Quick Start + Python SDK 示例 |
| Demo 说明 | 如何运行 + 预期输出 |

---

## 5. 验收标准

### 5.1 功能验收

| 功能 | 验收标准 |
|---|---|
| Python SDK 安装 | `PYTHONPATH=crates/evif-python python3 -c "from evif import Client; print(Client())"` |
| Python SDK README | `wc -l crates/evif-python/README.md` > 100 |
| REST API 健康 | `curl -s http://localhost:8081/api/v1/health` 返回 JSON |
| Agent Demo 可运行 | `curl -s http://localhost:8081/api/v1/health && ./demos/agent_workflow/start_demo.sh` |
| Clippy 全量通过 | `cargo clippy --workspace --all-targets -- -D warnings` 退出 0 |

### 5.2 性能验收

| 指标 | 目标 |
|---|---|
| SDK 导入时间 | < 1 秒 |
| Health check 响应 | < 100ms |
| Demo 启动时间 | < 30 秒 |

### 5.3 文档验收

| 文档 | 验收标准 |
|---|---|
| Python SDK README | 包含安装、Quick Start、API Reference |
| 主 README | 包含 60 秒上手指南 |
| Demo 说明 | 包含运行方式和预期输出 |

---

## 6. 不在 MVP 范围内

### 6.1 暂不实现的功能

| 功能 | 原因 | 后续计划 |
|---|---|---|
| JWT/OAuth | 大工程，MVP 用 API Key 足够 | mem21+ |
| 分布式集群 | 架构级改动 | N4 路线 |
| 云存储插件 | 先验证核心 | 1.0 后 |
| 供应链安全 | 可后续处理 | Q2 |
| GraphQL | REST 已够用 | 1.0 后 |

### 6.2 暂不修复的问题

| 问题 | 原因 | 后续计划 |
|---|---|---|
| system-configuration panic | 第三方 crate，不影响核心包 | 1.0 后 |
| 26 个集成测试失败 | 标记为 e2e 即可 | Day 5 |
| clippy 1 个 warning | 不阻断编译 | Day 9 |

---

## 7. MVP 后的路线图

### 7.1 1.0 稳定版（4-6 周）

| 功能 | 说明 |
|---|---|
| API 稳定性承诺 | 语义化版本 |
| 云存储插件 | S3/Azure/GCS 生产级 |
| JWT/OAuth | 企业级认证 |
| 配置热更新 | N12 |

### 7.2 2.0 分布式（3-6 个月）

| 功能 | 说明 |
|---|---|
| N4 集群 | Service discovery + Leader election |
| 全局限流 | 跨节点 |
| 多区域复制 | Geo-replication |

---

## 8. 风险与应对

| 风险 | 影响 | 应对 |
|---|---|---|
| Python SDK 有隐藏 bug | 用户体验差 | Day 5 全面测试 |
| Demo 运行不稳定 | 无法展示价值 | Day 8-9 E2E 验证 |
| 文档不够清晰 | 上手困难 | Day 10 文档审核 |
| 其他问题导致 delay | 超过 2 周 | 聚焦 MVP，削减 scope |

---

## 9. 关键指标

### 9.1 MVP 成功指标

| 指标 | 目标 |
|---|---|
| Python SDK 可导入 | ✅ |
| Demo 可运行 | ✅ |
| README 存在且完整 | ✅ |
| Clippy 通过 | ✅ |
| 用户可 5 分钟上手 | ✅ |

### 9.2 后续目标（1.0 稳定版）

| 指标 | 目标 |
|---|---|
| 云存储插件支持 | S3, Azure, GCS |
| 企业级认证 | JWT/OAuth |
| API 稳定性 | 语义化版本 |
| 测试覆盖率 | > 80% |

---

## 10. 最终判断

**MVP 目标**：2 周内让外部 Python 开发者/AI Agent 能 5 分钟内体验到 EVIF 的多 Agent 协调和统一存储接口价值。

**核心策略**：
1. 修复 Python SDK bug（签名 + 端点）
2. 写清楚文档（README + Demo 说明）
3. 确保 Demo 可运行（queue + pipe）

**不做**：
- 不做 JWT/OAuth（MVP 用 API Key）
- 不做分布式（N4 路线图后续）
- 不做云存储插件（1.0 后）

**一句话：先让人用起来，再做对。**

---

## 11. 与 mem17/mem18/mem19 的关系

| mem | 关注点 | mem20 MVP 覆盖 |
|---|---|---|
| mem17 | 生产平台可靠性（N4/JWT/供应链） | ❌ 暂不覆盖（MVP 后处理） |
| mem18 | 对标 AGFS（SDK/demos/shell） | ✅ 覆盖 Python SDK + Agent Demo |
| mem19 | 核心功能实现（Python SDK + Agent Demo） | ✅ 基于此完成 bug 修复 + 文档 |
| mem20 | MVP 1.0 实现计划 | ✅ 本 mem |

**Mem20 是 Mem19 的收尾 + Mem18 的最终交付**，确保 Python SDK 和 Agent Demo 从"能跑"变成"好用"。