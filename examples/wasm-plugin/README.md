# EVIF WASM Plugin Example

这是一个使用 Extism PDK 开发的 EVIF WASM 插件示例。

## 功能

这个插件演示了一个简单的键值存储文件系统：
- `create`: 创建键值对
- `read`: 读取值
- `write`: 写入值
- `readdir`: 列出所有键
- `stat`: 获取键信息
- `remove`: 删除键

## 构建

### 前置要求

1. 安装 Rust 工具链
2. 安装 WASI 目标：`rustup target add wasm32-wasi`

### 编译

```bash
cargo build --release --target wasm32-wasi
```

编译后的 WASM 文件位于：`target/wasm32-wasi/release/evif_example_plugin.wasm`

## 使用

### 通过 REST API 加载

```bash
curl -X POST http://localhost:8080/api/v1/plugins/wasm/load \
  -H "Content-Type: application/json" \
  -d '{
    "wasm_path": "/path/to/evif_example_plugin.wasm",
    "name": "example_kv",
    "mount": "/kv"
  }'
```

### 使用插件

```bash
# 写入键值对
curl -X PUT "http://localhost:8080/api/v1/files?path=/kv/mykey" \
  -H "Content-Type: application/json" \
  -d '{"data": "Hello EVIF!"}'

# 读取值
curl "http://localhost:8080/api/v1/files?path=/kv/mykey"

# 列出所有键
curl "http://localhost:8080/api/v1/directories?path=/kv"

# 删除键
curl -X DELETE "http://localhost:8080/api/v1/files?path=/kv/mykey"
```

## 插件接口

插件需要实现以下导出函数：

### evif_create

创建文件/键值对。

**输入**:
```json
{
  "path": "string",
  "perm": 0
}
```

**输出**:
```json
{
  "success": true,
  "error": null
}
```

### evif_read

读取文件内容。

**输入**:
```json
{
  "path": "string",
  "offset": 0,
  "size": 0
}
```

**输出**:
```json
{
  "data": "base64_encoded_data",
  "error": null
}
```

### evif_write

写入文件内容。

**输入**:
```json
{
  "path": "string",
  "data": "base64_encoded_data",
  "offset": -1,
  "flags": 0
}
```

**输出**:
```json
{
  "bytes_written": 10,
  "error": null
}
```

### evif_readdir

列出目录内容。

**输入**:
```json
{
  "path": "string"
}
```

**输出**:
```json
{
  "files": [
    {
      "name": "file1",
      "size": 100,
      "mode": 0,
      "modified": "2024-01-01T00:00:00Z",
      "is_dir": false
    }
  ],
  "error": null
}
```

### evif_stat

获取文件信息。

**输入**:
```json
{
  "path": "string"
}
```

**输出**:
```json
{
  "file": {
    "name": "file1",
    "size": 100,
    "mode": 0,
    "modified": "2024-01-01T00:00:00Z",
    "is_dir": false
  },
  "error": null
}
```

### evif_remove

删除文件。

**输入**:
```json
{
  "path": "string"
}
```

**输出**:
```json
{
  "success": true,
  "error": null
}
```

### evif_rename

重命名文件。

**输入**:
```json
{
  "old_path": "string",
  "new_path": "string"
}
```

**输出**:
```json
{
  "success": true,
  "error": null
}
```

### evif_remove_all

递归删除目录。

**输入**:
```json
{
  "path": "string"
}
```

**输出**:
```json
{
  "success": true,
  "error": null
}
```

## 开发指南

### 1. 创建新的插件项目

```bash
cargo new --lib my_evif_plugin
cd my_evif_plugin
```

### 2. 添加依赖

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
extism-pdk = "1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
base64 = "0.22"
```

### 3. 实现插件接口

```rust
use extism_pdk::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct ReadRequest {
    path: String,
    offset: u64,
    size: u64,
}

#[derive(Serialize)]
struct ReadResponse {
    data: String,  // base64 encoded
    error: Option<String>,
}

#[plugin_fn]
pub fn evif_read(input: String) -> FnResult<String> {
    let req: ReadRequest = serde_json::from_str(&input)?;

    // 实现读取逻辑

    let response = ReadResponse {
        data: base64::encode(b"Hello from WASM!"),
        error: None,
    };

    Ok(serde_json::to_string(&response)?)
}
```

### 4. 编译和测试

```bash
cargo build --release --target wasm32-wasi
```

## 高级特性

### 使用 Host Functions

EVIF 提供的 host functions 让 WASM 插件可以调用宿主系统功能：

```rust
extern "C" {
    /// 调用 EVIF 核心存储层
    fn evif_host_read(path: *const u8, len: usize) -> Vec<u8>;
    fn evif_host_write(path: *const u8, data: *const u8, len: usize);
}
```

### 持久化存储

插件可以使用 Extism 的 KV 存储来持久化数据：

```rust
#[plugin_fn]
pub fn evif_write(input: String) -> FnResult<String> {
    let req: WriteRequest = serde_json::from_str(&input)?;

    // 使用 Extism KV 存储
    var::set(&req.path, req.data)?;

    Ok(serde_json::json!({"bytes_written": req.data.len()}).to_string())
}
```

## 调试

### 启用日志

```bash
# 设置环境变量
RUST_LOG=debug cargo run --bin evif-rest
```

### 查看 WASM 插件输出

Extism 插件的日志会通过 EVIF 的日志系统输出。

## 参考资料

- [Extism Documentation](https://extism.org/docs)
- [Extism PDK Guide](https://extism.org/docs/pdk)
- [EVIF Documentation](../../README.md)
- [WASI Standard](https://wasi.dev/)
