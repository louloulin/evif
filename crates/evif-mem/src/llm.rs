//! LLM Client Abstraction
//!
//! Provides a trait-based abstraction for LLM clients (OpenAI, Anthropic, local models)
//! to enable memory extraction, analysis, and other LLM-powered operations.

use crate::error::{MemError, MemResult};
use crate::models::MemoryItem;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// LLM Client Trait
///
/// Abstract interface for LLM operations needed by the memory platform.
#[async_trait]
pub trait LLMClient: Send + Sync {
    /// Generate text completion
    async fn generate(&self, prompt: &str) -> MemResult<String>;

    /// Extract structured memories from text
    ///
    /// Uses LLM to extract MemoryItems from raw text content.
    /// Returns a list of extracted memory items with types, summaries, and tags.
    async fn extract_memories(&self, text: &str) -> MemResult<Vec<MemoryItem>>;

    /// Generate embedding vector for text
    ///
    /// Note: This is separate from EmbeddingClient in embedding.rs
    /// because some LLM providers (OpenAI) offer both completion and embedding APIs.
    async fn embed(&self, text: &str) -> MemResult<Vec<f32>>;

    /// Analyze a category of memories
    ///
    /// Given a list of memory contents, analyze and generate insights.
    async fn analyze_category(&self, memories: &[String]) -> MemResult<CategoryAnalysis>;

    /// Rerank memory items based on query relevance
    ///
    /// Given a query and a list of items, reorder them by relevance.
    async fn rerank(&self, query: &str, items: Vec<MemoryItem>) -> MemResult<Vec<MemoryItem>>;
}

/// Category Analysis Result
///
/// LLM-generated analysis of a memory category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryAnalysis {
    /// Category name suggestion
    pub name: String,

    /// Category description
    pub description: String,

    /// Summary of common themes
    pub themes: Vec<String>,

    /// Suggested tags
    pub tags: Vec<String>,
}

/// OpenAI Client
///
/// LLM client implementation using OpenAI API.
pub struct OpenAIClient {
    api_key: String,
    model: String,
    embedding_model: String,
    client: reqwest::Client,
    base_url: String,
}

impl OpenAIClient {
    /// Create a new OpenAI client
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "gpt-4o".to_string(),
            embedding_model: "text-embedding-3-small".to_string(),
            client: reqwest::Client::new(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(
        api_key: String,
        model: String,
        embedding_model: String,
        base_url: Option<String>,
    ) -> Self {
        Self {
            api_key,
            model,
            embedding_model,
            client: reqwest::Client::new(),
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        }
    }
}

#[async_trait]
impl LLMClient for OpenAIClient {
    async fn generate(&self, prompt: &str) -> MemResult<String> {
        #[derive(Serialize)]
        struct Request {
            model: String,
            messages: Vec<Message>,
            temperature: f32,
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
            message: MessageResponse,
        }

        #[derive(Deserialize)]
        struct MessageResponse {
            content: String,
        }

        let request = Request {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: 0.7,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| MemError::Llm(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(MemError::Llm(format!("API error {}: {}", status, body)));
        }

        let result: Response = response
            .json()
            .await
            .map_err(|e| MemError::Llm(format!("Parse error: {}", e)))?;

        result
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| MemError::Llm("No response generated".to_string()))
    }

    async fn extract_memories(&self, _text: &str) -> MemResult<Vec<MemoryItem>> {
        // TODO: Implement LLM-based memory extraction
        // For now, return empty vec (will be implemented in Task 4)
        Ok(vec![])
    }

    async fn embed(&self, text: &str) -> MemResult<Vec<f32>> {
        #[derive(Serialize)]
        struct Request {
            model: String,
            input: String,
        }

        #[derive(Deserialize)]
        struct Response {
            data: Vec<EmbeddingData>,
        }

        #[derive(Deserialize)]
        struct EmbeddingData {
            embedding: Vec<f32>,
        }

        let request = Request {
            model: self.embedding_model.clone(),
            input: text.to_string(),
        };

        let response = self
            .client
            .post(format!("{}/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| MemError::Embedding(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(MemError::Embedding(format!(
                "API error {}: {}",
                status, body
            )));
        }

        let result: Response = response
            .json()
            .await
            .map_err(|e| MemError::Embedding(format!("Parse error: {}", e)))?;

        result
            .data
            .first()
            .map(|d| d.embedding.clone())
            .ok_or_else(|| MemError::Embedding("No embedding returned".to_string()))
    }

    async fn analyze_category(&self, _memories: &[String]) -> MemResult<CategoryAnalysis> {
        // TODO: Implement category analysis
        // For now, return placeholder (will be implemented later)
        Ok(CategoryAnalysis {
            name: "uncategorized".to_string(),
            description: "Default category".to_string(),
            themes: vec![],
            tags: vec![],
        })
    }

    async fn rerank(&self, _query: &str, items: Vec<MemoryItem>) -> MemResult<Vec<MemoryItem>> {
        // TODO: Implement reranking logic
        // For now, return items as-is (will be implemented in Task 5)
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_client_creation() {
        let client = OpenAIClient::new("test-key".to_string());
        assert_eq!(client.model, "gpt-4o");
        assert_eq!(client.embedding_model, "text-embedding-3-small");
    }

    #[test]
    fn test_openai_client_custom_config() {
        let client = OpenAIClient::with_config(
            "test-key".to_string(),
            "gpt-4o-mini".to_string(),
            "text-embedding-3-large".to_string(),
            Some("https://custom.openai.com/v1".to_string()),
        );
        assert_eq!(client.model, "gpt-4o-mini");
        assert_eq!(client.embedding_model, "text-embedding-3-large");
        assert_eq!(client.base_url, "https://custom.openai.com/v1");
    }

    // Note: Integration tests with real API calls should be in tests/ directory
    // and use mock servers or environment variables for API keys
}
