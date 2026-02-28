// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

//! 审计日志模块 - 记录所有认证和授权操作

use crate::{AuthError, AuthResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::Mutex;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

/// 审计事件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditEventType {
    /// 能力授予
    CapabilityGranted,
    /// 能力撤销
    CapabilityRevoked,
    /// 权限检查成功
    AccessGranted,
    /// 权限检查失败
    AccessDenied,
    /// 策略变更
    PolicyChanged,
    /// 认证失败
    AuthenticationFailed,
    /// 会话创建
    SessionCreated,
    /// 会话终止
    SessionTerminated,
}

/// 审计事件记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// 事件ID
    pub id: uuid::Uuid,
    /// 事件类型
    pub event_type: AuditEventType,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 主体ID (谁执行的操作)
    pub principal_id: Option<uuid::Uuid>,
    /// 资源ID (操作的节点)
    pub resource_id: Option<uuid::Uuid>,
    /// 操作结果
    pub success: bool,
    /// 事件详情
    pub details: String,
    /// IP地址 (可选)
    pub ip_address: Option<String>,
    /// 用户代理 (可选)
    pub user_agent: Option<String>,
}

impl AuditEvent {
    /// 创建新的审计事件
    pub fn new(event_type: AuditEventType, details: String) -> Self {
        AuditEvent {
            id: uuid::Uuid::new_v4(),
            event_type,
            timestamp: Utc::now(),
            principal_id: None,
            resource_id: None,
            success: true,
            details,
            ip_address: None,
            user_agent: None,
        }
    }

    /// 设置主体ID
    pub fn with_principal_id(mut self, principal_id: uuid::Uuid) -> Self {
        self.principal_id = Some(principal_id);
        self
    }

    /// 设置资源ID
    pub fn with_resource_id(mut self, resource_id: uuid::Uuid) -> Self {
        self.resource_id = Some(resource_id);
        self
    }

    /// 设置成功状态
    pub fn with_success(mut self, success: bool) -> Self {
        self.success = success;
        self
    }

    /// 设置IP地址
    pub fn with_ip_address(mut self, ip: String) -> Self {
        self.ip_address = Some(ip);
        self
    }

    /// 设置用户代理
    pub fn with_user_agent(mut self, agent: String) -> Self {
        self.user_agent = Some(agent);
        self
    }
}

/// 审计日志配置
#[derive(Debug, Clone)]
pub struct AuditConfig {
    /// 是否启用审计日志
    pub enabled: bool,
    /// 日志文件路径
    pub log_path: Option<String>,
    /// 日志轮转大小 (字节)
    pub rotation_size: usize,
    /// 是否同步写入
    pub sync_write: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        AuditConfig {
            enabled: true,
            log_path: Some("evif_audit.log".to_string()),
            rotation_size: 10 * 1024 * 1024, // 10MB
            sync_write: false,
        }
    }
}

/// 审计日志器 trait
pub trait AuditLogger: Send + Sync {
    /// 记录审计事件
    fn log(&self, event: AuditEvent) -> AuthResult<()>;

    /// 查询审计事件
    fn query(&self, filter: AuditFilter) -> AuthResult<Vec<AuditEvent>>;

    /// 清理旧事件
    fn prune(&self, before: DateTime<Utc>) -> AuthResult<usize>;
}

/// 内存审计日志器
pub struct MemoryAuditLogger {
    events: Arc<Mutex<Vec<AuditEvent>>>,
    config: AuditConfig,
}

impl MemoryAuditLogger {
    pub fn new() -> Self {
        MemoryAuditLogger {
            events: Arc::new(Mutex::new(Vec::new())),
            config: AuditConfig::default(),
        }
    }

    pub fn with_config(config: AuditConfig) -> Self {
        MemoryAuditLogger {
            events: Arc::new(Mutex::new(Vec::new())),
            config,
        }
    }

    pub fn event_count(&self) -> usize {
        self.events.lock().len()
    }
}

impl Default for MemoryAuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditLogger for MemoryAuditLogger {
    fn log(&self, event: AuditEvent) -> AuthResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut events = self.events.lock();
        events.push(event);

