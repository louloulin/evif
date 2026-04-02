# 决策记录

- 2026-04-02：按照“最佳最小方式”收口 Phase 17，不扩展额外接口，只验证并保留已经接入的多租户、加密存储、增量同步、GraphQL API 四项能力。
- 2026-04-02：真实验证采用 `cargo test -p evif-rest --test multi_tenant --test encryption_at_rest --test incremental_sync --test graphql_api -- --nocapture`，以 Phase 17 对应 19 个集成测试作为完成标准。
- 2026-04-02：补充 `GET /api/v1/tenants/:id` 集成测试，Phase 17 实际验证标准更新为 20 个集成测试全部通过。
- 2026-04-02：同步修正 mem14.md 中前文残留的“需实现 / 待实现 / 规划中 / 未来规划”旧表述，统一为与当前实现状态一致的历史描述。
- 2026-04-02：全库生产级分析结论写入 mem15.md。功能实现进度按 mem14.md 严格口径评估为 98.3%，生产成熟度评估为 2.9/5（约 58%），后续优先级为“先过严格门禁，再补观测，再补状态可靠性与部署闭环”。
- 2026-04-02：按 mem15.md 的最佳最小实现优先选择 Phase B 而非 Phase A，因为“让 TrafficStats 成为可信信号”可以形成独立闭环，风险和改动面都明显小于全仓 clippy 清零。
- 2026-04-02：新增 TrafficMetricsMiddleware，将真实请求统计接入 TrafficStats；新增 metrics_traffic.rs 集成测试，真实验证命令为 `cargo test -p evif-rest --test metrics_traffic metrics_traffic_counts_real_requests -- --nocapture` 和 `cargo test -p evif-rest --lib --tests --quiet`。
- 2026-04-02：Phase B 第二个最小子项选择“标准 `/metrics` Prometheus 文本接口”而不是 tracing 初始化，因为它可在现有接线基础上形成独立可抓取闭环，改动更小且验证信号更直接。
- 2026-04-02：先新增 `metrics_prometheus_endpoint_exposes_standard_text_format` 集成测试并确认其因 `Content-Type` 不符合 `text/plain; version=0.0.4; charset=utf-8` 而失败，再将 `/metrics` 响应头修正为 Prometheus 标准文本格式。
- 2026-04-02：Phase B 第三个最小子项选择“request id / correlation id 请求标识接线”而不是 tracing_subscriber 初始化，因为它能独立形成请求链路可追踪闭环，且可以用真实 HTTP 集成测试直接验证响应头行为。
- 2026-04-02：先新增 `request_identity.rs` 集成测试并确认当前服务既不会生成 `x-request-id`，也不会透传客户端提供的 `x-request-id / x-correlation-id`，再补充 RequestIdentityMiddleware、全路由接线以及 CORS 允许/暴露头。
- 2026-04-02：Phase B 第四个最小子项选择“关键路由 success/error/latency 指标”而不是 tracing_subscriber 初始化，因为当前 `/metrics` 只能看请求量级，仍缺少问题定位最需要的成功率、错误率和时延信号；该缺口可以用真实 HTTP 抓取直接验证。
- 2026-04-02：先新增 `metrics_prometheus_endpoint_exposes_success_error_and_latency_by_operation` 集成测试并确认 `/metrics` 尚未导出按 operation 维度的成功数、错误数和时延指标，再将这些指标真实接到 TrafficMetricsMiddleware 和 Prometheus 文本导出，同时修正 `/api/v1/metrics/operations` 的错误计数口径。
