---
title: Session notifications
description: HTTP or shell hooks for session events (OpenClaw-style gateways).
read_when:
  - You want outbound webhooks independent of memory pipeline hooks.
---

# Session notifications (`config.json` → `notifications`)

Session notifications send **JSON** to an **HTTP URL** and/or a **shell command** (stdin) when tool results or agent turns complete. They are **independent** of **`memory.pipeline.hook_*`** (embedding / memory side effects).

- **User guide (config fields):** [Config & security](./config-security) — same file, different section.
- **Maintainer lifecycle (MCP):** repo [`docs/mcp-stdio-lifecycle.md`](https://github.com/qingjiuzys/anycode/blob/main/docs/mcp-stdio-lifecycle.md)

## Fields (summary)

| Field | Meaning |
|-------|---------|
| `after_tool_result` | Fire after each tool result (when configured). |
| `after_agent_turn` | Fire when the assistant finishes a turn with no further tool calls. |
| `http_url` | `http` or `https` POST; body is JSON. |
| `http_timeout_ms` | Client timeout. |
| `http_headers` | Extra headers; values expand **`${VAR}`** from the environment (missing → empty). |
| `shell_command` | Run via `/bin/sh -c` (Unix) or `cmd /C` (Windows); **JSON written to stdin** (UTF-8). |
| `shell_timeout_ms` | Subprocess wall timeout. |
| `max_body_bytes` | Caps serialized **`excerpt`**; must be between **256** and **524288**. |
| `tool_deny_prefixes` | Skip notify when tool name starts with one of these prefixes (e.g. `mcp__`). |

Empty / whitespace-only **`http_url`** and **`shell_command`** mean “not configured” for that channel (same as `is_configured()` in code).

## JSON payload

Each delivery uses **`schema_version`: 1** and a unique **`event_id`** (UUID string) for gateway deduplication.

Example (values illustrative):

```json
{
  "schema_version": 1,
  "event_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
  "event": "tool_result",
  "session_id": "…",
  "task_id": "…",
  "turn": 2,
  "tool_name": "bash",
  "excerpt": "…",
  "excerpt_truncated": false,
  "timestamp": "2026-04-20T12:00:00.000Z",
  "working_directory": "/path/to/project"
}
```

Events include at least **`tool_result`** and **`agent_turn`** (exact strings match runtime).

## Difference from `memory.pipeline` hooks

| | **`notifications`** | **`memory.pipeline.hook_*`** |
|--|--------------------|--------------------------------|
| Purpose | Outbound integration (gateways, scripts) | Memory / embedding pipeline |
| Payload | Versioned JSON (`schema_version`, `event_id`, …) | Pipeline-specific |
| Failure | Logged; **does not** fail the agent turn | Depends on hook implementation |

## OpenClaw-style gateway (minimal)

Point **`http_url`** at your gateway (e.g. `https://gateway.example/hooks/anycode`). Optionally set **`Authorization: Bearer ${OPENCLAW_TOKEN}`** in **`http_headers`**. Your service should accept **POST JSON**, respond **2xx**, and use **`event_id`** if you need idempotent ingest.

## Observability

At **`tracing`** level **debug**, target **`anycode_session_notify`** logs **host**, **event**, **excerpt_truncated**, and **elapsed_ms** for HTTP (no full excerpt or secrets).

Chinese: [会话通知（中文）](/zh/guide/notifications).
