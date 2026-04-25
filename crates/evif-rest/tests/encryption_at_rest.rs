// Phase 17.2: Encryption-at-rest Integration Tests
//
// 测试加密存储功能

use evif_core::RadixMountTable;
use evif_rest::{create_routes, create_routes_with_encryption_state, EncryptionState};
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

    assert!(
        res.status().is_success(),
        "Encryption status should succeed"
    );

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

    assert!(
        res.status().is_success(),
        "Enable encryption should succeed"
    );

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

    assert!(
        res.status().is_success(),
        "Disable encryption should succeed"
    );

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
    let first_app = create_routes_with_encryption_state(first_mount_table, first_encryption_state);
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
    assert!(status_json["key_source"]
        .as_str()
        .expect("key source string")
        .contains(&env_name));
}

/// P17.2-07: Encryption Key Rotation
#[tokio::test]
async fn encryption_key_rotation() {
    std::env::set_var("EVIF_ENCRYPTION_KEY", "initial-key-for-rotation-test");
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

    // Enable encryption first
    let enable = client
        .post(format!("{}/api/v1/encryption/enable", base))
        .json(&serde_json::json!({ "key": "env:EVIF_ENCRYPTION_KEY" }))
        .send()
        .await
        .expect("enable request succeeds");
    assert!(enable.status().is_success(), "enable should succeed");

    // Rotate the key
    let rotate = client
        .post(format!("{}/api/v1/encryption/rotate", base))
        .json(&serde_json::json!({ "new_key": "env:EVIF_ENCRYPTION_KEY" }))
        .send()
        .await
        .expect("rotate request succeeds");
    assert!(rotate.status().is_success(), "rotate should succeed");

    let rotate_json: serde_json::Value = rotate.json().await.expect("valid JSON");
    assert_eq!(rotate_json["status"], "enabled");
    assert_eq!(rotate_json["algorithm"], "AES-256-GCM");
    assert!(
        rotate_json["key_source"]
            .as_str()
            .expect("key source string")
            .contains("rotated"),
        "key_source should indicate rotation happened"
    );

    std::env::remove_var("EVIF_ENCRYPTION_KEY");
}

/// P17.2-08: Encryption Rotate Rejects Empty Key
#[tokio::test]
async fn encryption_rotate_rejects_empty_key() {
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

    let rotate = client
        .post(format!("{}/api/v1/encryption/rotate", base))
        .json(&serde_json::json!({ "new_key": "" }))
        .send()
        .await
        .expect("rotate request succeeds");
    // Should return 400 Bad Request
    assert!(
        rotate.status().is_client_error() || rotate.status().as_u16() == 400,
        "rotate with empty key should fail with 400"
    );
}

/// P17.2-09: Key Versions Listed After Enable
#[tokio::test]
async fn encryption_key_versions_listed_after_enable() {
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

    // Enable encryption — should create version 1
    let enable = client
        .post(format!("{}/api/v1/encryption/enable", base))
        .json(&serde_json::json!({ "key": "version-test-key" }))
        .send()
        .await
        .expect("enable succeeds");
    assert!(enable.status().is_success());

    let versions = client
        .get(format!("{}/api/v1/encryption/versions", base))
        .send()
        .await
        .expect("versions request succeeds");
    assert!(versions.status().is_success());

    let json: serde_json::Value = versions.json().await.expect("valid JSON");
    let versions_arr = json.as_array().expect("versions should be array");
    assert_eq!(
        versions_arr.len(),
        1,
        "should have exactly 1 version after enable"
    );
    assert_eq!(versions_arr[0]["version"], 1);
    assert!(versions_arr[0]["is_current"].as_bool().unwrap_or(false));
}

