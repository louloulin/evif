// E2E REST API Tests for EVIF
//
// Tests 30 REST endpoints to validate the EVIF REST API
//
// Requirements:
// - EVIF server running on http://localhost:8081

use reqwest::{Client, StatusCode, Response};
use serde_json::Value;
use std::time::Duration;

const BASE_URL: &str = "http://localhost:8081/api/v1";
const TEST_TIMEOUT: Duration = Duration::from_secs(60);

struct TestContext {
    client: Client,
}

impl TestContext {
    fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(TEST_TIMEOUT)
                .build()
                .expect("Failed to create HTTP client"),
        }
    }
}

/// Helper: Check if server is running
async fn check_server_ready(client: &Client) -> bool {
    client
        .get(&format!("{}/health", BASE_URL.replace("/api/v1", "")))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Helper: Assert response status and return JSON
async fn assert_success(resp: Response) -> Value {
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        panic!(
            "Request failed with status {}: {}",
            status,
            body
        );
    }

    serde_json::from_str(&body).unwrap_or_else(|_| Value::Null)
}

/// Helper: Assert specific status code
async fn assert_status(resp: Response, expected: StatusCode) -> Value {
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if status != expected {
        panic!(
            "Expected status {}, got {}: {}",
            expected, status, body
        );
    }

    if !body.is_empty() {
        serde_json::from_str(&body).unwrap_or(Value::Null)
    } else {
        Value::Null
    }
}

// ============================================================================
// Category 1: Health & Status (2 endpoints)
// ============================================================================

#[tokio::test]
async fn e2e_01_health_root() {
    let ctx = TestContext::new();

    let response = ctx
        .client
        .get(&format!("{}/health", BASE_URL.replace("/api/v1", "")))
        .send()
        .await
        .expect("Health request failed");

    let json = assert_success(response).await;

    assert_eq!(json["status"], "healthy");
    assert!(json.get("timestamp").is_some() || json.get("version").is_some());
}

#[tokio::test]
async fn e2e_02_health_v1() {
    let ctx = TestContext::new();

    let response = ctx
        .client
        .get(format!("{}/health", BASE_URL))
        .send()
        .await
        .expect("Health v1 request failed");

    let json = assert_success(response).await;

    assert_eq!(json["status"], "healthy");
    assert!(json.get("version").is_some());
    assert!(json.get("uptime").is_some());
}

// ============================================================================
// Category 2: Mount Management (3 endpoints)
// ============================================================================

#[tokio::test]
async fn e2e_03_list_mounts() {
    let ctx = TestContext::new();

    let response = ctx
        .client
        .get(format!("{}/mounts", BASE_URL))
        .send()
        .await
        .expect("List mounts request failed");

    let json = assert_success(response).await;

    // Should return an array of mounts
    assert!(json.is_array() || json.get("mounts").is_some());
}

#[tokio::test]
async fn e2e_04_mount_plugin() {
    let ctx = TestContext::new();

    let response = ctx
        .client
        .post(format!("{}/mount", BASE_URL))
        .json(&serde_json::json!({
            "plugin": "localfs",
            "path": "/e2e-test-mount",
            "config": {
                "root": "/tmp/evif-e2e"
            }
        }))
        .send()
        .await
        .expect("Mount request failed");

    // Should return 200 (already exists) or 201 (created)
    assert!(response.status().is_success());

    // Cleanup
    let _ = ctx
        .client
        .post(format!("{}/unmount", BASE_URL))
        .json(&serde_json::json!({
            "path": "/e2e-test-mount"
        }))
        .send()
        .await;
}

#[tokio::test]
async fn e2e_05_unmount_plugin() {
    let ctx = TestContext::new();

    // First mount
    let _ = ctx
        .client
        .post(format!("{}/mount", BASE_URL))
        .json(&serde_json::json!({
            "plugin": "memfs",
            "path": "/e2e-test-unmount"
        }))
        .send()
        .await;

    // Then unmount
    let response = ctx
        .client
        .post(format!("{}/unmount", BASE_URL))
        .json(&serde_json::json!({
            "path": "/e2e-test-unmount"
        }))
        .send()
        .await
        .expect("Unmount request failed");

    assert_success(response).await;
}

// ============================================================================
// Category 3: Plugin Discovery (5 endpoints)
// ============================================================================

