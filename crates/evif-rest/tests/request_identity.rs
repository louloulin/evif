use evif_core::RadixMountTable;
use evif_rest::create_routes;
use std::sync::Arc;

async fn spawn_app() -> (String, reqwest::Client) {
    let app = create_routes(Arc::new(RadixMountTable::new()));
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
async fn request_identity_generates_headers_when_missing() {
    let (base, client) = spawn_app().await;

    let response = client
        .get(format!("{}/api/v1/health", base))
        .send()
        .await
        .expect("health request succeeds");
    assert!(response.status().is_success(), "health should succeed");

    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let correlation_id = response
        .headers()
        .get("x-correlation-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    assert!(
        !request_id.is_empty(),
        "server should generate x-request-id when client does not provide one",
    );
    assert!(
        uuid::Uuid::parse_str(request_id).is_ok(),
        "generated x-request-id should be a UUID",
    );
    assert_eq!(
        correlation_id, request_id,
        "missing correlation id should default to the resolved request id",
    );
}

#[tokio::test]
async fn request_identity_preserves_client_supplied_headers() {
    let (base, client) = spawn_app().await;

    let response = client
        .get(format!("{}/api/v1/health", base))
        .header("x-request-id", "req-from-client-123")
        .header("x-correlation-id", "corr-from-client-456")
        .send()
        .await
        .expect("health request succeeds");
    assert!(response.status().is_success(), "health should succeed");

    assert_eq!(
        response
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok()),
        Some("req-from-client-123"),
        "server should propagate client-supplied request id",
    );
    assert_eq!(
        response
            .headers()
            .get("x-correlation-id")
            .and_then(|value| value.to_str().ok()),
        Some("corr-from-client-456"),
        "server should propagate client-supplied correlation id",
    );
}
