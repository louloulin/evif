# Technologies - EVIF Phase 0 Implementation

**Research Date**: 2025-02-08
**Task**: evif-agfs-gap-analysis
**Focus**: Available libraries and frameworks for Phase 0

## 1. Async Runtime & Concurrency

### Tokio (v1.35)
**Workspace Dependency**: `Cargo.toml:31`
```toml
tokio = { version = "1.35", features = ["full"] }
```

**Features Available**:
- `time` module for sleep/duration (⏱️ **needed for retry backoff**)
- `runtime` for blocking on async (used in FUSE)
- `multi-thread` scheduler (already used)

**Usage Pattern for Backoff**:
```rust
use tokio::time::{sleep, Duration};
use std::time::Duration;

async fn retry_with_backoff() {
    sleep(Duration::from_millis(100)).await;  // First retry
    sleep(Duration::from_millis(400)).await;  // Second retry
    sleep(Duration::from_millis(1600)).await; // Third retry
}
```

**Jitter Implementation**:
```rust
use rand::Rng;

let base_delay = Duration::from_millis(100);
let jitter = rand::thread_rng().gen_range(0..100); // 0-100ms
let actual_delay = base_delay + Duration::from_millis(jitter);
sleep(actual_delay).await;
```

**Dependency Check**: `rand` crate NOT in workspace dependencies.
- **Option 1**: Add `rand = "0.8"` to workspace dependencies
- **Option 2**: Use simpler approach: `Duration::from_millis(100 + (rand::random::<u64>() % 100))`

---

## 2. HTTP Client

### reqwest (v0.12)
**Plugin Dependency**: `crates/evif-plugins/Cargo.toml:37`
```toml
reqwest = { version = "0.12", features = ["json"] }
```

**Capabilities**:
- ✅ Built-in timeout support: `.timeout(Duration)`
- ✅ Async operations via `.send().await`
- ✅ Connection pooling (automatic)
- ❌ No built-in retry (need to implement)
- ✅ Error types: `reqwest::Error` can distinguish:
  - `reqwest::Error::TimedOut` → use for `EvifError::Timeout`
  - `reqwest::Error::Connection` → use for `EvifError::Network`
  - `reqwest::Error::Request` → check status code

**Timeout Implementation**:
```rust
let response = client
    .get(&url)
    .timeout(Duration::from_secs(30))
    .send()
    .await?;

// Check if timeout occurred
if let Err(reqwest::Error::TimedOut) = result {
    return Err(EvifError::Timeout(30));
}
```

**Status Code Detection**:
```rust
use reqwest::StatusCode;

match response.status() {
    s if s.is_success() => Ok(response),
    StatusCode::NOT_FOUND => Err(EvifError::NotFound(path.to_string())),
    StatusCode::REQUEST_TIMEOUT | // 408
    StatusCode::TOO_MANY_REQUESTS | // 429
    s if s.is_server_error() => // 5xx
        Err(EvifError::Network(format!("HTTP {}: retryable", s))),
    s => Err(EvifError::Http(format!("HTTP {}", s))),
}
```

---

## 3. Error Handling

### thiserror (v1.56)
**Workspace Dependency**: `Cargo.toml:45`
```toml
thiserror = "1.0.56"
```

**Already Used**: `EvifError` enum derives `thiserror::Error` (error.rs:9)

**Phase 0 Usage**:
- No changes needed to error types
- All required variants exist: `Timeout`, `Network`, `Http`
- Just need to use correct variant in right place

**Error Conversion Pattern**:
```rust
// Convert reqwest::Error to EvifError
impl From<reqwest::Error> for EvifError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            EvifError::Timeout(0) // extract actual timeout if available
        } else if err.is_connect() || err.is_body() || err.is_decode() {
            EvifError::Network(err.to_string())
        } else {
            EvifError::Http(err.to_string())
        }
    }
}
```

---

## 4. Logging & Tracing

### tracing (v0.1.40)
**Workspace Dependency**: `Cargo.toml:35-36`
```toml
tracing = "0.1.40"
tracing-subscriber = { version = "0.3.18", features = ["env-filter"] }
```

**Usage Pattern**:
```rust
use tracing::{debug, info, warn, error};

// Log retry attempt
warn!(
    attempt = attempt_num,
    max_attempts = 3,
    delay_ms = delay.as_millis(),
    "Retrying FUSE read operation after transient error"
);

// Log final failure
error!(
    path = %path,
    attempts = 3,
    "FUSE read failed after all retry attempts"
);
```

**Structured Logging** (recommended for Phase 0):
- Use field syntax: `path = %path` (display) or `path = ?path` (debug)
- Add context: attempt numbers, delays, error types
- Helps with debugging retry issues in production

---

## 5. Retry Pattern Libraries

### Available Options:

**Option 1: Custom Implementation (RECOMMENDED)**
- Pros:
  - No new dependencies
  - Full control over retry logic
  - Matches design spec exactly
  - Can optimize for EVIF's specific needs
- Cons:
  - More code to write (~50-100 lines)
  - Need to test edge cases

