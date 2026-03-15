# evif-mem TypeScript SDK

TypeScript SDK for the EVIF Memory API. Provides methods for creating, retrieving, and searching memories.

## Installation

```bash
npm install evif-mem
# or
yarn add evif-mem
# or
pnpm add evif-mem
```

## Quick Start

```typescript
import { EvifMemoryClient, MemoryConfig, MemoryType } from 'evif-mem';

const config = new MemoryConfig({
  apiUrl: 'http://localhost:8080',
  apiKey: 'your-api-key', // optional
});

const client = new EvifMemoryClient(config);

// Create a memory
const memory = await client.createMemory(
  'User prefers dark mode in the application',
  {
    memoryType: MemoryType.PROFILE,
    tags: ['ui', 'preferences'],
  }
);

// Search memories
const results = await client.searchMemories('user preferences', {
  k: 5,
  threshold: 0.5,
  mode: 'vector',
});

for (const result of results) {
  console.log(`${result.memory.content} (score: ${result.score})`);
}

// Close the client when done
await client.close();
```

## Configuration

```typescript
interface MemoryConfigOptions {
  /** Base URL of the EVIF Memory API */
  apiUrl: string;
  /** Optional API key for authentication */
  apiKey?: string;
  /** Request timeout in milliseconds (default: 30000) */
  timeout?: number;
  /** Maximum number of retry attempts (default: 3) */
  maxRetries?: number;
}
```

## API Reference

### Client Methods

#### `createMemory(content, options?)`

Create a new memory.

```typescript
const memory = await client.createMemory(
  'User prefers dark mode',
  {
    memoryType: MemoryType.PROFILE,
    tags: ['ui'],
    modality: Modality.TEXT,
    metadata: { source: 'settings' },
  }
);
```

#### `getMemory(memoryId)`

Get a specific memory by ID.

```typescript
const memory = await client.getMemory('mem-123');
```

#### `listMemories(options?)`

List all memories with pagination.

```typescript
const memories = await client.listMemories({
  limit: 100,
  offset: 0,
});
```

#### `searchMemories(query, options?)`

Search memories by semantic similarity.

```typescript
const results = await client.searchMemories('user preferences', {
  k: 10,
  threshold: 0.5,
  mode: 'vector', // 'vector' | 'hybrid' | 'rag'
});
```

#### `deleteMemory(memoryId)`

Delete a memory.

```typescript
await client.deleteMemory('mem-123');
```

#### `listCategories()`

List all categories.

```typescript
const categories = await client.listCategories();
```

#### `getCategory(categoryId)`

Get a specific category.

```typescript
const category = await client.getCategory('cat-123');
```

#### `getCategoryMemories(categoryId, limit?)`

Get memories in a category.

```typescript
const memories = await client.getCategoryMemories('cat-123', 100);
```

#### `queryGraph(queryType, options?)`

Query the knowledge graph.

```typescript
const result = await client.queryGraph(GraphQueryType.TIMELINE, {
  startNode: 'mem-123',
  maxDepth: 3,
  eventType: 'knowledge',
});

console.log('Nodes:', result.nodes);
console.log('Timeline:', result.timeline);
console.log('Total:', result.total);
```

## Data Models

### MemoryType

```typescript
enum MemoryType {
  PROFILE = 'profile',
  EVENT = 'event',
  KNOWLEDGE = 'knowledge',
  BEHAVIOR = 'behavior',
  SKILL = 'skill',
  TOOL = 'tool',
  CONVERSATION = 'conversation',
  DOCUMENT = 'document',
}
```

### Modality

```typescript
enum Modality {
  TEXT = 'text',
  CONVERSATION = 'conversation',
  DOCUMENT = 'document',
  IMAGE = 'image',
  VIDEO = 'video',
  AUDIO = 'audio',
}
```

### GraphQueryType

```typescript
enum GraphQueryType {
  CAUSAL_CHAIN = 'causal_chain',
  TIMELINE = 'timeline',
  TEMPORAL_BFS = 'temporal_bfs',
  TEMPORAL_PATH = 'temporal_path',
}
```

### Memory

```typescript
interface Memory {
  id: string;
  content: string;
  summary: string;
  memory_type: MemoryType | string;
  tags: string[];
  embedding?: number[];
  reinforcement_count: number;
  last_reinforced_at?: string;
  created_at: string;
  updated_at: string;
  references: string[];
  user_id?: string;
  tenant_id?: string;
  metadata: Record<string, unknown>;
}
```

### Category

```typescript
interface Category {
  id: string;
  name: string;
  description: string;
  summary: string;
  item_count: number;
  embedding?: number[];
  created_at: string;
  updated_at: string;
}
```

### GraphResult

```typescript
interface GraphResult {
  query_type: string;
  nodes?: GraphNode[];
  paths?: GraphPathInfo[];
  timeline?: TimelineEvent[];
  total: number;
}
```

## Development

### Build

```bash
npm run build
```

### Test

```bash
npm test
# or watch mode
npm run test:watch
```

## License

MIT
