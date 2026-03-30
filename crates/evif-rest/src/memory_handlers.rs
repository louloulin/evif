// EVIF Memory REST API Handlers
//
// Memory management HTTP interfaces
// Implements mem.md API design

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path as FsPath, PathBuf},
    sync::Arc,
};

use evif_mem::models::{MemoryCategory, MemoryItem, MemoryType};
use evif_mem::storage::{MemoryStorage, SQLiteStorage};

trait MemoryStore: Send + Sync {
    fn put_item(&self, item: MemoryItem) -> evif_mem::error::MemResult<()>;
    fn get_item(&self, id: &str) -> evif_mem::error::MemResult<MemoryItem>;
    fn get_all_items(&self) -> Vec<MemoryItem>;
    fn get_category(&self, id: &str) -> evif_mem::error::MemResult<MemoryCategory>;
    fn get_all_categories(&self) -> Vec<MemoryCategory>;
    fn get_items_in_category(&self, category_id: &str) -> Vec<MemoryItem>;
}

impl MemoryStore for MemoryStorage {
    fn put_item(&self, item: MemoryItem) -> evif_mem::error::MemResult<()> {
        MemoryStorage::put_item(self, item)
    }

    fn get_item(&self, id: &str) -> evif_mem::error::MemResult<MemoryItem> {
        MemoryStorage::get_item(self, id)
    }

    fn get_all_items(&self) -> Vec<MemoryItem> {
        MemoryStorage::get_all_items(self)
    }

    fn get_category(&self, id: &str) -> evif_mem::error::MemResult<MemoryCategory> {
        MemoryStorage::get_category(self, id)
    }

    fn get_all_categories(&self) -> Vec<MemoryCategory> {
        MemoryStorage::get_all_categories(self)
    }

    fn get_items_in_category(&self, category_id: &str) -> Vec<MemoryItem> {
        MemoryStorage::get_items_in_category(self, category_id)
    }
}

impl MemoryStore for SQLiteStorage {
    fn put_item(&self, item: MemoryItem) -> evif_mem::error::MemResult<()> {
        SQLiteStorage::put_item(self, item)
    }

    fn get_item(&self, id: &str) -> evif_mem::error::MemResult<MemoryItem> {
        SQLiteStorage::get_item(self, id)
    }

    fn get_all_items(&self) -> Vec<MemoryItem> {
        SQLiteStorage::get_all_items(self)
    }

    fn get_category(&self, id: &str) -> evif_mem::error::MemResult<MemoryCategory> {
        SQLiteStorage::get_category(self, id)
    }

    fn get_all_categories(&self) -> Vec<MemoryCategory> {
        SQLiteStorage::get_all_categories(self)
    }

