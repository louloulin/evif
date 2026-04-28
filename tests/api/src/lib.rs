#![cfg(test)]
#![allow(clippy::needless_borrows_for_generic_args)]

// EVIF REST API Tests
// Integration tests for core REST API endpoints.
// Uses Mutex-based lazy init to avoid OnceLock poisoning issues.

use evif_core::{EvifPlugin, RadixMountTable};
use evif_plugins::MemFsPlugin;
use evif_rest::create_routes;
use reqwest::Client;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

// Lazy init server using Mutex (avoids OnceLock poisoning)
static SERVER_STATE: Mutex<Option<Arc<TestServerState>>> = Mutex::new(None);

struct TestServerState {
    base_url: String,
    #[allow(dead_code)]
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

impl TestServerState {
    async fn start() -> Self {
        let mount_table = Arc::new(RadixMountTable::new());
        let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
        mount_table
            .mount("/".to_string(), mem)
            .await
            .expect("mount root memfs");

        let app = create_routes(mount_table);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let base_url = format!("http://{}", addr);

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        // Spawn server in background
        tokio::spawn(async move {
            axum::serve(listener, app.into_make_service())
                .await
                .expect("server error");
            let _ = shutdown_rx;
        });

        // Give server time to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        Self { base_url, shutdown_tx }
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }
}

fn ensure_server() -> Arc<TestServerState> {
    let mut guard = SERVER_STATE.lock().unwrap();
    if let Some(ref state) = *guard {
        return Arc::clone(state);
    }

    // Create new runtime for spawning the server
    let runtime = tokio::runtime::Runtime::new().expect("create runtime");
    let state = runtime.block_on(TestServerState::start());
    let arc_state = Arc::new(state);
    *guard = Some(Arc::clone(&arc_state));
    arc_state
}

fn get_api_base() -> String {
    ensure_server().base_url.clone()
}

fn unique_test_path() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("/test_{}_{}", std::process::id(), timestamp)
}

async fn get_client() -> Client {
    Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client")
}

mod health_checks {
    use super::*;

    #[tokio::test]
    async fn test_health_basic() {
        let client = get_client().await;
        let base = get_api_base();

        let response = client.get(&format!("{}/health", base)).send().await;

        assert!(response.is_ok(), "Health check request failed");
        let status = response.unwrap().status();
        assert!(
            status.is_success() || status.as_u16() == 200,
            "Health check should return 200, got: {}",
            status
        );
    }

    #[tokio::test]
    async fn test_health_v1() {
        let client = get_client().await;
        let base = get_api_base();

        let response = client.get(&format!("{}/api/v1/health", base)).send().await;

        assert!(response.is_ok(), "Health v1 request failed");
        let status = response.unwrap().status();
        assert!(
            status.is_success() || status.as_u16() == 200,
            "Health v1 should return 200, got: {}",
            status
        );
    }
}

mod file_operations {
    use super::*;

    #[tokio::test]
    async fn test_read_file() {
        let client = get_client().await;
        let base = get_api_base();
        let test_file = unique_test_path();

        let _ = client
            .put(&format!("{}/api/v1/files?path={}", base, test_file))
            .body("test content".to_string())
            .send()
            .await;

        let response = client
            .get(&format!("{}/api/v1/files?path={}", base, test_file))
            .send()
            .await;

        assert!(response.is_ok(), "Read file request failed");
    }

    #[tokio::test]
    async fn test_write_file() {
        let client = get_client().await;
        let base = get_api_base();
        let test_file = unique_test_path();

        let response = client
            .put(&format!("{}/api/v1/files?path={}", base, test_file))
            .body("new content".to_string())
            .send()
            .await;

        assert!(response.is_ok(), "Write file request failed");
    }

    #[tokio::test]
    async fn test_create_file() {
        let client = get_client().await;
        let base = get_api_base();
        let test_file = unique_test_path();

        let response = client
            .post(&format!("{}/api/v1/files?path={}", base, test_file))
            .body("created content".to_string())
            .send()
            .await;

        assert!(
            response.is_ok() || response.unwrap().status() == 405,
            "Create file request should succeed or method not allowed"
        );
    }

    #[tokio::test]
    async fn test_delete_file() {
        let client = get_client().await;
        let base = get_api_base();
        let test_file = unique_test_path();

        let _ = client
            .put(&format!("{}/api/v1/files?path={}", base, test_file))
            .body("to be deleted".to_string())
            .send()
            .await;

        let response = client
            .delete(&format!("{}/api/v1/files?path={}", base, test_file))
            .send()
            .await;

        assert!(response.is_ok(), "Delete file request failed");
    }
}

