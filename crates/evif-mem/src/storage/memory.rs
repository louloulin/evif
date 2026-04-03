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
    // User-scoped indexes
    items_by_user: Arc<DashMap<String, Vec<String>>>, // user_id -> item_ids
    resources_by_user: Arc<DashMap<String, Vec<String>>>, // user_id -> resource_ids
    categories_by_user: Arc<DashMap<String, Vec<String>>>, // user_id -> category_ids
    // Tenant-scoped indexes
    items_by_tenant: Arc<DashMap<String, Vec<String>>>, // tenant_id -> item_ids
    resources_by_tenant: Arc<DashMap<String, Vec<String>>>, // tenant_id -> resource_ids
    categories_by_tenant: Arc<DashMap<String, Vec<String>>>, // tenant_id -> category_ids
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
            // User-scoped indexes
            items_by_user: Arc::new(DashMap::new()),
            resources_by_user: Arc::new(DashMap::new()),
            categories_by_user: Arc::new(DashMap::new()),
            // Tenant-scoped indexes
            items_by_tenant: Arc::new(DashMap::new()),
            resources_by_tenant: Arc::new(DashMap::new()),
            categories_by_tenant: Arc::new(DashMap::new()),
        }
    }

    // Resource operations
    pub fn put_resource(&self, resource: Resource) -> MemResult<()> {
        // Index by user if user_id is set
        if let Some(ref user_id) = resource.user_id {
            self.resources_by_user
                .entry(user_id.clone())
                .or_default()
                .push(resource.id.clone());
        }

        // Index by tenant if tenant_id is set
        if let Some(ref tenant_id) = resource.tenant_id {
            self.resources_by_tenant
                .entry(tenant_id.clone())
                .or_default()
                .push(resource.id.clone());
        }

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
            .or_default()
            .push(item.id.clone());

        // Add to index by hash
        if let Some(ref hash) = item.content_hash {
            self.items_by_hash.insert(hash.clone(), item.id.clone());
        }

        // Index by user if user_id is set
        if let Some(ref user_id) = item.user_id {
            self.items_by_user
                .entry(user_id.clone())
                .or_default()
                .push(item.id.clone());
        }

        // Index by tenant if tenant_id is set
        if let Some(ref tenant_id) = item.tenant_id {
            self.items_by_tenant
                .entry(tenant_id.clone())
                .or_default()
                .push(item.id.clone());
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

    /// Get memory items by content hash (for deduplication check)
    pub fn get_items_by_hash(&self, hash: &str) -> MemResult<Vec<MemoryItem>> {
        let mut results = Vec::new();
        // Search through all items to find matching hash
        // This is O(n) but acceptable for in-memory storage
        // For production, consider adding a reverse index
        for item in self.items.iter() {
            if item.content_hash.as_deref() == Some(hash) {
                results.push(item.clone());
            }
        }
        Ok(results)
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

    /// Get all items for a specific user (user-scoped)
    pub fn get_items_by_user(&self, user_id: &str) -> Vec<MemoryItem> {
        self.items_by_user
            .get(user_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.items.get(id).map(|i| i.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all resources for a specific user (user-scoped)
    pub fn get_resources_by_user(&self, user_id: &str) -> Vec<Resource> {
        self.resources_by_user
            .get(user_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.resources.get(id).map(|r| r.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all categories for a specific user (user-scoped)
    pub fn get_categories_by_user(&self, user_id: &str) -> Vec<MemoryCategory> {
        self.categories_by_user
            .get(user_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.categories.get(id).map(|c| c.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if an item belongs to a user
    pub fn item_belongs_to_user(&self, item_id: &str, user_id: &str) -> bool {
        if let Some(item) = self.items.get(item_id) {
            if let Some(ref item_user_id) = item.user_id {
                return item_user_id == user_id;
            }
        }
        false
    }

    /// Check if a resource belongs to a user
    pub fn resource_belongs_to_user(&self, resource_id: &str, user_id: &str) -> bool {
        if let Some(resource) = self.resources.get(resource_id) {
            if let Some(ref resource_user_id) = resource.user_id {
                return resource_user_id == user_id;
            }
        }
        false
    }

    // ============ Tenant-scoped operations ============

    /// Get all items for a specific tenant (tenant-scoped)
    pub fn get_items_by_tenant(&self, tenant_id: &str) -> Vec<MemoryItem> {
        self.items_by_tenant
            .get(tenant_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.items.get(id).map(|i| i.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all resources for a specific tenant (tenant-scoped)
    pub fn get_resources_by_tenant(&self, tenant_id: &str) -> Vec<Resource> {
        self.resources_by_tenant
            .get(tenant_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.resources.get(id).map(|r| r.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all categories for a specific tenant (tenant-scoped)
    pub fn get_categories_by_tenant(&self, tenant_id: &str) -> Vec<MemoryCategory> {
        self.categories_by_tenant
            .get(tenant_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.categories.get(id).map(|c| c.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if an item belongs to a tenant
    pub fn item_belongs_to_tenant(&self, item_id: &str, tenant_id: &str) -> bool {
        if let Some(item) = self.items.get(item_id) {
            if let Some(ref item_tenant_id) = item.tenant_id {
                return item_tenant_id == tenant_id;
            }
        }
        false
    }

    /// Check if a resource belongs to a tenant
    pub fn resource_belongs_to_tenant(&self, resource_id: &str, tenant_id: &str) -> bool {
        if let Some(resource) = self.resources.get(resource_id) {
            if let Some(ref resource_tenant_id) = resource.tenant_id {
                return resource_tenant_id == tenant_id;
            }
        }
        false
    }

    /// Get all tenants (for admin purposes)
    pub fn get_all_tenants(&self) -> Vec<String> {
        self.items_by_tenant
            .iter()
            .map(|k| k.key().clone())
            .collect()
    }

    /// Get item count for a tenant
    pub fn item_count_by_tenant(&self, tenant_id: &str) -> usize {
        self.items_by_tenant
            .get(tenant_id)
            .map(|ids| ids.len())
            .unwrap_or(0)
    }

    /// Get resource count for a tenant
    pub fn resource_count_by_tenant(&self, tenant_id: &str) -> usize {
        self.resources_by_tenant
            .get(tenant_id)
            .map(|ids| ids.len())
            .unwrap_or(0)
    }

    pub fn item_count(&self) -> MemResult<usize> {
        Ok(self.items.len())
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

        // Index by user if user_id is set
        if let Some(ref user_id) = category.user_id {
            self.categories_by_user
                .entry(user_id.clone())
                .or_default()
                .push(category.id.clone());
        }

        // Index by tenant if tenant_id is set
        if let Some(ref tenant_id) = category.tenant_id {
            self.categories_by_tenant
                .entry(tenant_id.clone())
                .or_default()
                .push(category.id.clone());
        }

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
        self.category_items.insert(relation.id.clone(), relation);

        // Update category item count and add to index
        let mut ids = self
            .items_by_category
            .entry(category_id.to_string())
            .or_default();

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
    use crate::models::{compute_content_hash, MemoryType, Modality};

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

        let item1 = MemoryItem::new(
            MemoryType::Profile,
            "Profile 1".to_string(),
            "Content 1".to_string(),
        );
        let item2 = MemoryItem::new(
            MemoryType::Profile,
            "Profile 2".to_string(),
            "Content 2".to_string(),
        );
        let item3 = MemoryItem::new(
            MemoryType::Event,
            "Event 1".to_string(),
            "Content 3".to_string(),
        );

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

        let mut item1 = MemoryItem::new(
            MemoryType::Profile,
            "Same summary".to_string(),
            "Content".to_string(),
        );
        item1.content_hash = Some(compute_content_hash("Same summary", &MemoryType::Profile));

        let mut item2 = MemoryItem::new(
            MemoryType::Profile,
            "Same summary".to_string(),
            "Content".to_string(),
        );
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

        let category = MemoryCategory::new(
            "Programming".to_string(),
            "Related to programming".to_string(),
        );
        storage.put_category(category.clone()).unwrap();

        let retrieved = storage.get_category(&category.id).unwrap();
        assert_eq!(retrieved.name, "Programming");

        let all_categories = storage.get_all_categories();
        assert_eq!(all_categories.len(), 1);
    }

    #[test]
    fn test_link_item_to_category() {
        let storage = MemoryStorage::new();

        let item = MemoryItem::new(
            MemoryType::Skill,
            "Rust programming".to_string(),
            "Content".to_string(),
        );
        let category =
            MemoryCategory::new("Programming".to_string(), "Programming skills".to_string());

        storage.put_item(item.clone()).unwrap();
        storage.put_category(category.clone()).unwrap();
        storage
            .link_item_to_category(&item.id, &category.id)
            .unwrap();

        let items_in_category = storage.get_items_in_category(&category.id);
        assert_eq!(items_in_category.len(), 1);
    }

    #[test]
    fn test_ref_id_generation() {
        let mut item = MemoryItem::new(
            MemoryType::Profile,
            "Test".to_string(),
            "Content".to_string(),
        );
        let ref_id = item.generate_ref_id();

        assert!(!ref_id.is_empty());
        assert_eq!(ref_id.len(), 6);
        assert_eq!(item.ref_id, Some(ref_id));
    }

    // Multi-user tests
    #[test]
    fn test_user_scoped_items() {
        let storage = MemoryStorage::new();

        // Create items for different users
        let item1 = MemoryItem::new(
            MemoryType::Profile,
            "User1 profile".to_string(),
            "Content".to_string(),
        )
        .with_user_context("user1".to_string(), None);

        let item2 = MemoryItem::new(
            MemoryType::Profile,
            "User2 profile".to_string(),
            "Content".to_string(),
        )
        .with_user_context("user2".to_string(), None);

        let item3 = MemoryItem::new(
            MemoryType::Event,
            "User1 event".to_string(),
            "Content".to_string(),
        )
        .with_user_context("user1".to_string(), None);

        storage.put_item(item1.clone()).unwrap();
        storage.put_item(item2.clone()).unwrap();
        storage.put_item(item3.clone()).unwrap();

        // Get items for user1
        let user1_items = storage.get_items_by_user("user1");
        assert_eq!(user1_items.len(), 2);

        // Get items for user2
        let user2_items = storage.get_items_by_user("user2");
        assert_eq!(user2_items.len(), 1);
    }

    #[test]
    fn test_user_scoped_resources() {
        let storage = MemoryStorage::new();

        let resource1 = Resource::new("http://example.com/1".to_string(), Modality::Document)
            .with_user_context("user1".to_string(), None);

        let resource2 = Resource::new("http://example.com/2".to_string(), Modality::Document)
            .with_user_context("user2".to_string(), None);

        storage.put_resource(resource1.clone()).unwrap();
        storage.put_resource(resource2.clone()).unwrap();

        let user1_resources = storage.get_resources_by_user("user1");
        assert_eq!(user1_resources.len(), 1);

        let user2_resources = storage.get_resources_by_user("user2");
        assert_eq!(user2_resources.len(), 1);
    }

    #[test]
    fn test_user_scoped_categories() {
        let storage = MemoryStorage::new();

        let cat1 = MemoryCategory::new("User1 category".to_string(), "Description".to_string())
            .with_user_context("user1".to_string(), None);

        let cat2 = MemoryCategory::new("User2 category".to_string(), "Description".to_string())
            .with_user_context("user2".to_string(), None);

        storage.put_category(cat1.clone()).unwrap();
        storage.put_category(cat2.clone()).unwrap();

        let user1_categories = storage.get_categories_by_user("user1");
        assert_eq!(user1_categories.len(), 1);

        let user2_categories = storage.get_categories_by_user("user2");
        assert_eq!(user2_categories.len(), 1);
    }

    #[test]
    fn test_item_belongs_to_user() {
        let storage = MemoryStorage::new();

        let item = MemoryItem::new(
            MemoryType::Profile,
            "Test".to_string(),
            "Content".to_string(),
        )
        .with_user_context("user1".to_string(), None);

        storage.put_item(item.clone()).unwrap();

        assert!(storage.item_belongs_to_user(&item.id, "user1"));
        assert!(!storage.item_belongs_to_user(&item.id, "user2"));
    }

    #[test]
    fn test_tenant_isolation() {
        let storage = MemoryStorage::new();

        // Create items with same user_id but different tenant_id
        let item1 = MemoryItem::new(
            MemoryType::Profile,
            "Tenant1 profile".to_string(),
            "Content".to_string(),
        )
        .with_user_context("user1".to_string(), Some("tenant1".to_string()));

        let item2 = MemoryItem::new(
            MemoryType::Profile,
            "Tenant2 profile".to_string(),
            "Content".to_string(),
        )
        .with_user_context("user1".to_string(), Some("tenant2".to_string()));

        storage.put_item(item1.clone()).unwrap();
        storage.put_item(item2.clone()).unwrap();

        // Both should be accessible by user1
        let user1_items = storage.get_items_by_user("user1");
        assert_eq!(user1_items.len(), 2);

        // Check tenant isolation in item
        assert_eq!(item1.tenant_id, Some("tenant1".to_string()));
        assert_eq!(item2.tenant_id, Some("tenant2".to_string()));
    }

    #[test]
    fn test_user_scope_can_access() {
        use crate::models::UserScope;

        // Test public resource (no user_id)
        let scope = UserScope::new("user1".to_string());
        assert!(scope.can_access(&None, &None)); // Public resource

        // Test user's own resource
        assert!(scope.can_access(&Some("user1".to_string()), &None));

        // Test another user's resource
        assert!(!scope.can_access(&Some("user2".to_string()), &None));

        // Test tenant isolation
        let scope_with_tenant =
            UserScope::new("user1".to_string()).with_tenant("tenant1".to_string());
        assert!(
            scope_with_tenant.can_access(&Some("user1".to_string()), &Some("tenant1".to_string()))
        );
        assert!(
            !scope_with_tenant.can_access(&Some("user1".to_string()), &Some("tenant2".to_string()))
        );
    }

    #[test]
    fn test_tenant_scoped_items() {
        let storage = MemoryStorage::new();

        // Create items with different tenant_ids
        let item1 = MemoryItem::new(
            MemoryType::Profile,
            "Profile 1".to_string(),
            "Content 1".to_string(),
        )
        .with_user_context("user1".to_string(), Some("tenant1".to_string()));

        let item2 = MemoryItem::new(
            MemoryType::Profile,
            "Profile 2".to_string(),
            "Content 2".to_string(),
        )
        .with_user_context("user2".to_string(), Some("tenant1".to_string()));

        let item3 = MemoryItem::new(
            MemoryType::Profile,
            "Profile 3".to_string(),
            "Content 3".to_string(),
        )
        .with_user_context("user3".to_string(), Some("tenant2".to_string()));

        let item1_id = item1.id.clone();
        let item3_id = item3.id.clone();

        storage.put_item(item1).unwrap();
        storage.put_item(item2).unwrap();
        storage.put_item(item3).unwrap();

        // Get items by tenant
        let tenant1_items = storage.get_items_by_tenant("tenant1");
        assert_eq!(tenant1_items.len(), 2);

        let tenant2_items = storage.get_items_by_tenant("tenant2");
        assert_eq!(tenant2_items.len(), 1);

        // Test item_belongs_to_tenant
        assert!(storage.item_belongs_to_tenant(&item1_id, "tenant1"));
        assert!(!storage.item_belongs_to_tenant(&item1_id, "tenant2"));
        assert!(storage.item_belongs_to_tenant(&item3_id, "tenant2"));
        assert!(!storage.item_belongs_to_tenant(&item3_id, "tenant1"));
    }

    #[test]
    fn test_tenant_scoped_resources() {
        let storage = MemoryStorage::new();

        // Create resources with different tenant_ids
        let resource1 = Resource::new("http://example.com/1".to_string(), Modality::Document)
            .with_user_context("user1".to_string(), Some("tenant1".to_string()));

        let resource2 = Resource::new("http://example.com/2".to_string(), Modality::Document)
            .with_user_context("user2".to_string(), Some("tenant1".to_string()));

        let resource3 = Resource::new("http://example.com/3".to_string(), Modality::Document)
            .with_user_context("user3".to_string(), Some("tenant2".to_string()));

        storage.put_resource(resource1.clone()).unwrap();
        storage.put_resource(resource2.clone()).unwrap();
        storage.put_resource(resource3.clone()).unwrap();

        // Get resources by tenant
        let tenant1_resources = storage.get_resources_by_tenant("tenant1");
        assert_eq!(tenant1_resources.len(), 2);

        let tenant2_resources = storage.get_resources_by_tenant("tenant2");
        assert_eq!(tenant2_resources.len(), 1);

        // Test resource_belongs_to_tenant
        assert!(storage.resource_belongs_to_tenant(&resource1.id, "tenant1"));
        assert!(!storage.resource_belongs_to_tenant(&resource1.id, "tenant2"));
    }

    #[test]
    fn test_tenant_counts() {
        let storage = MemoryStorage::new();

        // Create items for different tenants
        for i in 0..3 {
            let item = MemoryItem::new(
                MemoryType::Knowledge,
                format!("Knowledge {}", i),
                format!("Content {}", i),
            )
            .with_user_context(format!("user{}", i), Some("tenant1".to_string()));
            storage.put_item(item).unwrap();
        }

        for i in 0..2 {
            let item = MemoryItem::new(
                MemoryType::Knowledge,
                format!("Knowledge {}", i + 10),
                format!("Content {}", i + 10),
            )
            .with_user_context(format!("user{}", i + 10), Some("tenant2".to_string()));
            storage.put_item(item).unwrap();
        }

        // Test item_count_by_tenant
        assert_eq!(storage.item_count_by_tenant("tenant1"), 3);
        assert_eq!(storage.item_count_by_tenant("tenant2"), 2);
        assert_eq!(storage.item_count_by_tenant("tenant3"), 0);

        // Test get_all_tenants
        let tenants = storage.get_all_tenants();
        assert!(tenants.contains(&"tenant1".to_string()));
        assert!(tenants.contains(&"tenant2".to_string()));
    }
}