        // 限制内存中的事件数量
        if events.len() > 10000 {
            events.remove(0);
        }

        Ok(())
    }

    fn query(&self, filter: AuditFilter) -> AuthResult<Vec<AuditEvent>> {
        let events = self.events.lock();
        let filtered: Vec<AuditEvent> = events
            .iter()
            .filter(|e| filter.matches(e))
            .cloned()
            .collect();

        Ok(filtered)
    }

    fn prune(&self, before: DateTime<Utc>) -> AuthResult<usize> {
        let mut events = self.events.lock();
        let original_len = events.len();
        events.retain(|e| e.timestamp > before);
        Ok(original_len - events.len())
    }
}

/// 文件审计日志器
pub struct FileAuditLogger {
    inner: MemoryAuditLogger,
    log_path: String,
}

impl FileAuditLogger {
    pub fn new<P: AsRef<Path>>(path: P) -> AuthResult<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        Ok(FileAuditLogger {
            inner: MemoryAuditLogger::new(),
            log_path: path_str,
        })
    }

    pub fn with_config<P: AsRef<Path>>(path: P, config: AuditConfig) -> AuthResult<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        Ok(FileAuditLogger {
            inner: MemoryAuditLogger::with_config(config),
            log_path: path_str,
        })
    }

    /// 将事件写入文件
    fn write_to_file(&self, event: &AuditEvent) -> AuthResult<()> {
        let log_line = format!(
            "{} | {:?} | principal={:?} | resource={:?} | success={} | {}\n",
            event.timestamp.format("%Y-%m-%d %H:%M:%S%.3f UTC"),
            event.event_type,
            event.principal_id,
            event.resource_id,
            event.success,
            event.details
        );

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .map_err(|e| AuthError::IoError(format!("Failed to open audit log: {}", e)))?;

        file.write_all(log_line.as_bytes())
            .map_err(|e| AuthError::IoError(format!("Failed to write audit log: {}", e)))?;

        file.flush()
            .map_err(|e| AuthError::IoError(format!("Failed to flush audit log: {}", e)))?;

        Ok(())
    }
}

impl AuditLogger for FileAuditLogger {
    fn log(&self, event: AuditEvent) -> AuthResult<()> {
        // 先记录到内存
        self.inner.log(event.clone())?;

        // 然后写入文件
        self.write_to_file(&event)?;

        Ok(())
    }

    fn query(&self, filter: AuditFilter) -> AuthResult<Vec<AuditEvent>> {
        self.inner.query(filter)
    }

    fn prune(&self, before: DateTime<Utc>) -> AuthResult<usize> {
        self.inner.prune(before)
    }
}

/// 审计日志过滤器
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    pub event_type: Option<AuditEventType>,
    pub principal_id: Option<uuid::Uuid>,
    pub resource_id: Option<uuid::Uuid>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub success_only: Option<bool>,
}

impl AuditFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_event_type(mut self, event_type: AuditEventType) -> Self {
        self.event_type = Some(event_type);
        self
    }

    pub fn with_principal_id(mut self, principal_id: uuid::Uuid) -> Self {
        self.principal_id = Some(principal_id);
        self
    }

    pub fn with_resource_id(mut self, resource_id: uuid::Uuid) -> Self {
        self.resource_id = Some(resource_id);
        self
    }

    pub fn with_start_time(mut self, start: DateTime<Utc>) -> Self {
        self.start_time = Some(start);
        self
    }

    pub fn with_end_time(mut self, end: DateTime<Utc>) -> Self {
        self.end_time = Some(end);
        self
    }

    pub fn with_success_only(mut self, success: bool) -> Self {
        self.success_only = Some(success);
        self
    }

    pub fn matches(&self, event: &AuditEvent) -> bool {
        if let Some(ref event_type) = self.event_type {
            if &event.event_type != event_type {
                return false;
            }
        }

        if let Some(ref principal_id) = self.principal_id {
            if event.principal_id.as_ref() != Some(principal_id) {
                return false;
            }
        }

        if let Some(ref resource_id) = self.resource_id {
            if event.resource_id.as_ref() != Some(resource_id) {
                return false;
            }
        }

        if let Some(start) = self.start_time {
            if event.timestamp < start {
                return false;
            }
        }

        if let Some(end) = self.end_time {
            if event.timestamp > end {
                return false;
            }
        }

        if let Some(success) = self.success_only {
            if event.success != success {
                return false;
            }
        }

        true
    }
}

