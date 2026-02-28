# EVIF MCP Server

Model Context Protocol (MCP) server for EVIF integration with Claude Desktop and other MCP clients.

## Overview

The EVIF MCP server exposes EVIF filesystem operations as MCP tools, allowing Claude Desktop and other MCP clients to interact with EVIF-mountable filesystems directly.

## Installation

### From Source

```bash
cargo install --path crates/evif-mcp
```

### Configuration

Add the following to your Claude Desktop MCP configuration file:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "evif": {
      "command": "evif-mcp",
      "args": [
        "--server-url",
        "http://localhost:8080"
      ],
      "env": {
        "EVIF_URL": "http://localhost:8080",
        "RUST_LOG": "evif_mcp=debug"
      }
    }
  }
}
```

## Available Tools

The MCP server provides the following tools:

### File Operations

- **evif_ls**: List files in a directory
- **evif_cat**: Read file contents
- **evif_write**: Write content to a file
- **evif_mkdir**: Create a directory
- **evif_rm**: Remove a file or directory
- **evif_stat**: Get file information
- **evif_mv**: Move or rename a file
- **evif_cp**: Copy a file

### Plugin Operations

- **evif_mount**: Mount a plugin
- **evif_unmount**: Unmount a plugin
- **evif_mounts**: List all mount points

### Advanced Operations

- **evif_grep**: Search for text in files
- **evif_health**: Check server health
- **evif_open_handle**: Open a file handle
- **evif_close_handle**: Close a file handle

## Usage Examples

### List Files

```json
{
  "tool": "evif_ls",
  "arguments": {
    "path": "/s3/my-bucket"
  }
}
```

### Read File

```json
{
  "tool": "evif_cat",
  "arguments": {
    "path": "/s3/my-bucket/document.txt",
    "offset": 0,
    "size": 1024
  }
}
```

### Write File

```json
{
  "tool": "evif_write",
  "arguments": {
    "path": "/s3/my-bucket/new-file.txt",
    "content": "Hello, World!",
    "offset": -1
  }
}
```

### Mount Plugin

```json
{
  "tool": "evif_mount",
  "arguments": {
    "plugin": "s3fs",
    "path": "/s3",
    "config": {
      "bucket": "my-bucket",
      "region": "us-east-1",
      "access_key": "AKIA...",
      "secret_key": "..."
    }
  }
}
```

## Architecture

```
┌─────────────────┐
│ Claude Desktop  │
│   (MCP Client)  │
└────────┬────────┘
         │ JSON-RPC
         ↓
┌─────────────────┐
│  EVIF MCP       │
│  Server         │
│  (17 Tools)     │
└────────┬────────┘
         │ HTTP REST API
         ↓
┌─────────────────┐
│  EVIF Server    │
│  (16 Plugins)   │
└─────────────────┘
```

## Development

### Running Tests

```bash
cargo test --package evif-mcp
```

### Running with Claude Desktop

1. Start EVIF server:
```bash
evif-server --port 8080
```

2. Configure Claude Desktop (see Configuration above)

3. Restart Claude Desktop

4. Check Claude Desktop logs for MCP connection status

## Troubleshooting

### Connection Issues

- Verify EVIF server is running: `curl http://localhost:8080/health`
- Check MCP server logs in Claude Desktop logs
- Ensure the EVIF URL in configuration matches server address

### Tool Failures

- Check EVIF server logs for errors
- Verify plugins are mounted: use `evif_mounts` tool
- Check file permissions and paths

### Performance

- Enable caching in plugin configuration
- Use appropriate batch sizes for operations
- Consider HandleFS for large file operations

## Related Documentation

- [SKILL.md](../skills/SKILL.md) - Main Agent Skill documentation
- [evif-manage.md](../skills/evif-manage.md) - Plugin management guide
- [evif-s3.md](../skills/evif-s3.md) - S3 plugin best practices
