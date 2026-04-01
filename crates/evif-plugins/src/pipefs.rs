use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::Utc;
use evif_core::{EvifError, EvifPlugin, EvifResult, FileInfo, PluginConfigParam, WriteFlags};
use tokio::sync::RwLock;

use crate::queuefs::QueueBackend;

#[derive(Clone)]
struct PipeRecord {
    input: Vec<u8>,
    output: Vec<u8>,
    status: String,
    assignee: String,
    timeout_secs: u64,
    updated_at: Instant,
}

impl PipeRecord {
    fn new() -> Self {
        Self {
            input: Vec::new(),
            output: Vec::new(),
            status: "pending".to_string(),
            assignee: String::new(),
            timeout_secs: 300,
            updated_at: Instant::now(),
        }
    }

    fn expired(&self) -> bool {
        self.updated_at.elapsed() >= Duration::from_secs(self.timeout_secs)
    }
}

pub struct PipeFsPlugin {
    pipes: RwLock<HashMap<String, PipeRecord>>,
    subscribers: RwLock<HashMap<String, Vec<u8>>>,
    backend: Option<Arc<dyn QueueBackend>>,
}

impl PipeFsPlugin {
    pub fn new() -> Self {
        Self {
            pipes: RwLock::new(HashMap::new()),
            subscribers: RwLock::new(HashMap::new()),
            backend: None,
        }
    }

    /// Create a PipeFsPlugin with an optional persistence backend.
    ///
    /// When a backend is provided, pipe input/output messages are persisted
    /// through the backend so they survive across PipeFsPlugin instances.
    pub fn new_with_backend(backend: Arc<dyn QueueBackend>) -> Self {
        Self {
            pipes: RwLock::new(HashMap::new()),
            subscribers: RwLock::new(HashMap::new()),
            backend: Some(backend),
        }
    }

    fn readme_text(&self) -> String {
        r#"PipeFS Plugin - Agent pipe coordination

Directories created at the root become bidirectional pipes:
- input
- output
- status
- assignee
- timeout

Broadcast channels live under /broadcast/subscribers/<name>/output
"#
        .to_string()
    }

    async fn cleanup_expired(&self) {
        let mut pipes = self.pipes.write().await;
        pipes.retain(|_, pipe| !pipe.expired());
    }

    fn parts(path: &str) -> Vec<&str> {
        path.trim_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect()
    }

    fn scalar_info(name: &str, is_dir: bool, size: usize) -> FileInfo {
        FileInfo {
            name: name.to_string(),
            size: size as u64,
            mode: if is_dir { 0o755 } else { 0o644 },
            modified: Utc::now(),
            is_dir,
        }
    }

    async fn list_root(&self) -> Vec<FileInfo> {
        let mut entries = vec![
            Self::scalar_info("README", false, self.readme_text().len()),
            Self::scalar_info("broadcast", true, 0),
        ];

        let pipes = self.pipes.read().await;
        for name in pipes.keys() {
            entries.push(Self::scalar_info(name, true, 0));
        }
        entries
    }

    async fn pipe_info(&self, name: &str) -> EvifResult<PipeRecord> {
        self.cleanup_expired().await;
        let pipes = self.pipes.read().await;
        pipes
            .get(name)
            .cloned()
            .ok_or_else(|| EvifError::NotFound(name.to_string()))
    }

    /// Ensure a pipe record exists in memory. If missing but a backend is
    /// present and the backend queues exist, materialize a fresh record so
    /// reads can fall through to the backend.
    async fn ensure_pipe(&self, name: &str) -> EvifResult<()> {
        {
            let pipes = self.pipes.read().await;
            if pipes.contains_key(name) {
                return Ok(());
            }
        }
        // Not in memory; check backend
        if let Some(ref backend) = self.backend {
            if backend.queue_exists(&format!("pipe:{}:input", name)).await {
                let mut pipes = self.pipes.write().await;
                // Double-check after acquiring write lock
                if !pipes.contains_key(name) {
                    pipes.insert(name.to_string(), PipeRecord::new());
                }
                return Ok(());
            }
        }
        Err(EvifError::NotFound(name.to_string()))
    }
}

impl Default for PipeFsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EvifPlugin for PipeFsPlugin {
    fn name(&self) -> &str {
        "pipefs"
    }

    fn get_readme(&self) -> String {
        self.readme_text()
    }

    fn get_config_params(&self) -> Vec<PluginConfigParam> {
        vec![]
    }

    async fn create(&self, path: &str, _perm: u32) -> EvifResult<()> {
        let parts = Self::parts(path);
        if parts.len() == 2 {
            return Ok(());
        }
        Err(EvifError::InvalidPath(path.to_string()))
    }

