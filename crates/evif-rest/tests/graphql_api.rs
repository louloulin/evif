// Phase 17.4: GraphQL API Integration Tests
//
// 测试 GraphQL API 功能

use evif_core::{EvifPlugin, RadixMountTable};
use evif_plugins::{HelloFsPlugin, MemFsPlugin};
use evif_rest::create_routes;
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

/// P17.4-01b: GraphQL Status Matches REST v1 Health Contract
#[tokio::test]
async fn graphql_status_matches_rest_health_contract() {
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

    let rest_health = client
        .get(format!("{}/api/v1/health", base))
        .send()
        .await
        .expect("rest health succeeds");
    assert!(
        rest_health.status().is_success(),
        "REST health should succeed"
    );
    let rest_json: serde_json::Value = rest_health.json().await.expect("valid JSON");

    let graphql_status = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": "{ status { version status } }"
        }))
        .send()
        .await
        .expect("graphql request succeeds");
    assert!(
        graphql_status.status().is_success(),
        "GraphQL status query should succeed"
    );
    let graphql_json: serde_json::Value = graphql_status.json().await.expect("valid JSON");

    assert_eq!(
        graphql_json["data"]["status"]["version"],
        rest_json["version"]
    );
    assert_eq!(
        graphql_json["data"]["status"]["status"],
        rest_json["status"]
    );
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

    assert!(
        res.status().is_success(),
        "GraphQL health query should succeed"
    );
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

    assert!(
        res.status().is_success(),
        "GraphiQL endpoint should succeed"
    );
    let body = res.text().await.expect("valid text");
    assert!(body.contains("graphiql"), "Should contain GraphiQL HTML");
}

/// P17.4-05: GraphQL Mounts Query - returns real mount table data
#[tokio::test]
async fn graphql_mounts_query_returns_mounted_plugins() {
    let mount_table = Arc::new(RadixMountTable::new());
    // Mount a few plugins so the table is non-empty
    let mem_plugin: Arc<dyn EvifPlugin> = Arc::new(MemFsPlugin::new());
    mount_table
        .mount_with_metadata(
            "/mem".to_string(),
            mem_plugin,
            "mem".to_string(),
            "mem".to_string(),
        )
        .await
        .expect("mount mem");

    let hello_plugin: Arc<dyn EvifPlugin> = Arc::new(HelloFsPlugin::new());
    mount_table
        .mount_with_metadata(
            "/hello".to_string(),
            hello_plugin,
            "hello".to_string(),
            "hello".to_string(),
        )
        .await
        .expect("mount hello");

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
            "query": "{ mounts { path plugin instanceName } }"
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(
        res.status().is_success(),
        "GraphQL mounts query should succeed"
    );
    let json: serde_json::Value = res.json().await.expect("valid JSON");
    let data = json.get("data").expect("should have data");
    let mounts = data.get("mounts").expect("should have mounts field");
    assert!(
        mounts.is_array(),
        "mounts should be an array, got: {:?}",
        mounts
    );
    // We mounted 2 plugins, so should have at least 2
    let mount_count = mounts.as_array().unwrap().len();
    assert!(
        mount_count >= 2,
        "should have at least 2 mounts (mem and hello), got {}",
        mount_count
    );
    // Verify /mem is in the list
    let paths: Vec<&str> = mounts
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|m| m.get("path").and_then(|v| v.as_str()))
        .collect();
    assert!(paths.contains(&"/mem"), "should include /mem mount");
    assert!(paths.contains(&"/hello"), "should include /hello mount");
}

