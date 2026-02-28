#!/bin/bash

# EVIF WebSocket 测试服务器 - 独立版本
# 用于在没有完整 EVIF 构建的情况下测试前端

echo "🚀 启动 EVIF WebSocket 测试服务器..."
echo ""

# 检查端口占用
if lsof -Pi :8080 -sTCP:LISTEN -t >/dev/null ; then
    echo "⚠️  端口 8080 已被占用"
    echo "   请先关闭其他服务"
    exit 1
fi

# 创建临时 Python WebSocket 服务器
cat > /tmp/evif_ws_server.py << 'PYTHON_EOF'
#!/usr/bin/env python3
"""
EVIF WebSocket 测试服务器
模拟 EVIF 后端的 WebSocket 功能
"""

import asyncio
import websockets
import json
from datetime import datetime

# 模拟文件系统
MOCK_FS = {
    "/": {
        "name": "",
        "is_dir": True,
        "children": [
            {"name": "mem", "path": "/mem", "is_dir": True},
            {"name": "local", "path": "/local", "is_dir": True},
            {"name": "hello", "path": "/hello", "is_dir": True},
        ]
    },
    "/mem": {
        "name": "mem",
        "is_dir": True,
        "children": [
            {"name": "test.txt", "path": "/mem/test.txt", "is_dir": False, "content": "Hello from EVIF!"},
        ]
    },
    "/local": {
        "name": "local",
        "is_dir": True,
        "children": [
            {"name": "README.md", "path": "/local/README.md", "is_dir": False, "content": "# EVIF Local\n\nThis is a test file."},
        ]
    },
    "/hello": {
        "name": "hello",
        "is_dir": True,
        "children": []
    },
}

async def handle_command(command: str) -> dict:
    """处理命令并返回响应"""
    parts = command.strip().split()
    if not parts:
        return {"type": "output", "data": {"output": "$ "}}

    cmd = parts[0]
    args = parts[1:]

    if cmd == "help":
        help_text = """
Available Commands:
  help              - Show this help message
  clear             - Clear terminal screen
  ls [path]         - List files in directory (default: /)
  cat <path>        - Read file content
  stat <path>       - Get file metadata
  mounts            - List all mount points
  pwd               - Print working directory
  echo <text>       - Echo text back
  date              - Show current date/time

Examples:
  ls /
  cat /mem/test.txt
  stat /local/
  mounts
"""
        return {"type": "output", "data": {"output": help_text + "\r\n$ "}}

    elif cmd == "clear":
        return {"type": "output", "data": {"output": "\x1b[2J\x1b[H$ "}}

    elif cmd == "ls":
        path = args[0] if args else "/"
        if path in MOCK_FS:
            items = MOCK_FS[path].get("children", [])
            output = "\n".join([
                f"{'📁' if item['is_dir'] else '📄'} {item['name']}"
                for item in items
            ])
            return {"type": "output", "data": {"output": f"{output}\r\n$ "}}
        else:
            return {"type": "error", "data": {"message": f"Directory not found: {path}"}}

    elif cmd == "cat":
        if not args:
            return {"type": "error", "data": {"message": "Usage: cat <path>"}}
        path = args[0]
        dir_path = "/".join(path.split("/")[:-1]) or "/"
        filename = path.split("/")[-1]

        if dir_path in MOCK_FS:
            for item in MOCK_FS[dir_path].get("children", []):
                if item["name"] == filename and not item["is_dir"]:
                    return {"type": "output", "data": {"output": f"{item.get('content', '')}\r\n$ "}}

        return {"type": "error", "data": {"message": f"File not found: {path}"}}

    elif cmd == "stat":
        if not args:
            return {"type": "error", "data": {"message": "Usage: stat <path>"}}
        path = args[0]
        if path in MOCK_FS:
            node = MOCK_FS[path]
            info = f"Name: {node['name']}\nType: {'Directory' if node['is_dir'] else 'File'}"
            return {"type": "output", "data": {"output": f"{info}\r\n$ "}}
        else:
            return {"type": "error", "data": {"message": f"Path not found: {path}"}}

    elif cmd == "mounts":
        mounts = "\n".join([
            "  /mem -> MemFsPlugin (In-memory filesystem)",
            "  /local -> LocalFsPlugin (/tmp/evif-local)",
            "  /hello -> HelloFsPlugin (Demo filesystem)",
        ])
        return {"type": "output", "data": {"output": f"Mount points:\n{mounts}\r\n$ "}}

    elif cmd == "pwd":
        return {"type": "output", "data": {"output": "/\r\n$ "}}

    elif cmd == "echo":
        text = " ".join(args)
        return {"type": "output", "data": {"output": f"{text}\r\n$ "}}

    elif cmd == "date":
        now = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        return {"type": "output", "data": {"output": f"{now}\r\n$ "}}

    else:
        return {"type": "error", "data": {"message": f"Unknown command: {cmd}. Type 'help' for available commands."}}

async def handle_websocket(websocket, path):
    """处理 WebSocket 连接"""
    print("✅ WebSocket 连接已建立")

    try:
        # 发送欢迎消息
        welcome = {
            "type": "output",
            "data": {"output": "\r\n\x1b[1;36mEVIF 2.2 - WebSocket Terminal (测试模式)\x1b[0m\r\n"}
        }
        await websocket.send(json.dumps(welcome))

        # 发送提示符
        prompt = {"type": "output", "data": {"output": "$ "}}
        await websocket.send(json.dumps(prompt))

        # 处理消息
        async for message in websocket:
            try:
                data = json.loads(message)
                print(f"📨 收到命令: {data}")

                if data.get("type") == "command":
                    command = data.get("data", {}).get("command", "")
                    response = await handle_command(command)
                    await websocket.send(json.dumps(response))

            except json.JSONDecodeError:
                error = {"type": "error", "data": {"message": "Invalid JSON format"}}
                await websocket.send(json.dumps(error))
            except Exception as e:
                error = {"type": "error", "data": {"message": str(e)}}
                await websocket.send(json.dumps(error))

    except websockets.exceptions.ConnectionClosed:
        print("❌ WebSocket 连接已关闭")
    except Exception as e:
        print(f"❌ 错误: {e}")

async def main():
    """启动 WebSocket 服务器"""
    print("🚀 EVIF WebSocket 测试服务器启动中...")
    print("📍 地址: ws://localhost:8080")
    print("⏹️  按 Ctrl+C 停止服务器")
    print("")

    async with websockets.serve(handle_websocket, "localhost", 8080):
        await asyncio.Future()  # 永远运行

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n\n👋 服务器已停止")
PYTHON_EOF

chmod +x /tmp/evif_ws_server.py

# 检查 Python 和 websockets
if ! command -v python3 >/dev/null 2>&1; then
    echo "❌ Python 3 未安装"
    exit 1
fi

if ! python3 -c "import websockets" 2>/dev/null; then
    echo "📦 安装 websockets..."
    pip3 install websockets
fi

echo "✅ 启动 WebSocket 测试服务器..."
echo ""
python3 /tmp/evif_ws_server.py
