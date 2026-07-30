# anycode-docx 组件索引

**链路**：MD 写作 → **HTML 预览** → **DOCX 终稿**

| 层 | 文件 | 作用 |
|----|------|------|
| 写作 | `report.md` | Agent 填内容、git diff、validate |
| 预览 | `report.preview.html` | Workbench 浏览器评审（FDE 样式） |
| 终稿 | `report.docx` | 对外交付、Word 编辑 |

## 模板

| 文件 | 场景 |
|------|------|
| `work-report.md` | 周报 / 月报 |
| `performance-review.md` | 述职 / OKR |
| `briefing-note.md` | 短简报 |

## validate 规则

- ≥1 H1、≥2 H2
- ≥1 行 Decision/Action（含负责人 + 日期）
- 禁 placeholder

## 命令

```bash
~/.anycode/skills/anycode-docx/run report.md report.docx
open report.preview.html   # 先预览再发 docx
```
