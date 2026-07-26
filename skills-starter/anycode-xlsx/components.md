# anycode-xlsx 组件索引

**链路**：`workbook.json` 填数 → validate → **XLSX 终稿**（+ 可选 HTML 预览）

| 层 | 文件 | 作用 |
|----|------|------|
| 数据源 | `workbook.json` | 结构化 sheet/rows，validate |
| 预览 | `workbook.preview.html` | Workbench 快速看表（可选） |
| 终稿 | `workbook.xlsx` | 对外交付、Excel 编辑 |

## 模板

| 文件 | 场景 |
|------|------|
| `workbook-sales.json` | 销售 |
| `workbook-finance.json` | 财务季报 |
| `workbook-kpi.json` | KPI |

## 命令

```bash
~/.anycode/skills/anycode-xlsx/run workbook.json output.xlsx
```
