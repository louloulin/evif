// Phase 17.3: Incremental Sync Protocol Integration Tests
//
// 测试增量同步协议功能

use evif_rest::create_routes;
use evif_core::RadixMountTable;
use std::sync::Arc;

/// P17.3-01: Sync Status Endpoint
#[tokio::test]
async fn sync_status_endpoint() {
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
        .get(format!("{}/api/v1/sync/status", base))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "Sync status should succeed");

    let json: serde_json::Value = res.json().await.expect("valid JSON");
    assert!(json.get("last_version").is_some());
    assert!(json.get("pending_changes").is_some());
    assert!(json.get("tracked_paths").is_some());
}

/// P17.3-02: Get Sync Version
#[tokio::test]
async fn sync_version_endpoint() {
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
        .get(format!("{}/api/v1/sync/version", base))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "Sync version should succeed");

    let json: serde_json::Value = res.json().await.expect("valid JSON");
    assert!(json.get("version").is_some());
    assert_eq!(json["version"], 0);
}

/// P17.3-03: Apply Delta Changes
#[tokio::test]
async fn sync_apply_delta() {
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
        .post(format!("{}/api/v1/sync/delta", base))
        .json(&serde_json::json!({
            "base_version": 0,
            "changes": [
                {
                    "path": "/context/L0/current",
                    "op": "modified",
                    "version": 1,
                    "timestamp": "2026-04-02T10:00:00Z"
                },
                {
                    "path": "/context/L1/decisions.md",
                    "op": "created",
                    "version": 2,
                    "timestamp": "2026-04-02T10:01:00Z"
                }
            ]
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "Delta apply should succeed");

    let json: serde_json::Value = res.json().await.expect("valid JSON");
    assert_eq!(json["accepted"], 2);
    assert_eq!(json["synced_version"], 2);
    assert!(json["conflicts"].as_array().unwrap().is_empty());
}

/// P17.3-04: Get Path Version
#[tokio::test]
async fn sync_path_version() {
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

    // First sync a path
    client
        .post(format!("{}/api/v1/sync/delta", base))
        .json(&serde_json::json!({
            "base_version": 0,
            "changes": [
                {
                    "path": "testfile",
                    "op": "created",
                    "version": 5,
                    "timestamp": "2026-04-02T10:00:00Z"
                }
            ]
        }))
        .send()
        .await
        .expect("request succeeds");

    // Then get the path version
    let res = client
        .get(format!("{}/api/v1/sync/testfile/version", base))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "Path version should succeed");
    let json: serde_json::Value = res.json().await.expect("valid JSON");
    assert_eq!(json["version"], 5);
}

/// P17.3-05: Delta With Empty Changes Error
#[tokio::test]
async fn sync_delta_empty_changes_error() {
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
        .post(format!("{}/api/v1/sync/delta", base))
        .json(&serde_json::json!({
            "base_version": 0,
            "changes": []
        }))
        .send()
        .await
        .expect("request succeeds");

    // Should return 400 error for empty changes
    assert!(
        res.status().is_client_error(),
        "Empty changes should return 4xx error"
    );
}
