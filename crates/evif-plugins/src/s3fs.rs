// S3FS - AWS S3 云存储插件
//
// 提供完整的 AWS S3 和 S3 兼容存储 (MinIO, LocalStack 等) 支持

use evif_core::{EvifPlugin, FileInfo, WriteFlags, EvifResult, EvifError};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use chrono::{DateTime, Utc};

#[cfg(feature = "s3fs")]
use aws_config::BehaviorVersion;
#[cfg(feature = "s3fs")]
use aws_sdk_s3::Client;
#[cfg(feature = "s3fs")]
use aws_sdk_s3::primitives::ByteStream;
#[cfg(feature = "s3fs")]
use aws_credential_types::Credentials;

/// S3 配置
#[derive(Clone, Debug)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub endpoint: Option<String>,
    pub prefix: Option<String>,
    pub disable_ssl: bool,
    pub force_path_style: bool,
    /// 分片上传阈值（字节），超过此大小自动使用分片上传（默认 8MB）
    pub multipart_threshold: u64,
    /// 每个分片大小（字节），默认 8MB
    pub multipart_chunk_size: u64,
}

impl Default for S3Config {
    fn default() -> Self {
        Self {
            bucket: String::new(),
            region: "us-east-1".to_string(),
            access_key_id: None,
            secret_access_key: None,
            endpoint: None,
            prefix: None,
            disable_ssl: false,
            force_path_style: false,
            multipart_threshold: 8 * 1024 * 1024,       // 8MB
            multipart_chunk_size: 8 * 1024 * 1024,       // 8MB
        }
    }
}

/// 缓存条目
struct CacheEntry<T> {
    data: T,
    created_at: Instant,
}

impl<T> CacheEntry<T> {
    fn new(data: T) -> Self {
        Self {
            data,
            created_at: Instant::now(),
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() > ttl
    }
}

/// 目录列表缓存
pub struct DirCache {
    cache: HashMap<String, CacheEntry<Vec<FileInfo>>>,
    lru_list: Vec<String>, // 简化版 LRU (使用 Vec 代替 LinkedList)
    max_size: usize,
    ttl: Duration,
    enabled: bool,
    hits: u64,
    misses: u64,
}

impl DirCache {
    pub fn new(max_size: usize, ttl: Duration, enabled: bool) -> Self {
        Self {
            cache: HashMap::new(),
            lru_list: Vec::new(),
            max_size: max_size.max(1),
            ttl,
            enabled,
            hits: 0,
            misses: 0,
        }
    }

    pub fn get(&mut self, path: &str) -> Option<Vec<FileInfo>> {
        if !self.enabled {
            return None;
        }

        // Check entry existence and validity (clone data before mutating)
        let entry_info = self.cache.get(path).map(|entry| {
            (!entry.is_expired(self.ttl), entry.data.clone())
        });

        match entry_info {
            Some((true, data)) => {
                self.hits += 1;
                self.move_to_front(path);
                return Some(data);
            }
            Some((false, _)) => {
                // Expired entry - clean up
                self.cache.remove(path);
                self.lru_list.retain(|p| p != path);
            }
            None => {}
        }

        self.misses += 1;
        None
    }

    pub fn put(&mut self, path: String, files: Vec<FileInfo>) {
        if !self.enabled {
            return;
        }

        // 淘汰最老条目
        if self.cache.len() >= self.max_size && !self.cache.contains_key(&path) {
            if let Some(oldest) = self.lru_list.pop() {
                self.cache.remove(&oldest);
            }
        }

        // 插入新条目
        self.cache.insert(path.clone(), CacheEntry::new(files));
        self.lru_list.retain(|p| p != &path);
        self.lru_list.insert(0, path);
    }

    pub fn invalidate(&mut self, path: &str) {
        self.cache.remove(path);
        self.lru_list.retain(|p| p != path);
    }

