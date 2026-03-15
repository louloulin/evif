//! SQLite storage backend
//!
//! Persistent storage using SQLite database with optional vector extension support.

use crate::error::{MemError, MemResult};
use crate::models::{CategoryItem, MemoryCategory, MemoryItem, Resource};
use rusqlite::{params, Connection, Row};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// SQLite-based storage for memory items
///
/// Provides persistent storage with full-text search capabilities.
/// Thread-safe through Arc<Mutex<Connection>>.
pub struct SQLiteStorage {
    conn: Arc<Mutex<Connection>>,
}

impl SQLiteStorage {
    /// Create a new SQLite storage instance
    ///
    /// # Arguments
    /// * `path` - Path to the SQLite database file. Use ":memory:" for in-memory database.
    ///
    /// # Returns
    /// A new SQLiteStorage instance with initialized schema
    pub fn new<P: AsRef<Path>>(path: P) -> MemResult<Self> {
        let conn = Connection::open(path)
            .map_err(|e| MemError::Storage(format!("Failed to open database: {}", e)))?;

        let storage = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        storage.initialize_schema()?;
        Ok(storage)
    }

    /// Create an in-memory SQLite database
    pub fn in_memory() -> MemResult<Self> {
        Self::new(":memory:")
    }

    /// Initialize database schema
    fn initialize_schema(&self) -> MemResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            r#"
            -- Resources table
            CREATE TABLE IF NOT EXISTS resources (
                id TEXT PRIMARY KEY,
                url TEXT NOT NULL,
                modality TEXT NOT NULL,
                local_path TEXT,
                caption TEXT,
                embedding_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- Memory items table
            CREATE TABLE IF NOT EXISTS memory_items (
                id TEXT PRIMARY KEY,
                resource_id TEXT,
                memory_type TEXT NOT NULL,
                summary TEXT NOT NULL,
                content TEXT NOT NULL,
                embedding_id TEXT,
                happened_at TEXT,
                content_hash TEXT,
                reinforcement_count INTEGER DEFAULT 0,
                last_reinforced_at TEXT,
                ref_id TEXT,
                category_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- Categories table
            CREATE TABLE IF NOT EXISTS categories (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                embedding_id TEXT,
                summary TEXT,
                item_count INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- Category-Item relationships table
            CREATE TABLE IF NOT EXISTS category_items (
                id TEXT PRIMARY KEY,
                item_id TEXT NOT NULL,
                category_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (item_id) REFERENCES memory_items(id),
                FOREIGN KEY (category_id) REFERENCES categories(id),
                UNIQUE(item_id, category_id)
            );

            -- Indexes for performance
            CREATE INDEX IF NOT EXISTS idx_items_type ON memory_items(memory_type);
            CREATE INDEX IF NOT EXISTS idx_items_hash ON memory_items(content_hash);
            CREATE INDEX IF NOT EXISTS idx_items_ref_id ON memory_items(ref_id);
            CREATE INDEX IF NOT EXISTS idx_category_items_item ON category_items(item_id);
            CREATE INDEX IF NOT EXISTS idx_category_items_category ON category_items(category_id);
            "#,
        )
        .map_err(|e| MemError::Storage(format!("Failed to initialize schema: {}", e)))?;

        Ok(())
    }

    // Resource operations

    pub fn put_resource(&self, resource: Resource) -> MemResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO resources (id, url, modality, local_path, caption, embedding_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                resource.id,
                resource.url,
                resource.modality.to_string(),
                resource.local_path,
                resource.caption,
                resource.embedding_id,
                resource.created_at.to_rfc3339(),
                resource.updated_at.to_rfc3339()
            ],
        )
        .map_err(|e| MemError::Storage(format!("Failed to put resource: {}", e)))?;

