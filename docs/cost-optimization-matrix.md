# EVIF Cost Optimization Matrix

> Status: MVP 6.0 planning document  
> Purpose: make EVIF's cost reduction model explicit for AI Agent workloads.

## Product Thesis

AI Agent cost is not only model pricing. Real Agent systems pay for repeated context loading, bespoke integration code, oversized tool outputs, failed retries, coordination overhead, and long-term maintenance.

EVIF reduces cost by turning external systems and Agent working state into a consistent, bounded, cacheable, and retrievable file interface.

## Cost Categories

| Cost Category | Common Failure Mode | EVIF Lever |
|---------------|---------------------|------------|
| Token cost | Tools return huge files or JSON blobs | `max_lines`, compact output, output filtering |
| Context setup cost | Every session repeats background and decisions | `/context/L0`, `/context/L1`, `/context/L2` |
| Retrieval cost | Agent loads everything because search is coarse | vector memory, bounded search, compact results |
| Integration cost | Every API requires custom glue | plugin + mount adapter model |
| Coordination cost | Multi-agent work is copied through prompts | `/pipes`, `/queue` |
| Failure recovery cost | Downstream calls fail unpredictably | circuit breakers, retries, health, metrics |
| Maintenance cost | Each platform has a different workflow | CLI, REST, MCP, SDKs over the same model |

## Optimization Primitives

| Primitive | Where It Appears | Cost Reduced | Product Rule |
|-----------|------------------|--------------|--------------|
| `max_lines` | MCP/file reads | Token cost | Default reads should be bounded |
| `mode=head/tail/snippet/full` | MCP/file reads | Token cost | Full reads should be explicit |
| `compact=true` | Memory/search results | Token cost | Search should return concise answers first |
| `limit` | Search/retrieval | Token and latency | Default result count should be small |
| `truncate_lines` | Output filter | Token cost | Long tool output should be clipped |
| `compact_json` | Output filter | Token cost | Null/empty noise should be removed |
| `max_string_length` | Output filter | Token cost | Huge fields should not enter context by default |
| Tool cache | MCP layer | Repeated tool cost | Repeated deterministic calls should be reusable |
| Batch operations | Core/API layer | Tool-loop latency | Repeated small operations should be grouped |
| Streaming | Core/plugin layer | Memory and token spikes | Large data should not require full materialization |

## Agent Workflow Cost Model

| Workflow | Without EVIF | With EVIF |
|----------|--------------|-----------|
| Start a coding task | Reload project context into prompt | Read `/context/L0`, `/context/L1`, selected `/context/L2` |
| Reuse a workflow | Copy/paste instructions | Discover `/skills/*.SKILL.md` |
| Delegate to another Agent | Prompt-forward task and context | Write to `/pipes/<task>/input`, read result |
| Search prior knowledge | Paste logs/docs into model | Query `/memories` with compact result limit |
| Connect a new system | Write custom API wrapper | Mount a plugin adapter |
| Inspect a large file | Load entire file into context | `evif_cat` with `max_lines` and `mode` |
| Read noisy JSON | Feed raw API output to model | Output filter with compact JSON and truncation |

## Default Cost Policy

MVP 6.0 should make low-cost behavior the default.

| Policy | Default | Rationale |
|--------|---------|-----------|
| File reads | Bounded | Prevent accidental context flooding |
| Search results | Compact and limited | Preserve relevance and reduce tokens |
| JSON output | Compacted | Remove structural noise |
| Large strings | Truncated | Avoid runaway tool responses |
| Full output | Opt-in | Make expensive operations intentional |
| Repeated calls | Cache-aware | Avoid duplicate tool loops |
| Multi-step operations | Batch when possible | Reduce round trips |

## Measurement Targets

| Metric | Target Direction | Example Measurement |
|--------|------------------|---------------------|
| Average tool response size | Down | Bytes/tokens per MCP call |
| Context reload frequency | Down | Number of repeated background prompts per task |
| Integration code per system | Down | Lines of custom glue outside connector |
| Multi-agent handoff overhead | Down | Prompt tokens replaced by `/pipes` artifacts |
| Failed downstream loops | Down | Retry count and circuit breaker events |
| Time to first Agent demo | Down | Minutes from install to working workflow |

## MVP 6.0 Cost-Optimized Tool Contract

Every Agent-facing tool should document:

- default response bound
- full-output escape hatch
- compact mode behavior
- pagination or continuation behavior
- cache behavior
- error size limits
- safe examples for large resources

## Recommended Implementation Plan

| Priority | Work Item | Output |
|----------|-----------|--------|
| P0 | Define low-cost defaults for MCP core tools | MCP tool contract doc |
| P0 | Add README examples using bounded reads and compact search | Lower-cost onboarding path |
| P1 | Add conformance tests for oversized output behavior | Prevent regressions |
| P1 | Record response-size metrics by operation | Measurable optimization |
| P2 | Add per-connector cost notes | Connector-specific guidance |

## Open Decisions

- Whether all MCP tools should reject unbounded full reads unless explicitly requested.
- Whether default `max_lines` should vary by file type.
- Whether memory search should return summaries by default or raw excerpts.
- Whether connector manifests should declare expected output size classes.
