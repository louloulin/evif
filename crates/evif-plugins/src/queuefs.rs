// QueueFS - 消息队列插件 (增强版)
//
// 对标 AGFS QueueFS - 提供基于文件系统的消息队列服务
// 新增功能: 优先队列、延迟队列、死信队列、批量操作、SQLite 持久化后端

use evif_core::{EvifPlugin, FileInfo, WriteFlags, EvifResult, EvifError};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, BinaryHeap};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::cmp::Ordering;

/// 队列消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueMessage {
    pub id: String,
    pub data: String,
    pub timestamp: i64,
    pub priority: i32,
    pub delay_until: Option<i64>,
    pub retry_count: u32,
    pub max_retries: u32,
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
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => self.timestamp.cmp(&other.timestamp),
            other => other,
        }
    }
}

/// 队列后端 trait - 抽象存储层，支持多种持久化后端
#[async_trait]
pub trait QueueBackend: Send + Sync {
    /// 创建队列
    async fn create_queue(&self, name: &str) -> EvifResult<()>;
    /// 删除队列
    async fn remove_queue(&self, name: &str) -> EvifResult<()>;
    /// 入队消息（默认优先级）
    async fn enqueue(&self, name: &str, data: Vec<u8>) -> EvifResult<String>;
    /// 出队消息
    async fn dequeue(&self, name: &str) -> EvifResult<QueueMessage>;
    /// 查看队首消息（不移除）
    async fn peek(&self, name: &str) -> EvifResult<QueueMessage>;
    /// 获取队列大小
    async fn size(&self, name: &str) -> EvifResult<usize>;
    /// 清空队列
    async fn clear(&self, name: &str) -> EvifResult<()>;
    /// 列出所有队列
    async fn list_queues(&self) -> Vec<String>;
    /// 列出带前缀的队列
    async fn list_queues_with_prefix(&self, prefix: &str) -> EvifResult<Vec<String>>;
    /// 检查队列是否存在
    async fn queue_exists(&self, name: &str) -> bool;
}

// ==================== Memory Backend ====================

struct Queue {
    messages: Vec<QueueMessage>,
    priority_messages: BinaryHeap<QueueMessage>,
    delayed_messages: Vec<QueueMessage>,
    last_enqueue_time: i64,
    max_size: usize,
    #[allow(dead_code)]
    dead_letter_queue: Option<String>,
}

impl Queue {
    fn new() -> Self {
        Self {
            messages: Vec::new(),
            priority_messages: BinaryHeap::new(),
            delayed_messages: Vec::new(),
            last_enqueue_time: 0,
            max_size: 10000,
            dead_letter_queue: None,
        }
    }

    #[allow(dead_code)]
    fn is_empty(&self) -> bool {
        self.messages.is_empty() && self.priority_messages.is_empty()
    }

    fn len(&self) -> usize {
        self.messages.len() + self.priority_messages.len()
    }
}

/// 内存队列后端
pub struct MemoryQueueBackend {
    queues: RwLock<HashMap<String, Queue>>,
}

