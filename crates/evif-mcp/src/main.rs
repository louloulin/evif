// EVIF MCP Server - 可执行文件

use clap::Parser;
use evif_mcp::{EvifMcpServer, McpServerConfig};

#[derive(clap::Parser, Debug)]
#[command(
    name = "evif-mcp",
    about = "EVIF MCP Server — Model Context Protocol for AI Agents",
    version
)]
struct Args {
    /// EVIF REST API URL
    #[arg(long, env = "EVIF_URL", value_name = "URL")]
    url: Option<String>,

    /// Server name advertised in MCP protocol
    #[arg(long, env = "EVIF_MCP_SERVER_NAME", value_name = "NAME")]
    server_name: Option<String>,
}

impl Args {
    fn into_config(self) -> McpServerConfig {
        McpServerConfig::from_cli(self.url, self.server_name)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();
    let config = args.into_config();

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
