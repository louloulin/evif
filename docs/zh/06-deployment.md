# EVIF 部署与运维指南

## 1. 部署选项

### 1.1 本地开发

```bash
# 克隆并构建
git clone https://github.com/evif/evif
cd evif
cargo build --release

# 启动服务器
EVIF_REST_AUTH_MODE=disabled ./target/release/evif-rest --port 8081
```

### 1.2 Docker

```bash
# 拉取镜像 (发布后)
docker pull evif/evif:latest

# 运行容器
docker run -d \
  --name evif \
  -p 8081:8081 \
  -e EVIF_REST_AUTH_MODE=disabled \
  evif/evif:latest
```

### 1.3 Docker Compose

```yaml
# docker-compose.yml
version: '3.8'

services:
  evif:
    image: evif/evif:latest
    ports:
      - "8081:8081"
    environment:
      - EVIF_REST_AUTH_MODE=disabled
      - EVIF_LOG_DIR=/data/logs
    volumes:
      - evif-data:/data
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8081/api/v1/health"]
      interval: 30s
      timeout: 10s
      retries: 3

volumes:
  evif-data:
```

### 1.4 生产环境 Docker

```yaml
# docker-compose.prod.yml
version: '3.8'

services:
  evif:
    image: evif/evif:latest
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
        reservations:
          cpus: '0.5'
          memory: 512M
    ports:
      - "8081:8081"
    environment:
      - EVIF_REST_AUTH_MODE=api-key
      - EVIF_API_KEY=${EVIF_API_KEY}
      - EVIF_LOG_DIR=/data/logs
      - EVIF_METRICS_ENABLED=true
      - EVIF_PROMETHEUS_PORT=9090
    volumes:
      - evif-data:/data
    restart: unless-stopped
```

## 2. 配置

### 2.1 环境变量

| 变量 | 默认值 | 描述 |
|------|--------|------|
| `EVIF_REST_HOST` | `0.0.0.0` | 绑定地址 |
| `EVIF_REST_PORT` | `8081` | HTTP 端口 |
| `EVIF_REST_AUTH_MODE` | `api-key` | 认证模式: disabled, api-key, capability |
| `EVIF_API_KEY` | - | 认证 API 密钥 |
| `EVIF_LOG_DIR` | `logs` | 日志目录 |
| `EVIF_CONFIG_FILE` | - | 配置文件路径 |
| `EVIF_MOUNTS` | - | 挂载配置 |
| `EVIF_TLS_CERT_FILE` | - | TLS 证书 |
| `EVIF_TLS_KEY_FILE` | - | TLS 私钥 |
| `EVIF_METRICS_ENABLED` | `false` | 启用 Prometheus 指标 |
| `EVIF_PROMETHEUS_PORT` | `9090` | 指标端口 |

### 2.2 配置文件

```toml
# evif.toml
[server]
host = "0.0.0.0"
port = 8081
workers = 4
max_connections = 1000
request_timeout = "30s"

[auth]
mode = "capability"
api_key = "your-key-here"

[[mounts]]
path = "/mem"
plugin = "memfs"

[[mounts]]
path = "/context"
plugin = "contextfs"

[[mounts]]
path = "/skills"
plugin = "skillfs"

[[mounts]]
path = "/pipes"
plugin = "pipefs"

[[mounts]]
path = "/data"
plugin = "localfs"
config = { root = "/var/evif/data" }
```

### 2.3 启动命令

```bash
# 基本
./evif-rest --port 8081

# 使用配置文件
./evif-rest --config /etc/evif/evif.toml

# 使用 TLS
./evif-rest \
  --port 8081 \
  --tls-cert /etc/evif/cert.pem \
  --tls-key /etc/evif/key.pem

# 生产环境
./evif-rest \
  --config /etc/evif/evif.toml \
  --production \
  --auth-mode api-key
```

## 3. 健康检查

### 3.1 健康端点

```bash
curl http://localhost:8081/api/v1/health
```

