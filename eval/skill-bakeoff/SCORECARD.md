# Skill Bake-off SCORECARD (pointer)

最新完整跑批：[`runs/20260801-001322/SCORECARD.md`](./runs/20260801-001322/SCORECARD.md)

- Model: `deepseek-v4-flash`
- 22/22 cases 有结果（19 completed / 3 timeout / 0 error）
- **Decision (2026-08-01):** 除 `claude-api` 外，17 个候选已 promote 进 `skills-starter/`（并同步桌面 resources + `~/.anycode/skills`）

## 快速印象（机器侧，非最终决策）

| 倾向 | ids | 说明 |
|---|---|---|
| 产出扎实 | frontend-design, webapp-testing, doc-coauthoring, internal-comms, mcp-builder, theme-factory, web-artifacts-builder, slack-gif-creator, skill-creator, claude-api, vercel-web-design-guidelines, vercel-composition-patterns, vercel-writing-guidelines, find-skills | 指定 `out/` 有文件 |
| 弱 / 需复跑 | algorithmic-art, vercel-react-best-practices, design-taste-frontend | 0 文件或 timeout 无产物 |
| 基线对照 | anycode-xlsx(timeout但仍有 xlsx), deep-research, verify-discover, mindmap | 已有内置对照 |

请打开各 case 的 `artifacts/` 目视后再勾选是否内置。
