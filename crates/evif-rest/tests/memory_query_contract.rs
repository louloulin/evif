use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use evif_core::RadixMountTable;
use evif_rest::create_routes;
use serde_json::Value;
use std::sync::Arc;
use tower05::util::ServiceExt;

async fn create_memory(app: &axum::Router, content: &str) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/memories")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "content": content,
                        "modality": "text"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("create memory response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("create memory body");
    let json: Value = serde_json::from_slice(&body).expect("create memory json");
    json["memory_id"]
        .as_str()
        .expect("memory_id present")
        .to_string()
}

#[tokio::test]
async fn test_memory_query_uses_memories_endpoint_and_legacy_graph_endpoint_is_gone() {
    let mount_table = Arc::new(RadixMountTable::new());
    let app = create_routes(mount_table);

    let first = create_memory(&app, "alpha memory").await;
    let second = create_memory(&app, "beta memory").await;

    let timeline_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/memories/query")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"query_type":"timeline"}"#))
                .unwrap(),
        )
        .await
        .expect("timeline response");

    assert_eq!(timeline_response.status(), StatusCode::OK);
    let timeline_body = to_bytes(timeline_response.into_body(), usize::MAX)
        .await
        .expect("timeline body");
    let timeline_json: Value = serde_json::from_slice(&timeline_body).expect("timeline json");
    let timeline_node_ids: Vec<String> = timeline_json["nodes"]
        .as_array()
        .expect("timeline nodes array")
        .iter()
        .map(|node| node["id"].as_str().unwrap().to_string())
        .collect();

    assert_eq!(timeline_json["query_type"], "timeline");
    assert_eq!(timeline_json["total"], 2);
    assert_eq!(timeline_node_ids, vec![first, second]);

    let legacy_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/{}/query", "graph"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"query_type":"timeline"}"#))
                .unwrap(),
        )
        .await
        .expect("legacy response");
    assert_eq!(legacy_response.status(), StatusCode::NOT_FOUND);
}