**响应**:
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime": 3600
}
```

### 3.2 就绪端点

```bash
curl http://localhost:8081/api/v1/readiness
```

### 3.3 Docker Healthcheck

```dockerfile
HEALTHCHECK --interval=30s --timeout=10s --retries=3 \
  CMD curl -f http://localhost:8081/api/v1/health || exit 1
```

## 4. 监控

### 4.1 Prometheus 指标

```bash
# 启用指标
export EVIF_METRICS_ENABLED=true
export EVIF_PROMETHEUS_PORT=9090

# 抓取配置
cat <<EOF > /etc/prometheus/scrape_configs.yml
- job_name: 'evif'
  static_configs:
    - targets: ['localhost:9090']
  metrics_path: '/api/v1/metrics'
EOF
```

### 4.2 可用指标

| 指标 | 类型 | 描述 |
|------|------|------|
| `evif_http_requests_total` | Counter | HTTP 请求总数 |
| `evif_http_request_duration_seconds` | Histogram | 请求延迟 |
| `evif_http_requests_in_flight` | Gauge | 并发请求数 |
| `evif_file_operations_total` | Counter | 按类型分类的文件操作 |
| `evif_file_operation_duration_seconds` | Histogram | 文件操作延迟 |
| `evif_cache_hits_total` | Counter | 缓存命中 |
| `evif_cache_misses_total` | Counter | 缓存未命中 |
| `evif_handles_open` | Gauge | 打开的文件句柄 |
| `evif_mounts_total` | Gauge | 挂载数量 |

### 4.3 Grafana Dashboard

从 `docs/grafana/evif-dashboard.json` 导入：

```bash
# 或通过 API
curl -X POST http://localhost:3000/api/dashboards/db \
  -H "Content-Type: application/json" \
  -d @docs/grafana/evif-dashboard.json
```

## 5. 日志

### 5.1 日志格式

```json
{
  "timestamp": "2026-04-27T10:00:00Z",
  "level": "INFO",
  "target": "evif_rest",
  "message": "Request completed",
  "request_id": "uuid",
  "method": "GET",
  "path": "/api/v1/health",
  "status": 200,
  "duration_ms": 2
}
```

### 5.2 日志级别

| 级别 | 用途 |
|------|------|
| `error` | 需要关注的错误 |
| `warn` | 警告 (如已弃用) |
| `info` | 正常运行 |
| `debug` | 详细调试信息 |
| `trace` | 非常详细的追踪 |

### 5.3 日志轮转

日志每天轮转。生产环境配置：

```bash
# 使用外部日志轮转
mv logs/evif-2026-04-27.log /var/log/evif/
```

## 6. 安全

### 6.1 认证模式

**禁用** (开发):
```bash
EVIF_REST_AUTH_MODE=disabled
```

**API 密钥** (生产):
```bash
EVIF_REST_AUTH_MODE=api-key
EVIF_API_KEY=your-secret-key
```

**基于 Capability** (企业):
```bash
EVIF_REST_AUTH_MODE=capability
```

### 6.2 TLS 配置

```bash
# 生成自签名证书
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes

# 使用 TLS 启动
./evif-rest \
  --port 8443 \
  --tls-cert cert.pem \
  --tls-key key.pem
```

### 6.3 速率限制

默认每客户端限制：
- 读操作: 1000/分钟
- 写操作: 100/分钟
- 内存操作: 100/分钟

### 6.4 CORS 配置

```bash
# 允许所有来源 (开发)
EVIF_CORS_ENABLED=true

# 限制来源
EVIF_CORS_ENABLED=true
EVIF_CORS_ORIGINS=https://app.example.com,https://admin.example.com
```

## 7. 备份与恢复

### 7.1 数据目录

| 目录 | 用途 | 备份频率 |
|------|------|----------|
| `/data` | 文件存储 | 每天 |
| `/logs` | 应用日志 | 每周 |
| `/context` | 上下文数据 | 每天 |
| `/mem` | 内存 (不备份) | - |

### 7.2 备份脚本

```bash
#!/bin/bash
# backup-evif.sh

