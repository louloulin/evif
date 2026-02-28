# EVIF 监控指标说明（Phase 11.1）

本文档说明 evif-rest 提供的监控与指标接口、字段含义及典型用法，供运维与前端监控页对接。

---

## 一、端点概览

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/metrics/traffic` | GET | 流量统计（请求数、读写字节、错误数等） |
| `/api/v1/metrics/operations` | GET | 按操作类型聚合（read/write/list/other） |
| `/api/v1/metrics/status` | GET | 系统状态（健康、运行时长、挂载数、流量与操作汇总） |
| `/api/v1/metrics/reset` | POST | 重置所有计数（用于测试或清零周期统计） |

基础 URL 与 EVIF REST 一致（如 `http://localhost:8080`）。

---

## 二、流量统计 GET /api/v1/metrics/traffic

### 响应字段

| 字段 | 类型 | 说明 |
|------|------|------|
| total_requests | number | 总请求次数 |
| total_bytes_read | number | 总读取字节数 |
| total_bytes_written | number | 总写入字节数 |
| total_errors | number | 总错误次数 |
| read_count | number | 读操作次数 |
| write_count | number | 写操作次数 |
| list_count | number | 列表操作次数 |
| other_count | number | 其他操作次数 |
| average_read_size | number | 平均单次读取字节数（read_count>0 时） |
| average_write_size | number | 平均单次写入字节数（write_count>0 时） |

### 示例

```json
{
  "total_requests": 100,
  "total_bytes_read": 40960,
  "total_bytes_written": 1024,
  "total_errors": 0,
  "read_count": 10,
  "write_count": 2,
  "list_count": 5,
  "other_count": 83,
  "average_read_size": 4096,
  "average_write_size": 512
}
```

### 用法

- 仪表盘：展示总请求、读写吞吐、错误率。
- 周期统计：可定时拉取并做差值，得到周期内增量；重置请使用 POST `/api/v1/metrics/reset`（慎用）。

---

## 三、操作统计 GET /api/v1/metrics/operations

### 响应

数组，每项包含：

| 字段 | 类型 | 说明 |
|------|------|------|
| operation | string | 操作类型：`read`、`write`、`list`、`other` |
| count | number | 该类型操作次数 |
| bytes | number | 相关字节数（read/write 有值，list/other 多为 0） |
| errors | number | 该类型错误次数 |

### 示例

```json
[
  { "operation": "read", "count": 10, "bytes": 40960, "errors": 0 },
  { "operation": "write", "count": 2, "bytes": 1024, "errors": 0 },
  { "operation": "list", "count": 5, "bytes": 0, "errors": 0 },
  { "operation": "other", "count": 83, "bytes": 0, "errors": 0 }
]
```

### 用法

- 按操作类型展示占比、吞吐与错误率。
- 与 traffic 结合可计算各类型平均每次字节数。

---

## 四、系统状态 GET /api/v1/metrics/status

### 响应字段

| 字段 | 类型 | 说明 |
|------|------|------|
| status | string | 健康状态，如 `healthy` |
| uptime_secs | number | 进程运行时长（秒） |
| uptime | number | 同 uptime_secs |
| mounts | object | 挂载信息 |
| mounts.count | number | 当前挂载点数量 |
| mounts.list | string[] | 挂载路径列表 |
| traffic | object | 同 GET /api/v1/metrics/traffic 的响应体 |
| operations | array | 同 GET /api/v1/metrics/operations 的响应体 |

### 示例

```json
{
  "status": "healthy",
  "uptime_secs": 3600,
  "uptime": 3600,
  "mounts": { "count": 3, "list": ["/mem", "/hello", "/local"] },
  "traffic": { "total_requests": 100, "total_bytes_read": 40960, ... },
  "operations": [ ... ]
}
```

### 用法

- 监控页一次性拉取：健康、运行时长、挂载数、流量与操作汇总。
- evif-web 的 SystemStatus 等组件可直接使用本接口。

---

## 五、重置指标 POST /api/v1/metrics/reset

- **请求**：无 body。
- **响应**：`{ "message": "Metrics reset successfully" }`。
- **作用**：将 traffic/operations 相关计数清零；不影响挂载与业务数据。
- **注意**：生产环境慎用；多用于测试或按周期清零统计。

---

## 六、告警与 Prometheus（可选）

- **当前版本**：仅提供上述 HTTP JSON 接口，**未**内置告警或 Prometheus 暴露。
- **可选扩展**：
  - 在现有 `/api/v1/metrics/*` 上增加维度（如按路径/插件）或聚合周期。
  - 增加简单阈值告警（如 total_errors > N 时写日志或回调）。
  - 新增 Prometheus 暴露端点（如 `/metrics` 的 text format），由运维抓取。

实现上述扩展时，建议在本文档中补充端点与字段说明。

---

**文档版本**：与 EVIF 2.4 Phase 11.1 对应；文档说明监控指标与用法。
