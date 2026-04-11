---
title: Contributing extensions
description: Where to add tools, LLM providers, channels, and memory-related code in the anyCode workspace.
summary: Checklist-style map of crates and files for contributors extending the default runtime.
read_when:
  - You add a default tool, provider, or channel integration.
---

# Contributing extensions

This page lists **where to change code** for common extensions. For layering rules and orchestration authority, see [Architecture](./architecture). For **ADR**-style decisions (e.g. `AgentRuntime` as the only orchestration path), see the repo folder `docs/adr/`.

## New default tool

1. Implement [`Tool`](https://github.com/qingjiuzys/anycode/blob/main/crates/core/src/traits.rs) in `crates/tools` (or a submodule).
2. Register in `crates/tools/src/registry.rs` — follow the **checklist comment** at the top of that file (`ins!`, `DEFAULT_TOOL_IDS`, tests).
3. If the tool is sensitive (writes files, spawns sub-agents, etc.), add its API name to `SECURITY_SENSITIVE_TOOL_IDS` in `crates/tools/src/catalog.rs` so `bootstrap` can register `SecurityLayer` policies consistently.
4. Run `cargo test -p anycode-tools` and `cargo test --workspace`.

## New LLM provider or transport

1. Implement `LLMClient` in `crates/llm` (see existing providers under `crates/llm/src/providers/`).
2. Wire routing in `transport_for_provider_id` / `MultiProviderLlmClient` as appropriate (`crates/llm/src/lib.rs`, `provider_catalog.rs`).
3. Add or extend tests under `crates/llm`.

## New channel (WeChat / web / …)

1. Implement `ChannelHandler` from `anycode-core` in `crates/channels`.
2. The main CLI path may not depend on `channels` yet; integrate at the composition root (`crates/cli`) when adding a user-facing entrypoint.

## Memory backends and pipeline

- **File / hybrid / noop**: configured via CLI `bootstrap` → `build_memory_layer` (`crates/cli/src/bootstrap/mod.rs`).
- **Pipeline** (vector + optional embedding): types in `crates/core/src/memory_pipeline.rs`, implementation `crates/memory`. See [`docs/adr/001-memory-pipeline-and-store.md`](https://github.com/qingjiuzys/anycode/blob/main/docs/adr/001-memory-pipeline-and-store.md).

## Quick navigation

| Goal | First file to open |
|------|-------------------|
| Tool registry | `crates/tools/src/registry.rs` |
| Tool catalog / sensitive IDs | `crates/tools/src/catalog.rs` |
| Runtime assembly | `crates/cli/src/bootstrap/runtime.rs` |
| Agent loop | `crates/agent/src/runtime/session.rs`, `mod.rs` |
