// EVIF CLI - Main entry point

use anyhow::Result;
use clap::Parser;
use evif_cli::cli::EvifCli;
use tracing::Level;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    let cli = EvifCli::parse();
    cli.run().await?;

    Ok(())
}
