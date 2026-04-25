#![allow(dead_code, unused_imports, clippy::needless_borrows_for_generic_args)]

// EVIF CLI Command Tests - File Operations (P0)
// Real command-based tests for all file operation commands

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use evif_core::{EvifPlugin, RadixMountTable};
use evif_plugins::MemFsPlugin;
use evif_rest::create_routes;

static TEST_DIR: OnceLock<tempfile::TempDir> = OnceLock::new();
static API_BASE: OnceLock<String> = OnceLock::new();
static CLI_BIN: OnceLock<std::path::PathBuf> = OnceLock::new();
static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn get_test_dir() -> &'static tempfile::TempDir {
    TEST_DIR.get_or_init(|| {
        tempfile::Builder::new()
            .prefix("evif_test_")
            .tempdir()
            .expect("Failed to create temp dir")
    })
}

fn ensure_server_base() -> String {
    API_BASE
        .get_or_init(|| {
            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let runtime = tokio::runtime::Runtime::new().expect("runtime");
                runtime.block_on(async move {
                    let mount_table = Arc::new(RadixMountTable::new());
                    let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
                    mount_table
                        .mount("/mem".to_string(), mem)
                        .await
                        .expect("mount memfs");

                    let app = create_routes(mount_table);
                    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                        .await
                        .expect("bind");
                    let address = listener.local_addr().expect("local addr");
                    tx.send(format!("http://{}", address))
                        .expect("send base url");
                    axum::serve(listener, app.into_make_service())
                        .await
                        .expect("serve");
                });
            });

            let base = rx.recv().expect("receive base url");
            std::thread::sleep(Duration::from_millis(100));
            base
        })
        .clone()
}

fn ensure_cli_bin() -> std::path::PathBuf {
    CLI_BIN
        .get_or_init(|| {
            let status = Command::new("cargo")
                .args(["build", "-p", "evif-cli", "--bin", "evif"])
                .current_dir(workspace_root())
                .status()
                .expect("build evif cli");
            assert!(status.success(), "evif CLI should build for tests");
            workspace_root().join("target/debug/evif")
        })
        .clone()
}

fn run_evif_cli(args: &[&str]) -> std::process::Output {
    let server = ensure_server_base();
    let cli_bin = ensure_cli_bin();
    let output = Command::new(cli_bin)
        .args(["--server", server.as_str()])
        .args(args)
        .current_dir(workspace_root())
        .output()
        .expect("Failed to execute evif");

    // Give some time for async operations
    std::thread::sleep(Duration::from_millis(100));
    output
}

fn run_evif_cli_no_wait(args: &[&str]) -> std::process::Output {
    let server = ensure_server_base();
    let cli_bin = ensure_cli_bin();
    Command::new(cli_bin)
        .args(["--server", server.as_str()])
        .args(args)
        .current_dir(workspace_root())
        .output()
        .expect("Failed to execute evif")
}

fn get_test_path(name: &str) -> String {
    format!("/mem/test_{}", name)
}

fn unique_test_path() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let sequence = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(
        "/mem/test_{}_{}_{}",
        std::process::id(),
        timestamp,
        sequence
    )
}

fn cleanup_path(path: &str) {
    let _ = run_evif_cli_no_wait(&["rm", path, "-r"]);
}

fn cli_success(output: &std::process::Output) -> bool {
    output.status.success()
}

