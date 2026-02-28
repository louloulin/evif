# EVIF 2.2 - WebSocket 实现说明

## 概述

本文档说明 EVIF 2.2 WebSocket 后端的实现状态和使用方法。

## 实现状态

### ✅ 已完成

1. **WebSocket 处理器模块** (`crates/evif-rest/src/ws_handlers.rs`)
   - ✅ WebSocket 连接升级处理
   - ✅ JSON 消息协议定义
   - ✅ 终端命令执行引擎
   - ✅ 支持的命令：help, ls, cat, stat, mounts, pwd, echo, clear
   - ✅ 错误处理和响应

2. **路由集成** (`crates/evif-rest/src/routes.rs`)
   - ✅ WebSocket 端点：`/ws`
   - ✅ 状态管理

3. **依赖添加** (`crates/evif-rest/Cargo.toml`)
   - ✅ tokio-tungstenite v0.21.0
   - ✅ futures-util v0.3.28

### ⏳ 待完成

1. **编译问题修复**
   - ⏳ 修复 evif-rest 现有编译错误
   - ⏳ 确保 WebSocket 模块正常编译

2. **测试验证**
   - ⏳ WebSocket 连接测试
   - ⏳ 命令执行测试
   - ⏳ 前后端集成测试

## WebSocket 协议

### 消息格式

所有 WebSocket 消息使用 JSON 格式：

```typescript
// 客户端发送命令
{
  "type": "command",
  "data": {
    "command": "ls /"
  }
}

// 服务器响应 - 输出
{
  "type": "output",
  "data": {
    "output": "📁 mem\n📁 local\n📁 hello\n\n$ "
  }
}

// 服务器响应 - 错误
{
  "type": "error",
  "data": {
    "message": "Failed to list directory: Not found"
  }
}
```

### 支持的命令

| 命令 | 参数 | 描述 | 示例 |
|------|------|------|------|
| `help` | 无 | 显示帮助信息 | `help` |
| `clear` | 无 | 清空终端屏幕 | `clear` |
| `ls` | [path] | 列出目录内容 | `ls /`, `ls /mem` |
| `cat` | <path> | 读取文件内容 | `cat /mem/test.txt` |
| `stat` | <path> | 获取文件元数据 | `stat /local/` |
| `mounts` | 无 | 列出所有挂载点 | `mounts` |
| `pwd` | 无 | 打印当前目录 | `pwd` |
| `echo` | <text> | 回显文本 | `echo Hello` |

## 前端集成

### Terminal.tsx 连接

前端的 `Terminal.tsx` 组件已经准备好连接 WebSocket：

```typescript
// WebSocket 连接 (ws://localhost:8080/ws)
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onopen = () => {
  console.log('WebSocket connected');
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  if (data.type === 'output') {
    xterm.write(data.output);
  } else if (data.type === 'error') {
    xterm.write(`\r\n\x1b[1;31mError: ${data.message}\x1b[0m\r\n`);
  }
};

// 发送命令
ws.send(JSON.stringify({
  type: 'command',
  data: { command: 'ls /' }
}));
```

## 测试步骤

### 1. 启动 EVIF REST 服务器

```bash
cd evif
cargo run --bin evif-rest
```

服务器将在 `http://localhost:8080` 启动，WebSocket 端点为 `ws://localhost:8080/ws`。

### 2. 启动前端开发服务器

```bash
cd evif-web
bun run dev
```

前端将在 `http://localhost:3000` 启动。

### 3. 测试 WebSocket 连接

打开浏览器开发者工具，在终端中执行：

```javascript
// 测试 WebSocket 连接
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onopen = () => {
  console.log('✅ WebSocket connected');

  // 发送测试命令
  ws.send(JSON.stringify({
    type: 'command',
    data: { command: 'help' }
  }));
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('📨 Received:', data);
};

ws.onerror = (error) => {
  console.error('❌ WebSocket error:', error);
};
```

## 当前实现细节

### WebSocket 消息类型 (Rust)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WSMessage {
    #[serde(rename = "command")]
    Command { command: String },
    #[serde(rename = "output")]
    Output { output: String },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "file_update")]
    FileUpdate { path: String, content: String },
}
```

### 命令执行流程

1. 客户端发送 `WSMessage::Command`
2. 服务器解析命令
3. 调用 `execute_command()`
4. 根据命令类型调用相应的 EVIF API
5. 返回 `WSMessage::Output` 或 `WSMessage::Error`

### 集成的 EVIF API

- `mount_table.list_nodes(path)` - 列出文件
- `mount_table.read_file(path, offset, len)` - 读取文件
- `mount_table.get_node(path)` - 获取节点信息
- `mount_table.list_mounts()` - 列出挂载点

## 已知问题

1. **编译错误**
   - `batch_handlers.rs` 中有类型不匹配问题
   - 需要修复现有代码后再添加 WebSocket

2. **WebSocket 测试**
   - 需要编译通过后才能测试
   - 需要验证前后端集成

3. **命令扩展**
   - 当前仅支持基础命令
   - 可以添加更多高级命令（如 search, upload, download）

## 下一步工作

1. **修复编译** ⏳
   - 修复 batch_handlers 的类型错误
   - 确保所有模块正常编译

2. **功能测试** ⏳
   - 测试所有命令
   - 验证错误处理
   - 测试并发连接

3. **功能增强** ⏳
   - 添加文件上传支持
   - 添加实时文件更新推送
   - 添加多终端会话支持

## 相关文件

- `crates/evif-rest/src/ws_handlers.rs` - WebSocket 处理器
- `crates/evif-rest/src/routes.rs` - 路由定义
- `crates/evif-rest/src/lib.rs` - 模块导出
- `crates/evif-rest/Cargo.toml` - 依赖配置
- `evif-web/src/components/Terminal.tsx` - 前端终端组件

## 参考资料

- Axum WebSocket 文档: https://docs.rs/axum/latest/axum/extract/ws/index.html
- tokio-tungstenite: https://docs.rs/tokio-tungstenite/
- XTerm.js 文档: https://xtermjs.org/

---

**文档版本**: 1.0
**创建日期**: 2026-01-29
**状态**: ⏳ 实现中，待修复编译错误
