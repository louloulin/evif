// GPTFS - OpenAI API 文件系统插件
//
// 基于 AGFS GPTFS 实现，提供异步 OpenAI API 调用能力
//
// 核心功能:
// - 异步 Job 队列处理
// - Worker Pool 并发控制
// - OpenAI API 集成
// - LocalFS 持久化存储
// - 重试机制和超时控制

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use evif_core::{EvifError, EvifPlugin, EvifResult, FileInfo, WriteFlags};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, Semaphore};
use uuid::Uuid;

// 导入 LocalFS 作为持久化存储
use crate::localfs::LocalFsPlugin;

/// GPTFS 配置
#[derive(Debug, Clone)]
pub struct GptfsConfig {
    /// OpenAI API Key
    pub api_key: String,

    /// API Host (用于自定义端点)
    pub api_host: Option<String>,

    /// 挂载路径
    pub mount_path: String,

    /// Worker 数量
    pub workers: usize,

    /// 请求超时 (秒)
    pub timeout: u64,

    /// 最大重试次数
    pub max_retries: usize,

    /// 持久化存储路径
    pub storage_path: String,
}

impl Default for GptfsConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            api_host: Some("https://api.openai.com/v1/chat/completions".to_string()),
            mount_path: "/gpt".to_string(),
            workers: 3,
            timeout: 60,
            max_retries: 3,
            storage_path: format!("/tmp/evif_gptfs_{}", Uuid::new_v4()),
        }
    }
}

/// Job 状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "processing")]
    Processing,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed(String),
}

/// Job 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub request_path: String,
    pub response_path: String,
    pub data: Vec<u8>,
    pub timestamp: DateTime<Utc>,
    pub status: JobStatus,
    pub duration: Option<u64>, // 毫秒
    pub error: Option<String>,
}

/// Job 状态请求 (用于写入状态文件)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRequest {
    pub job_id: String,
    pub status: String,
    pub timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// GPTFS 插件
pub struct GptfsPlugin {
    config: GptfsConfig,
    base_fs: Arc<LocalFsPlugin>, // 用于持久化

    // Job 管理
    jobs: Arc<RwLock<HashMap<String, Job>>>,
    job_queue: Arc<Mutex<Vec<String>>>, // Job ID 队列
    semaphore: Arc<Semaphore>, // 并发限制

    // 后台任务控制
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
}

impl GptfsPlugin {
    pub async fn new(config: GptfsConfig) -> EvifResult<Self> {
        // 创建基础文件系统用于持久化
        let base_fs = Arc::new(LocalFsPlugin::new(&config.storage_path));

        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);

        let plugin = Self {
            config,
            base_fs,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            job_queue: Arc::new(Mutex::new(Vec::new())),
            semaphore: Arc::new(Semaphore::new(3)), // 最多3个并发请求
            shutdown_tx,
        };

        // 启动 worker pool
        plugin.start_workers().await;

