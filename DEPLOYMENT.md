# EVIF 1.8 生产部署指南

**版本**: 1.8.0
**更新日期**: 2025-01-25

---

## 📋 目录

1. [系统要求](#系统要求)
2. [快速部署](#快速部署)
3. [配置管理](#配置管理)
4. [性能优化](#性能优化)
5. [监控告警](#监控告警)
6. [故障排除](#故障排除)
7. [最佳实践](#最佳实践)

---

## 系统要求

### 最低配置

| 资源 | 最低要求 | 推荐配置 |
|------|---------|---------|
| CPU | 2核 | 4核+ |
| 内存 | 2GB | 8GB+ |
| 磁盘 | 10GB | 50GB+ SSD |
| 操作系统 | Linux/macOS/Windows | Linux (Ubuntu 20.04+) |
| Rust | 1.70+ | 1.75+ |

### 依赖服务

- **PostgreSQL** (可选，用于持久化)
- **Redis** (可选，用于分布式缓存)
- **S3兼容存储** (可选，用于对象存储)
- **向量数据库** (可选，用于向量搜索)

---

## 快速部署

### 方式1: 使用预编译二进制

```bash
# 下载最新版本
curl -sSL https://github.com/evif/evif/releases/latest/download/evif-linux-amd64.tar.gz | tar xz

# 安装
sudo mv evif /usr/local/bin/
sudo chmod +x /usr/local/bin/evif

# 验证安装
evif --version
```

### 方式2: 从源码编译

```bash
# 克隆仓库
git clone https://github.com/evif/evif.git
cd evif

# 编译发布版本
cargo build --release

# 安装
sudo cp target/release/evif /usr/local/bin/
sudo cp target/release/evifd /usr/local/bin/
```

### 方式3: Docker部署

```bash
# 拉取镜像
docker pull evif/evif:1.8.0

# 运行服务器
docker run -d \
  --name evif-server \
  -p 8080:8080 \
  -v /data/evif:/data \
  evif/evif:1.8.0

# 查看日志
docker logs -f evif-server
```

---

## 配置管理

### 1. 基础配置文件

创建 `/etc/evif/config.toml`:

```toml
[server]
bind_address = "0.0.0.0"
port = 8080
timeout_secs = 30
max_connections = 10000
worker_threads = 8

[plugins]
plugins_dir = "/usr/local/lib/evif/plugins"

[plugins.auto_mount]
{ plugin = "memfs", path = "/memfs" }
{ plugin = "localfs", path = "/local", config = { root = "/data/evif" } }

[cache]
enabled = true
metadata_ttl_secs = 60
directory_ttl_secs = 30
max_entries = 100000

[logging]
level = "info"
format = "json"

[logging.file]
path = "/var/log/evif/evif.log"

[logging.file.rotation]
max_size_mb = 100
max_files = 30

[security.tls]
enabled = true
cert_path = "/etc/evif/certs/server.crt"
key_path = "/etc/evif/certs/server.key"
```

### 2. 环境变量配置

```bash
# 服务器配置
export EVIF_BIND_ADDRESS=0.0.0.0
export EVIF_PORT=8080
export EVIF_TIMEOUT=30
export EVIF_MAX_CONNECTIONS=10000

# 缓存配置
export EVIF_CACHE_ENABLED=true
export EVIF_CACHE_MAX_ENTRIES=100000

# 日志配置
export EVIF_LOG_LEVEL=info
export EVIF_LOG_FORMAT=json

# 插件配置
export EVIF_PLUGINS_DIR=/usr/local/lib/evif/plugins
```

### 3. systemd服务配置

创建 `/etc/systemd/system/evif.service`:

```ini
[Unit]
Description=EVIF File System Server
After=network.target

[Service]
Type=simple
User=evif
Group=evif
WorkingDirectory=/var/lib/evif
ExecStart=/usr/local/bin/evifd --config /etc/evif/config.toml
Restart=always
RestartSec=5s
StandardOutput=journal
StandardError=journal
SyslogIdentifier=evif

# 资源限制
LimitNOFILE=65536
LimitNPROC=4096

# 安全加固
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true

[Install]
WantedBy=multi-user.target
```

启动服务:

```bash
# 重载systemd配置
sudo systemctl daemon-reload

# 启用开机自启
sudo systemctl enable evif

# 启动服务
sudo systemctl start evif

# 查看状态
sudo systemctl status evif

# 查看日志
sudo journalctl -u evif -f
```

---

## 性能优化

### 1. 缓存优化

```toml
[cache]
enabled = true
metadata_ttl_secs = 120      # 元数据缓存2分钟
directory_ttl_secs = 60     # 目录缓存1分钟
max_entries = 500000         # 增加到50万条
```

**效果**: 减少50%+的文件系统调用

### 2. 连接池优化

```toml
[server]
max_connections = 10000      # 最大连接数
worker_threads = 16           # 增加worker线程

[performance]
connection_pool_size = 100
max_idle_connections = 20
```

### 3. 批量操作优化

使用批量API而非单个操作:

```bash
# ❌ 低效: 单个操作
for file in *.txt; do
  evif write "/s3/bucket/$file" "$file"
done

# ✅ 高效: 批量操作
evif batch write /s3/bucket/ *.txt
```

### 4. 插件配置优化

```toml
# S3优化
[plugins.plugin_configs.s3fs]
max_connections = 100
part_size = 8388608  # 8MB分块
timeout_secs = 300
retry_max_attempts = 3

# VectorFS优化
[plugins.plugin_configs.vectorfs]
embedding_batch_size = 100
index_refresh_interval = 3600
```

---

## 监控告警

### Prometheus集成

EVIF内置Prometheus指标，在配置文件中启用:

```toml
[monitoring]
prometheus_enabled = true
prometheus_port = 9090
metrics_path = "/metrics"
```

访问指标: `http://localhost:9090/metrics`

**关键指标**:

```prometheus
# 请求总数
evif_requests_total

# 请求延迟
evif_request_latency_seconds

# 活动连接数
evif_active_connections

# 错误率
evif_errors_total

# 队列深度
evif_queue_depth
```

### Grafana Dashboard

导入Grafana面板配置:

```json
{
  "dashboard": {
    "title": "EVIF File System",
    "panels": [
      {
        "title": "Request Rate",
        "targets": [
          {
            "expr": "rate(evif_requests_total[5m])"
          }
        ]
      },
      {
        "title": "Latency P99",
        "targets": [
          {
            "expr": "histogram_quantile(0.99, evif_request_latency_seconds)"
          }
        ]
      }
    ]
  }
}
```

### 告警规则

Prometheus告警示例:

```yaml
groups:
  - name: evif_alerts
    interval: 30s
    rules:
      - alert: HighErrorRate
        expr: rate(evif_errors_total[5m]) > 0.05
        for: 5m
        annotations:
          summary: "Error rate above 5%"

      - alert: HighLatency
        expr: histogram_quantile(0.99, evif_request_latency_seconds) > 1
        for: 5m
        annotations:
          summary: "P99 latency above 1 second"

      - alert: QueueBacklog
        expr: evif_queue_depth{queue="tasks"} > 10000
        for: 10m
        annotations:
          summary: "Task queue backlog > 10000"
```

---

## 故障排除

### 常见问题

#### 1. 服务无法启动

**症状**: `evifd`启动失败

**排查**:
```bash
# 检查配置文件
evifd --config /etc/evif/config.toml --validate

# 检查端口占用
sudo lsof -i :8080

# 检查日志
sudo journalctl -u evif -n 50
```

#### 2. 内存占用过高

**症状**: 内存使用持续增长

**排查**:
```bash
# 查看内存使用
evif stats memory

# 重启服务
sudo systemctl restart evif
```

**解决**:
```toml
[cache]
max_entries = 50000  # 减少缓存条目
```

#### 3. 插件加载失败

**症状**: 无法挂载插件

**排查**:
```bash
# 检查插件目录
ls -la /usr/local/lib/evif/plugins/

# 检查插件日志
evif health --plugins
```

#### 4. 性能下降

**症状**: 响应时间变慢

**排查**:
```bash
# 查看性能统计
evif stats performance

# 查看慢查询
evif stats slow_queries
```

**解决**:
- 增加缓存
- 启用批量操作
- 调整worker线程数

---

## 最佳实践

### 1. 安全配置

```toml
[security]
enabled = true
api_keys = ["key1", "key2"]

[security.tls]
enabled = true
min_version = "1.2"
cipher_suites = ["TLS_AES_256_GCM_SHA384"]

[security.cors]
allowed_origins = ["https://trusted-domain.com"]
allowed_methods = ["GET", "POST", "PUT", "DELETE"]
allowed_headers = ["Content-Type", "Authorization"]
```

### 2. 备份策略

```bash
#!/bin/bash
# backup.sh - 每日备份脚本

BACKUP_DIR=/backup/evif/$(date +%Y%m%d)
mkdir -p "$BACKUP_DIR"

# 备份配置
cp /etc/evif/config.toml "$BACKUP_DIR/"

# 备份插件
tar -czf "$BACKUP_DIR/plugins.tar.gz" /usr/local/lib/evif/plugins/

# 备份数据
evif download /s3/bucket/* "$BACKUP_DIR/data/"

# 保留最近30天
find /backup/evif -mtime +30 -exec rm -rf {} \;
```

### 3. 高可用部署

**负载均衡配置**:

```nginx
upstream evif_backend {
    server evif1.example.com:8080 weight=3;
    server evif2.example.com:8080 weight=2;
    server evif3.example.com:8080 weight=1;

    keepalive 32;
}

server {
    listen 80;

    location / {
        proxy_pass http://evif_backend;
        proxy_http_version 1.1;
        proxy_set_header Connection "";
    }
}
```

**健康检查**:

```bash
#!/bin/bash
# health_check.sh

while true; do
  for host in evif1 evif2 evif3; do
    if ! curl -f "http://${host}:8080/health"; then
      echo "Alert: $host is down!"
      # 发送告警
    fi
  done
  sleep 60
done
```

### 4. 容量规划

| 并发请求数 | CPU | 内存 | 带宽 |
|-----------|-----|------|------|
| 1000 | 2核 | 4GB | 100Mbps |
| 10000 | 8核 | 16GB | 1Gbps |
| 100000 | 32核 | 64GB | 10Gbps |

### 5. 日志管理

```bash
# 日志轮转
/etc/logrotate.d/evif:

/var/log/evif/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 0640 evif evif
    postrotate
        systemctl reload evif > /dev/null 2>&1 || true
    endscript
}
```

---

## 升级指南

### 滚动升级（零停机）

```bash
# 1. 下载新版本
curl -sSL https://github.com/evif/evif/releases/download/v1.8.1/evif-linux-amd64.tar.gz | tar xz

# 2. 备份当前版本
sudo cp /usr/local/bin/evif /usr/local/bin/evif.backup

# 3. 更新二进制
sudo mv evif /usr/local/bin/

# 4. 热重载配置（如果支持）
sudo systemctl reload evif

# 5. 验证新版本
evif --version
curl http://localhost:8080/health
```

---

## 性能基准

### 吞吐量测试

使用wrk进行基准测试:

```bash
# GET请求测试
wrk -t12 -c400 -d30s http://localhost:8080/api/v1/files?path=/test.txt

# PUT请求测试
wrk -t12 -c400 -d30s -s POST http://localhost:8080/api/v1/files?path=/test.txt \
  --body='{"data":"test content"}'
```

### 预期性能

| 操作 | 吞吐量 | P99延迟 |
|------|--------|---------|
| 文件读取 | 10,000 ops/s | <10ms |
| 文件写入 | 5,000 ops/s | <20ms |
| 目录列出 | 8,000 ops/s | <15ms |
| 队列入队 | 20,000 ops/s | <5ms |
| 向量搜索 | 1,000 ops/s | <50ms |

---

## 总结

✅ **生产就绪**: EVIF 1.8已达到生产级别质量
✅ **高性能**: 支持10,000+并发连接
✅ **可监控**: 完整的Prometheus指标
✅ **易部署**: 支持多种部署方式
✅ **可扩展**: 插件化架构，易于扩展

**推荐部署方案**:
- 开发环境: Docker Compose
- 测试环境: Kubernetes
- 生产环境: 裸金属 + 负载均衡

---

**文档版本**: 1.8.0
**最后更新**: 2025-01-25
