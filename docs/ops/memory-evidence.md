# Memory Evidence Index

anyCode now writes a minimal evidence index for tool outputs that often carry
recoverable facts:

```text
~/.anycode/memory/evidence.jsonl
```

Indexed tools include file/search/fetch/MCP style tools:

- `FileRead` / `Read`
- `Grep` / `Glob`
- `WebFetch` / `WebSearch`
- `mcp` and dynamic `mcp__server__tool` calls

Rows include `task_id`, `tool_name`, a stable `content_hash`, and a short
preview. The index is designed for future compaction checkpoints: summaries can
refer to exact evidence hashes instead of relying only on lossy text summaries.

## Scope Rules

- Local **dream consolidation** (ADR 014) may promote structured episode events into long-term memory while retaining `evidence_hash` links.
- This is not a full memory-wiki; cloud sync (if enabled) stores **ciphertext only**.
- Channel messages should not automatically become project memory.
- Full graph memory remains a spike until evidence-index usage proves valuable.

