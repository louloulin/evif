// EVIF Protocol - 线协议定义和序列化

pub mod codec;
pub mod error;
pub mod message;
pub mod request;
pub mod response;
pub mod stream;

// Re-export main types
pub use codec::{MessagePackCodec, ProtocolCodec};
pub use error::{ProtocolError, ProtocolResult};
pub use message::{Message, MessageKind, MessageType};
pub use request::{FileOperation, Request, RequestKind, SeekPosition};
pub use response::{Response, ResponseKind, ResponseStatus};
pub use stream::{StreamChunk, StreamHandle};

/// 协议版本
pub const PROTOCOL_VERSION: u32 = 1;

/// 协议魔数（用于验证）
pub const PROTOCOL_MAGIC: &[u8] = b"EVIF";

/// 最大消息大小（16MB）
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// 默认超时（30秒）
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// 协议特性
pub struct ProtocolFeatures;

impl ProtocolFeatures {
    /// 支持的序列化格式
    pub fn supported_formats() -> &'static [&'static str] {
        &["msgpack", "json", "protobuf"]
    }

    /// 支持的压缩算法
    pub fn supported_compression() -> &'static [&'static str] {
        &["none", "zlib", "lz4", "zstd"]
    }

    /// 支持的流式传输
    pub fn supports_streaming() -> bool {
        true
    }

    /// 支持的批量操作
    pub fn supports_batch() -> bool {
        true
    }
}
