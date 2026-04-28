# EVIF 智能体集成指南

## 1. 概览

EVIF 为 AI 智能体提供持久化上下文、可复用技能和多智能体协同。本指南涵盖与 Claude Code、Codex 和 OpenClaw 的集成。

## 2. Claude Code 集成

### 2.1 架构

```
┌─────────────────────────────────────────────────────────────┐
│                    Claude Code 会话                         │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  CLAUDE.md (会话指令)                                │   │
│  │  • 启动时检查 EVIF 健康状态                         │   │
│  │  • 读取 /context/L0/current (如可用)               │   │
│  │  • 写入进度到 /context/L1/decisions.md             │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  MCP 工具 (安装 @evif/mcp-server 后)               │   │
│  │  • evif_context_get, evif_context_set              │   │
│  │  • evif_skill_run, evif_memory_search              │   │
│  │  • evif_pipe_create, evif_pipe_send               │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  .claude/skills/*.SKILL.md (文档)                   │   │
│  │  • evif-context.SKILL.md                             │   │
│  │  • evif-workflows.SKILL.md                           │   │
│  │  • evif-pipes.SKILL.md                               │   │
│  │  • evif-memory.SKILL.md                             │   │
│  │  • evif-quickref.SKILL.md                            │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
         ┌────────────────────┼────────────────────┐
         │                    │                    │
         ▼                    ▼                    ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  MCP Server     │  │  REST API       │  │  CLI            │
│ @evif/mcp-server│  │  localhost:8081 │  │  evif CLI       │
└─────────────────┘  └─────────────────┘  └─────────────────┘
         │                    │                    │
         └────────────────────┼────────────────────┘
                              │
                    ┌─────────▼─────────┐
                    │   EVIF Server     │
                    │   evif-rest      │
                    └───────────────────┘
```

### 2.2 配置方式

#### 方式 A: MCP Server (推荐)

```bash
# 安装 MCP server
npm install -g @evif/mcp-server

# 添加到 Claude Code
claude mcp add @evif/mcp-server
```

或在 `~/.claude/settings.json` 中配置：

```json
{
  "mcpServers": {
    "evif": {
      "command": "npx",
      "args": ["-y", "@evif/mcp-server"],
      "env": {
        "EVIF_BASE_URL": "http://localhost:8081",
        "EVIF_AUTH_MODE": "disabled"
      }
    }
  }
}
```

#### 方式 B: preSession Hook (零额外安装)

在 `~/.claude/settings.json` 中配置：

```json
{
  "hooks": {
    "preSession": "evif start --daemon --port 8081 2>/dev/null || true"
  }
}
```

### 2.3 CLAUDE.md 配置

添加到项目的 `CLAUDE.md`：

```markdown
## EVIF 上下文集成

EVIF 在 Claude Code 会话间提供持久化上下文。

### 配置
EVIF 服务器应通过 preSession hook 自动启动。
检查: `curl -s http://localhost:8081/api/v1/health`

### 使用 (EVIF 可用时)
1. 会话开始: 读取 `/context/L0/current` → 了解当前任务
2. 会话开始: 读取 `/context/L1/decisions.md` → 了解历史决策
3. 工作时: 任务变化时更新 `/context/L0/current`
4. 工作时: 为重要决策追加到 `/context/L1/decisions.md`
5. 会话结束: 写入总结到 `/context/L0/current`

### 回退 (EVIF 不可用时)
- 静默跳过 EVIF 操作
- 继续正常的 Claude Code 行为

### 命令
- `evif cat /context/L0/current` - 读取当前任务
- `evif write -c "..." /context/L0/current` - 更新当前任务
- `evif cat /context/L1/decisions.md` - 读取决策
- `evif ls /skills/` - 列出可用技能
- `evif skill run code-review "Review src/"` - 运行技能
```

### 2.4 可用 MCP 工具

安装 `@evif/mcp-server` 后，Claude Code 获得：

```json
// 上下文管理
evif_context_get(layer: "L0" | "L1" | "L2") → string
evif_context_set(layer: string, content: string, append?: boolean) → void

// 文件操作
evif_ls(path: string) → FileEntry[]
evif_cat(path: string) → string
evif_write(path: string, content: string) → void
evif_mkdir(path: string) → void
evif_rm(path: string) → void

// 技能
evif_skill_list() → Skill[]
evif_skill_run(name: string, input: string) → string

// 记忆
evif_memory_search(query: string, limit?: number) → MemoryResult[]
evif_memory_store(content: string, modality?: string) → void

