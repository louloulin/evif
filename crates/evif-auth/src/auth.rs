// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::audit::AuditLogManager;
use crate::{AuthResult, CapId, Capability, Principal, PrincipalId};
use dashmap::DashMap;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

/// JWT 验证错误
#[derive(Error, Debug)]
pub enum JwtError {
    #[error("Invalid token format")]
    InvalidFormat,
    #[error("Token expired")]
    Expired,
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Missing required claim: {0}")]
    MissingClaim(String),
    #[error("Token not yet valid")]
    NotYetValid,
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
}

/// JWT Claims 结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user or service ID)
    pub sub: String,
    /// Expiration time (Unix timestamp)
    pub exp: usize,
    /// Issued at (Unix timestamp)
    pub iat: usize,
    /// Issuer
    pub iss: Option<String>,
    /// Audience
    pub aud: Option<String>,
    /// Role/permissions
    pub role: Option<String>,
    /// Custom claims
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

impl Claims {
    /// 创建新的 claims
    pub fn new(sub: String, exp_hours: i64) -> Self {
        let now = chrono::Utc::now().timestamp() as usize;
        Self {
            sub,
            exp: now + (exp_hours as usize * 3600),
            iat: now,
            iss: None,
            aud: None,
            role: None,
            extra: serde_json::Value::Null,
        }
    }
}

/// JWT 验证器配置
#[derive(Clone)]
pub struct JwtValidator {
    /// 验证密钥（用于 HS256）
    secret: Option<String>,
    /// JWKS URL（用于 RS256）
    jwks_url: Option<String>,
    /// 期望的 issuer
    expected_issuer: Option<String>,
    /// 期望的 audience
    expected_audience: Option<String>,
}

impl JwtValidator {
    /// 从密钥创建验证器（HS256）
    pub fn with_secret(secret: &str) -> Self {
        Self {
            secret: Some(secret.to_string()),
            jwks_url: None,
            expected_issuer: None,
            expected_audience: None,
        }
    }

    /// 设置期望的 issuer
    pub fn with_issuer(mut self, issuer: &str) -> Self {
        self.expected_issuer = Some(issuer.to_string());
        self
    }

    /// 设置期望的 audience
    pub fn with_audience(mut self, audience: &str) -> Self {
        self.expected_audience = Some(audience.to_string());
        self
    }

    /// 验证 JWT token
    pub fn validate(&self, token: &str) -> Result<Claims, JwtError> {
        // 如果没有配置密钥，返回错误
        let secret = self.secret.as_ref()
            .ok_or_else(|| JwtError::ValidationFailed("No JWT secret configured".to_string()))?;

        let mut validation = Validation::default();

        // 设置 issuer 验证
        if let Some(ref issuer) = self.expected_issuer {
            validation.set_issuer(&[issuer]);
        }

        // 设置 audience 验证
        if let Some(ref audience) = self.expected_audience {
            validation.set_audience(&[audience]);
        }

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        )
        .map_err(|e| {
            use jsonwebtoken::errors::ErrorKind;
            match e.kind() {
                ErrorKind::ExpiredSignature => JwtError::Expired,
                ErrorKind::InvalidSignature => JwtError::InvalidSignature,
                ErrorKind::InvalidToken => JwtError::InvalidFormat,
                ErrorKind::ImmatureSignature => JwtError::NotYetValid,
                _ => JwtError::ValidationFailed(e.to_string()),
            }
        })?;

        Ok(token_data.claims)
    }
}

/// JWT 认证结果
#[derive(Debug, Clone)]
pub enum JwtAuthResult {
    /// 认证成功，返回用户 ID
    Success(String),
    /// JWT 格式不正确或不是 JWT token
    NotJwt,
    /// JWT 验证失败
    Invalid(String),
}

/// 从 Authorization header 提取并验证 JWT
pub fn extract_and_validate_jwt(
    auth_header: &str,
    validator: &JwtValidator,
) -> JwtAuthResult {
    // 检查 Bearer token 格式
    let token = if auth_header.starts_with("Bearer ") {
        &auth_header[7..]
    } else if auth_header.starts_with("bearer ") {
        &auth_header[7..]
    } else {
        // 不是 Bearer token，可能是 API Key
        return JwtAuthResult::NotJwt;
    };

    // 尝试解析 JWT
    match validator.validate(token) {
        Ok(claims) => JwtAuthResult::Success(claims.sub),
        Err(e) => JwtAuthResult::Invalid(e.to_string()),
    }
}

/// 生成 JWT token（用于测试）
#[allow(dead_code)]
pub fn generate_jwt(sub: &str, secret: &str, exp_hours: i64) -> Result<String, JwtError> {
    let claims = Claims::new(sub.to_string(), exp_hours);

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| JwtError::ValidationFailed(e.to_string()))
}

