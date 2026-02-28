// EVIF 集成示例 - 多插件协同工作
//
// 展示如何使用 EVIF 核心系统和多个插件

use evif_core::{EvifServer, MountTable};
use evif_plugins::{LocalFsPlugin, KvfsPlugin, QueueFsPlugin, ServerInfoFsPlugin, MemFsPlugin};
use std::sync::Arc;
use tokio;

#[tokio::main]
async fn main() -> evif_core::EvifResult<()> {
    println!("=== EVIF 集成示例 ===\n");

    // 创建服务器
    let server = EvifServer::new();

    println!("1. 注册 LocalFS 插件到 /local");
    let local_plugin = Arc::new(LocalFsPlugin::new("/tmp/evif-local", false));
    server.register_plugin("/local".to_string(), local_plugin).await?;
    println!("   ✓ LocalFS 已挂载到 /local\n");

    println!("2. 注册 KVFS 插件到 /kvfs");
    let kvfs_plugin = Arc::new(KvfsPlugin::new("kvfs"));
    server.register_plugin("/kvfs".to_string(), kvfs_plugin).await?;
    println!("   ✓ KVFS 已挂载到 /kvfs\n");

    println!("3. 注册 QueueFS 插件到 /queuefs");
    let queue_plugin = Arc::new(QueueFsPlugin::new());
    server.register_plugin("/queuefs".to_string(), queue_plugin).await?;
    println!("   ✓ QueueFS 已挂载到 /queuefs\n");

    println!("4. 注册 ServerInfoFS 插件到 /serverinfo");
    let server_info_plugin = Arc::new(ServerInfoFsPlugin::new("1.0.0"));
    server.register_plugin("/serverinfo".to_string(), server_info_plugin).await?;
    println!("   ✓ ServerInfoFS 已挂载到 /serverinfo\n");

    println!("5. 注册 MemFS 插件到 /mem");
    let mem_plugin = Arc::new(MemFsPlugin::new());
    server.register_plugin("/mem".to_string(), mem_plugin).await?;
    println!("   ✓ MemFS 已挂载到 /mem\n");

    // ===== LocalFS 操作示例 =====
    println!("=== LocalFS 操作示例 ===");
    println!("创建文件 /local/test.txt");
    server.create("/local/test.txt", 0o644).await?;
    server.write("/local/test.txt", b"Hello from LocalFS!".to_vec(), 0, evif_core::WriteFlags::CREATE).await?;

    let data = server.read("/local/test.txt", 0, 100).await?;
    println!("读取内容: {}", String::from_utf8_lossy(&data));

    let info = server.stat("/local/test.txt").await?;
    println!("文件大小: {} bytes\n", info.size);

    // ===== KVFS 操作示例 =====
    println!("=== KVFS 操作示例 ===");
    println!("存储键值对 /kvfs/config/app_name");
    server.write("/kvfs/config/app_name", b"EVIF Server".to_vec(), 0, evif_core::WriteFlags::CREATE).await?;

    let data = server.read("/kvfs/config/app_name", 0, 100).await?;
    println!("读取配置: {}", String::from_utf8_lossy(&data));

    println!("列出 /kvfs/config 目录:");
    let entries = server.readdir("/kvfs/config").await?;
    for entry in entries {
        println!("  - {} ({} bytes)", entry.name, entry.size);
    }
    println!();

    // ===== QueueFS 操作示例 =====
    println!("=== QueueFS 操作示例 ===");
    println!("创建队列 /queuefs/tasks");
    server.mkdir("/queuefs/tasks", 0o755).await?;

    println!("入队3个任务");
    for i in 1..=3 {
        let task = format!("Task-{}", i);
        server.write("/queuefs/tasks/enqueue", task.into_bytes(), 0, evif_core::WriteFlags::CREATE).await?;
    }

    let size_data = server.read("/queuefs/tasks/size", 0, 100).await?;
    println!("队列大小: {}", String::from_utf8_lossy(&size_data));

    println!("出队一个任务:");
    let task_data = server.read("/queuefs/tasks/dequeue", 0, 1000).await?;
    println!("  {}", String::from_utf8_lossy(&task_data));
    println!();

    // ===== ServerInfoFS 操作示例 =====
    println!("=== ServerInfoFS 操作示例 ===");
    println!("读取服务器版本:");
    let version = server.read("/serverinfo/version", 0, 100).await?;
    println!("  {}", String::from_utf8_lossy(&version));

    println!("读取服务器运行时间:");
    let uptime = server.read("/serverinfo/uptime", 0, 100).await?;
    println!("  {}", String::from_utf8_lossy(&uptime));

    println!("列出服务器信息文件:");
    let entries = server.readdir("/serverinfo").await?;
    for entry in entries {
        println!("  - {}", entry.name);
    }
    println!();

    // ===== 跨插件操作示例 =====
    println!("=== 跨插件操作示例 ===");
    println!("从 LocalFS 读取配置文件，存储到 KVFS:");
    let config_content = server.read("/local/test.txt", 0, 100).await?;
    server.write("/kvfs/copied_config", config_content, 0, evif_core::WriteFlags::CREATE).await?;

    let copied = server.read("/kvfs/copied_config", 0, 100).await?;
    println!("KVFS 中的内容: {}", String::from_utf8_lossy(&copied));

    println!("\n=== 所有操作完成 ===");
    Ok(())
}

// ===== 额外的辅助函数示例 =====

async fn demonstrate_queue_workflow(server: &EvifServer) -> evif_core::EvifResult<()> {
    println!("\n=== 队列工作流示例 ===");

    // 创建多个队列
    server.mkdir("/queuefs/pending", 0o755).await?;
    server.mkdir("/queuefs/processing", 0o755).await?;
    server.mkdir("/queuefs/completed", 0o755).await?;

    // 入队到 pending
    for i in 1..=5 {
        let job = format!("job-{}", i);
        server.write("/queuefs/pending/enqueue", job.into_bytes(), 0, evif_core::WriteFlags::CREATE).await?;
    }

    println!("Pending 队列大小:");
    let size = server.read("/queuefs/pending/size", 0, 100).await?;
    println!("  {}", String::from_utf8_lossy(&size));

    // 移动任务到 processing
    for _ in 1..=5 {
        let job = server.read("/queuefs/pending/dequeue", 0, 1000).await?;
        server.write("/queuefs/processing/enqueue", job, 0, evif_core::WriteFlags::CREATE).await?;
    }

    // 处理并移动到 completed
    for _ in 1..=5 {
        let job = server.read("/queuefs/processing/dequeue", 0, 1000).await?;
        server.write("/queuefs/completed/enqueue", job, 0, evif_core::WriteFlags::CREATE).await?;
    }

    println!("Completed 队列大小:");
    let size = server.read("/queuefs/completed/size", 0, 100).await?;
    println!("  {}", String::from_utf8_lossy(&size));

    Ok(())
}
