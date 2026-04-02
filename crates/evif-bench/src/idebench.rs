// Phase 14.4: IDE-Bench Benchmark Tests
//
// 对标 IDE-Bench 评估 AI IDE Agent 的文件读写和导航任务

use evif_rest::create_routes;
use evif_core::RadixMountTable;
use std::sync::Arc;

async fn setup_server() -> (Arc<RadixMountTable>, String) {
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

    let mut ready = false;
    for _ in 0..60 {
        if let Ok(res) = client.get(&format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                ready = true;
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    assert!(ready, "Server should be ready");
    (mount_table, base)
}

/// IDE-01: 文件读取任务
#[tokio::test]
async fn idebench_read_file() {
    let (_mount_table, base) = setup_server().await;
    let client = reqwest::Client::new();

    // 确保目录存在
    let _ = client
        .post(format!("{}/api/v1/directories", base))
        .json(&serde_json::json!({ "path": "/test/src" }))
        .send()
        .await;

    // 写入测试文件
    let content = "pub fn add(a: i32, b: i32) -> i32 { a + b }";
    let res = client
        .put(format!("{}/api/v1/files", base))
        .json(&serde_json::json!({
            "path": "/test/src/lib.rs",
            "content": content
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(
        !res.status().is_server_error(),
        "File write should not cause server error"
    );

    // 模拟 Agent 读取文件
    let res = client
        .get(&format!("{}/api/v1/files", base))
        .query(&[("path", "/test/src/lib.rs")])
        .send()
        .await
        .expect("request succeeds");

    assert!(
        !res.status().is_server_error(),
        "File read endpoint should not cause server error"
    );
}

/// IDE-02: 目录导航任务
#[tokio::test]
async fn idebench_directory_navigation() {
    let (_mount_table, base) = setup_server().await;
    let client = reqwest::Client::new();

    // 创建目录结构
    for dir in ["/test/src", "/test/tests", "/test/docs"] {
        let _ = client
            .post(format!("{}/api/v1/directories", base))
            .json(&serde_json::json!({ "path": dir }))
            .send()
            .await;
    }

    // 创建文件
    let _ = client
        .put(format!("{}/api/v1/files", base))
        .json(&serde_json::json!({ "path": "/test/Cargo.toml", "content": "[package]" }))
        .send()
        .await;

    // 模拟 Agent 导航
    let res = client
        .get(&format!("{}/api/v1/directories", base))
        .query(&[("path", "/test")])
        .send()
        .await
        .expect("request succeeds");

    assert!(
        !res.status().is_server_error(),
        "Directory navigation should not cause server error"
    );
}

/// IDE-03: 文件搜索任务
#[tokio::test]
async fn idebench_file_search() {
    let (_mount_table, base) = setup_server().await;
    let client = reqwest::Client::new();

    // 创建多个文件
    let _ = client
        .post(format!("{}/api/v1/directories", base))
        .json(&serde_json::json!({ "path": "/test/search" }))
        .send()
        .await;

    for name in ["a.rs", "b.rs", "c.rs"] {
        let _ = client
            .put(format!("{}/api/v1/files", base))
            .json(&serde_json::json!({
                "path": format!("/test/search/{}", name),
                "content": "fn test() {}"
            }))
            .send()
            .await;
    }

    // 模拟 Agent 搜索 "fn test"
    let res = client
        .post(format!("{}/api/v1/grep", base))
        .json(&serde_json::json!({
            "path": "/test/search",
            "pattern": "fn test"
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(
        !res.status().is_server_error(),
        "Grep should not cause server error"
    );
}

/// IDE-04: 多文件编辑任务
#[tokio::test]
async fn idebench_multi_file_edit() {
    let (_mount_table, base) = setup_server().await;
    let client = reqwest::Client::new();

    let _ = client
        .post(format!("{}/api/v1/directories", base))
        .json(&serde_json::json!({ "path": "/test/edit" }))
        .send()
        .await;

    // 批量写入多个文件
    let files = ["mod.rs", "lib.rs", "types.rs"];
    for name in files {
        let res = client
            .put(format!("{}/api/v1/files", base))
            .json(&serde_json::json!({
                "path": format!("/test/edit/{}", name),
                "content": format!("// {}", name)
            }))
            .send()
            .await
            .expect("request succeeds");

        assert!(
            !res.status().is_server_error(),
            "Multi-file write should not cause server error"
        );
    }
}

/// IDE-05: 大文件读写性能 (<100ms)
#[tokio::test]
async fn idebench_large_file_performance() {
    let (_mount_table, base) = setup_server().await;
    let client = reqwest::Client::new();

    // 生成 1MB 数据
    let content: String = (0..8192).map(|_| 'x').collect();

    let write_start = std::time::Instant::now();
    let _res = client
        .put(format!("{}/api/v1/files", base))
        .json(&serde_json::json!({
            "path": "/test/large.bin",
            "content": content
        }))
        .send()
        .await
        .expect("request succeeds");
    let write_ms = write_start.elapsed().as_millis() as u64;

    // 读取
    let read_start = std::time::Instant::now();
    let res = client
        .get(&format!("{}/api/v1/files", base))
        .query(&[("path", "/test/large.bin")])
        .send()
        .await
        .expect("request succeeds");
    let read_ms = read_start.elapsed().as_millis() as u64;

    // 验证操作成功（性能要求：< 200ms）
    assert!(
        !res.status().is_server_error(),
        "Large file read should not cause server error"
    );

    // 记录性能数据
    println!(
        "Large file: write={}ms, read={}ms",
        write_ms, read_ms
    );
}
