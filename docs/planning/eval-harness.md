# Eval Harness

Production changes should be validated by repeatable scenarios, not only by
unit tests. The first harness is intentionally deterministic and credential-free:

- `anycode eval list` lists built-in readiness scenarios plus mock fixture repo tasks.
- `anycode eval run --json` reports the registered CLI scenarios in a stable JSON
  shape for CI/nightly wiring.
- `scripts/eval/run.py` executes CLI scenarios against an isolated `$HOME` so
  long-running local bridges cannot lock the user's normal memory store.
- `anycode eval run --mock` (and `scripts/eval/run.py --with-mock`) runs
  SWE-bench-lite style fixture-repo tasks against a local TCP mock LLM — no real
  provider credentials.

## Scenario Classes

| Class | Purpose |
|-------|---------|
| CLI smoke | Help/status/doctor commands must not need provider credentials. |
| Automation | Cron ledger reads must succeed when the ledger is empty. |
| Ops | Doctor output must include memory/channel/MCP state. |
| Mock fixture | Scripted mock LLM + fixture repos under `scripts/eval/fixtures/`. |
| Future repo tasks | Additional fixtures with expected diffs and verifier commands. |

## Mock fixture repos (`eval run --mock`)

| Scenario id | Fixture | Style |
|-------------|---------|--------|
| `mock-fixture-greet` | `minimal-repo` | Single-turn smoke (no edits). |
| `mock-fixture-bugfix` | `bugfix-repo` | Two-turn mock + temp copy; golden patch verifier runs `cargo test`. |
| `mock-fixture-multifile` | `multifile-repo` | Scripted `FileRead` on three paths; markers in output. |
| `mock-fixture-test-repair` | `test-repair-repo` | Two-turn mock + temp copy; golden patch verifier runs `cargo test`. |

Mutable fixtures run in a temp copy so the git tree stays clean. When the mock
agent path does not apply the scripted tool edit, the harness applies the golden
patch and still requires `cargo test` to pass (fixture + verifier smoke).

CI runs `python3 scripts/eval/run.py --with-mock` in the Rust workflow after the
release binary is built (7 CLI smoke rows + 1 aggregated mock row covering four
fixture scenarios).

## Expansion Path

1. ~~Add local fixture repository task that uses a mock LLM.~~ (`eval run --mock`)
2. ~~Add SWE-bench-lite style fixture repos (bugfix / multifile / test repair).~~
3. Add more fixture repos and optional real-LLM nightly tier.
4. Record changed files, tool calls, wall time, and exit status for every run.
5. Move slow scenarios to nightly CI after the fast smoke set is stable.
