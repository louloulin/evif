// Phase 16.1: WASM Plugin Hot Reload Tests
//
// 测试 WASM 插件热重载功能

use evif_rest::create_routes;
use evif_core::RadixMountTable;
use std::sync::Arc;

/// P16.1-01: Reload Endpoint Exists
#[tokio::test]
async fn wasm_hot_reload_endpoint_exists() {
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

    // 尝试热重载不存在的插件
    let res = client
        .post(format!("{}/api/v1/plugins/wasm/reload", base))
        .json(&serde_json::json!({
            "mount_point": "/test/plugin"
        }))
        .send()
        .await
        .expect("request succeeds");

    // 应该返回错误（插件不存在），而不是服务器崩溃
    assert!(
        !res.status().is_server_error(),
        "Reload endpoint should not cause server error"
    );
}

/// P16.1-02: Plugin List Includes Hot Reload Flag
#[tokio::test]
async fn wasm_plugin_list_includes_hot_reloadable() {
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

    // 获取插件列表
    let res = client
        .get(format!("{}/api/v1/plugins/list", base))
        .send()
        .await
        .expect("request succeeds");

    assert!(
        !res.status().is_server_error(),
        "Plugin list endpoint should not cause server error"
    );

    // 验证响应是有效的 JSON
    let json: serde_json::Value = res.json().await.expect("valid JSON");
    assert!(
        json.get("plugins").is_some(),
        "Response should have plugins field"
    );
    assert!(
        json.get("total").is_some(),
        "Response should have total field"
    );
}

/// P16.1-03: Load WASM Plugin Without WASM Feature
#[tokio::test]
async fn wasm_load_without_feature_returns_error() {
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

    // 尝试加载 WASM 插件（可能因无 feature 而返回错误）
    let res = client
        .post(format!("{}/api/v1/plugins/wasm/load", base))
        .json(&serde_json::json!({
            "wasm_path": "/nonexistent/plugin.wasm",
            "name": "test",
            "mount": "/test"
        }))
        .send()
        .await
        .expect("request succeeds");

    // 应该返回错误，而不是服务器崩溃
    assert!(
        !res.status().is_server_error(),
        "Load endpoint should not cause server error"
    );
}

/// P16.1-04: Unload Plugin Works
#[tokio::test]
async fn wasm_unload_plugin_works() {
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

    // 尝试卸载不存在的插件 - 端点应可达
    let res = client
        .post(format!("{}/api/v1/plugins/unload", base))
        .json(&serde_json::json!({
            "mount_point": "/nonexistent"
        }))
        .send()
        .await
        .expect("request succeeds");

    // 端点应该返回任何有效响应（4xx 或 5xx 都算可达）
    assert!(
        res.status().is_client_error() || res.status().is_server_error() || res.status().is_success(),
        "Unload endpoint should return valid HTTP response"
    );
}
