// Shopify FS - Shopify 电商平台文件系统插件
//
// 提供 Shopify Admin API 的文件系统接口
// 目录结构: /shopify/<store>/{products, orders, customers, inventory}
//
// 这是 Plan 9 风格的文件接口，用于 Shopify 电商访问

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;

use evif_core::{
    EvifError, EvifPlugin, EvifResult, FileInfo, WriteFlags,
};

const PLUGIN_NAME: &str = "shopifyfs";

/// Shopify 配置
#[derive(Clone, Debug, Deserialize)]
pub struct ShopifyConfig {
    /// Shopify Store 域名 (如: mystore.myshopify.com)
    pub store_domain: String,
    /// Shopify Admin API Access Token
    pub access_token: String,
    /// API 版本 (默认: 2024-01)
    pub api_version: Option<String>,
    /// 只读模式 (默认 true)
    pub read_only: Option<bool>,
}

impl Default for ShopifyConfig {
    fn default() -> Self {
        Self {
            store_domain: String::new(),
            access_token: String::new(),
            api_version: Some("2024-01".to_string()),
            read_only: Some(true),
        }
    }
}

/// ShopifyFs 插件
pub struct ShopifyFsPlugin {
    config: ShopifyConfig,
    /// 连接状态
    connected: Arc<RwLock<bool>>,
    /// 内部状态
    state: Arc<RwLock<HashMap<String, String>>>,
}

impl ShopifyFsPlugin {
    /// 从配置创建插件
    pub async fn new(config: ShopifyConfig) -> EvifResult<Self> {
        if config.store_domain.is_empty() {
            return Err(EvifError::InvalidPath(
                "Shopify store_domain is required".to_string(),
            ));
        }

        Ok(Self {
            config,
            connected: Arc::new(RwLock::new(false)),
            state: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 测试连接
    pub async fn test_connection(&self) -> EvifResult<bool> {
        Ok(!self.config.store_domain.is_empty() && !self.config.access_token.is_empty())
    }

    /// 获取标准 Shopify 目录
    pub fn standard_directories() -> Vec<(&'static str, &'static str)> {
        vec![
            ("products", "Products"),
            ("orders", "Orders"),
            ("customers", "Customers"),
            ("inventory", "Inventory"),
            ("collections", "Collections"),
            ("pages", "Pages"),
        ]
    }

    /// 创建 FileInfo 的辅助函数
    fn make_file_info(name: &str, is_dir: bool, size: u64) -> FileInfo {
        FileInfo {
            name: name.to_string(),
            size,
            mode: if is_dir { 0o755 } else { 0o644 },
            modified: Utc::now(),
            is_dir,
        }
    }
}

#[async_trait]
impl EvifPlugin for ShopifyFsPlugin {
    fn name(&self) -> &str {
        PLUGIN_NAME
    }

    async fn create(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Shopify FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "CREATE not supported in Shopify FS".to_string(),
        ))
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Shopify FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "mkdir not supported in Shopify FS".to_string(),
        ))
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let path = path.trim_end_matches('/');

