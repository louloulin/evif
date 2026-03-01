# 第九章 部署指南

本章介绍 EVIF (Extensible Virtual File System) 在生产环境中的部署方法、配置管理和运维最佳实践。

## 目录

- [部署概述](#部署概述)
- [前置条件与依赖](#前置条件与依赖)
- [从源码构建](#从源码构建)
- [二进制部署](#二进制部署)
- [Docker 部署](#docker-部署)
- [Docker Compose 栈](#docker-compose-栈)
- [Systemd 服务](#systemd-服务)
- [配置管理](#配置管理)
- [生产环境部署](#生产环境部署)
- [监控与日志](#监控与日志)
- [故障排查](#故障排查)
- [升级与迁移](#升级与迁移)

## 部署概述

### 部署架构

EVIF 采用模块化设计,支持多种部署方式:

```
┌─────────────────────────────────────────┐
│           客户端层                       │
│  ┌──────────┐  ┌──────────┐  ┌────────┐ │
│  │ CLI 工具 │  │ REST API │  │ FUSE  │ │
│  └──────────┘  └──────────┘  └────────┘ │
└─────────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────────┐
│           EVIF 运行时                    │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐   │
│  │ 图引擎 │ │存储层 │ │认证  │ │插件  │   │
│  └──────┘ └──────┘ └──────┘ └──────┘   │
└─────────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────────┐
│           后端服务                       │
│  ┌──────┐  ┌──────┐  ┌──────────┐      │
│  │ Redis│  │本地FS│  │ 云存储S3 │      │
│  └──────┘  └──────┘  └──────────┘      │
└─────────────────────────────────────────┘
```

### 部署模式

| 模式 | 适用场景 | 优势 | 劣势 |
|------|----------|------|------|
| **单机部署** | 开发、测试、小规模应用 | 简单、快速 | 单点故障 |
| **Docker 容器** | 生产环境、微服务架构 | 可移植、易扩展 | 需要 Docker 环境 |
| **Kubernetes** | 大规模、高可用集群 | 自动扩缩容、自愈 | 复杂度高 |
| **Systemd 服务** | Linux 服务器 | 自动重启、日志管理 | 仅限 Linux |

### 核心组件

- **evif-cli**: 命令行工具,用于管理和操作 EVIF
- **evif-rest**: REST API 服务器,提供 HTTP 接口
- **evif-fuse**: FUSE 集成,提供文件系统挂载 (可选)
- **evif-runtime**: 运行时核心,协调所有组件

## 前置条件与依赖

### 系统要求

#### 最低配置

- **操作系统**: Linux (内核 3.10+) 或 macOS (10.15+)
- **CPU**: 1 核心
- **内存**: 512MB RAM
- **磁盘**: 100MB 可用空间
- **网络**: 可选 (用于远程存储和 API 访问)

#### 推荐配置 (生产环境)

- **操作系统**: Linux (内核 5.4+) 或 macOS (12+)
- **CPU**: 2+ 核心
- **内存**: 2GB+ RAM
- **磁盘**: 1GB+ 可用空间 (SSD 推荐)
- **网络**: 1Gbps+ (用于云存储访问)

### 软件依赖

#### 构建依赖

```bash
# Rust 工具链 (1.70+)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 验证安装
rustc --version
cargo --version
```

#### 运行时依赖

**必需依赖**

- **无**: EVIF 核心功能无外部依赖

**可选依赖**

```bash
# FUSE 支持 (用于文件系统挂载)
# Linux
sudo apt-get install libfuse-dev fuse  # Debian/Ubuntu
sudo yum install fuse-devel             # CentOS/RHEL

# macOS
brew install --cask macfuse

# Redis (用于缓存)
sudo apt-get install redis-server  # Debian/Ubuntu
sudo yum install redis              # CentOS/RHEL
brew install redis                  # macOS

# OpenSSL (用于 TLS 支持)
sudo apt-get install libssl-dev  # Debian/Ubuntu
```

#### 云存储依赖 (可选)

```bash
# AWS S3
pip install awscli
aws configure

# Azure Blob Storage
pip install azure-storage-blob

# Google Cloud Storage
pip install google-cloud-storage
```

### 网络要求

#### 端口使用

| 端口 | 协议 | 用途 | 可配置 |
|------|------|------|--------|
| 8081 | HTTP | REST API | 是 |
| 8082 | HTTP | gRPC Web | 是 |
| 6379 | TCP | Redis (可选) | 是 |

#### 防火墙配置

```bash
# Ubuntu/Debian (ufw)
sudo ufw allow 8081/tcp
sudo ufw allow 8082/tcp

# CentOS/RHEL (firewalld)
sudo firewall-cmd --permanent --add-port=8081/tcp
sudo firewall-cmd --permanent --add-port=8082/tcp
sudo firewall-cmd --reload
```

## 从源码构建

### 克隆仓库

```bash
# 克隆主仓库
git clone https://github.com/evif/evif.git
cd evif

# 或使用 SSH
git clone git@github.com:evif/evif.git
cd evif

# 检出特定版本
git checkout v0.1.0
```

### 构建工作空间

EVIF 使用 Cargo 工作空间,包含多个 crates:

```bash
# 查看所有成员
cat Cargo.toml | grep -A 20 "members"

# 构建所有 crates (Debug 模式)
cargo build

# 构建所有 crates (Release 模式,推荐)
cargo build --release

# 仅构建特定 crate
cargo build -p evif-cli --release
cargo build -p evif-rest --release
cargo build -p evif-fuse --release
```

### 编译选项

#### 优化级别

```bash
# Debug 构建 (默认)
cargo build

# Release 构建 (优化)
cargo build --release

# 自定义优化
cargo build --release \
  --profile release-lto  # 启用 LTO
```

#### 功能特性

```bash
# 构建 FUSE 支持
cargo build --release --features fuse

# 构建所有特性
cargo build --release --all-features

# 查看可用特性
cargo build --release --features help
```

### 测试构建

```bash
# 运行所有测试
cargo test --workspace

# 运行特定 crate 测试
cargo test -p evif-graph
cargo test -p evif-storage
cargo test -p evif-auth

# 运行测试并显示输出
cargo test --workspace -- --nocapture

# 运行测试并生成覆盖率报告
cargo install cargo-tarpaulin
cargo tarpaulin --workspace --out Html
```

### 验证构建

```bash
# 验证二进制文件
ls -lh target/release/

# 应该包含:
# - evif (CLI 工具)
# - evif-rest (REST 服务器)
# - evif-fuse (FUSE 守护进程,如果启用)

# 检查二进制信息
file target/release/evif
# => ELF 64-bit LSB executable, x86-64

# 查看依赖库
ldd target/release/evif  # Linux
otool -L target/release/evif  # macOS
```

## 二进制部署

### 获取二进制文件

#### 从源码构建

```bash
# 在构建机器上
cargo build --release

# 二进制文件位置
ls -lh target/release/evif
ls -lh target/release/evif-rest
```

#### 下载预构建版本

```bash
# Linux AMD64
wget https://github.com/evif/evif/releases/latest/download/evif-linux-amd64.tar.gz
tar -xzf evif-linux-amd64.tar.gz

# Linux ARM64
wget https://github.com/evif/evif/releases/latest/download/evif-linux-arm64.tar.gz
tar -xzf evif-linux-arm64.tar.gz

# macOS AMD64
wget https://github.com/evif/evif/releases/latest/download/evif-darwin-amd64.tar.gz
tar -xzf evif-darwin-amd64.tar.gz

# macOS ARM64 (Apple Silicon)
wget https://github.com/evif/evif/releases/latest/download/evif-darwin-arm64.tar.gz
tar -xzf evif-darwin-arm64.tar.gz
```

### 安装二进制文件

#### Linux

```bash
# 复制到系统路径
sudo cp evif /usr/local/bin/
sudo cp evif-rest /usr/local/bin/
sudo cp evif-fuse /usr/local/bin/  # 如果使用 FUSE

# 设置可执行权限
sudo chmod +x /usr/local/bin/evif*
sudo chown root:root /usr/local/bin/evif*

# 验证安装
evif --version
evif-rest --version
```

#### macOS

```bash
# 复制到系统路径
sudo cp evif /usr/local/bin/
sudo cp evif-rest /usr/local/bin/
sudo cp evif-fuse /usr/local/bin/

# 设置可执行权限
sudo chmod +x /usr/local/bin/evif*
sudo chown root:wheel /usr/local/bin/evif*

# 验证安装
evif --version
```

### 创建目录结构

```bash
# 创建配置目录
sudo mkdir -p /etc/evif
sudo mkdir -p /etc/evif/plugins
sudo mkdir -p /etc/evif/certs

# 创建数据目录
sudo mkdir -p /var/lib/evif
sudo mkdir -p /var/lib/evif/data
sudo mkdir -p /var/lib/evif/cache

# 创建日志目录
sudo mkdir -p /var/log/evif

# 设置权限
sudo chown -R evif:evif /etc/evif
sudo chown -R evif:evif /var/lib/evif
sudo chown -R evif:evif /var/log/evif

# 或使用当前用户 (开发环境)
mkdir -p ~/.evif/{config,data,cache,logs}
```

### 配置文件

创建 `/etc/evif/evif.toml`:

```toml
# EVIF 配置文件
[server]
host = "0.0.0.0"
port = 8081
workers = 4

[logging]
level = "info"
file = "/var/log/evif/evif.log"

[storage]
cache_dir = "/var/lib/evif/cache"
data_dir = "/var/lib/evif/data"

[mounts."/mem"]
plugin = "memfs"

[mounts."/local"]
plugin = "localfs"
root = "/var/lib/evif/data"
```

### 快速启动

```bash
# 启动 REST 服务器
evif-rest --config /etc/evif/evif.toml

# 后台运行
nohup evif-rest --config /etc/evif/evif.toml \
  >> /var/log/evif/evif-rest.log 2>&1 &

# 查看日志
tail -f /var/log/evif/evif-rest.log

# 验证服务
curl http://localhost:8081/health
```

## Docker 部署

### Dockerfile

创建 `Dockerfile`:

```dockerfile
# 多阶段构建
FROM rust:1.75-slim as builder

WORKDIR /build

# 安装构建依赖
RUN apt-get update && \
    apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 复制源码
COPY . .

# 构建 Release 版本
RUN cargo build --release

# 运行时镜像
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && \
    apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 创建非 root 用户
RUN useradd -r -s /bin/false evif

# 创建目录
RUN mkdir -p /etc/evif /var/lib/evif /var/log/evif && \
    chown -R evif:evif /etc/evif /var/lib/evif /var/log/evif

# 复制二进制文件
COPY --from=builder /build/target/release/evif-rest /usr/local/bin/
COPY --from=builder /build/target/release/evif-fuse /usr/local/bin/

# 复制配置文件
COPY config/evif.toml /etc/evif/

# 切换用户
USER evif

# 暴露端口
EXPOSE 8081

# 健康检查
HEALTHCHECK --interval=30s --timeout=3s \
  CMD curl -f http://localhost:8081/health || exit 1

# 启动服务
CMD ["evif-rest", "--config", "/etc/evif/evif.toml"]
```

### 构建镜像

```bash
# 构建镜像
docker build -t evif:latest .

# 使用构建参数
docker build \
  --build-arg VERSION=0.1.0 \
  --build-arg BUILD_DATE=$(date -u +'%Y-%m-%dT%H:%M:%SZ') \
  -t evif:0.1.0 .

# 查看镜像
docker images | grep evif
```

### 运行容器

#### 基础运行

```bash
# 运行容器
docker run -d \
  --name evif \
  -p 8081:8081 \
  -v evif-data:/var/lib/evif \
  -v evif-logs:/var/log/evif \
  evif:latest

# 查看日志
docker logs -f evif

# 停止容器
docker stop evif

# 删除容器
docker rm evif
```

#### 挂载本地文件系统

```bash
docker run -d \
  --name evif \
  -p 8081:8081 \
  -v /host/data:/var/lib/evif/data \
  -v /host/config:/etc/evif \
  evif:latest
```

#### 环境变量配置

```bash
docker run -d \
  --name evif \
  -p 8081:8081 \
  -e RUST_LOG=debug \
  -e EVIF_PORT=8081 \
  -e EVIF_WORKERS=4 \
  -v evif-data:/var/lib/evif \
  evif:latest
```

### Docker 网络

```bash
# 创建网络
docker network create evif-network

# 连接多个容器
docker run -d \
  --name evif \
  --network evif-network \
  -p 8081:8081 \
  evif:latest

docker run -d \
  --name redis \
  --network evif-network \
  redis:latest

# 测试连接
docker exec evif curl redis:6379
```

### 资源限制

```bash
# 限制内存和 CPU
docker run -d \
  --name evif \
  -p 8081:8081 \
  --memory="1g" \
  --memory-swap="2g" \
  --cpus="2.0" \
  evif:latest

# 查看资源使用
docker stats evif
```

## Docker Compose 栈

### docker-compose.yml

创建 `docker-compose.yml`:

```yaml
version: '3.8'

services:
  evif:
    build: .
    image: evif:latest
    container_name: evif
    ports:
      - "8081:8081"
      - "8082:8082"
    volumes:
      - evif-data:/var/lib/evif
      - evif-logs:/var/log/evif
      - ./config/evif.toml:/etc/evif/evif.toml:ro
    environment:
      - RUST_LOG=info
      - EVIF_PORT=8081
      - EVIF_WORKERS=4
    depends_on:
      - redis
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8081/health"]
      interval: 30s
      timeout: 3s
      retries: 3
    networks:
      - evif-network

  redis:
    image: redis:7-alpine
    container_name: evif-redis
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    command: redis-server --appendonly yes
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 3s
      retries: 3
    networks:
      - evif-network

  nginx:
    image: nginx:alpine
    container_name: evif-nginx
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx/nginx.conf:/etc/nginx/nginx.conf:ro
      - ./nginx/ssl:/etc/nginx/ssl:ro
    depends_on:
      - evif
    restart: unless-stopped
    networks:
      - evif-network

volumes:
  evif-data:
    driver: local
  evif-logs:
    driver: local
  redis-data:
    driver: local

networks:
  evif-network:
    driver: bridge
```

### 启动栈

```bash
# 启动所有服务
docker-compose up -d

# 查看状态
docker-compose ps

# 查看日志
docker-compose logs -f

# 查看特定服务日志
docker-compose logs -f evif
```

### 管理服务

```bash
# 停止服务
docker-compose stop

# 启动服务
docker-compose start

# 重启服务
docker-compose restart

# 删除所有容器
docker-compose down

# 删除容器和数据卷
docker-compose down -v

# 重新构建镜像
docker-compose build --no-cache

# 扩展服务
docker-compose up -d --scale evif=3
```

### 生产环境配置

创建 `docker-compose.prod.yml`:

```yaml
version: '3.8'

services:
  evif:
    image: evif:0.1.0
    deploy:
      replicas: 3
      resources:
        limits:
          cpus: '2.0'
          memory: 2G
        reservations:
          cpus: '1.0'
          memory: 1G
      restart_policy:
        condition: on-failure
        delay: 5s
        max_attempts: 3
      update_config:
        parallelism: 1
        delay: 10s
        order: start-first
    environment:
      - RUST_LOG=warn
      - EVIF_CACHE_SIZE=524288000  # 500MB
    secrets:
      - evif_cert
      - evif_key
    networks:
      - evif-network

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    secrets:
      - nginx_cert
      - nginx_key
    networks:
      - evif-network

secrets:
  evif_cert:
    file: ./certs/evif.crt
  evif_key:
    file: ./certs/evif.key
  nginx_cert:
    file: ./certs/nginx.crt
  nginx_key:
    file: ./certs/nginx.key

networks:
  evif-network:
    driver: overlay
```

部署到 Swarm:

```bash
# 初始化 Swarm
docker swarm init

# 部署栈
docker stack deploy -c docker-compose.prod.yml evif

# 查看服务
docker service ls

# 查看日志
docker service logs evif_evif

# 扩展服务
docker service scale evif_evif=5

# 删除栈
docker stack rm evif
```

## Systemd 服务

### 创建服务文件

创建 `/etc/systemd/system/evif.service`:

```ini
[Unit]
Description=EVIF REST Server
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=evif
Group=evif

# 工作目录
WorkingDirectory=/var/lib/evif

# 启动命令
ExecStart=/usr/local/bin/evif-rest \
    --config /etc/evif/evif.toml \
    --port 8081

# 重启策略
Restart=always
RestartSec=10

# 资源限制
LimitNOFILE=65536
LimitNPROC=4096
MemoryMax=2G
CPUQuota=200%

# 安全选项
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/evif /var/log/evif

# 日志
StandardOutput=journal
StandardError=journal
SyslogIdentifier=evif

[Install]
WantedBy=multi-user.target
```

### 创建用户

```bash
# 创建系统用户
sudo useradd -r -s /bin/false evif
sudo usermod -a -G fuse evif  # 如果使用 FUSE

# 验证用户
id evif
```

### 启用和管理服务

```bash
# 重新加载 systemd 配置
sudo systemctl daemon-reload

# 启用服务 (开机自启)
sudo systemctl enable evif

# 启动服务
sudo systemctl start evif

# 查看状态
sudo systemctl status evif

# 查看日志
sudo journalctl -u evif -f

# 停止服务
sudo systemctl stop evif

# 重启服务
sudo systemctl restart evif

# 禁用服务
sudo systemctl disable evif
```

### 服务依赖

创建 `/etc/systemd/system/evif@.service` (模板服务):

```ini
[Unit]
Description=EVIF %i Service
After=network.target redis.service
Wants=redis.service

[Service]
Type=simple
User=evif
Group=evif
ExecStart=/usr/local/bin/evif-%i \
    --config /etc/evif/%i.toml
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

启动多个实例:

```bash
# 启动 REST 服务
sudo systemctl enable evif@rest
sudo systemctl start evif@rest

# 启动 FUSE 服务
sudo systemctl enable evif@fuse
sudo systemctl start evif@fuse
```

### 日志管理

```bash
# 查看最近日志
sudo journalctl -u evif -n 100

# 查看今天日志
sudo journalctl -u evif --since today

# 查看最近 1 小时日志
sudo journalctl -u evif --since "1 hour ago"

# 持续查看日志
sudo journalctl -u evif -f

# 导出日志
sudo journalctl -u evif --since "2024-01-01" > evif.log

# 配置日志持久化
sudo mkdir -p /var/log/journal
sudo systemd-tmpfiles --create --prefix /var/log/journal
```

### 定时任务

创建 `/etc/systemd/system/evif-backup.timer`:

```ini
[Unit]
Description=EVIF Backup Timer
Requires=evif-backup.service

[Timer]
OnCalendar=daily
Persistent=true

[Install]
WantedBy=timers.target
```

创建 `/etc/systemd/system/evif-backup.service`:

```ini
[Unit]
Description=EVIF Backup Service

[Service]
Type=oneshot
User=evif
ExecStart=/usr/local/bin/evif-backup.sh
```

启用定时任务:

```bash
sudo systemctl enable evif-backup.timer
sudo systemctl start evif-backup.timer
sudo systemctl list-timers
```

## 配置管理

### 环境变量

#### 服务器配置

```bash
# /etc/evif/evif.conf
export EVIF_HOST=0.0.0.0
export EVIF_PORT=8081
export EVIF_WORKERS=4

# 日志配置
export RUST_LOG=info
export RUST_LOG_STYLE=always

# 缓存配置
export EVIF_CACHE_ENABLED=true
export EVIF_CACHE_SIZE=104857600  # 100MB

# 存储配置
export EVIF_DATA_DIR=/var/lib/evif/data
export EVIF_CACHE_DIR=/var/lib/evif/cache
```

#### 云存储配置

```bash
# AWS S3
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
export AWS_DEFAULT_REGION=us-east-1
export AWS_ENDPOINT=https://s3.amazonaws.com

# Azure Blob Storage
export AZURE_STORAGE_ACCOUNT=your_account
export AZURE_STORAGE_KEY=your_key
export AZURE_ENDPOINT=https://your_account.blob.core.windows.net

# Google Cloud Storage
export GOOGLE_APPLICATION_CREDENTIALS=/etc/evif/gcp-credentials.json
export GCP_PROJECT_ID=your_project_id
export GCP_BUCKET=your_bucket
```

#### 安全配置

```bash
# TLS 证书
export EVIF_TLS_ENABLED=true
export EVIF_TLS_CERT=/etc/evif/certs/server.crt
export EVIF_TLS_KEY=/etc/evif/certs/server.key

# API 密钥
export EVIF_API_KEY=your_api_key

# 认证
export EVIF_AUTH_ENABLED=true
export EVIF_AUTH_PROVIDER=jwt
export EVIF_JWT_SECRET=your_jwt_secret
```

### 配置文件

#### TOML 配置

创建 `/etc/evif/evif.toml`:

```toml
# EVIF 主配置文件

[server]
host = "0.0.0.0"
port = 8081
workers = 4
max_connections = 10000

[logging]
level = "info"  # debug, info, warn, error
format = "json"  # json, pretty
file = "/var/log/evif/evif.log"
max_size = "100MB"
max_backups = 10
max_age = 30

[storage]
type = "memory"  # memory, disk, s3, azure, gcs
cache_dir = "/var/lib/evif/cache"
data_dir = "/var/lib/evif/data"

[storage.cache]
enabled = true
size_mb = 100
ttl_seconds = 3600

[mounts."/mem"]
plugin = "memfs"
readonly = false

[mounts."/local"]
plugin = "localfs"
root = "/var/lib/evif/data"
readonly = false

[mounts."/s3"]
plugin = "s3fs"
bucket = "my-bucket"
region = "us-east-1"
readonly = false

[auth]
enabled = true
type = "jwt"
secret = "${EVIF_JWT_SECRET}"
ttl_hours = 24

[tls]
enabled = true
cert = "/etc/evif/certs/server.crt"
key = "/etc/evif/certs/server.key"
client_auth = false

[metrics]
enabled = true
port = 9090
path = "/metrics"
```

### 配置验证

```bash
# 验证配置文件
evif-rest --validate-config /etc/evif/evif.toml

# 测试配置
evif-rest --dry-run --config /etc/evif/evif.toml

# 查看当前配置
curl http://localhost:8081/api/v1/config

# 导出配置
curl http://localhost:8081/api/v1/config > /etc/evif/evif-backup.toml
```

### 配置重载

```bash
# 发送 SIGHUP 信号重载配置
sudo systemctl reload evif

# 或使用 kill 命令
sudo kill -HUP $(pidof evif-rest)

# 验证重载
sudo journalctl -u evif --since "1 minute ago"
```

## 生产环境部署

### 反向代理配置

#### Nginx

创建 `/etc/nginx/conf.d/evif.conf`:

```nginx
upstream evif_backend {
    least_conn;
    server 127.0.0.1:8081 weight=1;
    server 127.0.0.1:8082 weight=1;
    keepalive 32;
}

server {
    listen 80;
    server_name evif.example.com;

    # 重定向到 HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name evif.example.com;

    # TLS 证书
    ssl_certificate /etc/letsencrypt/live/evif.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/evif.example.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;

    # 日志
    access_log /var/log/nginx/evif_access.log;
    error_log /var/log/nginx/evif_error.log;

    # 客户端上传大小限制
    client_max_body_size 100M;

    # 超时设置
    proxy_connect_timeout 60s;
    proxy_send_timeout 60s;
    proxy_read_timeout 60s;

    # 代理设置
    location / {
        proxy_pass http://evif_backend;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # CORS 头
        add_header Access-Control-Allow-Origin *;
        add_header Access-Control-Allow-Methods "GET, POST, PUT, DELETE, OPTIONS";
        add_header Access-Control-Allow-Headers "Content-Type, Authorization";
    }

    # WebSocket 支持
    location /ws {
        proxy_pass http://evif_backend;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }

    # 健康检查
    location /health {
        proxy_pass http://evif_backend/health;
        access_log off;
    }
}
```

#### Caddy

创建 `Caddyfile`:

```caddyfile
evif.example.com {
    reverse_proxy localhost:8081

    # TLS 自动配置
    tls {
        dns cloudflare
        resolvers 1.1.1.1
    }

    # 日志
    log {
        output file /var/log/caddy/evif.log
        format json
    }

    # 响应头
    header {
        Access-Control-Allow-Origin *
        Access-Control-Allow-Methods "GET, POST, PUT, DELETE, OPTIONS"
        Access-Control-Allow-Headers "Content-Type, Authorization"
        X-Frame-Options "DENY"
        X-Content-Type-Options "nosniff"
    }
}
```

### 负载均衡

#### HAProxy

创建 `/etc/haproxy/haproxy.cfg`:

```haproxy
defaults
    mode http
    timeout connect 5000ms
    timeout client 50000ms
    timeout server 50000ms

frontend evif_frontend
    bind *:80
    default_backend evif_backend

backend evif_backend
    balance roundrobin
    server evif1 127.0.0.1:8081 check
    server evif2 127.0.0.1:8082 check
    server evif3 127.0.0.1:8083 check

listen stats
    bind *:8404
    stats enable
    stats uri /stats
    stats refresh 10s
```

### 高可用部署

#### 主从复制

```bash
# 主节点
evif-rest --role master --port 8081

# 从节点
evif-rest --role replica --master http://master:8081 --port 8082
```

#### 集群模式

```bash
# 节点 1
evif-rest \
    --cluster-enabled \
    --cluster-node-id node1 \
    --cluster-addr 192.168.1.10:8081 \
    --cluster-peers node2=192.168.1.11:8082,node3=192.168.1.12:8083

# 节点 2
evif-rest \
    --cluster-enabled \
    --cluster-node-id node2 \
    --cluster-addr 192.168.1.11:8082 \
    --cluster-peers node1=192.168.1.10:8081,node3=192.168.1.12:8083

# 节点 3
evif-rest \
    --cluster-enabled \
    --cluster-node-id node3 \
    --cluster-addr 192.168.1.12:8083 \
    --cluster-peers node1=192.168.1.10:8081,node2=192.168.1.11:8082
```

### 备份策略

#### 数据备份脚本

创建 `/usr/local/bin/evif-backup.sh`:

```bash
#!/bin/bash

BACKUP_DIR="/backup/evif"
DATA_DIR="/var/lib/evif/data"
DATE=$(date +%Y%m%d_%H%M%S)

# 创建备份目录
mkdir -p "$BACKUP_DIR"

# 备份数据
tar -czf "$BACKUP_DIR/evif-data-$DATE.tar.gz" -C "$DATA_DIR" .

# 备份配置
cp /etc/evif/evif.toml "$BACKUP_DIR/evif-config-$DATE.toml"

# 删除 30 天前的备份
find "$BACKUP_DIR" -name "evif-*" -mtime +30 -delete

# 上传到 S3 (可选)
aws s3 cp "$BACKUP_DIR/evif-data-$DATE.tar.gz" \
    s3://my-backup-bucket/evif/

echo "Backup completed: evif-data-$DATE.tar.gz"
```

设置可执行权限:

```bash
chmod +x /usr/local/bin/evif-backup.sh
```

添加到 crontab:

```bash
# 每天凌晨 2 点执行备份
0 2 * * * /usr/local/bin/evif-backup.sh
```

## 监控与日志

### 日志配置

#### 结构化日志

```toml
# /etc/evif/evif.toml
[logging]
level = "info"
format = "json"
outputs = ["console", "file"]

[logging.file]
path = "/var/log/evif/evif.log"
rotation = "daily"
retention = 30

[logging.console]
color = true
```

#### 日志级别

| 级别 | 描述 | 用途 |
|------|------|------|
| `trace` | 最详细 | 跟踪执行流程 |
| `debug` | 调试信息 | 开发和调试 |
| `info` | 一般信息 | 正常运行 (推荐) |
| `warn` | 警告信息 | 潜在问题 |
| `error` | 错误信息 | 错误和异常 |

### 日志收集

#### rsyslog 配置

创建 `/etc/rsyslog.d/evif.conf`:

```
# EVIF 日志收集
if $programname == 'evif' then /var/log/evif/evif.log
& stop
```

#### Filebeat 配置

创建 `/etc/filebeat/modules.d/evif.yml`:

```yaml
- module: evif
  log:
    enabled: true
    var.paths: ["/var/log/evif/*.log"]
    var.input_type: "log"

    # 解析 JSON 日志
    json.keys_under_root: true
    json.add_error_key: true

    # 字段配置
    fields:
      service: evif
      environment: production

    fields_under_root: true
```

### 监控指标

#### Prometheus 集成

EVIF 提供 Prometheus 指标端点:

```bash
# 启用指标
evif-rest --metrics-enabled --metrics-port 9090

# 访问指标
curl http://localhost:9090/metrics
```

Prometheus 配置:

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'evif'
    scrape_interval: 15s
    static_configs:
      - targets: ['localhost:9090']
    metrics_path: '/metrics'
```

#### Grafana 仪表板

创建 Grafana 仪表板查询:

```promql
# 请求速率
rate(evif_requests_total[5m])

# 错误率
rate(evif_errors_total[5m]) / rate(evif_requests_total[5m])

# 响应时间
histogram_quantile(0.95, rate(evif_request_duration_seconds_bucket[5m]))

# 内存使用
evif_memory_usage_bytes

# 连接数
evif_active_connections
```

### 健康检查

#### HTTP 健康检查

```bash
# 基本健康检查
curl http://localhost:8081/health

# 响应
{
  "status": "healthy",
  "timestamp": "2026-03-01T12:00:00.000000Z",
  "version": "0.1.0",
  "uptime_seconds": 3600,
  "memory_usage_mb": 128,
  "active_connections": 10
}
```

#### 就绪检查

```bash
curl http://localhost:8081/ready

# 响应
{
  "ready": true,
  "checks": {
    "storage": "ok",
    "cache": "ok",
    "plugins": "ok"
  }
}
```

### 告警配置

#### Alertmanager 规则

创建 `alerts.yml`:

```yaml
groups:
  - name: evif_alerts
    rules:
      - alert: EVIFDown
        expr: up{job="evif"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "EVIF service is down"

      - alert: EVIFHighErrorRate
        expr: rate(evif_errors_total[5m]) > 0.05
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "EVIF error rate is high"

      - alert: EVIFHighMemory
        expr: evif_memory_usage_bytes > 1073741824
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "EVIF memory usage is high"
```

## 故障排查

### 常见问题

#### 服务无法启动

**症状**

```bash
sudo systemctl start evif
# 服务启动失败
```

**诊断**

```bash
# 查看服务状态
sudo systemctl status evif

# 查看日志
sudo journalctl -u evif -n 50

# 检查端口占用
sudo netstat -tulpn | grep 8081
# 或
sudo lsof -i :8081
```

**解决方案**

```bash
# 1. 端口被占用
# 终止占用进程
sudo kill -9 <PID>

# 或更改端口
evif-rest --port 8082

# 2. 权限问题
sudo chown -R evif:evif /var/lib/evif
sudo chown -R evif:evif /var/log/evif

# 3. 配置文件错误
evif-rest --validate-config /etc/evif/evif.toml
```

#### 性能问题

**症状**

- 响应缓慢
- CPU/内存使用率过高

**诊断**

```bash
# 查看资源使用
top -p $(pidof evif-rest)

# 查看内存使用
ps aux | grep evif-rest

# 分析性能
sudo perf top -p $(pidof evif-rest)
```

**解决方案**

```bash
# 1. 增加工作线程
evif-rest --workers 8

# 2. 限制缓存大小
export EVIF_CACHE_SIZE=52428800  # 50MB

# 3. 启用压缩
export EVIF_COMPRESSION_ENABLED=true

# 4. 调整 Tokio 运行时
export TOKIO_WORKER_THREADS=4
```

#### 内存泄漏

**症状**

- 内存使用持续增长
- 服务最终崩溃

**诊断**

```bash
# 监控内存使用
watch -n 5 'ps aux | grep evif-rest'

# 使用 Valgrind 分析
sudo valgrind --leak-check=full \
    --show-leak-kinds=all \
    evif-rest
```

**解决方案**

```bash
# 1. 设置内存限制
sudo systemctl set-property evif MemoryMax=2G

# 2. 定期重启 (临时方案)
# 添加到 crontab
0 3 * * * /usr/bin/systemctl restart evif

# 3. 升级到最新版本
cargo install --path crates/evif-rest --force-reinstall
```

#### 存储问题

**症状**

- 文件操作失败
- 磁盘空间不足

**诊断**

```bash
# 检查磁盘空间
df -h /var/lib/evif

# 检查文件描述符限制
ulimit -n

# 查看打开的文件
sudo lsof -p $(pidof evif-rest) | wc -l
```

**解决方案**

```bash
# 1. 清理缓存
rm -rf /var/lib/evif/cache/*

# 2. 增加文件描述符限制
# 编辑 /etc/systemd/system/evif.service
[Service]
LimitNOFILE=65536

# 3. 扩容磁盘
# 根据具体环境操作
```

### 日志分析

#### 错误日志

```bash
# 查看错误日志
sudo journalctl -u evif -p err -n 100

# 搜索特定错误
sudo journalctl -u evif | grep -i "error"

# 导出错误日志
sudo journalctl -u evif -p err > evif-errors.log
```

#### 性能日志

```bash
# 启用性能日志
export RUST_LOG=evif=perf

# 分析慢查询
sudo journalctl -u evif | grep "slow query"

# 查看响应时间
sudo journalctl -u evif | grep "duration"
```

### 调试模式

```bash
# 启用调试日志
export RUST_LOG=debug
evif-rest

# 启用跟踪
export RUST_LOG=trace
export RUST_BACKTRACE=1
evif-rest

# 使用 GDB
sudo gdb -p $(pidof evif-rest)
(gdb) bt  # 查看堆栈
(gdb) thread apply all bt  # 查看所有线程堆栈
```

## 升级与迁移

### 升级前准备

#### 备份数据

```bash
# 停止服务
sudo systemctl stop evif

# 备份数据目录
sudo tar -czf /backup/evif-data-$(date +%Y%m%d).tar.gz \
    /var/lib/evif

# 备份配置文件
sudo cp /etc/evif/evif.toml \
    /backup/evif-config-$(date +%Y%m%d).toml

# 验证备份
sudo tar -tzf /backup/evif-data-$(date +%Y%m%d).tar.gz | head -20
```

#### 检查兼容性

```bash
# 查看当前版本
evif --version

# 查看升级说明
cat /path/to/evif/RELEASES.md
```

### 滚动升级

#### 二进制升级

```bash
# 1. 下载新版本
wget https://github.com/evif/evif/releases/download/v0.2.0/evif-linux-amd64.tar.gz
tar -xzf evif-linux-amd64.tar.gz

# 2. 验证新版本
./evif --version

# 3. 备份旧版本
sudo cp /usr/local/bin/evif-rest \
    /usr/local/bin/evif-rest.backup

# 4. 安装新版本
sudo cp evif-rest /usr/local/bin/
sudo chmod +x /usr/local/bin/evif-rest

# 5. 重启服务
sudo systemctl restart evif

# 6. 验证升级
curl http://localhost:8081/health
sudo journalctl -u evif -n 50
```

#### Docker 升级

```bash
# 1. 拉取新镜像
docker pull evif:0.2.0

# 2. 停止旧容器
docker stop evif
docker rm evif

# 3. 启动新容器
docker run -d \
  --name evif \
  -p 8081:8081 \
  -v evif-data:/var/lib/evif \
  evif:0.2.0

# 4. 验证升级
docker logs -f evif
```

### 灰度发布

```bash
# 部署新版本到部分实例
docker-compose up -d --scale evif=2

# 监控新版本
docker-compose ps
docker-compose logs -f evif

# 逐步增加新版本实例
docker-compose up -d --scale evif=4

# 监控指标
curl http://localhost:9090/metrics
```

### 回滚策略

```bash
# 二进制回滚
sudo systemctl stop evif
sudo cp /usr/local/bin/evif-rest.backup \
    /usr/local/bin/evif-rest
sudo systemctl start evif

# Docker 回滚
docker stop evif
docker rm evif
docker run -d \
  --name evif \
  -p 8081:8081 \
  -v evif-data:/var/lib/evif \
  evif:0.1.0  # 旧版本

# 验证回滚
curl http://localhost:8081/health
```

### 数据迁移

#### 从旧版本迁移

```bash
# 使用迁移工具
evif-migrate \
    --source /var/lib/evif/old \
    --destination /var/lib/evif/new \
    --format json
```

#### 从其他系统迁移

```bash
# 从文件系统导入
evif import --source /path/to/files --dest /imported

# 从 S3 导入
evif import --source s3://my-bucket --dest /imported

# 验证导入
evif ls /imported
```

## 总结

本章涵盖了 EVIF 在生产环境中的完整部署流程:

- ✅ 了解多种部署方式 (二进制、Docker、Systemd)
- ✅ 掌握配置管理技巧
- ✅ 实现高可用和负载均衡
- ✅ 设置监控和日志收集
- ✅ 排查常见问题
- ✅ 执行平滑升级和迁移

## 下一步

- **第十章**: [高级主题](./chapter-10-advanced-topics.md) - 深入了解性能优化、插件开发和扩展功能
- **API 参考**: [第七章](./chapter-7-api-reference.md) - 完整的 API 文档
- **安全指南**: [第八章](./chapter-8-authentication-security.md) - 认证与安全最佳实践

## 参考资源

- **官方文档**: https://docs.evif.io
- **GitHub 仓库**: https://github.com/evif/evif
- **Docker Hub**: https://hub.docker.com/r/evif/evif
- **社区支持**: https://github.com/evif/evif/discussions
