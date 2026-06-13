# Autonomous 8-Hour Plan (2026-05-23)

User resting — no questions. Execute in order; stop only on hard blockers.

## Hour 0–1 — Hygiene & baseline

- [ ] `cargo clippy --fix` safe batch (unused imports, mut, sort_by_key)
- [ ] `npx playwright install` + full e2e (API + acceptance)
- [ ] Baseline: fmt, clippy, test, release build

## Hour 1–3 — CLI `model_interactive` split

- [ ] Extract `app_config/model_interactive/{routing,provider_flows}.rs`
- [ ] Keep `model_interactive.rs` as thin entry + re-exports
- [ ] `cargo test -p anycode` config/model smoke

## Hour 3–5 — Agent loop dedupe (conservative)

- [ ] Extract shared tool-call logging + `prepare_tool_result` callers already done
- [ ] Extract LLM turn polling block to `agentic_loop.rs` if diff stays small
- [ ] Agent integration tests green

## Hour 5–6 — Budget harness wiring (minimal)

- [ ] Wire `RuntimeBudgetState` into `execute_task` / `execute_turn` when `TaskBudget` non-empty
- [ ] Log `[budget_*]` events; stop on hard limit
- [ ] Unit tests in `budget.rs` already exist

## Hour 6–7 — Docs & tree

- [ ] `refactor-map.md`, `PROJECT_TREE.txt`, EN/ZH architecture
- [ ] `docs/autonomous-8h-log.md` progress notes

## Hour 7–8 — Final CI gate

- [ ] Full workspace test + UI build + e2e
- [ ] `cargo build --release -p anycode`
- [ ] Mark plan checkboxes in log

## Out of scope (defer)

- Tier 2 Workbench (SSO, Connector OAuth)
- Full `model_interactive` UX rewrite
- Terminal 100k tier
