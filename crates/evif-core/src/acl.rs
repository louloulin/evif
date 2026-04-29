// ACL (Access Control List) System
//
// 提供细粒度的文件访问控制，支持多用户和权限管理
//
// 功能：
// - ACL 数据结构
// - 权限位定义
// - 继承和默认规则
// - 权限验证逻辑

use crate::error::EvifResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

// 权限位定义
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AclPermissions: u32 {
        // 基本权限
        const READ      = 1 << 0;  // 读权限
        const WRITE     = 1 << 1;  // 写权限
        const EXECUTE   = 1 << 2;  // 执行权限

        // 高级权限
        const DELETE     = 1 << 3;  // 删除权限
        const READ_ACL  = 1 << 4;  // 读取 ACL 权限
        const WRITE_ACL = 1 << 5;  // 修改 ACL 权限

        // 管理权限
        const ADMIN      = 1 << 7;  // 管理员权限

        // 组合权限
        const ALL        = 0xFF;    // 所有权限
    }
}

impl Default for AclPermissions {
    fn default() -> Self {
        AclPermissions::READ | AclPermissions::WRITE | AclPermissions::EXECUTE
    }
}

// 为 AclPermissions 实现序列化支持
impl Serialize for AclPermissions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.bits().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AclPermissions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bits = u32::deserialize(deserializer)?;
        Ok(AclPermissions::from_bits_retain(bits))
    }
}

/// ACL 条目类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AclType {
    /// 用户 ACL
    User,

    /// 组 ACL
    Group,

    /// 其他/所有人 ACL
    Other,

    /// 掩码 ACL（特殊权限）
    Mask,
}

/// ACL 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AclEntry {
    /// ACL 类型
    pub acl_type: AclType,

    /// 标识符（用户名/组名）
    pub identifier: String,

    /// 权限
    pub permissions: AclPermissions,

    /// 是否继承（用于目录）
    pub inherit: bool,

    /// 是否是默认 ACL
    pub is_default: bool,
}

impl AclEntry {
    /// 创建用户 ACL
    pub fn user(identifier: String, permissions: AclPermissions) -> Self {
        Self {
            acl_type: AclType::User,
            identifier,
            permissions,
            inherit: false,
            is_default: false,
        }
    }

    /// 创建组 ACL
    pub fn group(identifier: String, permissions: AclPermissions) -> Self {
        Self {
            acl_type: AclType::Group,
            identifier,
            permissions,
            inherit: false,
            is_default: false,
        }
    }

    /// 创建继承 ACL
    pub fn inherit(mut self) -> Self {
        self.inherit = true;
        self
    }

    /// 创建默认 ACL
    pub fn default_acl(mut self) -> Self {
        self.is_default = true;
        self
    }
}

/// 用户上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    /// 用户名
    pub username: String,

    /// 所属组
    pub groups: Vec<String>,

    /// 是否是管理员
    pub is_admin: bool,

    /// 认证令牌
    pub token: Option<String>,
}

impl UserContext {
    pub fn new(username: String, groups: Vec<String>, is_admin: bool) -> Self {
        Self {
            username,
            groups,
            is_admin,
            token: None,
        }
    }

    pub fn anonymous() -> Self {
        Self {
            username: "anonymous".to_string(),
            groups: vec!["anonymous".to_string()],
            is_admin: false,
            token: None,
        }
    }

    pub fn with_token(mut self, token: String) -> Self {
        self.token = Some(token);
        self
    }
}

/// ACL 检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AclCheckResult {
    /// 是否允许访问
    pub allowed: bool,

    /// 命中的权限
    pub permissions: AclPermissions,

    /// 匹配的 ACL 条目
    pub matched_entry: Option<AclEntry>,

    /// 拒绝原因
    pub denied_reason: Option<String>,
}

/// ACL 管理器
#[derive(Clone)]
pub struct AclManager {
    /// ACL 存储映射 (path -> acl_list)
    acls: Arc<RwLock<HashMap<String, Vec<AclEntry>>>>,

    /// 用户缓存
    user_cache: Arc<RwLock<HashMap<String, UserContext>>>,

    /// 是否启用 ACL
    enabled: Arc<RwLock<bool>>,
}

impl AclManager {
    pub fn new(enabled: bool) -> Self {
        Self {
            acls: Arc::new(RwLock::new(HashMap::new())),
            user_cache: Arc::new(RwLock::new(HashMap::new())),
            enabled: Arc::new(RwLock::new(enabled)),
        }
    }