#[tokio::test]
async fn e2e_06_list_plugins() {
    let ctx = TestContext::new();

    let response = ctx
        .client
        .get(format!("{}/plugins", BASE_URL))
        .send()
        .await
        .expect("List plugins request failed");

    let json = assert_success(response).await;

    // Should return an array of plugins
    assert!(json.is_array() || json.get("plugins").is_some());
}

#[tokio::test]
async fn e2e_07_get_plugin_config() {
    let ctx = TestContext::new();

    let response = ctx
        .client
        .get(format!("{}/plugins/localfs/config", BASE_URL))
        .send()
        .await
        .expect("Get plugin config request failed");

    let json = assert_success(response).await;

    assert_eq!(json["name"], "localfs");
    assert!(json.get("params").is_some() || json.get("config_params").is_some());
}

#[tokio::test]
async fn e2e_08_get_plugin_readme() {
    let ctx = TestContext::new();

    let response = ctx
        .client
        .get(format!("{}/plugins/localfs/readme", BASE_URL))
        .send()
        .await
        .expect("Get plugin readme request failed");

    let json = assert_success(response).await;

    assert_eq!(json["name"], "localfs");
    assert!(json.get("readme").is_some());
}

#[tokio::test]
async fn e2e_09_plugin_not_found() {
    let ctx = TestContext::new();

    let response = ctx
        .client
        .get(format!("{}/plugins/nonexistent/config", BASE_URL))
        .send()
        .await
        .expect("Plugin not found request failed");

    assert_status(response, StatusCode::NOT_FOUND).await;
}

// ============================================================================
// Category 4: File Operations (8 endpoints)
// ============================================================================

#[tokio::test]
async fn e2e_10_create_file() {
    let ctx = TestContext::new();

    // Use timestamp to ensure unique path
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let response = ctx
        .client
        .post(format!("{}/files", BASE_URL))
        .json(&serde_json::json!({
            "path": format!("/mem/e2e_test_{}.txt", timestamp),
            "content": "Hello E2E"
        }))
        .send()
        .await
        .expect("Create file request failed");

    // Accept 200 OK or 201 Created (both indicate success)
    let status = response.status();
    assert!(status.is_success(), "Create file failed with status {}", status);
}

#[tokio::test]
async fn e2e_11_read_file() {
    let ctx = TestContext::new();

    // Use timestamp for unique path
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let file_path = format!("/mem/e2e_read_test_{}.txt", timestamp);

    // First create the file
    let _ = ctx
        .client
        .post(format!("{}/files", BASE_URL))
        .json(&serde_json::json!({
            "path": file_path
        }))
        .send()
        .await;

    // Then write content
    let _ = ctx
        .client
        .put(format!("{}/files", BASE_URL))
        .query(&[("path", &file_path)])
        .json(&serde_json::json!({
            "data": "Read test content"
        }))
        .send()
        .await;

    // Then read it
    let response = ctx
        .client
        .get(format!("{}/files", BASE_URL))
        .query(&[("path", &file_path)])
        .send()
        .await
        .expect("Read file request failed");

    let json = assert_success(response).await;

    assert_eq!(json["content"], "Read test content");
}

#[tokio::test]
async fn e2e_12_read_file_not_found() {
    let ctx = TestContext::new();

    let response = ctx
        .client
        .get(format!("{}/files", BASE_URL))
        .query(&[("path", "/mem/nonexistent.txt")])
        .send()
        .await
        .expect("Read file not found request failed");

    assert_status(response, StatusCode::NOT_FOUND).await;
}

#[tokio::test]
async fn e2e_13_write_file() {
    let ctx = TestContext::new();

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let file_path = format!("/mem/e2e_write_test_{}.txt", timestamp);

    // First create empty file
    let _ = ctx
        .client
        .post(format!("{}/files", BASE_URL))
        .json(&serde_json::json!({
            "path": file_path
        }))
        .send()
        .await;

    // Write initial content
    let _ = ctx
        .client
        .put(format!("{}/files", BASE_URL))
        .query(&[("path", &file_path)])
        .json(&serde_json::json!({
            "data": "Initial"
        }))
        .send()
        .await;

    // Then write updated content
    let response = ctx
        .client
        .put(format!("{}/files", BASE_URL))
        .query(&[("path", &file_path)])
        .json(&serde_json::json!({
            "data": "Updated content"
        }))
        .send()
        .await
        .expect("Write file request failed");

    assert_success(response).await;
}

