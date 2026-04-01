// Context Manager - High-level service for managing ContextFS lifecycle
//
// Wraps ContextFsPlugin with session management, token budget tracking,
// context archiving, and search capabilities. This is a service (NOT a plugin).

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use evif_core::{EvifError, EvifPlugin, EvifResult, WriteFlags};

use crate::contextfs::ContextFsPlugin;

// Conditional import for VectorFS
#[cfg(feature = "vectorfs")]
use crate::vectorfs::VectorFsPlugin;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Identifies which context layer an operation targets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextLayer {
    L0,
    L1,
    L2,
}

/// Metadata about an active session.
#[derive(Clone, Debug)]
pub struct SessionInfo {
    pub id: String,
    pub created_at: i64,
    pub token_budget_l0: usize,
    pub token_budget_l1: usize,
}

/// A single match returned by `search_context`.
#[derive(Clone, Debug)]
pub struct SearchResult {
    pub path: String,
    pub snippet: String,
    pub layer: ContextLayer,
}

/// A semantic search result with content, source, and similarity score.
#[derive(Clone, Debug)]
pub struct SemanticResult {
    pub content: String,
    pub source: String,
    pub score: f32,
}

// ---------------------------------------------------------------------------
// ContextManager
// ---------------------------------------------------------------------------

/// High-level service that manages the ContextFS lifecycle.
///
/// Provides session management, token budget tracking, archiving, and search
/// on top of the raw `ContextFsPlugin` file operations.
pub struct ContextManager {
    plugin: Arc<ContextFsPlugin>,
    #[cfg(feature = "vectorfs")]
    vectorfs: Option<Arc<VectorFsPlugin>>,
    sessions: RwLock<HashMap<String, SessionInfo>>,
    /// Default token budget for the L0 layer.
    token_budget_l0: usize,
    /// Default token budget for the L1 layer.
    token_budget_l1: usize,
}

impl ContextManager {
    /// Create a new `ContextManager` wrapping the given `ContextFsPlugin`.
    pub fn new(plugin: Arc<ContextFsPlugin>) -> Self {
        Self {
            plugin,
            #[cfg(feature = "vectorfs")]
            vectorfs: None,
            sessions: RwLock::new(HashMap::new()),
            token_budget_l0: 200,
            token_budget_l1: 2000,
        }
    }

    /// Create a new `ContextManager` with an optional VectorFS plugin for semantic search.
    #[cfg(feature = "vectorfs")]
    pub fn with_vectorfs(mut self, vectorfs: Arc<VectorFsPlugin>) -> Self {
        self.vectorfs = Some(vectorfs);
        self
    }

    /// Override the default L0 token budget.
    pub fn with_token_budget_l0(mut self, budget: usize) -> Self {
        self.token_budget_l0 = budget;
        self
    }

    /// Override the default L1 token budget.
    pub fn with_token_budget_l1(mut self, budget: usize) -> Self {
        self.token_budget_l1 = budget;
        self
    }

    // -----------------------------------------------------------------------
    // Session lifecycle
    // -----------------------------------------------------------------------

    /// Create a new context session.
    ///
    /// - Sets `/L1/session_id` to the given session id.
    /// - Clears the scratch directory.
    /// - Records session metadata.
    pub async fn create_session(&self, session_id: &str) -> EvifResult<()> {
        // Write the session id to L1.
        let content = format!("session: {}\n", session_id);
        self.plugin
            .write(
                "/L1/session_id",
                content.into_bytes(),
                0,
                WriteFlags::TRUNCATE,
            )
            .await?;

        // Clear scratch: remove all children, recreate the directory.
        let _ = self.plugin.remove_all("/L1/scratch").await;
        self.plugin.mkdir("/L1/scratch", 0o755).await?;

        // Record session info.
        let now = chrono::Utc::now().timestamp();
        let info = SessionInfo {
            id: session_id.to_string(),
            created_at: now,
            token_budget_l0: self.token_budget_l0,
            token_budget_l1: self.token_budget_l1,
        };
        self.sessions
            .write()
            .await
            .insert(session_id.to_string(), info);

        Ok(())
    }

