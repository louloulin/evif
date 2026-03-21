# EVIF Memory Python SDK

A Python SDK for the EVIF Memory Platform - AI-native memory storage with vector search, categorization, and self-evolution.

## Features

- **Memory Storage**: Store and retrieve memories with semantic search
- **Vector Search**: Fast similarity-based memory retrieval
- **Category Organization**: Automatic memory categorization
- **Multi-Modal Support**: Text, conversation, document support
- **REST API Client**: Easy integration with existing systems

## Installation

```bash
pip install evif-mem
```

## Quick Start

```python
import asyncio
from evif_mem import EvifMemoryClient, MemoryConfig

async def main():
    # Initialize client
    config = MemoryConfig(
        api_url="http://localhost:8080",
        api_key="your-api-key"
    )
    client = EvifMemoryClient(config)

    # Store a memory
    memory = await client.create_memory(
        content="User prefers dark mode UI",
        memory_type="preference",
        tags=["ui", "preferences"]
    )
    print(f"Created memory: {memory.id}")

    # Search memories
    results = await client.search_memories(
        query="user interface preferences",
        k=5
    )
    for result in results:
        print(f"- {result.content} (score: {result.score})")

asyncio.run(main())
```

## API Reference

### EvifMemoryClient

Main client for interacting with the EVIF Memory API.

#### Methods

- `create_memory(content, memory_type, tags)`: Store a new memory
- `search_memories(query, k, threshold)`: Search memories by semantic similarity
- `get_memory(memory_id)`: Get a specific memory
- `list_memories(limit, offset)`: List all memories
- `list_categories()`: List all categories
- `get_category(category_id)`: Get a specific category
- `get_category_memories(category_id)`: Get memories in a category
- `query_memories(query_type, ...)`: Query memory timeline and relationship views

### MemoryConfig

Configuration for the client.

- `api_url`: Base URL of the EVIF Memory API
- `api_key`: API key for authentication
- `timeout`: Request timeout in seconds (default: 30)
- `max_retries`: Maximum number of retries (default: 3)

## Development

```bash
# Install in development mode
pip install -e .

# Run tests
pytest tests/

# Format code
black evif_mem/
ruff check evif_mem/
```

## License

MIT