    pub fn invalidate_prefix(&mut self, prefix: &str) {
        let to_remove: Vec<String> = self.cache
            .keys()
            .filter(|k| *k == prefix || (k.len() > prefix.len() && k.starts_with(prefix) && k.as_bytes()[prefix.len()] == b'/'))
            .map(|k| k.clone())
            .collect();

        for key in to_remove {
            self.cache.remove(&key);
            self.lru_list.retain(|p| p != &key);
        }
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.lru_list.clear();
    }

    fn move_to_front(&mut self, path: &str) {
        if let Some(pos) = self.lru_list.iter().position(|p| p == path) {
            self.lru_list.remove(pos);
        }
        self.lru_list.insert(0, path.to_string());
    }

    #[cfg(test)]
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            return 0.0;
        }
        self.hits as f64 / total as f64
    }
}

/// 元数据缓存
pub struct StatCache {
    cache: HashMap<String, CacheEntry<FileInfo>>,
    lru_list: Vec<String>,
    max_size: usize,
    ttl: Duration,
    enabled: bool,
    hits: u64,
    misses: u64,
}

impl StatCache {
    pub fn new(max_size: usize, ttl: Duration, enabled: bool) -> Self {
        Self {
            cache: HashMap::new(),
            lru_list: Vec::new(),
            max_size: max_size.max(1),
            ttl,
            enabled,
            hits: 0,
            misses: 0,
        }
    }

    pub fn get(&mut self, path: &str) -> Option<FileInfo> {
        if !self.enabled {
            return None;
        }

        // Check entry existence and validity (clone data before mutating)
        let entry_info = self.cache.get(path).map(|entry| {
            (!entry.is_expired(self.ttl), entry.data.clone())
        });

        match entry_info {
            Some((true, data)) => {
                self.hits += 1;
                self.move_to_front(path);
                return Some(data);
            }
            Some((false, _)) => {
                // Expired entry - clean up
                self.cache.remove(path);
                self.lru_list.retain(|p| p != path);
            }
            None => {}
        }

        self.misses += 1;
        None
    }

    pub fn put(&mut self, path: String, info: FileInfo) {
        if !self.enabled {
            return;
        }

        if self.cache.len() >= self.max_size && !self.cache.contains_key(&path) {
            if let Some(oldest) = self.lru_list.pop() {
                self.cache.remove(&oldest);
            }
        }

        self.cache.insert(path.clone(), CacheEntry::new(info));
        self.lru_list.retain(|p| p != &path);
        self.lru_list.insert(0, path);
    }

    pub fn invalidate(&mut self, path: &str) {
        self.cache.remove(path);
        self.lru_list.retain(|p| p != path);
    }

    pub fn invalidate_prefix(&mut self, prefix: &str) {
        let to_remove: Vec<String> = self.cache
            .keys()
            .filter(|k| *k == prefix || (k.len() > prefix.len() && k.starts_with(prefix) && k.as_bytes()[prefix.len()] == b'/'))
            .map(|k| k.clone())
            .collect();

        for key in to_remove {
            self.cache.remove(&key);
            self.lru_list.retain(|p| p != &key);
        }
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.lru_list.clear();
    }

    fn move_to_front(&mut self, path: &str) {
        if let Some(pos) = self.lru_list.iter().position(|p| p == path) {
            self.lru_list.remove(pos);
        }
        self.lru_list.insert(0, path.to_string());
    }
}

/// S3FS 插件
#[cfg(feature = "s3fs")]
pub struct S3fsPlugin {
    client: Arc<Client>,
    config: S3Config,
    dir_cache: Arc<RwLock<DirCache>>,
    stat_cache: Arc<RwLock<StatCache>>,
}