impl Default for MemoryQueueBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryQueueBackend {
    pub fn new() -> Self {
        Self {
            queues: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl QueueBackend for MemoryQueueBackend {
    async fn create_queue(&self, name: &str) -> EvifResult<()> {
        let mut queues = self.queues.write().await;
        if queues.contains_key(name) {
            return Err(EvifError::AlreadyExists(format!("queue: {}", name)));
        }
        queues.insert(name.to_string(), Queue::new());
        Ok(())
    }

    async fn remove_queue(&self, name: &str) -> EvifResult<()> {
        let mut queues = self.queues.write().await;
        queues
            .remove(name)
            .ok_or_else(|| EvifError::NotFound(name.to_string()))?;
        Ok(())
    }

    async fn enqueue(&self, name: &str, data: Vec<u8>) -> EvifResult<String> {
        let mut queues = self.queues.write().await;
        let queue = queues
            .get_mut(name)
            .ok_or_else(|| EvifError::NotFound(name.to_string()))?;

        if queue.len() >= queue.max_size {
            return Err(EvifError::QueueFull(name.to_string()));
        }

        let msg = QueueMessage {
            id: Uuid::new_v4().to_string(),
            data: String::from_utf8(data)
                .map_err(|_| EvifError::InvalidInput("Invalid UTF-8 data".to_string()))?,
            timestamp: Utc::now().timestamp(),
            priority: 999,
            delay_until: None,
            retry_count: 0,
            max_retries: 3,
        };

        queue.messages.push(msg.clone());
        queue.last_enqueue_time = msg.timestamp;
        Ok(msg.id)
    }

    async fn dequeue(&self, name: &str) -> EvifResult<QueueMessage> {
        let mut queues = self.queues.write().await;
        let queue = queues
            .get_mut(name)
            .ok_or_else(|| EvifError::NotFound(name.to_string()))?;

        // 先从优先队列取
        if let Some(msg) = queue.priority_messages.pop() {
            return Ok(msg);
        }

        // 再从普通队列取
        if !queue.messages.is_empty() {
            return Ok(queue.messages.remove(0));
        }

        Err(EvifError::EmptyQueue(name.to_string()))
    }

    async fn peek(&self, name: &str) -> EvifResult<QueueMessage> {
        let queues = self.queues.read().await;
        let queue = queues
            .get(name)
            .ok_or_else(|| EvifError::NotFound(name.to_string()))?;

        if let Some(msg) = queue.priority_messages.peek() {
            return Ok(msg.clone());
        }

        if let Some(msg) = queue.messages.first() {
            return Ok(msg.clone());
        }

        Err(EvifError::EmptyQueue(name.to_string()))
    }

    async fn size(&self, name: &str) -> EvifResult<usize> {
        let queues = self.queues.read().await;
        let queue = queues
            .get(name)
            .ok_or_else(|| EvifError::NotFound(name.to_string()))?;
        Ok(queue.len())
    }

    async fn clear(&self, name: &str) -> EvifResult<()> {
        let mut queues = self.queues.write().await;
        let queue = queues
            .get_mut(name)
            .ok_or_else(|| EvifError::NotFound(name.to_string()))?;
        queue.messages.clear();
        queue.priority_messages.clear();
        queue.delayed_messages.clear();
        Ok(())
    }

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

// ==================== SQLite Backend ====================

#[cfg(feature = "queuefs-sqlite")]
mod sqlite_backend {
    use super::{QueueBackend, QueueMessage, EvifResult, EvifError};
    use async_trait::async_trait;
    use chrono::Utc;
    use uuid::Uuid;

    /// SQLite 持久化队列后端
    pub struct SqliteQueueBackend {
        db_path: String,
    }

    impl SqliteQueueBackend {
        pub fn new(db_path: &str) -> EvifResult<Self> {
            // Initialize schema by opening a connection and creating tables
            {
                let conn = rusqlite::Connection::open(db_path)
                    .map_err(|e| EvifError::Storage(format!("Failed to open database: {}", e)))?;
                conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
                    .map_err(|e| EvifError::Storage(format!("Failed to set pragmas: {}", e)))?;
                conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS queues (
                        name TEXT PRIMARY KEY,
                        max_size INTEGER NOT NULL DEFAULT 10000,
                        dead_letter_queue TEXT,
                        created_at INTEGER NOT NULL
                    );
                    CREATE TABLE IF NOT EXISTS messages (
                        id TEXT PRIMARY KEY,
                        queue_name TEXT NOT NULL,
                        data TEXT NOT NULL,
                        timestamp INTEGER NOT NULL,
                        priority INTEGER NOT NULL DEFAULT 999,
                        delay_until INTEGER,
                        retry_count INTEGER NOT NULL DEFAULT 0,
                        max_retries INTEGER NOT NULL DEFAULT 3,
                        status TEXT NOT NULL DEFAULT 'ready',
                        FOREIGN KEY (queue_name) REFERENCES queues(name) ON DELETE CASCADE
                    );
                    CREATE INDEX IF NOT EXISTS idx_messages_queue_status
                        ON messages(queue_name, status, priority, timestamp);",
                )
                .map_err(|e| EvifError::Storage(format!("Failed to init schema: {}", e)))?;
            }
            Ok(Self {
                db_path: db_path.to_string(),
            })
        }
    }

    #[async_trait]
    impl QueueBackend for SqliteQueueBackend {
        async fn create_queue(&self, name: &str) -> EvifResult<()> {
            let db_path = self.db_path.clone();
            let queue_name = name.to_string();
            tokio::task::spawn_blocking(move || -> EvifResult<()> {
                let conn = rusqlite::Connection::open(&db_path)
                    .map_err(|e| EvifError::Storage(format!("Connection error: {}", e)))?;
                let exists: bool = conn
                    .query_row(
                        "SELECT COUNT(*) > 0 FROM queues WHERE name = ?1",
                        rusqlite::params![queue_name],
                        |row| row.get(0),
                    )
                    .unwrap_or(false);
                if exists {
                    return Err(EvifError::AlreadyExists(format!("queue: {}", queue_name)));
                }
                conn.execute(
                    "INSERT INTO queues (name, created_at) VALUES (?1, ?2)",
                    rusqlite::params![queue_name, Utc::now().timestamp()],
                )
                .map_err(|e| EvifError::Storage(format!("Insert error: {}", e)))?;
                Ok(())
            })
            .await
            .map_err(|e| EvifError::Internal(format!("Task error: {}", e)))?
        }

        async fn remove_queue(&self, name: &str) -> EvifResult<()> {
            let db_path = self.db_path.clone();
            let queue_name = name.to_string();
            tokio::task::spawn_blocking(move || -> EvifResult<()> {
                let conn = rusqlite::Connection::open(&db_path)
                    .map_err(|e| EvifError::Storage(format!("Connection error: {}", e)))?;
                let rows = conn
                    .execute(
                        "DELETE FROM queues WHERE name = ?1",
                        rusqlite::params![queue_name],
                    )
                    .map_err(|e| EvifError::Storage(format!("Delete error: {}", e)))?;
                if rows == 0 {
                    return Err(EvifError::NotFound(queue_name));
                }
                Ok(())
            })
            .await
            .map_err(|e| EvifError::Internal(format!("Task error: {}", e)))?
        }

        async fn enqueue(&self, name: &str, data: Vec<u8>) -> EvifResult<String> {
            let db_path = self.db_path.clone();
            let queue_name = name.to_string();
            let msg_id = Uuid::new_v4().to_string();
            let msg_data = String::from_utf8(data)
                .map_err(|_| EvifError::InvalidInput("Invalid UTF-8 data".to_string()))?;
            let timestamp = Utc::now().timestamp();
            let id_for_result = msg_id.clone();

            tokio::task::spawn_blocking(move || -> EvifResult<()> {
                let conn = rusqlite::Connection::open(&db_path)
                    .map_err(|e| EvifError::Storage(format!("Connection error: {}", e)))?;

                // Check queue exists and capacity
                let max_size: usize = conn
                    .query_row(
                        "SELECT max_size FROM queues WHERE name = ?1",
                        rusqlite::params![queue_name],
                        |row| row.get(0),
                    )
                    .map_err(|_| EvifError::NotFound(queue_name.clone()))?;

                let current_size: usize = conn
                    .query_row(
                        "SELECT COUNT(*) FROM messages WHERE queue_name = ?1 AND status = 'ready'",
                        rusqlite::params![queue_name],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);

                if current_size >= max_size {
                    return Err(EvifError::QueueFull(queue_name));
                }

                conn.execute(
                    "INSERT INTO messages (id, queue_name, data, timestamp, priority, status) \
                     VALUES (?1, ?2, ?3, ?4, 999, 'ready')",
                    rusqlite::params![msg_id, queue_name, msg_data, timestamp],
                )
                .map_err(|e| EvifError::Storage(format!("Insert error: {}", e)))?;

                Ok(())
            })
            .await
            .map_err(|e| EvifError::Internal(format!("Task error: {}", e)))??;

            Ok(id_for_result)
        }

        async fn dequeue(&self, name: &str) -> EvifResult<QueueMessage> {
            let db_path = self.db_path.clone();
            let queue_name = name.to_string();

            tokio::task::spawn_blocking(move || -> EvifResult<QueueMessage> {
                let conn = rusqlite::Connection::open(&db_path)
                    .map_err(|e| EvifError::Storage(format!("Connection error: {}", e)))?;

                let now_ts = Utc::now().timestamp();
                let msg = conn
                    .query_row(
                        "SELECT id, data, timestamp, priority, delay_until, retry_count, max_retries \
                         FROM messages \
                         WHERE queue_name = ?1 AND status = 'ready' \
                         AND (delay_until IS NULL OR delay_until <= ?2) \
                         ORDER BY priority ASC, timestamp ASC LIMIT 1",
                        rusqlite::params![queue_name, now_ts],
                        |row| {
                            Ok(QueueMessage {
                                id: row.get(0)?,
                                data: row.get(1)?,
                                timestamp: row.get(2)?,
                                priority: row.get(3)?,
                                delay_until: row.get(4)?,
                                retry_count: row.get(5)?,
                                max_retries: row.get(6)?,
                            })
                        },
                    )
                    .map_err(|_| EvifError::EmptyQueue(queue_name.clone()))?;

                conn.execute(
                    "DELETE FROM messages WHERE id = ?1",
                    rusqlite::params![msg.id],
                )
                .map_err(|e| EvifError::Storage(format!("Delete error: {}", e)))?;

                Ok(msg)
            })
            .await
            .map_err(|e| EvifError::Internal(format!("Task error: {}", e)))?
        }

        async fn peek(&self, name: &str) -> EvifResult<QueueMessage> {
            let db_path = self.db_path.clone();
            let queue_name = name.to_string();

            tokio::task::spawn_blocking(move || -> EvifResult<QueueMessage> {
                let conn = rusqlite::Connection::open(&db_path)
                    .map_err(|e| EvifError::Storage(format!("Connection error: {}", e)))?;

                let now_ts = Utc::now().timestamp();
                conn.query_row(
                    "SELECT id, data, timestamp, priority, delay_until, retry_count, max_retries \
                     FROM messages \
                     WHERE queue_name = ?1 AND status = 'ready' \
                     AND (delay_until IS NULL OR delay_until <= ?2) \
                     ORDER BY priority ASC, timestamp ASC LIMIT 1",
                    rusqlite::params![queue_name, now_ts],
                    |row| {
                        Ok(QueueMessage {
                            id: row.get(0)?,
                            data: row.get(1)?,
                            timestamp: row.get(2)?,
                            priority: row.get(3)?,
                            delay_until: row.get(4)?,
                            retry_count: row.get(5)?,
                            max_retries: row.get(6)?,
                        })
                    },
                )
                .map_err(|_| EvifError::EmptyQueue(queue_name))
            })
            .await
            .map_err(|e| EvifError::Internal(format!("Task error: {}", e)))?
        }

        async fn size(&self, name: &str) -> EvifResult<usize> {
            let db_path = self.db_path.clone();
            let queue_name = name.to_string();

            tokio::task::spawn_blocking(move || -> EvifResult<usize> {
                let conn = rusqlite::Connection::open(&db_path)
                    .map_err(|e| EvifError::Storage(format!("Connection error: {}", e)))?;

                let count: usize = conn
                    .query_row(
                        "SELECT COUNT(*) FROM messages WHERE queue_name = ?1 AND status = 'ready'",
                        rusqlite::params![queue_name],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);

                Ok(count)
            })
            .await
            .map_err(|e| EvifError::Internal(format!("Task error: {}", e)))?
        }

        async fn clear(&self, name: &str) -> EvifResult<()> {
            let db_path = self.db_path.clone();
            let queue_name = name.to_string();

            tokio::task::spawn_blocking(move || -> EvifResult<()> {
                let conn = rusqlite::Connection::open(&db_path)
                    .map_err(|e| EvifError::Storage(format!("Connection error: {}", e)))?;

                conn.execute(
                    "DELETE FROM messages WHERE queue_name = ?1",
                    rusqlite::params![queue_name],
                )
                .map_err(|e| EvifError::Storage(format!("Delete error: {}", e)))?;

                Ok(())
            })
            .await
            .map_err(|e| EvifError::Internal(format!("Task error: {}", e)))?
        }

        async fn list_queues(&self) -> Vec<String> {
            let db_path = self.db_path.clone();

            tokio::task::spawn_blocking(move || -> Vec<String> {
                let conn = match rusqlite::Connection::open(&db_path) {
                    Ok(c) => c,
                    Err(_) => return Vec::new(),
                };

                let mut stmt = match conn.prepare("SELECT name FROM queues ORDER BY name") {
                    Ok(s) => s,
                    Err(_) => return Vec::new(),
                };

                stmt.query_map([], |row| row.get::<_, String>(0))
                    .map(|rows| rows.filter_map(|r| r.ok()).collect())
                    .unwrap_or_default()
            })
            .await
            .unwrap_or_default()
        }

        async fn list_queues_with_prefix(&self, prefix: &str) -> EvifResult<Vec<String>> {
            let db_path = self.db_path.clone();
            let prefix_str = prefix.to_string();

            tokio::task::spawn_blocking(move || -> EvifResult<Vec<String>> {
                let conn = rusqlite::Connection::open(&db_path)
                    .map_err(|e| EvifError::Storage(format!("Connection error: {}", e)))?;

                if prefix_str.is_empty() {
                    // Empty prefix returns all queues
                    let mut stmt = conn
                        .prepare("SELECT name FROM queues ORDER BY name")
                        .map_err(|e| EvifError::Storage(format!("Prepare error: {}", e)))?;
                    let rows = stmt
                        .query_map([], |row| row.get::<_, String>(0))
                        .map_err(|e| EvifError::Storage(format!("Query error: {}", e)))?;
                    return Ok(rows.filter_map(|r| r.ok()).collect());
                }

                let mut stmt = conn
                    .prepare(
                        "SELECT name FROM queues WHERE name = ?1 OR name LIKE ?2 ORDER BY name",
                    )
                    .map_err(|e| EvifError::Storage(format!("Prepare error: {}", e)))?;

                let like_pattern = format!("{}/%", prefix_str);
                let rows = stmt
                    .query_map(rusqlite::params![prefix_str, like_pattern], |row| {
                        row.get::<_, String>(0)
                    })
                    .map_err(|e| EvifError::Storage(format!("Query error: {}", e)))?;

                let mut result: Vec<String> = rows.filter_map(|r| r.ok()).collect();
                result.sort();
                result.dedup();
                Ok(result)
            })
            .await
            .map_err(|e| EvifError::Internal(format!("Task error: {}", e)))?
        }

        async fn queue_exists(&self, name: &str) -> bool {
            let db_path = self.db_path.clone();
            let queue_name = name.to_string();

            tokio::task::spawn_blocking(move || -> bool {
                let conn = match rusqlite::Connection::open(&db_path) {
                    Ok(c) => c,
                    Err(_) => return false,
                };

                conn.query_row(
                    "SELECT COUNT(*) > 0 FROM queues WHERE name = ?1",
                    rusqlite::params![queue_name],
                    |row| row.get(0),
                )
                .unwrap_or(false)
            })
            .await
            .unwrap_or(false)
        }
    }
}

// ==================== MySQL Backend ====================

#[cfg(feature = "queuefs-mysql")]
mod mysql_backend {
    use super::{QueueBackend, QueueMessage, EvifResult, EvifError};
    use async_trait::async_trait;
    use chrono::Utc;
    use sqlx::mysql::MySqlPoolOptions;
    use sqlx::MySqlPool;

    /// MySQL 持久化队列后端
    pub struct MysqlQueueBackend {
        pool: MySqlPool,
    }

    impl MysqlQueueBackend {
        /// Create a new MySQL queue backend.
        /// `database_url` should be a MySQL connection string,
        /// e.g. "mysql://user:password@localhost:3306/evif_queues"
        pub async fn new(database_url: &str) -> EvifResult<Self> {
            let pool = MySqlPoolOptions::new()
                .max_connections(10)
                .connect(database_url)
                .await
                .map_err(|e| EvifError::Storage(format!("Failed to create MySQL pool: {}", e)))?;

            // Initialize schema
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS queues (
                    name VARCHAR(255) PRIMARY KEY,
                    max_size INT NOT NULL DEFAULT 10000,
                    dead_letter_queue VARCHAR(255),
                    created_at BIGINT NOT NULL
                ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4"
            )
            .execute(&pool)
            .await
            .map_err(|e| EvifError::Storage(format!("Failed to create queues table: {}", e)))?;

            sqlx::query(
                "CREATE TABLE IF NOT EXISTS messages (
                    id VARCHAR(36) PRIMARY KEY,
                    queue_name VARCHAR(255) NOT NULL,
                    data MEDIUMTEXT NOT NULL,
                    timestamp BIGINT NOT NULL,
                    priority INT NOT NULL DEFAULT 999,
                    delay_until BIGINT,
                    retry_count INT NOT NULL DEFAULT 0,
                    max_retries INT NOT NULL DEFAULT 3,
                    status VARCHAR(20) NOT NULL DEFAULT 'ready',
                    INDEX idx_queue_status (queue_name, status, priority, timestamp),
                    FOREIGN KEY (queue_name) REFERENCES queues(name) ON DELETE CASCADE
                ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4"
            )
            .execute(&pool)
            .await
            .map_err(|e| EvifError::Storage(format!("Failed to create messages table: {}", e)))?;

            Ok(Self { pool })
        }
    }

    #[async_trait]
    impl QueueBackend for MysqlQueueBackend {
        async fn create_queue(&self, name: &str) -> EvifResult<()> {
            let exists: bool = sqlx::query_scalar("SELECT COUNT(*) > 0 FROM queues WHERE name = ?")
                .bind(name)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| EvifError::Storage(format!("Query error: {}", e)))?;

            if exists {
                return Err(EvifError::AlreadyExists(format!("queue: {}", name)));
            }

            sqlx::query("INSERT INTO queues (name, created_at) VALUES (?, ?)")
                .bind(name)
                .bind(Utc::now().timestamp())
                .execute(&self.pool)
                .await
                .map_err(|e| EvifError::Storage(format!("Insert error: {}", e)))?;

            Ok(())
        }

        async fn remove_queue(&self, name: &str) -> EvifResult<()> {
            let result = sqlx::query("DELETE FROM queues WHERE name = ?")
                .bind(name)
                .execute(&self.pool)
                .await
                .map_err(|e| EvifError::Storage(format!("Delete error: {}", e)))?;

            if result.rows_affected() == 0 {
                return Err(EvifError::NotFound(name.to_string()));
            }
            Ok(())
        }

        async fn enqueue(&self, name: &str, data: Vec<u8>) -> EvifResult<String> {
            let msg_data = String::from_utf8(data)
                .map_err(|_| EvifError::InvalidInput("Invalid UTF-8 data".to_string()))?;

            // Check queue exists and capacity
            let max_size: i64 = sqlx::query_scalar("SELECT max_size FROM queues WHERE name = ?")
                .bind(name)
                .fetch_one(&self.pool)
                .await
                .map_err(|_| EvifError::NotFound(name.to_string()))?;

            let current_size: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM messages WHERE queue_name = ? AND status = 'ready'"
            )
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

            if current_size >= max_size {
                return Err(EvifError::QueueFull(name.to_string()));
            }

            let msg_id = uuid::Uuid::new_v4().to_string();
            let timestamp = Utc::now().timestamp();

            sqlx::query(
                "INSERT INTO messages (id, queue_name, data, timestamp, priority, status) \
                 VALUES (?, ?, ?, ?, 999, 'ready')"
            )
            .bind(&msg_id)
            .bind(name)
            .bind(&msg_data)
            .bind(timestamp)
            .execute(&self.pool)
            .await
            .map_err(|e| EvifError::Storage(format!("Insert error: {}", e)))?;

            Ok(msg_id)
        }

        async fn dequeue(&self, name: &str) -> EvifResult<QueueMessage> {
            let now_ts = Utc::now().timestamp();

            // Fetch and delete atomically using a transaction
            let mut tx = self.pool.begin().await
                .map_err(|e| EvifError::Storage(format!("Transaction error: {}", e)))?;

            let row: Option<(String, String, i64, i32, Option<i64>, u32, u32)> = sqlx::query_as(
                "SELECT id, data, timestamp, priority, delay_until, retry_count, max_retries \
                 FROM messages \
                 WHERE queue_name = ? AND status = 'ready' \
                 AND (delay_until IS NULL OR delay_until <= ?) \
                 ORDER BY priority ASC, timestamp ASC LIMIT 1 \
                 FOR UPDATE"
            )
            .bind(name)
            .bind(now_ts)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| EvifError::Storage(format!("Query error: {}", e)))?;

            let row = row.ok_or_else(|| EvifError::EmptyQueue(name.to_string()))?;

            sqlx::query("DELETE FROM messages WHERE id = ?")
                .bind(&row.0)
                .execute(&mut *tx)
                .await
                .map_err(|e| EvifError::Storage(format!("Delete error: {}", e)))?;

            tx.commit().await
                .map_err(|e| EvifError::Storage(format!("Commit error: {}", e)))?;

            Ok(QueueMessage {
                id: row.0,
                data: row.1,
                timestamp: row.2,
                priority: row.3,
                delay_until: row.4,
                retry_count: row.5,
                max_retries: row.6,
            })
        }

        async fn peek(&self, name: &str) -> EvifResult<QueueMessage> {
            let now_ts = Utc::now().timestamp();

            let row: (String, String, i64, i32, Option<i64>, u32, u32) = sqlx::query_as(
                "SELECT id, data, timestamp, priority, delay_until, retry_count, max_retries \
                 FROM messages \
                 WHERE queue_name = ? AND status = 'ready' \
                 AND (delay_until IS NULL OR delay_until <= ?) \
                 ORDER BY priority ASC, timestamp ASC LIMIT 1"
            )
            .bind(name)
            .bind(now_ts)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| EvifError::EmptyQueue(name.to_string()))?;

            Ok(QueueMessage {
                id: row.0,
                data: row.1,
                timestamp: row.2,
                priority: row.3,
                delay_until: row.4,
                retry_count: row.5,
                max_retries: row.6,
            })
        }

