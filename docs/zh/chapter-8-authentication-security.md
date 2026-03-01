# 第八章：认证与安全 (Chapter 8: Authentication & Security)

## 目录 (Table of Contents)

1. [安全架构概述 (Security Architecture Overview)](#安全架构概述-security-architecture-overview)
2. [认证机制 (Authentication Mechanisms)](#认证机制-authentication-mechanisms)
3. [授权模型 (Authorization Model)](#授权模型-authorization-model)
4. [审计日志系统 (Audit Logging System)](#审计日志系统-audit-logging-system)
5. [静态加密 (Encryption at Rest)](#静态加密-encryption-at-rest)
6. [安全最佳实践 (Security Best Practices)](#安全最佳实践-security-best-practices)
7. [威胁模型 (Threat Models)](#威胁模型-threat-models)
8. [安全配置指南 (Security Configuration Guide)](#安全配置指南-security-configuration-guide)

---

## 安全架构概述 (Security Architecture Overview)

### 设计原则 (Design Principles)

EVIF 采用了**纵深防御 (Defense in Depth)** 的安全架构，在多个层次提供保护：

```
┌─────────────────────────────────────────────────────────────┐
│                   EVIF Security Stack                       │
├─────────────────────────────────────────────────────────────┤
│  Network Layer:  TLS 1.3, API Key Authentication           │
├─────────────────────────────────────────────────────────────┤
│  Application Layer:  Capability-based Authorization         │
│                       (Principal → Capability → Permission)  │
├─────────────────────────────────────────────────────────────┤
│  Data Layer:  AES-256-GCM Encryption at Rest               │
│               Argon2id Key Derivation                       │
├─────────────────────────────────────────────────────────────┤
│  Audit Layer:  Comprehensive Logging (Memory/File)         │
│                Tamper-Evident Audit Trail                  │
└─────────────────────────────────────────────────────────────┘
```

**核心设计理念 (Core Design Philosophy):**

1. **最小权限原则 (Principle of Least Privilege)**: 每个主体只拥有完成任务所需的最小权限
2. **能力安全 (Capability-Based Security)**: 访问权限通过不可伪造的能力对象传递
3. **默认拒绝 (Default Deny)**: Strict 模式下，未明确授权的操作默认拒绝
4. **审计可追溯 (Audit Trail)**: 所有安全相关操作均记录可审计日志
5. **透明加密 (Transparent Encryption)**: 存储层自动加密，对上层应用透明

### 安全组件概览 (Security Components Overview)

| 组件 (Component) | 职责 (Responsibility) | 位置 (Location) |
|------------------|----------------------|-----------------|
| **AuthManager** | 认证管理器，处理权限检查和授权 | `crates/evif-auth/src/auth.rs` |
| **Capability** | 能力对象，封装访问权限 | `crates/evif-auth/src/capability.rs` |
| **Principal** | 主体标识（用户/服务/系统） | `crates/evif-auth/src/capability.rs` |
| **AuditLogger** | 审计日志接口和实现 | `crates/evif-auth/src/audit.rs` |
| **EncryptedFS** | 静态加密插件 | `crates/evif-plugins/src/encryptedfs.rs` |

---

## 认证机制 (Authentication Mechanisms)

### 主体类型 (Principal Types)

EVIF 使用 **Principal**（主体）来标识操作者：

```rust
pub enum Principal {
    User(UUID),      // 用户主体
    Service(UUID),   // 服务主体
    System,          // 系统主体（超级用户）
}
```

**使用场景 (Usage Scenarios):**

| 主体类型 (Type) | 使用场景 (Use Case) | 权限级别 (Permission Level) |
|-----------------|---------------------|----------------------------|
| `User` | 终端用户、API 客户端 | 基于能力授予的权限 |
| `Service` | 后台服务、自动化任务 | 基于能力授予的权限 |
| `System` | 系统内部操作 | 超级用户，绕过权限检查 |

**代码示例 (Code Example):**

```rust
use evif_auth::{Principal, AuthManager};
use uuid::Uuid;

// 创建用户主体
let user_id = Uuid::new_v4();
let user_principal = Principal::User(user_id);

// 创建服务主体
let service_id = Uuid::new_v4();
let service_principal = Principal::Service(service_id);

// 系统主体（无需 ID）
let system_principal = Principal::System;

// 系统主体总是通过权限检查
let auth = AuthManager::new();
let result = auth.check(&system_principal, &node_id, Permission::Write)?;
assert!(result); // 总是返回 true
```

### 认证策略 (Authentication Policies)

EVIF 支持两种认证策略：

#### 1. 开放策略 (Open Policy)

**特点 (Characteristics):**
- 允许所有操作，不检查权限
- 适用于开发环境、本地测试
- 仍然记录审计日志

```rust
use evif_auth::{AuthManager, AuthPolicy};

// 创建开放策略的认证管理器
let auth = AuthManager::with_policy(AuthPolicy::Open);

// 任何主体都可以访问任何资源
let user = Principal::User(Uuid::new_v4());
let resource_id = Uuid::new_v4();
let can_access = auth.check(&user, &resource_id, Permission::Write)?;
assert!(can_access); // 总是 true
```

#### 2. 严格策略 (Strict Policy)

**特点 (Characteristics):**
- 默认拒绝所有未明确授权的操作
- 需要通过 Capability 授予权限
- 适用于生产环境

```rust
use evif_auth::{AuthManager, AuthPolicy, Capability, Permissions};

// 创建严格策略的认证管理器
let auth = AuthManager::with_policy(AuthPolicy::Strict);

let user = Principal::User(Uuid::new_v4());
let resource_id = Uuid::new_v4();

// 未授权前无法访问
let can_access = auth.check(&user, &resource_id, Permission::Read)?;
assert!(!can_access); // false

// 授予权限后可以访问
let cap = Capability::new(
    user.get_id().unwrap(),
    resource_id,
    Permissions::read()
);
auth.grant(cap)?;

let can_access = auth.check(&user, &resource_id, Permission::Read)?;
assert!(can_access); // true
```

**策略选择指南 (Policy Selection Guide):**

| 环境 (Environment) | 推荐策略 (Recommended Policy) | 原因 (Reasoning) |
|--------------------|------------------------------|------------------|
| 本地开发 (Local Dev) | `Open` | 快速迭代，无需权限管理 |
| 内部测试 (Internal Testing) | `Strict` | 模拟生产环境，发现权限问题 |
| 生产环境 (Production) | `Strict` | 最小权限，最大化安全性 |

---

## 授权模型 (Authorization Model)

### 能力安全模型 (Capability-Based Security Model)

EVIF 采用**能力安全模型**，而非传统的 RBAC（基于角色的访问控制）。

**核心概念 (Core Concepts):**

```
┌──────────────┐      授予 (grant)      ┌──────────────┐
│  Principal   │ ────────────────────> │  Capability  │
│  (主体)      │                        │  (能力)      │
└──────────────┘                        └──────────────┘
                                             │
                                             │ 持有 (holds)
                                             │
                                             ▼
                                      ┌──────────────┐
                                      │  Permissions │
                                      │  - read      │
                                      │  - write     │
                                      │  - execute   │
                                      │  - admin     │
                                      └──────────────┘
```

**能力对象 (Capability Object):**

```rust
pub struct Capability {
    pub id: CapId,                    // 能力唯一 ID
    pub holder: PrincipalId,          // 持有者 ID
    pub node: Uuid,                   // 目标资源 ID
    pub permissions: Permissions,     // 权限集合
    pub expires: Option<DateTime<Utc>>, // 过期时间（可选）
}
```

**特性 (Features):**

1. **不可伪造 (Unforgeable)**: Capability ID 通过 UUID 生成，无法预测
2. **可传递 (Transferable)**: 可以通过安全通道传递给其他主体
3. **可过期 (Expirable)**: 支持设置过期时间，实现临时权限
4. **可撤销 (Revocable)**: 通过 ID 可以随时撤销权限

### 权限类型 (Permission Types)

EVIF 定义了四种基本权限：

```rust
pub enum Permission {
    Read,    // 读取权限
    Write,   // 写入权限
    Execute, // 执行权限
    Admin,   // 管理员权限
}

pub struct Permissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub admin: bool,
}
```

**权限组合示例 (Permission Combinations):**

```rust
use evif_auth::{Permissions};

// 只读权限
let read_only = Permissions::read();
// { read: true, write: false, execute: false, admin: false }

// 读写权限
let read_write = Permissions::read_write();
// { read: true, write: true, execute: false, admin: false }

// 完全权限
let full = Permissions::all();
// { read: true, write: true, execute: true, admin: true }

// 自定义权限
let custom = Permissions {
    read: true,
    write: false,
    execute: true,
    admin: false,
};
```

**权限与操作映射 (Permission to Operation Mapping):**

| 操作 (Operation) | 所需权限 (Required Permission) | 说明 (Description) |
|------------------|-------------------------------|---------------------|
| 读取文件内容 | `Read` | 打开文件并读取数据 |
| 列出目录 | `Read` | 列出目录下的子节点 |
| 写入文件 | `Write` | 修改文件内容 |
| 创建文件/目录 | `Write` | 在父目录下创建新节点 |
| 删除文件/目录 | `Write` | 删除节点 |
| 重命名/移动 | `Write` | 修改节点路径 |
| 执行文件 | `Execute` | 执行可执行文件 |
| 修改权限 | `Admin` | 授予/撤销其他主体的权限 |
| 更改所有权 | `Admin` | 修改资源的所有者 |

### 完整授权流程 (Complete Authorization Flow)

```rust
use evif_auth::{
    AuthManager, AuthPolicy, Capability, Permissions,
    Principal, Permission, AuditLogManager
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建认证管理器（严格策略 + 审计日志）
    let audit_log = AuditLogManager::from_file("evif_audit.log")?;
    let auth = AuthManager::with_policy(AuthPolicy::Strict)
        .with_audit_log(audit_log);

    // 2. 创建主体和资源
    let alice = Principal::User(Uuid::new_v4());
    let file_node = Uuid::new_v4();

    // 3. 创建能力对象
    let cap = Capability::new(
        alice.get_id().unwrap(),
        file_node,
        Permissions::read_write(),
    );

    // 4. 授予权限（记录审计日志）
    let cap_id = auth.grant(cap).await?;
    println!("Capability granted: {}", cap_id);

    // 5. 检查权限（自动记录审计日志）
    let can_read = auth.check(&alice, &file_node, Permission::Read).await?;
    println!("Alice can read: {}", can_read); // true

    let can_write = auth.check(&alice, &file_node, Permission::Write).await?;
    println!("Alice can write: {}", can_write); // true

    let can_execute = auth.check(&alice, &file_node, Permission::Execute).await?;
    println!("Alice can execute: {}", can_execute); // false

    // 6. 撤销权限（记录审计日志）
    auth.revoke(&cap_id).await?;
    println!("Capability revoked");

    // 7. 撤销后无法访问
    let can_read_after = auth.check(&alice, &file_node, Permission::Read).await?;
    println!("Alice can read after revoke: {}", can_read_after); // false

    Ok(())
}
```

### 临时权限管理 (Temporary Permissions)

使用过期时间实现临时访问权限：

```rust
use evif_auth::{Capability, Permissions};
use chrono::{Utc, Duration};
use uuid::Uuid;

let holder = Uuid::new_v4();
let resource = Uuid::new_v4();

// 创建 1 小时后过期的临时能力
let temp_cap = Capability::new(holder, resource, Permissions::read())
    .with_expiry(Utc::now() + Duration::hours(1));

auth.grant(temp_cap).await?;

// 立即检查 - 有效
let now = auth.check(&principal, &resource, Permission::Read).await?;
assert!(now); // true

// 等待过期后检查 - 失效
tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
let expired = auth.check(&principal, &resource, Permission::Read).await?;
assert!(!expired); // false
```

**使用场景 (Use Cases):**
- **临时共享**: 给第三方临时访问权限
- **定时任务**: 只在特定时间窗口内运行的服务
- **会话管理**: 用户登录会话的自动过期

---

## 审计日志系统 (Audit Logging System)

### 审计事件类型 (Audit Event Types)

EVIF 记录所有安全相关操作，支持 9 种审计事件类型：

```rust
pub enum AuditEventType {
    CapabilityGranted,      // 能力授予
    CapabilityRevoked,      // 能力撤销
    AccessGranted,          // 访问授权
    AccessDenied,           // 访问拒绝
    PolicyChanged,          // 策略变更
    AuthenticationFailed,   // 认证失败
    SessionCreated,         // 会话创建
    SessionTerminated,      // 会话终止
}
```

### 审计事件结构 (Audit Event Structure)

```rust
pub struct AuditEvent {
    pub id: Uuid,                    // 事件唯一 ID
    pub event_type: AuditEventType,  // 事件类型
    pub timestamp: DateTime<Utc>,    // 时间戳（UTC）
    pub principal_id: Option<Uuid>,  // 主体 ID（谁执行的操作）
    pub resource_id: Option<Uuid>,   // 资源 ID（操作的目标）
    pub success: bool,               // 操作结果
    pub details: String,             // 事件详情
    pub ip_address: Option<String>,  // IP 地址（可选）
    pub user_agent: Option<String>,  // 用户代理（可选）
}
```

### 审计日志实现 (Audit Logger Implementations)

#### 1. 内存审计日志器 (Memory Audit Logger)

**特点 (Features):**
- 高性能，无 I/O 开销
- 最多保存 10,000 条事件
- 适用于开发和测试环境

```rust
use evif_auth::{MemoryAuditLogger, AuditLogManager, AuditEvent, AuditEventType};

// 创建内存审计日志器
let logger = MemoryAuditLogger::new();
let audit = AuditLogManager::new(logger);

// 记录事件自动添加到内存
let principal_id = Uuid::new_v4();
let resource_id = Uuid::new_v4();
audit.log_capability_granted(principal_id, resource_id)?;

// 查询事件
let events = audit.query(AuditFilter::new())?;
println!("Total events: {}", events.len());
```

#### 2. 文件审计日志器 (File Audit Logger)

**特点 (Features):**
- 持久化存储，重启不丢失
- 支持 Log Rotation（10MB 默认）
- 可选同步写入模式

```rust
use evif_auth::{FileAuditLogger, AuditLogManager, AuditConfig};
use std::path::Path;

// 创建配置
let config = AuditConfig {
    enabled: true,
    log_path: Some("/var/log/evif/audit.log".to_string()),
    rotation_size: 10 * 1024 * 1024, // 10MB
    sync_write: true, // 同步写入，确保持久化
};

// 创建文件审计日志器
let audit = AuditLogManager::from_file("/var/log/evif/audit.log")?;

// 记录事件（同时写入内存和文件）
audit.log_access_granted(principal_id, resource_id, "read")?;
```

**日志格式 (Log Format):**

```
2025-03-01 12:34:56.789 UTC | AccessGranted | principal=550e8400-e29b-41d4-a716-446655440000 | resource=6ba7b810-9dad-11d1-80b4-00c04fd430c8 | success=true | Access granted: read permission for principal 550e8400-e29b-41d4-a716-446655440000 on resource 6ba7b810-9dad-11d1-80b4-00c04fd430c8
```

### 审计日志查询 (Audit Log Querying)

使用 `AuditFilter` 查询特定条件的审计事件：

```rust
use evif_auth::{AuditFilter, AuditEventType};
use chrono::{Utc, Duration};

// 查询最近 1 小时所有访问拒绝事件
let one_hour_ago = Utc::now() - Duration::hours(1);

let filter = AuditFilter::new()
    .with_event_type(AuditEventType::AccessDenied)
    .with_start_time(one_hour_ago)
    .with_success_only(false);

let denied_events = audit.query(filter)?;

for event in denied_events {
    println!("Denied access: {} -> {}", event.principal_id.unwrap(), event.resource_id.unwrap());
}
```

**查询条件 (Query Conditions):**

| 方法 (Method) | 参数类型 (Parameter) | 说明 (Description) |
|---------------|---------------------|---------------------|
| `with_event_type()` | `AuditEventType` | 按事件类型过滤 |
| `with_principal_id()` | `Uuid` | 按主体 ID 过滤 |
| `with_resource_id()` | `Uuid` | 按资源 ID 过滤 |
| `with_start_time()` | `DateTime<Utc>` | 时间范围开始 |
| `with_end_time()` | `DateTime<Utc>` | 时间范围结束 |
| `with_success_only()` | `bool` | 成功/失败过滤 |

### 审计日志清理 (Audit Log Pruning)

定期清理旧的审计事件：

```rust
use chrono::{Utc, Duration};

// 删除 30 天前的事件
let cutoff = Utc::now() - Duration::days(30);
let deleted_count = audit.prune(cutoff)?;

println!("Deleted {} old audit events", deleted_count);
```

### 审计日志最佳实践 (Audit Logging Best Practices)

**生产环境配置 (Production Configuration):**

```rust
use evif_auth::{AuditConfig, FileAuditLogger};

let config = AuditConfig {
    enabled: true,
    log_path: Some("/var/log/evif/audit.log".to_string()),
    rotation_size: 100 * 1024 * 1024, // 100MB
    sync_write: true, // 确保持久化
};

// 配置日志轮转（使用外部工具如 logrotate）
// - /etc/logrotate.d/evif:
//   /var/log/evif/audit.log {
//       daily
//       rotate 30
//       compress
//       delaycompress
//       missingok
//       notifempty
//   }
```

**安全建议 (Security Recommendations):**

1. **写保护**: 审计日志文件应设置为只追加模式（`chmod a-w`）
2. **独立存储**: 将审计日志存储在独立的、安全的文件系统
3. **实时监控**: 使用工具如 `tail -f` 或 SIEM 系统实时监控审计日志
4. **定期备份**: 将审计日志备份到不可变的存储（如 WORM 存储）
5. **访问控制**: 只有管理员才能访问审计日志文件

---

## 静态加密 (Encryption at Rest)

### EncryptedFS 插件 (EncryptedFS Plugin)

EVIF 提供 **EncryptedFS** 插件，实现透明的静态数据加密。

**加密算法 (Encryption Algorithm):**

```
Master Password (用户提供的密码)
        │
        ▼
Argon2id Key Derivation (密钥派生)
        │
        ▼
256-bit Encryption Key (加密密钥)
        │
        ▼
AES-256-GCM (认证加密)
        │
        ├─> Encrypted Data (加密数据)
        └─> Authentication Tag (认证标签)
```

**技术规格 (Technical Specifications):**

| 组件 (Component) | 算法/参数 (Algorithm/Parameter) | 说明 (Description) |
|------------------|-------------------------------|---------------------|
| **加密算法** | AES-256-GCM | 认证加密，提供机密性和完整性 |
| **密钥派生** | Argon2id | 抗 GPU/ASIC 攻击的密钥派生 |
| **Nonce 大小** | 96 bits (12 bytes) | 每个文件唯一，防止重放攻击 |
| **认证标签** | 128 bits (16 bytes) | 检测数据篡改 |
| **内存成本** | 64 MB (默认) | Argon2id 内存参数 |
| **迭代次数** | 3 (默认) | Argon2id 时间参数 |
| **并行度** | 4 (默认) | Argon2id 并行参数 |

### EncryptedFS 配置 (EncryptedFS Configuration)

```rust
use evif_plugins::encryptedfs::{EncryptedFsPlugin, EncryptedConfig};
use std::sync::Arc;

let config = EncryptedConfig {
    master_password: "your-secure-password-here".to_string(),
    argon2_memory_kb: 65536,    // 64 MB
    argon2_iterations: 3,
    argon2_parallelism: 4,
};

let encrypted_fs = EncryptedFsPlugin::new(backend, config)?;
```

### EncryptedFS 使用示例 (EncryptedFS Usage Example)

```rust
use evif_core::EvifPlugin;
use evif_plugins::encryptedfs::{EncryptedFsPlugin, EncryptedConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建后端存储（如 MemoryFs）
    let backend = Arc::new(MemoryFsPlugin::new());

    // 创建 EncryptedFS 包装器
    let config = EncryptedConfig {
        master_password: "secure-password-123".to_string(),
        ..Default::default()
    };

    let encrypted_fs = Arc::new(EncryptedFsPlugin::new(backend, config)?);

    // 写入数据（自动加密）
    encrypted_fs.write(
        "/secret.txt",
        b"sensitive data".to_vec(),
        WriteFlags::default()
    ).await?;

    // 读取数据（自动解密）
    let data = encrypted_fs.read("/secret.txt").await?;
    assert_eq!(data, b"sensitive data");

    // 底层存储的是加密数据
    let encrypted_data = backend.read("/secret.txt").await?;
    assert_ne!(encrypted_data, b"sensitive data");

    Ok(())
}
```

### 加密文件格式 (Encrypted File Format)

每个加密文件包含元数据头部和加密数据：

```
┌────────────────────────────────────────────────────────┐
│  Header Length (4 bytes, big-endian)                   │
├────────────────────────────────────────────────────────┤
│  JSON Header (variable length)                         │
│  {                                                      │
│    "version": 1,                                       │
│    "nonce": "base64-encoded-nonce",                    │
│    "tag": "base64-encoded-auth-tag"                    │
│  }                                                      │
├────────────────────────────────────────────────────────┤
│  Encrypted Data (AES-256-GCM ciphertext)               │
└────────────────────────────────────────────────────────┘
```

**元数据版本控制 (Metadata Versioning):**

```rust
pub enum EncryptionVersion {
    V1 = 1,  // AES-256-GCM + Argon2id
    // 未来可以添加新版本
    // V2 = 2,  // ChaCha20-Poly1305 + Scrypt
}
```

### 密码安全管理 (Password Security Management)

**推荐做法 (Best Practices):**

1. **不要硬编码密码**: 使用环境变量或密钥管理服务
2. **使用强密码**: 至少 32 个字符，包含大小写字母、数字、符号
3. **定期轮换**: 每季度更换主密码（需要重新加密数据）
4. **安全存储**: 使用 AWS Secrets Manager、HashiCorp Vault 等

```rust
use std::env;

// 从环境变量读取密码
let master_password = env::var("EVIF_ENCRYPTION_PASSWORD")
    .expect("EVIF_ENCRYPTION_PASSWORD must be set");

let config = EncryptedConfig {
    master_password,
    ..Default::default()
};
```

### 加密性能考虑 (Encryption Performance Considerations)

**性能影响 (Performance Impact):**

| 操作 (Operation) | 性能影响 (Performance Overhead) | 说明 (Notes) |
|------------------|-------------------------------|--------------|
| 读取 (Read) | ~5-10% | 需要解密 + 验证认证标签 |
| 写入 (Write) | ~10-15% | 需要生成 nonce + 加密 |
| 随机访问 | 不适用 | 必须读取整个文件 |
| 并发访问 | 良好 | 每个文件独立加密 |

**优化建议 (Optimization Tips):**

1. **使用缓存**: EncryptedFS 内置缓存，减少重复解密
2. **调整 Argon2 参数**: 降低内存/迭代次数以提升性能（牺牲安全性）
3. **批量操作**: 尽量批量读写，减少加密/解密次数

```rust
// 性能优化配置（低安全性）
let fast_config = EncryptedConfig {
    master_password: "password".to_string(),
    argon2_memory_kb: 16384,    // 16 MB (vs 64 MB)
    argon2_iterations: 1,        // 1 (vs 3)
    argon2_parallelism: 2,       // 2 (vs 4)
};
```

---

## 安全最佳实践 (Security Best Practices)

### 部署安全 (Deployment Security)

#### 1. 网络层安全 (Network Layer Security)

**使用 TLS 1.3:**

```nginx
# nginx 反向代理配置
server {
    listen 443 ssl http2;
    server_name evif.example.com;

    ssl_certificate /etc/ssl/certs/evif.crt;
    ssl_certificate_key /etc/ssl/private/evif.key;
    ssl_protocols TLSv1.3;
    ssl_ciphers 'TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256';
    ssl_prefer_server_ciphers off;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

**防火墙规则 (Firewall Rules):**

```bash
# 仅允许特定 IP 访问 EVIF API
iptables -A INPUT -p tcp --dport 8080 -s 10.0.0.0/8 -j ACCEPT
iptables -A INPUT -p tcp --dport 8080 -j DROP
```

#### 2. 访问控制 (Access Control)

**API Key 管理:**

```bash
# 生成强随机 API Key
openssl rand -hex 32

# 存储在安全的环境变量中
export EVIF_API_KEY="7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8"
```

**定期轮换 API Keys:**

```bash
# 每 90 天轮换 API Key
0 0 1 */3 * root /usr/local/bin/rotate-evif-api-key.sh
```

#### 3. 隔离部署 (Isolated Deployment)

**使用 Docker 容器:**

```dockerfile
FROM ubuntu:22.04

# 创建非 root 用户
RUN useradd -m -u 1000 evif

# 复制二进制文件
COPY evif-server /usr/local/bin/
COPY evif-fuse /usr/local/bin/

# 设置权限
RUN chmod +x /usr/local/bin/evif-* && \
    chown evif:evif /usr/local/bin/evif-*

# 切换到非 root 用户
USER evif

# 暴露端口
EXPOSE 8080

# 健康检查
HEALTHCHECK --interval=30s --timeout=3s \
  CMD curl -f http://localhost:8080/health || exit 1

CMD ["evif-server"]
```

**安全 Docker Compose 配置:**

```yaml
version: '3.8'
services:
  evif:
    image: evif:latest
    container_name: evif-server
    restart: unless-stopped
    environment:
      - EVIF_AUTH_POLICY=Strict
      - EVIF_API_KEY=${EVIF_API_KEY}
      - EVIF_AUDIT_LOG_PATH=/var/log/evif/audit.log
    volumes:
      - evif-data:/data
      - evif-logs:/var/log/evif
    networks:
      - internal
    ports:
      - "127.0.0.1:8080:8080"  # 仅本地访问
    security_opt:
      - no-new-privileges:true
    read_only: true  # 只读文件系统
    tmpfs:
      - /tmp:rw,noexec,nosuid,size=100m

networks:
  internal:
    driver: bridge

volumes:
  evif-data:
  evif-logs:
```

### 应用层安全 (Application Layer Security)

#### 1. 输入验证 (Input Validation)

```rust
use validator::Validate;

#[derive(Validate, Deserialize)]
struct CreateUserRequest {
    #[validate(length(min = 1, max = 100))]
    username: String,

    #[validate(email)]
    email: String,

    #[validate(length(min = 12, max = 128))]
    password: String,
}

fn create_user(req: CreateUserRequest) -> Result<()> {
    req.validate()?;
    // 处理用户创建...
}
```

#### 2. 输出编码 (Output Encoding)

```rust
use ammonia::clean;

// 对用户输入进行 HTML 转义，防止 XSS
let safe_html = clean(user_input);
```

#### 3. SQL 注入防护 (SQL Injection Protection)

EVIF 使用参数化查询，天然防御 SQL 注入：

```rust
// 安全（使用参数化查询）
client.query(
    "SELECT * FROM files WHERE owner_id = $1",
    &[&user_id]
).await?;

// 不安全（不要这样做！）
client.query(
    &format!("SELECT * FROM files WHERE owner_id = '{}', user_id),
    &[]
).await?;
```

### 数据层安全 (Data Layer Security)

#### 1. 加密敏感数据 (Encrypt Sensitive Data)

```rust
use evif_plugins::encryptedfs::EncryptedFsPlugin;

// 对包含敏感信息的文件使用 EncryptedFS
let secure_backend = EncryptedFsPlugin::new(backend, encryption_config)?;

// 普通文件使用普通后端
let normal_backend = MemoryFsPlugin::new();
```

#### 2. 安全备份 (Secure Backup)

```bash
#!/bin/bash
# secure-backup.sh

# 1. 停止 EVIF 服务
systemctl stop evif

# 2. 备份数据目录
tar -czf /backup/evif-data-$(date +%Y%m%d).tar.gz /var/lib/evif

# 3. 使用 GPG 加密备份
gpg --symmetric --cipher-algo AES256 \
    /backup/evif-data-$(date +%Y%m%d).tar.gz

# 4. 上传到异地存储
aws s3 cp /backup/evif-data-$(date +%Y%m%d).tar.gz.gpg \
    s3://secure-backup-bucket/

# 5. 删除本地未加密备份
shred -u /backup/evif-data-$(date +%Y%m%d).tar.gz

# 6. 重启 EVIF 服务
systemctl start evif
```

### 运维安全 (Operational Security)

#### 1. 最小权限原则 (Principle of Least Privilege)

```bash
# EVIF 服务运行在专用账户下
useradd -r -s /bin/false evif

# 数据目录权限
chown -R evif:evif /var/lib/evif
chmod 700 /var/lib/evif

# 日志目录权限
chown -R evif:adm /var/log/evif
chmod 750 /var/log/evif

# 审计日志权限
chmod 600 /var/log/evif/audit.log
chattr +a /var/log/evif/audit.log  # 仅追加模式
```

#### 2. 安全更新 (Security Updates)

```bash
#!/bin/bash
# security-update.sh

# 每周检查安全更新
0 3 * * 0 root /usr/local/bin/security-update.sh

# 更新系统包
apt-get update
apt-get upgrade -y

# 检查 EVIF 更新
cargo install evif-cli --force

# 重启服务
systemctl restart evif
```

#### 3. 安全监控 (Security Monitoring)

```bash
# 监控审计日志中的可疑活动
tail -f /var/log/evif/audit.log | \
    grep -i "AccessDenied\|AuthenticationFailed" | \
    while read line; do
        # 发送告警
        echo "$line" | mail -s "EVIF Security Alert" security@example.com
    done
```

---

## 威胁模型 (Threat Models)

### 常见威胁与缓解措施 (Common Threats and Mitigations)

#### 1. 未授权访问 (Unauthorized Access)

**威胁描述 (Threat Description):**
攻击者试图访问其无权限的资源。

**缓解措施 (Mitigations):**

| 措施 (Mitigation) | 实现 (Implementation) |
|------------------|----------------------|
| 强认证策略 | 使用 `AuthPolicy::Strict` |
| 能力验证 | 所有操作前调用 `auth.check()` |
| API Key 验证 | REST API 要求有效的 `X-API-Key` |
| 审计日志 | 记录所有访问拒绝事件 |

**代码示例 (Code Example):**

```rust
use evif_auth::{AuthManager, AuthPolicy};

// 生产环境必须使用 Strict 策略
let auth = AuthManager::with_policy(AuthPolicy::Strict);

// 所有操作前验证权限
let can_access = auth.check(&principal, &resource, Permission::Read)?;
if !can_access {
    return Err(AuthError::Forbidden("Insufficient permissions".to_string()));
}

// 继续执行操作...
```

#### 2. 权限提升 (Privilege Escalation)

**威胁描述 (Threat Description):**
低权限用户试图获取管理员权限。

**缓解措施 (Mitigations):**

| 措施 (Mitigation) | 实现 (Implementation) |
|------------------|----------------------|
| Admin 权限隔离 | 只有明确授予 Admin 权限才能执行管理操作 |
| 能力不可伪造 | Capability ID 使用 UUID，无法猜测 |
| 撤销机制 | 立即撤销可疑能力 |
| 审计日志 | 记录所有权限变更操作 |

**代码示例 (Code Example):**

```rust
// 管理操作需要显式 Admin 权限
fn grant_capability(
    auth: &AuthManager,
    operator: &Principal,
    target: &Principal,
    resource: Uuid,
    permissions: Permissions,
) -> AuthResult<()> {
    // 1. 验证操作者是否有 Admin 权限
    let has_admin = auth.check(operator, &resource, Permission::Admin)?;
    if !has_admin {
        audit.log_access_denied(
            operator.get_id()?,
            resource,
            "Admin",
            "operator lacks Admin permission"
        )?;
        return Err(AuthError::Forbidden("Admin permission required".to_string()));
    }

    // 2. 授予权限
    let cap = Capability::new(target.get_id()?, resource, permissions);
    auth.grant(cap)?;

    // 3. 记录审计日志
    audit.log_capability_granted(target.get_id()?, resource)?;

    Ok(())
}
```

#### 3. 重放攻击 (Replay Attack)

**威胁描述 (Threat Description):**
攻击者截获并重放有效的请求。

**缓解措施 (Mitigations):**

| 措施 (Mitigation) | 实现 (Implementation) |
|------------------|----------------------|
| 请求时间戳 | REST API 验证请求时间戳（5 分钟窗口） |
| Nonce 重放防护 | EncryptedFS 使用唯一的每文件 nonce |
| TLS 保护 | 网络层使用 TLS 1.3 防止中间人攻击 |

**代码示例 (Code Example):**

```rust
use chrono::{Utc, Duration};

const REQUEST_MAX_AGE_SECONDS: i64 = 300; // 5 分钟

fn validate_request_timestamp(timestamp: DateTime<Utc>) -> AuthResult<()> {
    let now = Utc::now();
    let age = now.signed_duration_since(timestamp);

    if age.num_seconds().abs() > REQUEST_MAX_AGE_SECONDS {
        return Err(AuthError::InvalidToken("Request too old".to_string()));
    }

    Ok(())
}
```

#### 4. 中间人攻击 (Man-in-the-Middle Attack)

**威胁描述 (Threat Description):**
攻击者拦截并修改通信数据。

**缓解措施 (Mitigations):**

| 措施 (Mitigation) | 实现 (Implementation) |
|------------------|----------------------|
| TLS 1.3 | 所有网络通信强制使用 TLS 1.3 |
| 证书固定 (Certificate Pinning) | 客户端验证服务器证书 |
| HMAC 签名 | API 请求使用 HMAC 签名 |

**配置示例 (Configuration Example):**

```nginx
# nginx 配置：强制 TLS 1.3
ssl_protocols TLSv1.3;
ssl_ciphers 'TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256';
ssl_prefer_server_ciphers off;
```

#### 5. 数据泄露 (Data Exfiltration)

**威胁描述 (Threat Description):**
攻击者窃取敏感数据。

**缓解措施 (Mitigations):**

| 措施 (Mitigation) | 实现 (Implementation) |
|------------------|----------------------|
| 静态加密 | EncryptedFS 加密所有敏感文件 |
| 传输加密 | TLS 1.3 保护网络传输 |
| 访问控制 | 严格的能力验证 |
| 数据脱敏 | 日志中不记录敏感数据 |

**代码示例 (Code Example):**

```rust
use evif_plugins::encryptedfs::EncryptedFsPlugin;

// 对敏感目录使用加密
let sensitive_dirs = vec!["/secrets", "/pii", "/financial"];

for dir in sensitive_dirs {
    let encrypted_backend = EncryptedFsPlugin::new(
        backend.clone(),
        encryption_config.clone()
    )?;
    // 所有写入该目录的数据都会自动加密
}
```

### 威胁建模方法论 (Threat Modeling Methodology)

**STRIDE 模型 (STRIDE Model):**

| 威胁类别 (Category) | EVIF 缓解措施 (EVIF Mitigation) |
|---------------------|-------------------------------|
| **S**poofing (伪装) | API Key 认证, Principal 身份验证 |
| **T**ampering (篡改) | AES-GCM 认证标签, 审计日志 |
| **R**epudiation (抵赖) | 完整审计日志, 时间戳记录 |
| **I**nformation Disclosure (信息泄露) | 静态加密, TLS 传输加密 |
| **D**enial of Service (拒绝服务) | 速率限制, 资源配额 |
| **E**levation of Privilege (权限提升) | 能力验证, Admin 权限隔离 |

---

## 安全配置指南 (Security Configuration Guide)

### 开发环境配置 (Development Environment)

**目标 (Goal):** 易用性优先，降低安全要求

```toml
# evif-config.toml

[server]
host = "127.0.0.1"
port = 8080

[auth]
policy = "Open"  # 开放策略，无需认证

[audit]
enabled = true
log_path = "evif-dev-audit.log"
sync_write = false  # 异步写入，提升性能

[storage]
backend = "Memory"  # 内存存储
```

**启动命令 (Startup Command):**

```bash
evif-server --config evif-config.toml --dev-mode
```

### 测试环境配置 (Testing Environment)

**目标 (Goal):** 模拟生产环境，但允许调试

```toml
# evif-config.toml

[server]
host = "0.0.0.0"
port = 8080

[auth]
policy = "Strict"  # 严格策略

[audit]
enabled = true
log_path = "/var/log/evif/audit.log"
sync_write = true  # 同步写入，确保不丢失

[storage]
backend = "Sled"
path = "/var/lib/evif/data"

[api]
require_api_key = true
rate_limit = 1000  # 每分钟 1000 次请求
```

**启动命令 (Startup Command):**

```bash
export EVIF_API_KEY="test-api-key-123"
evif-server --config evif-config.toml
```

### 生产环境配置 (Production Environment)

**目标 (Goal):** 最大安全性，性能优化

```toml
# evif-config.toml

[server]
host = "0.0.0.0"
port = 8080
workers = 4  # 多进程部署

[auth]
policy = "Strict"  # 必须使用严格策略

[audit]
enabled = true
log_path = "/var/log/evif/audit.log"
rotation_size = 104857600  # 100MB
sync_write = true  # 必须同步写入

[storage]
backend = "RocksDB"
path = "/var/lib/evif/data"

[api]
require_api_key = true
rate_limit = 100  # 每分钟 100 次请求
max_request_size = 10485760  # 10MB

[security]
tls_cert = "/etc/ssl/certs/evif.crt"
tls_key = "/etc/ssl/private/evif.key"
tls_protocols = ["TLSv1.3"]
tls_ciphers = ["TLS_AES_256_GCM_SHA384"]

[encryption]
enabled = true
master_password_env = "EVIF_ENCRYPTION_PASSWORD"
argon2_memory_kb = 65536  # 64MB
argon2_iterations = 3
argon2_parallelism = 4
```

**启动命令 (Startup Command):**

```bash
# 1. 设置环境变量（从密钥管理系统读取）
export EVIF_API_KEY=$(vault kv get -field=api_key secret/evif)
export EVIF_ENCRYPTION_PASSWORD=$(vault kv get -field=encryption_password secret/evif)

# 2. 启动服务
sudo -u evif evif-server --config /etc/evif/config.toml

# 3. 验证服务状态
systemctl status evif
```

**Systemd 服务文件 (Systemd Service File):**

```ini
# /etc/systemd/system/evif.service

[Unit]
Description=EVIF Storage Service
After=network.target
Wants=network-online.target

[Service]
Type=notify
User=evif
Group=evif

# 环境变量
EnvironmentFile=/etc/evif/evif.env
ExecStart=/usr/local/bin/evif-server --config /etc/evif/config.toml
ExecReload=/bin/kill -HUP $MAINPID

# 安全配置
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/evif /var/log/evif
UMask=0027

# 资源限制
LimitNOFILE=65536
LimitNPROC=4096

# 重启策略
Restart=always
RestartSec=10s

[Install]
WantedBy=multi-user.target
```

**启用服务 (Enable Service):**

```bash
sudo systemctl daemon-reload
sudo systemctl enable evif
sudo systemctl start evif
```

### 安全检查清单 (Security Checklist)

部署前检查：

```bash
#!/bin/bash
# security-check.sh

echo "=== EVIF Security Checklist ==="

# 1. 检查认证策略
if grep -q 'policy = "Strict"' /etc/evif/config.toml; then
    echo "✓ Auth policy is Strict"
else
    echo "✗ Auth policy is NOT Strict"
fi

# 2. 检查 API Key
if [ -n "$EVIF_API_KEY" ] && [ ${#EVIF_API_KEY} -ge 32 ]; then
    echo "✓ API Key is set and sufficiently long"
else
    echo "✗ API Key is missing or too short"
fi

# 3. 检查审计日志
if [ -f /var/log/evif/audit.log ]; then
    echo "✓ Audit log file exists"
else
    echo "✗ Audit log file missing"
fi

# 4. 检查文件权限
if stat -c %a /var/log/evif/audit.log | grep -q '600'; then
    echo "✓ Audit log has correct permissions (600)"
else
    echo "✗ Audit log has incorrect permissions"
fi

# 5. 检查 TLS 证书
if [ -f /etc/ssl/certs/evif.crt ] && [ -f /etc/ssl/private/evif.key ]; then
    echo "✓ TLS certificates exist"
else
    echo "✗ TLS certificates missing"
fi

# 6. 检查服务运行用户
if grep -q '^User=evif' /etc/systemd/system/evif.service; then
    echo "✓ Service runs as non-root user"
else
    echo "✗ Service may run as root"
fi

# 7. 检查防火墙
if iptables -L -n | grep -q 'dpt:8080.*ACCEPT'; then
    echo "✗ Firewall allows direct access to port 8080"
else
    echo "✓ Firewall restricts access to port 8080"
fi

echo "=== Checklist Complete ==="
```

---

## 总结 (Summary)

本章全面介绍了 EVIF 的安全架构和最佳实践：

**核心要点 (Key Takeaways):**

1. **纵深防御 (Defense in Depth)**: 网络、应用、数据、审计四层安全防护
2. **能力安全模型 (Capability-Based Security)**: 基于 Capability 的授权，而非 RBAC
3. **审计可追溯 (Audit Trail)**: 所有安全操作记录可审计日志，支持合规要求
4. **静态加密 (Encryption at Rest)**: EncryptedFS 插件提供透明的 AES-256-GCM 加密
5. **安全配置 (Security Configuration)**: 开发、测试、生产环境的不同配置策略

**下一步 (Next Steps):**

- [第九章：部署指南 (Chapter 9: Deployment Guide)](chapter-9-deployment.md) - 生产环境部署和运维
- [第七章：API 参考 (Chapter 7: API Reference)](chapter-7-api-reference.md) - REST/gRPC API 详细文档
- [第五章：插件开发 (Chapter 5: Plugin Development)](chapter-5-plugin-development.md) - 自定义安全插件开发

**参考资料 (References):**

- OWASP Top 10: https://owasp.org/www-project-top-ten/
- NIST Cybersecurity Framework: https://www.nist.gov/cyberframework
- CWE (Common Weakness Enumeration): https://cwe.mitre.org/
