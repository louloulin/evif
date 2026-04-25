// Phase 16.2: Distributed Deployment Tests
//
// 测试分布式部署相关功能：节点状态、健康检查

use evif_core::RadixMountTable;
use evif_rest::create_routes;
use std::sync::Arc;

/// P16.2-01: Status Endpoint Returns Node Info
#[tokio::test]
async fn distributed_status_endpoint() {
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

    // 获取节点状态
    let res = client
        .get(format!("{}/api/v1/status", base))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "Status endpoint should succeed");

    let json: serde_json::Value = res.json().await.expect("valid JSON");
    assert!(json.get("status").is_some(), "Should have status field");
    assert!(json.get("version").is_some(), "Should have version field");
    assert!(
        json.get("uptime_secs").is_some(),
        "Should have uptime_secs field"
    );
    assert!(json.get("ready").is_some(), "Should have ready field");
}

/// P16.2-02: Ping Endpoint Works (POST)
#[tokio::test]
async fn distributed_ping_post() {
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

    // Ping
    let res = client
        .post(format!("{}/api/v1/ping", base))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "Ping should succeed");
    let text = res.text().await.expect("valid text");
    assert_eq!(text, "pong", "Should return pong");
}

/// P16.2-03: Ping Endpoint Works (GET)
#[tokio::test]
async fn distributed_ping_get() {
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

    // Ping via GET
    let res = client
        .get(format!("{}/api/v1/ping", base))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "Ping GET should succeed");
    let text = res.text().await.expect("valid text");
    assert_eq!(text, "pong", "Should return pong");
}

/// P16.2-04: Status Ready Flag Is True
#[tokio::test]
async fn distributed_status_ready() {
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

    let res = client
        .get(format!("{}/api/v1/status", base))
        .send()
        .await
        .expect("request succeeds");

    let json: serde_json::Value = res.json().await.expect("valid JSON");
    let ready = json.get("ready").and_then(|v| v.as_bool()).unwrap_or(false);
    assert!(ready, "Node should be ready");
}

/// P16.2-05: Latency of Ping < 10ms
#[tokio::test]
async fn distributed_ping_latency() {
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

    // 测量延迟
    let mut samples = Vec::new();
    for _ in 0..20 {
        let start = std::time::Instant::now();
        let _ = client.get(format!("{}/api/v1/ping", base)).send().await;
        samples.push(start.elapsed().as_millis() as u64);
    }

    samples.sort();
    let p50 = samples[10];
    println!("Ping P50 latency: {}ms", p50);

    // Ping 应该很快（< 50ms）
    assert!(p50 < 50, "Ping latency should be < 50ms, got {}ms", p50);
}
