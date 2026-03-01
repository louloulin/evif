# EVIF Function Verification Test Specification

## Summary
Automated command-based testing suite for EVIF virtual filesystem that validates 68+ CLI commands, 66+ REST API endpoints, and 30+ plugins using real command execution (curl, evif CLI) rather than unit tests.

## Given-When-Then Acceptance Criteria

### CLI Command Tests

**GIVEN** EVIF server is running on localhost:8080
**WHEN** executing P0 file operation commands
**THEN** all commands return success with correct output:

| Command | Input | Expected Output |
|---------|-------|-----------------|
| `ls /` | path=/ | List root directory contents |
| `ls -l /` | -l flag | Long format with permissions, size, date |
| `ls -r /` | -r flag | Recursive listing of all subdirs |
| `cat /test.txt` | file path | Full file content output |
| `write /new.txt -c "hello"` | path + content | File created, content correct |
| `write /new.txt -c "world" -a` | append flag | Content appended to end |
| `mkdir /newdir` | directory path | Directory created |
| `mkdir /a/b/c -p` | -p flag | Parent dirs auto-created |
| `rm /file.txt` | file path | File deleted |
| `rm /dir -r` | -r flag | Directory and contents deleted |
| `mv /src /dst` | src, dst | File moved/renamed |
| `cp /src /dst` | src, dst | File copied, content identical |
| `stat /file.txt` | file path | Return type, size, times, perms |
| `touch /empty.txt` | file path | Empty file created |
| `head /file.txt -n 5` | path + lines | First 5 lines |
| `tail /file.txt -n 5` | path + lines | Last 5 lines |
| `tree / -d 2` | path + depth | Directory tree structure |

**GIVEN** EVIF server running
**WHEN** executing P0 plugin management commands
**THEN** plugin operations succeed:

| Command | Input | Expected Output |
|---------|-------|-----------------|
| `mount memfs /memory` | plugin, path | Plugin mounted at path |
| `mount localfs /local -c '{"root":"/tmp"}'` | config | Mounted with config |
| `unmount /memory` | mount point | Plugin unmounted |
| `mounts` | no args | List all active mounts |

**GIVEN** EVIF server running
**WHEN** executing P0 system commands
**THEN** system status returned:

| Command | Input | Expected Output |
|---------|-------|-----------------|
| `health` | no args | Status, version, uptime |
| `stats` | no args | Connection stats, metrics |
| `repl` | no args | Enter interactive mode |

### REST API Tests

**GIVEN** EVIF REST server on localhost:8080
**WHEN** making HTTP requests to P0 endpoints
**THEN** correct status codes and responses:

| Endpoint | Method | Input | Expected Output |
|----------|--------|-------|-----------------|
| `/health` | GET | - | `{status: "ok"}` |
| `/api/v1/health` | GET | - | `{status, version, uptime}` |
| `/api/v1/files?path=/` | GET | query param | File list JSON |
| `/api/v1/files?path=/f.txt` | PUT | body content | File written, 200 OK |
| `/api/v1/files` | POST | `{path,content}` | File created, 201 Created |
| `/api/v1/files?path=/f.txt` | DELETE | query param | File deleted, 200 OK |
| `/api/v1/directories?path=/` | GET | query param | Directory listing |
| `/api/v1/directories` | POST | `{path}` | Dir created, 201 Created |
| `/api/v1/directories?path=/d` | DELETE | query param | Dir deleted, 200 OK |
| `/api/v1/mounts` | GET | - | Mount list JSON |
| `/api/v1/mount` | POST | `{plugin,path}` | Mounted, 200 OK |
| `/api/v1/unmount` | POST | `{path}` | Unmounted, 200 OK |

**GIVEN** EVIF REST server running
**WHEN** making P1 handle management requests
**THEN** handle operations complete:

