// Phase 17.1: Multi-tenant Support Integration Tests
//
// 测试多租户隔离和租户管理功能

use evif_rest::create_routes;
use evif_core::RadixMountTable;
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
        if let Ok(res) = client.get(&format!("{}/api/v1/health", base)).send().await {
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
        if let Ok(res) = client.get(&format!("{}/api/v1/health", base)).send().await {
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
        if let Ok(res) = client.get(&format!("{}/api/v1/health", base)).send().await {
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

    assert!(res.status().is_success(), "Get current tenant should succeed");

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
        if let Ok(res) = client.get(&format!("{}/api/v1/health", base)).send().await {
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

    assert!(get_res.status().is_success(), "Get tenant by id should succeed");

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
        if let Ok(res) = client.get(&format!("{}/api/v1/health", base)).send().await {
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
        if let Ok(res) = client.get(&format!("{}/api/v1/health", base)).send().await {
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

    assert!(delete_res.status().is_success(), "Delete non-default tenant should succeed");
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
        if let Ok(res) = client.get(&format!("{}/api/v1/health", base)).send().await {
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