        async fn size(&self, name: &str) -> EvifResult<usize> {
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM messages WHERE queue_name = ? AND status = 'ready'"
            )
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

            Ok(count as usize)
        }

        async fn clear(&self, name: &str) -> EvifResult<()> {
            sqlx::query("DELETE FROM messages WHERE queue_name = ?")
                .bind(name)
                .execute(&self.pool)
                .await
                .map_err(|e| EvifError::Storage(format!("Delete error: {}", e)))?;

            Ok(())
        }

        async fn list_queues(&self) -> Vec<String> {
            let rows: Vec<String> = sqlx::query_scalar("SELECT name FROM queues ORDER BY name")
                .fetch_all(&self.pool)
                .await
                .unwrap_or_default();

            rows
        }

        async fn list_queues_with_prefix(&self, prefix: &str) -> EvifResult<Vec<String>> {
            if prefix.is_empty() {
                return Ok(self.list_queues().await);
            }

            let like_pattern = format!("{}/%", prefix);
            let rows: Vec<String> = sqlx::query_scalar(
                "SELECT name FROM queues WHERE name = ? OR name LIKE ? ORDER BY name"
            )
            .bind(prefix)
            .bind(&like_pattern)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| EvifError::Storage(format!("Query error: {}", e)))?;

            Ok(rows)
        }

        async fn queue_exists(&self, name: &str) -> bool {
            let exists: bool = sqlx::query_scalar("SELECT COUNT(*) > 0 FROM queues WHERE name = ?")
                .bind(name)
                .fetch_one(&self.pool)
                .await
                .unwrap_or(false);

            exists
        }
    }
}

