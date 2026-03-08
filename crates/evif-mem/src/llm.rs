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

    /// Analyze an image and extract description
    ///
    /// Uses vision API to analyze image content and generate description + caption.
    /// Image data should be in a common format (JPEG, PNG, etc.).
    async fn analyze_image(&self, image_data: &[u8], mime_type: &str) -> MemResult<ImageAnalysis>;
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

/// Image Analysis Result
///
/// LLM-generated analysis of an image for memory extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAnalysis {
    /// Detailed description of the image content
    pub description: String,

    /// Short caption summarizing the image
    pub caption: String,
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

/// Anthropic Client
///
/// LLM client implementation using Anthropic API (Claude).
pub struct AnthropicClient {
    api_key: String,
    model: String,
    client: reqwest::Client,
    base_url: String,
}

/// Ollama Client
///
/// LLM client implementation using Ollama (local LLM server).
/// Supports both text generation and embeddings.
pub struct OllamaClient {
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

    async fn analyze_image(&self, image_data: &[u8], mime_type: &str) -> MemResult<ImageAnalysis> {
        use base64::{engine::general_purpose::STANDARD, Engine as _};

        #[derive(Serialize)]
        struct Request {
            model: String,
            messages: Vec<Message>,
            max_tokens: u32,
        }

        #[derive(Serialize)]
        struct Message {
            role: String,
            content: Vec<Content>,
        }

        #[derive(Serialize)]
        struct Content {
            #[serde(rename = "type")]
            content_type: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            text: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            image_url: Option<ImageUrl>,
        }

        #[derive(Serialize)]
        struct ImageUrl {
            url: String,
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

        // Encode image to base64
        let base64_image = STANDARD.encode(image_data);
        let data_url = format!("data:{};base64,{}", mime_type, base64_image);

        let request = Request {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: vec![
                    Content {
                        content_type: "text".to_string(),
                        text: Some("Analyze this image and provide a detailed description and a short caption. Format your response as JSON: {\"description\": \"<detailed description>\", \"caption\": \"<short caption>\"}".to_string()),
                        image_url: None,
                    },
                    Content {
                        content_type: "image_url".to_string(),
                        text: None,
                        image_url: Some(ImageUrl { url: data_url }),
                    },
                ],
            }],
            max_tokens: 1024,
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

        let content = result
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| MemError::Llm("No response generated".to_string()))?;

        // Parse JSON response
        let analysis: ImageAnalysis = serde_json::from_str(&content)
            .map_err(|e| MemError::Llm(format!("Failed to parse image analysis: {}", e)))?;

        Ok(analysis)
    }
}

impl AnthropicClient {
    /// Create a new Anthropic client
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "claude-3-5-sonnet-20241022".to_string(),
            client: reqwest::Client::new(),
            base_url: "https://api.anthropic.com/v1".to_string(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(api_key: String, model: String, base_url: Option<String>) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
            base_url: base_url.unwrap_or_else(|| "https://api.anthropic.com/v1".to_string()),
        }
    }
}

