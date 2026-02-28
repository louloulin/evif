# EVIF MCP Server

EVIF的Model Context Protocol (MCP) 服务器,提供17个工具给Claude Desktop和其他MCP客户端。

## 功能特性

✅ **17个MCP工具** - 完整对标AGFS文件系统操作
✅ **3个Prompts** - 文件探索、批量操作、数据分析
✅ **1个Resource** - 根文件系统访问
✅ **stdio传输** - 标准输入输出通信

## 工具列表

### 文件操作
- `evif_ls` - 列出目录文件
- `evif_cat` - 读取文件内容
- `evif_write` - 写入文件内容
- `evif_rm` - 删除文件或目录
- `evif_mv` - 移动/重命名文件
- `evif_cp` - 复制文件
- `evif_stat` - 获取文件信息

### 目录操作
- `evif_mkdir` - 创建目录

### 插件管理
- `evif_mount` - 挂载插件
- `evif_unmount` - 卸载插件
- `evif_mounts` - 列出所有挂载点

### 高级操作
- `evif_grep` - 文件内容搜索
- `evif_health` - 健康检查

### HandleFS
- `evif_open_handle` - 打开文件句柄
- `evif_close_handle` - 关闭文件句柄

## 使用方法

### 1. 启动EVIF REST API服务器

```bash
cd crates/evif-rest
cargo run
```

服务器将在 http://localhost:8080 启动。

### 2. 配置Claude Desktop

在Claude Desktop的配置文件中添加:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "evif": {
      "command": "/path/to/evif/target/debug/evif-mcp",
      "env": {
        "EVIF_URL": "http://localhost:8080"
      }
    }
  }
}
```

### 3. 重启Claude Desktop

重启后,Claude将能够访问所有EVIF文件系统功能。

## 直接运行测试

```bash
# 设置EVIF URL
export EVIF_URL=http://localhost:8080

# 启动MCP服务器(等待stdio输入)
cargo run --bin evif-mcp

# 另一个终端测试
echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | cargo run --bin evif-mcp
```

## MCP工具调用示例

### 列出文件

```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "evif_ls",
    "arguments": {
      "path": "/mem"
    }
  },
  "id": 1
}
```

### 读取文件

```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "evif_cat",
    "arguments": {
      "path": "/mem/test.txt"
    }
  },
  "id": 2
}
```

### 创建目录

```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "evif_mkdir",
    "arguments": {
      "path": "/mem/newdir",
      "mode": 493
    }
  },
  "id": 3
}
```

## 与Claude的对话示例

```
用户: 列出/mem目录的文件
Claude: [调用evif_ls] /mem目录包含: test.txt, data/
```

```
用户: 读取/mem/test.txt文件
Claude: [调用evif_cat] 文件内容是: Hello EVIF!
```

```
用户: 在/mem目录创建一个subdir目录
Claude: [调用evif_mkdir] 目录已创建
```

## 架构

```
┌─────────────┐     stdio     ┌──────────────┐
│ Claude Desktop│◄──────────────►│  EVIF MCP     │
└─────────────┘               │  Server       │
                              └──────────────┘
                                     │
                                     ▼
                              ┌──────────────┐
                              │  EVIF REST    │
                              │  API Server   │
                              └──────────────┘
                                     │
                                     ▼
                              ┌──────────────┐
                              │  EVIF Core    │
                              │  (Plugins)    │
                              └──────────────┘
```

## 开发状态

- ✅ 17个工具全部实现
- ✅ stdio传输实现
- ✅ 与EVIF REST API集成
- ⚠️ 需要完善错误处理
- ⚠️ 需要添加更多测试

## 下一步

- [ ] 实现SSE传输协议
- [ ] 添加资源URI解析
- [ ] 完善Prompt模板
- [ ] 添加更多单元测试