    fn get_items_in_category(&self, category_id: &str) -> Vec<MemoryItem> {
        SQLiteStorage::get_items_in_category(self, category_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryBackendKind {
    InMemory,
    SQLite,
}

impl MemoryBackendKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InMemory => "memory",
            Self::SQLite => "sqlite",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryBackendConfig {
    backend: MemoryBackendKind,
    sqlite_path: Option<PathBuf>,
}

impl MemoryBackendConfig {
    pub fn in_memory() -> Self {
        Self {
            backend: MemoryBackendKind::InMemory,
            sqlite_path: None,
        }
    }

    pub fn sqlite<P: Into<PathBuf>>(path: P) -> Self {
        Self {
            backend: MemoryBackendKind::SQLite,
            sqlite_path: Some(path.into()),
        }
    }

    pub fn backend(&self) -> &MemoryBackendKind {
        &self.backend
    }

    pub fn sqlite_path(&self) -> Option<&FsPath> {
        self.sqlite_path.as_deref()
    }

    pub fn from_env() -> Result<Self, String> {
        let backend = env::var("EVIF_REST_MEMORY_BACKEND")
            .unwrap_or_else(|_| "memory".to_string())
            .trim()
            .to_ascii_lowercase();

        match backend.as_str() {
            "" | "memory" | "in-memory" | "in_memory" => Ok(Self::in_memory()),
            "sqlite" => {
                let path = env::var("EVIF_REST_MEMORY_SQLITE_PATH").map_err(|_| {
                    "EVIF_REST_MEMORY_SQLITE_PATH is required when EVIF_REST_MEMORY_BACKEND=sqlite"
                        .to_string()
                })?;
                let trimmed = path.trim();
                if trimmed.is_empty() {
                    return Err(
                        "EVIF_REST_MEMORY_SQLITE_PATH cannot be empty when SQLite backend is enabled"
                            .to_string(),
                    );
                }
                Ok(Self::sqlite(trimmed))
            }
            other => Err(format!(
                "Unsupported EVIF_REST_MEMORY_BACKEND '{}'. Expected one of: memory, sqlite",
                other
            )),
        }
    }
}

/// Check if production mode is enabled
pub fn is_production_mode() -> bool {
    std::env::var("EVIF_REST_PRODUCTION_MODE")
        .map(|v| v.trim().to_ascii_lowercase() == "true" || v == "1")
        .unwrap_or(false)
}

/// Validate memory backend configuration for production mode
/// In production mode, in-memory backend is not allowed (data would be lost on restart)
pub fn validate_memory_for_production(config: &MemoryBackendConfig) -> Result<(), String> {
    if !is_production_mode() {
        return Ok(());
    }

    match config.backend() {
        MemoryBackendKind::InMemory => Err(
            "EVIF_REST_PRODUCTION_MODE requires persistent memory backend. \
             Set EVIF_REST_MEMORY_BACKEND=sqlite and EVIF_REST_MEMORY_SQLITE_PATH=/path/to/db".to_string()
        ),
        MemoryBackendKind::SQLite => {
            // SQLite for is acceptable production
            Ok(())
        }
    }
}

impl Default for MemoryBackendConfig {
    fn default() -> Self {
        Self::in_memory()
    }
}

fn ensure_sqlite_parent(path: &FsPath) -> Result<(), String> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "Failed to create SQLite parent directory '{}': {}",
                parent.display(),
                err
            )
        })?;
    }

    Ok(())
}

/// Memory state shared across handlers
#[derive(Clone)]
pub struct MemoryState {
    /// Storage for memory items
    storage: Arc<dyn MemoryStore>,
    backend: MemoryBackendKind,
}

impl MemoryState {
    /// Create new memory state with storage
    pub fn new() -> Self {
        Self::in_memory()
    }

    pub fn in_memory() -> Self {
        Self {
            storage: Arc::new(MemoryStorage::new()),
            backend: MemoryBackendKind::InMemory,
        }
    }

    pub fn from_config(config: &MemoryBackendConfig) -> Result<Self, String> {
        match config.backend() {
            MemoryBackendKind::InMemory => Ok(Self::in_memory()),
            MemoryBackendKind::SQLite => {
                let path = config.sqlite_path().ok_or_else(|| {
                    "SQLite backend requires a configured database path".to_string()
                })?;
                ensure_sqlite_parent(path)?;
                let storage = SQLiteStorage::new(path).map_err(|err| {
                    format!(
                        "Failed to initialize SQLite memory backend at '{}': {}",
                        path.display(),
                        err
                    )
                })?;
                Ok(Self {
                    storage: Arc::new(storage),
                    backend: MemoryBackendKind::SQLite,
                })
            }
        }
    }

    pub fn backend_name(&self) -> &'static str {
        self.backend.as_str()
    }
}

