// Phase 16.4: LLM Local Model Integration Tests
//
// 测试 LLM 本地模型（Ollama）集成功能

use evif_rest::create_routes;
use evif_core::RadixMountTable;
use std::sync::Arc;

/// P16.4-01: LLM Status Endpoint
#[tokio::test]
async fn llm_status_endpoint() {
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
        if let Ok(res) = client.get(&format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    let res = client
        .get(format!("{}/api/v1/llm/status", base))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "LLM status should succeed");

    let json: serde_json::Value = res.json().await.expect("valid JSON");
    assert!(json.get("status").is_some());
    assert!(json.get("providers").is_some());
}

/// P16.4-02: Ollama Provider Listed
#[tokio::test]
async fn llm_ollama_provider_listed() {
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
        if let Ok(res) = client.get(&format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    let res = client
        .get(format!("{}/api/v1/llm/status", base))
        .send()
        .await
        .expect("request succeeds");

    let json: serde_json::Value = res.json().await.expect("valid JSON");
    let providers = json.get("providers").and_then(|p| p.as_array()).unwrap();
    let names: Vec<&str> = providers
        .iter()
        .filter_map(|p| p.get("name").and_then(|n| n.as_str()))
        .collect();

    assert!(names.contains(&"ollama"), "Ollama should be listed");
}

/// P16.4-03: LLM Complete Works
#[tokio::test]
async fn llm_complete_works() {
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
        if let Ok(res) = client.get(&format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    let res = client
        .post(format!("{}/api/v1/llm/complete", base))
        .json(&serde_json::json!({
            "provider": "ollama",
            "model": "llama3",
            "prompt": "What is Rust?"
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "LLM complete should succeed");

    let json: serde_json::Value = res.json().await.expect("valid JSON");
    assert!(json.get("text").is_some());
    assert!(json.get("model").is_some());
    assert_eq!(json["provider"], "ollama");
}

/// P16.4-04: LLM Complete Empty Prompt Error
#[tokio::test]
async fn llm_complete_empty_prompt() {
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
        if let Ok(res) = client.get(&format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    let res = client
        .post(format!("{}/api/v1/llm/complete", base))
        .json(&serde_json::json!({
            "prompt": ""
        }))
        .send()
        .await
        .expect("request succeeds");

    // 应该返回 400 错误
    assert!(
        res.status().is_client_error(),
        "Empty prompt should return 4xx error"
    );
}

/// P16.4-05: LLM Ping Works
#[tokio::test]
async fn llm_ping_works() {
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
        if let Ok(res) = client.get(&format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    let res = client
        .post(format!("{}/api/v1/llm/ping", base))
        .json(&serde_json::json!({
            "provider": "ollama"
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(res.status().is_success(), "LLM ping should succeed");

    let json: serde_json::Value = res.json().await.expect("valid JSON");
    assert_eq!(json["provider"], "ollama");
    assert_eq!(json["status"], "available");
}
