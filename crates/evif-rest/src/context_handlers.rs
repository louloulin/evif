// Context API Handlers - Semantic search and summarization endpoints

use crate::{RestError, RestResult};
use axum::{extract::State, Json};
use evif_plugins::{ContextManager, SemanticResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Request/Response types
#[derive(Debug, Deserialize)]
pub struct SemanticSearchRequest {
    pub query: String,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct SemanticSearchResponse {
    pub results: Vec<SemanticResultJson>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct SemanticResultJson {
    pub content: String,
    pub source: String,
    pub score: f32,
}

impl From<SemanticResult> for SemanticResultJson {
    fn from(result: SemanticResult) -> Self {
        Self {
            content: result.content,
            source: result.source,
            score: result.score,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SummarizeRequest {
    pub content: String,
    #[serde(default)]
    pub max_length: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct SummarizeResponse {
    pub summary: String,
}

// Context state that holds the ContextManager
#[derive(Clone)]
pub struct ContextState {
    pub context_manager: Arc<ContextManager>,
}

impl ContextState {
    pub fn new(context_manager: ContextManager) -> Self {
        Self {
            context_manager: Arc::new(context_manager),
        }
    }
}

/// POST /context/semantic_search - Semantic search across L2 knowledge base
pub async fn semantic_search(
    State(state): State<ContextState>,
    Json(req): Json<SemanticSearchRequest>,
) -> RestResult<Json<SemanticSearchResponse>> {
    let limit = req.limit.unwrap_or(5);

    let results = state
        .context_manager
        .semantic_search(&req.query, limit)
        .await
        .map_err(|e| RestError::Internal(e.to_string()))?;

    let json_results: Vec<SemanticResultJson> =
        results.into_iter().map(SemanticResultJson::from).collect();

    Ok(Json(SemanticSearchResponse {
        count: json_results.len(),
        results: json_results,
    }))
}

/// POST /context/summarize - Generate summary for given content
pub async fn summarize(
    State(state): State<ContextState>,
    Json(req): Json<SummarizeRequest>,
) -> RestResult<Json<SummarizeResponse>> {
    let max_length = req.max_length.unwrap_or(200);

    let summary = state
        .context_manager
        .generate_summary(&req.content, max_length)
        .await
        .map_err(|e| RestError::Internal(e.to_string()))?;

    Ok(Json(SummarizeResponse { summary }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use evif_plugins::ContextFsPlugin;

    fn make_context_state() -> ContextState {
        let plugin = Arc::new(ContextFsPlugin::new());
        let manager = ContextManager::new(plugin);
        ContextState::new(manager)
    }

    #[tokio::test]
    async fn test_semantic_search_request_deserialize() {
        let json = r#"{"query": "test search", "limit": 10}"#;
        let req: SemanticSearchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.query, "test search");
        assert_eq!(req.limit, Some(10));
    }

    #[tokio::test]
    async fn test_semantic_search_request_default_limit() {
        let json = r#"{"query": "test search"}"#;
        let req: SemanticSearchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.query, "test search");
        assert_eq!(req.limit, None);
    }

    #[tokio::test]
    async fn test_summarize_request_deserialize() {
        let json = r#"{"content": "test content", "max_length": 100}"#;
        let req: SummarizeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "test content");
        assert_eq!(req.max_length, Some(100));
    }

    #[tokio::test]
    async fn test_summarize_request_default_max_length() {
        let json = r#"{"content": "test content"}"#;
        let req: SummarizeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "test content");
        assert_eq!(req.max_length, None);
    }

    #[tokio::test]
    async fn test_summarize_endpoint() {
        let state = make_context_state();

        let req = SummarizeRequest {
            content: "This is a short test content.".to_string(),
            max_length: Some(100),
        };

        let result = summarize(State(state), Json(req)).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.summary, "This is a short test content.");
    }

    #[tokio::test]
    async fn test_summarize_long_content() {
        let state = make_context_state();

        let long_content = "word ".repeat(100);
        let req = SummarizeRequest {
            content: long_content,
            max_length: Some(50),
        };

        let result = summarize(State(state), Json(req)).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.summary.len() <= 60);
        assert!(response.summary.ends_with("..."));
    }
}
