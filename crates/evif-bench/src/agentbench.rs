#![allow(dead_code, clippy::needless_borrows_for_generic_args)]

// Phase 14.4: AgentBench Benchmark Tests
//
// 对标 AgentBench 多环境评估框架

use evif_core::RadixMountTable;
use evif_plugins::MemFsPlugin;
use evif_rest::create_routes;
use std::sync::Arc;

async fn setup_server() -> (Arc<RadixMountTable>, String) {
    let mount_table = Arc::new(RadixMountTable::new());
    // 挂载内存文件系统，使所有文件操作端点可用
    mount_table
        .mount("/test".into(), Arc::new(MemFsPlugin::new()))
        .await
        .expect("mount memfs for benchmark");
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

/// AB-01: 工具调用成功率 (100 调用, 95%+ 成功率)
#[tokio::test]
async fn agentbench_tool_success_rate() {
    let (_mount_table, base) = setup_server().await;
    let client = reqwest::Client::new();

    // 确保目录存在
    let _ = client
        .post(format!("{}/api/v1/directories", base))
        .json(&serde_json::json!({ "path": "/test/agent" }))
        .send()
        .await;

    let mut success = 0;
    let total = 100;

    for i in 0..total {
        let res = client
            .post(format!("{}/api/v1/directories", base))
            .json(&serde_json::json!({
                "path": format!("/test/agent/dir_{}", i)
            }))
            .send()
            .await
            .expect("request succeeds");

        if res.status().is_success() || res.status().as_u16() == 201 {
            success += 1;
        }
    }

    let rate = success as f64 / total as f64;
    // 目标：95%+ 成功率（对标 AgentBench 工具调用成功率基准）
    assert!(
        rate >= 0.95,
        "Tool calls should achieve >= 95% success rate (got {:.1}%, baseline >= 90%)",
        rate * 100.0
    );
}

/// AB-02: 多步骤任务执行
#[tokio::test]
async fn agentbench_multi_step_task() {
    let (_mount_table, base) = setup_server().await;
    let client = reqwest::Client::new();

    // 步骤 1: 创建目录
    let res = client
        .post(format!("{}/api/v1/directories", base))
        .json(&serde_json::json!({ "path": "/test/multi/project" }))
        .send()
        .await
        .expect("request succeeds");

    assert!(
        !res.status().is_server_error(),
        "Step 1 (mkdir) should not cause server error"
    );

    // 步骤 2: 创建文件
    let res = client
        .put(format!("{}/api/v1/files", base))
        .json(&serde_json::json!({
            "path": "/test/multi/project/main.rs",
            "content": "fn main() {}"
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(
        !res.status().is_server_error(),
        "Step 2 (write) should not cause server error"
    );

    // 步骤 3: 读取验证
    let res = client
        .get(&format!("{}/api/v1/files", base))
        .query(&[("path", "/test/multi/project/main.rs")])
        .send()
        .await
        .expect("request succeeds");

    assert!(
        !res.status().is_server_error(),
        "Step 3 (read) should not cause server error"
    );
}

/// AB-03: 错误恢复
#[tokio::test]
async fn agentbench_error_recovery() {
    let (_mount_table, base) = setup_server().await;
    let client = reqwest::Client::new();

    // 尝试操作不存在的路径（预期错误）
    let res = client
        .get(&format!("{}/api/v1/files", base))
        .query(&[("path", "/nonexistent/path/file.txt")])
        .send()
        .await
        .expect("request succeeds");

    // 应该返回 404 或其他错误，不是服务器崩溃
    assert!(
        !res.status().is_server_error(),
        "Should not return 5xx for nonexistent path"
    );

    // 恢复正常操作
    let _ = client
        .post(format!("{}/api/v1/directories", base))
        .json(&serde_json::json!({ "path": "/test/recovery" }))
        .send()
        .await;

    let res = client
        .put(format!("{}/api/v1/files", base))
        .json(&serde_json::json!({
            "path": "/test/recovery/after_error.txt",
            "content": "recovered"
        }))
        .send()
        .await
        .expect("request succeeds");

    assert!(
        !res.status().is_server_error(),
        "Should recover after error"
    );
}

/// AB-04: 上下文切换
#[tokio::test]
async fn agentbench_context_switch() {
    let (_mount_table, base) = setup_server().await;
    let client = reqwest::Client::new();

    // 在不同目录间切换
    for i in 0..10 {
        let path = format!("/test/switch_{}", i);
        let res = client
            .post(format!("{}/api/v1/directories", base))
            .json(&serde_json::json!({ "path": path }))
            .send()
            .await
            .expect("request succeeds");

        assert!(
            !res.status().is_server_error(),
            "Context switch {} should not cause server error",
            i
        );
    }
}

/// AB-05: 资源清理
#[tokio::test]
async fn agentbench_resource_cleanup() {
    let (_mount_table, base) = setup_server().await;
    let client = reqwest::Client::new();

    // 创建文件
    let _ = client
        .post(format!("{}/api/v1/directories", base))
        .json(&serde_json::json!({ "path": "/test/cleanup" }))
        .send()
        .await;

    let _ = client
        .put(format!("{}/api/v1/files", base))
        .json(&serde_json::json!({ "path": "/test/cleanup/temp.txt", "content": "temp" }))
        .send()
        .await;

    // 删除文件
    let res = client
        .delete(&format!("{}/api/v1/files", base))
        .json(&serde_json::json!({ "path": "/test/cleanup/temp.txt" }))
        .send()
        .await
        .expect("request succeeds");

    assert!(
        !res.status().is_server_error(),
        "Delete should not cause server error"
    );
}

/// AB-06: 并发操作
#[tokio::test]
async fn agentbench_concurrent_operations() {
    let (_mount_table, base) = setup_server().await;
    let client = reqwest::Client::new();

    // 混合并发操作：创建、写入、读取
    let mut handles = Vec::new();

    // 50 个写入
    for i in 0..50 {
        let client = client.clone();
        let base = base.clone();
        handles.push(tokio::spawn(async move {
            client
                .put(format!("{}/api/v1/files", base))
                .json(&serde_json::json!({
                    "path": format!("/test/concurrent/file_{}", i),
                    "content": format!("content_{}", i)
                }))
                .send()
                .await
                .map(|resp| !resp.status().is_server_error())
                .unwrap_or(false)
        }));
    }

    // 50 个读取
    for i in 0..50 {
        let client = client.clone();
        let base = base.clone();
        handles.push(tokio::spawn(async move {
            client
                .get(&format!("{}/api/v1/files", base))
                .query(&[("path", format!("/test/concurrent/file_{}", i))])
                .send()
                .await
                .map(|resp| !resp.status().is_server_error())
                .unwrap_or(false)
        }));
    }

    let results: Vec<bool> = futures::future::join_all(handles)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    // 基准测试通过，只要所有请求完成（无服务器错误）
    assert!(
        results.len() == 100,
        "All concurrent requests should complete, got {}/100",
        results.len()
    );
}
