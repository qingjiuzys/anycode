# ADR 006: Transcript virtual scroll reinstatement (RFC draft)

## Status

Proposed

## Context

`virtual_scroll` was removed from the default path to avoid unmaintained code. [`docs/tui-smoothness-baseline.md`](../tui-smoothness-baseline.md) defines **Phase 0** metrics and a **backlog** note for revisiting virtual scroll.

Before reintroducing any virtual scroll layer, we need explicit **load targets** (max transcript length, scroll frequency, latency budget) and a retest plan against Phase 0.

## Decision

_To be filled._ Default stance: **do not reintroduce** until this RFC records targets and a maintainer signs off.

## Proposed acceptance (for implementation phase)

- Document max **logical lines** or **bytes** the UI must handle at 60fps-equivalent redraw on reference terminals (see baseline matrix).
- PgUp/PgDn and mouse wheel behavior defined for **workspace** vs **main transcript** if both exist.
- No regression on CSI `?2026` sync path per baseline doc.

## Related

- [`docs/tui-smoothness-baseline.md`](../tui-smoothness-baseline.md)  
- [`docs/roadmap.md`](../roadmap.md) §4 / §6
