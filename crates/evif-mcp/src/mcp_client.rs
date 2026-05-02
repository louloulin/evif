// MCP JSON-RPC Client - 外部 MCP Server 通信
//
// 通过 stdio 与外部 MCP Server 通信
// 实现 JSON-RPC 2.0 协议

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use thiserror::Error;

/// MCP 客户端错误
#[derive(Error, Debug)]
pub enum McpClientError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("MCP error: {0}")]
    McpError(String),

    #[error("Timeout waiting for response")]
    Timeout,

    #[error("Server not initialized")]
    NotInitialized,

    #[error("Process exited: {0}")]
    ProcessExited(i32),
}

/// JSON-RPC 请求
#[derive(Debug, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        static REQUEST_COUNTER: std::sync::OnceLock<RwLock<u64>> = std::sync::OnceLock::new();
        let id = {
            let counter = REQUEST_COUNTER.get_or_init(|| RwLock::new(1));
            let mut guard = counter.write().unwrap();
            let current = *guard;
            *guard = current + 1;
            current
        };

        Self {
            jsonrpc: "2.0",
            id,
            method: method.into(),
            params,
        }
    }
}

/// JSON-RPC 响应
#[derive(Debug, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 错误
#[derive(Debug, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(default)]
    pub data: Option<Value>,
}

/// MCP 服务器能力
#[derive(Debug, Clone, Deserialize)]
pub struct ServerCapabilities {
    #[serde(default)]
    pub tools: Option<ToolsCapability>,
    #[serde(default)]
    pub resources: Option<ResourcesCapability>,
    #[serde(default)]
    pub prompts: Option<PromptsCapability>,
    #[serde(default)]
    pub sampling: Option<SamplingCapability>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolsCapability {}

#[derive(Debug, Clone, Deserialize)]
pub struct ResourcesCapability {
    #[serde(default)]
    pub subscribe: Option<bool>,
    #[serde(default)]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PromptsCapability {}

#[derive(Debug, Clone, Deserialize)]
pub struct SamplingCapability {}

/// MCP 工具
#[derive(Debug, Clone, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
}

/// MCP 资源
#[derive(Debug, Clone, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

/// MCP Prompt
#[derive(Debug, Clone, Deserialize)]
pub struct Prompt {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Option<Vec<PromptArgument>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: Option<bool>,
}

/// MCP 客户端状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClientState {
    Created,
    Initializing,
    Initialized,
    Error,
}

/// MCP JSON-RPC Client
pub struct McpClient {
    #[allow(dead_code)]
    /// 服务器名称
    name: String,
    #[allow(dead_code)]
    /// 子进程
    process: Option<Child>,
    /// 标准输入
    stdin: Option<ChildStdin>,
    /// 标准输出
    stdout: Option<BufReader<ChildStdout>>,
    /// 客户端状态
    state: Arc<RwLock<ClientState>>,
    /// 服务器能力
    capabilities: Arc<RwLock<Option<ServerCapabilities>>>,
    /// 协议版本
    protocol_version: Arc<RwLock<Option<String>>>,
    /// 服务器信息
    server_info: Arc<RwLock<Option<ServerInfo>>>,
    /// 请求超时时间
    timeout: Duration,
    /// 最后活动时间
    last_activity: Arc<RwLock<Instant>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

impl McpClient {
    /// 创建新的 MCP Client
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            process: None,
            stdin: None,
            stdout: None,
            state: Arc::new(RwLock::new(ClientState::Created)),
            capabilities: Arc::new(RwLock::new(None)),
            protocol_version: Arc::new(RwLock::new(None)),
            server_info: Arc::new(RwLock::new(None)),
            timeout: Duration::from_secs(30),
            last_activity: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// 设置超时时间
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// 启动 MCP Server 进程
    pub fn start(
        &mut self,
        command: &str,
        args: &[String],
        env: Option<&HashMap<String, String>>,
    ) -> Result<(), McpClientError> {
        // 构建子进程
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        // 设置环境变量
        if let Some(env_vars) = env {
            for (key, value) in env_vars {
                cmd.env(key, value);
            }
        }

        // 启动进程
        let mut child = cmd.spawn()?;

        let stdin = child.stdin.take().expect("Failed to take stdin");
        let stdout = child.stdout.take().expect("Failed to take stdout");

        self.stdin = Some(stdin);
        self.stdout = Some(BufReader::new(stdout));
        self.process = Some(child);

        // 更新状态
        {
            let mut state = self.state.write().unwrap();
            *state = ClientState::Initializing;
        }

        Ok(())
    }