// 管道 (多智能体)
evif_pipe_create(name: string) → void
evif_pipe_send(name: string, data: string) → void
evif_pipe_status(name: string) → PipeStatus
```

### 2.5 示例会话

```
用户: 开始处理认证模块

Claude Code:
1. 检查 EVIF 健康状态 (通过 preSession hook, EVIF 正在运行)
2. 读取 /context/L0/current → "上次会话: 审查 PR #123"
3. 读取 /context/L1/decisions.md → "选择 JWT 而非 sessions"
4. 更新 /context/L0/current → "处理认证模块 (JWT 实现)"
5. 写入 /context/L1/decisions.md → "- 2026-04-27: 实现 JWT 认证"

[Claude Code 处理认证模块]

用户: 休息一下

Claude Code:
1. 写入 /context/L0/current → "暂停: 需要测试 JWT 实现"
2. 写入 /context/L1/decisions.md → "- 2026-04-27: JWT 实现完成，待测试"

[会话结束, Claude Code 退出]

下次会话:
用户: 继续

Claude Code:
1. 读取 /context/L0/current → "暂停: 需要测试 JWT 实现"
2. 读取 /context/L1/decisions.md → 查看所有历史决策
3. 从上次离开的地方继续
```

## 3. Codex 集成

### 3.1 架构

Codex (OpenAI 的 CLI 智能体) 可通过 Python SDK 使用 EVIF。

### 3.2 配置

```python
# codex_evif_plugin.py
import os
from evif import Client

class EVIFPlugin:
    def __init__(self, endpoint="http://localhost:8081"):
        self.endpoint = endpoint
        self.client = None

    def on_start(self):
        """会话开始时调用。"""
        try:
            self.client = Client(self.endpoint)
            # 验证连接
            if self.client.health().status == "healthy":
                self.restore_context()
        except Exception as e:
            print(f"EVIF 不可用: {e}")
            self.client = None

    def on_exit(self):
        """会话结束时调用。"""
        if self.client:
            self.save_context()

    def restore_context(self):
        """从 EVIF 恢复会话上下文。"""
        try:
            current = self.client.cat("/context/L0/current")
            if current:
                self.context = current.decode()
        except:
            self.context = ""

    def save_context(self):
        """保存会话上下文到 EVIF。"""
        if self.client and self.context:
            self.client.write("/context/L0/current", self.context)

    def check_health(self):
        """检查 EVIF 是否可用。"""
        if not self.client:
            return False
        try:
            return self.client.health().status == "healthy"
        except:
            return False
```

### 3.3 使用

```python
# 在 Codex prompt 或插件中使用
from codex_evif_plugin import EVIFPlugin

evif = EVIFPlugin()

# 检查 EVIF 是否可用
if evif.check_health():
    # 使用 EVIF 获取上下文
    current_task = evif.context
    print(f"继续: {current_task}")
else:
    print("EVIF 不可用")

# ... 工作 ...

# 保存进度
evif.context = "完成认证模块，移动到测试"
```

### 3.4 Codex 配置

添加到 `~/.codex/config.json`:

```json
{
  "plugins": [
    {
      "name": "evif",
      "module": "codex_evif_plugin",
      "enabled": true
    }
  ],
  "env": {
    "EVIF_ENDPOINT": "http://localhost:8081"
  }
}
```

## 4. OpenClaw 集成

### 4.1 架构

OpenClaw 使用 EVIF 的 PipeFS 进行多智能体协同。

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Manager       │     │   Worker A     │     │   Worker B     │
│   Agent         │     │                 │     │                 │
│                 │     │                 │     │                 │
│ • 创建管道      │     │ • 轮询管道     │     │ • 轮询管道     │
│ • 分配任务      │     │ • 执行         │     │ • 执行         │
│ • 聚合结果      │     │ • 报告结果     │     │ • 报告结果     │
└────────┬────────┘     └────────┬────────┘     └────────┬────────┘
         │                       │                        │
         │     ┌─────────────────┼────────────────────────┤
         │     │                 │                        │
         ▼     ▼                 ▼                        ▼
┌─────────────────────────────────────────────────────────────────┐
│                        EVIF PipeFS                              │
│  /pipes/review-pr-123/                                          │
│  ├── input: "Review authentication module"                     │
│  ├── assignee: "worker-a"                                       │
│  ├── status: "running"                                          │
│  └── output: "Found 2 issues: SQL injection, weak password"     │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 Worker 实现

```python
from evif import Client
from openclaw import Agent, Task
import asyncio

