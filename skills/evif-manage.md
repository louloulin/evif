---
name: evif-manage
description: "EVIF 插件管理技能 - 挂载、卸载、配置插件"
parent: "evif"
tags: ["evif", "plugin-management", "mount", "filesystem"]
trigger_keywords: ["evif mount", "evif unmount", "evif插件", "挂载", "卸载"]
---

# EVIF 插件管理

本文档详细介绍 EVIF 的插件挂载、卸载和配置管理。

## 挂载插件

### 基础语法

```bash
evif mount <plugin-name> <mount-point> [options]
```

### 参数说明

- `plugin-name`: 插件名称 (localfs, s3fs, vectorfs, etc.)
- `mount-point`: 挂载路径 (如 /local, /s3, /vector)
- `options`: 插件特定配置参数

## 插件挂载示例

### 1. 本地文件系统 (localfs)

**基础挂载:**
```bash
evif mount localfs /local --storage-path=/home/user/data
```

**只读模式:**
```bash
evif mount localfs /local --storage-path=/home/user/data --read-only=true
```

**多路径挂载:**
```bash
evif mount localfs /local-data --storage-path=/mnt/data
evif mount localfs /local-backup --storage-path=/mnt/backup
```

### 2. S3 兼容存储 (s3fs)

**AWS S3:**
```bash
evif mount s3fs /s3 \
  --region=us-east-1 \
  --bucket=my-bucket \
  --access-key=AKIAIOSFODNN7EXAMPLE \
  --secret-key=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
```

**MinIO (兼容 S3):**
```bash
evif mount s3fs /minio \
  --region=us-east-1 \
  --bucket=my-bucket \
  --access-key=minioadmin \
  --secret-key=minioadmin \
  --endpoint=http://localhost:9000 \
  --force-path-style=true
```

**使用环境变量:**
```bash
export AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE
export AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
export AWS_DEFAULT_REGION=us-east-1

evif mount s3fs /s3 --bucket=my-bucket
```

**S3 缓存配置:**
```bash
evif mount s3fs /s3 \
  --bucket=my-bucket \
  --region=us-east-1 \
  --cache-enabled=true \
  --cache-ttl-dir=30 \
  --cache-ttl-stat=60 \
  --cache-max-size=1000
```

### 3. 向量文件系统 (vectorfs)

**基础配置:**
```bash
evif mount vectorfs /vector \
  --s3-bucket=vector-docs \
  --tidb-host=localhost:4000 \
  --tidb-user=root \
  --tidb-password=yourpassword \
  --openai-api-key=sk-...
```

**完整配置:**
```bash
evif mount vectorfs /vector \
  --s3-bucket=vector-docs \
  --s3-region=us-east-1 \
  --tidb-host=localhost:4000 \
  --tidb-port=4000 \
  --tidb-user=root \
  --tidb-password=yourpassword \
  --tidb-database=vectorfs \
  --openai-api-key=sk-... \
  --embedding-model=text-embedding-3-small \
  --embedding-dimensions=1536 \
  --chunk-size=1000 \
  --chunk-overlap=200 \
  --index-workers=4
```

**使用其他嵌入服务:**
```bash
evif mount vectorfs /vector \
  --s3-bucket=vector-docs \
  --tidb-host=localhost:4000 \
  --embedding-provider=custom \
  --embedding-endpoint=http://localhost:8080/embeddings \
  --embedding-dimensions=768
```

### 4. 消息队列 (queuefs)

**内存后端:**
```bash
evif mount queuefs /queue --backend=memory
```

**SQLite 后端:**
```bash
evif mount queuefs /queue \
  --backend=sqlite \
  --db-path=/var/lib/evif/queue.db
```

**TiDB 后端 (生产环境):**
```bash
evif mount queuefs /queue \
  --backend=tidb \
  --tidb-host=localhost:4000 \
  --tidb-user=root \
  --tidb-password=yourpassword \
  --tidb-database=queuefs
```

**队列配置:**
```bash
evif mount queuefs /queue \
  --backend=sqlite \
  --db-path=/var/lib/evif/queue.db \
  --max-queue-size=10000 \
  --message-ttl=3600 \
  --enable-persistence=true
```

### 5. 内存文件系统 (memfs)

**基础挂载:**
```bash
evif mount memfs /mem
```

