use axum::{extract::Query, routing::get, Json, Router};
use base64::Engine;
use evif_core::{EvifPlugin, WriteFlags};
use evif_plugins::ProxyFsPlugin;
use serde::Deserialize;

#[derive(Deserialize)]
struct PathQuery {
    #[allow(dead_code)]
    path: String,
}

#[derive(Deserialize)]
struct WriteBody {
    data: String,
    encoding: String,
}

#[tokio::test]
async fn proxyfs_maps_to_evif_rest_contract() {
    async fn read_file(Query(query): Query<PathQuery>) -> Json<serde_json::Value> {
        let _ = query;
        Json(serde_json::json!({
            "content": "hello",
            "data": base64::engine::general_purpose::STANDARD.encode(b"hello"),
            "size": 5
        }))
    }

    async fn write_file(
        Query(query): Query<PathQuery>,
        Json(body): Json<WriteBody>,
    ) -> Json<serde_json::Value> {
        let _ = query;
        assert_eq!(body.encoding, "base64");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(body.data)
            .expect("decode");
        assert_eq!(decoded, b"hello");
        Json(serde_json::json!({
            "bytes_written": 5,
            "path": "/hello.txt"
        }))
    }

    async fn delete_file(Query(query): Query<PathQuery>) -> Json<serde_json::Value> {
        let _ = query;
        Json(serde_json::json!({
            "message": "File deleted",
            "path": "/hello.txt"
        }))
    }

    async fn list_directory(Query(query): Query<PathQuery>) -> Json<serde_json::Value> {
        let _ = query;
        Json(serde_json::json!({
            "path": "/",
            "files": [{
                "id": null,
                "name": "hello.txt",
                "path": "/hello.txt",
                "is_dir": false,
                "size": 5,
                "modified": "2026-01-01T00:00:00Z",
                "created": "2026-01-01T00:00:00Z"
            }]
        }))
    }

    async fn stat_file(Query(query): Query<PathQuery>) -> Json<serde_json::Value> {
        let _ = query;
        Json(serde_json::json!({
            "path": "/hello.txt",
            "size": 5,
            "is_dir": false,
            "modified": "2026-01-01T00:00:00Z",
            "created": "2026-01-01T00:00:00Z"
        }))
    }

    let app = Router::new()
        .route(
            "/api/v1/files",
            get(read_file).put(write_file).delete(delete_file),
        )
        .route("/api/v1/directories", get(list_directory))
        .route("/api/v1/stat", get(stat_file));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });

    let base_url = format!("http://{}/api/v1", addr);
    let plugin = ProxyFsPlugin::new(&base_url);

    let data = plugin.read("/hello.txt", 0, 0).await.expect("read");
    assert_eq!(data, b"hello");

    let entries = plugin.readdir("/").await.expect("readdir");
    assert!(entries.iter().any(|entry| entry.name == "hello.txt"));
    assert!(entries.iter().any(|entry| entry.name == "reload"));

    let info = plugin.stat("/hello.txt").await.expect("stat");
    assert_eq!(info.name, "hello.txt");
    assert_eq!(info.size, 5);

    let written = plugin
        .write("/hello.txt", b"hello".to_vec(), 0, WriteFlags::NONE)
        .await
        .expect("write");
    assert_eq!(written, 5);

    plugin.remove("/hello.txt").await.expect("remove");

    server.abort();
}