#[cfg(feature = "s3fs")]
impl S3fsPlugin {
    /// 创建新的 S3FS 插件
    pub async fn new(config: S3Config) -> EvifResult<Self> {
        if config.bucket.is_empty() {
            return Err(EvifError::InvalidPath("bucket name is required".to_string()));
        }

        // 加载 AWS 配置
        let mut loader = aws_config::defaults(BehaviorVersion::latest());

        // 设置访问密钥 (如果提供)
        if let (Some(akid), Some(secret)) = (&config.access_key_id, &config.secret_access_key) {
            let creds = Credentials::new(
                akid,
                secret,
                None,
                None,
                "evif-s3fs",
            );

            loader = loader.credentials_provider(creds);
        }

        let aws_config = loader.load().await;

        // 配置 S3 客户端 (使用 Builder pattern)
        let mut builder = aws_sdk_s3::config::Builder::from(&aws_config)
            .region(aws_sdk_s3::config::Region::new(config.region.clone()));

        // 设置自定义 endpoint (MinIO 兼容)
        if let Some(endpoint) = &config.endpoint {
            builder = builder.endpoint_url(endpoint);
        }

        // 强制使用路径风格 (MinIO 必需)
        if config.force_path_style {
            builder = builder.force_path_style(true);
        }

        let client = Arc::new(Client::from_conf(builder.build()));

        // 创建缓存
        let dir_cache = Arc::new(RwLock::new(DirCache::new(
            1000,                              // max_size
            Duration::from_secs(30),           // ttl: 30s
            true,                              // enabled
        )));

        let stat_cache = Arc::new(RwLock::new(StatCache::new(
            5000,                              // max_size
            Duration::from_secs(60),           // ttl: 60s
            true,                              // enabled
        )));

        Ok(Self {
            client,
            config,
            dir_cache,
            stat_cache,
        })
    }

    /// 标准化 S3 key
    fn normalize_s3_key(&self, path: &str) -> EvifResult<String> {
        let mut key = path.trim_start_matches('/').to_string();

        // 添加前缀
        if let Some(prefix) = &self.config.prefix {
            if !prefix.is_empty() {
                key = format!("{}/{}", prefix.trim_end_matches('/'), key);
            }
        }

        Ok(key)
    }

    /// 获取父路径
    fn get_parent_path(&self, path: &str) -> String {
        if path.is_empty() || path == "/" {
            return String::new();
        }

        let path = path.trim_end_matches('/');
        if let Some(pos) = path.rfind('/') {
            path[..pos].to_string()
        } else {
            String::new()
        }
    }

    /// 获取基名
    fn get_basename(&self, path: &str) -> String {
        let path = path.trim_end_matches('/');
        if let Some(pos) = path.rfind('/') {
            path[pos + 1..].to_string()
        } else {
            path.to_string()
        }
    }

