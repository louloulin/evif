// Phase 13.3: AgentBench Benchmark Tests
//
// AgentBench 对标: 多环境评估框架，测试 EVIF 作为 Agent Tool 的表现
// 测试工具调用成功率、多步骤任务、错误恢复、上下文切换

use evif_core::{EvifPlugin, WriteFlags};
use evif_plugins::ContextFsPlugin;

// ---------------------------------------------------------------------------
// AgentBench 对标测试
// ---------------------------------------------------------------------------

/// AB-01: Tool Use Success Rate
/// 目标: 100 次工具调用，95%+ 成功率
#[tokio::test]
async fn agentbench_tool_use_success_rate() {
    let plugin = ContextFsPlugin::new();
    let mut success = 0usize;
    let total = 100usize;

    for i in 0..total {
        let result = plugin.mkdir(&format!("/L0/dir_{}", i), 0o755).await;
        if result.is_ok() {
            success += 1;
        }
    }

    let rate = success as f64 / total as f64;
    assert!(
        rate >= 0.95,
        "Tool success rate {:.1}% < 95%",
        rate * 100.0
    );
}

/// AB-02: Multi-Step Task
/// 对标 AgentBench: 完整的多步骤任务执行
#[tokio::test]
async fn agentbench_multi_step_task() {
    let plugin = ContextFsPlugin::new();

    // 步骤 1: 创建项目目录
    plugin.mkdir("/test_agentbench", 0o755).await.expect("mkdir project");

    // 步骤 2: 创建源文件
    plugin.create("/test_agentbench/main.rs", 0o644).await.expect("create main.rs");
    plugin
        .write(
            "/test_agentbench/main.rs",
            b"fn main() {}\n".to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write main.rs");

    // 步骤 3: 创建配置文件
    plugin.create("/test_agentbench/Cargo.toml", 0o644).await.expect("create Cargo.toml");
    plugin
        .write(
            "/test_agentbench/Cargo.toml",
            b"[package]\nname = \"test\"\n".to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write Cargo.toml");

    // 步骤 4: 读取验证
    let content = plugin
        .read("/test_agentbench/main.rs", 0, 0)
        .await
        .expect("read main.rs");
    let content_str = String::from_utf8_lossy(&content);
    assert!(content_str.contains("fn main"), "Content should contain 'fn main'");

    // 步骤 5: 列出目录验证结构
    let entries = plugin.readdir("/test_agentbench").await.expect("list dir");
    let names: Vec<String> = entries.into_iter().map(|e| e.name).collect();
    assert!(names.contains(&"main.rs".to_string()), "Should have main.rs");
    assert!(
        names.contains(&"Cargo.toml".to_string()),
        "Should have Cargo.toml"
    );
}

/// AB-03: Error Recovery
/// 对标 AgentBench: 失败后自动恢复
#[tokio::test]
async fn agentbench_error_recovery() {
    let plugin = ContextFsPlugin::new();

    // 尝试读取不存在的文件
    let read_result = plugin.read("/nonexistent/file.txt", 0, 0).await;
    assert!(read_result.is_err(), "Non-existent file read should fail");

    // 创建文件后重试
    plugin.mkdir("/nonexistent", 0o755).await.expect("mkdir");
    plugin
        .create("/nonexistent/file.txt", 0o644)
        .await
        .expect("create");
    plugin
        .write(
            "/nonexistent/file.txt",
            b"recovered content".to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write");

    // 验证恢复成功
    let content = plugin
        .read("/nonexistent/file.txt", 0, 0)
        .await
        .expect("read recovered");
    assert_eq!(
        content.as_slice(),
        b"recovered content",
        "Recovered content should match"
    );
}

/// AB-04: Context Switching
/// 对标 AgentBench: Agent 在不同上下文间切换
#[tokio::test]
async fn agentbench_context_switching() {
    let plugin = ContextFsPlugin::new();

    // 上下文 A: 项目 A
    plugin.mkdir("/context_a", 0o755).await.expect("mkdir context_a");
    plugin
        .create("/context_a/file_a.txt", 0o644)
        .await
        .expect("create file_a");
    plugin
        .write(
            "/context_a/file_a.txt",
            b"context A data".to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write file_a");

    // 上下文 B: 项目 B
    plugin.mkdir("/context_b", 0o755).await.expect("mkdir context_b");
    plugin
        .create("/context_b/file_b.txt", 0o644)
        .await
        .expect("create file_b");
    plugin
        .write(
            "/context_b/file_b.txt",
            b"context B data".to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write file_b");

    // 切换到上下文 A
    let content_a = plugin
        .read("/context_a/file_a.txt", 0, 0)
        .await
        .expect("read context_a");
    assert_eq!(
        content_a.as_slice(),
        b"context A data",
        "Context A should have its data"
    );

    // 切换到上下文 B
    let content_b = plugin
        .read("/context_b/file_b.txt", 0, 0)
        .await
        .expect("read context_b");
    assert_eq!(
        content_b.as_slice(),
        b"context B data",
        "Context B should have its data"
    );

    // 验证上下文 A 仍然存在（未被覆盖）
    let content_a2 = plugin
        .read("/context_a/file_a.txt", 0, 0)
        .await
        .expect("read context_a again");
    assert_eq!(
        content_a2.as_slice(),
        b"context A data",
        "Context A should persist after switch"
    );
}

/// AB-05: Resource Cleanup
/// 对标 AgentBench: Agent 执行后正确清理资源
#[tokio::test]
async fn agentbench_resource_cleanup() {
    let plugin = ContextFsPlugin::new();

    plugin.mkdir("/tmp", 0o755).await.expect("mkdir /tmp");
    // 创建临时文件
    for i in 0..10u32 {
        plugin
            .create(&format!("/tmp/cleanup_{}.txt", i), 0o644)
            .await
            .expect("create temp file");
        plugin
            .write(
                &format!("/tmp/cleanup_{}.txt", i),
                format!("temp data {}", i).into_bytes(),
                0,
                WriteFlags::TRUNCATE,
            )
            .await
            .expect("write temp file");
    }

    // 验证文件存在
    for i in 0..10u32 {
        let stat = plugin
            .stat(&format!("/tmp/cleanup_{}.txt", i))
            .await
            .expect("stat temp file");
        assert!(!stat.is_dir, "Should be a file");
    }

    // 清理文件
    for i in 0..10u32 {
        plugin
            .remove(&format!("/tmp/cleanup_{}.txt", i))
            .await
            .expect("remove temp file");
    }

    // 验证清理成功
    for i in 0..10u32 {
        let stat = plugin
            .stat(&format!("/tmp/cleanup_{}.txt", i))
            .await;
        assert!(stat.is_err(), "File {} should be deleted", i);
    }
}

/// AB-06: Concurrent Operations
/// 对标 AgentBench: 多个 Agent 并发操作
#[tokio::test]
async fn agentbench_concurrent_operations() {
    let mut handles = Vec::new();

    for i in 0..50u32 {
        let plugin = ContextFsPlugin::new();
        let path = format!("/concurrent_agent_{}/data.txt", i);
        let content = format!("Agent {} data", i);

        handles.push(tokio::spawn(async move {
            plugin
                .mkdir(&format!("/concurrent_agent_{}", i), 0o755)
                .await?;
            plugin.create(&path, 0o644).await?;
            plugin
                .write(&path, content.into_bytes(), 0, WriteFlags::TRUNCATE)
                .await?;
            plugin.read(&path, 0, 0).await?;
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        }));
    }

    let results: Vec<Result<_, _>> = futures::future::join_all(handles).await;
    let success_count = results.iter().filter(|r| r.is_ok()).count();

    assert!(
        success_count >= 47,
        "Concurrent operations success rate {:.1}% < 94%",
        success_count as f64 * 2.0
    );
}

/// AB-07: Environment Verification
/// 对标 AgentBench: 验证执行前环境状态
#[tokio::test]
async fn agentbench_environment_verification() {
    let plugin = ContextFsPlugin::new();

    // 验证 L0 层已初始化
    let l0_entries = plugin.readdir("/L0").await.expect("list L0");
    assert!(
        l0_entries.iter().any(|e| e.name == "current"),
        "L0 should have 'current' file"
    );

    // 验证 L1 层已初始化
    let l1_entries = plugin.readdir("/L1").await.expect("list L1");
    assert!(
        l1_entries.iter().any(|e| e.name == "session_id"),
        "L1 should have 'session_id' file"
    );

    // 验证 L2 层已初始化
    let l2_stat = plugin.stat("/L2").await.expect("stat L2");
    assert!(l2_stat.is_dir, "L2 should be a directory");

    // 验证根目录
    let root_stat = plugin.stat("/").await.expect("stat root");
    assert!(root_stat.is_dir, "Root should be a directory");
}

/// AB-08: Long-Running Task
/// 对标 AgentBench: 长时间运行的任务
#[tokio::test]
async fn agentbench_long_running_task() {
    let plugin = ContextFsPlugin::new();

    plugin.mkdir("/long_running", 0o755).await.expect("mkdir /long_running");
    // 创建顶层目录
    plugin.mkdir("/long_running/src", 0o755).await.expect("mkdir src");
    plugin.mkdir("/long_running/tests", 0o755).await.expect("mkdir tests");
    plugin.mkdir("/long_running/docs", 0o755).await.expect("mkdir docs");
    // 创建二级目录
    plugin.mkdir("/long_running/src/handlers", 0o755).await.expect("mkdir handlers");
    plugin.mkdir("/long_running/src/models", 0o755).await.expect("mkdir models");
    plugin.mkdir("/long_running/src/utils", 0o755).await.expect("mkdir utils");

    // 在每个目录创建文件
    let file_dirs = vec![
        "/long_running/src",
        "/long_running/src/handlers",
        "/long_running/src/models",
        "/long_running/src/utils",
        "/long_running/tests",
        "/long_running/docs",
    ];
    for dir in &file_dirs {
        let filename = dir.split('/').next_back().unwrap();
        let file_path = format!("{}/{}.rs", dir, filename);
        plugin.create(&file_path, 0o644).await.expect("create");
        plugin
            .write(
                &file_path,
                format!("// {} module\n", filename).into_bytes(),
                0,
                WriteFlags::TRUNCATE,
            )
            .await
            .expect("write");
    }

    // 验证所有文件和目录
    // 检查子目录有文件
    let src_entries = plugin.readdir("/long_running/src").await.expect("readdir /long_running/src");
    assert!(!src_entries.is_empty(), "/long_running/src should have files");

    let tests_entries = plugin.readdir("/long_running/tests").await.expect("readdir /long_running/tests");
    assert!(!tests_entries.is_empty(), "/long_running/tests should have files");

    // 检查顶层目录结构
    let root_entries = plugin.readdir("/long_running").await.expect("readdir /long_running");
    let root_names: Vec<String> = root_entries.into_iter().map(|e| e.name).collect();
    assert!(root_names.contains(&"src".to_string()), "Should have src/");
    assert!(root_names.contains(&"tests".to_string()), "Should have tests/");
    assert!(root_names.contains(&"docs".to_string()), "Should have docs/");
}
