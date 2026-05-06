// 上下文恢复率验证测试
//
// 验证 ContextFS 三层上下文系统的决策恢复能力
// 目标：>90% 决策可恢复

use evif_core::{EvifPlugin, WriteFlags};
use evif_plugins::ContextFsPlugin;

/// 测试 1：L1 决策写入后可恢复
///
/// 写入 10 个决策，验证全部可读回
#[tokio::test]
async fn test_l1_decisions_full_recovery() {
    let plugin = ContextFsPlugin::new();

    // 确保初始化
    let _ = plugin.readdir("/L0").await;

    // 写入 10 个决策到 L1
    let decisions: Vec<&str> = vec![
        "Rust as primary language",
        "Axum as web framework",
        "MCP protocol for Agent communication",
        "Skill system uses SKILL.md format",
        "MemoryFS uses vector storage",
        "ContextFS three-layer architecture",
        "PipeFS multi-Agent coordination",
        "Install script uses curl-bash pattern",
        "CLI commands unified evif prefix",
        "Token optimization truncate-compress",
    ];

    // Build all decisions content at once
    let mut all_content = String::new();
    for (i, decision) in decisions.iter().enumerate() {
        all_content.push_str(&format!("- Decision {}: {}\n", i + 1, decision));
    }

    plugin
        .write(
            "/L1/decisions.md",
            all_content.as_bytes().to_vec(),
            0,
            WriteFlags::empty(),
        )
        .await
        .expect("write all decisions should succeed");

    // 读取并验证恢复
    let content = plugin.read("/L1/decisions.md", 0, 100000).await.expect("read decisions");
    let content_str = String::from_utf8_lossy(&content);

    let mut recovered = 0;
    for decision in decisions.iter() {
        if content_str.contains(decision) {
            recovered += 1;
        }
    }

    let recovery_rate = recovered as f64 / decisions.len() as f64 * 100.0;
    assert!(
        recovery_rate >= 90.0,
        "L1 decision recovery rate should be >= 90%, actual: {:.1}% ({} / {})",
        recovery_rate, recovered, decisions.len()
    );
}

/// 测试 2：L1 到 L2 归档后可恢复
///
/// 写入决策到 L1，执行 end_session 归档到 L2，验证 L2 中可读回
#[tokio::test]
async fn test_l1_to_l2_archive_recovery() {
    let plugin = ContextFsPlugin::new();

    // 确保 L1 初始化
    let _ = plugin.readdir("/L1").await;

    // 写入决策到 L1
    let test_decisions = vec![
        "Architecture: Radix Tree routing",
        "Performance: parking_lot RwLock",
        "Safety: Result instead of expect",
    ];

    let mut all_decisions = String::new();
    for d in &test_decisions {
        all_decisions.push_str(&format!("- {}\n", d));
    }

    plugin
        .write("/L1/decisions.md", all_decisions.as_bytes().to_vec(), 0, WriteFlags::empty())
        .await
        .expect("write L1 decisions");

    // 设置 session
    plugin.start_session("test-session-001").await.expect("start session");

    // 写入 L0 当前状态
    plugin
        .write(
            "/L0/current",
            "Testing context recovery rate".as_bytes().to_vec(),
            0,
            WriteFlags::empty(),
        )
        .await
        .expect("write L0 current");

    // 结束 session（会归档 L1 到 L2）
    plugin.end_session().await.expect("end session");

    // 验证 L2 history 中有归档文件
    let l2_history = plugin.readdir("/L2/history").await.expect("read L2 history");
    assert!(
        !l2_history.is_empty(),
        "L2/history should contain archived decision files"
    );

    // 在归档文件中搜索决策
    let mut recovered = 0;
    for entry in &l2_history {
        if entry.name.contains("decisions") {
            let path = format!("/L2/history/{}", entry.name);
            if let Ok(content) = plugin.read(&path, 0, 100000).await {
                let content_str = String::from_utf8_lossy(&content);
                for d in &test_decisions {
                    if content_str.contains(d) {
                        recovered += 1;
                    }
                }
            }
        }
    }

    let recovery_rate = recovered as f64 / test_decisions.len() as f64 * 100.0;
    assert!(
        recovery_rate >= 90.0,
        "L2 archive recovery rate should be >= 90%, actual: {:.1}% ({} / {})",
        recovery_rate, recovered, test_decisions.len()
    );
}

