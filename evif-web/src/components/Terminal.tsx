import React, { useEffect, useRef, forwardRef, useImperativeHandle } from 'react';
import { Terminal as XTerm } from '@xterm/xterm';
import '@xterm/xterm/css/xterm.css';
import { getToken } from '@/lib/auth';

export interface TerminalRef {
  write: (data: string) => void;
  sendCommand: (command: string) => void;
}

const Terminal = forwardRef<TerminalRef, {}>((props, ref) => {
  const terminalRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<XTerm | null>(null);

  useImperativeHandle(ref, () => ({
    write: (data: string) => {
      if (xtermRef.current) {
        xtermRef.current.write(data);
      }
    },
    sendCommand: (command: string) => {
      if (websocketRef.current?.readyState === WebSocket.OPEN) {
        websocketRef.current.send(JSON.stringify({
          type: 'command',
          command: command
        }));
      }
    }
  }));

  const websocketRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    if (!terminalRef.current) return;

    // Create XTerm instance
    const xterm = new XTerm({
      theme: {
        background: '#1e1e1e',
        foreground: '#d4d4d4',
        cursor: '#d4d4d4',
        black: '#000000',
        red: '#cd3131',
        green: '#0dbc79',
        yellow: '#e5e510',
        blue: '#2472c8',
        magenta: '#bc3fbc',
        cyan: '#11a8cd',
        white: '#e5e5e5',
        brightBlack: '#666666',
        brightRed: '#f14c4c',
        brightGreen: '#23d18b',
        brightYellow: '#f5f543',
        brightBlue: '#3b8eea',
        brightMagenta: '#d670d6',
        brightCyan: '#29b8db',
        brightWhite: '#ffffff',
      },
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      fontSize: 14,
      cursorBlink: true,
      cursorStyle: 'block',
    });

    // Mount terminal
    xterm.open(terminalRef.current);

    xtermRef.current = xterm;

    // Welcome message
    xterm.write('\r\n\x1b[1;36mEVIF 2.2 - 图文件系统终端\x1b[0m\r\n');
    xterm.write('输入 \x1b[1;33mhelp\x1b[0m 查看可用命令\r\n\r\n');
    xterm.write('$ ');

    // Setup WebSocket connection — connect to same origin so Vite/proxy or reverse proxy can forward to backend
    let wsUrl = (window.location.protocol === 'https:' ? 'wss://' : 'ws://') + window.location.host + '/ws';
    const token = getToken()
    if (token) {
      const suffix = `token=${encodeURIComponent(token)}`
      wsUrl = wsUrl.includes('?') ? `${wsUrl}&${suffix}` : `${wsUrl}?${suffix}`
    }
    const ws = new WebSocket(wsUrl);

    ws.onopen = () => {
      console.log('WebSocket connected');
    };

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        // 根据 ws_handlers.rs，WSMessage 使用 #[serde(tag = "type", content = "data")]
        // 对于 Output { output: String }，序列化格式是: {"type":"output","data":{"output":"..."}}
        // 对于 Error { message: String }，序列化格式是: {"type":"error","data":{"message":"..."}}
        // 对于 Command { command: String }，序列化格式是: {"type":"command","data":{"command":"..."}}
        
        if (data.type === 'output') {
          // data.data 是一个对象，包含 output 字段
          if (data.data && typeof data.data === 'object' && data.data.output) {
            xterm.write(data.data.output);
          } else if (typeof data.data === 'string') {
            // 兼容可能的字符串格式
            xterm.write(data.data);
          } else if (data.output) {
            // 兼容旧格式 {"type":"output","output":"..."}
            xterm.write(data.output);
          } else {
            console.warn('WebSocket output message missing output field:', data);
          }
        } else if (data.type === 'error') {
          const errorMsg = (data.data && data.data.message) || 
                          (typeof data.data === 'string' ? data.data : null) ||
                          data.message || 
                          'Unknown error';
          xterm.write(`\r\n\x1b[1;31mError: ${errorMsg}\x1b[0m\r\n$ `);
        } else if (data.type === 'command') {
          // 命令确认，通常不需要显示
          // console.log('Command received:', data.data);
        } else {
          // 未知格式，尝试提取可能的输出
          console.warn('Unknown WebSocket message format:', data);
          if (data.data && data.data.output) {
            xterm.write(data.data.output);
          } else if (typeof data.data === 'string') {
            xterm.write(data.data);
          } else if (data.output) {
            xterm.write(data.output);
          } else if (typeof event.data === 'string') {
            xterm.write(event.data);
          }
        }
      } catch (e) {
        // JSON解析失败，可能是纯文本输出
        const text = event.data;
        if (typeof text === 'string') {
          // 纯文本，直接写入（xterm会处理ANSI转义序列）
          xterm.write(text);
        } else {
          console.error('Failed to process WebSocket message:', e, event.data);
        }
      }
    };

    ws.onerror = () => {
      xterm.write('\r\n\x1b[2;90m[WebSocket 连接异常]\x1b[0m\r\n');
    };

    ws.onclose = () => {
      xterm.write('\r\n\x1b[2;90m[WebSocket 已断开，终端命令需后端支持]\x1b[0m\r\n');
    };

    websocketRef.current = ws;

    // Handle user input
    let currentLine = '';
    xterm.onData((data) => {
      if (data === '\r' || data === '\n') {
        // Enter key - execute command
        if (currentLine.trim()) {
          if (ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify({
              type: 'command',
              command: currentLine.trim()
            }));
          } else {
            // Fallback to local command handling
            const command = currentLine.trim().split(' ')[0];
            const args = currentLine.trim().split(' ').slice(1);

            if (command === 'clear') {
              xterm.clear();
            } else if (command === 'help') {
              xterm.write('\r\nAvailable commands:\r\n');
              xterm.write('  clear    - Clear terminal\r\n');
              xterm.write('  help     - Show this help\r\n');
              xterm.write('  ls       - List files (via API)\r\n');
              xterm.write('  cat      - Read file (via API)\r\n');
            } else if (command === 'ls') {
              xterm.write('\r\nFetching files...\r\n');
            } else {
              xterm.write(`\r\nCommand not found: ${command}\r\n`);
            }
          }
        }
        currentLine = '';
        xterm.write('\r\n$ ');
      } else if (data === '\u007f' || data === '\b') {
        // Backspace
        if (currentLine.length > 0) {
          currentLine = currentLine.slice(0, -1);
          xterm.write('\b \b');
        }
      } else if (data >= String.fromCharCode(0x20) && data <= String.fromCharCode(0x7E)) {
        // Printable characters
        currentLine += data;
        xterm.write(data);
      }
    });

    // Handle window resize - manually fit terminal
    const handleResize = () => {
      if (terminalRef.current && xtermRef.current) {
        const terminalElement = terminalRef.current;
        // 获取实际可用尺寸
        // terminalElement 是 xterm 的容器div，其父容器是 terminal-wrapper
        const wrapper = terminalElement.parentElement;
        if (!wrapper) return;
        
        // 获取wrapper的实际可用尺寸（减去padding）
        const computedStyle = window.getComputedStyle(wrapper);
        const paddingLeft = parseFloat(computedStyle.paddingLeft) || 0;
        const paddingRight = parseFloat(computedStyle.paddingRight) || 0;
        const paddingTop = parseFloat(computedStyle.paddingTop) || 0;
        const paddingBottom = parseFloat(computedStyle.paddingBottom) || 0;
        
        // 获取terminalElement本身的尺寸（它应该占满wrapper减去padding）
        const availableWidth = terminalElement.clientWidth || (wrapper.clientWidth - paddingLeft - paddingRight);
        const availableHeight = terminalElement.clientHeight || (wrapper.clientHeight - paddingTop - paddingBottom);
        
        // xterm使用字符宽度和高度来计算列数和行数
        // 根据fontSize: 14，字符宽度约8.4px，行高约16.2px
        // 使用更准确的计算
        const charWidth = 8.4;
        const charHeight = 16.2;
        
        // 确保最小尺寸，避免计算错误
        const cols = Math.max(10, Math.floor(availableWidth / charWidth));
        const rows = Math.max(5, Math.floor(availableHeight / charHeight));
        
        // 只有在尺寸有效时才调整
        if (cols > 0 && rows > 0 && availableWidth > 0 && availableHeight > 0) {
          xtermRef.current.resize(cols, rows);
        }
      }
    };

    // 使用 ResizeObserver 监听容器尺寸变化；用 requestAnimationFrame 包裹避免 "ResizeObserver loop" 控制台警告
    let resizeObserver: ResizeObserver | null = null;
    if (terminalRef.current) {
      resizeObserver = new ResizeObserver(() => {
        requestAnimationFrame(handleResize);
      });
      resizeObserver.observe(terminalRef.current);
    }

    window.addEventListener('resize', handleResize);

    // Initial fit - 延迟以确保DOM已渲染
    setTimeout(handleResize, 200);

    // Cleanup
    return () => {
      window.removeEventListener('resize', handleResize);
      if (resizeObserver) {
        resizeObserver.disconnect();
      }
      ws.close();
      xterm.dispose();
    };
  }, []);

  return (
    <div className="terminal-wrapper">
      <div ref={terminalRef} style={{ height: '100%' }} />
    </div>
  );
});

export default Terminal;
