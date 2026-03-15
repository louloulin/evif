// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type PrincipalId = Uuid;
pub type CapId = Uuid;

/// 主体类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Principal {
    User(Uuid),
    Service(Uuid),
    System,
}

/// 权限
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub admin: bool,
}

impl Permissions {
    pub fn new() -> Self {
        Permissions {
            read: false,
            write: false,
            execute: false,
            admin: false,
        }
    }

    pub fn read() -> Self {
        Permissions {
            read: true,
            write: false,
            execute: false,
            admin: false,
        }
    }

    pub fn read_write() -> Self {
        Permissions {
            read: true,
            write: true,
            execute: false,
            admin: false,
        }
    }

    pub fn all() -> Self {
        Permissions {
            read: true,
            write: true,
            execute: true,
            admin: true,
        }
    }
}

impl Default for Permissions {
    fn default() -> Self {
        Self::new()
    }
}

/// 能力 - 表示访问权限
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub id: CapId,
    pub holder: PrincipalId,
    pub node: Uuid,
    pub permissions: Permissions,
    pub expires: Option<DateTime<Utc>>,
}

impl Capability {
    pub fn new(holder: PrincipalId, node: Uuid, permissions: Permissions) -> Self {
        Capability {
            id: Uuid::new_v4(),
            holder,
            node,
            permissions,
            expires: None,
        }
    }

    pub fn with_expiry(mut self, expires: DateTime<Utc>) -> Self {
        self.expires = Some(expires);
        self
    }

    pub fn is_valid(&self) -> bool {
        if let Some(expiry) = self.expires {
            expiry > Utc::now()
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permissions() {
        let perm = Permissions::read();
        assert!(perm.read);
        assert!(!perm.write);
    }

    #[test]
    fn test_capability() {
        let holder = Uuid::new_v4();
        let node = Uuid::new_v4();
        let cap = Capability::new(holder, node, Permissions::read());
        assert!(cap.is_valid());
    }

    #[test]
    fn test_capability_expiry() {
        let holder = Uuid::new_v4();
        let node = Uuid::new_v4();
        let expired_cap = Capability::new(holder, node, Permissions::read())
            .with_expiry(Utc::now() - chrono::Duration::hours(1));
        assert!(!expired_cap.is_valid());
    }
}
