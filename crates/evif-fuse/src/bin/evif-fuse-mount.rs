// EVIF FUSE 挂载示例程序
//
// 演示如何使用 FUSE 将 EVIF 挂载为本地文件系统
//
// 使用方法：
//   cargo run --bin evif-fuse-mount -- <mount_point> [options]
//
// 示例：
//   cargo run --bin evif-fuse-mount -- /tmp/evif --readonly
//   cargo run --bin evif-fuse-mount -- /tmp/evif --readwrite

use evif_core::RadixMountTable;
use evif_fuse::{mount_evif_background, FuseMountConfig, FuseMountBuilder};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    // 解析命令行参数
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <mount_point> [options]", args[0]);
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --readonly       只读挂载（默认）");
        eprintln!("  --readwrite      读写挂载");
        eprintln!("  --allow-other    允许其他用户访问");
        eprintln!("  --cache-size N   设置缓存大小（默认：10000）");
        eprintln!("  --cache-timeout N 设置缓存超时秒数（默认：60）");
        eprintln!();
        eprintln!("示例:");
        eprintln!("  {} /tmp/evif --readonly", args[0]);
        eprintln!("  {} /tmp/evif --readwrite --allow-other", args[0]);
        std::process::exit(1);
    }

    let mount_point = PathBuf::from(&args[1]);
    let mut allow_write = false;
    let mut allow_other = false;
    let mut cache_size = 10000;
    let mut cache_timeout = 60;

    // 解析选项
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--readonly" => {
                allow_write = false;
            }
            "--readwrite" => {
                allow_write = true;
            }
            "--allow-other" => {
                allow_other = true;
            }
            "--cache-size" => {
                if i + 1 < args.len() {
                    cache_size = args[i + 1].parse().unwrap_or(10000);
                    i += 1;
                }
            }
            "--cache-timeout" => {
                if i + 1 < args.len() {
                    cache_timeout = args[i + 1].parse().unwrap_or(60);
                    i += 1;
                }
            }
            _ => {
                eprintln!("未知选项: {}", args[i]);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    // 检查挂载点是否存在
    if !mount_point.exists() {
        error!("挂载点不存在: {}", mount_point.display());
        eprintln!("请先创建挂载点目录:");
        eprintln!("  mkdir -p {}", mount_point.display());
        std::process::exit(1);
    }

    // 创建挂载表
    let mount_table = Arc::new(RadixMountTable::new());

    // 挂载插件示例
    // 注意：在使用前，需要在 Cargo.toml 中添加相应插件依赖
    //
    // 示例 1: 挂载本地文件系统
    // let localfs_plugin = evif_plugins::LocalFs::new("/path/to/data")?;
    // mount_table.mount("/local", Arc::new(localfs_plugin)).await?;
    //
    // 示例 2: 挂载内存文件系统
    // let memfs_plugin = evif_plugins::MemFs::new()?;
    // mount_table.mount("/mem", Arc::new(memfs_plugin)).await?;
    //
    // 示例 3: 挂载 HTTP 文件系统
    // let httpfs_plugin = evif_plugins::HttpFs::new("https://example.com")?;
    // mount_table.mount("/http", Arc::new(httpfs_plugin)).await?;
    //
    // 示例 4: 挂载键值存储
    // let kvfs_plugin = evif_plugins::KvFs::new()?;
    // mount_table.mount("/kv", Arc::new(kvfs_plugin)).await?;

    info!("挂载表初始化完成");
    info!("提示: 使用前请先挂载所需的插件");

    // 构建挂载配置
    let config = FuseMountBuilder::new()
        .mount_point(&mount_point)
        .root_path(Path::new("/"))
        .allow_write(allow_write)
        .allow_other(allow_other)
        .cache_size(cache_size)
        .cache_timeout(cache_timeout)
        .build()?;

    info!("挂载配置:");
    info!("  挂载点: {}", config.mount_point.display());
    info!("  根路径: {}", config.root_path.display());
    info!("  写操作: {}", if config.allow_write { "允许" } else { "禁止" });
    info!("  其他用户: {}", if config.allow_other { "允许" } else { "禁止" });
    info!("  缓存大小: {}", config.cache_size);
    info!("  缓存超时: {} 秒", config.cache_timeout);

    // 挂载 FUSE 文件系统
    info!("开始挂载 FUSE 文件系统...");
    // 后台挂载（返回 session）
    let session = mount_evif_background(mount_table, &mount_point, config)?;

    info!("✓ FUSE 文件系统挂载成功!");
    info!("  挂载点: {}", mount_point.display());
    info!("");
    info!("提示:");
    info!("  使用 Ctrl+C 卸载文件系统");
    info!("  或使用命令: fusermount -u {}", mount_point.display());
    info!("");

    // 等待信号
    info!("文件系统运行中，按 Ctrl+C 停止...");

    // 等待 Ctrl+C 信号
    tokio::signal::ctrl_c().await?;
    info!("收到 Ctrl+C 信号，开始卸载...");

    info!("文件系统已卸载");
    Ok(())
}