/// 认证策略
#[derive(Debug, Clone, Default)]
pub enum AuthPolicy {
    /// 开放策略：所有操作都允许
    Open,
    /// 严格策略：需要明确授权
    #[default]
    Strict,
}

/// 认证管理器
pub struct AuthManager {
    capabilities: DashMap<CapId, Capability>,
    policy: Arc<RwLock<AuthPolicy>>,
    audit_log: Option<Arc<AuditLogManager>>,
    jwt_validator: Option<JwtValidator>,
}

impl AuthManager {
    pub fn new() -> Self {
        AuthManager {
            capabilities: DashMap::new(),
            policy: Arc::new(RwLock::new(AuthPolicy::default())),
            audit_log: None,
            jwt_validator: None,
        }
    }

    pub fn with_policy(policy: AuthPolicy) -> Self {
        AuthManager {
            capabilities: DashMap::new(),
            policy: Arc::new(RwLock::new(policy)),
            audit_log: None,
            jwt_validator: None,
        }
    }

    /// 创建带有审计日志的认证管理器
    pub fn with_audit_log(mut self, audit_log: Arc<AuditLogManager>) -> Self {
        self.audit_log = Some(audit_log);
        self
    }

    /// 创建带有 JWT 验证器的认证管理器
    pub fn with_jwt_validator(mut self, validator: JwtValidator) -> Self {
        self.jwt_validator = Some(validator);
        self
    }

    /// 获取审计日志管理器
    pub fn audit_log(&self) -> Option<&Arc<AuditLogManager>> {
        self.audit_log.as_ref()
    }

    /// 设置审计日志管理器
    pub fn set_audit_log(&mut self, audit_log: Arc<AuditLogManager>) {
        self.audit_log = Some(audit_log);
    }

    /// 获取 JWT 验证器
    pub fn jwt_validator(&self) -> Option<&JwtValidator> {
        self.jwt_validator.as_ref()
    }

    /// 设置 JWT 验证器
    pub fn set_jwt_validator(&mut self, validator: JwtValidator) {
        self.jwt_validator = Some(validator);
    }

    /// 从 Authorization header 验证 JWT 并返回用户 ID
    pub fn validate_jwt(&self, auth_header: &str) -> Result<String, JwtError> {
        let validator = self.jwt_validator.as_ref()
            .ok_or_else(|| JwtError::ValidationFailed("JWT validator not configured".to_string()))?;

        match extract_and_validate_jwt(auth_header, validator) {
            JwtAuthResult::Success(sub) => Ok(sub),
            JwtAuthResult::NotJwt => Err(JwtError::InvalidFormat),
            JwtAuthResult::Invalid(msg) => Err(JwtError::ValidationFailed(msg)),
        }
    }

    /// 检查是否配置了 JWT
    pub fn has_jwt_support(&self) -> bool {
        self.jwt_validator.is_some()
    }

    /// 授予权限
    pub fn grant(&self, cap: Capability) -> AuthResult<CapId> {
        let id = cap.id;
        let holder = cap.holder;
        let node = cap.node;

        self.capabilities.insert(id, cap);

        // 记录审计日志
        if let Some(ref audit) = self.audit_log {
            let _ = audit.log_capability_granted(holder, node);
        }

        Ok(id)
    }

    /// 撤销权限
    pub fn revoke(&self, cap_id: &CapId) -> AuthResult<()> {
        if let Some((_, cap)) = self.capabilities.remove(cap_id) {
            // 记录审计日志
            if let Some(ref audit) = self.audit_log {
                let _ = audit.log_capability_revoked(cap.holder, cap.node);
            }
        }
        Ok(())
    }

    /// 检查权限
    pub fn check(
        &self,
        principal: &Principal,
        node: &uuid::Uuid,
        required_perm: Permission,
    ) -> AuthResult<bool> {
        let policy = self.policy.read();

        let principal_id = match principal {
            Principal::System => {
                // 系统主体总是有权限，但仍记录审计日志
                if let Some(ref audit) = self.audit_log {
                    let perm_str = format!("{:?}", required_perm);
                    let _ = audit.log_access_granted(uuid::Uuid::new_v4(), *node, &perm_str);
                }
                return Ok(true);
            }
            Principal::User(id) | Principal::Service(id) => *id,
        };

        match *policy {
            AuthPolicy::Open => {
                if let Some(ref audit) = self.audit_log {
                    let perm_str = format!("{:?}", required_perm);
                    let _ = audit.log_access_granted(principal_id, *node, &perm_str);
                }
                Ok(true)
            }
            AuthPolicy::Strict => {
                // 在严格模式下，需要检查具体权限
                for cap in self.capabilities.iter() {
                    let cap = cap.value();
                    if cap.holder == principal_id && cap.node == *node && cap.is_valid() {
                        let has_permission = match required_perm {
                            Permission::Read => cap.permissions.read,
                            Permission::Write => cap.permissions.write,
                            Permission::Execute => cap.permissions.execute,
                            Permission::Admin => cap.permissions.admin,
                        };

                        // 记录审计日志
                        if let Some(ref audit) = self.audit_log {
                            let perm_str = format!("{:?}", required_perm);
                            if has_permission {
                                let _ = audit.log_access_granted(principal_id, *node, &perm_str);
                            } else {
                                let _ = audit.log_access_denied(
                                    principal_id,
                                    *node,
                                    &perm_str,
                                    "insufficient permissions",
                                );
                            }
                        }

                        return Ok(has_permission);
                    }
                }

                // 没有找到匹配的能力
                if let Some(ref audit) = self.audit_log {
                    let perm_str = format!("{:?}", required_perm);
                    let _ = audit.log_access_denied(
                        principal_id,
                        *node,
                        &perm_str,
                        "no capability found",
                    );
                }

                Ok(false)
            }
        }
    }