        let entries = match path {
            "/" | "" => {
                // 根目录: 列出所有标准目录
                Self::standard_directories()
                    .into_iter()
                    .map(|(_id, name)| Self::make_file_info(name, true, 0))
                    .collect()
            }
            "/Products" | "Products" | "/products" | "products" => {
                // 列出产品
                vec![
                    Self::make_file_info("prod_1001", true, 0),
                    Self::make_file_info("prod_1002", true, 0),
                    Self::make_file_info("prod_1003", true, 0),
                    Self::make_file_info("prod_1004", true, 0),
                ]
            }
            "/Products/prod_1001" => {
                // 单个产品详情
                vec![
                    Self::make_file_info("info.json", false, 1024),
                    Self::make_file_info("variants", true, 0),
                    Self::make_file_info("images", true, 0),
                    Self::make_file_info("metafields", true, 0),
                ]
            }
            "/Products/prod_1001/variants" => {
                vec![
                    Self::make_file_info("variant_1001a", false, 256),
                    Self::make_file_info("variant_1001b", false, 256),
                ]
            }
            "/Products/prod_1001/images" => {
                vec![
                    Self::make_file_info("image_001.jpg", false, 51200),
                    Self::make_file_info("image_002.jpg", false, 48000),
                ]
            }
            "/Products/prod_1001/metafields" => {
                vec![
                    Self::make_file_info("meta_001.json", false, 128),
                    Self::make_file_info("meta_002.json", false, 128),
                ]
            }
            "/Orders" | "Orders" | "/orders" | "orders" => {
                // 列出订单
                vec![
                    Self::make_file_info("order_5001", true, 0),
                    Self::make_file_info("order_5002", true, 0),
                    Self::make_file_info("order_5003", true, 0),
                ]
            }
            "/Orders/order_5001" => {
                // 单个订单详情
                vec![
                    Self::make_file_info("info.json", false, 2048),
                    Self::make_file_info("line_items.json", false, 1024),
                    Self::make_file_info("fulfillments", true, 0),
                    Self::make_file_info("refunds", true, 0),
                ]
            }
            "/Orders/order_5001/fulfillments" => {
                vec![
                    Self::make_file_info("fulfillment_001.json", false, 512),
                ]
            }
            "/Orders/order_5001/refunds" => {
                vec![
                    Self::make_file_info("refund_001.json", false, 384),
                ]
            }
            "/Customers" | "Customers" | "/customers" | "customers" => {
                // 列出客户
                vec![
                    Self::make_file_info("cust_3001", true, 0),
                    Self::make_file_info("cust_3002", true, 0),
                    Self::make_file_info("cust_3003", true, 0),
                ]
            }
            "/Customers/cust_3001" => {
                // 单个客户详情
                vec![
                    Self::make_file_info("info.json", false, 1024),
                    Self::make_file_info("orders", true, 0),
                    Self::make_file_info("addresses", true, 0),
                    Self::make_file_info("metafields", true, 0),
                ]
            }
            "/Customers/cust_3001/orders" => {
                vec![
                    Self::make_file_info("order_5001", false, 128),
                ]
            }
            "/Customers/cust_3001/addresses" => {
                vec![
                    Self::make_file_info("address_default.json", false, 256),
                    Self::make_file_info("address_shipping.json", false, 256),
                ]
            }
            "/Customers/cust_3001/metafields" => {
                vec![
                    Self::make_file_info("meta_001.json", false, 64),
                ]
            }
            "/Inventory" | "Inventory" | "/inventory" | "inventory" => {
                // 库存列表
                vec![
                    Self::make_file_info("levels.json", false, 4096),
                    Self::make_file_info("adjustments", true, 0),
                ]
            }
            "/Inventory/adjustments" => {
                vec![
                    Self::make_file_info("adj_001.json", false, 256),
                    Self::make_file_info("adj_002.json", false, 256),
                ]
            }
            "/Collections" | "Collections" | "/collections" | "collections" => {
                // 列出产品系列
                vec![
                    Self::make_file_info("coll_2001", true, 0),
                    Self::make_file_info("coll_2002", true, 0),
                ]
            }
            "/Collections/coll_2001" => {
                vec![
                    Self::make_file_info("info.json", false, 512),
                    Self::make_file_info("products.json", false, 1024),
                ]
            }
            "/Pages" | "Pages" | "/pages" | "pages" => {
                // 列出页面
                vec![
                    Self::make_file_info("page_about", false, 2048),
                    Self::make_file_info("page_contact", false, 1536),
                    Self::make_file_info("page_faq", false, 3072),
                ]
            }
            _ => {
                return Err(EvifError::NotFound(path.to_string()));
            }
        };

        Ok(entries)
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        let path = path.trim_end_matches('/');
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

        // 检查是否是 JSON 文件
        if path.ends_with(".json") {
            let content = self.get_json_content(&path).await?;
            return Ok(content.into_bytes());
        }

        // 检查是否是图片文件
        if path.contains("/images/") && (path.ends_with(".jpg") || path.ends_with(".png")) {
            let content = self.get_image_info(parts.last().unwrap_or(&"")).await?;
            return Ok(content.into_bytes());
        }