impl Default for MemoryState {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize memory state
pub fn create_memory_state() -> MemoryState {
    MemoryState::new()
}

/// Initialize memory state from explicit backend config
pub fn create_memory_state_from_config(config: &MemoryBackendConfig) -> Result<MemoryState, String> {
    MemoryState::from_config(config)
}

/// Initialize memory state from environment variables
pub fn create_memory_state_from_env() -> Result<MemoryState, String> {
    let config = MemoryBackendConfig::from_env()?;
    create_memory_state_from_config(&config)
}

/// Initialize memory pipelines (to be called on startup with proper dependencies)
pub async fn init_memory_pipelines(
    _state: &MemoryState,
    _openai_api_key: &str,
) -> Result<(), String> {
    // Pipeline initialization is optional - handlers work with basic storage
    // Pipeline can be added later when LLM integration is needed
    Ok(())
}

/// Create memory request
#[derive(Debug, Deserialize)]
pub struct CreateMemoryRequest {
    #[serde(alias = "text")]
    pub content: String,
    #[serde(default = "default_modality")]
    pub modality: String,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

fn default_modality() -> String {
    "text".to_string()
}

/// Create memory response
#[derive(Debug, Serialize)]
pub struct CreateMemoryResponse {
    pub memory_id: String,
    pub extracted_items: Vec<ExtractedMemoryItem>,
}

/// Extracted memory item
#[derive(Debug, Serialize)]
pub struct ExtractedMemoryItem {
    pub id: String,
    #[serde(rename = "type")]
    pub memory_type: String,
    pub summary: String,
    pub category: Option<String>,
}

/// Search memory request
#[derive(Debug, Deserialize)]
pub struct SearchMemoryRequest {
    pub query: String,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_vector_k", alias = "k")]
    pub vector_k: usize,
    #[serde(default = "default_llm_top_n", alias = "top_n")]
    pub llm_top_n: usize,
}

fn default_mode() -> String {
    "vector".to_string()
}

fn default_vector_k() -> usize {
    10
}

fn default_llm_top_n() -> usize {
    5
}

/// Search memory response
#[derive(Debug, Serialize)]
pub struct SearchMemoryResponse {
    pub results: Vec<MemorySearchResult>,
    pub total: usize,
}

/// Memory search result
#[derive(Debug, Serialize)]
pub struct MemorySearchResult {
    pub id: String,
    #[serde(rename = "type")]
    pub memory_type: String,
    pub content: String,
    pub score: f32,
    pub category: Option<String>,
    pub created: String,
    pub updated: String,
}

/// Memory item response
#[derive(Debug, Serialize)]
pub struct MemoryItemResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub memory_type: String,
    pub content: String,
    pub summary: String,
    pub created: String,
    pub updated: String,
}

/// Category response
#[derive(Debug, Serialize)]
pub struct CategoryResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub item_count: u32,
    pub created: String,
    pub updated: String,
}

/// Category with memories response
#[derive(Debug, Serialize)]
pub struct CategoryWithMemoriesResponse {
    pub category: CategoryResponse,
    pub memories: Vec<MemoryItemResponse>,
}

/// Memory handlers
pub struct MemoryHandlers;

impl MemoryHandlers {
    /// Create memory (POST /api/v1/memories)
    ///
    /// Creates a new memory from the given content.
    /// Stores the content directly as a MemoryItem for now.
    pub async fn create_memory(
        State(state): State<MemoryState>,
        Json(req): Json<CreateMemoryRequest>,
    ) -> Result<Json<CreateMemoryResponse>, MemoryError> {
        // Create a simple memory item from the content
        let memory_item = MemoryItem::new(
            MemoryType::Knowledge,
            req.content.chars().take(100).collect(),
            req.content,
        );

        // Store in memory
        if let Err(e) = state.storage.put_item(memory_item.clone()) {
            return Err(MemoryError::Internal(e.to_string()));
        }

        let memory_id = memory_item.id.clone();
        let extracted_items = vec![ExtractedMemoryItem {
            id: memory_item.id,
            memory_type: "knowledge".to_string(),
            summary: memory_item.summary,
            category: None,
        }];

        Ok(Json(CreateMemoryResponse {
            memory_id,
            extracted_items,
        }))
    }

    /// Search memories (POST /api/v1/memories/search)
    ///
    /// Searches memories using simple content matching.
    /// For now, performs basic text search on stored memories.
    pub async fn search_memories(
        State(state): State<MemoryState>,
        Json(req): Json<SearchMemoryRequest>,
    ) -> Result<Json<SearchMemoryResponse>, MemoryError> {
        // Get all items and do simple text matching
        let all_items = state.storage.get_all_items();

        // Simple search - filter by content containing query
        let query_lower = req.query.to_lowercase();
        let results: Vec<MemorySearchResult> = all_items
            .into_iter()
            .filter(|item| {
                item.content.to_lowercase().contains(&query_lower)
                    || item.summary.to_lowercase().contains(&query_lower)
            })
            .take(req.vector_k)
            .enumerate()
            .map(|(idx, item)| {
                // Calculate a simple score based on position (higher = better)
                let score = 1.0 - (idx as f32 * 0.1);
                MemorySearchResult {
                    id: item.id,
                    memory_type: format!("{:?}", item.memory_type),
                    content: item.content,
                    score: score.max(0.0),
                    category: item.category_id,
                    created: item.created_at.to_rfc3339(),
                    updated: item.updated_at.to_rfc3339(),
                }
            })
            .collect();

        let total = results.len();
        Ok(Json(SearchMemoryResponse { results, total }))
    }

