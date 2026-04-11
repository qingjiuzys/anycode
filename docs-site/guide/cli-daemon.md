---
title: HTTP daemon (removed)
description: The anycode HTTP daemon subcommand is removed; use run, REPL/TUI, or channel bridges instead.
summary: Historical note and ADR 003 — no localhost POST /v1/tasks API in tree.
read_when:
  - You followed an old link to `anycode daemon` or POST /v1/tasks.
---

# HTTP daemon (removed)

The **`anycode daemon`** HTTP server (**`GET /health`**, **`POST /v1/tasks`**) is **not** part of anyCode anymore. The **`daemon`** subcommand is **rejected** by the CLI (same as other removed commands), and the old `daemon_http` module was deleted.

**Use instead**

- **`anycode run`** for one-shot tasks from scripts or CI.  
- **`anycode repl`** / **`anycode tui`** for interactive sessions.  
- **`anycode channel …`** / **`anycode scheduler`** for long-lived or cron-style automation.  
- Shell out to the CLI from your own service if you need a custom HTTP front door.

**Decision record:** [ADR 003](https://github.com/qingjiuzys/anycode/blob/main/docs/adr/003-http-daemon-deprecated.md) (repository `docs/adr/`).

**Maintainer backlog:** [`docs/roadmap.md`](https://github.com/qingjiuzys/anycode/blob/main/docs/roadmap.md).

## Related

- [CLI overview](./cli) — current subcommands  
- [Architecture](./architecture)  
- [Roadmap](./roadmap) — MVP and tools matrix (no daemon)

Chinese: [HTTP 守护进程（已移除）](/zh/guide/cli-daemon).
