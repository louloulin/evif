//! MemPlugin - EVIF Plugin Implementation
//!
//! Exposes memory platform as a filesystem for EVIF mounting.

use async_trait::async_trait;
use chrono::Utc;
use evif_core::{EvifError, EvifPlugin, EvifResult, FileInfo, PluginConfigParam, WriteFlags};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::MemError;
use crate::models::{compute_content_hash, MdFrontmatter, MemoryCategory, MemoryItem, MemoryType};
use crate::storage::memory::MemoryStorage;

/// Memory filesystem node
#[derive(Debug)]
struct MemFsNode {
    name: String,
    is_dir: bool,
    data: Vec<u8>,
    mode: u32,
    modified: chrono::DateTime<chrono::Utc>,
    children: Option<HashMap<String, Arc<RwLock<MemFsNode>>>>,
    // Memory-specific fields
    memory_id: Option<String>,
    node_type: Option<MemFsNodeType>,
}

#[derive(Debug, Clone)]
enum MemFsNodeType {
    Resource,
    Item,
    Category,
}

/// MemPlugin configuration
#[derive(Debug, Clone)]
pub struct MemPluginConfig {
    /// Default permission mode for created files
    pub default_mode: u32,
    /// Enable MD file format (YAML frontmatter + content)
    pub use_md_format: bool,
}

impl Default for MemPluginConfig {
    fn default() -> Self {
        Self {
            default_mode: 0o644,
            use_md_format: true,
        }
    }
}

/// Memory Filesystem Plugin
pub struct MemPlugin {
    root: Arc<RwLock<MemFsNode>>,
    storage: Arc<MemoryStorage>,
    config: MemPluginConfig,
}

impl MemPlugin {
    pub fn new() -> Self {
        Self {
            root: Arc::new(RwLock::new(MemFsNode::new_dir("/".to_string(), 0o755))),
            storage: Arc::new(MemoryStorage::new()),
            config: MemPluginConfig::default(),
        }
    }

    pub fn with_config(config: MemPluginConfig) -> Self {
        Self {
            root: Arc::new(RwLock::new(MemFsNode::new_dir("/".to_string(), 0o755))),
            storage: Arc::new(MemoryStorage::new()),
            config,
        }
    }

    /// Get storage backend
    pub fn storage(&self) -> &Arc<MemoryStorage> {
        &self.storage
    }

    /// Find node by path
    async fn find_node(&self, path: &str) -> EvifResult<Arc<RwLock<MemFsNode>>> {
        let clean_path = path.trim_start_matches('/');

        if clean_path.is_empty() || clean_path == "/" {
            return Ok(Arc::clone(&self.root));
        }

        let parts: Vec<&str> = clean_path.split('/').filter(|s| !s.is_empty()).collect();
        let mut current = Arc::clone(&self.root);

        for part in parts {
            let next_current = {
                let node_ref = current.read().await;

                if !node_ref.is_dir {
                    return Err(EvifError::InvalidPath("Not a directory".to_string()));
                }

                if let Some(children) = &node_ref.children {
                    if let Some(child) = children.get(part) {
                        Some(Arc::clone(child))
                    } else {
                        return Err(EvifError::NotFound(path.to_string()));
                    }
                } else {
                    return Err(EvifError::InvalidPath("Not a directory".to_string()));
                }
            };

            if let Some(next) = next_current {
                current = next;
            }
        }

        Ok(current)
    }

    /// Find parent node and basename
    async fn find_parent(&self, path: &str) -> EvifResult<(Arc<RwLock<MemFsNode>>, String)> {
        let clean_path = path.trim_start_matches('/');

        if clean_path.is_empty() || clean_path == "/" {
            return Err(EvifError::InvalidPath("Root has no parent".to_string()));
        }

        let parts: Vec<&str> = clean_path.split('/').filter(|s| !s.is_empty()).collect();

        if parts.is_empty() {
            return Err(EvifError::InvalidPath("Invalid path".to_string()));
        }

        let parent_path = if parts.len() == 1 {
            String::from("/")
        } else {
            format!("/{}", parts[..parts.len() - 1].join("/"))
        };

        let basename = parts[parts.len() - 1].to_string();
        let parent = self.find_node(&parent_path).await?;
        Ok((parent, basename))
    }

    /// Convert memory item to MD file format
    fn item_to_md(&self, item: &MemoryItem) -> String {
        let frontmatter = MdFrontmatter::from_memory_item(
            item,
            vec![], // TODO: tags
            vec![], // TODO: references
        );

        let yaml = serde_yaml::to_string(&frontmatter).unwrap_or_default();
        format!("---\n{}---\n\n{}", yaml, item.content)
    }

