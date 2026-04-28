# EVIF Plugin System

## 1. Overview

The `evif-plugins` crate provides ~40 plugin implementations across categories. Total codebase: ~19,000 lines of Rust.

## 2. Module Structure

```
crates/evif-plugins/src/
├── lib.rs                     # Plugin exports and catalog
├── catalog.rs                 # Plugin registry and discovery
│
├── ── Agent Primitives ──
├── contextfs.rs               # Three-layer context (957 LOC)
├── context_manager.rs         # Context search and management
├── skillfs.rs                # SKILL.md workflow execution (1159 LOC)
├── skill_runtime.rs          # Skill execution runtime
├── pipefs.rs                 # Multi-agent coordination
├── queuefs.rs                # FIFO task queue (1578 LOC)
├── vectorfs.rs               # Vector/memory storage
│
├── ── Storage Backends ──
├── memfs.rs                  # In-memory storage
├── localfs.rs               # Local filesystem
├── kvfs.rs                   # Key-value store
├── s3fs.rs                  # AWS S3
├── s3fs_opendal.rs          # S3 via OpenDAL
├── gcsfs.rs                 # Google Cloud Storage
├── azureblobfs.rs           # Azure Blob Storage
├── aliyunossfs.rs           # Alibaba Cloud OSS
├── tencentcosfs.rs          # Tencent Cloud COS
├── huaweiobsfs.rs           # Huawei Cloud OBS
├── miniofs.rs               # MinIO (S3-compatible)
├── sqlfs.rs                 # SQL database (MySQL/PostgreSQL)
├── sqlfs2.rs                # SQLite
├── ftpfs.rs                 # FTP
├── sftpfs.rs               # SFTP
├── webdavfs.rs             # WebDAV
├── httpfs.rs               # HTTP(S)
├── opendal.rs              # Apache OpenDAL
│
├── ── Enhancement Plugins ──
├── encryptedfs.rs           # AES-GCM encryption
├── streamfs.rs              # Streaming wrapper
├── streamrotatefs.rs        # Log rotation
├── tieredfs.rs             # Hot/warm/cold tiering
├── handlefs.rs              # Handle operations
├── proxyfs.rs              # Proxy to another EVIF
│
├── ── Utility Plugins ──
├── devfs.rs                 # /dev/null-like
├── hellofs.rs              # Hello world plugin
├── heartbeatfs.rs          # Health check plugin
├── serverinfofs.rs         # Server info
├── gptfs.rs               # LLM integration
└── catalog.rs              # Plugin catalog
```

## 3. Agent Primitive Plugins

### 3.1 ContextFS

Three-layer persistent context for AI agents.

#### File Structure

```
/context/
├── L0/                    # Immediate context (session)
│   ├── current           # Current task description
│   ├── recent_ops        # Recent operations
│   └── budget_status     # Token budget
├── L1/                    # Session context (persistent)
│   ├── decisions.md       # Decisions with rationale
│   └── scratch/          # Temporary workspace
└── L2/                    # Project knowledge (long-term)
    ├── architecture.md    # Architecture docs
    ├── patterns.md       # Code patterns
    ├── runbooks/         # Operational guides
    └── history/          # Historical sessions
```

#### Key Implementation

```rust
pub struct ContextFsPlugin {
    layers: Arc<[ContextLayer]>,
    budget: ContextTokenBudget,
    search_index: Arc<ContextSearchIndex>,
}

pub struct ContextLayer {
    name: String,
    path: PathBuf,
    plugin: Arc<dyn EvifPlugin>,
    persistence: Persistence,
}

pub struct ContextTokenBudget {
    pub total: u64,
    pub used: AtomicU64,
    pub warning_threshold: f32,
}
```

#### Usage

```bash
# Set current task
evif write -c "Review PR #123" /context/L0/current

# Record decision
evif write -c "Chose JWT over sessions for stateless auth" /context/L1/decisions.md

# Update architecture
evif write -c "# Architecture\n\n..." /context/L2/architecture.md

# Search context
curl -X POST /api/v1/context/search -d '{"query": "authentication"}'
```

### 3.2 SkillFS

Reusable workflow skills with SKILL.md standard.

#### SKILL.md Format

```yaml
---
name: code-review
description: "Review code for bugs, security issues, and maintainability"
triggers:
  - "review"
  - "code review"
  - "check my code"
version: "1.0"
author: "team@example.com"
---

# Code Review Workflow

## Input
Target code path or content

## Process
1. Parse target files
2. Identify language and framework
3. Run static analysis
4. Check common vulnerability patterns
5. Generate review report

## Output
Structured review report with severity levels

## Example
```bash
evif write -c "Review src/auth/login.rs" /skills/code-review/input
```
```

