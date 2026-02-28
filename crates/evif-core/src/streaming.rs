// Streaming Support - 流式文件支持
//
// 对标 AGFS StreamReader 和 Streamer 接口
// 支持实时数据流（日志、事件流等）

use crate::error::EvifResult;
use async_trait::async_trait;
use std::time::Duration;

/// 流读取器
///
/// 对标 AGFS StreamReader (filesystem.go lines 106-119)
/// 用于从文件流中读取数据块
#[async_trait]
pub trait StreamReader: Send + Sync {
    /// 读取下一个数据块
    ///
    /// # 参数
    /// - `timeout`: 超时时间
    ///
    /// # 返回
    /// - (数据, 是否结束流, 错误)
    ///
    /// # AGFS 对标
    /// ```go
    /// ReadChunk(timeout time.Duration) ([]byte, bool, error)
    /// ```
    async fn read_chunk(&mut self, timeout: Duration) -> EvifResult<(Vec<u8>, bool)>;

    /// 关闭流
    async fn close(&mut self) -> EvifResult<()>;

    /// 检查流是否已结束
    fn is_finished(&self) -> bool;
}

/// 流式文件系统
///
/// 对标 AGFS Streamer (filesystem.go lines 121-128)
/// 支持流式读取的文件系统接口
#[async_trait]
pub trait Streamer: Send + Sync {
    /// 打开文件流
    ///
    /// # 参数
    /// - `path`: 文件路径
    ///
    /// # 返回
    /// 流读取器
    ///
    /// # AGFS 对标
    /// ```go
    /// OpenStream(path string) (StreamReader, error)
    /// ```
    async fn open_stream(&self, path: &str) -> EvifResult<Box<dyn StreamReader>>;
}

/// 内存流读取器实现
///
/// 用于测试和基础流式操作
pub struct MemoryStreamReader {
    data: Vec<u8>,
    position: usize,
    finished: bool,
    chunk_size: usize,
}

impl MemoryStreamReader {
    /// 创建新的内存流读取器
    pub fn new(data: Vec<u8>, chunk_size: usize) -> Self {
        Self {
            data,
            position: 0,
            finished: false,
            chunk_size,
        }
    }

    /// 设置块大小
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }
}

#[async_trait]
impl StreamReader for MemoryStreamReader {
    async fn read_chunk(&mut self, _timeout: Duration) -> EvifResult<(Vec<u8>, bool)> {
        if self.position >= self.data.len() {
            self.finished = true;
            return Ok((Vec::new(), true));
        }

        let remaining = self.data.len() - self.position;
        let to_read = std::cmp::min(self.chunk_size, remaining);

        let chunk = self.data[self.position..self.position + to_read].to_vec();
        self.position += to_read;

        let is_finished = self.position >= self.data.len();
        if is_finished {
            self.finished = true;
        }

        Ok((chunk, is_finished))
    }

    async fn close(&mut self) -> EvifResult<()> {
        self.finished = true;
        Ok(())
    }

    fn is_finished(&self) -> bool {
        self.finished
    }
}

/// 行分隔流读取器
///
/// 按行读取数据流（类似BufRead::lines）
pub struct LineReader {
    reader: Box<dyn StreamReader>,
    buffer: Vec<u8>,
    finished: bool,
}

impl LineReader {
    /// 创建新的行读取器
    pub fn new(reader: Box<dyn StreamReader>) -> Self {
        Self {
            reader,
            buffer: Vec::new(),
            finished: false,
        }
    }

    /// 读取一行
    ///
    /// # 返回
    /// - Option<行数据>（None表示流结束）
    pub async fn read_line(&mut self, timeout: Duration) -> EvifResult<Option<String>> {
        loop {
            // 查找换行符
            if let Some(pos) = self.buffer.iter().position(|&b| b == b'\n') {
                let line_bytes = self.buffer.drain(..=pos).collect::<Vec<_>>();
                // 移除\n
                let line_bytes = if line_bytes.last() == Some(&b'\n') {
                    &line_bytes[..line_bytes.len() - 1]
                } else {
                    &line_bytes[..]
                };

                return Ok(Some(String::from_utf8_lossy(line_bytes).to_string()));
            }

            // 没有换行符，需要读取更多数据
            let (chunk, finished) = self.reader.read_chunk(timeout).await?;

            if chunk.is_empty() && finished {
                // 流结束，返回剩余buffer
                if !self.buffer.is_empty() {
                    let line = String::from_utf8_lossy(&self.buffer).to_string();
                    self.buffer.clear();
                    self.finished = true;
                    return Ok(Some(line));
                }
                self.finished = true;
                return Ok(None);
            }

            self.buffer.extend_from_slice(&chunk);
        }
    }

    /// 检查是否已结束
    pub fn is_finished(&self) -> bool {
        self.finished
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_stream_reader() {
        let data = b"Hello, World!".to_vec();
        let mut reader = MemoryStreamReader::new(data, 5);

        let (chunk1, finished1) = reader.read_chunk(Duration::from_secs(1)).await.unwrap();
        assert_eq!(chunk1, b"Hello");
        assert!(!finished1);

        let (chunk2, finished2) = reader.read_chunk(Duration::from_secs(1)).await.unwrap();
        assert_eq!(chunk2, b", Wor");
        assert!(!finished2);

        let (chunk3, finished3) = reader.read_chunk(Duration::from_secs(1)).await.unwrap();
        assert_eq!(chunk3, b"ld!");
        assert!(finished3);
    }

    #[tokio::test]
    async fn test_line_reader() {
        let data = b"Line 1\nLine 2\nLine 3".to_vec();
        let stream_reader = Box::new(MemoryStreamReader::new(data, 1024));
        let mut reader = LineReader::new(stream_reader);

        let line1 = reader.read_line(Duration::from_secs(1)).await.unwrap();
        assert_eq!(line1, Some("Line 1".to_string()));

        let line2 = reader.read_line(Duration::from_secs(1)).await.unwrap();
        assert_eq!(line2, Some("Line 2".to_string()));

        let line3 = reader.read_line(Duration::from_secs(1)).await.unwrap();
        assert_eq!(line3, Some("Line 3".to_string()));

        let line4 = reader.read_line(Duration::from_secs(1)).await.unwrap();
        assert_eq!(line4, None);
    }

    #[tokio::test]
    async fn test_stream_close() {
        let data = b"Test data".to_vec();
        let mut reader = MemoryStreamReader::new(data, 4);

        reader.close().await.unwrap();
        assert!(reader.is_finished());
    }
}
