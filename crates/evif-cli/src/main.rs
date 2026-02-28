// EVIF CLI - Main entry point

mod cli;
mod commands;
mod repl;
mod script;
mod completer;

use anyhow::Result;
use clap::Parser;
use cli::EvifCli;
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
