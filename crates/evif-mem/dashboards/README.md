# evif-mem Grafana Dashboards

> Grafana dashboard templates for evif-mem Prometheus metrics monitoring.

## Quick Start

### Using Docker Compose

```bash
cd dashboards
docker-compose up -d
```

Then access Grafana at http://localhost:3000:
- Username: `admin`
- Password: `admin`

### Manual Setup

1. Copy the dashboard JSON to your Grafana provisioning directory:
   ```bash
   cp dashboards/evif-mem-overview.json /etc/grafana/provisioning/dashboards/
   ```

2. Configure Prometheus to scrape evif-mem metrics:
   ```yaml
   scrape_configs:
     - job_name: 'evif-mem'
       static_configs:
         - targets: ['localhost:9091']
   ```

## Dashboard Panels

### Overview
- **Total Memorize Operations**: Counter for all memorize operations
- **Total Retrieve Operations**: Counter for all retrieve operations
- **Total Evolve Operations**: Counter for all evolve operations
- **Total Errors**: Error counter

### Storage Metrics
- **Storage Totals**: Memory items, categories, and resources over time
- **Active Operations**: Currently running memorize/retrieve/evolve operations

### Operation Latency
- **Average Latency**: Mean latency for each operation type
- **Latency Distribution**: p95 and p99 percentiles for memorize operations

### Operation Rates
- **Operation Rates**: Operations per second for each type
- **Error Rate**: Percentage of operations that resulted in errors

## Metrics Reference

| Metric | Type | Description |
|--------|------|-------------|
| `evif_mem_memorize_total` | Counter | Total memorize operations |
| `evif_mem_retrieve_total` | Counter | Total retrieve operations |
| `evif_mem_evolve_total` | Counter | Total evolve operations |
| `evif_mem_errors_total` | Counter | Total errors |
| `evif_mem_memorize_duration_seconds` | Histogram | Memorize operation duration |
| `evif_mem_retrieve_duration_seconds` | Histogram | Retrieve operation duration |
| `evif_mem_evolve_duration_seconds` | Histogram | Evolve operation duration |
| `evif_mem_active_memorize` | Gauge | Active memorize operations |
| `evif_mem_active_retrieve` | Gauge | Active retrieve operations |
| `evif_mem_active_evolve` | Gauge | Active evolve operations |
| `evif_mem_memory_items_total` | Gauge | Total memory items |
| `evif_mem_categories_total` | Gauge | Total categories |
| `evif_mem_resources_total` | Gauge | Total resources |

## Integration with evif-mem

Enable metrics in your evif-mem configuration:

```rust
use evif_mem::metrics::{MetricsRegistry, MetricsConfig};

let registry = MetricsRegistry::new();
registry.init(MetricsConfig::default()).await.unwrap();
```

Or via feature flag:
```toml
# Cargo.toml
evif-mem = { version = "0.1.0", features = ["metrics"] }
```

## License

MIT