        Ok(())
    }

    pub fn get_resource(&self, id: &str) -> MemResult<Resource> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, url, modality, local_path, caption, embedding_id, created_at, updated_at FROM resources WHERE id = ?1",
            )
            .map_err(|e| MemError::Storage(format!("Failed to prepare statement: {}", e)))?;

        let resource = stmt
            .query_row(params![id], |row| self.row_to_resource(row))
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    MemError::NotFound(format!("Resource not found: {}", id))
                }
                e => MemError::Storage(format!("Failed to get resource: {}", e)),
            })?;

        Ok(resource)
    }

    // Memory item operations

    pub fn put_item(&self, mut item: MemoryItem) -> MemResult<()> {
        let conn = self.conn.lock().unwrap();

        // Check for duplicates by content hash
        if let Some(ref hash) = item.content_hash {
            let existing: Option<(String, i32)> = conn
                .query_row(
                    "SELECT id, reinforcement_count FROM memory_items WHERE content_hash = ?1",
                    params![hash],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .ok();

            if let Some((existing_id, count)) = existing {
                // Increment reinforcement count and update timestamp
                let now = chrono::Utc::now();
                conn.execute(
                    "UPDATE memory_items SET reinforcement_count = ?1, last_reinforced_at = ?2, updated_at = ?3 WHERE id = ?4",
                    params![
                        count + 1,
                        now.to_rfc3339(),
                        now.to_rfc3339(),
                        existing_id
                    ],
                )
                .map_err(|e| {
                    MemError::Storage(format!("Failed to update reinforcement: {}", e))
                })?;
                return Ok(());
            }
        }

        // Generate ref_id if not set
        if item.ref_id.is_none() {
            item.generate_ref_id();
        }

        conn.execute(
            "INSERT OR REPLACE INTO memory_items
             (id, resource_id, memory_type, summary, content, embedding_id, happened_at, content_hash, ref_id, reinforcement_count, last_reinforced_at, category_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                item.id,
                item.resource_id,
                item.memory_type.to_string(),
                item.summary,
                item.content,
                item.embedding_id,
                item.happened_at.map(|t| t.to_rfc3339()),
                item.content_hash,
                item.ref_id,
                item.reinforcement_count,
                item.last_reinforced_at.map(|t| t.to_rfc3339()),
                item.category_id,
                item.created_at.to_rfc3339(),
                item.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| MemError::Storage(format!("Failed to put item: {}", e)))?;

        Ok(())
    }

    pub fn get_item(&self, id: &str) -> MemResult<MemoryItem> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, resource_id, memory_type, summary, content, embedding_id,
                        happened_at, content_hash, ref_id, reinforcement_count, last_reinforced_at, category_id, created_at, updated_at
                 FROM memory_items WHERE id = ?1",
            )
            .map_err(|e| MemError::Storage(format!("Failed to prepare statement: {}", e)))?;

        let item = stmt
            .query_row(params![id], |row| self.row_to_memory_item(row))
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    MemError::NotFound(format!("Memory item not found: {}", id))
                }
                e => MemError::Storage(format!("Failed to get item: {}", e)),
            })?;

        Ok(item)
    }

    pub fn get_items_by_hash(&self, hash: &str) -> MemResult<Vec<MemoryItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, resource_id, memory_type, summary, content, embedding_id,
                        happened_at, content_hash, ref_id, reinforcement_count, last_reinforced_at, category_id, created_at, updated_at
                 FROM memory_items WHERE content_hash = ?1",
            )
            .map_err(|e| MemError::Storage(format!("Failed to prepare statement: {}", e)))?;

        let items = stmt
            .query_map(params![hash], |row| self.row_to_memory_item(row))
            .map_err(|e| MemError::Storage(format!("Failed to query items: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| MemError::Storage(format!("Failed to collect items: {}", e)))?;

        Ok(items)
    }

    pub fn get_items_by_type(&self, memory_type: &str) -> Vec<MemoryItem> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare(
            "SELECT id, resource_id, memory_type, summary, content, embedding_id,
                    happened_at, content_hash, ref_id, reinforcement_count, last_reinforced_at, category_id, created_at, updated_at
             FROM memory_items WHERE memory_type = ?1",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let items = stmt
            .query_map(params![memory_type], |row| self.row_to_memory_item(row))
            .ok()
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>().ok())
            .unwrap_or_default();

        items
    }

    pub fn get_all_items(&self) -> Vec<MemoryItem> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare(
            "SELECT id, resource_id, memory_type, summary, content, embedding_id,
                    happened_at, content_hash, ref_id, reinforcement_count, last_reinforced_at, category_id, created_at, updated_at
             FROM memory_items",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let items = stmt
            .query_map([], |row| self.row_to_memory_item(row))
            .ok()
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>().ok())
            .unwrap_or_default();

        items
    }

    pub fn delete_item(&self, id: &str) -> MemResult<()> {
        let conn = self.conn.lock().unwrap();

        let rows_affected = conn
            .execute("DELETE FROM memory_items WHERE id = ?1", params![id])
            .map_err(|e| MemError::Storage(format!("Failed to delete item: {}", e)))?;

        if rows_affected == 0 {
            return Err(MemError::NotFound(format!("Item not found: {}", id)));
        }

        Ok(())
    }

    // Category operations

    pub fn put_category(&self, category: MemoryCategory) -> MemResult<()> {
        let conn = self.conn.lock().unwrap();

        // Count items in this category
        let item_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM category_items WHERE category_id = ?1",
                params![category.id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        conn.execute(
            "INSERT OR REPLACE INTO categories
             (id, name, description, embedding_id, summary, item_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                category.id,
                category.name,
                category.description,
                category.embedding_id,
                category.summary,
                item_count,
                category.created_at.to_rfc3339(),
                category.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| MemError::Storage(format!("Failed to put category: {}", e)))?;

        Ok(())
    }

    pub fn get_category(&self, id: &str) -> MemResult<MemoryCategory> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, embedding_id, summary, item_count, created_at, updated_at
                 FROM categories WHERE id = ?1",
            )
            .map_err(|e| MemError::Storage(format!("Failed to prepare statement: {}", e)))?;

        let category = stmt
            .query_row(params![id], |row| self.row_to_category(row))
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    MemError::NotFound(format!("Category not found: {}", id))
                }
                e => MemError::Storage(format!("Failed to get category: {}", e)),
            })?;

        Ok(category)
    }

    pub fn get_all_categories(&self) -> Vec<MemoryCategory> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare(
            "SELECT id, name, description, embedding_id, summary, item_count, created_at, updated_at
             FROM categories",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let categories = stmt
            .query_map([], |row| self.row_to_category(row))
            .ok()
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>().ok())
            .unwrap_or_default();

        categories
    }

    pub fn delete_category(&self, id: &str) -> MemResult<()> {
        let conn = self.conn.lock().unwrap();

        let rows_affected = conn
            .execute("DELETE FROM categories WHERE id = ?1", params![id])
            .map_err(|e| MemError::Storage(format!("Failed to delete category: {}", e)))?;

        if rows_affected == 0 {
            return Err(MemError::NotFound(format!("Category not found: {}", id)));
        }

        Ok(())
    }

    // Category-Item relationship

    pub fn link_item_to_category(&self, item_id: &str, category_id: &str) -> MemResult<()> {
        // Verify both exist
        self.get_item(item_id)?;
        self.get_category(category_id)?;

        let conn = self.conn.lock().unwrap();
        let relation = CategoryItem::new(item_id.to_string(), category_id.to_string());

        conn.execute(
            "INSERT OR IGNORE INTO category_items (id, item_id, category_id, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                relation.id,
                item_id,
                category_id,
                relation.created_at.to_rfc3339()
            ],
        )
        .map_err(|e| MemError::Storage(format!("Failed to link item to category: {}", e)))?;

        // Update category item count
        conn.execute(
            "UPDATE categories SET item_count = (
                SELECT COUNT(*) FROM category_items WHERE category_id = ?1
             ) WHERE id = ?1",
            params![category_id],
        )
        .map_err(|e| MemError::Storage(format!("Failed to update category item count: {}", e)))?;

        Ok(())
    }

    pub fn get_items_in_category(&self, category_id: &str) -> Vec<MemoryItem> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare(
            "SELECT m.id, m.resource_id, m.memory_type, m.summary, m.content, m.embedding_id,
                    m.happened_at, m.content_hash, m.ref_id, m.reinforcement_count, m.last_reinforced_at, m.category_id, m.created_at, m.updated_at
             FROM memory_items m
             INNER JOIN category_items c ON m.id = c.item_id
             WHERE c.category_id = ?1",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let items = stmt
            .query_map(params![category_id], |row| self.row_to_memory_item(row))
            .ok()
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>().ok())
            .unwrap_or_default();

        items
    }

    // Helper methods for row conversion

    fn row_to_resource(&self, row: &Row) -> Result<Resource, rusqlite::Error> {
        use crate::models::Modality;

        let modality_str: String = row.get(2)?;
        let modality = Modality::from_str(&modality_str).unwrap_or(Modality::Document);

        Ok(Resource {
            id: row.get(0)?,
            url: row.get(1)?,
            modality,
            local_path: row.get(3)?,
            caption: row.get(4)?,
            embedding_id: row.get(5)?,
            user_id: None,
            tenant_id: None,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        })
    }

    fn row_to_memory_item(&self, row: &Row) -> Result<MemoryItem, rusqlite::Error> {
        use crate::models::MemoryType;

        let memory_type_str: String = row.get(2)?;
        let memory_type = MemoryType::from_str(&memory_type_str).unwrap_or(MemoryType::Knowledge);

        let happened_at_str: Option<String> = row.get(6)?;
        let happened_at = happened_at_str
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        let last_reinforced_str: Option<String> = row.get(10)?;
        let last_reinforced_at = last_reinforced_str
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        Ok(MemoryItem {
            id: row.get(0)?,
            resource_id: row.get(1)?,
            memory_type,
            summary: row.get(3)?,
            content: row.get(4)?,
            embedding_id: row.get(5)?,
            happened_at,
            content_hash: row.get(7)?,
            ref_id: row.get(8)?,
            reinforcement_count: row.get(9)?,
            last_reinforced_at,
            category_id: row.get(11)?,
            user_id: None,
            tenant_id: None,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(12)?)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(13)?)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        })
    }

    fn row_to_category(&self, row: &Row) -> Result<MemoryCategory, rusqlite::Error> {
        Ok(MemoryCategory {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            embedding_id: row.get(3)?,
            summary: row.get(4)?,
            item_count: row.get(5)?,
            user_id: None,
            tenant_id: None,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{compute_content_hash, MemoryType, Modality};

    #[test]
    fn test_sqlite_storage_creation() {
        let storage = SQLiteStorage::in_memory().unwrap();
        assert!(storage.get_all_items().is_empty());
    }

    #[test]
    fn test_put_and_get_resource() {
        let storage = SQLiteStorage::in_memory().unwrap();
        let resource = Resource::new("http://example.com".to_string(), Modality::Conversation);

        storage.put_resource(resource.clone()).unwrap();
        let retrieved = storage.get_resource(&resource.id).unwrap();

        assert_eq!(retrieved.id, resource.id);
        assert_eq!(retrieved.url, "http://example.com");
    }

    #[test]
    fn test_put_and_get_item() {
        let storage = SQLiteStorage::in_memory().unwrap();
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
        let storage = SQLiteStorage::in_memory().unwrap();

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
        let storage = SQLiteStorage::in_memory().unwrap();

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
        let storage = SQLiteStorage::in_memory().unwrap();

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
        let storage = SQLiteStorage::in_memory().unwrap();

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

        // Check category item count updated
        let updated_category = storage.get_category(&category.id).unwrap();
        assert_eq!(updated_category.item_count, 1);
    }

    #[test]
    fn test_delete_item() {
        let storage = SQLiteStorage::in_memory().unwrap();
        let item = MemoryItem::new(
            MemoryType::Profile,
            "Test".to_string(),
            "Content".to_string(),
        );

        storage.put_item(item.clone()).unwrap();
        storage.delete_item(&item.id).unwrap();

        assert!(storage.get_item(&item.id).is_err());
    }

    #[test]
    fn test_embedding_id_storage() {
        let storage = SQLiteStorage::in_memory().unwrap();
        let mut item = MemoryItem::new(
            MemoryType::Profile,
            "Test".to_string(),
            "Content".to_string(),
        );
        item.embedding_id = Some("emb-12345".to_string());

        storage.put_item(item.clone()).unwrap();
        let retrieved = storage.get_item(&item.id).unwrap();

        assert_eq!(retrieved.embedding_id, Some("emb-12345".to_string()));
    }
}
