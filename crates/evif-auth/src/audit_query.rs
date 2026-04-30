// Copyright 2025 EVIF Development Team
// SPDX-License-Identifier: MIT OR Apache-2.0

//! 审计查询增强模块
//!
//! 提供高级审计查询、统计报告和导出功能。

use crate::{AuditEvent, AuditFilter, AuditLogManager, AuthResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 导出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// JSON 格式
    Json,
    /// CSV 格式
    Csv,
}

impl Default for ExportFormat {
    fn default() -> Self {
        ExportFormat::Json
    }
}

/// 审计查询条件（增强版）
///
/// 在 `AuditFilter` 基础上增加了排序和分页功能
#[derive(Debug, Clone, Default)]
pub struct AuditQuery {
    /// 基础过滤条件
    pub filter: AuditFilter,
    /// 排序字段
    pub sort_by: Option<SortField>,
    /// 排序方向
    pub sort_order: SortOrder,
    /// 分页：跳过前 N 条
    pub offset: Option<usize>,
    /// 分页：最多返回 N 条
    pub limit: Option<usize>,
}

/// 排序字段
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    /// 按时间排序
    Timestamp,
    /// 按事件类型排序
    EventType,
    /// 按主体ID排序
    PrincipalId,
    /// 按成功状态排序
    Success,
}

impl Default for SortField {
    fn default() -> Self {
        SortField::Timestamp
    }
}

/// 排序方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    /// 升序
    Asc,
    /// 降序
    Desc,
}

impl Default for SortOrder {
    fn default() -> Self {
        SortOrder::Desc
    }
}

/// 审计统计报告
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditStats {
    /// 总事件数
    pub total_events: u64,
    /// 成功事件数
    pub success_count: u64,
    /// 失败事件数
    pub failure_count: u64,
    /// 按事件类型统计
    pub events_by_type: HashMap<String, u64>,
    /// 按日期统计（格式: YYYY-MM-DD）
    pub events_by_date: HashMap<String, u64>,
    /// 成功率 (0.0 - 1.0)
    pub success_rate: f64,
    /// 时间范围
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
}

impl AuditStats {
    /// 创建空的统计报告
    pub fn new() -> Self {
        AuditStats {
            total_events: 0,
            success_count: 0,
            failure_count: 0,
            events_by_type: HashMap::new(),
            events_by_date: HashMap::new(),
            success_rate: 0.0,
            time_range: None,
        }
    }
}

impl Default for AuditStats {
    fn default() -> Self {
        Self::new()
    }
}

/// 审计查询构建器
#[derive(Debug, Clone, Default)]
pub struct AuditQueryBuilder {
    query: AuditQuery,
}

impl AuditQueryBuilder {
    pub fn new() -> Self {
        AuditQueryBuilder {
            query: AuditQuery::default(),
        }
    }

    /// 设置过滤条件
    pub fn with_filter(mut self, filter: AuditFilter) -> Self {
        self.query.filter = filter;
        self
    }

    /// 按时间排序
    pub fn sort_by_time(mut self) -> Self {
        self.query.sort_by = Some(SortField::Timestamp);
        self
    }

    /// 按事件类型排序
    pub fn sort_by_event_type(mut self) -> Self {
        self.query.sort_by = Some(SortField::EventType);
        self
    }

    /// 按成功状态排序
    pub fn sort_by_success(mut self) -> Self {
        self.query.sort_by = Some(SortField::Success);
        self
    }

    /// 升序
    pub fn ascending(mut self) -> Self {
        self.query.sort_order = SortOrder::Asc;
        self
    }

    /// 降序
    pub fn descending(mut self) -> Self {
        self.query.sort_order = SortOrder::Desc;
        self
    }

    /// 设置分页偏移
    pub fn offset(mut self, offset: usize) -> Self {
        self.query.offset = Some(offset);
        self
    }

    /// 设置分页限制
    pub fn limit(mut self, limit: usize) -> Self {
        self.query.limit = Some(limit);
        self
    }

    /// 构建查询
    pub fn build(self) -> AuditQuery {
        self.query
    }
}

/// 从 `AuditFilter` 快速构建查询
impl From<AuditFilter> for AuditQuery {
    fn from(filter: AuditFilter) -> Self {
        AuditQuery {
            filter,
            ..Default::default()
        }
    }
}