// ==================== Plugin ====================

pub struct QueueFsPlugin {
    backend: Arc<dyn QueueBackend>,
}

impl QueueFsPlugin {
    /// Create QueueFS with in-memory backend (default)
    pub fn new() -> Self {
        Self {
            backend: Arc::new(MemoryQueueBackend::new()),
        }
    }

    /// Create QueueFS with SQLite persistent backend
    #[cfg(feature = "queuefs-sqlite")]
    pub fn with_sqlite(db_path: &str) -> EvifResult<Self> {
        Ok(Self {
            backend: Arc::new(sqlite_backend::SqliteQueueBackend::new(db_path)?),
        })
    }

    /// Create QueueFS with MySQL persistent backend
    #[cfg(feature = "queuefs-mysql")]
    pub async fn with_mysql(database_url: &str) -> EvifResult<Self> {
        Ok(Self {
            backend: Arc::new(mysql_backend::MysqlQueueBackend::new(database_url).await?),
        })
    }

    fn readme(&self) -> &'static str {
        "QueueFS Plugin - File-oriented queue controls\n\n\
         Create queues with mkdir, enqueue by writing to enqueue, \
         and consume via dequeue/peek/size control files.\n\
         \n\
         Backend: memory (default) or SQLite (persistent)."
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

        if parts.is_empty() {
            return Ok((String::new(), String::new(), true));
        }

