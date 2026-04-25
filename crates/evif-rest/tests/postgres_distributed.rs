use evif_core::RadixMountTable;
use evif_rest::{
    create_memory_state_from_config, create_routes_with_memory_state, MemoryBackendConfig,
};
use std::net::TcpListener as StdTcpListener;
use std::process::Command;
use std::sync::Arc;
use tokio::task::JoinHandle;

struct TestPostgresInstance {
    data_dir: tempfile::TempDir,
    port: u16,
}

impl TestPostgresInstance {
    fn start() -> Result<Self, String> {
        let data_dir = tempfile::tempdir()
            .map_err(|err| format!("failed to create temp postgres dir: {err}"))?;
        let cluster_dir = data_dir.path().join("cluster");
        let socket_dir = data_dir.path().join("socket");
        std::fs::create_dir_all(&cluster_dir)
            .map_err(|err| format!("failed to create postgres cluster dir: {err}"))?;
        std::fs::create_dir_all(&socket_dir)
            .map_err(|err| format!("failed to create postgres socket dir: {err}"))?;
        let cluster_dir_str = cluster_dir.to_string_lossy().into_owned();

        let init_output = Command::new("initdb")
            .args(["-D", &cluster_dir_str, "-A", "trust", "-U", "postgres"])
            .output()
            .map_err(|err| format!("failed to spawn initdb: {err}"))?;
        if !init_output.status.success() {
            return Err(format!(
                "initdb failed: {}",
                String::from_utf8_lossy(&init_output.stderr)
            ));
        }

        let port = StdTcpListener::bind("127.0.0.1:0")
            .map_err(|err| format!("failed to allocate postgres port: {err}"))?
            .local_addr()
            .map_err(|err| format!("failed to read allocated port: {err}"))?
            .port();

        let log_path = data_dir.path().join("postgres.log");
        let start_output = Command::new("pg_ctl")
            .args([
                "-D",
                &cluster_dir_str,
                "-l",
                log_path.to_string_lossy().as_ref(),
                "-o",
                &format!("-F -p {port} -k {}", socket_dir.display()),
                "-w",
                "start",
            ])
            .output()
            .map_err(|err| format!("failed to spawn pg_ctl start: {err}"))?;
        if !start_output.status.success() {
            return Err(format!(
                "pg_ctl start failed: {}",
                String::from_utf8_lossy(&start_output.stderr)
            ));
        }

        Ok(Self { data_dir, port })
    }

    fn cluster_dir(&self) -> std::path::PathBuf {
        self.data_dir.path().join("cluster")
    }

    fn socket_dir(&self) -> std::path::PathBuf {
        self.data_dir.path().join("socket")
    }

    fn connection_string(&self) -> String {
        format!("postgres://postgres@127.0.0.1:{}/postgres", self.port)
    }

