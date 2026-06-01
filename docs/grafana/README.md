# EVIF Grafana Dashboards

## Overview

This directory contains Grafana dashboard configurations for EVIF monitoring and observability.

## Files

| File | Description |
|------|-------------|
| `evif-dashboard.json` | Main system overview dashboard |
| `evif-alerts.json` | Alert rules for Prometheus Alertmanager |

## Dashboard Sections

### System Overview
- System uptime
- API requests/second
- P99 latency
- Active tenants count

### VFS Operations
- Read/Write operation rates
- Read/Write throughput (bytes/s)
- Operation latency percentiles

### MCP Protocol
- Tool invocation rates
- Tool latency percentiles
- Tool usage by type

### Business Metrics
- Monthly Recurring Revenue (MRR)
- Subscriptions by plan
- Marketplace GMV

## Installation

### Import Dashboard

1. Open Grafana
2. Click "+" → "Import"
3. Upload `evif-dashboard.json` or paste JSON content
4. Select Prometheus datasource
5. Click "Import"

### Prometheus Metrics

Add these metrics to your Prometheus scrape config:

```yaml
scrape_configs:
  - job_name: 'evif'
    static_configs:
      - targets: ['localhost:9090']
    metrics_path: '/metrics'
```

## Available Metrics

### HTTP Metrics
- `evif_http_requests_total` - Total HTTP requests
- `evif_http_request_duration_seconds` - Request duration histogram

### VFS Metrics
- `evif_vfs_read_total` - Total read operations
- `evif_vfs_write_total` - Total write operations
- `evif_vfs_bytes_read_total` - Total bytes read
- `evif_vfs_bytes_written_total` - Total bytes written

### MCP Metrics
- `evif_mcp_tools_invoked_total` - Tool invocations
- `evif_mcp_latency_seconds` - Tool latency histogram
- `evif_mcp_errors_total` - Tool errors

### Memory Metrics
- `evif_memory_items_total` - Total memory items
- `evif_memory_searches_total` - Search operations
- `evif_memory_embeddings_total` - Embedding generations

### Business Metrics
- `evif_active_tenants` - Active tenant count
- `evif_stripe_revenue_cents` - Revenue in cents
- `evif_subscriptions_total` - Subscriptions by plan

## Alert Rules

### Critical Alerts

```yaml
groups:
  - name: evif.critical
    rules:
      - alert: EVIFDown
        expr: up{job="evif"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "EVIF instance is down"
          
      - alert: HighErrorRate
        expr: sum(rate(evif_http_requests_total{status=~"5.."}[5m])) > 0.01
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High error rate detected"
```

### Warning Alerts

```yaml
      - alert: HighLatency
        expr: histogram_quantile(0.99, sum(rate(evif_http_request_duration_seconds_bucket[5m]))) > 1
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "P99 latency above 1 second"
```

## SLO Targets

| Metric | Target | Alert Threshold |
|--------|--------|----------------|
| Availability | 99.9% | < 99.5% |
| P99 Latency | < 500ms | > 1s |
| Error Rate | < 0.1% | > 1% |

## Screenshots

### System Overview
![Overview](overview.png)

### VFS Operations
![VFS](vfs.png)

### MCP Protocol
![MCP](mcp.png)

## Customization

### Variables

You can add dashboard variables for filtering:

```json
{
  "templating": {
    "list": [
      {
        "name": "tenant",
        "type": "query",
        "query": "label_values(evif_http_requests_total, tenant_id)"
      }
    ]
  }
}
```

## Support

For issues with dashboards, contact the EVIF team.
