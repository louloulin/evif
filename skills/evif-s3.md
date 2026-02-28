---
name: evif-s3
description: "EVIF S3最佳实践 - AWS S3兼容存储使用指南"
parent: "evif"
tags: ["evif", "s3", "storage", "aws", "cloud-storage", "minio"]
trigger_keywords: ["evif s3", "s3存储", "对象存储", "aws", "minio"]
---

# EVIF S3 最佳实践

本文档详细介绍使用 EVIF S3FS 与 AWS S3 及兼容服务的最佳实践、性能优化和生产部署建议。

## 基础配置

### 1. AWS S3 配置

**基础挂载:**
```bash
evif mount s3fs /s3 \
  --region=us-east-1 \
  --bucket=my-bucket \
  --access-key=AKIAIOSFODNN7EXAMPLE \
  --secret-key=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
```

**使用凭证文件:**
```bash
# ~/.aws/credentials
[default]
aws_access_key_id = AKIAIOSFODNN7EXAMPLE
aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY

# ~/.aws/config
[default]
region = us-east-1

# 挂载 (自动读取凭证)
evif mount s3fs /s3 --bucket=my-bucket
```

**使用 IAM 角色 (EC2):**
```bash
# 无需提供凭证,自动使用 IAM 角色
evif mount s3fs /s3 --bucket=my-bucket --use-iam-role=true
```

### 2. MinIO 配置

**自托管 MinIO:**
```bash
evif mount s3fs /minio \
  --region=us-east-1 \
  --bucket=my-bucket \
  --access-key=minioadmin \
  --secret-key=minioadmin \
  --endpoint=http://localhost:9000 \
  --force-path-style=true
```

**MinIO Gateway (到 Azure Blob):**
```bash
evif mount s3fs /azure \
  --endpoint=http://minio-gateway:9000 \
  --bucket=my-container \
  --access-key=azure-account \
  --secret-key=azure-key \
  --force-path-style=true
```

### 3. 其他 S3 兼容服务

**Wasabi:**
```bash
evif mount s3fs /wasabi \
  --region=us-east-1 \
  --bucket=my-bucket \
  --endpoint=https://s3.wasabisys.com \
  --access-key=... \
  --secret-key=...
```

**DigitalOcean Spaces:**
```bash
evif mount s3fs /spaces \
  --region=nyc3 \
  --bucket=my-space \
  --endpoint=https://nyc3.digitaloceanspaces.com \
  --access-key=... \
  --secret-key=...
```

**Backblaze B2:**
```bash
evif mount s3fs /b2 \
  --region=us-west-002 \
  --bucket=my-bucket \
  --endpoint=https://s3.us-west-002.backblazeb2.com \
  --access-key=... \
  --secret-key=...
```

## 缓存策略

### 1. 目录列表缓存

**配置缓存:**
```bash
evif mount s3fs /s3 \
  --bucket=my-bucket \
  --cache-enabled=true \
  --cache-ttl-dir=30 \
  --cache-ttl-stat=60
```

**缓存策略:**
- `cache-ttl-dir`: 目录列表缓存时间 (秒)
  - 短时间 (10-30s): 适合频繁变更的目录
  - 长时间 (60-300s): 适合稳定的目录

- `cache-ttl-stat`: 文件元数据缓存时间 (秒)
  - 通常设置为 `cache-ttl-dir` 的 2 倍

### 2. 内存缓存配置

```bash
evif mount s3fs /s3 \
  --bucket=my-bucket \
  --cache-enabled=true \
  --cache-max-size=10000 \
  --cache-eviction-policy=lru  # 或 lfu, fifo
```

### 3. 缓存预热

**启动时预热:**
```bash
# 预热常用目录
evif write /s3/cache/preload "/path/to/popular/dir"

# 查看缓存状态
evif cat /s3/cache/stats
```

## 性能优化

### 1. 并发上传

**使用多线程上传:**
```bash
# 并发上传多个文件
ls *.txt | parallel -j 4 'evif write /s3/my-bucket/{} {}'

# 或使用 xargs
ls *.txt | xargs -P 4 -I {} evif write /s3/my-bucket/{} {}
```

### 2. 分块上传

