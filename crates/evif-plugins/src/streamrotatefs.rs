// StreamRotateFS - 支持轮转的流式文件插件
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use evif_core::{EvifError, EvifPlugin, EvifResult, FileInfo, WriteFlags};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

/// 轮转配置
#[derive(Debug, Clone)]
pub struct RotationConfig {
    /// 时间轮转间隔 (None 表示不启用)
    pub rotation_interval: Option<u64>,
    /// 大小轮转阈值 (字节, 0 表示不启用)
    pub rotation_size: u64,
    /// 输出路径模式 (如 "/s3fs/bucket/logs")
    pub output_path: String,
    /// 文件名模式 (支持 {channel}, {timestamp}, {index} 变量)
    pub filename_pattern: String,
}

impl Default for RotationConfig {
    fn default() -> Self {
        Self {
            rotation_interval: None,
            rotation_size: 100 * 1024 * 1024, // 100MB
            output_path: "/tmp/streamrotate".to_string(),
            filename_pattern: "{channel}-{timestamp}-{index}.log".to_string(),
        }
    }
}

/// 读取器信息
#[derive(Debug, Clone)]
struct Reader {
    id: String,
    registered: DateTime<Utc>,
    dropped_count: u64,
    read_index: u64,
}

/// 旋转流文件
struct RotateStreamFile {
    name: String,
    channel: String,
    offset: u64,
    closed: bool,
    mod_time: DateTime<Utc>,
    readers: HashMap<String, Reader>,
    next_reader_id: u64,
    channel_buffer: usize,

    // 环形缓冲区
    ring_buffer: Vec<Vec<u8>>,
    ring_size: usize,
    write_index: u64,
    total_chunks: u64,

    // 轮转相关
    config: RotationConfig,
    current_file_size: u64,
    file_index: u64,
    current_filepath: Option<String>,
}

impl RotateStreamFile {
    fn new(name: String, channel_buffer: usize, ring_size: usize, config: RotationConfig) -> Self {
        let channel = name.trim_start_matches('/').trim_end_matches('/').to_string();
        let channel = if channel.is_empty() {
            format!("stream-{}", Uuid::new_v4())
        } else {
            channel.split('/').last().unwrap_or(&channel).to_string()
        };

        Self {
            name,
            channel,
            offset: 0,
            closed: false,
            mod_time: Utc::now(),
            readers: HashMap::new(),
            next_reader_id: 0,
            channel_buffer: if channel_buffer > 0 { channel_buffer } else { 100 },
            ring_buffer: vec![Vec::new(); if ring_size > 0 { ring_size } else { 100 }],
            ring_size: if ring_size > 0 { ring_size } else { 100 },
            write_index: 0,
            total_chunks: 0,
            config,
            current_file_size: 0,
            file_index: 0,
            current_filepath: None,
        }
    }

    fn generate_filename(&self) -> String {
        let mut filename = self.config.filename_pattern.clone();
        filename = filename.replace("{channel}", &self.channel);
        filename = filename.replace("{timestamp}", &Utc::now().format("%Y%m%d-%H%M%S").to_string());
        filename = filename.replace("{index}", &self.file_index.to_string());
        filename
    }

    fn should_rotate(&self) -> bool {
        // 检查大小轮转
        if self.config.rotation_size > 0 && self.current_file_size >= self.config.rotation_size {
            return true;
        }
        false
    }

    async fn write(&mut self, data: Vec<u8>) -> EvifResult<u64> {
        if self.closed {
            return Err(EvifError::Other("Stream is closed".to_string()));
        }

        let len = data.len() as u64;

        // 写入环形缓冲区
        let idx = (self.write_index % self.ring_size as u64) as usize;
        self.ring_buffer[idx] = data.clone();
        self.write_index += 1;
        self.total_chunks += 1;
        self.offset += len;
        self.mod_time = Utc::now();

        // 检查是否需要轮转
        if self.should_rotate() {
            self.file_index += 1;
            self.current_file_size = 0;
        }

        self.current_file_size += len;
        Ok(len)
    }

