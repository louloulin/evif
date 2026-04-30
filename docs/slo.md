# EVIF SLO 定义与 Error Budget

> 创建时间：2026-04-08
> 适用版本：EVIF 0.1.0+
> 目的：定义 EVIF 生产运行的质量目标，为 on-call 提供可测量的基准

---

## 一、SLO 架构原则

EVIF 作为 **AI Agent 文件系统**，其 SLO 分为两层：

- **基础设施层**：REST API 服务质量（evif-rest）
- **数据面层**：文件系统操作正确性（ContextFS/SkillFS/PipeFS）

---

## 二、API SLO（evif-rest）

### 2.1 可用性目标

| SLO 名称 | 目标值 | 测量方式 |
|----------|--------|----------|
| **API 可用率** | ≥ 99.5% / 月 | `GET /api/v1/health` 成功率 |
| **启动成功率** | ≥ 99.9% / 启动 | 服务启动后 5s 内 /health 返回 200 |
| **P0 端点可用率** | 100% / 月 | health、files、directories 端点 |

**计算公式**：
```
可用率 = (成功请求数 / 总请求数) × 100%
Error Budget = (1 - 可用率目标) × 月总分钟数
```

99.5% 可用率 = **每月 3.6 小时 Error Budget**

### 2.2 性能目标

| SLO 名称 | 目标值 | 测量方式 |
|----------|--------|----------|
| **吞吐量基线** | ≥ 1 req/s | `evif-bench` PE-01 测试（3s 内至少完成 1 请求）|
| **P50 延迟** | ≤ 20ms | `evif-bench` PE-04 健康检查 P50 |
| **P99 延迟** | ≤ 200ms | `evif-bench` PE-02 |
| **并发写入稳定性** | 100% 完成 | `evif-bench` PE-03（100 并发无 5xx）|

**实测基线**（2026-04-08，MacBook Pro 开发环境，多次测量）

| 指标 | 实测值 | 目标值 | 状态 |
|------|--------|--------|------|
| 吞吐量 | **~7000 req/s**（6600-7700 范围） | ≥ 1 req/s | ✅ GREEN |
| P50 延迟 | 0ms | ≤ 20ms | ✅ GREEN |
| P99 延迟 | 1ms | ≤ 200ms | ✅ GREEN |
| 并发写入 | 100/100 完成 | 100% | ✅ GREEN |

> ⚠️ **注意**：开发环境实测值不能直接用于生产 SLO。生产 SLO 应在目标部署环境（如 Linux 服务器）上重新测量。

### 2.3 错误率目标

| SLO 名称 | 目标值 | 测量方式 |
|----------|--------|----------|
| **5xx 错误率** | ≤ 0.1% | 服务器错误 / 总请求 |
| **API 失败率（功能）** | ≤ 1% | 业务错误响应 / 总请求 |

---

## 三、文件系统 SLO

### 3.1 操作正确性

| SLO 名称 | 目标值 | 测量方式 |
|----------|--------|----------|
| **文件操作成功率** | ≥ 99.5% | 读/写/删除操作成功比例 |
| **挂载表路由正确率** | 100% | 路径解析到正确插件 |
| **Handle 泄漏率** | 0 / 月 | 未关闭 handle 数量 |

### 3.2 数据一致性

| SLO 名称 | 目标值 | 说明 |
|----------|--------|------|
| **加密状态一致性** | 100% | 加密文件解密后与原文一致 |
| **增量同步数据完整性** | 100% | delta 应用后文件内容一致 |
| **多租户隔离性** | 100% | 租户 A 无法访问租户 B 数据 |

---

## 四、安全 SLO

| SLO 名称 | 目标值 | 说明 |
|----------|--------|------|
| **依赖漏洞数量** | 0 高危 | `cargo audit` 无 RUSTSEC 高危漏洞 |
| **认证覆盖率** | 100% | 受保护端点全部需要认证 |
| **审计日志完整性** | ≥ 99.9% | 写操作审计日志记录比例 |

> **当前状态**：Phase H（依赖安全）已大幅改善。通过 `cargo update` 和 wasmtime 升级修复了所有 aws-lc-sys 高危漏洞和 wasmtime CRITICAL 沙箱逃逸漏洞。剩余漏洞均为第三方上游未提供修复（protobuf v2 受限于 prometheus，rsa Marvin Attack 无官方修复）。

---

## 五、On-Call Runbook

### 5.1 关键监控信号

```
告警阈值（建议 Prometheus 规则）：
  - api_requests_total{status=~"5.."} / api_requests_total > 0.01  → P2
  - api_latency_p99_seconds > 0.2  → P2
  - evif_rest_up == 0  → P1
```

### 5.2 快速检查清单

**服务健康检查**：
```bash
# 1. API 是否响应
curl http://localhost:8081/api/v1/health

# 2. 吞吐量基线
cargo test -p evif-bench performance_throughput -- --nocapture

# 3. clippy + test 门禁
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

**状态文件检查**：
```bash
# 租户状态
cat /var/lib/evif/tenant_state.json  # 或 $EVIF_REST_TENANT_STATE_PATH

# 加密状态
cat /var/lib/evif/encryption_state.json  # 或 $EVIF_REST_ENCRYPTION_STATE_PATH

# 同步状态
cat /var/lib/evif/sync_state.json  # 或 $EVIF_REST_SYNC_STATE_PATH
```

---

## 六、Error Budget 策略

| Error Budget 剩余 | 策略 |
|-------------------|------|
| ≥ 50% | 正常推进功能开发 |
| 25-50% | 暂停非紧急功能，专注可靠性工作 |
| < 25% | 冻结功能发布，优先修复稳定性问题 |
| < 5% | 启动事后复盘（Postmortem），制定改进计划 |

---

## 七、SLO 测量频率

| SLO 类型 | 测量频率 | 报告频率 |
|----------|----------|----------|
| API 可用率 | 实时（Prometheus） | 每周 |
| 性能基线 | 每发布版本 | 每季度 |
| 安全 SLO | 每 `cargo audit` 运行 | 每季度 |

---

## 八、当前 SLO 状态（2026-04-08）

| SLO 类别 | 状态 | 说明 |
|----------|------|------|
| API 可用率 | ⚠️ 无生产测量 | 需要生产环境 Prometheus |
| 吞吐量基线 | ✅ GREEN | ~7000 req/s（6600-7700 范围，开发环境）|
| P99 延迟 | ✅ GREEN | 1ms（开发环境）|
| 并发稳定性 | ✅ GREEN | 100/100 完成 |
| 依赖安全 | 🟡 YELLOW | aws-lc-sys/wasmtime 已修复，protobuf/rsa 待上游 |
| 认证覆盖 | ⚠️ 部分覆盖 | 需确认所有 P0 端点 |
| 审计日志 | ⚠️ 未测量 | 需接入日志系统 |
