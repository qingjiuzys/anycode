# ADR 005: `/clear` vs plain-text transcript buffer (draft)

## Status

Proposed

## Context

**`/clear`** today clears agent-facing state (`rebuild_for_agent`, messages, stream HUD, **`stream_exit_dump_anchor`**, etc.) as described in the architecture session table. The **plain-text transcript** used for Inline stream rendering may not be identical to `messages` serialization.

Open questions:

- Should `/clear` also define **viewport-only** reset (scroll position) without clearing history?
- After `/clear`, should **scrollback dump** on exit always start from empty anchor, or match user expectation of “blank screen = blank dump”?

## Decision

_To be filled after aligning with target product behavior (e.g. Claude Code)._

## Consequences

_Depends on decision; may affect [`ReplLineState`](../../crates/cli/src/repl_inline.rs) and `tasks_repl` handlers._

## Related

- [`docs/architecture.md`](../architecture.md) — `/clear` row  
- [`docs/roadmap.md`](../roadmap.md) §6  
- `ANYCODE_STREAM_EXIT_SCROLLBACK_DUMP` in [`docs-site/guide/cli-sessions.md`](../../docs-site/guide/cli-sessions.md)
