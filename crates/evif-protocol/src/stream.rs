// 流式传输支持

use serde::{Deserialize, Serialize};

/// 流句柄
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StreamHandle(u64);

impl StreamHandle {
    /// 创建新的流句柄
    pub fn new() -> Self {
        StreamHandle(uuid::Uuid::new_v4().as_u128() as u64)
    }

    /// 从 ID 创建
    pub fn from_id(id: u64) -> Self {
        StreamHandle(id)
    }

    /// 获取 ID
    pub fn id(&self) -> u64 {
        self.0
    }

    /// 是否有效
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl Default for StreamHandle {
    fn default() -> Self {
        Self::new()
    }
}

/// 流数据块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// 流 ID
    pub stream_id: StreamHandle,

    /// 块序号
    pub sequence: u64,

    /// 是否为最后一个块
    pub is_last: bool,

    /// 数据
    pub data: Vec<u8>,

    /// 元数据
    pub metadata: Option<StreamMetadata>,
}

impl StreamChunk {
    /// 创建新的流块
    pub fn new(stream_id: StreamHandle, sequence: u64, data: Vec<u8>) -> Self {
        StreamChunk {
            stream_id,
            sequence,
            is_last: false,
            data,
            metadata: None,
        }
    }

    /// 创建最后一个块
    pub fn last(stream_id: StreamHandle, sequence: u64, data: Vec<u8>) -> Self {
        StreamChunk {
            stream_id,
            sequence,
            is_last: true,
            data,
            metadata: None,
        }
    }

    /// 获取数据大小
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// 是否为空块
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// 估算序列化大小
    pub fn estimated_size(&self) -> usize {
        32 + self.data.len()
    }
}

/// 流元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMetadata {
    /// 总大小
    pub total_size: Option<u64>,

    /// 内容类型
    pub content_type: Option<String>,

    /// 文件名
    pub filename: Option<String>,

    /// 校验和
    pub checksum: Option<String>,

    /// 自定义属性
    pub attributes: Vec<(String, String)>,
}

impl Default for StreamMetadata {
    fn default() -> Self {
        StreamMetadata {
            total_size: None,
            content_type: None,
            filename: None,
            checksum: None,
            attributes: Vec::new(),
        }
    }
}

/// 流方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamDirection {
    /// 上行（客户端到服务器）
    Upload,

    /// 下行（服务器到客户端）
    Download,

    /// 双向
    Bidirectional,
}

/// 流配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// 方向
    pub direction: StreamDirection,

    /// 块大小
    pub chunk_size: usize,

    /// 超时（秒）
    pub timeout: Option<u64>,

    /// 压缩
    pub compression: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        StreamConfig {
            direction: StreamDirection::Download,
            chunk_size: 64 * 1024, // 64KB
            timeout: Some(30),
            compression: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_handle() {
        let handle = StreamHandle::new();
        assert!(handle.is_valid());

        let id = handle.id();
        assert_eq!(StreamHandle::from_id(id).id(), id);
    }

    #[test]
    fn test_stream_chunk() {
        let stream_id = StreamHandle::new();
        let data = vec![1u8, 2, 3, 4];

        let chunk = StreamChunk::new(stream_id, 0, data.clone());
        assert_eq!(chunk.size(), 4);
        assert!(!chunk.is_empty());
        assert_eq!(chunk.is_last, false);

        let last = StreamChunk::last(stream_id, 1, data);
        assert_eq!(last.is_last, true);
    }

    #[test]
    fn test_stream_config() {
        let config = StreamConfig::default();
        assert_eq!(config.chunk_size, 64 * 1024);
        assert_eq!(config.direction, StreamDirection::Download);
    }

    #[test]
    fn test_metadata() {
        let metadata = StreamMetadata {
            total_size: Some(1024),
            content_type: Some("text/plain".to_string()),
            filename: Some("test.txt".to_string()),
            checksum: Some("abc123".to_string()),
            attributes: vec![("key".to_string(), "value".to_string())],
        };

        assert_eq!(metadata.total_size, Some(1024));
        assert_eq!(metadata.content_type, Some("text/plain".to_string()));
    }
}