#[tokio::test]
async fn e2e_14_delete_file() {
    let ctx = TestContext::new();

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let file_path = format!("/mem/e2e_delete_test_{}.txt", timestamp);

    // First create empty file
    let _ = ctx
        .client
        .post(format!("{}/files", BASE_URL))
        .json(&serde_json::json!({
            "path": file_path
        }))
        .send()
        .await;

    // Then delete
    let response = ctx
        .client
        .delete(format!("{}/files", BASE_URL))
        .query(&[("path", &file_path)])
        .send()
        .await
        .expect("Delete file request failed");

    assert_success(response).await;

    // Verify deleted
    let read_resp = ctx
        .client
        .get(format!("{}/files", BASE_URL))
        .query(&[("path", &file_path)])
        .send()
        .await
        .expect("Verify delete request failed");

    assert_eq!(read_resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Category 5: Directory Operations (3 endpoints)
// ============================================================================

#[tokio::test]
async fn e2e_15_create_directory() {
    let ctx = TestContext::new();

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let response = ctx
        .client
        .post(format!("{}/directories", BASE_URL))
        .json(&serde_json::json!({
            "path": format!("/mem/e2e_test_dir_{}", timestamp)
        }))
        .send()
        .await
        .expect("Create directory request failed");

    let status = response.status();
    assert!(status.is_success(), "Create directory failed with status {}", status);
}

#[tokio::test]
async fn e2e_16_list_directory() {
    let ctx = TestContext::new();

    // Create some files
    let _ = ctx
        .client
        .post(format!("{}/files", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem/file1.txt",
            "content": "test1"
        }))
        .send()
        .await;

    let _ = ctx
        .client
        .post(format!("{}/files", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem/file2.txt",
            "content": "test2"
        }))
        .send()
        .await;

    // List directory
    let response = ctx
        .client
        .get(format!("{}/directories", BASE_URL))
        .query(&[("path", "/mem")])
        .send()
        .await
        .expect("List directory request failed");

    let json = assert_success(response).await;

    // Should contain the files we created
    let entries = if json.is_array() {
        json.as_array().unwrap()
    } else if let Some(arr) = json.get("entries") {
        arr.as_array().unwrap()
    } else if let Some(arr) = json.get("files") {
        arr.as_array().unwrap()
    } else {
        panic!("Unexpected directory listing format");
    };

    assert!(entries.len() >= 2);
}

#[tokio::test]
async fn e2e_17_delete_directory() {
    let ctx = TestContext::new();

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let dir_path = format!("/mem/e2e_dir_to_delete_{}", timestamp);

    // Create directory
    let _ = ctx
        .client
        .post(format!("{}/directories", BASE_URL))
        .json(&serde_json::json!({
            "path": dir_path
        }))
        .send()
        .await;

    // Delete it
    let response = ctx
        .client
        .delete(format!("{}/directories", BASE_URL))
        .query(&[("path", &dir_path)])
        .send()
        .await
        .expect("Delete directory request failed");

    assert_success(response).await;
}

// ============================================================================
// Category 6: Metadata Operations (3 endpoints)
// ============================================================================

#[tokio::test]
async fn e2e_18_stat_file() {
    let ctx = TestContext::new();

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let file_path = format!("/mem/e2e_stat_test_{}.txt", timestamp);

    // Create file
    let _ = ctx
        .client
        .post(format!("{}/files", BASE_URL))
        .json(&serde_json::json!({
            "path": file_path
        }))
        .send()
        .await;

    // Stat it
    let response = ctx
        .client
        .get(format!("{}/stat", BASE_URL))
        .query(&[("path", &file_path)])
        .send()
        .await
        .expect("Stat request failed");

    let json = assert_success(response).await;

    // The FileStat API returns: path, size, is_dir, modified, created
    assert_eq!(json["path"], file_path);
    assert_eq!(json["is_dir"], false);
}

#[tokio::test]
async fn e2e_19_digest_file() {
    let ctx = TestContext::new();

    // Create file
    let _ = ctx
        .client
        .post(format!("{}/files", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem/e2e_digest_test.txt",
            "content": "digest test content"
        }))
        .send()
        .await;

    // Calculate digest
    let response = ctx
        .client
        .post(format!("{}/digest", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem/e2e_digest_test.txt",
            "algorithm": "sha256"
        }))
        .send()
        .await
        .expect("Digest request failed");

    let json = assert_success(response).await;

    assert!(json.get("hash").is_some() || json.get("digest").is_some());
}

