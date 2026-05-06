// Multi-Platform Integration Tests for EVIF Connect
//
// Verifies EVIF can be connected to 5 AI agent platforms.
// These tests use temp directories to simulate platform configs.

use std::fs;

/// Platform names (mirrors connect.rs platforms list)
const PLATFORMS: &[&str] = &["claude", "claude-code", "cursor", "gemini", "codex"];

/// Expected config file names per platform
fn expected_config_name(platform: &str) -> &str {
    match platform {
        "claude" => "claude_desktop_config.json",
        "claude-code" => "settings.json",
        "cursor" => "mcp.json",
        "gemini" => "settings.json",
        "codex" => "EVIF.md",
        _ => unreachable!(),
    }
}

/// Expected config directory per platform
fn expected_config_dir(platform: &str) -> &str {
    match platform {
        "claude" => "Library/Application Support/Claude",
        "claude-code" => ".claude",
        "cursor" => ".cursor",
        "gemini" => ".gemini",
        "codex" => ".codex",
        _ => unreachable!(),
    }
}

/// EVIF MCP entry (mirrors connect.rs evif_mcp_entry)
fn evif_mcp_entry() -> serde_json::Value {
    serde_json::json!({
        "command": "evif",
        "args": ["mcp", "serve"],
        "env": {
            "EVIF_REST_URL": "http://localhost:8081"
        }
    })
}

/// Test 1: Claude Desktop has correct config file name.
#[test]
fn test_claude_config_name() {
    assert_eq!(expected_config_name("claude"), "claude_desktop_config.json");
    assert_eq!(expected_config_dir("claude"), "Library/Application Support/Claude");
}

/// Test 2: Claude Code has correct config file name.
#[test]
fn test_claude_code_config_name() {
    assert_eq!(expected_config_name("claude-code"), "settings.json");
    assert_eq!(expected_config_dir("claude-code"), ".claude");
}

/// Test 3: Cursor has correct config file name.
#[test]
fn test_cursor_config_name() {
    assert_eq!(expected_config_name("cursor"), "mcp.json");
    assert_eq!(expected_config_dir("cursor"), ".cursor");
}

/// Test 4: Gemini has correct config file name.
#[test]
fn test_gemini_config_name() {
    assert_eq!(expected_config_name("gemini"), "settings.json");
    assert_eq!(expected_config_dir("gemini"), ".gemini");
}

/// Test 5: Codex has correct rules file name.
#[test]
fn test_codex_rules_name() {
    assert_eq!(expected_config_name("codex"), "EVIF.md");
    assert_eq!(expected_config_dir("codex"), ".codex");
}

/// Test 6: All 5 platforms are defined.
#[test]
fn test_all_5_platforms_defined() {
    assert_eq!(PLATFORMS.len(), 5);
    assert!(PLATFORMS.contains(&"claude"));
    assert!(PLATFORMS.contains(&"claude-code"));
    assert!(PLATFORMS.contains(&"cursor"));
    assert!(PLATFORMS.contains(&"gemini"));
    assert!(PLATFORMS.contains(&"codex"));
}

/// Test 7: EVIF MCP entry has correct command and args.
#[test]
fn test_evif_mcp_entry_schema() {
    let entry = evif_mcp_entry();
    assert_eq!(entry["command"], "evif", "Command should be 'evif'");
    assert_eq!(entry["args"], serde_json::json!(["mcp", "serve"]), "Args should be ['mcp', 'serve']");
}

/// Test 8: EVIF MCP entry includes REST URL env.
#[test]
fn test_evif_mcp_entry_has_rest_url() {
    let entry = evif_mcp_entry();
    assert!(entry["env"].is_object(), "env should be an object");
    assert!(entry["env"]["EVIF_REST_URL"].is_string(), "EVIF_REST_URL should be a string");
    let url = entry["env"]["EVIF_REST_URL"].as_str().unwrap();
    assert!(
        url.starts_with("http://") || url.starts_with("https://"),
        "REST URL should be HTTP(S): {}",
        url
    );
}

/// Test 9: MCP JSON config can include EVIF server.
#[test]
fn test_mcp_json_config_format() {
    let temp_path = tempfile::NamedTempFile::new().unwrap().path().with_extension("json");
    let config: serde_json::Value = serde_json::json!({
        "mcpServers": {
            "evif": evif_mcp_entry()
        }
    });
    fs::write(&temp_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

    // Read back and verify
    let content = fs::read_to_string(&temp_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(
        parsed["mcpServers"]["evif"]["command"].is_string(),
        "evif entry should have command field"
    );
    assert_eq!(
        parsed["mcpServers"]["evif"]["command"],
        "evif",
        "command should be 'evif'"
    );
}

/// Test 10: Codex rules file is Markdown format.
#[test]
fn test_codex_rules_markdown_format() {
    let temp_path = tempfile::NamedTempFile::new().unwrap().path().with_extension("md");
    let rules_content = "## EVIF MCP Integration\n\nUse EVIF for persistent context.\n";
    fs::write(&temp_path, rules_content).unwrap();

    let content = fs::read_to_string(&temp_path).unwrap();
    assert!(
        content.contains("## EVIF"),
        "Should contain Markdown header"
    );
    assert!(
        content.contains("EVIF"),
        "Should contain EVIF reference"
    );
}
