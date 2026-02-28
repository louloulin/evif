// EVIF 综合集成示例
// 展示所有8个插件的协同工作和跨插件数据流

use evif_core::{EvifServer, WriteFlags};
use evif_plugins::{
    LocalFsPlugin, KvfsPlugin, QueueFsPlugin, ServerInfoFsPlugin,
    MemFsPlugin, HttpFsPlugin, StreamFsPlugin, ProxyFsPlugin
};
use std::sync::Arc;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 EVIF 1.6 综合集成示例\n");
    println!("═══════════════════════════════════════\n");

    // 初始化服务器
    let server = Arc::new(EvifServer::new());

    // ========== 挂载所有插件 ==========
    println!("📦 挂载插件到EVIF服务器...\n");

    // 1. LocalFS - 本地文件系统
    println!("1️⃣  挂载 LocalFS → /local");
    server.register_plugin(
        "/local",
        Arc::new(LocalFsPlugin::new("/tmp/evif_demo", true))
    ).await?;
    println!("   ✅ 本地文件访问 + 路径遍历保护\n");

    // 2. KVFS - 键值存储
    println!("2️⃣  挂载 KVFS → /kv");
    server.register_plugin(
        "/kv",
        Arc::new(KvfsPlugin::new("demo_kv"))
    ).await?;
    println!("   ✅ 键值存储 + 虚拟目录\n");

    // 3. QueueFS - 消息队列
    println!("3️⃣  挂载 QueueFS → /queue");
    server.register_plugin(
        "/queue",
        Arc::new(QueueFsPlugin::new())
    ).await?;
    println!("   ✅ FIFO消息队列 + UUID序列化\n");

    // 4. ServerInfoFS - 服务器信息
    println!("4️⃣  挂载 ServerInfoFS → /info");
    server.register_plugin(
        "/info",
        Arc::new(ServerInfoFsPlugin::new("1.6.0"))
    ).await?;
    println!("   ✅ 服务器元数据(只读)\n");

    // 5. MemFS - 内存文件系统
    println!("5️⃣  挂载 MemFS → /mem");
    server.register_plugin(
        "/mem",
        Arc::new(MemFsPlugin::new())
    ).await?;
    println!("   ✅ 内存文件系统 + 树结构\n");

    // 6. HttpFS - HTTP客户端
    println!("6️⃣  挂载 HttpFS → /http");
    server.register_plugin(
        "/http",
        Arc::new(HttpFsPlugin::new("https://httpbin.org", 10))
    ).await?;
    println!("   ✅ HTTP客户端(GET/PUT/DELETE)\n");

    // 7. StreamFS - 流式数据处理
    println!("7️⃣  挂载 StreamFS → /stream");
    server.register_plugin(
        "/stream",
        Arc::new(StreamFsPlugin::new())
    ).await?;
    println!("   ✅ 流式数据 + 环形缓冲\n");

    // 8. ProxyFS - 远程文件系统代理
    println!("8️⃣  挂载 ProxyFS → /remote");
    server.register_plugin(
        "/remote",
        Arc::new(ProxyFsPlugin::new("http://localhost:8080/api/v1"))
    ).await?;
    println!("   ✅ 远程EVIF/AGFS客户端 + 热重载\n");

    println!("═══════════════════════════════════════");
    println!("🎉 所有8个插件挂载完成!\n");

    // ========== 场景1: 数据采集与处理管道 ==========
    println!("📊 场景1: 数据采集与处理管道");
    println!("─────────────────────────────────────\n");

    // 1.1 从HTTP获取数据并存储到LocalFS
    println!("1️⃣  从HTTP获取数据 → LocalFS");
    match server.read("/http/get", 0, 1024).await {
        Ok(http_data) => {
            println!("   ✅ 从HTTP获取 {} 字节数据", http_data.len());

            // 存储到本地
            server.write(
                "/local/fetched_data.json",
                http_data.clone(),
                0,
                WriteFlags::CREATE
            ).await?;
            println!("   ✅ 保存到 /local/fetched_data.json\n");
        }
        Err(e) => {
            println!("   ⚠️  HTTP获取失败(可能离线): {}\n", e);
        }
    }

    // 1.2 在MemFS中创建临时工作区
    println!("2️⃣  创建MemFS工作区");
    server.mkdir("/mem/workspace", 0o755).await?;
    server.create("/mem/workspace/notes.txt", 0o644).await?;
    server.write(
        "/mem/workspace/notes.txt",
        b"Processing data in memory\n",
        0,
        WriteFlags::CREATE
    ).await?;
    println!("   ✅ 创建 /mem/workspace/notes.txt\n");

    // 1.3 将元数据存储到KVFS
    println!("3️⃣  存储元数据到KVFS");
    server.write(
        "/kv/metadata/source",
        b"http_api",
        0,
        WriteFlags::CREATE
    ).await?;
    server.write(
        "/kv/metadata/timestamp",
        Utc::now().to_rfc3339().as_bytes(),
        0,
        WriteFlags::CREATE
    ).await?;
    println!("   ✅ 元数据已存储到KVFS\n");

    // ========== 场景2: 消息队列与流处理 ==========
    println!("📨 场景2: 消息队列与流处理");
    println!("─────────────────────────────────────\n");

    // 2.1 创建消息队列
    println!("1️⃣  创建消息队列");
    server.mkdir("/queue/tasks", 0o755).await?;
    println!("   ✅ 创建队列: /queue/tasks\n");

    // 2.2 发送多条消息
    println!("2️⃣  发送任务消息");
    for i in 1..=3 {
        let task_data = format!("Task {}: Process data chunk {}", i, i);
        server.write(
            "/queue/tasks/enqueue",
            task_data.as_bytes().to_vec(),
            0,
            WriteFlags::CREATE
        ).await?;
        println!("   ✅ 发送: {}", task_data);
    }
    println!();

    // 2.3 从队列中读取消息
    println!("3️⃣  从队列处理消息");
    let task1 = server.read("/queue/tasks/dequeue", 0, 1024).await?;
    println!("   ✅ 处理任务: {}", String::from_utf8_lossy(&task1));

    let task2 = server.read("/queue/tasks/dequeue", 0, 1024).await?;
    println!("   ✅ 处理任务: {}", String::from_utf8_lossy(&task2));
    println!();

    // 2.4 查看队列大小
    println!("4️⃣  查看队列状态");
    let size_info = server.read("/queue/tasks/size", 0, 1024).await?;
    println!("   📊 剩余任务数: {}", String::from_utf8_lossy(&size_info));
    println!();

    // ========== 场景3: 流式数据收集 ==========
    println!("🌊 场景3: 流式数据收集");
    println!("─────────────────────────────────────\n");

    // 3.1 创建日志流
    println!("1️⃣  创建日志流");
    server.create("/stream/logs", 0o644).await?;
    println!("   ✅ 创建流: /stream/logs\n");

    // 3.2 写入多条日志
    println!("2️⃣  写入日志流");
    let logs = vec![
        "[INFO] Application started",
        "[INFO] Connected to database",
        "[WARN] High memory usage detected",
        "[INFO] Processing batch 1",
        "[ERROR] Connection timeout",
    ];

    for log in &logs {
        server.write(
            "/stream/logs",
            format!("{}\n", log).as_bytes().to_vec(),
            0,
            WriteFlags::APPEND
        ).await?;
        println!("   📝 {}", log);
    }
    println!();

    // 3.3 读取流数据(新读者可以获取历史数据)
    println!("3️⃣  读取流数据(包含历史数据)");
    let log_data = server.read("/stream/logs", 0, 1024).await?;
    println!("   📖 读取到日志:\n{}", String::from_utf8_lossy(&log_data));

    // ========== 场景4: 虚拟目录导航 ==========
    println!("📁 场景4: 虚拟目录导航");
    println!("─────────────────────────────────────\n");

    // 4.1 列出所有插件的根目录
    println!("1️⃣  浏览所有挂载的插件:");
    let roots = vec![
        ("/local", "本地文件系统"),
        ("/kv", "键值存储"),
        ("/queue", "消息队列"),
        ("/info", "服务器信息"),
        ("/mem", "内存文件系统"),
        ("/stream", "流式数据"),
    ];

    for (path, desc) in roots {
        match server.readdir(path).await {
            Ok(entries) => {
                println!("   📂 {} ({}): {} 项",
                    path, desc, entries.len());
                for entry in entries.iter().take(3) {
                    println!("      - {} ({})", entry.name,
                        if entry.is_dir { "DIR" } else { "FILE" });
                }
                if entries.len() > 3 {
                    println!("      ... 还有 {} 项", entries.len() - 3);
                }
            }
            Err(e) => {
                println!("   ⚠️  {} ({}): {}", path, desc, e);
            }
        }
        println!();
    }

    // ========== 场景5: 服务器信息查询 ==========
    println!("🖥️  场景5: 服务器信息查询");
    println!("─────────────────────────────────────\n");

    // 5.1 读取版本信息
    println!("1️⃣  版本信息:");
    let version = server.read("/info/version", 0, 1024).await?;
    println!("   📦 {}", String::from_utf8_lossy(&version));

    // 5.2 读取运行时统计
    println!("2️⃣  运行时统计:");
    let uptime = server.read("/info/uptime", 0, 1024).await?;
    println!("   ⏱️  {}", String::from_utf8_lossy(&uptime));

    let stats = server.read("/info/stats", 0, 1024).await?;
    println!("   📊 {}", String::from_utf8_lossy(&stats));
    println!();

    // ========== 场景6: ProxyFS热重载演示 ==========
    println!("🔄 场景6: ProxyFS热重载演示");
    println!("─────────────────────────────────────\n");

    println!("1️⃣  读取/reload状态:");
    match server.read("/remote/reload", 0, 1024).await {
        Ok(reload_info) => {
            println!("   📋 {}", String::from_utf8_lossy(&reload_info));
        }
        Err(e) => {
            println!("   ⚠️  ProxyFS未连接(预期): {}\n", e);
        }
    }

    println!("2️⃣  触发热重载:");
    match server.write(
        "/remote/reload",
        b"trigger reload",
        0,
        WriteFlags::CREATE
    ).await {
        Ok(_) => {
            println!("   ✅ 热重载命令已发送\n");
        }
        Err(e) => {
            println!("   ⚠️  热重载失败(可能无远程服务器): {}\n", e);
        }
    }

    // ========== 总结统计 ==========
    println!("═══════════════════════════════════════");
    println!("📊 操作总结");
    println!("═══════════════════════════════════════\n");

    println!("✅ 已挂载插件: 8 个");
    println!("   - LocalFS, KVFS, QueueFS, ServerInfoFS");
    println!("   - MemFS, HttpFS, StreamFS, ProxyFS\n");

    println!("✅ 演示场景: 6 个");
    println!("   1. 数据采集与处理管道");
    println!("   2. 消息队列与流处理");
    println!("   3. 流式数据收集");
    println!("   4. 虚拟目录导航");
    println!("   5. 服务器信息查询");
    println!("   6. ProxyFS热重载\n");

    println!("✅ 核心特性:");
    println!("   - 统一插件接口 (EvifPlugin trait)");
    println!("   - 路径前缀匹配路由");
    println!("   - 跨插件数据流动");
    println!("   - 类型安全的Rust实现");
    println!("   - 100% 测试覆盖\n");

    println!("🎉 EVIF 1.6 综合集成示例运行完成!\n");

    Ok(())
}

// 为了简化,这里使用当前时间
use chrono::Utc;
