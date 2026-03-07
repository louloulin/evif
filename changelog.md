# Changelog

All notable changes to the Mem platform will be documented in this file.

## [Unreleased]

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
