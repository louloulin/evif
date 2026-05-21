// MCP HTTP Handlers - MCP 协议的 HTTP 桥接
//
// 提供 HTTP 端点访问 MCP 工具，复用 evif-mcp crate 的工具定义
//
// 端点:
// - GET  /api/v1/mcp/tools         - 列出所有 MCP 工具
// - POST /api/v1/mcp/call          - 调用 MCP 工具
// - GET  /api/v1/mcp/health        - MCP 服务健康检查

use axum::{
    extract::Extension,
    http::StatusCode,
    response::IntoResponse,
    routing, Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

// MCP types re-exported from evif-mcp
pub use evif_mcp::{Tool, McpServerConfig};

/// MCP HTTP 状态
pub struct McpHttpState {
    pub server: Arc<evif_mcp::EvifMcpServer>,
}

/// MCP 调用请求
#[derive(Debug, Deserialize)]
pub struct McpCallRequest {
    pub tool: String,
    #[serde(default)]
    pub args: HashMap<String, Value>,
}

/// MCP 工具列表响应
#[derive(Debug, Serialize)]
pub struct ToolsResponse {
    pub tools: Vec<Tool>,
    pub count: usize,
}

/// MCP 调用响应
#[derive(Debug, Serialize)]
pub struct CallResponse {
    pub success: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
}

/// 健康检查响应
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub tools_count: usize,
    pub version: String,
}

/// GET /api/v1/mcp/tools - 列出所有 MCP 工具
pub async fn list_tools(
    Extension(state): Extension<Arc<McpHttpState>>,
) -> impl IntoResponse {
    let tools: Vec<Tool> = state.server.list_tools().await;
    let count = tools.len();

    Json(ToolsResponse { tools, count })
}

/// POST /api/v1/mcp/call - 调用 MCP 工具
pub async fn call_tool(
    Extension(state): Extension<Arc<McpHttpState>>,
    Json(req): Json<McpCallRequest>,
) -> Result<Json<CallResponse>, (StatusCode, Json<CallResponse>)> {
    let args = serde_json::to_value(&req.args).unwrap_or(json!({}));
    let result = state.server.call_tool(&req.tool, args).await;
    match result {
        Ok(value) => Ok(Json(CallResponse {
            success: true,
            result: Some(value),
            error: None,
        })),
        Err(e) => {
            tracing::warn!("MCP tool call failed: {} - {}", req.tool, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(CallResponse {
                success: false,
                result: None,
                error: Some(e),
            })))
        }
    }
}

/// GET /api/v1/mcp/health - MCP 服务健康检查
pub async fn health(
    Extension(state): Extension<Arc<McpHttpState>>,
) -> impl IntoResponse {
    let tools: Vec<Tool> = state.server.list_tools().await;
    let count = tools.len();

    Json(HealthResponse {
        status: "healthy".to_string(),
        tools_count: count,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// 创建 MCP HTTP 状态
pub fn create_mcp_state() -> Arc<McpHttpState> {
    let config = McpServerConfig::default();
    let server = evif_mcp::EvifMcpServer::new(config);

    Arc::new(McpHttpState { server })
}

/// 创建 MCP HTTP 路由 (带状态)
pub fn create_mcp_routes(mcp_state: Arc<McpHttpState>) -> Router {
    Router::new()
        .route("/api/v1/mcp/tools", axum::routing::get(list_tools))
        .route("/api/v1/mcp/call", axum::routing::post(call_tool))
        .route("/api/v1/mcp/health", axum::routing::get(health))
        .layer(axum::extract::Extension(mcp_state))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_health() {
        let state = create_mcp_state().await;
        let tools: Vec<Tool> = state.server.list_tools().await;

        assert!(!tools.is_empty(), "MCP tools should be loaded");
    }
}
