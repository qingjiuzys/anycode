# Skill Bake-off

真实模型评测开源 Agent Skills 候选（`deepseek-v4-flash`），**先看产出再决定是否内置**。

## 流程

```bash
# 1) 从 /tmp/skill-bakeoff-src 等来源装载候选（需先 clone anthropics/skills + vercel-labs/agent-skills）
python3 eval/skill-bakeoff/scripts/stage_skills.py --clean

# 2) 跑全量（默认 :43199，写入 runs/<timestamp>/）
python3 eval/skill-bakeoff/scripts/run_bakeoff.py --port 43199

# 子集
python3 eval/skill-bakeoff/scripts/run_bakeoff.py --only baseline-mindmap,cand-frontend-design
```

## 产物

- `CANDIDATES.md` — 18 候选 + 4 基线清单
- `skills-candidates/` — 带 `bakeoff-` 前缀的暂存 skill（不进 `skills-starter`）
- `runs/<ts>/SCORECARD.md` — 评分卡（人工列待填）
- `runs/<ts>/<case>/artifacts/` — 模型落盘产物
- `runs/LATEST` → 最近一次 run

## 决策门

填完 SCORECARD 的 `human_quality` / `ship_builtin?` 后，再决定是否改编进 `skills-starter/`。
