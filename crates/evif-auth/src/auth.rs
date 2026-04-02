// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::audit::AuditLogManager;
use crate::{AuthResult, CapId, Capability, Principal, PrincipalId};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;

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
}

impl AuthManager {
    pub fn new() -> Self {
        AuthManager {
            capabilities: DashMap::new(),
            policy: Arc::new(RwLock::new(AuthPolicy::default())),
            audit_log: None,
        }
    }

    pub fn with_policy(policy: AuthPolicy) -> Self {
        AuthManager {
            capabilities: DashMap::new(),
            policy: Arc::new(RwLock::new(policy)),
            audit_log: None,
        }
    }

    /// 创建带有审计日志的认证管理器
    pub fn with_audit_log(mut self, audit_log: Arc<AuditLogManager>) -> Self {
        self.audit_log = Some(audit_log);
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
}
