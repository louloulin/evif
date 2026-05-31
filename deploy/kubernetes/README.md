# EVIF Kubernetes Deployment

## P1-5: Kubernetes Helm Chart

本目录包含 EVIF 生产部署所需的 Kubernetes 资源。

### 目录结构

```
deploy/kubernetes/
├── README.md                    # 本文件
├── helm/
│   └── evif/                   # Helm Chart
│       ├── Chart.yaml          # Chart 定义
│       ├── values.yaml         # 默认配置
│       ├── templates/         # K8s 资源模板
│       │   ├── deployment.yaml
│       │   ├── service.yaml
│       │   ├── pvc.yaml
│       │   └── _helpers.tpl
│       └── crds/              # CRD 定义 (未来)
└── kustomize/                  # Kustomize 覆盖 (未来)
```

### 快速部署

```bash
# 安装 EVIF
helm install evif ./deploy/kubernetes/helm/evif

# 升级
helm upgrade evif ./deploy/kubernetes/helm/evif

# 卸载
helm uninstall evif
```

### 配置示例

```yaml
# custom-values.yaml
replicaCount: 3

resources:
  limits:
    cpu: 4000m
    memory: 4Gi
  requests:
    cpu: 1000m
    memory: 1Gi

env:
  RUST_LOG: "debug"
  EVIF_REST_AUTH_MODE: "enabled"
```

### Prometheus 监控

Chart 内置 ServiceMonitor，当 `serviceMonitor.enabled=true` 时自动创建 Prometheus 抓取配置。

### 未来计划

- [ ] PrometheusOperator CRD 支持
- [ ] Vertical Pod Autoscaler (VPA) 配置
- [ ] Pod Disruption Budget (PDB)
- [ ] NetworkPolicy 配置
