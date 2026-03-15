// 协议消息定义

use crate::error::ProtocolResult;
use crate::request::Request;
use crate::response::Response;
use crate::stream::StreamChunk;
use serde::{Deserialize, Serialize};

/// 消息类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    /// 请求消息
    Request,

    /// 响应消息
    Response,

    /// 流式数据块
    Stream,

    /// 错误消息
    Error,

    /// 心跳消息
    Heartbeat,
}

/// 消息种类（内部使用）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    Request,
    Response,
    Stream,
    Error,
    Heartbeat,
}

/// 协议消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// 请求消息
    Request(Request),

    /// 响应消息
    Response(Response),

    /// 流式数据块
    Stream(StreamChunk),

    /// 错误消息
    Error { code: u32, message: String },

    /// 心跳消息
    Heartbeat { timestamp: u64, sequence: u64 },
}

impl Message {
    /// 获取消息类型
    pub fn message_type(&self) -> MessageType {
        match self {
            Message::Request(_) => MessageType::Request,
            Message::Response(_) => MessageType::Response,
            Message::Stream(_) => MessageType::Stream,
            Message::Error { .. } => MessageType::Error,
            Message::Heartbeat { .. } => MessageType::Heartbeat,
        }
    }

    /// 获取消息种类
    pub fn kind(&self) -> MessageKind {
        match self {
            Message::Request(_) => MessageKind::Request,
            Message::Response(_) => MessageKind::Response,
            Message::Stream(_) => MessageKind::Stream,
            Message::Error { .. } => MessageKind::Error,
            Message::Heartbeat { .. } => MessageKind::Heartbeat,
        }
    }

    /// 是否为请求消息
    pub fn is_request(&self) -> bool {
        matches!(self, Message::Request(_))
    }

    /// 是否为响应消息
    pub fn is_response(&self) -> bool {
        matches!(self, Message::Response(_))
    }

    /// 是否为错误消息
    pub fn is_error(&self) -> bool {
        matches!(self, Message::Error { .. })
    }

    /// 获取序列化大小（估算）
    pub fn estimated_size(&self) -> usize {
        // 简化估算，实际可以使用 rmp_serde::encode::to_vec 来精确计算
        match self {
            Message::Request(req) => req.estimated_size(),
            Message::Response(res) => res.estimated_size(),
            Message::Stream(chunk) => chunk.estimated_size(),
            Message::Error { message, .. } => message.len(),
            Message::Heartbeat { .. } => 16,
        }
    }
}

/// 消息头
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageHeader {
    /// 协议版本
    pub version: u32,

    /// 消息类型
    pub message_type: MessageType,

    /// 消息大小
    pub size: u32,

    /// 时间戳
    pub timestamp: u64,

    /// 消息 ID（用于匹配请求/响应）
    pub message_id: Option<u64>,

    /// 流 ID（用于流式传输）
    pub stream_id: Option<u64>,

    /// 压缩标志
    pub compressed: bool,

    /// 校验和
    pub checksum: Option<u32>,
}

impl Default for MessageHeader {
    fn default() -> Self {
        MessageHeader {
            version: crate::PROTOCOL_VERSION,
            message_type: MessageType::Request,
            size: 0,
            timestamp: 0,
            message_id: None,
            stream_id: None,
            compressed: false,
            checksum: None,
        }
    }
}

impl MessageHeader {
    /// 创建新的消息头
    pub fn new(message_type: MessageType) -> Self {
        let mut header = Self::default();
        header.message_type = message_type;
        header
    }

    /// 验证消息头
    pub fn validate(&self) -> ProtocolResult<()> {
        if self.version != crate::PROTOCOL_VERSION {
            return Err(crate::ProtocolError::VersionMismatch {
                expected: crate::PROTOCOL_VERSION,
                actual: self.version,
            });
        }

        if self.size as usize > crate::MAX_MESSAGE_SIZE {
            return Err(crate::ProtocolError::MessageTooLarge {
                size: self.size as usize,
                max_size: crate::MAX_MESSAGE_SIZE,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type() {
        let req = Message::Request(crate::Request::ping());
        assert!(req.is_request());
        assert!(!req.is_response());
        assert!(!req.is_error());
    }

    #[test]
    fn test_message_header_default() {
        let header = MessageHeader::default();
        assert_eq!(header.version, crate::PROTOCOL_VERSION);
    }

    #[test]
    fn test_message_header_validate() {
        let mut header = MessageHeader::new(MessageType::Request);
        assert!(header.validate().is_ok());

        header.version = 999;
        assert!(header.validate().is_err());

        header.version = crate::PROTOCOL_VERSION;
        header.size = (crate::MAX_MESSAGE_SIZE + 1) as u32;
        assert!(header.validate().is_err());
    }

    #[test]
    fn test_error_message() {
        let err = Message::Error {
            code: 404,
            message: "Not Found".to_string(),
        };

        assert!(err.is_error());
        match err {
            Message::Error { code, message } => {
                assert_eq!(code, 404);
                assert_eq!(message, "Not Found");
            }
            _ => panic!("Expected error message"),
        }
    }
}
