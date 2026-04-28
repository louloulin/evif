# EVIF Deployment & Operations Guide

## 1. Deployment Options

### 1.1 Local Development

```bash
# Clone and build
git clone https://github.com/evif/evif
cd evif
cargo build --release

# Start server
EVIF_REST_AUTH_MODE=disabled ./target/release/evif-rest --port 8081
```

### 1.2 Docker

```bash
# Pull image (when published)
docker pull evif/evif:latest

# Run container
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

### 1.4 Production Docker

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

## 2. Configuration

### 2.1 Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `EVIF_REST_HOST` | `0.0.0.0` | Bind address |
| `EVIF_REST_PORT` | `8081` | HTTP port |
| `EVIF_REST_AUTH_MODE` | `api-key` | Auth mode: disabled, api-key, capability |
| `EVIF_API_KEY` | - | API key for authentication |
| `EVIF_LOG_DIR` | `logs` | Log directory |
| `EVIF_CONFIG_FILE` | - | Config file path |
| `EVIF_MOUNTS` | - | Mount configuration |
| `EVIF_TLS_CERT_FILE` | - | TLS certificate |
| `EVIF_TLS_KEY_FILE` | - | TLS private key |
| `EVIF_METRICS_ENABLED` | `false` | Enable Prometheus metrics |
| `EVIF_PROMETHEUS_PORT` | `9090` | Metrics port |

### 2.2 Config File

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

### 2.3 Startup Command

```bash
# Basic
./evif-rest --port 8081

# With config file
./evif-rest --config /etc/evif/evif.toml

# With TLS
./evif-rest \
  --port 8081 \
  --tls-cert /etc/evif/cert.pem \
  --tls-key /etc/evif/key.pem

# Production
./evif-rest \
  --config /etc/evif/evif.toml \
  --production \
  --auth-mode api-key
```

## 3. Health Checks

### 3.1 Health Endpoint

```bash
curl http://localhost:8081/api/v1/health
```

**Response**:
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime": 3600
}
```

### 3.2 Readiness Endpoint

```bash
curl http://localhost:8081/api/v1/readiness
```

### 3.3 Docker Healthcheck

```dockerfile
HEALTHCHECK --interval=30s --timeout=10s --retries=3 \
  CMD curl -f http://localhost:8081/api/v1/health || exit 1
```

## 4. Monitoring

### 4.1 Prometheus Metrics

```bash
# Enable metrics
export EVIF_METRICS_ENABLED=true
export EVIF_PROMETHEUS_PORT=9090

# Scrape config
cat <<EOF > /etc/prometheus/scrape_configs.yml
- job_name: 'evif'
  static_configs:
    - targets: ['localhost:9090']
  metrics_path: '/api/v1/metrics'
EOF
```

### 4.2 Available Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `evif_http_requests_total` | Counter | Total HTTP requests |
| `evif_http_request_duration_seconds` | Histogram | Request latency |
| `evif_http_requests_in_flight` | Gauge | Concurrent requests |
| `evif_file_operations_total` | Counter | File operations by type |
| `evif_file_operation_duration_seconds` | Histogram | File operation latency |
| `evif_cache_hits_total` | Counter | Cache hits |
| `evif_cache_misses_total` | Counter | Cache misses |
| `evif_handles_open` | Gauge | Open file handles |
| `evif_mounts_total` | Gauge | Number of mounts |

### 4.3 Grafana Dashboard

Import from `docs/grafana/evif-dashboard.json`:

```bash
# Or via API
curl -X POST http://localhost:3000/api/dashboards/db \
  -H "Content-Type: application/json" \
  -d @docs/grafana/evif-dashboard.json
```

## 5. Logging

### 5.1 Log Format

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

### 5.2 Log Levels

| Level | Use Case |
|-------|----------|
| `error` | Errors requiring attention |
| `warn` | Warnings (e.g., deprecated usage) |
| `info` | Normal operations |
| `debug` | Detailed debugging info |
| `trace` | Very detailed tracing |

### 5.3 Log Rotation

Logs rotate daily. Configure in production:

```bash
# Use external log rotation
mv logs/evif-2026-04-27.log /var/log/evif/
```

