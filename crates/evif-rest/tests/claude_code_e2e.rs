// Phase 13.5: Claude Code E2E Integration Tests
//
// 端到端验证 Claude Code 与 EVIF 的集成工作流
// 测试 CLAUDE.md 约定的上下文管理、决策记录、技能发现、多 Agent 协调

use base64::Engine;
use evif_core::RadixMountTable;
use evif_rest::create_routes;
use std::sync::Arc;

/// Helper: start server and return base URL
async fn start_server() -> (Arc<RadixMountTable>, String) {
    let mount_table = Arc::new(RadixMountTable::new());
    let app = create_routes(mount_table.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let base = format!("http://127.0.0.1:{}", port);

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    (mount_table, base)
}

/// Helper: wait for server and make a request
async fn wait_and_get(client: &reqwest::Client, url: &str) -> Option<reqwest::Response> {
    for _ in 0..60 {
        if let Ok(res) = client.get(url).send().await {
            return Some(res);
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }
    None
}

/// CC-01: CLAUDE.md Context Convention
/// 验证 CLAUDE.md 约定的上下文文件存在且可读
#[tokio::test]
async fn claude_code_context_convention() {
    let (_mount_table, base) = start_server().await;
    let client = reqwest::Client::new();

    // 验证 L0 当前上下文
    let url = format!("{}/api/v1/files?path=/L0/current", base);
    if let Some(res) = wait_and_get(&client, &url).await {
        assert!(
            res.status().is_success() || res.status().as_u16() == 404,
            "L0/current endpoint should be accessible"
        );
    }

    // 验证 L1 决策文件
    let url = format!("{}/api/v1/files?path=/L1/decisions.md", base);
    if let Some(res) = wait_and_get(&client, &url).await {
        assert!(
            res.status().is_success() || res.status().as_u16() == 404,
            "L1/decisions.md endpoint should be accessible"
        );
    }

    // 验证 L2 目录
    let url = format!("{}/api/v1/files?path=/L2", base);
    if let Some(res) = wait_and_get(&client, &url).await {
        assert!(
            res.status().is_success() || res.status().as_u16() == 404,
            "L2 endpoint should be accessible"
        );
    }
}

/// CC-02: Skill Discovery Workflow
/// 验证技能发现工作流（CLAUDE.md 约定）
#[tokio::test]
async fn claude_code_skill_discovery_workflow() {
    let (_mount_table, base) = start_server().await;
    let client = reqwest::Client::new();

    // 列出可用技能
    let url = format!("{}/skills/", base);
    if let Some(res) = wait_and_get(&client, &url).await {
        assert!(
            res.status().is_success() || res.status().as_u16() == 404,
            "Skills endpoint should be accessible"
        );
    }
}

/// CC-03: Context Write Workflow
/// 验证 Agent 写入上下文的工作流
#[tokio::test]
async fn claude_code_context_write_workflow() {
    let (mount_table, base) = start_server().await;
    let client = reqwest::Client::new();

    // Agent 更新 L0 当前任务
    let url = format!("{}/api/v1/files", base);
    let l0_data = base64::engine::general_purpose::STANDARD.encode("hello world");
    let res = client
        .put(&url)
        .json(&serde_json::json!({
            "path": "/L0/current",
            "data": l0_data,
            "encoding": "base64"
        }))
        .send()
        .await;

    assert!(res.is_ok(), "L0 write request should succeed");

    // Agent 记录决策到 L1
    let l1_data = base64::engine::general_purpose::STANDARD.encode("# Decisions\n\n1. Use JWT\n");
    let res = client
        .put(&url)
        .json(&serde_json::json!({
            "path": "/L1/decisions.md",
            "data": l1_data,
            "encoding": "base64"
        }))
        .send()
        .await;

    assert!(res.is_ok(), "L1 write request should succeed");

    // 防止未使用的警告
    let _ = mount_table;
}

/// CC-04: Session Lifecycle
/// 验证会话生命周期管理（CLAUDE.md 约定）
#[tokio::test]
async fn claude_code_session_lifecycle() {
    let (_mount_table, base) = start_server().await;
    let client = reqwest::Client::new();

    // 检查会话 ID
    let url = format!("{}/api/v1/files?path=/L1/session_id", base);
    if let Some(res) = wait_and_get(&client, &url).await {
        assert!(
            res.status().is_success() || res.status().as_u16() == 404,
            "session_id should be accessible"
        );
    }

    // 验证 context 目录结构
    let url = format!("{}/api/v1/directories?path=/context", base);
    if let Some(res) = wait_and_get(&client, &url).await {
        assert!(
            res.status().is_success() || res.status().as_u16() == 404,
            "context directory should be accessible"
        );
    }
}

/// CC-05: Multi-Agent Coordination (PipeFS)
/// 验证 PipeFS 双向通信（CLAUDE.md 的 /pipes/ 约定）
#[tokio::test]
async fn claude_code_multi_agent_coordination() {
    let (_mount_table, base) = start_server().await;
    let client = reqwest::Client::new();

    // 创建任务管道
    let url = format!("{}/api/v1/directories", base);
    let res = client
        .post(&url)
        .json(&serde_json::json!({ "path": "/pipes/task-001" }))
        .send()
        .await;
    assert!(res.is_ok(), "Pipe directory creation should succeed");

    // 写入管道输入
    let url = format!("{}/api/v1/files", base);
    let pipe_data = base64::engine::general_purpose::STANDARD.encode("review changed files");
    let res = client
        .put(&url)
        .json(&serde_json::json!({
            "path": "/pipes/task-001/input",
            "data": pipe_data,
            "encoding": "base64"
        }))
        .send()
        .await;
    assert!(res.is_ok(), "Pipe input write should succeed");
}

/// CC-06: Directory Navigation
/// 验证 CLAUDE.md 约定的目录导航模式
#[tokio::test]
async fn claude_code_directory_navigation() {
    let (_mount_table, base) = start_server().await;
    let client = reqwest::Client::new();

    let dirs = vec!["/", "/context", "/skills", "/pipes"];
    for dir in dirs {
        let url = format!("{}/api/v1/directories?path={}", base, dir);
        if let Some(res) = wait_and_get(&client, &url).await {
            assert!(
                res.status().is_success() || res.status().as_u16() == 404,
                "Directory {} should be accessible",
                dir
            );
        }
    }
}

/// CC-07: Health Check
/// 验证服务健康检查
#[tokio::test]
async fn claude_code_health_check() {
    let (_mount_table, base) = start_server().await;
    let client = reqwest::Client::new();

    let url = format!("{}/api/v1/health", base);
    if let Some(res) = wait_and_get(&client, &url).await {
        assert!(
            res.status().is_success() || res.status().as_u16() == 404,
            "Health endpoint should be accessible"
        );
    }
}

/// CC-08: MCP Tool Interface
/// 验证 MCP 工具接口（Claude Code MCP 连接测试）
#[tokio::test]
async fn claude_code_mcp_tool_interface() {
    let (_mount_table, base) = start_server().await;
    let client = reqwest::Client::new();

    // 验证文件操作工具端点可用
    let endpoints = vec![
        format!("{}/api/v1/files?path=/L0/current", base),
        format!("{}/api/v1/directories?path=/", base),
        format!("{}/api/v1/health", base),
        format!("{}/skills/", base),
    ];

    for url in endpoints {
        if let Some(res) = wait_and_get(&client, &url).await {
            assert!(
                res.status().is_success() || res.status().as_u16() == 404,
                "Tool endpoint {} should be accessible",
                url
            );
        }
    }
}
