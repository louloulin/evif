// ServerInfoFS - 服务器信息插件
//
// 对标 AGFS ServerInfoFS - 提供服务器运行时元数据

use evif_core::{EvifPlugin, FileInfo, WriteFlags, EvifResult, EvifError};
use async_trait::async_trait;
use chrono::Utc;
use std::time::Instant;

pub struct ServerInfoFsPlugin {
    start_time: Instant,
    version: String,
}

impl ServerInfoFsPlugin {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            start_time: Instant::now(),
            version: version.into(),
        }
    }

    fn known_file(path: &str) -> Option<&str> {
        match path.trim_start_matches('/') {
            "version" => Some("version"),
            "uptime" => Some("uptime"),
            "info" => Some("info"),
            "stats" => Some("stats"),
            "README" => Some("README"),
            _ => None,
        }
    }
}

#[async_trait]
impl EvifPlugin for ServerInfoFsPlugin {
    fn name(&self) -> &str {
        "serverinfofs"
    }

    async fn create(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        Err(EvifError::InvalidPath("Cannot create files in serverinfofs".to_string()))
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        Err(EvifError::InvalidPath("Cannot create directories in serverinfofs".to_string()))
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        let file_name = path.trim_start_matches('/');

        let content = match file_name {
            "version" => self.version.clone(),
            "uptime" => {
                let uptime = self.start_time.elapsed();
                format!("{:.2}s", uptime.as_secs_f64())
            }
            "info" => {
                let uptime = self.start_time.elapsed();
                serde_json::json!({
                    "version": self.version,
                    "uptime_secs": uptime.as_secs_f64(),
                    "uptime_human": format!("{:.2}s", uptime.as_secs_f64()),
                    "start_time": Utc::now().timestamp() - uptime.as_secs() as i64,
                }).to_string()
            }
            "stats" => {
                // 简单的运行时统计
                serde_json::json!({
                    "goroutines": 0,  // Rust不直接暴露goroutine数量
                    "memory": "N/A",  // 可以通过系统API获取
                    "version": self.version,
                }).to_string()
            }
            "README" => {
                self.get_readme()
            }
            _ => return Err(EvifError::NotFound(path.to_string())),
        };

        Ok(content.into_bytes())
    }

    async fn write(&self, _path: &str, _data: Vec<u8>, _offset: i64, _flags: WriteFlags)
        -> EvifResult<u64>
    {
        Err(EvifError::ReadOnly)
    }

    async fn readdir(&self, _path: &str) -> EvifResult<Vec<FileInfo>> {
        if _path != "/" && !_path.is_empty() {
            return Err(EvifError::InvalidPath("Not a directory".to_string()));
        }

        let files = vec![
            "version",
            "uptime",
            "info",
            "stats",
            "README",
        ];

        let now = Utc::now();
        let entries = files.into_iter().map(|name| FileInfo {
            name: name.to_string(),
            size: 0,
            mode: 0o444, // read-only
            modified: now,
            is_dir: false,
        }).collect();

        Ok(entries)
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let file_name = path.trim_start_matches('/');
        let now = Utc::now();

        if file_name.is_empty() || file_name == "/" {
            return Ok(FileInfo {
                name: "/".to_string(),
                size: 0,
                mode: 0o444,
                modified: now,
                is_dir: true,
            });
        }

        let known = Self::known_file(path).ok_or_else(|| EvifError::NotFound(path.to_string()))?;

        Ok(FileInfo {
            name: known.to_string(),
            size: 0,
            mode: 0o444,
            modified: now,
            is_dir: false,
        })
    }

    async fn remove(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::ReadOnly)
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        Err(EvifError::ReadOnly)
    }

    async fn remove_all(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::ReadOnly)
    }
}

impl ServerInfoFsPlugin {
    fn get_readme(&self) -> String {
        format!(r#"ServerInfoFS Plugin - Server Metadata and Information

This plugin provides runtime information about the EVIF server.

USAGE:
  View server version:
    cat /version

  View server uptime:
    cat /uptime

  View server info:
    cat /info

  View runtime stats:
    cat /stats

FILES:
  /version  - Server version information
  /uptime   - Server uptime since start
  /info     - Complete server information (JSON)
  /stats    - Runtime statistics
  /README   - This file

EXAMPLES:
  # Check server version
  cat /serverinfofs/version
  {}

  # Check server uptime
  cat /serverinfofs/uptime

  # View complete server info
  cat /serverinfofs/info
"#, self.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_serverinfofs_basic() {
        let plugin = ServerInfoFsPlugin::new("1.0.0");

        // 测试读取版本
        let version_data = plugin.read("/version", 0, 100).await.unwrap();
        assert_eq!(version_data, b"1.0.0");

        // 测试读取uptime
        let uptime_data = plugin.read("/uptime", 0, 100).await.unwrap();
        assert!(!uptime_data.is_empty());
        assert!(uptime_data.ends_with(b"s"));

        // 测试读取info (JSON格式)
        let info_data = plugin.read("/info", 0, 1000).await.unwrap();
        assert!(!info_data.is_empty());
        let info_str = String::from_utf8(info_data).unwrap();
        assert!(info_str.contains("version"));
        assert!(info_str.contains("uptime"));

        // 测试readdir
        let entries = plugin.readdir("/").await.unwrap();
        assert!(entries.iter().any(|e| e.name == "version"));
        assert!(entries.iter().any(|e| e.name == "uptime"));
        assert!(entries.iter().any(|e| e.name == "info"));

        // 测试stat
        let info = plugin.stat("/version").await.unwrap();
        assert_eq!(info.name, "version");
        assert_eq!(info.mode, 0o444);
        assert!(!info.is_dir);

        // 测试只读
        let result = plugin.write("/version", b"test".to_vec(), 0, WriteFlags::CREATE).await;
        assert!(result.is_err());
    }
}
