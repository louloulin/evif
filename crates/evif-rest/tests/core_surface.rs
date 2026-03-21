use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use evif_core::RadixMountTable;
use evif_rest::create_routes;
use std::sync::Arc;
use tower05::util::ServiceExt;

#[tokio::test]
async fn test_legacy_graph_routes_are_not_exposed() {
    let mount_table = Arc::new(RadixMountTable::new());
    let app = create_routes(mount_table);

    let get_node = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/nodes/some-node")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("get node response");
    assert_eq!(get_node.status(), StatusCode::NOT_FOUND);

    let query = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/query")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"query":"anything"}"#))
                .unwrap(),
        )
        .await
        .expect("query response");
    assert_eq!(query.status(), StatusCode::NOT_FOUND);

    let stats = app
        .oneshot(
            Request::builder()
                .uri("/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("stats response");
    assert_eq!(stats.status(), StatusCode::NOT_FOUND);
}