    /// S3 分片上传
    ///
    /// 支持大于阈值（默认 8MB）的文件分片上传
    /// 流程: CreateMultipartUpload → UploadPart (并发) → CompleteMultipartUpload
    pub async fn multipart_upload(&self, s3_path: &str, data: Vec<u8>) -> EvifResult<()> {
        use aws_sdk_s3::primitives::ByteStream;
        use aws_sdk_s3::types::CompletedMultipartUpload;
        use aws_sdk_s3::types::CompletedPart;

        let part_size = self.config.multipart_chunk_size as usize;
        let total_size = data.len();

        // 1. 初始化分片上传
        let create_resp = self.client
            .create_multipart_upload()
            .bucket(&self.config.bucket)
            .key(s3_path)
            .send()
            .await
            .map_err(|e| EvifError::Storage(format!("CreateMultipartUpload failed: {}", e)))?;

        let upload_id = create_resp.upload_id()
            .ok_or_else(|| EvifError::Internal("No upload_id in CreateMultipartUpload response".to_string()))?
            .to_string();

        log::info!(
            "Multipart upload started: {} (upload_id={}, total={}bytes, part_size={}bytes)",
            s3_path, upload_id, total_size, part_size
        );

        // 2. 分片并发上传
        let mut parts: Vec<CompletedPart> = Vec::new();
        let mut part_number: i32 = 0;
        let mut offset = 0;

        while offset < total_size {
            part_number += 1;
            let end = (offset + part_size).min(total_size);
            let chunk = data[offset..end].to_vec();

            match self.client
                .upload_part()
                .bucket(&self.config.bucket)
                .key(s3_path)
                .upload_id(&upload_id)
                .part_number(part_number)
                .body(ByteStream::from(chunk))
                .send()
                .await
            {
                Ok(resp) => {
                    let e_tag = resp.e_tag()
                        .map(|s| s.to_string())
                        .unwrap_or_default();

                    parts.push(
                        CompletedPart::builder()
                            .part_number(part_number)
                            .e_tag(e_tag)
                            .build()
                    );

                    log::debug!(
                        "Uploaded part #{}/{} (offset={}, size={})",
                        part_number, (total_size + part_size - 1) / part_size,
                        offset, end - offset
                    );
                }
                Err(e) => {
                    // 上传失败，中止分片上传
                    log::error!("Upload part {} failed: {}", part_number, e);
                    if let Err(abort_err) = self.client
                        .abort_multipart_upload()
                        .bucket(&self.config.bucket)
                        .key(s3_path)
                        .upload_id(&upload_id)
                        .send()
                        .await
                    {
                        log::error!("Failed to abort multipart upload: {}", abort_err);
                    }
                    return Err(EvifError::Internal(format!(
                        "Multipart upload failed at part {}: {}", part_number, e
                    )));
                }
            }

            offset = end;
        }

        // 3. 完成分片上传（按 part_number 排序确保正确性）
        parts.sort_by_key(|p| p.part_number().unwrap_or(0));

        let completed = CompletedMultipartUpload::builder()
            .set_parts(Some(parts))
            .build();

        self.client
            .complete_multipart_upload()
            .bucket(&self.config.bucket)
            .key(s3_path)
            .upload_id(&upload_id)
            .multipart_upload(completed)
            .send()
            .await
            .map_err(|e| EvifError::Storage(format!("CompleteMultipartUpload failed: {}", e)))?;

        log::info!(
            "Multipart upload completed: {} ({} parts, {}bytes)",
            s3_path, part_number, total_size
        );

        Ok(())
    }
}

#[cfg(feature = "s3fs")]
#[async_trait]
impl EvifPlugin for S3fsPlugin {
    fn name(&self) -> &str {
        "s3fs"
    }

    async fn create(&self, path: &str, _perm: u32) -> EvifResult<()> {
        let s3_path = self.normalize_s3_key(path)?;

        // 检查是否已存在
        let head_result = self.client
            .head_object()
            .bucket(&self.config.bucket)
            .key(&s3_path)
            .send()
            .await;

        if head_result.is_ok() {
            return Err(EvifError::InvalidPath(format!("file already exists: {}", path)));
        }

        // 创建空文件
        self.client
            .put_object()
            .bucket(&self.config.bucket)
            .key(&s3_path)
            .body(ByteStream::from(Vec::new()))
            .send()
            .await
            .map_err(|e| EvifError::Storage(format!("PutObject failed: {}", e)))?;

        // 失效缓存
        let parent = self.get_parent_path(&s3_path);
        self.dir_cache.write().await.invalidate(&parent);
        self.stat_cache.write().await.invalidate(&s3_path);

        Ok(())
    }

