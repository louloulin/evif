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
                let response = evif_protocol::Response::success(req.id, evif_protocol::ResponseKind::Pong);
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
}


// Dummy Transport for blocking client
pub struct DummyTransport;

#[async_trait::async_trait]
impl Transport for DummyTransport {
    async fn send(&self, _message: Message) -> Result<Message, TransportError> {
        Err(TransportError::ConnectionFailed("Dummy transport not implemented".to_string()))
    }
}
