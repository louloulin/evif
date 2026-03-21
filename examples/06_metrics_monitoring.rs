//! EVIF Prometheus监控指标示例
//!
//! 演示如何在不依赖 graph 引擎的前提下使用 evif-metrics 收集和导出监控指标

use evif_metrics::PrometheusMetricsRegistry;
use tokio::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== EVIF Prometheus监控系统示例 ===\n");

    // 1. 创建指标注册表
    println!("1. 创建Prometheus指标注册表...");
    let registry = PrometheusMetricsRegistry::new()?;
    println!("✓ 注册表创建成功\n");

    // 2. 注册Counter指标（单调递增）
    println!("2. 注册Counter指标...");
    registry.register_counter(
        "evif_operations_total",
        "Total number of EVIF operations",
        &["operation_type"]
    ).await?;

    registry.register_counter(
        "evif_nodes_created_total",
        "Total number of nodes created",
        &["node_type"]
    ).await?;

    println!("✓ Counter指标注册成功\n");

    // 3. 注册Gauge指标（可增减）
    println!("3. 注册Gauge指标...");
    registry.register_gauge(
        "evif_active_connections",
        "Number of active connections",
        &[]
    ).await?;

    registry.register_gauge(
        "evif_memory_usage_bytes",
        "Current memory usage in bytes",
        &["component"]
    ).await?;

    println!("✓ Gauge指标注册成功\n");

    // 4. 注册Histogram指标（分布统计）
    println!("4. 注册Histogram指标...");
    registry.register_histogram(
        "evif_operation_duration_seconds",
        "Operation execution duration",
        &["operation"],
        None  // 使用默认bucket
    ).await?;

    println!("✓ Histogram指标注册成功\n");

    // 5. 模拟操作并记录指标
    println!("5. 模拟操作并记录指标...\n");

    // 模拟核心文件操作并记录指标
    println!("a) 记录文件创建操作...");
    for i in 0..5 {
        let start = Instant::now();
        tokio::time::sleep(Duration::from_millis(5)).await;
        let duration = start.elapsed();

        registry.counter_inc("evif_nodes_created_total", &["file"]).await?;

        registry.histogram_observe(
            "evif_operation_duration_seconds",
            &["create_file"],
            duration.as_secs_f64()
        ).await?;

        println!("  - 创建文件 {} ({:.2}ms)", i + 1, duration.as_millis());
    }

    registry.counter_inc_by("evif_operations_total", &["create_file"], 5.0).await?;
    println!();

    // 模拟连接数变化
    println!("b) 模拟连接数变化...");
    registry.gauge_set("evif_active_connections", &[], 10.0).await?;
    println!("  - 活跃连接: 10");

    tokio::time::sleep(Duration::from_millis(100)).await;
    registry.gauge_inc("evif_active_connections", &[]).await?;
    println!("  - 活跃连接: 11 (+1)");

    tokio::time::sleep(Duration::from_millis(100)).await;
    registry.gauge_inc("evif_active_connections", &[]).await?;
    println!("  - 活跃连接: 12 (+1)");

    tokio::time::sleep(Duration::from_millis(100)).await;
    registry.gauge_dec("evif_active_connections", &[]).await?;
    println!("  - 活跃连接: 11 (-1)\n");

    // 记录内存使用
    println!("c) 记录内存使用...");
    registry.gauge_set("evif_memory_usage_bytes", &["graph"], 1024000.0).await?;
    registry.gauge_set("evif_memory_usage_bytes", &["storage"], 2048000.0).await?;
    registry.gauge_set("evif_memory_usage_bytes", &["cache"], 512000.0).await?;
    println!("  - Graph: 1024000 bytes");
    println!("  - Storage: 2048000 bytes");
    println!("  - Cache: 512000 bytes\n");

    // 6. 模拟读取操作
    println!("d) 执行读取操作...");
    for i in 0..3 {
        let start = Instant::now();
        tokio::time::sleep(Duration::from_millis(3)).await;
        let duration = start.elapsed();

        registry.counter_inc("evif_operations_total", &["read"]).await?;
        registry.histogram_observe(
            "evif_operation_duration_seconds",
            &["read"],
            duration.as_secs_f64()
        ).await?;

        println!("  - 读取 {} ({:.2}ms)", i + 1, duration.as_millis());
    }
    println!();

    // 7. 导出Prometheus格式指标
    println!("6. 导出Prometheus格式指标...");
    let metrics = registry.export()?;
    println!("✓ 指标导出成功\n");
    println!("--- Prometheus指标输出 ---");
    println!("{}", metrics);
    println!("--- 输出结束 ---\n");

    // 8. 统计摘要
    println!("7. 指标统计摘要:");
    println!("  - Counter指标: 2个");
    println!("  - Gauge指标: 2个");
    println!("  - Histogram指标: 1个");
    println!("  - 总操作数: 8次 (5次创建文件 + 3次读取)");
    println!("  - 活跃连接: 11个");
    println!();

    println!("=== 示例完成 ===");
    Ok(())
}
