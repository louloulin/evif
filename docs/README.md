# EVIF Documentation

## OpenAPI Specification

The complete API specification is available at `openapi/evif-api.yaml`.

### Using the OpenAPI Spec

1. **View in Swagger UI**:
   - Open https://petstore.swagger.io/
   - Paste the URL to the raw YAML file

2. **Generate Client SDKs**:
   ```bash
   # Using openapi-generator
   openapi-generator-cli generate \
     -i openapi/evif-api.yaml \
     -g python \
     -o sdk/python

   # Using redocly
   npx @redocly/openapi-cli bundle openapi/evif-api.yaml -o dist/bundle.yaml
   ```

3. **Validate the Spec**:
   ```bash
   npx @redocly/openapi-cli lint openapi/evif-api.yaml
   ```

## API Documentation

### Authentication

All API requests require authentication via API key:

```bash
curl -H "X-API-Key: your-api-key" https://api.evif.io/v1/health
```

### Rate Limiting

- **Free tier**: 1,000 requests/day
- **Pro tier**: 100,000 requests/day
- **Team tier**: 1,000,000 requests/day
- **Enterprise**: Unlimited

### Error Codes

| Code | Description |
|------|-------------|
| EVIF_0400 | Bad Request |
| EVIF_0401 | Unauthorized |
| EVIF_0403 | Forbidden |
| EVIF_0404 | Not Found |
| EVIF_0500 | Internal Error |
| EVIF_0504 | Timeout |

## Metrics

### VFS Metrics

| Metric | Description |
|--------|-------------|
| `evif_vfs_read_bytes_total` | Total bytes read |
| `evif_vfs_write_bytes_total` | Total bytes written |
| `evif_vfs_operations_total` | Total operations by type |
| `evif_vfs_latency_seconds` | Operation latency histogram |

### MCP Metrics

| Metric | Description |
|--------|-------------|
| `evif_mcp_tools_invoked_total` | Tool invocations |
| `evif_mcp_latency_seconds` | Tool invocation latency |
| `evif_mcp_errors_total` | Tool errors |

### Memory Metrics

| Metric | Description |
|--------|-------------|
| `evif_memory_items_total` | Total memory items |
| `evif_memory_searches_total` | Search operations |
| `evif_memory_embeddings_total` | Embedding generations |
