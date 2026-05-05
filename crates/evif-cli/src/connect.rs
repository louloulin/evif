// EVIF Connect - AI Platform Integration
//
// Connect EVIF as an MCP server to AI agent platforms.
// Supports: claude, claude-code, cursor, gemini, codex

use anyhow::{anyhow, Result};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

/// MCP server entry for EVIF
fn evif_mcp_entry() -> serde_json::Value {
    json!({
        "command": "evif",
        "args": ["mcp", "serve"],
        "env": {
            "EVIF_REST_URL": "http://localhost:8081"
        }
    })
}

/// Platform descriptor
struct PlatformInfo {
    name: &'static str,
    display_name: &'static str,
    method: &'static str,
    config_path: fn() -> Option<PathBuf>,
}

/// Get all supported platforms
fn platforms() -> Vec<PlatformInfo> {
    vec![
        PlatformInfo {
            name: "claude",
            display_name: "Claude Desktop",
            method: "MCP",
            config_path: || {
                dirs::home_dir().map(|h| {
                    h.join("Library/Application Support/Claude/claude_desktop_config.json")
                })
            },
        },
        PlatformInfo {
            name: "claude-code",
            display_name: "Claude Code",
            method: "MCP+Hook",
            config_path: || dirs::home_dir().map(|h| h.join(".claude/settings.json")),
        },
        PlatformInfo {
            name: "cursor",
            display_name: "Cursor",
            method: "MCP",
            config_path: || dirs::home_dir().map(|h| h.join(".cursor/mcp.json")),
        },
        PlatformInfo {
            name: "gemini",
            display_name: "Gemini CLI",
            method: "MCP",
            config_path: || dirs::home_dir().map(|h| h.join(".gemini/settings.json")),
        },
        PlatformInfo {
            name: "codex",
            display_name: "OpenAI Codex",
            method: "Rules",
            config_path: || dirs::home_dir().map(|h| h.join(".codex/EVIF.md")),
        },
    ]
}

/// Find a platform by name
fn find_platform(name: &str) -> Option<PlatformInfo> {
    platforms()
        .into_iter()
        .find(|p| p.name == name)
        .map(|p| PlatformInfo {
            name: p.name,
            display_name: p.display_name,
            method: p.method,
            config_path: p.config_path,
        })
}

/// List all supported platforms
pub fn list_platforms() {
    println!("{:<15} {:<12} {}", "Platform", "Method", "Config Path");
    println!("{}", "-".repeat(70));
    for p in platforms() {
        let path_str = match (p.config_path)() {
            Some(path) => path.display().to_string(),
            None => "(not available)".to_string(),
        };
        println!("{:<15} {:<12} {}", p.name, p.method, path_str);
    }
}

/// Patch a JSON config file to add EVIF MCP server entry.
/// Creates backup, ensures idempotency, uses atomic write.
fn patch_json_config(path: &PathBuf) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("No parent directory for {}", path.display()))?;
    fs::create_dir_all(parent)?;

    // Read existing config or create empty
    let mut config: serde_json::Value = if path.exists() {
        let content = fs::read_to_string(path)?;
        if content.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str(&content)?
        }
    } else {
        json!({})
    };

    // Check if already connected (idempotent)
    if config
        .get("mcpServers")
        .and_then(|s| s.get("evif"))
        .is_some()
    {
        println!(
            "  ✓ Already connected to {} (evif entry exists)",
            path.display()
        );
        return Ok(());
    }

    // Backup existing config
    if path.exists() {
        let backup = path.with_extension("json.bak");
        fs::copy(path, &backup)?;
        println!("  Backup: {}", backup.display());
    }

    // Ensure mcpServers object exists and insert evif
    if config.get("mcpServers").is_none() {
        config["mcpServers"] = json!({});
    }
    config["mcpServers"]["evif"] = evif_mcp_entry();

    // Atomic write: temp file + rename
    let tmp_path = path.with_extension("json.tmp");
    let json_str = serde_json::to_string_pretty(&config)?;
    fs::write(&tmp_path, json_str)?;
    fs::rename(&tmp_path, path)?;

    println!("  ✓ Connected: {}", path.display());
    Ok(())
}

/// Remove EVIF MCP server entry from a JSON config file.
fn unpatch_json_config(path: &PathBuf) -> Result<()> {
    if !path.exists() {
        println!("  Config not found: {} (skipped)", path.display());
        return Ok(());
    }

    let content = fs::read_to_string(path)?;
    let mut config: serde_json::Value = serde_json::from_str(&content)?;

    if config
        .get("mcpServers")
        .and_then(|s| s.get("evif"))
        .is_none()
    {
        println!("  Not connected: {} (no evif entry)", path.display());
        return Ok(());
    }

    // Backup
    let backup = path.with_extension("json.bak");
    fs::copy(path, &backup)?;

    // Remove evif entry
    if let Some(servers) = config.get_mut("mcpServers") {
        if let Some(obj) = servers.as_object_mut() {
            obj.remove("evif");
        }
    }

    // Atomic write
    let tmp_path = path.with_extension("json.tmp");
    let json_str = serde_json::to_string_pretty(&config)?;
    fs::write(&tmp_path, json_str)?;
    fs::rename(&tmp_path, path)?;

    println!("  ✓ Disconnected: {}", path.display());
    Ok(())
}

