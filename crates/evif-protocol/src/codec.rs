// 编解码器实现

use crate::error::ProtocolResult;
use crate::message::{Message, MessageHeader};
use crate::ProtocolError;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Write};

/// 协议编解码器 trait
pub trait ProtocolCodec: Send + Sync {
    /// 编码消息
    fn encode(&self, msg: &Message) -> ProtocolResult<Vec<u8>>;

    /// 解码消息
    fn decode(&self, data: &[u8]) -> ProtocolResult<Message>;

    /// 编码消息头
    fn encode_header(&self, header: &MessageHeader) -> ProtocolResult<Vec<u8>>;

    /// 解码消息头
    fn decode_header(&self, data: &[u8]) -> ProtocolResult<MessageHeader>;
}

/// MessagePack 编解码器
#[derive(Debug, Clone)]
pub struct MessagePackCodec {
    max_size: usize,
}

impl MessagePackCodec {
    /// 创建新的 MessagePack 编解码器
    pub fn new() -> Self {
        MessagePackCodec {
            max_size: crate::MAX_MESSAGE_SIZE,
        }
    }

    /// 设置最大消息大小
    pub fn with_max_size(mut self, max_size: usize) -> Self {
        self.max_size = max_size;
        self
    }
}

impl Default for MessagePackCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolCodec for MessagePackCodec {
    fn encode(&self, msg: &Message) -> ProtocolResult<Vec<u8>> {
        // 创建消息头
        let header = MessageHeader {
            version: crate::PROTOCOL_VERSION,
            message_type: msg.message_type(),
            size: msg.estimated_size() as u32,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            message_id: Some(uuid::Uuid::new_v4().as_u128() as u64),
            stream_id: None,
            compressed: false,
            checksum: None,
        };

        // 编码消息头
        let header_bytes = self.encode_header(&header)?;

        // 编码消息体
        let mut body_buf = Vec::new();
        msg.serialize(&mut Serializer::new(&mut body_buf))
            .map_err(|e| ProtocolError::SerializationError(e.to_string()))?;

        // 组合头部和主体
        let mut result = Vec::with_capacity(header_bytes.len() + body_buf.len());
        result.extend_from_slice(&header_bytes);
        result.extend_from_slice(&body_buf);

        Ok(result)
    }

    fn decode(&self, data: &[u8]) -> ProtocolResult<Message> {
        if data.len() > self.max_size {
            return Err(ProtocolError::MessageTooLarge {
                size: data.len(),
                max_size: self.max_size,
            });
        }

        // 解码消息体
        let mut de = Deserializer::new(Cursor::new(data));
        let msg = Message::deserialize(&mut de)
            .map_err(|e| ProtocolError::DeserializationError(e.to_string()))?;

        Ok(msg)
    }

    fn encode_header(&self, header: &MessageHeader) -> ProtocolResult<Vec<u8>> {
        let mut buf = Vec::new();

        // 写入魔数
        buf.write_all(crate::PROTOCOL_MAGIC)
            .map_err(|e| ProtocolError::IoError(e.to_string()))?;

        // 编码头部
        header.serialize(&mut Serializer::new(&mut buf))
            .map_err(|e| ProtocolError::SerializationError(e.to_string()))?;

        Ok(buf)
    }

    fn decode_header(&self, data: &[u8]) -> ProtocolResult<MessageHeader> {
        // 检查魔数
        if data.len() < crate::PROTOCOL_MAGIC.len() {
            return Err(ProtocolError::InvalidMessage("数据过短".to_string()));
        }

        let magic = &data[..crate::PROTOCOL_MAGIC.len()];
        if magic != crate::PROTOCOL_MAGIC {
            return Err(ProtocolError::InvalidMagic {
                expected: crate::PROTOCOL_MAGIC.to_vec(),
                actual: magic.to_vec(),
            });
        }

        // 解码头部
        let mut de = Deserializer::new(Cursor::new(&data[crate::PROTOCOL_MAGIC.len()..]));
        let header = MessageHeader::deserialize(&mut de)
            .map_err(|e| ProtocolError::DeserializationError(e.to_string()))?;

        // 验证头部
        header.validate()?;

        Ok(header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codec_ping_request() {
        let codec = MessagePackCodec::new();
        let req = crate::Request::ping();
        let msg = Message::Request(req);

        let encoded = codec.encode(&msg).unwrap();
        assert!(!encoded.is_empty());

        // 验证魔数
        assert_eq!(&encoded[..4], b"EVIF");
    }

    #[test]
    fn test_codec_header() {
        let codec = MessagePackCodec::new();
        let header = MessageHeader::new(crate::MessageType::Request);

        let encoded = codec.encode_header(&header).unwrap();
        assert!(!encoded.is_empty());

        // 验证魔数
        assert_eq!(&encoded[..4], b"EVIF");

        let decoded = codec.decode_header(&encoded).unwrap();
        assert_eq!(decoded.message_type, crate::MessageType::Request);
    }

    #[test]
    fn test_codec_error_handling() {
        let codec = MessagePackCodec::new().with_max_size(100);

        // 测试消息过大
        let large_data = vec![0u8; 200];
        let result: ProtocolResult<Message> = codec.decode(&large_data);
        assert!(matches!(result, Err(ProtocolError::MessageTooLarge { .. })));
    }

    #[test]
    fn test_codec_invalid_magic() {
        let codec = MessagePackCodec::new();
        let invalid_data = b"INVALID";

        let result = codec.decode_header(invalid_data);
        assert!(matches!(result, Err(ProtocolError::InvalidMagic { .. })));
    }

    #[test]
    fn test_codec_pong_response() {
        let codec = MessagePackCodec::new();
        let resp = crate::Response::pong(123);
        let msg = Message::Response(resp);

        // 编码消息
        let encoded = codec.encode(&msg).unwrap();
        assert!(!encoded.is_empty());

        // 先解码头部
        let header = codec.decode_header(&encoded).unwrap();
        assert_eq!(header.message_type, crate::MessageType::Response);

        // 计算主体起始位置（魔数 + MessagePack 编码的头部大小）
        // 由于 MessagePack 头部大小可变，我们重新编码头部来获取大小
        let header_encoded = codec.encode_header(&header).unwrap();
        let body_start = header_encoded.len();

        // 解码消息体
        let body_data = &encoded[body_start..];
        let decoded = codec.decode(body_data).unwrap();
        match decoded {
            Message::Response(r) => {
                assert_eq!(r.request_id, 123);
            }
            _ => panic!("Expected response message"),
        }
    }
}