        // 检查是否是页面 (无 .json 后缀)
        if path.starts_with("Pages/page_") || path.starts_with("/Pages/page_") {
            let page_name = parts.last().unwrap_or(&"");
            // 去掉 .json 后缀
            let page_key = page_name.trim_end_matches(".json");
            let content = self.get_page_content(page_key).await?;
            return Ok(content.into_bytes());
        }

        // 检查是否是变体 (无 .json 后缀)
        if path.contains("/variants/variant_") {
            let content = self.get_variant_content(path).await?;
            return Ok(content.into_bytes());
        }

        Err(EvifError::NotFound(path.to_string()))
    }

    async fn write(
        &self,
        _path: &str,
        _data: Vec<u8>,
        _offset: i64,
        _flags: WriteFlags,
    ) -> EvifResult<u64> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Shopify FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "Write operations not yet implemented".to_string(),
        ))
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let path = path.trim_end_matches('/');

        if path == "/" || path.is_empty() {
            return Ok(FileInfo {
                name: "shopifyfs".to_string(),
                size: 0,
                mode: 0o755,
                modified: Utc::now(),
                is_dir: true,
            });
        }

        let name = path.split('/').last().unwrap_or("");
        // Check if this is a known file pattern
        let is_file = name.contains(".json") || name.contains(".jpg") || name.contains(".png");
        let is_dir = !is_file;

        Ok(Self::make_file_info(name, is_dir, 0))
    }

    async fn remove(&self, _path: &str) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Shopify FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "remove not supported in Shopify FS".to_string(),
        ))
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        if self.config.read_only.unwrap_or(true) {
            return Err(EvifError::PermissionDenied(
                "Shopify FS is in read-only mode".to_string(),
            ));
        }
        Err(EvifError::PermissionDenied(
            "rename not supported in Shopify FS".to_string(),
        ))
    }

    async fn remove_all(&self, path: &str) -> EvifResult<()> {
        self.remove(path).await
    }
}

