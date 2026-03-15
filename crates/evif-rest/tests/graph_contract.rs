use evif_core::RadixMountTable;
use evif_rest::create_routes;
use std::sync::Arc;
use tokio::net::TcpListener;

async fn spawn_graph_test_app() -> (String, reqwest::Client) {
    let mount_table = Arc::new(RadixMountTable::new());
    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    for _ in 0..50 {
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        if client
            .get(format!("{}/health", base))
            .send()
            .await
            .ok()
            .map(|res| res.status().is_success())
            == Some(true)
        {
            return (base, client);
        }
    }

    panic!("server did not become ready in time");
}

async fn create_memory_via_api(client: &reqwest::Client, base: &str, content: &str) -> String {
    let response = client
        .post(format!("{}/api/v1/memories", base))
        .json(&serde_json::json!({
            "content": content,
            "modality": "text"
        }))
        .send()
        .await
        .expect("create memory request");

    assert!(
        response.status().is_success(),
        "create memory should succeed"
    );
    let body: serde_json::Value = response.json().await.expect("create memory body");
    body["memory_id"]
        .as_str()
        .expect("memory_id present")
        .to_string()
}

#[tokio::test]
async fn test_graph_query_timeline_and_temporal_path_use_real_memory_order() {
    let (base, client) = spawn_graph_test_app().await;

    let first = create_memory_via_api(&client, &base, "alpha memory").await;
    tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
    let second = create_memory_via_api(&client, &base, "beta memory").await;
    tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
    let third = create_memory_via_api(&client, &base, "gamma memory").await;
    let expected_ids = vec![first.clone(), second.clone(), third.clone()];

    let timeline_response = client
        .post(format!("{}/api/v1/graph/query", base))
        .json(&serde_json::json!({
            "query_type": "timeline"
        }))
        .send()
        .await
        .expect("timeline request");

    assert!(timeline_response.status().is_success());
    let timeline_body: serde_json::Value = timeline_response.json().await.expect("timeline body");
    let timeline_node_ids: Vec<String> = timeline_body["nodes"]
        .as_array()
        .expect("timeline nodes array")
        .iter()
        .map(|node| node["id"].as_str().unwrap().to_string())
        .collect();
    let timeline_event_ids: Vec<String> = timeline_body["timeline"]
        .as_array()
        .expect("timeline events array")
        .iter()
        .map(|event| event["node_id"].as_str().unwrap().to_string())
        .collect();

    assert_eq!(timeline_body["query_type"], "timeline");
    assert_eq!(timeline_body["total"], 3);
    assert_eq!(timeline_node_ids, expected_ids);
    assert_eq!(timeline_event_ids, expected_ids);

    let path_response = client
        .post(format!("{}/api/v1/graph/query", base))
        .json(&serde_json::json!({
            "query_type": "temporal_path",
            "start_node": first,
            "end_node": third
        }))
        .send()
        .await
        .expect("temporal path request");

    assert!(path_response.status().is_success());
    let path_body: serde_json::Value = path_response.json().await.expect("path body");
    let paths = path_body["paths"].as_array().expect("paths array");
    assert_eq!(path_body["query_type"], "temporal_path");
    assert_eq!(path_body["total"], 1);
    assert_eq!(
        paths[0]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|node| node.as_str().unwrap().to_string())
            .collect::<Vec<_>>(),
        expected_ids
    );
    assert_eq!(
        paths[0]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|edge| edge.as_str().unwrap().to_string())
            .collect::<Vec<_>>(),
        vec!["Before".to_string(), "Before".to_string()]
    );
}

#[tokio::test]
async fn test_graph_query_accepts_legacy_node_id_alias() {
    let (base, client) = spawn_graph_test_app().await;

    let first = create_memory_via_api(&client, &base, "legacy one").await;
    tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
    let second = create_memory_via_api(&client, &base, "legacy two").await;

    let response = client
        .post(format!("{}/api/v1/graph/query", base))
        .json(&serde_json::json!({
            "query_type": "causal_chain",
            "node_id": second,
            "max_depth": 1
        }))
        .send()
        .await
        .expect("legacy alias request");

    assert!(response.status().is_success());
    let body: serde_json::Value = response.json().await.expect("legacy alias body");
    let nodes = body["nodes"].as_array().expect("nodes array");
    assert_eq!(body["query_type"], "causal_chain");
    assert_eq!(body["total"], 2);
    assert_eq!(
        nodes.first().unwrap()["id"].as_str().unwrap(),
        first.as_str()
    );
}

#[tokio::test]
async fn test_graph_query_unknown_node_returns_not_found() {
    let (base, client) = spawn_graph_test_app().await;
    let _ = create_memory_via_api(&client, &base, "known memory").await;

    let response = client
        .post(format!("{}/api/v1/graph/query", base))
        .json(&serde_json::json!({
            "query_type": "temporal_bfs",
            "start_node": "missing-node"
        }))
        .send()
        .await
        .expect("missing node request");

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    let body: serde_json::Value = response.json().await.expect("missing node body");
    assert!(
        body["message"]
            .as_str()
            .unwrap_or_default()
            .contains("Graph node 'missing-node' not found"),
        "unexpected body: {:?}",
        body
    );
}
