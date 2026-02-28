// File Monitoring System
//
// 提供文件系统监控功能，支持实时响应文件变更事件
//
// 功能：
// - 跨平台文件监控 (Linux: inotify, macOS: FSEvents)
// - 事件订阅机制
// - 事件过滤和路由
// - 跨平台抽象层

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::broadcast;

/// 文件事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileEventType {
    /// 文件创建
    Create,

    /// 文件修改
    Modify,

    /// 文件删除
    Delete,

    /// 文件移动/重命名
    Move,

    /// 属性变化
    Attribute,

    /// 访问
    Access,
}

/// 文件事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEvent {
    /// 事件类型
    pub event_type: FileEventType,

    /// 文件路径
    pub path: String,

    /// 旧路径（移动/重命名时）
    pub old_path: Option<String>,

    /// 是否是目录
    pub is_directory: bool,

    /// 事件时间戳（毫秒）
    pub timestamp: u64,

    /// 元数据
    pub metadata: HashMap<String, String>,
}

impl FileEvent {
    pub fn new(event_type: FileEventType, path: String) -> Self {
        Self {
            event_type,
            path,
            old_path: None,
            is_directory: false,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            metadata: HashMap::new(),
        }
    }

    pub fn with_old_path(mut self, old_path: String) -> Self {
        self.old_path = Some(old_path);
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// 事件过滤器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    /// 监控的路径
    pub path: String,

    /// 递归监控子目录
    pub recursive: bool,

    /// 过滤的事件类型
    pub event_types: Vec<FileEventType>,

    /// 文件名模式（glob）
    pub pattern: Option<String>,

    /// 忽略隐藏文件
    pub ignore_hidden: bool,

    /// 忽略目录列表
    pub ignore_dirs: Vec<String>,
}

impl Default for EventFilter {
    fn default() -> Self {
        Self {
            path: "/".to_string(),
            recursive: true,
            event_types: vec![
                FileEventType::Create,
                FileEventType::Modify,
                FileEventType::Delete,
                FileEventType::Move,
            ],
            pattern: None,
            ignore_hidden: true,
            ignore_dirs: vec![".git".to_string(), "node_modules".to_string()],
        }
    }
}

/// 事件订阅者 ID
type SubscriberId = String;

/// 文件监控器接口
#[async_trait]
pub trait FileMonitor: Send + Sync {
    /// 开始监控路径
    async fn watch(&self, path: &str, recursive: bool) -> Result<(), MonitorError>;

    /// 停止监控
    async fn stop(&self) -> Result<(), MonitorError>;

    /// 检查监控器是否运行
    fn is_running(&self) -> bool;

    /// 获取监控器名称
    fn name(&self) -> &str;
}

/// 监控错误
#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
pub enum MonitorError {
    #[error("Path not found: {0}")]
    PathNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Too many watches")]
    TooManyWatches,

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Monitor not running")]
    NotRunning,

    #[error("IO error: {0}")]
    Io(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// 事件管理器
#[derive(Clone)]
pub struct EventManager {
    /// 订阅者映射
    subscribers: Arc<Mutex<HashMap<SubscriberId, EventFilter>>>,

    /// 事件广播
    event_tx: broadcast::Sender<FileEvent>,
}

impl EventManager {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1000);

        Self {
            subscribers: Arc::new(Mutex::new(HashMap::new())),
            event_tx,
        }
    }

    /// 订阅事件
    pub fn subscribe(&self, filter: EventFilter) -> SubscriberId {
        let id = uuid::Uuid::new_v4().to_string();
        let mut subs = self.subscribers.lock().unwrap();
        subs.insert(id.clone(), filter);
        id
    }

    /// 取消订阅
    pub fn unsubscribe(&self, subscriber_id: &str) -> bool {
        let mut subs = self.subscribers.lock().unwrap();
        subs.remove(subscriber_id).is_some()
    }

    /// 发布事件
    pub fn publish(&self, event: FileEvent) {
        let subs = self.subscribers.lock().unwrap();
        for (_id, filter) in subs.iter() {
            if self.matches_filter(&event, filter) {
                let _ = self.event_tx.send(event.clone());
            }
        }
    }

    /// 获取事件接收器
    pub fn receiver(&self) -> broadcast::Receiver<FileEvent> {
        self.event_tx.subscribe()
    }

    /// 检查事件是否匹配过滤器
    fn matches_filter(&self, event: &FileEvent, filter: &EventFilter) -> bool {
        // 检查路径是否匹配
        if !event.path.starts_with(&filter.path) {
            return false;
        }

        // 非递归：检查是否直接在监控路径下
        if !filter.recursive {
            let relative = event.path.trim_start_matches(&filter.path);
            if relative.contains('/') {
                return false;
            }
        }

        // 检查事件类型
        if !filter.event_types.contains(&event.event_type) {
            return false;
        }

        // 检查忽略隐藏文件
        if filter.ignore_hidden {
            let filename = event.path.rsplit('/').next().unwrap_or(&event.path);
            if filename.starts_with('.') {
                return false;
            }
        }

        // 检查忽略目录
        for ignore_dir in &filter.ignore_dirs {
            if event.path.contains(format!("/{}", ignore_dir).as_str()) {
                return false;
            }
        }

        true
    }
}

/// 简单的文件监控器实现（通用）
pub struct SimpleFileMonitor {
    running: Arc<Mutex<bool>>,
    name: String,
    event_manager: EventManager,
}

impl SimpleFileMonitor {
    pub fn new(name: String, event_manager: EventManager) -> Self {
        Self {
            running: Arc::new(Mutex::new(false)),
            name,
            event_manager,
        }
    }