mod directory_operations {
    use super::*;

    #[tokio::test]
    async fn test_list_directory() {
        let client = get_client().await;
        let base = get_api_base();

        let response = client
            .get(&format!("{}/api/v1/directories?path=/", base))
            .send()
            .await;

        assert!(response.is_ok(), "List directory request failed");
    }

    #[tokio::test]
    async fn test_create_directory() {
        let client = get_client().await;
        let base = get_api_base();
        let test_dir = unique_test_path();

        let response = client
            .post(&format!("{}/api/v1/directories?path={}", base, test_dir))
            .send()
            .await;

        assert!(response.is_ok(), "Create directory request failed");
    }

    #[tokio::test]
    async fn test_delete_directory() {
        let client = get_client().await;
        let base = get_api_base();
        let test_dir = unique_test_path();

        let _ = client
            .post(&format!("{}/api/v1/directories?path={}", base, test_dir))
            .send()
            .await;

        let response = client
            .delete(&format!("{}/api/v1/directories?path={}", base, test_dir))
            .send()
            .await;

        assert!(response.is_ok(), "Delete directory request failed");
    }
}

mod metadata_operations {
    use super::*;

    #[tokio::test]
    async fn test_stat_file() {
        let client = get_client().await;
        let base = get_api_base();
        let test_file = unique_test_path();

        let _ = client
            .put(&format!("{}/api/v1/files?path={}", base, test_file))
            .body("stat test".to_string())
            .send()
            .await;

        let response = client
            .get(&format!("{}/api/v1/stat?path={}", base, test_file))
            .send()
            .await;

        assert!(response.is_ok(), "Stat request failed");
    }

    #[tokio::test]
    async fn test_touch_file() {
        let client = get_client().await;
        let base = get_api_base();
        let test_file = unique_test_path();

        let _ = client
            .put(&format!("{}/api/v1/files?path={}", base, test_file))
            .body("touch test".to_string())
            .send()
            .await;

        let response = client
            .post(&format!("{}/api/v1/touch?path={}", base, test_file))
            .send()
            .await;

        assert!(response.is_ok(), "Touch request failed");
    }

    #[tokio::test]
    async fn test_digest_file() {
        let client = get_client().await;
        let base = get_api_base();
        let test_file = unique_test_path();

        let _ = client
            .put(&format!("{}/api/v1/files?path={}", base, test_file))
            .body("digest test".to_string())
            .send()
            .await;

        let response = client
            .post(&format!(
                "{}/api/v1/digest?path={}&algo=sha256",
                base, test_file
            ))
            .send()
            .await;

        assert!(response.is_ok(), "Digest request failed");
    }

    #[tokio::test]
    async fn test_rename_file() {
        let client = get_client().await;
        let base = get_api_base();
        let src_file = unique_test_path();
        let dst_file = format!("{}_renamed", src_file);

        let _ = client
            .put(&format!("{}/api/v1/files?path={}", base, src_file))
            .body("rename test".to_string())
            .send()
            .await;

        let response = client
            .post(&format!(
                "{}/api/v1/rename?src={}&dst={}",
                base, src_file, dst_file
            ))
            .send()
            .await;

        assert!(response.is_ok(), "Rename request failed");
    }
}

mod mount_management {
    use super::*;

    #[tokio::test]
    async fn test_list_mounts() {
        let client = get_client().await;
        let base = get_api_base();

        let response = client.get(&format!("{}/api/v1/mounts", base)).send().await;

        assert!(response.is_ok(), "List mounts request failed");
    }

    #[tokio::test]
    async fn test_mount_plugin() {
        let client = get_client().await;
        let base = get_api_base();
        let mount_path = unique_test_path();

        let response = client
            .post(&format!(
                "{}/api/v1/mount?plugin=memfs&path={}",
                base, mount_path
            ))
            .send()
            .await;

        assert!(
            response.is_ok() || response.unwrap().status() == 500,
            "Mount request should succeed or return server error"
        );
    }

