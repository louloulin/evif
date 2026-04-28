# EVIF SDK Integration Guide

## 1. Overview

EVIF provides SDKs for Python, TypeScript, and Go. Each SDK provides idiomatic access to EVIF's virtual filesystem and Agent primitives.

## 2. Python SDK

**Package**: `evif` (local install: `pip install -e crates/evif-python`)

### 2.1 Installation

```bash
# From local source (development)
pip install -e /path/to/evif/crates/evif-python

# From PyPI (when published)
pip install evif
```

### 2.2 Quick Start

```python
from evif import Client

# Sync client (recommended for CLI scripts)
client = Client("http://localhost:8081")
print(client.health())
print(client.ls("/mem"))
```

### 2.3 Async Client

```python
import asyncio
from evif import EvifClient

async def main():
    async with EvifClient("http://localhost:8081") as client:
        await client.connect()

        # File operations
        await client.write("/mem/test.txt", "Hello, EVIF!")
        content = await client.cat("/mem/test.txt")

        # Health check
        status = await client.health()
        print(f"Server: {status.status}")

asyncio.run(main())
```

### 2.4 File Operations

```python
# List directory
entries = client.ls("/mem")
for entry in entries:
    print(f"{'dir' if entry.is_dir else 'file'}: {entry.name}")

# Read file
content = client.cat("/mem/test.txt")
if isinstance(content, bytes):
    content = content.decode()

# Write file
bytes_written = client.write("/mem/test.txt", "Hello!")

# Create directory
client.mkdir("/mem/new_dir")

# Remove file
client.rm("/mem/test.txt")

# Stat
info = client.stat("/mem/test.txt")
print(f"Size: {info.size}")

# Move/Rename
client.mv("/mem/old.txt", "/mem/new.txt")

# Copy
client.cp("/mem/src.txt", "/mem/dst.txt")

# Grep
matches = client.grep("/mem", "pattern")
for match in matches:
    print(match)
```

### 2.5 Context Operations

```python
from evif import ContextApi

# Note: Context operations are available via file ops
# or via the context.py module

# Set current task
client.write("/context/L0/current", "Review PR #123")

# Read current context
content = client.cat("/context/L0/current")

# Record decision
client.write("/context/L1/decisions.md",
             "- Chose JWT over sessions\n",
             append=True)

# List L2 knowledge
entries = client.ls("/context/L2")
```

### 2.6 Memory Operations

```python
# Store a memory
result = client.memory_store(
    content="User prefers dark mode",
    modality="preference",
    metadata={"user": "dev1", "tool": "vscode"}
)
memory_id = result.get("memory_id", result.get("id"))

# List memories
memories = client.memory_list(modality="preference")
for m in memories:
    print(f"{m['id']}: {m['content']}")

# Search memories
results = client.memory_search("editor theme")
for r in results:
    print(f"{r.get('score', 0):.2f}: {r['content']}")
```

### 2.7 Queue Operations

```python
# Push task to queue
import json
client.write("/queue/tasks/enqueue", json.dumps({
    "type": "review",
    "target": "src/auth/"
}))

# Pop task (atomic)
data = client.cat("/queue/tasks/dequeue")
if data:
    task = json.loads(data.decode())

# Check size
size_info = client.cat("/queue/tasks/size")
print(f"Queue size: {size_info.decode()}")
```

### 2.8 Mount Operations

```python
# List mounts
mounts = client.mounts()
for mount in mounts:
    print(f"{mount.path} -> {mount.plugin}")

# Mount a plugin
client.mount("s3fs", "/s3", {
    "bucket": "my-bucket",
    "region": "us-east-1"
})

# Unmount
client.unmount("/s3")
```

### 2.9 Error Handling

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

### 2.10 Streaming

```python
# Read large file in chunks
async for chunk in client.stream("/mem/large.bin"):
    process(chunk)

# Write from iterator
async def data_generator():
    for i in range(100):
        yield f"chunk {i}".encode()

await client.stream_write("/mem/output.bin", data_generator())
```