class EVIFWorker(Agent):
    """使用 EVIF 进行任务协同的 OpenClaw worker。"""

    def __init__(self, name: str, endpoint="http://localhost:8081"):
        super().__init__(name)
        self.evif = Client(endpoint)

    async def poll_tasks(self, queue_name: str, interval: int = 5):
        """轮询待处理任务。"""
        while True:
            try:
                # 检查待处理的管道
                entries = self.evif.ls("/pipes")
                for entry in entries:
                    if entry.is_dir:
                        pipe_name = entry.name
                        status = self.evif.cat(f"/pipes/{pipe_name}/status")
                        assignee = self.evif.cat(f"/pipes/{pipe_name}/assignee")

                        if status == "pending" and not assignee:
                            # 认领并处理任务
                            await self.process_task(pipe_name)
            except Exception as e:
                print(f"轮询错误: {e}")

            await asyncio.sleep(interval)

    async def process_task(self, pipe_name: str):
        """处理管道中的单个任务。"""
        # 认领任务
        self.evif.write(f"/pipes/{pipe_name}/assignee", self.name)
        self.evif.write(f"/pipes/{pipe_name}/status", "running")

        # 获取输入
        input_data = self.evif.cat(f"/pipes/{pipe_name}/input")
        if isinstance(input_data, bytes):
            input_data = input_data.decode()

        # 处理
        result = await self.execute_task(input_data)

        # 写入结果
        self.evif.write(f"/pipes/{pipe_name}/output", result)
        self.evif.write(f"/pipes/{pipe_name}/status", "complete")

        # 存储到记忆
        self.evif.memory_store(
            f"完成 {pipe_name}: {result[:100]}",
            modality="event"
        )

    async def execute_task(self, task_input: str) -> str:
        """执行实际任务。在子类中覆盖。"""
        return f"[{self.name}] processed: {task_input}"


# 运行 worker
async def main():
    worker = EVIFWorker("worker-1")
    await worker.poll_tasks("/pipes")

asyncio.run(main())
```

### 4.3 Manager 实现

```python
from evif import Client

class EVIFManager:
    """通过 EVIF 创建任务的 OpenClaw manager。"""

    def __init__(self, endpoint="http://localhost:8081"):
        self.evif = Client(endpoint)

    def create_task(self, name: str, input_data: str, timeout: int = 3600):
        """创建新任务管道。"""
        # 创建管道目录
        self.evif.mkdir(f"/pipes/{name}")

        # 写入任务输入
        self.evif.write(f"/pipes/{name}/input", input_data)

        # 设置超时
        self.evif.write(f"/pipes/{name}/timeout", str(timeout))

        # 初始状态
        self.evif.write(f"/pipes/{name}/status", "pending")

        return name

    def get_result(self, name: str, timeout: int = 300) -> str:
        """等待并返回任务结果。"""
        import time
        start = time.time()

        while time.time() - start < timeout:
            status = self.evif.cat(f"/pipes/{name}/status")
            if isinstance(status, bytes):
                status = status.decode()
            status = status.strip()

            if status == "complete":
                result = self.evif.cat(f"/pipes/{name}/output")
                if isinstance(result, bytes):
                    result = result.decode()
                return result
            elif status == "error":
                return f"错误: {self.evif.cat(f'/pipes/{name}/output')}"

            time.sleep(1)

        return "超时"

    def create_and_wait(self, name: str, input_data: str) -> str:
        """创建任务并等待结果。"""
        self.create_task(name, input_data)
        return self.get_result(name)


# 使用
manager = EVIFManager()
result = manager.create_and_wait(
    "review-auth-123",
    "Review src/auth/login.rs for security issues"
)
print(result)
```

### 4.4 OpenClaw 配置

```json
{
  "workers": [
    {
      "name": "review-worker",
      "image": "openclaw/worker:latest",
      "env": {
        "EVIF_ENDPOINT": "http://host.docker.internal:8081"
      },
      "command": "python worker.py"
    }
  ],
  "coordination": {
    "backend": "evif",
    "endpoint": "http://host.docker.internal:8081"
  }
}
```

## 5. 智能体平台的技能

### 5.1 标准技能格式

```yaml
---
name: code-review
description: "审查代码中的 bug 和安全问题"
triggers:
  - "review code"
  - "代码审查"
  - "check my code"
version: "1.0"
---

# Code Review Skill

