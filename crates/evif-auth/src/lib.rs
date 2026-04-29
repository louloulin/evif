// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

//! EVIF 认证和授权层

pub mod audit;
pub mod auth;
pub mod capability;
pub mod error;

pub use audit::{
    AuditConfig, AuditEvent, AuditEventType, AuditFilter, AuditLogManager, AuditLogger,
    FileAuditLogger, MemoryAuditLogger,
};
pub use auth::{AuthManager, AuthPolicy, JwtError, JwtValidator, Permission};
pub use capability::{CapId, Capability, Permissions, Principal, PrincipalId};
pub use error::{AuthError, AuthResult};