| Endpoint | Method | Input | Expected Output |
|----------|--------|-------|-----------------|
| `/api/v1/handles/open` | POST | `{path,mode}` | `{handle_id}` |
| `/api/v1/handles/:id` | GET | handle ID | Handle details |
| `/api/v1/handles/:id/read` | POST | `{offset,size}` | File data chunk |
| `/api/v1/handles/:id/write` | POST | `{data,offset}` | Bytes written |
| `/api/v1/handles/:id/seek` | POST | `{position}` | New position |
| `/api/v1/handles/:id/sync` | POST | - | Synced to storage |
| `/api/v1/handles/:id/close` | POST | - | Handle closed |
| `/api/v1/handles/:id/renew` | POST | `{ttl}` | TTL extended |
| `/api/v1/handles` | GET | - | All handles |
| `/api/v1/handles/stats` | GET | - | Handle statistics |

### Plugin Tests

**GIVEN** EVIF server with plugin system loaded
**WHEN** mounting P0 storage plugins
**THEN** plugins operate correctly:

| Plugin | Test Operations | Expected Result |
|--------|----------------|-----------------|
| `memfs` | write, read, list | All ops succeed, in-memory |
| `localfs` | write, read, list | Ops map to local FS |
| `hellofs` | read file | Returns hello message |

**GIVEN** External cloud credentials available
**WHEN** testing P1 cloud storage plugins
**THEN** cloud operations complete:

| Plugin | Required Config | Test Operations |
|--------|-----------------|-----------------|
| `s3fs` | `{bucket,region,access_key,secret_key}` | List, read, write objects |
| `miniofs` | `{endpoint,bucket,access_key,secret_key}` | S3-compatible ops |
| `azureblobfs` | `{container_name,connection_string}` | Blob read/write |
| `gcsfs` | `{bucket,credentials}` | Object operations |
| `aliyunossfs` | `{bucket,access_key_id,access_key_secret}` | OSS operations |

### Batch Operations

**GIVEN** Multiple files exist in EVIF
**WHEN** executing P1 batch commands
**THEN** batch operations complete:

| Command | Input | Expected Output |
|---------|-------|-----------------|
| `batch-copy` | `{sources,destination}` | All files copied concurrently |
| `batch-delete` | `{paths,recursive}` | All files deleted |
| `batch-progress <id>` | operation ID | Progress percentage |
| `batch-cancel <id>` | operation ID | Operation cancelled |

### Search and Analysis

**GIVEN** Files with searchable content exist
**WHEN** executing P1 search commands
**THEN** search results accurate:

| Command | Input | Expected Output |
|---------|-------|-----------------|
| `grep "pattern" /path` | pattern, path | Matching lines with line numbers |
| `grep -r "pattern" /path` | -r flag | Recursive search results |
| `checksum /file -a md5` | path, algo | MD5 hash value |
| `diff /f1 /f2` | two paths | Unified diff output |
| `du /path -r` | path, -r flag | File count and total size |
| `find /path -n "*.txt"` | path, name pattern | Matching file paths |
| `locate "pattern"` | search pattern | Fast file location results |

### Transfer Commands

**GIVEN** Local files and EVIF server running
**WHEN** executing P1 transfer commands
**THEN** transfers complete:

| Command | Input | Expected Output |
|---------|-------|-----------------|
| `upload /local/path /remote/path` | local, remote | File uploaded to EVIF |
| `download /remote/path /local/path` | remote, local | File downloaded from EVIF |

## Input/Output Examples

### Example 1: Basic File Operations
```bash
# Start server
cargo run -p evif-rest &
SERVER_PID=$!
sleep 5

# Create file
evif write /test.txt -c "Hello EVIF"
# Output: File created: /test.txt

# Read file
evif cat /test.txt
# Output: Hello EVIF

# List directory
evif ls /
# Output: test.txt

# Cleanup
evif rm /test.txt
kill $SERVER_PID
```

