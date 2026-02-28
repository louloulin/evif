# EVIF S3FS 插件设计文档

**日期**: 2025-01-24
**优先级**: 🔴 P0 (阻塞生产使用)
**参考**: AGFS S3FS (1062 行 + 353 行缓存)

---

## 📋 设计目标

### 核心功能
1. **AWS S3 集成**: 完整的 S3 对象存储支持
2. **S3 兼容性**: 支持 MinIO, LocalStack, Aliyun OSS 等
3. **双层缓存**: dirCache + statCache LRU 缓存
4. **流式处理**: 大文件流式读写
5. **高性能**: 缓存命中率 > 90%

### 对标 AGFS S3FS

| 特性 | AGFS | EVIF 设计 | 优先级 |
|-----|------|----------|--------|
| 基础操作 (CRUD) | ✅ | ✅ | P0 |
| 双层缓存 | ✅ | ✅ | P0 |
| 流式读写 | ✅ | ✅ | P0 |
| 分片上传 | ❌ | ✅ | P1 |
| MinIO 兼容 | ✅ | ✅ | P0 |
| Prefix 隔离 | ✅ | ✅ | P0 |
| Range 请求 | ✅ | ✅ | P1 |
| Multipart | ❌ | ✅ | P1 |

---

## 🏗️ 架构设计

### 1. 插件结构

```rust
pub struct S3fsPlugin {
    // S3 客户端
    client: aws_sdk_s3::Client,
    config: S3Config,

    // 双层缓存
    dir_cache: Arc<RwLock<DirCache>>,
    stat_cache: Arc<RwLock<StatCache>>,

    // 运行时统计
    stats: Arc<RwLock<S3Stats>>,
}

pub struct S3Config {
    pub bucket: String,
    pub region: String,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub endpoint: Option<String>,         // MinIO 兼容
    pub prefix: Option<String>,           // 命名空间隔离
    pub disable_ssl: bool,
    pub force_path_style: bool,           // MinIO 必需

    // 缓存配置
    pub cache_enabled: bool,
    pub dir_cache_ttl: Duration,
    pub stat_cache_ttl: Duration,
    pub cache_max_size: usize,
}

pub struct S3Stats {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub requests_total: u64,
}
```

### 2. 缓存架构

#### DirCache (目录列表缓存)

```rust
pub struct DirCache {
    cache: HashMap<String, CacheEntry<Vec<FileInfo>>>,
    lru_list: LinkedList<String>,         // LRU 淘汰链表
    max_size: usize,
    ttl: Duration,
    enabled: bool,
    stats: CacheStats,
}

impl DirCache {
    pub fn get(&self, path: &str) -> Option<Vec<FileInfo>> {
        if !self.enabled { return None; }

        let entry = self.cache.get(path)?;
        if entry.is_expired(self.ttl) {
            return None;
        }

        // 更新 LRU (移到最前)
        self.move_to_front(path);
        Some(entry.data.clone())
    }

    pub fn put(&mut self, path: String, files: Vec<FileInfo>) {
        if !self.enabled { return; }

        // 淘汰最老条目
        if self.cache.len() >= self.max_size {
            self.evict_oldest();
        }

        // 插入新条目
        self.cache.insert(path.clone(), CacheEntry::new(files));
        self.lru_list.push_front(path);
    }

    pub fn invalidate(&mut self, path: &str) {
        self.cache.remove(path);
    }

    pub fn invalidate_prefix(&mut self, prefix: &str) {
        // 删除所有前缀匹配的条目
        self.cache.retain(|path, _| {
            !path.starts_with(prefix) || !path[prefix.len()..].starts_with('/')
        });
    }
}
```

#### StatCache (元数据缓存)

```rust
pub struct StatCache {
    cache: HashMap<String, CacheEntry<FileInfo>>,
    lru_list: LinkedList<String>,
    max_size: usize,
    ttl: Duration,
    enabled: bool,
}

impl StatCache {
    // 实现类似 DirCache
}
```

### 3. 核心方法实现

#### 3.1 Read (支持 Range 请求)

```rust
async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>> {
    let s3_path = self.normalize_s3_key(path)?;

    // 尝试从 stat cache 获取文件大小
    if let Some(info) = self.stat_cache.read().await.get(&s3_path) {
        if offset >= info.size {
            return Ok(Vec::new());  // EOF
        }
    }

    // 使用 S3 Range 请求
    if offset > 0 || size > 0 {
        let range = format!("bytes={}-{}", offset, offset + size - 1);
        let result = self.client
            .get_object()
            .bucket(&self.config.bucket)
            .key(&s3_path)
            .range(range)
            .send()
            .await?;

        return Ok(result.body.collect().await?.to_vec());
    }

    // 完整文件读取
    let result = self.client
        .get_object()
        .bucket(&self.config.bucket)
        .key(&s3_path)
        .send()
        .await?;

    Ok(result.body.collect().await?.to_vec())
}
```

#### 3.2 Write (完整对象替换)

