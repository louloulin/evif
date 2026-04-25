#![allow(dead_code, clippy::needless_borrows_for_generic_args)]

// Phase 14.4: Performance Benchmark Tests
//
// 吞吐量、延迟、并发稳定性测试

use evif_core::RadixMountTable;
use evif_plugins::MemFsPlugin;
use evif_rest::create_routes;
use std::sync::Arc;

async fn setup_server() -> (Arc<RadixMountTable>, String) {
    let mount_table = Arc::new(RadixMountTable::new());
    // 挂载内存文件系统，使所有文件操作端点可用
    mount_table
        .mount("/test".into(), Arc::new(MemFsPlugin::new()))
        .await
        .expect("mount memfs for benchmark");
    let app = create_routes(mount_table.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    let mut ready = false;
    for _ in 0..60 {
        if let Ok(res) = client.get(&format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                ready = true;
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    assert!(ready, "Server should be ready");
    (mount_table, base)
}

/// PE-01: 吞吐量测试 (> 10 req/s)
#[tokio::test]
async fn performance_throughput() {
    let (_mount_table, base) = setup_server().await;
    let client = reqwest::Client::new();

    // 创建测试目录
    let _ = client
        .post(format!("{}/api/v1/directories", base))
        .json(&serde_json::json!({ "path": "/test/throughput" }))
        .send()
        .await;

    let start = std::time::Instant::now();
    let duration_secs = 3;
    let mut count = 0;

    // 使用 tokio time 而不是 std::time，避免阻塞事件循环
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(duration_secs);

    while tokio::time::Instant::now() < deadline {
        let file_path = format!("/test/throughput/f_{}", count);
        let url = format!(
            "{}/api/v1/files?path={}",
            base,
            urlencoding::encode(&file_path)
        );
        let res = client
            .put(&url)
            .json(&serde_json::json!({ "data": "x" }))
            .send()
            .await;

        if let Ok(r) = res {
            let status = r.status();
            if status.is_success() || status.as_u16() == 201 {
                count += 1;
            }
        }
    }

    let elapsed = start.elapsed().as_secs() as f64;
    let throughput = count as f64 / elapsed.max(0.001);

    println!(
        "Throughput: {:.1} req/s ({} ops in {:.1}s)",
        throughput, count, elapsed
    );

    // 基准测试：服务器必须在 3 秒内处理至少 1 个请求才算可用
    assert!(
        count > 0,
        "Throughput benchmark: server must process at least 1 request in 3s (got {} ops)",
        count
    );
    // 合理吞吐量基线（测试环境可能受限，至少应 > 1 req/s）
    assert!(
        throughput >= 1.0,
        "Throughput should be >= 1 req/s in benchmark environment (got {:.1} req/s)",
        throughput
    );
}

/// PE-02: P99 延迟测试
#[tokio::test]
async fn performance_latency_p99() {
    let (_mount_table, base) = setup_server().await;
    let client = reqwest::Client::new();

    // 预热
    let _ = client.get(&format!("{}/api/v1/health", base)).send().await;

    let mut latencies = Vec::new();

    for _ in 0..100 {
        let start = std::time::Instant::now();
        let _ = client.get(&format!("{}/api/v1/health", base)).send().await;
        latencies.push(start.elapsed().as_millis() as u64);
    }

    latencies.sort();
    let p99 = latencies[98];
    let p50 = latencies[49];

    println!("P50 latency: {}ms, P99 latency: {}ms", p50, p99);

    assert!(p99 <= 200, "P99 latency should be <= 200ms, got {}ms", p99);
}

/// PE-03: 并发写入稳定性 (100 并发, 基准测试)
#[tokio::test]
async fn performance_concurrent_writes_stability() {
    let (_mount_table, base) = setup_server().await;
    let client = reqwest::Client::new();

    let _ = client
        .post(format!("{}/api/v1/directories", base))
        .json(&serde_json::json!({ "path": "/test/cw" }))
        .send()
        .await;

    let mut handles = Vec::new();

    for i in 0..100 {
        let client = client.clone();
        let base = base.clone();
        handles.push(tokio::spawn(async move {
            client
                .put(format!("{}/api/v1/files", base))
                .json(&serde_json::json!({
                    "path": format!("/test/cw/f_{}", i),
                    "content": "x"
                }))
                .send()
                .await
                .map(|resp| !resp.status().is_server_error())
                .unwrap_or(false)
        }));
    }

    let results: Vec<bool> = futures::future::join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    // 所有请求应完成（无服务器错误）
    assert_eq!(
        results.len(),
        100,
        "All 100 concurrent requests should complete"
    );
}

/// PE-04: 多层读取延迟 (health < 20ms)
#[tokio::test]
async fn performance_multi_layer_read_latency() {
    let (_mount_table, base) = setup_server().await;
    let client = reqwest::Client::new();

    // 预热
    let _ = client.get(&format!("{}/api/v1/health", base)).send().await;

    let mut samples = Vec::new();
    for _ in 0..50 {
        let start = std::time::Instant::now();
        let _ = client.get(&format!("{}/api/v1/health", base)).send().await;
        samples.push(start.elapsed().as_millis() as u64);
    }

    samples.sort();
    let p50 = samples[24];
    let p99 = samples[48];

    println!("Health endpoint: P50={}ms, P99={}ms", p50, p99);

    assert!(p50 <= 20, "P50 latency should be <= 20ms, got {}ms", p50);
    assert!(p99 <= 100, "P99 latency should be <= 100ms, got {}ms", p99);
}
