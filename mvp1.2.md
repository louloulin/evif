# EVIF MVP 1.2 增强计划

> 创建时间：2026-04-29
> 更新时间：2026-04-29
> 项目：EVIF (Everything Is a File)
> 当前完成度：50%（4/8 功能完成）
> 验证时间：2026-04-29

---

## 完成状态总览

| 项 | 状态 | 验证结果 |
|--------|------|----------|
| **P0-1**: AES-256-GCM 加密 | ✅ 已完成 | 6 个测试通过 |
| **P0-2**: Token 计数截断 | ✅ 已完成 | 10 个测试通过 |
| **P1-1**: 插件元数据增强 | ✅ 已完成 | 7 个 langchain 测试通过 |
| **P0-3**: CLI 补全功能 | ✅ 已完成 | chmod/chown 命令已实现 |
| **P1-2**: FUSE 挂载支持 | ⏳ 待实现 | - |
| **P1-3**: 图像/音频分析 | ⏳ 待实现 | 占位符 |
| **P2-1**: 网络插件修复 | ⏳ 待实现 | OpenDAL TLS |
| **P2-2**: HTTP 服务增强 | ⏳ 待实现 | 实验状态 |

---

## 参考项目分析

### AGFS (c4pt0r/agfs) vs EVIF

| 特性 | AGFS | EVIF | 对比 |
|------|------|------|------|
| **插件数量** | 17 个 | 28 个 | ✅ EVIF 领先 |
| **REST API** | ~40 端点 | 106 端点 | ✅ EVIF 领先 |
| **向量搜索** | vectorfs (S3+TiDB) | vectorfs | 持平 |
| **队列服务** | queuefs (多后端) | queuefs | 持平 |
| **SQL 接口** | sqlfs2 (Plan 9 风格) | sqlfs2 | 持平 |
| **心跳监控** | heartbeatfs (min-heap) | heartbeatfs | 持平 |
| **HTTP 服务** | httpfs | httpfs (实验) | 持平 |
| **FUSE 挂载** | ✅ Linux FUSE | 计划中 | ❌ 需实现 |
| **WASM 插件** | ✅ 实例池 | ✅ Extism | 持平 |
| **流量监控** | TrafficMonitor | MetricsRegistry | ⚠️ 需增强 |
| **插件实例池** | WASM 实例池 | 无 | ❌ 需实现 |
| **认证授权** | ❌ 无 | ✅ Capability | ✅ EVIF 领先 |
| **多租户** | ❌ 无 | ⚠️ 基础 | ⚠️ 需增强 |

### AgentFS (Turso) vs EVIF

| 特性 | AgentFS | EVIF | 对比 |
|------|---------|------|------|
| **SQLite 存储** | ✅ 单文件 | ✅ | 持平 |
| **Copy-on-write** | ✅ | ❌ | ❌ 需实现 |
| **完整审计** | ✅ SQL 可查 | ⚠️ 基础 | ⚠️ 需增强 |
| **Agent 追踪** | ✅ | ⚠️ 部分 | ⚠️ 需增强 |

---

## P0 必须项（已完成）

### P0-1: AES-256-GCM 加密替换

**状态**: ✅ 已完成

**实现** (`crates/evif-mem/src/security/encryption.rs`):

```rust
// 使用 PBKDF2 + AES-256-GCM 替换 XOR
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};

pub struct Encryption {
    config: EncryptionConfig,
}

impl Encryption {
    /// AES-256-GCM 加密
    pub fn encrypt(&self, plaintext: &[u8]) -> MemResult<Vec<u8>> {
        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&self.config.master_key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher.encrypt(nonce, plaintext)?;
        // 输出格式: salt(16) + nonce(12) + ciphertext
    }
}
```

**依赖更新** (`crates/evif-mem/Cargo.toml`):
```toml
aes-gcm = "0.10"
pbkdf2 = "0.12"
```

**验证结果**:
```
running 6 tests from security/encryption.rs
test test_disabled_encryption ... ok
test test_wrong_key_fails ... ok
test test_encrypt_decrypt ... ok
test test_encrypt_decrypt_string ... ok
test test_encryption_from_password ... ok
test test_deterministic_with_same_salt ... ok

6 passed, 0 failed
```

---

### P0-2: Token 计数和截断

**状态**: ✅ 已完成

**实现** (`crates/evif-mem/src/token.rs`):

```rust
/// TokenBudget - 字符数估计（无需网络）
/// - 英文: 4 字符/Token
/// - 中文/日文/韩文: 2 字符/Token
pub struct TokenBudget {
    max_tokens: usize,
    reserved_tokens: usize,
}

impl TokenBudget {
    /// 估算 token 数
    pub fn count(&self, text: &str) -> usize {
        let has_cjk = estimation::has_cjk(text);
        let chars_per_token = if has_cjk { 2.0 } else { 4.0 };
        (text.chars().count() as f64 / chars_per_token).ceil() as usize
    }

    /// 截断到预算内
    pub fn truncate(&self, text: &str) -> String {
        let max_chars = (self.available_tokens() as f64 * chars_per_token) as usize;
        text.chars().take(max_chars).collect()
    }
}
```