    /// 启用/禁用 ACL
    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.write() = enabled;
    }

    /// 检查 ACL 是否启用
    pub fn is_enabled(&self) -> bool {
        *self.enabled.read()
    }

    /// 设置文件 ACL
    pub async fn set_acl(&self, path: String, entries: Vec<AclEntry>) -> EvifResult<()> {
        let mut acls = self.acls.write();
        acls.insert(path, entries);
        Ok(())
    }

    /// 获取文件 ACL
    pub async fn get_acl(&self, path: &str) -> Option<Vec<AclEntry>> {
        let acls = self.acls.read();
        acls.get(path).cloned()
    }

    /// 删除文件 ACL
    pub async fn remove_acl(&self, path: &str) -> EvifResult<()> {
        let mut acls = self.acls.write();
        acls.remove(path);
        Ok(())
    }

    /// 添加用户上下文
    pub fn add_user(&self, user: UserContext) {
        let mut cache = self.user_cache.write();
        cache.insert(user.username.clone(), user);
    }

    /// 获取用户上下文
    pub fn get_user(&self, username: &str) -> Option<UserContext> {
        let cache = self.user_cache.read();
        cache.get(username).cloned()
    }

    /// 检查权限
    pub async fn check_permission(
        &self,
        path: &str,
        user: &UserContext,
        required: AclPermissions,
    ) -> AclCheckResult {
        // 如果 ACL 未启用，允许所有操作
        if !self.is_enabled() {
            return AclCheckResult {
                allowed: true,
                permissions: AclPermissions::ALL,
                matched_entry: None,
                denied_reason: None,
            };
        }

        // 管理员拥有所有权限
        if user.is_admin {
            return AclCheckResult {
                allowed: true,
                permissions: AclPermissions::ALL,
                matched_entry: None,
                denied_reason: None,
            };
        }

        // 获取文件 ACL
        let acls = match self.get_acl(path).await {
            Some(entries) => entries,
            None => {
                // 没有显式 ACL，使用默认规则
                return self.check_default_permission(user, required);
            }
        };

        // 检查所有 ACL 条目
        for entry in &acls {
            if let Some(permissions) = self.match_acl_entry(entry, user) {
                // 检查是否拥有所需权限
                if permissions.contains(required) {
                    return AclCheckResult {
                        allowed: true,
                        permissions,
                        matched_entry: Some(entry.clone()),
                        denied_reason: None,
                    };
                }
            }
        }

        // 没有匹配的 ACL，拒绝访问
        AclCheckResult {
            allowed: false,
            permissions: AclPermissions::empty(),
            matched_entry: None,
            denied_reason: Some("No matching ACL entry".to_string()),
        }
    }

    /// 检查默认权限
    fn check_default_permission(
        &self,
        user: &UserContext,
        required: AclPermissions,
    ) -> AclCheckResult {
        // 默认规则：
        // 1. 文件所有者拥有所有权限
        // 2. 组成员拥有读写权限
        // 3. 其他人拥有只读权限

        if user.username == "owner" || user.is_admin {
            return AclCheckResult {
                allowed: true,
                permissions: AclPermissions::ALL,
                matched_entry: None,
                denied_reason: None,
            };
        }

        if user.groups.contains(&"users".to_string()) {
            let permissions = AclPermissions::READ | AclPermissions::WRITE;
            if permissions.contains(required) {
                return AclCheckResult {
                    allowed: true,
                    permissions,
                    matched_entry: None,
                    denied_reason: None,
                };
            }
            return AclCheckResult {
                allowed: false,
                permissions,
                matched_entry: None,
                denied_reason: Some("Insufficient group permissions".to_string()),
            };
        }

        // 默认其他人只有读权限
        if required == AclPermissions::READ {
            return AclCheckResult {
                allowed: true,
                permissions: AclPermissions::READ,
                matched_entry: None,
                denied_reason: None,
            };
        }

        AclCheckResult {
            allowed: false,
            permissions: AclPermissions::empty(),
            matched_entry: None,
            denied_reason: Some("Default permission denied".to_string()),
        }
    }

    /// 匹配 ACL 条目
    fn match_acl_entry(&self, entry: &AclEntry, user: &UserContext) -> Option<AclPermissions> {
        match entry.acl_type {
            AclType::User => {
                if entry.identifier == user.username || user.is_admin {
                    Some(entry.permissions)
                } else {
                    None
                }
            }
            AclType::Group => {
                if user.groups.contains(&entry.identifier) || user.is_admin {
                    Some(entry.permissions)
                } else {
                    None
                }
            }
            AclType::Other => {
                // 其他 ACL 总是匹配
                Some(entry.permissions)
            }
            AclType::Mask => {
                // 掩码 ACL 用于权限过滤，暂不处理
                None
            }
        }
    }

    /// 批量设置 ACL
    pub async fn set_acl_batch(&self, acl_map: HashMap<String, Vec<AclEntry>>) -> EvifResult<()> {
        let mut acls = self.acls.write();
        for (path, entries) in acl_map {
            acls.insert(path, entries);
        }
        Ok(())
    }

    /// 获取所有 ACL
    pub async fn get_all_acls(&self) -> HashMap<String, Vec<AclEntry>> {
        let acls = self.acls.read();
        acls.clone()
    }

    /// 清除所有 ACL
    pub async fn clear_all(&self) {
        let mut acls = self.acls.write();
        acls.clear();
    }

    /// 清除用户缓存
    pub fn clear_user_cache(&self) {
        let mut cache = self.user_cache.write();
        cache.clear();
    }
}