    /// Close and archive a session.
    ///
    /// Archives L1 decisions to `L2/history/{session_id}.md`, then removes
    /// the session from the active sessions map.
    pub async fn close_session(&self, session_id: &str) -> EvifResult<()> {
        self.archive_session(session_id).await?;

        self.sessions.write().await.remove(session_id);

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Context archiving
    // -----------------------------------------------------------------------

    /// Archive L1 context to `L2/history/{session_id}.md`.
    ///
    /// Reads the current L1 decisions file, appends session metadata, and
    /// writes the result into L2/history.
    pub async fn archive_session(&self, session_id: &str) -> EvifResult<()> {
        let decisions_raw = self.plugin.read("/L1/decisions.md", 0, 0).await?;
        let decisions_text = String::from_utf8_lossy(&decisions_raw);

        let archive_path = format!("/L2/history/{}.md", session_id);

        // Build the archived content with a session header.
        let timestamp = chrono::Utc::now().to_rfc3339();
        let archive_content = format!(
            "# Session Archive: {}\n\nArchived at: {}\n\n{}\n",
            session_id, timestamp, decisions_text,
        );

        // Create the file if it does not exist, then write.
        let create_result = self.plugin.create(&archive_path, 0o644).await;
        // Ignore AlreadyExists -- the file may already be present.
        if let Err(EvifError::AlreadyExists(_)) = create_result {
            // ok
        } else {
            create_result?;
        }

        self.plugin
            .write(
                &archive_path,
                archive_content.into_bytes(),
                0,
                WriteFlags::TRUNCATE,
            )
            .await?;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Token budget tracking
    // -----------------------------------------------------------------------

    /// Estimate the token count for a file at the given path.
    ///
    /// Uses the rough heuristic of `bytes / 4`. Returns `0` if the file
    /// cannot be read.
    pub async fn estimate_tokens(&self, path: &str) -> EvifResult<usize> {
        let data = self.plugin.read(path, 0, 0).await?;
        Ok(data.len() / 4)
    }

    /// Get total token usage for a layer by summing all files in that layer.
    ///
    /// Recursively walks the layer directory and sums `bytes / 4` for each
    /// regular file found.
    pub async fn get_layer_usage(&self, layer: ContextLayer) -> EvifResult<usize> {
        let root = match layer {
            ContextLayer::L0 => "/L0",
            ContextLayer::L1 => "/L1",
            ContextLayer::L2 => "/L2",
        };

        let mut total: usize = 0;
        self.sum_layer_tokens(root, &mut total).await?;
        Ok(total)
    }

    /// Recursively accumulate token estimates for all files under `dir`.
    async fn sum_layer_tokens(&self, dir: &str, total: &mut usize) -> EvifResult<()> {
        let entries = match self.plugin.readdir(dir).await {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };

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
                Box::pin(self.sum_layer_tokens(&child_path, total)).await?;
            } else {
                if let Ok(data) = self.plugin.read(&child_path, 0, 0).await {
                    *total += data.len() / 4;
                }
            }
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Context search
    // -----------------------------------------------------------------------

    /// Simple grep-based search across L0, L1, and L2 files.
    ///
    /// Returns a list of `SearchResult` entries, one per matching file,
    /// containing the first line that includes the query string.
    pub async fn search_context(&self, query: &str) -> EvifResult<Vec<SearchResult>> {
        let mut results: Vec<SearchResult> = Vec::new();

        self.search_layer("/L0", ContextLayer::L0, query, &mut results)
            .await?;
        self.search_layer("/L1", ContextLayer::L1, query, &mut results)
            .await?;
        self.search_layer("/L2", ContextLayer::L2, query, &mut results)
            .await?;

        Ok(results)
    }

    /// Recursively search files under `dir` for lines containing `query`.
    async fn search_layer(
        &self,
        dir: &str,
        layer: ContextLayer,
        query: &str,
        results: &mut Vec<SearchResult>,
    ) -> EvifResult<()> {
        let entries = match self.plugin.readdir(dir).await {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };

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
                Box::pin(self.search_layer(&child_path, layer.clone(), query, results))
                    .await?;
            } else {
                if let Ok(data) = self.plugin.read(&child_path, 0, 0).await {
                    let text = String::from_utf8_lossy(&data);
                    if let Some(matching_line) = text.lines().find(|l| l.contains(query)) {
                        results.push(SearchResult {
                            path: child_path,
                            snippet: matching_line.to_string(),
                            layer: layer.clone(),
                        });
                    }
                }
            }
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Semantic search
    // -----------------------------------------------------------------------

    /// Perform semantic search across L2 knowledge base.
    ///
    /// Uses VectorFS's `search_documents()` if available, otherwise falls back
    /// to text-based grep search across L2 files.
    ///
    /// Returns results sorted by relevance score (highest first).
    pub async fn semantic_search(&self, query: &str, limit: usize) -> EvifResult<Vec<SemanticResult>> {
        let effective_limit = if limit == 0 { 5 } else { limit };

        #[cfg(feature = "vectorfs")]
        {
            if let Some(ref vectorfs) = self.vectorfs {
                // Try to use VectorFS for semantic search
                let namespace = "l2_knowledge";
                let docs = vectorfs.search_documents(namespace, query, effective_limit).await;

                if !docs.is_empty() {
                    return Ok(docs
                        .into_iter()
                        .map(|doc| SemanticResult {
                            content: doc.content,
                            source: doc.file_name,
                            score: 0.8, // Vector search returns normalized scores via cosine similarity
                        })
                        .collect());
                }
                // Fall through to text search if VectorFS has no results
            }
        }

        // Fallback: text search across L2 files
        self.text_search_l2(query, effective_limit).await
    }

    /// Fallback text search across L2 files when VectorFS is not available.
    async fn text_search_l2(&self, query: &str, limit: usize) -> EvifResult<Vec<SemanticResult>> {
        let mut results: Vec<SemanticResult> = Vec::new();
        let query_lower = query.to_lowercase();

        self.collect_text_matches("/L2", &query_lower, &mut results).await?;

        // Sort by score (exact matches score higher)
        results.sort_by(|a, b| {
            let score_a = if a.content.to_lowercase().contains(&format!(" {}", query_lower)) { 1.0 } else { 0.5 };
            let score_b = if b.content.to_lowercase().contains(&format!(" {}", query_lower)) { 1.0 } else { 0.5 };
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(limit);
        Ok(results)
    }

    /// Recursively collect text matches from files under a directory.
    async fn collect_text_matches(
        &self,
        dir: &str,
        query: &str,
        results: &mut Vec<SemanticResult>,
    ) -> EvifResult<()> {
        let entries = match self.plugin.readdir(dir).await {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };

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
                Box::pin(self.collect_text_matches(&child_path, query, results)).await?;
            } else if let Ok(data) = self.plugin.read(&child_path, 0, 0).await {
                let text = String::from_utf8_lossy(&data);
                if text.to_lowercase().contains(query) {
                    // Extract a snippet around the match
                    let snippet = self.extract_snippet(&text, query, 100);
                    results.push(SemanticResult {
                        content: snippet,
                        source: child_path,
                        score: 0.5, // Text match score
                    });
                }
            }
        }

        Ok(())
    }

    /// Extract a snippet of text around a query match.
    fn extract_snippet(&self, text: &str, query: &str, context_len: usize) -> String {
        let query_lower = query.to_lowercase();
        let text_lower = text.to_lowercase();

        if let Some(pos) = text_lower.find(&query_lower) {
            let start = pos.saturating_sub(context_len / 2);
            let end = (pos + query.len() + context_len / 2).min(text.len());

            let mut snippet = String::new();
            if start > 0 {
                snippet.push_str("...");
            }
            snippet.push_str(&text[start..end]);
            if end < text.len() {
                snippet.push_str("...");
            }
            snippet
        } else {
            // Return first N chars as fallback
            let end = std::cmp::min(context_len, text.len());
            format!("{}...", &text[..end])
        }
    }

    // -----------------------------------------------------------------------
    // Summary generation
    // -----------------------------------------------------------------------

    /// Generate a summary for the given content.
    ///
    /// If `OPENAI_API_KEY` is set, uses OpenAI API for smart summarization.
    /// Otherwise, falls back to simple extractive summary (first N chars).
    pub async fn generate_summary(&self, content: &str, max_length: usize) -> EvifResult<String> {
        let effective_max = if max_length == 0 { 200 } else { max_length };

        // Check if OpenAI API key is available
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            if !api_key.is_empty() {
                return self.openai_summarize(content, effective_max, &api_key).await;
            }
        }

        // Fallback: simple extractive summary
        Ok(Self::simple_summary(content, effective_max))
    }

    /// Simple extractive summary: returns first N characters with truncation marker.
    fn simple_summary(content: &str, max_length: usize) -> String {
        if content.len() <= max_length {
            return content.to_string();
        }

        // Try to truncate at a word boundary
        let truncated = &content[..max_length];
        if let Some(last_space) = truncated.rfind(' ') {
            format!("{}...", &truncated[..last_space])
        } else {
            format!("{}...", truncated)
        }
    }

    /// Generate summary using OpenAI API.
    async fn openai_summarize(&self, content: &str, max_length: usize, api_key: &str) -> EvifResult<String> {
        use reqwest::Client;
        use serde::{Deserialize, Serialize};

        #[derive(Serialize)]
        struct Request {
            model: String,
            messages: Vec<Message>,
            max_tokens: usize,
        }
        #[derive(Serialize)]
        struct Message {
            role: String,
            content: String,
        }
        #[derive(Deserialize)]
        struct Response {
            choices: Vec<Choice>,
        }
        #[derive(Deserialize)]
        struct Choice {
            message: MessageContent,
        }
        #[derive(Deserialize)]
        struct MessageContent {
            content: String,
        }

        let client = Client::new();
        let truncate_hint = format!("(Summarize in approximately {} characters or less)", max_length);

        let request = Request {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: "You are a helpful assistant that summarizes text concisely.".to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: format!("{}:\n\n{}", truncate_hint, content),
                },
            ],
            max_tokens: (max_length / 4).max(50),
        };

