// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

//! EVIF 认证和授权层

pub mod error;
pub mod capability;
pub mod auth;
pub mod audit;

pub use error::{AuthError, AuthResult};
pub use capability::{Capability, Permissions, Principal, PrincipalId, CapId};
pub use auth::{AuthManager, AuthPolicy, Permission};
pub use audit::{
    AuditEvent, AuditEventType, AuditLogger, AuditLogManager,
    MemoryAuditLogger, FileAuditLogger, AuditFilter, AuditConfig
};
