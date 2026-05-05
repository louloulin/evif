//! EVIF Client SDK
//!
//! This crate provides a Rust client library for interacting with the EVIF
//! (Everything Is a File) virtual filesystem backend via HTTP REST API.
//!
//! # Overview
//!
//! The `EvifClient` is the main entry point for all operations. It communicates
//! with the EVIF backend over HTTP, providing a type-safe interface for:
//!
//! - File operations: read, write, list, delete, rename, stat
//! - Directory operations: create, remove recursively
//! - Plugin mounts: attach filesystem plugins
//! - Advanced operations: digest/checksum, grep search, permissions
//!
//! # Example
//!
//! ```ignore
//! use evif_client::{ClientConfig, EvifClient};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ClientConfig::default();
//!     let client = EvifClient::new(config).await?;
//!
//!     // List files
//!     let files = client.ls("/").await?;
//!     for f in files {
//!         println!("{}: {} bytes", f.name, f.size);
//!     }
//!
//!     // Health check
//!     let health = client.health().await?;
//!     println!("Status: {} v{}", health.status, health.version);
//!
//!     Ok(())
//! }
//! ```
//!
//! # Error Handling
//!
//! All operations return `ClientResult<T>` which is `Result<T, ClientError>`.
//! See [`ClientError`] for the list of possible error variants.

mod client;

pub use client::{ClientConfig, EvifClient, GrepMatch, HealthInfo, MountInfo};

/// Errors that can occur when using the EVIF client.
///
/// This enum captures all failure modes from network transport to protocol
/// violations and authentication failures.
///
/// # Variants
///
/// - [`Transport`](ClientError::Transport) — Network-level failures (connection refused, DNS errors, etc.)
/// - [`Protocol`](ClientError::Protocol) — Invalid server responses or data corruption
/// - [`AuthFailed`](ClientError::AuthFailed) — Authentication rejected by the server
/// - [`Timeout`](ClientError::Timeout) — Operation exceeded its time limit
/// - [`Io`](ClientError::Io) — Underlying I/O errors (file system, etc.)
///
/// # Example
///
/// ```ignore
/// match client.ls("/").await {
///     Ok(files) => println!("{} files", files.len()),
///     Err(ClientError::Transport(msg)) => eprintln!("Network error: {}", msg),
///     Err(ClientError::Protocol(msg)) => eprintln!("Server said: {}", msg),
///     Err(ClientError::Timeout) => eprintln!("Request timed out"),
///     Err(ClientError::Io(e)) => eprintln!("IO error: {}", e),
///     Err(ClientError::AuthFailed(msg)) => eprintln!("Auth rejected: {}", msg),
/// }
/// ```
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// Network-level failure — e.g., connection refused, DNS lookup failed,
    /// connection reset, or other transport-layer errors.
    #[error("transport error: {0}")]
    Transport(String),

    /// Server returned a response that is malformed, unreadable, or violates
    /// the expected protocol (e.g., missing fields, wrong type, bad timestamp).
    #[error("protocol error: {0}")]
    Protocol(String),

    /// The server rejected our credentials or authentication token.
    #[error("authentication failed: {0}")]
    AuthFailed(String),

    /// The operation did not complete within the configured timeout.
    #[error("timeout: operation took too long")]
    Timeout,

    /// An underlying I/O error from the standard library.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// A shorthand for `Result<T, ClientError>`, used throughout the SDK.
///
/// # Type Parameters
///
/// - `T` — The success value type.
#[allow(dead_code)]
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
