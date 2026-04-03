// Phase 17.2: Encryption-at-rest Integration Tests
//
// 测试加密存储功能

use evif_rest::{create_routes, create_routes_with_encryption_state, EncryptionState};
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

/// P17.2-05: Encryption Persistence Survives Restart With Env Key
#[tokio::test]
async fn encryption_persistence_survives_restart_with_env_key() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let state_path = temp_dir.path().join("encryption-state.json");
    let env_name = format!(
        "EVIF_PERSIST_ENCRYPTION_KEY_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("unix time")
            .as_nanos()
    );
    std::env::set_var(&env_name, "persisted-secret-key");

    let first_mount_table = Arc::new(RadixMountTable::new());
    let first_encryption_state =
        EncryptionState::persistent(&state_path).expect("persistent encryption state");
    let first_app =
        create_routes_with_encryption_state(first_mount_table, first_encryption_state);
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

    let enabled = client
        .post(format!("{}/api/v1/encryption/enable", first_base))
        .json(&serde_json::json!({
            "key": format!("env:{}", env_name)
        }))
        .send()
        .await
        .expect("enable request succeeds");
    assert!(enabled.status().is_success(), "enable should succeed");

    let second_mount_table = Arc::new(RadixMountTable::new());
    let second_encryption_state =
        EncryptionState::persistent(&state_path).expect("persistent encryption reload");
    let second_app =
        create_routes_with_encryption_state(second_mount_table, second_encryption_state);
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

    let status = client
        .get(format!("{}/api/v1/encryption/status", second_base))
        .send()
        .await
        .expect("status request succeeds");
    assert!(status.status().is_success(), "status should succeed");

    let status_json: serde_json::Value = status.json().await.expect("valid JSON");
    assert_eq!(status_json["status"], "enabled");
    assert_eq!(status_json["algorithm"], "AES-256-GCM");
    assert!(
        status_json["key_source"]
            .as_str()
            .expect("key source string")
            .contains(&env_name)
    );
}
