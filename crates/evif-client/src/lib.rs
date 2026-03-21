// EVIF 客户端 SDK

mod client;

pub use client::{ClientConfig, EvifClient, GrepMatch, HealthInfo, MountInfo};

/// 客户端错误类型
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("transport error: {0}")]
    Transport(String),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("authentication failed: {0}")]
    AuthFailed(String),

    #[error("timeout: operation took too long")]
    Timeout,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type ClientResult<T> = Result<T, ClientError>;