**Option 2: tokio-retry (EXTERNAL)**
- Crate: `tokio-retry = "0.3"`
- Pros:
  - Tested library
  - Flexible backoff strategies
- Cons:
  - New dependency
  - May not match exact spec requirements
  - Less control

**Option 3: backoff (EXTERNAL)**
- Crate: `backoff = "0.4"`
- Pros:
  - Popular retry library
  - Exponential backoff with jitter built-in
- Cons:
  - New dependency
  - More complex than needed
  - Designed for generic use cases

**RECOMMENDATION**: Custom implementation
- Phase 0 is small scope (just FUSE I/O + HTTPFS)
- Tokio's `sleep()` is sufficient
- Full control over retry logic matches design spec exactly
- Can add specialized logging/context easily

---

## 6. FUSE Library

### fuser (version TBD)
**Used in**: `crates/evif-fuse`

**Error Handling**:
- Returns libc error codes: `libc::EIO`, `libc::ENOENT`, `libc::EINTR`
- Already mapped in FUSE handlers

**Phase 0 Integration**:
- Need to detect retryable errors: `EIO`, `EINTR`
- Wrap `read_async()` and `write_async()` calls
- Log retry attempts with `warn!()`
- Return `EIO` only after all retries exhausted

**Retryable libc Errors**:
```rust
fn is_retryable_libc_error(code: i32) -> bool {
    matches!(code, libc::EIO | libc::EINTR)
}
```

---

## 7. Testing Framework

### Available Tools:
- `tokio-test = "0.4.3"` ✅ (async testing)
- `tempfile = "3.8"` ✅ (temporary files)
- `proptest = "1.4.0"` ✅ (property-based, if needed)
- `criterion = "0.5.1"` ✅ (benchmarking, if needed)

**E2E Testing** (Phase 1, separate task):
- Will use Playwright MCP (external tool)
- Not in Rust dependencies
- Task: `task-1770549344-d854`

---

## 8. Serialization (for health endpoint)

### serde & serde_json
**Workspace Dependencies**: `Cargo.toml:39-40`
```toml
serde = { version = "1.0.196", features = ["derive"] }
serde_json = "1.0.113"
```

**Already Used**: Plugin config schemas (plugin_handlers.rs:24-48)

**Phase 0 Usage**:
```rust
#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,  // "ok" | "error"
    message: Option<String>,
}

// In handler
Ok(Json(HealthResponse {
    status: "ok".to_string(),
    message: None,
}))
```

---

## 9. No Additional Dependencies Required

**Key Finding**: Phase 0 can be implemented with **ZERO new dependencies**

All required functionality:
- ✅ Async sleep/duration: `tokio::time`
- ✅ HTTP client with timeout: `reqwest`
- ✅ Error types: `EvifError` (Timeout, Network variants)
- ✅ Logging: `tracing`
- ✅ Serialization: `serde`

**Optional Additions** (if needed):
- `rand = "0.8"` - for jitter implementation
  - Can also use: `std::time::SystemTime::now().nanos() % 100`
  - Simpler: random 0-100ms using system time

---

## 10. Integration Dependencies

### Axum Framework
**Used in**: `crates/evif-rest`

**Pattern** (from plugin_handlers.rs:6-13):
```rust
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
```

**Phase 0 Health Endpoint**:
```rust
// Add to PluginHandlers impl
pub async fn get_plugin_health(
    State(state): State<PluginState>,
    Path(name): Path<String>,
) -> Result<Json<HealthResponse>, PluginError> {
    let mount_paths = state.mount_table.list_mounts().await;

    // Find plugin by name
    let plugin = mount_paths
        .iter()
        .find_map(|path| state.mount_table.lookup(path).await)
        .ok_or_else(|| PluginError::NotFound(format!("Plugin: {}", name)))?;

    // Call health check
    match plugin.health().await {
        Ok(()) => Ok(Json(HealthResponse {
            status: "ok".to_string(),
            message: None,
        })),
        Err(e) => Ok(Json(HealthResponse {
            status: "error".to_string(),
            message: Some(e.to_string()),
        })),
    }
}
```

---

## Summary: Technology Stack for Phase 0

| Component | Technology | Version | Status |
|-----------|-----------|---------|--------|
| Async Runtime | Tokio | 1.35 | ✅ Available |
| HTTP Client | reqwest | 0.12 | ✅ Available |
| Error Handling | thiserror | 1.56 | ✅ Available |
| Logging | tracing | 0.1.40 | ✅ Available |
| Serialization | serde/serde_json | 1.0/1.113 | ✅ Available |
| Web Framework | axum | (in evif-rest) | ✅ Available |
| FUSE | fuser | (in evif-fuse) | ✅ Available |
| Retry Logic | Custom | - | ✅ To implement |
| Jitter | System time / rand | - | ⚠️ Optional |

**Effort Estimate**:
- FUSE I/O retry wrapper: ~50 lines of Rust
- HTTPFS retry decorator: ~80 lines of Rust
- Plugin health endpoint: ~30 lines of Rust
- Integration and testing: ~40 lines of Rust

**Total**: ~200 lines of new Rust code for Phase 0
