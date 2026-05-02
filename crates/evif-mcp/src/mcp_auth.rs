// MCP Authentication - MCP Token 验证和认证管理
//
// 实现 MCP Server 和 Client 的认证机制
// 支持 Bearer Token、API Key 等认证方式

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use chrono::{DateTime, Utc, Duration};

/// 认证错误
#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Missing token")]
    MissingToken,

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Token expired at {0}")]
    TokenExpired(DateTime<Utc>),

    #[error("Insufficient permissions: {0}")]
    InsufficientPermissions(String),

    #[error("Auth not configured")]
    NotConfigured,

    #[error("Auth service error: {0}")]
    ServiceError(String),
}

/// 认证类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    /// Bearer Token (JWT)
    Bearer,
    /// API Key
    ApiKey,
    /// OAuth
    OAuth,
    /// 无认证
    None,
}

impl Default for AuthType {
    fn default() -> Self {
        Self::None
    }
}

/// MCP Token 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToken {
    /// Token 标识
    pub id: String,
    /// Token 密钥 (哈希)
    pub secret_hash: String,
    /// 关联的服务器名称
    pub server: Option<String>,
    /// 关联的客户端名称
    pub client: Option<String>,
    /// 能力范围
    pub scopes: Vec<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 过期时间
    pub expires_at: Option<DateTime<Utc>>,
    /// 是否启用
    pub enabled: bool,
}

impl McpToken {
    /// 创建新的 Token
    pub fn new(id: impl Into<String>, secret_hash: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            secret_hash: secret_hash.into(),
            server: None,
            client: None,
            scopes: Vec::new(),
            created_at: Utc::now(),
            expires_at: None,
            enabled: true,
        }
    }

    /// 设置服务器
    pub fn with_server(mut self, server: impl Into<String>) -> Self {
        self.server = Some(server.into());
        self
    }

    /// 设置客户端
    pub fn with_client(mut self, client: impl Into<String>) -> Self {
        self.client = Some(client.into());
        self
    }

    /// 添加权限范围
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scopes.push(scope.into());
        self
    }

    /// 设置过期时间
    pub fn with_expiry(self, duration: Duration) -> Self {
        Self {
            expires_at: Some(Utc::now() + duration),
            ..self
        }
    }

    /// 检查是否过期
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    /// 检查是否有效
    pub fn is_valid(&self) -> bool {
        self.enabled && !self.is_expired()
    }

    /// 检查是否有指定权限
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope || s == "*")
    }
}

/// MCP 认证配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpAuthConfig {
    /// 认证类型
    pub auth_type: AuthType,
    /// Token 文件路径
    #[serde(default)]
    pub token_file: Option<String>,
    /// API Key 环境变量名
    #[serde(default)]
    pub api_key_env: Option<String>,
    /// JWT 密钥 (Base64 编码)
    #[serde(default)]
    pub jwt_secret: Option<String>,
    /// 允许的服务器列表
    #[serde(default)]
    pub allowed_servers: Vec<String>,
    /// 默认过期时间 (秒)
    #[serde(default = "default_expiry")]
    pub default_expiry_secs: i64,
}

fn default_expiry() -> i64 {
    3600 // 1 小时
}

impl Default for McpAuthConfig {
    fn default() -> Self {
        Self {
            auth_type: AuthType::None,
            token_file: None,
            api_key_env: None,
            jwt_secret: None,
            allowed_servers: Vec::new(),
            default_expiry_secs: default_expiry(),
        }
    }
}

/// MCP 会话信息
#[derive(Debug, Clone)]
pub struct McpSession {
    /// 会话 ID
    pub id: String,
    /// 关联的 Token ID
    pub token_id: String,
    /// 客户端名称
    pub client_name: String,
    /// 服务器名称
    pub server_name: String,
    /// 权限范围
    pub scopes: Vec<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后活动时间
    pub last_activity: DateTime<Utc>,
    /// 是否活跃
    pub active: bool,
}

impl McpSession {
    /// 创建新会话
    pub fn new(id: impl Into<String>, token_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            token_id: token_id.into(),
            client_name: String::new(),
            server_name: String::new(),
            scopes: Vec::new(),
            created_at: now,
            last_activity: now,
            active: true,
        }
    }

    /// 更新活动时间
    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }

    /// 标记为非活跃
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

/// MCP 认证管理器
pub struct McpAuth {
    /// 认证配置
    config: McpAuthConfig,
    /// Token 注册表
    tokens: Arc<RwLock<HashMap<String, McpToken>>>,
    /// 活跃会话
    sessions: Arc<RwLock<HashMap<String, McpSession>>>,
}