impl AuditLogManager {
    /// 高级查询审计事件
    ///
    /// 支持排序和分页
    pub fn query_advanced(&self, query: AuditQuery) -> AuthResult<Vec<AuditEvent>> {
        let mut events = self.logger().query(query.filter.clone())?;

        // 排序
        if let Some(sort_field) = query.sort_by {
            match sort_field {
                SortField::Timestamp => {
                    events.sort_by(|a, b| {
                        if query.sort_order == SortOrder::Asc {
                            a.timestamp.cmp(&b.timestamp)
                        } else {
                            b.timestamp.cmp(&a.timestamp)
                        }
                    });
                }
                SortField::EventType => {
                    events.sort_by(|a, b| {
                        let ord = format!("{:?}", a.event_type).cmp(&format!("{:?}", b.event_type));
                        if query.sort_order == SortOrder::Asc {
                            ord
                        } else {
                            ord.reverse()
                        }
                    });
                }
                SortField::PrincipalId => {
                    events.sort_by(|a, b| {
                        let ord = a.principal_id.cmp(&b.principal_id);
                        if query.sort_order == SortOrder::Asc {
                            ord
                        } else {
                            ord.reverse()
                        }
                    });
                }
                SortField::Success => {
                    events.sort_by(|a, b| {
                        let ord = a.success.cmp(&b.success);
                        if query.sort_order == SortOrder::Asc {
                            ord
                        } else {
                            ord.reverse()
                        }
                    });
                }
            }
        }

        // 分页
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(events.len());

        if offset >= events.len() {
            return Ok(Vec::new());
        }

        let end = (offset + limit).min(events.len());
        Ok(events[offset..end].to_vec())
    }

    /// 生成审计统计报告
    ///
    /// 根据查询条件统计事件数量和成功率
    pub fn stats(&self, filter: AuditFilter) -> AuthResult<AuditStats> {
        let events = self.logger().query(filter.clone())?;
        let mut stats = AuditStats::new();

        stats.total_events = events.len() as u64;

        for event in &events {
            if event.success {
                stats.success_count += 1;
            } else {
                stats.failure_count += 1;
            }

            // 按事件类型统计
            let type_name = format!("{:?}", event.event_type);
            *stats.events_by_type.entry(type_name).or_insert(0) += 1;

            // 按日期统计
            let date_key = event.timestamp.format("%Y-%m-%d").to_string();
            *stats.events_by_date.entry(date_key).or_insert(0) += 1;
        }

        // 计算成功率
        if stats.total_events > 0 {
            stats.success_rate = stats.success_count as f64 / stats.total_events as f64;
        }

        // 计算时间范围
        if !events.is_empty() {
            let min_time = events.iter().map(|e| e.timestamp).min().unwrap();
            let max_time = events.iter().map(|e| e.timestamp).max().unwrap();
            stats.time_range = Some((min_time, max_time));
        }

        Ok(stats)
    }