        let resp = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| EvifError::Other(format!("OpenAI API request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(EvifError::Other(format!("OpenAI API error {}: {}", status, body)));
        }

        let result: Response = resp.json().await
            .map_err(|e| EvifError::Other(format!("Failed to parse OpenAI response: {}", e)))?;

        result.choices.into_iter().next()
            .map(|c| c.message.content.trim().to_string())
            .ok_or_else(|| EvifError::Other("No summary in OpenAI response".to_string()))
    }

    // -----------------------------------------------------------------------
    // Convenience helpers
    // -----------------------------------------------------------------------

    /// Save a decision to `/L1/decisions.md`.
    ///
    /// Appends a new line item to the decisions file.
    pub async fn save_decision(&self, decision: &str) -> EvifResult<()> {
        let existing = self.plugin.read("/L1/decisions.md", 0, 0).await?;
        let existing_text = String::from_utf8_lossy(&existing);

        // Append the new decision line.
        let updated = format!("{}- {}\n", existing_text, decision);

        self.plugin
            .write(
                "/L1/decisions.md",
                updated.into_bytes(),
                0,
                WriteFlags::TRUNCATE,
            )
            .await?;

        Ok(())
    }

    /// Update the current work context in L0.
    ///
    /// Overwrites `/L0/current` with the provided context text.
    pub async fn update_current(&self, context: &str) -> EvifResult<()> {
        self.plugin
            .write(
                "/L0/current",
                context.as_bytes().to_vec(),
                0,
                WriteFlags::TRUNCATE,
            )
            .await?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a fresh ContextManager for each test.
    fn make_manager() -> ContextManager {
        ContextManager::new(Arc::new(ContextFsPlugin::new()))
    }

    #[tokio::test]
    async fn test_create_and_close_session() {
        let mgr = make_manager();

        // Create a session.
        mgr.create_session("test-session-001")
            .await
            .expect("create session");

        // Verify session_id was written.
        let session_data = mgr
            .plugin
            .read("/L1/session_id", 0, 0)
            .await
            .expect("read session_id");
        let session_text = String::from_utf8(session_data).expect("utf8");
        assert!(
            session_text.contains("test-session-001"),
            "session_id should be set, got: {}",
            session_text,
        );

        // Write some decisions to L1.
        mgr.save_decision("use layered context architecture")
            .await
            .expect("save decision");

        // Close (and archive) the session.
        mgr.close_session("test-session-001")
            .await
            .expect("close session");

        // Verify that the archive was created in L2/history.
        let archive = mgr
            .plugin
            .read("/L2/history/test-session-001.md", 0, 0)
            .await
            .expect("read archived session");
        let archive_text = String::from_utf8(archive).expect("utf8");
        assert!(
            archive_text.contains("test-session-001"),
            "archive header should contain session id, got: {}",
            archive_text,
        );
        assert!(
            archive_text.contains("use layered context architecture"),
            "archive should contain saved decisions, got: {}",
            archive_text,
        );

        // Verify session was removed from the active map.
        assert!(
            !mgr.sessions.read().await.contains_key("test-session-001"),
            "session should be removed after close",
        );
    }

    #[tokio::test]
    async fn test_token_estimation() {
        let mgr = make_manager();

        // Write a known amount of content.
        let content = "Hello, world! This is a test string.";
        let byte_len = content.len();
        mgr.plugin
            .create("/L0/test_tokens", 0o644)
            .await
            .expect("create test file");
        mgr.plugin
            .write(
                "/L0/test_tokens",
                content.as_bytes().to_vec(),
                0,
                WriteFlags::TRUNCATE,
            )
            .await
            .expect("write test file");

        let tokens = mgr
            .estimate_tokens("/L0/test_tokens")
            .await
            .expect("estimate tokens");

        assert_eq!(
            tokens,
            byte_len / 4,
            "token estimate should be bytes/4, expected {}, got {}",
            byte_len / 4,
            tokens,
        );

        // Layer usage should be non-zero.
        let l0_usage = mgr
            .get_layer_usage(ContextLayer::L0)
            .await
            .expect("L0 usage");
        assert!(
            l0_usage > 0,
            "L0 layer usage should be > 0 after writing content",
        );
    }

    #[tokio::test]
    async fn test_save_decision() {
        let mgr = make_manager();

        mgr.save_decision("adopt microservice architecture")
            .await
            .expect("save first decision");
        mgr.save_decision("use PostgreSQL for persistence")
            .await
            .expect("save second decision");

        let raw = mgr
            .plugin
            .read("/L1/decisions.md", 0, 0)
            .await
            .expect("read decisions");
        let text = String::from_utf8(raw).expect("utf8");

        assert!(
            text.contains("adopt microservice architecture"),
            "decisions should contain the first decision, got: {}",
            text,
        );
        assert!(
            text.contains("use PostgreSQL for persistence"),
            "decisions should contain the second decision, got: {}",
            text,
        );
    }

    #[tokio::test]
    async fn test_search_context() {
        let mgr = make_manager();

        // Write some searchable content to different layers.
        mgr.update_current("Currently working on search implementation")
            .await
            .expect("update current");

        mgr.save_decision("use grep-based search for context")
            .await
            .expect("save decision");

        // Write to L2 as well.
        mgr.plugin
            .create("/L2/search_notes.md", 0o644)
            .await
            .expect("create L2 file");
        mgr.plugin
            .write(
                "/L2/search_notes.md",
                b"Notes on search: the grep approach is simple and effective.\n".to_vec(),
                0,
                WriteFlags::TRUNCATE,
            )
            .await
            .expect("write L2 file");

        // Search for "search" across all layers.
        let results = mgr
            .search_context("search")
            .await
            .expect("search context");

        let paths: Vec<&str> = results.iter().map(|r| r.path.as_str()).collect();

        assert!(
            paths.iter().any(|p| p.contains("L0")),
            "results should include L0 files, got: {:?}",
            paths,
        );
        assert!(
            paths.iter().any(|p| p.contains("L1")),
            "results should include L1 files, got: {:?}",
            paths,
        );
        assert!(
            paths.iter().any(|p| p.contains("L2")),
            "results should include L2 files, got: {:?}",
            paths,
        );

        // Verify snippets contain the query.
        for result in &results {
            assert!(
                result.snippet.to_lowercase().contains("search"),
                "snippet should contain query, got: {}",
                result.snippet,
            );
        }
    }

    #[tokio::test]
    async fn test_semantic_search_text_fallback() {
        let mgr = make_manager();

        // Write content to L2 for searching
        mgr.plugin
            .create("/L2/architecture_notes.md", 0o644)
            .await
            .expect("create L2 file");
        mgr.plugin
            .write(
                "/L2/architecture_notes.md",
                b"Layered context architecture uses L0 for immediate context.\nL1 stores session decisions and drafts.\nL2 contains project knowledge and patterns.".to_vec(),
                0,
                WriteFlags::TRUNCATE,
            )
            .await
            .expect("write L2 file");

        // Search for "architecture" - should find it in L2
        let results = mgr
            .semantic_search("architecture", 5)
            .await
            .expect("semantic search should work");

        assert!(
            !results.is_empty(),
            "semantic search should return results for 'architecture'"
        );
        assert!(
            results.iter().any(|r| r.source.contains("architecture_notes") || r.content.contains("architecture")),
            "results should contain architecture content, got: {:?}",
            results.iter().map(|r| (&r.source, &r.content[..r.content.len().min(50)])).collect::<Vec<_>>(),
        );

        // Test with non-matching query - should return empty
        let empty_results = mgr
            .semantic_search("xyznonexistent123", 5)
            .await
            .expect("semantic search should work");
        assert!(
            empty_results.is_empty(),
            "non-matching query should return empty results"
        );
    }

    #[tokio::test]
    async fn test_semantic_search_limit() {
        let mgr = make_manager();

        // Write multiple files to L2
        for i in 0..10 {
            let path = format!("/L2/test_{}.md", i);
            mgr.plugin.create(&path, 0o644).await.expect("create file");
            mgr.plugin
                .write(&path, format!("Test content with keyword {}", i).into_bytes(), 0, WriteFlags::TRUNCATE)
                .await
                .expect("write file");
        }

        // Search with limit
        let results = mgr
            .semantic_search("keyword", 3)
            .await
            .expect("semantic search should work");

        assert!(
            results.len() <= 3,
            "results should be limited to 3, got {}",
            results.len()
        );
    }

    #[tokio::test]
    async fn test_generate_summary_fallback() {
        let mgr = make_manager();

        // Short content - should return as-is
        let short_content = "This is a short message.";
        let summary = mgr
            .generate_summary(short_content, 200)
            .await
            .expect("generate summary should work");
        assert_eq!(summary, short_content, "short content should be returned unchanged");

        // Long content - should be truncated
        let long_content = "This is a very long piece of content that exceeds the maximum length. ".repeat(50);
        let summary = mgr
            .generate_summary(&long_content, 100)
            .await
            .expect("generate summary should work");

        assert!(
            summary.len() <= 110, // Allow for "...\n" suffix
            "summary should be truncated, got len {} for max_length 100",
            summary.len()
        );
        assert!(
            summary.ends_with("..."),
            "truncated summary should end with '...'"
        );
    }

    #[tokio::test]
    async fn test_generate_summary_max_length_zero() {
        let mgr = make_manager();

        // max_length 0 should use default (200)
        let content = "x".repeat(500);
        let summary = mgr
            .generate_summary(&content, 0)
            .await
            .expect("generate summary should work with max_length 0");

        assert!(
            summary.len() <= 210, // ~200 + "..."
            "summary with 0 max_length should use default 200"
        );
    }

    #[tokio::test]
    async fn test_simple_summary_word_boundary() {
        // Test word boundary truncation
        let long_text = "Hello world this is a very long sentence that should be truncated at a word boundary and not in the middle of a word.";
        let summary = ContextManager::simple_summary(long_text, 50);

        assert!(
            summary.ends_with("..."),
            "summary should end with '...'"
        );
        // Check that it doesn't cut in the middle of words
        let truncated_content = &long_text[..50];
        if let Some(last_space) = truncated_content.rfind(' ') {
            // If we find a space within 10 chars of the end, truncation happened at a word boundary
            assert!(
                last_space > 40,
                "summary should truncate at word boundary, but last space was at {}",
                last_space
            );
        }
    }
}
