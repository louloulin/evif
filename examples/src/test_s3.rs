// S3 测试程序 - 基于 OpenDAL
//
// 测试 S3 和 S3 兼容存储 (MinIO) 的 OpenDAL 集成

use evif_core::{EvifPlugin, WriteFlags};
use evif_plugins::{OpendalPlugin, OpendalConfig, OpendalService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 EVIF 2.1 - S3 OpenDAL 集成测试\n");

    // 测试说明
    println!("📋 S3 测试说明:");
    println!("  本测试需要以下环境变量之一:");
    println!("  1. AWS S3:");
    println!("     - AWS_S3_BUCKET");
    println!("     - AWS_S3_REGION (可选，默认 us-east-1)");
    println!("     - AWS_ACCESS_KEY_ID");
    println!("     - AWS_SECRET_ACCESS_KEY");
    println!("  2. MinIO:");
    println!("     - MINIO_ENDPOINT (例如: http://localhost:9000)");
    println!("     - MINIO_BUCKET");
    println!("     - MINIO_ACCESS_KEY");
    println!("     - MINIO_SECRET_KEY");
    println!();

    // 检查环境变量
    let use_minio = std::env::var("MINIO_ENDPOINT").is_ok();

    if use_minio {
        println!("🔧 使用 MinIO 模式");
        test_minio().await?;
    } else if std::env::var("AWS_S3_BUCKET").is_ok() {
        println!("🔧 使用 AWS S3 模式");
        test_aws_s3().await?;
    } else {
        println!("⚠️  未配置 S3 环境变量，跳过测试");
        println!("   如需测试，请设置上述环境变量");
        println!("\n💡 提示: 您可以使用 Docker 运行 MinIO:");
        println!("   docker run -p 9000:9000 -p 9001:9001 \\");
        println!("     -e MINIO_ROOT_USER=minioadmin \\");
        println!("     -e MINIO_ROOT_PASSWORD=minioadmin \\");
        println!("     minio/minio server /data --console-address ':9001'");
        return Ok(());
    }

    println!("\n🎉 所有 S3 测试通过!");
    Ok(())
}

async fn test_minio() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = std::env::var("MINIO_ENDPOINT")?;
    let bucket = std::env::var("MINIO_BUCKET").unwrap_or_else(|_| "test-bucket".to_string());
    let access_key = std::env::var("MINIO_ACCESS_KEY").unwrap_or_else(|_| "minioadmin".to_string());
    let secret_key = std::env::var("MINIO_SECRET_KEY").unwrap_or_else(|_| "minioadmin".to_string());

    println!("  - Endpoint: {}", endpoint);
    println!("  - Bucket: {}", bucket);
    println!("  - Access Key: {}", access_key);
    println!();

    // 创建 MinIO 配置
    let config = OpendalConfig {
        service: OpendalService::S3,
        bucket: Some(bucket),
        endpoint: Some(endpoint),
        access_key: Some(access_key),
        secret_key: Some(secret_key),
        ..Default::default()
    };

    let plugin = OpendalPlugin::from_config(config).await?;

    // 运行测试
    run_s3_tests(plugin, "MinIO").await?;

    Ok(())
}

async fn test_aws_s3() -> Result<(), Box<dyn std::error::Error>> {
    let bucket = std::env::var("AWS_S3_BUCKET")?;
    let region = std::env::var("AWS_S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());
    let access_key = std::env::var("AWS_ACCESS_KEY_ID")?;
    let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY")?;

    println!("  - Bucket: {}", bucket);
    println!("  - Region: {}", region);
    println!();

    // 使用配置创建插件
    let config = OpendalConfig {
        service: OpendalService::S3,
        bucket: Some(bucket),
        region: Some(region),
        access_key: Some(access_key),
        secret_key: Some(secret_key),
        ..Default::default()
    };

    let plugin = OpendalPlugin::from_config(config).await?;

    // 运行测试
    run_s3_tests(plugin, "AWS S3").await?;

    Ok(())
}

async fn run_s3_tests(plugin: OpendalPlugin, service_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let test_prefix = format!("evif-test-{}/", uuid::Uuid::new_v4());

    // 测试 1: 创建文件
    println!("  ✅ 测试 1: 创建文件");
    let test_path = format!("{}test.txt", test_prefix);
    plugin.create(&test_path, 0o644).await?;
    println!("     创建文件: {}", test_path);

    // 测试 2: 写入文件
    println!("  ✅ 测试 2: 写入文件");
    plugin.write(&test_path, b"Hello, S3!".to_vec(), -1, WriteFlags::empty()).await?;
    println!("     写入数据: 'Hello, S3!'");

    // 测试 3: 读取文件
    println!("  ✅ 测试 3: 读取文件");
    let data = plugin.read(&test_path, 0, 0).await?;
    assert_eq!(data, b"Hello, S3!");
    println!("     读取数据: {:?}", String::from_utf8_lossy(&data));

    // 测试 4: 获取文件信息
    println!("  ✅ 测试 4: 获取文件信息");
    let info = plugin.stat(&test_path).await?;
    println!("     大小: {} 字节", info.size);
    println!("     目录: {}", info.is_dir);
    assert_eq!(info.size, 12); // "Hello, S3!" = 12 bytes

    // 测试 5: 创建目录
    println!("  ✅ 测试 5: 创建目录");
    let dir_path = format!("{}test-dir/", test_prefix);
    plugin.mkdir(&dir_path, 0o755).await?;
    println!("     创建目录: {}", dir_path);

    // 测试 6: 列出目录
    println!("  ✅ 测试 6: 列出目录");
    let files = plugin.readdir(&format!("{}{}", test_prefix, "test-dir/")).await?;
    println!("     目录 {} 包含 {} 个文件", dir_path, files.len());

    // 测试 7: 重命名文件
    println!("  ✅ 测试 7: 重命名文件");
    let new_path = format!("{}renamed.txt", test_prefix);
    plugin.rename(&test_path, &new_path).await?;
    println!("     重命名: {} -> {}", test_path, new_path);

    // 验证重命名
    let data = plugin.read(&new_path, 0, 0).await?;
    assert_eq!(data, b"Hello, S3!");

    // 测试 8: 删除文件
    println!("  ✅ 测试 8: 删除文件");
    plugin.remove(&new_path).await?;
    println!("     删除文件: {}", new_path);

    // 测试 9: 递归删除
    println!("  ✅ 测试 9: 递归删除目录");
    plugin.remove_all(&dir_path).await?;
    println!("     递归删除: {}", dir_path);

    // 清理：删除所有测试文件
    println!("  🧹 清理测试文件...");
    match plugin.remove_all(&test_prefix).await {
        Ok(_) => println!("     清理完成"),
        Err(e) => println!("     清理警告: {}", e),
    }

    println!("\n  ✨ {} 所有测试通过!", service_name);
    Ok(())
}
