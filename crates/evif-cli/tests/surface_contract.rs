use std::process::Command;

fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn test_cli_help_does_not_expose_graph_commands() {
    let output = Command::new("cargo")
        .args(["run", "-p", "evif-cli", "--bin", "evif", "--", "--help"])
        .current_dir(workspace_root())
        .output()
        .expect("Failed to execute evif");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
    let command_lines: Vec<String> = stdout
        .lines()
        .map(|line| line.trim_start().to_string())
        .collect();

    assert!(!command_lines.iter().any(|line| line.starts_with("query")));
    assert!(!command_lines.iter().any(|line| line.starts_with("get ")));
    assert!(!command_lines.iter().any(|line| line == "get"));
    assert!(!command_lines.iter().any(|line| line.starts_with("create ")));
    assert!(!command_lines.iter().any(|line| line == "create"));
    assert!(!command_lines.iter().any(|line| line.starts_with("delete ")));
    assert!(!command_lines.iter().any(|line| line == "delete"));
}
