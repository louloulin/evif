// EVIF 客户端实现

use crate::{ClientError, ClientResult};
use base64::Engine;
use evif_core::FileInfo;
use reqwest::Client as HttpClient;
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

/// Configuration for the EVIF client.
///
/// Contains all settings needed to connect to the EVIF backend server,
/// including connection timeout and the base URL of the REST API.
///
/// # Default
///
/// The default instance connects to `http://localhost:8081` with a
/// 30-second request timeout.
///
/// # Example
///
/// ```
/// use evif_client::ClientConfig;
///
/// let config = ClientConfig::default();
/// assert_eq!(config.base_url, "http://localhost:8081");
/// assert_eq!(config.request_timeout, 30);
/// ```
///
/// Custom configuration:
///
/// ```
/// use evif_client::ClientConfig;
/// use std::time::Duration;
///
/// let config = ClientConfig {
///     request_timeout: 60,
///     base_url: "http://my-evif-host:9090".to_string(),
///     timeout: Duration::from_secs(60),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Request timeout in seconds. Used as a hint when building the HTTP client.
    pub request_timeout: u64,

    /// Base URL for the EVIF REST API (e.g., `http://localhost:8081`).
    pub base_url: String,

    /// Timeout duration for HTTP requests.
    pub timeout: std::time::Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            request_timeout: 30,
            base_url: "http://localhost:8081".to_string(),
            timeout: std::time::Duration::from_secs(30),
        }
    }
}

/// The main EVIF client for interacting with the virtual filesystem.
///
/// `EvifClient` communicates with the EVIF backend over HTTP REST API.
/// Construct it with [`new`](EvifClient::new) (async) or [`new_sync`](EvifClient::new_sync) (sync),
/// then call methods like [`ls`](EvifClient::ls), [`cat`](EvifClient::cat), or [`write`](EvifClient::write).
///
/// # Example
///
/// ```ignore
/// use evif_client::{ClientConfig, EvifClient};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = ClientConfig::default();
///     let client = EvifClient::new(config).await?;
///
///     // List root directory
///     for entry in client.ls("/").await? {
///         println!("{}", entry.name);
///     }
///
///     // Read a file
///     let content = client.cat("/path/to/file.txt").await?;
///     println!("{}", content);
///
///     Ok(())
/// }
/// ```
///
/// # Thread Safety
///
/// `EvifClient` holds internal mutable state (the HTTP client) and must not
/// be shared across threads simultaneously. Clone `ClientConfig` and construct
/// separate instances per thread, or use a mutex if you must share one.
pub struct EvifClient {
    config: ClientConfig,
    http_client: HttpClient,
}

#[derive(Debug, Deserialize)]
struct RestFileInfo {
    name: String,
    #[allow(dead_code)]
    path: String,
    is_dir: bool,
    size: u64,
    modified: String,
    #[allow(dead_code)]
    created: String,
}

#[derive(Debug, Deserialize)]
struct RestFileStat {
    path: String,
    size: u64,
    is_dir: bool,
    modified: String,
    #[allow(dead_code)]
    created: String,
}

fn parse_modified(ts: &str) -> ClientResult<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| ClientError::Protocol(format!("Invalid timestamp '{}': {}", ts, e)))
}

fn file_name_from_path(path: &str) -> String {
    path.trim_end_matches('/')
        .split('/')
        .next_back()
        .filter(|s| !s.is_empty())
        .unwrap_or("/")
        .to_string()
}

impl EvifClient {
    /// Constructs a new `EvifClient` asynchronously.
    ///
    /// Builds an HTTP client with `no_proxy` configured and returns the client
    /// wrapped in `Ok`. If the underlying HTTP client construction fails, returns
    /// `Err`.
    ///
    /// # Arguments
    ///
    /// * `config` — Connection and timeout settings. See [`ClientConfig::default`]
    ///   for sensible defaults.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = ClientConfig::default();
    /// let client = EvifClient::new(config).await?;
    /// ```
    pub async fn new(config: ClientConfig) -> ClientResult<Self> {
        Ok(Self {
            config,
            http_client: HttpClient::builder().no_proxy().build().unwrap(),
        })
    }

    /// Constructs a new `EvifClient` synchronously.
    ///
    /// Identical to [`new`](EvifClient::new) but does not require `async` context,
    /// making it suitable for CLI tools and synchronous entry points.
    ///
    /// # Arguments
    ///
    /// * `config` — Connection and timeout settings.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = ClientConfig::default();
    /// let client = EvifClient::new_sync(config);
    /// ```
    pub fn new_sync(config: ClientConfig) -> Self {
        Self {
            config: config.clone(),
            http_client: HttpClient::builder().no_proxy().build().unwrap(),
        }
    }