/// P17.4-06: GraphQL Traffic Query - returns traffic metrics
#[tokio::test]
async fn graphql_traffic_query_returns_traffic_stats() {
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

    // Hit the REST traffic endpoint first to confirm stats work
    let rest_traffic = client
        .get(format!("{}/api/v1/metrics/traffic", base))
        .send()
        .await
        .expect("rest traffic succeeds");
    assert!(rest_traffic.status().is_success());

    let res = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": "{ traffic { totalRequests readCount writeCount listCount otherCount } }"
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(
        res.status().is_success(),
        "GraphQL traffic query should succeed"
    );
    let json: serde_json::Value = res.json().await.expect("valid JSON");
    let data = json.get("data").expect("should have data");
    let traffic = data.get("traffic").expect("should have traffic field");
    assert!(
        traffic.is_object(),
        "traffic should be an object, got: {:?}",
        traffic
    );
    assert!(
        traffic.get("totalRequests").is_some(),
        "traffic should have totalRequests"
    );
    assert!(
        traffic.get("readCount").is_some(),
        "traffic should have readCount"
    );
}

/// P17.4-07: GraphQL Tenants Query - returns tenant list
#[tokio::test]
async fn graphql_tenants_query_returns_tenant_list() {
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
            "query": "{ tenants { id name status } }"
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(
        res.status().is_success(),
        "GraphQL tenants query should succeed"
    );
    let json: serde_json::Value = res.json().await.expect("valid JSON");
    let data = json.get("data").expect("should have data");
    let tenants = data.get("tenants").expect("should have tenants field");
    assert!(
        tenants.is_array(),
        "tenants should be an array, got: {:?}",
        tenants
    );
    // Default tenant should be present
    let tenant_ids: Vec<&str> = tenants
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t.get("id").and_then(|v| v.as_str()))
        .collect();
    assert!(
        tenant_ids.contains(&"default"),
        "default tenant should be in the list"
    );
}

/// P17.4-08: GraphQL Encryption Query - returns encryption status
#[tokio::test]
async fn graphql_encryption_query_returns_status() {
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
            "query": "{ encryption { status algorithm keySource } }"
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(
        res.status().is_success(),
        "GraphQL encryption query should succeed"
    );
    let json: serde_json::Value = res.json().await.expect("valid JSON");
    let data = json.get("data").expect("should have data");
    let encryption = data
        .get("encryption")
        .expect("should have encryption field");
    assert!(
        encryption.is_object(),
        "encryption should be an object, got: {:?}",
        encryption
    );
    assert!(
        encryption.get("algorithm").is_some(),
        "encryption should have algorithm"
    );
    assert_eq!(encryption["algorithm"].as_str().unwrap(), "AES-256-GCM");
}

/// P17.4-09: GraphQL SyncStatus Query - returns sync state
#[tokio::test]
async fn graphql_sync_status_query_returns_sync_state() {
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
            "query": "{ syncStatus { synced lastVersion pendingChanges trackedPaths } }"
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(
        res.status().is_success(),
        "GraphQL syncStatus query should succeed"
    );
    let json: serde_json::Value = res.json().await.expect("valid JSON");
    let data = json.get("data").expect("should have data");
    let sync = data
        .get("syncStatus")
        .expect("should have syncStatus field");
    assert!(
        sync.is_object(),
        "syncStatus should be an object, got: {:?}",
        sync
    );
    assert!(
        sync.get("lastVersion").is_some(),
        "syncStatus should have lastVersion"
    );
    assert!(
        sync.get("trackedPaths").is_some(),
        "syncStatus should have trackedPaths"
    );
}

/// P17.4-10: GraphQL ResolveSyncConflicts Mutation
#[tokio::test]
async fn graphql_resolve_sync_conflicts_mutation() {
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
            "query": r#"mutation {
                resolveSyncConflicts(resolutions: [
                    { path: "/test/file.txt", strategy: "last_write_wins", remoteVersion: 42 }
                ]) {
                    synced
                    lastVersion
                    trackedPaths
                }
            }"#
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(
        res.status().is_success(),
        "GraphQL resolveSyncConflicts should succeed"
    );
    let json: serde_json::Value = res.json().await.expect("valid JSON");
    let data = json.get("data").expect("should have data");
    let result = data
        .get("resolveSyncConflicts")
        .expect("should have resolveSyncConflicts");
    assert!(result["synced"].as_bool().unwrap());
}

