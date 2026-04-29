# EVIF Examples

This directory contains working examples demonstrating EVIF core functionality.

## Directory Structure

| Path | Description |
|------|-------------|
| `wasm-plugin/` | WASM plugin using Extism PDK |
| `python-sdk/` | Python SDK usage examples |
| `agent-workflow/` | Multi-agent coordination demos |

## Quick Examples

### In-Memory Filesystem

```bash
evif mkdir /mem/demo
evif write /mem/demo/data.txt -c "Hello EVIF"
evif cat /mem/demo/data.txt
```

### Cloud Storage

```bash
# Mount S3 bucket
evif mount s3fs /s3fs --bucket my-bucket

# Upload file
evif write /s3fs/data.json -c '{"key": "value"}'
```

### Agent Context

```bash
# Set current task
evif write /context/L0/current -c "Implement JWT authentication"

# Add decisions
evif write /context/L1/decisions.md -c "- Use HS256 signing\n- Validate expiry"

# List skills
evif ls /skills
```

### Task Queue

```bash
evif mkdir /queue/tasks
echo '{"type": "review", "data": "PR #42"}' | evif write /queue/tasks/enqueue -
evif cat /queue/tasks/dequeue
```

## See Also

- [docs/GETTING_STARTED.md](../docs/GETTING_STARTED.md) - Quick start guide
- [docs/05-agent-integration.md](../docs/05-agent-integration.md) - Agent integration
- [crates/evif-python/README.md](../crates/evif-python/README.md) - Python SDK