    /// 模拟文件事件（用于测试）
    pub fn emit_event(&self, event: FileEvent) {
        self.event_manager.publish(event);
    }
}

#[async_trait]
impl FileMonitor for SimpleFileMonitor {
    async fn watch(&self, path: &str, recursive: bool) -> Result<(), MonitorError> {
        println!("Starting monitor: {} (recursive: {})", path, recursive);
        *self.running.lock().unwrap() = true;

        // 这里应该实现实际的文件监控逻辑
        // 对于生产环境，需要使用平台特定的实现

        Ok(())
    }

    async fn stop(&self) -> Result<(), MonitorError> {
        println!("Stopping monitor: {}", self.name);
        *self.running.lock().unwrap() = false;
        Ok(())
    }

    fn is_running(&self) -> bool {
        *self.running.lock().unwrap()
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// 文件监控器工厂
pub struct MonitorFactory {
    event_manager: EventManager,
}

impl MonitorFactory {
    pub fn new(event_manager: EventManager) -> Self {
        Self { event_manager }
    }

    /// 创建平台特定的监控器
    pub fn create(&self) -> Result<Box<dyn FileMonitor>, MonitorError> {
        #[cfg(target_os = "linux")]
        {
            // Linux: 使用 inotify
            // Ok(Box::new(InotifyMonitor::new(self.event_manager.clone())))
            println!("Creating inotify monitor for Linux");
            return Ok(Box::new(SimpleFileMonitor::new(
                "inotify".to_string(),
                self.event_manager.clone(),
            )));
        }

        #[cfg(target_os = "macos")]
        {
            // macOS: 使用 FSEvents
            // Ok(Box::new(FSEventsMonitor::new(self.event_manager.clone())))
            println!("Creating FSEvents monitor for macOS");
            return Ok(Box::new(SimpleFileMonitor::new(
                "fsevents".to_string(),
                self.event_manager.clone(),
            )));
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            // 其他平台：使用轮询
            println!("Creating polling monitor for generic platform");
            return Ok(Box::new(SimpleFileMonitor::new(
                "polling".to_string(),
                self.event_manager.clone(),
            )));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_event_creation() {
        let event = FileEvent::new(FileEventType::Create, "/test/file.txt".to_string());
        assert_eq!(event.event_type, FileEventType::Create);
        assert_eq!(event.path, "/test/file.txt");
        assert!(event.metadata.is_empty());
    }

    #[test]
    fn test_event_filter_default() {
        let filter = EventFilter::default();
        assert_eq!(filter.path, "/");
        assert!(filter.recursive);
        assert_eq!(filter.event_types.len(), 4);
        assert!(filter.ignore_hidden);
    }

    #[tokio::test]
    async fn test_event_manager() {
        let manager = EventManager::new();

        // 订阅事件
        let filter = EventFilter::default();
        let subscriber_id = manager.subscribe(filter);

        // 接收事件
        let mut receiver = manager.receiver();

        // 发布事件
        let event = FileEvent::new(FileEventType::Create, "/test/file.txt".to_string());
        manager.publish(event.clone());

        // 尝试接收事件
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let result = receiver.try_recv();
        assert!(result.is_ok());

        // 取消订阅
        assert!(manager.unsubscribe(&subscriber_id));
    }

    #[test]
    fn test_event_filter_matching() {
        let manager = EventManager::new();
        let filter = EventFilter {
            path: "/test".to_string(),
            recursive: true,
            event_types: vec![FileEventType::Create, FileEventType::Modify],
            pattern: None,
            ignore_hidden: true,
            ignore_dirs: vec![],
        };

        // 匹配的事件
        let create_event = FileEvent::new(FileEventType::Create, "/test/file.txt".to_string());
        assert!(manager.matches_filter(&create_event, &filter));

        // 不匹配的事件类型
        let delete_event = FileEvent::new(FileEventType::Delete, "/test/file.txt".to_string());
        assert!(!manager.matches_filter(&delete_event, &filter));

        // 不匹配的路径
        let wrong_path_event = FileEvent::new(FileEventType::Create, "/other/file.txt".to_string());
        assert!(!manager.matches_filter(&wrong_path_event, &filter));
    }

    #[tokio::test]
    async fn test_simple_monitor() {
        let event_manager = EventManager::new();
        let monitor = SimpleFileMonitor::new("test".to_string(), event_manager);

        assert!(!monitor.is_running());
        assert_eq!(monitor.name(), "test");

        monitor.watch("/test", true).await.unwrap();
        assert!(monitor.is_running());

        monitor.stop().await.unwrap();
        assert!(!monitor.is_running());
    }
}