#[async_trait]
impl LLMClient for AnthropicClient {
    async fn generate(&self, prompt: &str) -> MemResult<String> {
        #[derive(Serialize)]
        struct Request {
            model: String,
            max_tokens: u32,
            messages: Vec<Message>,
        }

        #[derive(Serialize)]
        struct Message {
            role: String,
            content: String,
        }

        #[derive(Deserialize)]
        struct Response {
            content: Vec<ContentBlock>,
        }

        #[derive(Deserialize)]
        struct ContentBlock {
            text: String,
        }

        let request = Request {
            model: self.model.clone(),
            max_tokens: 4096,
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let response = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
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
            .content
            .first()
            .map(|c| c.text.clone())
            .ok_or_else(|| MemError::Llm("No response generated".to_string()))
    }

    async fn extract_memories(&self, _text: &str) -> MemResult<Vec<MemoryItem>> {
        // TODO: Implement LLM-based memory extraction
        // For now, return empty vec (will be implemented in Task 4)
        Ok(vec![])
    }

    async fn embed(&self, _text: &str) -> MemResult<Vec<f32>> {
        // Anthropic doesn't provide an embeddings API
        // Users should use OpenAI or other embedding services
        Err(MemError::Embedding(
            "Anthropic does not provide embeddings API. Use OpenAI or other embedding services.".to_string()
        ))
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

    async fn analyze_image(&self, image_data: &[u8], mime_type: &str) -> MemResult<ImageAnalysis> {
        use base64::{engine::general_purpose::STANDARD, Engine as _};

        #[derive(Serialize)]
        struct Request {
            model: String,
            max_tokens: u32,
            messages: Vec<Message>,
        }

        #[derive(Serialize)]
        struct Message {
            role: String,
            content: Vec<Content>,
        }

        #[derive(Serialize)]
        struct Content {
            #[serde(rename = "type")]
            content_type: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            text: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            source: Option<ImageSource>,
        }

        #[derive(Serialize)]
        struct ImageSource {
            #[serde(rename = "type")]
            source_type: String,
            media_type: String,
            data: String,
        }

        #[derive(Deserialize)]
        struct Response {
            content: Vec<ContentBlock>,
        }

        #[derive(Deserialize)]
        struct ContentBlock {
            text: String,
        }

        let base64_image = STANDARD.encode(image_data);

        let request = Request {
            model: self.model.clone(),
            max_tokens: 1024,
            messages: vec![Message {
                role: "user".to_string(),
                content: vec![
                    Content {
                        content_type: "text".to_string(),
                        text: Some("Analyze this image and provide a detailed description and a short caption. Format your response as JSON: {\"description\": \"<detailed description>\", \"caption\": \"<short caption>\"}".to_string()),
                        source: None,
                    },
                    Content {
                        content_type: "image".to_string(),
                        text: None,
                        source: Some(ImageSource {
                            source_type: "base64".to_string(),
                            media_type: mime_type.to_string(),
                            data: base64_image,
                        }),
                    },
                ],
            }],
        };

        let response = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
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

        let content = result
            .content
            .first()
            .map(|c| c.text.clone())
            .ok_or_else(|| MemError::Llm("No response generated".to_string()))?;

        // Parse JSON response
        let analysis: ImageAnalysis = serde_json::from_str(&content)
            .map_err(|e| MemError::Llm(format!("Failed to parse image analysis: {}", e)))?;

        Ok(analysis)
    }
}

impl OllamaClient {
    /// Create a new Ollama client with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom configuration
    pub fn with_config(
        model: String,
        embedding_model: String,
        base_url: Option<String>,
    ) -> Self {
        Self {
            model,
            embedding_model,
            client: reqwest::Client::new(),
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
        }
    }

    /// Get the configured model
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get the configured embedding model
    pub fn embedding_model(&self) -> &str {
        &self.embedding_model
    }
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self {
            model: "llama2".to_string(),
            embedding_model: "nomic-embed-text".to_string(),
            client: reqwest::Client::new(),
            base_url: "http://localhost:11434".to_string(),
        }
    }
}

#[async_trait]
impl LLMClient for OllamaClient {
    async fn generate(&self, prompt: &str) -> MemResult<String> {
        #[derive(Serialize)]
        struct Request {
            model: String,
            prompt: String,
            stream: bool,
        }

        #[derive(Deserialize)]
        struct Response {
            response: String,
        }

        let request = Request {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
        };

        let response = self
            .client
            .post(format!("{}/api/generate", self.base_url))
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

        Ok(result.response)
    }

    async fn extract_memories(&self, _text: &str) -> MemResult<Vec<MemoryItem>> {
        // TODO: Implement LLM-based memory extraction with Ollama
        // For now, return empty vec
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
            embedding: Vec<f32>,
        }

        let request = Request {
            model: self.embedding_model.clone(),
            input: text.to_string(),
        };

        let response = self
            .client
            .post(format!("{}/api/embeddings", self.base_url))
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

        Ok(result.embedding)
    }

