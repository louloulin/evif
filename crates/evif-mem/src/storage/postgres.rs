//! PostgreSQL storage backend
//!
//! Provides a production-grade PostgreSQL storage implementation
//! with connection pooling and async operations.

use crate::error::{MemError, MemResult};
use crate::models::{MemoryCategory, MemoryItem, Resource};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;
use std::sync::Arc;

/// PostgreSQL storage for memory items
pub struct PostgresStorage {
    pool: Arc<PgPool>,
}

impl PostgresStorage {
    /// Create a new PostgreSQL storage with connection string
    pub async fn new(connection_string: &str) -> MemResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(connection_string)
            .await
            .map_err(|e| MemError::Storage(format!("Failed to connect to PostgreSQL: {}", e)))?;

        // Initialize schema
        Self::init_schema(&pool).await?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Create a new PostgreSQL storage with custom pool options
    pub async fn with_options(
        connection_string: &str,
        max_connections: u32,
        min_connections: u32,
    ) -> MemResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .min_connections(min_connections)
            .connect(connection_string)
            .await
            .map_err(|e| MemError::Storage(format!("Failed to connect to PostgreSQL: {}", e)))?;

        // Initialize schema
        Self::init_schema(&pool).await?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Initialize the database schema
    async fn init_schema(pool: &PgPool) -> MemResult<()> {
        // Create tables if they don't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS resources (
                id VARCHAR(255) PRIMARY KEY,
                url TEXT NOT NULL,
                modality VARCHAR(50) NOT NULL,
                local_path TEXT,
                caption TEXT,
                embedding_id VARCHAR(255),
                user_id VARCHAR(255),
                tenant_id VARCHAR(255),
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| MemError::Storage(format!("Failed to create resources table: {}", e)))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS memory_items (
                id VARCHAR(255) PRIMARY KEY,
                ref_id VARCHAR(20),
                resource_id VARCHAR(255),
                memory_type VARCHAR(50) NOT NULL,
                summary TEXT NOT NULL,
                content TEXT NOT NULL,
                embedding_id VARCHAR(255),
                happened_at TIMESTAMPTZ,
                content_hash VARCHAR(64),
                reinforcement_count INTEGER DEFAULT 0,
                last_reinforced_at TIMESTAMPTZ,
                category_id VARCHAR(255),
                user_id VARCHAR(255),
                tenant_id VARCHAR(255),
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| MemError::Storage(format!("Failed to create memory_items table: {}", e)))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS categories (
                id VARCHAR(255) PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                description TEXT,
                embedding_id VARCHAR(255),
                summary TEXT,
                item_count INTEGER DEFAULT 0,
                user_id VARCHAR(255),
                tenant_id VARCHAR(255),
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| MemError::Storage(format!("Failed to create categories table: {}", e)))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS category_items (
                id VARCHAR(255) PRIMARY KEY,
                item_id VARCHAR(255) NOT NULL,
                category_id VARCHAR(255) NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(item_id, category_id)
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| MemError::Storage(format!("Failed to create category_items table: {}", e)))?;

        // Create indexes for better query performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_items_user ON memory_items(user_id)")
            .execute(pool)
            .await
            .map_err(|e| MemError::Storage(format!("Failed to create index: {}", e)))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_items_tenant ON memory_items(tenant_id)")
            .execute(pool)
            .await
            .map_err(|e| MemError::Storage(format!("Failed to create index: {}", e)))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_items_type ON memory_items(memory_type)")
            .execute(pool)
            .await
            .map_err(|e| MemError::Storage(format!("Failed to create index: {}", e)))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_resources_user ON resources(user_id)")
            .execute(pool)
            .await
            .map_err(|e| MemError::Storage(format!("Failed to create index: {}", e)))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_categories_user ON categories(user_id)")
            .execute(pool)
            .await
            .map_err(|e| MemError::Storage(format!("Failed to create index: {}", e)))?;

