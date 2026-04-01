use evif_core::RadixMountTable;
use evif_rest::create_routes;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::test]
async fn skill_and_pipe_plugins_can_be_mounted_via_rest() {
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

        let _ = client
            .post(format!("{}/api/v1/mount", base))
            .json(&serde_json::json!({ "path": "/skills", "plugin": "skill" }))
            .send()
            .await;
        let _ = client
            .post(format!("{}/api/v1/mount", base))
            .json(&serde_json::json!({ "path": "/pipes", "plugin": "pipe" }))
            .send()
            .await;

        let skill_res = client
            .get(format!("{}/api/v1/files?path=/skills/code-review/SKILL.md", base))
            .send()
            .await;
        let pipe_res = client
            .post(format!("{}/api/v1/directories", base))
            .json(&serde_json::json!({ "path": "/pipes/task-001" }))
            .send()
            .await;

        if let (Ok(skill), Ok(pipe)) = (skill_res, pipe_res) {
            if skill.status().is_success() && pipe.status().is_success() {
                let write_ok = client
                    .put(format!("{}/api/v1/files?path=/pipes/task-001/input", base))
                    .json(&serde_json::json!({ "data": "job payload", "encoding": null }))
                    .send()
                    .await
                    .ok();
                let status_ok = client
                    .get(format!("{}/api/v1/files?path=/pipes/task-001/status", base))
                    .send()
                    .await
                    .ok();

                if let (Some(write_ok), Some(status_ok)) = (write_ok, status_ok) {
                    if write_ok.status().is_success() && status_ok.status().is_success() {
                        let json: serde_json::Value = status_ok.json().await.unwrap();
                        if json["content"].as_str() == Some("running") {
                            return;
                        }
                    }
                }
            }
        }

        if attempt == 79 {
            panic!("skillfs/pipefs REST contract did not succeed in time");
        }
    }
}