#[tokio::test]
async fn e2e_20_touch_file() {
    let ctx = TestContext::new();

    // Create file
    let _ = ctx
        .client
        .post(format!("{}/files", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem/e2e_touch_test.txt",
            "content": "touch test"
        }))
        .send()
        .await;

    // Touch it
    let response = ctx
        .client
        .post(format!("{}/touch", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem/e2e_touch_test.txt"
        }))
        .send()
        .await
        .expect("Touch request failed");

    assert_success(response).await;
}

// ============================================================================
// Category 7: Advanced Operations (2 endpoints)
// ============================================================================

#[tokio::test]
async fn e2e_21_rename_file() {
    let ctx = TestContext::new();

    // Create file
    let _ = ctx
        .client
        .post(format!("{}/files", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem/e2e_rename_old.txt",
            "content": "rename test"
        }))
        .send()
        .await;

    // Rename it
    let response = ctx
        .client
        .post(format!("{}/rename", BASE_URL))
        .json(&serde_json::json!({
            "from": "/mem/e2e_rename_old.txt",
            "to": "/mem/e2e_rename_new.txt"
        }))
        .send()
        .await
        .expect("Rename request failed");

    assert_success(response).await;
}

#[tokio::test]
async fn e2e_22_grep_content() {
    let ctx = TestContext::new();

    // Create files with content
    let _ = ctx
        .client
        .post(format!("{}/files", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem/grep1.txt",
            "content": "hello world\nfoo bar\ntest pattern"
        }))
        .send()
        .await;

    let _ = ctx
        .client
        .post(format!("{}/files", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem/grep2.txt",
            "content": "pattern match\nno match here"
        }))
        .send()
        .await;

    // Search for pattern
    let response = ctx
        .client
        .post(format!("{}/grep", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem",
            "pattern": "pattern"
        }))
        .send()
        .await
        .expect("Grep request failed");

    let json = assert_success(response).await;

    // Should return matches
    assert!(json.get("matches").is_some() || json.get("results").is_some());
}

// ============================================================================
// Category 8: Handle Operations (8 endpoints)
// NOTE: HandleFS is not yet fully implemented, these tests are skipped
// ============================================================================

#[tokio::test]
#[ignore = "HandleFS not yet implemented"]
async fn e2e_23_open_handle() {
    let ctx = TestContext::new();

    // Create file
    let _ = ctx
        .client
        .post(format!("{}/files", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem/e2e_handle_test.txt",
            "content": "handle test content"
        }))
        .send()
        .await;

    // Open handle
    let response = ctx
        .client
        .post(format!("{}/handles/open", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem/e2e_handle_test.txt",
            "flags": "r"
        }))
        .send()
        .await
        .expect("Open handle request failed");

    let json = assert_success(response).await;

    // Should return handle ID
    assert!(json.get("handle_id").is_some() || json.get("id").is_some());
}

#[tokio::test]
#[ignore = "HandleFS not yet implemented"]
async fn e2e_24_get_handle() {
    let ctx = TestContext::new();

    // Create and open file
    let _ = ctx
        .client
        .post(format!("{}/files", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem/e2e_get_handle_test.txt",
            "content": "test"
        }))
        .send()
        .await;

    let open_resp = ctx
        .client
        .post(format!("{}/handles/open", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem/e2e_get_handle_test.txt",
            "flags": "r"
        }))
        .send()
        .await
        .unwrap();

    let open_json: Value = open_resp.json().await.unwrap_or_default();
    let handle_id = open_json["handle_id"].as_str()
        .or_else(|| open_json["id"].as_str())
        .unwrap();

    // Get handle info
    let response = ctx
        .client
        .get(format!("{}/handles/{}", BASE_URL, handle_id))
        .send()
        .await
        .expect("Get handle request failed");

    assert_success(response).await;
}

#[tokio::test]
#[ignore = "HandleFS not yet implemented"]
async fn e2e_25_read_handle() {
    let ctx = TestContext::new();

    // Create and open file
    let _ = ctx
        .client
        .post(format!("{}/files", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem/e2e_read_handle_test.txt",
            "content": "handle read test"
        }))
        .send()
        .await;

    let open_resp = ctx
        .client
        .post(format!("{}/handles/open", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem/e2e_read_handle_test.txt",
            "flags": "r"
        }))
        .send()
        .await
        .unwrap();

    let open_json: Value = open_resp.json().await.unwrap_or_default();
    let handle_id = open_json["handle_id"].as_str()
        .or_else(|| open_json["id"].as_str())
        .unwrap();

    // Read from handle
    let response = ctx
        .client
        .post(format!("{}/handles/{}/read", BASE_URL, handle_id))
        .json(&serde_json::json!({
            "size": 1024
        }))
        .send()
        .await
        .expect("Read handle request failed");

    let json = assert_success(response).await;

    assert!(json.get("data").is_some() || json.get("content").is_some());
}