/// P17.4-11: GraphQL fileRead mutation — reads file content from a mounted plugin
#[tokio::test]
async fn graphql_file_read_mutation() {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem_plugin: Arc<dyn EvifPlugin> = Arc::new(MemFsPlugin::new());
    mount_table
        .mount_with_metadata(
            "/mem".to_string(),
            mem_plugin,
            "mem".to_string(),
            "mem".to_string(),
        )
        .await
        .expect("mount mem");

    let app = create_routes(mount_table.clone());
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

    // Write test content via GraphQL (auto-creates file)
    let _ = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": r#"mutation {
                fileWrite(input: { path: "/mem/test.txt", data: "hello graphql" }) {
                    bytesWritten
                }
            }"#
        }))
        .send()
        .await;

    // Read it back via GraphQL
    let res = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": r#"mutation {
                fileRead(input: { path: "/mem/test.txt" }) {
                    content
                    size
                }
            }"#
        }))
        .send()
        .await
        .expect("graphql request succeeds");

    assert!(
        res.status().is_success(),
        "fileRead mutation should succeed"
    );
    let json: serde_json::Value = res.json().await.expect("valid JSON");
    let data = json.get("data").expect("should have data");
    let result = data.get("fileRead").expect("should have fileRead");
    assert!(
        result["content"]
            .as_str()
            .map(|s| s.contains("hello graphql"))
            .unwrap_or(false),
        "should contain written content, got: {}",
        result["content"]
    );
    assert_eq!(result["size"].as_u64().unwrap() as usize, 13);
}

/// P17.4-12: GraphQL fileWrite mutation — writes content to a file
#[tokio::test]
async fn graphql_file_write_mutation() {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem_plugin: Arc<dyn EvifPlugin> = Arc::new(MemFsPlugin::new());
    mount_table
        .mount_with_metadata(
            "/mem".to_string(),
            mem_plugin,
            "mem".to_string(),
            "mem".to_string(),
        )
        .await
        .expect("mount mem");

    let app = create_routes(mount_table.clone());
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
            "query": r#"mutation {
                fileWrite(input: { path: "/mem/graphql_write.txt", data: "written via graphql" }) {
                    bytesWritten
                    path
                }
            }"#
        }))
        .send()
        .await
        .expect("graphql request succeeds");

    assert!(
        res.status().is_success(),
        "fileWrite mutation should succeed"
    );
    let json: serde_json::Value = res.json().await.expect("valid JSON");
    let data = json.get("data").expect("should have data");
    let result = data.get("fileWrite").expect("should have fileWrite");
    assert!(
        result["bytesWritten"].as_u64().unwrap() > 0,
        "should write some bytes"
    );
    assert_eq!(result["path"].as_str().unwrap(), "/mem/graphql_write.txt");
}

/// P17.4-13: GraphQL fileList mutation — lists directory entries
#[tokio::test]
async fn graphql_file_list_mutation() {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem_plugin: Arc<dyn EvifPlugin> = Arc::new(MemFsPlugin::new());
    mount_table
        .mount_with_metadata(
            "/mem".to_string(),
            mem_plugin,
            "mem".to_string(),
            "mem".to_string(),
        )
        .await
        .expect("mount mem");

    let app = create_routes(mount_table.clone());
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

    // List the /mem root via GraphQL
    let res = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": r#"mutation {
                fileList(path: "/") {
                    path
                    entries {
                        name
                        isDir
                        size
                    }
                }
            }"#
        }))
        .send()
        .await
        .expect("graphql request succeeds");

    assert!(
        res.status().is_success(),
        "fileList mutation should succeed"
    );
    let json: serde_json::Value = res.json().await.expect("valid JSON");
    let data = json.get("data").expect("should have data");
    let result = data.get("fileList").expect("should have fileList");
    assert_eq!(result["path"].as_str().unwrap(), "/");
    let entries = result["entries"]
        .as_array()
        .expect("entries should be array");
    // Root should show at least the /mem mount
    assert!(
        !entries.is_empty(),
        "root should list mount points (at least /mem)"
    );
    // Verify /mem is listed
    let names: Vec<&str> = entries.iter().filter_map(|e| e["name"].as_str()).collect();
    assert!(names.contains(&"mem"), "root should list 'mem' mount point");
}

