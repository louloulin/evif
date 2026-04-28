# EVIF 核心模块

EVIF Core 是整个系统的引擎，提供插件注册、路径路由、Handle 管理等核心功能。约 26 个模块，~7,500 行 Rust 代码。

## 1. 模块概览

```
evif-core/src/
├── lib.rs                    # 公共导出
├── plugin.rs                 # 插件 trait 定义
├── error.rs                  # 错误类型
├── handle.rs                 # Handle 管理
├── mount_table.rs            # Radix 挂载表
├── cache.rs                  # 缓存管理
├── circuit_breaker.rs        # 熔断器
├── batch.rs                  # 批量操作
├── context.rs                # 上下文管理
├── skill.rs                  # 技能管理
├── pipe.rs                   # 管道管理
├── memory.rs                 # 内存管理
├── radix_tree.rs             # 基数树实现
├── lease.rs                  # 租约机制
├── file_handle.rs            # 文件句柄
├── metadata.rs               # 元数据
├── dir_entry.rs              # 目录项
├── open_options.rs           # 打开选项
├── create_options.rs         # 创建选项
├── mount_config.rs           # 挂载配置
├── plugin_registry.rs        # 插件注册表
├── plugin_manager.rs         # 插件管理器
├── router.rs                 # 路由器
├── request.rs                # 请求处理
├── response.rs               # 响应处理
├── middleware.rs             # 中间件
└── ...                       # 其他核心模块
```

## 2. 插件 Trait

```rust
// crates/evif-core/src/plugin.rs

use async_trait::async_trait;
use bytes::Bytes;
use std::path::Path;
use crate::{EvifResult, FileHandle, Metadata, DirEntry, CreateOptions};

/// 插件 Trait - 所有文件系统插件必须实现
#[async_trait]
pub trait EvifPlugin: Send + Sync {
    /// 创建文件
    async fn create(&self, path: &Path, options: CreateOptions) -> EvifResult<FileHandle>;

    /// 读取文件内容
    async fn read(&self, handle: &FileHandle, offset: u64, size: u64) -> EvifResult<Bytes>;

    /// 写入文件内容
    async fn write(&self, handle: &FileHandle, offset: u64, data: Bytes) -> EvifResult<u64>;

    /// 追加写入
    async fn append(&self, handle: &FileHandle, data: Bytes) -> EvifResult<u64>;

    /// 读取目录
    async fn readdir(&self, path: &Path, offset: u64) -> EvifResult<Vec<DirEntry>>;

    /// 获取元数据
    async fn stat(&self, path: &Path) -> EvifResult<Metadata>;

    /// 删除文件
    async fn remove(&self, path: &Path) -> EvifResult<()>;

    /// 重命名
    async fn rename(&self, old: &Path, new: &Path) -> EvifResult<()>;

    /// 递归删除
    async fn remove_all(&self, path: &Path) -> EvifResult<u32>;

    /// 创建目录
    async fn mkdir(&self, path: &Path, mode: u32) -> EvifResult<()>;

    /// 删除目录
    async fn rmdir(&self, path: &Path) -> EvifResult<()>;

    /// 复制文件
    async fn copy(&self, src: &Path, dst: &Path) -> EvifResult<u64>;

    /// 获取文件系统信息
    async fn fsinfo(&self) -> EvifResult<FsInfo>;
}
```

## 3. Radix Mount Table

基数树实现 O(k) 路径路由，k 为路径深度：

