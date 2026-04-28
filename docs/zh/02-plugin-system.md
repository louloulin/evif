# EVIF 插件系统

40+ 插件实现，~19,000 行 Rust 代码。插件是 EVIF 的核心扩展机制。

## 1. 插件架构

```
┌─────────────────────────────────────────────────────────────┐
│                     Plugin Registry                         │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐           │
│  │   memfs     │ │   contextfs │ │   skillfs   │           │
│  │   localfs   │ │   pipefs    │ │   queuefs   │           │
│  │   s3fs      │ │   gcsfs     │ │   azurefs   │           │
│  └─────────────┘ └─────────────┘ └─────────────┘           │
└─────────────────────────────────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────┐
│                      Plugin Trait                           │
│  create() | read() | write() | readdir() | stat()          │
│  remove() | rename() | remove_all() | mkdir() | ...        │
└─────────────────────────────────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────┐
│                    插件实现                                 │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐           │
│  │  文件存储    │ │   云存储    │ │   数据库    │           │
│  │  memfs      │ │   s3fs      │ │  SQLiteFS   │           │
│  │  localfs    │ │   gcsfs     │ │ PostgreSQLFS│           │
│  │             │ │  azurefs    │ │             │           │
│  └─────────────┘ └─────────────┘ └─────────────┘           │
└─────────────────────────────────────────────────────────────┘
```

## 2. 智能体原语插件

### 2.1 ContextFS - 上下文文件系统

三层上下文系统，为智能体提供跨会话持久化：

```
/context/
├── L0/            # 当前任务 (临时，进程生命周期)
│   └── current    # 单行描述当前任务
├── L1/            # 会话决策 (持久)
│   └── decisions.md  # 带时间戳的决策记录
└── L2/            # 项目知识 (长期)
    ├── architecture.md
    └── patterns.md
```

**特性**：
- L0: 单行当前任务，快速读写
- L1: Markdown 格式决策记录，可追加
- L2: 结构化项目知识，支持全文搜索

**使用示例**：
```bash
# 会话开始 - 恢复上下文
evif cat /context/L0/current          # "实现认证模块"
evif cat /context/L1/decisions.md     # 查看历史决策

# 会话中 - 记录决策
evif write -c "选择 JWT 而非 Session" /context/L1/decisions.md

# 会话结束 - 保存状态
evif write -c "完成 JWT 实现，待测试" /context/L0/current
```

### 2.2 SkillFS - 技能文件系统

可复用的 SKILL.md 工作流：

```
/skills/
├── code-review/       # 代码审查技能
│   ├── SKILL.md
│   ├── input         # 输入文件路径
│   └── output        # 审查结果
├── test-gen/         # 测试生成技能
│   ├── SKILL.md
│   ├── input
│   └── output
├── doc-gen/          # 文档生成技能
│   ├── SKILL.md
│   ├── input
│   └── output
└── security-audit/   # 安全审计技能
    ├── SKILL.md
    ├── input
    └── output
```

**SKILL.md 格式**：
```yaml
---
name: code-review
description: "代码安全审查"
triggers:
  - "review code"
  - "代码审查"
version: "1.0"
---

# Code Review Skill

## 输入
将待审查的代码路径写入 `input` 文件

## 执行步骤
1. 读取 input 文件中的代码路径
2. 分析代码安全问题
3. 输出审查报告到 output 文件

## 输出格式
- 发现的问题列表
- 严重程度评级
- 修复建议
```

**使用示例**：
```bash
# 运行代码审查技能
evif write -c "Review src/auth/login.rs" /skills/code-review/input

# 等待执行完成
sleep 2

# 查看结果
evif cat /skills/code-review/output
```

### 2.3 PipeFS - 管道文件系统

多智能体消息传递和任务协调：

```
/pipes/
├── review-pr-123/     # PR 审查管道
│   ├── input          # 任务输入
│   ├── output         # 任务输出
│   ├── status         # pending/running/complete/error
│   ├── assignee       # 当前处理者
│   └── metadata      # 附加信息
└── my-task/          # 自定义管道
    ├── input
    ├── output
    └── status
```

**状态机**：
```
pending ──► running ──► complete
    │          │
    └──────────┴──► error
```