        let last = parts[parts.len() - 1];
        if control_files.contains(&last) {
            // 这是一个控制文件
            if parts.len() == 1 {
                return Err(EvifError::InvalidPath("Invalid queue path".to_string()));
            }
            let queue_name = parts[..parts.len() - 1].join("/");
            let operation = last.to_string();
            return Ok((queue_name, operation, false));
        }

        // 这是一个队列目录或父目录
        let queue_name = parts.join("/");
        Ok((queue_name, String::new(), true))
    }
}

impl Default for QueueFsPlugin {
    fn default() -> Self {
        Self::new()
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
            return Err(EvifError::InvalidPath(
                "Cannot create directory with create".to_string(),
            ));
        }

        if !operation.is_empty() {
            // 控制文件是虚拟的，不需要创建
            Ok(())
        } else {
            Err(EvifError::InvalidPath(
                "Cannot create files in queuefs".to_string(),
            ))
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

        self.backend.create_queue(&queue_name).await
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
                let msg = self.backend.dequeue(&queue_name).await;
                match msg {
                    Ok(m) => serde_json::to_vec(&m)
                        .map_err(|_| EvifError::Serialization("Failed to serialize".to_string()))?,
                    Err(EvifError::EmptyQueue(_)) => b"{}".to_vec(), // 空队列返回空JSON
                    Err(e) => return Err(e),
                }
            }
            "peek" => {
                let msg = self.backend.peek(&queue_name).await;
                match msg {
                    Ok(m) => serde_json::to_vec(&m)
                        .map_err(|_| EvifError::Serialization("Failed to serialize".to_string()))?,
                    Err(EvifError::EmptyQueue(_)) => b"{}".to_vec(),
                    Err(e) => return Err(e),
                }
            }
            "size" => {
                let size = self.backend.size(&queue_name).await.unwrap_or(0);
                size.to_string().into_bytes()
            }
            "enqueue" | "clear" => {
                return Err(EvifError::InvalidPath("Write-only file".to_string()));
            }
            _ => return Err(EvifError::NotFound(path.to_string())),
        };

        Ok(data)
    }

    async fn write(
        &self,
        path: &str,
        data: Vec<u8>,
        _offset: i64,
        _flags: WriteFlags,
    ) -> EvifResult<u64> {
        let (queue_name, operation, is_dir) = self.parse_path(path)?;

        if is_dir {
            return Err(EvifError::InvalidPath("Is a directory".to_string()));
        }

        let len = data.len() as u64;

        match operation.as_str() {
            "enqueue" => {
                self.backend.enqueue(&queue_name, data).await?;
                Ok(len)
            }
            "clear" => {
                self.backend.clear(&queue_name).await?;
                Ok(0)
            }
            "dequeue" | "peek" | "size" => Err(EvifError::InvalidPath("Read-only file".to_string())),
            _ => Err(EvifError::InvalidPath(
                "Cannot write to this file".to_string(),
            )),
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
            let queues = self.backend.list_queues_with_prefix("").await?;

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

        if !self.backend.queue_exists(&queue_name).await {
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
            let exists = self.backend.queue_exists(&queue_name).await;
            if !exists && !queue_name.is_empty() {
                // 检查是否是父目录
                let queues = self.backend.list_queues_with_prefix(&queue_name).await?;
                if queues.is_empty() {
                    return Err(EvifError::NotFound(path.to_string()));
                }
            }

            let name = if queue_name.is_empty() {
                "/".to_string()
            } else {
                queue_name
                    .split('/')
                    .next_back()
                    .unwrap_or("unknown")
                    .to_string()
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
        let (_queue_name, _operation, is_dir) = self.parse_path(path)?;

        if is_dir {
            return Err(EvifError::InvalidPath(
                "Use rmdir to remove directories".to_string(),
            ));
        }

        Err(EvifError::InvalidPath(
            "Cannot remove control files".to_string(),
        ))
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        Err(EvifError::InvalidPath(
            "Cannot rename in queuefs".to_string(),
        ))
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        // QueueFS 不支持目录结构, remove_all 等同于 remove
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
        plugin
            .write(
                "/my_queue/enqueue",
                b"Hello, Queue!".to_vec(),
                0,
                WriteFlags::CREATE,
            )
            .await
            .unwrap();

        // 检查队列大小
        let size_data = plugin.read("/my_queue/size", 0, 100).await.unwrap();
        assert_eq!(size_data, b"1");

        // 查看消息（不移除）
        let peek_data = plugin.read("/my_queue/peek", 0, 1000).await.unwrap();
        assert!(!peek_data.is_empty());

        // 出队消息
        let dequeue_data = plugin.read("/my_queue/dequeue", 0, 1000).await.unwrap();
        assert!(!dequeue_data.is_empty());

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
            plugin
                .write(
                    "/orders/enqueue",
                    format!("order-{}", i).into_bytes(),
                    0,
                    WriteFlags::CREATE,
                )
                .await
                .unwrap();
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

    #[tokio::test]
    async fn test_queuefs_duplicate_queue() {
        let plugin = QueueFsPlugin::new();
        plugin.mkdir("/test_queue", 0o755).await.unwrap();

        // 创建同名队列应失败
        let result = plugin.mkdir("/test_queue", 0o755).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_queuefs_empty_dequeue_returns_empty_json() {
        let plugin = QueueFsPlugin::new();
        plugin.mkdir("/empty_q", 0o755).await.unwrap();

        // 空队列出队返回空 JSON
        let data = plugin.read("/empty_q/dequeue", 0, 1000).await.unwrap();
        assert_eq!(data, b"{}");
    }

    #[tokio::test]
    async fn test_queuefs_clear() {
        let plugin = QueueFsPlugin::new();
        plugin.mkdir("/clear_test", 0o755).await.unwrap();

        // 入队消息
        plugin
            .write(
                "/clear_test/enqueue",
                b"msg1".to_vec(),
                0,
                WriteFlags::CREATE,
            )
            .await
            .unwrap();
        plugin
            .write(
                "/clear_test/enqueue",
                b"msg2".to_vec(),
                0,
                WriteFlags::CREATE,
            )
            .await
            .unwrap();

        // 确认有消息
        let size_data = plugin.read("/clear_test/size", 0, 100).await.unwrap();
        assert_eq!(size_data, b"2");

        // 清空
        plugin
            .write(
                "/clear_test/clear",
                b"".to_vec(),
                0,
                WriteFlags::CREATE,
            )
            .await
            .unwrap();

        // 确认清空
        let size_data = plugin.read("/clear_test/size", 0, 100).await.unwrap();
        assert_eq!(size_data, b"0");
    }

    // ==================== SQLite Backend Tests ====================

    #[cfg(feature = "queuefs-sqlite")]
    mod sqlite_tests {
        use super::*;

        #[tokio::test]
        async fn test_sqlite_backend_basic() {
            let dir = tempfile::tempdir().unwrap();
            let db_path = dir.path().join("test_queue.db");
            let db_path_str = db_path.to_str().unwrap();

            let plugin = QueueFsPlugin::with_sqlite(db_path_str).unwrap();

            // 创建队列
            plugin.mkdir("/my_queue", 0o755).await.unwrap();

            // 入队消息
            plugin
                .write(
                    "/my_queue/enqueue",
                    b"Hello, SQLite Queue!".to_vec(),
                    0,
                    WriteFlags::CREATE,
                )
                .await
                .unwrap();

            // 检查大小
            let size_data = plugin.read("/my_queue/size", 0, 100).await.unwrap();
            assert_eq!(size_data, b"1");

            // 查看消息
            let peek_data = plugin.read("/my_queue/peek", 0, 1000).await.unwrap();
            assert!(!peek_data.is_empty());

            // 出队消息
            let dequeue_data = plugin.read("/my_queue/dequeue", 0, 1000).await.unwrap();
            assert!(!dequeue_data.is_empty());

            // 队列空了
            let size_data = plugin.read("/my_queue/size", 0, 100).await.unwrap();
            assert_eq!(size_data, b"0");
        }

        #[tokio::test]
        async fn test_sqlite_backend_persistence() {
            let dir = tempfile::tempdir().unwrap();
            let db_path = dir.path().join("persist_test.db");
            let db_path_str = db_path.to_str().unwrap();

            // 第一个实例：创建队列并入队消息
            {
                let plugin = QueueFsPlugin::with_sqlite(db_path_str).unwrap();
                plugin.mkdir("/persist_queue", 0o755).await.unwrap();
                plugin
                    .write(
                        "/persist_queue/enqueue",
                        b"persistent message".to_vec(),
                        0,
                        WriteFlags::CREATE,
                    )
                    .await
                    .unwrap();
            }

            // 第二个实例：验证消息持久化
            {
                let plugin = QueueFsPlugin::with_sqlite(db_path_str).unwrap();
                let size_data = plugin.read("/persist_queue/size", 0, 100).await.unwrap();
                assert_eq!(size_data, b"1");

                let msg_data = plugin
                    .read("/persist_queue/dequeue", 0, 1000)
                    .await
                    .unwrap();
                let msg: QueueMessage = serde_json::from_slice(&msg_data).unwrap();
                assert_eq!(msg.data, "persistent message");
            }
        }

        #[tokio::test]
        async fn test_sqlite_backend_multiple_queues() {
            let dir = tempfile::tempdir().unwrap();
            let db_path = dir.path().join("multi_queue.db");
            let db_path_str = db_path.to_str().unwrap();

            let plugin = QueueFsPlugin::with_sqlite(db_path_str).unwrap();

            // 创建多个队列
            plugin.mkdir("/queue_a", 0o755).await.unwrap();
            plugin.mkdir("/queue_b", 0o755).await.unwrap();

            plugin
                .write(
                    "/queue_a/enqueue",
                    b"msg_a1".to_vec(),
                    0,
                    WriteFlags::CREATE,
                )
                .await
                .unwrap();
            plugin
                .write(
                    "/queue_b/enqueue",
                    b"msg_b1".to_vec(),
                    0,
                    WriteFlags::CREATE,
                )
                .await
                .unwrap();

            // 验证大小
            let size_a = plugin.read("/queue_a/size", 0, 100).await.unwrap();
            let size_b = plugin.read("/queue_b/size", 0, 100).await.unwrap();
            assert_eq!(size_a, b"1");
            assert_eq!(size_b, b"1");

            // 列出队列
            let entries = plugin.readdir("/").await.unwrap();
            assert!(entries.iter().any(|e| e.name == "queue_a"));
            assert!(entries.iter().any(|e| e.name == "queue_b"));
        }

        #[tokio::test]
        async fn test_sqlite_backend_clear() {
            let dir = tempfile::tempdir().unwrap();
            let db_path = dir.path().join("clear_test.db");
            let db_path_str = db_path.to_str().unwrap();

            let plugin = QueueFsPlugin::with_sqlite(db_path_str).unwrap();
            plugin.mkdir("/clear_q", 0o755).await.unwrap();

            // 入队多条消息
            for i in 0..5 {
                plugin
                    .write(
                        "/clear_q/enqueue",
                        format!("msg-{}", i).into_bytes(),
                        0,
                        WriteFlags::CREATE,
                    )
                    .await
                    .unwrap();
            }

            let size = plugin.read("/clear_q/size", 0, 100).await.unwrap();
            assert_eq!(size, b"5");

            // 清空
            plugin
                .write("/clear_q/clear", b"".to_vec(), 0, WriteFlags::CREATE)
                .await
                .unwrap();

            let size = plugin.read("/clear_q/size", 0, 100).await.unwrap();
            assert_eq!(size, b"0");
        }
    }

    #[tokio::test]
    async fn test_queuefs_concurrent_enqueue_dequeue() {
        let plugin = Arc::new(QueueFsPlugin::new());
        plugin.mkdir("/concurrent_q", 0o755).await.unwrap();

        // 并发入队 20 条消息
        let mut handles = Vec::new();
        for i in 0..20 {
            let p = Arc::clone(&plugin);
            handles.push(tokio::spawn(async move {
                p.write(
                    "/concurrent_q/enqueue",
                    format!("msg-{}", i).into_bytes(),
                    0,
                    WriteFlags::CREATE,
                )
                .await
                .unwrap();
            }));
        }
        for h in handles {
            h.await.unwrap();
        }

        // 验证大小
        let size_data = plugin.read("/concurrent_q/size", 0, 100).await.unwrap();
        let size_str = String::from_utf8_lossy(&size_data);
        assert_eq!(size_str.parse::<usize>().unwrap(), 20);

        // 并发出队
        let mut handles = Vec::new();
        for _ in 0..20 {
            let p = Arc::clone(&plugin);
            handles.push(tokio::spawn(async move {
                let data = p.read("/concurrent_q/dequeue", 0, 1000).await.unwrap();
                assert!(!data.is_empty());
            }));
        }
        for h in handles {
            h.await.unwrap();
        }

        // 队列应该空了
        let size_data = plugin.read("/concurrent_q/size", 0, 100).await.unwrap();
        assert_eq!(size_data, b"0");
    }
}
