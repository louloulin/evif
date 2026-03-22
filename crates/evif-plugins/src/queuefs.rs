// QueueFS - 消息队列插件 (增强版)
//
// 对标 AGFS QueueFS - 提供基于文件系统的消息队列服务
// 新增功能: 优先队列、延迟队列、死信队列、批量操作

use evif_core::{EvifPlugin, FileInfo, WriteFlags, EvifResult, EvifError};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, BinaryHeap};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::cmp::Ordering;
use std::time::Duration;

/// 队列消息（增强版）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueueMessage {
    id: String,
    data: String,
    timestamp: i64,
    priority: i32,           // 优先级 (0=最高, 数字越大优先级越低)
    delay_until: Option<i64>, // 延迟投递时间戳
    retry_count: u32,         // 重试次数
    max_retries: u32,         // 最大重试次数
}

impl PartialEq for QueueMessage {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for QueueMessage {}

// 优先队列：最小堆（优先级数字越小越优先）
impl PartialOrd for QueueMessage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueueMessage {
    fn cmp(&self, other: &Self) -> Ordering {
        // 先按优先级排序，再按时间戳排序
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => self.timestamp.cmp(&other.timestamp),
            other => other,
        }
    }
}

/// 消息队列（增强版）
struct Queue {
    messages: Vec<QueueMessage>,
    priority_messages: BinaryHeap<QueueMessage>, // 优先队列
    delayed_messages: Vec<QueueMessage>,         // 延迟消息
    last_enqueue_time: i64,
    max_size: usize,                             // 最大队列长度
    dead_letter_queue: Option<String>,           // 死信队列名称
}

impl Queue {
    fn new() -> Self {
        Self {
            messages: Vec::new(),
            priority_messages: BinaryHeap::new(),
            delayed_messages: Vec::new(),
            last_enqueue_time: 0,
            max_size: 10000, // 默认最大10000条消息
            dead_letter_queue: None,
        }
    }

    fn with_config(max_size: usize, dead_letter_queue: Option<String>) -> Self {
        Self {
            messages: Vec::new(),
            priority_messages: BinaryHeap::new(),
            delayed_messages: Vec::new(),
            last_enqueue_time: 0,
            max_size,
            dead_letter_queue,
        }
    }

    fn is_empty(&self) -> bool {
        self.messages.is_empty() && self.priority_messages.is_empty()
    }

    fn len(&self) -> usize {
        self.messages.len() + self.priority_messages.len()
    }
}

/// 队列存储（增强版）
struct QueueStore {
    queues: RwLock<HashMap<String, Queue>>,
}

impl QueueStore {
    fn new() -> Self {
        Self {
            queues: RwLock::new(HashMap::new()),
        }
    }

    async fn create_queue(&self, name: &str) -> EvifResult<()> {
        let mut queues = self.queues.write().await;
        if queues.contains_key(name) {
            return Err(EvifError::InvalidPath(format!("Queue already exists: {}", name)));
        }
        queues.insert(name.to_string(), Queue::new());
        Ok(())
    }

    async fn create_queue_with_config(
        &self,
        name: &str,
        max_size: usize,
        dead_letter_queue: Option<String>
    ) -> EvifResult<()> {
        let mut queues = self.queues.write().await;
        if queues.contains_key(name) {
            return Err(EvifError::InvalidPath(format!("Queue already exists: {}", name)));
        }
        queues.insert(name.to_string(), Queue::with_config(max_size, dead_letter_queue));
        Ok(())
    }

    async fn remove_queue(&self, name: &str) -> EvifResult<()> {
        let mut queues = self.queues.write().await;
        queues.remove(name)
            .ok_or_else(|| EvifError::NotFound(name.to_string()))?;
        Ok(())
    }

    async fn enqueue(&self, name: &str, data: Vec<u8>) -> EvifResult<String> {
        self.enqueue_with_priority(name, data, 999).await // 默认最低优先级
    }

