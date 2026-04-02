// Phase 17.4: GraphQL API Integration Tests
//
// 测试 GraphQL API 功能

use evif_rest::create_routes;
use evif_core::RadixMountTable;
use std::sync::Arc;

/// P17.4-01: GraphQL Status Query
#[tokio::test]
async fn graphql_status_query() {
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

    for _ in 0..60 {
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    // Query status via GraphQL
    let res = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": "{ status { version status } }"
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "GraphQL query should succeed");
    let json: serde_json::Value = res.json().await.expect("valid JSON");
    assert!(json.get("data").is_some());
    assert!(json["data"]["status"]["version"].is_string());
    assert!(json["data"]["status"]["status"].is_string());
}

/// P17.4-02: GraphQL Health Query
#[tokio::test]
async fn graphql_health_query() {
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

    for _ in 0..60 {
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    let res = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": "{ health }"
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "GraphQL health query should succeed");
    let json: serde_json::Value = res.json().await.expect("valid JSON");
    assert_eq!(json["data"]["health"], true);
}

/// P17.4-03: GraphQL Echo Mutation
#[tokio::test]
async fn graphql_echo_mutation() {
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

    for _ in 0..60 {
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    let res = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": "mutation { echo(message: \"hello evif\") }"
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "GraphQL mutation should succeed");
    let json: serde_json::Value = res.json().await.expect("valid JSON");
    assert_eq!(json["data"]["echo"], "hello evif");
}

/// P17.4-04: GraphQL GraphiQL IDE Endpoint
#[tokio::test]
async fn graphql_graphiql_endpoint() {
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

    for _ in 0..60 {
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    let res = client
        .get(format!("{}/api/v1/graphql/graphiql", base))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "GraphiQL endpoint should succeed");
    let body = res.text().await.expect("valid text");
    assert!(body.contains("graphiql"), "Should contain GraphiQL HTML");
}
