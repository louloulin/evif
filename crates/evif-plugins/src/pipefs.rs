use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::Utc;
use evif_core::{EvifError, EvifPlugin, EvifResult, FileInfo, PluginConfigParam, WriteFlags};
use tokio::sync::{Notify, RwLock};

use crate::queuefs::QueueBackend;

/// Valid pipe states for the state machine.
const STATE_PENDING: &str = "pending";
const STATE_RUNNING: &str = "running";
const STATE_COMPLETED: &str = "completed";
const STATE_ERROR: &str = "error";
const STATE_TIMEOUT: &str = "timeout";

/// Valid state transitions: from -> set of valid next states.
fn valid_next_states(current: &str) -> &'static [&'static str] {
    match current {
        STATE_PENDING => &[STATE_RUNNING, STATE_ERROR, STATE_TIMEOUT],
        STATE_RUNNING => &[STATE_COMPLETED, STATE_ERROR, STATE_TIMEOUT],
        STATE_COMPLETED => &[STATE_RUNNING], // allow re-use
        STATE_ERROR => &[STATE_PENDING],      // allow retry
        STATE_TIMEOUT => &[STATE_PENDING],    // allow retry
        _ => &[],
    }
}

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
    /// Per-pipe notification signal. Writers notify when output is written.
    notifiers: RwLock<HashMap<String, Arc<Notify>>>,
    backend: Option<Arc<dyn QueueBackend>>,
}

impl Clone for PipeFsPlugin {
    fn clone(&self) -> Self {
        Self {
            pipes: RwLock::new(HashMap::new()),
            subscribers: RwLock::new(HashMap::new()),
            notifiers: RwLock::new(HashMap::new()),
            backend: self.backend.clone(),
        }
    }
}

impl PipeFsPlugin {
    pub fn new() -> Self {
        Self {
            pipes: RwLock::new(HashMap::new()),
            subscribers: RwLock::new(HashMap::new()),
            notifiers: RwLock::new(HashMap::new()),
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
            notifiers: RwLock::new(HashMap::new()),
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

State machine: pending -> running -> completed
               pending -> error / timeout
               error/timeout -> pending (retry)
               completed -> running (re-use)

Atomic claim: write to /<pipe>/claim only succeeds if assignee is empty.
Wait for result: use wait_for_result() to block until output is written.
"#
        .to_string()
    }

    /// Get or create a Notify for a pipe name.
    async fn get_notifier(&self, name: &str) -> Arc<Notify> {
        let notifiers = self.notifiers.read().await;
        if let Some(notify) = notifiers.get(name) {
            return Arc::clone(notify);
        }
        drop(notifiers);
        let mut notifiers = self.notifiers.write().await;
        Arc::clone(notifiers.entry(name.to_string()).or_insert_with(|| Arc::new(Notify::new())))
    }

    /// Atomically claim a pipe. Only succeeds if the pipe has no assignee.
    ///
    /// Returns Ok(()) if the claim succeeded, or an error if already claimed.
    /// This prevents two agents from claiming the same pipe simultaneously.
    pub async fn try_claim(&self, pipe_name: &str, agent_id: &str) -> EvifResult<()> {
        let mut pipes = self.pipes.write().await;
        let pipe = pipes
            .get_mut(pipe_name)
            .ok_or_else(|| EvifError::NotFound(pipe_name.to_string()))?;

        if !pipe.assignee.is_empty() && pipe.assignee != agent_id {
            return Err(EvifError::InvalidInput(format!(
                "Pipe '{}' already claimed by '{}'",
                pipe_name, pipe.assignee
            )));
        }

        pipe.assignee = agent_id.to_string();
        pipe.updated_at = Instant::now();
        Ok(())
    }

    /// Wait for a pipe to have output written. Blocks until the pipe reaches
    /// "completed", "error", or "timeout" state, or until the timeout expires.
    ///
    /// Returns the pipe's output bytes, or an error on timeout.
    pub async fn wait_for_result(
        &self,
        pipe_name: &str,
        timeout: Duration,
    ) -> EvifResult<Vec<u8>> {
        // First check if already completed
        {
            let pipes = self.pipes.read().await;
            if let Some(pipe) = pipes.get(pipe_name) {
                if pipe.status == STATE_COMPLETED {
                    return Ok(pipe.output.clone());
                }
                if pipe.status == STATE_ERROR || pipe.status == STATE_TIMEOUT {
                    return Err(EvifError::InvalidInput(format!(
                        "Pipe '{}' in {} state",
                        pipe_name, pipe.status
                    )));
                }
            } else {
                return Err(EvifError::NotFound(pipe_name.to_string()));
            }
        }

        // Wait for notification with timeout
        let notify = self.get_notifier(pipe_name).await;
        match tokio::time::timeout(timeout, notify.notified()).await {
            Ok(()) => {
                let pipes = self.pipes.read().await;
                let pipe = pipes
                    .get(pipe_name)
                    .ok_or_else(|| EvifError::NotFound(pipe_name.to_string()))?;
                if pipe.status == STATE_COMPLETED {
                    Ok(pipe.output.clone())
                } else {
                    Err(EvifError::InvalidInput(format!(
                        "Pipe '{}' in {} state after notification",
                        pipe_name, pipe.status
                    )))
                }
            }
            Err(_) => {
                // Timeout - update pipe state
                let mut pipes = self.pipes.write().await;
                if let Some(pipe) = pipes.get_mut(pipe_name) {
                    if pipe.status == STATE_RUNNING {
                        pipe.status = STATE_TIMEOUT.to_string();
                    }
                }
                Err(EvifError::InvalidInput(format!(
                    "Pipe '{}' timed out after {:?}",
                    pipe_name, timeout
                )))
            }
        }
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
                        if pipe.status == STATE_PENDING {
                            pipe.status = STATE_RUNNING.to_string();
                        }
                        // Persist to backend
                        if let Some(ref backend) = self.backend {
                            let _ = backend.enqueue(&format!("pipe:{}:input", pipe_name), data.clone()).await;
                        }
                    }
                    "output" => {
                        pipe.output = data.clone();
                        pipe.status = STATE_COMPLETED.to_string();
                        // Persist to backend
                        if let Some(ref backend) = self.backend {
                            let _ = backend.enqueue(&format!("pipe:{}:output", pipe_name), data.clone()).await;
                        }
                        // Drop write lock before notifying
                        drop(pipes);
                        // Notify waiters that output is ready
                        let notify = {
                            let notifiers = self.notifiers.read().await;
                            notifiers.get(*pipe_name).cloned()
                        };
                        if let Some(notify) = notify {
                            notify.notify_waiters();
                        }
                    }
                    "status" => {
                        let new_status = String::from_utf8(data.clone())
                            .map_err(|err| EvifError::InvalidInput(err.to_string()))?;
                        // Validate state transition
                        if !valid_next_states(&pipe.status).contains(&new_status.as_str()) {
                            return Err(EvifError::InvalidInput(format!(
                                "Invalid state transition from '{}' to '{}'",
                                pipe.status, new_status
                            )));
                        }
                        pipe.status = new_status;
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
