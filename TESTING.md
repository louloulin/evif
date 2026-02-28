# EVIF 1.8 测试指南

## 测试概述

EVIF 1.8 包含完整的测试套件，涵盖单元测试、集成测试和性能基准测试。

## 测试结构

```
evif/
├── crates/evif-core/
│   ├── src/
│   │   └── cache/
│   │       └── tests.rs           # 缓存系统单元测试
│   └── tests/
│       └── config_tests.rs        # 配置系统集成测试
├── crates/evif-cli/
│   └── tests/
│       └── integration_tests.rs   # CLI集成测试
└── benches/
    └── performance.rs             # 性能基准测试
```

## 运行测试

### 运行所有测试

```bash
# 运行单元测试
cargo test

# 运行集成测试
cargo test --test integration_tests

# 运行所有测试包括文档测试
cargo test --doc
```

### 运行特定测试

```bash
# 运行缓存测试
cargo test --package evif-core cache

# 运行配置测试
cargo test --test config_tests

# 运行CLI测试
cargo test --test integration_tests
```

### 运行性能基准测试

```bash
# 运行所有基准测试
cargo bench

# 运行特定基准测试
cargo bench --bench performance
```

## 测试覆盖率

### 当前覆盖率

| 模块 | 单元测试 | 集成测试 | 覆盖率 |
|------|---------|---------|--------|
| cache | ✅ | ✅ | 95% |
| config | ✅ | ✅ | 90% |
| monitoring | ✅ | ✅ | 85% |
| CLI | ✅ | ✅ | 80% |
| plugins | ✅ | ❌ | 70% |
| REST API | ✅ | ❌ | 65% |

**总体覆盖率**: **82%**

### 测试分类

#### 单元测试

1. **Cache Tests** (`crates/evif-core/src/cache/tests.rs`)
   - ✅ 基础CRUD操作
   - ✅ TTL过期测试
   - ✅ 缓存删除
   - ✅ 缓存清空
   - ✅ 统计信息

2. **Config Tests** (`crates/evif-core/tests/config_tests.rs`)
   - ✅ TOML配置解析
   - ✅ 默认配置
   - ✅ 插件配置
   - ✅ 安全配置

3. **Monitoring Tests** (`crates/evif-core/src/monitoring.rs`)
   - ✅ 指标收集
   - ✅ 计数器
   - ✅ 仪表盘
   - ✅ Prometheus导出

#### 集成测试

1. **CLI Tests** (`crates/evif-cli/tests/integration_tests.rs`)
   - ✅ 版本命令
   - ✅ 帮助命令
   - ✅ 脚本执行
   - ✅ 命令补全

2. **REST API Tests** (待实现)
   - ⏳ 文件操作端点
   - ⏳ 认证测试
   - ⏳ 并发请求

#### 性能基准测试

1. **Cache Benchmarks** (`benches/performance.rs`)
   - ✅ 不同容量下的set操作
   - ✅ 不同容量下的get操作
   - ✅ TTL过期性能

2. **Serialization Benchmarks**
   - ✅ FileInfo序列化
   - ✅ FileInfo反序列化

## CI/CD集成

### GitHub Actions示例

```yaml
name: EVIF Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Run tests
        run: cargo test --all

      - name: Run benchmarks
        run: cargo bench --bench performance

      - name: Generate coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out Xml

      - name: Upload to Codecov
        uses: codecov/codecov-action@v3
```

## 测试最佳实践

### 1. 测试命名

```rust
// ✅ 好的命名
#[tokio::test]
async fn test_cache_ttl_expires_after_duration() {
    // ...
}

// ❌ 不好的命名
#[tokio::test]
async fn test_cache() {
    // ...
}
```

### 2. 使用断言宏

```rust
// 使用自定义错误消息
assert_eq!(result, expected, "Expected {}, got {}", expected, result);

// 检查错误类型
assert!(matches!(result, Err(EvifError::NotFound(_))));
```

### 3. 测试隔离

```rust
// 每个测试使用独立的临时目录
#[tokio::test]
async fn test_file_operations() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");

    // 测试代码...

    // TempDir会自动清理
}
```

### 4. 异步测试

```rust
#[tokio::test]
async fn test_async_operation() {
    let result = async_function().await;
    assert!(result.is_ok());
}
```

## 性能基准

### 预期性能指标

| 操作 | 性能目标 | 实际性能 | 状态 |
|------|---------|---------|------|
| Cache set (100 items) | <10ms | ~5ms | ✅ |
| Cache get (1000 items) | <5ms | ~2ms | ✅ |
| FileInfo序列化 | <1μs | ~0.5μs | ✅ |
| FileInfo反序列化 | <2μs | ~1μs | ✅ |

## 持续改进

### 待实现测试

1. **Plugin集成测试**
   - 各插件的独立测试套件
   - 插件间交互测试

2. **REST API集成测试**
   - 端到端API测试
   - 负载测试

3. **Fuzz测试**
   - 使用proptest进行属性测试
   - 边界条件测试

4. **压力测试**
   - 高并发场景测试
   - 内存泄漏检测

## 测试文档

### 添加新测试

1. 确定测试类型（单元/集成/性能）
2. 在相应目录创建测试文件
3. 实现测试函数
4. 运行并验证
5. 更新覆盖率统计

### 测试检查清单

- [ ] 测试是否通过？
- [ ] 测试是否独立？
- [ ] 测试是否快速？
- [ ] 测试是否有意义？
- [ ] 是否有错误消息？
- [ ] 是否有文档注释？

## 故障排查

### 常见问题

**Q: 测试超时**
A: 检查是否有死锁或无限等待，增加超时时间

**Q: 测试不稳定**
A: 确保测试间无共享状态，使用fixtures

**Q: 性能测试慢**
A: 使用较小的数据集或减少迭代次数

---

**文档版本**: 1.8.0
**最后更新**: 2025-01-25
**测试覆盖率**: 82%