**使用示例**：
```bash
# 创建管道
evif mkdir /pipes/review-auth

# 写入任务
evif write -c "Review auth module for security" /pipes/review-auth/input

# 设置为待处理
evif write -c "pending" /pipes/review-auth/status

# 另一个智能体认领任务
evif write -c "worker-1" /pipes/review-auth/assignee
evif write -c "running" /pipes/review-auth/status

# 执行并完成
evif write -c "Found SQL injection vulnerability" /pipes/review-auth/output
evif write -c "complete" /pipes/review-auth/status
```

### 2.4 QueueFS - 队列文件系统

任务队列，支持 enqueue/dequeue 原子操作：

```
/queue/
└── tasks/            # 任务队列
    ├── enqueue       # 写入数据入队
    ├── dequeue       # 读取数据出队 (原子)
    ├── size          # 当前队列长度
    └── peek          # 查看队首元素
```

**使用示例**：
```bash
# 入队任务
evif write -c '{"type": "review", "target": "src/auth/"}' /queue/tasks/enqueue
evif write -c '{"type": "build", "target": "v1.2.0"}' /queue/tasks/enqueue

# 查看队列大小
evif cat /queue/tasks/size

# 出队任务 (原子操作)
evif cat /queue/tasks/dequeue

# 再次查看大小
evif cat /queue/tasks/size
```

### 2.5 VectorFS - 向量内存

向量搜索，支持语义记忆：

```
/memories/
├── index/            # 向量索引
├── store/            # 存储数据
└── metadata/         # 元数据
```

**使用示例**：
```python
from evif import Client

client = Client("http://localhost:8081")

# 存储记忆
client.memory_store(
    "用户偏好深色主题",
    modality="preference",
    metadata={"category": "ui", "source": "user"}
)

# 语义搜索
results = client.memory_search("主题设置")
for r in results:
    print(f"{r['score']:.2f}: {r['content']}")
```

## 3. 存储插件

### 3.1 memfs - 内存文件系统

进程内存储，临时数据：

```rust
// crates/evif-plugins/src/memfs.rs

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct MemFS {
    storage: RwLock<HashMap<PathBuf, Bytes>>,
}

impl MemFS {
    pub fn new() -> Self {
        Self {
            storage: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl EvifPlugin for MemFS {
    async fn create(&self, path: &Path, options: CreateOptions) -> EvifResult<FileHandle> {
        let storage = self.storage.write().unwrap();
        storage.insert(path.to_path_buf(), Bytes::new());
        Ok(FileHandle::new(path))
    }

    async fn read(&self, handle: &FileHandle, offset: u64, size: u64) -> EvifResult<Bytes> {
        let storage = self.storage.read().unwrap();
        storage.get(&handle.path)
            .map(|data| data.slice(offset as usize..))
            .ok_or(EvifError::PathNotFound(handle.path.to_string_lossy().to_string()))
    }

    async fn write(&self, handle: &FileHandle, offset: u64, data: Bytes) -> EvifResult<u64> {
        let mut storage = self.storage.write().unwrap();
        storage.insert(handle.path.clone(), data.clone());
        Ok(data.len() as u64)
    }

    // ... 其他方法
}
```

### 3.2 localfs - 本地文件系统

持久化到本地磁盘：

```rust
// crates/evif-plugins/src/localfs.rs

pub struct LocalFS {
    root: PathBuf,
}

impl LocalFS {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    fn resolve_path(&self, path: &Path) -> PathBuf {
        self.root.join(path)
    }
}
```

### 3.3 云存储插件

| 插件 | 云服务 | 协议 |
|------|--------|------|
| s3fs | Amazon S3 | AWS SDK |
| gcsfs | Google Cloud Storage | GCS SDK |
| azurefs | Azure Blob Storage | Azure SDK |
| ossfs | Alibaba OSS | OSS SDK |
| miniofs | MinIO | S3 兼容 |

**配置示例**：
```toml
[[mounts]]
path = "/s3"
plugin = "s3fs"
config = {
    bucket = "my-bucket",
    region = "us-east-1",
    access_key = "...",
    secret_key = "..."
}
```

## 4. 数据库插件

### 4.1 SQLiteFS

```toml
[[mounts]]
path = "/sqlite"
plugin = "sqlitefs"
config = {
    path = "/var/evif/data.db"
}
```

### 4.2 PostgreSQLFS

```toml
[[mounts]]
path = "/postgres"
plugin = "postgresqlfs"
config = {
    connection_string = "postgresql://user:pass@localhost/db"
}
```

