// EVIF 客户端 SDK

mod client;
mod transport;
mod cache;

pub use client::{EvifClient, ClientConfig, HealthInfo, MountInfo, GrepMatch};
pub use transport::{Transport, TransportError};
pub use cache::ClientCache;

use evif_graph::NodeId;

/// 客户端错误类型
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("transport error: {0}")]
    Transport(#[from] TransportError),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("node not found: {0}")]
    NodeNotFound(NodeId),

    #[error("authentication failed: {0}")]
    AuthFailed(String),

    #[error("timeout: operation took too long")]
    Timeout,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type ClientResult<T> = Result<T, ClientError>;