### Example 2: REST API with curl
```bash
# Health check
curl http://localhost:8080/health
# Output: {"status":"ok"}

# Create file
curl -X POST http://localhost:8080/api/v1/files \
  -H "Content-Type: application/json" \
  -d '{"path":"/api.txt","content":"API test"}'
# Output: {"path":"/api.txt","size":8}

# Read file
curl http://localhost:8080/api/v1/files?path=/api.txt
# Output: API test

# Delete file
curl -X DELETE "http://localhost:8080/api/v1/files?path=/api.txt"
# Output: {"success":true}
```

### Example 3: Handle Management
```bash
# Open handle
curl -X POST http://localhost:8080/api/v1/handles/open \
  -H "Content-Type: application/json" \
  -d '{"path":"/large.txt","mode":"read"}'
# Output: {"handle_id":"hdl-abc123","path":"/large.txt"}

# Read chunk
curl -X POST http://localhost:8080/api/v1/handles/hdl-abc123/read \
  -H "Content-Type: application/json" \
  -d '{"offset":0,"size":1024}'
# Output: {"data":"...","bytes_read":1024}

# Close handle
curl -X POST http://localhost:8080/api/v1/handles/hdl-abc123/close
# Output: {"success":true}
```

### Example 4: Plugin Mounting
```bash
# Mount memfs
evif mount memfs /memory
# Output: Plugin 'memfs' mounted at /memory

# Write to memfs
evif write /memory/data.txt -c "in-memory data"
# Output: File created: /memory/data.txt

# Verify mount
evif mounts
# Output: /memory -> memfs (config: {})

# Unmount
evif unmount /memory
# Output: Plugin unmounted from /memory
```

### Example 5: Batch Operations
```bash
# Setup test files
for i in {1..5}; do evif write /file$i.txt -c "content $i"; done

# Batch copy
curl -X POST http://localhost:8080/api/v1/batch/copy \
  -H "Content-Type: application/json" \
  -d '{"sources":["/file1.txt","/file2.txt"],"destination":"/backup/"}'
# Output: {"operation_id":"batch-xyz789","status":"started"}

# Check progress
curl http://localhost:8080/api/v1/batch/progress/batch-xyz789
# Output: {"operation_id":"batch-xyz789","progress":100,"status":"completed"}
```

## Edge Cases and Error Conditions

### Error Scenarios

| Scenario | Input | Expected Behavior |
|----------|-------|-------------------|
| File not found | `cat /nonexistent.txt` | Error: file not found, non-zero exit |
| Permission denied | `rm /readonly/file.txt` | Error: permission denied |
| Invalid path | `ls /path/../invalid` | Error: invalid path format |
| Directory not empty | `rm /dir` (no -r) | Error: directory not empty |
| Plugin already mounted | `mount memfs /memory` (duplicate) | Error: path already mounted |
| Invalid handle ID | `/api/v1/handles/invalid/read` | Error: handle not found, 404 |
| Handle expired | Read expired handle | Error: handle TTL expired |
| Concurrent write conflict | Multiple writes to same handle | Last write wins, no locking guarantee |
| Network timeout | S3 operation with slow network | Timeout error, retryable |
| Invalid JSON payload | Malformed request body | Error: invalid JSON, 400 Bad Request |
| Missing query param | `/api/v1/files` (no path) | Error: path parameter required, 400 |

### Boundary Conditions

| Condition | Test Case | Expected Behavior |
|-----------|-----------|-------------------|
| Empty directory | `ls /empty` | Return empty list `[]` |
| Large file (>1GB) | `write /large.bin -c <big>` | Success, streaming write |
| Deep path | `mkdir /a/b/c/d/e/f/g` | Success with -p flag |
| Long filename (255 chars) | Create 255-char file | Success |
| Unicode filename | `mkdir /中文目录` | Success, UTF-8 support |
| Zero-byte file | `touch /empty` | Success, size=0 |
| Handle at EOF | Read beyond file size | Return EOF/empty data |
| Concurrent handles | Open 100 handles | All succeed, managed correctly |