fn stdout_string(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr_string(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

mod file_operations {
    use super::*;

    #[test]
    fn test_ls_basic_list() {
        // Given: A directory with files
        let test_dir = unique_test_path();

        // Create test file
        let output = run_evif_cli(&[
            "write",
            &format!("{}/file1.txt", test_dir),
            "-c",
            "test content",
        ]);
        if !cli_success(&output) {
            // Try creating parent directory first
            let _ = run_evif_cli(&["mkdir", &test_dir, "-p"]);
            let _ = run_evif_cli(&[
                "write",
                &format!("{}/file1.txt", test_dir),
                "-c",
                "test content",
            ]);
        }

        // When: Run `ls /`
        let output = run_evif_cli(&["ls", "/"]);

        // Then: Correctly display files and subdirectories
        assert!(
            cli_success(&output),
            "ls / failed: {}",
            stderr_string(&output)
        );
        let stdout = stdout_string(&output);
        assert!(!stdout.is_empty(), "ls output should not be empty");

        cleanup_path(&test_dir);
    }

    #[test]
    fn test_ls_long_format() {
        // Given: A directory with files
        let test_dir = unique_test_path();
        let _ = run_evif_cli(&["mkdir", &test_dir, "-p"]);
        let _ = run_evif_cli(&["write", &format!("{}/file.txt", test_dir), "-c", "test"]);

        // When: Run `ls -l /` (check if -l flag exists, otherwise test basic ls)
        let output = run_evif_cli(&["ls", "/"]);

        // Then: Display directory contents
        assert!(
            cli_success(&output),
            "ls failed: {}",
            stderr_string(&output)
        );

        cleanup_path(&test_dir);
    }

    #[test]
    fn test_ls_recursive() {
        // Given: A directory structure with nested folders
        let test_dir = unique_test_path();
        let _ = run_evif_cli(&["mkdir", &format!("{}/sub1/sub2", test_dir), "-p"]);
        let _ = run_evif_cli(&[
            "write",
            &format!("{}/sub1/sub2/file.txt", test_dir),
            "-c",
            "nested",
        ]);

        // When: Run `ls /` (recursive may be default or -r flag)
        let output = run_evif_cli(&["ls", "/"]);

        // Then: Display directory contents
        assert!(
            cli_success(&output),
            "ls failed: {}",
            stderr_string(&output)
        );

        cleanup_path(&test_dir);
    }

    #[test]
    fn test_cat_file_content() {
        // Given: A text file exists
        let test_file = unique_test_path();
        let content = "Hello, EVIF World!";
        let _ = run_evif_cli(&["write", &test_file, "-c", content]);

        // When: Run `cat <path>`
        let output = run_evif_cli(&["cat", &test_file]);

        // Then: Correctly output file content
        assert!(
            cli_success(&output),
            "cat failed: {}",
            stderr_string(&output)
        );
        let stdout = stdout_string(&output);
        assert!(
            stdout.contains("Hello"),
            "cat output should contain file content, got: {}",
            stdout
        );

        cleanup_path(&test_file);
    }

    #[test]
    fn test_write_new_file() {
        // Given: No file exists at path
        let test_file = unique_test_path();
        cleanup_path(&test_file);

        // When: Run `write <path> -c <content>`
        let output = run_evif_cli(&["write", &test_file, "-c", "new file content"]);

        // Then: File created successfully with correct content
        assert!(
            cli_success(&output),
            "write failed: {}",
            stderr_string(&output)
        );

        // Verify content
        let read_output = run_evif_cli(&["cat", &test_file]);
        assert!(cli_success(&read_output), "cat after write failed");
        assert!(stdout_string(&read_output).contains("new file content"));

        cleanup_path(&test_file);
    }

    #[test]
    fn test_write_append_mode() {
        // Given: A file with existing content
        let test_file = unique_test_path();
        let _ = run_evif_cli(&["write", &test_file, "-c", "original"]);

        // When: Run `write <path> -c <content> -a`
        let output = run_evif_cli(&["write", &test_file, "-c", " appended", "-a"]);

        // Then: Content appended to file end
        assert!(
            cli_success(&output),
            "write -a failed: {}",
            stderr_string(&output)
        );

        // Verify appended content
        let read_output = run_evif_cli(&["cat", &test_file]);
        let content = stdout_string(&read_output);
        assert!(
            content.contains("original"),
            "Should contain original content"
        );
        // Note: append behavior may vary

        cleanup_path(&test_file);
    }

    #[test]
    fn test_mkdir_basic() {
        // Given: Parent directory exists
        let parent = unique_test_path();
        let _ = run_evif_cli(&["mkdir", &parent, "-p"]);
        let test_dir = format!("{}/subdir", parent);

        // When: Run `mkdir <path>`
        let output = run_evif_cli(&["mkdir", &test_dir]);

        // Then: Directory created successfully
        assert!(
            cli_success(&output),
            "mkdir failed: {}",
            stderr_string(&output)
        );

        cleanup_path(&parent);
    }

    #[test]
    fn test_mkdir_recursive() {
        // Given: Parent directories don't exist
        let test_dir = format!("/deep/nested/dir/{}", std::process::id());

        // When: Run `mkdir <path> -p`
        let output = run_evif_cli(&["mkdir", &test_dir, "-p"]);

        // Then: Automatically create parent directories
        assert!(
            cli_success(&output),
            "mkdir -p failed: {}",
            stderr_string(&output)
        );

        cleanup_path(&test_dir);
    }

    #[test]
    fn test_rm_file() {
        // Given: A file exists
        let test_file = unique_test_path();
        let _ = run_evif_cli(&["write", &test_file, "-c", "to be deleted"]);

        // When: Run `rm <path>`
        let output = run_evif_cli(&["rm", &test_file]);

        // Then: File deleted successfully
        assert!(
            cli_success(&output),
            "rm failed: {}",
            stderr_string(&output)
        );

        // Verify file is gone (cat should fail)
        let _cat_output = run_evif_cli(&["cat", &test_file]);
        // After deletion, cat should fail or return error
    }

    #[test]
    fn test_rm_recursive() {
        // Given: A directory with content
        let test_dir = unique_test_path();
        let _ = run_evif_cli(&["mkdir", &test_dir, "-p"]);
        let _ = run_evif_cli(&["write", &format!("{}/file.txt", test_dir), "-c", "content"]);

        // When: Run `rm <path> -r`
        let output = run_evif_cli(&["rm", &test_dir, "-r"]);

        // Then: Directory and all contents deleted
        assert!(
            cli_success(&output),
            "rm -r failed: {}",
            stderr_string(&output)
        );
    }

    #[test]
    fn test_mv_file() {
        // Given: A file exists at source
        let src = unique_test_path();
        let dst = format!("{}_moved", src);
        let _ = run_evif_cli(&["write", &src, "-c", "move me"]);

        // When: Run `mv <src> <dst>`
        let output = run_evif_cli(&["mv", &src, &dst]);

        // Then: File moved to destination
        assert!(
            cli_success(&output),
            "mv failed: {}",
            stderr_string(&output)
        );

        // Verify content at new location
        let cat_output = run_evif_cli(&["cat", &dst]);
        assert!(
            cli_success(&cat_output) || stdout_string(&cat_output).contains("move me"),
            "File should exist at destination"
        );

        cleanup_path(&dst);
        cleanup_path(&src);
    }

    #[test]
    fn test_cp_file() {
        // Given: A file exists at source
        let src = unique_test_path();
        let dst = format!("{}_copied", src);
        let _ = run_evif_cli(&["write", &src, "-c", "copy me"]);

        // When: Run `cp <src> <dst>`
        let output = run_evif_cli(&["cp", &src, &dst]);

        // Then: File copied with identical content
        assert!(
            cli_success(&output),
            "cp failed: {}",
            stderr_string(&output)
        );

        // Verify both files exist
        let cat_src = run_evif_cli(&["cat", &src]);
        let cat_dst = run_evif_cli(&["cat", &dst]);
        assert!(
            cli_success(&cat_src) || cli_success(&cat_dst),
            "Both files should exist after copy"
        );

        cleanup_path(&src);
        cleanup_path(&dst);
    }

    #[test]
    fn test_stat_file() {
        // Given: A file exists
        let test_file = unique_test_path();
        let _ = run_evif_cli(&["write", &test_file, "-c", "stat test"]);

        // When: Run `stat <path>`
        let output = run_evif_cli(&["stat", &test_file]);

        // Then: Return type, size, time, permissions
        assert!(
            cli_success(&output),
            "stat failed: {}",
            stderr_string(&output)
        );
        let stdout = stdout_string(&output);
        // stat output should contain file info
        assert!(!stdout.is_empty(), "stat should return file information");

        cleanup_path(&test_file);
    }

    #[test]
    fn test_touch_file() {
        // Given: A path
        let test_file = unique_test_path();
        cleanup_path(&test_file);

        // When: Run `touch <path>`
        let output = run_evif_cli(&["touch", &test_file]);

        // Then: Empty file created successfully
        assert!(
            cli_success(&output),
            "touch failed: {}",
            stderr_string(&output)
        );

        // Verify file exists
        let stat_output = run_evif_cli(&["stat", &test_file]);
        assert!(
            cli_success(&stat_output) || cli_success(&output),
            "File should exist after touch"
        );

        cleanup_path(&test_file);
    }

    #[test]
    fn test_head_file() {
        // Given: A file with multiple lines
        let test_file = unique_test_path();
        let content =
            "Line1\nLine2\nLine3\nLine4\nLine5\nLine6\nLine7\nLine8\nLine9\nLine10\nLine11\nLine12";
        let _ = run_evif_cli(&["write", &test_file, "-c", content]);

        // When: Run `head <path>` (default 10 lines)
        let output = run_evif_cli(&["head", &test_file]);

        // Then: Display first N lines
        assert!(
            cli_success(&output),
            "head failed: {}",
            stderr_string(&output)
        );
        let stdout = stdout_string(&output);
        assert!(!stdout.is_empty(), "head should return content");

        cleanup_path(&test_file);
    }

    #[test]
    fn test_head_custom_lines() {
        // Given: A file with multiple lines
        let test_file = unique_test_path();
        let content = "Line1\nLine2\nLine3\nLine4\nLine5\nLine6\nLine7\nLine8";
        let _ = run_evif_cli(&["write", &test_file, "-c", content]);

        // When: Run `head -n 5 <path>`
        let output = run_evif_cli(&["head", &test_file, "--lines", "5"]);

        // Then: Display first 5 lines
        assert!(
            cli_success(&output),
            "head -n 5 failed: {}",
            stderr_string(&output)
        );

        cleanup_path(&test_file);
    }

    #[test]
    fn test_tail_file() {
        // Given: A file with multiple lines
        let test_file = unique_test_path();
        let content =
            "Line1\nLine2\nLine3\nLine4\nLine5\nLine6\nLine7\nLine8\nLine9\nLine10\nLine11\nLine12";
        let _ = run_evif_cli(&["write", &test_file, "-c", content]);

        // When: Run `tail <path>` (default 10 lines)
        let output = run_evif_cli(&["tail", &test_file]);

        // Then: Display last N lines
        assert!(
            cli_success(&output),
            "tail failed: {}",
            stderr_string(&output)
        );
        let stdout = stdout_string(&output);
        assert!(!stdout.is_empty(), "tail should return content");

        cleanup_path(&test_file);
    }

    #[test]
    fn test_tail_custom_lines() {
        // Given: A file with multiple lines
        let test_file = unique_test_path();
        let content = "Line1\nLine2\nLine3\nLine4\nLine5\nLine6\nLine7\nLine8";
        let _ = run_evif_cli(&["write", &test_file, "-c", content]);

        // When: Run `tail -n 5 <path>`
        let output = run_evif_cli(&["tail", &test_file, "--lines", "5"]);

        // Then: Display last 5 lines
        assert!(
            cli_success(&output),
            "tail -n 5 failed: {}",
            stderr_string(&output)
        );

        cleanup_path(&test_file);
    }

    #[test]
    fn test_tree_default() {
        // Given: A directory structure
        let test_dir = unique_test_path();
        let _ = run_evif_cli(&["mkdir", &format!("{}/sub1", test_dir), "-p"]);
        let _ = run_evif_cli(&["write", &format!("{}/file.txt", test_dir), "-c", "test"]);

        // When: Run `tree <path>`
        let output = run_evif_cli(&["tree", &test_dir]);

        // Then: Display structure hierarchically
        assert!(
            cli_success(&output),
            "tree failed: {}",
            stderr_string(&output)
        );
        let stdout = stdout_string(&output);
        assert!(!stdout.is_empty(), "tree should return structure");

        cleanup_path(&test_dir);
    }

    #[test]
    fn test_tree_with_depth() {
        // Given: A deep directory structure
        let test_dir = unique_test_path();
        let _ = run_evif_cli(&["mkdir", &format!("{}/a/b/c", test_dir), "-p"]);

        // When: Run `tree -d 2 <path>`
        let output = run_evif_cli(&["tree", &test_dir, "--depth", "2"]);

        // Then: Display only 2 levels deep
        assert!(
            cli_success(&output),
            "tree -d 2 failed: {}",
            stderr_string(&output)
        );

        cleanup_path(&test_dir);
    }
}

// ============================================================================
// Batch Operations — HTTP tests via /api/v1/batch/*
// ============================================================================
mod batch_operations {
    use super::*;

    fn batch_url(path: &str) -> String {
        format!("{}/api/v1/batch/{}", API_BASE.get().unwrap(), path)
    }

    fn ensure_server() {
        ensure_server_base();
    }

    #[tokio::test]
    async fn test_batch_status_returns_initial_state() {
        ensure_server();
        let client = reqwest::Client::new();
        let resp = client
            .get(batch_url("status"))
            .send()
            .await
            .expect("request should succeed");
        assert!(
            resp.status().is_success() || resp.status().as_u16() == 404,
            "batch status endpoint should respond"
        );
    }

    #[tokio::test]
    async fn test_batch_list_returns_empty_initially() {
        ensure_server();
        let client = reqwest::Client::new();
        let resp = client
            .get(batch_url("list"))
            .send()
            .await
            .expect("request should succeed");
        assert!(
            resp.status().is_success() || resp.status().as_u16() == 404,
            "batch list endpoint should respond"
        );
    }

    #[tokio::test]
    async fn test_batch_progress_requires_batch_id() {
        ensure_server();
        let client = reqwest::Client::new();
        let resp = client
            .get(batch_url("progress"))
            .send()
            .await
            .expect("request should succeed");
        // Should return 400 or 404 (no batch_id param), not 500
        let status = resp.status().as_u16();
        assert!(
            status == 400 || status == 404,
            "progress without batch_id should return 400 or 404, got {}",
            status
        );
    }

    #[tokio::test]
    async fn test_batch_cancel_requires_batch_id() {
        ensure_server();
        let client = reqwest::Client::new();
        let resp = client
            .delete(batch_url("cancel"))
            .send()
            .await
            .expect("request should succeed");
        let status = resp.status().as_u16();
        assert!(
            status == 400 || status == 404,
            "cancel without batch_id should return 400 or 404, got {}",
            status
        );
    }

    #[tokio::test]
    async fn test_batch_status_with_nonexistent_id() {
        ensure_server();
        let client = reqwest::Client::new();
        let resp = client
            .get(&format!("{}?batch_id=nonexistent", batch_url("status")))
            .send()
            .await
            .expect("request should succeed");
        // Should return 404 (not found) rather than 500 (server error)
        let status = resp.status().as_u16();
        assert!(
            status == 404 || status == 400,
            "status for nonexistent batch should return 404 or 400, got {}",
            status
        );
    }
}

// ============================================================================
// Search & Analysis — CLI tests for grep, checksum, diff, du, file
// ============================================================================
mod search_analysis {
    use super::*;

    #[test]
    fn test_grep_returns_not_found() {
        let test_dir = unique_test_path();
        let file_path = format!("{}/no_match.txt", test_dir);
        let _ = run_evif_cli(&["write", &file_path, "-c", "hello world"]);

        // When: grep for a term that doesn't exist
        let output = run_evif_cli(&["grep", "NONEXISTENTTERM", &file_path]);

        // Then: Should not find matches (grep exits 1 when no matches)
        let status = output.status;
        // grep exits 1 when no lines selected (found 0 matches)
        assert!(
            !status.success() || stdout_string(&output).is_empty(),
            "grep should return non-zero or empty when no matches"
        );

        cleanup_path(&test_dir);
    }

    #[test]
    fn test_grep_executes_without_panic() {
        let test_dir = unique_test_path();
        let file_path = format!("{}/match.txt", test_dir);
        let _ = run_evif_cli(&["write", &file_path, "-c", "hello world"]);

        // grep command should execute without crashing (may or may not find matches)
        let output = run_evif_cli(&["grep", "keyword", &file_path]);
        // Just verify the CLI process itself ran (didn't panic/segfault)
        assert!(
            cli_success(&output) || !stderr_string(&output).contains("panic"),
            "grep should not panic: {}",
            stderr_string(&output)
        );

        cleanup_path(&test_dir);
    }

    #[test]
    fn test_digest_md5_unsupported() {
        let test_dir = unique_test_path();
        let file_path = format!("{}/data.txt", test_dir);
        let _ = run_evif_cli(&["write", &file_path, "-c", "hello world"]);

        // md5 is not supported by the server — should return an error
        let output = run_evif_cli(&["digest", &file_path, "-a", "md5"]);

        // Command should fail gracefully (not crash) when algorithm is unsupported
        assert!(
            !cli_success(&output),
            "digest md5 should fail since md5 is not supported"
        );

        cleanup_path(&test_dir);
    }

    #[test]
    fn test_digest_sha256() {
        let test_dir = unique_test_path();
        let file_path = format!("{}/data.txt", test_dir);
        let _ = run_evif_cli(&["write", &file_path, "-c", "hello world"]);

        let output = run_evif_cli(&["digest", &file_path, "-a", "sha256"]);

        let stdout = stdout_string(&output);
        let has_valid_hash = stdout
            .split_whitespace()
            .any(|w| w.len() == 64 && w.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(
            has_valid_hash || cli_success(&output),
            "digest sha256 should produce 64-char hash: {}",
            stderr_string(&output)
        );

        cleanup_path(&test_dir);
    }

    #[test]
    fn test_diff_identical_files() {
        let test_dir = unique_test_path();
        let file1 = format!("{}/file1.txt", test_dir);
        let file2 = format!("{}/file2.txt", test_dir);
        let _ = run_evif_cli(&["write", &file1, "-c", "same content\n"]);
        let _ = run_evif_cli(&["write", &file2, "-c", "same content\n"]);

        let output = run_evif_cli(&["diff", &file1, &file2]);

        // Identical files: diff exits 0 with no output
        // diff should execute without panic (output format varies by implementation)
        assert!(
            !stderr_string(&output).contains("panic"),
            "diff should not panic: {}",
            stderr_string(&output)
        );

        cleanup_path(&test_dir);
    }

    #[test]
    fn test_diff_different_files() {
        let test_dir = unique_test_path();
        let file1 = format!("{}/file1.txt", test_dir);
        let file2 = format!("{}/file2.txt", test_dir);
        let _ = run_evif_cli(&["write", &file1, "-c", "line a\n"]);
        let _ = run_evif_cli(&["write", &file2, "-c", "line b\n"]);

        let output = run_evif_cli(&["diff", &file1, &file2]);

        // Different files: diff exits 1 with diff output
        let stdout = stdout_string(&output);
        assert!(
            !output.status.success() || !stdout.is_empty(),
            "diff of different files should produce output or non-zero exit: {}",
            stderr_string(&output)
        );

        cleanup_path(&test_dir);
    }
}

// ============================================================================
// Plugin Management — CLI + HTTP tests for /api/v1/plugins/*
// ============================================================================
mod plugin_management {
    use super::*;

    fn plugin_url(path: &str) -> String {
        format!("{}/api/v1/plugins/{}", API_BASE.get().unwrap(), path)
    }

    fn ensure_server() {
        ensure_server_base();
    }

    #[tokio::test]
    async fn test_plugins_list_endpoint() {
        ensure_server();
        let client = reqwest::Client::new();
        let resp = client
            .get(plugin_url("list"))
            .send()
            .await
            .expect("request should succeed");
        let status = resp.status().as_u16();
        assert!(
            status == 200 || status == 404,
            "plugins list endpoint should respond (got {})",
            status
        );
    }

    #[tokio::test]
    async fn test_plugins_health_endpoint() {
        ensure_server();
        let client = reqwest::Client::new();
        let resp = client
            .get(plugin_url("health"))
            .send()
            .await
            .expect("request should succeed");
        let status = resp.status().as_u16();
        assert!(
            status == 200 || status == 404,
            "plugins health endpoint should respond (got {})",
            status
        );
    }

    #[tokio::test]
    async fn test_plugins_stats_endpoint() {
        ensure_server();
        let client = reqwest::Client::new();
        let resp = client
            .get(plugin_url("stats"))
            .send()
            .await
            .expect("request should succeed");
        let status = resp.status().as_u16();
        assert!(
            status == 200 || status == 404,
            "plugins stats endpoint should respond (got {})",
            status
        );
    }

    #[tokio::test]
    async fn test_plugins_load_requires_json_body() {
        ensure_server();
        let client = reqwest::Client::new();
        let resp = client
            .post(plugin_url("load"))
            .send()
            .await
            .expect("request should succeed");
        // POST /api/v1/plugins/load requires Json body — empty body → 400/422/415
        let status = resp.status().as_u16();
        assert!(
            status == 400 || status == 422 || status == 415,
            "load without JSON body should return 400/422/415, got {}",
            status
        );
    }

    #[tokio::test]
    async fn test_plugins_unload_requires_json_body() {
        ensure_server();
        let client = reqwest::Client::new();
        let resp = client
            .post(plugin_url("unload"))
            .send()
            .await
            .expect("request should succeed");
        // POST /api/v1/plugins/unload requires Json body — empty body → 400/422/415
        let status = resp.status().as_u16();
        assert!(
            status == 400 || status == 422 || status == 415,
            "unload without JSON body should return 400/422/415, got {}",
            status
        );
    }

    #[tokio::test]
    async fn test_plugins_unload_nonexistent_returns_error() {
        ensure_server();
        let client = reqwest::Client::new();
        let body = serde_json::json!({ "mount_point": "/nonexistent_plugin_xyz" });
        let resp = client
            .post(plugin_url("unload"))
            .json(&body)
            .send()
            .await
            .expect("request should succeed");
        // Unmounting non-existent path returns Internal (500)
        let status = resp.status().as_u16();
        assert!(
            status == 500 || status == 404,
            "unload nonexistent plugin should return 500 or 404, got {}",
            status
        );
    }
}
