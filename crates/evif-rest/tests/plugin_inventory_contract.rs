use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use evif_core::RadixMountTable;
use evif_rest::create_routes;
use serde_json::Value;
use std::{collections::BTreeSet, sync::Arc};
use tower05::util::ServiceExt;

#[tokio::test]
async fn test_available_plugins_separate_core_and_experimental_inventory() {
    let mount_table = Arc::new(RadixMountTable::new());
    let app = create_routes(mount_table);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/plugins/available")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("available plugins response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("available plugins body");
    let json: Value = serde_json::from_slice(&body).expect("available plugins json");
    let plugins = json["plugins"].as_array().expect("plugins array");

    let core_ids: BTreeSet<&str> = plugins
        .iter()
        .filter(|plugin| plugin["support_tier"].as_str() == Some("core"))
        .map(|plugin| plugin["id"].as_str().expect("plugin id"))
        .collect();
    let expected_core_ids = BTreeSet::from([
        "contextfs",
        "heartbeatfs",
        "hellofs",
        "kvfs",
        "localfs",
        "memfs",
        "pipefs",
        "proxyfs",
        "queuefs",
        "serverinfofs",
        "skillfs",
        "sqlfs2",
        "streamfs",
    ]);

    assert_eq!(core_ids, expected_core_ids);

    let httpfs = plugins
        .iter()
        .find(|plugin| plugin["id"].as_str() == Some("httpfs"))
        .expect("httpfs entry");
    assert_eq!(httpfs["support_tier"], "experimental");
}
