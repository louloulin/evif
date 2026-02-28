# 图 API 状态说明（Phase 11.3）

## 当前状态：占位、未实现

EVIF REST 中以下端点**未实现图功能**，仅返回 **500 Internal Server Error**，并提示使用文件系统 API：

| 路径 | 方法 | 说明 |
|------|------|------|
| `/nodes/:id` | GET | 返回："Graph functionality not implemented. Filesystem operations are available via /api/v1/* endpoints." |
| `/nodes/:id` | DELETE | 提示使用 /api/v1/files 等文件系统操作 |
| `/nodes/create/:node_type` | POST | 提示使用 /api/v1/files/create |
| `/query` | POST | 提示使用 /api/v1/files/list 等 |
| `/nodes/:id/children` | GET | 提示使用 /api/v1/directories 等 |
| `/stats` | GET | 返回占位结构（uptime_secs/total_nodes 等为 0） |

## 决策结论

- **不保留图 API 业务实现**：当前 EVIF 主路径以「挂载表 + 插件文件系统」为核心，与 AGFS 对标的是文件/目录/句柄/流式能力，**图能力非本周期范围**。
- **保留路由占位**：为避免破坏可能引用上述路径的客户端，路由保留，统一返回明确错误信息与替代建议。
- **替代方式**：所有“节点/查询/子节点”类需求请使用：
  - **文件/目录**：GET/POST/DELETE `/api/v1/files`、`/api/v1/directories`，GET `/api/v1/stat`，POST `/api/v1/rename` 等；
  - **列表/搜索**：GET `/api/v1/directories?path=...`，POST `/api/v1/grep`。

若未来需真正的图能力，需基于 `evif-graph` 实现上述端点的业务逻辑，并在本文档中更新为「已实现」并注明版本。

---

**文档版本**：与 EVIF 2.4 Phase 11.3 对应；文档中已明确图功能状态。
