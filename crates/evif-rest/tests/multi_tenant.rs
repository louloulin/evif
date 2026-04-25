// Phase 17.1: Multi-tenant Support Integration Tests
//
// 测试多租户隔离和租户管理功能

use evif_core::{EvifPlugin, RadixMountTable};
use evif_plugins::MemFsPlugin;
use evif_rest::{
    create_routes, create_routes_with_tenant_state, TenantInfo, TenantState, TenantStatus,
};
use std::sync::Arc;

/// P17.1-01: List Tenants Endpoint
#[tokio::test]
async fn tenant_list_endpoint() {
    let mount_table = Arc::new(RadixMountTable::new());
    let app = create_routes(mount_table);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    // 等待服务器就绪
    for _ in 0..60 {
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    let res = client
        .get(format!("{}/api/v1/tenants", base))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "Tenant list should succeed");

    let json: serde_json::Value = res.json().await.expect("valid JSON");
    // Default tenant should be pre-created
    assert!(json.is_array(), "Response should be an array");
}

/// P17.1-02: Create Tenant
#[tokio::test]
async fn tenant_create() {
    let mount_table = Arc::new(RadixMountTable::new());
    let app = create_routes(mount_table);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    for _ in 0..60 {
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    let res = client
        .post(format!("{}/api/v1/tenants", base))
        .json(&serde_json::json!({
            "name": "test-tenant",
            "storage_quota": 1073741824u64
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "Tenant create should succeed");

    let json: serde_json::Value = res.json().await.expect("valid JSON");
    assert_eq!(json["name"], "test-tenant");
    assert_eq!(json["status"], "active");
    assert!(json["id"].is_string(), "Should have UUID id");
    assert_eq!(json["storage_quota"], 1073741824u64);
}

/// P17.1-03: Get Current Tenant
#[tokio::test]
async fn tenant_get_current() {
    let mount_table = Arc::new(RadixMountTable::new());
    let app = create_routes(mount_table);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    for _ in 0..60 {
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    // GET /api/v1/tenants/me - without X-Tenant-ID should return default
    let res = client
        .get(format!("{}/api/v1/tenants/me", base))
        .send()
        .await
        .expect("request succeeds");

    assert!(
        res.status().is_success(),
        "Get current tenant should succeed"
    );

    let json: serde_json::Value = res.json().await.expect("valid JSON");
    assert_eq!(json["id"], "default");
    assert_eq!(json["status"], "active");
}

/// P17.1-04: Get Tenant By ID
#[tokio::test]
async fn tenant_get_by_id() {
    let mount_table = Arc::new(RadixMountTable::new());
    let app = create_routes(mount_table);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    for _ in 0..60 {
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    let create_res = client
        .post(format!("{}/api/v1/tenants", base))
        .json(&serde_json::json!({
            "name": "lookup-tenant",
            "storage_quota": 2048u64
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(
        create_res.status().is_success(),
        "Tenant create should succeed before lookup"
    );

    let created: serde_json::Value = create_res.json().await.expect("valid JSON");
    let tenant_id = created["id"].as_str().expect("tenant id should exist");

    let get_res = client
        .get(format!("{}/api/v1/tenants/{}", base, tenant_id))
        .send()
        .await
        .expect("request succeeds");

    assert!(
        get_res.status().is_success(),
        "Get tenant by id should succeed"
    );

    let json: serde_json::Value = get_res.json().await.expect("valid JSON");
    assert_eq!(json["id"], tenant_id);
    assert_eq!(json["name"], "lookup-tenant");
    assert_eq!(json["storage_quota"], 2048u64);
    assert_eq!(json["status"], "active");
}

/// P17.1-05: Create Tenant With Empty Name Error
#[tokio::test]
async fn tenant_create_empty_name_error() {
    let mount_table = Arc::new(RadixMountTable::new());
    let app = create_routes(mount_table);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    for _ in 0..60 {
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    let res = client
        .post(format!("{}/api/v1/tenants", base))
        .json(&serde_json::json!({
            "name": ""
        }))
        .send()
        .await
        .expect("request succeeds");

    // Should return 400 error for empty name
    assert!(
        res.status().is_client_error(),
        "Empty name should return 4xx error"
    );
}

/// P17.1-06: Delete Non-default Tenant
#[tokio::test]
async fn tenant_delete_non_default() {
    let mount_table = Arc::new(RadixMountTable::new());
    let app = create_routes(mount_table);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    for _ in 0..60 {
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    // Create a tenant first
    let create_res = client
        .post(format!("{}/api/v1/tenants", base))
        .json(&serde_json::json!({
            "name": "temp-tenant"
        }))
        .send()
        .await
        .expect("request succeeds");

    let created: serde_json::Value = create_res.json().await.expect("valid JSON");
    let tenant_id = created["id"].as_str().unwrap();

    // Delete the tenant
    let delete_res = client
        .delete(format!("{}/api/v1/tenants/{}", base, tenant_id))
        .send()
        .await
        .expect("request succeeds");

    assert!(
        delete_res.status().is_success(),
        "Delete non-default tenant should succeed"
    );
}

/// P17.1-07: Cannot Delete Default Tenant
#[tokio::test]
async fn tenant_cannot_delete_default() {
    let mount_table = Arc::new(RadixMountTable::new());
    let app = create_routes(mount_table);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    for _ in 0..60 {
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    let res = client
        .delete(format!("{}/api/v1/tenants/default", base))
        .send()
        .await
        .expect("request succeeds");

    // Should return error for default tenant
    assert!(
        res.status().is_client_error(),
        "Cannot delete default tenant"
    );
}

/// P17.1-08: Tenant Persistence Survives Restart
#[tokio::test]
async fn tenant_persistence_survives_restart() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let state_path = temp_dir.path().join("tenant-state.json");

    let first_mount_table = Arc::new(RadixMountTable::new());
    let first_tenant_state = TenantState::persistent(&state_path).expect("persistent tenant state");
    let first_app = create_routes_with_tenant_state(first_mount_table, first_tenant_state);
    let first_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let first_port = first_listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(first_listener, first_app.into_make_service())
            .await
            .expect("serve");
    });

    let first_base = format!("http://127.0.0.1:{}", first_port);
    let client = reqwest::Client::new();

    for _ in 0..60 {
        if let Ok(res) = client
            .get(format!("{}/api/v1/health", first_base))
            .send()
            .await
        {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    let created = client
        .post(format!("{}/api/v1/tenants", first_base))
        .json(&serde_json::json!({
            "name": "persisted-tenant",
            "storage_quota": 4096u64
        }))
        .send()
        .await
        .expect("create request succeeds");
    assert!(
        created.status().is_success(),
        "tenant create should succeed"
    );
    let created_json: serde_json::Value = created.json().await.expect("valid JSON");
    let tenant_id = created_json["id"].as_str().expect("tenant id").to_string();

    let second_mount_table = Arc::new(RadixMountTable::new());
    let second_tenant_state =
        TenantState::persistent(&state_path).expect("persistent tenant state reload");
    let second_app = create_routes_with_tenant_state(second_mount_table, second_tenant_state);
    let second_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let second_port = second_listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(second_listener, second_app.into_make_service())
            .await
            .expect("serve");
    });

    let second_base = format!("http://127.0.0.1:{}", second_port);

    for _ in 0..60 {
        if let Ok(res) = client
            .get(format!("{}/api/v1/health", second_base))
            .send()
            .await
        {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    let restored = client
        .get(format!("{}/api/v1/tenants/{}", second_base, tenant_id))
        .send()
        .await
        .expect("restore request succeeds");
    assert!(
        restored.status().is_success(),
        "tenant should survive restart when persistence is enabled"
    );

    let restored_json: serde_json::Value = restored.json().await.expect("valid JSON");
    assert_eq!(restored_json["name"], "persisted-tenant");
    assert_eq!(restored_json["storage_quota"], 4096u64);
}

/// P17.1-08: Tenant Storage Quota PATCH Endpoint
#[tokio::test]
async fn tenant_quota_patch_endpoint() {
    let mount_table = Arc::new(RadixMountTable::new());
    let app = evif_rest::create_routes(mount_table);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    for _ in 0..60 {
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    // Update the default tenant's storage quota
    let quota = client
        .patch(format!("{}/api/v1/tenants/default/quota", base))
        .json(&serde_json::json!({ "storage_quota": 1073741824u64 })) // 1 GB
        .send()
        .await
        .expect("quota patch succeeds");

    assert!(quota.status().is_success(), "quota PATCH should succeed");
    let quota_json: serde_json::Value = quota.json().await.expect("valid JSON");
    assert_eq!(quota_json["id"], "default");
    assert_eq!(quota_json["storage_quota"], 1073741824u64);

    // Verify the quota was actually updated via GET
    let get = client
        .get(format!("{}/api/v1/tenants/default", base))
        .send()
        .await
        .expect("get tenant succeeds");
    let get_json: serde_json::Value = get.json().await.expect("valid JSON");
    assert_eq!(get_json["storage_quota"], 1073741824u64);
}

/// P17.1-09: Tenant Quota Patch NonExistent Tenant Returns 404
#[tokio::test]
async fn tenant_quota_patch_nonexistent_tenant() {
    let mount_table = Arc::new(RadixMountTable::new());
    let app = evif_rest::create_routes(mount_table);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    for _ in 0..60 {
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    let quota = client
        .patch(format!("{}/api/v1/tenants/nonexistent-id-123/quota", base))
        .json(&serde_json::json!({ "storage_quota": 1024u64 }))
        .send()
        .await
        .expect("quota patch succeeds");
    // Should return 404 Not Found
    assert!(
        quota.status().as_u16() == 404 || quota.status().is_client_error(),
        "quota patch for nonexistent tenant should return 404, got {}",
        quota.status()
    );
}

/// P17.1-10: Write fails when storage quota is exceeded
#[tokio::test]
async fn tenant_write_rejected_when_quota_exceeded() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let state_path = temp_dir.path().join("quota-enforcement-state.json");

    let tenant_state = TenantState::persistent(&state_path).expect("persistent tenant state");
    // Set default tenant quota to 20 bytes
    tenant_state.update_storage_quota_sync("default", 20);

    let mount_table = Arc::new(RadixMountTable::new());
    let mem_plugin: Arc<dyn EvifPlugin> = Arc::new(MemFsPlugin::new());
    mount_table
        .mount_with_metadata(
            "/data".to_string(),
            mem_plugin,
            "mem".to_string(),
            "mem".to_string(),
        )
        .await
        .expect("mount mem plugin");

    let app = evif_rest::create_routes_with_tenant_state(mount_table, tenant_state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    for _ in 0..60 {
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    // Create file first, then write content
    let _ = client
        .post(format!("{}/api/v1/files", base))
        .query(&[("path", "/data/file1.txt")])
        .json(&serde_json::json!({ "path": "/data/file1.txt" }))
        .send()
        .await;

    // First write of 10 bytes — within quota, should succeed
    let r1 = client
        .put(format!("{}/api/v1/files", base))
        .query(&[("path", "/data/file1.txt")])
        .json(&serde_json::json!({ "data": "1234567890" })) // 10 bytes
        .send()
        .await
        .expect("first write succeeds");
    assert!(
        r1.status().is_success(),
        "first write within quota should succeed"
    );

    // Create file2 first
    let _ = client
        .post(format!("{}/api/v1/files", base))
        .query(&[("path", "/data/file2.txt")])
        .json(&serde_json::json!({ "path": "/data/file2.txt" }))
        .send()
        .await;

    // Second write of 15 bytes — would exceed 20-byte quota (10 + 15 = 25 > 20)
    let r2 = client
        .put(format!("{}/api/v1/files", base))
        .query(&[("path", "/data/file2.txt")])
        .json(&serde_json::json!({ "data": "123456789012345" })) // 15 bytes
        .send()
        .await
        .expect("second write request completes");
    assert!(
        r2.status().is_client_error() || r2.status().as_u16() == 400,
        "write exceeding quota should return 400, got {}",
        r2.status()
    );

    let err_body: serde_json::Value = r2.json().await.expect("valid error JSON");
    assert!(
        err_body["message"]
            .as_str()
            .map(|s| s.contains("quota") || s.contains("Quota"))
            .unwrap_or(false),
        "error message should mention quota, got: {}",
        err_body["message"]
    );
}

/// P17.1-11: Storage usage is tracked after successful writes
#[tokio::test]
async fn tenant_storage_used_tracked_after_writes() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let state_path = temp_dir.path().join("storage-tracking-state.json");

    let tenant_state = TenantState::persistent(&state_path).expect("persistent tenant state");
    let mount_table = Arc::new(RadixMountTable::new());
    let mem_plugin: Arc<dyn EvifPlugin> = Arc::new(MemFsPlugin::new());
    mount_table
        .mount_with_metadata(
            "/data".to_string(),
            mem_plugin,
            "mem".to_string(),
            "mem".to_string(),
        )
        .await
        .expect("mount mem plugin");
    let app = evif_rest::create_routes_with_tenant_state(mount_table, tenant_state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    for _ in 0..60 {
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    // Create file first
    let _ = client
        .post(format!("{}/api/v1/files", base))
        .query(&[("path", "/data/track.txt")])
        .json(&serde_json::json!({ "path": "/data/track.txt" }))
        .send()
        .await;

    // Write 5 bytes
    let _ = client
        .put(format!("{}/api/v1/files", base))
        .query(&[("path", "/data/track.txt")])
        .json(&serde_json::json!({ "data": "ABCDE" })) // 5 bytes
        .send()
        .await
        .expect("write succeeds");

    // Check storage_used via tenant endpoint
    let tenant_res = client
        .get(format!("{}/api/v1/tenants/default", base))
        .send()
        .await
        .expect("tenant get succeeds");
    let tenant_json: serde_json::Value = tenant_res.json().await.expect("valid JSON");
    assert!(
        tenant_json["storage_used"].as_u64().unwrap_or(0) >= 5,
        "storage_used should track at least 5 bytes, got {}",
        tenant_json["storage_used"]
    );
}

/// P17.1-13: REST write respects X-Tenant-ID header for quota isolation
#[tokio::test]
async fn tenant_write_respects_x_tenant_id_header() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let state_path = temp_dir.path().join("x-tenant-isolation-state.json");

    let tenant_state = TenantState::persistent(&state_path).expect("persistent tenant state");
    // Insert test-tenant with 10-byte quota directly (avoids random UUID id issue)
    tenant_state.insert_tenant(
        "test-tenant",
        TenantInfo {
            id: "test-tenant".to_string(),
            name: "test-tenant".to_string(),
            storage_quota: 10,
            storage_used: 0,
            status: TenantStatus::Active,
            created_at: chrono::Utc::now().to_rfc3339(),
        },
    );

    let mount_table = Arc::new(RadixMountTable::new());
    let mem_plugin: Arc<dyn EvifPlugin> = Arc::new(MemFsPlugin::new());
    mount_table
        .mount_with_metadata(
            "/data".to_string(),
            mem_plugin,
            "mem".to_string(),
            "mem".to_string(),
        )
        .await
        .expect("mount mem plugin");

    let app = create_routes_with_tenant_state(mount_table, tenant_state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    for _ in 0..60 {
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    // Create files first (required for MemFsPlugin write)
    let _ = client
        .post(format!("{}/api/v1/files", base))
        .header("x-tenant-id", "test-tenant")
        .json(&serde_json::json!({ "path": "/data/f1.txt" }))
        .send()
        .await;
    let _ = client
        .post(format!("{}/api/v1/files", base))
        .header("x-tenant-id", "test-tenant")
        .json(&serde_json::json!({ "path": "/data/f2.txt" }))
        .send()
        .await;
    let _ = client
        .post(format!("{}/api/v1/files", base))
        .header("x-tenant-id", "default")
        .json(&serde_json::json!({ "path": "/data/f3.txt" }))
        .send()
        .await;

    // Write 5 bytes to test-tenant (should succeed: 5 <= 10 quota)
    let write1 = client
        .put(format!("{}/api/v1/files", base))
        .header("x-tenant-id", "test-tenant")
        .query(&[("path", "/data/f1.txt")])
        .json(&serde_json::json!({ "data": "HELLO" })) // 5 bytes
        .send()
        .await
        .expect("write succeeds");
    assert!(
        write1.status().is_success(),
        "write within quota should succeed: {}",
        write1.text().await.unwrap_or_default()
    );

    // Write 10 more bytes to test-tenant (should fail: 15 > 10 quota)
    let write2 = client
        .put(format!("{}/api/v1/files", base))
        .header("x-tenant-id", "test-tenant")
        .query(&[("path", "/data/f2.txt")])
        .json(&serde_json::json!({ "data": "0123456789" })) // 10 bytes
        .send()
        .await
        .expect("write request succeeds");
    assert!(
        write2.status().is_client_error(),
        "write exceeding quota should fail: status={}",
        write2.status()
    );

    // Write 5 bytes to default tenant (should succeed: default has unlimited quota)
    let write3 = client
        .put(format!("{}/api/v1/files", base))
        .header("x-tenant-id", "default")
        .query(&[("path", "/data/f3.txt")])
        .json(&serde_json::json!({ "data": "WORLD" })) // 5 bytes
        .send()
        .await
        .expect("write succeeds");
    assert!(
        write3.status().is_success(),
        "default tenant has unlimited quota: {}",
        write3.text().await.unwrap_or_default()
    );
}
