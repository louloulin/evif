// Phase 7.4: API 契约与 evif-client 一致性集成测试
//
// 验证：list_mounts 返回 { "mounts": [...] }；read 返回 { "content", "data", "size" }；
// write 接受 { "data", "encoding": "base64" }。

use base64::Engine;
use evif_rest::create_routes;
use evif_core::{RadixMountTable, EvifPlugin};
use evif_plugins::MemFsPlugin;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::test]
async fn test_list_mounts_returns_mounts_key() {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    mount_table.mount("/mem".to_string(), mem).await.unwrap();

    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    for _ in 0..50 {
        let url = format!("http://127.0.0.1:{}/api/v1/mounts", port);
        if let Ok(res) = reqwest::get(&url).await {
            if res.status().is_success() {
                let json: serde_json::Value = res.json().await.unwrap();
                assert!(json.get("mounts").is_some(), "response must have 'mounts' key: {:?}", json);
                assert!(json["mounts"].is_array(), "'mounts' must be array");
                return;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    panic!("server did not become ready in time");
}

#[tokio::test]
async fn test_read_file_returns_data_and_content() {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    mount_table.mount("/mem".to_string(), mem).await.unwrap();

    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    for i in 0..80 {
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        let mkdir_url = format!("{}/api/v1/directories", base);
        let _ = client.post(&mkdir_url).json(&serde_json::json!({ "path": "/mem" })).send().await;
        let create_url = format!("{}/api/v1/files", base);
        let _ = client.post(&create_url).json(&serde_json::json!({ "path": "/mem/read_contract_test.txt" })).send().await;
        let write_url = format!("{}/api/v1/files?path=/mem/read_contract_test.txt", base);
        let data_b64 = base64::engine::general_purpose::STANDARD.encode(b"hello contract");
        let _ = client.put(&write_url).json(&serde_json::json!({ "data": data_b64, "encoding": "base64" })).send().await;
        let read_url = format!("{}/api/v1/files?path=/mem/read_contract_test.txt", base);
        if let Ok(res) = client.get(&read_url).send().await {
            if res.status().is_success() {
                let json: serde_json::Value = res.json().await.unwrap();
                assert!(json.get("data").is_some(), "read response must have 'data' (base64): {:?}", json);
                assert!(json.get("content").is_some(), "read response must have 'content': {:?}", json);
                assert!(json.get("size").is_some(), "read response must have 'size': {:?}", json);
                return;
            }
        }
        if i == 79 {
            panic!("read contract test did not succeed in time");
        }
    }
}

#[tokio::test]
async fn test_write_file_accepts_base64_encoding() {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    mount_table.mount("/mem".to_string(), mem).await.unwrap();

    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();
    let path = "/mem/write_contract_test.txt";
    let data_b64 = base64::engine::general_purpose::STANDARD.encode(b"written via base64");

    for i in 0..80 {
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        let _ = client.post(&format!("{}/api/v1/directories", base)).json(&serde_json::json!({ "path": "/mem" })).send().await;
        let _ = client.post(&format!("{}/api/v1/files", base)).json(&serde_json::json!({ "path": path })).send().await;
        let write_url = format!("{}/api/v1/files?path={}", base, path);
        let body = serde_json::json!({ "data": data_b64, "encoding": "base64" });
        if let Ok(res) = client.put(&write_url).json(&body).send().await {
            if res.status().is_success() {
                let read_url = format!("{}/api/v1/files?path={}", base, path);
                let read_res = client.get(&read_url).send().await.unwrap();
                let json: serde_json::Value = read_res.json().await.unwrap();
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(json["data"].as_str().unwrap())
                    .unwrap();
                assert_eq!(decoded, b"written via base64");
                return;
            }
        }
        if i == 79 {
            panic!("write base64 contract test did not succeed in time");
        }
    }
}

// ---------- Phase 8: 插件 readme/config 与 mount 校验 ----------

#[tokio::test]
async fn test_get_plugin_readme_returns_content() {
    let mount_table = Arc::new(RadixMountTable::new());
    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    for _ in 0..50 {
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        let url = format!("{}/api/v1/plugins/mem/readme", base);
        if let Ok(res) = client.get(&url).send().await {
            if res.status().is_success() {
                let json: serde_json::Value = res.json().await.unwrap();
                assert!(json.get("name").is_some());
                assert_eq!(json["name"], "memfs");
                assert!(json.get("readme").is_some());
                let readme = json["readme"].as_str().unwrap();
                assert!(readme.contains("MemFS"), "readme should describe MemFS: {}", readme);
                return;
            }
        }
    }
    panic!("get plugin readme did not succeed in time");
}

#[tokio::test]
async fn test_get_plugin_config_returns_params() {
    let mount_table = Arc::new(RadixMountTable::new());
    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    for _ in 0..50 {
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        let url = format!("{}/api/v1/plugins/local/config", base);
        if let Ok(res) = client.get(&url).send().await {
            if res.status().is_success() {
                let json: serde_json::Value = res.json().await.unwrap();
                assert!(json.get("name").is_some());
                assert_eq!(json["name"], "localfs");
                assert!(json.get("params").is_some());
                let params = json["params"].as_array().unwrap();
                assert!(!params.is_empty(), "localfs should have root param");
                let has_root = params.iter().any(|p| p.get("name").and_then(|n| n.as_str()) == Some("root"));
                assert!(has_root, "params should include root: {:?}", params);
                return;
            }
        }
    }
    panic!("get plugin config did not succeed in time");
}

#[tokio::test]
async fn test_mount_local_with_invalid_config_fails() {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    mount_table.mount("/mem".to_string(), mem).await.unwrap();

    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    for _ in 0..50 {
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        let url = format!("{}/api/v1/mount", base);
        let body = serde_json::json!({
            "path": "/local",
            "plugin": "local",
            "config": { "root": "" }
        });
        if let Ok(res) = client.post(&url).json(&body).send().await {
            if res.status().is_client_error() {
                let status = res.status();
                let text = res.text().await.unwrap();
                assert!(text.contains("validation") || text.contains("root") || status.as_u16() == 400,
                    "expect 400 or validation/root message: {} {}", status, text);
                return;
            }
        }
    }
    panic!("mount with invalid config should have failed with 4xx");
}

// ---------- GET /api/v1/health 契约（与 evif-client/CLI 一致）----------

#[tokio::test]
async fn test_api_v1_health_returns_status_version_uptime() {
    let mount_table = Arc::new(RadixMountTable::new());
    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    for _ in 0..50 {
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        let url = format!("{}/api/v1/health", base);
        if let Ok(res) = client.get(&url).send().await {
            if res.status().is_success() {
                let json: serde_json::Value = res.json().await.unwrap();
                assert!(json.get("status").is_some(), "response must have 'status': {:?}", json);
                assert_eq!(json["status"], "healthy");
                assert!(json.get("version").is_some(), "response must have 'version': {:?}", json);
                assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
                assert!(json.get("uptime").is_some(), "response must have 'uptime' (seconds): {:?}", json);
                return;
            }
        }
    }
    panic!("GET /api/v1/health did not succeed in time");
}

#[tokio::test]
async fn test_root_health_matches_canonical_version_and_reports_timestamp() {
    let mount_table = Arc::new(RadixMountTable::new());
    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    for _ in 0..50 {
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;

        let root_url = format!("{}/health", base);
        let api_url = format!("{}/api/v1/health", base);

        if let (Ok(root_res), Ok(api_res)) = (
            client.get(&root_url).send().await,
            client.get(&api_url).send().await,
        ) {
            if root_res.status().is_success() && api_res.status().is_success() {
                let root_json: serde_json::Value = root_res.json().await.unwrap();
                let api_json: serde_json::Value = api_res.json().await.unwrap();

                assert_eq!(root_json["status"], "healthy");
                assert_eq!(api_json["status"], "healthy");
                assert_eq!(root_json["version"], env!("CARGO_PKG_VERSION"));
                assert_eq!(root_json["version"], api_json["version"]);
                assert!(root_json.get("uptime").is_some(), "root health must include uptime: {:?}", root_json);
                assert!(root_json.get("timestamp").is_some(), "root health must include timestamp: {:?}", root_json);
                return;
            }
        }
    }

    panic!("GET /health and /api/v1/health did not succeed in time");
}

// ---------- Phase 12.3: 关键路径集成测试（mount → list → create → write → read → unmount）----------

#[tokio::test]
async fn test_key_path_mount_list_write_read_unmount() {
    let mount_table = Arc::new(RadixMountTable::new());
    let app = create_routes(mount_table.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    for _ in 0..50 {
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        // 1. 动态挂载 mem 到 /mkey
        let mount_url = format!("{}/api/v1/mount", base);
        let mount_body = serde_json::json!({ "path": "/mkey", "plugin": "mem", "config": {} });
        if client.post(&mount_url).json(&mount_body).send().await.ok().map(|r| r.status().is_success()) != Some(true) {
            continue;
        }
        // 2. 列出挂载点，确认 /mkey 存在
        let mounts_url = format!("{}/api/v1/mounts", base);
        let mounts_res = match client.get(&mounts_url).send().await {
            Ok(r) if r.status().is_success() => r.json::<serde_json::Value>().await.ok(),
            _ => continue,
        };
        let mounts = match mounts_res.and_then(|j| j.get("mounts").and_then(|a| a.as_array()).cloned()) {
            Some(m) => m,
            _ => continue,
        };
        if !mounts.iter().any(|m| m.get("path").and_then(|p| p.as_str()) == Some("/mkey")) {
            continue;
        }
        // 3. 创建目录占位、创建文件、写入、读取
        let _ = client.post(&format!("{}/api/v1/directories", base)).json(&serde_json::json!({ "path": "/mkey" })).send().await;
        let _ = client.post(&format!("{}/api/v1/files", base)).json(&serde_json::json!({ "path": "/mkey/keypath.txt" })).send().await;
        let write_url = format!("{}/api/v1/files?path=/mkey/keypath.txt", base);
        let payload = serde_json::json!({ "data": "keypath-content", "encoding": null });
        if !client.put(&write_url).json(&payload).send().await.ok().map(|r| r.status().is_success()).unwrap_or(false) {
            continue;
        }
        let read_url = format!("{}/api/v1/files?path=/mkey/keypath.txt", base);
        let read_res = match client.get(&read_url).send().await {
            Ok(r) if r.status().is_success() => r.json::<serde_json::Value>().await.ok(),
            _ => continue,
        };
        let content = read_res.and_then(|j| j.get("content").and_then(|c| c.as_str()).map(String::from));
        if content.as_deref() != Some("keypath-content") {
            continue;
        }
        // 4. 卸载
        let unmount_url = format!("{}/api/v1/unmount", base);
        let unmount_body = serde_json::json!({ "path": "/mkey" });
        let unmount_ok = client.post(&unmount_url).json(&unmount_body).send().await.ok().map(|r| r.status().is_success()).unwrap_or(false);
        if unmount_ok {
            let after_res = client.get(&mounts_url).send().await.ok();
            let after = match after_res {
                Some(r) if r.status().is_success() => r.json::<serde_json::Value>().await.ok(),
                _ => None,
            };
            let after_mounts = after.and_then(|j| j.get("mounts").and_then(|a| a.as_array()).cloned()).unwrap_or_default();
            assert!(!after_mounts.iter().any(|m| m.get("path").and_then(|p| p.as_str()) == Some("/mkey")), "unmount should remove /mkey");
            return;
        }
    }
    panic!("key path mount/list/write/read/unmount did not complete in time");
}

// Task 05: Test root path listing returns mount points
#[tokio::test]
async fn test_list_root_directory() {

    let mount_table = Arc::new(RadixMountTable::new());
    let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    mount_table.mount("/mem".to_string(), mem).await.unwrap();

    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    // Wait for server to be ready and test root listing
    for _ in 0..50 {
        let url = format!("http://127.0.0.1:{}/api/v1/fs/list?path=/", port);
        if let Ok(res) = reqwest::get(&url).await {
            if res.status().is_success() {
                let json: serde_json::Value = res.json().await.unwrap();
                // Verify response structure (FsListResponse uses "nodes" not "files")
                assert!(json.get("nodes").is_some(), "response must have 'nodes' key");

                // Verify mount points are returned
                let nodes = json["nodes"].as_array().expect("nodes must be array");
                assert_eq!(nodes.len(), 1, "should have 1 mount point");
                assert_eq!(nodes[0]["name"], "mem", "mount point name should be 'mem'");
                assert_eq!(nodes[0]["is_dir"], true, "mount point should be directory");
                assert_eq!(nodes[0]["path"], "/mem", "mount point path should be '/mem'");
                return;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    panic!("server did not return root listing in time");
}

// Task 06: Test file read operations with nested paths
#[tokio::test]
async fn test_read_file_in_nested_path() {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    mount_table.mount("/mem".to_string(), mem.clone()).await.unwrap();

    // Create directory and test file in nested path, then write content
    mem.mkdir("/nested", 0o755).await.unwrap();
    mem.create("/nested/test.txt", 0o644).await.unwrap();
    mem.write("/nested/test.txt", b"Hello, World!".to_vec(), 0, evif_core::WriteFlags::NONE).await.unwrap();

    // Verify mount exists
    let mounts = mount_table.list_mounts().await;
    assert_eq!(mounts.len(), 1, "should have 1 mount");
    assert_eq!(mounts[0], "/mem", "mount path should be /mem");

    // Test lookup_with_path directly
    let (plugin_opt, relative_path) = mount_table.lookup_with_path("/mem/nested/test.txt").await;
    assert!(plugin_opt.is_some(), "plugin should be found for /mem/nested/test.txt");
    assert_eq!(relative_path, "/nested/test.txt", "relative path should be /nested/test.txt");

    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    // Wait for server and test file read
    for _ in 0..50 {
        // First check if server is ready
        let health_url = format!("http://127.0.0.1:{}/api/v1/health", port);
        if let Ok(res) = reqwest::get(&health_url).await {
            if !res.status().is_success() {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                continue;
            }
        }

        let url = format!("http://127.0.0.1:{}/api/v1/fs/read?path=/mem/nested/test.txt", port);
        if let Ok(res) = reqwest::get(&url).await {
            if res.status().is_success() {
                let json: serde_json::Value = res.json().await.unwrap();
                assert!(json.get("content").is_some(), "response must have 'content' key");
                assert_eq!(json["content"], "Hello, World!", "file content should match");
                return;
            } else {
                // Print error for debugging
                let status = res.status();
                let text = res.text().await.unwrap_or_else(|_| "unable to read response".to_string());
                eprintln!("Request failed with status {}: {}", status, text);
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    panic!("server did not return file content in time");
}

#[tokio::test]
async fn test_stat_file_in_nested_path() {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    mount_table.mount("/mem".to_string(), mem.clone()).await.unwrap();

    // Create directory and test file in nested path, then write content
    mem.mkdir("/nested", 0o755).await.unwrap();
    mem.create("/nested/test.txt", 0o644).await.unwrap();
    mem.write("/nested/test.txt", b"Hello, World!".to_vec(), 0, evif_core::WriteFlags::NONE).await.unwrap();

    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    // Wait for server and test file stat
    for _ in 0..50 {
        let url = format!("http://127.0.0.1:{}/api/v1/stat?path=/mem/nested/test.txt", port);
        if let Ok(res) = reqwest::get(&url).await {
            if res.status().is_success() {
                let json: serde_json::Value = res.json().await.unwrap();
                assert!(json.get("path").is_some(), "response must have 'path' key");
                assert_eq!(json["size"], 13, "file size should be 13 bytes");
                assert_eq!(json["is_dir"], false, "should not be directory");
                return;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    panic!("server did not return file stat in time");
}

#[tokio::test]
async fn test_digest_file_in_nested_path() {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    mount_table.mount("/mem".to_string(), mem.clone()).await.unwrap();

    // Create directory and test file in nested path, then write content
    mem.mkdir("/nested", 0o755).await.unwrap();
    mem.create("/nested/test.txt", 0o644).await.unwrap();
    mem.write("/nested/test.txt", b"Hello, World!".to_vec(), 0, evif_core::WriteFlags::NONE).await.unwrap();

    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    // Wait for server and test file digest
    for _ in 0..50 {
        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{}/api/v1/digest", port);
        let body = serde_json::json!({"path": "/mem/nested/test.txt", "algorithm": "sha256"});

        if let Ok(res) = client.post(&url).json(&body).send().await {
            if res.status().is_success() {
                let json: serde_json::Value = res.json().await.unwrap();
                assert!(json.get("hash").is_some(), "response must have 'hash' key");
                // sha256 of "Hello, World!" is dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f
                assert_eq!(
                    json["hash"],
                    "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f",
                    "checksum should match sha256 of 'Hello, World!'"
                );
                return;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    panic!("server did not return file digest in time");
}

#[tokio::test]
async fn test_create_file_in_nested_path() {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    mount_table.mount("/mem".to_string(), mem.clone()).await.unwrap();

    // Create the nested directory first
    mem.mkdir("/new", 0o755).await.unwrap();

    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    // Wait for server and create file via API
    for _ in 0..50 {
        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{}/api/v1/fs/create", port);
        let body = serde_json::json!({"path": "/mem/new/test.txt"});

        if let Ok(res) = client.post(&url).json(&body).send().await {
            if res.status() == 200 || res.status() == 201 {
                // Verify file was created with correct content
                let content = mem.read("/new/test.txt", 0, 0).await.unwrap();
                assert_eq!(content.len(), 0, "file should be empty");
                return;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    panic!("server did not create file in time");
}

#[tokio::test]
async fn test_write_file_in_nested_path() {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    mount_table.mount("/mem".to_string(), mem.clone()).await.unwrap();

    // Create existing file
    mem.create("/existing.txt", 0o644).await.unwrap();
    mem.write("/existing.txt", b"Initial".to_vec(), 0, evif_core::WriteFlags::NONE).await.unwrap();

    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    // Wait for server and write to file via API
    for _ in 0..50 {
        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{}/api/v1/fs/write", port);
        let body = serde_json::json!({
            "path": "/mem/existing.txt",
            "content": "Updated content"
        });

        if let Ok(res) = client.post(&url).query(&[("path", "/mem/existing.txt")]).json(&body).send().await {
            if res.status().is_success() {
                // Verify content was updated
                let content = mem.read("/existing.txt", 0, 0).await.unwrap();
                assert_eq!(String::from_utf8(content).unwrap(), "Updated content");
                return;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    panic!("server did not write file in time");
}

#[tokio::test]
async fn test_touch_file_in_nested_path() {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    mount_table.mount("/mem".to_string(), mem.clone()).await.unwrap();

    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    // Wait for server and touch new file via API
    for _ in 0..50 {
        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{}/api/v1/touch", port);
        let body = serde_json::json!({"path": "/mem/new.txt"});

        if let Ok(res) = client.post(&url).json(&body).send().await {
            if res.status() == 201 || res.status().is_success() {
                // Verify empty file was created
                let content = mem.read("/new.txt", 0, 0).await.unwrap();
                assert_eq!(content, b"", "file should be empty");
                return;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    panic!("server did not touch file in time");
}

#[tokio::test]
async fn test_create_directory_in_nested_path() {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    mount_table.mount("/mem".to_string(), mem.clone()).await.unwrap();

    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    // Create directory via API using /api/v1/directories endpoint
    for _ in 0..50 {
        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{}/api/v1/directories?path=/mem/new/dir", port);
        let body = serde_json::json!({"path": "/mem/new/dir", "parents": true});

        if let Ok(res) = client.post(&url).json(&body).send().await {
            if res.status() == 201 || res.status().is_success() {
                // Verify directory was created
                let files = mem.readdir("/new").await.unwrap();
                assert!(!files.is_empty(), "directory should be created");
                return;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    panic!("server did not create directory in time");
}

#[tokio::test]
async fn test_delete_directory_in_nested_path() {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    mount_table.mount("/mem".to_string(), mem.clone()).await.unwrap();

    // Create directory to delete
    mem.mkdir("/dir", 0o755).await.unwrap();
    mem.mkdir("/dir/to", 0o755).await.unwrap();
    mem.mkdir("/dir/to/delete", 0o755).await.unwrap();

    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    // Delete directory via API using /api/v1/directories endpoint
    for _ in 0..50 {
        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{}/api/v1/directories?path=/mem/dir/to/delete", port);

        if let Ok(res) = client.delete(&url).send().await {
            if res.status().is_success() {
                // Verify directory was deleted - readdir should fail or return empty
                let result = mem.readdir("/dir/to/delete").await;
                assert!(result.is_err(), "directory should be deleted");

                // Also verify parent still exists
                let parent_files = mem.readdir("/dir/to").await.unwrap();
                assert!(parent_files.is_empty() || !parent_files.iter().any(|f| f.name == "delete"),
                       "deleted directory should not exist");
                return;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    panic!("server did not delete directory in time");
}

/// Test rename operation in nested path (Task 09)
#[tokio::test]
async fn test_rename_file_in_nested_path() {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    mount_table.mount("/mem".to_string(), mem.clone()).await.unwrap();

    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    // Wait for server and create test file
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Create parent directory and file in nested path
    mem.mkdir("/nested", 0o755).await.unwrap();
    mem.create("/nested/old.txt", 0o644).await.unwrap();
    mem.write("/nested/old.txt", b"content".to_vec(), 0, evif_core::WriteFlags::NONE).await.unwrap();

    // Rename file via API
    for _ in 0..50 {
        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{}/api/v1/rename", port);

        if let Ok(res) = client
            .post(&url)
            .json(&serde_json::json!({
                "from": "/mem/nested/old.txt",
                "to": "/mem/nested/new.txt"
            }))
            .send()
            .await
        {
            if res.status().is_success() {
                // Verify old path doesn't exist
                let result = mem.read("/nested/old.txt", 0, 0).await;
                assert!(result.is_err(), "old path should not exist after rename");

                // Verify new path exists with content
                let content = mem.read("/nested/new.txt", 0, 0).await.unwrap();
                assert_eq!(content, b"content");
                return;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    panic!("server did not rename file in time");
}

/// Test grep operation in nested path (Task 09)
#[tokio::test]
async fn test_grep_in_nested_path() {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    mount_table.mount("/mem".to_string(), mem.clone()).await.unwrap();

    let app = create_routes(mount_table);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    // Wait for server and create test files
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Create nested directory with files containing "World"
    mem.mkdir("/nested", 0o755).await.unwrap();
    mem.create("/nested/file1.txt", 0o644).await.unwrap();
    mem.write("/nested/file1.txt", b"Hello World".to_vec(), 0, evif_core::WriteFlags::NONE).await.unwrap();
    mem.create("/nested/file2.txt", 0o644).await.unwrap();
    mem.write("/nested/file2.txt", b"Goodbye World".to_vec(), 0, evif_core::WriteFlags::NONE).await.unwrap();

    // Search via API
    for _ in 0..50 {
        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{}/api/v1/grep", port);

        if let Ok(res) = client
            .post(&url)
            .json(&serde_json::json!({
                "path": "/mem/nested",
                "pattern": "World",
                "recursive": true
            }))
            .send()
            .await
        {
            if res.status().is_success() {
                let body = res.text().await.unwrap();
                let json: serde_json::Value = serde_json::from_str(&body).unwrap();

                if let Some(matches) = json.get("matches") {
                    if let Some(matches_array) = matches.as_array() {
                        // Should find 2 matches (both files contain "World")
                        assert_eq!(matches_array.len(), 2, "should find 2 matches for 'World'");
                        return;
                    }
                }
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    panic!("server did not grep files in time");
}
