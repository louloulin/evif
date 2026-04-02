// WebSocket 处理器 - 支持终端命令和实时通信

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use evif_core::RadixMountTable;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// WebSocket 应用状态
#[derive(Clone)]
pub struct WebSocketState {
    pub mount_table: Arc<RadixMountTable>,
    /// If set, WebSocket connections must present a valid token.
    pub api_keys: Option<Vec<String>>,
}

/// Query parameters for WebSocket upgrade (token-based auth).
#[derive(Debug, Deserialize)]
pub struct WsAuthQuery {
    pub token: Option<String>,
}

/// WebSocket 消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WSMessage {
    /// 命令执行
    #[serde(rename = "command")]
    Command { command: String },
    /// 输出响应
    #[serde(rename = "output")]
    Output { output: String },
    /// 错误响应
    #[serde(rename = "error")]
    Error { message: String },
    /// 文件更新通知
    #[serde(rename = "file_update")]
    FileUpdate { path: String, content: String },
}

/// WebSocket 处理器
pub struct WebSocketHandlers;

impl WebSocketHandlers {
    /// WebSocket 升级处理器 — validates token when API keys are configured.
    pub async fn websocket_handler(
        ws: WebSocketUpgrade,
        Query(query): Query<WsAuthQuery>,
        State(state): State<WebSocketState>,
    ) -> impl IntoResponse {
        // Auth check: if api_keys are configured, require a matching token.
        if let Some(ref keys) = state.api_keys {
            let token = match &query.token {
                Some(t) => t,
                None => {
                    warn!("WebSocket rejected: missing token");
                    return (
                        axum::http::StatusCode::UNAUTHORIZED,
                        "Missing authentication token",
                    )
                        .into_response();
                }
            };
            if !keys.iter().any(|k| k == token) {
                warn!("WebSocket rejected: invalid token");
                return (
                    axum::http::StatusCode::FORBIDDEN,
                    "Invalid authentication token",
                )
                    .into_response();
            }
        }

        ws.on_upgrade(move |socket| Self::handle_socket(socket, state))
    }