    async fn analyze_category(&self, memories: &[String]) -> MemResult<CategoryAnalysis> {
        let prompt = format!(
            "Analyze these memories and provide:\n1. A category name\n2. A description\n3. Common themes (list)\n4. Suggested tags (list)\n\nMemories:\n{}",
            memories.join("\n- ")
        );

        let response = self.generate(&prompt).await?;

        // Parse simple key-value format
        let mut name = "Uncategorized".to_string();
        let mut description = "".to_string();
        let mut themes = vec![];
        let mut tags = vec![];

        for line in response.lines() {
            let line = line.trim();
            if line.starts_with("name:") || line.starts_with("Name:") {
                name = line.split(':').nth(1).unwrap_or("Uncategorized").trim().to_string();
            } else if line.starts_with("description:") || line.starts_with("Description:") {
                description = line.split(':').nth(1).unwrap_or("").trim().to_string();
            } else if line.starts_with("themes:") || line.starts_with("Themes:") {
                let theme_str = line.split(':').nth(1).unwrap_or("");
                themes = theme_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            } else if line.starts_with("tags:") || line.starts_with("Tags:") {
                let tag_str = line.split(':').nth(1).unwrap_or("");
                tags = tag_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }

        Ok(CategoryAnalysis {
            name,
            description,
            themes,
            tags,
        })
    }

    async fn rerank(&self, query: &str, mut items: Vec<MemoryItem>) -> MemResult<Vec<MemoryItem>> {
        // Simple reranking based on keyword matching
        // Collect as owned strings to avoid borrow issues
        let query_terms: Vec<String> = query.to_lowercase().split_whitespace().map(String::from).collect();

        for item in items.iter_mut() {
            let content_lower = item.content.to_lowercase();
            let summary_lower = item.summary.to_lowercase();
            let mut score = 0.0;

            for term in &query_terms {
                if content_lower.contains(term.as_str()) {
                    score += 1.0;
                }
                if summary_lower.contains(term.as_str()) {
                    score += 0.5;
                }
            }

            // Normalize by content length
            if !content_lower.is_empty() {
                score /= (content_lower.len() as f32 / 100.0).max(1.0);
            }
        }

        // Sort by relevance (content match > summary match)
        items.sort_by(|a, b| {
            let a_content = a.content.to_lowercase();
            let b_content = b.content.to_lowercase();
            let a_summary = a.summary.to_lowercase();
            let b_summary = b.summary.to_lowercase();

            let a_score = query_terms.iter().filter(|t| a_content.contains(t.as_str())).count() as i32
                + query_terms.iter().filter(|t| a_summary.contains(t.as_str())).count() as i32;
            let b_score = query_terms.iter().filter(|t| b_content.contains(t.as_str())).count() as i32
                + query_terms.iter().filter(|t| b_summary.contains(t.as_str())).count() as i32;

            b_score.cmp(&a_score)
        });

        Ok(items)
    }

    async fn analyze_image(&self, _image_data: &[u8], _mime_type: &str) -> MemResult<ImageAnalysis> {
        // Ollama doesn't support vision directly
        // Return a placeholder indicating this needs external processing
        Ok(ImageAnalysis {
            description: "Ollama does not support image analysis natively".to_string(),
            caption: "Image analysis unavailable".to_string(),
        })
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

    #[test]
    fn test_anthropic_client_creation() {
        let client = AnthropicClient::new("test-key".to_string());
        assert_eq!(client.model, "claude-3-5-sonnet-20241022");
    }

    #[test]
    fn test_anthropic_client_custom_config() {
        let client = AnthropicClient::with_config(
            "test-key".to_string(),
            "claude-3-opus-20240229".to_string(),
            Some("https://custom.anthropic.com/v1".to_string()),
        );
        assert_eq!(client.model, "claude-3-opus-20240229");
        assert_eq!(client.base_url, "https://custom.anthropic.com/v1");
    }

    #[test]
    fn test_ollama_client_default() {
        let client = OllamaClient::new();
        assert_eq!(client.model, "llama2");
        assert_eq!(client.embedding_model, "nomic-embed-text");
        assert_eq!(client.base_url, "http://localhost:11434");
    }

    #[test]
    fn test_ollama_client_custom_config() {
        let client = OllamaClient::with_config(
            "mistral".to_string(),
            "mxbai-embed-large".to_string(),
            Some("http://192.168.1.100:11434".to_string()),
        );
        assert_eq!(client.model, "mistral");
        assert_eq!(client.embedding_model, "mxbai-embed-large");
        assert_eq!(client.base_url, "http://192.168.1.100:11434");
    }

    #[test]
    fn test_ollama_client_model_accessors() {
        let client = OllamaClient::with_config(
            "codellama".to_string(),
            "snowflake-arctic-embed".to_string(),
            None,
        );
        assert_eq!(client.model(), "codellama");
        assert_eq!(client.embedding_model(), "snowflake-arctic-embed");
    }

    // Note: Integration tests with real API calls should be in tests/ directory
    // and use mock servers or environment variables for API keys
}
