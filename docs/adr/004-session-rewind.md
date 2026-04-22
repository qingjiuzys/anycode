# ADR 004: Session rewind / undo presentation (draft)

## Status

Proposed

## Context

Users may want to **rewind** or **undo** visible conversation state relative to persisted **`sessions`** JSON. Today, load/resume and `/clear` semantics are documented in [`docs/architecture.md`](../architecture.md) and [`docs/roadmap.md`](../roadmap.md), but there is no agreed product behavior for:

- Rewinding **transcript** vs **agent `messages`** vs **on-disk snapshot** together or independently.
- Whether rewind is **UI-only** (scroll/viewport) or **mutates** the session file and the next model turn.

## Decision

_To be filled after discussion._

## Options (for discussion)

1. **Snapshot-only**: Rewind = load an older snapshot file (if we add versioning or multiple checkpoints).
2. **Messages truncate**: Remove tail of `messages` and rebuild transcript from scratch (expensive; must match stream/TUI rebuild rules).
3. **No rewind**: Document that anyCode does not support rewind; users use `/clear` + export.

## Consequences

_Depends on chosen option._

## Related

- [`docs/roadmap.md`](../roadmap.md) §6  
- Session parity table in [`docs/architecture.md`](../architecture.md)
