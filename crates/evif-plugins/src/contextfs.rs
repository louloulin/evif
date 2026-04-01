use async_trait::async_trait;
use evif_core::{EvifError, EvifPlugin, EvifResult, FileInfo, PluginConfigParam, WriteFlags};
use std::sync::Arc;
use tokio::sync::OnceCell;

use crate::MemFsPlugin;

/// Default threshold (bytes) above which L2 files get a companion `.summary`.
const DEFAULT_MAX_FILE_SIZE: usize = 4096;
/// Default number of recent operations tracked in `/L0/recent_ops`.
const DEFAULT_MAX_RECENT_OPS: usize = 10;
/// Number of lines to keep when generating a summary.
const SUMMARY_HEAD_LINES: usize = 8;
/// Default token budget limit (GPT-4 context window).
const DEFAULT_BUDGET_LIMIT: usize = 200_000;
/// Heuristic: 1 token is approximately 4 bytes.
const BYTES_PER_TOKEN: usize = 4;

// ---------------------------------------------------------------------------
// Token budget & budget status types
// ---------------------------------------------------------------------------

/// Token budget breakdown across context layers.
#[derive(Clone, Debug)]
pub struct ContextTokenBudget {
    pub l0_tokens: usize,
    pub l1_tokens: usize,
    pub l2_tokens: usize,
    pub total_tokens: usize,
    pub budget_limit: usize,
}

/// Budget severity level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BudgetLevel {
    /// Usage below 50% of the budget.
    Ok,
    /// Usage between 50% and 80% of the budget.
    Warning,
    /// Usage above 80% of the budget.
    Critical,
}

/// Current budget status with computed usage percentage and severity level.
#[derive(Clone, Debug)]
pub struct BudgetStatus {
    pub used_tokens: usize,
    pub budget_limit: usize,
    pub usage_percent: f64,
    pub level: BudgetLevel,
}

// ---------------------------------------------------------------------------
// Persistence backend trait (always defined, never feature-gated)
// ---------------------------------------------------------------------------

/// Backend for persisting ContextFS L0/L1 file data across sessions.
pub trait PersistenceBackend: Send + Sync {
    /// Persist the byte content of a file at the given path.
    fn save(&self, path: &str, data: &[u8]);

    /// Load the byte content of a file at the given path, if it exists.
    fn load(&self, path: &str) -> Option<Vec<u8>>;

    /// List all persisted paths.
    fn list_paths(&self) -> Vec<String>;
}

// ---------------------------------------------------------------------------
// SQLite-backed persistence (feature-gated)
// ---------------------------------------------------------------------------

#[cfg(feature = "sqlfs")]
pub struct SqlitePersistence {
    conn: std::sync::Mutex<rusqlite::Connection>,
}

#[cfg(feature = "sqlfs")]
impl SqlitePersistence {
    pub fn new(db_path: &str) -> Self {
        let conn = rusqlite::Connection::open(db_path)
            .expect("failed to open SQLite database for ContextFS persistence");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS contextfs_kv (
                path  TEXT PRIMARY KEY,
                data  BLOB NOT NULL
            );",
        )
        .expect("failed to create contextfs_kv table");
        Self {
            conn: std::sync::Mutex::new(conn),
        }
    }
}