**限制大小:**
```bash
evif mount memfs /mem --max-size=1GB
```

### 6. SQL 文件系统 (sqlfs)

**SQLite:**
```bash
evif mount sqlfs /sql \
  --db-type=sqlite \
  --db-path=/var/lib/evif/sqlfs.db
```

**PostgreSQL:**
```bash
evif mount sqlfs /sql \
  --db-type=postgres \
  --host=localhost \
  --port=5432 \
  --database=sqlfs \
  --user=postgres \
  --password=yourpassword
```

**MySQL:**
```bash
evif mount sqlfs /sql \
  --db-type=mysql \
  --host=localhost \
  --port=3306 \
  --database=sqlfs \
  --user=root \
  --password=yourpassword
```

### 7. GPT 文件系统 (gptfs)

**OpenAI:**
```bash
evif mount gptfs /gpt \
  --api-key=sk-... \
  --model=gpt-4 \
  --max-concurrent-jobs=5
```

**使用 Azure OpenAI:**
```bash
evif mount gptfs /gpt \
  --api-key=your-azure-key \
  --endpoint=https://your-resource.openai.azure.com \
  --deployment-name=gpt-4 \
  --api-version=2024-02-15-preview
```

### 8. HTTP 文件系统 (httpfs)

**基础挂载:**
```bash
evif mount httpfs /http --base-url=https://api.example.com
```

**带认证:**
```bash
evif mount httpfs /http \
  --base-url=https://api.example.com \
  --api-key=your-api-key \
  --timeout=30
```

### 9. 键值存储 (kvfs)

```bash
evif mount kvfs /kv --storage-path=/var/lib/evif/kv.db
```

### 10. 代理文件系统 (proxyfs)

```bash
evif mount proxyfs /remote \
  --remote-url=http://remote-evif-server:8080 \
  --api-key=optional-api-key
```

## 卸载插件

### 基础语法

```bash
evif unmount <mount-point>
```

### 示例

```bash
# 卸载单个挂载点
evif unmount /local

# 强制卸载 (即使有活跃连接)
evif unmount /s3 --force

# 卸载所有
evif unmount --all
```

## 列出挂载点

### 基础命令

```bash
evif mounts
```

### 输出示例

```
Mount Points:
  /local   → localfs
    storage_path: /home/user/data
    status: active
    files: 1,234
    size: 45.6 GB

  /s3      → s3fs
    bucket: my-bucket
    region: us-east-1
    status: active
    objects: 5,678
    size: 123.4 GB

  /vector  → vectorfs
    s3_bucket: vector-docs
    status: active
    namespaces: 3
    documents: 10,234

  /queue   → queuefs
    backend: sqlite
    status: active
    queues: 5
    messages: 1,234

  /mem     → memfs
    status: active
    files: 45
    size: 256 MB
```

### 详细信息

```bash
# 详细模式
evif mounts --verbose

# JSON 格式
evif mounts --json

# 只显示活跃的
evif mounts --status=active

# 过滤插件类型
evif mounts --plugin=s3fs
```

## 插件配置

### 配置文件方式

编辑 `~/.evif/config.toml`:

```toml
[mounts.local]
plugin = "localfs"
path = "/local"
config = { storage_path = "/home/user/data" }

[mounts.s3]
plugin = "s3fs"
path = "/s3"
config = {
    region = "us-east-1",
    bucket = "my-bucket",
    access_key = "AKIAIOSFODNN7EXAMPLE",
    secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
}

[mounts.vector]
plugin = "vectorfs"
path = "/vector"
config = {
    s3_bucket = "vector-docs",
    tidb_host = "localhost:4000",
    openai_api_key = "sk-..."
}

[mounts.queue]
plugin = "queuefs"
path = "/queue"
config = {
    backend = "sqlite",
    db_path = "/var/lib/evif/queue.db"
}
```

### 启动时自动挂载

EVIF 服务器启动时会自动加载配置文件中的挂载点:

```bash
evif-server --config ~/.evif/config.toml
```

### 环境变量方式

