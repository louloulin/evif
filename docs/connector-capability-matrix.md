# EVIF Connector Capability Matrix

> Status: MVP 6.0 planning document  
> Purpose: define EVIF as a universal connectivity layer for AI Agents, not just a collection of plugins.

## Product Thesis

EVIF connects heterogeneous systems through a consistent mount and file interface. AI Agents should not need a custom mental model for every API, database, SaaS tool, or storage backend. They should work with a small set of durable concepts:

- files
- directories
- context
- skills
- memories
- pipes
- queues
- mounts

This matrix turns the current plugin surface into a product map.

## Capability Levels

| Level | Meaning | Agent Value |
|-------|---------|-------------|
| L0: Discover | Connector can be listed and inspected | Agent can understand available systems |
| L1: Read | Connector supports read/list/search-style access | Agent can gather external context |
| L2: Write | Connector supports create/update/delete-style operations | Agent can take action |
| L3: Coordinate | Connector supports queue, pipe, event, or workflow semantics | Agent can collaborate or automate |
| L4: Optimize | Connector supports compact output, cache, batch, stream, or bounded retrieval | Agent can reduce token and tool-loop cost |

## Core Agent Primitives

| Connector | Mount | Primary Capability | Level | MVP Role |
|-----------|-------|--------------------|-------|----------|
| `contextfs` | `/context` | L0/L1/L2 working context | L2 | Core |
| `skillfs` | `/skills` | `SKILL.md` discovery and invocation | L2 | Core |
| `pipefs` | `/pipes` | Multi-agent task exchange | L3 | Core |
| `queuefs` | `/queue` | Durable asynchronous tasks | L3 | Core |
| `vectorfs` | `/memories` | Semantic memory retrieval | L4 | Core |
| `memfs` | `/mem` | In-memory filesystem | L2 | Core demo |

## Storage Connectors

| Connector | System | Read | Write | Search/List | Notes |
|-----------|--------|------|-------|-------------|-------|
| `localfs` | Local filesystem | Yes | Yes | List | Useful for local development and demos |
| `httpfs` | HTTP/HTTPS resources | Yes | Limited | URL-based | Useful as a generic web resource adapter |
| `s3fs` / `s3fs_opendal` | Amazon S3 | Yes | Yes | List | Cloud object storage adapter |
| `gcsfs` | Google Cloud Storage | Yes | Yes | List | Cloud object storage adapter |
| `azureblobfs` | Azure Blob Storage | Yes | Yes | List | Cloud object storage adapter |
| `aliyunossfs` | Aliyun OSS | Yes | Yes | List | Cloud object storage adapter |
| `tencentcosfs` | Tencent COS | Yes | Yes | List | Cloud object storage adapter |
| `huaweiobsfs` | Huawei OBS | Yes | Yes | List | Cloud object storage adapter |
| `miniofs` | MinIO | Yes | Yes | List | S3-compatible local/private storage |
| `webdavfs` | WebDAV | Yes | Yes | List | Enterprise and legacy storage integration |
| `ftpfs` | FTP | Yes | Yes | List | Legacy protocol integration |
| `sftpfs` | SFTP | Yes | Yes | List | Secure legacy/server integration |

## Database Connectors

| Connector | System | Read | Write | Query | Notes |
|-----------|--------|------|-------|-------|-------|
| `sqlfs` | SQLite | Yes | Yes | SQL-backed | Good for local durable demos |
| `sqlfs2` | SQLite variant | Yes | Yes | SQL-backed | Needs product-level consolidation decision |
| `postgresfs` | PostgreSQL | Yes | Yes | SQL-backed | Good for production persistence |

## Application Connectors

| Connector | System | Read | Write | Agent Use Case |
|-----------|--------|------|-------|----------------|
| `githubfs` | GitHub | Yes | Yes | Code, issues, PR automation |
| `gmailfs` | Gmail | Yes | Yes | Email triage, drafting, follow-up |
| `slackfs` | Slack | Yes | Yes | Team messaging and coordination |
| `discordfs` | Discord | Yes | Yes | Community automation |
| `notionfs` | Notion | Yes | Yes | Knowledge base and docs |
| `telegramfs` | Telegram | Yes | Yes | Messaging automation |
| `teamsfs` | Microsoft Teams | Yes | Yes | Enterprise collaboration |
| `shopifyfs` | Shopify | Yes | Yes | Commerce automation |

## System and Utility Connectors

| Connector | Role | Agent Value |
|-----------|------|-------------|
| `encryptedfs` | Encryption wrapper | Protect sensitive context and memory |
| `proxyfs` | Proxy/wrapper filesystem | Add seams for policy, logging, or routing |
| `tieredfs` | Multi-tier storage | Balance performance and persistence |
| `streamfs` / `streamrotatefs` | Streaming/log-like data | Handle growing outputs without loading all content |
| `handlefs` | Handle exposure | Represent open resources in file form |
| `heartbeatfs` | Liveness/status | Support supervision and monitoring |
| `serverinfofs` | Server metadata | Let Agents inspect runtime state |
| `skill_runtime` | Skill execution support | Move skills from passive docs toward executable workflows |

## Platform Integration

EVIF also connects to AI development platforms through `evif connect`.

| Platform | Method | Role |
|----------|--------|------|
| Claude Desktop | MCP JSON config | Agent client integration |
| Claude Code | MCP JSON config | Coding-agent integration |
| Cursor | MCP JSON config | IDE-agent integration |
| Gemini CLI | MCP JSON config | CLI-agent integration |
| OpenAI Codex | Rules file / AGENTS.md | Coding-agent workflow integration |

## MVP 6.0 Priorities

| Priority | Work Item | Reason |
|----------|-----------|--------|
| P0 | Define official core connector set: `contextfs`, `skillfs`, `pipefs`, `queuefs`, `vectorfs`, `memfs` | These are the Agent product primitives |
| P0 | Document connector capability levels | Prevent “plugin count” from replacing product clarity |
| P1 | Add compact-output expectations per connector category | Make cost optimization part of the connector contract |
| P1 | Separate core connectors from experimental connectors | Improve trust and onboarding |
| P2 | Add connector conformance tests | Ensure new connectors behave predictably |

## Open Decisions

- Whether `sqlfs` and `sqlfs2` should be consolidated or clearly differentiated.
- Which SaaS connectors should be considered “officially supported” versus experimental.
- Whether every connector should declare a machine-readable capability manifest.
- Whether compact output, pagination, and bounded reads should be mandatory for all L1/L2 connectors.
