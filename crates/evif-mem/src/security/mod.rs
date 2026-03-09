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
pub mod encryption;
#[cfg(feature = "security")]
pub mod rbac;
#[cfg(feature = "security")]
pub mod audit;

pub mod masking;

#[cfg(feature = "security")]
pub use encryption::{Encryption, EncryptionConfig};
#[cfg(feature = "security")]
pub use rbac::{
    Rbac, RbacConfig, Role, Permission, Resource, Action,
    RoleBuilder, PermissionBuilder,
};
#[cfg(feature = "security")]
pub use audit::{AuditLogger, AuditConfig, AuditEvent, AuditLevel};

pub use masking::{mask_sensitive_data, MaskConfig, SensitiveField};

// Re-export MemResult for convenience
use crate::error::MemResult;
