// EVIF MCP Server - 可执行文件

use evif_mcp::{EvifMcpServer, McpServerConfig};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = McpServerConfig {
        evif_url: std::env::var("EVIF_URL").unwrap_or_else(|_| "http://localhost:8081".to_string()),
        server_name: "evif-mcp".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    tracing::info!("Starting EVIF MCP Server v{}", config.version);
    tracing::info!("Connecting to EVIF at: {}", config.evif_url);

    let server = EvifMcpServer::new(config);

    // 等待初始化
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let tools = server.list_tools().await;
    tracing::info!("Loaded {} tools", tools.len());
    for tool in &tools {
        tracing::info!("  - {}", tool.name);
    }

    // 运行 stdio MCP 服务器
    tracing::info!("MCP server running on stdio...");
    server.run_stdio().await?;

    Ok(())
}
