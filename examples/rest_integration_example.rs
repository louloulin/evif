// EVIF REST API 集成示例
// 展示如何将新的插件架构集成到现有的 REST API 服务

use evif_core::{EvifServer, MountTable};
use evif_plugins::{
    LocalFsPlugin, KvfsPlugin, QueueFsPlugin, ServerInfoFsPlugin,
    MemFsPlugin, HttpFsPlugin, StreamFsPlugin
};
use std::sync::Arc;
use warp::Filter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化 EVIF 服务器
    let evif_server = Arc::new(EvifServer::new());

    // 挂载所有插件
    setup_plugins(&evif_server).await?;

    // 启动 REST API
    start_rest_api(evif_server).await?;

    Ok(())
}

/// 设置所有插件
async fn setup_plugins(server: &Arc<EvifServer>) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Initializing EVIF Plugins...\n");

    // 1. LocalFS - 本地文件系统
    println!("1️⃣  Mounting LocalFS at /local");
    server.register_plugin(
        "/local",
        Arc::new(LocalFsPlugin::new("/tmp/evif", false))
    ).await?;
    println!("   ✅ LocalFS mounted\n");

    // 2. KVFS - 键值存储
    println!("2️⃣  Mounting KVFS at /kvfs");
    server.register_plugin(
        "/kvfs",
        Arc::new(KvfsPlugin::new("evif_kv"))
    ).await?;
    println!("   ✅ KVFS mounted\n");

    // 3. QueueFS - 消息队列
    println!("3️⃣  Mounting QueueFS at /queue");
    server.register_plugin(
        "/queue",
        Arc::new(QueueFsPlugin::new())
    ).await?;
    println!("   ✅ QueueFS mounted\n");

    // 4. ServerInfoFS - 服务器信息
    println!("4️⃣  Mounting ServerInfoFS at /server");
    server.register_plugin(
        "/server",
        Arc::new(ServerInfoFsPlugin::new("1.6.0"))
    ).await?;
    println!("   ✅ ServerInfoFS mounted\n");

    // 5. MemFS - 内存文件系统
    println!("5️⃣  Mounting MemFS at /mem");
    server.register_plugin(
        "/mem",
        Arc::new(MemFsPlugin::new())
    ).await?;
    println!("   ✅ MemFS mounted\n");

    // 6. HttpFS - HTTP 客户端
    println!("6️⃣  Mounting HttpFS at /http");
    server.register_plugin(
        "/http",
        Arc::new(HttpFsPlugin::new("https://httpbin.org", 30))
    ).await?;
    println!("   ✅ HttpFS mounted\n");

    // 7. StreamFS - 流式数据
    println!("7️⃣  Mounting StreamFS at /stream");
    server.register_plugin(
        "/stream",
        Arc::new(StreamFsPlugin::new())
    ).await?;
    println!("   ✅ StreamFS mounted\n");

    println!("🎉 All plugins mounted successfully!\n");
    Ok(())
}