```rust
// crates/evif-core/src/mount_table.rs

use std::collections::HashMap;
use std::sync::Arc;

/// Radix Mount Table - 高效路径路由
pub struct RadixMountTable {
    root: MountNode,
    plugins: HashMap<String, Arc<dyn EvifPlugin>>,
}

struct MountNode {
    children: HashMap<String, MountNode>,
    plugin: Option<Arc<dyn EvifPlugin>>,
    wildcard: Option<Arc<dyn EvifPlugin>>,  // 通配符路径 *
}

impl RadixMountTable {
    /// 挂载插件到路径
    pub fn mount(&mut self, path: &str, plugin: Arc<dyn EvifPlugin>) -> EvifResult<()> {
        let path = Self::normalize_path(path)?;
        self.insert_node(&path, plugin)
    }

    /// 解析路径到插件
    pub fn resolve(&self, path: &str) -> Option<Arc<dyn EvifPlugin>> {
        let path = Self::normalize_path(path)?;
        self.resolve_recursive(&path, 0)
    }

    /// 解析递归
    fn resolve_recursive(&self, path: &[&str], depth: usize) -> Option<Arc<dyn EvifPlugin>> {
        if path.is_empty() {
            return self.plugin.clone();
        }

        let segment = path[0];

        // 精确匹配
        if let Some(child) = self.children.get(segment) {
            if let Some(plugin) = child.resolve_recursive(&path[1..], depth + 1) {
                return Some(plugin);
            }
        }

        // 通配符匹配 (如 /context/*)
        if let Some(child) = self.children.get("*") {
            return child.plugin.clone();
        }

        // 前缀匹配 (如 /data/...)
        self.wildcard.clone()
    }

    /// 规范化路径
    fn normalize_path(path: &str) -> EvifResult<Vec<&str>> {
        let parts: Vec<&str> = path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        Ok(parts)
    }
}
```

### 挂载示例

```
/mem ────────────► memfs
/context/* ──────► contextfs
/skills/* ───────► skillfs
/pipes/* ────────► pipefs
/queue/* ────────► queuefs
/data ───────────► localfs
/s3 ─────────────► s3fs
```

## 4. Handle Manager

Handle 管理器负责文件句柄的创建、追踪和释放：

```rust
// crates/evif-core/src/handle.rs

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;

/// Handle 管理器
pub struct HandleManager {
    handles: RwLock<HashMap<u64, FileHandle>>,
    counter: AtomicU64,
    leases: RwLock<HashMap<u64, Lease>>,
}

impl HandleManager {
    /// 创建 Handle
    pub async fn open(&self, path: &Path, flags: OpenFlags) -> EvifResult<u64> {
        let handle_id = self.counter.fetch_add(1, Ordering::SeqCst);

        let handle = FileHandle::new(
            handle_id,
            path.to_path_buf(),
            flags,
            Lease::new(60), // 默认 60 秒租约
        );

        self.handles.write().await.insert(handle_id, handle);

        Ok(handle_id)
    }

    /// 获取 Handle
    pub async fn get(&self, id: u64) -> EvifResult<FileHandle> {
        self.handles.read()
            .await
            .get(&id)
            .cloned()
            .ok_or(EvifError::HandleNotFound(id))
    }

    /// 关闭 Handle
    pub async fn close(&self, id: u64) -> EvifResult<()> {
        self.handles.write()
            .await
            .remove(&id)
            .ok_or(EvifError::HandleNotFound(id))?;
        Ok(())
    }

    /// 续租
    pub async fn renew(&self, id: u64, seconds: u64) -> EvifResult<()> {
        let mut leases = self.leases.write().await;
        let lease = leases.entry(id).or_insert_with(|| Lease::new(0));
        lease.renew(Duration::from_secs(seconds));
        Ok(())
    }

    /// 清理过期 Handle
    pub async fn cleanup(&self) -> u64 {
        let now = Instant::now();
        let mut handles = self.handles.write().await;
        let mut count = 0;

        handles.retain(|id, handle| {
            if self.is_expired(&handle.lease, now) {
                count += 1;
                false
            } else {
                true
            }
        });

        count
    }
}

/// 租约
struct Lease {
    expires: Instant,
    duration: Duration,
}

impl Lease {
    fn new(seconds: u64) -> Self {
        Self {
            expires: Instant::now() + Duration::from_secs(seconds),
            duration: Duration::from_secs(seconds),
        }
    }

    fn renew(&mut self, duration: Duration) {
        self.expires = Instant::now() + duration;
        self.duration = duration;
    }

    fn is_expired(&self, now: Instant) -> bool {
        now > self.expires
    }
}
```

