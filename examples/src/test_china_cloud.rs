// 国内云存储测试程序
//
// 测试阿里云 OSS、腾讯云 COS、华为云 OBS 的 OpenDAL 集成

use evif_core::{EvifPlugin, WriteFlags};
use evif_plugins::{
    AliyunOssConfig, AliyunOssFsPlugin, HuaweiObsConfig, HuaweiObsFsPlugin, TencentCosConfig,
    TencentCosFsPlugin,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 EVIF 2.1 - 国内云存储测试\n");

    // 测试说明
    println!("📋 国内云存储测试说明:");
    println!("  本测试需要以下环境变量之一:");
    println!("  1. 阿里云 OSS:");
    println!("     - ALIYUN_OSS_BUCKET");
    println!("     - ALIYUN_OSS_ACCESS_KEY_ID");
    println!("     - ALIYUN_OSS_ACCESS_KEY_SECRET");
    println!("     - ALIYUN_OSS_ENDPOINT (可选，例如: oss-cn-hangzhou.aliyuncs.com)");
    println!("  2. 腾讯云 COS:");
    println!("     - TENCENT_COS_BUCKET");
    println!("     - TENCENT_COS_SECRET_ID");
    println!("     - TENCENT_COS_SECRET_KEY");
    println!("     - TENCENT_COS_ENDPOINT (可选，例如: https://cos.ap-guangzhou.myqcloud.com)");
    println!("     - TENCENT_COS_REGION (可选，例如: ap-guangzhou)");
    println!("  3. 华为云 OBS:");
    println!("     - HUAWEI_OBS_BUCKET");
    println!("     - HUAWEI_OBS_ACCESS_KEY_ID");
    println!("     - HUAWEI_OBS_SECRET_ACCESS_KEY");
    println!("     - HUAWEI_OBS_ENDPOINT (可选，例如: obs.cn-north-4.myhuaweicloud.com)");
    println!();

    // 检查环境变量并运行相应测试
    if std::env::var("ALIYUN_OSS_BUCKET").is_ok() {
        println!("🔧 使用阿里云 OSS 模式");
        test_aliyun_oss().await?;
    } else if std::env::var("TENCENT_COS_BUCKET").is_ok() {
        println!("🔧 使用腾讯云 COS 模式");
        test_tencent_cos().await?;
    } else if std::env::var("HUAWEI_OBS_BUCKET").is_ok() {
        println!("🔧 使用华为云 OBS 模式");
        test_huawei_obs().await?;
    } else {
        println!("⚠️  未配置云存储环境变量，跳过测试");
        println!("   如需测试，请设置上述环境变量");
        println!("\n💡 提示: 参考配置示例文档");
        println!("   cat CLOUD_STORAGE_CONFIG_EXAMPLES.md");
        return Ok(());
    }

    println!("\n🎉 国内云存储测试完成!");
    Ok(())
}

async fn test_aliyun_oss() -> Result<(), Box<dyn std::error::Error>> {
    let bucket = std::env::var("ALIYUN_OSS_BUCKET")?;
    let access_key_id = std::env::var("ALIYUN_OSS_ACCESS_KEY_ID")?;
    let access_key_secret = std::env::var("ALIYUN_OSS_ACCESS_KEY_SECRET")?;
    let endpoint = std::env::var("ALIYUN_OSS_ENDPOINT").ok();

    println!("  - Bucket: {}", bucket);
    if let Some(ref ep) = endpoint {
        println!("  - Endpoint: {}", ep);
    }
    println!();

    // 创建阿里云 OSS 配置
    let config = AliyunOssConfig {
        bucket,
        access_key_id,
        access_key_secret,
        endpoint,
        root: Some("/evif-test".to_string()),
    };

    let plugin = AliyunOssFsPlugin::from_config(config).await?;

    // 运行测试
    run_china_cloud_tests(plugin, "阿里云 OSS").await?;

    Ok(())
}

