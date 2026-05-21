# EVIF Core Kernel Definitions

> Status: MVP 6.0  
> Purpose: Define 8 core kernels with owner, interfaces, and validation standards

## Kernel Overview

EVIF's core is organized into 8 interconnected kernels that provide the platform's fundamental capabilities:

```
┌─────────────────────────────────────────────────────────────────┐
│                         Access Layer                             │
│  CLI (60+ cmds) │ REST API (150 endpoints) │ MCP Server │ FUSE   │
├─────────────────────────────────────────────────────────────────┤
│                         Kernels                                  │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐            │
│  │ ContextKernel│ │ SkillKernel  │ │MemoryKernel  │            │
│  └──────────────┘ └──────────────┘ └──────────────┘            │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐            │
│  │Coordination  │ │Connectivity  │ │CostOptimize  │            │
│  │  Kernel      │ │  Kernel      │ │  Kernel      │            │
│  └──────────────┘ └──────────────┘ └──────────────┘            │
│  ┌──────────────┐ ┌──────────────┐                             │
│  │  Governance  │ │   Mount      │                             │
│  │  Kernel      │ │  Kernel      │                             │
│  └──────────────┘ └──────────────┘                             │
└─────────────────────────────────────────────────────────────────┘
```

## Kernel 1: Mount Kernel (evif-core)

**Owner**: `evif-core` crate  
**Purpose**: Unified path routing and filesystem mount abstraction

### Interfaces
```rust
// Core trait
pub trait EvifPlugin: Send + Sync {
    async fn create(&self, path: &str, content: &[u8]) -> Result<FileHandle>;
    async fn mkdir(&self, path: &str) -> Result<()>;
    async fn read(&self, handle: &FileHandle, offset: u64, size: u64) -> Result<Vec<u8>>;
    async fn write(&self, handle: &FileHandle, offset: u64, data: &[u8]) -> Result<()>;
    async fn readdir(&self, path: &str) -> Result<Vec<DirEntry>>;
    async fn stat(&self, path: &str) -> Result<FileInfo>;
    async fn remove(&self, path: &str, recursive: bool) -> Result<()>;
    async fn rename(&self, old: &str, new: &str) -> Result<()>;
}

// Mount table
pub struct MountTable {
    mounts: RadixTree<MountEntry>,
}
```

### Validation
```bash
cargo test -p evif-core -- mount
cargo test -p evif-core -- plugin
cargo test -p evif-core -- handle
```

---

## Kernel 2: Context Kernel (evif-plugins/contextfs)

**Owner**: `evif-plugins/contextfs`  
**Purpose**: Layered context management (L0/L1/L2)

### Interfaces
```rust
// Context paths
/context/L0/current      # Current task (one line)
/context/L1/decisions.md # Session decisions
/context/L2/{file}       # Project knowledge

// API
contextfs::read_current() -> String
contextfs::write_current(content: &str) -> Result<()>
contextfs::add_decision(decision: &str) -> Result<()>
contextfs::list_l2() -> Vec<PathBuf>
contextfs::read_l2(name: &str) -> Result<String>
```

### Validation
```bash
cargo test -p evif-plugins -- contextfs
# Expected: All contextfs tests pass
# Coverage: L0 read/write, L1 append, L2 list/read
```

---

## Kernel 3: Skill Kernel (evif-plugins/skillfs)

**Owner**: `evif-plugins/skillfs`  
**Purpose**: Skill discovery and execution

### Interfaces
```rust
// Skill structure
pub struct Skill {
    pub name: String,
    pub description: String,
    pub triggers: Vec<String>,
    pub content: String,  // SKILL.md content
}

// Operations
skillfs::discover() -> Vec<Skill>
skillfs::get(name: &str) -> Result<Skill>
skillfs::execute(name: &str, input: &str) -> Result<SkillResult>
```

### Validation
```bash
cargo test -p evif-plugins -- skillfs
# Expected: All skillfs tests pass
# Coverage: discovery, reading, execution
```

---

## Kernel 4: Memory Kernel (evif-mem)

**Owner**: `evif-mem` crate  
**Purpose**: Long-term memory with semantic search

### Interfaces
```rust
// Core operations
pub async fn memorize(content: &str, tags: Vec<&str>) -> Result<MemoryId>;
pub async fn retrieve(query: &str, k: usize) -> Result<Vec<MemoryEntry>>;
pub async fn search(query: &str, limit: usize, compact: bool) -> Result<SearchResult>;

// Storage backends
pub trait MemoryStorage: Send + Sync {
    async fn store(&self, entry: &MemoryEntry) -> Result<()>;
    async fn query(&self, embedding: &[f32], k: usize) -> Result<Vec<MemoryId>>;
}
```

### Validation
```bash
cargo test -p evif-mem
# Expected: All evif-mem tests pass
# Coverage: embeddings, retrieval, storage
```

---

## Kernel 5: Coordination Kernel (evif-plugins/pipefs + queuefs)

**Owner**: `evif-plugins/pipefs`, `evif-plugins/queuefs`  
**Purpose**: Multi-agent task coordination