impl McpAuth {
    /// 创建新的认证管理器
    pub fn new(config: McpAuthConfig) -> Self {
        Self {
            config,
            tokens: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 从配置创建
    pub fn from_config(config: &McpAuthConfig) -> Self {
        Self::new(config.clone())
    }

    /// 注册 Token
    pub fn register_token(&self, token: McpToken) {
        let mut tokens = self.tokens.write().unwrap();
        tokens.insert(token.id.clone(), token);
    }

    /// 移除 Token
    pub fn remove_token(&self, token_id: &str) -> Option<McpToken> {
        let mut tokens = self.tokens.write().unwrap();
        tokens.remove(token_id)
    }

    /// 获取 Token
    pub fn get_token(&self, token_id: &str) -> Option<McpToken> {
        let tokens = self.tokens.read().unwrap();
        tokens.get(token_id).cloned()
    }

    /// 验证 Token 有效性
    pub fn validate_token(&self, token_id: &str, secret: Option<&str>) -> Result<McpToken, AuthError> {
        let tokens = self.tokens.read().unwrap();

        let token = tokens.get(token_id)
            .ok_or_else(|| AuthError::InvalidToken("Token not found".to_string()))?
            .clone();

        drop(tokens);

        // 检查是否启用
        if !token.enabled {
            return Err(AuthError::InvalidToken("Token is disabled".to_string()));
        }

        // 检查是否过期
        if token.is_expired() {
            return Err(AuthError::TokenExpired(token.expires_at.unwrap()));
        }

        // 如果提供了密钥，验证哈希
        if let Some(secret) = secret {
            if token.secret_hash != secret && !self.verify_secret(secret, &token.secret_hash) {
                return Err(AuthError::InvalidToken("Secret mismatch".to_string()));
            }
        }

        Ok(token)
    }

    /// 验证密钥 (简化版本，实际应使用 bcrypt 或 argon2)
    fn verify_secret(&self, secret: &str, hash: &str) -> bool {
        // 简化实现：直接比较
        // 生产环境应使用 bcrypt 或 argon2
        secret == hash
    }

    /// 创建会话
    pub fn create_session(&self, token_id: &str) -> Result<McpSession, AuthError> {
        let token = self.validate_token(token_id, None)?;

        let session_id = uuid::Uuid::new_v4().to_string();
        let mut session = McpSession::new(session_id, token_id);

        session.client_name = token.client.unwrap_or_default();
        session.server_name = token.server.unwrap_or_default();
        session.scopes = token.scopes.clone();

        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(session.id.clone(), session.clone());

        Ok(session)
    }

    /// 获取会话
    pub fn get_session(&self, session_id: &str) -> Option<McpSession> {
        let sessions = self.sessions.read().unwrap();
        sessions.get(session_id).cloned()
    }

    /// 更新会话活动时间
    pub fn touch_session(&self, session_id: &str) -> Result<(), AuthError> {
        let mut sessions = self.sessions.write().unwrap();

        if let Some(session) = sessions.get_mut(session_id) {
            session.touch();
            Ok(())
        } else {
            Err(AuthError::InvalidToken("Session not found".to_string()))
        }
    }

    /// 销毁会话
    pub fn destroy_session(&self, session_id: &str) -> Result<(), AuthError> {
        let mut sessions = self.sessions.write().unwrap();

        if let Some(session) = sessions.get_mut(session_id) {
            session.deactivate();
            sessions.remove(session_id);
            Ok(())
        } else {
            Err(AuthError::InvalidToken("Session not found".to_string()))
        }
    }

    /// 验证会话权限
    pub fn check_session_scope(&self, session_id: &str, scope: &str) -> Result<(), AuthError> {
        let sessions = self.sessions.read().unwrap();

        let session = sessions.get(session_id)
            .ok_or_else(|| AuthError::InvalidToken("Session not found".to_string()))?;

        if !session.active {
            return Err(AuthError::InvalidToken("Session is inactive".to_string()));
        }

        if !session.scopes.iter().any(|s| s == scope || s == "*") {
            return Err(AuthError::InsufficientPermissions(scope.to_string()));
        }

        Ok(())
    }

    /// 列出所有 Token
    pub fn list_tokens(&self) -> Vec<McpToken> {
        let tokens = self.tokens.read().unwrap();
        tokens.values().cloned().collect()
    }

    /// 列出所有会话
    pub fn list_sessions(&self) -> Vec<McpSession> {
        let sessions = self.sessions.read().unwrap();
        sessions.values().cloned().collect()
    }

    /// 清理过期会话
    pub fn cleanup_expired_sessions(&self, max_idle: Duration) -> usize {
        let now = Utc::now();
        let mut sessions = self.sessions.write().unwrap();
        let mut removed = 0;

        sessions.retain(|_, session| {
            let expired = !session.active || (now - session.last_activity) > max_idle;
            if expired {
                removed += 1;
            }
            !expired
        });

        removed
    }

    /// 从环境变量获取 API Key
    pub fn get_api_key_from_env(&self) -> Option<String> {
        self.config.api_key_env.as_ref()
            .and_then(|env_name| std::env::var(env_name).ok())
    }

    /// 验证 Bearer Token
    pub fn validate_bearer_token(&self, auth_header: &str) -> Result<McpToken, AuthError> {
        if !auth_header.starts_with("Bearer ") {
            return Err(AuthError::InvalidToken("Invalid Bearer format".to_string()));
        }

        let token_id = auth_header.trim_start_matches("Bearer ");

        // 简化：使用 token_id 作为密钥
        // 生产环境应使用 JWT 解析
        self.validate_token(token_id, None)
    }

    /// 验证 API Key
    pub fn validate_api_key(&self, api_key: &str) -> Result<McpToken, AuthError> {
        // 简化实现：API Key 格式为 "token_id:secret"
        let parts: Vec<&str> = api_key.split(':').collect();

        if parts.len() != 2 {
            return Err(AuthError::InvalidToken("Invalid API Key format".to_string()));
        }

        let (token_id, secret) = (parts[0], parts[1]);
        self.validate_token(token_id, Some(secret))
    }

    /// 获取认证配置
    pub fn get_config(&self) -> &McpAuthConfig {
        &self.config
    }

    /// 检查是否启用认证
    pub fn is_enabled(&self) -> bool {
        self.config.auth_type != AuthType::None
    }
}

impl Default for McpAuth {
    fn default() -> Self {
        Self::new(McpAuthConfig::default())
    }
}

/// 认证结果
#[derive(Debug, Clone)]
pub struct AuthResult {
    /// 是否认证成功
    pub success: bool,
    /// 会话 ID (如果成功)
    pub session_id: Option<String>,
    /// 错误信息 (如果失败)
    pub error: Option<String>,
    /// 权限范围
    pub scopes: Vec<String>,
}

impl AuthResult {
    /// 创建成功结果
    pub fn success(session_id: String, scopes: Vec<String>) -> Self {
        Self {
            success: true,
            session_id: Some(session_id),
            error: None,
            scopes,
        }
    }

    /// 创建失败结果
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            session_id: None,
            error: Some(error.into()),
            scopes: Vec::new(),
        }
    }
}

