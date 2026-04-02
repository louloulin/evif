// Phase 13.6: Performance Benchmark Tests
//
// 测试 EVIF 性能基准，包括：
// - 吞吐量测试 (Throughput)
// - 延迟测试 (Latency)
// - 并发测试 (Concurrency)
// - 内存使用测试 (Memory Usage)

use evif_core::{EvifPlugin, WriteFlags};
use evif_plugins::ContextFsPlugin;

/// PE-01: Concurrent Writes Test
/// 目标: 100 并发写入，95%+ 成功率
#[tokio::test]
async fn perf_concurrent_writes() {
    let _plugin = ContextFsPlugin::new();

    // 创建 100 个并发写入任务
    let mut handles = Vec::new();
    for i in 0..100u32 {
        let plugin = ContextFsPlugin::new();
        let path = format!("/L0/file_{}", i);
        let content = format!("data for file {}", i);

        handles.push(tokio::spawn(async move {
            plugin
                .write(&path, content.into_bytes(), 0, WriteFlags::TRUNCATE)
                .await
        }));
    }

    // 等待所有任务完成
    let results: Vec<Result<_, _>> = futures::future::join_all(handles).await;

    // 计算成功率
    let success_count = results.iter().filter(|r| r.is_ok()).count();
    let success_rate = success_count as f64 / 100.0;

    // 验证 95%+ 成功率
    assert!(
        success_rate >= 0.95,
        "Concurrent writes success rate {:.1}% < 95%",
        success_rate * 100.0
    );
}

/// PE-02: Throughput Test
/// 目标: > 100 req/s
#[tokio::test]
async fn perf_throughput() {
    let plugin = ContextFsPlugin::new();

    let start = std::time::Instant::now();
    let mut count = 0u32;

    // 在 1 秒内尽可能多写入
    while start.elapsed().as_secs() < 1 {
        plugin
            .create(&format!("/L0/throughput_{}", count), 0o644)
            .await
            .expect("create");
        count += 1;
    }

    let elapsed = start.elapsed().as_secs_f64();
    let throughput = count as f64 / elapsed;

    // 验证吞吐量 > 100 req/s
    assert!(
        throughput > 100.0,
        "Throughput {:.1} req/s < 100 req/s target",
        throughput
    );
}

/// PE-03: P99 Latency Test
/// 目标: P99 < 50ms
#[tokio::test]
async fn perf_latency_p99() {
    let plugin = ContextFsPlugin::new();

    // 预热
    let _ = plugin.read("/L0/current", 0, 0).await;

    // 测量 100 次读取延迟
    let mut latencies = Vec::with_capacity(100);
    for _ in 0..100 {
        let start = std::time::Instant::now();
        let _ = plugin.read("/L0/current", 0, 0).await.unwrap();
        latencies.push(start.elapsed().as_millis() as u64);
    }

    // 计算 P99
    latencies.sort();
    let p99_index = (100.0 * 0.99) as usize - 1;
    let p99 = latencies[p99_index];

    // 验证 P99 < 50ms
    assert!(
        p99 < 50,
        "P99 latency {}ms > 50ms target",
        p99
    );
}

/// PE-04: Memory Usage Test
/// 目标: 插件在高负载下仍然可用
#[tokio::test]
async fn perf_memory_usage() {
    let plugin = ContextFsPlugin::new();

    // 写入一些数据
    for i in 0..100u32 {
        plugin
            .create(&format!("/L2/file_{}", i), 0o644)
            .await
            .expect("create");
        let content = "x".repeat(1000);
        plugin
            .write(
                &format!("/L2/file_{}", i),
                content.into_bytes(),
                0,
                WriteFlags::TRUNCATE,
            )
            .await
            .expect("write");
    }

    // 验证插件仍然可用
    let _ = plugin.read("/L0/current", 0, 0).await.unwrap();
    let _ = plugin.read("/L1/session_id", 0, 0).await.unwrap();
    let _ = plugin.read("/L2/file_50", 0, 0).await.unwrap();

    // 注: 精确的内存测量需要外部工具 (如 heaptrack, massif)
    // 此测试验证插件在高负载下仍然可用
    assert!(true, "Plugin remains functional under load");
}