## 5. 增强插件

### 5.1 EncryptedFS

透明加密插件：

```toml
[[mounts]]
path = "/secure"
plugin = "encryptedfs"
config = {
    key = "your-encryption-key",
    algorithm = "AES-256-GCM"
}
```

### 5.2 TieredFS

自动分层存储：

```toml
[[mounts]]
path = "/tiered"
plugin = "tieredfs"
config = {
    hot_tier = "/mem",
    warm_tier = "/data",
    cold_tier = "/s3",
    move_after_days = 30
}
```

### 5.3 SnapshotFS

快照备份：

```toml
[[mounts]]
path = "/snapshots"
plugin = "snapshotfs"
config = {
    source = "/data",
    schedule = "0 2 * * *",  # 每天凌晨 2 点
    retention_days = 7
}
```

## 6. 创建新插件

### 6.1 插件结构

```rust
// crates/evif-plugins/src/myplugin.rs

use async_trait::async_trait;
use bytes::Bytes;
use evif_core::{EvifPlugin, EvifError, EvifResult, FileHandle, CreateOptions};
use std::path::Path;
use std::sync::Arc;

/// 插件配置
#[derive(Debug, Clone)]
pub struct MyConfig {
    pub option1: String,
    pub option2: bool,
}

/// 我的自定义插件
pub struct MyPlugin {
    config: MyConfig,
}

impl MyPlugin {
    pub fn new(config: Option<&Value>) -> EvifResult<Self> {
        let config = config
            .and_then(|c| serde_json::from_value(c.clone()).ok())
            .unwrap_or(MyConfig {
                option1: "default".to_string(),
                option2: false,
            });
        Ok(Self { config })
    }
}

#[async_trait]
impl EvifPlugin for MyPlugin {
    async fn create(&self, path: &Path, options: CreateOptions) -> EvifResult<FileHandle> {
        Ok(FileHandle::new(path))
    }

    async fn read(&self, handle: &FileHandle, offset: u64, size: u64) -> EvifResult<Bytes> {
        Ok(Bytes::new())
    }

    async fn write(&self, handle: &FileHandle, offset: u64, data: Bytes) -> EvifResult<u64> {
        Ok(data.len() as u64)
    }

    async fn readdir(&self, path: &Path, offset: u64) -> EvifResult<Vec<DirEntry>> {
        Ok(vec![])
    }

    async fn stat(&self, path: &Path) -> EvifResult<Metadata> {
        Ok(Metadata::default())
    }

    async fn remove(&self, path: &Path) -> EvifResult<()> {
        Ok(())
    }

    async fn rename(&self, old: &Path, new: &Path) -> EvifResult<()> {
        Ok(())
    }

    async fn remove_all(&self, path: &Path) -> EvifResult<u32> {
        Ok(0)
    }
}

/// 插件工厂函数
pub fn create_my_plugin(config: Option<&Value>) -> EvifResult<Arc<dyn EvifPlugin>> {
    Ok(Arc::new(MyPlugin::new(config)?))
}
```

### 6.2 注册插件

```rust
// crates/evif-plugins/src/lib.rs

pub mod myplugin;
pub use myplugin::create_my_plugin;
```

### 6.3 添加到构建

```toml
# crates/evif-plugins/Cargo.toml

[features]
default = ["memfs", "localfs", "contextfs", ...]
myplugin = []
```

## 7. 插件对比

| 插件 | 类型 | 持久化 | 性能 | 用途 |
|------|------|--------|------|------|
| memfs | 内存 | 否 | 最快 | 临时数据、缓存 |
| localfs | 磁盘 | 是 | 快 | 持久存储 |
| contextfs | 虚拟 | 是 | 快 | 智能体上下文 |
| skillfs | 虚拟 | 是 | 快 | 技能执行 |
| pipefs | 虚拟 | 是 | 快 | 多智能体通信 |
| queuefs | 虚拟 | 是 | 快 | 任务队列 |
| s3fs | 云 | 是 | 中 | 云存储 |
| gcsfs | 云 | 是 | 中 | 云存储 |
| sqlitefs | 数据库 | 是 | 中 | 结构化数据 |

## 8. 相关文档

- [核心模块](01-core-modules.md)
- [开发者指南](../07-developer-guide.md)
- [REST API 参考](03-rest-api.md)