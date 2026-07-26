# Agent execution quality — eval protocol (M0)

Four arms share TaskCompiler, GatePolicy, CompletionGuard, model, temperature, and budget.
Only two factors switch:

| arm | `ANYCODE_EVAL_EXPERIENCE` | `ANYCODE_EVAL_SKILLS` |
|-----|---------------------------|------------------------|
| baseline | 0 | 0 |
| experience_only | 1 | 0 |
| skill_only | 0 | 1 |
| experience_skill | 1 | 1 |

Require `ANYCODE_EVAL_MODE=1` for overrides (ignored otherwise).

## Data splits

- `train/` — write Experience cards / Skill SOP only
- `dev/` — tune routing, diagnostics, validators (not promotion)
- `hidden/` — promotion evidence; never enter Experience examples, Skill bodies, or judge prompts
- `challenge/` — no-match skill, near-miss retrieval, dependency missing, approval denied

Legacy `test/benchmarks/experience-baseline/scenes-v3.json` cases are **smoke only**.
Do **not** use Python `PACK_CARDS` mirrors for Enhanced arms — Experience must come from
the Rust `TaskCompiler` / builtin pack injection path under `ANYCODE_EVAL_*`.

## Model

```
quality = task_fixed_effect + βE*E + βS*S + βES*(E*S) + error
```

Primary contrasts: `Q11-Q10` (Skill vs Experience-only), `Q11-Q01` (Experience vs Skill-only).
Cluster bootstrap by task (10k), Holm-corrected CIs.
If 30 tasks still cannot reach power≥0.8 for +8/100, mark promotion **inconclusive**.

## Runner

Use `test/runner/executor.py` (Dashboard → AgentRuntime), not one-shot chat scripts.

```bash
ANYCODE_EVAL_MODE=1 ANYCODE_EVAL_EXPERIENCE=1 ANYCODE_EVAL_SKILLS=1 \
  python3 test/run.py --suite agent-quality --models deepseek-v4-flash
```

Each result must record: task_spec, experience hits, selected skills, tool trace,
gate report, artifact hashes, tokens, latency, cost, effective arm.
