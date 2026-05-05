# Changelog

All notable changes to EVIF will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-05-05

### Added
- `evif connect` command for AI platform integration (Claude Desktop, Claude Code, Cursor, Gemini CLI, OpenAI Codex)
  - Atomic config file patching with backup and idempotency
  - `--list`, `--check`, `--disconnect` flags
- Token optimization for MCP tools:
  - `evif_cat`: `max_lines` (default 100) and `mode` (head|tail|snippet|full) parameters
  - `evif_memory_search`: `compact` mode (default true), `limit` reduced from 10 to 3
  - Estimated 60-90% savings on cat calls, 40-60% on memory search
- `[lib]` target in evif-cli for unit testing support
- CHANGELOG.md

### Fixed
- Replaced `std::sync::RwLock` with `parking_lot::RwLock` across mcp_router.rs, mcp_auth.rs, mcp_server_plugin.rs
  - Eliminates 53 poison-prone `.unwrap()` calls that could crash the MCP server
- Replaced `expect()` with `Result` propagation in contextfs.rs SQLite persistence
  - Prevents process crash when database is inaccessible
- Eliminated all 94 compiler warnings across the workspace (evif-plugins 74, evif-mem 4, evif-metrics 3, evif-rest 2, evif-auth 1, evif-cli 4)

### Changed
- evif-cli now has both `[lib]` and `[[bin]]` targets, enabling `cargo test -p evif-cli` for unit tests

## [0.1.0] - 2026-04-29

### Added
- Initial release of EVIF (Everything Is a File)
- MCP Server with 19 tools, 3 resources via stdio JSON-RPC
- VFS Core with plugin registry (47 plugins)
- ContextFS: L0/L1/L2 three-layer context system
- SkillFS: SKILL.md skill discovery and execution
- PipeFS: Multi-agent task coordination
- MemoryFS: Vector storage with semantic search
- REST API with authentication and metrics
- FUSE mount support (optional)
- CLI with REPL and script execution
- 649+ tests across 13 test suites