    /// 导出审计日志
    ///
    /// 支持 JSON 和 CSV 格式导出
    pub fn export(&self, format: ExportFormat, filter: AuditFilter) -> AuthResult<Vec<u8>> {
        let events = self.logger().query(filter)?;

        match format {
            ExportFormat::Json => {
                let json = serde_json::to_string_pretty(&events)
                    .map_err(|e| crate::AuthError::IoError(format!("JSON serialization error: {}", e)))?;
                Ok(json.into_bytes())
            }
            ExportFormat::Csv => {
                let mut csv = String::new();
                // CSV 头部
                csv.push_str("id,timestamp,event_type,principal_id,resource_id,success,details,ip_address\n");

                for event in events {
                    csv.push_str(&format!(
                        "{},{:?},{:?},{:?},{:?},{},\"{}\",{}\n",
                        event.id,
                        event.timestamp,
                        event.event_type,
                        event.principal_id,
                        event.resource_id,
                        event.success,
                        event.details.replace('"', "\\\""),
                        event.ip_address.unwrap_or_default()
                    ));
                }

                Ok(csv.into_bytes())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuditEvent, AuditEventType};

    fn create_test_manager() -> AuditLogManager {
        AuditLogManager::from_memory()
    }

    fn create_test_event(event_type: AuditEventType, success: bool) -> AuditEvent {
        AuditEvent::new(event_type, "Test event".to_string()).with_success(success)
    }

    #[test]
    fn test_audit_query_builder() {
        let filter = AuditFilter::new();
        let query = AuditQueryBuilder::new()
            .with_filter(filter)
            .sort_by_time()
            .descending()
            .offset(10)
            .limit(20)
            .build();

        assert!(query.sort_by == Some(SortField::Timestamp));
        assert!(query.sort_order == SortOrder::Desc);
        assert_eq!(query.offset, Some(10));
        assert_eq!(query.limit, Some(20));
    }

    #[test]
    fn test_audit_stats_empty() {
        let manager = create_test_manager();
        let stats = manager.stats(AuditFilter::new()).unwrap();

        assert_eq!(stats.total_events, 0);
        assert_eq!(stats.success_count, 0);
        assert_eq!(stats.failure_count, 0);
        assert_eq!(stats.success_rate, 0.0);
        assert!(stats.time_range.is_none());
    }

    #[test]
    fn test_audit_stats_basic() {
        let manager = create_test_manager();

        // 添加测试事件
        let event1 = create_test_event(AuditEventType::AccessGranted, true);
        let event2 = create_test_event(AuditEventType::AccessDenied, false);
        let event3 = create_test_event(AuditEventType::CapabilityGranted, true);

        manager.logger().log(event1).unwrap();
        manager.logger().log(event2).unwrap();
        manager.logger().log(event3).unwrap();

        let stats = manager.stats(AuditFilter::new()).unwrap();

        assert_eq!(stats.total_events, 3);
        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.failure_count, 1);
        assert!((stats.success_rate - 2.0 / 3.0).abs() < 0.001);

        // 检查按类型统计
        assert_eq!(stats.events_by_type.get("AccessGranted"), Some(&1));
        assert_eq!(stats.events_by_type.get("AccessDenied"), Some(&1));
        assert_eq!(stats.events_by_type.get("CapabilityGranted"), Some(&1));

        // 检查时间范围
        assert!(stats.time_range.is_some());
    }

    #[test]
    fn test_audit_query_advanced_sorting() {
        let manager = create_test_manager();

        // 添加多个事件
        let event1 = AuditEvent::new(AuditEventType::AccessGranted, "First".to_string()).with_success(true);
        let event2 = AuditEvent::new(AuditEventType::AccessDenied, "Second".to_string()).with_success(false);
        let event3 = AuditEvent::new(AuditEventType::PolicyChanged, "Third".to_string()).with_success(true);

        manager.logger().log(event1.clone()).unwrap();
        manager.logger().log(event2.clone()).unwrap();
        manager.logger().log(event3.clone()).unwrap();

        // 测试按成功状态升序
        let query = AuditQueryBuilder::new()
            .sort_by_success()
            .ascending()
            .build();

        let results = manager.query_advanced(query).unwrap();
        assert_eq!(results.len(), 3);
        assert!(!results[0].success); // false first
        assert!(results[1].success);
        assert!(results[2].success);
    }

    #[test]
    fn test_audit_query_advanced_pagination() {
        let manager = create_test_manager();

        // 添加 5 个事件
        for i in 0..5 {
            let event = AuditEvent::new(AuditEventType::AccessGranted, format!("Event {}", i));
            manager.logger().log(event).unwrap();
        }

        // 测试分页
        let query = AuditQueryBuilder::new()
            .offset(1)
            .limit(2)
            .build();

        let results = manager.query_advanced(query).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].details, "Event 1");
        assert_eq!(results[1].details, "Event 2");
    }

    #[test]
    fn test_audit_query_advanced_offset_beyond() {
        let manager = create_test_manager();

        let event = AuditEvent::new(AuditEventType::AccessGranted, "Only".to_string());
        manager.logger().log(event).unwrap();

        // 偏移超出范围
        let query = AuditQueryBuilder::new()
            .offset(10)
            .build();

        let results = manager.query_advanced(query).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_export_json() {
        let manager = create_test_manager();
        let event = create_test_event(AuditEventType::AccessGranted, true);
        manager.logger().log(event.clone()).unwrap();

        let data = manager.export(ExportFormat::Json, AuditFilter::new()).unwrap();
        let json_str = String::from_utf8(data).unwrap();

        assert!(json_str.contains("AccessGranted"));
        assert!(json_str.contains("Test event"));
    }

    #[test]
    fn test_export_csv() {
        let manager = create_test_manager();
        let event = create_test_event(AuditEventType::AccessGranted, true);
        manager.logger().log(event.clone()).unwrap();

        let data = manager.export(ExportFormat::Csv, AuditFilter::new()).unwrap();
        let csv_str = String::from_utf8(data).unwrap();

        assert!(csv_str.contains("id,timestamp,event_type"));
        assert!(csv_str.contains("AccessGranted"));
    }

    #[test]
    fn test_from_audit_filter() {
        let filter = AuditFilter::new().with_success_only(true);
        let query: AuditQuery = filter.into();

        assert_eq!(query.filter.success_only, Some(true));
        assert!(query.sort_by.is_none());
        assert_eq!(query.offset, None);
    }

    #[test]
    fn test_audit_stats_filtering() {
        let manager = create_test_manager();

        let event1 = create_test_event(AuditEventType::AccessGranted, true);
        let event2 = create_test_event(AuditEventType::AccessDenied, false);

        manager.logger().log(event1).unwrap();
        manager.logger().log(event2).unwrap();

        // 只统计成功事件
        let filter = AuditFilter::new().with_success_only(true);
        let stats = manager.stats(filter).unwrap();

        assert_eq!(stats.total_events, 1);
        assert_eq!(stats.success_count, 1);
        assert_eq!(stats.failure_count, 0);
        assert_eq!(stats.success_rate, 1.0);
    }

    #[test]
    fn test_audit_stats_events_by_date() {
        let manager = create_test_manager();

        let event = AuditEvent::new(AuditEventType::AccessGranted, "Today".to_string());
        manager.logger().log(event).unwrap();

        let stats = manager.stats(AuditFilter::new()).unwrap();

        // 检查日期统计包含今天的日期
        let today = Utc::now().format("%Y-%m-%d").to_string();
        assert!(stats.events_by_date.contains_key(&today));
        assert_eq!(stats.events_by_date.get(&today), Some(&1));
    }
}