```rust
async fn write(&self, path: &str, data: Vec<u8>, offset: i64, flags: WriteFlags)
    -> EvifResult<u64>
{
    let s3_path = self.normalize_s3_key(path)?;

    // S3 不支持 offset 写入
    if offset > 0 && offset != -1 {
        return Err(EvifError::InvalidPath(
            "S3 does not support offset writes".to_string()
        ));
    }

    // 检查是否为目录
    if s3_path.ends_with('/') {
        return Err(EvifError::InvalidPath(format!("is a directory: {}", path)));
    }

    // 上传到 S3
    let body = aws_sdk_s3::types::ByteStream::from(data.clone());
    self.client
        .put_object()
        .bucket(&self.config.bucket)
        .key(&s3_path)
        .body(body)
        .send()
        .await?;

    // 失效缓存
    let parent = self.get_parent_path(&s3_path);
    self.dir_cache.write().await.invalidate(&parent);
    self.stat_cache.write().await.invalidate(&s3_path);

    Ok(data.len() as u64)
}
```

#### 3.3 ReadDir (带缓存)

```rust
async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
    let s3_path = self.normalize_s3_key(path)?;

    // 1. 尝试从缓存读取
    if let Some(cached) = self.dir_cache.read().await.get(&s3_path) {
        return Ok(cached);
    }

    // 2. 从 S3 列出对象
    let prefix = if s3_path.is_empty() {
        String::new()
    } else {
        format!("{}/", s3_path.trim_end_matches('/'))
    };

    let result = self.client
        .list_objects_v2()
        .bucket(&self.config.bucket)
        .prefix(&prefix)
        .delimiter("/")  // 分隔符用于模拟目录
        .send()
        .await?;

    let mut files = Vec::new();

    // 处理文件 (CommonPrefixes 是子目录)
    for obj in result.contents().unwrap_or(&[]) {
        let key = obj.key().unwrap_or(&prefix);
        let relative_key = key.strip_prefix(&prefix).unwrap_or(key);

        if relative_key.is_empty() {
            continue;  // 跳过目录标记本身
        }

        files.push(FileInfo {
            name: relative_key.to_string(),
            size: obj.size().unwrap_or(0) as u64,
            mode: 0o644,
            modified: obj.last_modified().unwrap().clone(),
            is_dir: false,
        });
    }

    // 处理子目录
    for prefix in result.common_prefixes().unwrap_or(&[]) {
        let prefix_str = prefix.prefix().unwrap_or(&String::new());
        let relative = prefix_str.strip_prefix(&prefix).unwrap_or(prefix_str);
        let dir_name = relative.trim_end_matches('/');

        if !dir_name.is_empty() {
            files.push(FileInfo {
                name: dir_name.to_string(),
                size: 0,
                mode: 0o755,
                modified: chrono::Utc::now(),
                is_dir: true,
            });
        }
    }

    // 3. 写入缓存
    self.dir_cache.write().await.put(s3_path.clone(), files.clone());

    Ok(files)
}
```

#### 3.4 Stat (带缓存)

```rust
async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
    let s3_path = self.normalize_s3_key(path)?;

    // 根目录特殊处理
    if s3_path.is_empty() || s3_path == "/" {
        return Ok(FileInfo {
            name: "/".to_string(),
            size: 0,
            mode: 0o755,
            modified: chrono::Utc::now(),
            is_dir: true,
        });
    }

    // 1. 尝试从缓存读取
    if let Some(cached) = self.stat_cache.read().await.get(&s3_path) {
        return Ok(cached);
    }

    // 2. 尝试作为文件
    match self.client
        .head_object()
        .bucket(&self.config.bucket)
        .key(&s3_path)
        .send()
        .await
    {
        Ok(head) => {
            let info = FileInfo {
                name: self.get_basename(&s3_path),
                size: head.content_length().unwrap_or(0) as u64,
                mode: 0o644,
                modified: head.last_modified().unwrap().clone(),
                is_dir: false,
            };

            // 写入缓存
            self.stat_cache.write().await.put(s3_path.clone(), info.clone());
            return Ok(info);
        }
        Err(e) => {
            // 3. 尝试作为目录
            let prefix = format!("{}/", s3_path.trim_end_matches('/'));
            let result = self.client
                .list_objects_v2()
                .bucket(&self.config.bucket)
                .prefix(&prefix)
                .max_keys(1)
                .send()
                .await?;

            if result.contents().unwrap_or(&()).len() > 0
                || result.common_prefixes().unwrap_or(&()).len() > 0
            {
                let info = FileInfo {
                    name: self.get_basename(&s3_path),
                    size: 0,
                    mode: 0o755,
                    modified: chrono::Utc::now(),
                    is_dir: true,
                };

                self.stat_cache.write().await.put(s3_path.clone(), info.clone());
                return Ok(info);
            }

            // 4. 未找到
            return Err(EvifError::NotFound(path.to_string()));
        }
    }
}
```

#### 3.5 RemoveAll (批量删除)