    async fn mkdir(&self, path: &str, _perm: u32) -> EvifResult<()> {
        self.cleanup_expired().await;
        let parts = Self::parts(path);
        match parts.as_slice() {
            [pipe_name] if *pipe_name != "broadcast" => {
                let mut pipes = self.pipes.write().await;
                if pipes.contains_key(*pipe_name) {
                    return Err(EvifError::AlreadyExists((*pipe_name).to_string()));
                }
                pipes.insert((*pipe_name).to_string(), PipeRecord::new());
                drop(pipes);
                // Persist pipe creation to backend
                if let Some(ref backend) = self.backend {
                    let _ = backend.create_queue(&format!("pipe:{}:input", pipe_name)).await;
                    let _ = backend.create_queue(&format!("pipe:{}:output", pipe_name)).await;
                }
                Ok(())
            }
            ["broadcast"] | ["broadcast", "subscribers"] => Ok(()),
            ["broadcast", "subscribers", subscriber] => {
                self.subscribers
                    .write()
                    .await
                    .entry((*subscriber).to_string())
                    .or_default();
                Ok(())
            }
            _ => Err(EvifError::InvalidPath(path.to_string())),
        }
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        self.cleanup_expired().await;
        let parts = Self::parts(path);
        match parts.as_slice() {
            ["README"] => Ok(self.readme_text().into_bytes()),
            [pipe_name, field] if *pipe_name != "broadcast" => {
                self.ensure_pipe(pipe_name).await?;
                let pipe = self.pipe_info(pipe_name).await?;
                match *field {
                    "input" => {
                        if !pipe.input.is_empty() {
                            return Ok(pipe.input);
                        }
                        // Fall back to backend if in-memory is empty
                        if let Some(ref backend) = self.backend {
                            if let Ok(msg) = backend.peek(&format!("pipe:{}:input", pipe_name)).await {
                                return Ok(msg.data.into_bytes());
                            }
                        }
                        Ok(pipe.input)
                    }
                    "output" => {
                        if !pipe.output.is_empty() {
                            return Ok(pipe.output);
                        }
                        // Fall back to backend if in-memory is empty
                        if let Some(ref backend) = self.backend {
                            if let Ok(msg) = backend.peek(&format!("pipe:{}:output", pipe_name)).await {
                                return Ok(msg.data.into_bytes());
                            }
                        }
                        Ok(pipe.output)
                    }
                    "status" => Ok(pipe.status.into_bytes()),
                    "assignee" => Ok(pipe.assignee.into_bytes()),
                    "timeout" => Ok(pipe.timeout_secs.to_string().into_bytes()),
                    _ => Err(EvifError::NotFound(path.to_string())),
                }
            }
            ["broadcast", "input"] => Ok(Vec::new()),
            ["broadcast", "subscribers", subscriber, "output"] => self
                .subscribers
                .read()
                .await
                .get(*subscriber)
                .cloned()
                .ok_or_else(|| EvifError::NotFound(path.to_string())),
            _ => Err(EvifError::NotFound(path.to_string())),
        }
    }