    /// 获取主体的所有能力
    pub fn get_capabilities(&self, principal_id: &PrincipalId) -> Vec<Capability> {
        self.capabilities
            .iter()
            .filter(|entry| entry.value().holder == *principal_id)
            .map(|entry| entry.value().clone())
            .collect()
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 权限枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    Read,
    Write,
    Execute,
    Admin,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Permissions;
    use uuid::Uuid;

    #[test]
    fn test_auth_manager_grant() {
        let manager = AuthManager::new();
        let holder = Uuid::new_v4();
        let node = Uuid::new_v4();
        let cap = Capability::new(holder, node, Permissions::read());

        let cap_id = manager.grant(cap).unwrap();
        let retrieved = manager.capabilities.get(&cap_id);
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_auth_manager_revoke() {
        let manager = AuthManager::new();
        let holder = Uuid::new_v4();
        let node = Uuid::new_v4();
        let cap = Capability::new(holder, node, Permissions::read());

        let cap_id = manager.grant(cap).unwrap();
        manager.revoke(&cap_id).unwrap();

        let retrieved = manager.capabilities.get(&cap_id);
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_auth_check_open() {
        let manager = AuthManager::with_policy(AuthPolicy::Open);
        let principal = Principal::User(Uuid::new_v4());
        let node = Uuid::new_v4();

        let result = manager.check(&principal, &node, Permission::Read);
        assert!(result.unwrap());
    }

    #[test]
    fn test_auth_check_strict() {
        let manager = AuthManager::with_policy(AuthPolicy::Strict);
        let holder = Uuid::new_v4();
        let node = Uuid::new_v4();
        let cap = Capability::new(holder, node, Permissions::read());

        manager.grant(cap).unwrap();

        let principal = Principal::User(holder);
        let result = manager.check(&principal, &node, Permission::Read);
        assert!(result.unwrap());
    }

    #[test]
    fn test_jwt_generate_and_validate() {
        let secret = "test-secret-key-for-jwt-validation";
        let sub = "user-123";

        // 生成 JWT
        let token = generate_jwt(sub, secret, 1).unwrap();

        // 验证 JWT
        let validator = JwtValidator::with_secret(secret);
        let result = validator.validate(&token);

        assert!(result.is_ok());
        let claims = result.unwrap();
        assert_eq!(claims.sub, sub);
    }

    #[test]
    fn test_jwt_with_wrong_secret() {
        let secret = "test-secret-key";
        let wrong_secret = "wrong-secret-key";
        let sub = "user-123";

        // 生成 JWT with correct secret
        let token = generate_jwt(sub, secret, 1).unwrap();

        // 验证 JWT with wrong secret
        let validator = JwtValidator::with_secret(wrong_secret);
        let result = validator.validate(&token);

        assert!(result.is_err());
    }

    #[test]
    fn test_extract_jwt_from_bearer() {
        let secret = "test-secret-key";
        let token = generate_jwt("user-123", secret, 1).unwrap();
        let auth_header = format!("Bearer {}", token);

        let validator = JwtValidator::with_secret(secret);
        let result = extract_and_validate_jwt(&auth_header, &validator);

        match result {
            JwtAuthResult::Success(sub) => assert_eq!(sub, "user-123"),
            JwtAuthResult::NotJwt => panic!("Should not be NotJwt for Bearer token"),
            JwtAuthResult::Invalid(msg) => panic!("Should not be Invalid: {}", msg),
        }
    }

    #[test]
    fn test_extract_jwt_not_bearer() {
        let validator = JwtValidator::with_secret("secret");
        let result = extract_and_validate_jwt("api-key-123", &validator);

        assert!(matches!(result, JwtAuthResult::NotJwt));
    }

    #[test]
    fn test_auth_manager_with_jwt() {
        let secret = "jwt-test-secret";
        let validator = JwtValidator::with_secret(secret);

        let mut manager = AuthManager::with_policy(AuthPolicy::Strict);
        manager.set_jwt_validator(validator);

        assert!(manager.has_jwt_support());
    }
}
