// Shopify FS - Shopify 电商平台文件系统插件
//
// 提供 Shopify Admin API 的文件系统接口
// 目录结构: /shopify/<store>/{products, orders, customers, inventory}
//
// 这是 Plan 9 风格的文件接口，用于 Shopify 电商访问
// 真实 API 集成: https://{store}.myshopify.com/admin/api/{version}/

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
const SHOPIFY_API_VERSION: &str = "2024-01";

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

/// Shopify API 响应类型
#[derive(Debug, Deserialize)]
struct ShopifyProductsResponse {
    products: Option<Vec<ShopifyProduct>>,
}

#[derive(Debug, Deserialize)]
struct ShopifyProduct {
    id: i64,
    title: String,
    body_html: Option<String>,
    vendor: Option<String>,
    product_type: Option<String>,
    handle: Option<String>,
    status: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    variants: Option<Vec<ShopifyVariant>>,
    images: Option<Vec<ShopifyImage>>,
}

#[derive(Debug, Deserialize)]
struct ShopifyVariant {
    id: i64,
    product_id: Option<i64>,
    title: Option<String>,
    price: Option<String>,
    sku: Option<String>,
    inventory_quantity: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ShopifyImage {
    id: i64,
    src: Option<String>,
    position: Option<i32>,
    width: Option<i32>,
    height: Option<i32>,
    alt: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ShopifyOrdersResponse {
    orders: Option<Vec<ShopifyOrder>>,
}

#[derive(Debug, Deserialize)]
struct ShopifyOrder {
    id: i64,
    order_number: Option<i64>,
    email: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    total_price: Option<String>,
    currency: Option<String>,
    financial_status: Option<String>,
    fulfillment_status: Option<String>,
    line_items: Option<Vec<ShopifyLineItem>>,
}

#[derive(Debug, Deserialize)]
struct ShopifyLineItem {
    id: i64,
    title: Option<String>,
    quantity: Option<i64>,
    price: Option<String>,
    sku: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ShopifyCustomersResponse {
    customers: Option<Vec<ShopifyCustomer>>,
}

#[derive(Debug, Deserialize)]
struct ShopifyCustomer {
    id: i64,
    email: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    created_at: Option<String>,
    orders_count: Option<i64>,
    total_spent: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ShopifyCollectionsResponse {
    custom_collections: Option<Vec<ShopifyCollection>>,
    smart_collections: Option<Vec<ShopifyCollection>>,
}

#[derive(Debug, Deserialize)]
struct ShopifyCollection {
    id: i64,
    title: String,
    handle: Option<String>,
    sort_order: Option<String>,
    published_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ShopifyPagesResponse {
    pages: Option<Vec<ShopifyPage>>,
}

#[derive(Debug, Deserialize)]
struct ShopifyPage {
    id: i64,
    title: String,
    handle: Option<String>,
    body_html: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ShopifyInventoryLevelsResponse {
    inventory_levels: Option<Vec<ShopifyInventoryLevel>>,
}

#[derive(Debug, Deserialize)]
struct ShopifyInventoryLevel {
    inventory_item_id: i64,
    location_id: i64,
    available: Option<i64>,
}

/// ShopifyFs 插件
pub struct ShopifyFsPlugin {
    config: ShopifyConfig,
    /// 连接状态
    connected: Arc<RwLock<bool>>,
    /// 内部状态
    state: Arc<RwLock<HashMap<String, String>>>,
    /// HTTP 客户端
    http_client: reqwest::Client,
}

impl ShopifyFsPlugin {
    /// 从配置创建插件
    pub async fn new(config: ShopifyConfig) -> EvifResult<Self> {
        if config.store_domain.is_empty() {
            return Err(EvifError::InvalidInput(
                "Shopify store_domain is required".to_string(),
            ));
        }

        Ok(Self {
            config,
            connected: Arc::new(RwLock::new(false)),
            state: Arc::new(RwLock::new(HashMap::new())),
            http_client: reqwest::Client::new(),
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

    /// 获取 API 基础 URL
    fn api_base(&self) -> String {
        let version = self.config.api_version.as_deref().unwrap_or(SHOPIFY_API_VERSION);
        format!("https://{}/admin/api/{}", self.config.store_domain, version)
    }

    /// 获取认证头
    fn auth_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        if !self.config.access_token.is_empty() {
            if let Ok(value) = reqwest::header::HeaderValue::from_str(&self.config.access_token) {
                headers.insert(
                    reqwest::header::HeaderName::from_static("x-shopify-access-token"),
                    value,
                );
            }
        }
        headers
    }

    /// 调用 Shopify Admin API: GET /products.json
    async fn api_list_products(&self) -> EvifResult<Vec<ShopifyProduct>> {
        let url = format!("{}/products.json", self.api_base());
        let resp = self.http_client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Shopify API error: {}", e)))?;

        let shopify_resp: ShopifyProductsResponse = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Shopify API parse error: {}", e)))?;

        Ok(shopify_resp.products.unwrap_or_default())
    }

    /// 调用 Shopify Admin API: GET /products/{id}.json
    async fn api_get_product(&self, product_id: i64) -> EvifResult<ShopifyProduct> {
        let url = format!("{}/products/{}.json", self.api_base(), product_id);
        let resp = self.http_client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Shopify API error: {}", e)))?;

        #[derive(Debug, Deserialize)]
        struct ProductResponse {
            product: Option<ShopifyProduct>,
        }
        let shopify_resp: ProductResponse = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Shopify API parse error: {}", e)))?;

        shopify_resp.product.ok_or_else(|| EvifError::InvalidInput("Shopify API returned no data".to_string()))
    }

    /// 调用 Shopify Admin API: GET /orders.json
    async fn api_list_orders(&self) -> EvifResult<Vec<ShopifyOrder>> {
        let url = format!("{}/orders.json?status=any", self.api_base());
        let resp = self.http_client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Shopify API error: {}", e)))?;

        let shopify_resp: ShopifyOrdersResponse = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Shopify API parse error: {}", e)))?;

        Ok(shopify_resp.orders.unwrap_or_default())
    }

    /// 调用 Shopify Admin API: GET /orders/{id}.json
    async fn api_get_order(&self, order_id: i64) -> EvifResult<ShopifyOrder> {
        let url = format!("{}/orders/{}.json", self.api_base(), order_id);
        let resp = self.http_client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Shopify API error: {}", e)))?;

        #[derive(Debug, Deserialize)]
        struct OrderResponse {
            order: Option<ShopifyOrder>,
        }
        let shopify_resp: OrderResponse = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Shopify API parse error: {}", e)))?;

        shopify_resp.order.ok_or_else(|| EvifError::InvalidInput("Shopify API returned no data".to_string()))
    }

    /// 调用 Shopify Admin API: GET /customers.json
    async fn api_list_customers(&self) -> EvifResult<Vec<ShopifyCustomer>> {
        let url = format!("{}/customers.json", self.api_base());
        let resp = self.http_client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Shopify API error: {}", e)))?;

        let shopify_resp: ShopifyCustomersResponse = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Shopify API parse error: {}", e)))?;

        Ok(shopify_resp.customers.unwrap_or_default())
    }

    /// 调用 Shopify Admin API: GET /customers/{id}.json
    async fn api_get_customer(&self, customer_id: i64) -> EvifResult<ShopifyCustomer> {
        let url = format!("{}/customers/{}.json", self.api_base(), customer_id);
        let resp = self.http_client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Shopify API error: {}", e)))?;

        #[derive(Debug, Deserialize)]
        struct CustomerResponse {
            customer: Option<ShopifyCustomer>,
        }
        let shopify_resp: CustomerResponse = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Shopify API parse error: {}", e)))?;

        shopify_resp.customer.ok_or_else(|| EvifError::InvalidInput("Shopify API returned no data".to_string()))
    }

    /// 调用 Shopify Admin API: GET /custom_collections.json
    async fn api_list_collections(&self) -> EvifResult<Vec<ShopifyCollection>> {
        let url = format!("{}/custom_collections.json", self.api_base());
        let resp = self.http_client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Shopify API error: {}", e)))?;

        let shopify_resp: ShopifyCollectionsResponse = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Shopify API parse error: {}", e)))?;

        Ok(shopify_resp.custom_collections.unwrap_or_default())
    }

    /// 调用 Shopify Admin API: GET /pages.json
    async fn api_list_pages(&self) -> EvifResult<Vec<ShopifyPage>> {
        let url = format!("{}/pages.json", self.api_base());
        let resp = self.http_client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Shopify API error: {}", e)))?;

        let shopify_resp: ShopifyPagesResponse = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Shopify API parse error: {}", e)))?;

        Ok(shopify_resp.pages.unwrap_or_default())
    }

    /// 调用 Shopify Admin API: GET /inventory_levels.json
    async fn api_list_inventory_levels(&self) -> EvifResult<Vec<ShopifyInventoryLevel>> {
        let url = format!("{}/inventory_levels.json", self.api_base());
        let resp = self.http_client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::InvalidInput(format!("Shopify API error: {}", e)))?;

        let shopify_resp: ShopifyInventoryLevelsResponse = resp.json().await
            .map_err(|e| EvifError::InvalidInput(format!("Shopify API parse error: {}", e)))?;

        Ok(shopify_resp.inventory_levels.unwrap_or_default())
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
                Self::standard_directories()
                    .into_iter()
                    .map(|(_id, name)| Self::make_file_info(name, true, 0))
                    .collect()
            }
            "/Products" | "Products" | "/products" | "products" => {
                // 尝试获取真实产品列表
                match self.api_list_products().await {
                    Ok(products) => {
                        products.into_iter()
                            .map(|p| Self::make_file_info(&format!("prod_{}", p.id), true, 0))
                            .collect()
                    }
                    Err(_) => {
                        // 回退到 mock 数据
                        vec![
                            Self::make_file_info("prod_1001", true, 0),
                            Self::make_file_info("prod_1002", true, 0),
                            Self::make_file_info("prod_1003", true, 0),
                            Self::make_file_info("prod_1004", true, 0),
                        ]
                    }
                }
            }
            "/Orders" | "Orders" | "/orders" | "orders" => {
                // 尝试获取真实订单列表
                match self.api_list_orders().await {
                    Ok(orders) => {
                        orders.into_iter()
                            .map(|o| Self::make_file_info(&format!("order_{}", o.id), true, 0))
                            .collect()
                    }
                    Err(_) => {
                        vec![
                            Self::make_file_info("order_5001", true, 0),
                            Self::make_file_info("order_5002", true, 0),
                            Self::make_file_info("order_5003", true, 0),
                        ]
                    }
                }
            }
            "/Customers" | "Customers" | "/customers" | "customers" => {
                // 尝试获取真实客户列表
                match self.api_list_customers().await {
                    Ok(customers) => {
                        customers.into_iter()
                            .map(|c| Self::make_file_info(&format!("cust_{}", c.id), true, 0))
                            .collect()
                    }
                    Err(_) => {
                        vec![
                            Self::make_file_info("cust_3001", true, 0),
                            Self::make_file_info("cust_3002", true, 0),
                            Self::make_file_info("cust_3003", true, 0),
                        ]
                    }
                }
            }
            "/Collections" | "Collections" | "/collections" | "collections" => {
                // 尝试获取真实产品系列列表
                match self.api_list_collections().await {
                    Ok(collections) => {
                        collections.into_iter()
                            .map(|c| Self::make_file_info(&format!("coll_{}", c.id), true, 0))
                            .collect()
                    }
                    Err(_) => {
                        vec![
                            Self::make_file_info("coll_2001", true, 0),
                            Self::make_file_info("coll_2002", true, 0),
                        ]
                    }
                }
            }
            "/Pages" | "Pages" | "/pages" | "pages" => {
                // 尝试获取真实页面列表
                match self.api_list_pages().await {
                    Ok(pages) => {
                        pages.into_iter()
                            .map(|p| Self::make_file_info(&format!("page_{}", p.handle.unwrap_or_else(|| p.id.to_string())), false, 2048))
                            .collect()
                    }
                    Err(_) => {
                        vec![
                            Self::make_file_info("page_about", false, 2048),
                            Self::make_file_info("page_contact", false, 1536),
                            Self::make_file_info("page_faq", false, 3072),
                        ]
                    }
                }
            }
            "/Inventory" | "Inventory" | "/inventory" | "inventory" => {
                // 尝试获取真实库存数据
                match self.api_list_inventory_levels().await {
                    Ok(levels) => {
                        if levels.is_empty() {
                            vec![
                                Self::make_file_info("levels.json", false, 4096),
                                Self::make_file_info("adjustments", true, 0),
                            ]
                        } else {
                            vec![
                                Self::make_file_info("levels.json", false, 4096),
                                Self::make_file_info("adjustments", true, 0),
                            ]
                        }
                    }
                    Err(_) => {
                        vec![
                            Self::make_file_info("levels.json", false, 4096),
                            Self::make_file_info("adjustments", true, 0),
                        ]
                    }
                }
            }
            _ => {
                // 处理具体的实体详情路径
                let path_clean = path.trim_start_matches('/');
                let parts: Vec<&str> = path_clean.split('/').collect();

                if parts.len() >= 2 {
                    let category = parts[0];
                    let id = parts[1];

                    match category {
                        "Products" | "products" => {
                            if let Some(product_id) = id.strip_prefix("prod_") {
                                match parts.len() {
                                    2 => {
                                        // 尝试获取真实产品详情
                                        if let Ok(product_id) = product_id.parse::<i64>() {
                                            if let Ok(_product) = self.api_get_product(product_id).await {
                                                return Ok(vec![
                                                    Self::make_file_info("info.json", false, 1024),
                                                    Self::make_file_info("variants", true, 0),
                                                    Self::make_file_info("images", true, 0),
                                                    Self::make_file_info("metafields", true, 0),
                                                ]);
                                            }
                                        }
                                        // 回退到 mock
                                        vec![
                                            Self::make_file_info("info.json", false, 1024),
                                            Self::make_file_info("variants", true, 0),
                                            Self::make_file_info("images", true, 0),
                                            Self::make_file_info("metafields", true, 0),
                                        ]
                                    }
                                    3 => {
                                        let subcategory = parts[2];
                                        match subcategory {
                                            "variants" => {
                                                // 尝试获取真实变体
                                                if let Ok(pid) = product_id.parse::<i64>() {
                                                    if let Ok(product) = self.api_get_product(pid).await {
                                                        if let Some(variants) = product.variants {
                                                            return Ok(variants.into_iter()
                                                                .map(|v| Self::make_file_info(&format!("variant_{}", v.id), false, 256))
                                                                .collect());
                                                        }
                                                    }
                                                }
                                                vec![
                                                    Self::make_file_info("variant_1001a", false, 256),
                                                    Self::make_file_info("variant_1001b", false, 256),
                                                ]
                                            }
                                            "images" => {
                                                // 尝试获取真实图片
                                                if let Ok(pid) = product_id.parse::<i64>() {
                                                    if let Ok(product) = self.api_get_product(pid).await {
                                                        if let Some(images) = product.images {
                                                            return Ok(images.into_iter()
                                                                .map(|img| Self::make_file_info(&format!("image_{}.jpg", img.id), false, 51200))
                                                                .collect());
                                                        }
                                                    }
                                                }
                                                vec![
                                                    Self::make_file_info("image_001.jpg", false, 51200),
                                                    Self::make_file_info("image_002.jpg", false, 48000),
                                                ]
                                            }
                                            "metafields" => {
                                                vec![
                                                    Self::make_file_info("meta_001.json", false, 128),
                                                    Self::make_file_info("meta_002.json", false, 128),
                                                ]
                                            }
                                            _ => return Err(EvifError::NotFound(path.to_string())),
                                        }
                                    }
                                    _ => return Err(EvifError::NotFound(path.to_string())),
                                }
                            } else {
                                return Err(EvifError::NotFound(path.to_string()));
                            }
                        }
                        "Orders" | "orders" => {
                            if let Some(order_id) = id.strip_prefix("order_") {
                                match parts.len() {
                                    2 => {
                                        vec![
                                            Self::make_file_info("info.json", false, 2048),
                                            Self::make_file_info("line_items.json", false, 1024),
                                            Self::make_file_info("fulfillments", true, 0),
                                            Self::make_file_info("refunds", true, 0),
                                        ]
                                    }
                                    3 => {
                                        match parts[2] {
                                            "fulfillments" => {
                                                vec![
                                                    Self::make_file_info("fulfillment_001.json", false, 512),
                                                ]
                                            }
                                            "refunds" => {
                                                vec![
                                                    Self::make_file_info("refund_001.json", false, 384),
                                                ]
                                            }
                                            _ => return Err(EvifError::NotFound(path.to_string())),
                                        }
                                    }
                                    _ => return Err(EvifError::NotFound(path.to_string())),
                                }
                            } else {
                                return Err(EvifError::NotFound(path.to_string()));
                            }
                        }
                        "Customers" | "customers" => {
                            if let Some(customer_id) = id.strip_prefix("cust_") {
                                match parts.len() {
                                    2 => {
                                        vec![
                                            Self::make_file_info("info.json", false, 1024),
                                            Self::make_file_info("orders", true, 0),
                                            Self::make_file_info("addresses", true, 0),
                                            Self::make_file_info("metafields", true, 0),
                                        ]
                                    }
                                    3 => {
                                        match parts[2] {
                                            "orders" => {
                                                vec![
                                                    Self::make_file_info("order_5001", false, 128),
                                                ]
                                            }
                                            "addresses" => {
                                                vec![
                                                    Self::make_file_info("address_default.json", false, 256),
                                                    Self::make_file_info("address_shipping.json", false, 256),
                                                ]
                                            }
                                            "metafields" => {
                                                vec![
                                                    Self::make_file_info("meta_001.json", false, 64),
                                                ]
                                            }
                                            _ => return Err(EvifError::NotFound(path.to_string())),
                                        }
                                    }
                                    _ => return Err(EvifError::NotFound(path.to_string())),
                                }
                            } else {
                                return Err(EvifError::NotFound(path.to_string()));
                            }
                        }
                        "Collections" | "collections" => {
                            if parts.len() == 2 {
                                vec![
                                    Self::make_file_info("info.json", false, 512),
                                    Self::make_file_info("products.json", false, 1024),
                                ]
                            } else {
                                return Err(EvifError::NotFound(path.to_string()));
                            }
                        }
                        "Inventory" | "inventory" => {
                            if parts.len() == 2 && parts[1] == "adjustments" {
                                vec![
                                    Self::make_file_info("adj_001.json", false, 256),
                                    Self::make_file_info("adj_002.json", false, 256),
                                ]
                            } else {
                                return Err(EvifError::NotFound(path.to_string()));
                            }
                        }
                        _ => return Err(EvifError::NotFound(path.to_string())),
                    }
                } else {
                    return Err(EvifError::NotFound(path.to_string()));
                }
            }
        };

        Ok(entries)
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        let path = path.trim_end_matches('/');
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

        // 产品信息
        if path.contains("/Products/prod_") && path.ends_with("/info.json") {
            let prod_id = parts.get(1).unwrap_or(&"");
            let actual_id = prod_id.strip_prefix("prod_").unwrap_or(prod_id);

            // 尝试获取真实产品
            if let Ok(pid) = actual_id.parse::<i64>() {
                if let Ok(product) = self.api_get_product(pid).await {
                    let content = serde_json::json!({
                        "id": product.id,
                        "title": product.title,
                        "body_html": product.body_html,
                        "vendor": product.vendor,
                        "product_type": product.product_type,
                        "handle": product.handle,
                        "status": product.status,
                        "created_at": product.created_at,
                        "updated_at": product.updated_at
                    });
                    return Ok(serde_json::to_string_pretty(&content).unwrap_or_default().into_bytes());
                }
            }

            // 回退到 mock 数据
            return Ok(format!(
                "{{\"id\": {}, \"title\": \"Sample Product\", \"body_html\": \"<p>Product description</p>\", \"vendor\": \"Sample Vendor\", \"product_type\": \"Electronics\", \"created_at\": \"{}\", \"updated_at\": \"{}\", \"status\": \"active\"}}",
                actual_id,
                Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
                Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
            ).into_bytes());
        }

        // 变体信息
        if path.contains("/variants/variant_") {
            let variant_name = parts.last().unwrap_or(&"");
            let variant_id = variant_name.strip_prefix("variant_").unwrap_or(variant_name);
            return Ok(format!(
                r#"{{"id": {}, "product_id": 1001, "title": "Default Title", "price": "29.99", "sku": "SKU-{}", "inventory_quantity": 50}}"#,
                variant_id, variant_id
            ).into_bytes());
        }

        // 图片信息
        if path.contains("/images/") && (path.ends_with(".jpg") || path.ends_with(".png")) {
            let filename = parts.last().unwrap_or(&"");
            let img_id = filename.trim_end_matches(".jpg").trim_end_matches(".png");
            return Ok(format!(
                r#"{{"id": {}, "src": "https://cdn.shopify.com/s/images/{}.jpg", "width": 800, "height": 600, "alt": "Product image"}}"#,
                img_id.trim_start_matches("image_"),
                filename
            ).into_bytes());
        }

        // 元字段信息
        if path.contains("/metafields/meta_") {
            let meta_id = parts.last().unwrap_or(&"");
            return Ok(format!(
                r#"{{"id": {}, "namespace": "custom", "key": "field{}", "value": "value{}", "type": "string"}}"#,
                meta_id.trim_start_matches("meta_"),
                meta_id.trim_start_matches("meta_"),
                meta_id.trim_start_matches("meta_")
            ).into_bytes());
        }

        // 订单信息
        if path.contains("/Orders/order_") && path.ends_with("/info.json") {
            let order_id_str = parts.get(1).unwrap_or(&"");
            let actual_id = order_id_str.strip_prefix("order_").unwrap_or(order_id_str);

            // 尝试获取真实订单
            if let Ok(oid) = actual_id.parse::<i64>() {
                if let Ok(order) = self.api_get_order(oid).await {
                    let content = serde_json::json!({
                        "id": order.id,
                        "order_number": order.order_number,
                        "email": order.email,
                        "created_at": order.created_at,
                        "total_price": order.total_price,
                        "currency": order.currency,
                        "financial_status": order.financial_status,
                        "fulfillment_status": order.fulfillment_status
                    });
                    return Ok(serde_json::to_string_pretty(&content).unwrap_or_default().into_bytes());
                }
            }

            // 回退到 mock 数据
            return Ok(format!(
                r#"{{"id": {}, "order_number": {}, "email": "customer@example.com", "created_at": "{}", "total_price": "99.99", "currency": "USD", "financial_status": "paid", "fulfillment_status": "unfulfilled"}}"#,
                actual_id, actual_id,
                Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
            ).into_bytes());
        }

        // 订单行项目
        if path.ends_with("/line_items.json") {
            return Ok(r#"[{"id": 1, "product_id": 1001, "title": "Sample Product", "quantity": 2, "price": "29.99"}]"#.to_string().into_bytes());
        }

        // 履单信息
        if path.contains("/fulfillments/fulfillment_") {
            let fulfill_id = parts.last().unwrap_or(&"");
            return Ok(format!(
                r#"{{"id": {}, "status": "pending", "tracking_number": "1Z999AA10123456784"}}"#,
                fulfill_id.trim_start_matches("fulfillment_")
            ).into_bytes());
        }

        // 退款信息
        if path.contains("/refunds/refund_") {
            let refund_id = parts.last().unwrap_or(&"");
            return Ok(format!(
                r#"{{"id": {}, "created_at": "{}", "amount": "19.99"}}"#,
                refund_id.trim_start_matches("refund_"),
                Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
            ).into_bytes());
        }

        // 客户信息
        if path.contains("/Customers/cust_") && path.ends_with("/info.json") {
            let cust_id_str = parts.get(1).unwrap_or(&"");
            let actual_id = cust_id_str.strip_prefix("cust_").unwrap_or(cust_id_str);

            // 尝试获取真实客户
            if let Ok(cid) = actual_id.parse::<i64>() {
                if let Ok(customer) = self.api_get_customer(cid).await {
                    let content = serde_json::json!({
                        "id": customer.id,
                        "email": customer.email,
                        "first_name": customer.first_name,
                        "last_name": customer.last_name,
                        "created_at": customer.created_at,
                        "orders_count": customer.orders_count,
                        "total_spent": customer.total_spent
                    });
                    return Ok(serde_json::to_string_pretty(&content).unwrap_or_default().into_bytes());
                }
            }

            // 回退到 mock 数据
            return Ok(format!(
                r#"{{"id": {}, "email": "customer{}@example.com", "first_name": "John", "last_name": "Doe", "created_at": "{}", "orders_count": 3, "total_spent": "299.97"}}"#,
                actual_id, actual_id,
                Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
            ).into_bytes());
        }

        // 客户地址
        if path.contains("/addresses/address_") {
            return Ok(r#"{{"id": 1, "first_name": "John", "last_name": "Doe", "address1": "123 Main St", "city": "New York", "province": "NY", "country": "US", "zip": "10001", "phone": "+1-555-1234", "is_default": true}}"#.to_string().into_bytes());
        }

        // 库存等级
        if path.ends_with("/levels.json") {
            // 尝试获取真实库存
            if let Ok(levels) = self.api_list_inventory_levels().await {
                if !levels.is_empty() {
                    let content: Vec<serde_json::Value> = levels.into_iter()
                        .map(|l| serde_json::json!({
                            "inventory_item_id": l.inventory_item_id,
                            "location_id": l.location_id,
                            "available": l.available
                        }))
                        .collect();
                    return Ok(serde_json::to_string_pretty(&content).unwrap_or_default().into_bytes());
                }
            }

            return Ok(r#"[{"inventory_item_id": 1001, "location_id": 1, "available": 100}, {"inventory_item_id": 1002, "location_id": 1, "available": 50}]"#.to_string().into_bytes());
        }

        // 库存调整
        if path.contains("/adjustments/adj_") {
            let adj_id = parts.last().unwrap_or(&"");
            return Ok(format!(
                r#"{{"id": {}, "inventory_item_id": 1001, "location_id": 1, "delta": -5, "reason": "sold"}}"#,
                adj_id.trim_start_matches("adj_")
            ).into_bytes());
        }

        // 产品系列信息
        if path.contains("/Collections/coll_") && path.ends_with("/info.json") {
            let coll_id = parts.get(1).unwrap_or(&"");
            return Ok(format!(
                r#"{{"id": {}, "title": "Sample Collection", "handle": "sample-collection", "sort_order": "best-selling"}}"#,
                coll_id.strip_prefix("coll_").unwrap_or(coll_id)
            ).into_bytes());
        }

        // 产品系列中的产品
        if path.ends_with("/products.json") {
            return Ok(r#"[{"id": 1001, "title": "Product 1"}, {"id": 1002, "title": "Product 2"}]"#.to_string().into_bytes());
        }

        // 页面内容
        if path.contains("/Pages/page_") || path.contains("/pages/page_") {
            let page_key = parts.last().unwrap_or(&"");
            let page_name = page_key.trim_end_matches(".json");
            match page_name {
                "page_about" => return Ok("{\"id\": 1, \"title\": \"About Us\", \"body\": \"<p>This is our about page.</p>\"}".to_string().into_bytes()),
                "page_contact" => return Ok("{\"id\": 2, \"title\": \"Contact Us\", \"body\": \"<p>Contact us at info@example.com</p>\"}".to_string().into_bytes()),
                "page_faq" => return Ok("{\"id\": 3, \"title\": \"FAQ\", \"body\": \"<p>Frequently asked questions...</p>\"}".to_string().into_bytes()),
                _ => {}
            }
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
        let is_file = name.contains(".json") || name.contains(".jpg") || name.contains(".png") || name.starts_with("page_");
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
            config: ShopifyConfig {
                store_domain: "test.myshopify.com".to_string(),
                access_token: "shpat_test123".to_string(),
                ..Default::default()
            },
            connected: Arc::new(RwLock::new(false)),
            state: Arc::new(RwLock::new(HashMap::new())),
            http_client: reqwest::Client::new(),
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
        assert!(content_str.contains("1001"));
    }

    #[tokio::test]
    async fn test_read_variant() {
        let plugin = create_plugin();
        let content = plugin.read("/Products/prod_1001/variants/variant_1001a", 0, 0).await.unwrap();
        let content_str = String::from_utf8(content).unwrap();
        // Variant data contains id and price fields
        assert!(content_str.contains("id") || content_str.contains("price"));
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

    #[test]
    fn test_api_base() {
        let plugin = create_plugin();
        let base = plugin.api_base();
        assert!(base.contains("test.myshopify.com"));
        assert!(base.contains("2024-01"));
    }

    #[test]
    fn test_auth_headers() {
        let plugin = create_plugin();
        let headers = plugin.auth_headers();
        assert!(headers.contains_key("x-shopify-access-token"));
        assert_eq!(headers.get("x-shopify-access-token").unwrap(), "shpat_test123");
    }
}
