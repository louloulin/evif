// CLI 命令实现

use crate::repl::Repl;
use anyhow::Result;
use base64::Engine;
use chrono::Utc;
use evif_client::EvifClient;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;

pub struct EvifCommand {
    server: String,
    verbose: bool,
    client: EvifClient,
    cwd: Arc<Mutex<String>>,
    env_vars: Arc<Mutex<std::collections::HashMap<String, String>>>,
}

fn normalize_base_url(server: &str) -> String {
    let s = server.trim();
    if s.starts_with("http://") || s.starts_with("https://") {
        s.to_string()
    } else {
        format!("http://{}", s)
    }
}

#[allow(dead_code)]
impl EvifCommand {
    pub fn new(server: String, verbose: bool) -> Self {
        let base_url = normalize_base_url(&server);
        let config = evif_client::ClientConfig {
            request_timeout: 30,
            base_url,
            timeout: std::time::Duration::from_secs(30),
        };

        // 使用同步方法创建客户端
        let client = evif_client::EvifClient::new_sync(config);
        Self {
            server,
            verbose,
            client,
            cwd: Arc::new(Mutex::new("/".to_string())),
            env_vars: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub async fn ls_output(&self, path: Option<String>, long: bool) -> Result<String> {
        let path = path.unwrap_or_else(|| "/".to_string());
        let files = self.client.ls(&path).await?;

        let mut lines = Vec::new();
        if long {
            lines.push(format!(
                "{:<8} {:<12} {:<20} {:<40}",
                "Mode", "Size", "Modified", "Name"
            ));
            lines.push("-".repeat(80));
            for file in files {
                let mode = if file.is_dir {
                    "drwxr-xr-x"
                } else {
                    "-rw-r--r--"
                };
                let modified = file.modified.format("%Y-%m-%d %H:%M").to_string();
                let size = if file.is_dir {
                    "".to_string()
                } else {
                    format!("{}", file.size)
                };
                lines.push(format!(
                    "{:<8} {:<12} {:<20} {:<40}",
                    mode, size, modified, file.name
                ));
            }
        } else {
            for file in files {
                if file.is_dir {
                    lines.push(format!("{}/", file.name));
                } else {
                    lines.push(file.name);
                }
            }
        }

        Ok(if lines.is_empty() {
            String::new()
        } else {
            format!("{}\n", lines.join("\n"))
        })
    }

    /// 列出目录内容
    pub async fn ls(&self, path: Option<String>, _recursive: bool, long: bool) -> Result<()> {
        let output = self.ls_output(path, long).await?;
        print!("{}", output);
        Ok(())
    }

    pub async fn cat_output(&self, path: String) -> Result<String> {
        self.client.cat(&path).await.map_err(Into::into)
    }

    /// 显示文件内容
    pub async fn cat(&self, path: String) -> Result<()> {
        let content = self.cat_output(path).await?;
        print!("{}", content);
        Ok(())
    }

    /// 写入文件
    pub async fn write(&self, path: String, content: String, append: bool) -> Result<()> {
        self.client.write(&path, &content, append).await?;
        println!("Written to {}", path);
        Ok(())
    }

    /// 创建目录
    pub async fn mkdir(&self, path: String, parents: bool) -> Result<()> {
        if parents {
            // 递归创建目录
            let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
            let mut current = String::new();
            for part in parts {
                current = format!("{}/{}", current, part);
                if !current.is_empty() {
                    self.client.mkdir(&current, true).await.unwrap_or(());
                }
            }
        } else {
            self.client.mkdir(&path, false).await?;
        }
        println!("Created directory: {}", path);
        Ok(())
    }

    /// 删除文件或目录
    pub async fn rm(&self, path: String, recursive: bool) -> Result<()> {
        if recursive {
            self.client.remove_all(&path).await?;
        } else {
            self.client.remove(&path).await?;
        }
        println!("Removed: {}", path);
        Ok(())
    }

    /// 移动/重命名文件
    pub async fn mv(&self, src: String, dst: String) -> Result<()> {
        self.client.rename(&src, &dst).await?;
        println!("Moved: {} -> {}", src, dst);
        Ok(())
    }

    /// 复制文件
    pub async fn cp(&self, src: String, dst: String) -> Result<()> {
        let content = self.client.cat(&src).await?;
        self.client.write(&dst, &content, false).await?;
        println!("Copied: {} -> {}", src, dst);
        Ok(())
    }

    /// 显示文件状态
    pub async fn stat(&self, path: String) -> Result<()> {
        let info = self.client.stat(&path).await?;
        println!("File: {}", path);
        println!("  Size: {} bytes", info.size);
        println!("  Type: {}", if info.is_dir { "Directory" } else { "File" });
        println!("  Modified: {:?}", info.modified);
        println!("  Mode: {:o}", info.mode);
        Ok(())
    }

    /// 创建空文件
    pub async fn touch(&self, path: String) -> Result<()> {
        self.client.write(&path, "", false).await?;
        println!("Created: {}", path);
        Ok(())
    }

    /// 显示文件头部
    pub async fn head(&self, path: String, lines: usize) -> Result<()> {
        let content = self.client.cat(&path).await?;
        let content_lines: Vec<&str> = content.lines().take(lines).collect();
        for line in content_lines {
            println!("{}", line);
        }
        Ok(())
    }

    /// 显示文件尾部
    pub async fn tail(&self, path: String, lines: usize) -> Result<()> {
        let content = self.client.cat(&path).await?;
        let content_lines: Vec<&str> = content.lines().collect();
        let total_lines = content_lines.len();
        let start = total_lines.saturating_sub(lines);
        for line in content_lines.iter().skip(start) {
            println!("{}", line);
        }
        Ok(())
    }

    /// 显示文件类型
    pub async fn tree(&self, path: String, depth: usize, max_depth: usize) -> Result<()> {
        self.tree_inner(path, depth, max_depth).await
    }

    async fn tree_inner(&self, path: String, depth: usize, max_depth: usize) -> Result<()> {
        let files = self.client.ls(&path).await?;
        let indent = "  ".repeat(depth);

        for (i, file) in files.iter().enumerate() {
            let is_last = i == files.len() - 1;
            let prefix = if is_last { "└── " } else { "├── " };
            println!("{}{}{}", indent, prefix, file.name);

            if file.is_dir && depth < max_depth {
                let new_path = format!("{}/{}", path.trim_end_matches('/'), file.name);
                Box::pin(self.tree_inner(new_path, depth + 1, max_depth)).await?;
            }
        }
        Ok(())
    }

    /// 挂载插件
    pub async fn mount(&self, plugin: String, path: String, config: Option<String>) -> Result<()> {
        self.client.mount(&plugin, &path, config.as_deref()).await?;
        println!("Mounted {} at {}", plugin, path);
        Ok(())
    }

    /// 卸载插件
    pub async fn unmount(&self, path: String) -> Result<()> {
        self.client.unmount(&path).await?;
        println!("Unmounted {}", path);
        Ok(())
    }

    /// 列出挂载点
    pub async fn mounts(&self) -> Result<()> {
        let mounts = self.client.mounts().await?;
        println!("Mounted plugins:");
        for mount in mounts {
            println!("  {} at {}", mount.plugin, mount.path);
        }
        Ok(())
    }

    /// 健康检查
    pub async fn health(&self) -> Result<()> {
        let health = self.client.health().await?;
        println!("EVIF Server Status:");
        println!("  Status: {}", health.status);
        println!("  Version: {}", health.version);
        println!("  Uptime: {}s", health.uptime);
        Ok(())
    }

    /// 进入REPL模式
    pub async fn repl(&self) -> Result<()> {
        let mut repl = Repl::new(self.server.clone(), self.verbose);
        repl.run().await
    }

    pub async fn script(&self, path: String) -> Result<()> {
        crate::script::ScriptExecutor::execute_script_with_client(&path, self).await
    }

    /// 统计信息
    pub async fn stats(&self) -> Result<()> {
        println!("EVIF Statistics");
        println!("================");
        println!("Server: {}", self.server);
        println!("Status: Connected");

        let health = self.client.health().await.ok();
        if let Some(h) = health {
            println!("Version: {}", h.version);
            println!("Uptime: {}s", h.uptime);
        }
        Ok(())
    }

    // ============== 高级命令 ==============

    /// 修改文件权限
    pub async fn chmod(&self, path: String, mode: String) -> Result<()> {
        // 解析 mode 字符串为 u32 (支持八进制如 "755" 或十进制如 "493")
        let mode_val = if mode.starts_with("0") {
            u32::from_str_radix(&mode[1..], 8)
                .map_err(|e| anyhow::anyhow!("Invalid octal mode: {}", e))?
        } else {
            mode.parse::<u32>()
                .map_err(|e| anyhow::anyhow!("Invalid mode: {}", e))?
        };

        self.client.chmod(&path, mode_val).await?;
        println!("Changed mode of {} to {:o}", path, mode_val);
        Ok(())
    }

    /// 修改文件所有者
    pub async fn chown(&self, path: String, owner: String, group: Option<String>) -> Result<()> {
        self.client.chown(&path, &owner, group.as_deref()).await?;
        let group_info = group.map(|g| format!(":{}", g)).unwrap_or_default();
        println!("Changed owner of {} to {}{}", path, owner, group_info);
        Ok(())
    }

    /// 上传文件到EVIF
    pub async fn upload(&self, local_path: String, remote_path: String) -> Result<()> {
        // 读取本地文件
        let content = tokio::fs::read_to_string(&local_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read local file: {}", e))?;

        // 写入到EVIF
        self.client.write(&remote_path, &content, false).await?;
        println!("Uploaded: {} -> {}", local_path, remote_path);
        Ok(())
    }

    /// 从EVIF下载文件
    pub async fn download(&self, remote_path: String, local_path: String) -> Result<()> {
        // 从EVIF读取
        let content = self.client.cat(&remote_path).await?;

        // 写入本地文件
        tokio::fs::write(&local_path, content)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to write local file: {}", e))?;

        println!("Downloaded: {} -> {}", remote_path, local_path);
        Ok(())
    }

    /// 显示文件差异
    pub async fn diff(&self, path1: String, path2: String) -> Result<()> {
        let content1 = self.client.cat(&path1).await?;
        let content2 = self.client.cat(&path2).await?;

        if content1 == content2 {
            println!("Files are identical");
        } else {
            println!("Files differ:");
            println!("{}: {} bytes", path1, content1.len());
            println!("{}: {} bytes", path2, content2.len());

            // 简单的逐行对比
            let lines1: Vec<&str> = content1.lines().collect();
            let lines2: Vec<&str> = content2.lines().collect();

            for (i, (l1, l2)) in lines1.iter().zip(lines2.iter()).enumerate() {
                if l1 != l2 {
                    println!("Line {} differs:", i + 1);
                    println!("  {}: {}", path1, l1);
                    println!("  {}: {}", path2, l2);
                }
            }
        }
        Ok(())
    }

    /// 统计文件/目录大小
    pub async fn du(&self, path: String, recursive: bool) -> Result<()> {
        let mut total_size = 0u64;
        let mut total_files = 0usize;
        let mut total_dirs = 0usize;

        self.calculate_size(
            &path,
            &mut total_size,
            &mut total_files,
            &mut total_dirs,
            recursive,
        )
        .await?;

        println!("Size summary for: {}", path);
        println!("  Total size: {} bytes", total_size);
        println!("  Files: {}", total_files);
        println!("  Directories: {}", total_dirs);
        println!("  Size: {}", self.format_size(total_size));
        Ok(())
    }

    /// 递归计算目录大小
    async fn calculate_size(
        &self,
        path: &str,
        total_size: &mut u64,
        total_files: &mut usize,
        total_dirs: &mut usize,
        recursive: bool,
    ) -> Result<()> {
        let files = self.client.ls(path).await?;

        for file in files {
            if file.is_dir {
                *total_dirs += 1;
                if recursive {
                    let new_path = format!("{}/{}", path.trim_end_matches('/'), file.name);
                    Box::pin(self.calculate_size(
                        &new_path,
                        total_size,
                        total_files,
                        total_dirs,
                        recursive,
                    ))
                    .await?;
                }
            } else {
                *total_files += 1;
                *total_size += file.size;
            }
        }
        Ok(())
    }

    /// 格式化文件大小
    fn format_size(&self, size: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if size >= GB {
            format!("{:.2} GB", size as f64 / GB as f64)
        } else if size >= MB {
            format!("{:.2} MB", size as f64 / MB as f64)
        } else if size >= KB {
            format!("{:.2} KB", size as f64 / KB as f64)
        } else {
            format!("{} B", size)
        }
    }

    /// 监控文件变化
    pub async fn watch(&self, path: String, interval: u64) -> Result<()> {
        println!("Watching: {} (interval: {}s)", path, interval);
        println!("Press Ctrl+C to stop");

        let mut last_files = std::collections::HashMap::new();

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;

            let files = self.client.ls(&path).await?;
            let current_files: std::collections::HashMap<String, (bool, u64)> = files
                .iter()
                .map(|f| (f.name.clone(), (f.is_dir, f.size)))
                .collect();

            // 检测新增文件
            for name in current_files.keys() {
                if !last_files.contains_key(name) {
                    println!("[+] {}", name);
                }
            }

            // 检测删除文件
            for name in last_files.keys() {
                if !current_files.contains_key(name) {
                    println!("[-] {}", name);
                }
            }

            last_files = current_files;
        }
    }

    /// 显示文件类型
    pub async fn file_type(&self, path: String) -> Result<()> {
        let info = self.client.stat(&path).await?;

        if info.is_dir {
            println!("{}: directory", path);
        } else {
            // 根据扩展名判断文件类型
            let ext = std::path::Path::new(&path)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");

            let file_type = match ext {
                "txt" | "md" | "rst" => "text",
                "json" | "yaml" | "toml" => "data",
                "rs" | "go" | "py" | "js" | "ts" => "source code",
                "png" | "jpg" | "jpeg" | "gif" | "svg" => "image",
                "mp4" | "avi" | "mkv" | "mov" => "video",
                "mp3" | "wav" | "ogg" | "flac" => "audio",
                "zip" | "tar" | "gz" | "bz2" => "archive",
                _ => "unknown",
            };

            println!("{}: {} ({} bytes)", path, file_type, info.size);
        }
        Ok(())
    }

    /// 生成文件校验和（Phase 10.1：使用 evif-client digest，POST /api/v1/digest）
    pub async fn checksum(&self, path: String, algorithm: String) -> Result<()> {
        let algo = if algorithm.is_empty() {
            None
        } else {
            Some(algorithm.as_str())
        };
        let (algo_name, hash) = self.client.digest(&path, algo).await?;
        println!("{}  {}  {}", algo_name, hash, path);
        Ok(())
    }

    /// 正则搜索（Phase 10.1：POST /api/v1/grep）
    pub async fn grep(&self, path: String, pattern: String, recursive: bool) -> Result<()> {
        let matches = self.client.grep(&path, &pattern, Some(recursive)).await?;
        for m in matches {
            println!("{}:{}:{}", m.path, m.line, m.content);
        }
        Ok(())
    }

    /// 批量操作
    pub async fn batch(&self, commands: Vec<String>) -> Result<()> {
        println!("Error: Batch command parsing not fully integrated");
        println!("Use individual commands or full REPL mode instead.");
        for (i, cmd) in commands.iter().enumerate() {
            println!("[{}/{}] {}", i + 1, commands.len(), cmd);
        }
        Ok(())
    }

    // ============== 批量操作命令 (EVIF 1.9 新增) ==============

    /// 批量复制文件
    pub async fn batch_copy(
        &self,
        sources: Vec<String>,
        destination: String,
        concurrency: usize,
        progress: bool,
    ) -> Result<()> {
        println!("Batch Copy:");
        println!("  Destination: {}", destination);
        println!("  Files: {}", sources.len());
        println!("  Concurrency: {}", concurrency);
        println!(
            "  Progress: {}",
            if progress { "enabled" } else { "disabled" }
        );

        let url = format!("{}/api/v1/batch/copy", self.server);
        let body = serde_json::json!({
            "sources": sources,
            "destination": destination,
            "concurrency": concurrency
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Request failed: {}", e))?;

        if resp.status().is_success() {
            let json: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| anyhow::anyhow!("Parse failed: {}", e))?;

            if let Some(op_id) = json.get("operation_id").and_then(|v| v.as_str()) {
                println!("Operation started: {}", op_id);

                if progress {
                    // 查询进度
                    self.wait_for_completion(op_id).await?;
                }
            }
        } else {
            println!("Error: {}", resp.status());
        }

        Ok(())
    }

    /// 批量删除文件
    pub async fn batch_delete(
        &self,
        paths: Vec<String>,
        recursive: bool,
        concurrency: usize,
        progress: bool,
    ) -> Result<()> {
        println!("Batch Delete:");
        println!("  Files: {}", paths.len());
        println!("  Recursive: {}", recursive);
        println!("  Concurrency: {}", concurrency);
        println!(
            "  Progress: {}",
            if progress { "enabled" } else { "disabled" }
        );

        let url = format!("{}/api/v1/batch/delete", self.server);
        let body = serde_json::json!({
            "paths": paths,
            "recursive": recursive,
            "concurrency": concurrency
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Request failed: {}", e))?;

        if resp.status().is_success() {
            let json: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| anyhow::anyhow!("Parse failed: {}", e))?;

            if let Some(op_id) = json.get("operation_id").and_then(|v| v.as_str()) {
                println!("Operation started: {}", op_id);

                if progress {
                    // 查询进度
                    self.wait_for_completion(op_id).await?;
                }
            }
        } else {
            println!("Error: {}", resp.status());
        }

        Ok(())
    }

    /// 列出批量操作
    pub async fn batch_list(&self) -> Result<()> {
        let url = format!("{}/api/v1/batch/operations", self.server);

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Request failed: {}", e))?;

        if resp.status().is_success() {
            let json: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| anyhow::anyhow!("Parse failed: {}", e))?;

            println!("Active Batch Operations:");
            println!("======================");

            if let Some(operations) = json.get("operations").and_then(|v| v.as_array()) {
                for op in operations {
                    if let Some(id) = op.get("id").and_then(|v| v.as_str()) {
                        println!("  ID: {}", id);
                    }
                    if let Some(op_type) = op.get("operation_type").and_then(|v| v.as_str()) {
                        println!("  Type: {}", op_type);
                    }
                    if let Some(status) = op.get("status").and_then(|v| v.as_str()) {
                        println!("  Status: {}", status);
                    }
                    if let Some(progress) = op.get("progress").and_then(|v| v.as_f64()) {
                        println!("  Progress: {:.1}%", progress);
                    }
                    if let Some(current) = op.get("current_file").and_then(|v| v.as_str()) {
                        println!("  Current: {}", current);
                    }
                    println!();
                }
            }

            if let Some(count) = json.get("count").and_then(|v| v.as_i64()) {
                println!("Total: {} operations", count);
            }
        } else {
            println!("Error: {}", resp.status());
        }

        Ok(())
    }

    /// 获取操作进度
    pub async fn batch_progress(&self, operation_id: String) -> Result<()> {
        let url = format!("{}/api/v1/batch/progress/{}", self.server, operation_id);

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Request failed: {}", e))?;

        if resp.status().is_success() {
            let json: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| anyhow::anyhow!("Parse failed: {}", e))?;

            println!("Operation Progress: {}", operation_id);
            println!("====================");

            if let Some(op_type) = json.get("operation_type").and_then(|v| v.as_str()) {
                println!("  Type: {}", op_type);
            }
            if let Some(status) = json.get("status").and_then(|v| v.as_str()) {
                println!("  Status: {}", status);
            }
            if let Some(progress) = json.get("progress").and_then(|v| v.as_f64()) {
                println!("  Progress: {:.1}%", progress);
            }
            if let Some(current) = json.get("current_file").and_then(|v| v.as_str()) {
                println!("  Current: {}", current);
            }
            if let Some(start_time) = json.get("start_time").and_then(|v| v.as_i64()) {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64;
                println!("  Started: {} ms ago", now_ms.saturating_sub(start_time));
            }
        } else {
            println!("Error: {}", resp.status());
        }

        Ok(())
    }

    /// 取消批量操作
    pub async fn batch_cancel(&self, operation_id: String) -> Result<()> {
        println!("Cancelling operation: {}", operation_id);

        let url = format!("{}/api/v1/batch/operation/{}", self.server, operation_id);

        let client = reqwest::Client::new();
        let resp = client
            .delete(&url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Request failed: {}", e))?;

        if resp.status().is_success() {
            println!("Operation cancelled successfully");
        } else {
            println!("Error: {}", resp.status());
        }

        Ok(())
    }

    /// 等待操作完成
    async fn wait_for_completion(&self, operation_id: &str) -> Result<()> {
        println!("Waiting for operation to complete...");
        println!("Press Ctrl+C to stop monitoring");

        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            let url = format!("{}/api/v1/batch/progress/{}", self.server, operation_id);
            let client = reqwest::Client::new();

            match client.get(&url).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        if let Ok(json) = resp.json::<serde_json::Value>().await {
                            if let Some(status) = json.get("status").and_then(|v| v.as_str()) {
                                if let Some(progress) =
                                    json.get("progress").and_then(|v| v.as_f64())
                                {
                                    print!("\rProgress: {:.1}%", progress);
                                    std::io::stdout().flush().unwrap();
                                }

                                if status == "Completed" {
                                    println!("\nOperation completed!");
                                    return Ok(());
                                } else if status == "Failed" {
                                    println!("\nOperation failed!");
                                    if let Some(err) = json.get("error").and_then(|v| v.as_str()) {
                                        println!("Error: {}", err);
                                    }
                                    return Ok(());
                                } else if status == "Cancelled" {
                                    println!("\nOperation cancelled!");
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    // Ignore errors, retry
                }
            }
        }
    }

    // ============== 新增高级命令 ==============

    /// 输出文本
    pub async fn echo(&self, text: String) -> Result<()> {
        print!("{}", self.echo_output(text));
        Ok(())
    }

    pub fn echo_output(&self, text: String) -> String {
        format!("{}\n", text)
    }

    /// 切换当前工作目录
    pub async fn cd(&self, path: String) -> Result<()> {
        let current_cwd = self.cwd.lock().unwrap().clone();
        let new_cwd = if path.starts_with('/') {
            path
        } else {
            format!("{}/{}", current_cwd.trim_end_matches('/'), path)
        };
        drop(current_cwd);

        // 验证路径是否存在
        match self.client.stat(&new_cwd).await {
            Ok(info) if info.is_dir => {
                *self.cwd.lock().unwrap() = new_cwd;
                Ok(())
            }
            Ok(_) => Err(anyhow::anyhow!("Not a directory: {}", new_cwd)),
            Err(_e) => Err(anyhow::anyhow!("Directory not found: {}", new_cwd)),
        }
    }

    /// 打印当前工作目录
    pub async fn pwd(&self) -> Result<()> {
        print!("{}", self.pwd_output());
        Ok(())
    }

    pub fn pwd_output(&self) -> String {
        format!("{}\n", self.cwd.lock().unwrap())
    }

    /// 排序文本行
    pub async fn sort(
        &self,
        file: Option<String>,
        reverse: bool,
        numeric: bool,
        unique: bool,
    ) -> Result<()> {
        let content = if let Some(ref path) = file {
            // 判断是本地文件还是 EVIF 路径
            if path.starts_with('/') && !std::path::Path::new(path).exists() {
                self.client.cat(path).await?
            } else {
                tokio::fs::read_to_string(path)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to read local file: {}", e))?
            }
        } else {
            return Err(anyhow::anyhow!(
                "Reading from stdin not yet supported. Please provide a file."
            ));
        };

        let mut lines: Vec<&str> = content.lines().collect();

        if numeric {
            lines.sort_by(|a, b| {
                let a_val = a
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(f64::NAN);
                let b_val = b
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(f64::NAN);
                a_val
                    .partial_cmp(&b_val)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        } else {
            lines.sort();
        }

        if unique {
            lines.dedup();
        }

        if reverse {
            for line in lines.iter().rev() {
                println!("{}", line);
            }
        } else {
            for line in lines {
                println!("{}", line);
            }
        }

        Ok(())
    }

    /// 去重文本行
    pub async fn uniq(&self, file: Option<String>, count: bool) -> Result<()> {
        let content = if let Some(ref path) = file {
            if path.starts_with('/') && !std::path::Path::new(path).exists() {
                self.client.cat(path).await?
            } else {
                tokio::fs::read_to_string(path)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to read local file: {}", e))?
            }
        } else {
            return Err(anyhow::anyhow!(
                "Reading from stdin not yet supported. Please provide a file."
            ));
        };

        let mut prev_line = "";
        let mut count_val = 0;

        for line in content.lines() {
            if line == prev_line {
                count_val += 1;
            } else {
                if count && count_val > 0 {
                    println!("{} {}", count_val + 1, prev_line);
                } else if !prev_line.is_empty() {
                    println!("{}", prev_line);
                }
                prev_line = line;
                count_val = 0;
            }
        }

        // 输出最后一行
        if count && count_val > 0 {
            println!("{} {}", count_val + 1, prev_line);
        } else if !prev_line.is_empty() {
            println!("{}", prev_line);
        }

        Ok(())
    }

    /// 统计文本行数、字数、字节数
    pub async fn wc(
        &self,
        file: Option<String>,
        lines: bool,
        words: bool,
        bytes: bool,
    ) -> Result<()> {
        let content = if let Some(ref path) = file {
            if path.starts_with('/') && !std::path::Path::new(path).exists() {
                self.client.cat(path).await?
            } else {
                tokio::fs::read_to_string(path)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to read local file: {}", e))?
            }
        } else {
            return Err(anyhow::anyhow!(
                "Reading from stdin not yet supported. Please provide a file."
            ));
        };

        let line_count = content.lines().count();
        let word_count = content.split_whitespace().count();
        let byte_count = content.len();

        // 默认显示所有
        let show_lines = lines || !words && !bytes;
        let show_words = words || !lines && !bytes;
        let show_bytes = bytes || !lines && !words;

        let mut parts = Vec::new();
        if show_lines {
            parts.push(format!("{}", line_count));
        }
        if show_words {
            parts.push(format!("{}", word_count));
        }
        if show_bytes {
            parts.push(format!("{}", byte_count));
        }

        println!("{}", parts.join(" "));
        Ok(())
    }

    /// 显示当前日期时间
    pub async fn date(&self, format: Option<String>) -> Result<()> {
        let now = Utc::now();
        if let Some(fmt) = format {
            // 简单的格式化支持
            let formatted = now.format(&fmt).to_string();
            println!("{}", formatted);
        } else {
            println!("{}", now.to_rfc3339());
        }
        Ok(())
    }

    /// 延迟执行
    pub async fn sleep(&self, seconds: u64) -> Result<()> {
        tokio::time::sleep(tokio::time::Duration::from_secs(seconds)).await;
        Ok(())
    }

    /// Cut 命令：从每行提取指定部分
    pub async fn cut(
        &self,
        file: Option<String>,
        bytes: Option<String>,
        chars: Option<String>,
        fields: Option<String>,
        delimiter: Option<String>,
    ) -> Result<()> {
        let content = if let Some(ref path) = file {
            self.client.cat(path).await?
        } else {
            return Err(anyhow::anyhow!(
                "Reading from stdin not yet supported. Please provide a file."
            ));
        };

        for line in content.lines() {
            let result = if let Some(ref byte_ranges) = bytes {
                self.cut_bytes(line, byte_ranges)
            } else if let Some(ref char_ranges) = chars {
                self.cut_chars(line, char_ranges)
            } else if let Some(ref field_list) = fields {
                let delim = delimiter.as_deref().unwrap_or("\t");
                self.cut_fields(line, field_list, delim)
            } else {
                line.to_string()
            };
            println!("{}", result);
        }

        Ok(())
    }

    /// 按字节切割
    fn cut_bytes(&self, line: &str, ranges: &str) -> String {
        let bytes = line.as_bytes();
        let ranges = self.parse_ranges(ranges, bytes.len());
        let result: Vec<u8> = ranges
            .iter()
            .flat_map(|(start, end)| bytes.get(*start..*end).unwrap_or(&[]))
            .copied()
            .collect();
        String::from_utf8_lossy(&result).to_string()
    }

    /// 按字符切割
    fn cut_chars(&self, line: &str, ranges: &str) -> String {
        let chars: Vec<char> = line.chars().collect();
        let ranges = self.parse_ranges(ranges, chars.len());
        let result: String = ranges
            .iter()
            .flat_map(|(start, end)| chars.get(*start..*end).unwrap_or(&[]))
            .collect();
        result
    }

    /// 按字段切割
    fn cut_fields(&self, line: &str, field_list: &str, delimiter: &str) -> String {
        let fields: Vec<&str> = line.split(delimiter).collect();
        let selected: Vec<&str> = field_list
            .split(',')
            .filter_map(|s| s.trim().parse::<usize>().ok())
            .filter_map(|i| {
                if i > 0 && i <= fields.len() {
                    Some(fields[i - 1])
                } else {
                    None
                }
            })
            .collect();
        selected.join(delimiter)
    }

    /// 解析范围字符串（如 "1-5,8"）
    fn parse_ranges(&self, range_str: &str, max_len: usize) -> Vec<(usize, usize)> {
        let mut ranges = Vec::new();
        for part in range_str.split(',') {
            let part = part.trim();
            if let Some((start, end)) = part.split_once('-') {
                let start = start.trim().parse::<usize>().unwrap_or(1);
                let end = end.trim().parse::<usize>().unwrap_or(max_len);
                let start = start.saturating_sub(1).min(max_len);
                let end = end.min(max_len);
                if start < end {
                    ranges.push((start, end));
                }
            } else {
                let pos = part.parse::<usize>().unwrap_or(1);
                let pos = pos.saturating_sub(1).min(max_len);
                if pos < max_len {
                    ranges.push((pos, pos + 1));
                }
            }
        }
        ranges
    }

    /// Tr 命令：转换或删除字符
    pub async fn tr_(
        &self,
        file: Option<String>,
        from: String,
        to: String,
        delete: bool,
    ) -> Result<()> {
        let content = if let Some(ref path) = file {
            self.client.cat(path).await?
        } else {
            return Err(anyhow::anyhow!(
                "Reading from stdin not yet supported. Please provide a file."
            ));
        };

        let from_chars: Vec<char> = from.chars().collect();
        let to_chars: Vec<char> = to.chars().collect();

        for line in content.lines() {
            let result: String = if delete {
                line.chars().filter(|c| !from_chars.contains(c)).collect()
            } else {
                line.chars()
                    .map(|c| {
                        from_chars
                            .iter()
                            .position(|&fc| fc == c)
                            .and_then(|pos| to_chars.get(pos).copied())
                            .unwrap_or(c)
                    })
                    .collect()
            };
            println!("{}", result);
        }

        Ok(())
    }

    /// Base 命令：base64 编码/解码
    pub async fn base(&self, file: Option<String>, decode: bool) -> Result<()> {
        let content = if let Some(ref path) = file {
            self.client.cat(path).await?
        } else {
            return Err(anyhow::anyhow!(
                "Reading from stdin not yet supported. Please provide a file."
            ));
        };

        if decode {
            // 解码
            match base64::engine::general_purpose::STANDARD.decode(content.trim()) {
                Ok(decoded) => {
                    println!("{}", String::from_utf8_lossy(&decoded));
                }
                Err(_e) => {
                    return Err(anyhow::anyhow!("Invalid base64 input"));
                }
            }
        } else {
            // 编码
            let encoded = base64::engine::general_purpose::STANDARD.encode(content);
            println!("{}", encoded);
        }

        Ok(())
    }

    /// 显示环境变量
    pub async fn env(&self) -> Result<()> {
        let env = self.env_vars.lock().unwrap();
        for (key, value) in env.iter() {
            println!("{}={}", key, value);
        }
        Ok(())
    }

    /// 导出环境变量
    pub async fn export(&self, variable: String) -> Result<()> {
        if let Some((name, value)) = variable.split_once('=') {
            let mut env = self.env_vars.lock().unwrap();
            env.insert(name.to_string(), value.to_string());
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid export format. Use NAME=value"))
        }
    }

    /// 删除环境变量
    pub async fn unset(&self, name: String) -> Result<()> {
        let mut env = self.env_vars.lock().unwrap();
        env.remove(&name);
        Ok(())
    }

    /// 返回真（退出码 0）
    pub async fn true_cmd(&self) -> Result<()> {
        Ok(())
    }

    /// 返回假（退出码 1）
    pub async fn false_cmd(&self) -> Result<()> {
        Err(anyhow::anyhow!("false"))
    }

    /// 获取路径的基名（文件名部分）
    pub async fn basename(&self, path: String) -> Result<()> {
        let basename = std::path::Path::new(&path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        println!("{}", basename);
        Ok(())
    }

    /// 获取路径的目录名部分
    pub async fn dirname(&self, path: String) -> Result<()> {
        let dirname = std::path::Path::new(&path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or(".");
        println!("{}", dirname);
        Ok(())
    }

    /// 创建链接
    pub async fn ln(&self, target: String, link_name: String, symbolic: bool) -> Result<()> {
        if symbolic {
            // 符号链接通过 mount table 的 symlink 功能实现
            println!("Note: Symbolic links require backend support");
            println!("Target: {}", target);
            println!("Link name: {}", link_name);
        } else {
            // 硬链接（需要插件支持）
            println!("Note: Hard links require plugin support");
            println!("Target: {}", target);
            println!("Link name: {}", link_name);
        }
        Ok(())
    }

    /// 读取符号链接目标
    pub async fn readlink(&self, path: String) -> Result<()> {
        // 符号链接读取需要 backend 支持
        // 这里返回一个占位符消息
        println!("Readlink requires symlink support from backend");
        println!("Path: {}", path);
        Ok(())
    }

    /// 解析真实路径
    pub async fn realpath(&self, path: String) -> Result<()> {
        let resolved = if path.starts_with('/') {
            path
        } else {
            let cwd = self.cwd.lock().unwrap();
            format!("{}/{}", cwd.trim_end_matches('/'), path)
        };

        // 简单的路径规范化（移除 . 和 ..）
        let parts: Vec<&str> = resolved
            .split('/')
            .filter(|s| !s.is_empty() && *s != ".")
            .collect();
        let mut result = Vec::new();
        for part in parts {
            if part == ".." {
                result.pop();
            } else {
                result.push(part);
            }
        }

        let normalized = if result.is_empty() {
            "/".to_string()
        } else {
            "/".to_string() + &result.join("/")
        };

        println!("{}", normalized);
        Ok(())
    }

    /// 反转文件内容（字节级）
    pub async fn rev(&self, file: Option<String>) -> Result<()> {
        let content = if let Some(ref path) = file {
            // 判断是本地文件还是 EVIF 路径
            if path.starts_with('/') || path.contains(":/") {
                // EVIF 路径
                self.client.cat(path).await?
            } else {
                // 本地文件
                tokio::fs::read_to_string(path)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to read local file: {}", e))?
            }
        } else {
            return Err(anyhow::anyhow!(
                "Reading from stdin not yet supported. Please provide a file."
            ));
        };

        // 反转整个字符串
        let reversed: String = content.chars().rev().collect();
        println!("{}", reversed);
        Ok(())
    }

    /// 反转文件行顺序
    pub async fn tac(&self, file: Option<String>) -> Result<()> {
        let content = if let Some(ref path) = file {
            // 判断是本地文件还是 EVIF 路径
            if path.starts_with('/') && !std::path::Path::new(path).exists() {
                // EVIF 路径（假设不存在于本地文件系统）
                self.client.cat(path).await?
            } else {
                // 本地文件或已存在的路径
                tokio::fs::read_to_string(path)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to read local file: {}", e))?
            }
        } else {
            return Err(anyhow::anyhow!(
                "Reading from stdin not yet supported. Please provide a file."
            ));
        };

        // 按行分割，然后反转行数组
        let lines: Vec<&str> = content.lines().collect();
        for line in lines.into_iter().rev() {
            println!("{}", line);
        }
        Ok(())
    }

    /// 截断文件
    pub async fn truncate(&self, path: String, size: u64) -> Result<()> {
        // 截断需要 backend 支持
        // 这里通过写空内容实现简单的清零操作
        if size == 0 {
            self.client.write(&path, "", false).await?;
            println!("Truncated {} to 0 bytes", path);
        } else {
            println!("Truncate to specific size requires backend support");
            println!("Path: {}, Size: {}", path, size);
        }
        Ok(())
    }

    /// 分割文件
    pub async fn split(&self, file: Option<String>, lines: Option<usize>) -> Result<()> {
        let content = if let Some(ref path) = file {
            // 判断是本地文件还是 EVIF 路径
            if path.starts_with('/') && !std::path::Path::new(path).exists() {
                // EVIF 路径
                self.client.cat(path).await?
            } else {
                // 本地文件
                tokio::fs::read_to_string(path)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to read local file: {}", e))?
            }
        } else {
            return Err(anyhow::anyhow!(
                "Reading from stdin not yet supported. Please provide a file."
            ));
        };

        let lines_per_split = lines.unwrap_or(1000);
        let all_lines: Vec<&str> = content.lines().collect();

        for (i, chunk) in all_lines.chunks(lines_per_split).enumerate() {
            println!("=== Split {} ===", i + 1);
            for line in chunk {
                println!("{}", line);
            }
        }

        Ok(())
    }

    /// 查找文件（递归搜索目录）
    pub async fn find(&self, path: String, name: Option<&str>, type_: Option<&str>) -> Result<()> {
        let search_path = if path == "." {
            std::env::current_dir()?
        } else {
            std::path::PathBuf::from(&path)
        };

        self.find_recursive(
            &search_path,
            name.map(|s| s.to_string()),
            type_.map(|s| s.to_string()),
        )
        .await
    }

    /// 递归查找文件
    async fn find_recursive(
        &self,
        path: &std::path::Path,
        name: Option<String>,
        type_: Option<String>,
    ) -> Result<()> {
        let mut entries = tokio::fs::read_dir(path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read directory: {}", e))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read entry: {}", e))?
        {
            let entry_path = entry.path();
            let file_name_os = entry.file_name();
            let file_name = file_name_os.to_string_lossy();
            let is_dir = entry.path().is_dir();

            // 检查类型过滤
            if let Some(ref t) = type_ {
                match t.as_str() {
                    "f" | "file" => {
                        if is_dir {
                            continue;
                        }
                    }
                    "d" | "directory" => {
                        if !is_dir {
                            continue;
                        }
                    }
                    _ => {}
                }
            }

            // 检查名称匹配
            let name_match = if let Some(ref pattern) = name {
                if pattern.contains('*') || pattern.contains('?') {
                    // 简单的通配符匹配
                    let pattern_regex = pattern
                        .replace('.', "\\.")
                        .replace('*', ".*")
                        .replace('?', ".");
                    if let Ok(re) = regex::Regex::new(&pattern_regex) {
                        re.is_match(&file_name)
                    } else {
                        file_name == *pattern
                    }
                } else {
                    file_name.contains(pattern)
                }
            } else {
                true
            };

            if name_match {
                println!("{}", entry_path.display());
            }

            // 递归搜索子目录
            if is_dir {
                let name_clone = name.clone();
                let type_clone = type_.clone();
                Box::pin(self.find_recursive(&entry_path, name_clone, type_clone)).await?;
            }
        }

        Ok(())
    }

    /// 定位文件（简化版本，使用 find）
    pub async fn locate(&self, pattern: String) -> Result<()> {
        println!("Locate: searching for '{}'", pattern);
        println!("Note: Full locate requires database. Using find instead...");
        // 使用 find 作为替代实现
        self.find(".".to_string(), Some(&pattern), None).await
    }

    /// 查找命令路径
    pub async fn which(&self, command: String) -> Result<()> {
        if let Ok(path) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path) {
                let cmd_path = dir.join(&command);
                if cmd_path.exists() {
                    println!("{}", cmd_path.display());
                    return Ok(());
                }
            }
        }
        Err(anyhow::anyhow!("Command not found: {}", command))
    }

    /// 显示命令类型
    pub async fn type_cmd(&self, command: String) -> Result<()> {
        // 检查是否是别名、函数、内置命令或外部命令
        println!("{} is a command", command);
        Ok(())
    }

    /// 显示文件类型
    pub async fn file(&self, path: String) -> Result<()> {
        let path_obj = std::path::Path::new(&path);

        if !path_obj.exists() {
            println!("{}: cannot open", path);
            return Ok(());
        }

        let metadata = tokio::fs::metadata(&path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get metadata: {}", e))?;

        let file_type = if metadata.is_file() {
            let ext = path_obj.extension().and_then(|s| s.to_str()).unwrap_or("");

            match ext {
                "rs" => "Rust source",
                "go" => "Go source",
                "py" => "Python script",
                "js" => "JavaScript text",
                "txt" => "ASCII text",
                "json" => "JSON data",
                "md" => "Markdown text",
                _ => "data",
            }
        } else if metadata.is_dir() {
            "directory"
        } else if metadata.is_symlink() {
            "symbolic link"
        } else {
            "special file"
        };

        println!("{}: {}", path, file_type);
        Ok(())
    }

    // ============== 变量替换功能 ==============

    /// 展开字符串中的变量引用（$VAR 和 ${VAR} 语法）
    pub fn expand_variables(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();
        let env = self.env_vars.lock().unwrap();

        while let Some(c) = chars.next() {
            if c == '$' {
                // 检查是否是 ${VAR} 语法
                if chars.peek() == Some(&'{') {
                    chars.next(); // 消耗 '{'
                    let mut var_name = String::new();

                    // 读取变量名直到 '}'
                    while let Some(&inner_c) = chars.peek() {
                        if inner_c == '}' {
                            chars.next(); // 消耗 '}'
                            break;
                        }
                        var_name.push(chars.next().unwrap());
                    }

                    // 查找变量值
                    let value = self.get_variable_value(&var_name, &env);
                    result.push_str(&value);
                } else {
                    // $VAR 语法
                    let mut var_name = String::new();

                    // 读取变量名（字母、数字、下划线）
                    while let Some(&inner_c) = chars.peek() {
                        if inner_c.is_alphanumeric() || inner_c == '_' || inner_c == '?' {
                            var_name.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }

                    // 特殊变量
                    let value = match var_name.as_str() {
                        "?" => "0".to_string(),                // 上一个命令的退出码（简化实现）
                        "$" => std::process::id().to_string(), // 当前 PID
                        "0" => "evif".to_string(),             // shell 名称
                        _ => self.get_variable_value(&var_name, &env),
                    };

                    result.push_str(&value);
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// 获取变量值（优先从 shell 变量，然后从环境变量）
    fn get_variable_value(
        &self,
        name: &str,
        env: &std::collections::HashMap<String, String>,
    ) -> String {
        // 首先检查 shell 变量
        if let Some(value) = env.get(name) {
            return value.clone();
        }

        // 然后检查系统环境变量
        if let Ok(value) = std::env::var(name) {
            return value;
        }

        // 未找到变量，返回空字符串
        String::new()
    }

    /// 设置变量值
    pub fn set_variable(&self, name: String, value: String) {
        let mut env = self.env_vars.lock().unwrap();
        env.insert(name, value);
    }

    /// 获取变量值
    pub fn get_variable(&self, name: &str) -> Option<String> {
        let env = self.env_vars.lock().unwrap();
        env.get(name).cloned().or_else(|| std::env::var(name).ok())
    }

    /// 列出所有变量
    pub fn list_variables(&self) -> std::collections::HashMap<String, String> {
        let env = self.env_vars.lock().unwrap();
        env.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_creation() {
        let command = EvifCommand::new("localhost:50051".to_string(), false);
        assert_eq!(command.server, "localhost:50051");
        assert!(!command.verbose);
    }
}