    async fn mkdir(&self, path: &str, _perm: u32) -> EvifResult<()> {
        let s3_path = self.normalize_s3_key(path)?;

        // S3 中目录以 "/" 结尾的空对象表示
        let dir_key = format!("{}/", s3_path.trim_end_matches('/'));

        // 创建目录标记
        self.client
            .put_object()
            .bucket(&self.config.bucket)
            .key(&dir_key)
            .body(ByteStream::from(Vec::new()))
            .send()
            .await
            .map_err(|e| EvifError::Storage(format!("PutObject (mkdir) failed: {}", e)))?;

        // 失效缓存
        let parent = self.get_parent_path(&s3_path);
        self.dir_cache.write().await.invalidate(&parent);
        self.stat_cache.write().await.invalidate(&s3_path);

        Ok(())
    }

    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>> {
        let s3_path = self.normalize_s3_key(path)?;

        // 使用 Range 请求
        if offset > 0 || size > 0 {
            let range = format!("bytes={}-{}", offset, offset + size.saturating_sub(1));

            let result = self.client
                .get_object()
                .bucket(&self.config.bucket)
                .key(&s3_path)
                .range(range)
                .send()
                .await;

            match result {
                Ok(output) => {
                    let data = output.body.collect().await
                        .map_err(|e| EvifError::Storage(format!("Read body failed: {}", e)))?
                        .to_vec();
                    return Ok(data);
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("NoSuchKey") || err_str.contains("NotFound") {
                        return Err(EvifError::NotFound(path.to_string()));
                    }
                    return Err(EvifError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        err_str,
                    )));
                }
            }
        }

        // 完整文件读取
        let result = self.client
            .get_object()
            .bucket(&self.config.bucket)
            .key(&s3_path)
            .send()
            .await;

        match result {
            Ok(output) => {
                let data = output.body.collect().await
                    .map_err(|e| EvifError::Storage(format!("Read body failed: {}", e)))?
                    .to_vec();
                Ok(data)
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("NoSuchKey") || err_str.contains("NotFound") {
                    Err(EvifError::NotFound(path.to_string()))
                } else {
                    Err(EvifError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        err_str,
                    )))
                }
            }
        }
    }

    async fn write(&self, path: &str, data: Vec<u8>, offset: i64, _flags: WriteFlags)
        -> EvifResult<u64>
    {
        let s3_path = self.normalize_s3_key(path)?;

        // S3 不支持 offset 写入
        if offset > 0 && offset != -1 {
            return Err(EvifError::InvalidPath(
                "S3 does not support offset writes".to_string()
            ));
        }

        // 检查是否为目录
        if s3_path.ends_with('/') {
            return Err(EvifError::InvalidPath(format!("is a directory: {}", path)));
        }

        let data_len = data.len() as u64;

        // 大文件自动使用分片上传（默认阈值 8MB）
        if data_len > self.config.multipart_threshold {
            self.multipart_upload(&s3_path, data).await?;
        } else {
            // 小文件直接上传
            let body = ByteStream::from(data);
            self.client
                .put_object()
                .bucket(&self.config.bucket)
                .key(&s3_path)
                .body(body)
                .send()
                .await
                .map_err(|e| EvifError::Storage(format!("PutObject failed: {}", e)))?;
        }

        // 失效缓存
        let parent = self.get_parent_path(&s3_path);
        self.dir_cache.write().await.invalidate(&parent);
        self.stat_cache.write().await.invalidate(&s3_path);

        Ok(data_len)
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let s3_path = self.normalize_s3_key(path)?;

        // 1. 尝试从缓存读取
        if let Some(cached) = self.dir_cache.write().await.get(&s3_path) {
            return Ok(cached);
        }

        // 2. 从 S3 列出对象
        let prefix = if s3_path.is_empty() {
            String::new()
        } else {
            format!("{}/", s3_path.trim_end_matches('/'))
        };

        let result = self.client
            .list_objects_v2()
            .bucket(&self.config.bucket)
            .prefix(&prefix)
            .delimiter("/")
            .send()
            .await
            .map_err(|e| EvifError::Storage(format!("ListObjectsV2 failed: {}", e)))?;

        let mut files = Vec::new();

        // 处理文件
        if let Some(objects) = result.contents {
            for obj in objects {
                let key = obj.key().unwrap_or(&prefix);
                let relative_key = key.strip_prefix(&prefix).unwrap_or(key);

                if relative_key.is_empty() || relative_key.ends_with('/') {
                    continue; // 跳过目录标记
                }

                files.push(FileInfo {
                    name: relative_key.to_string(),
                    size: obj.size().unwrap_or(0) as u64,
                    mode: 0o644,
                    modified: aws_dt_to_chrono(obj.last_modified()),
                    is_dir: false,
                });
            }
        }

        // 处理子目录
        if let Some(common_prefixes) = result.common_prefixes {
            for cp in common_prefixes {
                if let Some(prefix_str) = cp.prefix {
                    let relative = prefix_str.strip_prefix(&prefix).unwrap_or(&prefix_str);
                    let dir_name = relative.trim_end_matches('/');

                    if !dir_name.is_empty() {
                        files.push(FileInfo {
                            name: dir_name.to_string(),
                            size: 0,
                            mode: 0o755,
                            modified: Utc::now(),
                            is_dir: true,
                        });
                    }
                }
            }
        }

        // 3. 写入缓存
        self.dir_cache.write().await.put(s3_path.clone(), files.clone());

        Ok(files)
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let s3_path = self.normalize_s3_key(path)?;

        // 根目录特殊处理
        if s3_path.is_empty() || s3_path == "/" {
            return Ok(FileInfo {
                name: "/".to_string(),
                size: 0,
                mode: 0o755,
                modified: Utc::now(),
                is_dir: true,
            });
        }

        // 1. 尝试从缓存读取
        if let Some(cached) = self.stat_cache.write().await.get(&s3_path) {
            return Ok(cached);
        }

        // 2. 尝试作为文件
        let head_result = self.client
            .head_object()
            .bucket(&self.config.bucket)
            .key(&s3_path)
            .send()
            .await;

        if let Ok(head) = head_result {
            let info = FileInfo {
                name: self.get_basename(path),
                size: head.content_length().unwrap_or(0) as u64,
                mode: 0o644,
                modified: aws_dt_to_chrono(head.last_modified()),
                is_dir: false,
            };

            // 写入缓存
            self.stat_cache.write().await.put(s3_path.clone(), info.clone());
            return Ok(info);
        }

        // 3. 尝试作为目录
        let prefix = format!("{}/", s3_path.trim_end_matches('/'));
        let list_result = self.client
            .list_objects_v2()
            .bucket(&self.config.bucket)
            .prefix(&prefix)
            .max_keys(1)
            .send()
            .await;

        if let Ok(result) = list_result {
            let has_objects = !result.contents().is_empty();
            let has_prefixes = !result.common_prefixes().is_empty();

            if has_objects || has_prefixes {
                let info = FileInfo {
                    name: self.get_basename(path),
                    size: 0,
                    mode: 0o755,
                    modified: Utc::now(),
                    is_dir: true,
                };

                self.stat_cache.write().await.put(s3_path.clone(), info.clone());
                return Ok(info);
            }
        }

        // 4. 未找到
        Err(EvifError::NotFound(path.to_string()))
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        let s3_path = self.normalize_s3_key(path)?;

        // 尝试删除对象
        self.client
            .delete_object()
            .bucket(&self.config.bucket)
            .key(&s3_path)
            .send()
            .await
            .map_err(|e| EvifError::Storage(format!("DeleteObject failed: {}", e)))?;

        // 失效缓存
        let parent = self.get_parent_path(&s3_path);
        self.dir_cache.write().await.invalidate(&parent);
        self.stat_cache.write().await.invalidate(&s3_path);

        Ok(())
    }

    async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()> {
        let old_s3_path = self.normalize_s3_key(old_path)?;
        let new_s3_path = self.normalize_s3_key(new_path)?;

        // 读取旧对象
        let result = self.client
            .get_object()
            .bucket(&self.config.bucket)
            .key(&old_s3_path)
            .send()
            .await
            .map_err(|e| EvifError::Storage(format!("GetObject failed: {}", e)))?;

        let data = result.body.collect().await
            .map_err(|e| EvifError::Storage(format!("Read body failed: {}", e)))?
            .to_vec();

        // 写入新位置
        let body = ByteStream::from(data);
        self.client
            .put_object()
            .bucket(&self.config.bucket)
            .key(&new_s3_path)
            .body(body)
            .send()
            .await
            .map_err(|e| EvifError::Storage(format!("PutObject failed: {}", e)))?;

        // 删除旧对象
        self.client
            .delete_object()
            .bucket(&self.config.bucket)
            .key(&old_s3_path)
            .send()
            .await
            .map_err(|e| EvifError::Storage(format!("DeleteObject failed: {}", e)))?;

        // 失效缓存
        let old_parent = self.get_parent_path(&old_s3_path);
        let new_parent = self.get_parent_path(&new_s3_path);
        self.dir_cache.write().await.invalidate(&old_parent);
        self.dir_cache.write().await.invalidate(&new_parent);
        self.stat_cache.write().await.invalidate(&old_s3_path);
        self.stat_cache.write().await.invalidate(&new_s3_path);

        Ok(())
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        let s3_path = self.normalize_s3_key(path)?;
        let prefix = format!("{}/", s3_path.trim_end_matches('/'));

        // 1. 列出所有对象
        let mut all_objects = Vec::new();
        let mut continuation_token = None;

        loop {
            let mut list_req = self.client
                .list_objects_v2()
                .bucket(&self.config.bucket)
                .prefix(&prefix)
                .max_keys(1000);

            if let Some(token) = &continuation_token {
                list_req = list_req.continuation_token(token);
            }

            let result = list_req.send().await
                .map_err(|e| EvifError::Storage(format!("ListObjectsV2 failed: {}", e)))?;

            if let Some(objects) = result.contents {
                for obj in objects {
                    if let Some(key) = obj.key {
                        all_objects.push(key);
                    }
                }
            }

            match result.next_continuation_token {
                Some(token) => continuation_token = Some(token),
                None => break,
            }
        }

        // 2. 批量删除 (每次最多 1000 个对象)
        use aws_sdk_s3::types::ObjectIdentifier;

        for chunk in all_objects.chunks(1000) {
            let delete_objs: Vec<_> = chunk.iter().map(|key| {
                ObjectIdentifier::builder()
                    .key(key)
                    .build()
                    .expect("ObjectIdentifier build should not fail")
            }).collect();

            let delete = aws_sdk_s3::types::Delete::builder()
                .set_objects(Some(delete_objs))
                .build()
                .expect("Delete build should not fail");

            self.client
                .delete_objects()
                .bucket(&self.config.bucket)
                .delete(delete)
                .send()
                .await
                .map_err(|e| EvifError::Storage(format!("DeleteObjects failed: {}", e)))?;
        }

        // 3. 失效缓存
        let parent = self.get_parent_path(&s3_path);
        self.dir_cache.write().await.invalidate(&parent);
        self.dir_cache.write().await.invalidate_prefix(&s3_path);
        self.stat_cache.write().await.invalidate_prefix(&s3_path);

        Ok(())
    }

    async fn chmod(&self, _path: &str, _mode: u32) -> EvifResult<()> {
        // S3 对象存储不支持 chmod 权限修改
        Err(EvifError::NotSupportedGeneric)
    }

    async fn truncate(&self, path: &str, size: u64) -> EvifResult<()> {
        let s3_path = self.normalize_s3_key(path)?;

        // 检查是否为目录
        if s3_path.ends_with('/') {
            return Err(EvifError::InvalidPath(format!("is a directory: {}", path)));
        }

        // S3 truncate 需要读取整个文件，截断后重新上传
        let result = self.client
            .get_object()
            .bucket(&self.config.bucket)
            .key(&s3_path)
            .send()
            .await;

        if let Ok(output) = result {
            let data = output.body.collect().await
                .map_err(|e| EvifError::Storage(format!("Read body failed: {}", e)))?
                .to_vec();
            let truncated_data = if data.len() > size as usize {
                &data[..size.min(data.len() as u64) as usize]
            } else {
                &data
            };

            // 重新上传
            let body = ByteStream::from(truncated_data.to_vec());
            self.client
                .put_object()
                .bucket(&self.config.bucket)
                .key(&s3_path)
                .body(body)
                .send()
                .await
                .map_err(|e| EvifError::Storage(format!("PutObject (truncate) failed: {}", e)))?;

            // 失效缓存
            let parent = self.get_parent_path(&s3_path);
            self.dir_cache.write().await.invalidate(&parent);
            self.stat_cache.write().await.invalidate(&s3_path);

            Ok(())
        } else {
            Err(EvifError::NotFound(path.to_string()))
        }
    }
}

