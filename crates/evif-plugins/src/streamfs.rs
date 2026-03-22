// Stream File System Plugin for EVIF
// 对标 AGFS StreamFS: 支持多读者多写者的流式文件系统
// 用途: 日志收集、实时数据流、事件总线、消息管道

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use evif_core::{EvifError, EvifPlugin, EvifResult, FileInfo, WriteFlags};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

/// 流文件配置
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// 每个读者通道缓冲区大小
    pub channel_buffer: usize,
    /// 环形缓冲区大小(历史数据块数)
    pub ring_buffer_size: usize,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            channel_buffer: 100,
            ring_buffer_size: 100,
        }
    }
}

/// 流文件
///
/// 支持多个并发读者和写者
/// 新读者可以从环形缓冲区获取历史数据
struct StreamFile {
    name: String,
    config: StreamConfig,
    created: DateTime<Utc>,
    modified: DateTime<Utc>,
    offset: u64,
    closed: bool,

    // 读者管理
    readers: HashMap<String, mpsc::Sender<Vec<u8>>>,
    next_reader_id: u64,

    // 环形缓冲区(历史数据)
    ring_buffer: Vec<Option<Vec<u8>>>,
    ring_write_pos: usize,
    ring_total_chunks: u64,
}

impl StreamFile {
    fn new(name: String, config: StreamConfig) -> Self {
        let now = Utc::now();
        Self {
            name,
            config: config.clone(),
            created: now,
            modified: now,
            offset: 0,
            closed: false,
            readers: HashMap::new(),
            next_reader_id: 0,
            ring_buffer: vec![None; config.ring_buffer_size],
            ring_write_pos: 0,
            ring_total_chunks: 0,
        }
    }

    /// 注册新读者
    fn register_reader(&mut self) -> (String, mpsc::Receiver<Vec<u8>>) {
        let reader_id = format!("reader_{}", self.next_reader_id);
        self.next_reader_id += 1;

        let (tx, rx) = mpsc::channel(self.config.channel_buffer);

        // 发送历史数据
        for i in 0..self.config.ring_buffer_size {
            let pos = (self.ring_write_pos + self.config.ring_buffer_size - i) % self.config.ring_buffer_size;

            // 从最旧的开始发送
            if let Some(chunk_ref) = self.ring_buffer.get(pos) {
                if let Some(chunk) = chunk_ref {
                    let _ = tx.try_send(chunk.clone());
                }
            }
        }

        self.readers.insert(reader_id.clone(), tx);
        (reader_id, rx)
    }

    /// 注销读者
    fn unregister_reader(&mut self, reader_id: &str) {
        self.readers.remove(reader_id);
    }

    /// 写入数据块
    fn write(&mut self, data: Vec<u8>) -> EvifResult<()> {
        if self.closed {
            return Err(EvifError::InvalidPath("Stream is closed".to_string()));
        }

        // 更新环形缓冲区
        self.ring_buffer[self.ring_write_pos] = Some(data.clone());
        self.ring_write_pos = (self.ring_write_pos + 1) % self.config.ring_buffer_size;
        self.ring_total_chunks += 1;
        self.offset += data.len() as u64;
        self.modified = Utc::now();

        // 发送给所有读者
        let mut dead_readers = Vec::new();
        for (reader_id, tx) in &mut self.readers {
            if tx.try_send(data.clone()).is_err() {
                // 读者通道已满,标记为死读者
                dead_readers.push(reader_id.clone());
            }
        }

        // 清理死读者
        for reader_id in dead_readers {
            self.readers.remove(&reader_id);
        }

        Ok(())
    }

    /// 关闭流
    fn close(&mut self) {
        self.closed = true;
        self.modified = Utc::now();

        // 关闭所有读者通道
        self.readers.clear();
    }

    /// 获取文件信息
    fn stat(&self) -> FileInfo {
        FileInfo {
            name: self.name.clone(),
            size: self.offset,
            mode: 0o644,
            modified: self.modified,
            is_dir: false,
        }
    }
}

/// StreamFS插件
pub struct StreamFsPlugin {
    streams: Arc<RwLock<HashMap<String, StreamFile>>>,
    config: StreamConfig,
}

impl StreamFsPlugin {
    pub fn new() -> Self {
        Self::with_config(StreamConfig::default())
    }

