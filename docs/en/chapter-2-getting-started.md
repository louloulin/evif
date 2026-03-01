# Chapter 2: Getting Started

Welcome to EVIF (Extensible Virtual File System)! This chapter will guide you through installation, configuration, and basic usage.

## Table of Contents

- [System Requirements](#system-requirements)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Basic Usage](#basic-usage)
- [Configuration](#configuration)
- [Troubleshooting](#troubleshooting)

## System Requirements

### Minimum Requirements

- **Operating System**: Linux or macOS (Windows support is under development)
- **Rust Version**: 1.70 or higher (for building from source)
- **Memory**: At least 512MB available memory
- **Disk Space**: At least 100MB for build and installation

### Optional Dependencies

- **FUSE**: For FUSE mounting functionality (Linux: `libfuse-dev`, macOS: FUSE for macOS)
- **Python 3.8+**: For Python bindings
- **Docker**: For containerized deployment

## Installation

### Method 1: Build from Source

**1. Clone the Repository**

```bash
git clone https://github.com/evif/evif.git
cd evif
```

**2. Build All Components**

```bash
# Build the entire workspace
cargo build --release

# This will compile all 19 crates, including:
# - evif-core (core abstractions)
# - evif-plugins (plugin collection)
# - evif-rest (REST API server)
# - evif-cli (command-line tool)
# - evif-fuse (FUSE integration)
# and more...
```

**3. Install CLI Tool**

```bash
# Install EVIF CLI
cargo install --path crates/evif-cli

# Verify installation
evif --version
```

**4. Install REST Server**

```bash
# Install EVIF REST server
cargo install --path crates/evif-rest

# Verify installation
evif-rest --version
```

**5. (Optional) Install FUSE Support**

```bash
# Install EVIF FUSE
cargo install --path crates/evif-fuse

# Verify installation
evif-fuse --version
```

### Method 2: Using Pre-built Binaries

**Linux**

```bash
# Download the latest version
wget https://github.com/evif/evif/releases/latest/download/evif-linux-amd64.tar.gz

# Extract
tar -xzf evif-linux-amd64.tar.gz

# Install
sudo cp evif /usr/local/bin/
sudo cp evif-rest /usr/local/bin/
```

**macOS**

```bash
# Using Homebrew
brew install evif

# Or download manually
wget https://github.com/evif/evif/releases/latest/download/evif-darwin-amd64.tar.gz
tar -xzf evif-darwin-amd64.tar.gz
sudo cp evif /usr/local/bin/
sudo cp evif-rest /usr/local/bin/
```

### Method 3: Using Docker

```bash
# Pull EVIF image
docker pull evif/evif:latest

# Run REST server
docker run -p 8081:8081 evif/evif:latest

# Or use docker-compose
docker-compose up -d
```

## Quick Start

### 1. Start the REST Server

**Basic Start**

```bash
# Start with default configuration (port 8081)
evif-rest

# Output:
# [2026-03-01T12:00:00Z INFO  evif_rest] Starting EVIF REST server
# [2026-03-01T12:00:00Z INFO  evif_rest] Mounting memfs at /mem
# [2026-03-01T12:00:00Z INFO  evif_rest] Mounting hellofs at /hello
# [2026-03-01T12:00:00Z INFO  evif_rest] Mounting localfs at /local with root: /tmp
# [2026-03-01T12:00:00Z INFO  evif_rest] Server listening on 0.0.0.0:8081
```

**Custom Configuration**

```bash
# Specify port
evif-rest --port 3000

# Specify bind address
evif-rest --host 127.0.0.1 --port 3000

# Enable debug logging
RUST_LOG=debug evif-rest
```

**Verify Service**

```bash
# Check health status
curl http://localhost:8081/health

# Response:
# {
#   "status": "healthy",
#   "timestamp": "2026-03-01T12:00:00.000000Z",
#   "version": "0.1.0"
# }

# List all mount points
curl http://localhost:8081/api/v1/mounts

# Response:
# {
#   "mounts": [
#     {"path": "/mem", "plugin": "memfs"},
#     {"path": "/hello", "plugin": "hellofs"},
#     {"path": "/local", "plugin": "localfs"}
#   ]
# }
```

### 2. Using the CLI Tool

**Start REPL Mode**

```bash
# Start interactive shell
evif

# Output:
# EVIF CLI v0.1.0
# Type 'help' for available commands
# Connected to http://localhost:8081
#
# evif> _
```

**Basic File Operations**

```bash
# Execute in REPL
evif> ls /
# => mem  hello  local

evif> ls /mem
# => (empty directory)

evif> create /mem/test.txt "Hello, EVIF!"
# => File created: /mem/test.txt

evif> read /mem/test.txt
# => Hello, EVIF!

evif> stat /mem/test.txt
# => {
#   "path": "/mem/test.txt",
#   "size": 13,
#   "modified": "2026-03-01T12:05:00Z",
#   "is_file": true
# }

evif> delete /mem/test.txt
# => File deleted: /mem/test.txt
```

**Batch Mode**

```bash
# Execute commands directly (without entering REPL)
evif ls /mem
evif create /mem/demo.txt "Quick demo"
evif read /mem/demo.txt

# Pipe operations
echo "Content from stdin" | evif write /mem/from_stdin.txt
```

### 3. Using REST API

**Create File**

```bash
# Create file using curl (Base64 encoded content)
curl -X POST http://localhost:8081/api/v1/files \
  -H "Content-Type: application/json" \
  -d '{
    "path": "/mem/hello.txt",
    "content": "SGVsbG8gRVZJRg=="  # "Hello EVIF" in Base64
  }'

# Response:
# {
#   "success": true,
#   "path": "/mem/hello.txt"
# }
```

**Read File**

```bash
# Read file
curl "http://localhost:8081/api/v1/files?path=/mem/hello.txt"

# Response (Base64 encoded):
# {
#   "path": "/mem/hello.txt",
#   "content": "SGVsbG8gRVZJRg==",
#   "size": 10
# }

# Decode content (Linux/macOS)
curl "http://localhost:8081/api/v1/files?path=/mem/hello.txt" \
  | jq -r '.content' \
  | base64 -d

# => Hello EVIF
```

**List Directory**

```bash
# List directory contents
curl "http://localhost:8081/api/v1/directories?path=/mem"

# Response:
# {
#   "path": "/mem",
#   "entries": [
#     {"name": "hello.txt", "is_file": true, "size": 10}
#   ]
# }
```

**Delete File**

```bash
curl -X DELETE "http://localhost:8081/api/v1/files?path=/mem/hello.txt"

# Response:
# {
#   "success": true
# }
```

## Basic Usage

### Mount Management

**View Mount Points**

```bash
# CLI
evif ls_mounts

# REST API
curl http://localhost:8081/api/v1/mounts
```

**Mount New Filesystem**

```bash
# CLI: Mount local filesystem
evif mount localfs /mydata --root /Users/username/data

# CLI: Mount memory filesystem
evif mount memfs /temp

# CLI: Mount S3 (requires AWS credentials configuration)
evif mount s3fs /mybucket --bucket my-bucket --region us-east-1
```

**Unmount Filesystem**

```bash
# CLI
evif umount /mydata

# REST API
curl -X POST http://localhost:8081/api/v1/unmount \
  -H "Content-Type: application/json" \
  -d '{"path": "/mydata"}'
```

### Advanced CLI Features

**Script Execution**

Create script file `setup.evif`:

```bash
# setup.evif
set DATA_DIR=/tmp/myproject
mount localfs /project --root $DATA_DIR
create /project/config.json '{"name":"demo","version":"1.0"}'
create /project/README.md '# My Project\n\nThis is a demo.'
ls /project
```

Execute script:

```bash
evif source setup.evif
```

**Variables and Environment**

```bash
# Set variables
evif set BUCKET=my-bucket
evif set REGION=us-east-1

# Use variables
evif mount s3fs /s3 --bucket $BUCKET --region $REGION
```

**Batch Operations**

```bash
# Create multiple files
evif batch <<EOF
create /mem/file1.txt "Content 1"
create /mem/file2.txt "Content 2"
create /mem/file3.txt "Content 3"
ls /mem
EOF
```

### WebSocket Terminal

EVIF provides WebSocket terminal for real-time interaction:

```javascript
// Connect using JavaScript
const ws = new WebSocket('ws://localhost:8081/ws');

ws.onopen = () => {
  console.log('Connected to EVIF WebSocket');

  // Send command
  ws.send(JSON.stringify({
    command: 'ls',
    args: ['/mem']
  }));
};

ws.onmessage = (event) => {
  const response = JSON.parse(event.data);
  console.log('Response:', response);
};
```

## Configuration

### Environment Variables

**Log Level**

```bash
# Set log level
export RUST_LOG=info     # default
export RUST_LOG=debug    # verbose debug information
export RUST_LOG=warn     # warnings and errors only
export RUST_LOG=error    # errors only

evif-rest
```

**Server Configuration**

```bash
# Server address
export EVIF_HOST=0.0.0.0
export EVIF_PORT=8081

# Worker threads
export EVIF_WORKERS=4

# Cache size (bytes)
export EVIF_CACHE_SIZE=104857600  # 100MB
```

**Plugin Configuration**

```bash
# AWS S3
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
export AWS_DEFAULT_REGION=us-east-1

# Azure Blob Storage
export AZURE_STORAGE_ACCOUNT=your_account
export AZURE_STORAGE_KEY=your_key

# Google Cloud Storage
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
```

### Configuration File (Future Feature)

Configuration file support is under development, will support:

```toml
# evif.toml (planned)
[server]
host = "0.0.0.0"
port = 8081
workers = 4

[mounts "/mem"]
plugin = "memfs"

[mounts "/s3"]
plugin = "s3fs"
bucket = "my-bucket"
region = "us-east-1"

[cache]
enabled = true
size_mb = 100
```

## Troubleshooting

### Port Already in Use

**Error Message**

```
Error: Os { code: 48, kind: AddrInUse, message: "Address already in use" }
```

**Solutions**

```bash
# Find process using the port
lsof -i :8081

# Or
netstat -tulpn | grep 8081

# Solution 1: Kill the process
kill -9 <PID>

# Solution 2: Use a different port
evif-rest --port 3000
```

### Permission Errors

**Error Message**

```
Error: Permission denied (os error 13)
```

**Solutions**

```bash
# Ensure directory access permissions
chmod +x /path/to/directory

# Or run with privileged user (not recommended for production)
sudo evif-rest
```

### FUSE Mount Failure

**Error Message**

```
Error: Failed to mount filesystem: Operation not permitted (os error 1)
```

**Solutions**

```bash
# Linux: Ensure user is in fuse group
sudo usermod -a -G fuse $USER
# Requires re-login to take effect

# macOS: Install FUSE for macOS
brew install --cask macfuse

# Verify FUSE is available
fusermount --version  # Linux
```

### Out of Memory

**Symptoms**

Slow response or service crashes.

**Solutions**

```bash
# Reduce cache size limit
export EVIF_CACHE_SIZE=52428800  # 50MB

# Or disable cache
export EVIF_CACHE_ENABLED=false

# Adjust Tokio runtime
export TOKIO_WORKER_THREADS=2
```

### Plugin Loading Failure

**Error Message**

```
Error: Failed to load plugin: Cannot open library
```

**Solutions**

```bash
# Verify dynamic library path
ls -l /path/to/plugin.so

# Check library dependencies
ldd /path/to/plugin.so  # Linux
otool -L /path/to/plugin.dylib  # macOS

# Ensure correct ABI version
nm /path/to/plugin.so | grep evif_plugin
```

### Getting Help

If the problem persists:

1. **Check Logs**
   ```bash
   RUST_LOG=debug evif-rest 2>&1 | tee evif.log
   ```

2. **Verify Versions**
   ```bash
   evif --version
   evif-rest --version
   ```

3. **Community Resources**
   - GitHub Issues: https://github.com/evif/evif/issues
   - Documentation: https://docs.rs/evif
   - Example Code: `examples/` directory

## Next Steps

Congratulations! You've completed the EVIF quick start. Here are some recommended learning paths:

- **📖 Architecture**: Read [Chapter 3: Architecture](chapter-3-architecture.md) to understand system design
- **🔌 Plugin Development**: See [Chapter 5: Plugin Development](chapter-5-plugin-development.md) to create custom plugins
- **📡 FUSE Integration**: Learn [Chapter 6: FUSE Integration](chapter-6-fuse-integration.md) to mount filesystems
- **📘 API Reference**: Browse [Chapter 7: API Reference](chapter-7-api-reference.md) for complete API documentation
- **🛡️ Security Guide**: Review [Chapter 8: Authentication & Security](chapter-8-authentication-security.md)

**Explore Example Code**

```bash
# View example projects
ls examples/

# Run examples
cargo run --example basic_usage
cargo run --example plugin_demo
```

**Join the Community**

- 🐛 Report Issues: https://github.com/evif/evif/issues
- 💬 Discussions: https://github.com/evif/evif/discussions
- 🔧 Contribute: Pull Requests welcome!