#### Execution Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ User writes │     │ SkillFS     │     │ Output      │
│ to input    │────▶│ executes    │────▶│ available   │
└─────────────┘     └─────────────┘     └─────────────┘
```

#### Key Implementation

```rust
pub struct SkillFsPlugin {
    skills_dir: PathBuf,
    runtime: Arc<SkillRuntime>,
    registry: SkillRegistry,
}

pub struct SkillRuntime {
    executor: Arc<dyn SkillExecutor>,
    sandbox: Option<Arc<dyn Sandbox>>,
}

pub struct Skill {
    pub name: String,
    pub description: String,
    pub triggers: Vec<String>,
    pub content: String,
    pub front_matter: FrontMatter,
}

pub struct FrontMatter {
    pub name: String,
    pub description: String,
    pub triggers: Vec<String>,
    pub version: Option<String>,
    pub author: Option<String>,
}
```

### 3.3 PipeFS

Multi-agent coordination via state machine.

#### Pipe Structure

```
/pipes/{name}/
├── input      # Task input
├── output     # Task result
├── status     # pending → running → complete/error/timeout
├── assignee   # Worker agent ID
├── timeout    # TTL in seconds
├── priority   # Task priority
└── metadata/  # Additional metadata
    ├── created_at
    ├── started_at
    ├── completed_at
    └── error_message
```

#### Status Transitions

```
     ┌──────────┐
     │ pending  │
     └────┬─────┘
          │ agent claims
          ▼
     ┌──────────┐
     │ running  │◀──────────┐
     └────┬─────┘           │
          │ success         │ agent claims
          ▼                 │
     ┌──────────┐           │
     │ complete  │───────────┘
     └──────────┘
          │
          │ failure
          ▼
     ┌──────────┐
     │  error   │
     └──────────┘
```

#### Key Implementation

```rust
pub struct PipeFsPlugin {
    pipes_root: PathBuf,
    state_machine: Arc<PipeStateMachine>,
    timeout_manager: Arc<TimeoutManager>,
}

pub enum PipeStatus {
    Pending,
    Running,
    Complete,
    Error,
    Timeout,
}

pub struct PipeStateMachine {
    transitions: HashMap<PipeStatus, Vec<Transition>>,
}

impl PipeStateMachine {
    pub fn transition(&self, pipe: &mut Pipe, event: Event) -> Result<(), InvalidTransition> {
        // Validate state transitions
        // Update status atomically
        // Emit events
    }
}
```

### 3.4 QueueFS

FIFO task queue with atomic operations.

#### Queue Structure

```
/queue/{name}/
├── enqueue    # Write-only, appends item
├── dequeue    # Read + delete (atomic)
├── size       # Current queue depth
├── peek       # Read without delete
└── items/     # Item storage (internal)
    ├── 00001.json
    ├── 00002.json
    └── ...
```

#### Key Implementation

```rust
pub struct QueueFsPlugin {
    queues_root: PathBuf,
    backend: Arc<dyn QueueBackend>,
    atomic_ops: AtomicQueueOps,
}

pub trait QueueBackend: Send + Sync {
    async fn enqueue(&self, queue: &str, item: &[u8]) -> Result<u64, EvifError>;
    async fn dequeue(&self, queue: &str) -> Result<Option<(u64, Vec<u8>)>, EvifError>;
    async fn size(&self, queue: &str) -> Result<u64, EvifError>;
}

pub struct AtomicQueueOps {
    // Ensures exactly-once dequeue
    // Uses compare-and-swap
}
```

### 3.5 VectorFS

Semantic memory with vector embeddings.

#### Key Implementation

```rust
pub struct VectorFsPlugin {
    storage: Arc<dyn VectorStorage>,
    embedder: Arc<dyn Embedder>,
    dimension: usize,
}

pub trait VectorStorage: Send + Sync {
    async fn insert(&self, id: &str, vector: &[f32], metadata: Value) -> Result<(), EvifError>;
    async fn search(&self, query: &[f32], limit: usize) -> Result<Vec<SearchResult>, EvifError>;
    async fn delete(&self, id: &str) -> Result<(), EvifError>;
}

pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EvifError>;
}
```

## 4. Storage Plugins

### 4.1 Memory Storage (MemFS)

```rust
pub struct MemFsPlugin {
    storage: Arc<DashMap<String, MemFile>>,
    max_size: usize,
}