        Ok(plugin)
    }

    /// 启动 worker pool
    async fn start_workers(&self) {
        for worker_id in 0..self.config.workers {
            let jobs = Arc::clone(&self.jobs);
            let job_queue = Arc::clone(&self.job_queue);
            let semaphore = Arc::clone(&self.semaphore);
            let base_fs = Arc::clone(&self.base_fs);
            let timeout = self.config.timeout;
            let max_retries = self.config.max_retries;
            let api_key = self.config.api_key.clone();
            let api_host = self.config.api_host.clone();
            let mut shutdown_rx = self.shutdown_tx.subscribe();

            tokio::spawn(async move {
                log::info!("GPTFS worker {} started", worker_id);

                loop {
                    // 检查关闭信号或获取任务
                    tokio::select! {
                        _ = shutdown_rx.recv() => {
                            log::info!("GPTFS worker {} shutting down", worker_id);
                            break;
                        }
                        _ = semaphore.acquire() => {
                            // 获取下一个 job
                            let job_id = {
                                let mut queue = job_queue.lock().await;
                                if queue.is_empty() {
                                    continue;
                                }
                                queue.remove(0)
                            };

                            // 处理 job
                            let job = {
                                let mut jobs = jobs.write().await;
                                jobs.get_mut(&job_id).map(|j| {
                                    j.status = JobStatus::Processing;
                                    j.clone()
                                })
                            };

                            if let Some(mut job) = job {
                                log::info!("Worker {} processing job {}", worker_id, job.id);

                                // 调用 OpenAI API
                                let start = std::time::Instant::now();
                                let result = tokio::time::timeout(
                                    tokio::time::Duration::from_secs(timeout),
                                    Self::call_openai(&api_key, api_host.as_deref(), &job.data, max_retries)
                                ).await;

                                match result {
                                    Ok(Ok(response)) => {
                                        let duration = start.elapsed().as_millis() as u64;
                                        job.status = JobStatus::Completed;
                                        job.duration = Some(duration);
                                        job.error = None;

                                        // 保存响应
                                        let _ = base_fs.write(
                                            &job.response_path,
                                            response.clone(),
                                            0,
                                            WriteFlags::CREATE | WriteFlags::TRUNCATE
                                        ).await;

                                        // 更新 job
                                        let mut jobs = jobs.write().await;
                                        jobs.insert(job_id.clone(), job);

                                        log::info!("Job {} completed in {}ms", job_id, duration);
                                    }
                                    Ok(Err(e)) => {
                                        job.status = JobStatus::Failed(e.to_string());
                                        job.duration = Some(start.elapsed().as_millis() as u64);
                                        job.error = Some(e.to_string());

                                        let mut jobs = jobs.write().await;
                                        jobs.insert(job_id.clone(), job);

                                        log::error!("Job {} failed: {}", job_id, e);
                                    }
                                    Err(_) => {
                                        let e = "Timeout".to_string();
                                        job.status = JobStatus::Failed(e.clone());
                                        job.duration = Some(start.elapsed().as_millis() as u64);
                                        job.error = Some(e.clone());

                                        let mut jobs = jobs.write().await;
                                        jobs.insert(job_id.clone(), job);

                                        log::error!("Job {} timed out", job_id);
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }

        log::info!("GPTFS started {} workers", self.config.workers);
    }

    /// 调用 OpenAI API (带重试)
    async fn call_openai(
        api_key: &str,
        api_host: Option<&str>,
        prompt_data: &[u8],
        max_retries: usize,
    ) -> EvifResult<Vec<u8>> {
        let prompt = String::from_utf8(prompt_data.to_vec())
            .map_err(|_| EvifError::InvalidPath("Invalid UTF-8 prompt".to_string()))?;

        let host = api_host.unwrap_or("https://api.openai.com/v1/chat/completions");

        let mut retries = 0;
        loop {
            match Self::try_openai(api_key, host, &prompt).await {
                Ok(response) => return Ok(response.into_bytes()),
                Err(e) if retries < max_retries => {
                    retries += 1;
                    log::warn!("OpenAI API failed (attempt {}/{}): {}", retries, max_retries, e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000 * retries as u64)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// 单次 OpenAI API 调用
    async fn try_openai(api_key: &str, host: &str, prompt: &str) -> EvifResult<String> {
        let client = reqwest::Client::new();

        // 构建请求体
        let request_body = serde_json::json!({
            "model": "gpt-3.5-turbo",
            "messages": [
                {
                    "role": "system",
                    "content": "You are a helpful assistant."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.7
        });

        // 发送请求
        let response = client
            .post(host)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .timeout(tokio::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| EvifError::Other(format!("OpenAI API request failed: {}", e)))?;

        // 检查响应状态
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(EvifError::Other(format!("OpenAI API error {}: {}", status, body)));
        }

        // 解析响应
        #[derive(Deserialize)]
        struct OpenAIResponse {
            choices: Vec<Choice>,
        }

        #[derive(Deserialize)]
        struct Choice {
            message: Message,
        }

        #[derive(Deserialize)]
        struct Message {
            content: String,
        }

        let openai_resp: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| EvifError::Other(format!("Failed to parse OpenAI response: {}", e)))?;

        let content = openai_resp
            .choices
            .first()
            .and_then(|c| Some(c.message.content.clone()))
            .ok_or_else(|| EvifError::Other("Empty OpenAI response".to_string()))?;

        Ok(content)
    }

    /// 生成 Job ID
    fn generate_job_id() -> String {
        format!("job_{}", Uuid::new_v4())
    }

    /// 写入状态文件
    async fn write_job_status(&self, status_file: &str, req: JobRequest) {
        let data = match serde_json::to_string_pretty(&req) {
            Ok(d) => d,
            Err(e) => {
                log::error!("Failed to marshal job status: {}", e);
                return;
            }
        };

        let _ = self.base_fs.write(
            status_file,
            data.into_bytes(),
            0,
            WriteFlags::CREATE | WriteFlags::TRUNCATE
        ).await;
    }

    /// 获取心跳状态
    pub async fn get_job_status(&self, name: &str) -> EvifResult<Job> {
        let jobs = self.jobs.read().await;
        jobs.get(name)
            .cloned()
            .ok_or_else(|| EvifError::NotFound(name.to_string()))
    }
}

#[async_trait]
impl EvifPlugin for GptfsPlugin {
    fn name(&self) -> &str {
        "GPTFS"
    }

    async fn create(&self, path: &str, _perm: u32) -> EvifResult<()> {
        // 允许创建 request 文件
        if path.ends_with("/request") {
            Ok(())
        } else {
            Err(EvifError::InvalidPath("Only /request files can be created".to_string()))
        }
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        // 自动创建 channel 目录
        Ok(())
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        let path = path.trim_start_matches('/');

        // 读取 response 文件 - 从持久化存储
        if path.ends_with("/response") {
            return self.base_fs.read(path, 0, 0).await;
        }

        // 读取 status 文件
        if path.ends_with("/status") {
            let channel = path
                .trim_start_matches("gpt/")
                .trim_end_matches("/status");

            let jobs = self.jobs.read().await;
            let job = jobs
                .values()
                .find(|j| j.request_path == format!("/gpt/{}/request", channel))
                .ok_or_else(|| EvifError::NotFound(path.to_string()))?;

            let job_req = JobRequest {
                job_id: job.id.clone(),
                status: match &job.status {
                    JobStatus::Pending => "pending".to_string(),
                    JobStatus::Processing => "processing".to_string(),
                    JobStatus::Completed => "completed".to_string(),
                    JobStatus::Failed(e) => format!("failed: {}", e),
                },
                timestamp: job.timestamp.timestamp(),
                message: job.error.clone(),
            };

            let status_json = serde_json::to_string_pretty(&job_req)
                .map_err(|e| EvifError::Other(format!("JSON serialize error: {}", e)))?;

            return Ok(status_json.into_bytes());
        }

        Err(EvifError::NotFound(path.to_string()))
    }

    async fn write(&self, path: &str, data: Vec<u8>, _offset: i64, _flags: WriteFlags) -> EvifResult<u64> {
        let path = path.trim_start_matches('/');

        // 写入 request 文件 -> 创建新 job
        if path.ends_with("/request") {
            let job_id = Self::generate_job_id();
            let channel = path
                .trim_start_matches("gpt/")
                .trim_end_matches("/request");

            let request_path = format!("/gpt/{}/request", channel);
            let response_path = format!("/gpt/{}/response.txt", channel);
            let status_path = format!("/gpt/{}/status.json", channel);

            let job = Job {
                id: job_id.clone(),
                request_path: request_path.clone(),
                response_path,
                data: data.clone(),
                timestamp: Utc::now(),
                status: JobStatus::Pending,
                duration: None,
                error: None,
            };

            // 保存 job
            let mut jobs = self.jobs.write().await;
            jobs.insert(job_id.clone(), job.clone());

            // 写入初始状态
            self.write_job_status(
                &status_path,
                JobRequest {
                    job_id: job_id.clone(),
                    status: "pending".to_string(),
                    timestamp: Utc::now().timestamp(),
                    message: Some("Job queued for processing".to_string()),
                },
            ).await;

            // 添加到队列
            let mut queue = self.job_queue.lock().await;
            queue.push(job_id.clone());

            log::info!("Job {} queued for processing", job_id);

            Ok(data.len() as u64)
        } else {
            Err(EvifError::InvalidPath("Write to /request files only".to_string()))
        }
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let path = path.trim_start_matches('/');

        if path == "gpt" || path.is_empty() {
            let jobs = self.jobs.read().await;
            let channels: std::collections::HashSet<String> = jobs
                .values()
                .map(|j| {
                    j.request_path
                        .trim_start_matches("/gpt/")
                        .trim_end_matches("/request")
                        .to_string()
                })
                .collect();

            let mut entries = Vec::new();
            for channel in channels {
                entries.push(FileInfo {
                    name: channel.clone(),
                    size: 0,
                    mode: 0o755,
                    modified: Utc::now(),
                    is_dir: true,
                });
            }
            Ok(entries)
        } else {
            Err(EvifError::NotFound(path.to_string()))
        }
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        if path.ends_with("/request") || path.ends_with("/response") || path.ends_with("/status") {
            let name = path.split('/').last().unwrap_or("").to_string();
            Ok(FileInfo {
                name,
                size: 0,
                mode: 0o644,
                modified: Utc::now(),
                is_dir: false,
            })
        } else {
            Err(EvifError::NotFound(path.to_string()))
        }
    }

    async fn remove(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::NotSupported)
    }

    async fn remove_all(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::NotSupported)
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        Err(EvifError::NotSupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[cfg(feature = "gptfs")]
    async fn test_gptfs_basic() {
        // 跳过如果没有 API key
        if std::env::var("OPENAI_API_KEY").is_err() {
            return;
        }

        let config = GptfsConfig::default();
        let plugin = GptfsPlugin::new(config).await.unwrap();

        // 创建目录
        plugin.mkdir("/gpt/test", 0o755).await.unwrap();

        // 创建 request 文件
        plugin.create("/gpt/test/request", 0o644).await.unwrap();

        // 写入请求
        let prompt = b"Hello, GPT!";
        plugin.write("/gpt/test/request", prompt.to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        // 等待处理完成
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // 读取状态
        let status = plugin.read("/gpt/test/status", 0, 0).await;
        assert!(status.is_ok());

        // 如果完成,读取响应
        if let Ok(status_data) = status {
            let status_str = String::from_utf8(status_data).unwrap();
            if status_str.contains("completed") {
                let response = plugin.read("/gpt/test/response.txt", 0, 0).await;
                assert!(response.is_ok());
            }
        }
    }

    #[tokio::test]
    async fn test_gptfs_readdir() {
        let config = GptfsConfig::default();
        let plugin = GptfsPlugin::new(config).await.unwrap();

        // 创建多个 job
        plugin.mkdir("/gpt/channel1", 0o755).await.unwrap();
        plugin.write("/gpt/channel1/request", b"test1".to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        plugin.mkdir("/gpt/channel2", 0o755).await.unwrap();
        plugin.write("/gpt/channel2/request", b"test2".to_vec(), 0, WriteFlags::CREATE).await.unwrap();

        // 列出目录
        let entries = plugin.readdir("/gpt").await.unwrap();
        assert!(entries.len() >= 2);
        assert!(entries.iter().any(|e| e.name == "channel1"));
        assert!(entries.iter().any(|e| e.name == "channel2"));
    }
}