    async fn write(
        &self,
        path: &str,
        data: Vec<u8>,
        _offset: i64,
        _flags: WriteFlags,
    ) -> EvifResult<u64> {
        self.cleanup_expired().await;
        let parts = Self::parts(path);
        match parts.as_slice() {
            [pipe_name, field] if *pipe_name != "broadcast" => {
                let mut pipes = self.pipes.write().await;
                let pipe = pipes
                    .get_mut(*pipe_name)
                    .ok_or_else(|| EvifError::NotFound((*pipe_name).to_string()))?;
                pipe.updated_at = Instant::now();
                let data_len = data.len() as u64;
                match *field {
                    "input" => {
                        pipe.input = data.clone();
                        if pipe.status == "pending" {
                            pipe.status = "running".to_string();
                        }
                        // Persist to backend
                        if let Some(ref backend) = self.backend {
                            let _ = backend.enqueue(&format!("pipe:{}:input", pipe_name), data.clone()).await;
                        }
                    }
                    "output" => {
                        pipe.output = data.clone();
                        pipe.status = "completed".to_string();
                        // Persist to backend
                        if let Some(ref backend) = self.backend {
                            let _ = backend.enqueue(&format!("pipe:{}:output", pipe_name), data.clone()).await;
                        }
                    }
                    "status" => {
                        pipe.status = String::from_utf8(data.clone())
                            .map_err(|err| EvifError::InvalidInput(err.to_string()))?;
                    }
                    "assignee" => {
                        pipe.assignee = String::from_utf8(data.clone())
                            .map_err(|err| EvifError::InvalidInput(err.to_string()))?;
                    }
                    "timeout" => {
                        let value = String::from_utf8(data.clone())
                            .map_err(|err| EvifError::InvalidInput(err.to_string()))?;
                        pipe.timeout_secs = value
                            .trim()
                            .parse::<u64>()
                            .map_err(|err| EvifError::InvalidInput(err.to_string()))?;
                    }
                    _ => return Err(EvifError::NotFound(path.to_string())),
                }
                Ok(data_len)
            }
            ["broadcast", "input"] => {
                let mut subscribers = self.subscribers.write().await;
                for output in subscribers.values_mut() {
                    *output = data.clone();
                }
                Ok(data.len() as u64)
            }
            _ => Err(EvifError::InvalidPath(path.to_string())),
        }
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        self.cleanup_expired().await;
        let parts = Self::parts(path);
        match parts.as_slice() {
            [] => Ok(self.list_root().await),
            ["broadcast"] => Ok(vec![
                Self::scalar_info("input", false, 0),
                Self::scalar_info("subscribers", true, 0),
            ]),
            ["broadcast", "subscribers"] => {
                let subscribers = self.subscribers.read().await;
                Ok(subscribers
                    .keys()
                    .map(|name| Self::scalar_info(name, true, 0))
                    .collect())
            }
            ["broadcast", "subscribers", subscriber] => {
                let subscribers = self.subscribers.read().await;
                let data = subscribers
                    .get(*subscriber)
                    .ok_or_else(|| EvifError::NotFound(path.to_string()))?;
                Ok(vec![Self::scalar_info("output", false, data.len())])
            }
            [pipe_name] if *pipe_name != "broadcast" => {
                self.pipe_info(pipe_name).await?;
                Ok(vec![
                    Self::scalar_info("input", false, 0),
                    Self::scalar_info("output", false, 0),
                    Self::scalar_info("status", false, 0),
                    Self::scalar_info("assignee", false, 0),
                    Self::scalar_info("timeout", false, 0),
                ])
            }
            _ => Err(EvifError::NotFound(path.to_string())),
        }
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        self.cleanup_expired().await;
        let parts = Self::parts(path);
        match parts.as_slice() {
            [] => Ok(Self::scalar_info("/", true, 0)),
            ["README"] => Ok(Self::scalar_info("README", false, self.readme_text().len())),
            ["broadcast"] => Ok(Self::scalar_info("broadcast", true, 0)),
            ["broadcast", "subscribers"] => Ok(Self::scalar_info("subscribers", true, 0)),
            ["broadcast", "subscribers", subscriber] => {
                let subscribers = self.subscribers.read().await;
                if subscribers.contains_key(*subscriber) {
                    Ok(Self::scalar_info(subscriber, true, 0))
                } else {
                    Err(EvifError::NotFound(path.to_string()))
                }
            }
            ["broadcast", "subscribers", subscriber, "output"] => {
                let subscribers = self.subscribers.read().await;
                let size = subscribers
                    .get(*subscriber)
                    .map(|data| data.len())
                    .ok_or_else(|| EvifError::NotFound(path.to_string()))?;
                Ok(Self::scalar_info("output", false, size))
            }
            [pipe_name] if *pipe_name != "broadcast" => {
                self.pipe_info(pipe_name).await?;
                Ok(Self::scalar_info(pipe_name, true, 0))
            }
            [pipe_name, field] if *pipe_name != "broadcast" => {
                let pipe = self.pipe_info(pipe_name).await?;
                let size = match *field {
                    "input" => pipe.input.len(),
                    "output" => pipe.output.len(),
                    "status" => pipe.status.len(),
                    "assignee" => pipe.assignee.len(),
                    "timeout" => pipe.timeout_secs.to_string().len(),
                    _ => return Err(EvifError::NotFound(path.to_string())),
                };
                Ok(Self::scalar_info(field, false, size))
            }
            _ => Err(EvifError::NotFound(path.to_string())),
        }
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        self.cleanup_expired().await;
        let parts = Self::parts(path);
        match parts.as_slice() {
            [pipe_name] if *pipe_name != "broadcast" => {
                let removed = self.pipes.write().await.remove(*pipe_name);
                // Clean up backend queues
                if let Some(ref backend) = self.backend {
                    let _ = backend.remove_queue(&format!("pipe:{}:input", pipe_name)).await;
                    let _ = backend.remove_queue(&format!("pipe:{}:output", pipe_name)).await;
                }
                removed
                    .map(|_| ())
                    .ok_or_else(|| EvifError::NotFound(path.to_string()))
            }
            ["broadcast", "subscribers", subscriber] => {
                let removed = self.subscribers.write().await.remove(*subscriber);
                removed
                    .map(|_| ())
                    .ok_or_else(|| EvifError::NotFound(path.to_string()))
            }
            _ => Err(EvifError::NotSupportedGeneric),
        }
    }

    async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()> {
        self.cleanup_expired().await;
        let old_parts = Self::parts(old_path);
        let new_parts = Self::parts(new_path);

        match (old_parts.as_slice(), new_parts.as_slice()) {
            ([old_name], [new_name]) if *old_name != "broadcast" && *new_name != "broadcast" => {
                let mut pipes = self.pipes.write().await;
                let record = pipes
                    .remove(*old_name)
                    .ok_or_else(|| EvifError::NotFound(old_path.to_string()))?;
                pipes.insert((*new_name).to_string(), record);
                Ok(())
            }
            _ => Err(EvifError::NotSupportedGeneric),
        }
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        self.remove(path).await
    }
}
