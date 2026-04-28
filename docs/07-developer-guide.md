# EVIF Developer Guide

## 1. Getting Started

### 1.1 Prerequisites

- Rust 1.75+ (with `cargo`)
- OpenSSL (for TLS)
- Protocol Buffers (optional, for gRPC)

### 1.2 Build

```bash
# Clone repository
git clone https://github.com/evif/evif
cd evif

# Build all crates
cargo build --release

# Build specific crate
cargo build -p evif-rest

# Build with all features
cargo build --release --all-features
```

### 1.3 Run Tests

```bash
# Run all tests
cargo test --workspace

# Run specific test
cargo test -p evif-core radix_mount_table

# Run with output
cargo test -p evif-rest -- --nocapture

# Run integration tests (requires server)
cargo test -p evif-e2e -- --test-threads=1
```

### 1.4 Run Linting

```bash
# Format code
cargo fmt

# Lint
cargo clippy --workspace -- -D warnings

# Check
cargo check --workspace
```

## 2. Project Structure

```
evif/
├── Cargo.toml              # Workspace definition
├── crates/
│   ├── evif-core/         # Core engine (~26 modules)
│   │   ├── src/
│   │   │   ├── lib.rs    # Public exports
│   │   │   ├── plugin.rs # Plugin trait
│   │   │   ├── error.rs  # Error types
│   │   │   └── ...       # Other core modules
│   │   └── Cargo.toml
│   │
│   ├── evif-plugins/     # Plugin implementations
│   │   ├── src/
│   │   │   ├── lib.rs    # Plugin exports
│   │   │   ├── memfs.rs  # Memory plugin
│   │   │   ├── localfs.rs # Local fs plugin
│   │   │   └── ...       # 40+ other plugins
│   │   └── Cargo.toml
│   │
│   ├── evif-rest/        # REST API server
│   ├── evif-cli/         # CLI tool
│   ├── evif-python/       # Python SDK
│   ├── evif-sdk-ts/      # TypeScript SDK
│   ├── evif-sdk-go/      # Go SDK
│   ├── evif-mcp/         # MCP server
│   ├── evif-auth/        # Authentication
│   └── ...
│
├── tests/                 # Integration tests
├── examples/             # Example code
├── demos/                # Demo applications
└── docs/                # Documentation
```

## 3. Creating a New Plugin

### 3.1 Plugin Structure

```rust
// crates/evif-plugins/src/myplugin.rs

use async_trait::async_trait;
use bytes::Bytes;
use evif_core::{EvifPlugin, EvifError, EvifResult, FileHandle, CreateOptions};
use std::path::Path;

/// My custom plugin
pub struct MyPlugin {
    config: MyConfig,
}

#[derive(Debug, Clone)]
pub struct MyConfig {
    pub option1: String,
    pub option2: bool,
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
        // Create file
        Ok(FileHandle::new(path))
    }

    async fn read(&self, handle: &FileHandle, offset: u64, size: u64) -> EvifResult<Bytes> {
        // Read file content
        Ok(Bytes::new())
    }

    async fn write(&self, handle: &FileHandle, offset: u64, data: Bytes) -> EvifResult<u64> {
        // Write file content
        Ok(data.len() as u64)
    }

    async fn readdir(&self, path: &Path, offset: u64) -> EvifResult<Vec<DirEntry>> {
        // List directory
        Ok(vec![])
    }

    async fn stat(&self, path: &Path) -> EvifResult<Metadata> {
        // Get file metadata
        Ok(Metadata::default())
    }

    async fn remove(&self, path: &Path) -> EvifResult<()> {
        // Delete file
        Ok(())
    }

    async fn rename(&self, old: &Path, new: &Path) -> EvifResult<()> {
        // Rename file
        Ok(())
    }

    async fn remove_all(&self, path: &Path) -> EvifResult<u32> {
        // Delete directory recursively
        Ok(0)
    }
}

/// Plugin factory function
pub fn create_my_plugin(config: Option<&Value>) -> EvifResult<Arc<dyn EvifPlugin>> {
    Ok(Arc::new(MyPlugin::new(config)?))
}
```

### 3.2 Register Plugin

```rust
// crates/evif-plugins/src/lib.rs

pub mod myplugin;

pub use myplugin::create_my_plugin;
```

### 3.3 Add to Build

```toml
# crates/evif-plugins/Cargo.toml

[features]
default = ["memfs", "localfs", "contextfs", ...]
myplugin = []
```

## 4. Adding REST Endpoints

### 4.1 Create Handler

```rust
// crates/evif-rest/src/my_handler.rs

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};

pub async fn my_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Handler logic
    (StatusCode::OK, "OK")
}

pub fn routes() -> Router {
    Router::new()
        .route("/my-endpoint", get(my_handler))
        .route("/my-endpoint/:id", post(my_handler))
}
```

### 4.2 Register Routes

```rust
// crates/evif-rest/src/routes.rs

use crate::my_handler;

let app = Router::new()
    // ... existing routes
    .route("/api/v1/my-endpoint", get(my_handler::my_handler))
    .route("/api/v1/my-endpoint/:id", post(my_handler::my_handler_with_id))
    // ...
```

## 5. Python SDK Development

### 5.1 Project Structure

```
crates/evif-python/
├── evif/
│   ├── __init__.py      # Package exports
│   ├── client.py        # Async client
│   ├── sync.py          # Sync wrapper
│   ├── models.py        # Pydantic models
│   ├── memory.py        # Memory API mixin
│   ├── queue.py         # Queue API mixin
│   ├── exceptions.py    # Error classes
│   └── types.py         # Type definitions
├── tests/
│   └── ...
├── pyproject.toml
└── README.md
```