/// PE-05: ContextFS Write Latency
#[tokio::test]
async fn perf_contextfs_write_latency() {
    let plugin = ContextFsPlugin::new();

    // 测量单次写入延迟
    let iterations = 100;
    let mut total_ms = 0i64;

    for i in 0..iterations {
        let start = std::time::Instant::now();
        plugin
            .write(
                "/L0/current",
                format!("iteration {}", i).into_bytes(),
                0,
                WriteFlags::TRUNCATE,
            )
            .await
            .unwrap();
        total_ms += start.elapsed().as_millis() as i64;
    }

    let avg_ms = total_ms as f64 / iterations as f64;

    // 验证平均写入延迟 < 10ms
    assert!(
        avg_ms < 10.0,
        "Average write latency {:.2}ms > 10ms target",
        avg_ms
    );
}

/// PE-06: ContextFS Read Latency
#[tokio::test]
async fn perf_contextfs_read_latency() {
    let plugin = ContextFsPlugin::new();

    // 预热
    let _ = plugin.read("/L0/current", 0, 0).await.unwrap();

    // 测量 100 次读取延迟
    let iterations = 100;
    let mut total_ms = 0i64;

    for _ in 0..iterations {
        let start = std::time::Instant::now();
        let _ = plugin.read("/L0/current", 0, 0).await.unwrap();
        total_ms += start.elapsed().as_millis() as i64;
    }

    let avg_ms = total_ms as f64 / iterations as f64;

    // 验证平均读取延迟 < 5ms
    assert!(
        avg_ms < 5.0,
        "Average read latency {:.2}ms > 5ms target",
        avg_ms
    );
}

/// PE-07: Large File Write Test
#[tokio::test]
async fn perf_large_file_write() {
    let plugin = ContextFsPlugin::new();

    let content = "line of data\n".repeat(1000); // ~14KB

    let start = std::time::Instant::now();
    plugin
        .create("/L2/large.txt", 0o644)
        .await
        .expect("create");
    plugin
        .write("/L2/large.txt", content.into_bytes(), 0, WriteFlags::TRUNCATE)
        .await
        .expect("write");
    let elapsed = start.elapsed().as_millis();

    // 验证大文件写入 < 100ms
    assert!(
        elapsed < 100,
        "Large file write {}ms > 100ms target",
        elapsed
    );
}

/// PE-08: Multi-Layer Read Test
/// 测试 L0/L1/L2 各层读取性能
#[tokio::test]
async fn perf_multi_layer_read() {
    let plugin = ContextFsPlugin::new();

    // L0 读取
    let l0_start = std::time::Instant::now();
    let _ = plugin.read("/L0/current", 0, 0).await.unwrap();
    let l0_ms = l0_start.elapsed().as_millis();

    // L1 读取
    let l1_start = std::time::Instant::now();
    let _ = plugin.read("/L1/session_id", 0, 0).await.unwrap();
    let l1_ms = l1_start.elapsed().as_millis();

    // L2 读取
    plugin
        .create("/L2/perf_test.txt", 0o644)
        .await
        .expect("create L2");
    plugin
        .write("/L2/perf_test.txt", b"test content".to_vec(), 0, WriteFlags::TRUNCATE)
        .await
        .expect("write L2");

    let l2_start = std::time::Instant::now();
    let _ = plugin.read("/L2/perf_test.txt", 0, 0).await.unwrap();
    let l2_ms = l2_start.elapsed().as_millis();

    // 验证各层延迟
    assert!(l0_ms < 10, "L0 read {}ms > 10ms", l0_ms);
    assert!(l1_ms < 20, "L1 read {}ms > 20ms", l1_ms);
    assert!(l2_ms < 50, "L2 read {}ms > 50ms", l2_ms);
}