/// ACL 支持的文件系统扩展 trait
#[async_trait]
pub trait AclSupported: Send + Sync {
    /// 设置文件 ACL
    async fn set_file_acl(&self, path: &str, entries: Vec<AclEntry>) -> EvifResult<()>;

    /// 获取文件 ACL
    async fn get_file_acl(&self, path: &str) -> EvifResult<Vec<AclEntry>>;

    /// 删除文件 ACL
    async fn remove_file_acl(&self, path: &str) -> EvifResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acl_permissions() {
        let read = AclPermissions::READ;
        let write = AclPermissions::WRITE;
        let rw = read | write;

        assert!(rw.contains(AclPermissions::READ));
        assert!(rw.contains(AclPermissions::WRITE));
        assert!(!rw.contains(AclPermissions::EXECUTE));

        let all = AclPermissions::ALL;
        assert!(all.contains(rw));
    }

    #[test]
    fn test_acl_entry() {
        let user_entry = AclEntry::user(
            "alice".to_string(),
            AclPermissions::READ | AclPermissions::WRITE,
        );
        assert_eq!(user_entry.acl_type, AclType::User);
        assert_eq!(user_entry.identifier, "alice");

        let group_entry = AclEntry::group("developers".to_string(), AclPermissions::ALL)
            .inherit()
            .default_acl();
        assert!(group_entry.inherit);
        assert!(group_entry.is_default);
    }

    #[tokio::test]
    async fn test_acl_manager() {
        let manager = AclManager::new(true);

        // 添加用户
        let user = UserContext::new("alice".to_string(), vec!["developers".to_string()], false);
        manager.add_user(user);

        // 设置 ACL
        let entries = vec![
            AclEntry::user("alice".to_string(), AclPermissions::ALL),
            AclEntry::group(
                "developers".to_string(),
                AclPermissions::READ | AclPermissions::WRITE,
            ),
        ];
        manager
            .set_acl("/test/file.txt".to_string(), entries)
            .await
            .unwrap();

        // 检查权限
        let user = manager.get_user("alice").unwrap();
        let result = manager
            .check_permission("/test/file.txt", &user, AclPermissions::READ)
            .await;
        assert!(result.allowed);
        assert_eq!(result.permissions, AclPermissions::ALL);
    }

    #[tokio::test]
    async fn test_acl_default() {
        let manager = AclManager::new(true);

        let user = UserContext::new("bob".to_string(), vec!["users".to_string()], false);
        manager.add_user(user.clone());

        // 测试默认权限（未设置显式 ACL）
        let result = manager
            .check_permission("/some/file.txt", &user, AclPermissions::READ)
            .await;
        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_acl_admin() {
        let manager = AclManager::new(true);

        let admin = UserContext::new("admin".to_string(), vec![], true);
        manager.add_user(admin);

        // 管理员应该拥有所有权限
        let admin_user = manager.get_user("admin").unwrap();
        let result = manager
            .check_permission(
                "/protected/file.txt",
                &admin_user,
                AclPermissions::WRITE | AclPermissions::DELETE,
            )
            .await;
        assert!(result.allowed);
        assert_eq!(result.permissions, AclPermissions::ALL);
    }

    #[tokio::test]
    async fn test_acl_disabled() {
        let manager = AclManager::new(false); // 禁用 ACL

        let user = UserContext::anonymous();
        let result = manager
            .check_permission("/any/file.txt", &user, AclPermissions::ALL)
            .await;

        // ACL 禁用时，所有操作都允许
        assert!(result.allowed);
    }
}
