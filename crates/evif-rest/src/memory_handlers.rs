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
use std::sync::Arc;

use evif_mem::models::{MemoryItem, MemoryType};
use evif_mem::storage::memory::MemoryStorage;

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

/// Graph query request
#[derive(Debug, Deserialize)]
pub struct GraphQueryRequest {
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

fn graph_timestamp(item: &MemoryItem) -> DateTime<Utc> {
    item.happened_at.unwrap_or(item.created_at)
}

fn graph_node_type(item: &MemoryItem) -> &'static str {
    match item.memory_type {
        MemoryType::Event => "event",
        _ => "memory",
    }
}

fn to_graph_node(item: &MemoryItem) -> GraphNodeInfo {
    GraphNodeInfo {
        id: item.id.clone(),
        node_type: graph_node_type(item).to_string(),
        label: item.summary.clone(),
        timestamp: Some(graph_timestamp(item).to_rfc3339()),
    }
}

fn parse_graph_time(
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

fn filtered_graph_memories(
    state: &MemoryState,
    req: &GraphQueryRequest,
) -> Result<Vec<MemoryItem>, MemoryError> {
    let start_time = parse_graph_time("start_time", req.start_time.as_deref())?;
    let end_time = parse_graph_time("end_time", req.end_time.as_deref())?;
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
            let timestamp = graph_timestamp(item);
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
        graph_timestamp(left)
            .cmp(&graph_timestamp(right))
            .then_with(|| left.id.cmp(&right.id))
    });

    Ok(items)
}

fn find_graph_node_index(items: &[MemoryItem], node_id: &str) -> Result<usize, MemoryError> {
    items
        .iter()
        .position(|item| item.id == node_id)
        .ok_or_else(|| MemoryError::NotFound(format!("Graph node '{}' not found", node_id)))
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
    /// Projects a temporal graph from stored memories for timeline and traversal queries.
    pub async fn query_graph(
        State(state): State<MemoryState>,
        Json(req): Json<GraphQueryRequest>,
    ) -> Result<Json<GraphQueryResponse>, MemoryError> {
        match req.query_type.as_str() {
            "causal_chain" => {
                let start_node = req.start_node.as_deref().ok_or_else(|| {
                    MemoryError::BadRequest("start_node required for causal_chain query".to_string())
                })?;
                let items = filtered_graph_memories(&state, &req)?;
                let start_index = find_graph_node_index(&items, start_node)?;
                let chain_start = start_index.saturating_sub(req.max_depth);
                let nodes = items[chain_start..=start_index]
                    .iter()
                    .map(to_graph_node)
                    .collect::<Vec<_>>();
                let total = nodes.len();

                Ok(Json(GraphQueryResponse {
                    query_type: req.query_type,
                    nodes: Some(nodes),
                    paths: None,
                    timeline: None,
                    total,
                }))
            }

            "timeline" => {
                let mut items = filtered_graph_memories(&state, &req)?;

                if let Some(start_node) = req.start_node.as_deref() {
                    let start_index = find_graph_node_index(&items, start_node)?;
                    let end_index = std::cmp::min(items.len(), start_index + req.max_depth + 1);
                    items = items[start_index..end_index].to_vec();
                }

                let nodes = items.iter().map(to_graph_node).collect::<Vec<_>>();
                let timeline = items
                    .iter()
                    .map(|item| TimelineEventInfo {
                        node_id: item.id.clone(),
                        timestamp: graph_timestamp(item).to_rfc3339(),
                        event_type: item.memory_type.as_str().to_string(),
                    })
                    .collect::<Vec<_>>();
                let total = timeline.len();

                Ok(Json(GraphQueryResponse {
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
                let items = filtered_graph_memories(&state, &req)?;
                let start_index = find_graph_node_index(&items, start_node)?;
                let end_index = std::cmp::min(items.len(), start_index + req.max_depth + 1);
                let nodes = items[start_index..end_index]
                    .iter()
                    .map(to_graph_node)
                    .collect::<Vec<_>>();
                let total = nodes.len();

                Ok(Json(GraphQueryResponse {
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
                let items = filtered_graph_memories(&state, &req)?;
                let start_index = find_graph_node_index(&items, start_node)?;
                let end_index = find_graph_node_index(&items, end_node)?;

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
                let paths = vec![GraphPathInfo {
                    nodes,
                    edges,
                    narrative,
                }];

                Ok(Json(GraphQueryResponse {
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
    fn test_graph_query_request_accepts_legacy_node_id_alias() {
        let json = r#"{
            "query_type": "temporal_bfs",
            "node_id": "node-legacy"
        }"#;

        let req: GraphQueryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.query_type, "temporal_bfs");
        assert_eq!(req.start_node, Some("node-legacy".to_string()));
        assert_eq!(req.max_depth, 5);
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
