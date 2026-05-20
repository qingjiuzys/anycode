# Eval Harness

Production changes should be validated by repeatable scenarios, not only by
unit tests. The first harness is intentionally deterministic and credential-free:

- `anycode eval list` lists built-in readiness scenarios.
- `anycode eval run --json` reports the registered scenarios in a stable JSON
  shape for CI/nightly wiring.
- `scripts/eval/run.py` executes CLI scenarios against an isolated `$HOME` so
  long-running local bridges cannot lock the user's normal memory store.
- `anycode eval run --mock` (and `scripts/eval/run.py --with-mock`) runs a
  fixture-repo task against a local TCP mock LLM — no real provider credentials.

## Scenario Classes

| Class | Purpose |
|-------|---------|
| CLI smoke | Help/status/doctor commands must not need provider credentials. |
| Automation | Cron ledger reads must succeed when the ledger is empty. |
| Ops | Doctor output must include memory/channel/MCP state. |
| Mock fixture | `scripts/eval/fixtures/minimal-repo` + local mock LLM for `anycode run`. |
| Future repo tasks | SWE-style fixture repositories with expected diffs and commands. |

## Expansion Path

1. ~~Add local fixture repository task that uses a mock LLM.~~ (`eval run --mock`)
2. Add 10 local fixture repository tasks that use a mock LLM.
2. Record changed files, tool calls, wall time, and exit status for every run.
3. Move slow scenarios to nightly CI after the fast smoke set is stable.

