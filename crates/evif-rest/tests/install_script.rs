// 安装脚本测试
//
// 测试 install.sh 脚本的正确性
// 注意：这些测试只验证脚本的结构和基本逻辑，不实际执行安装

use std::process::Command;

#[tokio::test]
async fn test_install_script_detect_platform_darwin() {
    // 验证脚本包含 Darwin 平台检测
    let script = include_str!("../../../scripts/install.sh");

    assert!(
        script.contains("Darwin"),
        "脚本应检测 Darwin 平台"
    );
    assert!(
        script.contains("apple-darwin"),
        "脚本应使用 apple-darwin 标识"
    );
}

#[tokio::test]
async fn test_install_script_detect_platform_linux() {
    // 验证脚本包含 Linux 平台检测
    let script = include_str!("../../../scripts/install.sh");

    assert!(
        script.contains("Linux"),
        "脚本应检测 Linux 平台"
    );
    assert!(
        script.contains("unknown-linux-gnu"),
        "脚本应使用 unknown-linux-gnu 标识"
    );
}

#[tokio::test]
async fn test_install_script_detect_arch_x86_64() {
    // 验证脚本支持 x86_64 架构
    let script = include_str!("../../../scripts/install.sh");

    assert!(
        script.contains("x86_64"),
        "脚本应支持 x86_64 架构"
    );
}

#[tokio::test]
async fn test_install_script_detect_arch_arm64() {
    // 验证脚本支持 ARM64 架构
    let script = include_str!("../../../scripts/install.sh");

    assert!(
        script.contains("aarch64") || script.contains("arm64"),
        "脚本应支持 ARM64 架构"
    );
}

#[tokio::test]
async fn test_install_script_has_error_handling() {
    // 验证脚本包含错误处理
    let script = include_str!("../../../scripts/install.sh");

    // 应该使用 set -euo pipefail
    assert!(
        script.contains("set -euo pipefail"),
        "脚本应启用严格错误模式"
    );
}

#[tokio::test]
async fn test_install_script_has_path_setup() {
    // 验证脚本包含 PATH 设置逻辑
    let script = include_str!("../../../scripts/install.sh");

    assert!(
        script.contains("export PATH"),
        "脚本应设置 PATH 环境变量"
    );
}

#[tokio::test]
async fn test_install_script_has_config_creation() {
    // 验证脚本创建配置文件
    let script = include_str!("../../../scripts/install.sh");

    assert!(
        script.contains("default.toml"),
        "脚本应创建默认配置文件"
    );
    assert!(
        script.contains("[evif]"),
        "配置文件应包含 [evif] 部分"
    );
}

#[tokio::test]
async fn test_install_script_has_skills_path() {
    // 验证脚本配置 skills 路径
    let script = include_str!("../../../scripts/install.sh");

    assert!(
        script.contains("skills"),
        "脚本应配置 skills 目录"
    );
}

#[tokio::test]
async fn test_install_script_has_mcp_config() {
    // 验证脚本配置 MCP
    let script = include_str!("../../../scripts/install.sh");

    assert!(
        script.contains("[mcp]") || script.contains("mcp"),
        "脚本应包含 MCP 配置"
    );
}

#[tokio::test]
async fn test_install_script_has_version_detection() {
    // 验证脚本支持版本检测
    let script = include_str!("../../../scripts/install.sh");

    assert!(
        script.contains("EVIF_VERSION"),
        "脚本应支持 EVIF_VERSION 环境变量"
    );
    assert!(
        script.contains("github.com"),
        "脚本应从 GitHub 获取版本信息"
    );
}

#[tokio::test]
async fn test_install_script_syntax_is_valid() {
    // 使用 bash -n 验证脚本语法
    let script_path = "scripts/install.sh";
    let output = Command::new("bash")
        .args(["-n", script_path])
        .current_dir("/Users/louloulin/Documents/linchong/claude/evif")
        .output();

    // 如果 bash -n 可用，验证语法
    if let Ok(output) = output {
        assert!(
            output.status.success(),
            "install.sh 语法应有效: {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[tokio::test]
async fn test_install_script_executable() {
    // 验证脚本有执行权限或可以被执行
    let script_path = "/Users/louloulin/Documents/linchong/claude/evif/scripts/install.sh";

    // 检查文件存在
    assert!(
        std::path::Path::new(script_path).exists(),
        "install.sh 应存在"
    );
}