    /// 初始化 MCP 连接
    pub fn initialize(
        &mut self,
        client_name: &str,
        client_version: &str,
    ) -> Result<ServerCapabilities, McpClientError> {
        // 检查状态
        if self.stdout.is_none() {
            return Err(McpClientError::NotInitialized);
        }

        // 发送 initialize 请求
        let params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "roots": { "listChanged": true },
                "sampling": {}
            },
            "clientInfo": {
                "name": client_name,
                "version": client_version
            }
        });

        let response = self.send_request("initialize", Some(params))?;

        // 解析结果
        let result = response.result.ok_or_else(|| {
            McpClientError::McpError("No result in initialize response".to_string())
        })?;

        // 提取协议版本
        if let Some(version) = result.get("protocolVersion").and_then(|v| v.as_str()) {
            let mut pv = self.protocol_version.write().unwrap();
            *pv = Some(version.to_string());
        }

        // 提取服务器能力
        let capabilities = result
            .get("capabilities")
            .map(|c| serde_json::from_value(c.clone()))
            .transpose()?
            .unwrap_or(ServerCapabilities {
                tools: None,
                resources: None,
                prompts: None,
                sampling: None,
            });

        {
            let mut caps = self.capabilities.write().unwrap();
            *caps = Some(capabilities.clone());
        }

        // 提取服务器信息
        if let Some(info) = result.get("serverInfo").map(|i| serde_json::from_value(i.clone())).transpose()? {
            let mut si = self.server_info.write().unwrap();
            *si = Some(info);
        }

        // 发送 initialized 通知
        self.send_notification("initialized", None)?;

        // 更新状态
        {
            let mut state = self.state.write().unwrap();
            *state = ClientState::Initialized;
        }

        Ok(capabilities)
    }

    /// 列出工具
    pub fn list_tools(&mut self) -> Result<Vec<Tool>, McpClientError> {
        self.require_initialized()?;

        let response = self.send_request("tools/list", None)?;

        let result = response.result.ok_or_else(|| {
            McpClientError::McpError("No result in tools/list response".to_string())
        })?;

        let tools: Vec<Tool> = result
            .get("tools")
            .map(|t| serde_json::from_value(t.clone()))
            .transpose()?
            .unwrap_or_default();

        Ok(tools)
    }

    /// 调用工具
    pub fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: Option<Value>,
    ) -> Result<Value, McpClientError> {
        self.require_initialized()?;

        let params = serde_json::json!({
            "name": tool_name,
            "arguments": arguments.unwrap_or(Value::Object(serde_json::Map::new()))
        });

        let response = self.send_request("tools/call", Some(params))?;

        let result = response.result.ok_or_else(|| {
            McpClientError::McpError("No result in tools/call response".to_string())
        })?;

        Ok(result)
    }

    /// 列出资源
    pub fn list_resources(&mut self) -> Result<Vec<Resource>, McpClientError> {
        self.require_initialized()?;

        let response = self.send_request("resources/list", None)?;

        let result = response.result.ok_or_else(|| {
            McpClientError::McpError("No result in resources/list response".to_string())
        })?;

        let resources: Vec<Resource> = result
            .get("resources")
            .map(|r| serde_json::from_value(r.clone()))
            .transpose()?
            .unwrap_or_default();

        Ok(resources)
    }

    /// 列出 Prompts
    pub fn list_prompts(&mut self) -> Result<Vec<Prompt>, McpClientError> {
        self.require_initialized()?;

        let response = self.send_request("prompts/list", None)?;

        let result = response.result.ok_or_else(|| {
            McpClientError::McpError("No result in prompts/list response".to_string())
        })?;

        let prompts: Vec<Prompt> = result
            .get("prompts")
            .map(|p| serde_json::from_value(p.clone()))
            .transpose()?
            .unwrap_or_default();

        Ok(prompts)
    }

    /// Ping
    pub fn ping(&mut self) -> Result<(), McpClientError> {
        self.require_initialized()?;

        self.send_request("ping", None)?;
        Ok(())
    }

    /// 发送请求
    fn send_request(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<JsonRpcResponse, McpClientError> {
        let stdin = self.stdin.as_mut().ok_or(McpClientError::NotInitialized)?;
        let stdout = self.stdout.as_mut().ok_or(McpClientError::NotInitialized)?;

        // 构建请求
        let request = JsonRpcRequest::new(method, params);
        let request_str = serde_json::to_string(&request)?;
        let line = format!("{}\n", request_str);

        // 发送请求
        stdin.write_all(line.as_bytes())?;
        stdin.flush()?;

        // 更新活动时间
        {
            let mut last = self.last_activity.write().unwrap();
            *last = Instant::now();
        }

        // 读取响应
        let mut response_line = String::new();
        stdout.read_line(&mut response_line)?;

        // 解析响应
        let response: JsonRpcResponse = serde_json::from_str(&response_line)?;

        // 检查错误
        if let Some(error) = response.error {
            return Err(McpClientError::McpError(format!(
                "MCP error {}: {}",
                error.code, error.message
            )));
        }

        Ok(response)
    }

    /// 发送通知（无响应）
    fn send_notification(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<(), McpClientError> {
        let stdin = self.stdin.as_mut().ok_or(McpClientError::NotInitialized)?;

        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params.unwrap_or(Value::Null)
        });

        let notification_str = serde_json::to_string(&notification)?;
        let line = format!("{}\n", notification_str);

        stdin.write_all(line.as_bytes())?;
        stdin.flush()?;

        Ok(())
    }

    /// 检查是否已初始化
    fn require_initialized(&self) -> Result<(), McpClientError> {
        let state = self.state.read().unwrap();
        if *state != ClientState::Initialized {
            return Err(McpClientError::NotInitialized);
        }
        Ok(())
    }

    /// 获取状态
    pub fn get_state(&self) -> ClientState {
        *self.state.read().unwrap()
    }

    /// 获取服务器能力
    pub fn get_capabilities(&self) -> Option<ServerCapabilities> {
        self.capabilities.read().unwrap().clone()
    }

    /// 获取协议版本
    pub fn get_protocol_version(&self) -> Option<String> {
        self.protocol_version.read().unwrap().clone()
    }

    /// 获取服务器信息
    pub fn get_server_info(&self) -> Option<ServerInfo> {
        self.server_info.read().unwrap().clone()
    }

    /// 检查进程是否存活
    pub fn is_alive(&mut self) -> bool {
        if let Some(ref mut process) = self.process {
            process.try_wait().ok().map(|e| e.is_none()).unwrap_or(false)
        } else {
            false
        }
    }

    /// 优雅关闭
    pub fn shutdown(&mut self) -> Result<(), McpClientError> {
        if self.get_state() == ClientState::Initialized {
            let _ = self.send_request("shutdown", None);
        }

        // 终止进程
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
            let _ = process.wait();
        }

        {
            let mut state = self.state.write().unwrap();
            *state = ClientState::Created;
        }

        Ok(())
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

