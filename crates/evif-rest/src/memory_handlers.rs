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
use std::sync::RwLock;

/// Memory state shared across handlers
#[derive(Clone)]
pub struct MemoryState {
    /// Placeholder for memorize pipeline - will be initialized on startup
    pub initialized: Arc<RwLock<bool>>,
}

/// Initialize memory state
pub fn create_memory_state() -> MemoryState {
    MemoryState {
        initialized: Arc::new(RwLock::new(false)),
    }
}

/// Initialize memory state (to be called on startup with proper dependencies)
pub async fn init_memory_pipelines(_state: &MemoryState) -> Result<(), String> {
    // In production, this would initialize the actual pipelines with:
    // - LLM client (OpenAI/Anthropic/Ollama)
    // - Vector index (InMemoryVectorIndex / Faiss / Qdrant)
    // - Storage backend (Memory / Sled / RocksDB / SQLite)
    *(_state.initialized.write().map_err(|e| e.to_string())?) = true;
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
    pub tags: Vec<String>,
    pub created: String,
    pub updated: String,
    pub references: Vec<String>,
    pub category_ids: Vec<String>,
}

/// Memory handlers
pub struct MemoryHandlers;

impl MemoryHandlers {
    /// Create memory (POST /api/v1/memories)
    ///
    /// Creates a new memory from the given content.
    /// The content is processed through the MemorizePipeline to extract structured memories.
    pub async fn create_memory(
        State(state): State<MemoryState>,
        Json(req): Json<CreateMemoryRequest>,
    ) -> Result<Json<CreateMemoryResponse>, MemoryError> {
        // Check if memory system is initialized
        let initialized = *state.initialized.read()
            .map_err(|e| MemoryError::Internal(e.to_string()))?;

        if !initialized {
            // Return a placeholder response for now
            // In production: pipeline.memorize_text(&req.content).await?;
            let memory_id = uuid::Uuid::new_v4().to_string();

            return Ok(Json(CreateMemoryResponse {
                memory_id,
                extracted_items: vec![ExtractedMemoryItem {
                    id: uuid::Uuid::new_v4().to_string(),
                    memory_type: "knowledge".to_string(),
                    summary: format!("Extracted from: {}", &req.content[..req.content.len().min(50)]),
                    category: None,
                }],
            }));
        }

        // TODO: Call actual MemorizePipeline when initialized
        Err(MemoryError::NotInitialized)
    }

    /// Search memories (POST /api/v1/memories/search)
    ///
    /// Searches memories using the specified mode:
    /// - vector: Vector similarity search
    /// - hybrid: Combined vector + LLM reranking
    pub async fn search_memories(
        State(state): State<MemoryState>,
        Json(req): Json<SearchMemoryRequest>,
    ) -> Result<Json<SearchMemoryResponse>, MemoryError> {
        // Check if memory system is initialized
        let _initialized = *state.initialized.read()
            .map_err(|e| MemoryError::Internal(e.to_string()))?;

        // Return empty results for now
        // In production: pipeline.retrieve_text(&req.query, mode).await?;
        Ok(Json(SearchMemoryResponse {
            results: vec![],
            total: 0,
        }))
    }

    /// Get memory by ID (GET /api/v1/memories/{id})
    ///
    /// Retrieves a specific memory by its ID.
    pub async fn get_memory(
        State(_state): State<MemoryState>,
        Path(id): Path<String>,
    ) -> Result<Json<MemoryItemResponse>, MemoryError> {
        // TODO: Implement actual retrieval from storage
        let _ = id;

        // Return not found for now
        Err(MemoryError::NotFound(format!("Memory with id '{}' not found", id)))
    }

    /// List all memories (GET /api/v1/memories)
    ///
    /// Lists all stored memories.
    pub async fn list_memories(
        State(_state): State<MemoryState>,
    ) -> Result<Json<Vec<MemoryItemResponse>>, MemoryError> {
        // Return empty for now
        // In production: iterate through storage
        Ok(Json(vec![]))
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
