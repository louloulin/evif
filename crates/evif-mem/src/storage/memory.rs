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

        // Update category item count and add to index
        let mut ids = self.items_by_category
            .entry(category_id.to_string())
            .or_insert_with(Vec::new);

        // Only add if not already present
        if !ids.contains(&item_id.to_string()) {
            ids.push(item_id.to_string());
        }

        if let Some(mut category) = self.categories.get_mut(category_id) {
            category.item_count = ids.len() as u32;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MemoryType, Modality, compute_content_hash};

    #[test]
    fn test_put_and_get_resource() {
        let storage = MemoryStorage::new();
        let resource = Resource::new("http://example.com".to_string(), Modality::Conversation);

        storage.put_resource(resource.clone()).unwrap();
        let retrieved = storage.get_resource(&resource.id).unwrap();

        assert_eq!(retrieved.id, resource.id);
        assert_eq!(retrieved.url, "http://example.com");
    }

    #[test]
    fn test_put_and_get_item() {
        let storage = MemoryStorage::new();
        let item = MemoryItem::new(
            MemoryType::Profile,
            "User likes coffee".to_string(),
            "Detailed content about coffee preference".to_string(),
        );

        storage.put_item(item.clone()).unwrap();
        let retrieved = storage.get_item(&item.id).unwrap();

        assert_eq!(retrieved.id, item.id);
        assert_eq!(retrieved.summary, "User likes coffee");
    }

    #[test]
    fn test_get_items_by_type() {
        let storage = MemoryStorage::new();

        let item1 = MemoryItem::new(MemoryType::Profile, "Profile 1".to_string(), "Content 1".to_string());
        let item2 = MemoryItem::new(MemoryType::Profile, "Profile 2".to_string(), "Content 2".to_string());
        let item3 = MemoryItem::new(MemoryType::Event, "Event 1".to_string(), "Content 3".to_string());

        storage.put_item(item1).unwrap();
        storage.put_item(item2).unwrap();
        storage.put_item(item3).unwrap();

        let profiles = storage.get_items_by_type("profile");
        assert_eq!(profiles.len(), 2);

        let events = storage.get_items_by_type("event");
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_deduplication_by_hash() {
        let storage = MemoryStorage::new();

        let mut item1 = MemoryItem::new(MemoryType::Profile, "Same summary".to_string(), "Content".to_string());
        item1.content_hash = Some(compute_content_hash("Same summary", &MemoryType::Profile));

        let mut item2 = MemoryItem::new(MemoryType::Profile, "Same summary".to_string(), "Content".to_string());
        item2.content_hash = Some(compute_content_hash("Same summary", &MemoryType::Profile));

        storage.put_item(item1.clone()).unwrap();
        storage.put_item(item2).unwrap(); // This should trigger deduplication

        // Check reinforcement count increased
        let retrieved = storage.get_item(&item1.id).unwrap();
        assert_eq!(retrieved.reinforcement_count, 1);
    }

    #[test]
    fn test_category_operations() {
        let storage = MemoryStorage::new();

        let category = MemoryCategory::new("Programming".to_string(), "Related to programming".to_string());
        storage.put_category(category.clone()).unwrap();

        let retrieved = storage.get_category(&category.id).unwrap();
        assert_eq!(retrieved.name, "Programming");

        let all_categories = storage.get_all_categories();
        assert_eq!(all_categories.len(), 1);
    }

    #[test]
    fn test_link_item_to_category() {
        let storage = MemoryStorage::new();

        let item = MemoryItem::new(MemoryType::Skill, "Rust programming".to_string(), "Content".to_string());
        let category = MemoryCategory::new("Programming".to_string(), "Programming skills".to_string());

        storage.put_item(item.clone()).unwrap();
        storage.put_category(category.clone()).unwrap();
        storage.link_item_to_category(&item.id, &category.id).unwrap();

        let items_in_category = storage.get_items_in_category(&category.id);
        assert_eq!(items_in_category.len(), 1);
    }

    #[test]
    fn test_ref_id_generation() {
        let mut item = MemoryItem::new(MemoryType::Profile, "Test".to_string(), "Content".to_string());
        let ref_id = item.generate_ref_id();

        assert!(!ref_id.is_empty());
        assert_eq!(ref_id.len(), 6);
        assert_eq!(item.ref_id, Some(ref_id));
    }
}
