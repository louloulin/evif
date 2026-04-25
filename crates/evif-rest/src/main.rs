// EVIF REST Server

use clap::Parser;
use evif_rest::{EvifServer, ServerConfig};
use std::path::PathBuf;
use tracing::info;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
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

    // N10b: Log rotation — RollingFileAppender writes JSON logs to logs/ directory,
    // rotating to a new file at midnight. Old files named: evif-rest.2026-04-08.log, etc.
    let file_appender = RollingFileAppender::new(Rotation::DAILY, &log_dir, "evif-rest.log");

    // N10: JSON 结构化日志 — 所有 tracing 输出为 JSON 格式，便于 log aggregation 系统（ELK/Splunk/Loki）解析
    let _ = tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_target(false)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_span_events(FmtSpan::CLOSE)
                // Write JSON to rotating file
                .with_writer(file_appender),
        )
        // Also write human-readable output to stderr (useful in dev / docker logs)
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
