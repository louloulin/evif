# EVIF 2.1 云存储插件使用指南

本文档说明如何使用基于 OpenDAL 的云存储插件。

## 支持的云存储服务

EVIF 2.1 通过 OpenDAL 支持以下云存储服务：

### 1. AWS S3 / S3 兼容存储 (MinIO, Ceph, 等)

**配置示例**:
```rust
use evif_plugins::{OpendalPlugin, OpendalConfig, OpendalService};

// AWS S3
let config = OpendalConfig {
    service: OpendalService::S3,
    bucket: Some("my-bucket".to_string()),
    region: Some("us-east-1".to_string()),
    // access_key 和 secret_key 通过环境变量设置:
    // AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY
    ..Default::default()
};

let plugin = OpendalPlugin::from_config(config).await?;
```

**MinIO 配置**:
```rust
let config = OpendalConfig {
    service: OpendalService::S3,
    bucket: Some("test-bucket".to_string()),
    endpoint: Some("http://localhost:9000".to_string()),
    access_key: Some("minioadmin".to_string()),
    secret_key: Some("minioadmin".to_string()),
    ..Default::default()
};
```

### 2. Azure Blob Storage

**配置示例**:
```rust
use evif_plugins::azureblobfs::{AzureBlobFsPlugin, AzureBlobConfig};

let config = AzureBlobConfig {
    account_name: "myaccount".to_string(),
    account_key: "my-account-key".to_string(),
    container: "my-container".to_string(),
    mount_point: "/azureblob".to_string(),
    ..Default::default()
};

let plugin = AzureBlobFsPlugin::from_config(config).await?;
```

### 3. Google Cloud Storage

**配置示例**:
```rust
use evif_plugins::{OpendalPlugin, OpendalConfig, OpendalService};

let config = OpendalConfig {
    service: OpendalService::Gcs,
    bucket: Some("my-bucket".to_string()),
    // 凭证通过环境变量 GOOGLE_APPLICATION_CREDENTIALS 设置
    ..Default::default()
};

let plugin = OpendalPlugin::from_config(config).await?;
```

### 4. 阿里云 OSS

**配置示例**:
```rust
use evif_plugins::aliyunossfs::{AliyunOssFsPlugin, AliyunOssConfig};

let config = AliyunOssConfig {
    bucket: "my-bucket".to_string(),
    endpoint: "oss-cn-hangzhou.aliyuncs.com".to_string(),
    access_key_id: "my-access-key".to_string(),
    secret_access_key: "my-secret-key".to_string(),
    ..Default::default()
};

let plugin = AliyunOssFsPlugin::from_config(config).await?;
```

### 5. 腾讯云 COS

**配置示例**:
```rust
use evif_plugins::tencentcosfs::{TencentCosFsPlugin, TencentCosConfig};

let config = TencentCosConfig {
    bucket: "my-bucket".to_string(),
    region: "ap-guangzhou".to_string(),
    secret_id: "my-secret-id".to_string(),
    secret_key: "my-secret-key".to_string(),
    ..Default::default()
};

let plugin = TencentCosFsPlugin::from_config(config).await?;
```

### 6. 华为云 OBS

**配置示例**:
```rust
use evif_plugins::huaweiobsfs::{HuaweiObsFsPlugin, HuaweiObsConfig};

let config = HuaweiObsConfig {
    bucket: "my-bucket".to_string(),
    endpoint: "obs.cn-north-1.myhuaweicloud.com".to_string(),
    access_key_id: "my-access-key".to_string(),
    secret_access_key: "my-secret-key".to_string(),
    ..Default::default()
};

let plugin = HuaweiObsFsPlugin::from_config(config).await?;
```

### 7. MinIO (S3 兼容)