```bash
# S3 配置
export EVIF_S3_REGION="us-east-1"
export EVIF_S3_BUCKET="my-bucket"
export EVIF_S3_ACCESS_KEY="AKIAIOSFODNN7EXAMPLE"
export EVIF_S3_SECRET_KEY="wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"

# OpenAI 配置
export EVIF_OPENAI_API_KEY="sk-..."
export EVIF_OPENAI_MODEL="gpt-4"

# TiDB 配置
export EVIF_TIDB_HOST="localhost:4000"
export EVIF_TIDB_USER="root"
export EVIF_TIDB_PASSWORD="yourpassword"
```

## 动态加载

### 加载 WASM 插件

```bash
evif plugin load ./custom-plugin.wasm --mount-point=/custom
```

### 加载共享库插件

```bash
evif plugin load ./libcustom_plugin.so --mount-point=/custom
```

### 卸载动态插件

```bash
evif plugin unload custom-plugin
```

## 插件健康检查

### 检查所有插件

```bash
evif health --plugins
```

### 检查特定插件

```bash
evif health /s3
evif health /vector
```

### 输出示例

```
Plugin Health Check:
  /local   (localfs)     ✓ Healthy (0ms)
  /s3      (s3fs)        ✓ Healthy (23ms)
  /vector  (vectorfs)    ✓ Healthy (45ms)
  /queue   (queuefs)     ✓ Healthy (1ms)
  /mem     (memfs)       ✓ Healthy (0ms)
```

## 性能监控

### 插件统计

```bash
evif stats /s3
```

输出:
```
S3FS Statistics:
  Total requests: 1,234,567
  Cache hits: 987,654 (80%)
  Cache misses: 246,913 (20%)
  Average latency: 23ms
  Errors: 123 (0.01%)
  Active handles: 45
```

## 故障排查

### 常见问题

**1. 挂载失败: "Address already in use"**
```bash
# 检查是否已挂载
evif mounts

# 先卸载
evif unmount /local

# 重新挂载
evif mount localfs /local --storage-path=/data
```

**2. S3 连接超时**
```bash
# 检查网络连接
ping s3.amazonaws.com

# 检查凭证
aws sts get-caller-identity

# 增加超时时间
evif mount s3fs /s3 --bucket=my-bucket --timeout=60
```

**3. VectorFS 初始化失败**
```bash
# 检查 TiDB 连接
mysql -h localhost -P 4000 -u root -p

# 检查 S3 访问
aws s3 ls s3://vector-docs

# 验证 OpenAI API key
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer sk-..."
```

### 调试模式

```bash
# 启用调试日志
RUST_LOG=debug evif-server --config ~/.evif/config.toml

# 挂载时显示详细信息
evif mount s3fs /s3 --bucket=my-bucket --verbose
```

## 最佳实践

### 1. 生产环境配置

**使用持久化后端:**
- QueueFS: 使用 TiDB 或 SQLite,不使用 memory
- VectorFS: 配置专用的 S3 bucket 和 TiDB 实例

**启用缓存:**
- S3FS: 启用目录列表缓存
- 配置合理的 TTL (30-60秒)

**监控:**
- 定期检查插件健康状态
- 监控错误率和延迟

### 2. 安全建议

**凭证管理:**
- 使用环境变量存储敏感信息
- 不要在命令行中传递 API keys
- 定期轮换密钥

**访问控制:**
- 为不同插件使用不同的 IAM 角色
- 配置最小权限原则
- 使用只读挂载当不需要写入时

### 3. 性能优化

**连接池:**
- S3FS: 自动管理 HTTP 连接池
- 配置合理的最大连接数

**缓存策略:**
- 根据访问模式调整 TTL
- 热数据使用更长的缓存时间

**并发控制:**
- GPTFS: 限制并发任务数
- VectorFS: 配置合适的 worker 数量

## 高级主题

### 插件热重载

```bash
# 重新加载插件配置
evif reload /s3

# 重启插件
evif restart /vector
```

### 插件版本管理

```bash
# 列出插件版本
evif plugin versions

# 升级插件
evif plugin upgrade s3fs --version=1.2.3
```

### 自定义插件开发

参阅:
- [插件开发指南](../../docs/plugin-development.md)
- [API 文档](../../docs/api.md)
- [示例插件](../../examples/custom-plugin/)

---

**相关技能:**
- `SKILL.md` - EVIF 主技能
- `evif-s3.md` - S3 最佳实践
- `evif-vector.md` - 向量搜索详解
- `evif-queue.md` - 消息队列生产模式
