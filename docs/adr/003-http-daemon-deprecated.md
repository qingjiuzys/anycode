# ADR 003: HTTP daemon is not a supported product surface

## Status

Accepted

## Context

The codebase previously carried an HTTP **`anycode daemon`** path (`daemon_http` and related wiring). That module was **removed from the default build** as unused / unmaintained relative to the primary surfaces: **`run`**, **fullscreen TUI**, **stream REPL**, and **channel bridges** (WeChat / Telegram / Discord).

Documentation and roadmap items still referred to an “optional daemon HTTP” and “restore daemon” as an open question, which created false expectations.

## Decision

1. **Do not restore** the in-process **HTTP task daemon** as a first-class feature. No new work should reintroduce `daemon_http` or a parallel HTTP server inside the default CLI binary without a **new ADR** that explicitly supersedes this one.
2. **Integration model**: Remote or automated invocation should use **`anycode run`**, **`scheduler`**, **channel** commands, or external orchestration that shells out to the CLI — not a localhost HTTP API maintained inside this repository.
3. **Naming**: Other uses of the word “daemon” (e.g. WeChat **bridge** process) are unrelated to this ADR; they remain valid.

## Consequences

- Docs-site pages that described **`POST /v1/tasks`** are **historical / removed**; see the **HTTP daemon (removed)** stub (source: [`docs-site/guide/cli-daemon.md`](../../docs-site/guide/cli-daemon.md)).
- Maintainer backlog and MVP wording must **not** list “optional daemon HTTP” as in-scope.
- If a future maintainer needs HTTP, they should treat it as a **separate service or fork**, or draft **ADR 00x** with security, auth, and composition-root boundaries before landing code.

## Related

- [`docs/roadmap.md`](../roadmap.md) — SSOT backlog; daemon row under **Decisions**
- [`002-cli-composition-root.md`](002-cli-composition-root.md) — composition root (update text: no daemon path)
