# EVIF 1.6 使用指南

**版本**: 1.6.0
**日期**: 2025-01-24
**完成度**: 89% (8/9 核心插件)

---

## 📚 目录

1. [快速开始](#快速开始)
2. [插件参考](#插件参考)
3. [API文档](#api文档)
4. [使用示例](#使用示例)
5. [最佳实践](#最佳实践)
6. [故障排除](#故障排除)

---

## 🚀 快速开始

### 安装

```bash
# 克隆仓库
git clone https://github.com/your-org/evif.git
cd evif

# 构建项目
cargo build --release

# 运行测试
cargo test
```

### 基础使用

```rust
use evif_core::{EvifServer, WriteFlags};
use evif_plugins::{LocalFsPlugin, MemFsPlugin};
use std::sync::Arc;

#[tokio::main]
async fn main() -> evif_core::EvifResult<()> {
    // 创建服务器
    let server = Arc::new(EvifServer::new());

    // 挂载插件
    server.register_plugin(
        "/local",
        Arc::new(LocalFsPlugin::new("/tmp", false))
    ).await?;

    server.register_plugin(
        "/mem",
        Arc::new(MemFsPlugin::new())
    ).await?;

    // 使用插件
    server.write("/local/test.txt", b"Hello!", 0, WriteFlags::CREATE).await?;
    server.write("/mem/data.txt", b"In memory", 0, WriteFlags::CREATE).await?;

    // 读取数据
    let data = server.read("/local/test.txt", 0, 100).await?;
    println!("{}", String::from_utf8_lossy(&data));

    Ok(())
}
```

---

## 📦 插件参考

### 1. LocalFS - 本地文件系统

**功能**: 访问本地文件系统,带路径遍历保护

**挂载**:
```rust
server.register_plugin(
    "/local",
    Arc::new(LocalFsPlugin::new("/path/to/dir", true)) // true = 创建目录
).await?;
```

**特性**:
- ✅ 完整的文件读写操作
- ✅ 路径遍历攻击防护 (`canonicalize()` 验证)
- ✅ 父目录安全检查
- ⚠️  **安全**: 仅允许访问挂载点内的路径

**使用场景**:
- 本地文件存储
- 配置文件管理
- 日志文件写入

---

### 2. KVFS - 键值存储

**功能**: 扁平键值存储,支持虚拟目录层次

**挂载**:
```rust
server.register_plugin(
    "/kv",
    Arc::new(KvfsPlugin::new("my_store"))
).await?;
```

**特性**:
- ✅ 虚拟目录结构 (路径前缀)
- ✅ 路径深度计算
- ✅ 扁平存储,树形访问

**使用示例**:
```rust
// 写入键值
server.write("/kv/config/app/name", b"MyApp", 0, WriteFlags::CREATE).await?;
server.write("/kv/config/app/port", b"8080", 0, WriteFlags::CREATE).await?;

// 列出虚拟目录
let entries = server.readdir("/kv/config/app").await?;
// 返回: [FileInfo{name: "name"}, FileInfo{name: "port"}]
```

**使用场景**:
- 配置管理
- 元数据存储
- 缓存层

---

### 3. QueueFS - 消息队列

**功能**: FIFO消息队列,支持UUID和JSON序列化

**挂载**:
```rust
server.register_plugin(
    "/queue",
    Arc::new(QueueFsPlugin::new())
).await?;
```

**控制文件**:
- `/queue/{name}/enqueue` - 入队
- `/queue/{name}/dequeue` - 出队
- `/queue/{name}/peek` - 查看队首
- `/queue/{name}/size` - 队列大小
- `/queue/{name}/clear` - 清空队列

**使用示例**:
```rust
// 创建队列
server.mkdir("/queue/tasks", 0o755).await?;

// 发送消息
server.write(
    "/queue/tasks/enqueue",
    br#"{"task": "process", "id": 123}"#,
    0,
    WriteFlags::CREATE
).await?;

// 接收消息
let msg = server.read("/queue/tasks/dequeue", 0, 1024).await?;

// 查看队列大小
let size = server.read("/queue/tasks/size", 0, 1024).await?;
```

**使用场景**:
- 任务调度
- 异步消息处理
- 事件驱动架构

---

### 4. ServerInfoFS - 服务器信息

**功能**: 提供服务器元数据(只读)

**挂载**:
```rust
server.register_plugin(
    "/server",
    Arc::new(ServerInfoFsPlugin::new("1.6.0"))
).await?;
```

**虚拟文件**:
- `/server/version` - 版本信息
- `/server/uptime` - 运行时间
- `/server/stats` - 统计信息
- `/server/info` - 完整信息

**使用示例**:
```rust
let version = server.read("/server/version", 0, 1024).await?;
println!("{}", String::from_utf8_lossy(&version));
// 输出: {"version":"1.6.0","build":"2025-01-24"}
```

**使用场景**:
- 健康检查
- 监控端点
- 版本管理

---

### 5. MemFS - 内存文件系统

**功能**: 完全内存中的文件系统,支持树结构

**挂载**:
```rust
server.register_plugin(
    "/mem",
    Arc::new(MemFsPlugin::new())
).await?;
```

**特性**:
- ✅ 完整的层次结构
- ✅ 递归路径遍历
- ✅ 高速读写
- ⚠️  **重启后数据丢失**

**使用示例**:
```rust
// 创建目录结构
server.mkdir("/mem/data", 0o755).await?;
server.mkdir("/mem/data/cache", 0o755).await?;

// 写入文件
server.create("/mem/data/file.txt", 0o644).await?;
server.write(
    "/mem/data/file.txt",
    b"Hello, Memory!",
    0,
    WriteFlags::CREATE
).await?;

// 递归删除
server.remove("/mem/data").await?;
```

**使用场景**:
- 临时文件存储
- 高速缓存
- 测试环境

---

### 6. HttpFS - HTTP客户端

**功能**: 通过HTTP协议访问远程资源

**挂载**:
```rust
server.register_plugin(
    "/http",
    Arc::new(HttpFsPlugin::new("https://api.example.com", 30))
).await?;
```

**特性**:
- ✅ GET/PUT/DELETE/HEAD 操作
- ✅ 超时控制
- ✅ 自动错误处理(404 → NotFound)

**使用示例**:
```rust
// HTTP GET
let data = server.read("/http/users/1", 0, 1024).await?;

// HTTP PUT
server.write(
    "/http/data",
    br#"{"key": "value"}"#,
    0,
    WriteFlags::CREATE
).await?;

// HTTP DELETE
server.remove("/http/old/resource").await?;
```

**使用场景**:
- REST API客户端
- 微服务通信
- Webhook处理

---

### 7. StreamFS - 流式数据处理

**功能**: 多读者多写者流式数据,支持历史重放

**挂载**:
```rust
server.register_plugin(
        "/stream",
        Arc::new(StreamFsPlugin::new())
    ).await?;
```

**特性**:
- ✅ 多读者并发
- ✅ 环形缓冲区
- ✅ 新读者历史数据重放
- ✅ 死读者自动清理

**使用示例**:
```rust
// 创建流
server.create("/stream/logs", 0o644).await?;

// 写入多条日志
for i in 1..=10 {
    server.write(
        "/stream/logs",
        format!("Log line {}\n", i).as_bytes().to_vec(),
        0,
        WriteFlags::APPEND
    ).await?;
}

// 读取(新读者自动获取历史数据)
let logs = server.read("/stream/logs", 0, 1024).await?;
```

**使用场景**:
- 日志收集
- 实时数据流
- 事件总线

---

### 8. ProxyFS - 远程文件系统代理 🆕

**功能**: 连接远程EVIF/AGFS服务器,支持热重载

**挂载**:
```rust
server.register_plugin(
    "/remote",
    Arc::new(ProxyFsPlugin::new("http://localhost:8080/api/v1"))
).await?;
```

**特性**:
- ✅ EVIF/AGFS HTTP客户端
- ✅ 远程文件操作代理
- ✅ `/reload` 热重载控制文件
- ✅ 健康检查

**热重载**:
```rust
// 触发热重载
server.write(
    "/remote/reload",
    b"reload",
    0,
    WriteFlags::CREATE
).await?;

// 查看重载状态
let info = server.read("/remote/reload", 0, 1024).await?;
println!("{}", String::from_utf8_lossy(&info));
```

**使用场景**:
- 分布式文件系统
- 远程备份
- 负载均衡
- 故障转移

---

## 🔧 API文档

### EvifServer 核心方法

#### 挂载插件
```rust
pub async fn register_plugin(
    &self,
    mount_point: String,
    plugin: Arc<dyn EvifPlugin>
) -> EvifResult<()>
```

**参数**:
- `mount_point`: 挂载路径 (如 "/local", "/kv")
- `plugin`: 插件实例

**返回**: `EvifResult<()>`

---

#### 文件操作

**创建文件**:
```rust
pub async fn create(&self, path: &str, perm: u32) -> EvifResult<()>
```

**创建目录**:
```rust
pub async fn mkdir(&self, path: &str, perm: u32) -> EvifResult<()>
```

**读取文件**:
```rust
pub async fn read(
    &self,
    path: &str,
    offset: u64,
    size: u64
) -> EvifResult<Vec<u8>>
```

**写入文件**:
```rust
pub async fn write(
    &self,
    path: &str,
    data: Vec<u8>,
    offset: i64,
    flags: WriteFlags
) -> EvifResult<u64>
```

**列出目录**:
```rust
pub async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>>
```

**文件状态**:
```rust
pub async fn stat(&self, path: &str) -> EvifResult<FileInfo>
```

**删除文件**:
```rust
pub async fn remove(&self, path: &str) -> EvifResult<()>
```

**重命名**:
```rust
pub async fn rename(
    &self,
    old_path: &str,
    new_path: &str
) -> EvifResult<()>
```

---

### WriteFlags 标志

```rust
pub enum WriteFlags {
    const NONE = 0;       // 无标志
    const APPEND = 1;     // 追加写入
    const CREATE = 2;     // 创建文件
    const EXCLUSIVE = 4;  // 排他创建
    const TRUNCATE = 8;   // 截断文件
    const SYNC = 16;      // 同步写入
}
```

**使用**:
```rust
// 追加写入
server.write("/file.txt", data, 0, WriteFlags::APPEND).await?;

// 创建或截断
server.write("/file.txt", data, 0, WriteFlags::CREATE | WriteFlags::TRUNCATE).await?;
```

---

## 💡 最佳实践

### 1. 路径规范

**推荐**:
```rust
// ✅ 使用绝对路径
server.read("/local/config/app.json", 0, 1024).await?;

// ✅ 统一路径格式
let path = format!("/{}/{}", mount_point, relative_path);
```

**避免**:
```rust
// ❌ 混用相对路径
server.read("local/config.json", 0, 1024).await?;

// ❌ 重复斜杠
server.read("//local//config.json", 0, 1024).await?;
```

---

### 2. 错误处理

**推荐**:
```rust
use evif_core::EvifError;

match server.read("/local/file.txt", 0, 1024).await {
    Ok(data) => {
        // 处理数据
    }
    Err(EvifError::NotFound(path)) => {
        // 文件不存在
        eprintln!("File not found: {}", path);
    }
    Err(EvifError::InvalidPath(msg)) => {
        // 路径错误
        eprintln!("Invalid path: {}", msg);
    }
    Err(e) => {
        // 其他错误
        eprintln!("Error: {}", e);
    }
}
```

---

### 3. 资源清理

**推荐**:
```rust
// 使用Arc管理插件生命周期
let plugin = Arc::new(LocalFsPlugin::new("/tmp", false));
server.register_plugin("/local".to_string(), plugin).await?;

// 插件会在服务器销毁时自动清理
```

---

### 4. 性能优化

**批量操作**:
```rust
// ✅ 批量读取
let paths = vec!["/local/file1.txt", "/local/file2.txt"];
let futures: Vec<_> = paths.iter()
    .map(|&p| server.read(p, 0, 1024))
    .collect();

let results = futures::future::join_all(futures).await;
```

**并发写入**:
```rust
// ✅ 使用append避免冲突
server.write("/queue/tasks/enqueue", task1, 0, WriteFlags::APPEND).await?;
server.write("/queue/tasks/enqueue", task2, 0, WriteFlags::APPEND).await?;
```

---

## 🔍 故障排除

### 常见问题

#### 1. 路径遍历错误

**错误**: `InvalidPath: Path traversal detected`

**原因**: LocalFS检测到路径遍历攻击

**解决**:
```rust
// ✅ 确保路径在挂载点内
let base_path = "/safe/directory";
let plugin = LocalFsPlugin::new(base_path, false);

// ❌ 不要访问挂载点外的路径
server.read("/local/../../../etc/passwd", 0, 1024).await?;
```

---

#### 2. 插件未找到

**错误**: `NotFound: /unknown/path`

**原因**: 路径未匹配到任何插件

**解决**:
```rust
// ✅ 检查插件是否已挂载
server.register_plugin("/data", Arc::new(MemFsPlugin::new())).await?;

// ✅ 使用正确的挂载点
server.read("/data/file.txt", 0, 1024).await?;
```

---

#### 3. 队列为空

**错误**: 读取 `/queue/tasks/dequeue` 返回空

**原因**: 队列中没有消息

**解决**:
```rust
// ✅ 先检查队列大小
let size = server.read("/queue/tasks/size", 0, 1024).await?;
if String::from_utf8_lossy(&size) != "0" {
    let msg = server.read("/queue/tasks/dequeue", 0, 1024).await?;
    // 处理消息
}
```

---

#### 4. StreamFS无数据

**问题**: 读取流时没有数据

**原因**: 流为空或没有读者

**解决**:
```rust
// ✅ 先写入数据
server.create("/stream/logs", 0o644).await?;
server.write("/stream/logs", b"Log entry\n", 0, WriteFlags::APPEND).await?;

// ✅ 然后读取
let data = server.read("/stream/logs", 0, 1024).await?;
```

---

### 调试技巧

**启用日志**:
```rust
env_logger::init();

// 或使用自定义日志
println!("Reading from: {}", path);
```

**列出所有挂载点**:
```rust
// 遍历所有可能的挂载点
let mount_points = ["/local", "/kv", "/queue", "/mem"];

for &mount in &mount_points {
    match server.readdir(mount).await {
        Ok(entries) => {
            println!("{}: {} entries", mount, entries.len());
        }
        Err(_) => {
            println!("{}: not mounted", mount);
        }
    }
}
```

---

## 📚 更多资源

- **GitHub**: https://github.com/your-org/evif
- **文档**: https://docs.evif.rs
- **示例**: `examples/` 目录
- **AGFS参考**: https://github.com/c4pt0r/agfs

---

**文档版本**: 1.6.0
**最后更新**: 2025-01-24
**维护者**: EVIF Team