## 描述
此技能执行全面的代码审查，识别：
- 安全漏洞
- 性能问题
- 代码质量问题
- 最佳实践违规

## 使用
1. 将输入写入 `/skills/code-review/input`
2. 技能执行
3. 从 `/skills/code-review/output` 读取输出

## 示例
```
evif write -c "Review src/auth/login.rs" /skills/code-review/input
evif cat /skills/code-review/output
```
```

### 5.2 内置技能

| 技能 | 路径 | 用途 |
|------|------|------|
| code-review | /skills/code-review | 安全和质量审查 |
| test-gen | /skills/test-gen | 生成测试用例 |
| doc-gen | /skills/doc-gen | 生成文档 |
| refactor | /skills/refactor | 代码重构建议 |
| security-audit | /skills/security-audit | 安全漏洞扫描 |

### 5.3 创建自定义技能

```bash
# 创建技能目录
evif mkdir /skills/my-skill

# 写入 SKILL.md
evif write -c '---
name: my-skill
description: "My custom skill"
triggers:
  - "do my task"
---
# My Skill

此技能做 X, Y, Z。

## 步骤
1. 步骤一
2. 步骤二
3. 返回结果
' /skills/my-skill/SKILL.md
```

## 6. 记忆模式

### 6.1 存储智能体知识

```python
# 存储学习到的信息
client.memory_store(
    content="认证模块使用 JWT 令牌，1 小时过期",
    modality="knowledge",
    metadata={
        "module": "auth",
        "type": "implementation",
        "confidence": "high"
    }
)

# 存储用户偏好
client.memory_store(
    content="用户偏好 TypeScript 而非 JavaScript",
    modality="preference",
    metadata={
        "category": "language",
        "preference": "typescript"
    }
)

# 存储重要事件
client.memory_store(
    content="部署 v2.3.0 到生产环境",
    modality="event",
    metadata={
        "version": "2.3.0",
        "environment": "production"
    }
)
```

### 6.2 检索知识

```python
# 搜索相关知识
results = client.memory_search("authentication JWT token")
for r in results:
    if r.get('score', 0) > 0.8:
        print(f"{r['score']:.2f}: {r['content']}")

# 列出指定类型的记忆
preferences = client.memory_list(modality="preference")
```

## 7. 完整示例: 多智能体代码审查

### 7.1 配置

```bash
# 启动 EVIF
evif start

# 创建审查管道
evif mkdir /pipes/pr-review-123
```

### 7.2 Manager Agent

```python
from evif import Client

manager = Client("http://localhost:8081")

# 创建审查任务
manager.write("/pipes/pr-review-123/input",
    "Review PR #123:\n"
    "- Files: src/auth/login.rs, src/auth/register.rs\n"
    "- Focus: SQL injection, XSS, authentication bypass"
)
manager.write("/pipes/pr-review-123/status", "pending")

print("审查任务已创建。Worker 将拾取任务。")
```

### 7.3 Worker Agent

```python
from evif import Client
import time

worker = Client("http://localhost:8081")

def poll_for_tasks():
    while True:
        entries = worker.ls("/pipes")
        for entry in entries:
            if entry.is_dir:
                name = entry.name

                # 检查是否待处理
                try:
                    status = worker.cat(f"/pipes/{name}/status")
                    if isinstance(status, bytes):
                        status = status.decode().strip()

                    if status == "pending":
                        # 认领任务
                        worker.write(f"/pipes/{name}/assignee", "worker-1")
                        worker.write(f"/pipes/{name}/status", "running")

                        # 处理
                        task = worker.cat(f"/pipes/{name}/input")
                        result = perform_review(task)

                        # 完成
                        worker.write(f"/pipes/{name}/output", result)
                        worker.write(f"/pipes/{name}/status", "complete")

                        print(f"完成: {name}")
                except:
                    pass

        time.sleep(5)

def perform_review(task):
    # 实际审查逻辑
    return "发现 2 个问题:\n1. 登录中的 SQL 注入\n2. 缺少 CSRF token"

poll_for_tasks()
```

### 7.4 获取结果

```bash
# Manager 检查结果
evif cat /pipes/pr-review-123/status
evif cat /pipes/pr-review-123/output
```

## 8. 相关文档

- [架构概览](00-overview.md)
- [SDK 集成](04-sdk-integration.md)
- [REST API 参考](03-rest-api.md)
- [CLI 参考](../cli-mode.md)
- [MCP Server 配置](../mcp-server.md)