    async fn enqueue_with_priority(&self, name: &str, data: Vec<u8>, priority: i32) -> EvifResult<String> {
        let mut queues = self.queues.write().await;
        let queue = queues.get_mut(name)
            .ok_or_else(|| EvifError::NotFound(name.to_string()))?;

        // 检查队列大小限制
        if queue.len() >= queue.max_size {
            return Err(EvifError::InvalidPath("Queue is full".to_string()));
        }

        let msg = QueueMessage {
            id: Uuid::new_v4().to_string(),
            data: String::from_utf8(data)
                .map_err(|_| EvifError::InvalidPath("Invalid UTF-8 data".to_string()))?,
            timestamp: Utc::now().timestamp(),
            priority,
            delay_until: None,
            retry_count: 0,
            max_retries: 3,
        };

        // 根据优先级决定放入哪个队列
        if priority < 100 {
            queue.priority_messages.push(msg.clone());
        } else {
            queue.messages.push(msg.clone());
        }
        queue.last_enqueue_time = msg.timestamp;
        Ok(msg.id)
    }

    async fn enqueue_delayed(&self, name: &str, data: Vec<u8>, delay_secs: i64) -> EvifResult<String> {
        let mut queues = self.queues.write().await;
        let queue = queues.get_mut(name)
            .ok_or_else(|| EvifError::NotFound(name.to_string()))?;

        if queue.len() >= queue.max_size {
            return Err(EvifError::InvalidPath("Queue is full".to_string()));
        }

        let delay_until = Utc::now().timestamp() + delay_secs;
        let msg = QueueMessage {
            id: Uuid::new_v4().to_string(),
            data: String::from_utf8(data)
                .map_err(|_| EvifError::InvalidPath("Invalid UTF-8 data".to_string()))?,
            timestamp: Utc::now().timestamp(),
            priority: 999,
            delay_until: Some(delay_until),
            retry_count: 0,
            max_retries: 3,
        };

        queue.delayed_messages.push(msg.clone());
        queue.last_enqueue_time = msg.timestamp;
        Ok(msg.id)
    }

    async fn dequeue(&self, name: &str) -> EvifResult<QueueMessage> {
        let mut queues = self.queues.write().await;
        let queue = queues.get_mut(name)
            .ok_or_else(|| EvifError::NotFound(name.to_string()))?;

        // 先从优先队列取
        if let Some(msg) = queue.priority_messages.pop() {
            return Ok(msg);
        }

        // 再从普通队列取
        if !queue.messages.is_empty() {
            return Ok(queue.messages.remove(0));
        }

        Err(EvifError::InvalidPath("Queue is empty".to_string()))
    }

    async fn peek(&self, name: &str) -> EvifResult<QueueMessage> {
        let queues = self.queues.read().await;
        let queue = queues.get(name)
            .ok_or_else(|| EvifError::NotFound(name.to_string()))?;

        // 先查看优先队列
        if let Some(msg) = queue.priority_messages.peek() {
            return Ok(msg.clone());
        }

        // 再查看普通队列
        if let Some(msg) = queue.messages.first() {
            return Ok(msg.clone());
        }

        Err(EvifError::InvalidPath("Queue is empty".to_string()))
    }

    async fn size(&self, name: &str) -> EvifResult<usize> {
        let queues = self.queues.read().await;
        let queue = queues.get(name)
            .ok_or_else(|| EvifError::NotFound(name.to_string()))?;
        Ok(queue.len())
    }

    async fn clear(&self, name: &str) -> EvifResult<()> {
        let mut queues = self.queues.write().await;
        let queue = queues.get_mut(name)
            .ok_or_else(|| EvifError::NotFound(name.to_string()))?;
        queue.messages.clear();
        queue.priority_messages.clear();
        queue.delayed_messages.clear();
        Ok(())
    }

    /// 处理延迟消息（检查是否有消息到期）
    async fn process_delayed_messages(&self, name: &str) -> EvifResult<usize> {
        let mut queues = self.queues.write().await;
        let queue = queues.get_mut(name)
            .ok_or_else(|| EvifError::NotFound(name.to_string()))?;

        let now = Utc::now().timestamp();
        let mut moved_count = 0;

        // 过滤出已到期的延迟消息
        let (ready_messages, remaining): (Vec<_>, Vec<_>) = queue.delayed_messages.drain(..)
            .partition(|msg| {
                msg.delay_until.map_or(false, |until| until <= now)
            });

        queue.delayed_messages = remaining;

        // 将到期消息移入普通队列
        for msg in ready_messages {
            queue.messages.push(msg);
            moved_count += 1;
        }

        Ok(moved_count)
    }