    /// 处理 WebSocket 连接
    async fn handle_socket(socket: WebSocket, state: WebSocketState) {
        let (mut sender, mut receiver) = socket.split();

        info!("WebSocket connection established");

        // 发送欢迎消息
        let welcome = WSMessage::Output {
            output: "\r\n\x1b[1;36mEVIF 2.2 - WebSocket Terminal Connected\x1b[0m\r\n".to_string(),
        };
        if let Ok(msg) = serde_json::to_string(&welcome) {
            let _ = sender.send(Message::Text(msg)).await;
        }

        // 发送提示符
        let prompt = WSMessage::Output {
            output: "$ ".to_string(),
        };
        if let Ok(msg) = serde_json::to_string(&prompt) {
            let _ = sender.send(Message::Text(msg)).await;
        }

        // 处理接收到的消息
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    debug!("Received WebSocket message: {}", text);

                    // 解析消息
                    if let Ok(ws_msg) = serde_json::from_str::<WSMessage>(&text) {
                        match ws_msg {
                            WSMessage::Command { command } => {
                                // 执行命令并发送响应
                                let response = Self::execute_command(&command, &state).await;

                                if let Ok(resp_msg) = serde_json::to_string(&response) {
                                    if let Err(e) = sender.send(Message::Text(resp_msg)).await {
                                        error!("Failed to send WebSocket message: {}", e);
                                        break;
                                    }
                                }
                            }
                            _ => {
                                error!("Unsupported WebSocket message type");
                            }
                        }
                    } else {
                        error!("Failed to parse WebSocket message: {}", text);
                    }
                }
                Message::Close(_) => {
                    info!("WebSocket connection closed by client");
                    break;
                }
                _ => {}
            }
        }

        info!("WebSocket handler ended");
    }

    /// 执行终端命令
    async fn execute_command(command: &str, state: &WebSocketState) -> WSMessage {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return WSMessage::Output {
                output: "$ ".to_string(),
            };
        }

        let cmd = parts[0];
        let args = &parts[1..];

        match cmd {
            "help" => {
                let help_text = r#"
Available Commands:
  help              - Show this help message
  clear             - Clear terminal screen
  ls [path]         - List files in directory (default: /)
  cat <path>        - Read file content
  stat <path>       - Get file metadata
  mounts            - List all mount points
  pwd               - Print working directory
  echo <text>       - Echo text back

Examples:
  ls /
  cat /mem/test.txt
  stat /local/
  mounts
"#;
                WSMessage::Output {
                    output: format!("{}\r\n$ ", help_text),
                }
            }
            "clear" => {
                WSMessage::Output {
                    output: "\x1b[2J\x1b[H$ ".to_string(), // ANSI clear screen
                }
            }
            "ls" => {
                let path = if args.is_empty() { "/" } else { args[0] };

                // lookup 得到插件，再 readdir 列出目录
                match state.mount_table.lookup(path).await {
                    Some(plugin) => match plugin.readdir(path).await {
                        Ok(nodes) => {
                            let mut output = String::new();
                            for node in nodes {
                                let icon = if node.is_dir { "📁" } else { "📄" };
                                output.push_str(&format!("{} {}\r\n", icon, node.name));
                            }
                            WSMessage::Output {
                                output: format!("{}\r\n$ ", output),
                            }
                        }
                        Err(e) => WSMessage::Error {
                            message: format!("Failed to list directory: {}", e),
                        },
                    },
                    None => WSMessage::Error {
                        message: format!("Path not found: {}", path),
                    },
                }
            }
            "cat" => {
                if args.is_empty() {
                    return WSMessage::Error {
                        message: "Usage: cat <path>".to_string(),
                    };
                }

                let path = args[0];
                match state.mount_table.lookup(path).await {
                    Some(plugin) => match plugin.read(path, 0, 4096).await {
                        Ok(content) => {
                            let text = String::from_utf8_lossy(&content);
                            WSMessage::Output {
                                output: format!("{}\r\n$ ", text),
                            }
                        }
                        Err(e) => WSMessage::Error {
                            message: format!("Failed to read file: {}", e),
                        },
                    },
                    None => WSMessage::Error {
                        message: format!("Path not found: {}", path),
                    },
                }
            }
            "stat" => {
                if args.is_empty() {
                    return WSMessage::Error {
                        message: "Usage: stat <path>".to_string(),
                    };
                }

                let path = args[0];
                match state.mount_table.lookup(path).await {
                    Some(plugin) => match plugin.stat(path).await {
                        Ok(node) => {
                            let info = format!(
                                "Name: {}\nType: {}\nSize: {} bytes\nModified: {}",
                                node.name,
                                if node.is_dir { "Directory" } else { "File" },
                                node.size,
                                node.modified
                            );
                            WSMessage::Output {
                                output: format!("{}\r\n$ ", info),
                            }
                        }
                        Err(e) => WSMessage::Error {
                            message: format!("Failed to stat: {}", e),
                        },
                    },
                    None => WSMessage::Error {
                        message: format!("Path not found: {}", path),
                    },
                }
            }
            "mounts" => {
                let mounts = state.mount_table.list_mounts().await;
                let mut output = String::from("Mount points:\r\n");
                for mount in mounts {
                    output.push_str(&format!("  -> {}\r\n", mount));
                }
                WSMessage::Output {
                    output: format!("{}\r\n$ ", output),
                }
            }
            "pwd" => WSMessage::Output {
                output: "/\r\n$ ".to_string(),
            },
            "echo" => {
                let text = args.join(" ");
                WSMessage::Output {
                    output: format!("{}\r\n$ ", text),
                }
            }
            _ => WSMessage::Error {
                message: format!(
                    "Unknown command: {}. Type 'help' for available commands.",
                    cmd
                ),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_message_serialization() {
        let msg = WSMessage::Command {
            command: "ls /".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("command"));
        assert!(json.contains("ls /"));
    }

    #[test]
    fn test_ws_message_deserialization() {
        let json = r#"{"type":"command","data":{"command":"ls /"}}"#;
        let msg: WSMessage = serde_json::from_str(json).unwrap();
        match msg {
            WSMessage::Command { command } => {
                assert_eq!(command, "ls /");
            }
            _ => panic!("Expected Command message"),
        }
    }

    #[test]
    fn test_ws_auth_query_with_token() {
        let json = r#"{"token":"sk-test-key"}"#;
        let parsed: WsAuthQuery = serde_json::from_str(json).expect("Failed to parse WsAuthQuery");
        assert_eq!(parsed.token.as_deref(), Some("sk-test-key"));
    }

    #[test]
    fn test_ws_auth_query_missing_token() {
        let json = r#"{}"#;
        let parsed: WsAuthQuery = serde_json::from_str(json).expect("Failed to parse WsAuthQuery");
        assert!(parsed.token.is_none());
    }

    #[test]
    fn test_ws_state_api_keys_validation() {
        let state = WebSocketState {
            mount_table: Arc::new(RadixMountTable::new()),
            api_keys: Some(vec!["key-a".to_string(), "key-b".to_string()]),
        };

        // Valid key
        let keys = state.api_keys.as_ref().unwrap();
        assert!(keys.iter().any(|k| k == "key-a"));
        assert!(keys.iter().any(|k| k == "key-b"));

        // Invalid key
        assert!(!keys.iter().any(|k| k == "key-c"));
    }

    #[test]
    fn test_ws_state_no_auth() {
        let state = WebSocketState {
            mount_table: Arc::new(RadixMountTable::new()),
            api_keys: None,
        };
        assert!(state.api_keys.is_none());
    }
}