**大文件分块:**
```bash
# 使用 HandleFS 分块上传大文件
HANDLE_ID=$(evif handle open /s3/my-bucket/large-file.bin)

# 分块写入
CHUNK_SIZE=10485760  # 10MB
offset=0
while IFS= read -r -d '' chunk; do
    evif handle write $HANDLE_ID --offset=$offset --data="$chunk"
    offset=$((offset + CHUNK_SIZE))
done < large-file.bin

evif handle close $HANDLE_ID
```

**自动分块:**
```bash
# 使用 --multipart-upload 选项
evif write /s3/my-bucket/large-file.bin \
  --file=large-file.bin \
  --multipart-upload \
  --chunk-size=10485760  # 10MB
```

### 3. 连接池配置

```bash
evif mount s3fs /s3 \
  --bucket=my-bucket \
  --max-concurrent-requests=50 \
  --max-idle-connections=10 \
  --request-timeout=30
```

### 4. 区域优化

**选择最近的区域:**
```bash
# 美国东部
--region=us-east-1

# 美国西部
--region=us-west-2

# 欧洲
--region=eu-west-1

# 亚太
--region=ap-southeast-1
```

**使用 Transfer 加速:**
```bash
evif mount s3fs /s3 \
  --bucket=my-bucket \
  --accelerate-enabled=true
```

## 生命周期管理

### 1. 自动过期策略

**配置生命周期规则:**
```bash
# 配置30天后自动删除
evif write /s3/my-bucket/lifecycle '{
  "rules": [
    {
      "id": "expire-old-files",
      "status": "Enabled",
      "filter": {
        "prefix": "logs/",
        "age_days": 30
      },
      "actions": ["Delete"]
    }
  ]
}'
```

### 2. 版本控制

**启用版本控制:**
```bash
# 列出对象版本
evif ls /s3/my-bucket/document.txt?versions=true

# 恢复旧版本
evif write /s3/my-bucket/document.txt \
  --version-id=abc123
```

### 3. 对象锁定

**WORM (Write Once Read Many):**
```bash
# 锁定对象防止删除
evif write /s3/my-bucket/compliance/lock \
  --object-key=important.txt \
  --mode=GOVERNANCE \
  --retain-until=2025-12-31
```

## 安全最佳实践

### 1. 访问控制

**使用最小权限:**
```bash
# 只读挂载
evif mount s3fs /s3 \
  --bucket=my-bucket \
  --read-only=true

# 写入挂载
evif mount s3fs /s3-write \
  --bucket=my-bucket \
  --read-only=false
```

**限制访问前缀:**
```bash
# 只能访问特定前缀
evif mount s3fs /s3 \
  --bucket=my-bucket \
  --allowed-prefix=public/
```

### 2. 加密

**服务端加密 (SSE):**
```bash
# 使用 AES256 加密
evif write /s3/my-bucket/sensitive.txt \
  --file=sensitive.txt \
  --encryption=AES256

# 使用 KMS 加密
evif write /s3/my-bucket/sensitive.txt \
  --file=sensitive.txt \
  --encryption=aws:kms:us-east-1:123456789012:key-id
```

**客户端加密:**
```bash
# 上传前加密
encrypt_file() {
    local input=$1
    local output=$2

    # 使用 openssl 加密
    openssl enc -aes-256-cbc -salt -in "$input" -out "$output"

    # 上传加密文件
    evif write "/s3/my-bucket/encrypted/$(basename $output)" "$output"
}
```

### 3. 签名验证

**启用签名验证:**
```bash
evif mount s3fs /s3 \
  --bucket=my-bucket \
  --verify-integrity=true
```

## 成本优化

### 1. 存储类别

**使用合适的存储类别:**
```bash
# 标准存储 (频繁访问)
evif write /s3/my-bucket/hot.txt \
  --file=hot.txt \
  --storage-class=STANDARD

# 低频访问存储
evif write /s3/my-bucket/cold.txt \
  --file=cold.txt \
  --storage-class=STANDARD_IA

# 归档存储
evif write /s3/my-bucket/archive.txt \
  --file=archive.txt \
  --storage-class=GLACIER

# 深度归档
evif write /s3/my-bucket/deep-archive.txt \
  --file=deep-archive.txt \
  --storage-class=DEEP_ARCHIVE
```

**自动生命周期转换:**
```bash
# 配置自动转换存储类别
evif write /s3/my-bucket/lifecycle '{
  "rules": [
    {
      "id": "transition-to-ia",
      "status": "Enabled",
      "transitions": [
        {
          "days": 30,
          "storage_class": "STANDARD_IA"
        },
        {
          "days": 90,
          "storage_class": "GLACIER"
        }
      ]
    }
  ]
}'
```