**配置示例**:
```rust
use evif_plugins::miniofs::{MinioFsPlugin, MinioConfig};

let config = MinioConfig {
    bucket: "test-bucket".to_string(),
    endpoint: "http://localhost:9000".to_string(),
    access_key_id: "minioadmin".to_string(),
    secret_access_key: "minioadmin".to_string(),
    ..Default::default()
};

let plugin = MinioFsPlugin::from_config(config).await?;
```

## 环境变量配置

许多云存储服务支持通过环境变量设置凭证：

### AWS S3
```bash
export AWS_ACCESS_KEY_ID=your-access-key
export AWS_SECRET_ACCESS_KEY=your-secret-key
export AWS_REGION=us-east-1
```

### Azure Blob Storage
```bash
export AZURE_STORAGE_ACCOUNT=myaccount
export AZURE_STORAGE_KEY=my-account-key
```

### Google Cloud Storage
```bash
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
```

### 阿里云 OSS
```bash
export ALIBABA_CLOUD_ACCESS_KEY_ID=your-access-key
export ALIBABA_CLOUD_ACCESS_KEY_SECRET=your-secret-key
```

## 测试

### 本地测试 MinIO

使用 Docker 启动 MinIO:
```bash
docker run -d \
  -p 9000:9000 \
  -p 9001:9001 \
  --name minio \
  -e "MINIO_ROOT_USER=minioadmin" \
  -e "MINIO_ROOT_PASSWORD=minioadmin" \
  minio/minio server /data --console-address ':9001'
```

运行 S3 测试:
```bash
export MINIO_ENDPOINT=http://localhost:9000
export MINIO_BUCKET=test-bucket
export MINIO_ACCESS_KEY=minioadmin
export MINIO_SECRET_KEY=minioadmin

cargo run --bin test_s3
```

## 功能支持

所有云存储插件都支持以下 EVIF 文件系统操作：

- ✅ `create()` - 创建文件
- ✅ `mkdir()` - 创建目录
- ✅ `read()` - 读取文件
- ✅ `write()` - 写入文件
- ✅ `readdir()` - 列出目录
- ✅ `stat()` - 获取文件信息
- ✅ `remove()` - 删除文件
- ✅ `rename()` - 重命名文件/目录
- ✅ `remove_all()` - 递归删除

## 性能优化

OpenDAL 提供了以下性能优化：

1. **RangeReader** - 减少 98% 的 S3 API 调用
2. **连接池** - 自动连接管理
3. **并发优化** - 支持并发读写
4. **零拷贝** - 避免不必要的内存拷贝

## 最佳实践

1. **使用环境变量存储凭证** - 不要在代码中硬编码密钥
2. **启用缓存** - 对于频繁访问的文件启用缓存层
3. **使用重试机制** - OpenDAL 内置重试机制，自动处理临时故障
4. **监控和日志** - 启用日志记录以便调试

## 故障排查

### 问题 1: 认证失败
**解决方案**: 检查凭证是否正确，确保访问密钥有足够的权限

### 问题 2: 连接超时
**解决方案**: 检查网络连接，确认 endpoint URL 正确

### 问题 3: Bucket 不存在
**解决方案**: 确保指定的 bucket/container 已创建

### 问题 4: 权限不足
**解决方案**: 检查 IAM 权限或存储服务访问控制列表

## 相关资源

**Sources**:
- [OpenDAL Documentation](https://docs.rs/opendal/latest/opendal/)
- [OpenDAL Azblob Service](https://docs.rs/opendal/latest/opendal/services/struct.Azblob.html)
- [OpenDAL S3 Service](https://docs.rs/opendal/latest/opendal/services/struct.S3.html)
- [OpenDAL GCS Service](https://docs.rs/opendal/latest/opendal/services/struct.Gcs.html)
- [Apache OpenDAL GitHub](https://github.com/apache/opendal)
- [Apache OpenDAL Official Docs](https://opendal.apache.org/docs/rust/opendal/services/struct.Azblob.html)

---

**文档版本**: 1.0
**创建日期**: 2026-01-28
**作者**: EVIF 开发团队
