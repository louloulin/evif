# EVIF 生产故障应急手册

> 创建时间：2026-04-03
> 适用版本：EVIF 0.1.0+
> 目的：为 EVIF 生产运行中的常见故障提供分类响应指南

---

## 一、故障响应流程

```
发现异常
  │
  ▼
判断影响范围（见第二节）
  │
  ├── 影响用户 → P1 → 立即响应
  ├── 部分功能异常 → P2 → 4小时内响应
  └── 非功能异常（监控/日志） → P3 → 下个工作日
  │
  ▼
执行应急操作（见第三节）
  │
  ▼
恢复服务
  │
  ▼
记录事件（见第四节）
```

---

## 二、常见故障分类

### 2.1 服务无法启动

**症状**：`docker compose up` 后容器立即退出，或 `/api/v1/health` 无响应

**诊断**：
```bash
# 查看容器退出原因
docker compose logs evif-rest

# 检查环境变量校验错误（生产模式）
# 搜索日志中的 "EVIF_REST_PRODUCTION_MODE requires"
```

**常见原因与修复**：

| 原因 | 修复 |
|------|------|
| 缺少 `EVIF_REST_*_STATE_PATH` | 添加三个持久化路径环境变量 |
| SQLite 路径目录不存在 | 确保 `/data` 卷已挂载 |
| 端口 8081 被占用 | 更改 `EVIF_REST_PORT` 或杀掉占用进程 |
| 镜像损坏 | `docker pull evif/evif-rest:latest` 重新拉取 |

---

### 2.2 健康检查失败

**症状**：`curl http://localhost:8081/api/v1/health` 返回非 200 或 `status != healthy`

**诊断**：
```bash
# 检查健康端点详情
curl -v http://localhost:8081/api/v1/health

# 查看启动日志
docker compose logs evif-rest --tail=50
```

**常见原因与修复**：

| 原因 | 修复 |
|------|------|
| 插件挂载失败（panic） | 检查 `EVIF_CONFIG` 文件是否合法 JSON/YAML/TOML |
| 记忆后端启动失败 | 确认 SQLite 文件路径可写 |
| 插件加载回退到 MemFs | 警告：非预期行为，检查插件路径 |

---

### 2.3 认证失败

**症状**：API 返回 `401` / `403`，审计日志有 denied 记录

**诊断**：
```bash
# 查看审计日志
docker exec evif-rest cat /var/log/evif/audit.log

# 检查 API Key 是否正确传递
curl -H "X-API-Key: your-key" http://localhost:8081/api/v1/health
```

**常见原因与修复**：

| 原因 | 修复 |
|------|------|
| API Key 不正确 | 重新确认 `EVIF_REST_WRITE_API_KEYS` / `ADMIN_API_KEYS` |
| Key 格式错误（多余空格） | 确认 Key 中无前后空格，多 Key 用逗号分隔 |
| 加密端点用错 Key | `/api/v1/encryption/enable` 需要 `admin-key` |

---

### 2.4 状态数据丢失

**症状**：租户列表为空，同步版本重置，加密状态重置

**诊断**：
```bash
# 检查持久化文件
docker exec evif-rest ls -la /data/*.json

# 验证 JSON 格式
docker exec evif-rest cat /data/tenant-state.json | python3 -m json.tool
```

**常见原因与修复**：

| 原因 | 修复 |
|------|------|
| 卷未持久化 | 重启时使用 `docker compose up`（不要 `--volumes`） |
| JSON 文件被覆盖 | 检查是否有外部配置脚本误写 |
| 内存态启动（生产模式未设路径） | 添加三个 `*_STATE_PATH` 环境变量 |

---

### 2.5 高延迟 / 无响应

**症状**：API 响应时间 > 5s，或完全超时

**诊断**：
```bash
# 检查资源使用
docker stats evif-rest --no-stream

# 检查内存后端
curl http://localhost:8081/api/v1/metrics/traffic

# 查看是否有锁等待
docker compose logs evif-rest | grep -i "timeout\|blocked\|lock"
```

**常见原因与修复**：

| 原因 | 修复 |
|------|------|
| 资源限制过严 | 增大 `deploy.resources.limits.memory`（当前 2G） |
| SQLite 锁竞争 | 检查并发写场景，考虑连接池 |
| 插件操作阻塞 | 确认 `/api/v1/metrics/operations` 中的 `read/write/list` 延迟 |

---

## 三、应急操作快速命令

```bash
# ─── 紧急恢复 ─────────────────────────────────────────────────────────
# 1. 强制重启（保留卷）
docker compose -f docker-compose.yml -f docker-compose.prod.yml restart evif-rest

# 2. 如果重启无效，完全重建（仍保留命名卷）
docker compose -f docker-compose.yml -f docker-compose.prod.yml down
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d

# 3. 回滚到旧版本（见回滚手册 docs/production-rollback-guide.md）

# ─── 信息收集（用于事后分析） ──────────────────────────────────────
# 4. 导出日志
docker compose logs evif-rest > /tmp/evif-$(date +%Y%m%d%H%M%S).log

# 5. 导出指标
curl -s http://localhost:8081/api/v1/metrics/traffic > /tmp/evif-traffic-$(date +%Y%m%d%H%M%S).json
curl -s http://localhost:8081/api/v1/metrics/status > /tmp/evif-status-$(date +%Y%m%d%H%M%S).json
curl -s http://localhost:8081/metrics > /tmp/evif-prometheus-$(date +%Y%m%d%H%M%S).txt

# 6. 导出审计日志
docker exec evif-rest cat /var/log/evif/audit.log > /tmp/evif-audit-$(date +%Y%m%d%H%M%S).log
```

---

## 四、事件记录模板

每次故障解决后填写：

```markdown
## 事件记录

- **事件编号**：
- **发现时间**：
- **恢复时间**：
- **影响范围**：
- **根本原因**：
- **应急措施**：
- **预防措施**：
- **负责人**：
```

---

## 五、监控告警建议

建议为以下指标配置告警：

| 指标 | 告警阈值 | 建议动作 |
|------|----------|----------|
| `evif_total_errors` 增量 > 10/min | 错误突增 | 检查日志 |
| `/api/v1/health` 响应时间 > 3s | 健康检查慢 | 检查资源 |
| 容器重启次数 > 2/hour | 频繁重启 | 立即响应 |
| 磁盘使用 > 80% | 存储不足 | 清理或扩容 |
