use evif_core::{EvifPlugin, RadixMountTable};
use evif_plugins::MemFsPlugin;
use evif_rest::create_routes;
use std::sync::Arc;

fn metric_value(body: &str, metric_name: &str, operation: &str) -> Option<f64> {
    let prefix = format!(r#"{metric_name}{{operation="{operation}"}} "#);
    body.lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .and_then(|value| value.trim().parse::<f64>().ok())
}

async fn spawn_app() -> (String, reqwest::Client) {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    mount_table.mount("/mem".to_string(), mem).await.unwrap();
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

    (base, client)
}

#[tokio::test]
async fn metrics_traffic_counts_real_requests() {
    let (base, client) = spawn_app().await;

    let reset = client
        .post(format!("{}/api/v1/metrics/reset", base))
        .send()
        .await
        .expect("reset request succeeds");
    assert!(reset.status().is_success(), "metrics reset should succeed");

    let write = client
        .post(format!("{}/api/v1/files", base))
        .json(&serde_json::json!({
            "path": "/mem/traffic.txt"
        }))
        .send()
        .await
        .expect("create request succeeds");
    assert!(write.status().is_success(), "create should succeed");

    let write = client
        .put(format!("{}/api/v1/files?path=/mem/traffic.txt", base))
        .json(&serde_json::json!({
            "data": "traffic-content",
            "encoding": null
        }))
        .send()
        .await
        .expect("write request succeeds");
    assert!(write.status().is_success(), "write should succeed");

    let read = client
        .get(format!("{}/api/v1/files?path=/mem/traffic.txt", base))
        .send()
        .await
        .expect("read request succeeds");
    assert!(read.status().is_success(), "read should succeed");

    let list = client
        .get(format!("{}/api/v1/directories?path=/mem", base))
        .send()
        .await
        .expect("list request succeeds");
    assert!(list.status().is_success(), "list should succeed");

    let missing = client
        .get(format!("{}/api/v1/files?path=/mem/missing.txt", base))
        .send()
        .await
        .expect("missing request succeeds");
    assert!(
        missing.status().is_client_error() || missing.status().is_server_error(),
        "missing file should return error status"
    );

    let metrics = client
        .get(format!("{}/api/v1/metrics/traffic", base))
        .send()
        .await
        .expect("metrics request succeeds");
    assert!(metrics.status().is_success(), "metrics endpoint should succeed");

    let json: serde_json::Value = metrics.json().await.expect("metrics should be valid JSON");
    assert_eq!(json["total_requests"], 5, "expected 5 tracked requests");
    assert_eq!(json["write_count"], 2, "expected 2 write requests including create");
    assert_eq!(json["read_count"], 2, "expected 2 read requests including missing read");
    assert_eq!(json["list_count"], 1, "expected 1 list request");
    assert_eq!(json["total_errors"], 1, "expected 1 error request");
}

#[tokio::test]
async fn metrics_prometheus_endpoint_exposes_standard_text_format() {
    let (base, client) = spawn_app().await;

    let reset = client
        .post(format!("{}/api/v1/metrics/reset", base))
        .send()
        .await
        .expect("reset request succeeds");
    assert!(reset.status().is_success(), "metrics reset should succeed");

    let write = client
        .post(format!("{}/api/v1/files", base))
        .json(&serde_json::json!({
            "path": "/mem/prometheus.txt"
        }))
        .send()
        .await
        .expect("create request succeeds");
    assert!(write.status().is_success(), "create should succeed");

    let read = client
        .get(format!("{}/api/v1/files?path=/mem/prometheus.txt", base))
        .send()
        .await
        .expect("read request succeeds");
    assert!(read.status().is_success(), "read should succeed");

    let metrics = client
        .get(format!("{}/metrics", base))
        .send()
        .await
        .expect("metrics request succeeds");
    assert!(metrics.status().is_success(), "metrics endpoint should succeed");

    let content_type = metrics
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    assert_eq!(
        content_type,
        "text/plain; version=0.0.4; charset=utf-8",
        "prometheus endpoint should expose the standard text format",
    );

    let body = metrics.text().await.expect("metrics body should be readable");
    assert!(
        body.contains("# HELP evif_total_requests Total number of requests processed"),
        "metrics body should include HELP metadata",
    );
    assert!(
        body.contains("evif_total_requests 2"),
        "metrics body should reflect real request counts without counting the scrape itself",
    );
    assert!(
        body.contains("evif_write_count 1"),
        "metrics body should reflect write traffic",
    );
    assert!(
        body.contains("evif_read_count 1"),
        "metrics body should reflect read traffic",
    );
}

#[tokio::test]
async fn metrics_prometheus_endpoint_exposes_success_error_and_latency_by_operation() {
    let (base, client) = spawn_app().await;

    let reset = client
        .post(format!("{}/api/v1/metrics/reset", base))
        .send()
        .await
        .expect("reset request succeeds");
    assert!(reset.status().is_success(), "metrics reset should succeed");

    let create = client
        .post(format!("{}/api/v1/files", base))
        .json(&serde_json::json!({
            "path": "/mem/operation-metrics.txt"
        }))
        .send()
        .await
        .expect("create request succeeds");
    assert!(create.status().is_success(), "create should succeed");

    let write = client
        .put(format!("{}/api/v1/files?path=/mem/operation-metrics.txt", base))
        .json(&serde_json::json!({
            "data": "metrics-content",
            "encoding": null
        }))
        .send()
        .await
        .expect("write request succeeds");
    assert!(write.status().is_success(), "write should succeed");

    let read = client
        .get(format!("{}/api/v1/files?path=/mem/operation-metrics.txt", base))
        .send()
        .await
        .expect("read request succeeds");
    assert!(read.status().is_success(), "read should succeed");

    let list = client
        .get(format!("{}/api/v1/directories?path=/mem", base))
        .send()
        .await
        .expect("list request succeeds");
    assert!(list.status().is_success(), "list should succeed");

    let missing = client
        .get(format!("{}/api/v1/files?path=/mem/does-not-exist.txt", base))
        .send()
        .await
        .expect("missing request succeeds");
    assert!(
        missing.status().is_client_error() || missing.status().is_server_error(),
        "missing file should return error status"
    );

    let metrics = client
        .get(format!("{}/metrics", base))
        .send()
        .await
        .expect("metrics request succeeds");
    assert!(metrics.status().is_success(), "metrics endpoint should succeed");

    let body = metrics.text().await.expect("metrics body should be readable");
    assert!(
        body.contains("# HELP evif_operation_success_total Successful HTTP requests by operation"),
        "metrics body should include success metric metadata",
    );
    assert_eq!(
        metric_value(&body, "evif_operation_success_total", "write"),
        Some(2.0),
        "write success count should include create + write",
    );
    assert_eq!(
        metric_value(&body, "evif_operation_success_total", "read"),
        Some(1.0),
        "read success count should include the successful file read only",
    );
    assert_eq!(
        metric_value(&body, "evif_operation_success_total", "list"),
        Some(1.0),
        "list success count should be tracked",
    );
    assert_eq!(
        metric_value(&body, "evif_operation_error_total", "read"),
        Some(1.0),
        "missing read should count as a read error",
    );
    assert!(
        metric_value(&body, "evif_operation_latency_micros_total", "write")
            .is_some_and(|value| value >= 0.0),
        "write latency metric should be exported",
    );
    assert!(
        metric_value(&body, "evif_operation_latency_micros_average", "read")
            .is_some_and(|value| value >= 0.0),
        "read average latency metric should be exported",
    );
}