/// Check if EVIF is connected to a platform
fn check_connection(path: &PathBuf) -> bool {
    if !path.exists() {
        return false;
    }
    let content = fs::read_to_string(path).unwrap_or_default();
    let config: serde_json::Value = serde_json::from_str(&content).unwrap_or(json!({}));
    config
        .get("mcpServers")
        .and_then(|s| s.get("evif"))
        .is_some()
}

/// Connect EVIF to a specific platform
fn connect_platform(name: &str) -> Result<()> {
    let platform = find_platform(name)
        .ok_or_else(|| anyhow!("Unknown platform '{}'. Use --list to see supported platforms.", name))?;

    let config_path = (platform.config_path)()
        .ok_or_else(|| anyhow!("Cannot determine config path for {}", platform.display_name))?;

    println!("Connecting to {}...", platform.display_name);

    if name == "codex" {
        // Codex uses rules file, not JSON MCP config
        patch_rules_file(&config_path)?;
    } else {
        patch_json_config(&config_path)?;
    }

    println!(
        "\nDone! Restart {} to activate EVIF MCP Server.",
        platform.display_name
    );
    Ok(())
}

/// Disconnect EVIF from a specific platform
fn disconnect_platform(name: &str) -> Result<()> {
    let platform = find_platform(name)
        .ok_or_else(|| anyhow!("Unknown platform '{}'. Use --list to see supported platforms.", name))?;

    let config_path = (platform.config_path)()
        .ok_or_else(|| anyhow!("Cannot determine config path for {}", platform.display_name))?;

    println!("Disconnecting from {}...", platform.display_name);

    if name == "codex" {
        unpatch_rules_file(&config_path)?;
    } else {
        unpatch_json_config(&config_path)?;
    }

    println!("\nDone! EVIF removed from {}.", platform.display_name);
    Ok(())
}

/// Patch a rules file (for Codex and similar platforms)
fn patch_rules_file(path: &PathBuf) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("No parent directory"))?;
    fs::create_dir_all(parent)?;

    let evif_section = r#"
## EVIF MCP Integration
Use EVIF MCP tools for persistent context, memory, and file operations:
- `evif cat <path>` - Read files
- `evif write <path> <content>` - Write files
- `evif memory memorize <text>` - Store memories
- `evif memory search <query>` - Search memories
- `evif context read L0` - Read current task context
"#;

    if path.exists() {
        let content = fs::read_to_string(path)?;
        if content.contains("EVIF MCP Integration") {
            println!("  Already connected: {}", path.display());
            return Ok(());
        }
        // Backup
        let backup = path.with_extension("md.bak");
        fs::copy(path, &backup)?;
        println!("  Backup: {}", backup.display());
        // Append
        let mut new_content = content;
        new_content.push_str(evif_section);
        let tmp_path = path.with_extension("md.tmp");
        fs::write(&tmp_path, new_content)?;
        fs::rename(&tmp_path, path)?;
    } else {
        fs::write(path, evif_section.trim_start())?;
    }

    println!("  Connected: {}", path.display());
    Ok(())
}

/// Remove EVIF section from a rules file
fn unpatch_rules_file(path: &PathBuf) -> Result<()> {
    if !path.exists() {
        println!("  File not found: {} (skipped)", path.display());
        return Ok(());
    }

    let content = fs::read_to_string(path)?;
    if !content.contains("EVIF MCP Integration") {
        println!("  Not connected: {} (no EVIF section)", path.display());
        return Ok(());
    }

    // Backup
    let backup = path.with_extension("md.bak");
    fs::copy(path, &backup)?;

    // Remove EVIF section - find and remove everything from "## EVIF MCP Integration" onwards
    let new_content = if let Some(idx) = content.find("## EVIF MCP Integration") {
        // Get content before the EVIF section, trimming any trailing newlines
        let before = &content[..idx];
        before.trim_end().to_string()
    } else {
        content
    };

    let tmp_path = path.with_extension("md.tmp");
    fs::write(&tmp_path, new_content)?;
    fs::rename(&tmp_path, path)?;

    println!("  Disconnected: {}", path.display());
    Ok(())
}

/// Check all platform connections
fn check_all() {
    println!("{:<15} {:<10} {}", "Platform", "Status", "Config Path");
    println!("{}", "-".repeat(60));
    for p in platforms() {
        let path_str = match (p.config_path)() {
            Some(path) => path.display().to_string(),
            None => "(not available)".to_string(),
        };
        let connected = (p.config_path)().map(|p| check_connection(&p)).unwrap_or(false);
        let status = if connected { "✓ Connected" } else { "✗ Not connected" };
        println!("{:<15} {:<10} {}", p.name, status, path_str);
    }
}

