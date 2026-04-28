# EVIF SDK 集成指南

EVIF 提供 Python、TypeScript 和 Go 的 SDK。每个 SDK 都提供符合语言习惯的方式访问 EVIF 的虚拟文件系统和智能体原语。

## 1. 概览

| SDK | 包名 | 用途 |
|-----|------|------|
| Python | `evif` | CLI 脚本、数据处理 |
| TypeScript | `@evif/sdk` | Web 应用、Node.js |
| Go | `github.com/evif/evif-go` | 服务端、CLI |
| MCP | `@evif/mcp-server` | Claude Code 原生集成 |

## 2. Python SDK

**包**: `evif` (本地安装: `pip install -e crates/evif-python`)

### 2.1 安装

```bash
# 从本地源码安装 (开发)
pip install -e /path/to/evif/crates/evif-python

# 从 PyPI 安装 (发布后)
pip install evif
```

### 2.2 快速开始

```python
from evif import Client

# 同步客户端 (推荐用于 CLI 脚本)
client = Client("http://localhost:8081")
print(client.health())
print(client.ls("/mem"))
```

### 2.3 异步客户端

```python
import asyncio
from evif import EvifClient

async def main():
    async with EvifClient("http://localhost:8081") as client:
        await client.connect()

        # 文件操作
        await client.write("/mem/test.txt", "Hello, EVIF!")
        content = await client.cat("/mem/test.txt")

        # 健康检查
        status = await client.health()
        print(f"Server: {status.status}")

asyncio.run(main())
```

### 2.4 文件操作

```python
# 列出目录
entries = client.ls("/mem")
for entry in entries:
    print(f"{'dir' if entry.is_dir else 'file'}: {entry.name}")

# 读取文件
content = client.cat("/mem/test.txt")
if isinstance(content, bytes):
    content = content.decode()

# 写入文件
bytes_written = client.write("/mem/test.txt", "Hello!")

# 创建目录
client.mkdir("/mem/new_dir")

# 删除文件
client.rm("/mem/test.txt")

# Stat
info = client.stat("/mem/test.txt")
print(f"Size: {info.size}")

# 移动/重命名
client.mv("/mem/old.txt", "/mem/new.txt")

# 复制
client.cp("/mem/src.txt", "/mem/dst.txt")

# 搜索
matches = client.grep("/mem", "pattern")
for match in matches:
    print(match)
```

### 2.5 上下文操作

```python
from evif import ContextApi

# 设置当前任务
client.write("/context/L0/current", "Review PR #123")

# 读取当前上下文
content = client.cat("/context/L0/current")

# 记录决策
client.write("/context/L1/decisions.md",
             "- Chose JWT over sessions\n",
             append=True)

# 列出 L2 知识
entries = client.ls("/context/L2")
```

### 2.6 内存操作

```python
# 存储记忆
result = client.memory_store(
    content="User prefers dark mode",
    modality="preference",
    metadata={"user": "dev1", "tool": "vscode"}
)
memory_id = result.get("memory_id", result.get("id"))

# 列出记忆
memories = client.memory_list(modality="preference")
for m in memories:
    print(f"{m['id']}: {m['content']}")

# 搜索记忆
results = client.memory_search("editor theme")
for r in results:
    print(f"{r.get('score', 0):.2f}: {r['content']}")
```

### 2.7 队列操作

```python
# 推送任务到队列
import json
client.write("/queue/tasks/enqueue", json.dumps({
    "type": "review",
    "target": "src/auth/"
}))

# 弹出任务 (原子)
data = client.cat("/queue/tasks/dequeue")
if data:
    task = json.loads(data.decode())

# 查看大小
size_info = client.cat("/queue/tasks/size")
print(f"Queue size: {size_info.decode()}")
```

### 2.8 挂载操作

```python
# 列出挂载
mounts = client.mounts()
for mount in mounts:
    print(f"{mount.path} -> {mount.plugin}")

# 挂载插件
client.mount("s3fs", "/s3", {
    "bucket": "my-bucket",
    "region": "us-east-1"
})

# 卸载
client.unmount("/s3")
```

