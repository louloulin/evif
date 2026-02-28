# Broken Windows - Phase 0 Implementation Areas

**Research Date**: 2025-02-08
**Task**: evif-agfs-gap-analysis
**Focus**: Low-risk code smells in files Phase 0 will touch

## Definition

**Broken Window Theory**: Small issues (broken windows) in code that:
- Don't affect functionality
- Are low risk to fix
- Can be addressed opportunistically during Phase 0
- Should NOT require new tests or behavior changes

---

## 1. HTTPFS Error Misclassification

### File: `crates/evif-plugins/src/httpfs.rs`

#### Issue 1: Generic `InvalidPath` for All HTTP Errors
**Location**: Lines 62, 88, 110, 132, 162

**Current Code**:
```rust
.map_err(|e| EvifError::InvalidPath(format!("HTTP GET failed: {}", e)))?;
```

**Type**: error-handling
**Risk**: Low
**Fix**: Use correct error types based on error classification

**Proposed Fix**:
```rust
// Helper function
fn classify_reqwest_error(err: reqwest::Error) -> EvifError {
    if err.is_timeout() {
        EvifError::Timeout(0)
    } else if err.is_connect() || err.is_request() {
        EvifError::Network(err.to_string())
    } else {
        EvifError::Http(err.to_string())
    }
}

// Usage
.map_err(classify_reqwest_error)?
```

**Benefit**: Better error context for monitoring/debugging

---

#### Issue 2: No Status Code Differentiation
**Location**: Lines 64-72

**Current Code**:
```rust
if response.status().is_success() {
    Ok(response)
} else if response.status() == 404 {
    Err(EvifError::NotFound(path.to_string()))
} else {
    Err(EvifError::InvalidPath(format!(
        "HTTP error: {}",
        response.status()
    )))
}
```

**Type**: error-handling
**Risk**: Low
**Fix**: Distinguish retryable vs non-retryable HTTP errors

**Proposed Fix**:
```rust
use reqwest::StatusCode;

let status = response.status();
if status.is_success() {
    Ok(response)
} else if status == StatusCode::NOT_FOUND {
    Err(EvifError::NotFound(path.to_string()))
} else if matches!(status,
    StatusCode::REQUEST_TIMEOUT |      // 408
    StatusCode::TOO_MANY_REQUESTS |     // 429
    _ if status.is_server_error()       // 5xx
) {
    // Retryable errors
    Err(EvifError::Network(format!("HTTP {} (retryable)", status)))
} else {
    // Non-retryable
    Err(EvifError::Http(format!("HTTP {}", status)))
}
```

**Benefit**: Enables proper retry logic

---

#### Issue 3: Redundant URL Building Pattern
**Location**: Lines 42-50

**Current Code**:
```rust
fn build_url(&self, path: &str) -> String {
    let clean_path = path.trim_start_matches('/');
    if clean_path.is_empty() {
        format!("{}/", self.base_url)
    } else {
        format!("{}/{}", self.base_url, clean_path)
    }
}
```

**Type**: complexity (minor)
**Risk**: Low
**Fix**: Simplify logic

**Proposed Fix**:
```rust
fn build_url(&self, path: &str) -> String {
    let clean_path = path.trim_start_matches('/');
    format!("{}/{}", self.base_url.trim_end_matches('/'), clean_path)
}
```

**Benefit**: Simpler code, same behavior

---

## 2. FUSE Error Handling

### File: `crates/evif-fuse/src/lib.rs`

#### Issue 1: Generic `EIO` for All Read/Write Failures
**Location**: Lines 587, 626

**Current Code**:
```rust
Err(e) => {
    error!("read error: {}", e);
    reply.error(libc::EIO);
}
```

**Type**: error-handling
**Risk**: Low
**Fix**: Map EvifError to appropriate libc codes

**Proposed Fix**:
```rust
Err(e) => {
    error!("read error: {}", e);
    let libc_code = match e {
        EvifError::NotFound(_) => libc::ENOENT,
        EvifError::PermissionDenied(_) => libc::EACCES,
        _ => libc::EIO,
    };
    reply.error(libc_code);
}
```

**Benefit**: Better error propagation to userspace

---

#### Issue 2: Duplicate Runtime Clone Pattern
**Location**: Lines 276, 324, 452, 513, 559, 618, 661, 729, 786, 855, 916, 956, 1006