    /// Parse MD file to extract memory item
    fn md_to_item(&self, data: &str) -> EvifResult<MemoryItem> {
        if let Some(pos) = data.find("---\n") {
            if let Some(end) = data[pos + 4..].find("---\n") {
                let yaml = &data[pos + 4..pos + 4 + end];
                let content = data[pos + 4 + end + 4..].trim();

                if let Ok(frontmatter) = serde_yaml::from_str::<MdFrontmatter>(yaml) {
                    let memory_type = MemoryType::from_str(&frontmatter.memory_type)
                        .unwrap_or(MemoryType::Knowledge);

                    let mut item = MemoryItem::new(
                        memory_type,
                        String::new(), // summary from frontmatter
                        content.to_string(),
                    );
                    item.id = frontmatter.id;
                    item.content_hash = frontmatter.content_hash;
                    item.reinforcement_count = frontmatter.reinforcement_count;
                    item.ref_id = frontmatter.ref_id;
                    item.category_id = frontmatter.category_id;

                    return Ok(item);
                }
            }
        }

        // Fallback: treat entire content as knowledge item
        Ok(MemoryItem::new(
            MemoryType::Knowledge,
            String::new(),
            data.to_string(),
        ))
    }

    /// Sync memory storage to filesystem
    async fn sync_from_storage(&self) -> EvifResult<()> {
        // Build new root structure first, then replace
        let mut new_children: HashMap<String, Arc<RwLock<MemFsNode>>> = HashMap::new();

        // Sync categories
        let categories = self.storage.get_all_categories();
        for category in categories {
            let node = MemFsNode {
                name: category.name.clone(),
                is_dir: true,
                data: Vec::new(),
                mode: 0o755,
                modified: category.updated_at,
                children: Some(HashMap::new()),
                memory_id: Some(category.id.clone()),
                node_type: Some(MemFsNodeType::Category),
            };

            new_children.insert(category.name.clone(), Arc::new(RwLock::new(node)));
        }

        // Sync items by type
        for mt in ["profile", "event", "knowledge", "behavior", "skill", "tool"] {
            let items = self.storage.get_items_by_type(mt);

            let mut type_children: HashMap<String, Arc<RwLock<MemFsNode>>> = HashMap::new();

            // Add items under type directory
            for item in items {
                let content = if self.config.use_md_format {
                    self.item_to_md(&item)
                } else {
                    item.content.clone()
                };

                let node = MemFsNode {
                    name: format!(
                        "{}.md",
                        item.id.replace('-', "").chars().take(8).collect::<String>()
                    ),
                    is_dir: false,
                    data: content.into_bytes(),
                    mode: self.config.default_mode,
                    modified: item.updated_at,
                    children: None,
                    memory_id: Some(item.id.clone()),
                    node_type: Some(MemFsNodeType::Item),
                };

                type_children.insert(node.name.clone(), Arc::new(RwLock::new(node)));
            }

            let type_dir = MemFsNode {
                name: mt.to_string(),
                is_dir: true,
                data: Vec::new(),
                mode: 0o755,
                modified: Utc::now(),
                children: Some(type_children),
                memory_id: None,
                node_type: None,
            };

            new_children.insert(mt.to_string(), Arc::new(RwLock::new(type_dir)));
        }

        // Replace root children atomically
        let mut root = self.root.write().await;
        root.children = Some(new_children);

        Ok(())
    }
}

impl Default for MemPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl MemFsNode {
    fn new_file(name: String, mode: u32) -> Self {
        Self {
            name,
            is_dir: false,
            data: Vec::new(),
            mode,
            modified: Utc::now(),
            children: None,
            memory_id: None,
            node_type: None,
        }
    }

    fn new_dir(name: String, mode: u32) -> Self {
        Self {
            name,
            is_dir: true,
            data: Vec::new(),
            mode,
            modified: Utc::now(),
            children: Some(HashMap::new()),
            memory_id: None,
            node_type: None,
        }
    }
}

#[async_trait]
impl EvifPlugin for MemPlugin {
    fn name(&self) -> &str {
        "memfs"
    }

    fn get_readme(&self) -> String {
        r#"# MemFS (Memory Plugin)

Memory filesystem plugin - exposes memory platform as a filesystem.

## Configuration

- `use_md_format`: Use MD file format with YAML frontmatter (default: true)
- `default_mode`: Default permission mode for created files (default: 0o644)

## Path Structure

```
/                       # Root
├── profile/           # Profile memories
├── event/             # Event memories
├── knowledge/         # Knowledge memories
├── behavior/          # Behavior memories
├── skill/             # Skill memories
├── tool/              # Tool memories
└── {category}/        # Category directories
```

## Usage

- Mount at `/mem` to access memory as filesystem
- Each memory type has its own directory
- MD files contain YAML frontmatter + content
"#
        .to_string()
    }