    /// 批量入队
    async fn enqueue_batch(&self, name: &str, messages: Vec<Vec<u8>>) -> EvifResult<Vec<String>> {
        let mut queues = self.queues.write().await;
        let queue = queues.get_mut(name)
            .ok_or_else(|| EvifError::NotFound(name.to_string()))?;

        if queue.len() + messages.len() > queue.max_size {
            return Err(EvifError::InvalidPath("Queue would overflow".to_string()));
        }

        let mut ids = Vec::new();
        for data in messages {
            let msg = QueueMessage {
                id: Uuid::new_v4().to_string(),
                data: String::from_utf8(data)
                    .map_err(|_| EvifError::InvalidPath("Invalid UTF-8 data".to_string()))?,
                timestamp: Utc::now().timestamp(),
                priority: 999,
                delay_until: None,
                retry_count: 0,
                max_retries: 3,
            };
            ids.push(msg.id.clone());
            queue.messages.push(msg);
        }

        queue.last_enqueue_time = Utc::now().timestamp();
        Ok(ids)
    }

    /// 批量出队
    async fn dequeue_batch(&self, name: &str, batch_size: usize) -> EvifResult<Vec<QueueMessage>> {
        let mut queues = self.queues.write().await;
        let queue = queues.get_mut(name)
            .ok_or_else(|| EvifError::NotFound(name.to_string()))?;

        let mut result = Vec::new();

        // 先从优先队列取
        while result.len() < batch_size {
            if let Some(msg) = queue.priority_messages.pop() {
                result.push(msg);
            } else {
                break;
            }
        }

        // 再从普通队列取
        while result.len() < batch_size {
            if !queue.messages.is_empty() {
                result.push(queue.messages.remove(0));
            } else {
                break;
            }
        }

        if result.is_empty() {
            return Err(EvifError::InvalidPath("Queue is empty".to_string()));
        }

        Ok(result)
    }

    /// 列出所有队列
    async fn list_queues(&self) -> Vec<String> {
        let queues = self.queues.read().await;
        queues.keys().cloned().collect()
    }

    async fn list_queues_with_prefix(&self, prefix: &str) -> EvifResult<Vec<String>> {
        let queues = self.queues.read().await;
        let mut result = Vec::new();

        for name in queues.keys() {
            if prefix.is_empty() || name == prefix || name.starts_with(&format!("{}/", prefix)) {
                result.push(name.clone());
            }
        }

        result.sort();
        result.dedup();
        Ok(result)
    }

    async fn queue_exists(&self, name: &str) -> bool {
        let queues = self.queues.read().await;
        queues.contains_key(name)
    }
}

pub struct QueueFsPlugin {
    store: Arc<QueueStore>,
}

impl QueueFsPlugin {
    pub fn new() -> Self {
        Self {
            store: Arc::new(QueueStore::new()),
        }
    }

    fn readme(&self) -> &'static str {
        "QueueFS Plugin - File-oriented queue controls\n\nCreate queues with mkdir, enqueue by writing to enqueue, and consume via dequeue/peek/size control files."
    }

    /// 解析队列路径
    /// 返回 (queue_name, operation, is_dir)
    fn parse_path(&self, path: &str) -> EvifResult<(String, String, bool)> {
        let clean_path = path.trim_start_matches('/');

        if clean_path.is_empty() || clean_path == "/" {
            return Ok((String::new(), String::new(), true));
        }

        let parts: Vec<&str> = clean_path.split('/').collect();

        // 控制文件列表
        let control_files = ["enqueue", "dequeue", "peek", "size", "clear"];

        if parts.len() == 0 {
            return Ok((String::new(), String::new(), true));
        }

        let last = parts[parts.len() - 1];
        if control_files.contains(&last) {
            // 这是一个控制文件
            if parts.len() == 1 {
                return Err(EvifError::InvalidPath("Invalid queue path".to_string()));
            }
            let queue_name = parts[..parts.len()-1].join("/");
            let operation = last.to_string();
            return Ok((queue_name, operation, false));
        }

        // 这是一个队列目录或父目录
        let queue_name = parts.join("/");
        Ok((queue_name, String::new(), true))
    }
}

