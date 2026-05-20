# Terminal Transcript Load Model

Virtual scroll and rewind should not be implemented until the terminal layer has
a repeatable load model. This file defines the production gates for future work
on ADR 004, ADR 005, and ADR 006.

## Load Tiers

| Tier | Transcript Size | Acceptance |
|------|-----------------|------------|
| S | 10k rendered lines | Smooth scroll, no stale cells after resize |
| M | 50k rendered lines | Frame latency remains usable; memory growth bounded |
| L | 100k rendered lines | Degrades gracefully; no panic or terminal corruption |

## Measurements

For each tier record:

- terminal size
- render latency sample
- peak resident memory if available
- scroll correctness
- resize correctness
- `/clear` behavior
- exit scrollback dump behavior

## Next Implementation Order

1. ~~Add a synthetic transcript generator test helper.~~ (`term/transcript/synthetic.rs`)
2. ~~Measure current transcript pipeline on Tier S/M fixtures.~~ (`tier_*_transcript_pipeline_*` tests)
3. ~~Tier L proxy benchmark~~ (`tier_l_transcript_pipeline_degrades_gracefully`, `cargo test tier_l -- --ignored`)
4. ~~Wire ADR 006 virtual scroll into stream REPL transcript rendering.~~ (`repl/stream_viewport.rs`: viewport windowing + overscan; runtime tests in `stream_viewport` Tier S/M)
5. Only then change transcript storage or session rewind semantics.

## Implementation Status (2026-05)

| Component | Status |
|-----------|--------|
| Synthetic fixtures (`synthetic.rs`) | Done |
| Pipeline Tier S/M/L benchmarks | Done (L: `--ignored`) |
| Virtual scroll runtime (stream REPL) | **Done** — `prepare_stream_transcript_view` builds only `[global_off − overscan, global_off + viewport + overscan)` styled rows; layout cache still tracks full prefix sums |
| Tier S/M scroll+resize+clear correctness | Covered by `stream_viewport` unit tests |
| Tier L (100k lines) runtime | **Gap** — layout cache still O(n) wrap on content change; no dedicated 100k runtime benchmark yet |

