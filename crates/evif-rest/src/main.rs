use evif_rest::{EvifServer, ServerConfig};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), evif_rest::RestError> {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .try_init();

    let server = EvifServer::new(ServerConfig::default());
    server.run().await
}
