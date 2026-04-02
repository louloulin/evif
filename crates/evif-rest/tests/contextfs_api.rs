use evif_core::RadixMountTable;
use evif_rest::create_routes;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::test]
async fn context_plugin_can_be_mounted_and_read_via_rest_api() {
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

    for attempt in 0..80 {
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;

        let mount_res = client
            .post(format!("{}/api/v1/mount", base))
            .json(&serde_json::json!({
                "path": "/context",
                "plugin": "context"
            }))
            .send()
            .await;

        if let Ok(res) = mount_res {
            if !res.status().is_success() && res.status().as_u16() != 500 {
                panic!("unexpected mount status: {}", res.status());
            }
        }

        let read_res = client
            .get(format!("{}/api/v1/files?path=/context/L2/architecture.md", base))
            .send()
            .await;

        if let Ok(res) = read_res {
            if res.status().is_success() {
                let json: serde_json::Value = res.json().await.unwrap();
                let content = json["content"].as_str().unwrap_or_default();
                assert!(content.contains("项目架构"));
                return;
            }
        }

        if attempt == 79 {
            panic!("contextfs REST contract did not succeed in time");
        }
    }
}