/// 测试 3：L2 持久知识恢复
///
/// 验证 L2 中的架构文档和模式文档可以被完整读取
#[tokio::test]
async fn test_l2_knowledge_recovery() {
    let plugin = ContextFsPlugin::new();

    // 确保初始化
    let _ = plugin.readdir("/L2").await;

    // 验证种子文件存在
    let l2_files = plugin.readdir("/L2").await.expect("read L2");
    let file_names: Vec<String> = l2_files.iter().map(|f| f.name.clone()).collect();

    assert!(
        file_names.iter().any(|n| n.contains("architecture")),
        "L2 should contain architecture.md"
    );
    assert!(
        file_names.iter().any(|n| n.contains("patterns")),
        "L2 should contain patterns.md"
    );

    // 读取并验证内容
    let arch = plugin.read("/L2/architecture.md", 0, 100000).await;
    assert!(arch.is_ok(), "Should be able to read architecture.md");

    let arch_data = arch.unwrap();
    let arch_str = String::from_utf8_lossy(&arch_data);
    assert!(
        !arch_str.is_empty(),
        "architecture.md should have content"
    );
    assert!(
        arch_str.contains("EVIF") || arch_str.contains("architecture") || arch_str.contains("Architecture"),
        "architecture.md should contain architecture-related content"
    );

    let patterns = plugin.read("/L2/patterns.md", 0, 100000).await;
    assert!(patterns.is_ok(), "Should be able to read patterns.md");

    let patterns_data = patterns.unwrap();
    let patterns_str = String::from_utf8_lossy(&patterns_data);
    assert!(
        !patterns_str.is_empty(),
        "patterns.md should have content"
    );
}

/// 测试 4：会话生命周期中上下文完整性
///
/// 完整测试 start_session -> write -> end_session -> restore flow
#[tokio::test]
async fn test_session_lifecycle_context_integrity() {
    let plugin = ContextFsPlugin::new();

    // 开始 session
    plugin.start_session("integrity-test").await.expect("start session");

    // 写入 L0 当前任务
    plugin
        .write("/L0/current", "Test context integrity".as_bytes().to_vec(), 0, WriteFlags::empty())
        .await
        .expect("write L0");

    // 写入 L1 决策
    plugin
        .write(
            "/L1/decisions.md",
            "- ContextFS three-layer architecture\n- SQLite persistence support\n".as_bytes().to_vec(),
            0,
            WriteFlags::empty(),
        )
        .await
        .expect("write L1 decisions");

    // 写入 L2 知识
    plugin
        .write(
            "/L2/test_knowledge.md",
            "# Test Knowledge\n\nTest knowledge entry.\n".as_bytes().to_vec(),
            0,
            WriteFlags::empty(),
        )
        .await
        .expect("write L2 knowledge");

    // 验证所有层级可读
    let l0 = plugin.read("/L0/current", 0, 10000).await;
    assert!(l0.is_ok(), "L0 should be readable");
    let l0_data = l0.unwrap();
    let l0_str = String::from_utf8_lossy(&l0_data);
    assert!(l0_str.contains("Test"), "L0 content should be recoverable");

    let l1 = plugin.read("/L1/decisions.md", 0, 10000).await;
    assert!(l1.is_ok(), "L1 should be readable");
    let l1_data = l1.unwrap();
    let l1_str = String::from_utf8_lossy(&l1_data);
    assert!(l1_str.contains("ContextFS"), "L1 decisions should be recoverable");

    let l2 = plugin.read("/L2/test_knowledge.md", 0, 10000).await;
    assert!(l2.is_ok(), "L2 should be readable");
    let l2_data = l2.unwrap();
    let l2_str = String::from_utf8_lossy(&l2_data);
    assert!(l2_str.contains("Test Knowledge"), "L2 knowledge should be recoverable");

    // 结束 session
    plugin.end_session().await.expect("end session");

    // 验证 session 结束后 L2 有归档
    let l2_history = plugin.readdir("/L2/history").await;
    assert!(l2_history.is_ok(), "L2 history should be readable");
    assert!(
        !l2_history.unwrap().is_empty(),
        "L2 history should contain archives"
    );
}

/// 测试 5：Token 预算系统验证
///
/// 验证 estimate_tokens 和 check_budget 功能正常
#[tokio::test]
async fn test_token_budget_system() {
    let plugin = ContextFsPlugin::new();

    // 确保 L0 初始化
    let _ = plugin.readdir("/L0").await;

    // 写入一些内容
    plugin
        .write("/L0/current", "Token budget test content".as_bytes().to_vec(), 0, WriteFlags::empty())
        .await
        .expect("write L0");

    // 估算 tokens
    let budget = plugin.estimate_tokens().await.expect("estimate tokens");
    assert!(
        budget.total_tokens > 0,
        "total_tokens should be > 0"
    );
    assert!(
        budget.l0_tokens > 0 || budget.l1_tokens > 0 || budget.l2_tokens > 0,
        "At least one layer should have tokens"
    );

    // 检查预算
    let status = plugin.check_budget().await.expect("check budget");
    assert!(
        status.used_tokens > 0,
        "used_tokens should be > 0"
    );
    assert!(
        status.usage_percent < 100.0,
        "usage_percent should be < 100%"
    );
}