/// 审计日志管理器
pub struct AuditLogManager {
    logger: Arc<dyn AuditLogger>,
}

impl AuditLogManager {
    pub fn new(logger: Arc<dyn AuditLogger>) -> Self {
        AuditLogManager { logger }
    }

    pub fn from_memory() -> Self {
        AuditLogManager {
            logger: Arc::new(MemoryAuditLogger::new()),
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> AuthResult<Self> {
        Ok(AuditLogManager {
            logger: Arc::new(FileAuditLogger::new(path)?),
        })
    }

    /// 记录能力授予
    pub fn log_capability_granted(
        &self,
        principal_id: uuid::Uuid,
        resource_id: uuid::Uuid,
    ) -> AuthResult<()> {
        let event = AuditEvent::new(
            AuditEventType::CapabilityGranted,
            format!("Capability granted for principal {} on resource {}", principal_id, resource_id),
        )
            .with_principal_id(principal_id)
            .with_resource_id(resource_id);

        self.logger.log(event)
    }

    /// 记录能力撤销
    pub fn log_capability_revoked(
        &self,
        principal_id: uuid::Uuid,
        resource_id: uuid::Uuid,
    ) -> AuthResult<()> {
        let event = AuditEvent::new(
            AuditEventType::CapabilityRevoked,
            format!("Capability revoked for principal {} on resource {}", principal_id, resource_id),
        )
            .with_principal_id(principal_id)
            .with_resource_id(resource_id);

        self.logger.log(event)
    }

    /// 记录访问授权
    pub fn log_access_granted(
        &self,
        principal_id: uuid::Uuid,
        resource_id: uuid::Uuid,
        permission: &str,
    ) -> AuthResult<()> {
        let event = AuditEvent::new(
            AuditEventType::AccessGranted,
            format!("Access granted: {} permission for principal {} on resource {}", permission, principal_id, resource_id),
        )
            .with_principal_id(principal_id)
            .with_resource_id(resource_id)
            .with_success(true);

        self.logger.log(event)
    }

    /// 记录访问拒绝
    pub fn log_access_denied(
        &self,
        principal_id: uuid::Uuid,
        resource_id: uuid::Uuid,
        permission: &str,
        reason: &str,
    ) -> AuthResult<()> {
        let event = AuditEvent::new(
            AuditEventType::AccessDenied,
            format!("Access denied: {} permission for principal {} on resource {} - {}", permission, principal_id, resource_id, reason),
        )
            .with_principal_id(principal_id)
            .with_resource_id(resource_id)
            .with_success(false);

        self.logger.log(event)
    }

    /// 记录认证失败
    pub fn log_auth_failed(&self, principal_id: uuid::Uuid, reason: &str) -> AuthResult<()> {
        let event = AuditEvent::new(
            AuditEventType::AuthenticationFailed,
            format!("Authentication failed for principal {}: {}", principal_id, reason),
        )
            .with_principal_id(principal_id)
            .with_success(false);

        self.logger.log(event)
    }

    /// 记录策略变更
    pub fn log_policy_changed(&self, old_policy: &str, new_policy: &str) -> AuthResult<()> {
        let event = AuditEvent::new(
            AuditEventType::PolicyChanged,
            format!("Policy changed from {} to {}", old_policy, new_policy),
        );

        self.logger.log(event)
    }

    /// 查询审计日志
    pub fn query(&self, filter: AuditFilter) -> AuthResult<Vec<AuditEvent>> {
        self.logger.query(filter)
    }

    /// 清理旧日志
    pub fn prune(&self, before: DateTime<Utc>) -> AuthResult<usize> {
        self.logger.prune(before)
    }

    /// 获取底层日志器
    pub fn logger(&self) -> Arc<dyn AuditLogger> {
        self.logger.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent::new(
            AuditEventType::AccessGranted,
            "Test event".to_string(),
        );

        assert_eq!(event.details, "Test event");
        assert_eq!(event.success, true);
        assert!(event.principal_id.is_none());
        assert!(event.resource_id.is_none());
    }

    #[test]
    fn test_audit_event_builder() {
        let principal_id = uuid::Uuid::new_v4();
        let resource_id = uuid::Uuid::new_v4();

        let event = AuditEvent::new(
            AuditEventType::AccessDenied,
            "Test event".to_string(),
        )
            .with_principal_id(principal_id)
            .with_resource_id(resource_id)
            .with_success(false)
            .with_ip_address("127.0.0.1".to_string());

        assert_eq!(event.principal_id, Some(principal_id));
        assert_eq!(event.resource_id, Some(resource_id));
        assert_eq!(event.success, false);
        assert_eq!(event.ip_address, Some("127.0.0.1".to_string()));
    }

    #[test]
    fn test_memory_audit_logger() {
        let logger = MemoryAuditLogger::new();
        let event = AuditEvent::new(
            AuditEventType::AccessGranted,
            "Test event".to_string(),
        );

        logger.log(event.clone()).unwrap();
        assert_eq!(logger.event_count(), 1);

        let events = logger.query(AuditFilter::new()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].details, "Test event");
    }

    #[test]
    fn test_audit_filter() {
        let logger = MemoryAuditLogger::new();
        let principal_id = uuid::Uuid::new_v4();
        let resource_id = uuid::Uuid::new_v4();

        // 添加不同类型的事件
        let event1 = AuditEvent::new(
            AuditEventType::AccessGranted,
            "Granted".to_string(),
        ).with_principal_id(principal_id);

        let event2 = AuditEvent::new(
            AuditEventType::AccessDenied,
            "Denied".to_string(),
        ).with_principal_id(principal_id);

        logger.log(event1).unwrap();
        logger.log(event2).unwrap();

        // 测试过滤器
        let filter = AuditFilter::new()
            .with_event_type(AuditEventType::AccessGranted);

        let events = logger.query(filter).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].details, "Granted");
    }

    #[test]
    fn test_audit_log_prune() {
        let logger = MemoryAuditLogger::new();
        let event = AuditEvent::new(
            AuditEventType::AccessGranted,
            "Test event".to_string(),
        );

        logger.log(event).unwrap();

        // 删除所有旧事件
        let now = Utc::now();
        let count = logger.prune(now + chrono::Duration::hours(1)).unwrap();
        assert_eq!(count, 1);
        assert_eq!(logger.event_count(), 0);
    }

    #[test]
    fn test_audit_log_manager() {
        let manager = AuditLogManager::from_memory();
        let principal_id = uuid::Uuid::new_v4();
        let resource_id = uuid::Uuid::new_v4();

        // 测试记录不同类型的事件
        manager.log_capability_granted(principal_id, resource_id).unwrap();
        manager.log_access_granted(principal_id, resource_id, "read").unwrap();
        manager.log_access_denied(principal_id, resource_id, "write", "no permission").unwrap();

        let events = manager.query(AuditFilter::new()).unwrap();
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_audit_config_default() {
        let config = AuditConfig::default();
        assert!(config.enabled);
        assert_eq!(config.log_path, Some("evif_audit.log".to_string()));
        assert_eq!(config.rotation_size, 10 * 1024 * 1024);
        assert!(!config.sync_write);
    }

    #[test]
    fn test_audit_filter_matches() {
        let filter = AuditFilter::new()
            .with_event_type(AuditEventType::AccessDenied)
            .with_success_only(false);

        let matching_event = AuditEvent::new(
            AuditEventType::AccessDenied,
            "Test".to_string(),
        ).with_success(false);

        let non_matching_event = AuditEvent::new(
            AuditEventType::AccessGranted,
            "Test".to_string(),
        ).with_success(false);

        assert!(filter.matches(&matching_event));
        assert!(!filter.matches(&non_matching_event));
    }
}