/// Main entry point for the connect command
pub fn handle_connect(
    platform: Option<&str>,
    list: bool,
    check: bool,
    disconnect: bool,
) -> Result<()> {
    if list {
        list_platforms();
        return Ok(());
    }

    if check {
        check_all();
        return Ok(());
    }

    let name = platform.ok_or_else(|| {
        anyhow!("Specify a platform name. Use --list to see supported platforms.\n\nExamples:\n  evif connect claude\n  evif connect cursor\n  evif connect gemini")
    })?;

    if disconnect {
        disconnect_platform(name)
    } else {
        connect_platform(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_patch_json_config_creates_new() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.json");

        patch_json_config(&config_path).unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        let config: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert!(config.get("mcpServers").unwrap().get("evif").is_some());
        let evif = config["mcpServers"]["evif"].clone();
        assert_eq!(evif["command"], "evif");
        assert_eq!(evif["args"], json!(["mcp", "serve"]));
    }

    #[test]
    fn test_patch_json_config_idempotent() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.json");

        patch_json_config(&config_path).unwrap();
        patch_json_config(&config_path).unwrap(); // Should not fail or duplicate

        let content = fs::read_to_string(&config_path).unwrap();
        let config: serde_json::Value = serde_json::from_str(&content).unwrap();
        let servers = config["mcpServers"].as_object().unwrap();
        assert_eq!(servers.keys().filter(|k| *k == "evif").count(), 1);
    }

    #[test]
    fn test_patch_json_config_preserves_existing() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.json");

        // Write existing config with another MCP server
        let existing = json!({
            "mcpServers": {
                "github": {
                    "command": "github-mcp-server"
                }
            }
        });
        fs::write(&config_path, serde_json::to_string_pretty(&existing).unwrap()).unwrap();

        patch_json_config(&config_path).unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        let config: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Both github and evif should exist
        assert!(config["mcpServers"]["github"].is_object());
        assert!(config["mcpServers"]["evif"].is_object());

        // Backup should exist
        assert!(config_path.with_extension("json.bak").exists());
    }

    #[test]
    fn test_unpatch_json_config() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.json");

        patch_json_config(&config_path).unwrap();
        assert!(check_connection(&config_path));

        unpatch_json_config(&config_path).unwrap();
        assert!(!check_connection(&config_path));
    }

    #[test]
    fn test_unpatch_json_config_no_evif_entry() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.json");

        let existing = json!({"mcpServers": {"github": {"command": "test"}}});
        fs::write(&config_path, serde_json::to_string_pretty(&existing).unwrap()).unwrap();

        unpatch_json_config(&config_path).unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        let config: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(config["mcpServers"]["github"].is_object());
        assert!(config["mcpServers"].get("evif").is_none());
    }

    #[test]
    fn test_patch_rules_file() {
        let dir = TempDir::new().unwrap();
        let rules_path = dir.path().join("AGENTS.md");

        patch_rules_file(&rules_path).unwrap();

        let content = fs::read_to_string(&rules_path).unwrap();
        assert!(content.contains("EVIF MCP Integration"));
        assert!(content.contains("evif cat"));
    }

    #[test]
    fn test_patch_rules_file_idempotent() {
        let dir = TempDir::new().unwrap();
        let rules_path = dir.path().join("AGENTS.md");

        patch_rules_file(&rules_path).unwrap();
        patch_rules_file(&rules_path).unwrap(); // Should not duplicate

        let content = fs::read_to_string(&rules_path).unwrap();
        let count = content.matches("EVIF MCP Integration").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_unpatch_rules_file() {
        let dir = TempDir::new().unwrap();
        let rules_path = dir.path().join("AGENTS.md");

        patch_rules_file(&rules_path).unwrap();
        assert!(fs::read_to_string(&rules_path).unwrap().contains("EVIF MCP Integration"));

        unpatch_rules_file(&rules_path).unwrap();
        assert!(!fs::read_to_string(&rules_path).unwrap().contains("EVIF MCP Integration"));
    }

    #[test]
    fn test_evif_mcp_entry_schema() {
        let entry = evif_mcp_entry();
        assert_eq!(entry["command"], "evif");
        assert_eq!(entry["args"], json!(["mcp", "serve"]));
        assert!(entry["env"]["EVIF_REST_URL"].is_string());
    }

    #[test]
    fn test_find_platform() {
        assert!(find_platform("claude").is_some());
        assert!(find_platform("claude-code").is_some());
        assert!(find_platform("cursor").is_some());
        assert!(find_platform("gemini").is_some());
        assert!(find_platform("codex").is_some());
        assert!(find_platform("unknown").is_none());
    }
}
