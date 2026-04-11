# ADR 001: Memory pipeline vs `MemoryStore`

## Status

Accepted

## Context

anyCode supports multiple memory backends (`noop`, `file`, `hybrid`, `pipeline`) configured via CLI. The core trait `MemoryStore` (CRUD + recall) is the stable port; `MemoryPipeline` (in `anycode_core::memory_pipeline`) adds buffering, promotion, and optional vector recall.

## Decision

- **`MemoryStore`** remains the narrow port for tools and compatibility layers that need stable recall/save semantics.
- **`backend=pipeline`** uses a single implementation that implements **both** `MemoryStore` and `MemoryPipeline` where needed: the CLI exposes one constructed object through `build_memory_layer` (`crates/cli/src/bootstrap/mod.rs`), returning `(Arc<dyn MemoryStore>, Option<Arc<dyn MemoryPipeline>>)` so `AgentRuntime` can hook ingest and durability without duplicating backends.
- Callers should not assume `MemoryPipeline` is always `Some`; non-pipeline backends pass `None`.

## Consequences

- New persistence features should extend the pipeline implementation or add a new backend branch in `build_memory_layer`, not bypass the composition root.
- Tests for memory belong in `anycode-memory` and CLI bootstrap integration as appropriate.