### 2.9 错误处理

```python
from evif.exceptions import (
    EvifError,
    FileNotFoundError,
    PermissionError,
    TimeoutError
)

try:
    client.cat("/nonexistent")
except FileNotFoundError as e:
    print(f"File not found: {e}")
except PermissionError as e:
    print(f"Permission denied: {e}")
except TimeoutError as e:
    print(f"Request timed out: {e}")
except EvifError as e:
    print(f"EVIF error: {e}")
```

### 2.10 流式操作

```python
# 分块读取大文件
async for chunk in client.stream("/mem/large.bin"):
    process(chunk)

# 从迭代器写入
async def data_generator():
    for i in range(100):
        yield f"chunk {i}".encode()

await client.stream_write("/mem/output.bin", data_generator())
```

## 3. TypeScript SDK

**包**: `@evif/sdk` 或 `@evif/mcp-server`

### 3.1 安装

```bash
npm install @evif/sdk
```

### 3.2 快速开始

```typescript
import { EvifClient } from '@evif/sdk';

const client = new EvifClient({
  baseUrl: 'http://localhost:8081',
  timeout: 30000,
});

const status = await client.health();
console.log(`Server: ${status.status}`);

const files = await client.ls('/mem');
console.log(`${files.length} files`);
```

### 3.3 文件操作

```typescript
// 列出目录
const entries = await client.ls('/mem');

// 读取文件
const content = await client.cat('/mem/test.txt');
if (typeof content === 'string') {
    console.log(content);
}

// 写入文件
await client.write('/mem/test.txt', 'Hello!');

// 创建目录
await client.mkdir('/mem/newdir');

// 删除
await client.rm('/mem/test.txt');
```

### 3.4 类型定义

```typescript
interface FileEntry {
  name: string;
  path: string;
  size: number;
  is_dir: boolean;
  modified: string;
  created: string;
}

interface MountInfo {
  path: string;
  plugin: string;
  options: Record<string, any>;
}

interface HealthStatus {
  status: string;
  version: string;
  uptime: number;
}

interface Memory {
  id: string;
  content: string;
  type: string;
  created: string;
  updated: string;
}
```

## 4. Go SDK

**包**: `github.com/evif/evif-go`

### 4.1 安装

```bash
go get github.com/evif/evif-go
```

### 4.2 快速开始

```go
package main

import (
    "fmt"
    evif "github.com/evif/evif-go"
)

func main() {
    client := evif.NewClient("http://localhost:8081")

    status, err := client.Health()
    if err != nil {
        panic(err)
    }
    fmt.Printf("Server: %s (v%s)\n", status.Status, status.Version)

    files, err := client.Ls("/mem")
    if err != nil {
        panic(err)
    }
    fmt.Printf("%d files\n", len(files))
}
```

### 4.3 文件操作

```go
// 列出目录
entries, err := client.Ls("/mem")
if err != nil {
    return nil, err
}

// 读取文件
content, err := client.Cat("/mem/test.txt")
if err != nil {
    return nil, err
}

// 写入文件
bytesWritten, err := client.Write("/mem/test.txt", []byte("Hello!"))
if err != nil {
    return nil, err
}

// 创建目录
err = client.Mkdir("/mem/newdir")

// 删除
err = client.Rm("/mem/test.txt")
```

## 5. MCP Server

**包**: `@evif/mcp-server`

### 5.1 安装

```bash
npm install -g @evif/mcp-server
```

### 5.2 Claude Code 配置

```bash
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
        "EVIF_BASE_URL": "http://localhost:8081"
      }
    }
  }
}
```

### 5.3 可用工具

