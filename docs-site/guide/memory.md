---
title: Memory notes
description: anyCode memory stores vs OpenClaw-style memory extensions.
summary: Current backends, scopes, and a short parity backlog.
read_when:
  - You compare memory behavior with OpenClaw or Claude Code.
---

# Memory notes

## Current anyCode behavior

- **Backends**: `memory.backend` supports `file`, `hybrid`, `noop`, or **`pipeline`** (归根通道: ephemeral buffer → reinforce → hot Sled → optional vector; see `anycode_memory::RootReturnMemoryPipeline`). Aliases: `layered`, `guigen`.
- **Legacy Markdown**: With `pipeline` and `memory.pipeline.merge_legacy_file_recall` (default true), existing `*.md` under the memory root are merged **read-only** into recall alongside the hot layer.
- **Scope**: Project vs user memories flow through `anycode_memory` with keyword retrieval today; pipeline adds pre-semantic fragments before promotion.
- **Autosave**: Controlled by `memory.auto_save` and successful task completion hooks in the agent runtime. With **`pipeline`**, autosave **ingests** into the buffer (not a direct durable write); repeated touches promote to the hot store.

Optional `memory.pipeline` JSON fields include: `buffer_ttl_secs`, `max_buffer_fragments`, `promote_touch_threshold`, `reinforce_on_recall_match`, `merge_legacy_file_recall`, `buffer_wal_enabled`, `buffer_wal_fsync_every_n`, `hook_after_tool_result`, `hook_after_agent_turn`, `hook_max_bytes`, `hook_tool_deny_prefixes`, `embedding_enabled`, `embedding_model`, `embedding_base_url`, `embedding_provider`, `embedding_local_cache_dir`.

- **WAL**: With `buffer_wal_enabled` (default true), the ephemeral buffer is appended to a `*.pipeline.buffer.wal` JSONL file next to the hot Sled DB and replayed on startup. The last `fsync` happens periodically (`buffer_wal_fsync_every_n`), after each unit of work in long-running bridges (Telegram, Discord, built-in scheduler, WeChat task, `run`/orchestration-triggered work), and again when the pipeline store is dropped (normal process exit).
- **Vectors**: With `embedding_enabled` or a non-empty `embedding_model`, or **`embedding_provider` set to `local`**, the pipeline stores vectors in `*.pipeline.vec.sled` (cosine retrieval).  
  - **`embedding_provider`**: `http` (default) uses OpenAI-compatible `POST …/embeddings` and `llm.api_key` (override host with `embedding_base_url`).  
  - **`local`**: ONNX Runtime via [FastEmbed](https://github.com/Anush008/fastembed-rs) (`all-MiniLM-L6-v2`, downloaded on first use). Requires building anycode with **`--features embedding-local`**. Optional `embedding_local_cache_dir` overrides the model cache directory (otherwise fastembed’s default, often under `~/.cache/fastembed`).
- **CLI import**: `anycode memory import [--dry-run] [--limit N]` copies legacy Markdown memories from `memory.path` into the pipeline hot store (requires `memory.backend: pipeline`).

## OpenClaw parity (research backlog)

OpenClaw ships memory as an **extension** with its own retention and recall policies. A practical parity checklist:

1. **Write triggers**: when notes are created (task success only vs tool-driven vs explicit commands).
2. **Retrieval**: keyword vs embedding / hybrid; per-project isolation guarantees.
3. **Compaction interaction**: how memories survive `/compact` and automatic session compression.

Track improvements against this list in issue tracker milestones rather than duplicating OpenClaw internals inside the CLI binary.

## Related

- [Architecture](./architecture)  
- [Config & security](./config-security)  
