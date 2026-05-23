# Production Harness Hardening

**Status:** Planned · **Owner surface:** `AgentRuntime`, eval harness, tool catalog, memory pipeline, Digital Workbench

This plan adds a Tier 1.5 hardening layer between the completed Digital Workbench V3 control plane and the larger Tier 2/3 items such as connector write-back, SSO/RBAC, and desktop packaging.

The goal is to turn anycode from a locally observable agent CLI into a production-grade Multi-Agent Harness: the runtime keeps global control over budgets, tools, traceability, evaluation, and memory governance while agents provide local intelligence.

## Principles

1. **`AgentRuntime` remains the orchestration authority.** Do not introduce a parallel execution engine or let a planner directly invoke worker agents.
2. **Budgets are runtime constraints, not only dashboard reports.** Token, cost, duration, and tool-call limits must be checked during execution.
3. **Trace is the shared source of truth.** Dashboard replay, trajectory eval, audit reports, and provenance should consume the same structured execution trace.
4. **Tools are governed resources.** Tool registry metadata should capture risk, category, approval requirements, agent visibility, and audit policy.
5. **MCP is routed through the harness.** MCP tools stay behind whitelist/quota/trace controls; agents should not treat MCP servers as ungoverned direct access.
6. **Memory must be maintained.** Long-lived memory needs retention scoring, dry-run pruning, and evidence provenance before deeper graph-memory integration.

## Non-goals

- No marketing cloud, CDP, customer segmentation, or enterprise campaign journey features.
- No SSO/OIDC, RBAC enforcement, multi-tenant server mode, or Tauri desktop shell in this slice.
- No connector write-back until OAuth/write threat modeling is done.
- No replacement for the current workflow/task/runtime paths.

## Milestones

| Milestone | Outcome | Primary files |
|-----------|---------|---------------|
| M0 — Roadmap | Add this Tier 1.5 roadmap and keep English/Chinese planning docs aligned. | `docs/digital-workbench-next-steps*.md` |
| M1 — Execution trace | Emit structured trace events for tasks, turns, LLM calls, tool calls, gates, budgets, and task end states. | `crates/core`, `crates/agent/src/runtime`, `crates/dashboard` |
| M2 — Runtime budget | Enforce task-level token/cost/duration budgets with warning, degradation, and hard-stop states. | `crates/agent/src/runtime`, `crates/cli`, `crates/dashboard` |
| M3 — Trajectory eval | Extend mock eval to assert tool paths, repeated calls, forbidden tools, gates, and budget outcomes. | `crates/cli/src/commands/eval_mock.rs`, `scripts/eval` |
| M4 — Tool governance | Promote the tool catalog from id lists to metadata with risk, approval, category, and agent visibility. | `crates/tools/src/catalog.rs`, `crates/tools/src/registry.rs` |
| M5 — MCP governance | Add optional per-server quotas, strict whitelists, and MCP trace events. | `crates/tools`, `crates/cli`, `crates/dashboard` |
| M6 — Declarative plan | Validate workflow/plan metadata before execution; planners output plans, the harness decides execution. | `crates/core`, `crates/cli/src/tasks_workflow.rs` |
| M7 — Memory retention | Add retention dry-run/prune and evidence provenance for hot/vector memory. | `crates/core/src/memory_*`, `crates/memory` |
| M8 — Workbench operations | Surface budget, trace, eval verdicts, tool/MCP risk, and memory retention in the UI. | `crates/dashboard-ui` |

## Priority order

Start with the three foundation slices:

1. **Execution trace SSOT** — enables replay, eval, budget events, MCP provenance, and memory evidence.
2. **Runtime token/cost/duration budget** — moves cost control from retrospective metrics into the agent loop.
3. **Trajectory eval in CI** — prevents regressions where the final answer looks successful but the execution path is unsafe or wasteful.

After those land, implement tool governance and MCP governance, then declarative workflow planning and memory retention.

## Acceptance criteria

- `anycode run`, `goal`, `workflow`, `repl`, and cron sessions can emit a structured execution trace without breaking existing `output.log` ingestion.
- Budget strict mode can stop execution before another tool call after the hard limit is exceeded.
- Eval can fail a mock run based on trajectory violations even when the final text is otherwise successful.
- Tool metadata tests fail when a default tool is missing governance metadata.
- MCP strict mode exposes only whitelisted tools and enforces per-server call limits.
- Workflow validation rejects unknown agents, cyclic dependencies, and disallowed tools before execution.
- Memory prune supports dry-run output before destructive cleanup.
- Digital Workbench explains why a session is trusted, blocked, over budget, or trajectory-failed.

## Verification

Run the normal workspace checks plus the dashboard and eval suites:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace
python3 scripts/eval/run.py --with-mock
cd crates/dashboard-ui && npm test && npm run test:e2e
ANYCODE_BUILD_DASHBOARD_UI=1 ./scripts/build-dashboard-ui.sh
ANYCODE_BUILD_DASHBOARD_UI=1 cargo build --release -p anycode --features embedded-ui
cargo build --release -p anycode
```
