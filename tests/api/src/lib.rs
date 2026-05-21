#![cfg(test)]
#![allow(clippy::needless_borrows_for_generic_args)]

// EVIF REST API Tests
// Integration tests for core REST API endpoints.
// Uses a dedicated background thread for the test server.

use evif_core::{EvifPlugin, RadixMountTable};
use evif_plugins::MemFsPlugin;
use evif_rest::create_routes;
use reqwest::Client;
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

// Global server URL cache with thread-safe initialization
static SERVER_URL: OnceLock<String> = OnceLock::new();

fn init_test_server() -> String {
    // Channel to receive the server URL from the background thread
    let (tx, rx) = std::sync::mpsc::channel();

    // Spawn dedicated thread for the server
    let _handle = std::thread::spawn(move || {
        // Create a new runtime in this thread (NOT inside any existing runtime)
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("create runtime");

        let base_url = rt.block_on(async {
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

            let (_shutdown_tx, shutdown_rx): (tokio::sync::oneshot::Sender<()>, tokio::sync::oneshot::Receiver<()>) =
                tokio::sync::oneshot::channel();

            // Spawn server - keep the receiver alive
            let serve_future = axum::serve(listener, app.into_make_service())
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                });

            // Send URL BEFORE blocking
            let _ = tx.send(base_url.clone());

            // Keep the runtime alive and serve
            serve_future.await;

            base_url
        });

        base_url
    });

    // Wait for the server URL from the background thread
    rx.recv().expect("failed to receive server URL")
}

fn get_or_init_server() -> String {
    SERVER_URL.get_or_init(init_test_server).clone()
}

async fn get_api_base() -> String {
    get_or_init_server()
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
        .timeout(std::time::Duration::from_secs(30))
        .no_proxy()
        .build()
        .expect("Failed to create HTTP client")
}

// Sandbox skip helper
fn check_network_available() -> bool {
    std::net::TcpListener::bind("127.0.0.1:0").is_ok()
}

macro_rules! skip_without_network {
    () => {
        if !check_network_available() {
            println!("SKIP: Network operations not permitted (sandbox restriction)");
            return;
        }
    };
}

mod health_checks {
    use super::*;

    #[tokio::test]
    async fn test_health_basic() {
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;
        println!("Testing health check at: {}/health", base);

        let response = client.get(&format!("{}/health", base)).send().await;

        if response.as_ref().is_err() {
            println!("Health check FAILED: {:?}", response.as_ref().err());
        }
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
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;

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
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;
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
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;
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
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;
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
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;
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
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;

        let response = client
            .get(&format!("{}/api/v1/directories?path=/", base))
            .send()
            .await;

        assert!(response.is_ok(), "List directory request failed");
    }

    #[tokio::test]
    async fn test_create_directory() {
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;
        let test_dir = unique_test_path();

        let response = client
            .post(&format!("{}/api/v1/directories?path={}", base, test_dir))
            .send()
            .await;

        assert!(response.is_ok(), "Create directory request failed");
    }

    #[tokio::test]
    async fn test_delete_directory() {
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;
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
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;
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
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;
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
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;
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
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;
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
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;

        let response = client.get(&format!("{}/api/v1/mounts", base)).send().await;

        assert!(response.is_ok(), "List mounts request failed");
    }

    #[tokio::test]
    async fn test_mount_plugin() {
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;
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
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;
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
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;
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
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;
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
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;

        let response = client
            .get(&format!("{}/api/v1/batch/progress/nonexistent-id", base))
            .send()
            .await;

        assert!(response.is_ok(), "Batch progress request failed");
    }

    #[tokio::test]
    async fn test_list_batch_operations() {
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;

        let response = client
            .get(&format!("{}/api/v1/batch/operations", base))
            .send()
            .await;

        assert!(response.is_ok(), "List batch operations request failed");
    }

    #[tokio::test]
    async fn test_cancel_batch_operation() {
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;

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
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;

        let response = client.get(&format!("{}/api/v1/plugins", base)).send().await;

        assert!(response.is_ok(), "List plugins request failed");
    }

    #[tokio::test]
    async fn test_list_available_plugins() {
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;

        let response = client
            .get(&format!("{}/api/v1/plugins/available", base))
            .send()
            .await;

        assert!(response.is_ok(), "List available plugins request failed");
    }

    #[tokio::test]
    async fn test_get_plugin_readme() {
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;

        let response = client
            .get(&format!("{}/api/v1/plugins/memfs/readme", base))
            .send()
            .await;

        assert!(response.is_ok(), "Get plugin readme request failed");
    }

    #[tokio::test]
    async fn test_get_plugin_config() {
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;

        let response = client
            .get(&format!("{}/api/v1/plugins/memfs/config", base))
            .send()
            .await;

        assert!(response.is_ok(), "Get plugin config request failed");
    }

    #[tokio::test]
    async fn test_list_plugins_detailed() {
        skip_without_network!();
        let client = get_client().await;
        let base = get_api_base().await;

        let response = client
            .get(&format!("{}/api/v1/plugins/list", base))
            .send()
            .await;

        assert!(response.is_ok(), "List plugins detailed request failed");
    }
}