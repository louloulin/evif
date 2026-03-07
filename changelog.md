# Changelog

All notable changes to the Mem platform will be documented in this file.

## [Unreleased]

## [1.0.2] - 2026-03-07

### Added
- **REST API - Graph Query Endpoint** (Phase 2 feature early implementation)
  - `POST /api/v1/graph/query` - Query temporal knowledge graph
  - Supported query types:
    - `causal_chain` - Find causal chain from start_node
    - `timeline` - Get event timeline with optional filters
    - `temporal_bfs` - Time-ordered BFS traversal
    - `temporal_path` - Find path between two nodes
  - Request: GraphQueryRequest with query_type, start_node, end_node, max_depth, event_type, category, time range
  - Response: GraphQueryResponse with nodes, paths, timeline fields

### Tests
- 5 new unit tests for GraphQueryRequest/Response serialization

### Changed
- Phase 1 progress: ~70% → ~75% complete
- mem.md status: Phase 2 graph query infrastructure ready

## [1.0.1] - 2026-03-07

### Added
- **REST API - Category Endpoints**
  - `GET /api/v1/categories` - List all categories
  - `GET /api/v1/categories/{id}` - Get category by ID
  - `GET /api/v1/categories/{id}/memories` - Get memories in category

### Changed
- Phase 1 progress: ~60% → ~70% complete
- mem.md status: Design → Implementing

## [1.0.0] - 2026-03-07

### Added
- **Core Components**
  - Core data models (MemoryItem, Resource, MemoryCategory)
  - Vector index module with InMemoryVectorIndex
  - MemPlugin EVIF plugin with MD file format support
  - Embedding manager with LRU cache
  - LLM client abstraction with OpenAI implementation

- **Pipelines**
  - MemorizePipeline for memory extraction
  - RetrievePipeline with VectorSearch and Hybrid modes

- **REST API - Memory Endpoints**
  - `POST /api/v1/memories` - Create memory
  - `POST /api/v1/memories/search` - Search memories
  - `GET /api/v1/memories/{id}` - Get memory by ID
  - `GET /api/v1/memories` - List all memories
