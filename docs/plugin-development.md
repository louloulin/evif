# EVIF 插件开发指南

本文档说明如何基于 EVIF 实现并挂载自定义插件，与 AGFS 的 ServicePlugin/FileSystem 对标。实现 `EvifPlugin` 即可被 RadixMountTable 挂载；可选实现 `HandleFS`、`Streamer` 以支持句柄与流式接口。

---

## 一、EvifPlugin 必须实现的方法

所有插件必须实现 `evif_core::EvifPlugin` trait，并提供以下方法：

| 方法 | 说明 | 对标 AGFS |
|------|------|-----------|
| `name(&self) -> &str` | 插件名称，用于 REST 与挂载识别（如 `mem`、`localfs`） | ServicePlugin Name |
| `create(&self, path, perm) -> EvifResult<()>` | 创建空文件 | Create |
| `mkdir(&self, path, perm) -> EvifResult<()>` | 创建目录 | Mkdir |
| `read(&self, path, offset, size) -> EvifResult<Vec<u8>>` | 读取文件（offset/size，0 表示全部） | Read |
| `write(&self, path, data, offset, flags) -> EvifResult<u64>` | 写入文件；返回写入字节数 | Write |
| `readdir(&self, path) -> EvifResult<Vec<FileInfo>>` | 列出目录内容 | Readdir |
| `stat(&self, path) -> EvifResult<FileInfo>` | 获取文件/目录元数据 | Stat |
| `remove(&self, path) -> EvifResult<()>` | 删除文件或空目录 | Remove |
| `rename(&self, old_path, new_path) -> EvifResult<()>` | 重命名/移动 | Rename |
| `remove_all(&self, path) -> EvifResult<()>` | 递归删除目录及内容 | RemoveAll |

可选覆盖（默认返回不支持）：

- `symlink(target_path, link_path)`、`readlink(link_path)`：符号链接
- `chmod(path, mode)`：修改权限
- `truncate(path, size)`：截断文件

---

## 二、配置与校验约定（Phase 8）

### 2.1 validate(config)

挂载前由 evif-rest 调用；配置无效时应返回 `Err(EvifError::InvalidInput(...))`，阻止挂载并返回 400。

- 无配置插件：可默认 `Ok(())`。
- 需要配置的插件（如 LocalFS）：在 `validate` 中检查必填项（如 `config.root` 非空）。

示例（LocalFsPlugin）：

```rust
async fn validate(&self, config: Option<&serde_json::Value>) -> EvifResult<()> {
    if let Some(c) = config {
        if let Some(root) = c.get("root").and_then(|v| v.as_str()) {
            if root.trim().is_empty() {
                return Err(EvifError::InvalidInput("config.root must be non-empty".to_string()));
            }
        }
    }
    Ok(())
}
```

### 2.2 get_readme() -> String

返回插件说明文档（Markdown），用于 GET `/api/v1/plugins/:name/readme` 与 Web 管理界面展示。

- 建议包含：插件简介、配置参数表、挂载示例。

示例（MemFsPlugin）：

```rust
fn get_readme(&self) -> String {
    r#"# MemFS
内存文件系统插件，数据仅存于进程内存，重启后丢失。无需配置。
## 配置
无（无需配置参数）。
"#.to_string()
}
```

### 2.3 get_config_params() -> Vec<PluginConfigParam>

返回配置参数元数据，用于 GET `/api/v1/plugins/:name/config` 与前端表单。

`PluginConfigParam` 字段：`name`、`param_type`（如 `"string"`）、`required`、`default`、`description`。

示例（LocalFsPlugin）：

```rust
fn get_config_params(&self) -> Vec<PluginConfigParam> {
    vec![
        PluginConfigParam {
            name: "root".to_string(),
            param_type: "string".to_string(),
            required: true,
            default: Some("/tmp/evif-local".to_string()),
            description: Some("本地根目录路径".to_string()),
        },
    ]
}
```

无参数插件返回 `vec![]`。

---

## 三、HandleFS（可选）

若需支持 REST 句柄 API（`/api/v1/handles/open`、read、write、seek、close 等），需同时实现：