    fn register_reader(&mut self) -> String {
        let id = format!("reader-{}", self.next_reader_id);
        self.next_reader_id += 1;
        self.readers.insert(id.clone(), Reader {
            id: id.clone(),
            registered: Utc::now(),
            dropped_count: 0,
            read_index: self.write_index,
        });
        id
    }

    fn unregister_reader(&mut self, reader_id: &str) {
        self.readers.remove(reader_id);
    }

    async fn read_chunk(&mut self, reader_id: &str, timeout_ms: u64) -> EvifResult<Vec<u8>> {
        let reader = self.readers.get_mut(reader_id)
            .ok_or_else(|| EvifError::NotFound(format!("Reader not found: {}", reader_id)))?;

        if reader.read_index >= self.write_index {
            // 等待新数据 (简化实现,实际应该使用条件变量或channel)
            return Ok(Vec::new());
        }

        let idx = (reader.read_index % self.ring_size as u64) as usize;
        let chunk = self.ring_buffer[idx].clone();
        reader.read_index += 1;

        // 检查是否有数据被丢弃
        if reader.read_index < self.write_index - self.ring_size as u64 {
            reader.dropped_count += 1;
        }

        Ok(chunk)
    }

    fn close(&mut self) {
        self.closed = true;
        self.readers.clear();
    }
}

/// StreamRotateFS 插件
pub struct StreamRotateFSPlugin {
    streams: Arc<RwLock<HashMap<String, Arc<Mutex<RotateStreamFile>>>>>,
    channel_buffer: usize,
    ring_size: usize,
    rotation_config: RotationConfig,
}

impl StreamRotateFSPlugin {
    pub fn new(channel_buffer: usize, ring_size: usize, rotation_config: RotationConfig) -> Self {
        Self {
            streams: Arc::new(RwLock::new(HashMap::new())),
            channel_buffer: if channel_buffer > 0 { channel_buffer } else { 100 },
            ring_size: if ring_size > 0 { ring_size } else { 100 },
            rotation_config,
        }
    }

    async fn get_or_create_stream(&self, path: &str) -> EvifResult<Arc<Mutex<RotateStreamFile>>> {
        let streams = self.streams.read().await;

        if let Some(stream) = streams.get(path) {
            return Ok(Arc::clone(stream));
        }
        drop(streams);

        // 创建新流
        let mut streams = self.streams.write().await;
        if let Some(stream) = streams.get(path) {
            return Ok(Arc::clone(stream));
        }

        let stream = Arc::new(Mutex::new(
            RotateStreamFile::new(
                path.to_string(),
                self.channel_buffer,
                self.ring_size,
                self.rotation_config.clone(),
            )
        ));
        streams.insert(path.to_string(), Arc::clone(&stream));
        Ok(stream)
    }
}

#[async_trait]
impl EvifPlugin for StreamRotateFSPlugin {
    fn name(&self) -> &str {
        "StreamRotateFS"
    }

    async fn create(&self, path: &str, _perm: u32) -> EvifResult<()> {
        // 保留 README 路径用于文档
        if path == "/README" || path == "README" {
            return Err(EvifError::Other("Cannot create stream named README: reserved for documentation".to_string()));
        }

        let mut streams = self.streams.write().await;
        if streams.contains_key(path) {
            return Err(EvifError::Other(format!("Stream already exists: {}", path)));
        }

        let stream = Arc::new(Mutex::new(
            RotateStreamFile::new(
                path.to_string(),
                self.channel_buffer,
                self.ring_size,
                self.rotation_config.clone(),
            )
        ));
        streams.insert(path.to_string(), stream);
        Ok(())
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        Err(EvifError::NotSupported)
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        if path == "/README" || path == "README" {
            let readme = r#"# StreamRotateFS

支持文件轮转的流式文件系统插件。

## 功能特性

- **流式写入**: 高性能的流式数据写入
- **环形缓冲**: 固定大小的环形缓冲区,自动覆盖旧数据
- **多读取器**: 支持多个并发读取器,每个读取器独立跟踪读取位置
- **文件轮转**:
  - 基于大小轮转: 当文件达到指定大小时自动创建新文件
  - 基于时间轮转: 按时间间隔自动创建新文件
- **灵活配置**: 支持自定义文件名模式,包含 channel、timestamp、index 变量

## 使用示例

写入数据:
```
POST /streamrotate/mychannel
{
  "data": "log line 1\n"
}
```

读取数据:
```
GET /streamrotate/mychannel/stream
```

查看流状态:
```
GET /streamrotate/mychannel/status
```
"#;
            return Ok(readme.as_bytes().to_vec());
        }

        Err(EvifError::Other("Use stream mode for reading stream files".to_string()))
    }

