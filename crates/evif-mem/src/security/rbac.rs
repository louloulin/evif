//! Role-Based Access Control (RBAC) module
//!
//! Provides fine-grained access control for memory resources.

use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;

use crate::error::{MemError, MemResult};

/// Resource type for access control
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Resource {
    /// Memory item resource
    MemoryItem,
    /// Category resource
    Category,
    /// Resource (file/image/etc) resource
    Resource,
    /// Workflow resource
    Workflow,
    /// Pipeline resource
    Pipeline,
    /// User management resource
    User,
    /// System configuration resource
    Config,
    /// Audit log resource
    Audit,
    /// Custom resource
    Custom(String),
}

impl Resource {
    /// Get resource name as string
    pub fn name(&self) -> &str {
        match self {
            Resource::MemoryItem => "memory_item",
            Resource::Category => "category",
            Resource::Resource => "resource",
            Resource::Workflow => "workflow",
            Resource::Pipeline => "pipeline",
            Resource::User => "user",
            Resource::Config => "config",
            Resource::Audit => "audit",
            Resource::Custom(name) => name,
        }
    }
}

/// Action type for access control
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Action {
    /// Read access
    Read,
    /// Write access
    Write,
    /// Delete access
    Delete,
    /// Execute access (for workflows/pipelines)
    Execute,
    /// Admin access
    Admin,
    /// Custom action
    Custom(String),
}

impl Action {
    /// Get action name as string
    pub fn name(&self) -> &str {
        match self {
            Action::Read => "read",
            Action::Write => "write",
            Action::Delete => "delete",
            Action::Execute => "execute",
            Action::Admin => "admin",
            Action::Custom(name) => name,
        }
    }
}

/// Permission definition
#[derive(Debug, Clone)]
pub struct Permission {
    /// Resource type
    pub resource: Resource,
    /// Allowed actions
    pub actions: Vec<Action>,
}

impl Permission {
    /// Create new permission
    pub fn new(resource: Resource, actions: Vec<Action>) -> Self {
        Self { resource, actions }
    }

    /// Check if permission allows action on resource
    pub fn allows(&self, resource: &Resource, action: &Action) -> bool {
        // Check resource match (exact or custom wildcard)
        let resource_matches = match (&self.resource, resource) {
            (Resource::Custom(wildcard), r) if wildcard == "*" => true,
            (Resource::Custom(_), _) => false, // Custom requires exact match
            _ => &self.resource == resource,
        };

        if !resource_matches {
            return false;
        }

        // Check action match
        self.actions.iter().any(|a| match (a, action) {
            (Action::Custom(wildcard), _) if wildcard == "*" => true,
            (Action::Custom(_), _) => false,
            _ => a == action,
        })
    }
}

/// Permission builder for fluent API
pub struct PermissionBuilder {
    resource: Resource,
    actions: Vec<Action>,
}

impl PermissionBuilder {
    /// Create new builder
    pub fn new(resource: Resource) -> Self {
        Self {
            resource,
            actions: Vec::new(),
        }
    }

    /// Add read action
    pub fn with_read(mut self) -> Self {
        self.actions.push(Action::Read);
        self
    }

    /// Add write action
    pub fn with_write(mut self) -> Self {
        self.actions.push(Action::Write);
        self
    }

    /// Add delete action
    pub fn with_delete(mut self) -> Self {
        self.actions.push(Action::Delete);
        self
    }

    /// Add execute action
    pub fn with_execute(mut self) -> Self {
        self.actions.push(Action::Execute);
        self
    }

    /// Add admin action
    pub fn with_admin(mut self) -> Self {
        self.actions.push(Action::Admin);
        self
    }

    /// Add custom action
    pub fn with_action(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    /// Build permission
    pub fn build(self) -> Permission {
        Permission {
            resource: self.resource,
            actions: self.actions,
        }
    }
}

/// Role definition
#[derive(Debug, Clone)]
pub struct Role {
    /// Role name
    pub name: String,
    /// Role permissions
    pub permissions: Vec<Permission>,
    /// Role description
    pub description: Option<String>,
}

impl Role {
    /// Create new role
    pub fn new(name: String, permissions: Vec<Permission>) -> Self {
        Self {
            name,
            permissions,
            description: None,
        }
    }