#[cfg(feature = "sqlfs")]
impl PersistenceBackend for SqlitePersistence {
    fn save(&self, path: &str, data: &[u8]) {
        if let Ok(conn) = self.conn.lock() {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO contextfs_kv (path, data) VALUES (?1, ?2)",
                rusqlite::params![path, data],
            );
        }
    }

    fn load(&self, path: &str) -> Option<Vec<u8>> {
        let conn = self.conn.lock().ok()?;
        let mut stmt = conn
            .prepare("SELECT data FROM contextfs_kv WHERE path = ?1")
            .ok()?;
        let mut rows = stmt.query(rusqlite::params![path]).ok()?;
        let row = rows.next().ok()??;
        let data: Vec<u8> = row.get(0).ok()?;
        Some(data)
    }

    fn list_paths(&self) -> Vec<String> {
        let Ok(conn) = self.conn.lock() else {
            return vec![];
        };
        let mut stmt = match conn.prepare("SELECT path FROM contextfs_kv") {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let rows = match stmt.query_map([], |row| row.get::<_, String>(0)) {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        rows.filter_map(|r| r.ok()).collect()
    }
}

// ---------------------------------------------------------------------------
// ContextFsPlugin
// ---------------------------------------------------------------------------

pub struct ContextFsPlugin {
    inner: MemFsPlugin,
    initialized: OnceCell<()>,
    /// L2 files larger than this get a companion `.summary`.
    max_file_size: usize,
    /// Maximum number of entries retained in `/L0/recent_ops`.
    max_recent_ops: usize,
    /// Optional persistence backend for L0/L1 files.
    persistence: Option<Arc<dyn PersistenceBackend>>,
    /// Maximum number of tokens allowed across all layers.
    budget_limit: usize,
}

impl ContextFsPlugin {
    pub fn new() -> Self {
        Self {
            inner: MemFsPlugin::new(),
            initialized: OnceCell::const_new(),
            max_file_size: DEFAULT_MAX_FILE_SIZE,
            max_recent_ops: DEFAULT_MAX_RECENT_OPS,
            persistence: None,
            budget_limit: DEFAULT_BUDGET_LIMIT,
        }
    }

    /// Build with a custom max file size for auto-compression.
    pub fn with_max_file_size(mut self, size: usize) -> Self {
        self.max_file_size = size;
        self
    }

    /// Build with a custom max recent-ops count.
    pub fn with_max_recent_ops(mut self, count: usize) -> Self {
        self.max_recent_ops = count;
        self
    }

    /// Build with a custom token budget limit (default: 200 000 tokens).
    pub fn with_budget_limit(mut self, limit: usize) -> Self {
        self.budget_limit = limit;
        self
    }

    /// Create a ContextFsPlugin with SQLite-backed persistence for L0/L1 files.
    ///
    /// The database at `db_path` is created if it does not exist. On
    /// initialization, L0/L1 files previously persisted to the database are
    /// restored into the in-memory filesystem (seed content is used only when
    /// the database has no persisted data).
    #[cfg(feature = "sqlfs")]
    pub fn new_with_persistence(db_path: &str) -> Self {
        let backend = Arc::new(SqlitePersistence::new(db_path));
        Self {
            inner: MemFsPlugin::new(),
            initialized: OnceCell::const_new(),
            max_file_size: DEFAULT_MAX_FILE_SIZE,
            max_recent_ops: DEFAULT_MAX_RECENT_OPS,
            persistence: Some(backend),
            budget_limit: DEFAULT_BUDGET_LIMIT,
        }
    }

    fn readme_text(&self) -> String {
        r#"# ContextFS

EVIF 的分层上下文文件系统。

## 布局

- `L0/` 即时上下文，保存当前工作状态
- `L1/` 会话上下文，保存决策和临时草稿
- `L2/` 项目知识库，保存架构和模式沉淀
- `.meta` 保存当前最小实现的上下文策略说明

## 当前实现

- 预置 `L0/L1/L2` 目录和关键种子文件
- 支持通过标准文件操作读写上下文内容
- 允许在 `active_files/`、`scratch/`、`intermediate/`、`history/` 下继续扩展
- 自动压缩：L2 文件超过阈值时生成 `.summary` 伴生文件
- 操作追踪：L0/L1 写入操作记录到 `/L0/recent_ops`
"#
        .to_string()
    }

    fn meta_text(&self) -> String {
        let persistence_val = if self.persistence.is_some() {
            "sqlite"
        } else {
            "in-memory"
        };
        serde_json::json!({
            "version": 2,
            "compression": "auto-summary",
            "compression_threshold_bytes": self.max_file_size,
            "recent_ops_max": self.max_recent_ops,
            "persistence": persistence_val,
            "capabilities": [
                "auto-compression",
                "recent-ops-tracking",
                "summary-companion-files"
            ],
            "notes": [
                "L2 files exceeding the size threshold get a companion .summary file.",
                "Write operations to L0 and L1 are tracked in /L0/recent_ops as a JSON array.",
                "Summary files contain the first N lines of the original plus a truncation marker."
            ]
        })
        .to_string()
    }

    async fn ensure_initialized(&self) -> EvifResult<()> {
        self.initialized
            .get_or_try_init(|| async {
                for dir in [
                    "/L0",
                    "/L0/active_files",
                    "/L1",
                    "/L1/intermediate",
                    "/L1/scratch",
                    "/L2",
                    "/L2/history",
                    "/L2/embeddings",
                ] {
                    self.inner.mkdir(dir, 0o755).await?;
                }

                self.seed_file(
                    "/README",
                    self.readme_text(),
                )
                .await?;
                self.seed_file("/.meta", self.meta_text()).await?;
                self.seed_file(
                    "/L0/current",
                    "当前即时上下文\n- status: idle\n- focus: 等待 Agent 读取或写入当前任务\n".to_string(),
                )
                .await?;
                self.seed_file("/L0/recent_ops", "[]\n".to_string()).await?;
                self.seed_file(
                    "/L1/session_id",
                    "session: bootstrap\n".to_string(),
                )
                .await?;
                self.seed_file(
                    "/L1/decisions.md",
                    "# Session Decisions\n\n- 尚未记录会话决策\n".to_string(),
                )
                .await?;
                self.seed_file(
                    "/L2/architecture.md",
                    "# 项目架构\n\n当前最小实现将 ContextFS 暴露为 EVIF 内建插件。\n".to_string(),
                )
                .await?;
                self.seed_file(
                    "/L2/patterns.md",
                    "# Patterns\n\n- 分层上下文: L0 -> L1 -> L2\n- 文件优先的 Agent 接口\n".to_string(),
                )
                .await?;
                self.seed_file(
                    "/L2/best_practices.md",
                    "# Best Practices\n\n- 优先读取 L0，再按需深入 L1/L2\n- 将稳定决策沉淀到 L1/L2 文件\n".to_string(),
                )
                .await?;

                // Restore L0/L1 files from persistence if they exist.
                if let Some(ref backend) = self.persistence {
                    for path in backend.list_paths() {
                        if let Some(data) = backend.load(&path) {
                            // Ensure parent directories exist (they were
                            // created above, so this is a safety net for
                            // paths like /L0/active_files/foo.md).
                            if let Some(parent) = parent_path(&path) {
                                let _ = self.inner.mkdir(&parent, 0o755).await;
                            }
                            if self.inner.stat(&path).await.is_err() {
                                let _ = self.inner.create(&path, 0o644).await;
                            }
                            let _ = self
                                .inner
                                .write(&path, data, 0, WriteFlags::TRUNCATE)
                                .await;
                        }
                    }
                }

                Ok::<(), EvifError>(())
            })
            .await?;

        Ok(())
    }

    async fn seed_file(&self, path: &str, content: String) -> EvifResult<()> {
        self.inner.create(path, 0o644).await?;
        self.inner
            .write(path, content.into_bytes(), 0, WriteFlags::TRUNCATE)
            .await?;
        Ok(())
    }

    /// Determine whether a path is under L0 or L1.
    fn is_l0_or_l1(path: &str) -> bool {
        let p = path.trim_start_matches('/');
        p.starts_with("L0/") || p == "L0" || p.starts_with("L1/") || p == "L1"
    }

    /// Determine whether a path is under L2.
    fn is_l2(path: &str) -> bool {
        let p = path.trim_start_matches('/');
        p.starts_with("L2/") || p == "L2"
    }

    /// Generate a truncated summary from file content.
    ///
    /// Keeps the first `SUMMARY_HEAD_LINES` lines and appends a truncation
    /// marker with the original byte size.
    fn generate_summary(content: &[u8], original_size: usize) -> String {
        let text = String::from_utf8_lossy(content);
        let lines: Vec<&str> = text.lines().take(SUMMARY_HEAD_LINES).collect();
        let mut summary = lines.join("\n");
        summary.push_str(&format!(
            "\n\n... [truncated: showing {} of {} bytes] ...\n",
            content.len(),
            original_size,
        ));
        summary
    }

    /// Append an operation record to `/L0/recent_ops`, keeping only the last
    /// `max_recent_ops` entries.
    async fn track_operation(&self, op: &str, path: &str) -> EvifResult<()> {
        // Read the current JSON array.
        let raw = self.inner.read("/L0/recent_ops", 0, 0).await?;
        let raw_str = String::from_utf8_lossy(&raw);

        let mut arr: Vec<serde_json::Value> = match serde_json::from_str::<serde_json::Value>(
            raw_str.trim(),
        ) {
            Ok(serde_json::Value::Array(a)) => a,
            _ => vec![],
        };

        let entry = serde_json::json!({
            "op": op,
            "path": path,
        });
        arr.push(entry);

        // Keep only the last `max_recent_ops` entries.
        if arr.len() > self.max_recent_ops {
            let drain_count = arr.len() - self.max_recent_ops;
            arr.drain(0..drain_count);
        }

        let new_content = serde_json::to_string_pretty(&arr).unwrap_or_else(|_| "[]".to_string());
        self.inner
            .write(
                "/L0/recent_ops",
                format!("{}\n", new_content).into_bytes(),
                0,
                WriteFlags::TRUNCATE,
            )
            .await?;

        Ok(())
    }

    // -------------------------------------------------------------------
    // Token estimation
    // -------------------------------------------------------------------

    /// Estimate token counts for each context layer.
    ///
    /// Uses the heuristic of 1 token per 4 bytes. Recursively walks L0, L1,
    /// and L2 directories and sums the byte sizes of all regular files found.
    pub async fn estimate_tokens(&self) -> EvifResult<ContextTokenBudget> {
        self.ensure_initialized().await?;

        let l0_tokens = self.sum_layer_bytes("/L0").await? / BYTES_PER_TOKEN;
        let l1_tokens = self.sum_layer_bytes("/L1").await? / BYTES_PER_TOKEN;
        let l2_tokens = self.sum_layer_bytes("/L2").await? / BYTES_PER_TOKEN;

        let total_tokens = l0_tokens + l1_tokens + l2_tokens;

        Ok(ContextTokenBudget {
            l0_tokens,
            l1_tokens,
            l2_tokens,
            total_tokens,
            budget_limit: self.budget_limit,
        })
    }

    /// Recursively sum the byte sizes of all regular files under `dir`.
    async fn sum_layer_bytes(&self, dir: &str) -> EvifResult<usize> {
        let entries = match self.inner.readdir(dir).await {
            Ok(e) => e,
            Err(_) => return Ok(0),
        };

        let mut total: usize = 0;
        for entry in entries {
            if entry.name == "." || entry.name == ".." {
                continue;
            }
            let child_path = if dir == "/" {
                format!("/{}", entry.name)
            } else {
                format!("{}/{}", dir, entry.name)
            };

            if entry.is_dir {
                total += Box::pin(self.sum_layer_bytes(&child_path)).await?;
            } else {
                if let Ok(data) = self.inner.read(&child_path, 0, 0).await {
                    total += data.len();
                }
            }
        }

        Ok(total)
    }

    // -------------------------------------------------------------------
    // Budget checking
    // -------------------------------------------------------------------

    /// Check the current budget status.
    ///
    /// Internally calls `estimate_tokens()`, computes the usage percentage,
    /// determines the severity level, and writes the status as JSON to
    /// `/L0/budget_status`.
    pub async fn check_budget(&self) -> EvifResult<BudgetStatus> {
        let budget = self.estimate_tokens().await?;

        let usage_percent = if budget.budget_limit > 0 {
            (budget.total_tokens as f64 / budget.budget_limit as f64) * 100.0
        } else {
            0.0
        };

        let level = if usage_percent > 80.0 {
            BudgetLevel::Critical
        } else if usage_percent > 50.0 {
            BudgetLevel::Warning
        } else {
            BudgetLevel::Ok
        };

        let status = BudgetStatus {
            used_tokens: budget.total_tokens,
            budget_limit: budget.budget_limit,
            usage_percent,
            level: level.clone(),
        };

        // Best-effort: write budget status to /L0/budget_status as JSON.
        let status_json = serde_json::json!({
            "used_tokens": status.used_tokens,
            "budget_limit": status.budget_limit,
            "usage_percent": status.usage_percent,
            "level": match status.level {
                BudgetLevel::Ok => "ok",
                BudgetLevel::Warning => "warning",
                BudgetLevel::Critical => "critical",
            },
        });
        let status_str = format!("{}\n", serde_json::to_string_pretty(&status_json)
            .unwrap_or_else(|_| "{}".to_string()));

        // Ensure the file exists, then write.
        if self.inner.stat("/L0/budget_status").await.is_err() {
            let _ = self.inner.create("/L0/budget_status", 0o644).await;
        }
        let _ = self.inner
            .write(
                "/L0/budget_status",
                status_str.into_bytes(),
                0,
                WriteFlags::TRUNCATE,
            )
            .await;

        Ok(status)
    }

    /// Best-effort budget check after a write. Does not propagate errors.
    async fn maybe_update_budget_status(&self) {
        let _ = self.check_budget().await;
    }

    // -------------------------------------------------------------------
    // Session lifecycle
    // -------------------------------------------------------------------

    /// Start a new session.
    ///
    /// - Updates `/L1/session_id` to the provided session identifier.
    /// - Resets `/L0/recent_ops` to an empty array.
    /// - Writes a session start timestamp to `/L0/current`.
    pub async fn start_session(&self, session_id: &str) -> EvifResult<()> {
        self.ensure_initialized().await?;

        // Update session id in L1.
        let session_content = format!("session: {}\n", session_id);
        self.inner
            .write(
                "/L1/session_id",
                session_content.into_bytes(),
                0,
                WriteFlags::TRUNCATE,
            )
            .await?;

        // Reset recent_ops.
        self.inner
            .write(
                "/L0/recent_ops",
                b"[]\n".to_vec(),
                0,
                WriteFlags::TRUNCATE,
            )
            .await?;

        // Write session start timestamp to L0/current.
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let current_content = format!(
            "当前即时上下文\n- status: active\n- session: {}\n- started_at: {}\n",
            session_id, timestamp,
        );
        self.inner
            .write(
                "/L0/current",
                current_content.into_bytes(),
                0,
                WriteFlags::TRUNCATE,
            )
            .await?;

        Ok(())
    }

    /// End the current session.
    ///
    /// Archives key L1 files to `/L2/history` with a timestamp suffix.
    /// For example, `/L1/decisions.md` becomes
    /// `/L2/history/decisions_20260401_120000.md`. Then clears L1 scratch and
    /// intermediate directories and resets L0.
    pub async fn end_session(&self) -> EvifResult<()> {
        self.ensure_initialized().await?;

        let now = chrono::Local::now();
        let ts_suffix = now.format("%Y%m%d_%H%M%S").to_string();

        // Archive L1 files that should be preserved.
        let l1_files_to_archive = ["/L1/decisions.md", "/L1/session_id"];
        for l1_path in &l1_files_to_archive {
            if let Ok(data) = self.inner.read(l1_path, 0, 0).await {
                // Derive archive name: strip "/L1/" prefix and add timestamp.
                let file_name = l1_path.trim_start_matches("/L1/");
                let stem = file_name.trim_end_matches(".md");
                let archive_name = format!("{}_{}.md", stem, ts_suffix);
                let archive_path = format!("/L2/history/{}", archive_name);

                // Create the archive file.
                if self.inner.stat(&archive_path).await.is_err() {
                    let _ = self.inner.create(&archive_path, 0o644).await;
                }
                let _ = self.inner
                    .write(&archive_path, data, 0, WriteFlags::TRUNCATE)
                    .await;
            }
        }

        // Clear L1 scratch and intermediate directories.
        let _ = self.inner.remove_all("/L1/scratch").await;
        let _ = self.inner.mkdir("/L1/scratch", 0o755).await;
        let _ = self.inner.remove_all("/L1/intermediate").await;
        let _ = self.inner.mkdir("/L1/intermediate", 0o755).await;

        // Reset L0.
        let reset_current = "当前即时上下文\n- status: idle\n- focus: 等待 Agent 读取或写入当前任务\n";
        self.inner
            .write(
                "/L0/current",
                reset_current.as_bytes().to_vec(),
                0,
                WriteFlags::TRUNCATE,
            )
            .await?;
        self.inner
            .write("/L0/recent_ops", b"[]\n".to_vec(), 0, WriteFlags::TRUNCATE)
            .await?;

        Ok(())
    }
}

/// Extract the parent directory path (e.g. "/L0/active_files" from
/// "/L0/active_files/note.md"). Returns `None` for top-level entries.
fn parent_path(path: &str) -> Option<String> {
    let p = path.trim_end_matches('/');
    let idx = p.rfind('/')?;
    if idx == 0 {
        // Parent is root "/".
        Some("/".to_string())
    } else {
        Some(p[..idx].to_string())
    }
}

impl Default for ContextFsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EvifPlugin for ContextFsPlugin {
    fn name(&self) -> &str {
        "contextfs"
    }

    fn get_readme(&self) -> String {
        self.readme_text()
    }

    fn get_config_params(&self) -> Vec<PluginConfigParam> {
        let mut params = vec![
            PluginConfigParam {
                name: "max_file_size".to_string(),
                param_type: "int".to_string(),
                required: false,
                default: Some(DEFAULT_MAX_FILE_SIZE.to_string()),
                description: Some(
                    "L2 files exceeding this size (bytes) get a companion .summary".to_string(),
                ),
            },
            PluginConfigParam {
                name: "max_recent_ops".to_string(),
                param_type: "int".to_string(),
                required: false,
                default: Some(DEFAULT_MAX_RECENT_OPS.to_string()),
                description: Some(
                    "Maximum number of recent operations tracked in /L0/recent_ops".to_string(),
                ),
            },
        ];

        if self.persistence.is_some() {
            params.push(PluginConfigParam {
                name: "persistence_path".to_string(),
                param_type: "string".to_string(),
                required: false,
                default: None,
                description: Some(
                    "Path to the SQLite database file for cross-session persistence".to_string(),
                ),
            });
        }

        params
    }

    async fn create(&self, path: &str, perm: u32) -> EvifResult<()> {
        self.ensure_initialized().await?;
        self.inner.create(path, perm).await
    }

    async fn mkdir(&self, path: &str, perm: u32) -> EvifResult<()> {
        self.ensure_initialized().await?;
        self.inner.mkdir(path, perm).await
    }

    async fn read(&self, path: &str, offset: u64, size: u64) -> EvifResult<Vec<u8>> {
        self.ensure_initialized().await?;

        // Support reading `.summary` companion files. If the caller asks for a
        // path ending in `.summary` that does not exist, look for the base file
        // and generate the summary on the fly.
        if path.ends_with(".summary") && self.inner.stat(path).await.is_err() {
            let base_path = &path[..path.len() - ".summary".len()];
            if let Ok(base_data) = self.inner.read(base_path, 0, 0).await {
                let summary = Self::generate_summary(&base_data, base_data.len());
                return Ok(summary.into_bytes());
            }
        }

        self.inner.read(path, offset, size).await
    }

    async fn write(
        &self,
        path: &str,
        data: Vec<u8>,
        offset: i64,
        flags: WriteFlags,
    ) -> EvifResult<u64> {
        self.ensure_initialized().await?;

        // Perform the actual write first.
        let written = self.inner.write(path, data.clone(), offset, flags).await?;

        // Track the operation for L0/L1 writes.
        if Self::is_l0_or_l1(path) {
            // Best-effort: don't fail the write if tracking fails.
            let _ = self.track_operation("write", path).await;

            // Best-effort: persist L0/L1 data to the backend.
            if let Some(ref backend) = self.persistence {
                backend.save(path, &data);
            }

            // Best-effort: update budget status after L0/L1 writes.
            self.maybe_update_budget_status().await;
        }

        // Auto-compression for L2 files that exceed the threshold.
        if Self::is_l2(path) && !path.ends_with(".summary") && data.len() > self.max_file_size {
            let summary_content = Self::generate_summary(&data, data.len());
            let summary_path = format!("{}.summary", path);

            // Create the summary companion file if it does not exist yet.
            if self.inner.stat(&summary_path).await.is_err() {
                // Ignore creation error (might already exist due to a race).
                let _ = self.inner.create(&summary_path, 0o644).await;
            }
            // Best-effort write of the summary.
            let _ = self
                .inner
                .write(
                    &summary_path,
                    summary_content.into_bytes(),
                    0,
                    WriteFlags::TRUNCATE,
                )
                .await;
        }

        Ok(written)
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        self.ensure_initialized().await?;
        self.inner.readdir(path).await
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        self.ensure_initialized().await?;
        self.inner.stat(path).await
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        self.ensure_initialized().await?;
        self.inner.remove(path).await
    }

    async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()> {
        self.ensure_initialized().await?;
        self.inner.rename(old_path, new_path).await
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        self.ensure_initialized().await?;
        self.inner.remove_all(path).await
    }
}
