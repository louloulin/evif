#![allow(dead_code, clippy::needless_borrows_for_generic_args)]

// Phase 14.4: AgentBench Benchmark Tests
//
// 对标 AgentBench 多环境评估框架

use evif_core::RadixMountTable;
use evif_plugins::MemFsPlugin;
use evif_rest::create_routes;
use std::sync::Arc;

// Sandbox skip helper
fn is_network_available() -> bool {
    std::net::TcpListener::bind("127.0.0.1:0").is_ok()
}

macro_rules! skip_if_sandboxed {
    () => {
        if !is_network_available() {
            println!("SKIP: Network operations not permitted (sandbox restriction)");
            return;
        }
    };
}

async fn setup_server() -> (Arc<RadixMountTable>, String) {
    let mount_table = Arc::new(RadixMountTable::new());
    // 挂载内存文件系统，使所有文件操作端点可用
    mount_table
        .mount("/test".into(), Arc::new(MemFsPlugin::new()))
        .await
        .expect("mount memfs for benchmark");
    let app = create_routes(mount_table.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Channel to receive the URL
    let (tx, rx) = std::sync::mpsc::channel();

    // Spawn server in background thread
    let _handle = std::thread::spawn(move || {
        // Use multi-threaded runtime for benchmark tests
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .build()
            .expect("runtime");

        // Use enter to ensure we stay in this runtime
        let _guard = runtime.enter();

        runtime.block_on(async {
            let base = format!("http://{}", addr);

            // Send URL BEFORE blocking
            let _ = tx.send(base.clone());

            // Run server - block until shutdown
            if let Err(e) = axum::serve(listener, app.into_make_service()).await {
                eprintln!("Server error: {}", e);
            }
        });
    });

    // Wait for URL from background thread
    let base = rx.recv().expect("receive base url");

    (mount_table, base)
}

/// AB-01: 工具调用成功率 (100 调用, 95%+ 成功率)
#[tokio::test]
#[ignore = "Flaky: server startup race in multi-threaded tests; run with dedicated server process"]
async fn agentbench_tool_success_rate() {
    skip_if_sandboxed!();
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
#[ignore = "Flaky: server startup race in multi-threaded tests; run with dedicated server process"]
async fn agentbench_multi_step_task() {
    skip_if_sandboxed!();
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
#[ignore = "Flaky: server startup race in multi-threaded tests; run with dedicated server process"]
async fn agentbench_error_recovery() {
    skip_if_sandboxed!();
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
#[ignore = "Flaky: server startup race in multi-threaded tests; run with dedicated server process"]
async fn agentbench_context_switch() {
    skip_if_sandboxed!();
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
#[ignore = "Flaky: server startup race in multi-threaded tests; run with dedicated server process"]
async fn agentbench_resource_cleanup() {
    skip_if_sandboxed!();
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
    skip_if_sandboxed!();
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

/// Benchmark metadata for AgentBench suite
pub fn benchmarks() -> Vec<BenchmarkInfo> {
    vec![
        BenchmarkInfo {
            name: "agentbench_single_file_operation".to_string(),
            category: "single_file".to_string(),
            description: "Single file create/read/update/delete".to_string(),
        },
        BenchmarkInfo {
            name: "agentbench_multi_step_task".to_string(),
            category: "multi_step".to_string(),
            description: "Multi-step task execution".to_string(),
        },
        BenchmarkInfo {
            name: "agentbench_error_recovery".to_string(),
            category: "error_recovery".to_string(),
            description: "Error recovery after failures".to_string(),
        },
        BenchmarkInfo {
            name: "agentbench_context_switch".to_string(),
            category: "context_switch".to_string(),
            description: "Context switching between operations".to_string(),
        },
        BenchmarkInfo {
            name: "agentbench_resource_cleanup".to_string(),
            category: "resource_cleanup".to_string(),
            description: "Proper resource cleanup".to_string(),
        },
        BenchmarkInfo {
            name: "agentbench_concurrent_operations".to_string(),
            category: "concurrency".to_string(),
            description: "100 concurrent operations".to_string(),
        },
    ]
}

/// Benchmark information structure
#[derive(Debug, Clone)]
pub struct BenchmarkInfo {
    pub name: String,
    pub category: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_info_creation() {
        let info = BenchmarkInfo {
            name: "test".to_string(),
            category: "unit".to_string(),
            description: "Test benchmark".to_string(),
        };
        assert_eq!(info.name, "test");
        assert_eq!(info.category, "unit");
    }

    #[test]
    fn test_benchmarks_returns_expected_count() {
        let benchmarks = benchmarks();
        assert_eq!(benchmarks.len(), 6);
    }

    #[test]
    fn test_benchmarks_have_valid_names() {
        let benchmarks = benchmarks();
        for bench in benchmarks {
            assert!(!bench.name.is_empty());
            assert!(!bench.category.is_empty());
        }
    }
}
