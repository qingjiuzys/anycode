# Eval Harness

Structured scenario evals run through the same **`AgentRuntime::execute_task`** path as Workbench and daemon — no second agent loop.

## Commands

| Command | Purpose |
|---------|---------|
| `cargo test --workspace` | Primary CI validation (mock LLM integration tests in crate test suites). |
| `python3 test/run.py --profile smoke` | Scenario assembly layer over manifest cases; consumes structured **`EvalResult`** JSON (not raw `output.log` text). |
| `cargo test -p anycode-core eval` | Unit tests for eval contract / judge. |

CI: see `.github/workflows/eval.yml` (scenario smoke) and `.github/workflows/ci.yml` (full workspace tests).

## Contract (`crates/core/src/eval.rs`)

- **`EvalScenario`** — id, prompt, optional agent/mode, **`EvalExpectation`**
- **`EvalResult`** — status, message, **`ExecutionTraceEvent`** trace, optional final text
- **`judge_eval_scenario`** — shared assertion helper for trace + terminal status + text contains/excludes

Runtime entry: **`AgentRuntime::execute_eval_scenario`** (`crates/agent/src/runtime/execute_eval.rs`).

## Workbench smoke

Manual: `/setup`, short chat, automations, channel bridges via `anycode-daemon`.

Historical note: the removed **`anycode eval`** CLI subcommand and **`scripts/eval/`** tree are superseded by the contract above plus `test/run.py`.
