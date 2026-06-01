# EVIF TypeScript SDK

TypeScript/JavaScript SDK for EVIF API.

## Installation

```bash
npm install evif-sdk
```

## Usage

```typescript
import { EvifClient } from 'evif-sdk';

// Create client
const client = new EvifClient({
  baseUrl: 'https://api.evif.io/v1',
  apiKey: 'your-api-key',
});

// List files
const entries = await client.listDirectory('/mem');

// Read file
const content = await client.readFile('/mem/myfile.txt');

// Search memory
const results = await client.searchMemory({
  query: 'project documentation',
  limit: 10,
});

// List MCP tools
const tools = await client.listMcpTools();

// Call MCP tool
const result = await client.callMcpTool({
  tool: 'filesystem.read',
  arguments: { path: '/mem/file.txt' },
});
```

## API Reference

See the [OpenAPI documentation](https://github.com/louloulin/evif/blob/main/docs/openapi/evif-api.yaml) for complete API details.

## License

MIT OR Apache-2.0