#[tokio::test]
#[ignore = "HandleFS not yet implemented"]
async fn e2e_26_close_handle() {
    let ctx = TestContext::new();

    // Create and open file
    let _ = ctx
        .client
        .post(format!("{}/files", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem/e2e_close_handle_test.txt",
            "content": "close test"
        }))
        .send()
        .await;

    let open_resp = ctx
        .client
        .post(format!("{}/handles/open", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem/e2e_close_handle_test.txt",
            "flags": "r"
        }))
        .send()
        .await
        .unwrap();

    let open_json: Value = open_resp.json().await.unwrap_or_default();
    let handle_id = open_json["handle_id"].as_str()
        .or_else(|| open_json["id"].as_str())
        .unwrap();

    // Close handle
    let response = ctx
        .client
        .post(format!("{}/handles/{}/close", BASE_URL, handle_id))
        .send()
        .await
        .expect("Close handle request failed");

    assert_success(response).await;
}

// ============================================================================
// Category 9: Batch Operations (2 endpoints)
// ============================================================================

#[tokio::test]
async fn e2e_27_batch_copy() {
    let ctx = TestContext::new();

    // Create source files
    let _ = ctx
        .client
        .post(format!("{}/files", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem/batch_src1.txt",
            "content": "source 1"
        }))
        .send()
        .await;

    let _ = ctx
        .client
        .post(format!("{}/files", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem/batch_src2.txt",
            "content": "source 2"
        }))
        .send()
        .await;

    // Batch copy
    let response = ctx
        .client
        .post(format!("{}/batch/copy", BASE_URL))
        .json(&serde_json::json!({
            "sources": ["/mem/batch_src1.txt", "/mem/batch_src2.txt"],
            "destination": "/mem"
        }))
        .send()
        .await
        .expect("Batch copy request failed");

    assert_success(response).await;
}

#[tokio::test]
async fn e2e_28_batch_delete() {
    let ctx = TestContext::new();

    // Create files
    let _ = ctx
        .client
        .post(format!("{}/files", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem/batch_del1.txt",
            "content": "delete 1"
        }))
        .send()
        .await;

    let _ = ctx
        .client
        .post(format!("{}/files", BASE_URL))
        .json(&serde_json::json!({
            "path": "/mem/batch_del2.txt",
            "content": "delete 2"
        }))
        .send()
        .await;

    // Batch delete
    let response = ctx
        .client
        .post(format!("{}/batch/delete", BASE_URL))
        .json(&serde_json::json!({
            "paths": [
                "/mem/batch_del1.txt",
                "/mem/batch_del2.txt"
            ]
        }))
        .send()
        .await
        .expect("Batch delete request failed");

    assert_success(response).await;
}

// ============================================================================
// Category 10: Metrics (4 endpoints)
// ============================================================================

#[tokio::test]
async fn e2e_29_metrics_traffic() {
    let ctx = TestContext::new();

    let response = ctx
        .client
        .get(format!("{}/metrics/traffic", BASE_URL))
        .send()
        .await
        .expect("Traffic metrics request failed");

    let json = assert_success(response).await;

    // Should return traffic stats
    assert!(json.get("total_requests").is_some() || json.get("requests").is_some());
}

#[tokio::test]
async fn e2e_30_metrics_operations() {
    let ctx = TestContext::new();

    let response = ctx
        .client
        .get(format!("{}/metrics/operations", BASE_URL))
        .send()
        .await
        .expect("Operations metrics request failed");

    let json = assert_success(response).await;

    // Should return operation counts
    assert!(json.is_array() || json.get("operations").is_some());
}

// ============================================================================
// Test Runner: Server Health Check
// ============================================================================

#[tokio::test]
async fn e2e_server_ready() {
    let ctx = TestContext::new();

    // This test checks if the server is running
    // Run this first: cargo test e2e_server_ready

    assert!(
        check_server_ready(&ctx.client).await,
        "EVIF server is not running on {}. Start it with: cargo run --bin evif-rest",
        BASE_URL
    );
}