    fn get_config_params(&self) -> Vec<PluginConfigParam> {
        vec![
            PluginConfigParam {
                name: "use_md_format".to_string(),
                param_type: "bool".to_string(),
                required: false,
                default: Some("true".to_string()),
                description: Some("Use MD file format with YAML frontmatter".to_string()),
            },
            PluginConfigParam {
                name: "default_mode".to_string(),
                param_type: "int".to_string(),
                required: false,
                default: Some("420".to_string()),
                description: Some("Default permission mode for created files (octal)".to_string()),
            },
        ]
    }

    async fn validate(&self, config: Option<&serde_json::Value>) -> EvifResult<()> {
        if let Some(cfg) = config {
            if let Some(use_md) = cfg.get("use_md_format").and_then(|v| v.as_bool()) {
                let mut config = self.config.clone();
                config.use_md_format = use_md;
            }
        }
        Ok(())
    }

    async fn create(&self, path: &str, perm: u32) -> EvifResult<()> {
        let (parent, basename) = self.find_parent(path).await?;
        let mut parent_node = parent.write().await;

        if !parent_node.is_dir {
            return Err(EvifError::InvalidPath(
                "Parent is not a directory".to_string(),
            ));
        }

        if let Some(children) = &mut parent_node.children {
            if children.contains_key(&basename) {
                return Err(EvifError::InvalidPath("File already exists".to_string()));
            }

            children.insert(
                basename.clone(),
                Arc::new(RwLock::new(MemFsNode::new_file(basename, perm))),
            );

            Ok(())
        } else {
            Err(EvifError::InvalidPath(
                "Parent is not a directory".to_string(),
            ))
        }
    }

    async fn mkdir(&self, path: &str, perm: u32) -> EvifResult<()> {
        let (parent, basename) = self.find_parent(path).await?;
        let mut parent_node = parent.write().await;

        if !parent_node.is_dir {
            return Err(EvifError::InvalidPath(
                "Parent is not a directory".to_string(),
            ));
        }

        if let Some(children) = &mut parent_node.children {
            if children.contains_key(&basename) {
                return Err(EvifError::InvalidPath(
                    "Directory already exists".to_string(),
                ));
            }

            children.insert(
                basename.clone(),
                Arc::new(RwLock::new(MemFsNode::new_dir(basename, perm))),
            );

            Ok(())
        } else {
            Err(EvifError::InvalidPath(
                "Parent is not a directory".to_string(),
            ))
        }
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        let node = self.find_node(path).await?;
        let node_ref = node.read().await;

        if node_ref.is_dir {
            return Err(EvifError::InvalidPath("Is a directory".to_string()));
        }

        Ok(node_ref.data.clone())
    }

    async fn write(
        &self,
        path: &str,
        data: Vec<u8>,
        _offset: i64,
        _flags: WriteFlags,
    ) -> EvifResult<u64> {
        let node = self.find_node(path).await?;
        let mut node_ref = node.write().await;

        if node_ref.is_dir {
            return Err(EvifError::InvalidPath("Is a directory".to_string()));
        }

        // If using MD format and path ends with .md, try to parse as memory item
        if self.config.use_md_format && path.ends_with(".md") {
            if let Ok(content) = String::from_utf8(data.clone()) {
                if let Ok(item) = self.md_to_item(&content) {
                    // Store in memory backend
                    let mut item = item;
                    item.content_hash =
                        Some(compute_content_hash(&item.content, &item.memory_type));

                    if let Err(e) = self.storage.put_item(item) {
                        tracing::warn!("Failed to store memory item: {}", e);
                    }
                }
            }
        }

        node_ref.data = data;
        node_ref.modified = Utc::now();
        Ok(node_ref.data.len() as u64)
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let node = self.find_node(path).await?;
        let node_ref = node.read().await;

        if !node_ref.is_dir {
            return Err(EvifError::InvalidPath("Not a directory".to_string()));
        }

        let mut entries = Vec::new();

        if let Some(children) = &node_ref.children {
            for (name, child) in children.iter() {
                let child_ref = child.read().await;

                entries.push(FileInfo {
                    name: name.clone(),
                    size: child_ref.data.len() as u64,
                    mode: child_ref.mode,
                    modified: child_ref.modified,
                    is_dir: child_ref.is_dir,
                });
            }
        }

        Ok(entries)
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let node = self.find_node(path).await?;
        let node_ref = node.read().await;

        Ok(FileInfo {
            name: node_ref.name.clone(),
            size: node_ref.data.len() as u64,
            mode: node_ref.mode,
            modified: node_ref.modified,
            is_dir: node_ref.is_dir,
        })
    }

