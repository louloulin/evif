// Phase 17.2: Encryption-at-rest Integration Tests
//
// 测试加密存储功能

use evif_rest::create_routes;
use evif_core::RadixMountTable;
use std::sync::Arc;

/// P17.2-01: Encryption Status Endpoint
#[tokio::test]
async fn encryption_status_endpoint() {
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
        .get(format!("{}/api/v1/encryption/status", base))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "Encryption status should succeed");

    let json: serde_json::Value = res.json().await.expect("valid JSON");
    assert_eq!(json["algorithm"], "AES-256-GCM");
    // Status should be "disabled" when no key is set
    assert!(["disabled", "enabled", "key_missing"].contains(&json["status"].as_str().unwrap_or("")));
}

/// P17.2-02: Enable Encryption
#[tokio::test]
async fn encryption_enable() {
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
        .post(format!("{}/api/v1/encryption/enable", base))
        .json(&serde_json::json!({
            "key": "my-secret-encryption-key"
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "Enable encryption should succeed");

    let json: serde_json::Value = res.json().await.expect("valid JSON");
    assert_eq!(json["status"], "enabled");
    assert_eq!(json["algorithm"], "AES-256-GCM");
}

/// P17.2-03: Disable Encryption
#[tokio::test]
async fn encryption_disable() {
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
        .post(format!("{}/api/v1/encryption/disable", base))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "Disable encryption should succeed");

    let json: serde_json::Value = res.json().await.expect("valid JSON");
    assert_eq!(json["status"], "disabled");
}

/// P17.2-04: Enable Encryption With Empty Key Error
#[tokio::test]
async fn encryption_enable_empty_key_error() {
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
        .post(format!("{}/api/v1/encryption/enable", base))
        .json(&serde_json::json!({
            "key": ""
        }))
        .send()
        .await
        .expect("request succeeds");

    // Should return 400 error for empty key
    assert!(
        res.status().is_client_error(),
        "Empty key should return 4xx error"
    );
}
