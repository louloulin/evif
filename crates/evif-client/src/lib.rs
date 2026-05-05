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

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ClientError Variant Tests ====================

    #[test]
    fn test_client_error_transport_display() {
        let err = ClientError::Transport("connection refused".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("transport error"));
        assert!(msg.contains("connection refused"));
    }

    #[test]
    fn test_client_error_protocol_display() {
        let err = ClientError::Protocol("invalid response".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("protocol error"));
        assert!(msg.contains("invalid response"));
    }

    #[test]
    fn test_client_error_auth_failed_display() {
        let err = ClientError::AuthFailed("bad token".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("authentication failed"));
        assert!(msg.contains("bad token"));
    }

    #[test]
    fn test_client_error_timeout_display() {
        let err = ClientError::Timeout;
        let msg = format!("{}", err);
        assert!(msg.contains("timeout"));
    }

    #[test]
    fn test_client_error_io_from_std_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = ClientError::Io(io_err);
        let msg = format!("{}", err);
        assert!(msg.contains("file not found"));
    }

    #[test]
    fn test_client_error_debug() {
        let err = ClientError::Transport("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("Transport"));
    }

    // ==================== ClientResult Type Alias Tests ====================

    #[test]
    fn test_client_result_ok() {
        let result: ClientResult<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_client_result_err_transport() {
        let result: ClientResult<String> = Err(ClientError::Transport("fail".to_string()));
        assert!(result.is_err());
        match result {
            Err(ClientError::Transport(msg)) => assert_eq!(msg, "fail"),
            _ => panic!("Expected Transport error"),
        }
    }

    #[test]
    fn test_client_result_err_protocol() {
        let result: ClientResult<()> = Err(ClientError::Protocol("bad data".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_client_result_err_timeout() {
        let result: ClientResult<()> = Err(ClientError::Timeout);
        assert!(result.is_err());
    }

    // ==================== Re-export Tests ====================

    #[test]
    fn test_re_exports_are_accessible() {
        // These types are re-exported from lib.rs via pub use
        let _config: ClientConfig = ClientConfig::default();
        let _health: HealthInfo = HealthInfo {
            status: "ok".into(),
            version: "1.0".into(),
            uptime: 0,
        };
        let _mount: MountInfo = MountInfo {
            plugin: "test".into(),
            path: "/test".into(),
        };
        let _grep: GrepMatch = GrepMatch {
            path: "/file".into(),
            line: 1,
            content: "match".into(),
        };
    }

    #[test]
    fn test_re_exported_config_matches_client_config() {
        // Verify the re-exported ClientConfig is the same type
        let config = ClientConfig::default();
        let client::ClientConfig {
            request_timeout,
            base_url,
            timeout,
        } = config;
        assert_eq!(request_timeout, 30);
        assert_eq!(base_url, "http://localhost:8081");
        assert_eq!(timeout, std::time::Duration::from_secs(30));
    }

    #[test]
    fn test_client_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ClientError>();
    }
}