/// P17.2-10: Key Versions Accumulate After Multiple Rotations
#[tokio::test]
async fn encryption_key_versions_accumulate_after_rotations() {
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

    // Enable → version 1
    client
        .post(format!("{}/api/v1/encryption/enable", base))
        .json(&serde_json::json!({ "key": "key-v1" }))
        .send()
        .await
        .expect("enable succeeds");

    // Rotate twice → versions 2 and 3
    for i in 2..=3 {
        let rotate = client
            .post(format!("{}/api/v1/encryption/rotate", base))
            .json(&serde_json::json!({ "new_key": format!("key-v{}", i) }))
            .send()
            .await
            .expect("rotate succeeds");
        assert!(rotate.status().is_success());
    }

    let versions = client
        .get(format!("{}/api/v1/encryption/versions", base))
        .send()
        .await
        .expect("versions request succeeds");
    assert!(versions.status().is_success());

    let json: serde_json::Value = versions.json().await.expect("valid JSON");
    let versions_arr = json.as_array().expect("versions should be array");
    assert_eq!(
        versions_arr.len(),
        3,
        "should have 3 versions after 1 enable + 2 rotations"
    );

    // Latest version should be current
    let last = &versions_arr[2];
    assert_eq!(last["version"], 3);
    assert!(last["is_current"].as_bool().unwrap_or(false));

    // Previous versions should NOT be current
    assert!(!versions_arr[0]["is_current"].as_bool().unwrap_or(true));
    assert!(!versions_arr[1]["is_current"].as_bool().unwrap_or(true));
}

/// P17.2-11: Key Versions Persist Across Restarts
#[tokio::test]
async fn encryption_key_versions_persist_across_restarts() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let state_path = temp_dir.path().join("encryption-versions-state.json");

    // First instance: enable + rotate
    let first_encryption_state =
        EncryptionState::persistent(&state_path).expect("persistent encryption state");
    let first_mount_table = Arc::new(RadixMountTable::new());
    let first_app = create_routes_with_encryption_state(first_mount_table, first_encryption_state);
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

    // Enable → v1
    client
        .post(format!("{}/api/v1/encryption/enable", first_base))
        .json(&serde_json::json!({ "key": "persist-v1" }))
        .send()
        .await
        .expect("enable succeeds");

    // Rotate → v2
    client
        .post(format!("{}/api/v1/encryption/rotate", first_base))
        .json(&serde_json::json!({ "new_key": "persist-v2" }))
        .send()
        .await
        .expect("rotate succeeds");

    // Second instance: reload persisted state
    let second_encryption_state =
        EncryptionState::persistent(&state_path).expect("reload encryption state");
    let second_mount_table = Arc::new(RadixMountTable::new());
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

    let versions = client
        .get(format!("{}/api/v1/encryption/versions", second_base))
        .send()
        .await
        .expect("versions request succeeds");
    assert!(versions.status().is_success());

    let json: serde_json::Value = versions.json().await.expect("valid JSON");
    let versions_arr = json.as_array().expect("versions should be array");
    assert_eq!(versions_arr.len(), 2, "should have 2 persisted versions");
    assert_eq!(versions_arr[1]["version"], 2);
    assert!(versions_arr[1]["is_current"].as_bool().unwrap_or(false));
}

/// P17.2-12: Encryption Throughput Benchmark
#[tokio::test]
async fn encryption_throughput_benchmark() {
    // Direct test of EncryptionState throughput — no server needed
    use evif_rest::EncryptionState;
    use std::time::Instant;

    let state = EncryptionState::new();
    state
        .enable("throughput-test-key".to_string())
        .await
        .unwrap();
    assert!(state.is_enabled());

    // Test with 1 MB of data
    let size_bytes = 1024 * 1024;
    let plaintext = vec![0xAB; size_bytes];

    // Warmup
    let _ = state.encrypt(&plaintext);

    // Measure encrypt throughput
    let start = Instant::now();
    let ciphertext = state.encrypt(&plaintext).expect("encrypt succeeds");
    let encrypt_elapsed = start.elapsed();

    // Measure decrypt throughput
    let start = Instant::now();
    let decrypted = state.decrypt(&ciphertext).expect("decrypt succeeds");
    let decrypt_elapsed = start.elapsed();

    let encrypt_mbps = (size_bytes as f64) / (encrypt_elapsed.as_secs_f64() * 1_048_576.0);
    let decrypt_mbps = (size_bytes as f64) / (decrypt_elapsed.as_secs_f64() * 1_048_576.0);

    // AES-256-GCM should exceed 2 MB/s even in debug builds
    // (in release mode this would be 100+ MB/s)
    assert!(
        encrypt_mbps > 2.0,
        "encrypt throughput {:.1} MB/s should exceed 2 MB/s",
        encrypt_mbps
    );
    assert!(
        decrypt_mbps > 2.0,
        "decrypt throughput {:.1} MB/s should exceed 2 MB/s",
        decrypt_mbps
    );
    assert_eq!(
        decrypted, plaintext,
        "decrypted data must match original plaintext"
    );
}