    fn stop(&self) {
        let output = Command::new("pg_ctl")
            .args([
                "-D",
                self.cluster_dir().to_string_lossy().as_ref(),
                "-m",
                "fast",
                "-w",
                "stop",
            ])
            .output()
            .expect("spawn pg_ctl stop");
        assert!(
            output.status.success(),
            "pg_ctl stop failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn restart(&self) {
        let log_path = self.data_dir.path().join("postgres-restart.log");
        let output = Command::new("pg_ctl")
            .args([
                "-D",
                self.cluster_dir().to_string_lossy().as_ref(),
                "-l",
                log_path.to_string_lossy().as_ref(),
                "-o",
                &format!("-F -p {} -k {}", self.port, self.socket_dir().display()),
                "-w",
                "start",
            ])
            .output()
            .expect("spawn pg_ctl start");
        assert!(
            output.status.success(),
            "pg_ctl restart failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

impl Drop for TestPostgresInstance {
    fn drop(&mut self) {
        let cluster_dir = self.data_dir.path().join("cluster");
        let _ = Command::new("pg_ctl")
            .args([
                "-D",
                cluster_dir.to_string_lossy().as_ref(),
                "-m",
                "fast",
                "-w",
                "stop",
            ])
            .output();
    }
}

struct TestNode {
    base: String,
    client: reqwest::Client,
    handle: JoinHandle<()>,
}

impl TestNode {
    fn abort(self) {
        self.handle.abort();
    }
}

async fn spawn_app(memory_state: evif_rest::MemoryState) -> TestNode {
    let app = create_routes_with_memory_state(Arc::new(RadixMountTable::new()), memory_state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let handle = tokio::spawn(async move {
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

    TestNode {
        base,
        client,
        handle,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn postgres_memory_backend_supports_three_nodes_with_bounded_pool() {
    let postgres = TestPostgresInstance::start().unwrap();
    let config = MemoryBackendConfig::postgres_with_options(postgres.connection_string(), 2, 1);

    assert_eq!(config.postgres_max_connections(), Some(2));
    assert_eq!(config.postgres_min_connections(), Some(1));

    let node1 = spawn_app(create_memory_state_from_config(&config).await.unwrap()).await;
    let node2 = spawn_app(create_memory_state_from_config(&config).await.unwrap()).await;
    let node3 = spawn_app(create_memory_state_from_config(&config).await.unwrap()).await;

    let payloads = [
        ("node-1 memory", &node1),
        ("node-2 memory", &node2),
        ("node-3 memory", &node3),
    ];

    for (content, node) in payloads {
        let response = node
            .client
            .post(format!("{}/api/v1/memories", node.base))
            .json(&serde_json::json!({
                "content": content,
                "modality": "text"
            }))
            .send()
            .await
            .expect("create memory request succeeds");
        assert!(
            response.status().is_success(),
            "create memory should succeed"
        );
    }

    let response = node2
        .client
        .get(format!("{}/api/v1/memories", node2.base))
        .send()
        .await
        .expect("list memories succeeds");
    assert!(
        response.status().is_success(),
        "list memories should succeed"
    );

    let memories: Vec<serde_json::Value> = response.json().await.expect("valid memory list");
    let contents = memories
        .iter()
        .filter_map(|item| item.get("content").and_then(|value| value.as_str()))
        .collect::<Vec<_>>();

    assert!(contents.contains(&"node-1 memory"));
    assert!(contents.contains(&"node-2 memory"));
    assert!(contents.contains(&"node-3 memory"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn postgres_memory_backend_preserves_writes_under_concurrent_three_node_load() {
    let postgres = TestPostgresInstance::start().unwrap();
    let config = MemoryBackendConfig::postgres_with_options(postgres.connection_string(), 6, 1);

    let node1 = spawn_app(create_memory_state_from_config(&config).await.unwrap()).await;
    let node2 = spawn_app(create_memory_state_from_config(&config).await.unwrap()).await;
    let node3 = spawn_app(create_memory_state_from_config(&config).await.unwrap()).await;
    let nodes = [node1, node2, node3];

    let requests_per_node = 10usize;
    let mut tasks = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        for request_index in 0..requests_per_node {
            let client = node.client.clone();
            let base = node.base.clone();
            tasks.push(tokio::spawn(async move {
                let content = format!("node-{}-load-{}", index + 1, request_index);
                let response = client
                    .post(format!("{}/api/v1/memories", base))
                    .json(&serde_json::json!({
                        "content": content,
                        "modality": "text"
                    }))
                    .send()
                    .await
                    .expect("concurrent create request succeeds");
                assert!(
                    response.status().is_success(),
                    "concurrent create should succeed"
                );
            }));
        }
    }

    for task in tasks {
        task.await.unwrap();
    }

    let response = nodes[1]
        .client
        .get(format!("{}/api/v1/memories", nodes[1].base))
        .send()
        .await
        .expect("list memories succeeds");
    assert!(response.status().is_success(), "list memories should succeed");

    let memories: Vec<serde_json::Value> = response.json().await.expect("valid memory list");
    assert_eq!(
        memories.len(),
        nodes.len() * requests_per_node,
        "concurrent three-node writes should not lose data"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn postgres_memory_backend_survives_rest_node_restart() {
    let postgres = TestPostgresInstance::start().unwrap();
    let config = MemoryBackendConfig::postgres_with_options(postgres.connection_string(), 4, 1);

    let node1 = spawn_app(create_memory_state_from_config(&config).await.unwrap()).await;
    let node2 = spawn_app(create_memory_state_from_config(&config).await.unwrap()).await;

    let response = node1
        .client
        .post(format!("{}/api/v1/memories", node1.base))
        .json(&serde_json::json!({
            "content": "before-node-restart",
            "modality": "text"
        }))
        .send()
        .await
        .expect("initial write succeeds");
    assert!(response.status().is_success());

    node1.abort();

    let replacement = spawn_app(create_memory_state_from_config(&config).await.unwrap()).await;
    let list_response = replacement
        .client
        .get(format!("{}/api/v1/memories", replacement.base))
        .send()
        .await
        .expect("replacement node can list memories");
    assert!(list_response.status().is_success());
    let memories: Vec<serde_json::Value> =
        list_response.json().await.expect("valid memory list");
    assert!(
        memories
            .iter()
            .any(|item| item.get("content").and_then(|value| value.as_str())
                == Some("before-node-restart")),
        "replacement node should read data written before node restart"
    );

    let response = replacement
        .client
        .post(format!("{}/api/v1/memories", replacement.base))
        .json(&serde_json::json!({
            "content": "after-node-restart",
            "modality": "text"
        }))
        .send()
        .await
        .expect("replacement write succeeds");
    assert!(response.status().is_success());

    let list_response = node2
        .client
        .get(format!("{}/api/v1/memories", node2.base))
        .send()
        .await
        .expect("surviving node can list memories");
    assert!(list_response.status().is_success());
    let memories: Vec<serde_json::Value> =
        list_response.json().await.expect("valid memory list");
    let contents = memories
        .iter()
        .filter_map(|item| item.get("content").and_then(|value| value.as_str()))
        .collect::<Vec<_>>();

    assert!(contents.contains(&"before-node-restart"));
    assert!(contents.contains(&"after-node-restart"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn postgres_memory_backend_recovers_after_database_restart() {
    let postgres = TestPostgresInstance::start().unwrap();
    let config = MemoryBackendConfig::postgres_with_options(postgres.connection_string(), 4, 1);
    let node = spawn_app(create_memory_state_from_config(&config).await.unwrap()).await;

    let response = node
        .client
        .post(format!("{}/api/v1/memories", node.base))
        .json(&serde_json::json!({
            "content": "before-db-restart",
            "modality": "text"
        }))
        .send()
        .await
        .expect("initial write succeeds");
    assert!(response.status().is_success());

    postgres.stop();
    postgres.restart();

    let response = node
        .client
        .post(format!("{}/api/v1/memories", node.base))
        .json(&serde_json::json!({
            "content": "after-db-restart",
            "modality": "text"
        }))
        .send()
        .await
        .expect("write after database restart succeeds");
    assert!(
        response.status().is_success(),
        "existing REST node should recover after database restart"
    );

    let response = node
        .client
        .get(format!("{}/api/v1/memories", node.base))
        .send()
        .await
        .expect("list memories succeeds");
    assert!(response.status().is_success());
    let memories: Vec<serde_json::Value> = response.json().await.expect("valid memory list");
    let contents = memories
        .iter()
        .filter_map(|item| item.get("content").and_then(|value| value.as_str()))
        .collect::<Vec<_>>();

    assert!(contents.contains(&"before-db-restart"));
    assert!(contents.contains(&"after-db-restart"));
}
