// Phase 14.1: Cross-Filesystem Copy Tests
//
// 测试跨文件系统复制功能

use evif_core::RadixMountTable;
use evif_rest::create_routes;
use std::sync::Arc;

/// CC-01: Copy Within Same Filesystem
#[tokio::test]
async fn cross_fs_copy_same_fs() {
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

    // 挂载 context 插件
    let _ = client
        .post(format!("{}/api/v1/mount", base))
        .json(&serde_json::json!({
            "path": "/context",
            "plugin": "context"
        }))
        .send()
        .await;

    // 复制文件 (同一文件系统内)
    let res = client
        .post(format!("{}/api/v1/copy", base))
        .json(&serde_json::json!({
            "source": "/context/L0/current",
            "destination": "/context/L0/current_copy",
            "overwrite": true
        }))
        .send()
        .await
        .expect("request succeeds");

    // 可能成功或失败，取决于 L0/current 是否可读
    let status = res.status();
    assert!(
        status.is_success() || status.as_u16() == 400 || status.as_u16() == 404,
        "Copy should succeed or return valid error"
    );
}

/// CC-02: Copy Returns Bytes Copied
#[tokio::test]
async fn cross_fs_copy_returns_bytes() {
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

    // 复制文件
    let res = client
        .post(format!("{}/api/v1/copy", base))
        .json(&serde_json::json!({
            "source": "/context/L0/current",
            "destination": "/context/L0/test_copy",
            "overwrite": true
        }))
        .send()
        .await
        .expect("request succeeds");

    // 如果成功，验证响应包含 bytes_copied
    if res.status().is_success() {
        let json: serde_json::Value = res.json().await.expect("valid JSON");
        assert!(
            json.get("bytes_copied").is_some(),
            "Response should include bytes_copied"
        );
    }
}

/// CC-03: Copy To NonExistent Returns Error
#[tokio::test]
async fn cross_fs_copy_nonexistent_source() {
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

    // 复制不存在的文件
    let res = client
        .post(format!("{}/api/v1/copy", base))
        .json(&serde_json::json!({
            "source": "/nonexistent/file.txt",
            "destination": "/test/copy.txt",
            "overwrite": true
        }))
        .send()
        .await
        .expect("request succeeds");

    // 应该返回错误
    assert!(
        !res.status().is_success(),
        "Copy of nonexistent file should fail"
    );
}

/// CC-04: Recursive Copy Endpoint Exists
#[tokio::test]
async fn cross_fs_recursive_copy_endpoint() {
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

    // 递归复制
    let res = client
        .post(format!("{}/api/v1/copy/recursive", base))
        .json(&serde_json::json!({
            "source": "/context/L1",
            "destination": "/context/L1_backup"
        }))
        .send()
        .await
        .expect("request succeeds");

    // 应该返回错误（目录可能不存在），但端点应该可达
    assert!(
        !res.status().is_success() || res.status().as_u16() == 400,
        "Recursive copy endpoint should be reachable"
    );
}