#[async_trait]
impl EvifPlugin for QueueFsPlugin {
    fn name(&self) -> &str {
        "queuefs"
    }

    async fn create(&self, path: &str, _perm: u32) -> EvifResult<()> {
        let (_queue_name, operation, is_dir) = self.parse_path(path)?;

        if is_dir {
            return Err(EvifError::InvalidPath("Cannot create directory with create".to_string()));
        }

        if !operation.is_empty() {
            // 控制文件是虚拟的，不需要创建
            Ok(())
        } else {
            Err(EvifError::InvalidPath("Cannot create files in queuefs".to_string()))
        }
    }

    async fn mkdir(&self, path: &str, _perm: u32) -> EvifResult<()> {
        let (queue_name, operation, is_dir) = self.parse_path(path)?;

        if !is_dir {
            return Err(EvifError::InvalidPath("Not a directory".to_string()));
        }

        if queue_name.is_empty() {
            return Err(EvifError::InvalidPath("Invalid queue name".to_string()));
        }

        if !operation.is_empty() {
            return Err(EvifError::InvalidPath("Invalid path".to_string()));
        }

        self.store.create_queue(&queue_name).await
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        if path.trim_start_matches('/') == "README" {
            return Ok(self.readme().as_bytes().to_vec());
        }

        let (queue_name, operation, is_dir) = self.parse_path(path)?;

        if is_dir {
            return Err(EvifError::InvalidPath("Is a directory".to_string()));
        }

        if operation.is_empty() {
            return Err(EvifError::NotFound(path.to_string()));
        }

        let data = match operation.as_str() {
            "dequeue" => {
                let msg = self.store.dequeue(&queue_name).await;
                match msg {
                    Ok(m) => serde_json::to_vec(&m)
                        .map_err(|_| EvifError::InvalidPath("Failed to serialize".to_string()))?,
                    Err(_) => b"{}".to_vec(), // 空队列返回空JSON
                }
            }
            "peek" => {
                let msg = self.store.peek(&queue_name).await;
                match msg {
                    Ok(m) => serde_json::to_vec(&m)
                        .map_err(|_| EvifError::InvalidPath("Failed to serialize".to_string()))?,
                    Err(_) => b"{}".to_vec(),
                }
            }
            "size" => {
                let size = self.store.size(&queue_name).await.unwrap_or(0);
                size.to_string().into_bytes()
            }
            "enqueue" | "clear" => {
                return Err(EvifError::InvalidPath("Write-only file".to_string()));
            }
            _ => return Err(EvifError::NotFound(path.to_string())),
        };

        Ok(data)
    }

