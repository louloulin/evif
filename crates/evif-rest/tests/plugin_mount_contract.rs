use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use evif_core::{EvifPlugin, RadixMountTable};
use evif_plugins::MemFsPlugin;
use evif_rest::create_routes;
use serde_json::Value;
use std::sync::Arc;
use tower05::util::ServiceExt;

#[tokio::test]
async fn test_available_plugins_lists_core_inventory_and_mount_state() {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    mount_table.mount("/mem".to_string(), mem).await.unwrap();

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

    let memfs = plugins
        .iter()
        .find(|plugin| plugin["id"].as_str() == Some("memfs"))
        .expect("memfs entry");
    let localfs = plugins
        .iter()
        .find(|plugin| plugin["id"].as_str() == Some("localfs"))
        .expect("localfs entry");

    assert_eq!(memfs["support_tier"], "core");
    assert_eq!(memfs["is_mountable"], true);
    assert_eq!(memfs["is_mounted"], true);
    assert_eq!(memfs["mount_path"], "/mem");

    assert_eq!(localfs["support_tier"], "core");
    assert_eq!(localfs["is_mountable"], true);
    assert_eq!(localfs["is_mounted"], false);
}
