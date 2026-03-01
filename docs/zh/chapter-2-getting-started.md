# 第二章 快速开始

欢迎使用 EVIF (Extensible Virtual File System)! 本章将引导您完成安装、配置和基本使用。

## 目录

- [系统要求](#系统要求)
- [安装](#安装)
- [快速体验](#快速体验)
- [基本使用](#基本使用)
- [配置](#配置)
- [故障排除](#故障排除)

## 系统要求

### 最低要求

- **操作系统**: Linux 或 macOS (Windows 支持正在开发中)
- **Rust 版本**: 1.70 或更高版本 (用于从源码构建)
- **内存**: 至少 512MB 可用内存
- **磁盘空间**: 至少 100MB 用于构建和安装

### 可选依赖

- **FUSE**: 用于 FUSE 挂载功能 (Linux: `libfuse-dev`, macOS: FUSE for macOS)
- **Python 3.8+**: 用于 Python 绑定
- **Docker**: 用于容器化部署

## 安装

### 方法一: 从源码构建

**1. 克隆仓库**

```bash
git clone https://github.com/evif/evif.git
cd evif
```

**2. 构建所有组件**

```bash
# 构建整个工作空间
cargo build --release

# 这将编译所有 19 个 crates,包括:
# - evif-core (核心抽象)
# - evif-plugins (插件集合)
# - evif-rest (REST API 服务器)
# - evif-cli (命令行工具)
# - evif-fuse (FUSE 集成)
# 等等...
```

**3. 安装 CLI 工具**

```bash
# 安装 EVIF CLI
cargo install --path crates/evif-cli

# 验证安装
evif --version
```

**4. 安装 REST 服务器**

```bash
# 安装 EVIF REST 服务器
cargo install --path crates/evif-rest

# 验证安装
evif-rest --version
```

**5. (可选) 安装 FUSE 支持**

```bash
# 安装 EVIF FUSE
cargo install --path crates/evif-fuse

# 验证安装
evif-fuse --version
```

### 方法二: 使用预编译二进制文件

**Linux**

```bash
# 下载最新版本
wget https://github.com/evif/evif/releases/latest/download/evif-linux-amd64.tar.gz

# 解压
tar -xzf evif-linux-amd64.tar.gz

# 安装
sudo cp evif /usr/local/bin/
sudo cp evif-rest /usr/local/bin/
```

**macOS**

```bash
# 使用 Homebrew
brew install evif

# 或手动下载
wget https://github.com/evif/evif/releases/latest/download/evif-darwin-amd64.tar.gz
tar -xzf evif-darwin-amd64.tar.gz
sudo cp evif /usr/local/bin/
sudo cp evif-rest /usr/local/bin/
```

### 方法三: 使用 Docker

```bash
# 拉取 EVIF 镜像
docker pull evif/evif:latest

# 运行 REST 服务器
docker run -p 8081:8081 evif/evif:latest

# 或使用 docker-compose
docker-compose up -d
```

## 快速体验

### 1. 启动 REST 服务器

**基础启动**

```bash
# 使用默认配置启动 (端口 8081)
evif-rest

# 输出:
# [2026-03-01T12:00:00Z INFO  evif_rest] Starting EVIF REST server
# [2026-03-01T12:00:00Z INFO  evif_rest] Mounting memfs at /mem
# [2026-03-01T12:00:00Z INFO  evif_rest] Mounting hellofs at /hello
# [2026-03-01T12:00:00Z INFO  evif_rest] Mounting localfs at /local with root: /tmp
# [2026-03-01T12:00:00Z INFO  evif_rest] Server listening on 0.0.0.0:8081
```

**自定义配置**

```bash
# 指定端口
evif-rest --port 3000

# 指定绑定地址
evif-rest --host 127.0.0.1 --port 3000

# 启用调试日志
RUST_LOG=debug evif-rest
```

**验证服务**

```bash
# 检查健康状态
curl http://localhost:8081/health

# 响应:
# {
#   "status": "healthy",
#   "timestamp": "2026-03-01T12:00:00.000000Z",
#   "version": "0.1.0"
# }

# 列出所有挂载点
curl http://localhost:8081/api/v1/mounts

# 响应:
# {
#   "mounts": [
#     {"path": "/mem", "plugin": "memfs"},
#     {"path": "/hello", "plugin": "hellofs"},
#     {"path": "/local", "plugin": "localfs"}
#   ]
# }
```

### 2. 使用 CLI 工具

**启动 REPL 模式**

```bash
# 启动交互式 shell
evif

# 输出:
# EVIF CLI v0.1.0
# Type 'help' for available commands
# Connected to http://localhost:8081
#
# evif> _
```

**基本文件操作**

```bash
# 在 REPL 中执行
evif> ls /
# => mem  hello  local

evif> ls /mem
# => (空目录)

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

**批处理模式**

```bash
# 直接执行命令 (不进入 REPL)
evif ls /mem
evif create /mem/demo.txt "Quick demo"
evif read /mem/demo.txt

# 管道操作
echo "Content from stdin" | evif write /mem/from_stdin.txt
```

### 3. 使用 REST API

**创建文件**

```bash
# 使用 curl 创建文件 (Base64 编码内容)
curl -X POST http://localhost:8081/api/v1/files \
  -H "Content-Type: application/json" \
  -d '{
    "path": "/mem/hello.txt",
    "content": "SGVsbG8gRVZJRg=="  # "Hello EVIF" in Base64
  }'

# 响应:
# {
#   "success": true,
#   "path": "/mem/hello.txt"
# }
```

**读取文件**

```bash
# 读取文件
curl "http://localhost:8081/api/v1/files?path=/mem/hello.txt"

# 响应 (Base64 编码):
# {
#   "path": "/mem/hello.txt",
#   "content": "SGVsbG8gRVZJRg==",
#   "size": 10
# }

# 解码内容 (Linux/macOS)
curl "http://localhost:8081/api/v1/files?path=/mem/hello.txt" \
  | jq -r '.content' \
  | base64 -d

# => Hello EVIF
```

**列出目录**

```bash
# 列出目录内容
curl "http://localhost:8081/api/v1/directories?path=/mem"

# 响应:
# {
#   "path": "/mem",
#   "entries": [
#     {"name": "hello.txt", "is_file": true, "size": 10}
#   ]
# }
```

**删除文件**

```bash
curl -X DELETE "http://localhost:8081/api/v1/files?path=/mem/hello.txt"

# 响应:
# {
#   "success": true
# }
```

## 基本使用

### 挂载管理

**查看挂载点**

```bash
# CLI
evif ls_mounts

# REST API
curl http://localhost:8081/api/v1/mounts
```

**挂载新文件系统**

```bash
# CLI: 挂载本地文件系统
evif mount localfs /mydata --root /Users/username/data

# CLI: 挂载内存文件系统
evif mount memfs /temp

# CLI: 挂载 S3 (需要配置 AWS 凭证)
evif mount s3fs /mybucket --bucket my-bucket --region us-east-1
```

**卸载文件系统**

```bash
# CLI
evif umount /mydata

# REST API
curl -X POST http://localhost:8081/api/v1/unmount \
  -H "Content-Type: application/json" \
  -d '{"path": "/mydata"}'
```

### 高级 CLI 功能

**脚本执行**

创建脚本文件 `setup.evif`:

```bash
# setup.evif
set DATA_DIR=/tmp/myproject
mount localfs /project --root $DATA_DIR
create /project/config.json '{"name":"demo","version":"1.0"}'
create /project/README.md '# My Project\n\nThis is a demo.'
ls /project
```

执行脚本:

```bash
evif source setup.evif
```

**变量和环境**

```bash
# 设置变量
evif set BUCKET=my-bucket
evif set REGION=us-east-1

# 使用变量
evif mount s3fs /s3 --bucket $BUCKET --region $REGION
```

**批量操作**

```bash
# 创建多个文件
evif batch <<EOF
create /mem/file1.txt "Content 1"
create /mem/file2.txt "Content 2"
create /mem/file3.txt "Content 3"
ls /mem
EOF
```

### WebSocket 终端

EVIF 提供 WebSocket 终端进行实时交互:

```javascript
// 使用 JavaScript 连接 WebSocket
const ws = new WebSocket('ws://localhost:8081/ws');

ws.onopen = () => {
  console.log('Connected to EVIF WebSocket');

  // 发送命令
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

## 配置

### 环境变量

**日志级别**

```bash
# 设置日志级别
export RUST_LOG=info     # 默认
export RUST_LOG=debug    # 详细调试信息
export RUST_LOG=warn     # 仅警告和错误
export RUST_LOG=error    # 仅错误

evif-rest
```

**服务器配置**

```bash
# 服务器地址
export EVIF_HOST=0.0.0.0
export EVIF_PORT=8081

# 工作线程
export EVIF_WORKERS=4

# 缓存大小 (字节)
export EVIF_CACHE_SIZE=104857600  # 100MB
```

**插件配置**

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

### 配置文件 (未来功能)

配置文件支持正在开发中,未来将支持:

```toml
# evif.toml (规划中)
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

## 故障排除

### 端口已被占用

**错误信息**

```
Error: Os { code: 48, kind: AddrInUse, message: "Address already in use" }
```

**解决方案**

```bash
# 查找占用端口的进程
lsof -i :8081

# 或
netstat -tulpn | grep 8081

# 解决方案 1: 终止占用进程
kill -9 <PID>

# 解决方案 2: 使用其他端口
evif-rest --port 3000
```

### 权限错误

**错误信息**

```
Error: Permission denied (os error 13)
```

**解决方案**

```bash
# 确保有目录访问权限
chmod +x /path/to/directory

# 或使用有权限的用户运行
sudo evif-rest  # 不推荐生产环境
```

### FUSE 挂载失败

**错误信息**

```
Error: Failed to mount filesystem: Operation not permitted (os error 1)
```

**解决方案**

```bash
# Linux: 确保用户在 fuse 组中
sudo usermod -a -G fuse $USER
# 重新登录后生效

# macOS: 安装 FUSE for macOS
brew install --cask macfuse

# 验证 FUSE 可用
fusermount --version  # Linux
```

### 内存不足

**症状**

服务响应缓慢或崩溃。

**解决方案**

```bash
# 增加缓存大小限制
export EVIF_CACHE_SIZE=52428800  # 50MB

# 或禁用缓存
export EVIF_CACHE_ENABLED=false

# 调整 Tokio 运行时
export TOKIO_WORKER_THREADS=2
```

### 插件加载失败

**错误信息**

```
Error: Failed to load plugin: Cannot open library
```

**解决方案**

```bash
# 验证动态库路径
ls -l /path/to/plugin.so

# 检查库依赖
ldd /path/to/plugin.so  # Linux
otool -L /path/to/plugin.dylib  # macOS

# 确保使用正确的 ABI 版本
nm /path/to/plugin.so | grep evif_plugin
```

### 获取帮助

如果问题仍未解决:

1. **查看日志**
   ```bash
   RUST_LOG=debug evif-rest 2>&1 | tee evif.log
   ```

2. **检查版本**
   ```bash
   evif --version
   evif-rest --version
   ```

3. **社区资源**
   - GitHub Issues: https://github.com/evif/evif/issues
   - 文档: https://docs.rs/evif
   - 示例代码: `examples/` 目录

## 下一步

恭喜您已经完成了 EVIF 的快速入门! 以下是一些建议的学习路径:

- **📖 架构理解**: 阅读 [第三章:架构设计](../en/chapter-3-architecture.md) 了解系统设计
- **🔌 插件开发**: 查看 [第五章:插件开发](../en/chapter-5-plugin-development.md) 创建自定义插件
- **📡 FUSE 集成**: 学习 [第六章:FUSE 集成](../en/chapter-6-fuse-integration.md) 挂载文件系统
- **📘 API 参考**: 浏览 [第七章:API 参考](../en/chapter-7-api-reference.md) 了解完整 API
- **🛡️ 安全指南**: 查看 [第八章:认证与安全](../en/chapter-8-authentication-security.md)

**探索示例代码**

```bash
# 查看示例项目
ls examples/

# 运行示例
cargo run --example basic_usage
cargo run --example plugin_demo
```

**参与社区**

- 🐛 报告问题: https://github.com/evif/evif/issues
- 💬 提交讨论: https://github.com/evif/evif/discussions
- 🔧 贡献代码: 欢迎 Pull Request!