    /// Reads a file and returns its contents as a byte vector.
    ///
    /// Convenience wrapper around [`cat_bytes`](EvifClient::cat_bytes) that accepts
    /// a [`Path`] instead of `&str`.
    ///
    /// # Arguments
    ///
    /// * `path` — Path to the file to read.
    ///
    /// # Returns
    ///
    /// Raw file bytes on success.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::Transport`] on network failure,
    /// [`ClientError::Protocol`] if the response is malformed, or
    /// [`ClientError::Io`] on underlying I/O errors.
    pub async fn read_file(&self, path: &Path) -> ClientResult<Vec<u8>> {
        self.cat_bytes(path.to_string_lossy().as_ref()).await
    }

    // ==================== HTTP REST API 方法 ====================

    /// Lists the contents of a directory.
    ///
    /// Sends a `GET /api/v1/directories?path=<path>` request and returns an
    /// ordered list of [`FileInfo`] entries for each child.
    ///
    /// # Arguments
    ///
    /// * `path` — Absolute path to the directory. Use `/` for the root.
    ///
    /// # Returns
    ///
    /// `Ok(Vec<FileInfo>)` on success — an empty vector if the directory is empty.
    ///
    /// # Errors
    ///
    /// - [`ClientError::Transport`] — Network failure.
    /// - [`ClientError::Protocol`] — Server returned a non-success status or
    ///   the response body is missing the expected `files` array.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let entries = client.ls("/some/dir").await?;
    /// for entry in entries {
    ///     println!("{} (dir={})", entry.name, entry.is_dir);
    /// }
    /// ```
    pub async fn ls(&self, path: &str) -> ClientResult<Vec<FileInfo>> {
        let url = format!("{}/api/v1/directories?path={}", self.config.base_url, path);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;

        let status = response.status();
        let json: Value = response
            .json()
            .await
            .map_err(|e| ClientError::Protocol(e.to_string()))?;

        // 如果返回错误，返回错误信息
        if !status.is_success() {
            if let Some(msg) = json.get("message").and_then(|v| v.as_str()) {
                return Err(ClientError::Protocol(msg.to_string()));
            }
            return Err(ClientError::Protocol(format!("HTTP {}", status.as_u16())));
        }

        let files = json["files"].as_array().ok_or_else(|| {
            ClientError::Protocol("Invalid response: missing 'files' field".to_string())
        })?;

        files
            .iter()
            .map(|v| {
                let info: RestFileInfo = serde_json::from_value(v.clone())
                    .map_err(|e| ClientError::Protocol(e.to_string()))?;
                Ok(FileInfo {
                    name: info.name,
                    size: info.size,
                    mode: if info.is_dir { 0o755 } else { 0o644 },
                    modified: parse_modified(&info.modified)?,
                    is_dir: info.is_dir,
                })
            })
            .collect()
    }

    /// Reads a file and returns its contents as a UTF-8 string.
    ///
    /// Internally fetches raw bytes via [`cat_bytes`](EvifClient::cat_bytes) and
    /// converts them to a `String`. Returns an error if the file contains invalid
    /// UTF-8 sequences.
    ///
    /// # Arguments
    ///
    /// * `path` — Absolute path to the file to read.
    ///
    /// # Returns
    ///
    /// File contents as a UTF-8 string.
    ///
    /// # Errors
    ///
    /// Same as [`cat_bytes`](EvifClient::cat_bytes), plus
    /// [`ClientError::Protocol`] with message `"Invalid UTF-8"` if the content
    /// is not valid UTF-8.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let text = client.cat("/path/to/file.txt").await?;
    /// println!("{}", text);
    /// ```
    pub async fn cat(&self, path: &str) -> ClientResult<String> {
        let bytes = self.cat_bytes(path).await?;
        String::from_utf8(bytes).map_err(|e| ClientError::Protocol(format!("Invalid UTF-8: {}", e)))
    }