### Resource Limits

| Resource | Limit | Test | Expected |
|----------|-------|------|----------|
| File size | 5GB | Write 5GB file | Success |
| Files per dir | 10,000 | Create 10K files | Success (performance degraded) |
| Active handles | 1,000 | Open 1K handles | Success |
| Mount points | 100 | Mount 100 plugins | Success |
| Batch operation | 10,000 files | Batch delete 10K | Success (async) |

## Non-Functional Requirements

### Performance

| Metric | Target | Test Method |
|--------|--------|-------------|
| API P99 latency | <100ms | wrk -t4 -c100 -d30s |
| File operation throughput | >1000 ops/s | Concurrent operations |
| Handle open latency | <10ms | 1000 open calls |
| Directory list speed | <100ms for 1000 files | ls with large dir |
| Batch operation throughput | >500 files/sec | batch-copy with 10K files |
| Memory usage | <500MB | Monitor during 1K handles |
| Concurrent connections | >1000 | wrk with 1000 connections |

### Security

| Requirement | Implementation |
|-------------|----------------|
| Path traversal prevention | Reject paths with `../` |
| Input validation | Validate all JSON schemas |
| Rate limiting | Optional: configurable limits |
| Error messages | No sensitive data exposure |
| Handle security | UUID generation, expiration |
| Plugin isolation | Plugin sandbox (where available) |

### Reliability

| Requirement | Criteria |
|-------------|----------|
| Server uptime | Continuous operation 24h+ |
| Handle cleanup | Auto-close expired handles |
| Resource cleanup | Cleanup on server shutdown |
| Transaction safety | Plugin-dependent, no guarantees |
| Error recovery | Graceful degradation |

## Out of Scope

- **Unit tests**: This spec covers integration/command tests only
- **Graph query endpoints**: P3 feature, not fully implemented (query, get, create, delete)
- **Interactive commands**: REPL mode not testable in automation
- **Cloud provider dependencies**: Only test if credentials provided
- **FUSE mounting**: Requires sudo, OS-specific, separate test suite
- **gRPC endpoints**: Separate protocol, not covered
- **WebSocket endpoints**: Real-time, requires persistent connections
- **WASM plugin loading**: P2 feature, requires WASM runtime
- **Performance regression testing**: Requires baseline metrics

## Test Organization

### Test Categories

| Category | Tests | Priority |
|----------|-------|----------|
| CLI file operations | 16 commands | P0 |
| CLI plugin management | 4 commands | P0 |
| CLI system commands | 3 commands | P0 |
| CLI batch operations | 5 commands | P1 |
| CLI search/analysis | 9 commands | P1 |
| CLI text processing | 9 commands | P2 |
| CLI shell tools | 9 commands | P2 |
| CLI path tools | 8 commands | P2 |
| CLI transfer | 2 commands | P1 |
| CLI other tools | 2 commands | P2 |
| CLI interactive | 4 commands | P2 (skip) |
| REST health | 2 endpoints | P0 |
| REST files | 4 endpoints | P0 |
| REST directories | 3 endpoints | P0 |
| REST metadata | 4 endpoints | P1 |
| REST advanced | 1 endpoint | P1 |
| REST mounts | 3 endpoints | P0 |
| REST plugins | 7 endpoints | P1 |
| REST handles | 10 endpoints | P1 |
| REST batch | 5 endpoints | P1 |
| REST metrics | 4 endpoints | P2 |
| REST collaboration | 9 endpoints | P2 |
| REST compatibility | 5 endpoints | P1 |
| REST graph | 6 endpoints | P3 (skip) |
| REST websocket | 1 endpoint | P2 (skip) |
| Plugins (P0) | 3 plugins | P0 |
| Plugins (P1 cloud) | 9 plugins | P1 |
| Plugins (P1 DB) | 3 plugins | P1 |
| Plugins (P1 network) | 5 plugins | P1 |
| Plugins (P2) | 13 plugins | P2 |