## 5. Cache Manager

多级缓存支持：

```rust
// crates/evif-core/src/cache.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 缓存条目
struct CacheEntry {
    data: Bytes,
    size: usize,
    hits: AtomicU64,
    created: Instant,
    ttl: Duration,
}

/// 缓存管理器
pub struct CacheManager {
    entries: RwLock<HashMap<String, CacheEntry>>,
    max_entries: usize,
    max_size_bytes: usize,
    current_size: AtomicUsize,
}

impl CacheManager {
    /// 获取缓存
    pub async fn get(&self, key: &str) -> Option<Bytes> {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(key) {
            if !entry.is_expired() {
                entry.hits.fetch_add(1, Ordering::SeqCst);
                return Some(entry.data.clone());
            }
        }
        None
    }

    /// 设置缓存
    pub async fn set(&self, key: String, data: Bytes) {
        let size = data.len();
        let entry = CacheEntry::new(data);

        let mut entries = self.entries.write().await;

        // 移除过期条目
        entries.retain(|_, e| !e.is_expired());

        // 如果超过最大条目数，移除最少使用的
        while entries.len() >= self.max_entries {
            if let Some(lru) = self.find_lru(&entries) {
                entries.remove(&lru);
            }
        }

        entries.insert(key, entry);
    }

    /// 失效缓存
    pub async fn invalidate(&self, pattern: &str) {
        let mut entries = self.entries.write().await;
        entries.retain(|key, _| !key.starts_with(pattern));
    }

    /// 统计信息
    pub fn stats(&self) -> CacheStats {
        let entries = self.entries.blocking_read();
        let total_hits: u64 = entries.values().map(|e| e.hits.load(Ordering::SeqCst)).sum();
        CacheStats {
            entries: entries.len(),
            hits: total_hits,
            size_bytes: self.current_size.load(Ordering::SeqCst),
        }
    }
}
```

## 6. Circuit Breaker

熔断器模式防止级联故障：

```rust
// crates/evif-core/src/circuit_breaker.rs

use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub enum CircuitState {
    Closed,    // 正常
    Open,      // 熔断
    HalfOpen,   // 半开 (探测恢复)
}

/// 熔断器
pub struct CircuitBreaker {
    state: AtomicBool,
    failure_count: AtomicU64,
    last_failure: AtomicU64,
    threshold: u64,
    timeout: Duration,
}

impl CircuitBreaker {
    /// 检查是否可以执行
    pub fn can_execute(&self) -> bool {
        if !self.state.load(Ordering::SeqCst) {
            return true; // Closed
        }

        // 检查超时是否过期
        let last = self.last_failure.load(Ordering::SeqCst);
        let elapsed = Instant::now()
            .duration_since(Instant::UNIX_EPOCH + Duration::from_nanos(last));

        elapsed > self.timeout
    }

    /// 记录失败
    pub fn record_failure(&self) {
        self.failure_count.fetch_add(1, Ordering::SeqCst);
        self.last_failure.store(
            Instant::now().duration_since(Instant::UNIX_EPOCH).as_nanos() as u64,
            Ordering::SeqCst,
        );

        if self.failure_count.load(Ordering::SeqCst) >= self.threshold {
            self.state.store(true, Ordering::SeqCst); // Open
        }
    }

    /// 记录成功
    pub fn record_success(&self) {
        self.failure_count.store(0, Ordering::SeqCst);
        self.state.store(false, Ordering::SeqCst); // Closed
    }

    /// 获取状态
    pub fn state(&self) -> CircuitState {
        if self.state.load(Ordering::SeqCst) {
            CircuitState::Open
        } else {
            CircuitState::Closed
        }
    }
}
```

## 7. Batch Operations

批量操作优化多次小请求：