    /// Get memory by ID (GET /api/v1/memories/{id})
    ///
    /// Retrieves a specific memory by its ID.
    pub async fn get_memory(
        State(state): State<MemoryState>,
        Path(id): Path<String>,
    ) -> Result<Json<MemoryItemResponse>, MemoryError> {
        // Try to get the memory from storage
        match state.storage.get_item(&id) {
            Ok(item) => Ok(Json(MemoryItemResponse {
                id: item.id,
                memory_type: format!("{:?}", item.memory_type),
                content: item.content,
                summary: item.summary,
                created: item.created_at.to_rfc3339(),
                updated: item.updated_at.to_rfc3339(),
            })),
            Err(_) => Err(MemoryError::NotFound(format!(
                "Memory with id '{}' not found",
                id
            ))),
        }
    }

    /// List all memories (GET /api/v1/memories)
    ///
    /// Lists all stored memories.
    pub async fn list_memories(
        State(state): State<MemoryState>,
    ) -> Result<Json<Vec<MemoryItemResponse>>, MemoryError> {
        // Get all items from storage
        let items = state.storage.get_all_items();

        let responses: Vec<MemoryItemResponse> = items
            .into_iter()
            .map(|item| MemoryItemResponse {
                id: item.id,
                memory_type: format!("{:?}", item.memory_type),
                content: item.content,
                summary: item.summary,
                created: item.created_at.to_rfc3339(),
                updated: item.updated_at.to_rfc3339(),
            })
            .collect();

        Ok(Json(responses))
    }

    /// List all categories (GET /api/v1/categories)
    ///
    /// Lists all memory categories.
    pub async fn list_categories(
        State(state): State<MemoryState>,
    ) -> Result<Json<Vec<CategoryResponse>>, MemoryError> {
        let categories = state.storage.get_all_categories();

        let responses: Vec<CategoryResponse> = categories
            .into_iter()
            .map(|cat| CategoryResponse {
                id: cat.id,
                name: cat.name,
                description: cat.description,
                item_count: cat.item_count,
                created: cat.created_at.to_rfc3339(),
                updated: cat.updated_at.to_rfc3339(),
            })
            .collect();

        Ok(Json(responses))
    }

    /// Get category by ID (GET /api/v1/categories/{id})
    ///
    /// Retrieves a specific category by its ID.
    pub async fn get_category(
        State(state): State<MemoryState>,
        Path(id): Path<String>,
    ) -> Result<Json<CategoryResponse>, MemoryError> {
        match state.storage.get_category(&id) {
            Ok(cat) => Ok(Json(CategoryResponse {
                id: cat.id,
                name: cat.name,
                description: cat.description,
                item_count: cat.item_count,
                created: cat.created_at.to_rfc3339(),
                updated: cat.updated_at.to_rfc3339(),
            })),
            Err(_) => Err(MemoryError::NotFound(format!(
                "Category with id '{}' not found",
                id
            ))),
        }
    }

