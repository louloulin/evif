# Chapter 9: Deployment Guide

This chapter covers deployment methods, configuration management, and operational best practices for running EVIF (Extensible Virtual File System) in production environments.

## Table of Contents

- [Deployment Overview](#deployment-overview)
- [Prerequisites & Dependencies](#prerequisites-dependencies)
- [Build from Source](#build-from-source)
- [Binary Deployment](#binary-deployment)
- [Docker Deployment](#docker-deployment)
- [Docker Compose Stack](#docker-compose-stack)
- [Systemd Service](#systemd-service)
- [Configuration Management](#configuration-management)
- [Production Deployment](#production-deployment)
- [Monitoring & Logging](#monitoring-logging)
- [Troubleshooting](#troubleshooting)
- [Upgrade & Migration](#upgrade-migration)

## Deployment Overview

### Deployment Architecture

EVIF adopts a modular design supporting multiple deployment strategies:

```
┌─────────────────────────────────────────┐
│           Client Layer                  │
│  ┌──────────┐  ┌──────────┐  ┌────────┐ │
│  │ CLI Tool │  │ REST API │  │ FUSE  │ │
│  └──────────┘  └──────────┘  └────────┘ │
└─────────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────────┐
│           EVIF Runtime                   │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐   │
│  │Graph │ │Store │ │ Auth │ │Plugin│   │
│  └──────┘ └──────┘ └──────┘ └──────┘   │
└─────────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────────┐
│           Backend Services               │
│  ┌──────┐  ┌──────┐  ┌──────────┐      │
│  │Redis │ │LocalFS│  │Cloud S3  │      │
│  └──────┘  └──────┘  └──────────┘      │
└─────────────────────────────────────────┘
```

### Deployment Modes

| Mode | Use Case | Advantages | Disadvantages |
|------|----------|------------|---------------|
| **Standalone** | Development, testing, small-scale apps | Simple, fast | Single point of failure |
| **Docker** | Production, microservices | Portable, scalable | Requires Docker environment |
| **Kubernetes** | Large-scale, high-availability | Auto-scaling, self-healing | High complexity |
| **Systemd** | Linux servers | Auto-restart, log management | Linux only |

### Core Components

- **evif-cli**: Command-line tool for managing and operating EVIF
- **evif-rest**: REST API server providing HTTP interface
- **evif-fuse**: FUSE integration for filesystem mounting (optional)
- **evif-runtime**: Runtime core coordinating all components

## Prerequisites & Dependencies

### System Requirements

#### Minimum Configuration

- **OS**: Linux (kernel 3.10+) or macOS (10.15+)
- **CPU**: 1 core
- **Memory**: 512MB RAM
- **Disk**: 100MB available space
- **Network**: Optional (for remote storage and API access)

#### Recommended Configuration (Production)

- **OS**: Linux (kernel 5.4+) or macOS (12+)
- **CPU**: 2+ cores
- **Memory**: 2GB+ RAM
- **Disk**: 1GB+ available space (SSD recommended)
- **Network**: 1Gbps+ (for cloud storage access)

### Software Dependencies

#### Build Dependencies

```bash
# Rust toolchain (1.70+)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version
```

#### Runtime Dependencies

**Required Dependencies**

- **None**: EVIF core functionality has no external dependencies

**Optional Dependencies**

```bash
# FUSE support (for filesystem mounting)
# Linux
sudo apt-get install libfuse-dev fuse  # Debian/Ubuntu
sudo yum install fuse-devel             # CentOS/RHEL

# macOS
brew install --cask macfuse

# Redis (for caching)
sudo apt-get install redis-server  # Debian/Ubuntu
sudo yum install redis              # CentOS/RHEL
brew install redis                  # macOS

# OpenSSL (for TLS support)
sudo apt-get install libssl-dev  # Debian/Ubuntu
```

#### Cloud Storage Dependencies (Optional)

```bash
# AWS S3
pip install awscli
aws configure

# Azure Blob Storage
pip install azure-storage-blob

# Google Cloud Storage
pip install google-cloud-storage
```

### Network Requirements

#### Port Usage

| Port | Protocol | Purpose | Configurable |
|------|----------|---------|--------------|
| 8081 | HTTP | REST API | Yes |
| 8082 | HTTP | gRPC Web | Yes |
| 6379 | TCP | Redis (optional) | Yes |

#### Firewall Configuration

```bash
# Ubuntu/Debian (ufw)
sudo ufw allow 8081/tcp
sudo ufw allow 8082/tcp

# CentOS/RHEL (firewalld)
sudo firewall-cmd --permanent --add-port=8081/tcp
sudo firewall-cmd --permanent --add-port=8082/tcp
sudo firewall-cmd --reload
```

## Build from Source

### Clone Repository

```bash
# Clone main repository
git clone https://github.com/evif/evif.git
cd evif

# Or use SSH
git clone git@github.com:evif/evif.git
cd evif

# Checkout specific version
git checkout v0.1.0
```

### Build Workspace

EVIF uses Cargo workspace with multiple crates:

```bash
# View all members
cat Cargo.toml | grep -A 20 "members"

# Build all crates (Debug mode)
cargo build

# Build all crates (Release mode, recommended)
cargo build --release

# Build specific crate only
cargo build -p evif-cli --release
cargo build -p evif-rest --release
cargo build -p evif-fuse --release
```

### Compilation Options

#### Optimization Levels

```bash
# Debug build (default)
cargo build

# Release build (optimized)
cargo build --release

# Custom optimization
cargo build --release \
  --profile release-lto  # Enable LTO
```

#### Feature Flags

```bash
# Build with FUSE support
cargo build --release --features fuse

# Build with all features
cargo build --release --all-features

# View available features
cargo build --release --features help
```

### Testing Build

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p evif-graph
cargo test -p evif-storage
cargo test -p evif-auth

# Run tests with output
cargo test --workspace -- --nocapture

# Generate coverage report
cargo install cargo-tarpaulin
cargo tarpaulin --workspace --out Html
```

### Verify Build

```bash
# Verify binaries
ls -lh target/release/

# Should include:
# - evif (CLI tool)
# - evif-rest (REST server)
# - evif-fuse (FUSE daemon, if enabled)

# Check binary info
file target/release/evif
# => ELF 64-bit LSB executable, x86-64

# View dependencies
ldd target/release/evif  # Linux
otool -L target/release/evif  # macOS
```

## Binary Deployment

### Obtain Binaries

#### Build from Source

```bash
# On build machine
cargo build --release

# Binary locations
ls -lh target/release/evif
ls -lh target/release/evif-rest
```

#### Download Pre-built Versions

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

### Install Binaries

#### Linux

```bash
# Copy to system path
sudo cp evif /usr/local/bin/
sudo cp evif-rest /usr/local/bin/
sudo cp evif-fuse /usr/local/bin/  # If using FUSE

# Set executable permissions
sudo chmod +x /usr/local/bin/evif*
sudo chown root:root /usr/local/bin/evif*

# Verify installation
evif --version
evif-rest --version
```

#### macOS

```bash
# Copy to system path
sudo cp evif /usr/local/bin/
sudo cp evif-rest /usr/local/bin/
sudo cp evif-fuse /usr/local/bin/

# Set executable permissions
sudo chmod +x /usr/local/bin/evif*
sudo chown root:wheel /usr/local/bin/evif*

# Verify installation
evif --version
```

### Create Directory Structure

```bash
# Create config directories
sudo mkdir -p /etc/evif
sudo mkdir -p /etc/evif/plugins
sudo mkdir -p /etc/evif/certs

# Create data directories
sudo mkdir -p /var/lib/evif
sudo mkdir -p /var/lib/evif/data
sudo mkdir -p /var/lib/evif/cache

# Create log directories
sudo mkdir -p /var/log/evif

# Set permissions
sudo chown -R evif:evif /etc/evif
sudo chown -R evif:evif /var/lib/evif
sudo chown -R evif:evif /var/log/evif

# Or use current user (development)
mkdir -p ~/.evif/{config,data,cache,logs}
```

### Configuration File

Create `/etc/evif/evif.toml`:

```toml
# EVIF configuration file
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

### Quick Start

```bash
# Start REST server
evif-rest --config /etc/evif/evif.toml

# Run in background
nohup evif-rest --config /etc/evif/evif.toml \
  >> /var/log/evif/evif-rest.log 2>&1 &

# View logs
tail -f /var/log/evif/evif-rest.log

# Verify service
curl http://localhost:8081/health
```

## Docker Deployment

### Dockerfile

Create `Dockerfile`:

```dockerfile
# Multi-stage build
FROM rust:1.75-slim as builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && \
    apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy source code
COPY . .

# Build release version
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false evif

# Create directories
RUN mkdir -p /etc/evif /var/lib/evif /var/log/evif && \
    chown -R evif:evif /etc/evif /var/lib/evif /var/log/evif

# Copy binaries
COPY --from=builder /build/target/release/evif-rest /usr/local/bin/
COPY --from=builder /build/target/release/evif-fuse /usr/local/bin/

# Copy config file
COPY config/evif.toml /etc/evif/

# Switch user
USER evif

# Expose ports
EXPOSE 8081

# Health check
HEALTHCHECK --interval=30s --timeout=3s \
  CMD curl -f http://localhost:8081/health || exit 1

# Start service
CMD ["evif-rest", "--config", "/etc/evif/evif.toml"]
```

### Build Image

```bash
# Build image
docker build -t evif:latest .

# With build args
docker build \
  --build-arg VERSION=0.1.0 \
  --build-arg BUILD_DATE=$(date -u +'%Y-%m-%dT%H:%M:%SZ') \
  -t evif:0.1.0 .

# View images
docker images | grep evif
```

### Run Container

#### Basic Run

```bash
# Run container
docker run -d \
  --name evif \
  -p 8081:8081 \
  -v evif-data:/var/lib/evif \
  -v evif-logs:/var/log/evif \
  evif:latest

# View logs
docker logs -f evif

# Stop container
docker stop evif

# Remove container
docker rm evif
```

#### Mount Local Filesystem

```bash
docker run -d \
  --name evif \
  -p 8081:8081 \
  -v /host/data:/var/lib/evif/data \
  -v /host/config:/etc/evif \
  evif:latest
```

#### Environment Variables

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

### Docker Networking

```bash
# Create network
docker network create evif-network

# Connect multiple containers
docker run -d \
  --name evif \
  --network evif-network \
  -p 8081:8081 \
  evif:latest

docker run -d \
  --name redis \
  --network evif-network \
  redis:latest

# Test connection
docker exec evif curl redis:6379
```

### Resource Limits

```bash
# Limit memory and CPU
docker run -d \
  --name evif \
  -p 8081:8081 \
  --memory="1g" \
  --memory-swap="2g" \
  --cpus="2.0" \
  evif:latest

# View resource usage
docker stats evif
```

## Docker Compose Stack

### docker-compose.yml

Create `docker-compose.yml`:

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

### Start Stack

```bash
# Start all services
docker-compose up -d

# View status
docker-compose ps

# View logs
docker-compose logs -f

# View specific service logs
docker-compose logs -f evif
```

### Manage Services

```bash
# Stop services
docker-compose stop

# Start services
docker-compose start

# Restart services
docker-compose restart

# Remove all containers
docker-compose down

# Remove containers and volumes
docker-compose down -v

# Rebuild images
docker-compose build --no-cache

# Scale services
docker-compose up -d --scale evif=3
```

### Production Configuration

Create `docker-compose.prod.yml`:

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

Deploy to Swarm:

```bash
# Initialize Swarm
docker swarm init

# Deploy stack
docker stack deploy -c docker-compose.prod.yml evif

# View services
docker service ls

# View logs
docker service logs evif_evif

# Scale services
docker service scale evif_evif=5

# Remove stack
docker stack rm evif
```

## Systemd Service

### Create Service File

Create `/etc/systemd/system/evif.service`:

```ini
[Unit]
Description=EVIF REST Server
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=evif
Group=evif

# Working directory
WorkingDirectory=/var/lib/evif

# Start command
ExecStart=/usr/local/bin/evif-rest \
    --config /etc/evif/evif.toml \
    --port 8081

# Restart policy
Restart=always
RestartSec=10

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096
MemoryMax=2G
CPUQuota=200%

# Security options
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/evif /var/log/evif

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=evif

[Install]
WantedBy=multi-user.target
```

### Create User

```bash
# Create system user
sudo useradd -r -s /bin/false evif
sudo usermod -a -G fuse evif  # If using FUSE

# Verify user
id evif
```

### Enable and Manage Service

```bash
# Reload systemd configuration
sudo systemctl daemon-reload

# Enable service (auto-start on boot)
sudo systemctl enable evif

# Start service
sudo systemctl start evif

# View status
sudo systemctl status evif

# View logs
sudo journalctl -u evif -f

# Stop service
sudo systemctl stop evif

# Restart service
sudo systemctl restart evif

# Disable service
sudo systemctl disable evif
```

### Service Dependencies

Create `/etc/systemd/system/evif@.service` (template service):

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

Start multiple instances:

```bash
# Start REST service
sudo systemctl enable evif@rest
sudo systemctl start evif@rest

# Start FUSE service
sudo systemctl enable evif@fuse
sudo systemctl start evif@fuse
```

### Log Management

```bash
# View recent logs
sudo journalctl -u evif -n 100

# View today's logs
sudo journalctl -u evif --since today

# View logs from last hour
sudo journalctl -u evif --since "1 hour ago"

# Follow logs
sudo journalctl -u evif -f

# Export logs
sudo journalctl -u evif --since "2024-01-01" > evif.log

# Configure persistent logging
sudo mkdir -p /var/log/journal
sudo systemd-tmpfiles --create --prefix /var/log/journal
```

### Scheduled Tasks

Create `/etc/systemd/system/evif-backup.timer`:

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

Create `/etc/systemd/system/evif-backup.service`:

```ini
[Unit]
Description=EVIF Backup Service

[Service]
Type=oneshot
User=evif
ExecStart=/usr/local/bin/evif-backup.sh
```

Enable scheduled task:

```bash
sudo systemctl enable evif-backup.timer
sudo systemctl start evif-backup.timer
sudo systemctl list-timers
```

## Configuration Management

### Environment Variables

#### Server Configuration

```bash
# /etc/evif/evif.conf
export EVIF_HOST=0.0.0.0
export EVIF_PORT=8081
export EVIF_WORKERS=4

# Logging configuration
export RUST_LOG=info
export RUST_LOG_STYLE=always

# Cache configuration
export EVIF_CACHE_ENABLED=true
export EVIF_CACHE_SIZE=104857600  # 100MB

# Storage configuration
export EVIF_DATA_DIR=/var/lib/evif/data
export EVIF_CACHE_DIR=/var/lib/evif/cache
```

#### Cloud Storage Configuration

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

#### Security Configuration

```bash
# TLS certificates
export EVIF_TLS_ENABLED=true
export EVIF_TLS_CERT=/etc/evif/certs/server.crt
export EVIF_TLS_KEY=/etc/evif/certs/server.key

# API keys
export EVIF_API_KEY=your_api_key

# Authentication
export EVIF_AUTH_ENABLED=true
export EVIF_AUTH_PROVIDER=jwt
export EVIF_JWT_SECRET=your_jwt_secret
```

### Configuration Files

#### TOML Configuration

Create `/etc/evif/evif.toml`:

```toml
# EVIF main configuration file

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

### Configuration Validation

```bash
# Validate configuration file
evif-rest --validate-config /etc/evif/evif.toml

# Test configuration
evif-rest --dry-run --config /etc/evif/evif.toml

# View current configuration
curl http://localhost:8081/api/v1/config

# Export configuration
curl http://localhost:8081/api/v1/config > /etc/evif/evif-backup.toml
```

### Configuration Reload

```bash
# Send SIGHUP to reload config
sudo systemctl reload evif

# Or use kill command
sudo kill -HUP $(pidof evif-rest)

# Verify reload
sudo journalctl -u evif --since "1 minute ago"
```

## Production Deployment

### Reverse Proxy Configuration

#### Nginx

Create `/etc/nginx/conf.d/evif.conf`:

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

    # Redirect to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name evif.example.com;

    # TLS certificates
    ssl_certificate /etc/letsencrypt/live/evif.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/evif.example.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;

    # Logging
    access_log /var/log/nginx/evif_access.log;
    error_log /var/log/nginx/evif_error.log;

    # Client upload size limit
    client_max_body_size 100M;

    # Timeouts
    proxy_connect_timeout 60s;
    proxy_send_timeout 60s;
    proxy_read_timeout 60s;

    # Proxy settings
    location / {
        proxy_pass http://evif_backend;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # CORS headers
        add_header Access-Control-Allow-Origin *;
        add_header Access-Control-Allow-Methods "GET, POST, PUT, DELETE, OPTIONS";
        add_header Access-Control-Allow-Headers "Content-Type, Authorization";
    }

    # WebSocket support
    location /ws {
        proxy_pass http://evif_backend;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }

    # Health check
    location /health {
        proxy_pass http://evif_backend/health;
        access_log off;
    }
}
```

#### Caddy

Create `Caddyfile`:

```caddyfile
evif.example.com {
    reverse_proxy localhost:8081

    # Automatic TLS
    tls {
        dns cloudflare
        resolvers 1.1.1.1
    }

    # Logging
    log {
        output file /var/log/caddy/evif.log
        format json
    }

    # Response headers
    header {
        Access-Control-Allow-Origin *
        Access-Control-Allow-Methods "GET, POST, PUT, DELETE, OPTIONS"
        Access-Control-Allow-Headers "Content-Type, Authorization"
        X-Frame-Options "DENY"
        X-Content-Type-Options "nosniff"
    }
}
```

### Load Balancing

#### HAProxy

Create `/etc/haproxy/haproxy.cfg`:

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

### High Availability Deployment

#### Master-Replica Setup

```bash
# Master node
evif-rest --role master --port 8081

# Replica node
evif-rest --role replica --master http://master:8081 --port 8082
```

#### Cluster Mode

```bash
# Node 1
evif-rest \
    --cluster-enabled \
    --cluster-node-id node1 \
    --cluster-addr 192.168.1.10:8081 \
    --cluster-peers node2=192.168.1.11:8082,node3=192.168.1.12:8083

# Node 2
evif-rest \
    --cluster-enabled \
    --cluster-node-id node2 \
    --cluster-addr 192.168.1.11:8082 \
    --cluster-peers node1=192.168.1.10:8081,node3=192.168.1.12:8083

# Node 3
evif-rest \
    --cluster-enabled \
    --cluster-node-id node3 \
    --cluster-addr 192.168.1.12:8083 \
    --cluster-peers node1=192.168.1.10:8081,node2=192.168.1.11:8082
```

### Backup Strategy

#### Data Backup Script

Create `/usr/local/bin/evif-backup.sh`:

```bash
#!/bin/bash

BACKUP_DIR="/backup/evif"
DATA_DIR="/var/lib/evif/data"
DATE=$(date +%Y%m%d_%H%M%S)

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Backup data
tar -czf "$BACKUP_DIR/evif-data-$DATE.tar.gz" -C "$DATA_DIR" .

# Backup config
cp /etc/evif/evif.toml "$BACKUP_DIR/evif-config-$DATE.toml"

# Delete backups older than 30 days
find "$BACKUP_DIR" -name "evif-*" -mtime +30 -delete

# Upload to S3 (optional)
aws s3 cp "$BACKUP_DIR/evif-data-$DATE.tar.gz" \
    s3://my-backup-bucket/evif/

echo "Backup completed: evif-data-$DATE.tar.gz"
```

Set executable permissions:

```bash
chmod +x /usr/local/bin/evif-backup.sh
```

Add to crontab:

```bash
# Run backup daily at 2 AM
0 2 * * * /usr/local/bin/evif-backup.sh
```

## Monitoring & Logging

### Log Configuration

#### Structured Logging

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

#### Log Levels

| Level | Description | Use Case |
|-------|-------------|----------|
| `trace` | Most verbose | Trace execution flow |
| `debug` | Debug info | Development and debugging |
| `info` | General info | Normal operation (recommended) |
| `warn` | Warnings | Potential issues |
| `error` | Errors | Error conditions |

### Log Collection

#### rsyslog Configuration

Create `/etc/rsyslog.d/evif.conf`:

```
# EVIF log collection
if $programname == 'evif' then /var/log/evif/evif.log
& stop
```

#### Filebeat Configuration

Create `/etc/filebeat/modules.d/evif.yml`:

```yaml
- module: evif
  log:
    enabled: true
    var.paths: ["/var/log/evif/*.log"]
    var.input_type: "log"

    # Parse JSON logs
    json.keys_under_root: true
    json.add_error_key: true

    # Field configuration
    fields:
      service: evif
      environment: production

    fields_under_root: true
```

### Monitoring Metrics

#### Prometheus Integration

EVIF provides Prometheus metrics endpoint:

```bash
# Enable metrics
evif-rest --metrics-enabled --metrics-port 9090

# Access metrics
curl http://localhost:9090/metrics
```

Prometheus configuration:

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'evif'
    scrape_interval: 15s
    static_configs:
      - targets: ['localhost:9090']
    metrics_path: '/metrics'
```

#### Grafana Dashboard

Create Grafana dashboard queries:

```promql
# Request rate
rate(evif_requests_total[5m])

# Error rate
rate(evif_errors_total[5m]) / rate(evif_requests_total[5m])

# Response time
histogram_quantile(0.95, rate(evif_request_duration_seconds_bucket[5m]))

# Memory usage
evif_memory_usage_bytes

# Active connections
evif_active_connections
```

### Health Checks

#### HTTP Health Check

```bash
# Basic health check
curl http://localhost:8081/health

# Response
{
  "status": "healthy",
  "timestamp": "2026-03-01T12:00:00.000000Z",
  "version": "0.1.0",
  "uptime_seconds": 3600,
  "memory_usage_mb": 128,
  "active_connections": 10
}
```

#### Readiness Check

```bash
curl http://localhost:8081/ready

# Response
{
  "ready": true,
  "checks": {
    "storage": "ok",
    "cache": "ok",
    "plugins": "ok"
  }
}
```

### Alerting Configuration

#### Alertmanager Rules

Create `alerts.yml`:

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

## Troubleshooting

### Common Issues

#### Service Won't Start

**Symptoms**

```bash
sudo systemctl start evif
# Service fails to start
```

**Diagnosis**

```bash
# Check service status
sudo systemctl status evif

# View logs
sudo journalctl -u evif -n 50

# Check port usage
sudo netstat -tulpn | grep 8081
# Or
sudo lsof -i :8081
```

**Solutions**

```bash
# 1. Port already in use
# Kill occupying process
sudo kill -9 <PID>

# Or change port
evif-rest --port 8082

# 2. Permission issues
sudo chown -R evif:evif /var/lib/evif
sudo chown -R evif:evif /var/log/evif

# 3. Configuration file error
evif-rest --validate-config /etc/evif/evif.toml
```

#### Performance Issues

**Symptoms**

- Slow responses
- High CPU/memory usage

**Diagnosis**

```bash
# View resource usage
top -p $(pidof evif-rest)

# Check memory usage
ps aux | grep evif-rest

# Profile performance
sudo perf top -p $(pidof evif-rest)
```

**Solutions**

```bash
# 1. Increase worker threads
evif-rest --workers 8

# 2. Limit cache size
export EVIF_CACHE_SIZE=52428800  # 50MB

# 3. Enable compression
export EVIF_COMPRESSION_ENABLED=true

# 4. Adjust Tokio runtime
export TOKIO_WORKER_THREADS=4
```

#### Memory Leaks

**Symptoms**

- Memory usage continuously grows
- Service eventually crashes

**Diagnosis**

```bash
# Monitor memory usage
watch -n 5 'ps aux | grep evif-rest'

# Use Valgrind to analyze
sudo valgrind --leak-check=full \
    --show-leak-kinds=all \
    evif-rest
```

**Solutions**

```bash
# 1. Set memory limit
sudo systemctl set-property evif MemoryMax=2G

# 2. Periodic restart (temporary fix)
# Add to crontab
0 3 * * * /usr/bin/systemctl restart evif

# 3. Upgrade to latest version
cargo install --path crates/evif-rest --force-reinstall
```

#### Storage Issues

**Symptoms**

- File operations fail
- Disk space exhausted

**Diagnosis**

```bash
# Check disk space
df -h /var/lib/evif

# Check file descriptor limits
ulimit -n

# View open files
sudo lsof -p $(pidof evif-rest) | wc -l
```

**Solutions**

```bash
# 1. Clean cache
rm -rf /var/lib/evif/cache/*

# 2. Increase file descriptor limit
# Edit /etc/systemd/system/evif.service
[Service]
LimitNOFILE=65536

# 3. Expand disk
# Depends on your environment
```

### Log Analysis

#### Error Logs

```bash
# View error logs
sudo journalctl -u evif -p err -n 100

# Search for specific errors
sudo journalctl -u evif | grep -i "error"

# Export error logs
sudo journalctl -u evif -p err > evif-errors.log
```

#### Performance Logs

```bash
# Enable performance logging
export RUST_LOG=evif=perf

# Analyze slow queries
sudo journalctl -u evif | grep "slow query"

# View response times
sudo journalctl -u evif | grep "duration"
```

### Debug Mode

```bash
# Enable debug logging
export RUST_LOG=debug
evif-rest

# Enable tracing
export RUST_LOG=trace
export RUST_BACKTRACE=1
evif-rest

# Use GDB
sudo gdb -p $(pidof evif-rest)
(gdb) bt  # View stack
(gdb) thread apply all bt  # View all thread stacks
```

## Upgrade & Migration

### Pre-Upgrade Preparation

#### Backup Data

```bash
# Stop service
sudo systemctl stop evif

# Backup data directory
sudo tar -czf /backup/evif-data-$(date +%Y%m%d).tar.gz \
    /var/lib/evif

# Backup configuration
sudo cp /etc/evif/evif.toml \
    /backup/evif-config-$(date +%Y%m%d).toml

# Verify backup
sudo tar -tzf /backup/evif-data-$(date +%Y%m%d).tar.gz | head -20
```

#### Check Compatibility

```bash
# View current version
evif --version

# View release notes
cat /path/to/evif/RELEASES.md
```

### Rolling Upgrade

#### Binary Upgrade

```bash
# 1. Download new version
wget https://github.com/evif/evif/releases/download/v0.2.0/evif-linux-amd64.tar.gz
tar -xzf evif-linux-amd64.tar.gz

# 2. Verify new version
./evif --version

# 3. Backup old version
sudo cp /usr/local/bin/evif-rest \
    /usr/local/bin/evif-rest.backup

# 4. Install new version
sudo cp evif-rest /usr/local/bin/
sudo chmod +x /usr/local/bin/evif-rest

# 5. Restart service
sudo systemctl restart evif

# 6. Verify upgrade
curl http://localhost:8081/health
sudo journalctl -u evif -n 50
```

#### Docker Upgrade

```bash
# 1. Pull new image
docker pull evif:0.2.0

# 2. Stop old container
docker stop evif
docker rm evif

# 3. Start new container
docker run -d \
  --name evif \
  -p 8081:8081 \
  -v evif-data:/var/lib/evif \
  evif:0.2.0

# 4. Verify upgrade
docker logs -f evif
```

### Canary Deployment

```bash
# Deploy new version to subset of instances
docker-compose up -d --scale evif=2

# Monitor new version
docker-compose ps
docker-compose logs -f evif

# Gradually increase new version instances
docker-compose up -d --scale evif=4

# Monitor metrics
curl http://localhost:9090/metrics
```

### Rollback Strategy

```bash
# Binary rollback
sudo systemctl stop evif
sudo cp /usr/local/bin/evif-rest.backup \
    /usr/local/bin/evif-rest
sudo systemctl start evif

# Docker rollback
docker stop evif
docker rm evif
docker run -d \
  --name evif \
  -p 8081:8081 \
  -v evif-data:/var/lib/evif \
  evif:0.1.0  # Old version

# Verify rollback
curl http://localhost:8081/health
```

### Data Migration

#### Migrate from Old Version

```bash
# Use migration tool
evif-migrate \
    --source /var/lib/evif/old \
    --destination /var/lib/evif/new \
    --format json
```

#### Migrate from Other Systems

```bash
# Import from filesystem
evif import --source /path/to/files --dest /imported

# Import from S3
evif import --source s3://my-bucket --dest /imported

# Verify import
evif ls /imported
```

## Summary

This chapter covered the complete deployment process for EVIF in production environments:

- ✅ Understood multiple deployment methods (binary, Docker, Systemd)
- ✅ Mastered configuration management techniques
- ✅ Implemented high availability and load balancing
- ✅ Set up monitoring and log collection
- ✅ Troubleshot common issues
- ✅ Performed smooth upgrades and migrations

## Next Steps

- **Chapter 10**: [Advanced Topics](./chapter-10-advanced-topics.md) - Deep dive into performance optimization, plugin development, and extension features
- **API Reference**: [Chapter 7](./chapter-7-api-reference.md) - Complete API documentation
- **Security Guide**: [Chapter 8](./chapter-8-authentication-security.md) - Authentication and security best practices

## References

- **Official Documentation**: https://docs.evif.io
- **GitHub Repository**: https://github.com/evif/evif
- **Docker Hub**: https://hub.docker.com/r/evif/evif
- **Community Support**: https://github.com/evif/evif/discussions