### 2. 压缩

**上传前压缩:**
```bash
# 自动压缩文本文件
compress_and_upload() {
    local file=$1

    # 检查文件类型
    if file "$file" | grep -q "text"; then
        # 压缩
        gzip -c "$file" | evif write "/s3/my-bucket/compressed/$(basename $file).gz"
        echo "Compressed: $file"
    else
        # 直接上传
        evif write "/s3/my-bucket/$(basename $file)" --file="$file"
    fi
}
```

### 3. 删除策略

**批量删除旧文件:**
```bash
# 删除30天前的文件
evif write /s3/my-bucket/cleanup \
  --prefix=logs/ \
  --older-than=30days
```

## 监控和日志

### 1. 访问日志

**启用访问日志:**
```bash
# 查看访问日志
evif cat /s3/logs/access

# 实时监控
tail -f /s3/logs/access
```

### 2. 性能监控

**查看性能指标:**
```bash
# 请求延迟
evif cat /s3/metrics/latency

# 吞吐量
evif cat /s3/metrics/throughput

# 错误率
evif cat /s3/metrics/error_rate
```

### 3. 成本追踪

**追踪 API 调用成本:**
```bash
# 查看使用统计
evif cat /s3/stats/usage

# 按前缀统计
evif cat /s3/stats/by-prefix?prefix=logs/
```

## 备份和恢复

### 1. 跨区域复制

**配置复制规则:**
```bash
# 复制到另一个区域
evif write /s3/source-bucket/replication '{
  "role": "arn:aws:iam::123456789012:role/s3-replication",
  "rules": [
    {
      "source": "arn:aws:s3:::source-bucket",
      "destination": "arn:aws:s3:::destination-bucket",
      "status": "Enabled"
    }
  ]
}'
```

### 2. 版本控制恢复

**恢复删除对象:**
```bash
# 查看删除对象的版本
evif ls /s3/my-bucket/deleted-file.txt?versions=true

# 恢复
evif write /s3/my-bucket/deleted-file.txt \
  --restore-from-version-id=abc123
```

### 3. 批量备份

**备份到另一个 S3:**
```bash
# 批量复制
evif cp /s3/source-bucket/ /s3/backup-bucket/ --recursive
```

## 使用场景

### 场景1: 静态网站托管

```bash
# 上传网站文件
evif cp /local/website/ /s3/website-root/ --recursive

# 配置静态网站
evif write /s3/website-root/website-config '{
  "index_document": "index.html",
  "error_document": "error.html",
  "routing_rules": [
    {
      "condition": {
        "key_prefix": "assets/",
        "httpErrorCode": 404
      },
      "redirect": {
        "replaceKeyPrefix": "assets/",
        "replaceKeyWith": "assets/"
      }
    }
  ]
}'

# 设置公开访问
evif write /s3/website-root/policy '{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "PublicReadGetObject",
      "Effect": "Allow",
      "Principal": "*",
      "Action": "s3:GetObject",
      "Resource": "arn:aws:s3:::website-root/*"
    }
  ]
}'
```

### 场景2: 数据湖

**组织数据湖:**
```bash
# 按日期分区
evif write /s3/data-lake/raw/2025/01/25/data.json \
  --file=data.json

# 按类型组织
/ s3/
  ├─ data-lake/
  │   ├─ raw/           # 原始数据
  │   ├─ processed/     # 处理后的数据
  │   ├─ curated/       # 清洗后的数据
  │   └─ analytics/     # 分析结果
```

**生命周期管理:**
```bash
# 自动转换存储类别
evif write /s3/data-lake/lifecycle '{
  "rules": [
    {
      "id": "raw-to-processed",
      "filter": {"prefix": "raw/"},
      "transitions": [
        {"days": 7, "storage_class": "STANDARD_IA"},
        {"days": 30, "storage_class": "GLACIER"}
      ]
    },
    {
      "id": "processed-to-cold",
      "filter": {"prefix": "processed/"},
      "transitions": [
        {"days": 30, "storage_class": "GLACIER"}
      ]
    }
  ]
}'
```

### 场景3: 备份和归档