    pub fn with_config(config: StreamConfig) -> Self {
        Self {
            streams: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// 解析路径: /streamfs/{stream_name}
    fn parse_path(&self, path: &str) -> EvifResult<String> {
        let clean_path = path.trim_start_matches('/');

        if clean_path.is_empty() {
            return Err(EvifError::InvalidPath("Stream name required".to_string()));
        }

        Ok(clean_path.to_string())
    }

    /// 创建新流
    async fn create_stream(&self, name: &str) -> EvifResult<()> {
        let mut streams = self.streams.write().await;

        if streams.contains_key(name) {
            return Err(EvifError::InvalidPath(format!("Stream {} already exists", name)));
        }

        let stream = StreamFile::new(name.to_string(), self.config.clone());
        streams.insert(name.to_string(), stream);

        Ok(())
    }

    /// 写入流
    async fn write_stream(&self, name: &str, data: Vec<u8>) -> EvifResult<()> {
        let mut streams = self.streams.write().await;

        if let Some(stream) = streams.get_mut(&name.to_string()) {
            stream.write(data)
        } else {
            Err(EvifError::NotFound(name.to_string()))
        }
    }

    /// 读取流(返回内容和读者ID)
    async fn read_stream(&self, name: &str, size: usize) -> EvifResult<(Vec<u8>, String)> {
        let name_str = name.to_string();

        // 注册临时读者
        let (reader_id, mut rx) = {
            let mut streams = self.streams.write().await;

            if let Some(stream) = streams.get_mut(&name_str) {
                stream.register_reader()
            } else {
                return Err(EvifError::NotFound(name.to_string()));
            }
        };

        // 读取一个数据块
        let data = if let Some(chunk) = rx.recv().await {
            if size > 0 && chunk.len() > size {
                chunk[..size].to_vec()
            } else {
                chunk
            }
        } else {
            Vec::new()
        };

        // 注销读者
        {
            let mut streams = self.streams.write().await;
            if let Some(stream) = streams.get_mut(&name_str) {
                stream.unregister_reader(&reader_id);
            }
        }

        Ok((data, reader_id))
    }

    /// 关闭流
    async fn close_stream(&self, name: &str) -> EvifResult<()> {
        let mut streams = self.streams.write().await;

        if let Some(stream) = streams.get_mut(&name.to_string()) {
            stream.close();
            Ok(())
        } else {
            Err(EvifError::NotFound(name.to_string()))
        }
    }
}

#[async_trait]
impl EvifPlugin for StreamFsPlugin {
    fn name(&self) -> &str {
        "streamfs"
    }

    async fn create(&self, path: &str, _perm: u32) -> EvifResult<()> {
        let name = self.parse_path(path)?;
        self.create_stream(&name).await
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        Err(EvifError::InvalidPath(
            "StreamFS does not support directories".to_string(),
        ))
    }

    async fn read(&self, path: &str, _offset: u64, size: u64) -> EvifResult<Vec<u8>> {
        let name = self.parse_path(path)?;
        let (data, _) = self.read_stream(&name, size as usize).await?;
        Ok(data)
    }

    async fn write(
        &self,
        path: &str,
        data: Vec<u8>,
        _offset: i64,
        _flags: WriteFlags,
    ) -> EvifResult<u64> {
        let name = self.parse_path(path)?;
        let len = data.len() as u64;
        self.write_stream(&name, data).await?;
        Ok(len)
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let clean_path = path.trim_start_matches('/');

        if clean_path.is_empty() || clean_path == "/" {
            // 列出所有流
            let streams = self.streams.read().await;
            let mut entries = Vec::new();

            for (_name, stream) in streams.iter() {
                entries.push(stream.stat());
            }

            entries.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(entries)
        } else {
            // 流不是目录
            Err(EvifError::InvalidPath("Not a directory".to_string()))
        }
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let clean_path = path.trim_start_matches('/');
        if clean_path.is_empty() || clean_path == "/" {
            return Ok(FileInfo {
                name: "/".to_string(),
                size: 0,
                mode: 0o755,
                modified: Utc::now(),
                is_dir: true,
            });
        }

        let name = self.parse_path(path)?;
        let streams = self.streams.read().await;

        if let Some(stream) = streams.get(&name) {
            Ok(stream.stat())
        } else {
            Err(EvifError::NotFound(name.to_string()))
        }
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        let name = self.parse_path(path)?;
        self.close_stream(&name).await?;

        let mut streams = self.streams.write().await;
        streams.remove(&name);
        Ok(())
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        Err(EvifError::InvalidPath(
            "StreamFS does not support rename".to_string(),
        ))
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        // StreamFS 不支持目录结构,remove_all 等同于 remove
        self.remove(path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_streamfs_basic() {
        let plugin = StreamFsPlugin::new();

        // 创建流
        plugin.create("/logs/app", 0o644).await.unwrap();

        // 写入数据
        plugin
            .write("/logs/app", b"Hello, Stream!".to_vec(), 0, WriteFlags::CREATE)
            .await
            .unwrap();

        plugin
            .write("/logs/app", b"Second message".to_vec(), 0, WriteFlags::APPEND)
            .await
            .unwrap();

        // 读取数据
        let data = plugin.read("/logs/app", 0, 1024).await.unwrap();
        assert!(!data.is_empty());

        println!("Stream data: {}", String::from_utf8_lossy(&data));
    }

    #[tokio::test]
    async fn test_streamfs_stat() {
        let plugin = StreamFsPlugin::new();

        plugin.create("/metrics/system", 0o644).await.unwrap();

        let info = plugin.stat("/metrics/system").await.unwrap();
        assert_eq!(info.name, "metrics/system");
        assert_eq!(info.mode, 0o644);
        assert!(!info.is_dir);
    }

    #[tokio::test]
    async fn test_streamfs_readdir() {
        let plugin = StreamFsPlugin::new();

        plugin.create("/stream1", 0o644).await.unwrap();
        plugin.create("/stream2", 0o644).await.unwrap();

        let entries = plugin.readdir("/").await.unwrap();
        assert_eq!(entries.len(), 2);

        println!("Streams: {:?}", entries.iter().map(|e| &e.name).collect::<Vec<_>>());
    }

    #[tokio::test]
    async fn test_streamfs_reopen() {
        let plugin = StreamFsPlugin::new();

        // 创建并写入
        plugin.create("/logs/test", 0o644).await.unwrap();
        plugin
            .write("/logs/test", b"Message 1".to_vec(), 0, WriteFlags::CREATE)
            .await
            .unwrap();
        plugin
            .write("/logs/test", b"Message 2".to_vec(), 0, WriteFlags::APPEND)
            .await
            .unwrap();

        // 读取
        let data1 = plugin.read("/logs/test", 0, 1024).await.unwrap();
        assert!(!data1.is_empty());

        // 再次读取(应该能读取到,因为有环形缓冲区)
        let data2 = plugin.read("/logs/test", 0, 1024).await.unwrap();
        assert!(!data2.is_empty());
    }
}