## 6. Security

### 6.1 Authentication Modes

**Disabled** (Development):
```bash
EVIF_REST_AUTH_MODE=disabled
```

**API Key** (Production):
```bash
EVIF_REST_AUTH_MODE=api-key
EVIF_API_KEY=your-secret-key
```

**Capability-based** (Enterprise):
```bash
EVIF_REST_AUTH_MODE=capability
```

### 6.2 TLS Configuration

```bash
# Generate self-signed cert
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes

# Start with TLS
./evif-rest \
  --port 8443 \
  --tls-cert cert.pem \
  --tls-key key.pem
```

### 6.3 Rate Limiting

Default limits per client:
- Read operations: 1000/min
- Write operations: 100/min
- Memory operations: 100/min

### 6.4 CORS Configuration

```bash
# Allow all origins (dev)
EVIF_CORS_ENABLED=true

# Restrict origins
EVIF_CORS_ENABLED=true
EVIF_CORS_ORIGINS=https://app.example.com,https://admin.example.com
```

## 7. Backup & Recovery

### 7.1 Data Directories

| Directory | Purpose | Backup Frequency |
|-----------|---------|-----------------|
| `/data` | File storage | Daily |
| `/logs` | Application logs | Weekly |
| `/context` | Context data | Daily |
| `/mem` | In-memory (no backup) | - |

### 7.2 Backup Script

```bash
#!/bin/bash
# backup-evif.sh

BACKUP_DIR="/backups/evif"
DATE=$(date +%Y-%m-%d)

mkdir -p $BACKUP_DIR/$DATE

# Backup file storage
tar -czf $BACKUP_DIR/$DATE/data.tar.gz /var/evif/data

# Backup context
tar -czf $BACKUP_DIR/$DATE/context.tar.gz /var/evif/context

# Backup config
cp /etc/evif/evif.toml $BACKUP_DIR/$DATE/

# Cleanup old backups (keep 7 days)
find $BACKUP_DIR -type d -mtime +7 -exec rm -rf {} \;

echo "Backup completed: $DATE"
```

### 7.3 Restore

```bash
# Stop service
systemctl stop evif

# Restore data
tar -xzf /backups/evif/2026-04-27/data.tar.gz -C /

# Restore context
tar -xzf /backups/evif/2026-04-27/context.tar.gz -C /

# Start service
systemctl start evif
```

## 8. Performance Tuning

### 8.1 Worker Count

```toml
[server]
workers = 4  # Match CPU cores
```

### 8.2 Connection Pooling

```toml
[server]
max_connections = 1000
```

### 8.3 Cache Configuration

```toml
[cache]
enabled = true
max_entries = 10000
ttl_seconds = 300
```

### 8.4 Resource Limits

```dockerfile
# Limit CPU and memory
docker run -d \
  --cpus=2 \
  --memory=2g \
  evif/evif:latest
```

## 9. Troubleshooting

### 9.1 Server Won't Start

```bash
# Check port availability
lsof -i :8081

# Check logs
tail -f logs/evif.log

# Verify config
./evif-rest --config /etc/evif/evif.toml --validate
```

### 9.2 Slow Performance

```bash
# Check metrics
curl http://localhost:8081/api/v1/metrics

# Check cache hit rate
curl http://localhost:8081/api/v1/metrics/cache

# Enable debug logging
RUST_LOG=debug ./evif-rest
```

### 9.3 Out of Memory

```bash
# Monitor memory
watch -n 1 'ps aux | grep evif'

# Check handle count
curl http://localhost:8081/api/v1/handles | jq '.handles | length'

# Close idle handles
curl -X POST http://localhost:8081/api/v1/handles/cleanup
```

### 9.4 Disk Full

```bash
# Check disk usage
df -h

# Find large files
find /var/evif -type f -size +100M

# Clean old logs
rm -rf /var/evif/logs/*
```

## 10. Kubernetes Deployment

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

## 11. Related Documents

- [Getting Started](GETTING_STARTED.md)
- [Production Deployment](production-deployment.md)
- [Metrics Guide](metrics.md)
- [REST API Reference](03-rest-api.md)