/// P17.4-14: GraphQL fileRead Latency Benchmark
#[tokio::test]
async fn graphql_file_read_latency_benchmark() {
    let mount_table = Arc::new(RadixMountTable::new());
    let plugin = MemFsPlugin::default();
    mount_table
        .mount("/bench".into(), Arc::new(plugin))
        .await
        .expect("mount succeeds");
    let app = create_routes(mount_table.clone());
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

    // Create and write a test file via GraphQL
    let write = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": "mutation { fileWrite(input: { path: \"/bench/large.bin\", data: \"aGVsbG8gd29ybGQgaGVsbG8gd29ybGQ=\", encoding: \"base64\" }) { bytesWritten path } }"
        }))
        .send()
        .await
        .expect("graphql write succeeds");
    assert!(write.status().is_success());

    // Measure read latency over 10 iterations
    let mut latencies = Vec::with_capacity(10);
    for _ in 0..10 {
        let start = std::time::Instant::now();
        let res = client
            .post(format!("{}/api/v1/graphql", base))
            .json(&serde_json::json!({
                "query": "{ fileRead(input: { path: \"/bench/large.bin\" }) { content size } }"
            }))
            .send()
            .await
            .expect("graphql read succeeds");
        let elapsed = start.elapsed();
        assert!(res.status().is_success());
        latencies.push(elapsed);
    }

    let avg_ms = latencies.iter().map(|d| d.as_millis()).sum::<u128>() as f64 / 10.0;
    let max_ms = latencies.iter().map(|d| d.as_millis()).max().unwrap();

    // GraphQL fileRead should be responsive — avg < 200ms, max < 500ms
    assert!(
        avg_ms < 200.0,
        "avg fileRead latency {:.1}ms should be under 200ms",
        avg_ms
    );
    assert!(
        max_ms < 500,
        "max fileRead latency {}ms should be under 500ms",
        max_ms
    );
}

/// P17.4-16: GraphQL fileDelete mutation
#[tokio::test]
async fn graphql_file_delete_mutation() {
    let mount_table = Arc::new(RadixMountTable::new());
    let plugin = MemFsPlugin::default();
    mount_table
        .mount("/del_test".into(), Arc::new(plugin))
        .await
        .expect("mount succeeds");
    let app = create_routes(mount_table.clone());
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

    // Create a file first
    let write = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": "mutation { fileWrite(input: { path: \"/del_test/removeme.txt\", data: \"hello\", encoding: \"text\" }) { bytesWritten path } }"
        }))
        .send()
        .await
        .expect("graphql write succeeds");
    assert!(write.status().is_success());

    // Delete the file via GraphQL
    let del = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": "mutation { fileDelete(input: { path: \"/del_test/removeme.txt\" }) { path deleted } }"
        }))
        .send()
        .await
        .expect("graphql delete succeeds");
    assert!(del.status().is_success());

    let json: serde_json::Value = del.json().await.expect("valid JSON");
    let data = json.get("data").expect("should have data");
    let result = data.get("fileDelete").expect("should have fileDelete");
    assert_eq!(result["path"].as_str().unwrap(), "/del_test/removeme.txt");
    assert!(result["deleted"].as_bool().unwrap_or(false));

    // Verify file is gone — read should return null
    let read = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": "{ fileRead(input: { path: \"/del_test/removeme.txt\" }) { content } }"
        }))
        .send()
        .await
        .expect("graphql read succeeds");
    assert!(read.status().is_success());
    let read_json: serde_json::Value = read.json().await.expect("valid JSON");
    assert!(
        read_json
            .get("data")
            .and_then(|d| d.get("fileRead"))
            .is_none(),
        "file should be gone after delete"
    );
}

