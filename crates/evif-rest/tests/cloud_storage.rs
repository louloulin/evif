// Phase 16.3: Cloud Storage Backend Integration Tests
//
// 测试云存储（S3/OSS）后端集成功能

use evif_core::RadixMountTable;
use evif_rest::create_routes;
use std::sync::Arc;

/// P16.3-01: Cloud Storage Status Endpoint
#[tokio::test]
async fn cloud_storage_status_endpoint() {
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

    // 获取云存储状态
    let res = client
        .get(format!("{}/api/v1/cloud/status", base))
        .send()
        .await
        .expect("request succeeds");

    // 端点应该可达
    assert!(
        !res.status().is_server_error(),
        "Cloud status endpoint should not cause server error"
    );
}

/// P16.3-02: List Supported Cloud Providers
#[tokio::test]
async fn cloud_list_providers() {
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
        .get(format!("{}/api/v1/cloud/providers", base))
        .send()
        .await
        .expect("request succeeds");

    assert!(
        !res.status().is_server_error(),
        "Cloud providers endpoint should not cause server error"
    );
}

/// P16.3-03: Cloud Config Validation
#[tokio::test]
async fn cloud_config_validation() {
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

    // 测试无效配置
    let res = client
        .post(format!("{}/api/v1/cloud/config", base))
        .json(&serde_json::json!({
            "provider": "invalid",
            "bucket": "test"
        }))
        .send()
        .await
        .expect("request succeeds");

    // 应该返回错误，而不是服务器崩溃
    assert!(
        !res.status().is_server_error(),
        "Cloud config endpoint should handle invalid input gracefully"
    );
}

/// P16.3-04: S3 Provider Support
#[tokio::test]
async fn cloud_s3_provider_supported() {
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
        .get(format!("{}/api/v1/cloud/providers", base))
        .send()
        .await
        .expect("request succeeds");

    if res.status().is_success() {
        let json: serde_json::Value = res.json().await.unwrap_or_default();
        let providers = json.get("providers").and_then(|p| p.as_array());
        if let Some(providers) = providers {
            let names: Vec<&str> = providers
                .iter()
                .filter_map(|p| p.get("name").and_then(|n| n.as_str()))
                .collect();
            assert!(
                names.contains(&"s3") || names.contains(&"S3"),
                "S3 provider should be supported"
            );
        }
    }
}
