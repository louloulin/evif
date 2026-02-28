// gRPC 错误处理

use thiserror::Error;

/// gRPC 错误类型
#[derive(Debug, Error)]
pub enum GrpcError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("VFS error: {0}")]
    Vfs(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Address parse error: {0}")]
    AddrParse(#[from] std::net::AddrParseError),
}

pub type GrpcResult<T> = Result<T, GrpcError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = GrpcError::NotFound("test".to_string());
        assert_eq!(err.to_string(), "Not found: test");
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let grpc_err: GrpcError = io_err.into();
        assert!(matches!(grpc_err, GrpcError::Io(_)));
    }
}