async fn test_tencent_cos() -> Result<(), Box<dyn std::error::Error>> {
    let bucket = std::env::var("TENCENT_COS_BUCKET")?;
    let secret_id = std::env::var("TENCENT_COS_SECRET_ID")?;
    let secret_key = std::env::var("TENCENT_COS_SECRET_KEY")?;
    let endpoint = std::env::var("TENCENT_COS_ENDPOINT").ok();
    let region = std::env::var("TENCENT_COS_REGION").ok();

    println!("  - Bucket: {}", bucket);
    if let Some(ref ep) = endpoint {
        println!("  - Endpoint: {}", ep);
    }
    if let Some(ref r) = region {
        println!("  - Region: {}", r);
    }
    println!();

    // 创建腾讯云 COS 配置
    let config = TencentCosConfig {
        bucket,
        secret_id,
        secret_key,
        endpoint,
        region,
        root: Some("/evif-test".to_string()),
    };

    let plugin = TencentCosFsPlugin::from_config(config).await?;

    // 运行测试
    run_china_cloud_tests(plugin, "腾讯云 COS").await?;

    Ok(())
}

async fn test_huawei_obs() -> Result<(), Box<dyn std::error::Error>> {
    let bucket = std::env::var("HUAWEI_OBS_BUCKET")?;
    let access_key_id = std::env::var("HUAWEI_OBS_ACCESS_KEY_ID")?;
    let secret_access_key = std::env::var("HUAWEI_OBS_SECRET_ACCESS_KEY")?;
    let endpoint = std::env::var("HUAWEI_OBS_ENDPOINT").ok();

    println!("  - Bucket: {}", bucket);
    if let Some(ref ep) = endpoint {
        println!("  - Endpoint: {}", ep);
    }
    println!();

    // 创建华为云 OBS 配置
    let config = HuaweiObsConfig {
        bucket,
        access_key_id,
        secret_access_key,
        endpoint,
        root: Some("/evif-test".to_string()),
    };

    let plugin = HuaweiObsFsPlugin::from_config(config).await?;

    // 运行测试
    run_china_cloud_tests(plugin, "华为云 OBS").await?;

    Ok(())
}

async fn run_china_cloud_tests<P: EvifPlugin>(
    plugin: P,
    service_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let test_prefix = format!("evif-test-{}/", uuid::Uuid::new_v4());

    // 测试 1: 创建文件
    println!("  ✅ 测试 1: 创建文件");
    let test_path = format!("{}test.txt", test_prefix);
    plugin.create(&test_path, 0o644).await?;
    println!("     创建文件: {}", test_path);

    // 测试 2: 写入文件
    println!("  ✅ 测试 2: 写入文件");
    plugin
        .write(
            &test_path,
            b"Hello, China Cloud Storage!".to_vec(),
            -1,
            WriteFlags::empty(),
        )
        .await?;
    println!("     写入数据: 'Hello, 中国云存储!'");

    // 测试 3: 读取文件
    println!("  ✅ 测试 3: 读取文件");
    let data = plugin.read(&test_path, 0, 0).await?;
    assert_eq!(data, b"Hello, China Cloud Storage!");
    println!("     读取数据: {:?}", String::from_utf8_lossy(&data));

    // 测试 4: 获取文件信息
    println!("  ✅ 测试 4: 获取文件信息");
    let info = plugin.stat(&test_path).await?;
    println!("     大小: {} 字节", info.size);
    println!("     目录: {}", info.is_dir);
    assert_eq!(info.size, 23); // "Hello, 中国云存储!" = 23 bytes (UTF-8)

    // 测试 5: 创建目录
    println!("  ✅ 测试 5: 创建目录");
    let dir_path = format!("{}test-dir/", test_prefix);
    plugin.mkdir(&dir_path, 0o755).await?;
    println!("     创建目录: {}", dir_path);

    // 测试 6: 列出目录
    println!("  ✅ 测试 6: 列出目录");
    let files = plugin
        .readdir(&format!("{}{}", test_prefix, "test-dir/"))
        .await?;
    println!("     目录 {} 包含 {} 个文件", dir_path, files.len());

    // 测试 7: 重命名文件
    println!("  ✅ 测试 7: 重命名文件");
    let new_path = format!("{}renamed.txt", test_prefix);
    plugin.rename(&test_path, &new_path).await?;
    println!("     重命名: {} -> {}", test_path, new_path);

    // 验证重命名
    let data = plugin.read(&new_path, 0, 0).await?;
    assert_eq!(data, b"Hello, China Cloud Storage!");

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