1. **evif_core::HandleFS**：继承 `EvifPlugin`，增加 `open_handle`、`get_handle`、`close_handle`。
2. **evif_core::FileHandle**：实现 `read`、`read_at`、`write`、`write_at`、`seek`、`sync`、`close`、`stat`、`flags` 等。

并在 `EvifPlugin::as_handle_fs(&self)` 中返回 `Some(self)`，使 REST 层能向下转型为 HandleFS。

参考：`evif_plugins::handlefs::HandleFsPlugin`。

---

## 四、Streamer（可选）

若需支持流式读取（如日志流、事件流），需实现：

1. **evif_core::Streamer**：`open_stream(path) -> EvifResult<Box<dyn StreamReader>>`。
2. **evif_core::StreamReader**：`read_chunk(timeout) -> (Vec<u8>, bool)`、`close`、`is_finished`。

并在 `EvifPlugin::as_streamer(&self)` 中返回 `Some(self)`。

参考：`evif_core::streaming::MemoryStreamReader`、`evif_plugins::streamfs`。

---

## 五、最小插件示例（仅 EvifPlugin）

1. 在 `evif-plugins` 中新建模块（如 `myfs.rs`），实现 `EvifPlugin` 的上述必须方法；内部可用内存 HashMap 或本地路径等存储。
2. 在 `evif-plugins/src/lib.rs` 中 `pub mod myfs` 并 `pub use myfs::MyFsPlugin`。
3. 在 evif-rest 的 mount 处理器中增加分支：当 `plugin == "myfs"` 时，创建 `MyFsPlugin::new()` 并调用 `mount_table.mount(path, Arc::new(plugin))`。
4. 挂载：POST `/api/v1/mount`，Body `{ "path": "/my", "plugin": "myfs", "config": {} }`。

无配置时 `validate` 与 `get_config_params` 可使用 trait 默认实现（Ok(()) 与 vec![]）；`get_readme` 建议返回简短说明。

---

## 六、挂载与配置格式

- **启动配置**：`evif.json` 或 `EVIF_MOUNTS` 环境变量，格式为 `{ "mounts": [ { "path": "/mem", "plugin": "mem", "config": {} } ] }`。local 插件示例：`{ "path": "/local", "plugin": "local", "config": { "root": "/tmp/evif-local" } }`。
- **动态挂载**：POST `/api/v1/mount`，Body 同上；先执行 `plugin.validate(config)`，失败则 400。
- **REST 暴露**：GET `/api/v1/plugins/:name/readme`、GET `/api/v1/plugins/:name/config` 会通过内置插件名（mem/hello/local 等）创建临时实例并返回 `get_readme()`、`get_config_params()`。

---

## 七、错误与路径约定

- 使用 `evif_core::EvifError`（如 `NotFound`、`InvalidPath`、`InvalidInput`、`NotSupportedGeneric`）；REST 层会映射为 404/400/500。
- 路径：以 `/` 开头，挂载后为「挂载点 + 相对路径」（如 `/mem/foo.txt`）；插件内只需处理已去掉挂载前缀的路径或由 RadixMountTable 解析后的路径，依 evif-rest 约定为准。
- 本地文件系统插件必须做路径遍历检查（如 canonicalize 后确保在 base_path 下），参见 LocalFsPlugin::resolve_path。

---

## 八、参考实现

| 插件 | 说明 | 配置校验 | README/ConfigParams |
|------|------|----------|----------------------|
| MemFsPlugin | 内存文件系统 | 无 | 有 get_readme，无参数 |
| HelloFsPlugin | 只读示例 | 无 | 有 get_readme，无参数 |
| LocalFsPlugin | 本地目录挂载 | validate(root 非空) | 有 get_readme + get_config_params(root) |
| HandleFsPlugin | 句柄扩展 | - | 实现 HandleFS + FileHandle |

更多插件见 `crates/evif-plugins/src/*.rs`；AGFS 对照见 `agfs-server/pkg/plugins/` 与 `pkg/plugin/plugin.go`。

---

**文档版本**：与 EVIF 2.4 Phase 12.2 对应；实现后可通过 POST `/api/v1/mount` 挂载并在 GET `/api/v1/mounts` 中看到新挂载点。
