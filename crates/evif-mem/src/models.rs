//! Core data models for the memory platform

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Memory type - represents different categories of memories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryType {
    /// User profile - basic info, preferences, habits
    Profile,
    /// Event memory - significant events, experiences
    Event,
    /// Knowledge memory - learned knowledge, concepts
    Knowledge,
    /// Behavior memory - behavioral patterns, habits
    Behavior,
    /// Skill memory - skills, abilities
    Skill,
    /// Tool memory - tool usage experience
    Tool,
}

impl MemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryType::Profile => "profile",
            MemoryType::Event => "event",
            MemoryType::Knowledge => "knowledge",
            MemoryType::Behavior => "behavior",
            MemoryType::Skill => "skill",
            MemoryType::Tool => "tool",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "profile" => Some(MemoryType::Profile),
            "event" => Some(MemoryType::Event),
            "knowledge" => Some(MemoryType::Knowledge),
            "behavior" => Some(MemoryType::Behavior),
            "skill" => Some(MemoryType::Skill),
            "tool" => Some(MemoryType::Tool),
            _ => None,
        }
    }
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Modality - type of input resource
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Modality {
    Conversation,
    Document,
    Image,
    Video,
    Audio,
}

impl Modality {
    pub fn as_str(&self) -> &'static str {
        match self {
            Modality::Conversation => "conversation",
            Modality::Document => "document",
            Modality::Image => "image",
            Modality::Video => "video",
            Modality::Audio => "audio",
        }
    }
}

/// Resource - raw input data (conversation, document, image, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub url: String,
    pub modality: Modality,
    pub local_path: Option<String>,
    pub caption: Option<String>,
    pub embedding_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Resource {
    pub fn new(url: String, modality: Modality) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            url,
            modality,
            local_path: None,
            caption: None,
            embedding_id: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Memory Item - extracted structured memory from resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: String,
    pub resource_id: Option<String>,
    pub memory_type: MemoryType,
    pub summary: String,
    pub content: String,
    pub embedding_id: Option<String>,
    pub happened_at: Option<DateTime<Utc>>,
    // Extended fields stored as JSON
    pub content_hash: Option<String>,
    pub reinforcement_count: u32,
    pub last_reinforced_at: Option<DateTime<Utc>>,
    pub ref_id: Option<String>,
    pub category_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MemoryItem {
    pub fn new(memory_type: MemoryType, summary: String, content: String) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            resource_id: None,
            memory_type,
            summary,
            content,
            embedding_id: None,
            happened_at: None,
            content_hash: None,
            reinforcement_count: 0,
            last_reinforced_at: None,
            ref_id: None,
            category_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Generate short reference ID for cross-referencing
    pub fn generate_ref_id(&mut self) -> String {
        let ref_id = self.id.replace('-', "")[..6].to_string();
        self.ref_id = Some(ref_id.clone());
        ref_id
    }
}

/// Memory Category - organized topic/classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCategory {
    pub id: String,
    pub name: String,
    pub description: String,
    pub embedding_id: Option<String>,
    pub summary: Option<String>,
    pub item_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MemoryCategory {
    pub fn new(name: String, description: String) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description,
            embedding_id: None,
            summary: None,
            item_count: 0,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Category-Item relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryItem {
    pub id: String,
    pub item_id: String,
    pub category_id: String,
    pub created_at: DateTime<Utc>,
}

impl CategoryItem {
    pub fn new(item_id: String, category_id: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            item_id,
            category_id,
            created_at: Utc::now(),
        }
    }
}

/// Tool call record for tool memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool_name: String,
    pub input: HashMap<String, serde_json::Value>,
    pub output: String,
    pub success: bool,
    pub time_cost_ms: u64,
    pub token_cost: Option<u32>,
    pub score: Option<f32>,
    pub call_hash: String,
    pub called_at: DateTime<Utc>,
}

/// MD File Frontmatter - YAML metadata for MD file format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MdFrontmatter {
    pub id: String,
    #[serde(rename = "type")]
    pub memory_type: String,
    pub created: String,
    pub updated: String,
    #[serde(rename = "happened_at")]
    pub happened_at: Option<String>,
    pub tags: Vec<String>,
    #[serde(rename = "embedding_id")]
    pub embedding_id: Option<String>,
    #[serde(rename = "category_id")]
    pub category_id: Option<String>,
    #[serde(rename = "content_hash")]
    pub content_hash: Option<String>,
    #[serde(rename = "reinforcement_count")]
    pub reinforcement_count: u32,
    #[serde(rename = "ref_id")]
    pub ref_id: Option<String>,
    #[serde(rename = "references")]
    pub references: Vec<String>,
}

impl MdFrontmatter {
    pub fn from_memory_item(item: &MemoryItem, tags: Vec<String>, references: Vec<String>) -> Self {
        Self {
            id: item.id.clone(),
            memory_type: item.memory_type.to_string(),
            created: item.created_at.to_rfc3339(),
            updated: item.updated_at.to_rfc3339(),
            happened_at: item.happened_at.map(|dt| dt.to_rfc3339()),
            tags,
            embedding_id: item.embedding_id.clone(),
            category_id: item.category_id.clone(),
            content_hash: item.content_hash.clone(),
            reinforcement_count: item.reinforcement_count,
            ref_id: item.ref_id.clone(),
            references,
        }
    }
}

/// Compute content hash for deduplication
pub fn compute_content_hash(summary: &str, memory_type: &MemoryType) -> String {
    use sha2::{Digest, Sha256};
    let normalized = summary.to_lowercase();
    let content = format!("{}:{}", memory_type.as_str(), normalized);
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(&hasher.finalize()[..8])
}
