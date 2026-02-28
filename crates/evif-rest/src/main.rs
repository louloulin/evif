use evif_rest::{EvifServer, ServerConfig};

#[tokio::main]
async fn main() -> Result<(), evif_rest::RestError> {
    let server = EvifServer::new(ServerConfig::default());
    server.run().await
}

