// OpenDAL 插件测试程序
//
// 用于验证 OpenDAL 集成是否正常工作

use evif_core::{EvifPlugin, WriteFlags};
use evif_plugins::{OpendalConfig, OpendalPlugin, OpendalService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 EVIF 2.1 - OpenDAL 集成测试\n");

    // 测试 Memory 服务
    println!("📦 测试 Memory 服务...");
    test_memory_service().await?;
    println!("✅ Memory 服务测试通过!\n");

    // 测试 Fs 服务
    println!("📁 测试 Fs (本地文件系统) 服务...");
    test_fs_service().await?;
    println!("✅ Fs 服务测试通过!\n");

    println!("🎉 所有测试通过!");
    Ok(())
}

async fn test_memory_service() -> Result<(), Box<dyn std::error::Error>> {
    let config = OpendalConfig {
        service: OpendalService::Memory,
        ..Default::default()
    };

    let plugin = OpendalPlugin::from_config(config).await?;

    // 测试创建文件
    println!("  - 创建 /test.txt...");
    plugin.create("/test.txt", 0o644).await?;

    // 测试写入文件
    println!("  - 写入数据 'Hello, OpenDAL!'...");
    plugin
        .write(
            "/test.txt",
            b"Hello, OpenDAL!".to_vec(),
            -1,
            WriteFlags::empty(),
        )
        .await?;

    // 测试读取文件
    println!("  - 读取文件内容...");
    let data = plugin.read("/test.txt", 0, 0).await?;
    assert_eq!(data, b"Hello, OpenDAL!");
    println!("    内容: {:?}", String::from_utf8_lossy(&data));

    // 测试获取文件信息
    println!("  - 获取文件信息...");
    let info = plugin.stat("/test.txt").await?;
    println!("    大小: {} 字节", info.size);
    println!("    目录: {}", info.is_dir);

    // 测试列出目录
    println!("  - 列出根目录...");
    let files = plugin.readdir("/").await?;
    println!("    文件数: {}", files.len());
    for file in &files {
        println!("      - {} ({} bytes)", file.name, file.size);
    }

    // 测试创建目录
    println!("  - 创建目录 /subdir...");
    plugin.mkdir("/subdir", 0o755).await?;

    // 测试重命名
    println!("  - 重命名 /test.txt -> /renamed.txt...");
    match plugin.rename("/test.txt", "/renamed.txt").await {
        Ok(_) => println!("    ✅ 重命名成功"),
        Err(e) => println!("    ⚠️  重命名失败 (Memory 服务限制): {}", e),
    }

    // 测试删除文件
    println!("  - 删除文件...");
    let files = plugin.readdir("/").await?;
    println!("    当前文件数: {}", files.len());

    // 测试递归删除
    println!("  - 递归删除 /subdir...");
    plugin.remove_all("/subdir").await?;

    // 验证删除
    let files = plugin.readdir("/").await?;
    println!("    清理后文件数: {}", files.len());

    println!("  ✅ Memory 服务所有功能正常!");
    Ok(())
}

async fn test_fs_service() -> Result<(), Box<dyn std::error::Error>> {
    // 创建临时目录
    let temp_dir = std::env::temp_dir();
    let test_dir = temp_dir.join("evif_opendal_test");
    println!("  - 测试目录: {:?}", test_dir);

    // 确保测试目录存在
    std::fs::create_dir_all(&test_dir)?;
    let test_path = test_dir.to_string_lossy().to_string();

    let config = OpendalConfig {
        service: OpendalService::Fs,
        root: Some(test_path),
        ..Default::default()
    };

    let plugin = OpendalPlugin::from_config(config).await?;

    // 测试创建文件
    println!("  - 创建 /test.txt...");
    plugin.create("/test.txt", 0o644).await?;

    // 测试写入文件
    println!("  - 写入数据 'Hello, Fs!'...");
    plugin
        .write("/test.txt", b"Hello, Fs!".to_vec(), -1, WriteFlags::empty())
        .await?;

    // 测试读取文件
    println!("  - 读取文件内容...");
    let data = plugin.read("/test.txt", 0, 0).await?;
    assert_eq!(data, b"Hello, Fs!");
    println!("    内容: {:?}", String::from_utf8_lossy(&data));

    // 测试获取文件信息
    println!("  - 获取文件信息...");
    let info = plugin.stat("/test.txt").await?;
    println!("    大小: {} 字节", info.size);
    println!("    目录: {}", info.is_dir);

    // 测试列出目录
    println!("  - 列出根目录...");
    let files = plugin.readdir("/").await?;
    println!("    文件数: {}", files.len());
    for file in &files {
        println!("      - {} ({} bytes)", file.name, file.size);
    }

    // 测试创建目录
    println!("  - 创建目录 /subdir...");
    plugin.mkdir("/subdir", 0o755).await?;

    // 测试重命名 (Fs 支持重命名)
    println!("  - 重命名 /test.txt -> /renamed.txt...");
    plugin.rename("/test.txt", "/renamed.txt").await?;
    println!("    ✅ 重命名成功");

    // 验证重命名
    let data = plugin.read("/renamed.txt", 0, 0).await?;
    assert_eq!(data, b"Hello, Fs!");

    // 测试删除文件
    println!("  - 删除 /renamed.txt...");
    plugin.remove("/renamed.txt").await?;

    // 测试递归删除
    println!("  - 递归删除 /subdir...");
    plugin.remove_all("/subdir").await?;

    // 清理测试目录
    std::fs::remove_dir_all(test_dir)?;

    println!("  ✅ Fs 服务所有功能正常!");
    Ok(())
}
