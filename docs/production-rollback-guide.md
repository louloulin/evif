# EVIF 回滚手册

> 创建时间：2026-04-03
> 适用版本：EVIF 0.1.0+
> 触发条件：部署后健康检查失败、核心功能异常、严重回归

---

## 一、回滚决策树

```
部署后服务异常？
├── 健康检查失败（/api/v1/health 返回非 healthy）
│   ├── 查看日志：docker compose logs evif-rest
│   │   ├── panic/启动错误 → 回滚镜像
│   │   └── 配置校验失败 → 检查环境变量
│   └── curl http://localhost:8081/api/v1/health
├── 认证/鉴权异常
│   ├── 确认 EVIF_REST_WRITE_API_KEYS / ADMIN_API_KEYS 未丢失
│   └── 审计日志：/var/log/evif/audit.log
└── 状态数据异常（租户/同步/加密状态）
    └── 检查持久化卷 evif-data 中的 JSON 文件是否损坏
```

---

## 二、Docker Compose 回滚步骤

### 2.1 步骤 1：保留当前卷

```bash
# 确认数据卷存在（关键：数据不丢失）
docker volume ls | grep evif

# 可选：备份当前状态文件
docker run --rm \
  -v evif-data:/data \
  -v $(pwd)/backup:/backup \
  alpine \
  sh -c "cp /data/*.json /backup/ 2>/dev/null || true"

ls backup/
```

### 2.2 步骤 2：回滚镜像版本

```bash
# 查看已部署的镜像标签（假设使用 Docker Hub）
docker images | grep evif

# 回滚到上一个稳定版本（替换 <PREVIOUS_TAG>）
docker pull evif/evif-rest:<PREVIOUS_TAG>

# 或者使用 SHA（最精确）：
# docker pull evif/evif-rest@sha256:<OLD_SHA>
```

### 2.3 步骤 3：重新启动服务

```bash
# 使用之前工作正常的 docker-compose 配置
docker compose -f docker-compose.yml -f docker-compose.prod.yml \
  down --remove-orphans

docker compose -f docker-compose.yml -f docker-compose.prod.yml \
  up -d

# 等待健康检查通过
sleep 10
curl -sf http://localhost:8081/api/v1/health
```

### 2.4 步骤 4：验证

```bash
# 健康检查
curl http://localhost:8081/api/v1/health
# 期望：{"status":"healthy","version":"...","uptime_secs":...}

# 状态文件完整性
docker run --rm \
  -v evif-data:/data \
  alpine \
  sh -c 'for f in /data/*.json; do echo "=== $f ==="; cat "$f" | python3 -m json.tool > /dev/null && echo "valid JSON" || echo "INVALID JSON"; done'

# 持久化状态验证（生产模式已配置持久化路径）
docker run --rm \
  -v evif-data:/data \
  alpine \
  sh -c 'jq -r ".tenants // .version // empty" /data/tenant-state.json 2>/dev/null && echo "tenant state OK"'
```

---

## 三、Kubernetes 回滚（如适用）

```bash
# 查看 deployment 历史
kubectl rollout history deployment/evif-rest

# 回滚到上一个版本
kubectl rollout undo deployment/evif-rest

# 回滚到指定版本
kubectl rollout undo deployment/evif-rest --to-revision=<N>

# 验证
kubectl rollout status deployment/evif-rest
kubectl logs -l app=evif-rest --tail=20
```

---

## 四、数据状态回滚注意事项

### 4.1 租户状态（tenant-state.json）

- JSON 格式，删除后重新创建租户即可恢复（租户元数据）
- 租户下的业务数据取决于后端存储（如 SQLite）

### 4.2 同步状态（sync-state.json）

- 包含 `version / pending_changes / tracked_paths`
- 丢失后重新初始化，同步从头开始，不影响数据完整性

### 4.3 加密状态（encryption-state.json）

- 仅持久化 `enabled / key_source / key_reference`
- 裸 Key 不写入文件，依赖 `env:KEY_NAME` 引用恢复
- 确认 KEY_NAME 环境变量与恢复后的 JSON 一致

---

## 五、回滚后操作

1. 通知相关方（监控、值班）
2. 在监控系统中记录回滚事件
3. 在部署记录中标记失败版本，避免再次部署
4. 分析失败原因，更新部署检查清单
5. 如有数据损坏，启动数据恢复流程

---

## 六、快速检查清单

- [ ] 当前卷（evif-data）已确认存在
- [ ] 已备份状态 JSON 文件
- [ ] 目标回滚版本已验证可用
- [ ] 服务重启后健康检查通过
- [ ] 状态文件 JSON 格式有效
- [ ] 核心 API（读/写/列表）功能正常
- [ ] 已通知相关方
