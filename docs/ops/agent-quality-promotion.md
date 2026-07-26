# Agent execution quality — promotion gate (M4)

## Architecture (frozen)

Experience + Skill + AgentRuntime + independent GatePolicy/ValidatorRegistry.

| Layer | May decide completion? |
|-------|------------------------|
| Experience | No — strategies / failure modes only |
| Skill | No — SOP / templates / run scripts only |
| AgentRuntime | Executes + repairs |
| GatePolicy + ValidatorRegistry | Yes — independent of Skill self-report |

## Eval arms

Require `ANYCODE_EVAL_MODE=1`. Factors:

| arm | EXPERIENCE | SKILLS |
|-----|------------|--------|
| baseline | 0 | 0 |
| experience_only | 1 | 0 |
| skill_only | 0 | 1 |
| experience_skill | 1 | 1 |

Shared across arms: TaskCompiler, capability intent, CompletionGuard, validators, budget, model params.

## Promotion criteria (all required)

1. `delivery_quality_score` vs Experience-only ≥ **+8**, Holm-corrected task-clustered 95% CI lower bound **> 0**.
2. vs Skill-only ≥ **+3** with CI lower bound **> 0**; otherwise default to **Skill-only**.
3. No new P0 hidden failures; family final success non-inferior within **-5pp**.
4. Visual blind win-rate ≥ **60%**, dual-judge agreement ≥ **70%**.
5. No identical-diagnostics repair loops.
6. Per successful delivery cost ≤ **1.8×** baseline; latency ≤ **2.0×**.
7. Workbench and daemon/headless paths behave consistently.

If power < 0.8 at 30 distinct tasks for the +8 delta → **inconclusive** (do not loosen CI or treat reps as new tasks).

## Blind judge (visual / narrative only)

- Strip arm / model / filename; randomize left/right.
- ≥2 judges; report W/T/L + agreement.
- Judge **cannot** overturn P0 machine failures.
- Disagreement or machine-fail+judge-high → human arbitration.

## Human sample

- All visual regressions
- All judge disagreements
- All machine-fail + judge-high
- 10% random pairs per family

## Lab status

Until hidden promotion passes, keep Experience+Skill as **lab/opt-in**. Do not claim stable quality uplift to users.

## Commands

```bash
# Dry-run four-arm orchestration
python3 scripts/run-agent-quality.py --models deepseek-v4-flash --split dev --dry-run

# Office commercial pipeline (DOCX/PPTX/XLSX + quality-score vs baseline)
python3 scripts/run-agent-quality-office.py --scenes docx,pptx,xlsx,pptx_commercial \
  --arms experience_skill --baseline test/benchmarks/agent-quality/results/office-20260722-223000

# Summarize (synthetic lab rows)
python3 scripts/summarize-agent-quality.py \
  test/benchmarks/agent-quality/results/dev-web-synthetic.jsonl \
  --out test/benchmarks/agent-quality/results/dev-web-summary.json
```

## Office commercial promotion (M4 extension)

Machine gates (P1) for OfficeDelivery artifacts:

| Validator | Requirement |
|-----------|-------------|
| `office.docx_commercial` | Header/footer parts + Heading styles |
| `office.pptx_editable` | Real `a:t` text; **reject** full-slide raster decks |
| `office.pptx_density` | ≥5 native `<p:sp>`/slide average (pics excluded) |
| `office.pptx_render_thumbs` | Auto-render `evidence/slide-*.png` from HTML (Playwright) |
| `office.xlsx_style` | ≥3 sheets (Summary + Detail + Pricing) + styled header fill |

Promotion vs legacy baseline (`office-20260722-223000`):

- `editable_commercial_score` delta ≥ **+15**
- PPT: `office.pptx_editable` pass + shapes/slide ≥ **5** + ≥ **1** render thumb
- DOCX: header + footer present
- XLSX: ≥ **3** sheets

See `quality-score.json` emitted by `scripts/run-agent-quality-office.py`.

## Evidence locations

- Protocol: `test/benchmarks/agent-quality/PROTOCOL.md`
- Manifest: `test/benchmarks/agent-quality/manifest.json`
- Splits: `train/` `dev/` `hidden/` `challenge/`
- Mock repair loop: `crates/agent` test `completion_guard_repairs_then_passes_web_landing`
