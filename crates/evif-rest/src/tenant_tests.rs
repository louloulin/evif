// Phase 17.1: Multi-tenant Support - Unit Tests
//
// 测试 TenantState 的核心功能（无需网络）

#[cfg(test)]
mod tests {
    use crate::{
        CreateTenantRequest, TenantInfo, TenantState, TenantStatus,
    };
    use std::sync::Arc;

    #[test]
    fn test_tenant_state_new_has_default_tenant() {
        let state = TenantState::new();
        let tenants = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(state.list_tenants());

        assert_eq!(tenants.len(), 1);
        assert_eq!(tenants[0].id, "default");
        assert_eq!(tenants[0].name, "Default Tenant");
        assert_eq!(tenants[0].storage_quota, 0); // unlimited
        assert_eq!(tenants[0].storage_used, 0);
        assert_eq!(tenants[0].status, TenantStatus::Active);
    }

    #[test]
    fn test_create_tenant() {
        let state = TenantState::new();
        let state = Arc::new(state);

        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(state.create_tenant(CreateTenantRequest {
                name: "test-tenant".to_string(),
                storage_quota: Some(1024 * 1024),
            }));

        assert!(result.is_ok());
        let tenant = result.unwrap();
        assert_eq!(tenant.name, "test-tenant");
        assert_eq!(tenant.storage_quota, 1024 * 1024);
        assert_eq!(tenant.storage_used, 0);
        assert_eq!(tenant.status, TenantStatus::Active);
        assert!(!tenant.id.is_empty());
    }

    #[test]
    fn test_get_tenant() {
        let state = TenantState::new();
        let state = Arc::new(state);

        // Get default tenant
        let default = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(state.get_tenant("default"));
        assert!(default.is_some());
        assert_eq!(default.unwrap().id, "default");

        // Get non-existent tenant
        let none = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(state.get_tenant("nonexistent"));
        assert!(none.is_none());
    }

    #[test]
    fn test_delete_tenant() {
        let state = TenantState::new();
        let state = Arc::new(state);

        // Create a tenant first
        let created = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(state.create_tenant(CreateTenantRequest {
                name: "temp".to_string(),
                storage_quota: None,
            }));
        let tenant_id = created.unwrap().id;

        // Delete the tenant
        let deleted = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(state.delete_tenant(&tenant_id));
        assert!(deleted.unwrap());

        // Verify deleted
        let gone = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(state.get_tenant(&tenant_id));
        assert!(gone.is_none());
    }

    #[test]
    fn test_cannot_delete_default_tenant() {
        let state = TenantState::new();
        let state = Arc::new(state);

        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(state.delete_tenant("default"));

        // Should return false (cannot delete default)
        assert_eq!(result.unwrap(), false);

        // Default tenant should still exist
        let default = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(state.get_tenant("default"));
        assert!(default.is_some());
    }

    #[test]
    fn test_check_quota_unlimited() {
        let state = TenantState::new();
        let state = Arc::new(state);

        // Default tenant has unlimited quota (0 = unlimited)
        assert!(state.check_quota("default", 1_000_000_000)); // 1GB should pass
    }

    #[test]
    fn test_check_quota_enforcement() {
        let state = TenantState::new();
        let state = Arc::new(state);

        // Insert a tenant with 10-byte quota
        state.insert_tenant(
            "limited",
            TenantInfo {
                id: "limited".to_string(),
                name: "Limited Tenant".to_string(),
                storage_quota: 10,
                storage_used: 0,
                status: TenantStatus::Active,
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        );

        // 5 bytes should pass
        assert!(state.check_quota("limited", 5));

        // 10 bytes should pass (exactly at limit)
        assert!(state.check_quota("limited", 10));

        // 11 bytes should fail
        assert!(!state.check_quota("limited", 11));
    }

    #[test]
    fn test_check_quota_nonexistent_tenant() {
        let state = TenantState::new();

        // Non-existent tenant should be rejected
        assert!(!state.check_quota("nonexistent", 1));
    }

    #[test]
    fn test_record_write() {
        let state = TenantState::new();
        let state = Arc::new(state);

        // Insert a tenant with quota
        state.insert_tenant(
            "track",
            TenantInfo {
                id: "track".to_string(),
                name: "Track Tenant".to_string(),
                storage_quota: 100,
                storage_used: 0,
                status: TenantStatus::Active,
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        );

        // Record a write
        state.record_write("track", 30).unwrap();

        // Verify storage_used updated
        let tenant = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(state.get_tenant("track"));
        assert_eq!(tenant.unwrap().storage_used, 30);

        // Record another write
        state.record_write("track", 20).unwrap();

        let tenant = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(state.get_tenant("track"));
        assert_eq!(tenant.unwrap().storage_used, 50);
    }

    #[test]
    fn test_update_storage_quota() {
        let state = TenantState::new();
        let state = Arc::new(state);

        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(state.update_storage_quota("default", 5000));

        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.id, "default");
        assert_eq!(updated.storage_quota, 5000);
    }

    #[test]
    fn test_update_storage_quota_nonexistent() {
        let state = TenantState::new();
        let state = Arc::new(state);

        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(state.update_storage_quota("nonexistent", 1000));

        assert!(result.is_err());
    }

    #[test]
    fn test_effective_tenant_id() {
        // With header
        let header = Some(&axum::http::HeaderValue::from_static("tenant-123"));
        assert_eq!(TenantState::effective_tenant_id(header), "tenant-123");

        // With empty header
        let empty = Some(&axum::http::HeaderValue::from_static(""));
        assert_eq!(TenantState::effective_tenant_id(empty), "default");

        // Without header
        assert_eq!(TenantState::effective_tenant_id(None), "default");
    }

    #[test]
    fn test_tenant_status_serialization() {
        use serde_json;

        let active = TenantStatus::Active;
        let suspended = TenantStatus::Suspended;
        let deleted = TenantStatus::Deleted;

        assert_eq!(serde_json::to_string(&active).unwrap(), "\"active\"");
        assert_eq!(serde_json::to_string(&suspended).unwrap(), "\"suspended\"");
        assert_eq!(serde_json::to_string(&deleted).unwrap(), "\"deleted\"");
    }

    #[test]
    fn test_storage_used_saturation() {
        let state = TenantState::new();
        let state = Arc::new(state);

        state.insert_tenant(
            "sat",
            TenantInfo {
                id: "sat".to_string(),
                name: "Sat Tenant".to_string(),
                storage_quota: 100,
                storage_used: u64::MAX - 10, // Near max
                status: TenantStatus::Active,
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        );

        // Adding 20 should saturate, not overflow
        state.record_write("sat", 20).unwrap();

        let tenant = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(state.get_tenant("sat"));
        // Should be near max (saturated)
        assert!(tenant.unwrap().storage_used >= u64::MAX - 10);
    }
}
