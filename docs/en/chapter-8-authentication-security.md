# Chapter 8: Authentication & Security

## Table of Contents

1. [Security Architecture Overview](#security-architecture-overview)
2. [Authentication Mechanisms](#authentication-mechanisms)
3. [Authorization Model](#authorization-model)
4. [Audit Logging System](#audit-logging-system)
5. [Encryption at Rest](#encryption-at-rest)
6. [Security Best Practices](#security-best-practices)
7. [Threat Models](#threat-models)
8. [Security Configuration Guide](#security-configuration-guide)

---

## Security Architecture Overview

### Design Principles

EVIF adopts a **Defense in Depth** security architecture, providing protection at multiple layers:

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

**Core Design Philosophy:**

1. **Principle of Least Privilege**: Each principal only has the minimum permissions needed to complete tasks
2. **Capability-Based Security**: Access rights are transferred through unforgeable capability objects
3. **Default Deny**: In Strict mode, operations without explicit authorization are denied by default
4. **Audit Trail**: All security-related operations are recorded in auditable logs
5. **Transparent Encryption**: Storage layer automatically encrypts, transparent to upper-layer applications

### Security Components Overview

| Component | Responsibility | Location |
|-----------|----------------|----------|
| **AuthManager** | Authentication manager, handles permission checks and authorization | `crates/evif-auth/src/auth.rs` |
| **Capability** | Capability object, encapsulates access rights | `crates/evif-auth/src/capability.rs` |
| **Principal** | Principal identity (user/service/system) | `crates/evif-auth/src/capability.rs` |
| **AuditLogger** | Audit log interface and implementations | `crates/evif-auth/src/audit.rs` |
| **EncryptedFS** | Encryption at rest plugin | `crates/evif-plugins/src/encryptedfs.rs` |

---

## Authentication Mechanisms

### Principal Types

EVIF uses **Principal** to identify actors:

```rust
pub enum Principal {
    User(UUID),      // User principal
    Service(UUID),   // Service principal
    System,          // System principal (superuser)
}
```

**Usage Scenarios:**

| Principal Type | Use Case | Permission Level |
|----------------|----------|------------------|
| `User` | End users, API clients | Permissions based on granted capabilities |
| `Service` | Background services, automated tasks | Permissions based on granted capabilities |
| `System` | Internal system operations | Superuser, bypasses permission checks |

**Code Example:**

```rust
use evif_auth::{Principal, AuthManager};
use uuid::Uuid;

// Create user principal
let user_id = Uuid::new_v4();
let user_principal = Principal::User(user_id);

// Create service principal
let service_id = Uuid::new_v4();
let service_principal = Principal::Service(service_id);

// System principal (no ID needed)
let system_principal = Principal::System;

// System principal always passes permission checks
let auth = AuthManager::new();
let result = auth.check(&system_principal, &node_id, Permission::Write)?;
assert!(result); // Always returns true
```

### Authentication Policies

EVIF supports two authentication policies:

#### 1. Open Policy

**Characteristics:**
- Allows all operations, no permission checks
- Suitable for development environments and local testing
- Still records audit logs

```rust
use evif_auth::{AuthManager, AuthPolicy};

// Create authentication manager with open policy
let auth = AuthManager::with_policy(AuthPolicy::Open);

// Any principal can access any resource
let user = Principal::User(Uuid::new_v4());
let resource_id = Uuid::new_v4();
let can_access = auth.check(&user, &resource_id, Permission::Write)?;
assert!(can_access); // Always true
```

#### 2. Strict Policy

**Characteristics:**
- Denies all operations without explicit authorization by default
- Requires granting permissions through Capabilities
- Suitable for production environments

```rust
use evif_auth::{AuthManager, AuthPolicy, Capability, Permissions};

// Create authentication manager with strict policy
let auth = AuthManager::with_policy(AuthPolicy::Strict);

let user = Principal::User(Uuid::new_v4());
let resource_id = Uuid::new_v4();

// Cannot access before authorization
let can_access = auth.check(&user, &resource_id, Permission::Read)?;
assert!(!can_access); // false

// Can access after granting permission
let cap = Capability::new(
    user.get_id().unwrap(),
    resource_id,
    Permissions::read()
);
auth.grant(cap)?;

let can_access = auth.check(&user, &resource_id, Permission::Read)?;
assert!(can_access); // true
```

**Policy Selection Guide:**

| Environment | Recommended Policy | Reasoning |
|-------------|-------------------|-----------|
| Local Development | `Open` | Fast iteration, no permission management needed |
| Internal Testing | `Strict` | Simulate production environment, discover permission issues |
| Production | `Strict` | Least privilege, maximum security |

---

## Authorization Model

### Capability-Based Security Model

EVIF adopts a **capability-based security model**, rather than traditional RBAC (Role-Based Access Control).

**Core Concepts:**

```
┌──────────────┐      grant      ┌──────────────┐
│  Principal   │ ──────────────> │  Capability  │
│              │                 │              │
└──────────────┘                 └──────────────┘
                                       │
                                       │ holds
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

**Capability Object:**

```rust
pub struct Capability {
    pub id: CapId,                    // Capability unique ID
    pub holder: PrincipalId,          // Holder ID
    pub node: Uuid,                   // Target resource ID
    pub permissions: Permissions,     // Permission set
    pub expires: Option<DateTime<Utc>>, // Expiration time (optional)
}
```

**Features:**

1. **Unforgeable**: Capability IDs are generated via UUID, unpredictable
2. **Transferable**: Can be transferred to other principals through secure channels
3. **Expirable**: Supports setting expiration times for temporary permissions
4. **Revocable**: Permissions can be revoked at any time via ID

### Permission Types

EVIF defines four basic permissions:

```rust
pub enum Permission {
    Read,    // Read permission
    Write,   // Write permission
    Execute, // Execute permission
    Admin,   // Administrator permission
}

pub struct Permissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub admin: bool,
}
```

**Permission Combination Examples:**

```rust
use evif_auth::{Permissions};

// Read-only permission
let read_only = Permissions::read();
// { read: true, write: false, execute: false, admin: false }

// Read-write permission
let read_write = Permissions::read_write();
// { read: true, write: true, execute: false, admin: false }

// Full permission
let full = Permissions::all();
// { read: true, write: true, execute: true, admin: true }

// Custom permission
let custom = Permissions {
    read: true,
    write: false,
    execute: true,
    admin: false,
};
```

**Permission to Operation Mapping:**

| Operation | Required Permission | Description |
|-----------|-------------------|-------------|
| Read file contents | `Read` | Open file and read data |
| List directory | `Read` | List child nodes under directory |
| Write file | `Write` | Modify file contents |
| Create file/directory | `Write` | Create new nodes under parent directory |
| Delete file/directory | `Write` | Delete nodes |
| Rename/move | `Write` | Modify node paths |
| Execute file | `Execute` | Execute executable files |
| Modify permissions | `Admin` | Grant/revoke other principals' permissions |
| Change ownership | `Admin` | Modify resource owner |

### Complete Authorization Flow

```rust
use evif_auth::{
    AuthManager, AuthPolicy, Capability, Permissions,
    Principal, Permission, AuditLogManager
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create authentication manager (strict policy + audit log)
    let audit_log = AuditLogManager::from_file("evif_audit.log")?;
    let auth = AuthManager::with_policy(AuthPolicy::Strict)
        .with_audit_log(audit_log);

    // 2. Create principal and resource
    let alice = Principal::User(Uuid::new_v4());
    let file_node = Uuid::new_v4();

    // 3. Create capability object
    let cap = Capability::new(
        alice.get_id().unwrap(),
        file_node,
        Permissions::read_write(),
    );

    // 4. Grant permission (recorded in audit log)
    let cap_id = auth.grant(cap).await?;
    println!("Capability granted: {}", cap_id);

    // 5. Check permission (automatically recorded in audit log)
    let can_read = auth.check(&alice, &file_node, Permission::Read).await?;
    println!("Alice can read: {}", can_read); // true

    let can_write = auth.check(&alice, &file_node, Permission::Write).await?;
    println!("Alice can write: {}", can_write); // true

    let can_execute = auth.check(&alice, &file_node, Permission::Execute).await?;
    println!("Alice can execute: {}", can_execute); // false

    // 6. Revoke permission (recorded in audit log)
    auth.revoke(&cap_id).await?;
    println!("Capability revoked");

    // 7. Cannot access after revocation
    let can_read_after = auth.check(&alice, &file_node, Permission::Read).await?;
    println!("Alice can read after revoke: {}", can_read_after); // false

    Ok(())
}
```

### Temporary Permission Management

Use expiration times to implement temporary access permissions:

```rust
use evif_auth::{Capability, Permissions};
use chrono::{Utc, Duration};
use uuid::Uuid;

let holder = Uuid::new_v4();
let resource = Uuid::new_v4();

// Create temporary capability that expires in 1 hour
let temp_cap = Capability::new(holder, resource, Permissions::read())
    .with_expiry(Utc::now() + Duration::hours(1));

auth.grant(temp_cap).await?;

// Check immediately - valid
let now = auth.check(&principal, &resource, Permission::Read).await?;
assert!(now); // true

// Check after expiration - invalid
tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
let expired = auth.check(&principal, &resource, Permission::Read).await?;
assert!(!expired); // false
```

**Use Cases:**
- **Temporary Sharing**: Grant third parties temporary access permissions
- **Scheduled Tasks**: Services that only run within specific time windows
- **Session Management**: Automatic expiration of user login sessions

---

## Audit Logging System

### Audit Event Types

EVIF records all security-related operations, supporting 9 audit event types:

```rust
pub enum AuditEventType {
    CapabilityGranted,      // Capability granted
    CapabilityRevoked,      // Capability revoked
    AccessGranted,          // Access granted
    AccessDenied,           // Access denied
    PolicyChanged,          // Policy changed
    AuthenticationFailed,   // Authentication failed
    SessionCreated,         // Session created
    SessionTerminated,      // Session terminated
}
```

### Audit Event Structure

```rust
pub struct AuditEvent {
    pub id: Uuid,                    // Event unique ID
    pub event_type: AuditEventType,  // Event type
    pub timestamp: DateTime<Utc>,    // Timestamp (UTC)
    pub principal_id: Option<Uuid>,  // Principal ID (who performed the operation)
    pub resource_id: Option<Uuid>,   // Resource ID (target of operation)
    pub success: bool,               // Operation result
    pub details: String,             // Event details
    pub ip_address: Option<String>,  // IP address (optional)
    pub user_agent: Option<String>,  // User agent (optional)
}
```

### Audit Logger Implementations

#### 1. Memory Audit Logger

**Features:**
- High performance, no I/O overhead
- Stores up to 10,000 events
- Suitable for development and testing environments

```rust
use evif_auth::{MemoryAuditLogger, AuditLogManager, AuditEvent, AuditEventType};

// Create memory audit logger
let logger = MemoryAuditLogger::new();
let audit = AuditLogManager::new(logger);

// Recorded events are automatically added to memory
let principal_id = Uuid::new_v4();
let resource_id = Uuid::new_v4();
audit.log_capability_granted(principal_id, resource_id)?;

// Query events
let events = audit.query(AuditFilter::new())?;
println!("Total events: {}", events.len());
```

#### 2. File Audit Logger

**Features:**
- Persistent storage, survives restarts
- Supports log rotation (10MB default)
- Optional synchronous write mode

```rust
use evif_auth::{FileAuditLogger, AuditLogManager, AuditConfig};
use std::path::Path;

// Create configuration
let config = AuditConfig {
    enabled: true,
    log_path: Some("/var/log/evif/audit.log".to_string()),
    rotation_size: 10 * 1024 * 1024, // 10MB
    sync_write: true, // Synchronous write, ensure persistence
};

// Create file audit logger
let audit = AuditLogManager::from_file("/var/log/evif/audit.log")?;

// Record event (written to both memory and file)
audit.log_access_granted(principal_id, resource_id, "read")?;
```

**Log Format:**

```
2025-03-01 12:34:56.789 UTC | AccessGranted | principal=550e8400-e29b-41d4-a716-446655440000 | resource=6ba7b810-9dad-11d1-80b4-00c04fd430c8 | success=true | Access granted: read permission for principal 550e8400-e29b-41d4-a716-446655440000 on resource 6ba7b810-9dad-11d1-80b4-00c04fd430c8
```

### Audit Log Querying

Use `AuditFilter` to query audit events matching specific criteria:

```rust
use evif_auth::{AuditFilter, AuditEventType};
use chrono::{Utc, Duration};

// Query all access denied events in the last hour
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

**Query Conditions:**

| Method | Parameter Type | Description |
|--------|---------------|-------------|
| `with_event_type()` | `AuditEventType` | Filter by event type |
| `with_principal_id()` | `Uuid` | Filter by principal ID |
| `with_resource_id()` | `Uuid` | Filter by resource ID |
| `with_start_time()` | `DateTime<Utc>` | Time range start |
| `with_end_time()` | `DateTime<Utc>` | Time range end |
| `with_success_only()` | `bool` | Success/failure filter |

### Audit Log Pruning

Regularly clean up old audit events:

```rust
use chrono::{Utc, Duration};

// Delete events older than 30 days
let cutoff = Utc::now() - Duration::days(30);
let deleted_count = audit.prune(cutoff)?;

println!("Deleted {} old audit events", deleted_count);
```

### Audit Logging Best Practices

**Production Configuration:**

```rust
use evif_auth::{AuditConfig, FileAuditLogger};

let config = AuditConfig {
    enabled: true,
    log_path: Some("/var/log/evif/audit.log".to_string()),
    rotation_size: 100 * 1024 * 1024, // 100MB
    sync_write: true, // Ensure persistence
};

// Configure log rotation (using external tools like logrotate)
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

**Security Recommendations:**

1. **Write Protection**: Audit log files should be set to append-only mode (`chmod a-w`)
2. **Independent Storage**: Store audit logs on a separate, secure filesystem
3. **Real-time Monitoring**: Use tools like `tail -f` or SIEM systems for real-time monitoring
4. **Regular Backup**: Backup audit logs to immutable storage (e.g., WORM storage)
5. **Access Control**: Only administrators can access audit log files

---

## Encryption at Rest

### EncryptedFS Plugin

EVIF provides the **EncryptedFS** plugin for transparent data encryption at rest.

**Encryption Algorithm:**

```
Master Password (user-provided password)
        │
        ▼
Argon2id Key Derivation (key derivation)
        │
        ▼
256-bit Encryption Key (encryption key)
        │
        ▼
AES-256-GCM (authenticated encryption)
        │
        ├─> Encrypted Data (encrypted data)
        └─> Authentication Tag (authentication tag)
```

**Technical Specifications:**

| Component | Algorithm/Parameter | Description |
|-----------|-------------------|-------------|
| **Encryption Algorithm** | AES-256-GCM | Authenticated encryption, provides confidentiality and integrity |
| **Key Derivation** | Argon2id | GPU/ASIC attack resistant key derivation |
| **Nonce Size** | 96 bits (12 bytes) | Unique per file, prevents replay attacks |
| **Authentication Tag** | 128 bits (16 bytes) | Detects data tampering |
| **Memory Cost** | 64 MB (default) | Argon2id memory parameter |
| **Iterations** | 3 (default) | Argon2id time parameter |
| **Parallelism** | 4 (default) | Argon2id parallel parameter |

### EncryptedFS Configuration

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

### EncryptedFS Usage Example

```rust
use evif_core::EvifPlugin;
use evif_plugins::encryptedfs::{EncryptedFsPlugin, EncryptedConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create backend storage (e.g., MemoryFs)
    let backend = Arc::new(MemoryFsPlugin::new());

    // Create EncryptedFS wrapper
    let config = EncryptedConfig {
        master_password: "secure-password-123".to_string(),
        ..Default::default()
    };

    let encrypted_fs = Arc::new(EncryptedFsPlugin::new(backend, config)?);

    // Write data (automatically encrypted)
    encrypted_fs.write(
        "/secret.txt",
        b"sensitive data".to_vec(),
        WriteFlags::default()
    ).await?;

    // Read data (automatically decrypted)
    let data = encrypted_fs.read("/secret.txt").await?;
    assert_eq!(data, b"sensitive data");

    // Underlying storage contains encrypted data
    let encrypted_data = backend.read("/secret.txt").await?;
    assert_ne!(encrypted_data, b"sensitive data");

    Ok(())
}
```

### Encrypted File Format

Each encrypted file contains a metadata header and encrypted data:

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

**Metadata Versioning:**

```rust
pub enum EncryptionVersion {
    V1 = 1,  // AES-256-GCM + Argon2id
    // Future versions can be added
    // V2 = 2,  // ChaCha20-Poly1305 + Scrypt
}
```

### Password Security Management

**Best Practices:**

1. **Don't Hardcode Passwords**: Use environment variables or secret management services
2. **Use Strong Passwords**: At least 32 characters, including uppercase, lowercase, numbers, symbols
3. **Regular Rotation**: Change master password quarterly (requires re-encryption of data)
4. **Secure Storage**: Use AWS Secrets Manager, HashiCorp Vault, etc.

```rust
use std::env;

// Read password from environment variable
let master_password = env::var("EVIF_ENCRYPTION_PASSWORD")
    .expect("EVIF_ENCRYPTION_PASSWORD must be set");

let config = EncryptedConfig {
    master_password,
    ..Default::default()
};
```

### Encryption Performance Considerations

**Performance Impact:**

| Operation | Performance Overhead | Notes |
|-----------|---------------------|-------|
| Read | ~5-10% | Requires decryption + authentication tag verification |
| Write | ~10-15% | Requires nonce generation + encryption |
| Random Access | Not applicable | Must read entire file |
| Concurrent Access | Good | Each file encrypted independently |

**Optimization Tips:**

1. **Use Caching**: EncryptedFS has built-in caching to reduce repeated decryption
2. **Adjust Argon2 Parameters**: Lower memory/iterations to improve performance (sacrifices security)
3. **Batch Operations**: Prefer batch reads/writes to reduce encryption/decryption overhead

```rust
// Performance-optimized configuration (lower security)
let fast_config = EncryptedConfig {
    master_password: "password".to_string(),
    argon2_memory_kb: 16384,    // 16 MB (vs 64 MB)
    argon2_iterations: 1,        // 1 (vs 3)
    argon2_parallelism: 2,       // 2 (vs 4)
};
```

---

## Security Best Practices

### Deployment Security

#### 1. Network Layer Security

**Use TLS 1.3:**

```nginx
# nginx reverse proxy configuration
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

**Firewall Rules:**

```bash
# Only allow specific IPs to access EVIF API
iptables -A INPUT -p tcp --dport 8080 -s 10.0.0.0/8 -j ACCEPT
iptables -A INPUT -p tcp --dport 8080 -j DROP
```

#### 2. Access Control

**API Key Management:**

```bash
# Generate strong random API Key
openssl rand -hex 32

# Store in secure environment variable
export EVIF_API_KEY="7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8"
```

**Regular API Key Rotation:**

```bash
# Rotate API Key every 90 days
0 0 1 */3 * root /usr/local/bin/rotate-evif-api-key.sh
```

#### 3. Isolated Deployment

**Using Docker Containers:**

```dockerfile
FROM ubuntu:22.04

# Create non-root user
RUN useradd -m -u 1000 evif

# Copy binary files
COPY evif-server /usr/local/bin/
COPY evif-fuse /usr/local/bin/

# Set permissions
RUN chmod +x /usr/local/bin/evif-* && \
    chown evif:evif /usr/local/bin/evif-*

# Switch to non-root user
USER evif

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s \
  CMD curl -f http://localhost:8080/health || exit 1

CMD ["evif-server"]
```

**Secure Docker Compose Configuration:**

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
      - "127.0.0.1:8080:8080"  # Local access only
    security_opt:
      - no-new-privileges:true
    read_only: true  # Read-only filesystem
    tmpfs:
      - /tmp:rw,noexec,nosuid,size=100m

networks:
  internal:
    driver: bridge

volumes:
  evif-data:
  evif-logs:
```

### Application Layer Security

#### 1. Input Validation

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
    // Handle user creation...
}
```

#### 2. Output Encoding

```rust
use ammonia::clean;

// HTML escape user input to prevent XSS
let safe_html = clean(user_input);
```

#### 3. SQL Injection Protection

EVIF uses parameterized queries, naturally defending against SQL injection:

```rust
// Safe (using parameterized query)
client.query(
    "SELECT * FROM files WHERE owner_id = $1",
    &[&user_id]
).await?;

// Unsafe (don't do this!)
client.query(
    &format!("SELECT * FROM files WHERE owner_id = '{}', user_id),
    &[]
).await?;
```

### Data Layer Security

#### 1. Encrypt Sensitive Data

```rust
use evif_plugins::encryptedfs::EncryptedFsPlugin;

// Use EncryptedFS for files containing sensitive information
let secure_backend = EncryptedFsPlugin::new(backend, encryption_config)?;

// Use normal backend for regular files
let normal_backend = MemoryFsPlugin::new();
```

#### 2. Secure Backup

```bash
#!/bin/bash
# secure-backup.sh

# 1. Stop EVIF service
systemctl stop evif

# 2. Backup data directory
tar -czf /backup/evif-data-$(date +%Y%m%d).tar.gz /var/lib/evif

# 3. Encrypt backup with GPG
gpg --symmetric --cipher-algo AES256 \
    /backup/evif-data-$(date +%Y%m%d).tar.gz

# 4. Upload to offsite storage
aws s3 cp /backup/evif-data-$(date +%Y%m%d).tar.gz.gpg \
    s3://secure-backup-bucket/

# 5. Delete local unencrypted backup
shred -u /backup/evif-data-$(date +%Y%m%d).tar.gz

# 6. Restart EVIF service
systemctl start evif
```

### Operational Security

#### 1. Principle of Least Privilege

```bash
# EVIF service runs under dedicated account
useradd -r -s /bin/false evif

# Data directory permissions
chown -R evif:evif /var/lib/evif
chmod 700 /var/lib/evif

# Log directory permissions
chown -R evif:adm /var/log/evif
chmod 750 /var/log/evif

# Audit log permissions
chmod 600 /var/log/evif/audit.log
chattr +a /var/log/evif/audit.log  # Append-only mode
```

#### 2. Security Updates

```bash
#!/bin/bash
# security-update.sh

# Check for security updates weekly
0 3 * * 0 root /usr/local/bin/security-update.sh

# Update system packages
apt-get update
apt-get upgrade -y

# Check for EVIF updates
cargo install evif-cli --force

# Restart service
systemctl restart evif
```

#### 3. Security Monitoring

```bash
# Monitor audit log for suspicious activity
tail -f /var/log/evif/audit.log | \
    grep -i "AccessDenied\|AuthenticationFailed" | \
    while read line; do
        # Send alert
        echo "$line" | mail -s "EVIF Security Alert" security@example.com
    done
```

---

## Threat Models

### Common Threats and Mitigations

#### 1. Unauthorized Access

**Threat Description:**
Attackers attempt to access resources they don't have permission for.

**Mitigations:**

| Mitigation | Implementation |
|-----------|---------------|
| Strong authentication policy | Use `AuthPolicy::Strict` |
| Capability verification | Call `auth.check()` before all operations |
| API Key validation | REST API requires valid `X-API-Key` |
| Audit logging | Record all access denied events |

**Code Example:**

```rust
use evif_auth::{AuthManager, AuthPolicy};

// Production environment must use Strict policy
let auth = AuthManager::with_policy(AuthPolicy::Strict);

// Verify permission before all operations
let can_access = auth.check(&principal, &resource, Permission::Read)?;
if !can_access {
    return Err(AuthError::Forbidden("Insufficient permissions".to_string()));
}

// Continue with operation...
```

#### 2. Privilege Escalation

**Threat Description:**
Low-privilege users attempt to gain administrator privileges.

**Mitigations:**

| Mitigation | Implementation |
|-----------|---------------|
| Admin permission isolation | Only explicit Admin permission can perform administrative operations |
| Unforgeable capabilities | Capability IDs use UUIDs, cannot be guessed |
| Revocation mechanism | Immediately revoke suspicious capabilities |
| Audit logging | Record all permission change operations |

**Code Example:**

```rust
// Administrative operations require explicit Admin permission
fn grant_capability(
    auth: &AuthManager,
    operator: &Principal,
    target: &Principal,
    resource: Uuid,
    permissions: Permissions,
) -> AuthResult<()> {
    // 1. Verify operator has Admin permission
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

    // 2. Grant permission
    let cap = Capability::new(target.get_id()?, resource, permissions);
    auth.grant(cap)?;

    // 3. Record audit log
    audit.log_capability_granted(target.get_id()?, resource)?;

    Ok(())
}
```

#### 3. Replay Attack

**Threat Description:**
Attackers intercept and replay valid requests.

**Mitigations:**

| Mitigation | Implementation |
|-----------|---------------|
| Request timestamps | REST API validates request timestamps (5-minute window) |
| Nonce replay protection | EncryptedFS uses unique per-file nonces |
| TLS protection | Network layer uses TLS 1.3 to prevent MITM attacks |

**Code Example:**

```rust
use chrono::{Utc, Duration};

const REQUEST_MAX_AGE_SECONDS: i64 = 300; // 5 minutes

fn validate_request_timestamp(timestamp: DateTime<Utc>) -> AuthResult<()> {
    let now = Utc::now();
    let age = now.signed_duration_since(timestamp);

    if age.num_seconds().abs() > REQUEST_MAX_AGE_SECONDS {
        return Err(AuthError::InvalidToken("Request too old".to_string()));
    }

    Ok(())
}
```

#### 4. Man-in-the-Middle Attack

**Threat Description:**
Attackers intercept and modify communication data.

**Mitigations:**

| Mitigation | Implementation |
|-----------|---------------|
| TLS 1.3 | All network communication requires TLS 1.3 |
| Certificate Pinning | Clients verify server certificates |
| HMAC signatures | API requests use HMAC signatures |

**Configuration Example:**

```nginx
# nginx configuration: enforce TLS 1.3
ssl_protocols TLSv1.3;
ssl_ciphers 'TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256';
ssl_prefer_server_ciphers off;
```

#### 5. Data Exfiltration

**Threat Description:**
Attackers steal sensitive data.

**Mitigations:**

| Mitigation | Implementation |
|-----------|---------------|
| Encryption at rest | EncryptedFS encrypts all sensitive files |
| Encryption in transit | TLS 1.3 protects network transmission |
| Access control | Strict capability verification |
| Data masking | Don't log sensitive data |

**Code Example:**

```rust
use evif_plugins::encryptedfs::EncryptedFsPlugin;

// Use encryption for sensitive directories
let sensitive_dirs = vec!["/secrets", "/pii", "/financial"];

for dir in sensitive_dirs {
    let encrypted_backend = EncryptedFsPlugin::new(
        backend.clone(),
        encryption_config.clone()
    )?;
    // All data written to this directory will be automatically encrypted
}
```

### Threat Modeling Methodology

**STRIDE Model:**

| Threat Category | EVIF Mitigation |
|-----------------|----------------|
| **S**poofing | API Key authentication, Principal identity verification |
| **T**ampering | AES-GCM authentication tags, audit logs |
| **R**epudiation | Complete audit logs, timestamp recording |
| **I**nformation Disclosure | Encryption at rest, TLS encryption in transit |
| **D**enial of Service | Rate limiting, resource quotas |
| **E**levation of Privilege | Capability verification, Admin permission isolation |

---

## Security Configuration Guide

### Development Environment Configuration

**Goal:** Prioritize usability, reduce security requirements

```toml
# evif-config.toml

[server]
host = "127.0.0.1"
port = 8080

[auth]
policy = "Open"  # Open policy, no authentication needed

[audit]
enabled = true
log_path = "evif-dev-audit.log"
sync_write = false  # Asynchronous write, improve performance

[storage]
backend = "Memory"  # Memory storage
```

**Startup Command:**

```bash
evif-server --config evif-config.toml --dev-mode
```

### Testing Environment Configuration

**Goal:** Simulate production environment, but allow debugging

```toml
# evif-config.toml

[server]
host = "0.0.0.0"
port = 8080

[auth]
policy = "Strict"  # Strict policy

[audit]
enabled = true
log_path = "/var/log/evif/audit.log"
sync_write = true  # Synchronous write, ensure no loss

[storage]
backend = "Sled"
path = "/var/lib/evif/data"

[api]
require_api_key = true
rate_limit = 1000  # 1000 requests per minute
```

**Startup Command:**

```bash
export EVIF_API_KEY="test-api-key-123"
evif-server --config evif-config.toml
```

### Production Environment Configuration

**Goal:** Maximum security, performance optimization

```toml
# evif-config.toml

[server]
host = "0.0.0.0"
port = 8080
workers = 4  # Multi-process deployment

[auth]
policy = "Strict"  # Must use strict policy

[audit]
enabled = true
log_path = "/var/log/evif/audit.log"
rotation_size = 104857600  # 100MB
sync_write = true  # Must write synchronously

[storage]
backend = "RocksDB"
path = "/var/lib/evif/data"

[api]
require_api_key = true
rate_limit = 100  # 100 requests per minute
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

**Startup Command:**

```bash
# 1. Set environment variables (read from secret management system)
export EVIF_API_KEY=$(vault kv get -field=api_key secret/evif)
export EVIF_ENCRYPTION_PASSWORD=$(vault kv get -field=encryption_password secret/evif)

# 2. Start service
sudo -u evif evif-server --config /etc/evif/config.toml

# 3. Verify service status
systemctl status evif
```

**Systemd Service File:**

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

# Environment variables
EnvironmentFile=/etc/evif/evif.env
ExecStart=/usr/local/bin/evif-server --config /etc/evif/config.toml
ExecReload=/bin/kill -HUP $MAINPID

# Security configuration
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/evif /var/log/evif
UMask=0027

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

# Restart policy
Restart=always
RestartSec=10s

[Install]
WantedBy=multi-user.target
```

**Enable Service:**

```bash
sudo systemctl daemon-reload
sudo systemctl enable evif
sudo systemctl start evif
```

### Security Checklist

Pre-deployment check:

```bash
#!/bin/bash
# security-check.sh

echo "=== EVIF Security Checklist ==="

# 1. Check authentication policy
if grep -q 'policy = "Strict"' /etc/evif/config.toml; then
    echo "✓ Auth policy is Strict"
else
    echo "✗ Auth policy is NOT Strict"
fi

# 2. Check API Key
if [ -n "$EVIF_API_KEY" ] && [ ${#EVIF_API_KEY} -ge 32 ]; then
    echo "✓ API Key is set and sufficiently long"
else
    echo "✗ API Key is missing or too short"
fi

# 3. Check audit log
if [ -f /var/log/evif/audit.log ]; then
    echo "✓ Audit log file exists"
else
    echo "✗ Audit log file missing"
fi

# 4. Check file permissions
if stat -c %a /var/log/evif/audit.log | grep -q '600'; then
    echo "✓ Audit log has correct permissions (600)"
else
    echo "✗ Audit log has incorrect permissions"
fi

# 5. Check TLS certificates
if [ -f /etc/ssl/certs/evif.crt ] && [ -f /etc/ssl/private/evif.key ]; then
    echo "✓ TLS certificates exist"
else
    echo "✗ TLS certificates missing"
fi

# 6. Check service user
if grep -q '^User=evif' /etc/systemd/system/evif.service; then
    echo "✓ Service runs as non-root user"
else
    echo "✗ Service may run as root"
fi

# 7. Check firewall
if iptables -L -n | grep -q 'dpt:8080.*ACCEPT'; then
    echo "✗ Firewall allows direct access to port 8080"
else
    echo "✓ Firewall restricts access to port 8080"
fi

echo "=== Checklist Complete ==="
```

---

## Summary

This chapter comprehensively introduced EVIF's security architecture and best practices:

**Key Takeaways:**

1. **Defense in Depth**: Four-layer security protection: network, application, data, and audit
2. **Capability-Based Security**: Authorization based on Capabilities, not RBAC
3. **Audit Trail**: All security operations recorded in auditable logs, supporting compliance requirements
4. **Encryption at Rest**: EncryptedFS plugin provides transparent AES-256-GCM encryption
5. **Security Configuration**: Different configuration strategies for development, testing, and production environments

**Next Steps:**

- [Chapter 9: Deployment Guide](chapter-9-deployment.md) - Production environment deployment and operations
- [Chapter 7: API Reference](chapter-7-api-reference.md) - Detailed REST/gRPC API documentation
- [Chapter 5: Plugin Development](chapter-5-plugin-development.md) - Custom security plugin development

**References:**

- OWASP Top 10: https://owasp.org/www-project-top-ten/
- NIST Cybersecurity Framework: https://www.nist.gov/cyberframework
- CWE (Common Weakness Enumeration): https://cwe.mitre.org/
