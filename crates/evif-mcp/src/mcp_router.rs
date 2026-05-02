// MCP Router - 路径路由系统
//
// 实现 MCP Resource URI ↔ VFS Path 的双向转换
// 支持动态路径模式和规则匹配

use regex_lite::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use thiserror::Error;

/// 路由错误
#[derive(Error, Debug)]
pub enum RouterError {
    #[error("Path pattern mismatch: {0}")]
    PatternMismatch(String),

    #[error("Invalid URI: {0}")]
    InvalidUri(String),

    #[error("No matching route: {0}")]
    NoMatchingRoute(String),

    #[error("Invalid pattern: {0}")]
    InvalidPattern(String),
}

/// 路由方向
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RouteDirection {
    /// MCP Resource URI → VFS Path
    UriToPath,
    /// VFS Path → MCP Resource URI
    PathToUri,
}

/// 路由规则
#[derive(Debug, Clone)]
pub struct RouteRule {
    /// 源模式（支持正则或通配符）
    pub source_pattern: String,
    /// 目标模式（支持变量替换）
    pub target_pattern: String,
    /// 路由方向
    pub direction: RouteDirection,
    /// 是否启用
    pub enabled: bool,
    /// 优先级（数字越大优先级越高）
    pub priority: u32,
}

impl RouteRule {
    /// 创建新的路由规则
    pub fn new(source: impl Into<String>, target: impl Into<String>, direction: RouteDirection) -> Self {
        Self {
            source_pattern: source.into(),
            target_pattern: target.into(),
            direction,
            enabled: true,
            priority: 0,
        }
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// 启用/禁用
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// 路由匹配结果
#[derive(Debug, Clone)]
pub struct RouteMatch {
    /// 匹配的目标路径/URI
    pub target: String,
    /// 捕获的变量
    pub variables: HashMap<String, String>,
}

/// MCP Router - 路径路由系统
pub struct McpRouter {
    /// URI → Path 路由规则（按优先级排序）
    uri_to_path_rules: Arc<RwLock<Vec<RouteRule>>>,
    /// Path → URI 路由规则（按优先级排序）
    path_to_uri_rules: Arc<RwLock<Vec<RouteRule>>>,
    /// 预编译的正则缓存
    regex_cache: Arc<RwLock<HashMap<String, Regex>>>,
    /// 默认前缀映射
    default_prefixes: Arc<RwLock<HashMap<String, String>>>,
}

impl McpRouter {
    /// 创建新的 Router
    pub fn new() -> Self {
        let router = Self {
            uri_to_path_rules: Arc::new(RwLock::new(Vec::new())),
            path_to_uri_rules: Arc::new(RwLock::new(Vec::new())),
            regex_cache: Arc::new(RwLock::new(HashMap::new())),
            default_prefixes: Arc::new(RwLock::new(HashMap::new())),
        };

        // 注册默认前缀映射
        router.register_default_prefixes();

        router
    }

    /// 注册默认前缀映射
    fn register_default_prefixes(&self) {
        let mut prefixes = self.default_prefixes.write().unwrap();

        // EVIF 内部路径
        prefixes.insert("file:///context".to_string(), "/context".to_string());
        prefixes.insert("file:///skills".to_string(), "/skills".to_string());
        prefixes.insert("file:///pipes".to_string(), "/pipes".to_string());
        prefixes.insert("file:///memories".to_string(), "/memories".to_string());
        prefixes.insert("file:///queue".to_string(), "/queue".to_string());

        // MCP 服务器前缀
        prefixes.insert("github://".to_string(), "/mcp/github".to_string());
        prefixes.insert("slack://".to_string(), "/mcp/slack".to_string());
        prefixes.insert("notion://".to_string(), "/mcp/notion".to_string());
        prefixes.insert("postgres://".to_string(), "/mcp/postgres".to_string());
        prefixes.insert("s3://".to_string(), "/mcp/s3".to_string());
    }

    /// 添加路由规则
    pub fn add_rule(&self, rule: RouteRule) {
        let mut rules = match rule.direction {
            RouteDirection::UriToPath => self.uri_to_path_rules.write().unwrap(),
            RouteDirection::PathToUri => self.path_to_uri_rules.write().unwrap(),
        };

        rules.push(rule);
        rules.sort_by(|a, b| b.priority.cmp(&a.priority)); // 按优先级降序
    }

    /// 批量添加规则
    pub fn add_rules(&self, rules: impl IntoIterator<Item = RouteRule>) {
        for rule in rules {
            self.add_rule(rule);
        }
    }

    /// 添加 URI → Path 路由
    pub fn add_uri_to_path(&self, source: impl Into<String>, target: impl Into<String>) {
        self.add_rule(RouteRule::new(source, target, RouteDirection::UriToPath));
    }

    /// 添加 Path → URI 路由
    pub fn add_path_to_uri(&self, source: impl Into<String>, target: impl Into<String>) {
        self.add_rule(RouteRule::new(source, target, RouteDirection::PathToUri));
    }

    /// 转换 MCP Resource URI 到 VFS 路径
    pub fn uri_to_path(&self, uri: &str) -> Result<String, RouterError> {
        // 先尝试前缀匹配（快速路径）
        if let Some(path) = self.try_prefix_match(uri) {
            return Ok(path);
        }

        // 尝试规则匹配
        let rules = self.uri_to_path_rules.read().unwrap();
        for rule in rules.iter() {
            if !rule.enabled {
                continue;
            }

            if let Ok(target) = self.apply_rule(uri, rule) {
                return Ok(target);
            }
        }

        // 默认：移除 file:// 前缀
        if uri.starts_with("file://") {
            return Ok(uri.strip_prefix("file://").unwrap_or(uri).to_string());
        }

        Err(RouterError::NoMatchingRoute(format!(
            "No route found for URI: {}",
            uri
        )))
    }

    /// 转换 VFS 路径到 MCP Resource URI
    pub fn path_to_uri(&self, path: &str) -> Result<String, RouterError> {
        // 先尝试前缀匹配
        if let Some(uri) = self.try_reverse_prefix_match(path) {
            return Ok(uri);
        }

        // 尝试规则匹配
        let rules = self.path_to_uri_rules.read().unwrap();
        for rule in rules.iter() {
            if !rule.enabled {
                continue;
            }

            if let Ok(target) = self.apply_rule(path, rule) {
                return Ok(target);
            }
        }

        // 默认：添加 file:// 前缀
        Ok(format!("file://{}", path))
    }

    /// 尝试前缀匹配
    fn try_prefix_match(&self, uri: &str) -> Option<String> {
        let prefixes = self.default_prefixes.read().unwrap();

        // 按长度降序排序，优先匹配最长前缀
        let mut sorted: Vec<_> = prefixes.iter().collect();
        sorted.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        for (prefix, replacement) in sorted {
            if uri.starts_with(prefix) {
                let suffix = &uri[prefix.len()..];
                // 确保正确添加路径分隔符
                let result = if suffix.is_empty() {
                    replacement.clone()
                } else if suffix.starts_with('/') {
                    format!("{}{}", replacement, suffix)
                } else {
                    format!("{}/{}", replacement, suffix)
                };
                return Some(result);
            }
        }

        None
    }

    /// 尝试反向前缀匹配
    fn try_reverse_prefix_match(&self, path: &str) -> Option<String> {
        let prefixes = self.default_prefixes.read().unwrap();

        // 按长度降序排序
        let mut sorted: Vec<_> = prefixes.iter().collect();
        sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

        for (uri_prefix, path_prefix) in sorted {
            if path == path_prefix {
                // 精确匹配
                return Some(uri_prefix.clone());
            }
            if path.starts_with(path_prefix) && path_prefix != "/" {
                let suffix = &path[path_prefix.len()..];
                // 构建 URI：确保正确处理分隔符
                // 如果 replacement 已以 / 结尾，而 suffix 以 / 开头，则去掉 suffix 的前导 /
                let clean_suffix = if suffix.starts_with('/') && uri_prefix.ends_with('/') {
                    &suffix[1..]
                } else {
                    suffix
                };

                let result = if clean_suffix.is_empty() {
                    uri_prefix.clone()
                } else {
                    format!("{}{}", uri_prefix, clean_suffix)
                };
                return Some(result);
            }
        }

        None
    }

    /// 应用规则
    fn apply_rule(&self, input: &str, rule: &RouteRule) -> Result<String, RouterError> {
        // 获取或编译正则
        let regex = self.get_or_compile_regex(&rule.source_pattern)?;

        // 尝试匹配
        let caps = regex.captures(input);

        if let Some(captures) = caps {
            let mut result = rule.target_pattern.clone();

            // 获取完整匹配
            let full_match = captures.get(0).map(|m| m.as_str()).unwrap_or("");

            // 替换 $& (完整匹配)
            result = result.replace("$&", full_match);

            // 替换捕获组 ($1, $2, 等)
            for (i, cap) in captures.iter().enumerate() {
                if let Some(m) = cap {
                    let placeholder = format!("${}", i);
                    result = result.replace(&placeholder, m.as_str());
                }
            }

            // 替换命名捕获组
            for name in regex.capture_names() {
                if let Some(name) = name {
                    if let Some(m) = captures.name(name) {
                        let placeholder = format!("{{{}}}", name);
                        result = result.replace(&placeholder, m.as_str());
                    }
                }
            }

            Ok(result)
        } else {
            Err(RouterError::PatternMismatch(format!(
                "Pattern '{}' does not match '{}'",
                rule.source_pattern, input
            )))
        }
    }

    /// 获取或编译正则
    fn get_or_compile_regex(&self, pattern: &str) -> Result<Regex, RouterError> {
        // 检查缓存
        {
            let cache = self.regex_cache.read().unwrap();
            if let Some(regex) = cache.get(pattern) {
                return Ok(regex.clone());
            }
        }

        // 编译新的正则
        let regex = Regex::new(pattern)
            .map_err(|e| RouterError::InvalidPattern(e.to_string()))?;

        // 缓存
        {
            let mut cache = self.regex_cache.write().unwrap();
            cache.insert(pattern.to_string(), regex.clone());
        }

        Ok(regex)
    }

    /// 列出所有 URI → Path 规则
    pub fn list_uri_to_path_rules(&self) -> Vec<RouteRule> {
        self.uri_to_path_rules
            .read()
            .unwrap()
            .iter()
            .filter(|r| r.enabled)
            .cloned()
            .collect()
    }

    /// 列出所有 Path → URI 规则
    pub fn list_path_to_uri_rules(&self) -> Vec<RouteRule> {
        self.path_to_uri_rules
            .read()
            .unwrap()
            .iter()
            .filter(|r| r.enabled)
            .cloned()
            .collect()
    }

    /// 列出所有前缀映射
    pub fn list_prefixes(&self) -> HashMap<String, String> {
        self.default_prefixes.read().unwrap().clone()
    }

    /// 添加前缀映射
    pub fn add_prefix(&self, uri_prefix: impl Into<String>, path_prefix: impl Into<String>) {
        let mut prefixes = self.default_prefixes.write().unwrap();
        prefixes.insert(uri_prefix.into(), path_prefix.into());
    }

    /// 移除前缀映射
    pub fn remove_prefix(&self, uri_prefix: &str) {
        let mut prefixes = self.default_prefixes.write().unwrap();
        prefixes.remove(uri_prefix);
    }

    /// 清除所有规则
    pub fn clear_rules(&self) {
        self.uri_to_path_rules.write().unwrap().clear();
        self.path_to_uri_rules.write().unwrap().clear();
        self.regex_cache.write().unwrap().clear();
    }

    /// 从配置加载规则
    pub fn load_from_config(&self, config: &McpRouterConfig) {
        // 添加资源映射规则（使用精确前缀匹配）
        for (uri_prefix, path) in &config.resources {
            self.add_prefix(uri_prefix, path);
        }

        // 添加工具映射规则
        for (tool_name, mapping) in &config.tools {
            // 添加工具名到操作的映射
            self.add_uri_to_path(tool_name, &mapping.operation);
        }
    }
}

impl Default for McpRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// MCP Router 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRouterConfig {
    /// 资源映射
    pub resources: HashMap<String, String>,
    /// 工具映射
    pub tools: HashMap<String, ToolMapping>,
    /// Prompts 映射
    pub prompts: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMapping {
    pub operation: String,
    #[serde(default)]
    pub path_param: Option<String>,
    #[serde(default)]
    pub content_param: Option<String>,
}

impl Default for McpRouterConfig {
    fn default() -> Self {
        let mut resources = HashMap::new();
        resources.insert("file:///context".to_string(), "/context".to_string());
        resources.insert("file:///skills".to_string(), "/skills".to_string());
        resources.insert("file:///pipes".to_string(), "/pipes".to_string());
        resources.insert("github://".to_string(), "/mcp/github".to_string());
        resources.insert("notion://".to_string(), "/mcp/notion".to_string());

        let mut tools = HashMap::new();
        tools.insert(
            "evif_ls".to_string(),
            ToolMapping {
                operation: "readdir".to_string(),
                path_param: Some("path".to_string()),
                content_param: None,
            },
        );
        tools.insert(
            "evif_cat".to_string(),
            ToolMapping {
                operation: "read".to_string(),
                path_param: Some("path".to_string()),
                content_param: None,
            },
        );

        Self {
            resources,
            tools,
            prompts: HashMap::new(),
        }
    }
}

/// 路由统计
#[derive(Debug, Clone, Default)]
pub struct RouterStats {
    pub uri_to_path_count: usize,
    pub path_to_uri_count: usize,
    pub prefix_count: usize,
    pub regex_cache_size: usize,
}

impl McpRouter {
    /// 获取路由统计
    pub fn stats(&self) -> RouterStats {
        RouterStats {
            uri_to_path_count: self.uri_to_path_rules.read().unwrap().len(),
            path_to_uri_count: self.path_to_uri_rules.read().unwrap().len(),
            prefix_count: self.default_prefixes.read().unwrap().len(),
            regex_cache_size: self.regex_cache.read().unwrap().len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_uri_to_path() {
        let router = McpRouter::new();

        assert_eq!(router.uri_to_path("file:///context/L0/current").unwrap(), "/context/L0/current");
        assert_eq!(router.uri_to_path("file:///skills/code-review/SKILL.md").unwrap(), "/skills/code-review/SKILL.md");
        assert_eq!(router.uri_to_path("file:///pipes/agent-1/output").unwrap(), "/pipes/agent-1/output");
    }

    #[test]
    fn test_github_uri_to_path() {
        let router = McpRouter::new();

        assert_eq!(router.uri_to_path("github://owner/repo/issues").unwrap(), "/mcp/github/owner/repo/issues");
        assert_eq!(router.uri_to_path("github://octocat/Hello-World").unwrap(), "/mcp/github/octocat/Hello-World");
    }

    #[test]
    fn test_basic_path_to_uri() {
        let router = McpRouter::new();

        assert_eq!(router.path_to_uri("/context/L0/current").unwrap(), "file:///context/L0/current");
        assert_eq!(router.path_to_uri("/skills/code-review").unwrap(), "file:///skills/code-review");
        assert_eq!(router.path_to_uri("/mcp/github/owner/repo").unwrap(), "github://owner/repo");
    }

    #[test]
    fn test_add_custom_prefix() {
        let router = McpRouter::new();
        router.add_prefix("custom://", "/custom");

        assert_eq!(router.uri_to_path("custom://data/file.txt").unwrap(), "/custom/data/file.txt");
        assert_eq!(router.path_to_uri("/custom/data/file.txt").unwrap(), "custom://data/file.txt");
    }

    #[test]
    fn test_add_custom_rule() {
        let router = McpRouter::new();

        // 移除默认 s3:// 前缀（会映射到 /mcp/s3）
        router.remove_prefix("s3://");

        // 添加自定义正则规则，替换默认的 s3 前缀映射
        router.add_uri_to_path(r"^s3://([^/]+)/(.+)$", "/s3/$1/$2");

        assert_eq!(router.uri_to_path("s3://bucket/key").unwrap(), "/s3/bucket/key");
        assert_eq!(router.uri_to_path("s3://my-bucket/path/to/file").unwrap(), "/s3/my-bucket/path/to/file");
    }

    #[test]
    fn test_rule_priority() {
        let router = McpRouter::new();

        // 添加规则：高优先级先匹配
        router.add_rule(RouteRule::new(r"^/specific/.*$", "/override$&", RouteDirection::PathToUri)
            .with_priority(10));

        router.add_uri_to_path(r"^/specific/.*$", "/default$&");
        // 默认前缀会匹配所有 /specific 开头的路径

        // 高优先级规则应该先匹配
        assert_eq!(router.path_to_uri("/specific/path").unwrap(), "/override/specific/path");
    }

    #[test]
    fn test_no_matching_route() {
        let router = McpRouter::new();

        // 清除默认前缀
        router.default_prefixes.write().unwrap().clear();

        let result = router.uri_to_path("unknown://something");
        assert!(result.is_err());
    }

    #[test]
    fn test_router_stats() {
        let router = McpRouter::new();

        router.add_uri_to_path(r"^pattern1$", "/result1");
        router.add_path_to_uri(r"^pattern2$", "result2");
        router.add_prefix("test://", "/test");

        let stats = router.stats();
        assert_eq!(stats.uri_to_path_count, 1);
        assert_eq!(stats.path_to_uri_count, 1);
        assert!(stats.prefix_count > 5); // 默认前缀 + 1 个自定义
    }

    #[test]
    fn test_load_from_config() {
        let config = McpRouterConfig::default();
        let router = McpRouter::new();
        router.load_from_config(&config);

        assert!(router.uri_to_path("file:///context/test").is_ok());
        assert!(router.uri_to_path("file:///skills/test").is_ok());
    }

    #[test]
    fn test_list_prefixes() {
        let router = McpRouter::new();
        let prefixes = router.list_prefixes();

        assert!(prefixes.contains_key("file:///context"));
        assert!(prefixes.contains_key("file:///skills"));
        assert!(prefixes.contains_key("github://"));
    }

    #[test]
    fn test_remove_prefix() {
        let router = McpRouter::new();

        router.add_prefix("temp://", "/temp");
        assert!(router.uri_to_path("temp://file").is_ok());

        router.remove_prefix("temp://");
        // 现在会走默认逻辑
        let result = router.uri_to_path("temp://file");
        // temp:// 不是 file://，也不是 github:// 等，所以会失败
        assert!(result.is_err());
    }
}