/// MCP 客户端构建器
pub struct McpClientBuilder {
    name: String,
    command: Option<String>,
    args: Vec<String>,
    env: HashMap<String, String>,
    timeout: Duration,
}

impl McpClientBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            command: None,
            args: Vec::new(),
            env: HashMap::new(),
            timeout: Duration::from_secs(30),
        }
    }

    pub fn command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }

    pub fn args(mut self, args: impl IntoIterator<Item = String>) -> Self {
        self.args.extend(args);
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn build(self) -> Result<McpClient, McpClientError> {
        let command = self.command.ok_or_else(|| {
            McpClientError::McpError("Command not specified".to_string())
        })?;

        let mut client = McpClient::new(&self.name).with_timeout(self.timeout);
        client.start(&command, &self.args, Some(&self.env))?;

        Ok(client)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = McpClient::new("test");
        assert_eq!(client.get_state(), ClientState::Created);
    }

    #[test]
    fn test_client_builder() {
        // 测试构建器可以创建客户端
        let result = McpClientBuilder::new("test")
            .command("echo")
            .args(vec!["hello".to_string()])
            .timeout(std::time::Duration::from_secs(1))
            .build();

        // echo 进程可以启动，但不说话 JSON-RPC
        assert!(result.is_ok());

        // 客户端启动后立即尝试 initialize 会失败（因为 echo 不说话 MCP）
        let mut client = result.unwrap();
        let init_result = client.initialize("test", "1.0");
        assert!(init_result.is_err());
    }

    #[test]
    fn test_json_rpc_request() {
        let request = JsonRpcRequest::new("test_method", Some(serde_json::json!({"key": "value"})));
        let json = serde_json::to_string(&request).unwrap();

        assert!(json.contains("\"method\":\"test_method\""));
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":"));
    }

    #[test]
    fn test_json_rpc_response_parsing() {
        let response_json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"tools": []}
        }"#;

        let response: JsonRpcResponse = serde_json::from_str(response_json).unwrap();
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, 1);
        assert!(response.result.is_some());
    }

    #[test]
    fn test_json_rpc_error_parsing() {
        let error_json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32600,
                "message": "Invalid Request"
            }
        }"#;

        let response: JsonRpcResponse = serde_json::from_str(error_json).unwrap();
        assert!(response.error.is_some());

        let error = response.error.unwrap();
        assert_eq!(error.code, -32600);
        assert_eq!(error.message, "Invalid Request");
    }

    #[test]
    fn test_tools_capability_parsing() {
        let json = r#"{
            "name": "test_tool",
            "description": "A test tool",
            "input_schema": {"type": "object"}
        }"#;

        let tool: Tool = serde_json::from_str(json).unwrap();
        assert_eq!(tool.name, "test_tool");
        assert_eq!(tool.description, Some("A test tool".to_string()));
    }

    #[test]
    fn test_resources_capability_parsing() {
        let json = r#"{
            "uri": "file:///test",
            "name": "Test Resource",
            "mimeType": "text/plain"
        }"#;

        let resource: Resource = serde_json::from_str(json).unwrap();
        assert_eq!(resource.uri, "file:///test");
        assert_eq!(resource.name, Some("Test Resource".to_string()));
    }
}