```rust
async fn remove_all(&self, path: &str) -> EvifResult<()> {
    let s3_path = self.normalize_s3_key(path)?;
    let prefix = format!("{}/", s3_path.trim_end_matches('/'));

    // 1. 列出所有对象
    let mut all_objects = Vec::new();
    let mut continuation_token = None;

    loop {
        let mut list_req = self.client
            .list_objects_v2()
            .bucket(&self.config.bucket)
            .prefix(&prefix)
            .max_keys(1000);

        if let Some(token) = &continuation_token {
            list_req = list_req.continuation_token(token);
        }

        let result = list_req.send().await?;

        if let Some(objects) = result.contents() {
            for obj in objects {
                if let Some(key) = obj.key() {
                    all_objects.push(key.to_string());
                }
            }
        }

        if !result.next_continuation_token().is_some() {
            break;
        }

        continuation_token = result.next_continuation_token().map(|s| s.to_string());
    }

    // 2. 批量删除 (每次最多 1000 个对象)
    for chunk in all_objects.chunks(1000) {
        let delete_objs: Vec<_> = chunk.iter().map(|key| {
            aws_sdk_s3::types::ObjectIdentifier::builder()
                .key(key)
                .build()
        }).collect();

        self.client
            .delete_objects()
            .bucket(&self.config.bucket)
            .delete(
                aws_sdk_s3::types::Delete::builder()
                    .set_objects(Some(delete_objs))
                    .build()
            )
            .send()
            .await?;
    }

    // 3. 失效缓存
    let parent = self.get_parent_path(&s3_path);
    self.dir_cache.write().await.invalidate(&parent);
    self.dir_cache.write().await.invalidate_prefix(&s3_path);
    self.stat_cache.write().await.invalidate_prefix(&s3_path);

    Ok(())
}
```

---

## 🔧 依赖管理

### Cargo.toml

```toml
[dependencies]
# AWS SDK
aws-config = { version = "1.5", features = ["behavior-version-latest"] }
aws-sdk-s3 = { version = "1.40", features = ["behavior-version-latest"] }
aws-smithy-types = "1.2"

# 现有依赖
async-trait = "0.1"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
chrono = "0.4"

[features]
default = ["s3fs"]
s3fs = ["aws-config", "aws-sdk-s3", "aws-smithy-types"]
```

---

## 📊 性能优化

### 1. 缓存命中率目标

| 操作 | 目标命中率 | 性能提升 |
|-----|----------|---------|
| ReadDir | > 90% | 100-500x |
| Stat | > 95% | 100-500x |
| Read | N/A (流式) | 1x |

### 2. 并发性能

- **读操作**: RwLock 读锁,完全并发
- **写操作**: Mutex 保护,顺序执行
- **缓存**: 独立的 RwLock,减少锁竞争

### 3. 内存优化

- **流式读取**: 大文件使用 ByteStream,不加载到内存
- **批量删除**: 分块删除,避免内存爆炸
- **缓存限制**: LRU 淘汰,控制内存占用

---

## 🧪 测试计划

### 单元测试

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_s3fs_basic() {
        // 测试基础 CRUD 操作
    }

    #[tokio::test]
    async fn test_s3fs_cache() {
        // 测试缓存功能
    }

    #[tokio::test]
    async fn test_s3fs_minio() {
        // 测试 MinIO 兼容性
    }

    #[tokio::test]
    async fn test_s3fs_remove_all() {
        // 测试批量删除
    }

    #[tokio::test]
    async fn test_s3fs_prefix_isolation() {
        // 测试前缀隔离
    }
}
```

---

## 📝 实现步骤

### Phase 1: 基础功能 (P0)

1. ✅ 添加 AWS SDK 依赖
2. ✅ 实现 S3Config 结构
3. ✅ 实现 DirCache 和 StatCache
4. ⬜ 实现 S3fsPlugin 基础结构
5. ⬜ 实现 create, mkdir, remove, rename
6. ⬜ 实现 read, write (完整对象替换)
7. ⬜ 实现 readdir, stat (带缓存)
8. ⬜ 实现 remove_all (批量删除)

### Phase 2: 高级功能 (P1)

9. ⬜ 实现 Range 请求 (offset read)
10. ⬜ 实现流式读写 (ByteStream)
11. ⬜ 实现分片上传 (Multipart Upload)
12. ⬜ 添加性能统计 (cache hit rate)

### Phase 3: 测试和文档 (P2)

13. ⬜ 编写单元测试
14. ⬜ 编写集成测试 (MinIO)
15. ⬜ 更新文档

---

## ✅ 验收标准

### 功能验收

- ✅ 所有 9 个核心方法实现
- ✅ 双层缓存正常工作
- ✅ MinIO 兼容性测试通过
- ✅ RemoveAll 批量删除正常
- ✅ 单元测试覆盖率 > 80%

### 性能验收

- ✅ ReadDir 缓存命中率 > 90%
- ✅ Stat 缓存命中率 > 95%
- ✅ 大文件 (> 100MB) 流式读取内存占用 < 10MB

### 稳定性验收

- ✅ 所有测试通过
- ✅ 无内存泄漏
- ✅ 无死锁
- ✅ 错误处理完善

---

**下一步**: 开始实现 S3FS 插件 Phase 1
**预计工作量**: 5-7 天
**代码行数**: ~800 行 (不含测试)