    /// Check if role allows action on resource
    pub fn allows(&self, resource: &Resource, action: &Action) -> bool {
        self.permissions.iter().any(|p| p.allows(resource, action))
    }
}

/// Role builder for fluent API
pub struct RoleBuilder {
    name: String,
    permissions: Vec<Permission>,
    description: Option<String>,
}

impl RoleBuilder {
    /// Create new builder
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            permissions: Vec::new(),
            description: None,
        }
    }

    /// Add permission
    pub fn with_permission(mut self, permission: Permission) -> Self {
        self.permissions.push(permission);
        self
    }

    /// Add description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Build role
    pub fn build(self) -> Role {
        Role {
            name: self.name,
            permissions: self.permissions,
            description: self.description,
        }
    }
}

/// RBAC configuration
#[derive(Debug, Clone)]
pub struct RbacConfig {
    /// Enable RBAC flag
    pub enabled: bool,
    /// Default role for unauthenticated users
    pub default_role: Option<String>,
    /// Enable audit logging
    pub audit_enabled: bool,
}

impl Default for RbacConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_role: Some("guest".to_string()),
            audit_enabled: true,
        }
    }
}

/// Role-Based Access Control system
#[derive(Debug, Clone)]
pub struct Rbac {
    config: RbacConfig,
    roles: Arc<DashMap<String, Role>>,
    user_roles: Arc<DashMap<String, Vec<String>>>,
}

impl Rbac {
    /// Create new RBAC system
    pub fn new(config: RbacConfig) -> Self {
        let rbac = Self {
            config,
            roles: Arc::new(DashMap::new()),
            user_roles: Arc::new(DashMap::new()),
        };
        rbac.init_default_roles();
        rbac
    }

    /// Initialize default roles
    fn init_default_roles(&self) {
        // Admin role - full access
        let admin = RoleBuilder::new("admin")
            .with_description("Full system access")
            .with_permission(
                PermissionBuilder::new(Resource::Custom("*".to_string()))
                    .with_action(Action::Custom("*".to_string()))
                    .build(),
            )
            .build();
        self.roles.insert("admin".to_string(), admin);

        // Editor role - read, write, execute
        let editor = RoleBuilder::new("editor")
            .with_description("Can read, write and execute")
            .with_permission(
                PermissionBuilder::new(Resource::MemoryItem)
                    .with_read()
                    .with_write()
                    .with_delete()
                    .build(),
            )
            .with_permission(
                PermissionBuilder::new(Resource::Category)
                    .with_read()
                    .with_write()
                    .with_delete()
                    .build(),
            )
            .with_permission(
                PermissionBuilder::new(Resource::Resource)
                    .with_read()
                    .with_write()
                    .build(),
            )
            .with_permission(
                PermissionBuilder::new(Resource::Workflow)
                    .with_read()
                    .with_execute()
                    .build(),
            )
            .with_permission(
                PermissionBuilder::new(Resource::Pipeline)
                    .with_read()
                    .with_execute()
                    .build(),
            )
            .build();
        self.roles.insert("editor".to_string(), editor);

        // Viewer role - read only
        let viewer = RoleBuilder::new("viewer")
            .with_description("Read-only access")
            .with_permission(
                PermissionBuilder::new(Resource::MemoryItem)
                    .with_read()
                    .build(),
            )
            .with_permission(
                PermissionBuilder::new(Resource::Category)
                    .with_read()
                    .build(),
            )
            .with_permission(
                PermissionBuilder::new(Resource::Resource)
                    .with_read()
                    .build(),
            )
            .build();
        self.roles.insert("viewer".to_string(), viewer);

        // Guest role - minimal access
        let guest = RoleBuilder::new("guest")
            .with_description("Minimal access")
            .with_permission(
                PermissionBuilder::new(Resource::MemoryItem)
                    .with_read()
                    .build(),
            )
            .build();
        self.roles.insert("guest".to_string(), guest);
    }

    /// Register a custom role
    pub fn register_role(&self, role: Role) -> MemResult<()> {
        if self.roles.contains_key(&role.name) {
            return Err(MemError::Authorization(format!(
                "Role '{}' already exists",
                role.name
            )));
        }
        self.roles.insert(role.name.clone(), role);
        Ok(())
    }