```json
{
  "tools": [
    {
      "name": "evif_health",
      "description": "检查 EVIF 服务器健康状态",
      "inputSchema": {}
    },
    {
      "name": "evif_context_get",
      "description": "读取上下文层级",
      "inputSchema": {
        "type": "object",
        "properties": {
          "layer": {
            "type": "string",
            "enum": ["L0", "L1", "L2"]
          }
        }
      }
    },
    {
      "name": "evif_context_set",
      "description": "写入上下文层级",
      "inputSchema": {
        "type": "object",
        "properties": {
          "layer": { "type": "string" },
          "content": { "type": "string" },
          "append": { "type": "boolean" }
        },
        "required": ["layer", "content"]
      }
    },
    {
      "name": "evif_ls",
      "description": "列出目录",
      "inputSchema": {
        "type": "object",
        "properties": {
          "path": { "type": "string" }
        },
        "required": ["path"]
      }
    },
    {
      "name": "evif_cat",
      "description": "读取文件",
      "inputSchema": {
        "type": "object",
        "properties": {
          "path": { "type": "string" }
        },
        "required": ["path"]
      }
    },
    {
      "name": "evif_write",
      "description": "写入文件",
      "inputSchema": {
        "type": "object",
        "properties": {
          "path": { "type": "string" },
          "content": { "type": "string" }
        },
        "required": ["path", "content"]
      }
    },
    {
      "name": "evif_skill_list",
      "description": "列出可用技能",
      "inputSchema": {}
    },
    {
      "name": "evif_skill_run",
      "description": "运行技能",
      "inputSchema": {
        "type": "object",
        "properties": {
          "name": { "type": "string" },
          "input": { "type": "string" }
        },
        "required": ["name", "input"]
      }
    },
    {
      "name": "evif_memory_search",
      "description": "搜索记忆",
      "inputSchema": {
        "type": "object",
        "properties": {
          "query": { "type": "string" },
          "limit": { "type": "number" }
        },
        "required": ["query"]
      }
    },
    {
      "name": "evif_memory_store",
      "description": "存储记忆",
      "inputSchema": {
        "type": "object",
        "properties": {
          "content": { "type": "string" },
          "modality": { "type": "string" }
        },
        "required": ["content"]
      }
    },
    {
      "name": "evif_pipe_create",
      "description": "创建管道用于智能体协同",
      "inputSchema": {
        "type": "object",
        "properties": {
          "name": { "type": "string" }
        },
        "required": ["name"]
      }
    },
    {
      "name": "evif_pipe_send",
      "description": "发送消息到管道",
      "inputSchema": {
        "type": "object",
        "properties": {
          "name": { "type": "string" },
          "data": { "type": "string" }
        },
        "required": ["name", "data"]
      }
    },
    {
      "name": "evif_pipe_status",
      "description": "获取管道状态",
      "inputSchema": {
        "type": "object",
        "properties": {
          "name": { "type": "string" }
        },
        "required": ["name"]
      }
    }
  ]
}
```

## 6. 配置

### 6.1 环境变量

```bash
# 服务器 URL
export EVIF_BASE_URL=http://localhost:8081

# API 密钥
export EVIF_API_KEY=your-api-key

# 超时 (秒)
export EVIF_TIMEOUT=30

# 重试次数
export EVIF_MAX_RETRIES=3
```

### 6.2 客户端选项

**Python**:
```python
client = Client(
    base_url="http://localhost:8081",
    api_key="your-key",
    timeout=30,
    max_retries=3,
    trust_env=False  # 禁用代理读取
)
```

**TypeScript**:
```typescript
const client = new EvifClient({
  baseUrl: 'http://localhost:8081',
  apiKey: 'your-key',
  timeout: 30000,
  retries: 3,
});
```

**Go**:
```go
client := evif.NewClient(
    "http://localhost:8081",
    evif.WithAPIKey("your-key"),
    evif.WithTimeout(30*time.Second),
    evif.WithRetries(3),
)
```

## 7. 相关文档

- [REST API 参考](03-rest-api.md)
- [快速开始](../GETTING_STARTED.md)
- [智能体集成](05-agent-integration.md)