**Current Code** (repeated 13+ times):
```rust
let rt = self.runtime.clone();
```

**Type**: duplication
**Risk**: Low
**Fix**: Extract to helper method (if multiple clones needed)
**Note**: Actually NOT an issue - this is the correct pattern for Arc

**Verdict**: **DO NOT FIX** - Current approach is idiomatic Rust

---

#### Issue 3: Unused Variable in setattr
**Location**: Lines 294-305

**Current Code**:
```rust
fn setattr(
    &mut self,
    _req: &Request<'_>,
    ino: u64,
    _mode: Option<u32>,
    _uid: Option<u32>,
    _gid: Option<u32>,
    _size: Option<u64>,
    _atime: Option<TimeOrNow>,
    _mtime: Option<SystemTime>,
    _ctime: Option<SystemTime>,
    _fh: Option<u64>,
    _crtime: Option<SystemTime>,
    _chgtime: Option<SystemTime>,
    _bkuptime: Option<SystemTime>,
    _flags: Option<u32>,
    reply: ReplyAttr,
) {
```

**Type**: dead-code (partial)
**Risk**: Low
**Fix**: Implement or remove unused parameters

**Analysis**:
- `_mode`, `_size`, `_uid`, `_gid`, `_atime`, `_mtime` ARE used (Lines 331, 351, 361, 368)
- `_fh`, `_crtime`, `_chgtime`, `_bkuptime`, `_flags` are NOT used
- Prefix `_` correctly indicates "intentionally unused"

**Verdict**: **DO NOT FIX** - FUSE trait requires these parameters

---

## 3. Plugin Handler Patterns

### File: `crates/evif-rest/src/plugin_handlers.rs`

#### Issue 1: Hardcoded Plugin Configs
**Location**: Lines 175-268

**Current Code**:
```rust
match name.as_str() {
    "localfs" => Ok(Json(PluginConfigSchema {
        name: "localfs".to_string(),
        description: "Local filesystem plugin".to_string(),
        // ... 15 lines of hardcoded config
    })),
    "s3fs" => Ok(Json(PluginConfigSchema {
        // ... another 20 lines
    })),
    // ... repeats for 5+ plugins
}
```

**Type**: duplication
**Risk**: Low
**Fix**: Extract to registry or static map

**Proposed Fix**:
```rust
use once_cell::sync::Lazy;
use std::collections::HashMap;

static PLUGIN_SCHEMAS: Lazy<HashMap<&str, PluginConfigSchema>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("localfs", PluginConfigSchema { /* ... */ });
    m.insert("s3fs", PluginConfigSchema { /* ... */ });
    // ...
    m
});

// Usage
pub async fn get_plugin_config(
    State(_state): State<PluginState>,
    Path(name): Path<String>,
) -> Result<Json<PluginConfigSchema>, PluginError> {
    PLUGIN_SCHEMAS
        .get(name.as_str())
        .cloned()
        .ok_or_else(|| PluginError::NotFound(format!("Plugin not found: {}", name)))
        .map(Json)
}
```

**Dependency**: Add `once_cell = "1.19"` (or use `std::sync::OnceLock` in Rust 1.70+)

**Verdict**: **OPTIONAL** - Nice cleanup but not critical

---

#### Issue 2: String Allocation in Error Messages
**Location**: Lines 267, 318

**Current Code**:
```rust
Err(PluginError::NotFound(format!("Plugin not found: {}", name)))
```

**Type**: performance (minor)
**Risk**: Low
**Fix**: Use `impl Into<String>` or avoid allocation for hot paths

**Verdict**: **DO NOT FIX** - Not a hot path, clarity is more important

---

## 4. Missing Documentation

### File: `crates/evif-plugins/src/httpfs.rs`

#### Issue 1: No Module Documentation
**Location**: Lines 1-4

**Current Code**:
```rust
// HTTP File System Plugin for EVIF
// 对标 AGFS HTTPFS: 通过HTTP暴露文件系统内容
// 用途: 集成测试、REST API桥接、Web界面
```

**Type**: docs
**Risk**: Low
**Fix**: Convert to proper Rust doc comments