/// 将 AWS SDK DateTime 转换为 chrono DateTime<Utc>
/// 如果无法转换则返回当前时间
#[cfg(feature = "s3fs")]
fn aws_dt_to_chrono(dt: Option<&aws_smithy_types::DateTime>) -> DateTime<Utc> {
    dt.and_then(|dt| {
        DateTime::from_timestamp_millis(dt.to_millis().ok()?)
    }).unwrap_or_else(Utc::now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dir_cache_basic() {
        let mut cache = DirCache::new(3, Duration::from_secs(30), true);

        // 测试 put 和 get
        let files = vec![
            FileInfo {
                name: "test.txt".to_string(),
                size: 100,
                mode: 0o644,
                modified: Utc::now(),
                is_dir: false,
            }
        ];

        cache.put("/data".to_string(), files.clone());
        let retrieved = cache.get("/data");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().len(), 1);
    }

    #[test]
    fn test_dir_cache_lru_eviction() {
        let mut cache = DirCache::new(3, Duration::from_secs(30), true);

        cache.put("/a".to_string(), vec![]);
        cache.put("/b".to_string(), vec![]);
        cache.put("/c".to_string(), vec![]);
        cache.put("/d".to_string(), vec![]); // 应该淘汰 /a

        assert!(cache.get("/a").is_none());
        assert!(cache.get("/b").is_some());
        assert!(cache.get("/c").is_some());
        assert!(cache.get("/d").is_some());
    }

    #[test]
    fn test_dir_cache_invalidate() {
        let mut cache = DirCache::new(10, Duration::from_secs(30), true);

        cache.put("/data".to_string(), vec![]);
        cache.invalidate("/data");

        assert!(cache.get("/data").is_none());
    }

    #[test]
    fn test_dir_cache_invalidate_prefix() {
        let mut cache = DirCache::new(10, Duration::from_secs(30), true);

        cache.put("/data".to_string(), vec![]);
        cache.put("/data/subdir".to_string(), vec![]);
        cache.put("/other".to_string(), vec![]);

        cache.invalidate_prefix("/data");

        assert!(cache.get("/data").is_none());
        assert!(cache.get("/data/subdir").is_none());
        assert!(cache.get("/other").is_some()); // 不受影响
    }

    #[test]
    fn test_stat_cache_basic() {
        let mut cache = StatCache::new(3, Duration::from_secs(60), true);

        let info = FileInfo {
            name: "test.txt".to_string(),
            size: 100,
            mode: 0o644,
            modified: Utc::now(),
            is_dir: false,
        };

        cache.put("/test.txt".to_string(), info.clone());
        let retrieved = cache.get("/test.txt");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test.txt");
    }

    #[test]
    fn test_s3_config_default() {
        let config = S3Config::default();
        assert_eq!(config.region, "us-east-1");
        assert!(config.bucket.is_empty());
        assert!(config.endpoint.is_none());
    }
}