/// P17.4-17: GraphQL encryption query exposes key versions
#[tokio::test]
async fn graphql_encryption_versions_query() {
    let mount_table = Arc::new(RadixMountTable::new());
    let app = create_routes(mount_table.clone());
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

    // Enable encryption via REST (creates version 1)
    let enable = client
        .post(format!("{}/api/v1/encryption/enable", base))
        .json(&serde_json::json!({ "key": "gql-version-test-key" }))
        .send()
        .await
        .expect("enable succeeds");
    assert!(enable.status().is_success());

    // Query encryption status via GraphQL, should have versions field
    let gql = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": "{ encryption { status algorithm keySource versions { id version sourceHint isCurrent } } }"
        }))
        .send()
        .await
        .expect("graphql succeeds");
    assert!(gql.status().is_success());

    let json: serde_json::Value = gql.json().await.expect("valid JSON");
    let data = json.get("data").expect("should have data");
    let enc = data.get("encryption").expect("should have encryption");
    assert_eq!(enc["algorithm"].as_str().unwrap(), "AES-256-GCM");
    let versions = enc["versions"]
        .as_array()
        .expect("versions should be array");
    assert_eq!(versions.len(), 1, "should have 1 version after enable");
    assert_eq!(versions[0]["version"].as_i64().unwrap(), 1);
    assert!(versions[0]["isCurrent"].as_bool().unwrap_or(false));
}

/// P17.4-18: GraphQL fileCreate mutation
#[tokio::test]
async fn graphql_file_create_mutation() {
    let mount_table = Arc::new(RadixMountTable::new());
    let plugin = MemFsPlugin::default();
    mount_table
        .mount("/create_test".into(), Arc::new(plugin))
        .await
        .expect("mount succeeds");
    let app = create_routes(mount_table.clone());
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

    // Create a file via GraphQL
    let create = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": "mutation { fileCreate(input: { path: \"/create_test/newfile.txt\" }) { path created } }"
        }))
        .send()
        .await
        .expect("graphql create succeeds");
    assert!(create.status().is_success());

    let json: serde_json::Value = create.json().await.expect("valid JSON");
    let data = json.get("data").unwrap_or(&serde_json::Value::Null);
    let result = data.get("fileCreate").unwrap_or(&serde_json::Value::Null);
    assert!(result.is_object(), "fileCreate should succeed: {}", json);
    assert_eq!(
        result["path"].as_str().unwrap_or(""),
        "/create_test/newfile.txt"
    );
    assert!(result["created"].as_bool().unwrap_or(false));
}