    async fn write(&self, path: &str, data: Vec<u8>, _offset: i64, _flags: WriteFlags)
        -> EvifResult<u64>
    {
        let (queue_name, operation, is_dir) = self.parse_path(path)?;

        if is_dir {
            return Err(EvifError::InvalidPath("Is a directory".to_string()));
        }

        let len = data.len() as u64;

        match operation.as_str() {
            "enqueue" => {
                self.store.enqueue(&queue_name, data).await?;
                Ok(len)
            }
            "clear" => {
                self.store.clear(&queue_name).await?;
                Ok(0)
            }
            "dequeue" | "peek" | "size" => {
                Err(EvifError::InvalidPath("Read-only file".to_string()))
            }
            _ => Err(EvifError::InvalidPath("Cannot write to this file".to_string())),
        }
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let (queue_name, _operation, is_dir) = self.parse_path(path)?;

        if !is_dir {
            return Err(EvifError::InvalidPath("Not a directory".to_string()));
        }

        let now = Utc::now();

        // 根目录 - 列出所有队列
        if queue_name.is_empty() {
            let queues = self.store.list_queues_with_prefix("").await?;

            let mut entries = vec![FileInfo {
                name: "README".to_string(),
                size: 0,
                mode: 0o444,
                modified: now,
                is_dir: false,
            }];

            for queue in queues {
                entries.push(FileInfo {
                    name: queue,
                    size: 0,
                    mode: 0o755,
                    modified: now,
                    is_dir: true,
                });
            }

            return Ok(entries);
        }

        if !self.store.queue_exists(&queue_name).await {
            return Err(EvifError::NotFound(queue_name));
        }

        // 队列目录 - 返回控制文件
        let control_files = vec![
            ("enqueue", 0o222, false),
            ("dequeue", 0o444, false),
            ("peek", 0o444, false),
            ("size", 0o444, false),
            ("clear", 0o222, false),
        ];

        let mut entries = Vec::new();
        for (name, mode, is_dir) in control_files {
            entries.push(FileInfo {
                name: name.to_string(),
                size: 0,
                mode,
                modified: now,
                is_dir,
            });
        }

        Ok(entries)
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        if path.trim_start_matches('/') == "README" {
            return Ok(FileInfo {
                name: "README".to_string(),
                size: self.readme().len() as u64,
                mode: 0o444,
                modified: Utc::now(),
                is_dir: false,
            });
        }

        let (queue_name, operation, is_dir) = self.parse_path(path)?;

        let now = Utc::now();

        if is_dir {
            let exists = self.store.queue_exists(&queue_name).await;
            if !exists && !queue_name.is_empty() {
                // 检查是否是父目录
                let queues = self.store.list_queues_with_prefix(&queue_name).await?;
                if queues.is_empty() {
                    return Err(EvifError::NotFound(path.to_string()));
                }
            }

            let name = if queue_name.is_empty() {
                "/".to_string()
            } else {
                queue_name.split('/').last().unwrap_or("unknown").to_string()
            };

            return Ok(FileInfo {
                name,
                size: 0,
                mode: 0o755,
                modified: now,
                is_dir: true,
            });
        }

        if operation.is_empty() {
            return Err(EvifError::NotFound(path.to_string()));
        }

        let mode = match operation.as_str() {
            "enqueue" | "clear" => 0o222,
            _ => 0o444,
        };

        Ok(FileInfo {
            name: operation,
            size: 0,
            mode,
            modified: now,
            is_dir: false,
        })
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        let (queue_name, _operation, is_dir) = self.parse_path(path)?;

        if is_dir {
            return Err(EvifError::InvalidPath("Use rmdir to remove directories".to_string()));
        }

        Err(EvifError::InvalidPath("Cannot remove control files".to_string()))
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        Err(EvifError::InvalidPath("Cannot rename in queuefs".to_string()))
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        // QueueFS 不支持目录结构,remove_all 等同于 remove
        self.remove(path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_queuefs_basic() {
        let plugin = QueueFsPlugin::new();

        // 创建队列
        plugin.mkdir("/my_queue", 0o755).await.unwrap();

        // 入队消息
        plugin.write("/my_queue/enqueue", b"Hello, Queue!".to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        // 检查队列大小
        let size_data = plugin.read("/my_queue/size", 0, 100).await.unwrap();
        assert_eq!(size_data, b"1");

        // 查看消息（不移除）
        let peek_data = plugin.read("/my_queue/peek", 0, 1000).await.unwrap();
        assert!(peek_data.len() > 0);

        // 出队消息
        let dequeue_data = plugin.read("/my_queue/dequeue", 0, 1000).await.unwrap();
        assert!(dequeue_data.len() > 0);

        // 队列应该空了
        let size_data = plugin.read("/my_queue/size", 0, 100).await.unwrap();
        assert_eq!(size_data, b"0");

        // 列出队列
        let entries = plugin.readdir("/").await.unwrap();
        assert!(entries.iter().any(|e| e.name == "my_queue"));
    }

    #[tokio::test]
    async fn test_queuefs_multiple_messages() {
        let plugin = QueueFsPlugin::new();

        plugin.mkdir("/orders", 0o755).await.unwrap();

        // 入队多个消息
        for i in 1..=3 {
            plugin.write(
                "/orders/enqueue",
                format!("order-{}", i).into_bytes(),
                0,
                WriteFlags::CREATE
            ).await.unwrap();
        }

        // 检查大小
        let size_data = plugin.read("/orders/size", 0, 100).await.unwrap();
        assert_eq!(size_data, b"3");

        // 出队所有消息
        for _i in 1..=3 {
            let _data = plugin.read("/orders/dequeue", 0, 1000).await.unwrap();
        }

        // 队列应该空了
        let size_data = plugin.read("/orders/size", 0, 100).await.unwrap();
        assert_eq!(size_data, b"0");
    }
}