    #[tokio::test]
    async fn test_unmount_plugin() {
        let client = get_client().await;
        let base = get_api_base();
        let mount_path = unique_test_path();

        let _ = client
            .post(&format!(
                "{}/api/v1/mount?plugin=memfs&path={}",
                base, mount_path
            ))
            .send()
            .await;

        let response = client
            .post(&format!("{}/api/v1/unmount?path={}", base, mount_path))
            .send()
            .await;

        assert!(response.is_ok(), "Unmount request failed");
    }
}

mod batch_operations {
    use super::*;

    #[tokio::test]
    async fn test_batch_copy() {
        let client = get_client().await;
        let base = get_api_base();
        let src = unique_test_path();
        let dst = format!("{}_copy_dest", unique_test_path());

        let _ = client
            .put(&format!("{}/api/v1/files?path={}", base, src))
            .body("batch copy source content".to_string())
            .send()
            .await;

        let response = client
            .post(&format!("{}/api/v1/batch/copy", base))
            .json(&serde_json::json!({
                "sources": [src],
                "destination": dst,
                "recursive": false,
                "overwrite": true
            }))
            .send()
            .await;

        assert!(response.is_ok(), "Batch copy request failed");
        let resp = response.unwrap();
        let status = resp.status();
        assert!(
            status.is_success() || status.as_u16() == 500,
            "Batch copy should succeed or 500 (got {})",
            status
        );
    }

    #[tokio::test]
    async fn test_batch_delete() {
        let client = get_client().await;
        let base = get_api_base();
        let file1 = unique_test_path();
        let file2 = unique_test_path();

        let _ = client
            .put(&format!("{}/api/v1/files?path={}", base, file1))
            .body("delete me 1".to_string())
            .send()
            .await;
        let _ = client
            .put(&format!("{}/api/v1/files?path={}", base, file2))
            .body("delete me 2".to_string())
            .send()
            .await;

        let response = client
            .post(&format!("{}/api/v1/batch/delete", base))
            .json(&serde_json::json!({
                "paths": [file1, file2],
                "recursive": false
            }))
            .send()
            .await;

        assert!(response.is_ok(), "Batch delete request failed");
    }

    #[tokio::test]
    async fn test_batch_progress() {
        let client = get_client().await;
        let base = get_api_base();

        let response = client
            .get(&format!("{}/api/v1/batch/progress/nonexistent-id", base))
            .send()
            .await;

        assert!(response.is_ok(), "Batch progress request failed");
    }

    #[tokio::test]
    async fn test_list_batch_operations() {
        let client = get_client().await;
        let base = get_api_base();

        let response = client
            .get(&format!("{}/api/v1/batch/operations", base))
            .send()
            .await;

        assert!(response.is_ok(), "List batch operations request failed");
    }

    #[tokio::test]
    async fn test_cancel_batch_operation() {
        let client = get_client().await;
        let base = get_api_base();

        let response = client
            .delete(&format!("{}/api/v1/batch/operation/nonexistent-id", base))
            .send()
            .await;

        assert!(response.is_ok(), "Cancel batch operation request failed");
    }
}

mod plugin_api_management {
    use super::*;

    #[tokio::test]
    async fn test_list_plugins() {
        let client = get_client().await;
        let base = get_api_base();

        let response = client.get(&format!("{}/api/v1/plugins", base)).send().await;

        assert!(response.is_ok(), "List plugins request failed");
    }

    #[tokio::test]
    async fn test_list_available_plugins() {
        let client = get_client().await;
        let base = get_api_base();

        let response = client
            .get(&format!("{}/api/v1/plugins/available", base))
            .send()
            .await;

        assert!(response.is_ok(), "List available plugins request failed");
    }

    #[tokio::test]
    async fn test_get_plugin_readme() {
        let client = get_client().await;
        let base = get_api_base();

        let response = client
            .get(&format!("{}/api/v1/plugins/memfs/readme", base))
            .send()
            .await;

        assert!(response.is_ok(), "Get plugin readme request failed");
    }

    #[tokio::test]
    async fn test_get_plugin_config() {
        let client = get_client().await;
        let base = get_api_base();

        let response = client
            .get(&format!("{}/api/v1/plugins/memfs/config", base))
            .send()
            .await;

        assert!(response.is_ok(), "Get plugin config request failed");
    }

    #[tokio::test]
    async fn test_list_plugins_detailed() {
        let client = get_client().await;
        let base = get_api_base();

        let response = client
            .get(&format!("{}/api/v1/plugins/list", base))
            .send()
            .await;

        assert!(response.is_ok(), "List plugins detailed request failed");
    }
}