BACKUP_DIR="/backups/evif"
DATE=$(date +%Y-%m-%d)

mkdir -p $BACKUP_DIR/$DATE

# 备份文件存储
tar -czf $BACKUP_DIR/$DATE/data.tar.gz /var/evif/data

# 备份上下文
tar -czf $BACKUP_DIR/$DATE/context.tar.gz /var/evif/context

# 备份配置
cp /etc/evif/evif.toml $BACKUP_DIR/$DATE/

# 清理旧备份 (保留 7 天)
find $BACKUP_DIR -type d -mtime +7 -exec rm -rf {} \;

echo "备份完成: $DATE"
```

### 7.3 恢复

```bash
# 停止服务
systemctl stop evif

# 恢复数据
tar -xzf /backups/evif/2026-04-27/data.tar.gz -C /

# 恢复上下文
tar -xzf /backups/evif/2026-04-27/context.tar.gz -C /

# 启动服务
systemctl start evif
```

## 8. 性能调优

### 8.1 Worker 数量

```toml
[server]
workers = 4  # 匹配 CPU 核心数
```

### 8.2 连接池

```toml
[server]
max_connections = 1000
```

### 8.3 缓存配置

```toml
[cache]
enabled = true
max_entries = 10000
ttl_seconds = 300
```

### 8.4 资源限制

```dockerfile
# 限制 CPU 和内存
docker run -d \
  --cpus=2 \
  --memory=2g \
  evif/evif:latest
```

## 9. 故障排查

### 9.1 服务器无法启动

```bash
# 检查端口可用性
lsof -i :8081

# 检查日志
tail -f logs/evif.log

# 验证配置
./evif-rest --config /etc/evif/evif.toml --validate
```

### 9.2 性能缓慢

```bash
# 检查指标
curl http://localhost:8081/api/v1/metrics

# 检查缓存命中率
curl http://localhost:8081/api/v1/metrics/cache

# 启用调试日志
RUST_LOG=debug ./evif-rest
```

### 9.3 内存不足

```bash
# 监控内存
watch -n 1 'ps aux | grep evif'

# 检查句柄数
curl http://localhost:8081/api/v1/handles | jq '.handles | length'

# 清理空闲句柄
curl -X POST http://localhost:8081/api/v1/handles/cleanup
```

### 9.4 磁盘满

```bash
# 检查磁盘使用
df -h

# 查找大文件
find /var/evif -type f -size +100M

# 清理旧日志
rm -rf /var/evif/logs/*
```

## 10. Kubernetes 部署

### 10.1 Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: evif
  labels:
    app: evif
spec:
  replicas: 3
  selector:
    matchLabels:
      app: evif
  template:
    metadata:
      labels:
        app: evif
    spec:
      containers:
      - name: evif
        image: evif/evif:latest
        ports:
        - containerPort: 8081
        env:
        - name: EVIF_REST_AUTH_MODE
          value: "api-key"
        - name: EVIF_API_KEY
          valueFrom:
            secretKeyRef:
              name: evif-secrets
              key: api-key
        resources:
          limits:
            cpu: "2"
            memory: 2Gi
          requests:
            cpu: 500m
            memory: 512Mi
        livenessProbe:
          httpGet:
            path: /api/v1/health
            port: 8081
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /api/v1/readiness
            port: 8081
          initialDelaySeconds: 5
          periodSeconds: 10
```

### 10.2 Service

```yaml
apiVersion: v1
kind: Service
metadata:
  name: evif
spec:
  selector:
    app: evif
  ports:
  - port: 80
    targetPort: 8081
  type: ClusterIP
```

### 10.3 Horizontal Pod Autoscaler

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: evif-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: evif
  minReplicas: 2
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
```

## 11. 相关文档

- [快速开始](../GETTING_STARTED.md)
- [生产部署](../production-deployment.md)
- [指标指南](../metrics.md)
- [REST API 参考](03-rest-api.md)