```rust
// crates/evif-core/src/batch.rs

/// 批量操作类型
pub enum BatchOp {
    Read(PathBuf, u64, u64),       // path, offset, size
    Write(PathBuf, u64, Vec<u8>), // path, offset, data
    Delete(PathBuf),               // path
    Create(PathBuf, u32),          // path, mode
}

/// 批量操作结果
pub struct BatchResult {
    pub results: Vec<Result<Vec<u8>, EvifError>>,
    pub total_bytes: u64,
    pub duration_ms: u64,
}

/// 批量处理器
pub struct BatchProcessor {
    max_batch_size: usize,
    timeout: Duration,
}

impl BatchProcessor {
    /// 处理批量操作
    pub async fn process(&self, ops: Vec<BatchOp>) -> EvifResult<BatchResult> {
        let start = Instant::now();

        let mut results = Vec::with_capacity(ops.len());
        let mut total_bytes = 0u64;

        for op in ops {
            let result = match op {
                BatchOp::Read(path, offset, size) => {
                    self.read_batch(&path, offset, size).await
                }
                BatchOp::Write(path, offset, data) => {
                    total_bytes += data.len() as u64;
                    self.write_batch(&path, offset, data).await
                }
                BatchOp::Delete(path) => self.delete_batch(&path).await,
                BatchOp::Create(path, mode) => self.create_batch(&path, mode).await,
            };
            results.push(result);
        }

        Ok(BatchResult {
            results,
            total_bytes,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}
```

## 8. 错误处理

```rust
// crates/evif-core/src/error.rs

use thiserror::Error;

#[derive(Debug, Error)]
pub enum EvifError {
    #[error("path not found: {0}")]
    PathNotFound(String),

    #[error("path exists: {0}")]
    PathExists(String),

    #[error("is directory: {0}")]
    IsDirectory(String),

    #[error("not directory: {0}")]
    NotDirectory(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("handle not found: {0}")]
    HandleNotFound(u64),

    #[error("handle expired: {0}")]
    HandleExpired(u64),

    #[error("circuit breaker open")]
    CircuitBreakerOpen,

    #[error("invalid path: {0}")]
    InvalidPath(String),

    #[error("quota exceeded")]
    QuotaExceeded,

    #[error("plugin error: {0}")]
    Plugin(String),
}

/// 结果别名
pub type EvifResult<T> = Result<T, EvifError>;
```

## 9. 上下文管理

```rust
// crates/evif-core/src/context.rs

/// 上下文层级
pub enum ContextLayer {
    /// L0: 当前任务 (临时)
    L0,
    /// L1: 会话决策 (持久)
    L1,
    /// L2: 项目知识 (长期)
    L2,
}

/// 上下文管理器
pub struct ContextManager {
    layers: HashMap<ContextLayer, PathBuf>,
    filesystem: Arc<dyn EvifPlugin>,
}

impl ContextManager {
    /// 读取上下文
    pub async fn get(&self, layer: ContextLayer) -> EvifResult<String> {
        let path = self.layer_path(layer)?;
        let handle = self.filesystem.create(&path, CreateOptions::default()).await?;
        let data = self.filesystem.read(&handle, 0, usize::MAX as u64).await?;
        Ok(String::from_utf8_lossy(&data).to_string())
    }

    /// 写入上下文
    pub async fn set(&self, layer: ContextLayer, content: &str) -> EvifResult<()> {
        let path = self.layer_path(layer)?;
        // 写入逻辑...
        Ok(())
    }

    /// 追加上下文
    pub async fn append(&self, layer: ContextLayer, content: &str) -> EvifResult<()> {
        // 追加逻辑...
        Ok(())
    }

    fn layer_path(&self, layer: ContextLayer) -> EvifResult<PathBuf> {
        self.layers.get(&layer)
            .cloned()
            .ok_or(EvifError::InvalidPath(format!("invalid layer: {:?}", layer)))
    }
}
```

## 10. 相关文档

- [架构概览](00-overview.md)
- [插件系统](02-plugin-system.md)
- [REST API 参考](03-rest-api.md)