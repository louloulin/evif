// Phase 17.3: Incremental Sync Protocol Integration Tests
//
// 测试增量同步协议功能

use evif_rest::{create_routes, create_routes_with_sync_state, SyncState};
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
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
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
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
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
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
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
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
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
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
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

/// P17.3-06: Sync Persistence Survives Restart
#[tokio::test]
async fn sync_persistence_survives_restart() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let state_path = temp_dir.path().join("sync-state.json");

    let first_mount_table = Arc::new(RadixMountTable::new());
    let first_sync_state = SyncState::persistent(&state_path).expect("persistent sync state");
    let first_app = create_routes_with_sync_state(first_mount_table, first_sync_state);
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

    let applied = client
        .post(format!("{}/api/v1/sync/delta", first_base))
        .json(&serde_json::json!({
            "base_version": 0,
            "changes": [
                {
                    "path": "/context/L0/current",
                    "op": "modified",
                    "version": 3,
                    "timestamp": "2026-04-03T09:00:00Z"
                },
                {
                    "path": "/context/L1/decisions.md",
                    "op": "created",
                    "version": 4,
                    "timestamp": "2026-04-03T09:01:00Z"
                }
            ]
        }))
        .send()
        .await
        .expect("delta request succeeds");
    assert!(applied.status().is_success(), "delta apply should succeed");

    let second_mount_table = Arc::new(RadixMountTable::new());
    let second_sync_state =
        SyncState::persistent(&state_path).expect("persistent sync state reload");
    let second_app = create_routes_with_sync_state(second_mount_table, second_sync_state);
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

    let version = client
        .get(format!("{}/api/v1/sync/version", second_base))
        .send()
        .await
        .expect("version request succeeds");
    assert!(version.status().is_success(), "version lookup should succeed");
    let version_json: serde_json::Value = version.json().await.expect("valid JSON");
    assert_eq!(version_json["version"], 4);

    let path_version = client
        .get(format!(
            "{}/api/v1/sync/%2Fcontext%2FL1%2Fdecisions.md/version",
            second_base
        ))
        .send()
        .await
        .expect("path version request succeeds");
    assert!(
        path_version.status().is_success(),
        "path version lookup should succeed"
    );
    let path_version_json: serde_json::Value =
        path_version.json().await.expect("valid JSON");
    assert_eq!(path_version_json["version"], 4);

    let status = client
        .get(format!("{}/api/v1/sync/status", second_base))
        .send()
        .await
        .expect("status request succeeds");
    assert!(status.status().is_success(), "status lookup should succeed");
    let status_json: serde_json::Value = status.json().await.expect("valid JSON");
    assert_eq!(status_json["last_version"], 4);
    assert_eq!(status_json["pending_changes"], 2);
    assert!(
        status_json["tracked_paths"]
            .as_array()
            .expect("tracked paths array")
            .iter()
            .any(|path| path == "/context/L1/decisions.md")
    );
}