## 3. TypeScript SDK

**Package**: `@evif/sdk` or `@evif/mcp-server`

### 3.1 Installation

```bash
npm install @evif/sdk
```

### 3.2 Quick Start

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

### 3.3 File Operations

```typescript
// List directory
const entries = await client.ls('/mem');

// Read file
const content = await client.cat('/mem/test.txt');
if (typeof content === 'string') {
    console.log(content);
}

// Write file
await client.write('/mem/test.txt', 'Hello!');

// Create directory
await client.mkdir('/mem/newdir');

// Remove
await client.rm('/mem/test.txt');
```

### 3.4 Types

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

**Package**: `github.com/evif/evif-go`

### 4.1 Installation

```bash
go get github.com/evif/evif-go
```

### 4.2 Quick Start

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

### 4.3 File Operations

```go
// List directory
entries, err := client.Ls("/mem")
if err != nil {
    return nil, err
}

// Read file
content, err := client.Cat("/mem/test.txt")
if err != nil {
    return nil, err
}

// Write file
bytesWritten, err := client.Write("/mem/test.txt", []byte("Hello!"))
if err != nil {
    return nil, err
}

// Create directory
err = client.Mkdir("/mem/newdir")

// Remove
err = client.Rm("/mem/test.txt")
```

## 5. MCP Server

**Package**: `@evif/mcp-server`

### 5.1 Installation

```bash
npm install -g @evif/mcp-server
```

### 5.2 Claude Code Setup

```bash
# Add to Claude Code
claude mcp add @evif/mcp-server
```

Or add to `~/.claude/settings.json`:

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

### 5.3 Available Tools

```json
{
  "tools": [
    {
      "name": "evif_health",
      "description": "Check EVIF server health",
      "inputSchema": {}
    },
    {
      "name": "evif_context_get",
      "description": "Read context layer",
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
      "description": "Write to context layer",
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
      "description": "List directory",
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
      "description": "Read file",
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
      "description": "Write file",
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
      "description": "List available skills",
      "inputSchema": {}
    },
    {
      "name": "evif_skill_run",
      "description": "Run a skill",
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
      "description": "Search memories",
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
      "description": "Store a memory",
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
      "description": "Create a pipe for agent coordination",
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
      "description": "Send message to pipe",
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
      "description": "Get pipe status",
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

## 6. Configuration

### 6.1 Environment Variables

```bash
# Server URL
export EVIF_BASE_URL=http://localhost:8081

# API Key
export EVIF_API_KEY=your-api-key

# Timeout (seconds)
export EVIF_TIMEOUT=30

# Retry attempts
export EVIF_MAX_RETRIES=3
```

### 6.2 Client Options

**Python**:
```python
client = Client(
    base_url="http://localhost:8081",
    api_key="your-key",
    timeout=30,
    max_retries=3,
    trust_env=False  # Disable proxy reading
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

## 7. Streaming Examples

### 7.1 Python Streaming

```python
# Read large file with streaming
async with EvifClient() as client:
    async for chunk in client.stream_read("/data/large.bin"):
        await process_chunk(chunk)

# Write with streaming
async def data_source():
    for i in range(1000):
        yield f"data chunk {i}\n".encode()

await client.stream_write("/data/output.txt", data_source())
```

### 7.2 TypeScript Streaming

```typescript
// Read file as stream
const stream = await client.createReadStream('/data/large.bin');
for await (const chunk of stream) {
    process(chunk);
}

// Write from stream
const readable = new ReadableStream({
    start(controller) {
        // Generate data
        controller.enqueue(data);
        controller.close();
    }
});
await client.writeStream('/data/output.bin', readable);
```

## 8. Related Documents

- [REST API Reference](03-rest-api.md)
- [Getting Started](GETTING_STARTED.md)
- [Agent Integration](05-agent-integration.md)
