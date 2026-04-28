# EVIF Agent Integration Guide

## 1. Overview

EVIF provides persistent context, reusable skills, and multi-agent coordination for AI agents. This guide covers integration with Claude Code, Codex, and OpenClaw.

## 2. Claude Code Integration

### 2.1 Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Claude Code Session                        │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  CLAUDE.md (session instructions)                    │   │
│  │  • Check EVIF health on start                       │   │
│  │  • Read /context/L0/current if available            │   │
│  │  • Write progress to /context/L1/decisions.md       │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  MCP Tools (when @evif/mcp-server installed)        │   │
│  │  • evif_context_get, evif_context_set                │   │
│  │  • evif_skill_run, evif_memory_search               │   │
│  │  • evif_pipe_create, evif_pipe_send                 │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  .claude/skills/*.SKILL.md (documentation)          │   │
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

### 2.2 Setup Methods

#### Method A: MCP Server (Recommended)

```bash
# Install MCP server
npm install -g @evif/mcp-server

# Add to Claude Code
claude mcp add @evif/mcp-server
```

Or configure in `~/.claude/settings.json`:

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

#### Method B: preSession Hook (Zero Extra Install)

Configure in `~/.claude/settings.json`:

```json
{
  "hooks": {
    "preSession": "evif start --daemon --port 8081 2>/dev/null || true"
  }
}
```

### 2.3 CLAUDE.md Configuration

Add to your project's `CLAUDE.md`:

```markdown
## EVIF Context Integration

EVIF provides persistent context across Claude Code sessions.

### Setup
EVIF server should auto-start via preSession hook.
Check: `curl -s http://localhost:8081/api/v1/health`

### Usage (when EVIF available)
1. Session start: Read `/context/L0/current` → learn current task
2. Session start: Read `/context/L1/decisions.md` → learn past decisions
3. Work: Update `/context/L0/current` when task changes
4. Work: Append to `/context/L1/decisions.md` for important decisions
5. Session end: Write summary to `/context/L0/current`

### Fallback (when EVIF unavailable)
- Skip EVIF operations silently
- Continue with normal Claude Code behavior

### Commands
- `evif cat /context/L0/current` - Read current task
- `evif write -c "..." /context/L0/current` - Update current task
- `evif cat /context/L1/decisions.md` - Read decisions
- `evif ls /skills/` - List available skills
- `evif skill run code-review "Review src/"` - Run a skill
```

### 2.4 Available MCP Tools

Once `@evif/mcp-server` is installed, Claude Code has access to:

```json
// Context management
evif_context_get(layer: "L0" | "L1" | "L2") → string
evif_context_set(layer: string, content: string, append?: boolean) → void

// File operations
evif_ls(path: string) → FileEntry[]
evif_cat(path: string) → string
evif_write(path: string, content: string) → void
evif_mkdir(path: string) → void
evif_rm(path: string) → void

// Skills
evif_skill_list() → Skill[]
evif_skill_run(name: string, input: string) → string

// Memory
evif_memory_search(query: string, limit?: number) → MemoryResult[]
evif_memory_store(content: string, modality?: string) → void

// Pipes (Multi-agent)
evif_pipe_create(name: string) → void
evif_pipe_send(name: string, data: string) → void
evif_pipe_status(name: string) → PipeStatus
```

### 2.5 Example Session

```
User: Start working on the auth module

Claude Code:
1. Checks EVIF health (via preSession hook, EVIF is running)
2. Reads /context/L0/current → "Previous session: Review PR #123"
3. Reads /context/L1/decisions.md → "Chose JWT over sessions"
4. Updates /context/L0/current → "Working on auth module (JWT implementation)"
5. Writes to /context/L1/decisions.md → "- 2026-04-27: Implementing JWT auth"

[Claude Code works on auth module]

User: Take a break

Claude Code:
1. Writes to /context/L0/current → "Paused: Need to test JWT implementation"
2. Writes to /context/L1/decisions.md → "- 2026-04-27: JWT implementation complete, testing pending"

[Session ends, Claude Code exits]

Next session:
User: Continue

Claude Code:
1. Reads /context/L0/current → "Paused: Need to test JWT implementation"
2. Reads /context/L1/decisions.md → sees all previous decisions
3. Continues from exactly where left off
```

## 3. Codex Integration

### 3.1 Architecture

Codex (OpenAI's CLI agent) can use EVIF via Python SDK.

### 3.2 Setup

```python
# codex_evif_plugin.py
import os
from evif import Client

class EVIFPlugin:
    def __init__(self, endpoint="http://localhost:8081"):
        self.endpoint = endpoint
        self.client = None

    def on_start(self):
        """Called when Codex session starts."""
        try:
            self.client = Client(self.endpoint)
            # Verify connection
            if self.client.health().status == "healthy":
                self.restore_context()
        except Exception as e:
            print(f"EVIF not available: {e}")
            self.client = None

    def on_exit(self):
        """Called when Codex session ends."""
        if self.client:
            self.save_context()

    def restore_context(self):
        """Restore session context from EVIF."""
        try:
            current = self.client.cat("/context/L0/current")
            if current:
                self.context = current.decode()
        except:
            self.context = ""

    def save_context(self):
        """Save session context to EVIF."""
        if self.client and self.context:
            self.client.write("/context/L0/current", self.context)

    def check_health(self):
        """Check if EVIF is available."""
        if not self.client:
            return False
        try:
            return self.client.health().status == "healthy"
        except:
            return False
```

### 3.3 Usage

```python
# In your Codex prompt or plugin
from codex_evif_plugin import EVIFPlugin

evif = EVIFPlugin()

# Check if EVIF is available
if evif.check_health():
    # Use EVIF for context
    current_task = evif.context
    print(f"Continuing: {current_task}")
else:
    print("EVIF not available")

# ... do work ...

# Save progress
evif.context = "Completed auth module, moving to tests"
```

### 3.4 Codex Configuration

Add to `~/.codex/config.json`:

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

## 4. OpenClaw Integration

### 4.1 Architecture

OpenClaw uses EVIF's PipeFS for multi-agent coordination.

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Manager       │     │   Worker A     │     │   Worker B     │
│   Agent         │     │                 │     │                 │
│                 │     │                 │     │                 │
│ • Create pipes  │     │ • Poll pipes   │     │ • Poll pipes   │
│ • Assign tasks  │     │ • Execute       │     │ • Execute       │
│ • Aggregate     │     │ • Report result │     │ • Report result │
└────────┬────────┘     └────────┬────────┘     └────────┬────────┘
         │                       │                        │
         │     ┌─────────────────┼────────────────────────┤
         │     │                 │                        │
         ▼     ▼                 ▼                        ▼
┌─────────────────────────────────────────────────────────────────┐
│                        EVIF PipeFS                               │
│  /pipes/review-pr-123/                                          │
│  ├── input: "Review authentication module"                       │
│  ├── assignee: "worker-a"                                       │
│  ├── status: "running"                                          │
│  └── output: "Found 2 issues: SQL injection, weak password"     │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 Worker Implementation

```python
from evif import Client
from openclaw import Agent, Task
import asyncio

class EVIFWorker(Agent):
    """OpenClaw worker that uses EVIF for task coordination."""

    def __init__(self, name: str, endpoint="http://localhost:8081"):
        super().__init__(name)
        self.evif = Client(endpoint)

    async def poll_tasks(self, queue_name: str, interval: int = 5):
        """Poll queue for pending tasks."""
        while True:
            try:
                # Check for pending pipes
                entries = self.evif.ls("/pipes")
                for entry in entries:
                    if entry.is_dir:
                        pipe_name = entry.name
                        status = self.evif.cat(f"/pipes/{pipe_name}/status")
                        assignee = self.evif.cat(f"/pipes/{pipe_name}/assignee")

                        if status == "pending" and not assignee:
                            # Claim and process task
                            await self.process_task(pipe_name)
            except Exception as e:
                print(f"Poll error: {e}")

            await asyncio.sleep(interval)

    async def process_task(self, pipe_name: str):
        """Process a single task from a pipe."""
        # Claim task
        self.evif.write(f"/pipes/{pipe_name}/assignee", self.name)
        self.evif.write(f"/pipes/{pipe_name}/status", "running")

        # Get input
        input_data = self.evif.cat(f"/pipes/{pipe_name}/input")
        if isinstance(input_data, bytes):
            input_data = input_data.decode()

        # Process
        result = await self.execute_task(input_data)

        # Write result
        self.evif.write(f"/pipes/{pipe_name}/output", result)
        self.evif.write(f"/pipes/{pipe_name}/status", "complete")

        # Store in memory
        self.evif.memory_store(
            f"Completed {pipe_name}: {result[:100]}",
            modality="event"
        )

    async def execute_task(self, task_input: str) -> str:
        """Execute the actual task. Override in subclass."""
        # Default implementation - just echo
        return f"[{self.name}] processed: {task_input}"


# Run worker
async def main():
    worker = EVIFWorker("worker-1")
    await worker.poll_tasks("/pipes")

asyncio.run(main())
```

### 4.3 Manager Implementation

```python
from evif import Client

class EVIFManager:
    """OpenClaw manager that creates tasks via EVIF."""

    def __init__(self, endpoint="http://localhost:8081"):
        self.evif = Client(endpoint)

    def create_task(self, name: str, input_data: str, timeout: int = 3600):
        """Create a new task pipe."""
        # Create pipe directory
        self.evif.mkdir(f"/pipes/{name}")

        # Write task input
        self.evif.write(f"/pipes/{name}/input", input_data)

        # Set timeout
        self.evif.write(f"/pipes/{name}/timeout", str(timeout))

        # Initial status
        self.evif.write(f"/pipes/{name}/status", "pending")

        return name

    def get_result(self, name: str, timeout: int = 300) -> str:
        """Wait for and return task result."""
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
                return f"Error: {self.evif.cat(f'/pipes/{name}/output')}"

            time.sleep(1)

        return "Timeout"

    def create_and_wait(self, name: str, input_data: str) -> str:
        """Create task and wait for result."""
        self.create_task(name, input_data)
        return self.get_result(name)


# Usage
manager = EVIFManager()
result = manager.create_and_wait(
    "review-auth-123",
    "Review src/auth/login.rs for security issues"
)
print(result)
```

### 4.4 OpenClaw Configuration

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

## 5. Skills for Agent Platforms

### 5.1 Standard Skill Format

```yaml
---
name: code-review
description: "Review code for bugs and security issues"
triggers:
  - "review code"
  - "code review"
  - "check my code"
version: "1.0"
---

# Code Review Skill

## Description
This skill performs a comprehensive code review, identifying:
- Security vulnerabilities
- Performance issues
- Code quality problems
- Best practice violations

## Usage
1. Write input to `/skills/code-review/input`
2. Skill executes
3. Read output from `/skills/code-review/output`

## Example
```
evif write -c "Review src/auth/login.rs" /skills/code-review/input
evif cat /skills/code-review/output
```
```

### 5.2 Built-in Skills

| Skill | Path | Purpose |
|-------|------|---------|
| code-review | /skills/code-review | Security and quality review |
| test-gen | /skills/test-gen | Generate test cases |
| doc-gen | /skills/doc-gen | Generate documentation |
| refactor | /skills/refactor | Code refactoring suggestions |
| security-audit | /skills/security-audit | Security vulnerability scan |

### 5.3 Creating Custom Skills

```bash
# Create skill directory
evif mkdir /skills/my-skill

# Write SKILL.md
evif write -c '---
name: my-skill
description: "My custom skill"
triggers:
  - "do my task"
---
# My Skill

This skill does X, Y, Z.

## Steps
1. Step one
2. Step two
3. Return result
' /skills/my-skill/SKILL.md
```

## 6. Memory Patterns

### 6.1 Storing Agent Knowledge

```python
# Store learned information
client.memory_store(
    content="The auth module uses JWT tokens with 1-hour expiry",
    modality="knowledge",
    metadata={
        "module": "auth",
        "type": "implementation",
        "confidence": "high"
    }
)

# Store user preferences
client.memory_store(
    content="User prefers TypeScript over JavaScript",
    modality="preference",
    metadata={
        "category": "language",
        "preference": "typescript"
    }
)

# Store important events
client.memory_store(
    content="Deployed v2.3.0 to production",
    modality="event",
    metadata={
        "version": "2.3.0",
        "environment": "production"
    }
)
```

### 6.2 Retrieving Knowledge

```python
# Search relevant knowledge
results = client.memory_search("authentication JWT token")
for r in results:
    if r.get('score', 0) > 0.8:
        print(f"{r['score']:.2f}: {r['content']}")

# List all memories of a type
preferences = client.memory_list(modality="preference")
```

## 7. Complete Example: Multi-Agent Code Review

### 7.1 Setup

```bash
# Start EVIF
evif start

# Create review pipe
evif mkdir /pipes/pr-review-123
```

### 7.2 Manager Agent

```python
from evif import Client

manager = Client("http://localhost:8081")

# Create review task
manager.write("/pipes/pr-review-123/input",
    "Review PR #123:\n"
    "- Files: src/auth/login.rs, src/auth/register.rs\n"
    "- Focus: SQL injection, XSS, authentication bypass"
)
manager.write("/pipes/pr-review-123/status", "pending")

print("Review task created. Workers will pick it up.")
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

                # Check if pending
                try:
                    status = worker.cat(f"/pipes/{name}/status")
                    if isinstance(status, bytes):
                        status = status.decode().strip()

                    if status == "pending":
                        # Claim task
                        worker.write(f"/pipes/{name}/assignee", "worker-1")
                        worker.write(f"/pipes/{name}/status", "running")

                        # Process
                        task = worker.cat(f"/pipes/{name}/input")
                        result = perform_review(task)

                        # Complete
                        worker.write(f"/pipes/{name}/output", result)
                        worker.write(f"/pipes/{name}/status", "complete")

                        print(f"Completed: {name}")
                except:
                    pass

        time.sleep(5)

def perform_review(task):
    # Actual review logic
    return "Found 2 issues:\n1. SQL injection in login\n2. Missing CSRF token"

poll_for_tasks()
```

### 7.4 Get Results

```bash
# Manager checks result
evif cat /pipes/pr-review-123/status
evif cat /pipes/pr-review-123/output
```

## 8. Related Documents

- [Architecture Overview](00-overview.md)
- [SDK Integration](04-sdk-integration.md)
- [REST API Reference](03-rest-api.md)
- [CLI Reference](cli-mode.md)
- [MCP Server Setup](mcp-server.md)
