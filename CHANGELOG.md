# Changelog

All notable changes to EVIF will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-05-06

### Added

#### Multi-Agent Coordination (PipeFS)
- `wait_for_result()` with tokio::Notify blocking and timeout
- `try_claim()` for atomic pipe claiming (prevents race conditions)
- 5-state state machine: pending, running, completed, error, timeout
- Per-pipe Notify for wake-on-write
- Retry support: error/timeout → pending transitions
- 10 coordination tests in `evif-plugins/tests/pipefs_coordination.rs`

#### Write Operations (Platform Plugins)
- **gmailfs**: send, reply, drafts, labels, trash via Gmail REST API
- **slackfs**: post messages, reactions, delete via Slack Web API
- **discordfs**: send messages, embeds, reactions, delete via Discord REST API

#### Token Optimization
- Phase 2: Shortened 8 tool schemas and 2 prompt descriptions (~1000 tokens/session savings)
- Removed deprecated `text` alias from `evif_memorize`
- Output filtering pipeline (`evif-mcp/src/output_filter.rs`):
  - strip_ansi, truncate_lines, compact_json, max_string_length
  - 11 tests covering all filter stages
- Phase 1: `evif_cat max_lines/mode` parameters (60-90% savings)
- Phase 3: `evif_memory_search compact` mode (40-60% savings)

#### Platform Integration Tests
- 12 install script tests (`evif-rest/tests/install_script.rs`)
- 10 multi-platform connect tests (`evif-rest/tests/platform_connect.rs`)
- 5 context recovery tests (`evif-plugins/tests/context_recovery.rs`)
- 27 E2E REST API tests

#### Benchmark Suite
- 24 benchmark tests (`evif-bench`) covering performance, agent, IDE, L0, OSWorld scenarios

#### evif-client Library
- 46 unit tests with full API coverage
- 655 lines of documentation
- Transport layer with HTTP client

### Fixed
- Replaced all `std::sync::RwLock` with `parking_lot::RwLock` across evif-mcp (poison-resistant)
- Eliminated 53 `.unwrap()` calls on locks in mcp_router.rs, mcp_auth.rs, mcp_server_plugin.rs
- Replaced `expect()` with `Result` propagation in contextfs.rs SQLite persistence
- All 257 compiler warnings eliminated across workspace

### Changed
- MCP Server rate limiting: sliding window 1000 req/min
- evif-cli now has `[lib]` + `[[bin]]` targets for unit testing
- evif-metrics: 67 tests with comprehensive error, types, prometheus, traffic coverage

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
- MCP Server with 63 tools, 3 resources via stdio JSON-RPC
- VFS Core with plugin registry (40+ plugins)
- ContextFS: L0/L1/L2 three-layer context system
- SkillFS: SKILL.md skill discovery and execution
- PipeFS: Multi-agent task coordination
- MemoryFS: Vector storage with semantic search
- REST API with 150 endpoints, authentication and metrics
- FUSE mount support (optional)
- CLI with 60+ commands and REPL mode
- 600+ tests across 13 crates
- 13 crates: evif-core, evif-plugins, evif-rest, evif-cli, evif-mcp, evif-mem, evif-client, evif-metrics, evif-auth, evif-bench, evif-fuse, evif-macros, example-dynamic-plugin