//! Audit logging module
//!
//! Provides comprehensive audit trail for security events.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::{MemError, MemResult};

/// Audit event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEvent {
    /// Resource access event
    Access {
        user_id: String,
        resource: String,
        resource_id: String,
        action: String,
        success: bool,
    },
    /// Authentication event
    Auth {
        user_id: String,
        action: String,
        success: bool,
        reason: Option<String>,
    },
    /// Authorization event (permission check)
    Authorization {
        user_id: String,
        resource: String,
        action: String,
        granted: bool,
    },
    /// Data modification event
    Modify {
        user_id: String,
        resource: String,
        resource_id: String,
        operation: String,
    },
    /// Data deletion event
    Delete {
        user_id: String,
        resource: String,
        resource_id: String,
    },
    /// Export event
    Export {
        user_id: String,
        resource: String,
        count: usize,
    },
    /// Security event
    Security {
        user_id: Option<String>,
        event_type: String,
        details: String,
    },
    /// System event
    System { event_type: String, details: String },
}

impl AuditEvent {
    /// Get event type name
    pub fn event_type(&self) -> &str {
        match self {
            AuditEvent::Access { .. } => "access",
            AuditEvent::Auth { .. } => "auth",
            AuditEvent::Authorization { .. } => "authorization",
            AuditEvent::Modify { .. } => "modify",
            AuditEvent::Delete { .. } => "delete",
            AuditEvent::Export { .. } => "export",
            AuditEvent::Security { .. } => "security",
            AuditEvent::System { .. } => "system",
        }
    }
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique entry ID
    pub id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Event type
    pub event_type: String,
    /// Event details (JSON)
    pub event: AuditEvent,
    /// User ID (if authenticated)
    pub user_id: Option<String>,
    /// IP address (if available)
    pub ip_address: Option<String>,
}

/// Audit level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuditLevel {
    /// Debug level
    Debug,
    /// Info level
    Info,
    /// Warning level
    Warning,
    /// Error level
    Error,
    /// Critical level
    Critical,
}

impl AuditLevel {
    /// Get level name
    pub fn name(&self) -> &str {
        match self {
            AuditLevel::Debug => "DEBUG",
            AuditLevel::Info => "INFO",
            AuditLevel::Warning => "WARNING",
            AuditLevel::Error => "ERROR",
            AuditLevel::Critical => "CRITICAL",
        }
    }
}

/// Audit configuration
#[derive(Debug, Clone)]
pub struct AuditConfig {
    /// Enable audit logging
    pub enabled: bool,
    /// Minimum level to log
    pub min_level: AuditLevel,
    /// Maximum entries to keep in memory (0 = unlimited)
    pub max_entries: usize,
    /// Log to file
    pub log_to_file: bool,
    /// Log file path
    pub log_file: Option<String>,
    /// Log to stdout
    pub log_to_stdout: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_level: AuditLevel::Info,
            max_entries: 10000,
            log_to_file: false,
            log_file: None,
            log_to_stdout: true,
        }
    }
}

/// Audit logger
#[derive(Debug, Clone)]
pub struct AuditLogger {
    config: AuditConfig,
    entries: Arc<DashMap<String, AuditEntry>>,
    #[allow(dead_code)]
    // Keep for future file logging implementation
    file_handle: Arc<RwLock<Option<std::fs::File>>>,
}

impl AuditLogger {
    /// Create new audit logger
    pub fn new(config: AuditConfig) -> Self {
        Self {
            config,
            entries: Arc::new(DashMap::new()),
            file_handle: Arc::new(RwLock::new(None)),
        }
    }

    /// Log an audit event
    pub fn log(&self, event: AuditEvent) {
        if !self.config.enabled {
            return;
        }

        let entry = self.create_entry(event);

        // Add to in-memory store
        self.entries.insert(entry.id.clone(), entry.clone());

        // Trim old entries if needed
        if self.config.max_entries > 0 && self.entries.len() > self.config.max_entries {
            self.trim_entries();
        }

        // Log to stdout (using tracing)
        let level = self.get_level_for_event(&entry);
        if level >= self.config.min_level {
            let msg = format!(
                "[AUDIT] {} - {}: {:?}",
                entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
                entry.event_type,
                entry.event
            );
            match level {
                AuditLevel::Debug | AuditLevel::Info => tracing::info!("{}", msg),
                AuditLevel::Warning => tracing::warn!("{}", msg),
                AuditLevel::Error => tracing::error!("{}", msg),
                AuditLevel::Critical => tracing::error!("[CRITICAL] {}", msg),
            }
        }
    }

