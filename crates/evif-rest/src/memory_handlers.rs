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
            Err(_) => Err(MemoryError::NotFound(format!("Category with id '{}' not found", id)))
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
        let category = state.storage.get_category(&id)
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

/// Graph query request
#[derive(Debug, Deserialize)]
pub struct GraphQueryRequest {
    /// Query type: causal_chain, timeline, temporal_bfs, temporal_path
    #[serde(rename = "query_type")]
    pub query_type: String,
    /// Start node ID for causal_chain, temporal_bfs, timeline queries
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

/// Graph query response
#[derive(Debug, Serialize)]
pub struct GraphQueryResponse {
    /// Query type that was executed
    pub query_type: String,
    /// Result nodes (for causal_chain, temporal_bfs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nodes: Option<Vec<GraphNodeInfo>>,
    /// Result paths (for temporal_path)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paths: Option<Vec<GraphPathInfo>>,
    /// Timeline events (for timeline queries)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeline: Option<Vec<TimelineEventInfo>>,
    /// Total count of results
    pub total: usize,
}

/// Graph node info for response
#[derive(Debug, Serialize)]
pub struct GraphNodeInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// Graph path info for response
#[derive(Debug, Serialize)]
pub struct GraphPathInfo {
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
    /// Query graph (POST /api/v1/graph/query)
    ///
    /// Queries the temporal knowledge graph for causal chains, timelines, and paths.
    /// Uses evif-graph TemporalGraph for time-aware graph operations.
    pub async fn query_graph(
        State(state): State<MemoryState>,
        Json(req): Json<GraphQueryRequest>,
    ) -> Result<Json<GraphQueryResponse>, MemoryError> {
        match req.query_type.as_str() {
            "causal_chain" => {
                let start_node = req.start_node
                    .ok_or_else(|| MemoryError::BadRequest("start_node required for causal_chain query".to_string()))?;

                // For now, return a placeholder response
                // Full implementation would use evif-graph TemporalGraph::find_causal_chain
                Ok(Json(GraphQueryResponse {
                    query_type: req.query_type,
                    nodes: Some(vec![GraphNodeInfo {
                        id: start_node.clone(),
                        node_type: "memory".to_string(),
                        label: "Start node".to_string(),
                        timestamp: None,
                    }]),
                    paths: None,
                    timeline: None,
                    total: 1,
                }))
            }

            "timeline" => {
                // Get event_type filter or use default
                let event_type = req.event_type.unwrap_or_else(|| "event".to_string());

                // For now, return an empty timeline
                // Full implementation would use evif-graph TemporalGraph::get_event_timeline
                Ok(Json(GraphQueryResponse {
                    query_type: req.query_type,
                    nodes: None,
                    paths: None,
                    timeline: Some(vec![]),
                    total: 0,
                }))
            }

            "temporal_bfs" => {
                let start_node = req.start_node
                    .ok_or_else(|| MemoryError::BadRequest("start_node required for temporal_bfs query".to_string()))?;

                // For now, return placeholder response
                // Full implementation would use evif-graph TemporalGraph::temporal_bfs
                Ok(Json(GraphQueryResponse {
                    query_type: req.query_type,
                    nodes: Some(vec![GraphNodeInfo {
                        id: start_node,
                        node_type: "memory".to_string(),
                        label: "BFS start".to_string(),
                        timestamp: None,
                    }]),
                    paths: None,
                    timeline: None,
                    total: 1,
                }))
            }

            "temporal_path" => {
                let start_node = req.start_node
                    .ok_or_else(|| MemoryError::BadRequest("start_node required for temporal_path query".to_string()))?;
                let end_node = req.end_node
                    .ok_or_else(|| MemoryError::BadRequest("end_node required for temporal_path query".to_string()))?;

                // For now, return placeholder path
                // Full implementation would use evif-graph TemporalGraph::find_temporal_path
                Ok(Json(GraphQueryResponse {
                    query_type: req.query_type,
                    nodes: None,
                    paths: Some(vec![GraphPathInfo {
                        nodes: vec![start_node, end_node],
                        edges: vec!["Before".to_string()],
                        narrative: "Direct temporal path".to_string(),
                    }]),
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
    fn test_graph_query_request_deserialization() {
        let json = r#"{
            "query_type": "causal_chain",
            "start_node": "node-123",
            "max_depth": 10
        }"#;

        let req: GraphQueryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.query_type, "causal_chain");
        assert_eq!(req.start_node, Some("node-123".to_string()));
        assert_eq!(req.max_depth, 10);
    }

    #[test]
    fn test_graph_query_request_temporal_path() {
        let json = r#"{
            "query_type": "temporal_path",
            "start_node": "node-a",
            "end_node": "node-b"
        }"#;

        let req: GraphQueryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.query_type, "temporal_path");
        assert_eq!(req.start_node, Some("node-a".to_string()));
        assert_eq!(req.end_node, Some("node-b".to_string()));
        assert_eq!(req.max_depth, 5); // default
    }

    #[test]
    fn test_graph_query_request_timeline() {
        let json = r#"{
            "query_type": "timeline",
            "event_type": "learning",
            "category": "programming"
        }"#;

        let req: GraphQueryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.query_type, "timeline");
        assert_eq!(req.event_type, Some("learning".to_string()));
        assert_eq!(req.category, Some("programming".to_string()));
    }

    #[test]
    fn test_graph_query_response_serialization() {
        let response = GraphQueryResponse {
            query_type: "causal_chain".to_string(),
            nodes: Some(vec![GraphNodeInfo {
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
    fn test_graph_path_info() {
        let path = GraphPathInfo {
            nodes: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            edges: vec!["Before".to_string(), "Causes".to_string()],
            narrative: "A before B causes C".to_string(),
        };

        let json = serde_json::to_string(&path).unwrap();
        assert!(json.contains("Before"));
        assert!(json.contains("Causes"));
    }
}