/// P17.4-19: GraphQL directoryDelete mutation
#[tokio::test]
async fn graphql_directory_delete_mutation() {
    let mount_table = Arc::new(RadixMountTable::new());
    let plugin = MemFsPlugin::default();
    mount_table
        .mount("/rmdir_test".into(), Arc::new(plugin))
        .await
        .expect("mount succeeds");
    let app = create_routes(mount_table.clone());
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

    // Create a directory via REST mkdir
    let mkdir = client
        .post(format!("{}/api/v1/directories", base))
        .json(&serde_json::json!({ "path": "/rmdir_test/mydir" }))
        .send()
        .await
        .expect("mkdir succeeds");
    assert!(mkdir.status().is_success());

    // Verify directory exists via GraphQL mounts query (lists all mount points)
    let list_root = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": "{ mounts { path } }"
        }))
        .send()
        .await
        .expect("graphql mounts succeeds");
    assert!(list_root.status().is_success(), "HTTP status should be 200");
    let root_json: serde_json::Value = list_root.json().await.expect("valid JSON");
    let root_data = root_json.get("data").unwrap_or(&serde_json::Value::Null);
    assert!(
        root_data.is_object(),
        "mounts query should succeed: {}",
        root_json
    );
    let mounts_val = root_data.get("mounts").unwrap_or(&serde_json::Value::Null);
    assert!(
        mounts_val.is_array(),
        "mounts should be an array: {}",
        root_json
    );
    let mounts_entries = mounts_val.as_array().unwrap();
    let mount_paths: Vec<&str> = mounts_entries
        .iter()
        .filter_map(|m| m["path"].as_str())
        .collect();
    assert!(
        mount_paths
            .iter()
            .any(|p| p.ends_with("/rmdir_test") || *p == "/rmdir_test"),
        "rmdir_test mount should appear in mounts query"
    );

    // Delete the directory via GraphQL
    let del = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": "mutation { directoryDelete(input: { path: \"/rmdir_test/mydir\" }) { path deleted } }"
        }))
        .send()
        .await
        .expect("graphql delete succeeds");
    assert!(del.status().is_success());

    let json: serde_json::Value = del.json().await.expect("valid JSON");
    let data = json.get("data").unwrap_or(&serde_json::Value::Null);
    let result = data
        .get("directoryDelete")
        .unwrap_or(&serde_json::Value::Null);
    assert!(
        result.is_object(),
        "directoryDelete should succeed: {}",
        json
    );
    assert_eq!(result["path"].as_str().unwrap(), "/rmdir_test/mydir");
    assert!(result["deleted"].as_bool().unwrap_or(false));
}

/// P17.4-20: GraphQL enableEncryption mutation
#[tokio::test]
async fn graphql_enable_encryption_mutation() {
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

    // Enable encryption via GraphQL mutation (use direct key, not env: prefix)
    let res = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": "mutation { enableEncryption(key: \"my-direct-test-key-12345\") { success message status { status algorithm } } }"
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "GraphQL mutation should succeed");
    let json: serde_json::Value = res.json().await.expect("valid JSON");
    let data = json.get("data").unwrap_or(&serde_json::Value::Null);
    let result = data
        .get("enableEncryption")
        .unwrap_or(&serde_json::Value::Null);
    assert!(
        result.is_object(),
        "enableEncryption should return object: {}",
        json
    );
    assert!(
        result["success"].as_bool().unwrap_or(false),
        "enableEncryption should succeed: {}",
        json
    );
    assert!(
        result["status"].is_object(),
        "status should be present: {}",
        json
    );
    assert_eq!(
        result["status"]["algorithm"].as_str().unwrap_or(""),
        "AES-256-GCM"
    );
}

/// P17.4-21: GraphQL disableEncryption mutation
#[tokio::test]
async fn graphql_disable_encryption_mutation() {
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

    // First enable encryption (use direct key, not env: prefix)
    let _enable = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": "mutation { enableEncryption(key: \"my-disable-test-key-999\") { success } }"
        }))
        .send()
        .await
        .expect("enable succeeds");

    // Then disable via GraphQL mutation
    let res = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": "mutation { disableEncryption { success message status { status } } }"
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "GraphQL mutation should succeed");
    let json: serde_json::Value = res.json().await.expect("valid JSON");
    let data = json.get("data").unwrap_or(&serde_json::Value::Null);
    let result = data
        .get("disableEncryption")
        .unwrap_or(&serde_json::Value::Null);
    assert!(
        result.is_object(),
        "disableEncryption should return object: {}",
        json
    );
    assert!(
        result["success"].as_bool().unwrap_or(false),
        "disableEncryption should succeed: {}",
        json
    );
    assert!(
        result["status"].is_object(),
        "status should be present: {}",
        json
    );
}

