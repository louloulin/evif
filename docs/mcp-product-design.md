# EVIF MCP Server Product Design

> Status: MVP 6.0 - Ready for Production  
> Purpose: Define MCP as a first-class product surface for AI Agent integration

## Overview

The EVIF MCP Server provides AI agents with a unified tool interface for:
- Context management (L0/L1/L2)
- Skill discovery and invocation
- Pipe coordination (multi-agent)
- Memory retrieval (semantic search)
- Essential filesystem operations
- Cost-optimized responses

## MCP Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     MCP Client (Claude Code, etc.)          │
└─────────────────────┬───────────────────────────────────────┘
                      │ JSON-RPC 2.0
┌─────────────────────▼───────────────────────────────────────┐
│                   EVIF MCP Server                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │ Tool Router │──│ Tool Cache  │──│ Output Filter       │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              VfsBackend (L1: VFS, L2: HTTP, L3: Mock)   │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────┬───────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                   EVIF REST API / Core                       │
└─────────────────────────────────────────────────────────────┘
```

## Core Tool Set (Tier 1 - Primary)

These tools form the MVP core and are recommended for all agent integrations:

### Context Tools
| Tool | Purpose | Cost Optimization |
|------|---------|-------------------|
| `evif_cat` | Read context layer | `max_lines` default 100 |
| `evif_write` | Update context | Bounded write |

### Skill Tools
| Tool | Purpose | Cost Optimization |
|------|---------|-------------------|
| `evif_ls` | List skills | Compact output |
| `evif_cat` | Read skill file | Triggers + summary |

### Pipe Tools
| Tool | Purpose | Cost Optimization |
|------|---------|-------------------|
| `evif_mkdir` | Create pipe | N/A |
| `evif_write` | Write task | Bounded write |
| `evif_read` | Read result | Bounded read |

### Memory Tools
| Tool | Purpose | Cost Optimization |
|------|---------|-------------------|
| `evif_memorize` | Store memory | Async |
| `evif_search` | Semantic search | `compact` default true, `limit` default 3 |

### Filesystem Tools
| Tool | Purpose | Cost Optimization |
|------|---------|-------------------|
| `evif_ls` | List directory | Bounded output |
| `evif_cat` | Read file | `max_lines`, `mode` (head/tail/snippet) |
| `evif_write` | Write file | Bounded write |
| `evif_stat` | File info | Lightweight |

## Extended Tool Set (Tier 2)

These tools are available for advanced use cases:

- `evif_tree` - Directory tree visualization
- `evif_grep` - Search within files
- `evif_diff` - Compare files
- `evif_cp` / `evif_mv` / `evif_rm` - File operations
- `evif_du` - Disk usage
- `evif_hash` - File hashing

## Output Filter Configuration

Default filter settings for agent-friendly output:

```toml
[output_filter]
strip_ansi = true          # Remove color codes
max_lines = 200            # Max lines per response
compact_json = true        # Remove null/empty fields
max_string_length = 10000 # Max string size
```

## Tool Schema Example

```rust
// Example: evif_cat tool schema
Tool {
    name: "evif_cat".into(),
    description: "Read file content with bounded output".into(),
    input_schema: json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string",
                "description": "Path to file"
            },
            "max_lines": {
                "type": "integer",
                "description": "Maximum lines to return (default: 100)",
                "default": 100
            },
            "mode": {
                "type": "string",
                "enum": ["full", "head", "tail", "snippet"],
                "default": "head"
            }
        },
        "required": ["path"]
    })
}
```

## Cache Strategy

### Tool Call Cache
- Cache size: 100 entries (configurable)
- TTL: 5 minutes
- Eviction: LRU

### Prompt Cache
- Cache compiled prompts
- Reduce token overhead for repeated patterns

## Error Handling

| Error Type | MCP Code | Agent Guidance |
|------------|----------|----------------|
| Not Found | -32602 | "File not found, check path" |
| Permission Denied | -32603 | "Access denied, check permissions" |
| Rate Limited | -32604 | "Too many requests, retry after delay" |
| Server Error | -32605 | "Server error, try again" |

## Configuration

### CLI Options
```bash
evif-mcp --port 3000           # Server port
evif-mcp --evif-url http://localhost:8081  # Backend URL
evif-mcp --cache-size 100     # Tool cache size
evif-mcp --verbose            # Debug logging
```

### Environment Variables
```bash
EVIF_MCP_PORT=3000
EVIF_MCP_BACKEND_URL=http://localhost:8081
EVIF_API_KEY=your-api-key
```

## Integration with Claude Code

### Setup via settings.json
```json
{
  "mcpServers": {
    "evif": {
      "command": "cargo",
      "args": ["run", "-p", "evif-mcp"],
      "env": {
        "EVIF_MCP_BACKEND_URL": "http://localhost:8081"
      }
    }
  }
}
```

### CLAUDE.md Integration
```markdown
## EVIF Context
- L0: /context/L0/current (current task)
- L1: /context/L1/decisions.md (session decisions)
- L2: /context/L2/ (project knowledge)

## Skills
Skills are in .claude/skills/*.SKILL.md
Use 'evif ls /skills' to discover available skills.

## Pipes
Use /pipes/{name}/input and /pipes/{name}/output for multi-agent coordination.
```

## Performance Benchmarks

| Operation | Latency (p50) | Latency (p99) |
|-----------|---------------|---------------|
| evif_ls | 5ms | 20ms |
| evif_cat (small) | 10ms | 50ms |
| evif_cat (bounded) | 15ms | 80ms |
| evif_memorize | 50ms | 200ms |
| evif_search | 100ms | 500ms |

## Future Enhancements

1. **Streaming Support** - For large file responses
2. **Subscription Model** - Real-time updates for pipe status
3. **Batch Operations** - Multiple tool calls in single request
4. **Metrics Endpoint** - `/metrics` for monitoring

## Testing

```bash
# Unit tests
cargo test -p evif-mcp

# Integration tests
cargo test -p evif-rest --test mcp_integration

# E2E with Claude Code
./scripts/verify_skills_mcp.sh
```