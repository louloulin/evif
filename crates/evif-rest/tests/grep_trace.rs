// Phase 14.3: Search Trace Visualization Tests
//
// 测试 grep trace 功能：检索时记录轨迹，包括每步的路径、操作类型、命中数和延迟

use evif_core::RadixMountTable;
use evif_rest::create_routes;
use std::sync::Arc;

/// ST-01: Grep Trace Records Steps
#[tokio::test]
async fn grep_trace_records_steps() {
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

    // 等待服务器就绪并挂载 context 插件
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

    // 使用 trace=true 执行 grep
    let res = client
        .post(format!("{}/api/v1/grep", base))
        .json(&serde_json::json!({
            "path": "/context/L0",
            "pattern": "current",
            "trace": true
        }))
        .send()
        .await
        .expect("request succeeds");

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        panic!("grep failed with status {}: {}", status, body);
    }

    let json: serde_json::Value = res.json().await.expect("valid JSON");
    let trace = json.get("trace");
    assert!(
        trace.is_some(),
        "trace field should be present when trace=true, got: {}",
        json
    );
}

/// ST-02: Grep Without Trace Has No Trace Field
#[tokio::test]
async fn grep_without_trace_has_no_trace_field() {
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

    // 等待服务器就绪并挂载 context 插件
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

    let res = client
        .post(format!("{}/api/v1/grep", base))
        .json(&serde_json::json!({
            "path": "/context/L0",
            "pattern": "current"
        }))
        .send()
        .await
        .expect("request succeeds");

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        panic!("grep failed with status {}: {}", status, body);
    }

    let json: serde_json::Value = res.json().await.expect("valid JSON");
    let trace = json.get("trace");
    assert!(
        trace.is_none() || trace.as_ref().is_some_and(|v| v.is_null()),
        "trace field should be absent or null when trace not set, got: {}",
        json
    );
}

/// ST-03: Grep Trace Contains Latency Info
#[tokio::test]
async fn grep_trace_contains_latency_info() {
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

    // 等待服务器就绪并挂载 context 插件
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

    let res = client
        .post(format!("{}/api/v1/grep", base))
        .json(&serde_json::json!({
            "path": "/context/L1",
            "pattern": "session",
            "trace": true
        }))
        .send()
        .await
        .expect("request succeeds");

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        panic!("grep failed with status {}: {}", status, body);
    }

    let json: serde_json::Value = res.json().await.expect("valid JSON");
    let trace = json.get("trace");

    assert!(
        trace.is_some(),
        "trace field should be present, got: {}",
        json
    );

    let steps = trace.and_then(|t| t.as_array());
    assert!(
        steps.is_some() && !steps.unwrap().is_empty(),
        "trace should be a non-empty array, got: {:?}",
        trace
    );

    if let Some(arr) = steps {
        for step in arr {
            assert!(
                step.get("path").is_some(),
                "each trace step should have a path: {:?}",
                step
            );
            assert!(
                step.get("operation").is_some(),
                "each trace step should have an operation: {:?}",
                step
            );
            assert!(
                step.get("hits").is_some(),
                "each trace step should have hits: {:?}",
                step
            );
            assert!(
                step.get("latency_ms").is_some(),
                "each trace step should have latency_ms: {:?}",
                step
            );
        }
    }
}