    /// Assign role to user
    pub fn assign_role(&self, user_id: &str, role_name: &str) -> MemResult<()> {
        if !self.roles.contains_key(role_name) {
            return Err(MemError::Authorization(format!(
                "Role '{}' not found",
                role_name
            )));
        }

        let mut roles = self
            .user_roles
            .entry(user_id.to_string())
            .or_insert_with(Vec::new);

        if !roles.contains(&role_name.to_string()) {
            roles.push(role_name.to_string());
        }

        Ok(())
    }

    /// Remove role from user
    pub fn remove_role(&self, user_id: &str, role_name: &str) -> bool {
        if let Some(mut roles) = self.user_roles.get_mut(user_id) {
            let original_len = roles.len();
            roles.retain(|r| r != role_name);
            roles.len() < original_len
        } else {
            false
        }
    }

    /// Get user roles
    pub fn get_user_roles(&self, user_id: &str) -> Vec<String> {
        self.user_roles
            .get(user_id)
            .map(|r| r.clone())
            .unwrap_or_else(|| {
                self.config
                    .default_role
                    .clone()
                    .map(|r| vec![r])
                    .unwrap_or_default()
            })
    }

    /// Check if user has permission
    pub fn check_permission(&self, user_id: &str, resource: &Resource, action: &Action) -> bool {
        if !self.config.enabled {
            return true; // RBAC disabled, allow all
        }

        let roles = self.get_user_roles(user_id);

        for role_name in roles {
            if let Some(role) = self.roles.get(&role_name) {
                if role.allows(resource, action) {
                    return true;
                }
            }
        }

        false
    }

    /// Get role by name
    pub fn get_role(&self, name: &str) -> Option<Role> {
        self.roles.get(name).map(|r| (*r.value()).clone())
    }

    /// List all roles
    pub fn list_roles(&self) -> Vec<String> {
        self.roles.iter().map(|r| r.key().clone()).collect()
    }
}

#[cfg(feature = "security")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rbac_creation() {
        let rbac = Rbac::new(RbacConfig::default());
        assert!(rbac.roles.len() >= 4); // admin, editor, viewer, guest
    }

    #[test]
    fn test_default_roles() {
        let rbac = Rbac::new(RbacConfig::default());

        // Admin should have full access
        let admin_role = rbac.get_role("admin").unwrap();
        assert!(admin_role.allows(&Resource::MemoryItem, &Action::Read));
        assert!(admin_role.allows(&Resource::MemoryItem, &Action::Write));
        assert!(admin_role.allows(&Resource::MemoryItem, &Action::Delete));

        // Viewer should have read-only access
        let viewer_role = rbac.get_role("viewer").unwrap();
        assert!(viewer_role.allows(&Resource::MemoryItem, &Action::Read));
        assert!(!viewer_role.allows(&Resource::MemoryItem, &Action::Write));
    }

    #[test]
    fn test_user_role_assignment() {
        let rbac = Rbac::new(RbacConfig::default());

        rbac.assign_role("user1", "editor").unwrap();
        let roles = rbac.get_user_roles("user1");
        assert!(roles.contains(&"editor".to_string()));
    }

    #[test]
    fn test_permission_check() {
        let rbac = Rbac::new(RbacConfig::default());

        // Assign roles to users
        rbac.assign_role("admin_user", "admin").unwrap();
        rbac.assign_role("viewer_user", "viewer").unwrap();

        // Admin can do anything
        assert!(rbac.check_permission("admin_user", &Resource::MemoryItem, &Action::Read));
        assert!(rbac.check_permission("admin_user", &Resource::MemoryItem, &Action::Write));
        assert!(rbac.check_permission("admin_user", &Resource::MemoryItem, &Action::Delete));

        // Viewer can only read
        assert!(rbac.check_permission("viewer_user", &Resource::MemoryItem, &Action::Read));
        assert!(!rbac.check_permission("viewer_user", &Resource::MemoryItem, &Action::Write));
    }

    #[test]
    fn test_custom_role() {
        let rbac = Rbac::new(RbacConfig::default());

        let custom_role = RoleBuilder::new("custom")
            .with_permission(
                PermissionBuilder::new(Resource::Workflow)
                    .with_read()
                    .with_execute()
                    .build(),
            )
            .build();

        rbac.register_role(custom_role).unwrap();
        assert!(rbac.get_role("custom").is_some());
    }
}