    /// Get memories in category (GET /api/v1/categories/{id}/memories)
    ///
    /// Retrieves all memories belonging to a specific category.
    pub async fn get_category_memories(
        State(state): State<MemoryState>,
        Path(id): Path<String>,
    ) -> Result<Json<CategoryWithMemoriesResponse>, MemoryError> {
        // Get category
        let category = state
            .storage
            .get_category(&id)
            .map_err(|_| MemoryError::NotFound(format!("Category with id '{}' not found", id)))?;

        // Get memories in category
        let items = state.storage.get_items_in_category(&id);

        let memories: Vec<MemoryItemResponse> = items
            .into_iter()
            .map(|item| MemoryItemResponse {
                id: item.id,
                memory_type: format!("{:?}", item.memory_type),
                content: item.content,
                summary: item.summary,
                created: item.created_at.to_rfc3339(),
                updated: item.updated_at.to_rfc3339(),
            })
            .collect();

        Ok(Json(CategoryWithMemoriesResponse {
            category: CategoryResponse {
                id: category.id,
                name: category.name,
                description: category.description,
                item_count: category.item_count,
                created: category.created_at.to_rfc3339(),
                updated: category.updated_at.to_rfc3339(),
            },
            memories,
        }))
    }
}

/// Memory query request
#[derive(Debug, Deserialize)]
pub struct MemoryQueryRequest {
    /// Query type: causal_chain, timeline, temporal_bfs, temporal_path
    #[serde(rename = "query_type")]
    pub query_type: String,
    /// Start node ID for causal_chain, temporal_bfs, timeline queries
    #[serde(alias = "node_id")]
    pub start_node: Option<String>,
    /// End node ID for temporal_path queries
    pub end_node: Option<String>,
    /// Maximum depth for traversal queries
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    /// Event type filter for timeline queries
    pub event_type: Option<String>,
    /// Category filter for timeline queries
    pub category: Option<String>,
    /// Start time for time range queries
    pub start_time: Option<String>,
    /// End time for time range queries
    pub end_time: Option<String>,
}

fn default_max_depth() -> usize {
    5
}

fn memory_query_timestamp(item: &MemoryItem) -> DateTime<Utc> {
    item.happened_at.unwrap_or(item.created_at)
}

fn memory_query_node_type(item: &MemoryItem) -> &'static str {
    match item.memory_type {
        MemoryType::Event => "event",
        _ => "memory",
    }
}

fn to_memory_query_node(item: &MemoryItem) -> MemoryQueryNodeInfo {
    MemoryQueryNodeInfo {
        id: item.id.clone(),
        node_type: memory_query_node_type(item).to_string(),
        label: item.summary.clone(),
        timestamp: Some(memory_query_timestamp(item).to_rfc3339()),
    }
}

fn parse_memory_query_time(
    field_name: &str,
    value: Option<&str>,
) -> Result<Option<DateTime<Utc>>, MemoryError> {
    value
        .map(|raw| {
            chrono::DateTime::parse_from_rfc3339(raw)
                .map(|parsed| parsed.with_timezone(&Utc))
                .map_err(|err| {
                    MemoryError::BadRequest(format!(
                        "{} must be RFC3339 timestamp: {}",
                        field_name, err
                    ))
                })
        })
        .transpose()
}

fn filtered_memory_query_memories(
    state: &MemoryState,
    req: &MemoryQueryRequest,
) -> Result<Vec<MemoryItem>, MemoryError> {
    let start_time = parse_memory_query_time("start_time", req.start_time.as_deref())?;
    let end_time = parse_memory_query_time("end_time", req.end_time.as_deref())?;
    if let (Some(start), Some(end)) = (start_time, end_time) {
        if start > end {
            return Err(MemoryError::BadRequest(
                "start_time must be before or equal to end_time".to_string(),
            ));
        }
    }

    let event_type_filter = req.event_type.as_ref().map(|value| value.to_lowercase());
    let category_filter = req.category.as_deref();

    let mut items: Vec<MemoryItem> = state
        .storage
        .get_all_items()
        .into_iter()
        .filter(|item| {
            let timestamp = memory_query_timestamp(item);
            let category_matches = category_filter
                .map(|category| item.category_id.as_deref() == Some(category))
                .unwrap_or(true);
            let event_matches = event_type_filter
                .as_ref()
                .map(|event_type| item.memory_type.as_str() == event_type)
                .unwrap_or(true);
            let start_matches = start_time.map(|start| timestamp >= start).unwrap_or(true);
            let end_matches = end_time.map(|end| timestamp <= end).unwrap_or(true);

            category_matches && event_matches && start_matches && end_matches
        })
        .collect();

    items.sort_by(|left, right| {
        memory_query_timestamp(left)
            .cmp(&memory_query_timestamp(right))
            .then_with(|| left.id.cmp(&right.id))
    });

    Ok(items)
}

