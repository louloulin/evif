# EVIF 2.1 云存储配置示例

本文档提供所有支持的云存储服务的配置示例。

---

## 📋 目录

1. [国际云存储](#国际云存储)
   - [AWS S3](#aws-s3)
   - [Azure Blob Storage](#azure-blob-storage)
   - [Google Cloud Storage](#google-cloud-storage)
   - [MinIO](#minio)

2. [国内云存储](#国内云存储)
   - [阿里云 OSS](#阿里云-oss)
   - [腾讯云 COS](#腾讯云-cos)
   - [华为云 OBS](#华为云-obs)

3. [本地存储](#本地存储)
   - [Memory](#memory)
   - [Fs (本地文件系统)](#fs-本地文件系统)

---

## 国际云存储

### AWS S3

**环境变量配置**:
```bash
export AWS_S3_BUCKET=my-bucket
export AWS_S3_REGION=us-east-1
export AWS_ACCESS_KEY_ID=your-access-key-id
export AWS_SECRET_ACCESS_KEY=your-secret-access-key
```

**Rust 代码配置**:
```rust
use evif_plugins::{S3FsPlugin, S3Config};

#[tokio::main]
async fn main() -> evif_core::EvifResult<()> {
    let config = S3Config {
        bucket: "my-bucket".to_string(),
        region: Some("us-east-1".to_string()),
        access_key_id: std::env::var("AWS_ACCESS_KEY_ID").unwrap(),
        secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY").unwrap(),
        endpoint: None,  // 使用默认 AWS S3 endpoint
        root: Some("/path/to/root".to_string()),
    };

    let plugin = S3FsPlugin::from_config(config).await?;
    
    // 使用 plugin...
    Ok(())
}
```

**或使用 OpendalPlugin 直接配置**:
```rust
use evif_plugins::{OpendalPlugin, OpendalConfig, OpendalService};

let config = OpendalConfig {
    service: OpendalService::S3,
    bucket: Some("my-bucket".to_string()),
    region: Some("us-east-1".to_string()),
    access_key: Some(std::env::var("AWS_ACCESS_KEY_ID").unwrap()),
    secret_key: Some(std::env::var("AWS_SECRET_ACCESS_KEY").unwrap()),
    ..Default::default()
};

let plugin = OpendalPlugin::from_config(config).await?;
```

---

### Azure Blob Storage

**环境变量配置**:
```bash
export AZURE_ACCOUNT_NAME=myaccount
export AZURE_ACCOUNT_KEY=my-account-key
export AZURE_CONTAINER=my-container
```

**Rust 代码配置**:
```rust
use evif_plugins::{AzureBlobFsPlugin, AzureBlobConfig};

let config = AzureBlobConfig {
    account_name: std::env::var("AZURE_ACCOUNT_NAME").unwrap(),
    account_key: std::env::var("AZURE_ACCOUNT_KEY").unwrap(),
    container: std::env::var("AZURE_CONTAINER").unwrap(),
    endpoint: None,  // 可选: 使用自定义 endpoint
    root: Some("/path/to/root".to_string()),
};

let plugin = AzureBlobFsPlugin::from_config(config).await?;
```

---

### Google Cloud Storage

**环境变量配置**:
```bash
export GCP_BUCKET=my-bucket
# GCP 使用服务账号凭证，通过 GOOGLE_APPLICATION_CREDENTIALS 环境变量指定
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
```

**Rust 代码配置**:
```rust
use evif_plugins::{GcsFsPlugin, GcsConfig};

let config = GcsConfig {
    bucket: std::env::var("GCP_BUCKET").unwrap(),
    endpoint: None,  // 可选: 使用自定义 endpoint
    root: Some("/path/to/root".to_string()),
};

let plugin = GcsFsPlugin::from_config(config).await?;
```

---

### MinIO

**环境变量配置**:
```bash
export MINIO_ENDPOINT=http://localhost:9000
export MINIO_BUCKET=test-bucket
export MINIO_ACCESS_KEY=minioadmin
export MINIO_SECRET_KEY=minioadmin
```

**Rust 代码配置**:
```rust
use evif_plugins::{MinioFsPlugin, MinioConfig};

let config = MinioConfig {
    bucket: "test-bucket".to_string(),
    access_key: "minioadmin".to_string(),
    secret_key: "minioadmin".to_string(),
    endpoint: "http://localhost:9000".to_string(),
    region: Some("us-east-1".to_string()),
    root: Some("/data".to_string()),
};

let plugin = MinioFsPlugin::from_config(config).await?;
```

**使用 Docker 运行 MinIO**:
```bash
docker run -d -p 9000:9000 -p 9001:9001 \
  -e "MINIO_ROOT_USER=minioadmin" \
  -e "MINIO_ROOT_PASSWORD=minioadmin" \
  --name minio \
  minio/minio server /data --console-address ':9001'
```

---

## 国内云存储

### 阿里云 OSS

**环境变量配置**:
```bash
export ALIYUN_OSS_BUCKET=my-bucket
export ALIYUN_OSS_ACCESS_KEY_ID=your-access-key-id
export ALIYUN_OSS_ACCESS_KEY_SECRET=your-access-key-secret
export ALIYUN_OSS_ENDPOINT=oss-cn-hangzhou.aliyuncs.com
```

**Rust 代码配置**:
```rust
use evif_plugins::{AliyunOssFsPlugin, AliyunOssConfig};

let config = AliyunOssConfig {
    bucket: std::env::var("ALIYUN_OSS_BUCKET").unwrap(),
    access_key_id: std::env::var("ALIYUN_OSS_ACCESS_KEY_ID").unwrap(),
    access_key_secret: std::env::var("ALIYUN_OSS_ACCESS_KEY_SECRET").unwrap(),
    endpoint: Some(std::env::var("ALIYUN_OSS_ENDPOINT").unwrap()),
    root: Some("/data".to_string()),
};

let plugin = AliyunOssFsPlugin::from_config(config).await?;
```

**阿里云 OSS 区域列表**:
- `oss-cn-hangzhou` - 华东1（杭州）
- `oss-cn-shanghai` - 华东2（上海）
- `oss-cn-qingdao` - 华北1（青岛）
- `oss-cn-beijing` - 华北2（北京）
- `oss-cn-zhangjiakou` - 华北3（张家口）
- `oss-cn-shenzhen` - 华南1（深圳）
- `oss-cn-guangzhou` - 华南2（广州）
- `oss-cn-chengdu` - 西南1（成都）

---

### 腾讯云 COS

**环境变量配置**:
```bash
export TENCENT_COS_BUCKET=my-bucket
export TENCENT_COS_SECRET_ID=your-secret-id
export TENCENT_COS_SECRET_KEY=your-secret-key
export TENCENT_COS_REGION=ap-guangzhou
export TENCENT_COS_ENDPOINT=https://cos.ap-guangzhou.myqcloud.com
```

**Rust 代码配置**:
```rust
use evif_plugins::{TencentCosFsPlugin, TencentCosConfig};

let config = TencentCosConfig {
    bucket: std::env::var("TENCENT_COS_BUCKET").unwrap(),
    secret_id: std::env::var("TENCENT_COS_SECRET_ID").unwrap(),
    secret_key: std::env::var("TENCENT_COS_SECRET_KEY").unwrap(),
    endpoint: Some(std::env::var("TENCENT_COS_ENDPOINT").unwrap()),
    region: Some(std::env::var("TENCENT_COS_REGION").unwrap()),
    root: Some("/data".to_string()),
};

let plugin = TencentCosFsPlugin::from_config(config).await?;
```

**腾讯云 COS 区域列表**:
- `ap-guangzhou` - 广州
- `ap-shanghai` - 上海
- `ap-beijing` - 北京
- `ap-chengdu` - 成都
- `ap-chongqing` - 重庆
- `ap-singapore` - 新加坡

---

### 华为云 OBS

**环境变量配置**:
```bash
export HUAWEI_OBS_BUCKET=my-bucket
export HUAWEI_OBS_ACCESS_KEY_ID=your-access-key-id
export HUAWEI_OBS_SECRET_ACCESS_KEY=your-secret-access-key
export HUAWEI_OBS_ENDPOINT=obs.cn-north-4.myhuaweicloud.com
```

**Rust 代码配置**:
```rust
use evif_plugins::{HuaweiObsFsPlugin, HuaweiObsConfig};

let config = HuaweiObsConfig {
    bucket: std::env::var("HUAWEI_OBS_BUCKET").unwrap(),
    access_key_id: std::env::var("HUAWEI_OBS_ACCESS_KEY_ID").unwrap(),
    secret_access_key: std::env::var("HUAWEI_OBS_SECRET_ACCESS_KEY").unwrap(),
    endpoint: Some(std::env::var("HUAWEI_OBS_ENDPOINT").unwrap()),
    root: Some("/data".to_string()),
};

let plugin = HuaweiObsFsPlugin::from_config(config).await?;
```

**华为云 OBS 区域列表**:
- `obs.cn-north-4.myhuaweicloud.com` - 华北-北京四
- `obs.cn-south-4.myhuaweicloud.com` - 华南-广州
- `obs.cn-east-3.myhuaweicloud.com` - 华东-上海一
- `obs.cn-southwest-2.myhuaweicloud.com` - 西南-贵阳一

---

## 本地存储

### Memory

**Rust 代码配置**:
```rust
use evif_plugins::{OpendalPlugin, OpendalConfig, OpendalService};

let config = OpendalConfig {
    service: OpendalService::Memory,
    ..Default::default()
};

let plugin = OpendalPlugin::from_config(config).await?;
```

**用途**: 内存存储，用于测试和临时数据存储。

---

### Fs (本地文件系统)

**Rust 代码配置**:
```rust
use evif_plugins::{OpendalPlugin, OpendalConfig, OpendalService};

let config = OpendalConfig {
    service: OpendalService::Fs,
    root: Some("/tmp/evif-data".to_string()),  // 根目录
    ..Default::default()
};

let plugin = OpendalPlugin::from_config(config).await?;
```

**用途**: 本地文件系统访问，用于持久化存储。

---

## 🔧 通用配置模式

所有云存储服务都遵循统一的配置模式：

```rust
pub struct OpendalConfig {
    /// 服务类型（必需）
    pub service: OpendalService,
    
    /// Bucket/容器名称（云存储必需）
    pub bucket: Option<String>,
    
    /// 访问密钥（云存储通常需要）
    pub access_key: Option<String>,
    
    /// 密钥（云存储通常需要）
    pub secret_key: Option<String>,
    
    /// Endpoint（可选，用于兼容性存储）
    pub endpoint: Option<String>,
    
    /// Region（可选）
    pub region: Option<String>,
    
    /// 根路径（可选，指定存储根目录）
    pub root: Option<String>,
    
    /// ... 其他配置
}
```

---

## 🧪 测试示例

### 基本文件操作

```rust
use evif_core::EvifPlugin;

async fn test_plugin_operations<P: EvifPlugin>(plugin: &P) -> evif_core::EvifResult<()> {
    // 1. 创建文件
    plugin.create("/test.txt", 0o644).await?;
    
    // 2. 写入数据
    let data = b"Hello, EVIF 2.1!".to_vec();
    plugin.write("/test.txt", data, -1, evif_core::WriteFlags::empty()).await?;
    
    // 3. 读取文件
    let read_data = plugin.read("/test.txt", 0, 0).await?;
    assert_eq!(read_data, b"Hello, EVIF 2.1!");
    
    // 4. 获取文件信息
    let info = plugin.stat("/test.txt").await?;
    println!("File size: {} bytes", info.size);
    
    // 5. 列出目录
    let files = plugin.readdir("/").await?;
    for file in files {
        println!("  - {} ({} bytes)", file.name, file.size);
    }
    
    // 6. 删除文件
    plugin.remove("/test.txt").await?;
    
    Ok(())
}
```

---

## 📚 更多资源

- **EVIF 2.1 主文档**: [evif2.1.md](./evif2.1.md)
- **云存储使用指南**: [CLOUD_STORAGE_GUIDE.md](./CLOUD_STORAGE_GUIDE.md)
- **OpenDAL 官方文档**: https://opendal.apache.org/

---

**文档版本**: 1.0  
**更新日期**: 2026-01-28  
**支持状态**: 云存储配置完整支持
