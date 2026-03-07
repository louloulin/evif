# Changelog

All notable changes to the Mem platform will be documented in this file.

## [Unreleased]

## [1.0.4] - 2026-03-07

### Added
- **Categorizer Integration in MemorizePipeline** (Phase 1 feature)
  - Auto-categorizes memory items using vector similarity
  - `Categorizer::categorize()` method for memory classification
  - `find_similar_category()` for matching existing categories
  - `create_new_category()` with LLM-generated category descriptions
  - Category embeddings prefixed with "cat:" in vector index
  - Configurable similarity threshold (default: 0.7)

### Changed
- Phase 1 progress: ~80% → ~85% complete
- MemorizePipeline now calls categorizer after storing memory items
- Memory items automatically linked to categories

## [1.0.3] - 2026-03-07

### Added
- **RetrievePipeline - LLMRead Mode** (Phase 1 feature)
  - `RetrieveMode::LLMRead` variant for Mode 2 retrieval
  - Reads memories directly from category for LLM analysis
  - Uses LLM reasoning capability for deep analysis (no vector dependency)
  - Best for: "Analyze all knowledge in this category"
  - `llm_read_search()` method implementation
  - `format_memories_for_llm()` helper for text formatting
  - `build_llm_read_prompt()` for analysis prompt construction
  - `parse_llm_read_response()` for JSON response parsing
  - Graceful fallback on parsing failures

### Tests
- 5 new unit tests for LLMRead mode:
  - test_llm_read_mode_enum - Mode creation
  - test_llm_read_mode_serialization - JSON encoding/decoding
  - test_format_memories_for_llm - Memory formatting
  - test_llm_read_response_parsing - Response parsing
  - test_retrieve_pipeline_creation - Pipeline structure

### Changed
- Phase 1 progress: ~75% → ~80% complete
- RetrievePipeline now has 3 modes: VectorSearch, LLMRead, Hybrid

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
