# ADR 009: Graph memory spike

## Status

Proposed (spike complete; no runtime integration)

## Context

anyCode memory today is file-backed with optional vector retrieval (`pipeline` /
`hybrid`). Production evidence indexing (`~/.anycode/memory/evidence.jsonl`) gives
provenance for tool outputs but does not model relationships between entities,
sessions, or channel scopes.

Graph memory would help long-running agents answer questions like “which files did
we touch for issue X?” or “what did this cron job learn last week?” without stuffing
raw transcripts back into context.

## Decision (spike)

**Do not integrate graph memory in 2026-05.** Capture the spike constraints below
so a future slice can start without reopening architecture basics.

### Candidate model

- **Nodes**: `session`, `task`, `tool_call`, `artifact`, `memory_chunk`, `channel`.
- **Edges**: `derived_from`, `mentioned_in`, `executed_in`, `notified_via`.
- **Storage**: append-only JSONL edge log under `~/.anycode/memory/graph.jsonl`
  mirroring evidence indexing; optional SQLite projection for queries.

### Non-goals for v1 graph

- No automatic entity extraction LLM pass in the hot path.
- No cross-user shared graph.
- No replacement for compaction or vector retrieval.

### Integration hooks (when implemented)

1. Write edges from existing `tool_audit` + `evidence` append paths.
2. Expose `anycode memory graph query --json` read-only CLI before agent retrieval.
3. Gate agent retrieval behind config `memory.graph.enabled` default `false`.

## Consequences

- Evidence + audit JSONL remain the production SSOT for May 2026.
- Graph memory is documented but deferred; avoids half-integrated retrieval paths.