### Test Execution Order

1. **Phase 1: Server startup** (P0)
   - Start EVIF REST server
   - Verify health endpoint
   - Run 10 startup checks

2. **Phase 2: CLI P0 tests** (P0)
   - File operations (16 tests)
   - Plugin management (4 tests)
   - System commands (3 tests)
   - Total: 23 tests

3. **Phase 3: REST P0 tests** (P0)
   - Health checks (2 tests)
   - File operations (4 tests)
   - Directory operations (3 tests)
   - Mount management (3 tests)
   - Total: 12 tests

4. **Phase 4: Plugin P0 tests** (P0)
   - memfs, localfs, hellofs
   - Total: 3 tests

5. **Phase 5: Handle management** (P1)
   - 10 REST endpoints
   - Total: 10 tests

6. **Phase 6: Batch operations** (P1)
   - 5 CLI commands
   - 5 REST endpoints
   - Total: 10 tests

7. **Phase 7: Search and analysis** (P1)
   - 9 CLI commands
   - 1 REST endpoint
   - Total: 10 tests

8. **Phase 8: Transfer commands** (P1)
   - 2 CLI commands
   - Total: 2 tests

9. **Phase 9: P1 features** (P1)
   - REST metadata (4 tests)
   - REST plugins (7 tests)
   - REST compatibility (5 tests)
   - Total: 16 tests

**Total P0 tests: 48**
**Total P0+P1 tests: 96**

## Test Output Format

### Success Case
```
✓ PASS: ls /path
  Command: evif ls /
  Exit code: 0
  Output: file1.txt  file2.txt  dir1/
  Duration: 45ms
```

### Failure Case
```
✗ FAIL: cat /nonexistent
  Command: evif cat /nonexistent
  Exit code: 1
  Expected: file content
  Actual: Error: file not found
  Duration: 12ms
```

### Summary Report
```
EVIF Test Summary
=================
Total tests: 96
Passed: 94
Failed: 2
Skipped: 0
Duration: 45.3s

P0 tests: 48/48 passed
P1 tests: 46/48 passed

Failures:
  - cat /nonexistent: file not found handling (P1)
  - grep -r pattern: recursive search timeout (P1)
```

## Environment Setup

### Prerequisites
```bash
# Rust toolchain
rustc --version  # >= 1.70
cargo --version

# Optional: wrk for load testing
wrk --version

# Optional: Cloud credentials (skip if not available)
# export AWS_ACCESS_KEY_ID=...
# export AWS_SECRET_ACCESS_KEY=...
```

### Server Startup
```bash
# Build and start
cargo build --release -p evif-rest
cargo run -p evif-rest -- --host 0.0.0.0 --port 8080

# Wait for server
sleep 5

# Verify
curl http://localhost:8080/health
```

### Test Execution
```bash
# Run all tests
./run_tests.sh

# Run P0 only
./run_tests.sh --priority P0

# Run specific category
./run_tests.sh --category cli-files

# Run with verbose output
./run_tests.sh --verbose
```

## Acceptance Criteria Summary

The test suite is considered complete when:

- [ ] All 48 P0 tests pass
- [ ] 90%+ of P0+P1 tests pass (86/96)
- [ ] Test execution time < 60 seconds
- [ ] No memory leaks detected (valgrind/heaptrack clean)
- [ ] Server survives 24h continuous operation
- [ ] Error handling correct for all edge cases
- [ ] Performance metrics meet targets (P99 < 100ms)
- [ ] Test results reproducible across 3 runs
- [ ] Test report generated with pass/fail counts
- [ ] Test artifacts captured (logs, traces)

---

*Specification Version: 1.0*
*Created: 2026-02-26*
*Author: Spec Writer*
*Review Status: Pending Spec Critic review*