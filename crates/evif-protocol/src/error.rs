// 协议错误类型定义

use std::fmt;

/// 协议操作结果类型
pub type ProtocolResult<T> = std::result::Result<T, ProtocolError>;

/// 协议错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    /// 无效的消息格式
    InvalidMessage(String),

    /// 序列化错误
    SerializationError(String),

    /// 反序列化错误
    DeserializationError(String),

    /// 版本不匹配
    VersionMismatch { expected: u32, actual: u32 },

    /// 消息过大
    MessageTooLarge { size: usize, max_size: usize },

    /// 无效的魔数
    InvalidMagic { expected: Vec<u8>, actual: Vec<u8> },

    /// 超时
    Timeout,

    /// 连接中断
    ConnectionLost,

    /// 不支持的操作
    Unsupported(String),

    /// 流错误
    StreamError(String),

    /// 编解码错误
    CodecError(String),

    /// IO 错误
    IoError(String),

    /// 内部错误
    InternalError(String),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolError::InvalidMessage(msg) => write!(f, "无效的消息格式: {}", msg),
            ProtocolError::SerializationError(msg) => write!(f, "序列化错误: {}", msg),
            ProtocolError::DeserializationError(msg) => write!(f, "反序列化错误: {}", msg),
            ProtocolError::VersionMismatch { expected, actual } => {
                write!(f, "版本不匹配: 期望 {}, 实际 {}", expected, actual)
            }
            ProtocolError::MessageTooLarge { size, max_size } => {
                write!(f, "消息过大: {} 字节，最大 {} 字节", size, max_size)
            }
            ProtocolError::InvalidMagic { expected, actual } => {
                write!(f, "无效的魔数: 期望 {:?}, 实际 {:?}", expected, actual)
            }
            ProtocolError::Timeout => write!(f, "操作超时"),
            ProtocolError::ConnectionLost => write!(f, "连接中断"),
            ProtocolError::Unsupported(msg) => write!(f, "不支持的操作: {}", msg),
            ProtocolError::StreamError(msg) => write!(f, "流错误: {}", msg),
            ProtocolError::CodecError(msg) => write!(f, "编解码错误: {}", msg),
            ProtocolError::IoError(msg) => write!(f, "IO 错误: {}", msg),
            ProtocolError::InternalError(msg) => write!(f, "内部错误: {}", msg),
        }
    }
}

impl std::error::Error for ProtocolError {}

// 从标准错误类型转换
impl From<std::io::Error> for ProtocolError {
    fn from(err: std::io::Error) -> Self {
        ProtocolError::IoError(err.to_string())
    }
}

impl From<rmp_serde::encode::Error> for ProtocolError {
    fn from(err: rmp_serde::encode::Error) -> Self {
        ProtocolError::SerializationError(err.to_string())
    }
}

impl From<rmp_serde::decode::Error> for ProtocolError {
    fn from(err: rmp_serde::decode::Error) -> Self {
        ProtocolError::DeserializationError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ProtocolError::InvalidMessage("测试消息".to_string());
        assert_eq!(err.to_string(), "无效的消息格式: 测试消息");

        let err = ProtocolError::Timeout;
        assert_eq!(err.to_string(), "操作超时");
    }

    #[test]
    fn test_version_mismatch() {
        let err = ProtocolError::VersionMismatch {
            expected: 1,
            actual: 2,
        };
        assert!(err.to_string().contains("版本不匹配"));
        assert!(err.to_string().contains("期望 1"));
        assert!(err.to_string().contains("实际 2"));
    }
}
