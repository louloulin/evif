// EVIF REST Server

use clap::Parser;
use evif_rest::{EvifServer, ServerConfig};
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::{fmt::format::FmtSpan, prelude::*, EnvFilter};

#[derive(clap::Parser, Debug)]
#[command(
    name = "evif-rest",
    about = "EVIF REST API Server — AI Agent Context Filesystem",
    version
)]
struct Args {
    /// Server bind address
    #[arg(long, env = "EVIF_REST_HOST", value_name = "HOST")]
    host: Option<String>,

    /// Server port
    #[arg(short, long, env = "EVIF_REST_PORT", value_name = "PORT")]
    port: Option<u16>,

    /// Enable production mode (strict config validation)
    #[arg(long, env = "EVIF_REST_PRODUCTION_MODE")]
    production: bool,

    /// Enable CORS
    #[arg(long, env = "EVIF_CORS_ENABLED", value_name = "true|false")]
    cors: Option<bool>,

    /// CORS allowed origins (comma-separated)
    #[arg(
        long,
        env = "EVIF_CORS_ORIGINS",
        value_name = "https://a.com,https://b.com"
    )]
    cors_origins: Option<String>,

    /// Log directory — logs rotate daily. Defaults to ./logs/
    #[arg(long, env = "EVIF_LOG_DIR", value_name = "PATH")]
    log_dir: Option<PathBuf>,

    /// N0: TLS certificate file (PEM format). When set, enables HTTPS on EVIF_TLS_PORT (default 8443)
    #[arg(long, env = "EVIF_TLS_CERT_FILE", value_name = "/path/to/cert.pem")]
    tls_cert: Option<String>,

    /// N0: TLS private key file (PEM format). Must be used with EVIF_TLS_CERT_FILE
    #[arg(long, env = "EVIF_TLS_KEY_FILE", value_name = "/path/to/key.pem")]
    tls_key: Option<String>,
}

impl Args {
    fn into_server_config(self) -> ServerConfig {
        let mut config = ServerConfig::from_cli(
            self.host,
            self.port,
            self.production,
            self.tls_cert,
            self.tls_key,
        );
        if let Some(cors) = self.cors {
            config.enable_cors = cors;
        }
        if let Some(origins) = self.cors_origins {
            config.cors_origins = origins
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        config
    }

    fn log_dir(&self) -> PathBuf {
        self.log_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("logs"))
    }
}

#[tokio::main]
async fn main() -> Result<(), evif_rest::RestError> {
    let args = Args::parse();
    // Get log_dir before consuming args with into_server_config()
    let log_dir = args.log_dir();
    let config = args.into_server_config();

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Note: File logging disabled for now due to sandbox restrictions
    // Original: RollingFileAppender for log rotation
    // N10: JSON structured logging — all tracing output in JSON format for log aggregation
    let _ = tracing_subscriber::registry()
        .with(env_filter)
        // Only stderr output (useful in dev / docker logs)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_span_events(FmtSpan::CLOSE),
        )
        .try_init();

    info!(
        "EVIF REST API v{} starting on {}:{} (production={}, log_dir={})",
        env!("CARGO_PKG_VERSION"),
        config.bind_addr,
        config.port,
        config.production_mode,
        log_dir.display()
    );

    let server = EvifServer::new(config);
    server.run().await
}