**3-2-1 备份策略:**
```bash
# 备份到多个区域
evif cp /local/important/ /s3/backup-primary/ --recursive
evif cp /local/important/ /s3/backup-secondary/ --recursive
evif cp /local/important/ /s3/backup-tertiary/ --recursive

# 使用 Glacier 深度归档
evif write /s3/archive/old-data.tar.gz \
  --file=old-data.tar.gz \
  --storage-class=DEEP_ARCHIVE
```

**自动备份脚本:**
```bash
#!/bin/bash
# daily_backup.sh

BACKUP_DATE=$(date +%Y%m%d)
BACKUP_NAME="backup-$BACKUP_DATE.tar.gz"

# 创建备份
tar czf /tmp/$BACKUP_NAME /local/data/

# 上传到 S3
evif write "/s3/backups/$BACKUP_DATE/$BACKUP_NAME" \
  --file="/tmp/$BACKUP_NAME" \
  --storage-class=STANDARD_IA

# 设置30天后自动删除
evif write "/s3/backups/$BACKUP_DATE/$BACKUP_NAME/lifecycle" \
  --expire-after=30days

# 清理本地
rm /tmp/$BACKUP_NAME

echo "Backup completed: $BACKUP_NAME"
```

### 场景4: 日志聚合

**集中日志管理:**
```bash
# 应用日志按日期和类型组织
/app/logs/
  ├─ app1/
  │   ├─ 2025-01-25/
  │   │   ├─ error.log
  │   │   ├─ access.log
  │   │   └─ debug.log
  └─ app2/
      └─ 2025-01-25/
          ├─ error.log
          └─ access.log

# 上传到 S3
evif cp /app/logs/ /s3/application-logs/ --recursive

# 配置生命周期
# 7天后转为低频访问
# 30天后转为归档
# 90天后删除
```

### 场景5: CI/CD 工件

**构建产物存储:**
```bash
# 存储构建产物
BUILD_ID=$(git rev-parse --short HEAD)
BUILD_NAME="project-$BUILD_ID.tar.gz"

# 打包
tar czf /tmp/$BUILD_NAME build/

# 上传
evif write "/s3/builds/$BUILD_ID/$BUILD_NAME" \
  --file="/tmp/$BUILD_NAME" \
  --metadata="{
    \"project\": \"myproject\",
    \"branch\": \"main\",
    \"commit\": \"$BUILD_ID\",
    \"build_date\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"
  }"

# 清理旧构建 (保留最近20个)
evif write /s3/builds/cleanup --keep=20
```

## 故障排查

### 常见问题

**1. 连接超时**
```bash
# 增加超时时间
evif mount s3fs /s3 \
  --bucket=my-bucket \
  --request-timeout=60 \
  --connection-timeout=10
```

**2. 访问被拒绝**
```bash
# 检查凭证
aws sts get-caller-identity

# 检查 IAM 权限
evif cat /s3/debug/permissions
```

**3. 性能慢**
```bash
# 启用加速
evif mount s3fs /s3 \
  --bucket=my-bucket \
  --accelerate-enabled=true \
  --max-concurrent-requests=100
```

### 调试工具

**启用调试日志:**
```bash
# 启用详细日志
RUST_LOG=evif_s3=debug evif-server

# 查看 S3 API 调用
evif cat /s3/debug/api-calls
```

**网络诊断:**
```bash
# 测试连接
curl -I https://my-bucket.s3.amazonaws.com

# 追踪路由
traceroute s3.amazonaws.com
```

## 最佳实践清单

### 安全
- [ ] 使用 IAM 角色而非访问密钥
- [ ] 启用服务器端加密
- [ ] 使用 HTTPS
- [ ] 定期轮换凭证
- [ ] 限制访问权限

### 性能
- [ ] 使用 Transfer 加速
- [ ] 启用缓存
- [ ] 使用最近的区域
- [ ] 配置合适的并发数
- [ ] 使用多部分上传

### 成本
- [ ] 使用合适的存储类别
- [ ] 配置生命周期规则
- [ ] 压缩文本数据
- [ ] 定期清理旧数据
- [ ] 使用 S3 Intelligent-Tiering

### 可靠性
- [ ] 启用版本控制
- [ ] 配置跨区域复制
- [ ] 实现备份策略
- [ ] 监控错误率
- [ ] 实现重试逻辑

---

**相关技能:**
- `SKILL.md` - EVIF 主技能
- `evif-manage.md` - 插件管理