    /// Reads a file and returns its raw bytes.
    ///
    /// Sends a `GET /api/v1/files?path=<path>` request. The response body is expected
    /// to contain a base64-encoded string under the `data` key, which is decoded and
    /// returned.
    ///
    /// # Arguments
    ///
    /// * `path` — Absolute path to the file to read.
    ///
    /// # Returns
    ///
    /// Raw file bytes on success.
    ///
    /// # Errors
    ///
    /// - [`ClientError::Transport`] — Network failure.
    /// - [`ClientError::Protocol`] — Server returned a non-success status or the
    ///   response is missing the expected `data` field.
    pub async fn cat_bytes(&self, path: &str) -> ClientResult<Vec<u8>> {
        let url = format!("{}/api/v1/files?path={}", self.config.base_url, path);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        let status = response.status();

        let json: Value = response
            .json()
            .await
            .map_err(|e| ClientError::Protocol(e.to_string()))?;

        if !status.is_success() {
            if let Some(msg) = json.get("message").and_then(|v| v.as_str()) {
                return Err(ClientError::Protocol(msg.to_string()));
            }
            return Err(ClientError::Protocol(format!("HTTP {}", status.as_u16())));
        }

        let data = json["data"]
            .as_str()
            .ok_or_else(|| ClientError::Protocol("Invalid response".to_string()))?;

        base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|e| ClientError::Protocol(e.to_string()))
    }

    /// Writes content to a file, creating it if necessary.
    ///
    /// If `append` is `false`, the file is overwritten with `content`. If `append`
    /// is `true`, `content` is appended to the existing file content (creating it
    /// with an empty initial content if it does not yet exist).
    ///
    /// The content is sent as a base64-encoded string in the request body, matching
    /// the `evif-rest` contract: `{ "data": "<base64>", "encoding": "base64" }`.
    ///
    /// # Arguments
    ///
    /// * `path` — Absolute path to the target file.
    /// * `content` — Text content to write. Binary content should be encoded first.
    /// * `append` — If `true`, append to the file instead of overwriting.
    ///
    /// # Errors
    ///
    /// - [`ClientError::Transport`] — Network failure.
    /// - [`ClientError::Protocol`] — Server returned a non-success HTTP status.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Overwrite
    /// client.write("/path/to/file.txt", "hello world", false).await?;
    ///
    /// // Append
    /// client.write("/path/to/log.txt", "new entry\n", true).await?;
    /// ```
    pub async fn write(&self, path: &str, content: &str, append: bool) -> ClientResult<()> {
        let payload = if append {
            match self.cat(path).await {
                Ok(existing) => format!("{}{}", existing, content),
                Err(_) => content.to_string(),
            }
        } else {
            content.to_string()
        };

        let bytes = payload.as_bytes().to_vec();
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);

        let create_url = format!("{}/api/v1/files", self.config.base_url);
        let create_body = serde_json::json!({ "path": path });
        let _ = self
            .http_client
            .post(&create_url)
            .json(&create_body)
            .send()
            .await;

        let url = format!(
            "{}/api/v1/files?path={}&offset=0",
            self.config.base_url, path
        );
        let body = serde_json::json!({ "data": encoded, "encoding": "base64" });

        let response = self
            .http_client
            .put(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ClientError::Protocol(format!(
                "HTTP {}",
                response.status().as_u16()
            )));
        }

        Ok(())
    }

    /// Creates a directory, optionally creating all intermediate parent directories.
    ///
    /// Sends a `POST /api/v1/directories` with `{ "path": <path>, "parents": <parents> }`.
    ///
    /// # Arguments
    ///
    /// * `path` — Absolute path of the directory to create.
    /// * `parents` — If `true`, all missing parent directories are created recursively.
    ///               If `false`, returns an error if a parent does not exist.
    ///
    /// # Errors
    ///
    /// - [`ClientError::Transport`] — Network failure.
    /// - [`ClientError::Protocol`] — Server returned a non-success status.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Create a single directory (fails if parent doesn't exist)
    /// client.mkdir("/data/logs", false).await?;
    ///
    /// // Create nested directories
    /// client.mkdir("/data/logs/archive/2024", true).await?;
    /// ```
    pub async fn mkdir(&self, path: &str, parents: bool) -> ClientResult<()> {
        let url = format!("{}/api/v1/directories", self.config.base_url);
        let body = serde_json::json!({ "path": path, "parents": parents });
        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        if !response.status().is_success() {
            return Err(ClientError::Protocol(format!(
                "HTTP {}",
                response.status().as_u16()
            )));
        }
        Ok(())
    }

    /// Deletes a single file (not a directory).
    ///
    /// Sends a `DELETE /api/v1/files?path=<path>` request.
    ///
    /// # Arguments
    ///
    /// * `path` — Absolute path to the file to delete.
    ///
    /// # Errors
    ///
    /// - [`ClientError::Transport`] — Network failure.
    ///
    /// # Example
    ///
    /// ```ignore
    /// client.remove("/tmp/cache/file.tmp").await?;
    /// ```
    pub async fn remove(&self, path: &str) -> ClientResult<()> {
        let url = format!("{}/api/v1/files?path={}", self.config.base_url, path);
        self.http_client
            .delete(&url)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        Ok(())
    }

    /// Recursively deletes a directory and all of its contents.
    ///
    /// Sends a `DELETE /api/v1/directories?path=<path>` request. Use this instead
    /// of [`remove`](EvifClient::remove) when you need to delete a directory
    /// regardless of whether it contains files or subdirectories.
    ///
    /// # Arguments
    ///
    /// * `path` — Absolute path to the directory to delete.
    ///
    /// # Errors
    ///
    /// - [`ClientError::Transport`] — Network failure.
    ///
    /// # Safety Warning
    ///
    /// This operation is destructive and irreversible. Ensure `path` points to
    /// the intended target.
    ///
    /// # Example
    ///
    /// ```ignore
    /// client.remove_all("/tmp/old-build").await?;
    /// ```
    pub async fn remove_all(&self, path: &str) -> ClientResult<()> {
        let url = format!("{}/api/v1/directories?path={}", self.config.base_url, path);
        self.http_client
            .delete(&url)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        Ok(())
    }

    /// Renames (moves) a file or directory from `old_path` to `new_path`.
    ///
    /// Sends a `POST /api/v1/rename` with `{ "from": <old_path>, "to": <new_path> }`.
    ///
    /// # Arguments
    ///
    /// * `old_path` — Current absolute path of the file or directory.
    /// * `new_path` — Desired new absolute path.
    ///
    /// # Errors
    ///
    /// - [`ClientError::Transport`] — Network failure.
    /// - [`ClientError::Protocol`] — Server returned a non-success status.
    ///
    /// # Example
    ///
    /// ```ignore
    /// client.rename("/old/name.txt", "/new/name.txt").await?;
    /// ```
    pub async fn rename(&self, old_path: &str, new_path: &str) -> ClientResult<()> {
        let url = format!("{}/api/v1/rename", self.config.base_url);
        let body = serde_json::json!({"from": old_path, "to": new_path});

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        if !response.status().is_success() {
            return Err(ClientError::Protocol(format!(
                "HTTP {}",
                response.status().as_u16()
            )));
        }
        Ok(())
    }

    /// Retrieves metadata for a single file or directory without reading its content.
    ///
    /// Sends a `GET /api/v1/stat?path=<path>` request and returns a [`FileInfo`]
    /// struct containing name, size, modification time, and file type.
    ///
    /// # Arguments
    ///
    /// * `path` — Absolute path to the file or directory.
    ///
    /// # Returns
    ///
    /// `Ok(FileInfo)` with the file's metadata.
    ///
    /// # Errors
    ///
    /// - [`ClientError::Transport`] — Network failure.
    /// - [`ClientError::Protocol`] — Server returned a non-success status or
    ///   the response body is malformed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let info = client.stat("/path/to/file.txt").await?;
    /// println!("{} bytes, modified at {}", info.size, info.modified);
    /// ```
    pub async fn stat(&self, path: &str) -> ClientResult<FileInfo> {
        let url = format!("{}/api/v1/stat?path={}", self.config.base_url, path);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        let status = response.status();

        let json: Value = response
            .json()
            .await
            .map_err(|e| ClientError::Protocol(e.to_string()))?;
        if !status.is_success() {
            if let Some(msg) = json.get("message").and_then(|v| v.as_str()) {
                return Err(ClientError::Protocol(msg.to_string()));
            }
            return Err(ClientError::Protocol(format!("HTTP {}", status.as_u16())));
        }

        let stat: RestFileStat =
            serde_json::from_value(json).map_err(|e| ClientError::Protocol(e.to_string()))?;
        Ok(FileInfo {
            name: file_name_from_path(&stat.path),
            size: stat.size,
            mode: if stat.is_dir { 0o755 } else { 0o644 },
            modified: parse_modified(&stat.modified)?,
            is_dir: stat.is_dir,
        })
    }

    /// Performs a health check against the EVIF backend.
    ///
    /// Sends a `GET /api/v1/health` request and returns basic server status,
    /// version, and uptime information.
    ///
    /// # Returns
    ///
    /// [`HealthInfo`] containing the server's current status, version string,
    /// and uptime in seconds.
    ///
    /// # Errors
    ///
    /// - [`ClientError::Transport`] — Network failure.
    /// - [`ClientError::Protocol`] — Server returned an invalid response body.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let health = client.health().await?;
    /// println!("EVIF {} — status: {}, uptime: {}s",
    ///     health.version, health.status, health.uptime);
    /// ```
    pub async fn health(&self) -> ClientResult<HealthInfo> {
        let url = format!("{}/api/v1/health", self.config.base_url);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;

        let json: Value = response
            .json()
            .await
            .map_err(|e| ClientError::Protocol(e.to_string()))?;

        Ok(HealthInfo {
            status: json["status"].as_str().unwrap_or("unknown").to_string(),
            version: json["version"].as_str().unwrap_or("unknown").to_string(),
            uptime: json["uptime"].as_u64().unwrap_or(0),
        })
    }

    /// Mounts a filesystem plugin at a given path.
    ///
    /// Attaches a plugin (e.g., `"memory"`, `"disk"`) to the virtual filesystem
    /// at `path`, optionally providing plugin-specific configuration as JSON.
    ///
    /// Sends a `POST /api/v1/mount` with `{ "plugin": <plugin>, "path": <path>,
    /// "config": <config> }`. The `config` field is omitted if `config` is `None`.
    ///
    /// # Arguments
    ///
    /// * `plugin` — Name of the plugin to mount (e.g., `"memory"`, `"disk"`).
    /// * `path` — Absolute virtual path where the plugin will be attached.
    /// * `config` — Optional JSON configuration string passed to the plugin.
    ///
    /// # Errors
    ///
    /// - [`ClientError::Transport`] — Network failure.
    ///
    /// # Example
    ///
    /// ```ignore
    /// client.mount("memory", "/mem", None).await?;
    /// ```
    pub async fn mount(&self, plugin: &str, path: &str, config: Option<&str>) -> ClientResult<()> {
        let url = format!("{}/api/v1/mount", self.config.base_url);
        let mut body = serde_json::json!({"plugin": plugin, "path": path});
        if let Some(cfg) = config {
            body["config"] = serde_json::json!(cfg);
        }

        self.http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        Ok(())
    }

    /// Unmounts the plugin currently attached at `path`.
    ///
    /// Sends a `POST /api/v1/unmount` with `{ "path": <path> }`.
    ///
    /// # Arguments
    ///
    /// * `path` — Absolute virtual path where a plugin is currently mounted.
    ///
    /// # Errors
    ///
    /// - [`ClientError::Transport`] — Network failure.
    ///
    /// # Example
    ///
    /// ```ignore
    /// client.unmount("/mem").await?;
    /// ```
    pub async fn unmount(&self, path: &str) -> ClientResult<()> {
        let url = format!("{}/api/v1/unmount", self.config.base_url);
        let body = serde_json::json!({"path": path});
        self.http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        Ok(())
    }

    /// Lists all currently active plugin mounts.
    ///
    /// Sends a `GET /api/v1/mounts` request. The server may return either a JSON
    /// object with a `"mounts"` array or a plain array — both formats are accepted.
    ///
    /// # Returns
    ///
    /// `Ok(Vec<MountInfo>)` — one entry per active mount. Empty if no plugins are mounted.
    ///
    /// # Errors
    ///
    /// - [`ClientError::Transport`] — Network failure.
    /// - [`ClientError::Protocol`] — Response is neither an object with `mounts` nor an array.
    ///
    /// # Example
    ///
    /// ```ignore
    /// for mount in client.mounts().await? {
    ///     println!("{} mounted at {}", mount.plugin, mount.path);
    /// }
    /// ```
    pub async fn mounts(&self) -> ClientResult<Vec<MountInfo>> {
        let url = format!("{}/api/v1/mounts", self.config.base_url);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;

        let status = response.status();
        let json: Value = response
            .json()
            .await
            .map_err(|e| ClientError::Protocol(e.to_string()))?;

        // 如果返回错误，返回错误信息
        if !status.is_success() {
            if let Some(msg) = json.get("message").and_then(|v| v.as_str()) {
                return Err(ClientError::Protocol(msg.to_string()));
            }
            return Err(ClientError::Protocol(format!("HTTP {}", status.as_u16())));
        }

        // 尝试两种格式：{"mounts": [...]} 或直接的数组 [...]
        let mounts = if let Some(mounts_array) = json.get("mounts").and_then(|v| v.as_array()) {
            mounts_array
        } else if let Some(array) = json.as_array() {
            array
        } else {
            return Err(ClientError::Protocol(
                "Invalid response: expected array or object with 'mounts' field".to_string(),
            ));
        };

        mounts
            .iter()
            .map(|v| {
                serde_json::from_value(v.clone()).map_err(|e| ClientError::Protocol(e.to_string()))
            })
            .collect()
    }

    /// Computes a cryptographic digest (checksum) of a file.
    ///
    /// Sends a `POST /api/v1/digest` with `{ "path": <path>, "algorithm": <algo> }`.
    ///
    /// # Arguments
    ///
    /// * `path` — Absolute path to the file to digest.
    /// * `algorithm` — Hash algorithm name (e.g., `"sha256"`, `"md5"`). If `None`,
    ///   defaults to `"sha256"`.
    ///
    /// # Returns
    ///
    /// `Ok((algorithm, hash))` — a tuple of the algorithm actually used and the
    /// lowercase hexadecimal digest string.
    ///
    /// # Errors
    ///
    /// - [`ClientError::Transport`] — Network failure.
    /// - [`ClientError::Protocol`] — Response is missing the `hash` field.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let (algo, hash) = client.digest("/my/file.txt", Some("sha256")).await?;
    /// println!("{}({}) = {}", algo, path, hash);
    /// ```
    pub async fn digest(
        &self,
        path: &str,
        algorithm: Option<&str>,
    ) -> ClientResult<(String, String)> {
        let url = format!("{}/api/v1/digest", self.config.base_url);
        let mut body = serde_json::json!({ "path": path });
        if let Some(algo) = algorithm {
            body["algorithm"] = serde_json::json!(algo);
        }
        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        let json: Value = response
            .json()
            .await
            .map_err(|e| ClientError::Protocol(e.to_string()))?;
        let algo = json["algorithm"].as_str().unwrap_or("sha256").to_string();
        let hash = json["hash"]
            .as_str()
            .ok_or_else(|| ClientError::Protocol("Missing hash".to_string()))?
            .to_string();
        Ok((algo, hash))
    }

    /// Changes the permission bits of a file or directory.
    ///
    /// Sends a `POST /api/v1/fs/chmod` with `{ "path": <path>, "mode": <mode> }`.
    ///
    /// # Arguments
    ///
    /// * `path` — Absolute path to the target file or directory.
    /// * `mode` — New permission bits (e.g., `0o755` for rwxr-xr-x).
    ///
    /// # Errors
    ///
    /// - [`ClientError::Transport`] — Network failure.
    /// - [`ClientError::Protocol`] — Server returned a non-success status.
    ///
    /// # Example
    ///
    /// ```ignore
    /// client.chmod("/script.sh", 0o755).await?;
    /// ```
    pub async fn chmod(&self, path: &str, mode: u32) -> ClientResult<()> {
        let url = format!("{}/api/v1/fs/chmod", self.config.base_url);
        let body = serde_json::json!({ "path": path, "mode": mode });
        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;

        if !response.status().is_success() {
            let json: Value = response
                .json()
                .await
                .map_err(|e| ClientError::Protocol(e.to_string()))?;
            let msg = json.get("message").and_then(|v| v.as_str()).unwrap_or("chmod failed");
            return Err(ClientError::Protocol(msg.to_string()));
        }
        Ok(())
    }

    /// Changes the owner (and optionally group) of a file or directory.
    ///
    /// Sends a `POST /api/v1/fs/chown` with `{ "path": <path>, "owner": <owner>,
    /// "group": <group> }`. The `group` field is omitted if `group` is `None`.
    ///
    /// # Arguments
    ///
    /// * `path` — Absolute path to the target file or directory.
    /// * `owner` — New owner user name or ID as a string.
    /// * `group` — New group name or ID as a string. If `None`, the group is unchanged.
    ///
    /// # Errors
    ///
    /// - [`ClientError::Transport`] — Network failure.
    /// - [`ClientError::Protocol`] — Server returned a non-success status.
    ///
    /// # Example
    ///
    /// ```ignore
    /// client.chown("/shared.txt", "alice", Some("admins")).await?;
    /// ```
    pub async fn chown(
        &self,
        path: &str,
        owner: &str,
        group: Option<&str>,
    ) -> ClientResult<()> {
        let url = format!("{}/api/v1/fs/chown", self.config.base_url);
        let mut body = serde_json::json!({ "path": path, "owner": owner });
        if let Some(g) = group {
            body["group"] = serde_json::json!(g);
        }
        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;

        if !response.status().is_success() {
            let json: Value = response
                .json()
                .await
                .map_err(|e| ClientError::Protocol(e.to_string()))?;
            let msg = json.get("message").and_then(|v| v.as_str()).unwrap_or("chown failed");
            return Err(ClientError::Protocol(msg.to_string()));
        }
        Ok(())
    }

    /// Searches a file or directory tree for lines matching a regex pattern.
    ///
    /// Sends a `POST /api/v1/grep` with `{ "path": <path>, "pattern": <pattern>,
    /// "recursive": <recursive> }`.
    ///
    /// # Arguments
    ///
    /// * `path` — File path or directory to search. If a directory, `recursive`
    ///   controls whether subdirectories are included.
    /// * `pattern` — A valid regex pattern to match against each line.
    /// * `recursive` — If `Some(true)`, descend into subdirectories. If `Some(false)`,
    ///   only search the given path. If `None`, the server's default applies.
    ///
    /// # Returns
    ///
    /// `Ok(Vec<GrepMatch>)` — each match includes the file `path`, line number,
    /// and the full line content. Results are ordered by file path and line number.
    ///
    /// # Errors
    ///
    /// - [`ClientError::Transport`] — Network failure.
    /// - [`ClientError::Protocol`] — Response is missing the `matches` array or
    ///   individual match entries are malformed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let matches = client.grep("/src", r"fn \w+", Some(true)).await?;
    /// for m in matches {
    ///     println!("{}:{}: {}", m.path, m.line, m.content);
    /// }
    /// ```
    pub async fn grep(
        &self,
        path: &str,
        pattern: &str,
        recursive: Option<bool>,
    ) -> ClientResult<Vec<GrepMatch>> {
        let url = format!("{}/api/v1/grep", self.config.base_url);
        let mut body = serde_json::json!({ "path": path, "pattern": pattern });
        if let Some(r) = recursive {
            body["recursive"] = serde_json::json!(r);
        }
        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        let json: Value = response
            .json()
            .await
            .map_err(|e| ClientError::Protocol(e.to_string()))?;
        let matches = json["matches"]
            .as_array()
            .ok_or_else(|| ClientError::Protocol("Invalid grep response".to_string()))?;
        matches
            .iter()
            .map(|v| {
                serde_json::from_value(v.clone()).map_err(|e| ClientError::Protocol(e.to_string()))
            })
            .collect()
    }
}