fn find_memory_query_node_index(items: &[MemoryItem], node_id: &str) -> Result<usize, MemoryError> {
    items
        .iter()
        .position(|item| item.id == node_id)
        .ok_or_else(|| MemoryError::NotFound(format!("Memory node '{}' not found", node_id)))
}

/// Memory query response
#[derive(Debug, Serialize)]
pub struct MemoryQueryResponse {
    /// Query type that was executed
    pub query_type: String,
    /// Result nodes (for causal_chain, temporal_bfs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nodes: Option<Vec<MemoryQueryNodeInfo>>,
    /// Result paths (for temporal_path)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paths: Option<Vec<MemoryQueryPathInfo>>,
    /// Timeline events (for timeline queries)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeline: Option<Vec<TimelineEventInfo>>,
    /// Total count of results
    pub total: usize,
}

/// Memory query node info for response
#[derive(Debug, Serialize)]
pub struct MemoryQueryNodeInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// Memory query path info for response
#[derive(Debug, Serialize)]
pub struct MemoryQueryPathInfo {
    pub nodes: Vec<String>,
    pub edges: Vec<String>,
    pub narrative: String,
}

/// Timeline event info for response
#[derive(Debug, Serialize)]
pub struct TimelineEventInfo {
    pub node_id: String,
    pub timestamp: String,
    pub event_type: String,
}

impl MemoryHandlers {
    /// Query memories (POST /api/v1/memories/query)
    ///
    /// Projects timeline and relationship views from stored memories.
    pub async fn query_memories(
        State(state): State<MemoryState>,
        Json(req): Json<MemoryQueryRequest>,
    ) -> Result<Json<MemoryQueryResponse>, MemoryError> {
        match req.query_type.as_str() {
            "causal_chain" => {
                let start_node = req.start_node.as_deref().ok_or_else(|| {
                    MemoryError::BadRequest("start_node required for causal_chain query".to_string())
                })?;
                let items = filtered_memory_query_memories(&state, &req)?;
                let start_index = find_memory_query_node_index(&items, start_node)?;
                let chain_start = start_index.saturating_sub(req.max_depth);
                let nodes = items[chain_start..=start_index]
                    .iter()
                    .map(to_memory_query_node)
                    .collect::<Vec<_>>();
                let total = nodes.len();

                Ok(Json(MemoryQueryResponse {
                    query_type: req.query_type,
                    nodes: Some(nodes),
                    paths: None,
                    timeline: None,
                    total,
                }))
            }

            "timeline" => {
                let mut items = filtered_memory_query_memories(&state, &req)?;

                if let Some(start_node) = req.start_node.as_deref() {
                    let start_index = find_memory_query_node_index(&items, start_node)?;
                    let end_index = std::cmp::min(items.len(), start_index + req.max_depth + 1);
                    items = items[start_index..end_index].to_vec();
                }

                let nodes = items.iter().map(to_memory_query_node).collect::<Vec<_>>();
                let timeline = items
                    .iter()
                    .map(|item| TimelineEventInfo {
                        node_id: item.id.clone(),
                        timestamp: memory_query_timestamp(item).to_rfc3339(),
                        event_type: item.memory_type.as_str().to_string(),
                    })
                    .collect::<Vec<_>>();
                let total = timeline.len();

                Ok(Json(MemoryQueryResponse {
                    query_type: req.query_type,
                    nodes: Some(nodes),
                    paths: None,
                    timeline: Some(timeline),
                    total,
                }))
            }

            "temporal_bfs" => {
                let start_node = req.start_node.as_deref().ok_or_else(|| {
                    MemoryError::BadRequest("start_node required for temporal_bfs query".to_string())
                })?;
                let items = filtered_memory_query_memories(&state, &req)?;
                let start_index = find_memory_query_node_index(&items, start_node)?;
                let end_index = std::cmp::min(items.len(), start_index + req.max_depth + 1);
                let nodes = items[start_index..end_index]
                    .iter()
                    .map(to_memory_query_node)
                    .collect::<Vec<_>>();
                let total = nodes.len();

                Ok(Json(MemoryQueryResponse {
                    query_type: req.query_type,
                    nodes: Some(nodes),
                    paths: None,
                    timeline: None,
                    total,
                }))
            }

            "temporal_path" => {
                let start_node = req.start_node.as_deref().ok_or_else(|| {
                    MemoryError::BadRequest("start_node required for temporal_path query".to_string())
                })?;
                let end_node = req.end_node.as_deref().ok_or_else(|| {
                    MemoryError::BadRequest("end_node required for temporal_path query".to_string())
                })?;
                let items = filtered_memory_query_memories(&state, &req)?;
                let start_index = find_memory_query_node_index(&items, start_node)?;
                let end_index = find_memory_query_node_index(&items, end_node)?;

                let (nodes, edge_name, narrative) = if start_index <= end_index {
                    (
                        items[start_index..=end_index]
                            .iter()
                            .map(|item| item.id.clone())
                            .collect::<Vec<_>>(),
                        "Before",
                        format!(
                            "Forward temporal path across {} memories",
                            end_index - start_index + 1
                        ),
                    )
                } else {
                    (
                        items[end_index..=start_index]
                            .iter()
                            .rev()
                            .map(|item| item.id.clone())
                            .collect::<Vec<_>>(),
                        "After",
                        format!(
                            "Reverse temporal path across {} memories",
                            start_index - end_index + 1
                        ),
                    )
                };
                let edges = vec![edge_name.to_string(); nodes.len().saturating_sub(1)];
                let paths = vec![MemoryQueryPathInfo {
                    nodes,
                    edges,
                    narrative,
                }];

                Ok(Json(MemoryQueryResponse {
                    query_type: req.query_type,
                    nodes: None,
                    paths: Some(paths),
                    timeline: None,
                    total: 1,
                }))
            }

            _ => Err(MemoryError::BadRequest(format!(
                "Unknown query_type: {}. Supported types: causal_chain, timeline, temporal_bfs, temporal_path",
                req.query_type
            )))
        }
    }
}