        Ok(())
    }

    /// Get the underlying pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // Resource operations
    pub async fn put_resource(&self, resource: Resource) -> MemResult<()> {
        sqlx::query(
            r#"
            INSERT INTO resources (id, url, modality, local_path, caption, embedding_id, user_id, tenant_id, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, CURRENT_TIMESTAMP)
            ON CONFLICT (id) DO UPDATE SET
                url = EXCLUDED.url,
                modality = EXCLUDED.modality,
                local_path = EXCLUDED.local_path,
                caption = EXCLUDED.caption,
                embedding_id = EXCLUDED.embedding_id,
                user_id = EXCLUDED.user_id,
                tenant_id = EXCLUDED.tenant_id,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(&resource.id)
        .bind(&resource.url)
        .bind(resource.modality.to_string())
        .bind(&resource.local_path)
        .bind(&resource.caption)
        .bind(&resource.embedding_id)
        .bind(&resource.user_id)
        .bind(&resource.tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| MemError::Storage(format!("Failed to insert resource: {}", e)))?;

        Ok(())
    }

    pub async fn get_resource(&self, id: &str) -> MemResult<Resource> {
        let row = sqlx::query(
            "SELECT id, url, modality, local_path, caption, embedding_id, user_id, tenant_id, created_at, updated_at FROM resources WHERE id = $1"
        )
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => MemError::NotFound(format!("Resource not found: {}", id)),
            _ => MemError::Storage(format!("Failed to get resource: {}", e)),
        })?;

        Ok(Resource {
            id: row.get("id"),
            url: row.get("url"),
            modality: crate::models::Modality::from_str(&row.get::<String, _>("modality"))
                .unwrap_or(crate::models::Modality::Document),
            local_path: row.get("local_path"),
            caption: row.get("caption"),
            embedding_id: row.get("embedding_id"),
            user_id: row.get("user_id"),
            tenant_id: row.get("tenant_id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    // Memory item operations
    pub async fn put_item(&self, item: MemoryItem) -> MemResult<()> {
        // Check for duplicate by content_hash
        if let Some(ref hash) = item.content_hash {
            let existing: Option<String> =
                sqlx::query_scalar("SELECT id FROM memory_items WHERE content_hash = $1")
                    .bind(hash)
                    .fetch_optional(&*self.pool)
                    .await
                    .map_err(|e| MemError::Storage(format!("Failed to check duplicate: {}", e)))?;

            if let Some(existing_id) = existing {
                // Increment reinforcement count
                sqlx::query(
                    "UPDATE memory_items SET reinforcement_count = reinforcement_count + 1, last_reinforced_at = CURRENT_TIMESTAMP WHERE id = $1"
                )
                .bind(&existing_id)
                .execute(&*self.pool)
                .await
                .map_err(|e| MemError::Storage(format!("Failed to update reinforcement: {}", e)))?;
                return Ok(());
            }
        }

        sqlx::query(
            r#"
            INSERT INTO memory_items (id, ref_id, resource_id, memory_type, summary, content, embedding_id, happened_at, content_hash, reinforcement_count, last_reinforced_at, category_id, user_id, tenant_id, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, CURRENT_TIMESTAMP)
            ON CONFLICT (id) DO UPDATE SET
                ref_id = EXCLUDED.ref_id,
                resource_id = EXCLUDED.resource_id,
                memory_type = EXCLUDED.memory_type,
                summary = EXCLUDED.summary,
                content = EXCLUDED.content,
                embedding_id = EXCLUDED.embedding_id,
                happened_at = EXCLUDED.happened_at,
                content_hash = EXCLUDED.content_hash,
                reinforcement_count = EXCLUDED.reinforcement_count,
                last_reinforced_at = EXCLUDED.last_reinforced_at,
                category_id = EXCLUDED.category_id,
                user_id = EXCLUDED.user_id,
                tenant_id = EXCLUDED.tenant_id,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(&item.id)
        .bind(&item.ref_id)
        .bind(&item.resource_id)
        .bind(item.memory_type.to_string())
        .bind(&item.summary)
        .bind(&item.content)
        .bind(&item.embedding_id)
        .bind(item.happened_at)
        .bind(&item.content_hash)
        .bind(item.reinforcement_count as i32)
        .bind(item.last_reinforced_at)
        .bind(&item.category_id)
        .bind(&item.user_id)
        .bind(&item.tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| MemError::Storage(format!("Failed to insert item: {}", e)))?;

        Ok(())
    }

    pub async fn get_item(&self, id: &str) -> MemResult<MemoryItem> {
        let row = sqlx::query(
            r#"
            SELECT id, ref_id, resource_id, memory_type, summary, content, embedding_id, happened_at, content_hash, reinforcement_count, last_reinforced_at, category_id, user_id, tenant_id, created_at, updated_at
            FROM memory_items WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => MemError::NotFound(format!("Memory item not found: {}", id)),
            _ => MemError::Storage(format!("Failed to get item: {}", e)),
        })?;

        Ok(MemoryItem {
            id: row.get("id"),
            ref_id: row.get("ref_id"),
            resource_id: row.get("resource_id"),
            memory_type: crate::models::MemoryType::from_str(&row.get::<String, _>("memory_type"))
                .unwrap_or(crate::models::MemoryType::Knowledge),
            summary: row.get("summary"),
            content: row.get("content"),
            embedding_id: row.get("embedding_id"),
            happened_at: row.get("happened_at"),
            content_hash: row.get("content_hash"),
            reinforcement_count: row.get::<i32, _>("reinforcement_count") as u32,
            last_reinforced_at: row.get("last_reinforced_at"),
            category_id: row.get("category_id"),
            tags: row.get("tags"),
            references: row.get("references"),
            user_id: row.get("user_id"),
            tenant_id: row.get("tenant_id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    pub async fn get_items_by_type(&self, memory_type: &str) -> MemResult<Vec<MemoryItem>> {
        let rows = sqlx::query(
            r#"
            SELECT id, ref_id, resource_id, memory_type, summary, content, embedding_id, happened_at, content_hash, reinforcement_count, last_reinforced_at, category_id, user_id, tenant_id, created_at, updated_at
            FROM memory_items WHERE memory_type = $1
            "#
        )
        .bind(memory_type)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| MemError::Storage(format!("Failed to get items by type: {}", e)))?;

        let mut items = Vec::new();
        for row in rows {
            items.push(MemoryItem {
                id: row.get("id"),
                ref_id: row.get("ref_id"),
                resource_id: row.get("resource_id"),
                memory_type: crate::models::MemoryType::from_str(
                    &row.get::<String, _>("memory_type"),
                )
                .unwrap_or(crate::models::MemoryType::Knowledge),
                summary: row.get("summary"),
                content: row.get("content"),
                embedding_id: row.get("embedding_id"),
                happened_at: row.get("happened_at"),
                content_hash: row.get("content_hash"),
                reinforcement_count: row.get::<i32, _>("reinforcement_count") as u32,
last_reinforced_at: row.get("last_reinforced_at"),
                category_id: row.get("category_id"),
                tags: row.get("tags"),
                references: row.get("references"),
                user_id: row.get("user_id"),
                tenant_id: row.get("tenant_id"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(items)
    }

    pub async fn get_all_items(&self) -> MemResult<Vec<MemoryItem>> {
        let rows = sqlx::query(
            r#"
            SELECT id, ref_id, resource_id, memory_type, summary, content, embedding_id, happened_at, content_hash, reinforcement_count, last_reinforced_at
            FROM memory_items
            "#
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| MemError::Storage(format!("Failed to get all items: {}", e)))?;

        let mut items = Vec::new();
        for row in rows {
            items.push(MemoryItem {
                id: row.get("id"),
                ref_id: row.get("ref_id"),
                resource_id: row.get("resource_id"),
                memory_type: crate::models::MemoryType::from_str(
                    &row.get::<String, _>("memory_type"),
                )
                .unwrap_or(crate::models::MemoryType::Knowledge),
                summary: row.get("summary"),
                content: row.get("content"),
                embedding_id: row.get("embedding_id"),
                happened_at: row.get("happened_at"),
                content_hash: row.get("content_hash"),
                reinforcement_count: row.get::<i32, _>("reinforcement_count") as u32,
                last_reinforced_at: row.get("last_reinforced_at"),
                category_id: row.get("category_id"),
                tags: row.get("tags"),
                references: row.get("references"),
                user_id: row.get("user_id"),
                tenant_id: row.get("tenant_id"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(items)
    }

    pub async fn get_items_by_user(&self, user_id: &str) -> MemResult<Vec<MemoryItem>> {
        let rows = sqlx::query(
            r#"
            SELECT id, ref_id, resource_id, memory_type, summary, content, embedding_id, happened_at, content_hash, reinforcement_count, last_reinforced_at
            FROM memory_items WHERE user_id = $1
            "#
        )
        .bind(user_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| MemError::Storage(format!("Failed to get items by user: {}", e)))?;

        let mut items = Vec::new();
        for row in rows {
            items.push(MemoryItem {
                id: row.get("id"),
                ref_id: row.get("ref_id"),
                resource_id: row.get("resource_id"),
                memory_type: crate::models::MemoryType::from_str(
                    &row.get::<String, _>("memory_type"),
                )
                .unwrap_or(crate::models::MemoryType::Knowledge),
                summary: row.get("summary"),
                content: row.get("content"),
                embedding_id: row.get("embedding_id"),
                happened_at: row.get("happened_at"),
                content_hash: row.get("content_hash"),
                reinforcement_count: row.get::<i32, _>("reinforcement_count") as u32,
                last_reinforced_at: row.get("last_reinforced_at"),
                category_id: row.get("category_id"),
                tags: row.get("tags"),
                references: row.get("references"),
                user_id: row.get("user_id"),
                tenant_id: row.get("tenant_id"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(items)
    }

    pub async fn get_items_by_tenant(&self, tenant_id: &str) -> MemResult<Vec<MemoryItem>> {
        let rows = sqlx::query(
            r#"
            SELECT id, ref_id, resource_id, memory_type, summary, content, embedding_id, happened_at, content_hash, reinforcement_count, last_reinforced_at, category_id, user_id, tenant_id, created_at, updated_at
            FROM memory_items WHERE tenant_id = $1
            "#
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| MemError::Storage(format!("Failed to get items by tenant: {}", e)))?;

        let mut items = Vec::new();
        for row in rows {
            items.push(MemoryItem {
                id: row.get("id"),
                ref_id: row.get("ref_id"),
                resource_id: row.get("resource_id"),
                memory_type: crate::models::MemoryType::from_str(
                    &row.get::<String, _>("memory_type"),
                )
                .unwrap_or(crate::models::MemoryType::Knowledge),
                summary: row.get("summary"),
                content: row.get("content"),
                embedding_id: row.get("embedding_id"),
                happened_at: row.get("happened_at"),
                content_hash: row.get("content_hash"),
                reinforcement_count: row.get::<i32, _>("reinforcement_count") as u32,
                last_reinforced_at: row.get("last_reinforced_at"),
                category_id: row.get("category_id"),
                tags: row.get("tags"),
                references: row.get("references"),
                user_id: row.get("user_id"),
                tenant_id: row.get("tenant_id"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(items)
    }

    pub async fn item_belongs_to_user(&self, item_id: &str, user_id: &str) -> MemResult<bool> {
        let result: Option<String> =
            sqlx::query_scalar("SELECT user_id FROM memory_items WHERE id = $1 AND user_id = $2")
                .bind(item_id)
                .bind(user_id)
                .fetch_optional(&*self.pool)
                .await
                .map_err(|e| MemError::Storage(format!("Failed to check ownership: {}", e)))?;

        Ok(result.is_some())
    }

    pub async fn item_belongs_to_tenant(&self, item_id: &str, tenant_id: &str) -> MemResult<bool> {
        let result: Option<String> = sqlx::query_scalar(
            "SELECT tenant_id FROM memory_items WHERE id = $1 AND tenant_id = $2",
        )
        .bind(item_id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| MemError::Storage(format!("Failed to check tenant: {}", e)))?;

        Ok(result.is_some())
    }

    pub async fn get_all_tenants(&self) -> MemResult<Vec<String>> {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT tenant_id FROM memory_items WHERE tenant_id IS NOT NULL",
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| MemError::Storage(format!("Failed to get tenants: {}", e)))?;

        Ok(rows)
    }

    pub async fn item_count_by_tenant(&self, tenant_id: &str) -> MemResult<usize> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM memory_items WHERE tenant_id = $1")
                .bind(tenant_id)
                .fetch_one(&*self.pool)
                .await
                .map_err(|e| MemError::Storage(format!("Failed to count items: {}", e)))?;

        Ok(count as usize)
    }

    pub async fn item_count(&self) -> MemResult<usize> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memory_items")
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| MemError::Storage(format!("Failed to count items: {}", e)))?;

        Ok(count as usize)
    }

    pub async fn delete_item(&self, id: &str) -> MemResult<()> {
        sqlx::query("DELETE FROM memory_items WHERE id = $1")
            .bind(id)
            .execute(&*self.pool)
            .await
            .map_err(|e| MemError::Storage(format!("Failed to delete item: {}", e)))?;

        Ok(())
    }

    // Category operations
    pub async fn put_category(&self, category: MemoryCategory) -> MemResult<()> {
        sqlx::query(
            r#"
            INSERT INTO categories (id, name, description, embedding_id, summary, item_count, user_id, tenant_id, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, CURRENT_TIMESTAMP)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                embedding_id = EXCLUDED.embedding_id,
                summary = EXCLUDED.summary,
                item_count = EXCLUDED.item_count,
                user_id = EXCLUDED.user_id,
                tenant_id = EXCLUDED.tenant_id,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(&category.id)
        .bind(&category.name)
        .bind(&category.description)
        .bind(&category.embedding_id)
        .bind(&category.summary)
        .bind(category.item_count as i32)
        .bind(&category.user_id)
        .bind(&category.tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| MemError::Storage(format!("Failed to insert category: {}", e)))?;

        Ok(())
    }

    pub async fn get_category(&self, id: &str) -> MemResult<MemoryCategory> {
        let row = sqlx::query(
            "SELECT id, name, description, embedding_id, summary, item_count, user_id, tenant_id, created_at, updated_at FROM categories WHERE id = $1"
        )
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => MemError::NotFound(format!("Category not found: {}", id)),
            _ => MemError::Storage(format!("Failed to get category: {}", e)),
        })?;

        Ok(MemoryCategory {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            embedding_id: row.get("embedding_id"),
            summary: row.get("summary"),
            item_count: row.get::<i32, _>("item_count") as u32,
            user_id: row.get("user_id"),
            tenant_id: row.get("tenant_id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    pub async fn get_all_categories(&self) -> MemResult<Vec<MemoryCategory>> {
        let rows = sqlx::query(
            "SELECT id, name, description, embedding_id, summary, item_count, user_id, tenant_id, created_at, updated_at FROM categories"
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| MemError::Storage(format!("Failed to get all categories: {}", e)))?;

        let mut categories = Vec::new();
        for row in rows {
            categories.push(MemoryCategory {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                embedding_id: row.get("embedding_id"),
                summary: row.get("summary"),
                item_count: row.get::<i32, _>("item_count") as u32,
                user_id: row.get("user_id"),
                tenant_id: row.get("tenant_id"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(categories)
    }

    pub async fn delete_category(&self, id: &str) -> MemResult<()> {
        sqlx::query("DELETE FROM categories WHERE id = $1")
            .bind(id)
            .execute(&*self.pool)
            .await
            .map_err(|e| MemError::Storage(format!("Failed to delete category: {}", e)))?;

        Ok(())
    }

    // Category-Item relationship
    pub async fn link_item_to_category(&self, item_id: &str, category_id: &str) -> MemResult<()> {
        let id = format!("{}_{}", item_id, category_id);

        sqlx::query(
            "INSERT INTO category_items (id, item_id, category_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING"
        )
        .bind(&id)
        .bind(item_id)
        .bind(category_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| MemError::Storage(format!("Failed to link item to category: {}", e)))?;

        // Update category item count
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM category_items WHERE category_id = $1")
                .bind(category_id)
                .fetch_one(&*self.pool)
                .await
                .map_err(|e| MemError::Storage(format!("Failed to count category items: {}", e)))?;

        sqlx::query("UPDATE categories SET item_count = $1 WHERE id = $2")
            .bind(count)
            .bind(category_id)
            .execute(&*self.pool)
            .await
            .map_err(|e| MemError::Storage(format!("Failed to update category count: {}", e)))?;

        Ok(())
    }

    pub async fn get_items_in_category(&self, category_id: &str) -> MemResult<Vec<MemoryItem>> {
        let rows = sqlx::query(
            r#"
            SELECT m.id, m.ref_id, m.resource_id, m.memory_type, m.summary, m.content, m.embedding_id, m.happened_at, m.content_hash, m.reinforcement_count, m.last_reinforced_at, m.category_id, m.user_id, m.tenant_id, m.created_at, m.updated_at
            FROM memory_items m
            INNER JOIN category_items c ON m.id = c.item_id
            WHERE c.category_id = $1
            "#
        )
        .bind(category_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| MemError::Storage(format!("Failed to get items in category: {}", e)))?;

        let mut items = Vec::new();
        for row in rows {
            items.push(MemoryItem {
                id: row.get("id"),
                ref_id: row.get("ref_id"),
                resource_id: row.get("resource_id"),
                memory_type: crate::models::MemoryType::from_str(
                    &row.get::<String, _>("memory_type"),
                )
                .unwrap_or(crate::models::MemoryType::Knowledge),
                summary: row.get("summary"),
                content: row.get("content"),
                embedding_id: row.get("embedding_id"),
                happened_at: row.get("happened_at"),
                content_hash: row.get("content_hash"),
                reinforcement_count: row.get::<i32, _>("reinforcement_count") as u32,
                last_reinforced_at: row.get("last_reinforced_at"),
                category_id: row.get("category_id"),
                tags: row.get("tags"),
                references: row.get("references"),
                user_id: row.get("user_id"),
                tenant_id: row.get("tenant_id"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(items)
    }

    // Resource operations for user scope
    pub async fn get_resources_by_user(&self, user_id: &str) -> MemResult<Vec<Resource>> {
        let rows = sqlx::query(
            "SELECT id, url, modality, local_path, caption, embedding_id, user_id, tenant_id, created_at, updated_at FROM resources WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| MemError::Storage(format!("Failed to get resources by user: {}", e)))?;

        let mut resources = Vec::new();
        for row in rows {
            resources.push(Resource {
                id: row.get("id"),
                url: row.get("url"),
                modality: crate::models::Modality::from_str(&row.get::<String, _>("modality"))
                    .unwrap_or(crate::models::Modality::Document),
                local_path: row.get("local_path"),
                caption: row.get("caption"),
                embedding_id: row.get("embedding_id"),
                user_id: row.get("user_id"),
                tenant_id: row.get("tenant_id"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(resources)
    }

    pub async fn get_categories_by_user(&self, user_id: &str) -> MemResult<Vec<MemoryCategory>> {
        let rows = sqlx::query(
            "SELECT id, name, description, embedding_id, summary, item_count, user_id, tenant_id, created_at, updated_at FROM categories WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| MemError::Storage(format!("Failed to get categories by user: {}", e)))?;

        let mut categories = Vec::new();
        for row in rows {
            categories.push(MemoryCategory {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                embedding_id: row.get("embedding_id"),
                summary: row.get("summary"),
                item_count: row.get::<i32, _>("item_count") as u32,
                user_id: row.get("user_id"),
                tenant_id: row.get("tenant_id"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(categories)
    }

    pub async fn get_resources_by_tenant(&self, tenant_id: &str) -> MemResult<Vec<Resource>> {
        let rows = sqlx::query(
            "SELECT id, url, modality, local_path, caption, embedding_id, user_id, tenant_id, created_at, updated_at FROM resources WHERE tenant_id = $1"
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| MemError::Storage(format!("Failed to get resources by tenant: {}", e)))?;

        let mut resources = Vec::new();
        for row in rows {
            resources.push(Resource {
                id: row.get("id"),
                url: row.get("url"),
                modality: crate::models::Modality::from_str(&row.get::<String, _>("modality"))
                    .unwrap_or(crate::models::Modality::Document),
                local_path: row.get("local_path"),
                caption: row.get("caption"),
                embedding_id: row.get("embedding_id"),
                user_id: row.get("user_id"),
                tenant_id: row.get("tenant_id"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(resources)
    }

    pub async fn get_categories_by_tenant(
        &self,
        tenant_id: &str,
    ) -> MemResult<Vec<MemoryCategory>> {
        let rows = sqlx::query(
            "SELECT id, name, description, embedding_id, summary, item_count, user_id, tenant_id, created_at, updated_at FROM categories WHERE tenant_id = $1"
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| MemError::Storage(format!("Failed to get categories by tenant: {}", e)))?;

        let mut categories = Vec::new();
        for row in rows {
            categories.push(MemoryCategory {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                embedding_id: row.get("embedding_id"),
                summary: row.get("summary"),
                item_count: row.get::<i32, _>("item_count") as u32,
                user_id: row.get("user_id"),
                tenant_id: row.get("tenant_id"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(categories)
    }

    pub async fn resource_belongs_to_user(
        &self,
        resource_id: &str,
        user_id: &str,
    ) -> MemResult<bool> {
        let result: Option<String> =
            sqlx::query_scalar("SELECT user_id FROM resources WHERE id = $1 AND user_id = $2")
                .bind(resource_id)
                .bind(user_id)
                .fetch_optional(&*self.pool)
                .await
                .map_err(|e| MemError::Storage(format!("Failed to check ownership: {}", e)))?;

        Ok(result.is_some())
    }

    pub async fn resource_belongs_to_tenant(
        &self,
        resource_id: &str,
        tenant_id: &str,
    ) -> MemResult<bool> {
        let result: Option<String> =
            sqlx::query_scalar("SELECT tenant_id FROM resources WHERE id = $1 AND tenant_id = $2")
                .bind(resource_id)
                .bind(tenant_id)
                .fetch_optional(&*self.pool)
                .await
                .map_err(|e| MemError::Storage(format!("Failed to check tenant: {}", e)))?;

        Ok(result.is_some())
    }

    pub async fn resource_count_by_tenant(&self, tenant_id: &str) -> MemResult<usize> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM resources WHERE tenant_id = $1")
            .bind(tenant_id)
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| MemError::Storage(format!("Failed to count resources: {}", e)))?;

        Ok(count as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a PostgreSQL instance to run
    // Use TEST_DATABASE_URL environment variable or skip if not available

    #[tokio::test]
    #[ignore] // Requires PostgreSQL instance
    async fn test_postgres_connection() {
        let connection_string = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:password@localhost:5432/evif_test".to_string()
        });

        let storage = PostgresStorage::new(&connection_string).await;
        assert!(storage.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires PostgreSQL instance
    async fn test_postgres_item_operations() {
        let connection_string = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:password@localhost:5432/evif_test".to_string()
        });

        let storage = PostgresStorage::new(&connection_string).await.unwrap();

        let item = MemoryItem::new(
            crate::models::MemoryType::Profile,
            "Test profile".to_string(),
            "Test content".to_string(),
        );

        storage.put_item(item.clone()).await.unwrap();
        let retrieved = storage.get_item(&item.id).await.unwrap();

        assert_eq!(retrieved.id, item.id);
        assert_eq!(retrieved.summary, "Test profile");
    }
}