    async fn remove(&self, path: &str) -> EvifResult<()> {
        let (parent, basename) = self.find_parent(path).await?;

        let mut parent_node = parent.write().await;

        if !parent_node.is_dir {
            return Err(EvifError::InvalidPath(
                "Parent is not a directory".to_string(),
            ));
        }

        if let Some(children) = &mut parent_node.children {
            if children.remove(&basename).is_none() {
                return Err(EvifError::NotFound(path.to_string()));
            }
            Ok(())
        } else {
            Err(EvifError::InvalidPath(
                "Parent is not a directory".to_string(),
            ))
        }
    }

    async fn rename(&self, old_path: &str, new_path: &str) -> EvifResult<()> {
        let (old_parent, old_basename) = self.find_parent(old_path).await?;
        let (new_parent, new_basename) = self.find_parent(new_path).await?;

        let old_node = {
            let mut old_parent_node = old_parent.write().await;

            if !old_parent_node.is_dir {
                return Err(EvifError::InvalidPath(
                    "Parent is not a directory".to_string(),
                ));
            }

            if let Some(children) = &mut old_parent_node.children {
                children
                    .remove(&old_basename)
                    .ok_or_else(|| EvifError::NotFound(old_path.to_string()))?
            } else {
                return Err(EvifError::InvalidPath(
                    "Parent is not a directory".to_string(),
                ));
            }
        };

        let mut new_parent_node = new_parent.write().await;

        if !new_parent_node.is_dir {
            return Err(EvifError::InvalidPath(
                "Parent is not a directory".to_string(),
            ));
        }

        if let Some(children) = &mut new_parent_node.children {
            if children.contains_key(&new_basename) {
                let mut old_parent_node = old_parent.write().await;
                if let Some(old_children) = &mut old_parent_node.children {
                    old_children.insert(old_basename, old_node);
                }
                return Err(EvifError::InvalidPath("Target already exists".to_string()));
            }

            children.insert(new_basename, old_node);
            Ok(())
        } else {
            Err(EvifError::InvalidPath(
                "Parent is not a directory".to_string(),
            ))
        }
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        let clean_path = path.trim_start_matches('/');

        if clean_path.is_empty() || clean_path == "/" {
            return Err(EvifError::InvalidPath("Cannot remove root".to_string()));
        }

        let node = self.find_node(path).await?;

        let node_ref = node.read().await;
        if let Some(children) = &node_ref.children {
            let child_names: Vec<String> = children.keys().cloned().collect();
            drop(node_ref);

            for child_name in child_names {
                let child_path = format!("{}/{}", path.trim_end_matches('/'), child_name);
                self.remove_all(&child_path).await?;
            }
        }

        let (parent, basename) = self.find_parent(path).await?;
        let mut parent_ref = parent.write().await;
        if let Some(parent_children) = &mut parent_ref.children {
            parent_children.remove(&basename);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memfs_basic() {
        let fs = MemPlugin::new();

        // Create directory
        fs.mkdir("/test", 0o755).await.unwrap();

        // Create file
        fs.create("/test/file1.md", 0o644).await.unwrap();
        fs.write(
            "/test/file1.md",
            b"# Hello\n\nContent".to_vec(),
            0,
            WriteFlags::CREATE,
        )
        .await
        .unwrap();

        // Read file
        let data = fs.read("/test/file1.md", 0, 100).await.unwrap();
        assert_eq!(data, b"# Hello\n\nContent");

        // List directory
        let entries = fs.readdir("/test").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "file1.md");

        // File info
        let info = fs.stat("/test/file1.md").await.unwrap();
        assert_eq!(info.size, 16);
        assert!(!info.is_dir);
    }

    #[tokio::test]
    async fn test_memfs_delete() {
        let fs = MemPlugin::new();

        fs.mkdir("/test", 0o755).await.unwrap();
        fs.create("/test/file.txt", 0o644).await.unwrap();
        fs.write(
            "/test/file.txt",
            b"test data".to_vec(),
            0,
            WriteFlags::CREATE,
        )
        .await
        .unwrap();

        // Delete file
        fs.remove("/test/file.txt").await.unwrap();

        // Verify deleted
        let entries = fs.readdir("/test").await.unwrap();
        assert_eq!(entries.len(), 0);

        // Delete empty directory
        fs.remove("/test").await.unwrap();
    }

    #[tokio::test]
    async fn test_md_format() {
        let fs = MemPlugin::new();

        // Create test directory first
        fs.mkdir("/test", 0o755).await.unwrap();

        // Write MD content
        let md_content = r#"---
id: test123
type: knowledge
created: 2026-01-01T00:00:00Z
updated: 2026-01-01T00:00:00Z
tags: []
embedding_id: null
category_id: null
content_hash: null
reinforcement_count: 0
ref_id: null
references: []
---

This is the memory content."#;

        fs.create("/test/item.md", 0o644).await.unwrap();
        fs.write(
            "/test/item.md",
            md_content.as_bytes().to_vec(),
            0,
            WriteFlags::CREATE,
        )
        .await
        .unwrap();
    }
}