/// 启动 REST API
async fn start_rest_api(server: Arc<EvifServer>) -> Result<(), Box<dyn std::error::Error>> {
    println!("🌐 Starting REST API server on http://0.0.0.0:8080\n");

    // 文件读取路由
    let read_route = warp::path("api")
        .and(warp::path("v1"))
        .and(warp::path("read"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and_then({
            let server = Arc::clone(&server);
            move |path: String| {
                let server = Arc::clone(&server);
                async move {
                    let result = server.read(&format!("/{}", path), 0, 1024 * 1024).await;

                    match result {
                        Ok(data) => {
                            let content = String::from_utf8_lossy(&data).to_string();
                            Ok(warp::reply::json(&serde_json::json!({
                                "status": "success",
                                "path": path,
                                "data": content,
                                "size": data.len()
                            })))
                        }
                        Err(e) => {
                            Ok(warp::reply::json(&serde_json::json!({
                                "status": "error",
                                "error": e.to_string()
                            })))
                        }
                    }
                }
            }
        });

    // 文件写入路由
    let write_route = warp::path("api")
        .and(warp::path("v1"))
        .and(warp::path("write"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::body::json())
        .and_then({
            let server = Arc::clone(&server);
            move |path: String, body: serde_json::Value| {
                let server = Arc::clone(&server);
                async move {
                    let data = body.get("data")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .as_bytes()
                        .to_vec();

                    let result = server.write(
                        &format!("/{}", path),
                        data,
                        0,
                        evif_core::WriteFlags::CREATE
                    ).await;

                    match result {
                        Ok(size) => {
                            Ok(warp::reply::json(&serde_json::json!({
                                "status": "success",
                                "path": path,
                                "bytes_written": size
                            })))
                        }
                        Err(e) => {
                            Ok(warp::reply::json(&serde_json::json!({
                                "status": "error",
                                "error": e.to_string()
                            })))
                        }
                    }
                }
            }
        });

    // 目录列出路由
    let readdir_route = warp::path("api")
        .and(warp::path("v1"))
        .and(warp::path("readdir"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and_then({
            let server = Arc::clone(&server);
            move |path: String| {
                let server = Arc::clone(&server);
                async move {
                    let result = server.readdir(&format!("/{}", path)).await;

                    match result {
                        Ok(entries) => {
                            Ok(warp::reply::json(&serde_json::json!({
                                "status": "success",
                                "path": path,
                                "entries": entries
                            })))
                        }
                        Err(e) => {
                            Ok(warp::reply::json(&serde_json::json!({
                                "status": "error",
                                "error": e.to_string()
                            })))
                        }
                    }
                }
            }
        });

    // 文件状态路由
    let stat_route = warp::path("api")
        .and(warp::path("v1"))
        .and(warp::path("stat"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and_then({
            let server = Arc::clone(&server);
            move |path: String| {
                let server = Arc::clone(&server);
                async move {
                    let result = server.stat(&format!("/{}", path)).await;

                    match result {
                        Ok(info) => {
                            Ok(warp::reply::json(&serde_json::json!({
                                "status": "success",
                                "info": info
                            })))
                        }
                        Err(e) => {
                            Ok(warp::reply::json(&serde_json::json!({
                                "status": "error",
                                "error": e.to_string()
                            })))
                        }
                    }
                }
            }
        });

    // 健康检查路由
    let health_route = warp::path("health")
        .map(|| {
            warp::reply::json(&serde_json::json!({
                "status": "healthy",
                "version": "1.6.0",
                "plugins": [
                    "localfs",
                    "kvfs",
                    "queuefs",
                    "serverinfofs",
                    "memfs",
                    "httpfs",
                    "streamfs"
                ]
            }))
        });

    // 组合所有路由
    let routes = read_route
        .or(write_route)
        .or(readdir_route)
        .or(stat_route)
        .or(health_route)
        .with(warp::cors().allow_any_origin().allow_methods(vec!["GET", "POST", "OPTIONS"]));

    // 启动服务器
    warp::serve(routes)
        .run(([0, 0, 0, 0], 8080))
        .await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use evif_core::WriteFlags;

    #[tokio::test]
    async fn test_rest_integration() {
        let server = Arc::new(EvifServer::new());

        // 挂载测试插件
        server.register_plugin(
            "/test",
            Arc::new(MemFsPlugin::new())
        ).await.unwrap();

        // 测试文件操作
        server.create("/test/file.txt", 0o644).await.unwrap();
        server.write(
            "/test/file.txt",
            b"Hello, REST API!".to_vec(),
            0,
            WriteFlags::CREATE
        ).await.unwrap();

        let data = server.read("/test/file.txt", 0, 100).await.unwrap();
        assert_eq!(String::from_utf8_lossy(&data), "Hello, REST API!");

        // 测试目录列出
        let entries = server.readdir("/test").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "file.txt");

        // 测试文件状态
        let info = server.stat("/test/file.txt").await.unwrap();
        assert_eq!(info.name, "file.txt");
        assert_eq!(info.size, 16);
    }
}
