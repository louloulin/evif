// Phase 14.2: File Lock Tests
//
// 测试文件锁功能：防止并发写入同一文件的冲突

use evif_core::RadixMountTable;
use evif_rest::create_routes;
use std::sync::Arc;

/// FL-01: Lock Acquisition
#[tokio::test]
async fn file_lock_acquisition() {
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

    // 获取锁
    let res = client
        .post(format!("{}/api/v1/lock", base))
        .json(&serde_json::json!({
            "path": "/test/file.txt",
            "operation": "write"
        }))
        .send()
        .await
        .expect("request succeeds");

    // 应该成功或返回锁已存在的状态
    let status = res.status();
    assert!(
        status.is_success() || status.as_u16() == 409,
        "Lock acquisition should succeed or return conflict"
    );
}

/// FL-02: Lock Release
#[tokio::test]
async fn file_lock_release() {
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

    // 释放锁
    let res = client
        .delete(format!("{}/api/v1/lock", base))
        .json(&serde_json::json!({
            "path": "/test/file.txt"
        }))
        .send()
        .await
        .expect("request succeeds");

    // 应该成功
    let status = res.status();
    assert!(
        status.is_success() || status.as_u16() == 404,
        "Lock release should succeed or return not found"
    );
}

/// FL-03: Lock List
#[tokio::test]
async fn file_lock_list() {
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

    // 获取锁列表
    let res = client
        .get(format!("{}/api/v1/locks", base))
        .send()
        .await
        .expect("request succeeds");

    // 应该成功
    assert!(
        res.status().is_success(),
        "Lock list should succeed"
    );
}