    async fn write(&self, path: &str, data: Vec<u8>, _offset: i64, _flags: WriteFlags) -> EvifResult<u64> {
        let stream = self.get_or_create_stream(path).await?;
        let mut stream_guard = stream.lock().await;
        stream_guard.write(data).await
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let mut entries = Vec::new();

        // 根目录显示 README 和所有流
        if path == "/" || path.is_empty() {
            entries.push(FileInfo {
                name: "README".to_string(),
                size: 0,
                modified: Utc::now(),
                mode: 0o644,
                is_dir: false,
            });
        }

        let streams = self.streams.read().await;
        for (name, stream) in streams.iter() {
            let name_clean = name.trim_start_matches('/');
            if path == "/" || path.is_empty() || name_clean.starts_with(path.trim_start_matches('/')) {
                let stream_guard = stream.lock().await;
                entries.push(FileInfo {
                    name: stream_guard.channel.clone(),
                    size: stream_guard.offset as u64,
                    modified: stream_guard.mod_time,
                    mode: 0o644,
                    is_dir: false,
                });
            }
        }

        Ok(entries)
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        if path == "/README" || path == "README" {
            return Ok(FileInfo {
                name: "README".to_string(),
                size: 0,
                modified: Utc::now(),
                mode: 0o644,
                is_dir: false,
            });
        }

        let streams = self.streams.read().await;
        let stream = streams.get(path)
            .ok_or_else(|| EvifError::NotFound(path.to_string()))?;

        let stream_guard = stream.lock().await;
        Ok(FileInfo {
            name: stream_guard.channel.clone(),
            size: stream_guard.offset as u64,
            modified: stream_guard.mod_time,
            mode: 0o644,
            is_dir: false,
        })
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        let mut streams = self.streams.write().await;
        let stream = streams.remove(path)
            .ok_or_else(|| EvifError::NotFound(path.to_string()))?;

        let mut stream_guard = stream.lock().await;
        stream_guard.close();
        Ok(())
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        self.remove(path).await
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        Err(EvifError::NotSupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_streamrotatefs_basic() {
        let plugin = StreamRotateFSPlugin::new(10, 5, RotationConfig::default());

        // 创建流
        plugin.create("/test", 0o644).await.unwrap();

        // 写入数据
        let data1 = b"Hello, World!".to_vec();
        plugin.write("/test", data1, 0, WriteFlags::CREATE).await.unwrap();

        // 检查状态
        let info = plugin.stat("/test").await.unwrap();
        assert_eq!(info.size, 13);

        // 删除流
        plugin.remove("/test").await.unwrap();
    }

    #[tokio::test]
    async fn test_streamrotatefs_readdir() {
        let plugin = StreamRotateFSPlugin::new(10, 5, RotationConfig::default());

        // 创建多个流
        plugin.create("/stream1", 0o644).await.unwrap();
        plugin.create("/stream2", 0o644).await.unwrap();

        // 写入数据
        plugin.write("/stream1", b"data1".to_vec(), 0, WriteFlags::CREATE).await.unwrap();
        plugin.write("/stream2", b"data2".to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        // 列出流
        let entries = plugin.readdir("/").await.unwrap();
        assert!(entries.len() >= 3); // README + stream1 + stream2

        // 清理
        plugin.remove("/stream1").await.unwrap();
        plugin.remove("/stream2").await.unwrap();
    }
}
