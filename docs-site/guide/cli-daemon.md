---
title: Daemon (HTTP)
description: anycode daemon — shared runtime with run, health and task endpoints.
summary: Bind address, POST /v1/tasks JSON shape, and optional bearer token.
read_when:
  - You want HTTP-triggered tasks on localhost.
---

# Daemon (HTTP)

Uses the same **`initialize_runtime`** path as **`run`** (LLM, tools, **`SecurityLayer`**, sandbox).

```bash
anycode daemon --bind 127.0.0.1:8080
```

- **`GET /health`** — returns plain text **`ok`**.  
- **`POST /v1/tasks`** — **`Content-Type: application/json`**:

```json
{
  "agent": "general-purpose",
  "prompt": "Your task",
  "working_directory": null
}
```

**`working_directory`** omitted or **`null`** means the process current directory (canonicalized).

If **`ANYCODE_DAEMON_TOKEN`** is set, **`POST /v1/tasks`** must include:

- **`Authorization: Bearer <token>`**, or  
- **`X-Anycode-Token: <token>`**

**`/health`** is not token-protected. Prefer binding to loopback only.

## Related

- [Architecture](./architecture) — daemon shares **AgentRuntime** assembly  
- [Troubleshooting](./troubleshooting)  