    /// Create audit entry
    fn create_entry(&self, event: AuditEvent) -> AuditEntry {
        let user_id = match &event {
            AuditEvent::Access { user_id, .. } => Some(user_id.clone()),
            AuditEvent::Auth { user_id, .. } => Some(user_id.clone()),
            AuditEvent::Authorization { user_id, .. } => Some(user_id.clone()),
            AuditEvent::Modify { user_id, .. } => Some(user_id.clone()),
            AuditEvent::Delete { user_id, .. } => Some(user_id.clone()),
            AuditEvent::Export { user_id, .. } => Some(user_id.clone()),
            AuditEvent::Security { user_id, .. } => user_id.clone(),
            AuditEvent::System { .. } => None,
        };

        AuditEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type: event.event_type().to_string(),
            event,
            user_id,
            ip_address: None,
        }
    }

    /// Get audit level for event
    fn get_level_for_event(&self, entry: &AuditEntry) -> AuditLevel {
        match &entry.event {
            AuditEvent::Security { .. } => AuditLevel::Warning,
            AuditEvent::Auth { success, .. } => {
                if *success {
                    AuditLevel::Info
                } else {
                    AuditLevel::Warning
                }
            }
            AuditEvent::Authorization { granted, .. } => {
                if *granted {
                    AuditLevel::Debug
                } else {
                    AuditLevel::Warning
                }
            }
            AuditEvent::Delete { .. } => AuditLevel::Warning,
            _ => AuditLevel::Info,
        }
    }

    /// Trim old entries
    fn trim_entries(&self) {
        let excess = self.entries.len() - self.config.max_entries;
        if excess > 0 {
            let keys: Vec<_> = self
                .entries
                .iter()
                .take(excess)
                .map(|e| e.key().clone())
                .collect();

            for key in keys {
                self.entries.remove(&key);
            }
        }
    }

    /// Get entries for a user
    pub fn get_user_entries(&self, user_id: &str) -> Vec<AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.user_id.as_deref() == Some(user_id))
            .map(|e| e.value().clone())
            .collect()
    }

    /// Get entries for a resource
    pub fn get_resource_entries(&self, resource: &str) -> Vec<AuditEntry> {
        self.entries
            .iter()
            .filter(|e| match &e.event {
                AuditEvent::Access { resource: r, .. } => r == resource,
                AuditEvent::Modify { resource: r, .. } => r == resource,
                AuditEvent::Delete { resource: r, .. } => r == resource,
                _ => false,
            })
            .map(|e| e.value().clone())
            .collect()
    }

    /// Get recent entries
    pub fn get_recent(&self, limit: usize) -> Vec<AuditEntry> {
        // Collect all entries and sort by timestamp descending
        let mut entries: Vec<_> = self.entries.iter().map(|e| e.value().clone()).collect();

        // Sort by timestamp descending (most recent first)
        entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        entries.into_iter().take(limit).collect()
    }

    /// Get entry count
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// Convenience methods for common audit events
impl AuditLogger {
    /// Log access event
    pub fn log_access(
        &self,
        user_id: &str,
        resource: &str,
        resource_id: &str,
        action: &str,
        success: bool,
    ) {
        self.log(AuditEvent::Access {
            user_id: user_id.to_string(),
            resource: resource.to_string(),
            resource_id: resource_id.to_string(),
            action: action.to_string(),
            success,
        });
    }

    /// Log authentication event
    pub fn log_auth(&self, user_id: &str, action: &str, success: bool, reason: Option<String>) {
        self.log(AuditEvent::Auth {
            user_id: user_id.to_string(),
            action: action.to_string(),
            success,
            reason,
        });
    }

    /// Log authorization event
    pub fn log_authorization(&self, user_id: &str, resource: &str, action: &str, granted: bool) {
        self.log(AuditEvent::Authorization {
            user_id: user_id.to_string(),
            resource: resource.to_string(),
            action: action.to_string(),
            granted,
        });
    }

    /// Log security event
    pub fn log_security(&self, user_id: Option<String>, event_type: &str, details: &str) {
        self.log(AuditEvent::Security {
            user_id,
            event_type: event_type.to_string(),
            details: details.to_string(),
        });
    }
}

#[cfg(feature = "security")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_logger_creation() {
        let logger = AuditLogger::new(AuditConfig::default());
        assert!(logger.is_empty());
    }

    #[test]
    fn test_log_access() {
        let logger = AuditLogger::new(AuditConfig::default());
        logger.log_access("user1", "memory_item", "item-123", "read", true);

        assert_eq!(logger.len(), 1);
        let entries = logger.get_user_entries("user1");
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_log_auth_failure() {
        let logger = AuditLogger::new(AuditConfig::default());
        logger.log_auth(
            "user1",
            "login",
            false,
            Some("Invalid password".to_string()),
        );

        assert_eq!(logger.len(), 1);
    }

    #[test]
    fn test_get_recent() {
        let logger = AuditLogger::new(AuditConfig::default());

        for i in 0..15 {
            logger.log_access(
                &format!("user{}", i),
                "memory_item",
                &format!("id-{}", i),
                "read",
                true,
            );
        }

        let recent = logger.get_recent(5);
        assert_eq!(recent.len(), 5);
    }
}