/// Memory API errors
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Memory system not initialized")]
    NotInitialized,
}

impl axum::response::IntoResponse for MemoryError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            MemoryError::NotFound(msg) => (axum::http::StatusCode::NOT_FOUND, msg),
            MemoryError::BadRequest(msg) => (axum::http::StatusCode::BAD_REQUEST, msg),
            MemoryError::Internal(msg) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg),
            MemoryError::NotInitialized => (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "Memory system not initialized".to_string(),
            ),
        };

        let body = axum::Json(serde_json::json!({
            "error": status.to_string(),
            "message": message,
        }));

        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_memory_request_accepts_legacy_text_alias() {
        let json = r#"{
            "text": "remember this later",
            "modality": "conversation"
        }"#;

        let req: CreateMemoryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "remember this later");
        assert_eq!(req.modality, "conversation");
        assert!(req.metadata.is_none());
    }

    #[test]
    fn test_search_memory_request_accepts_legacy_k_alias() {
        let json = r#"{
            "query": "rust",
            "mode": "hybrid",
            "k": 7,
            "top_n": 4
        }"#;

        let req: SearchMemoryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.query, "rust");
        assert_eq!(req.mode, "hybrid");
        assert_eq!(req.vector_k, 7);
        assert_eq!(req.llm_top_n, 4);
    }

    #[test]
    fn test_memory_query_request_deserialization() {
        let json = r#"{
            "query_type": "causal_chain",
            "start_node": "node-123",
            "max_depth": 10
        }"#;

        let req: MemoryQueryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.query_type, "causal_chain");
        assert_eq!(req.start_node, Some("node-123".to_string()));
        assert_eq!(req.max_depth, 10);
    }

    #[test]
    fn test_memory_query_request_accepts_legacy_node_id_alias() {
        let json = r#"{
            "query_type": "temporal_bfs",
            "node_id": "node-legacy"
        }"#;

        let req: MemoryQueryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.query_type, "temporal_bfs");
        assert_eq!(req.start_node, Some("node-legacy".to_string()));
        assert_eq!(req.max_depth, 5);
    }

    #[test]
    fn test_memory_query_request_temporal_path() {
        let json = r#"{
            "query_type": "temporal_path",
            "start_node": "node-a",
            "end_node": "node-b"
        }"#;

        let req: MemoryQueryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.query_type, "temporal_path");
        assert_eq!(req.start_node, Some("node-a".to_string()));
        assert_eq!(req.end_node, Some("node-b".to_string()));
        assert_eq!(req.max_depth, 5); // default
    }

    #[test]
    fn test_memory_query_request_timeline() {
        let json = r#"{
            "query_type": "timeline",
            "event_type": "learning",
            "category": "programming"
        }"#;

        let req: MemoryQueryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.query_type, "timeline");
        assert_eq!(req.event_type, Some("learning".to_string()));
        assert_eq!(req.category, Some("programming".to_string()));
    }

    #[test]
    fn test_memory_query_response_serialization() {
        let response = MemoryQueryResponse {
            query_type: "causal_chain".to_string(),
            nodes: Some(vec![MemoryQueryNodeInfo {
                id: "node-1".to_string(),
                node_type: "memory".to_string(),
                label: "Start".to_string(),
                timestamp: Some("2026-03-07T00:00:00Z".to_string()),
            }]),
            paths: None,
            timeline: None,
            total: 1,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("causal_chain"));
        assert!(json.contains("node-1"));
    }

    #[test]
    fn test_memory_query_path_info() {
        let path = MemoryQueryPathInfo {
            nodes: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            edges: vec!["Before".to_string(), "Causes".to_string()],
            narrative: "A before B causes C".to_string(),
        };

        let json = serde_json::to_string(&path).unwrap();
        assert!(json.contains("Before"));
        assert!(json.contains("Causes"));
    }

    #[test]
    fn test_is_production_mode_env_var() {
        // All env var tests merged into one to avoid parallel test race condition
        // Default: not set → false
        std::env::remove_var("EVIF_REST_PRODUCTION_MODE");
        assert!(!is_production_mode());

        // "true" → true
        std::env::set_var("EVIF_REST_PRODUCTION_MODE", "true");
        assert!(is_production_mode());

        // "1" → true
        std::env::set_var("EVIF_REST_PRODUCTION_MODE", "1");
        assert!(is_production_mode());

        // "TRUE" (uppercase) → true
        std::env::set_var("EVIF_REST_PRODUCTION_MODE", "TRUE");
        assert!(is_production_mode());

        // "false" → false
        std::env::set_var("EVIF_REST_PRODUCTION_MODE", "false");
        assert!(!is_production_mode());

        // "0" → false
        std::env::set_var("EVIF_REST_PRODUCTION_MODE", "0");
        assert!(!is_production_mode());

        // empty → false
        std::env::set_var("EVIF_REST_PRODUCTION_MODE", "");
        assert!(!is_production_mode());

        // Clean up
        std::env::remove_var("EVIF_REST_PRODUCTION_MODE");
    }

    #[test]
    fn test_validate_memory_for_production_env() {
        // All production-mode-dependent validation tests merged to avoid env var race
        // Non-production mode allows in-memory backend
        std::env::remove_var("EVIF_REST_PRODUCTION_MODE");
        let config = MemoryBackendConfig::in_memory();
        let result = validate_memory_for_production(&config);
        assert!(result.is_ok());

        // Production mode rejects in-memory backend
        std::env::set_var("EVIF_REST_PRODUCTION_MODE", "true");
        let config = MemoryBackendConfig::in_memory();
        let result = validate_memory_for_production(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("persistent memory backend"));

        // Production mode accepts SQLite backend
        std::env::set_var("EVIF_REST_MEMORY_SQLITE_PATH", "/tmp/test-evif-memory.db");
        std::env::set_var("EVIF_REST_MEMORY_BACKEND", "sqlite");
        let config = MemoryBackendConfig::from_env().unwrap();
        let result = validate_memory_for_production(&config);
        assert!(result.is_ok());

        // Clean up
        std::env::remove_var("EVIF_REST_PRODUCTION_MODE");
        std::env::remove_var("EVIF_REST_MEMORY_SQLITE_PATH");
        std::env::remove_var("EVIF_REST_MEMORY_BACKEND");
    }
}