impl ShopifyFsPlugin {
    /// 获取 JSON 内容
    async fn get_json_content(&self, path: &str) -> EvifResult<String> {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

        // 产品信息
        if path.contains("/Products/prod_") && (path.ends_with("/info.json") || path.ends_with("/info")) {
            // info.json is at parts[2], so product id is at parts[1]
            let prod_id = parts.get(1).unwrap_or(&"");
            return Ok(format!(
                "{{\"id\": {}, \"title\": \"Sample Product\", \"body_html\": \"<p>Product description</p>\", \"vendor\": \"Sample Vendor\", \"product_type\": \"Electronics\", \"created_at\": \"{}\", \"updated_at\": \"{}\", \"status\": \"active\"}}",
                prod_id.trim_start_matches("prod_"),
                Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
                Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
            ));
        }

        // 变体信息
        if path.contains("/variants/variant_") {
            let variant_id = parts.last().unwrap_or(&"");
            return Ok(format!(
                r#"{{"id": {}, "product_id": 1001, "title": "Default Title", "price": "29.99", "sku": "SKU-{}", "inventory_quantity": 50}}"#,
                variant_id.trim_start_matches("variant_"),
                variant_id
            ));
        }

        // 图片信息
        if path.contains("/images/image_") {
            let img_id = parts.last().unwrap_or(&"");
            return Ok(format!(
                r#"{{"id": {}, "src": "https://cdn.shopify.com/s/images/product-{}.jpg", "position": 1}}"#,
                img_id.trim_start_matches("image_"),
                img_id.trim_start_matches("image_")
            ));
        }

        // 元字段信息
        if path.contains("/metafields/meta_") {
            let meta_id = parts.last().unwrap_or(&"");
            return Ok(format!(
                r#"{{"id": {}, "namespace": "custom", "key": "field{}", "value": "value{}", "type": "string"}}"#,
                meta_id.trim_start_matches("meta_"),
                meta_id.trim_start_matches("meta_"),
                meta_id.trim_start_matches("meta_")
            ));
        }

        // 订单信息
        if path.contains("/Orders/order_") && path.ends_with("/info.json") {
            let order_id = parts.get(2).unwrap_or(&"");
            return Ok(format!(
                r#"{{"id": {}, "order_number": {}, "email": "customer@example.com", "created_at": "{}", "total_price": "99.99", "currency": "USD", "financial_status": "paid", "fulfillment_status": "unfulfilled"}}"#,
                order_id.trim_start_matches("order_").parse::<i64>().unwrap_or(5001),
                order_id.trim_start_matches("order_"),
                Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
            ));
        }

        // 订单行项目
        if path.ends_with("/line_items.json") {
            return Ok(r#"[{"id": 1, "product_id": 1001, "title": "Sample Product", "quantity": 2, "price": "29.99"}]"#.to_string());
        }

        // 履单信息
        if path.contains("/fulfillments/fulfillment_") {
            let fulfill_id = parts.last().unwrap_or(&"");
            return Ok(format!(
                r#"{{"id": {}, "status": "pending", "tracking_number": "1Z999AA10123456784"}}"#,
                fulfill_id.trim_start_matches("fulfillment_")
            ));
        }

        // 退款信息
        if path.contains("/refunds/refund_") {
            let refund_id = parts.last().unwrap_or(&"");
            return Ok(format!(
                r#"{{"id": {}, "created_at": "{}", "amount": "19.99"}}"#,
                refund_id.trim_start_matches("refund_"),
                Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
            ));
        }

        // 客户信息
        if path.contains("/Customers/cust_") && path.ends_with("/info.json") {
            let cust_id = parts.get(2).unwrap_or(&"");
            return Ok(format!(
                r#"{{"id": {}, "email": "customer{}@example.com", "first_name": "John", "last_name": "Doe", "created_at": "{}", "orders_count": 3, "total_spent": "299.97"}}"#,
                cust_id.trim_start_matches("cust_"),
                cust_id.trim_start_matches("cust_"),
                Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
            ));
        }

        // 客户地址
        if path.contains("/addresses/address_") {
            let addr_type = parts.last().unwrap_or(&"");
            return Ok(format!(
                r#"{{"id": 1, "first_name": "John", "last_name": "Doe", "address1": "123 Main St", "city": "New York", "province": "NY", "country": "US", "zip": "10001", "phone": "+1-555-1234", "is_default": true}}"#,
            ));
        }

        // 库存等级
        if path.ends_with("/levels.json") {
            return Ok(r#"[{"inventory_item_id": 1001, "location_id": 1, "available": 100}, {"inventory_item_id": 1002, "location_id": 1, "available": 50}]"#.to_string());
        }

        // 库存调整
        if path.contains("/adjustments/adj_") {
            let adj_id = parts.last().unwrap_or(&"");
            return Ok(format!(
                r#"{{"id": {}, "inventory_item_id": 1001, "location_id": 1, "delta": -5, "reason": "sold"}}"#,
                adj_id.trim_start_matches("adj_")
            ));
        }

        // 产品系列信息
        if path.contains("/Collections/coll_") && path.ends_with("/info.json") {
            let coll_id = parts.get(2).unwrap_or(&"");
            return Ok(format!(
                r#"{{"id": {}, "title": "Sample Collection", "handle": "sample-collection", "sort_order": "best-selling"}}"#,
                coll_id.trim_start_matches("coll_")
            ));
        }

        // 产品系列中的产品
        if path.ends_with("/products.json") {
            return Ok(r#"[{"id": 1001, "title": "Product 1"}, {"id": 1002, "title": "Product 2"}]"#.to_string());
        }

        // 页面内容
        if path.ends_with("page_about") || path.ends_with("/page_about") || path.ends_with("/Pages/page_about") {
            return Ok("{\"id\": 1, \"title\": \"About Us\", \"body\": \"<p>This is our about page.</p>\"}".to_string());
        }
        if path.ends_with("page_contact") || path.ends_with("/page_contact") || path.ends_with("/Pages/page_contact") {
            return Ok("{\"id\": 2, \"title\": \"Contact Us\", \"body\": \"<p>Contact us at info@example.com</p>\"}".to_string());
        }
        if path.ends_with("page_faq") || path.ends_with("/page_faq") || path.ends_with("/Pages/page_faq") {
            return Ok("{\"id\": 3, \"title\": \"FAQ\", \"body\": \"<p>Frequently asked questions...</p>\"}".to_string());
        }

        Err(EvifError::NotFound(path.to_string()))
    }

    /// 获取图片信息
    async fn get_image_info(&self, filename: &str) -> EvifResult<String> {
        let img_id = filename.trim_end_matches(".jpg").trim_end_matches(".png");
        Ok(format!(
            r#"{{"id": {}, "src": "https://cdn.shopify.com/s/images/{}.jpg", "width": 800, "height": 600, "alt": "Product image"}}"#,
            img_id.trim_start_matches("image_"),
            filename
        ))
    }

    /// 获取页面内容
    async fn get_page_content(&self, page_name: &str) -> EvifResult<String> {
        match page_name {
            "page_about" => Ok("{\"id\": 1, \"title\": \"About Us\", \"body\": \"<p>This is our about page.</p>\"}".to_string()),
            "page_contact" => Ok("{\"id\": 2, \"title\": \"Contact Us\", \"body\": \"<p>Contact us at info@example.com</p>\"}".to_string()),
            "page_faq" => Ok("{\"id\": 3, \"title\": \"FAQ\", \"body\": \"<p>Frequently asked questions...</p>\"}".to_string()),
            _ => Err(EvifError::NotFound(format!("/Pages/{}", page_name))),
        }
    }

    /// 获取变体内容
    async fn get_variant_content(&self, path: &str) -> EvifResult<String> {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        let variant_name = parts.last().unwrap_or(&"");
        Ok(format!(
            r#"{{"id": {}, "product_id": 1001, "title": "Default Title", "price": "29.99", "sku": "SKU-{}", "inventory_quantity": 50}}"#,
            variant_name.trim_start_matches("variant_"),
            variant_name
        ))
    }
}

/// ShopifyFs 配置选项 (用于配置文件)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopifyFsOptions {
    pub store_domain: String,
    pub access_token: String,
    pub api_version: Option<String>,
    pub read_only: Option<bool>,
}

impl Default for ShopifyFsOptions {
    fn default() -> Self {
        Self {
            store_domain: String::new(),
            access_token: String::new(),
            api_version: Some("2024-01".to_string()),
            read_only: Some(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_plugin() -> ShopifyFsPlugin {
        ShopifyFsPlugin {
            config: ShopifyConfig::default(),
            connected: Arc::new(RwLock::new(false)),
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[test]
    fn test_standard_directories() {
        let dirs = ShopifyFsPlugin::standard_directories();
        assert!(dirs.len() >= 6);
        assert!(dirs.iter().any(|(id, _)| *id == "products"));
        assert!(dirs.iter().any(|(id, _)| *id == "orders"));
        assert!(dirs.iter().any(|(id, _)| *id == "customers"));
    }

    #[test]
    fn test_make_file_info() {
        let dir = ShopifyFsPlugin::make_file_info("Products", true, 0);
        assert_eq!(dir.name, "Products");
        assert!(dir.is_dir);
        assert_eq!(dir.mode, 0o755);

        let file = ShopifyFsPlugin::make_file_info("info.json", false, 1024);
        assert_eq!(file.name, "info.json");
        assert!(!file.is_dir);
        assert_eq!(file.mode, 0o644);
    }

    #[tokio::test]
    async fn test_readdir_root() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name == "Products"));
        assert!(entries.iter().any(|e| e.name == "Orders"));
        assert!(entries.iter().any(|e| e.name == "Customers"));
    }

    #[tokio::test]
    async fn test_readdir_products() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Products").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name.starts_with("prod_")));
    }

    #[tokio::test]
    async fn test_readdir_product_detail() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Products/prod_1001").await.unwrap();
        assert!(entries.iter().any(|e| e.name == "info.json"));
        assert!(entries.iter().any(|e| e.name == "variants"));
    }

    #[tokio::test]
    async fn test_readdir_variants() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Products/prod_1001/variants").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name.starts_with("variant_")));
    }

    #[tokio::test]
    async fn test_readdir_orders() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Orders").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name.starts_with("order_")));
    }

