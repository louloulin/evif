// EVIF Memory REST API Handlers
//
// Memory management HTTP interfaces
// Implements mem.md API design

use axum::{
    extract::{Path, State, Query},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use evif_mem::storage::memory::MemoryStorage;
use evif_mem::models::{MemoryItem, MemoryType};

/// Memory state shared across handlers
#[derive(Clone)]
pub struct MemoryState {
    /// Storage for memory items
    pub storage: Arc<MemoryStorage>,
}

impl MemoryState {
    /// Create new memory state with storage
    pub fn new() -> Self {
        Self {
            storage: Arc::new(MemoryStorage::new()),
        }
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
    pub content: String,
    #[serde(default = "default_modality")]
    pub modality: String,
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
    #[serde(default = "default_vector_k")]
    pub vector_k: usize,
    #[serde(default = "default_llm_top_n")]
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
                item.content.to_lowercase().contains(&query_lower) ||
                item.summary.to_lowercase().contains(&query_lower)
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
            Err(_) => Err(MemoryError::NotFound(format!("Memory with id '{}' not found", id)))
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