**Proposed Fix**:
```rust
//! HTTP File System Plugin for EVIF
//!
//! 对标 AGFS HTTPFS: 通过HTTP暴露文件系统内容
//!
//! # Usage
//!
//! ```no_run
//! use evif_plugins::HttpFsPlugin;
//!
//! let plugin = HttpFsPlugin::new("http://localhost:8080", 30);
//! ```
//!
//! # Features
//!
//! - File read/write via HTTP
//! - Metadata queries via HEAD
//! - Directory listing (basic HTML parsing)
//!
//! # Limitations
//!
//! - No retry logic (transient failures become permanent)
//! - Generic error handling (all errors → `InvalidPath`)
```

**Benefit**: Better IDE documentation, cargo doc output

---

#### Issue 2: Missing Error Code Documentation
**Location**: Throughout `httpfs.rs`

**Type**: docs
**Risk**: Low
**Fix**: Document which HTTP codes map to which errors

**Proposed Fix**:
```rust
/// Execute HTTP GET request with timeout
///
/// # Errors
///
/// - `EvifError::NotFound`: HTTP 404
/// - `EvifError::Timeout`: Request timeout
/// - `EvifError::Network`: Connection errors, 5xx, 408, 429
/// - `EvifError::Http`: Other HTTP status codes
async fn http_get(&self, path: &str) -> EvifResult<reqwest::Response> {
```

**Benefit**: Self-documenting API

---

## 5. Testing Gaps

### File: `crates/evif-plugins/src/httpfs.rs`

#### Issue 1: Offline-Only Tests
**Location**: Lines 277-294

**Current Code**:
```rust
#[tokio::test]
async fn test_httpfs_basic() {
    let plugin = HttpFsPlugin::new("https://httpbin.org", 30);

    match plugin.read("/get", 0, 1024).await {
        Ok(data) => {
            assert!(!data.is_empty());
        }
        Err(e) => {
            println!("HTTP read failed (expected in offline mode): {:?}", e);
        }
    }
}
```

**Type**: test-quality
**Risk**: Low
**Fix**: Add mocking or conditional compilation

**Proposed Fix**:
```rust
#[tokio::test]
#[cfg(feature = "integration-tests")]
async fn test_httpfs_integration() {
    // Only runs with --features integration-tests
    let plugin = HttpFsPlugin::new("https://httpbin.org", 30);
    let data = plugin.read("/get", 0, 1024).await.unwrap();
    assert!(!data.is_empty());
}

#[tokio::test]
async fn test_httpfs_url_building() {
    // Unit test (always runs)
    let plugin = HttpFsPlugin::new("http://localhost:8080", 10);
    assert_eq!(plugin.build_url(""), "http://localhost:8080/");
}
```

**Verdict**: **OPTIONAL** - Current approach is acceptable for testing

---

## Summary of Fixable Issues

### High Confidence (Safe to Fix During Phase 0):

1. **HTTPFS error classification** (Lines 62, 88, 110, 132, 162)
   - Change `InvalidPath` → correct error types
   - Enables retry logic
   - **EFFORT**: 1 hour

2. **HTTPFS status code differentiation** (Lines 64-72)
   - Add retryable vs non-retryable classification
   - **EFFORT**: 30 minutes

3. **HTTPFS URL building simplification** (Lines 42-50)
   - Simplify logic
   - **EFFORT**: 15 minutes

4. **FUSE error code mapping** (Lines 587, 626)
   - Map EvifError to appropriate libc codes
   - **EFFORT**: 30 minutes

5. **HTTPFS module documentation** (Lines 1-4)
   - Convert to proper doc comments
   - **EFFORT**: 30 minutes

**Total Fixable During Phase 0**: ~3 hours

### Optional (Nice-to-Have):

6. Plugin config registry extraction
   - **EFFORT**: 1-2 hours
   - **DEPENDENCY**: `once_cell` or Rust 1.70+

### Do NOT Fix:

- FUSE unused parameters (required by trait)
- FUSE runtime clone pattern (idiomatic Rust)
- Plugin handler string allocation (not hot path)
- HTTPFS offline tests (acceptable pattern)

---

## Implementation Strategy

**Phase 0 Blocking Work** (4-6 days):
1. FUSE I/O retry (1-2 days)
2. HTTPFS retry with jitter (2-3 days)
3. Plugin health endpoint (0.5 day)

**Broken Windows Fixes** (opportunistic, +0.5 day):
- Fix HTTPFS error classification (#1, #2 above)
- Add HTTPFS documentation (#5)
- Total additional time: ~2 hours

**Recommendation**: Include broken windows fixes in Phase 0
- Low risk
- Improves code quality
- Enables proper retry logic
- Minimal time overhead