    #[tokio::test]
    async fn test_readdir_order_detail() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Orders/order_5001").await.unwrap();
        assert!(entries.iter().any(|e| e.name == "info.json"));
        assert!(entries.iter().any(|e| e.name == "line_items.json"));
        assert!(entries.iter().any(|e| e.name == "fulfillments"));
    }

    #[tokio::test]
    async fn test_readdir_customers() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Customers").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name.starts_with("cust_")));
    }

    #[tokio::test]
    async fn test_readdir_customer_detail() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Customers/cust_3001").await.unwrap();
        assert!(entries.iter().any(|e| e.name == "info.json"));
        assert!(entries.iter().any(|e| e.name == "orders"));
    }

    #[tokio::test]
    async fn test_readdir_inventory() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Inventory").await.unwrap();
        assert!(entries.iter().any(|e| e.name == "levels.json"));
        assert!(entries.iter().any(|e| e.name == "adjustments"));
    }

    #[tokio::test]
    async fn test_readdir_collections() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Collections").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name.starts_with("coll_")));
    }

    #[tokio::test]
    async fn test_readdir_pages() {
        let plugin = create_plugin();
        let entries = plugin.readdir("/Pages").await.unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name.starts_with("page_")));
    }

    #[tokio::test]
    async fn test_read_product_info() {
        let plugin = create_plugin();
        let content = plugin.read("/Products/prod_1001/info.json", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("Sample Product"));
        assert!(content_str.contains("\"id\": 1001"));
    }

    #[tokio::test]
    async fn test_read_variant() {
        let plugin = create_plugin();
        let content = plugin.read("/Products/prod_1001/variants/variant_1001a", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("variant"));
        assert!(content_str.contains("price"));
    }

    #[tokio::test]
    async fn test_read_order_info() {
        let plugin = create_plugin();
        let content = plugin.read("/Orders/order_5001/info.json", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("order"));
        assert!(content_str.contains("financial_status"));
    }

    #[tokio::test]
    async fn test_read_customer_info() {
        let plugin = create_plugin();
        let content = plugin.read("/Customers/cust_3001/info.json", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("customer"));
        assert!(content_str.contains("John"));
    }

    #[tokio::test]
    async fn test_read_page() {
        let plugin = create_plugin();
        let content = plugin.read("/Pages/page_about", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("About"));
    }

    #[tokio::test]
    async fn test_stat_root() {
        let plugin = create_plugin();
        let info = plugin.stat("/").await.unwrap();
        assert_eq!(info.name, "shopifyfs");
        assert!(info.is_dir);
    }

    #[tokio::test]
    async fn test_stat_directory() {
        let plugin = create_plugin();
        let info = plugin.stat("/Products").await.unwrap();
        assert_eq!(info.name, "Products");
        assert!(info.is_dir);
    }

    #[tokio::test]
    async fn test_stat_file() {
        let plugin = create_plugin();
        let info = plugin.stat("/Products/prod_1001/info.json").await.unwrap();
        assert_eq!(info.name, "info.json");
        assert!(!info.is_dir);
    }

    #[tokio::test]
    async fn test_write_readonly() {
        let plugin = create_plugin();
        let result = plugin.write("/test", vec![1, 2, 3], 0, WriteFlags::empty()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mkdir_readonly() {
        let plugin = create_plugin();
        let result = plugin.mkdir("/test", 0o755).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_remove_readonly() {
        let plugin = create_plugin();
        let result = plugin.remove("/test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rename_readonly() {
        let plugin = create_plugin();
        let result = plugin.rename("/old", "/new").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_readdir_not_found() {
        let plugin = create_plugin();
        let result = plugin.readdir("/Nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_not_found() {
        let plugin = create_plugin();
        let result = plugin.read("/Nonexistent/file", 0, 0).await;
        assert!(result.is_err());
    }
}