struct MemFile {
    content: Bytes,
    metadata: Metadata,
    created: Instant,
    modified: Instant,
}
```

### 4.2 Local Storage (LocalFS)

```rust
pub struct LocalFsPlugin {
    root: PathBuf,
    follow_symlinks: bool,
}
```

### 4.3 Cloud Storage

| Plugin | Backend | Auth | Features |
|--------|---------|------|----------|
| S3FS | AWS S3 | IAM/Keys | Multipart, presigned URLs |
| GCSFS | Google Cloud | OAuth | Resumable uploads |
| AzureBlobFS | Azure | SAS/Managed ID | Append blobs |
| AliyunOSSFS | Alibaba | AccessKey | VPC support |
| TencentCOSFS | Tencent | SecretKey | COS v5 API |
| HuaweiOBSFS | Huawei | AK/SK | OBS SDK v3 |
| MinIOFS | MinIO | Keys | S3-compatible |

### 4.4 SQL Storage (SQLFS)

```rust
pub struct SqlFsPlugin {
    pool: SqlPool,
    table_prefix: String,
}

impl SqlFsPlugin {
    // Maps filesystem ops to SQL
    // CREATE TABLE IF NOT EXISTS fs_{path_hash}
    // Each row: path, content, metadata, created, modified
}
```

## 5. Enhancement Plugins

### 5.1 EncryptedFS

AES-GCM transparent encryption.

```rust
pub struct EncryptedFsPlugin {
    inner: Arc<dyn EvifPlugin>,
    cipher: AesGcm,
    key_id: String,
}

impl EncryptedFsPlugin {
    // Encrypt on write, decrypt on read
    // Key rotation support
    // IV/nonce management
}
```

### 5.2 TieredFS

Hot/warm/cold data tiering.

```rust
pub struct TieredFsPlugin {
    hot: Arc<dyn EvifPlugin>,    // SSD, fast
    warm: Arc<dyn EvifPlugin>,    // HDD, medium
    cold: Arc<dyn EvifPlugin>,    // Object storage, slow
    policy: TieringPolicy,
}

pub enum TieringPolicy {
    AgeBased { hot_days: u32, warm_days: u32 },
    AccessBased { hot_threshold: u32 },
    Custom(Box<dyn TieringEvaluator>),
}
```

### 5.3 StreamRotateFS

Log file rotation.

```rust
pub struct StreamRotateFsPlugin {
    inner: Arc<dyn EvifPlugin>,
    max_size: u64,
    max_files: usize,
    compression: Compression,
}
```

## 6. Plugin Development

### 6.1 Creating a New Plugin

```rust
use async_trait::async_trait;
use evif_core::{EvifPlugin, EvifError, EvifResult};

pub struct MyPlugin {
    config: MyConfig,
}

#[async_trait]
impl EvifPlugin for MyPlugin {
    async fn init(&self, config: Option<&Value>) -> EvifResult<()> {
        // Initialize plugin
        Ok(())
    }

    async fn create(&self, path: &Path, opts: CreateOptions) -> EvifResult<FileHandle> {
        // Create file
    }

    async fn read(&self, handle: &FileHandle, offset: u64, size: u64) -> EvifResult<Bytes> {
        // Read file
    }

    // ... implement other methods
}
```

### 6.2 Registering a Plugin

```rust
// In lib.rs
pub fn create_my_plugin(config: Option<&Value>) -> EvifResult<Arc<dyn EvifPlugin>> {
    Ok(Arc::new(MyPlugin::new(config)?))
}

// Export
pub use create_my_plugin as create_plugin;
```

### 6.3 WASM Plugin (Extism)

```rust
// examples/wasm-plugin/src/lib.rs
use extism_pdk::*;

#[plugin_fn]
pub fn create(input: CreateInput) -> FnResult<Json<CreateOutput>> {
    let path = input.path;
    // Create file in WASM memory
    // Return handle
    Ok(Json(CreateOutput { handle: 1 }))
}

#[plugin_fn]
pub fn read(input: ReadInput) -> FnResult<Json<ReadOutput>> {
    // Read file from WASM memory
    Ok(Json(ReadOutput { data: vec![] }))
}
```

## 7. Plugin Catalog

All plugins are registered in `catalog.rs`:

```rust
pub struct PluginCatalog {
    plugins: HashMap<String, PluginInfo>,
}

pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: Option<String>,
    pub capabilities: PluginCapabilities,
    pub create_fn: CreatePluginFn,
}
```

## 8. Related Documents

- [Plugin Development Guide](plugin-development.md)
- [Core Modules](01-core-modules.md)
- [REST API Reference](API.md)
