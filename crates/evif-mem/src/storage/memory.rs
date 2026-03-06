//! In-memory storage backend

use crate::error::{MemError, MemResult};
use crate::models::{CategoryItem, MemoryCategory, MemoryItem, Resource};
use dashmap::DashMap;
use std::sync::Arc;

/// In-memory storage for memory items
pub struct MemoryStorage {
    resources: Arc<DashMap<String, Resource>>,
    items: Arc<DashMap<String, MemoryItem>>,
    categories: Arc<DashMap<String, MemoryCategory>>,
    category_items: Arc<DashMap<String, CategoryItem>>,

    // Indexes
    items_by_type: Arc<DashMap<String, Vec<String>>>,
    items_by_category: Arc<DashMap<String, Vec<String>>>,
    items_by_hash: Arc<DashMap<String, String>>, // content_hash -> item_id
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            resources: Arc::new(DashMap::new()),
            items: Arc::new(DashMap::new()),
            categories: Arc::new(DashMap::new()),
            category_items: Arc::new(DashMap::new()),
            items_by_type: Arc::new(DashMap::new()),
            items_by_category: Arc::new(DashMap::new()),
            items_by_hash: Arc::new(DashMap::new()),
        }
    }

    // Resource operations
    pub fn put_resource(&self, resource: Resource) -> MemResult<()> {
        self.resources.insert(resource.id.clone(), resource);
        Ok(())
    }

    pub fn get_resource(&self, id: &str) -> MemResult<Resource> {
        self.resources
            .get(id)
            .map(|r| r.clone())
            .ok_or_else(|| MemError::NotFound(format!("Resource not found: {}", id)))
    }

    // Memory item operations
    pub fn put_item(&self, mut item: MemoryItem) -> MemResult<()> {
        // Check for duplicates
        if let Some(ref hash) = item.content_hash {
            if let Some(existing_id) = self.items_by_hash.get(hash) {
                // Increment reinforcement count
                if let Some(mut existing) = self.items.get_mut(existing_id.value()) {
                    existing.reinforcement_count += 1;
                    existing.last_reinforced_at = Some(chrono::Utc::now());
                    return Ok(());
                }
            }
        }

        // Generate ref_id if not set
        if item.ref_id.is_none() {
            item.generate_ref_id();
        }

        // Add to index by type
        let type_key = item.memory_type.to_string();
        self.items_by_type
            .entry(type_key)
            .or_insert_with(Vec::new)
            .push(item.id.clone());

        // Add to index by hash
        if let Some(ref hash) = item.content_hash {
            self.items_by_hash.insert(hash.clone(), item.id.clone());
        }

        self.items.insert(item.id.clone(), item);
        Ok(())
    }

    pub fn get_item(&self, id: &str) -> MemResult<MemoryItem> {
        self.items
            .get(id)
            .map(|i| i.clone())
            .ok_or_else(|| MemError::NotFound(format!("Memory item not found: {}", id)))
    }

    pub fn get_items_by_type(&self, memory_type: &str) -> Vec<MemoryItem> {
        self.items_by_type
            .get(memory_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.items.get(id).map(|i| i.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_all_items(&self) -> Vec<MemoryItem> {
        self.items.iter().map(|i| i.clone()).collect()
    }

    pub fn delete_item(&self, id: &str) -> MemResult<()> {
        if self.items.remove(id).is_none() {
            return Err(MemError::NotFound(format!("Item not found: {}", id)));
        }
        Ok(())
    }

    // Category operations
    pub fn put_category(&self, mut category: MemoryCategory) -> MemResult<()> {
        category.item_count = self
            .items_by_category
            .get(&category.id)
            .map(|v| v.len() as u32)
            .unwrap_or(0);

        self.categories.insert(category.id.clone(), category);
        Ok(())
    }

    pub fn get_category(&self, id: &str) -> MemResult<MemoryCategory> {
        self.categories
            .get(id)
            .map(|c| c.clone())
            .ok_or_else(|| MemError::NotFound(format!("Category not found: {}", id)))
    }

    pub fn get_all_categories(&self) -> Vec<MemoryCategory> {
        self.categories.iter().map(|c| c.clone()).collect()
    }

    pub fn delete_category(&self, id: &str) -> MemResult<()> {
        if self.categories.remove(id).is_none() {
            return Err(MemError::NotFound(format!("Category not found: {}", id)));
        }
        Ok(())
    }

    // Category-Item relationship
    pub fn link_item_to_category(&self, item_id: &str, category_id: &str) -> MemResult<()> {
        // Verify both exist
        self.get_item(item_id)?;
        self.get_category(category_id)?;

        let relation = CategoryItem::new(item_id.to_string(), category_id.to_string());
        self.category_items
            .insert(relation.id.clone(), relation);

        // Update category item count
        self.items_by_category
            .entry(category_id.to_string())
            .or_insert_with(Vec::new);

        if let Some(mut category) = self.categories.get_mut(category_id) {
            category.item_count += 1;
        }

        Ok(())
    }

    pub fn get_items_in_category(&self, category_id: &str) -> Vec<MemoryItem> {
        self.items_by_category
            .get(category_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.items.get(id).map(|i| i.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}
