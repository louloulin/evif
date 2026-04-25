use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use evif_core::RadixMountTable;
use evif_rest::create_routes;
use std::sync::Arc;
use tower05::util::ServiceExt;

#[tokio::test]
async fn request_body_limit_rejects_oversized_json_payload() {
    std::env::set_var("EVIF_REST_MAX_BODY_BYTES", "128");
    let app = create_routes(Arc::new(RadixMountTable::new()));
    std::env::remove_var("EVIF_REST_MAX_BODY_BYTES");

    let oversized_content = "x".repeat(512);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/memories")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "content": oversized_content,
                        "modality": "text"
                    })
                    .to_string(),
                ))
                .expect("request builds"),
        )
        .await
        .expect("response is returned");

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