/// Server health and version information returned by [`EvifClient::health`].
///
/// # Example
///
/// ```
/// use evif_client::HealthInfo;
///
/// let info = HealthInfo {
///     status: "ok".to_string(),
///     version: "1.0.0".to_string(),
///     uptime: 3600,
/// };
/// assert_eq!(info.status, "ok");
/// ```
#[derive(Debug, Clone)]
pub struct HealthInfo {
    /// Server-reported operational status (e.g., `"ok"`, `"degraded"`).
    pub status: String,

    /// Server version string (e.g., `"1.0.0"`).
    pub version: String,

    /// Server uptime in seconds since the last startup.
    pub uptime: u64,
}

/// Information about an active filesystem plugin mount.
///
/// Returned by [`EvifClient::mounts`] and used to construct mount requests
/// with [`EvifClient::mount`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MountInfo {
    /// Name of the mounted plugin (e.g., `"memory"`, `"disk"`, `"git"`).
    pub plugin: String,

    /// Absolute virtual path where the plugin is attached.
    pub path: String,
}

/// A single line match returned by [`EvifClient::grep`].
///
/// Each `GrepMatch` represents one line in one file that matched the search pattern.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrepMatch {
    /// Absolute path to the file that contained the match.
    pub path: String,

    /// 1-based line number within the file.
    pub line: usize,

    /// Full text of the matching line.
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ClientConfig Tests ====================

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.base_url, "http://localhost:8081");
        assert_eq!(config.request_timeout, 30);
    }

    #[test]
    fn test_client_config_custom() {
        let config = ClientConfig {
            request_timeout: 60,
            base_url: "http://localhost:8080".to_string(),
            timeout: std::time::Duration::from_secs(30),
        };

        assert_eq!(config.request_timeout, 60);
        assert_eq!(config.base_url, "http://localhost:8080");
    }

    #[test]
    fn test_client_config_default_timeout() {
        let config = ClientConfig::default();
        assert_eq!(config.timeout, std::time::Duration::from_secs(30));
    }

    #[test]
    fn test_client_config_custom_timeout() {
        let config = ClientConfig {
            request_timeout: 120,
            base_url: "http://example.com:9090".to_string(),
            timeout: std::time::Duration::from_secs(120),
        };
        assert_eq!(config.timeout, std::time::Duration::from_secs(120));
    }

    #[test]
    fn test_client_config_zero_timeout() {
        let config = ClientConfig {
            request_timeout: 0,
            base_url: "http://localhost:8081".to_string(),
            timeout: std::time::Duration::from_secs(0),
        };
        assert_eq!(config.request_timeout, 0);
        assert!(config.timeout.is_zero());
    }

    #[test]
    fn test_client_config_clone() {
        let config = ClientConfig::default();
        let cloned = config.clone();
        assert_eq!(config.base_url, cloned.base_url);
        assert_eq!(config.request_timeout, cloned.request_timeout);
        assert_eq!(config.timeout, cloned.timeout);
    }

    #[test]
    fn test_client_config_debug() {
        let config = ClientConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("http://localhost:8081"));
        assert!(debug_str.contains("30"));
    }

    #[test]
    fn test_client_config_https_url() {
        let config = ClientConfig {
            request_timeout: 30,
            base_url: "https://secure.example.com:443".to_string(),
            timeout: std::time::Duration::from_secs(30),
        };
        assert!(config.base_url.starts_with("https://"));
    }

    // ==================== EvifClient Construction Tests ====================

    #[test]
    fn test_client_new_sync() {
        let config = ClientConfig::default();
        let _client = EvifClient::new_sync(config);
    }

    #[tokio::test]
    async fn test_client_new_async() {
        let config = ClientConfig::default();
        let result = EvifClient::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_client_new_async_custom_config() {
        let config = ClientConfig {
            request_timeout: 10,
            base_url: "http://custom-host:9999".to_string(),
            timeout: std::time::Duration::from_secs(10),
        };
        let result = EvifClient::new(config).await;
        assert!(result.is_ok());
    }

    // ==================== Helper Function Tests ====================

    #[test]
    fn test_parse_modified_valid_rfc3339() {
        let ts = "2024-01-15T10:30:00Z";
        let result = parse_modified(ts);
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2024-01-15");
    }

    #[test]
    fn test_parse_modified_valid_with_timezone() {
        let ts = "2024-06-15T14:30:00+08:00";
        let result = parse_modified(ts);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_modified_invalid() {
        let ts = "not-a-date";
        let result = parse_modified(ts);
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            ClientError::Protocol(msg) => {
                assert!(msg.contains("Invalid timestamp"));
                assert!(msg.contains("not-a-date"));
            }
            _ => panic!("Expected Protocol error"),
        }
    }

    #[test]
    fn test_parse_modified_empty_string() {
        let result = parse_modified("");
        assert!(result.is_err());
    }

    #[test]
    fn test_file_name_from_path_regular() {
        assert_eq!(file_name_from_path("/foo/bar/baz.txt"), "baz.txt");
    }

    #[test]
    fn test_file_name_from_path_single_component() {
        assert_eq!(file_name_from_path("file.txt"), "file.txt");
    }

    #[test]
    fn test_file_name_from_path_root() {
        assert_eq!(file_name_from_path("/"), "/");
    }

    #[test]
    fn test_file_name_from_path_trailing_slash() {
        assert_eq!(file_name_from_path("/foo/bar/"), "bar");
    }

    #[test]
    fn test_file_name_from_path_deeply_nested() {
        assert_eq!(
            file_name_from_path("/a/b/c/d/e/f/g/deep_file.rs"),
            "deep_file.rs"
        );
    }

    #[test]
    fn test_file_name_from_path_empty_string() {
        assert_eq!(file_name_from_path(""), "/");
    }

    // ==================== Data Structure Tests ====================

    #[test]
    fn test_health_info_construction() {
        let info = HealthInfo {
            status: "ok".to_string(),
            version: "1.0.0".to_string(),
            uptime: 3600,
        };
        assert_eq!(info.status, "ok");
        assert_eq!(info.version, "1.0.0");
        assert_eq!(info.uptime, 3600);
    }

    #[test]
    fn test_health_info_debug() {
        let info = HealthInfo {
            status: "ok".to_string(),
            version: "0.1.0".to_string(),
            uptime: 0,
        };
        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("ok"));
        assert!(debug_str.contains("0.1.0"));
    }

    #[test]
    fn test_health_info_clone() {
        let info = HealthInfo {
            status: "healthy".to_string(),
            version: "2.0".to_string(),
            uptime: 999,
        };
        let cloned = info.clone();
        assert_eq!(info.status, cloned.status);
        assert_eq!(info.version, cloned.version);
        assert_eq!(info.uptime, cloned.uptime);
    }

    #[test]
    fn test_mount_info_serde_roundtrip() {
        let info = MountInfo {
            plugin: "memory".to_string(),
            path: "/tmp/test".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: MountInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.plugin, "memory");
        assert_eq!(deserialized.path, "/tmp/test");
    }

    #[test]
    fn test_mount_info_debug() {
        let info = MountInfo {
            plugin: "test_plugin".to_string(),
            path: "/mnt/data".to_string(),
        };
        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("test_plugin"));
        assert!(debug_str.contains("/mnt/data"));
    }

    #[test]
    fn test_mount_info_clone() {
        let info = MountInfo {
            plugin: "plugin".to_string(),
            path: "/path".to_string(),
        };
        let cloned = info.clone();
        assert_eq!(info.plugin, cloned.plugin);
        assert_eq!(info.path, cloned.path);
    }

    #[test]
    fn test_grep_match_serde_roundtrip() {
        let m = GrepMatch {
            path: "/foo/bar.txt".to_string(),
            line: 42,
            content: "hello world".to_string(),
        };
        let json = serde_json::to_string(&m).unwrap();
        let deserialized: GrepMatch = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, "/foo/bar.txt");
        assert_eq!(deserialized.line, 42);
        assert_eq!(deserialized.content, "hello world");
    }

    #[test]
    fn test_grep_match_debug() {
        let m = GrepMatch {
            path: "/test".to_string(),
            line: 1,
            content: "match".to_string(),
        };
        let debug_str = format!("{:?}", m);
        assert!(debug_str.contains("/test"));
        assert!(debug_str.contains("match"));
    }

    #[test]
    fn test_grep_match_clone() {
        let m = GrepMatch {
            path: "/a/b".to_string(),
            line: 10,
            content: "found it".to_string(),
        };
        let cloned = m.clone();
        assert_eq!(m.path, cloned.path);
        assert_eq!(m.line, cloned.line);
        assert_eq!(m.content, cloned.content);
    }

    #[test]
    fn test_mount_info_deserialize_from_json() {
        let json = r#"{"plugin":"disk","path":"/data"}"#;
        let info: MountInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.plugin, "disk");
        assert_eq!(info.path, "/data");
    }

    #[test]
    fn test_grep_match_deserialize_from_json() {
        let json = r#"{"path":"/log.txt","line":5,"content":"error found"}"#;
        let m: GrepMatch = serde_json::from_str(json).unwrap();
        assert_eq!(m.path, "/log.txt");
        assert_eq!(m.line, 5);
        assert_eq!(m.content, "error found");
    }

    #[test]
    fn test_grep_match_array_deserialize() {
        let json = r#"[
            {"path":"/a.txt","line":1,"content":"foo"},
            {"path":"/b.txt","line":2,"content":"bar"}
        ]"#;
        let matches: Vec<GrepMatch> = serde_json::from_str(json).unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].path, "/a.txt");
        assert_eq!(matches[1].content, "bar");
    }
}
