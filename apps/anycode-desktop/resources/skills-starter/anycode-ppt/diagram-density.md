# 讲解图信息密度（anycode-ppt）

**页数不限**（≥2 页即可）。按叙事选模板，同一模板可多次使用。

每页 **content** 类型 slide 须至少有 **一类主视觉** 或 **`<img>` 插图**，禁止「只有标题」空页。

## 主视觉类名（validate 会检查）

| 类名 | 用途 |
|------|------|
| `.ladder` | 流程/阶段梯子 |
| `.layer-stack` / `.layer-stack-4` | 分层架构 |
| `.agent-cycle` | 五步环状循环 |
| `.trio` | 三列卡片 |
| `.metrics` | KPI 数字块 |
| `.timeline` | 横向路线图 |
| `.checklist` | 行动清单 |
| `.quote` | 金句/结论 |
| `.duo` | 双卡对比 |
| `.diagram-box` | SVG/图示框 |
| `<img src="...">` | 插图/截图 |

## 页型选用原则

- **cover / closing**：可选；长 deck 可多个 section 分段
- **section**：议程 ≥2 项即可
- **content**：按 `components.md` 选模板
- 有现成架构图/产品截图 → `diagram-image.html`

## 验收

- 每 content 页：主视觉类 **或** `<img>` + 具体名词
- `run slides/` → validate 全绿 + 生成 `index.html`