### Interfaces
```rust
// Pipe operations
pipefs::create(pipe_id: &str) -> Result<()>
pipefs::write_input(pipe_id: &str, content: &str) -> Result<()>
pipefs::read_input(pipe_id: &str) -> Result<String>
pipefs::claim(pipe_id: &str, timeout: Duration) -> Result<bool>
pipefs::write_output(pipe_id: &str, content: &str) -> Result<()>

// Queue operations
queuefs::enqueue(task: &Task) -> Result<TaskId>
queuefs::dequeue() -> Result<Option<Task>>
queuefs::ack(task_id: &TaskId) -> Result<()>
queuefs::requeue(task_id: &TaskId) -> Result<()>
```

### Validation
```bash
cargo test -p evif-plugins -- pipefs
cargo test -p evif-plugins -- queuefs
# Expected: All coordination tests pass
```

---

## Kernel 6: Connectivity Kernel (evif-plugins catalog)

**Owner**: `evif-plugins` crate  
**Purpose**: Universal system connection layer

### Interfaces
```rust
// Plugin registry
pub struct PluginCatalog {
    pub core: Vec<PluginMetadata>,      // Core Agent primitives
    pub storage: Vec<PluginMetadata>,   // Storage adapters
    pub application: Vec<PluginMetadata>, // SaaS integrations
}

// Plugin capability levels
pub enum SupportTier {
    Core,      // 99.9% SLA
    Stable,    // 99.5% SLA
    Experimental, // Best effort
}
```

### Validation
```bash
cargo test -p evif-plugins -- catalog
# Check connector capability matrix
grep -l "support_tier" docs/connector-capability-matrix.md
```

---

## Kernel 7: Cost Optimization Kernel (evif-mcp + evif-core)

**Owner**: `evif-mcp`, `evif-core`  
**Purpose**: Token and latency optimization

### Interfaces
```rust
// Output filtering
pub struct OutputFilter {
    pub max_lines: usize,
    pub max_string_length: usize,
    pub strip_ansi: bool,
    pub compact_json: bool,
}

// Bounded read
pub async fn bounded_read(
    path: &str,
    max_lines: usize,
    mode: ReadMode,  // head, tail, snippet
) -> Result<String>;

// Compact search
pub async fn compact_search(
    query: &str,
    limit: usize,  // default 3
) -> Result<SearchResult>;
```

### Validation
```bash
# Verify cost optimization features
grep -q "max_lines" crates/evif-mcp/src/lib.rs
grep -q "compact" crates/evif-mcp/src/lib.rs
grep -q "bounded_read" crates/evif-core/src/

# Performance test
cargo bench -p evif-mcp
```

---

## Kernel 8: Governance Kernel (evif-auth)

**Owner**: `evif-auth` crate  
**Purpose**: Security, authentication, and audit

### Interfaces
```rust
// Authentication
pub trait AuthProvider: Send + Sync {
    async fn authenticate(&self, credentials: &Credentials) -> Result<Token>;
    async fn validate(&self, token: &Token) -> Result<UserId>;
    async fn revoke(&self, token: &Token) -> Result<()>;
}

// Capability-based access control
pub struct Capability {
    pub resource: String,
    pub actions: Vec<Action>,  // read, write, delete, admin
}

// Audit
pub trait AuditLogger: Send + Sync {
    async fn log(&self, event: &AuditEvent);
    async fn query(&self, filter: &AuditFilter) -> Vec<AuditEvent>;
}
```

### Validation
```bash
cargo test -p evif-auth
# Coverage: auth providers, capabilities, audit logging
```

---

## Kernel Interactions

```
                    ┌──────────────────┐
                    │   Mount Kernel   │
                    └────────┬─────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
┌───────────────┐  ┌─────────────────┐  ┌─────────────────┐
│ Context Kernel│  │  Skill Kernel   │  │ Memory Kernel   │
└───────────────┘  └─────────────────┘  └─────────────────┘
        │                    │                    │
        └────────────────────┼────────────────────┘
                             │
                             ▼
              ┌──────────────────────────┐
              │   Coordination Kernel    │
              │     (PipeFS + QueueFS)   │
              └──────────────────────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
┌───────────────┐  ┌─────────────────┐  ┌─────────────────┐
│ Connectivity  │  │ Cost Optim.     │  │  Governance     │
│   Kernel      │  │   Kernel        │  │   Kernel        │
└───────────────┘  └─────────────────┘  └─────────────────┘
```

## Release Criteria

Each kernel must pass:

1. **Unit tests**: `cargo test -p {crate} -- {kernel}`
2. **Integration tests**: Cross-kernel operations work correctly
3. **Performance benchmarks**: Latency within SLA
4. **Documentation**: Interfaces documented and examples provided
5. **Breaking changes**: Tracked in CHANGELOG

## Kernel Roadmap

| Kernel | MVP 6.0 | MVP 7.0 | MVP 8.0 |
|--------|---------|---------|---------|
| Mount | ✅ Stable | Streaming | WASM plugins |
| Context | ✅ Stable | L2 indexing | Auto-summarization |
| Skill | ✅ Stable | Execution sandbox | Version control |
| Memory | ✅ Stable | Hybrid search | Multi-modal |
| Coordination | ✅ Stable | Atomic transactions | Workflow DSL |
| Connectivity | ✅ Defined | Auto-discovery | Smart routing |
| Cost Optimize | ✅ Implemented | Adaptive batching | ML-based |
| Governance | ⚠️ Basic | Full audit | Compliance reports |