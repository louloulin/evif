//! Security module for evif-mem
//!
//! Provides encryption, RBAC, audit logging, and data masking capabilities.
//!
//! # Features
//!
//! - **Encryption**: AES-256 encryption for sensitive data
//! - **RBAC**: Role-Based Access Control with permissions
//! - **Audit Logging**: Comprehensive audit trail for security events
//! - **Data Masking**: Utilities for masking sensitive data
//!
//! # Usage
//!
//! ```ignore
//! use evif_mem::security::{Encryption, Rbac, AuditLogger, mask_sensitive_data};
//! ```

#[cfg(feature = "security")]
pub mod audit;
#[cfg(feature = "security")]
pub mod encryption;
#[cfg(feature = "security")]
pub mod rbac;

pub mod masking;

#[cfg(feature = "security")]
pub use audit::{AuditConfig, AuditEvent, AuditLevel, AuditLogger};
#[cfg(feature = "security")]
pub use encryption::{Encryption, EncryptionConfig};
#[cfg(feature = "security")]
pub use rbac::{
    Action, Permission, PermissionBuilder, Rbac, RbacConfig, Resource, Role, RoleBuilder,
};

pub use masking::{mask_sensitive_data, MaskConfig, SensitiveField};

// Re-export MemResult for convenience