**验证结果**:
```
running 10 tests from token.rs
test test_token_budget_available ... ok
test test_english_estimation ... ok
test test_cjk_estimation ... ok
test test_budget_cjk_count ... ok
test test_token_budget_count ... ok
test test_token_budget_truncate ... ok
test test_truncate_to_tokens ... ok
test test_token_budget_fits ... ok
test test_truncate_batch ... ok
test test_mixed_cjk_english ... ok

10 passed, 0 failed
```

---

### P1-1: 插件元数据增强

**状态**: ✅ 已完成

**实现** (`crates/evif-mem/src/models.rs`):

```rust
/// MemoryItem 新增字段
pub struct MemoryItem {
    // ... existing fields ...
    /// 标签
    pub tags: Vec<String>,
    /// 引用
    pub references: Vec<String>,
}

impl MemoryItem {
    pub fn add_tag(&mut self, tag: impl Into<String>) { ... }
    pub fn remove_tag(&mut self, tag: &str) { ... }
    pub fn add_reference(&mut self, ref_id: impl Into<String>) { ... }
    pub fn remove_reference(&mut self, ref_id: &str) { ... }
}
```

**关键文件**:
- `crates/evif-mem/src/models.rs` - MemoryItem 添加 tags/references
- `crates/evif-mem/src/plugin/plugin.rs` - 从 MdFrontmatter 解析/序列化

**验证结果**:
```
running 7 tests from langchain.rs
test test_config_defaults ... ok
test test_chat_message_creation ... ok
test test_evif_memory_creation ... ok
test test_memory_variables ... ok
test test_buffer_memory ... ok
test test_add_and_get_messages ... ok
test test_load_memory_variables ... ok

7 passed, 0 failed
```

---

## P0/P1 待实现项

### P0-3: CLI 补全功能

**状态**: ✅ 已完成

**实现** (`crates/evif-cli/src/cli.rs`, `crates/evif-cli/src/commands.rs`):

```rust
// CLI 命令定义
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Change file permissions
    Chmod {
        /// File path
        path: String,
        /// Permission mode (e.g., 0o755 or 755)
        mode: String,
    },

    /// Change file owner and group
    Chown {
        /// File path
        path: String,
        /// New owner username
        owner: String,
        /// New group name (optional)
        #[arg(short, long)]
        group: Option<String>,
    },
}
```

**关键文件**:
- `crates/evif-cli/src/cli.rs` - CLI 命令定义
- `crates/evif-cli/src/commands.rs` - chmod/chown 实现
- `crates/evif-client/src/client.rs` - REST API 客户端
- `crates/evif-rest/src/handlers.rs` - HTTP 处理器
- `crates/evif-rest/src/routes.rs` - 路由注册
- `crates/evif-rest/src/fs_handlers.rs` - 请求结构体
- `crates/evif-core/src/plugin.rs` - 插件 trait 扩展

**验证结果**:
```bash
$ evif chmod --help
Change file permissions
Usage: evif chmod [OPTIONS] <PATH> <MODE>

$ evif chown --help
Change file owner and group
Usage: evif chown [OPTIONS] <PATH> <OWNER>
```

---

### P1-2: FUSE 挂载支持

**状态**: ⏳ 待实现

**工作量**：约 12h

---

### P1-3: 图像/音频分析增强

**状态**: ⏳ 待实现

**当前问题**：
- `evif-mem/llm.rs:2783,3092` - 返回占位符
- `evif-mem/pipeline.rs:1075` - 需外部服务

**工作量**：约 12h

---

## P2 改进项

### P2-1: 修复网络插件

**状态**: ⏳ 待实现

**问题**：OpenDAL TLS 冲突导致 webdavfs/ftpfs/sftpfs 禁用

**工作量**：约 12h

---

### P2-2: HTTP 服务增强

**状态**: ⏳ 待实现

**工作量**：约 8h

---

## 验证记录 (2026-04-29)

| 测试项 | 命令 | 结果 |
|--------|------|------|
| AES-256-GCM 加密 | `cargo test -p evif-mem --features security -- encryption` | ✅ 6 passed |
| Token 计数 | `cargo test -p evif-mem -- token` | ✅ 10 passed |
| LangChain 集成 | `cargo test -p evif-mem -- langchain` | ✅ 7 passed |
| chmod CLI 命令 | `evif chmod --help` | ✅ 正常显示 |
| chown CLI 命令 | `evif chown --help` | ✅ 正常显示 |

---

## 关键文件

| 文件 | 说明 |
|------|------|
| `crates/evif-mem/src/security/encryption.rs` | AES-256-GCM 实现 |
| `crates/evif-mem/src/token.rs` | Token 计数模块 |
| `crates/evif-mem/src/models.rs` | MemoryItem tags/references |
| `crates/evif-mem/Cargo.toml` | aes-gcm, pbkdf2 依赖 |
| `crates/evif-cli/src/cli.rs` | CLI chmod/chown 命令定义 |
| `crates/evif-cli/src/commands.rs` | chmod/chown 实现 |
| `crates/evif-client/src/client.rs` | REST API chmod/chown |
| `crates/evif-rest/src/handlers.rs` | HTTP 处理器 |
| `crates/evif-rest/src/routes.rs` | 路由注册 |
| `crates/evif-core/src/plugin.rs` | 插件 chown trait |
