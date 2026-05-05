// 传输层抽象

use crate::ClientResult;
use evif_protocol::Message;
use std::sync::Arc;

/// 传输错误
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    #[error("send failed: {0}")]
    SendFailed(String),

    #[error("receive failed: {0}")]
    ReceiveFailed(String),

    #[error("invalid message: {0}")]
    InvalidMessage(String),
}

/// 传输层 trait
#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    /// 发送消息并接收响应
    async fn send(&self, message: Message) -> Result<Message, TransportError>;
}

/// 创建传输层
pub async fn create_transport(addr: &str) -> ClientResult<Arc<dyn Transport>> {
    // 简单实现：使用内存传输
    // 实际应用中可以使用 HTTP、gRPC 等
    Ok(Arc::new(MockTransport::new(addr.to_string())))
}

/// Mock 传输层（用于测试）
#[derive(Debug)]
struct MockTransport {
    _addr: String,
}

impl MockTransport {
    fn new(addr: String) -> Self {
        Self { _addr: addr }
    }
}

#[async_trait::async_trait]
impl Transport for MockTransport {
    async fn send(&self, message: Message) -> Result<Message, TransportError> {
        // 简单的 echo 实现
        match message {
            Message::Request(req) => {
                // 返回一个模拟响应
                let response =
                    evif_protocol::Response::success(req.id, evif_protocol::ResponseKind::Pong);
                Ok(Message::Response(response))
            }
            _ => Ok(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_transport() {
        let addr = "localhost:50051";
        let transport = create_transport(addr).await;
        assert!(transport.is_ok());
    }

    #[tokio::test]
    async fn test_create_transport_various_addresses() {
        let addresses = vec![
            "localhost:50051",
            "127.0.0.1:8080",
            "0.0.0.0:3000",
            "example.com:443",
            "[::1]:50051",
        ];
        for addr in addresses {
            let transport = create_transport(addr).await;
            assert!(transport.is_ok(), "Failed for address: {}", addr);
        }
    }

    #[tokio::test]
    async fn test_create_transport_returns_arc() {
        let transport = create_transport("localhost:50051").await.unwrap();
        let _arc_clone = transport.clone();
    }

    // ==================== TransportError Tests ====================

    #[test]
    fn test_transport_error_connection_failed() {
        let err = TransportError::ConnectionFailed("refused".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("connection failed"));
        assert!(msg.contains("refused"));
    }

    #[test]
    fn test_transport_error_send_failed() {
        let err = TransportError::SendFailed("timeout".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("send failed"));
        assert!(msg.contains("timeout"));
    }

    #[test]
    fn test_transport_error_receive_failed() {
        let err = TransportError::ReceiveFailed("closed".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("receive failed"));
        assert!(msg.contains("closed"));
    }

    #[test]
    fn test_transport_error_invalid_message() {
        let err = TransportError::InvalidMessage("bad frame".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("invalid message"));
        assert!(msg.contains("bad frame"));
    }

    #[test]
    fn test_transport_error_debug() {
        let err = TransportError::ConnectionFailed("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("ConnectionFailed"));
    }

    // ==================== MockTransport Tests ====================

    #[test]
    fn test_mock_transport_debug() {
        let mock = MockTransport::new("localhost:8080".to_string());
        let debug = format!("{:?}", mock);
        assert!(debug.contains("MockTransport"));
    }

    // ==================== DummyTransport Tests ====================

    #[tokio::test]
    async fn test_dummy_transport_returns_error() {
        let dummy = DummyTransport;
        let request = evif_protocol::Request {
            id: "test-id".to_string(),
            ..Default::default()
        };
        let result = dummy.send(Message::Request(request)).await;
        assert!(result.is_err());
        match result {
            Err(TransportError::ConnectionFailed(msg)) => {
                assert!(msg.contains("Dummy transport not implemented"));
            }
            _ => panic!("Expected ConnectionFailed error"),
        }
    }
}

// Dummy Transport for blocking client
pub struct DummyTransport;

#[async_trait::async_trait]
impl Transport for DummyTransport {
    async fn send(&self, _message: Message) -> Result<Message, TransportError> {
        Err(TransportError::ConnectionFailed(
            "Dummy transport not implemented".to_string(),
        ))
    }
}