### 5.2 Adding a New Method

```python
# evif/client.py

async def my_new_method(self, param: str) -> dict:
    """Description of the method.

    Args:
        param: Description of parameter

    Returns:
        Description of return value
    """
    result = await self._request("POST", "/api/v1/my-endpoint", json={"param": param})
    return result
```

### 5.3 Adding Tests

```python
# tests/test_my_method.py

import pytest
from evif import EvifClient

@pytest.mark.asyncio
async def test_my_new_method():
    async with EvifClient() as client:
        result = await client.my_new_method("test")
        assert "expected" in result
```

## 6. Testing

### 6.1 Unit Tests

```rust
// In the source file
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radix_lookup() {
        let table = RadixMountTable::new();
        // Test logic
    }
}
```

### 6.2 Integration Tests

```rust
// tests/api/my_test.rs

use evif_core::EvifServer;
use evif_rest::EvifHandlers;

#[tokio::test]
async fn test_my_endpoint() {
    // Start test server
    let server = EvifServer::test_new().await;

    // Make request
    let response = server.make_request(
        http::Method::GET,
        "/api/v1/my-endpoint",
    ).await;

    assert_eq!(response.status(), StatusCode::OK);
}
```

### 6.3 E2E Tests

```rust
// tests/e2e/test_integration.rs

#[tokio::test]
async fn test_full_workflow() {
    // Start real server
    let port = find_available_port();
    let server = spawn_server(port).await;

    // Wait for health
    wait_for_health(port).await;

    // Test workflow
    let client = EvifClient::new(port);
    client.mkdir("/mem/test").await;
    client.write("/mem/test/file.txt", "content").await;

    // Cleanup
    server.stop().await;
}
```

## 7. Documentation

### 7.1 Rust Documentation

```rust
//! My module provides X, Y, Z.
//!
//! # Example
//!
//! ```
//! use evif_core::MyStruct;
//! let instance = MyStruct::new();
//! ```

/// A brief description of the struct.
///
/// # Fields
///
/// * `field1` - Description of field1
/// * `field2` - Description of field2
///
/// # Example
///
/// ```
/// let s = MyStruct {
///     field1: "value".to_string(),
///     field2: 42,
/// };
/// ```
pub struct MyStruct {
    field1: String,
    field2: u32,
}
```

### 7.2 Markdown Documentation

```markdown
# My Feature

## Overview
Brief description.

## Usage
```bash
evif my-command --flag value
```

## API Reference
Describe API endpoints.

## Configuration
| Option | Default | Description |
|--------|---------|-------------|
| flag | false | Enable feature |
```

## 8. Code Style

### 8.1 Formatting

```bash
# Auto-format
cargo fmt

# Check formatting
cargo fmt -- --check
```

### 8.2 Linting

```bash
# Run clippy
cargo clippy --workspace -- -D warnings

# Fix auto-fixable issues
cargo clippy --workspace --fix --allow-dirty
```

### 8.3 Naming Conventions

| Item | Convention | Example |
|------|------------|---------|
| Struct | PascalCase | `MyStruct` |
| Enum | PascalCase | `MyEnum` |
| Function | snake_case | `my_function` |
| Module | snake_case | `my_module` |
| Constant | SCREAMING_SNAKE | `MY_CONSTANT` |
| Private field | snake_case | `my_field` |

### 8.4 Error Handling

```rust
// Use EvifError for EVIF-specific errors
pub type EvifResult<T> = Result<T, EvifError>;

// Prefer anyhow for application errors
pub type AppResult<T> = Result<T, anyhow::Error>;

// Use thiserror for library errors
#[derive(Debug, Error)]
pub enum MyError {
    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("operation failed")]
    OperationFailed(#[from] std::io::Error),
}
```

## 9. Performance

### 9.1 Benchmarking

```rust
// benches/my_bench.rs

use criterion::{black_box, criterion_group, Criterion};

fn bench_my_operation(c: &mut Criterion) {
    c.bench_function("my_operation", |b| {
        b.iter(|| {
            let data = black_box(some_data());
            my_operation(data)
        })
    });
}

criterion_group!(benches, bench_my_operation);
```

### 9.2 Profiling

```bash
# CPU profiling
cargo flamegraph --bin evif-rest -- -port 8081

# Memory profiling
cargo heap-profiler

# Async profiling
cargo instrument --output=trace.json
```

### 9.3 Key Optimizations

- Use `Arc` instead of cloning
- Use `Bytes` for zero-copy operations
- Use `DashMap` for concurrent hash maps
- Use `serde_json` with `no_cache` feature
- Use `tracing` with `no Subscriber` guard

## 10. Release Process

### 10.1 Version Bump

```bash
# Update version in Cargo.toml
cargo release patch  # 0.1.0 -> 0.1.1
cargo release minor # 0.1.0 -> 0.2.0
cargo release major # 0.1.0 -> 1.0.0
```

### 10.2 Changelog

```markdown
# Changelog

## [0.1.1] - 2026-04-27

### Added
- New `my-endpoint` API
- Python SDK support for X

### Fixed
- Bug in memory plugin

### Changed
- Updated dependency X to 2.0
```

### 10.3 Publish

```bash
# Build release
cargo build --release

# Run tests
cargo test --workspace

# Publish crates
cargo publish -p evif-core
cargo publish -p evif-plugins
cargo publish -p evif-rest
# etc.
```

## 11. Related Documents

- [Architecture Overview](00-overview.md)
- [Plugin System](02-plugin-system.md)
- [REST API Reference](03-rest-api.md)
- [SDK Integration](04-sdk-integration.md)