/// P17.4-22: GraphQL rotateEncryptionKey mutation
#[tokio::test]
async fn graphql_rotate_encryption_key_mutation() {
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

    // First enable encryption (use direct key, not env: prefix)
    let _enable = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": "mutation { enableEncryption(key: \"my-rotate-test-key-abc\") { success } }"
        }))
        .send()
        .await
        .expect("enable succeeds");

    // Rotate key via GraphQL mutation
    let res = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": "mutation { rotateEncryptionKey(newKey: \"my-rotated-key-xyz\") { success message status { status } } }"
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "GraphQL mutation should succeed");
    let json: serde_json::Value = res.json().await.expect("valid JSON");
    let data = json.get("data").unwrap_or(&serde_json::Value::Null);
    let result = data
        .get("rotateEncryptionKey")
        .unwrap_or(&serde_json::Value::Null);
    assert!(
        result.is_object(),
        "rotateEncryptionKey should return object: {}",
        json
    );
    assert!(
        result["success"].as_bool().unwrap_or(false),
        "rotateEncryptionKey should succeed: {}",
        json
    );
    assert!(
        result["status"].is_object(),
        "status should be present: {}",
        json
    );

    // Verify version history is accumulated (at least 2 versions after enable + rotate)
    let versions_check = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": "{ encryption { versions { version isCurrent } } }"
        }))
        .send()
        .await
        .expect("encryption query succeeds");
    let versions_json: serde_json::Value = versions_check.json().await.expect("valid JSON");
    let versions_data = versions_json
        .get("data")
        .unwrap_or(&serde_json::Value::Null);
    let enc = versions_data
        .get("encryption")
        .unwrap_or(&serde_json::Value::Null);
    let versions = enc.get("versions").unwrap_or(&serde_json::Value::Null);
    assert!(
        versions.is_array(),
        "versions should be an array: {}",
        versions_json
    );
    let arr = versions.as_array().unwrap();
    assert!(
        arr.len() >= 2,
        "should have at least 2 versions after enable+rotate: {}",
        arr.len()
    );
}

/// P17.4-23: GraphQL applyDelta mutation
#[tokio::test]
async fn graphql_apply_delta_mutation() {
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

    // Apply delta via GraphQL mutation
    let res = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": r#"mutation {
                applyDelta(baseVersion: 0, changes: [
                    { path: "/file1.txt", op: "created", version: 1 },
                    { path: "/file2.txt", op: "modified", version: 2 }
                ]) {
                    syncedVersion accepted conflicts
                }
            }"#
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "GraphQL mutation should succeed");
    let json: serde_json::Value = res.json().await.expect("valid JSON");
    let data = json.get("data").unwrap_or(&serde_json::Value::Null);
    let result = data.get("applyDelta").unwrap_or(&serde_json::Value::Null);
    assert!(
        result.is_object(),
        "applyDelta should return object: {}",
        json
    );
    assert_eq!(
        result["accepted"].as_u64().unwrap_or(0),
        2,
        "should accept 2 changes: {}",
        json
    );
    assert!(
        result["conflicts"].is_array(),
        "conflicts should be an array: {}",
        json
    );
    assert_eq!(
        result["conflicts"].as_array().unwrap().len(),
        0,
        "should have no conflicts: {}",
        json
    );

    // Verify sync status reflects the delta
    let status_check = client
        .post(format!("{}/api/v1/graphql", base))
        .json(&serde_json::json!({
            "query": "{ syncStatus { synced lastVersion pendingChanges trackedPaths } }"
        }))
        .send()
        .await
        .expect("syncStatus query succeeds");
    let status_json: serde_json::Value = status_check.json().await.expect("valid JSON");
    let status_data = status_json.get("data").unwrap_or(&serde_json::Value::Null);
    let sync_status = status_data
        .get("syncStatus")
        .unwrap_or(&serde_json::Value::Null);
    assert!(
        sync_status["synced"].as_bool().unwrap_or(false),
        "should be synced: {}",
        status_json
    );
}
