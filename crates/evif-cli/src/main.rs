// EVIF CLI - Main entry point

use anyhow::Result;
use clap::Parser;
use evif_cli::cli::EvifCli;
use tracing::Level;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with configurable format
    let env = std::env::var("EVIF_LOG_FORMAT").unwrap_or_else(|_| "pretty".to_string());

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(true);

    match env.as_str() {
        "json" => {
            // JSON format for log aggregation (ELK, Loki, etc.)
            subscriber.json().init();
        }
        _ => {
            // Pretty format for development
            subscriber.init();
        }
    }

    let cli = EvifCli::parse();
    cli.run().await?;

    Ok(())
}