/// 请求认证
pub fn authenticate_request(
    auth: &McpAuth,
    auth_header: Option<&str>,
    api_key: Option<&str>,
) -> AuthResult {
    // 如果认证未启用，直接成功
    if !auth.is_enabled() {
        return AuthResult::success("anonymous".to_string(), vec!["*".to_string()]);
    }

    // 尝试 Bearer Token
    if let Some(header) = auth_header {
        match auth.validate_bearer_token(header) {
            Ok(token) => {
                match auth.create_session(&token.id) {
                    Ok(session) => AuthResult::success(session.id, session.scopes),
                    Err(e) => AuthResult::failure(e.to_string()),
                }
            }
            Err(e) => AuthResult::failure(e.to_string()),
        }
    }
    // 尝试 API Key
    else if let Some(key) = api_key {
        match auth.validate_api_key(key) {
            Ok(token) => {
                match auth.create_session(&token.id) {
                    Ok(session) => AuthResult::success(session.id, session.scopes),
                    Err(e) => AuthResult::failure(e.to_string()),
                }
            }
            Err(e) => AuthResult::failure(e.to_string()),
        }
    }
    // 尝试环境变量 API Key
    else if let Some(key) = auth.get_api_key_from_env() {
        match auth.validate_api_key(&key) {
            Ok(token) => {
                match auth.create_session(&token.id) {
                    Ok(session) => AuthResult::success(session.id, session.scopes),
                    Err(e) => AuthResult::failure(e.to_string()),
                }
            }
            Err(e) => AuthResult::failure(e.to_string()),
        }
    }
    else {
        AuthResult::failure("No authentication provided")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_creation() {
        let token = McpToken::new("token1", "secret123")
            .with_server("github")
            .with_client("test-client")
            .with_scope("read")
            .with_scope("write")
            .with_expiry(Duration::hours(1));

        assert_eq!(token.id, "token1");
        assert!(token.server.is_some());
        assert_eq!(token.scopes.len(), 2);
        assert!(!token.is_expired());
        assert!(token.is_valid());
    }

    #[test]
    fn test_token_expiry() {
        let token = McpToken::new("expired", "secret")
            .with_expiry(Duration::hours(-1));

        assert!(token.is_expired());
        assert!(!token.is_valid());
    }

    #[test]
    fn test_scope_check() {
        let token = McpToken::new("test", "secret")
            .with_scope("read")
            .with_scope("write");

        assert!(token.has_scope("read"));
        assert!(token.has_scope("write"));
        assert!(!token.has_scope("admin"));
    }

    #[test]
    fn test_wildcard_scope() {
        let token = McpToken::new("admin", "secret")
            .with_scope("*");

        assert!(token.has_scope("read"));
        assert!(token.has_scope("write"));
        assert!(token.has_scope("admin"));
    }

    #[test]
    fn test_auth_creation() {
        let config = McpAuthConfig {
            auth_type: AuthType::Bearer,
            token_file: Some("/tmp/mcp-tokens.toml".to_string()),
            ..Default::default()
        };

        let auth = McpAuth::from_config(&config);

        assert!(auth.is_enabled());
        assert_eq!(auth.get_config().auth_type, AuthType::Bearer);
    }

    #[test]
    fn test_token_registration() {
        let auth = McpAuth::default();
        let token = McpToken::new("test-token", "secret123")
            .with_scope("read");

        auth.register_token(token.clone());

        let retrieved = auth.get_token("test-token");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "test-token");
    }

    #[test]
    fn test_token_validation() {
        let auth = McpAuth::default();
        let token = McpToken::new("valid-token", "secret123")
            .with_scope("read")
            .with_expiry(Duration::hours(1));

        auth.register_token(token);

        let result = auth.validate_token("valid-token", None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, "valid-token");
    }

    #[test]
    fn test_invalid_token() {
        let auth = McpAuth::default();

        let result = auth.validate_token("nonexistent", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_session_creation() {
        let auth = McpAuth::default();
        let token = McpToken::new("session-token", "secret")
            .with_server("github")
            .with_client("test")
            .with_scope("read");

        auth.register_token(token);

        let session = auth.create_session("session-token");
        assert!(session.is_ok());
        assert!(session.unwrap().active);
    }

    #[test]
    fn test_session_touch() {
        let auth = McpAuth::default();
        let token = McpToken::new("touch-token", "secret")
            .with_scope("read");

        auth.register_token(token);

        let session = auth.create_session("touch-token").unwrap();
        let session_id = session.id.clone();

        // 更新会话
        let result = auth.touch_session(&session_id);
        assert!(result.is_ok());

        let updated = auth.get_session(&session_id).unwrap();
        assert_eq!(updated.id, session_id);
    }

    #[test]
    fn test_session_scope_check() {
        let auth = McpAuth::default();
        let token = McpToken::new("scope-token", "secret")
            .with_scope("read");

        auth.register_token(token);

        let session = auth.create_session("scope-token").unwrap();
        let session_id = session.id;

        // 有 read 权限
        let result = auth.check_session_scope(&session_id, "read");
        assert!(result.is_ok());

        // 没有 write 权限
        let result = auth.check_session_scope(&session_id, "write");
        assert!(result.is_err());
    }

    #[test]
    fn test_authenticate_request_no_auth() {
        let auth = McpAuth::default();

        let result = authenticate_request(&auth, None, None);
        assert!(result.success);
    }

    #[test]
    fn test_authenticate_request_with_bearer() {
        let auth = McpAuth::default();
        let token = McpToken::new("bearer-token", "secret")
            .with_scope("read");

        auth.register_token(token);

        let result = authenticate_request(&auth, Some("Bearer bearer-token"), None);
        assert!(result.success);
        assert!(result.session_id.is_some());
    }

    #[test]
    fn test_authenticate_request_with_api_key() {
        let config = McpAuthConfig {
            auth_type: AuthType::ApiKey,
            ..Default::default()
        };
        let auth = McpAuth::new(config);
        let token = McpToken::new("api-token", "api-secret")
            .with_scope("write");

        auth.register_token(token);

        let result = authenticate_request(&auth, None, Some("api-token:api-secret"));
        assert!(result.success);
        assert!(result.scopes.contains(&"write".to_string()));
    }

    #[test]
    fn test_cleanup_expired_sessions() {
        let auth = McpAuth::default();
        let token = McpToken::new("cleanup-token", "secret")
            .with_scope("read");

        auth.register_token(token);

        let session = auth.create_session("cleanup-token").unwrap();
        let session_id = session.id.clone();

        // 手动标记为非活跃而不是销毁
        {
            let mut sessions = auth.sessions.write().unwrap();
            if let Some(s) = sessions.get_mut(&session_id) {
                s.active = false;
            }
        }

        // 清理超过 0 秒的空闲会话
        let removed = auth.cleanup_expired_sessions(Duration::seconds(0));
        assert_eq!(removed, 1);

        // 验证会话已删除
        assert!(auth.get_session(&session_id).is_none());
    }
}